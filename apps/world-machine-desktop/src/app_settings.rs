//! The app's own settings file: sound, display, language and the World
//! voice, stored under Application Support.

use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

const SETTINGS_VERSION: u32 = 1;
const SETTINGS_FILE_NAME: &str = "Analyst Settings.json";
static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static SETTINGS_UPDATE_LOCK: Mutex<()> = Mutex::new(());

/// Fields this version does not know (such as an older app's analyst
/// runtime paths) are ignored, so an older file still reads.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub version: u32,
    /// The local program the World voice writes with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pi_program: Option<PathBuf>,
    /// Whether the Worlds this app opens are given a voice — the same local
    /// program, asked to say what happened in a World's own words when you come
    /// back to it. Off unless somebody turns it on, and omitted from the file
    /// while it is off, so a settings file written before this existed reads
    /// exactly as it did.
    #[serde(default, skip_serializing_if = "is_off")]
    pub world_voice: bool,
    /// Which way the voice reaches a model. Absent means the local program,
    /// which is the only way that existed when this field did not, so a
    /// settings file written before it keeps behaving as it did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_voice_source: Option<VoiceSource>,
    /// Whether a World's window plays its landscape's quiet sound. Off
    /// unless somebody turns it on, and omitted from the file while off.
    #[serde(default, skip_serializing_if = "is_off")]
    pub ambient_sound: bool,
    /// The player's level for each sound (music, ambience, voices,
    /// interface), as a percentage. A sound not listed plays at its
    /// default level, and the field is omitted while all are.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub sound_levels: std::collections::BTreeMap<String, u8>,
    /// The language the app is shown in ("en", "zh-Hans"); absent follows
    /// the Mac.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// How large text is drawn, as a percentage from 100 to 200; absent is
    /// 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_scale: Option<u32>,
    /// Whether colours are drawn with more contrast; absent follows the
    /// Mac's Increase Contrast.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub increase_contrast: Option<bool>,
    /// Where a World shown as a strip goes: which edge, which display, and
    /// whether it stays in front. Omitted while it is as it starts.
    #[serde(default, skip_serializing_if = "StripSettings::is_default")]
    pub strip: StripSettings,
}

/// How a World shown as a strip along the edge of a screen is placed.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct StripSettings {
    /// Along the top of the screen rather than the bottom.
    #[serde(default, skip_serializing_if = "is_off")]
    pub top: bool,
    /// In front of every other window.
    #[serde(default, skip_serializing_if = "is_off")]
    pub always_on_top: bool,
    /// Which display it goes on, by the display's lasting identity; absent
    /// is the main display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

impl StripSettings {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

fn is_off(value: &bool) -> bool {
    !*value
}

/// A voice that is actually usable: switched on, with something behind it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfiguredVoice {
    Program(String),
    Key(String),
}

/// How a World's voice reaches a model.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VoiceSource {
    /// A program already on this Mac. Where it sends anything is between the
    /// observer and it.
    #[default]
    Program,
    /// A key the observer gave the app. The only way a World's contents leave
    /// the machine because of World Machine itself.
    Key,
}

impl AppSettings {
    /// The player's level for a sound, or its default if never set.
    pub fn sound_level(&self, channel: crate::ambience::Channel) -> u8 {
        self.sound_levels
            .get(sound_key(channel))
            .copied()
            .unwrap_or_else(|| channel.default_level())
            .min(100)
    }

    pub fn empty() -> Self {
        Self {
            version: SETTINGS_VERSION,
            pi_program: None,
            world_voice: false,
            world_voice_source: None,
            ambient_sound: false,
            sound_levels: Default::default(),
            language: None,
            text_scale: None,
            increase_contrast: None,
            strip: StripSettings::default(),
        }
    }

    /// Which way this app should give its Worlds a voice, if any.
    ///
    /// Every half has to be true — the voice turned on, a source chosen, and
    /// that source actually configured — so the rule lives here rather than
    /// being re-derived by each caller. A voice switched on with nothing behind
    /// it is no voice, and the Worlds read from their built-in copy.
    pub fn configured_voice(&self, stored_key: Option<String>) -> Option<ConfiguredVoice> {
        if !self.world_voice {
            return None;
        }
        match self.world_voice_source.unwrap_or_default() {
            VoiceSource::Program => Some(ConfiguredVoice::Program(
                self.pi_program.as_ref()?.display().to_string(),
            )),
            VoiceSource::Key => {
                let key = stored_key?;
                (!key.trim().is_empty()).then_some(ConfiguredVoice::Key(key))
            }
        }
    }

    pub fn validate(&self) -> Result<(), AppSettingsError> {
        if self.version != SETTINGS_VERSION {
            return Err(AppSettingsError::UnsupportedVersion(self.version));
        }
        if let Some(path) = self.pi_program.as_deref() {
            if !path.is_absolute() {
                return Err(AppSettingsError::InvalidPath {
                    field: "voice program",
                    path: path.to_path_buf(),
                });
            }
        }
        Ok(())
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug)]
pub enum AppSettingsError {
    Io(String),
    Malformed(String),
    UnsupportedVersion(u32),
    InvalidPath { field: &'static str, path: PathBuf },
}

impl fmt::Display for AppSettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => f.write_str(message),
            Self::Malformed(message) => {
                write!(f, "Settings are malformed: {message}")
            }
            Self::UnsupportedVersion(version) => {
                write!(f, "Settings use unsupported format version {version}")
            }
            Self::InvalidPath { field, path } => {
                write!(f, "The {field} path must be absolute: {}", path.display())
            }
        }
    }
}

impl std::error::Error for AppSettingsError {}

pub fn application_support_root() -> Result<PathBuf, AppSettingsError> {
    let home = env::var_os("HOME").ok_or_else(|| {
        AppSettingsError::Io("World Machine could not locate the user's home directory".to_string())
    })?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("World Machine"))
}

pub fn settings_path(root: &Path) -> PathBuf {
    root.join(SETTINGS_FILE_NAME)
}

pub fn load(root: &Path) -> Result<AppSettings, AppSettingsError> {
    let path = settings_path(root);
    if !path.exists() {
        return Ok(AppSettings::empty());
    }
    let mut file = File::open(&path).map_err(|error| {
        AppSettingsError::Io(format!(
            "World Machine could not read settings at {}: {error}",
            path.display()
        ))
    })?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|error| {
        AppSettingsError::Io(format!(
            "World Machine could not read settings at {}: {error}",
            path.display()
        ))
    })?;
    let settings: AppSettings = serde_json::from_str(&contents)
        .map_err(|error| AppSettingsError::Malformed(error.to_string()))?;
    settings.validate()?;
    Ok(settings)
}

pub fn save(root: &Path, settings: &AppSettings) -> Result<(), AppSettingsError> {
    settings.validate()?;
    fs::create_dir_all(root).map_err(|error| {
        AppSettingsError::Io(format!(
            "World Machine could not create settings directory {}: {error}",
            root.display()
        ))
    })?;
    let target = settings_path(root);
    let sequence = SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp = root.join(format!(
        ".{SETTINGS_FILE_NAME}.tmp-{}-{sequence}",
        std::process::id()
    ));
    let payload = serde_json::to_vec_pretty(settings)
        .map_err(|error| AppSettingsError::Malformed(error.to_string()))?;

    let write_result = write_temporary_settings(&temp, &payload);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }

    fs::rename(&temp, &target).map_err(|error| {
        let _ = fs::remove_file(&temp);
        AppSettingsError::Io(format!(
            "World Machine could not replace settings at {}: {error}",
            target.display()
        ))
    })?;
    Ok(())
}

pub fn save_pi_program(root: &Path, path: PathBuf) -> Result<(), AppSettingsError> {
    update_settings(root, move |settings| settings.pi_program = Some(path))
}

/// Turn a World's voice on or off. The program it uses is the one chosen
/// with `save_pi_program`; without one, this stays a preference with nothing
/// to act on, which is why the caller checks both.
pub fn save_world_voice(root: &Path, on: bool) -> Result<(), AppSettingsError> {
    update_settings(root, move |settings| settings.world_voice = on)
}

/// The player's level for one sound, from 0 to 100.
pub fn save_sound_level(
    root: &Path,
    channel: crate::ambience::Channel,
    percent: u8,
) -> Result<(), AppSettingsError> {
    update_settings(root, move |settings| {
        settings
            .sound_levels
            .insert(sound_key(channel).to_owned(), percent.min(100));
    })
}

fn sound_key(channel: crate::ambience::Channel) -> &'static str {
    match channel {
        crate::ambience::Channel::Music => "music",
        crate::ambience::Channel::Ambience => "ambience",
        crate::ambience::Channel::Voices => "voices",
        crate::ambience::Channel::Interface => "interface",
    }
}

/// The language the app is shown in, or `None` to follow the Mac.
pub fn save_language(root: &Path, language: Option<String>) -> Result<(), AppSettingsError> {
    update_settings(root, move |settings| settings.language = language)
}

/// How large text is drawn, from 100 to 200 percent.
pub fn save_text_scale(root: &Path, percent: u32) -> Result<(), AppSettingsError> {
    update_settings(root, move |settings| {
        settings.text_scale = (percent != 100).then_some(percent.clamp(100, 200))
    })
}

/// Whether colours are drawn with more contrast, or `None` to follow the
/// Mac.
pub fn save_increase_contrast(root: &Path, on: Option<bool>) -> Result<(), AppSettingsError> {
    update_settings(root, move |settings| settings.increase_contrast = on)
}

/// Whether World windows play their landscape's sound.
pub fn save_ambient_sound(root: &Path, on: bool) -> Result<(), AppSettingsError> {
    update_settings(root, move |settings| settings.ambient_sound = on)
}

/// Which way the voice reaches a model.
pub fn save_world_voice_source(root: &Path, source: VoiceSource) -> Result<(), AppSettingsError> {
    update_settings(root, move |settings| {
        settings.world_voice_source = Some(source)
    })
}

/// How a World shown as a strip is placed.
pub fn save_strip(root: &Path, strip: StripSettings) -> Result<(), AppSettingsError> {
    update_settings(root, move |settings| settings.strip = strip)
}

fn update_settings(
    root: &Path,
    update: impl FnOnce(&mut AppSettings),
) -> Result<(), AppSettingsError> {
    let _guard = SETTINGS_UPDATE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut settings = load(root)?;
    update(&mut settings);
    save(root, &settings)
}

fn write_temporary_settings(temp: &Path, payload: &[u8]) -> Result<(), AppSettingsError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(temp)
        .map_err(|error| {
            AppSettingsError::Io(format!(
                "World Machine could not write temporary settings {}: {error}",
                temp.display()
            ))
        })?;
    file.write_all(payload).map_err(|error| {
        AppSettingsError::Io(format!(
            "World Machine could not write temporary settings {}: {error}",
            temp.display()
        ))
    })?;
    file.write_all(b"\n").map_err(|error| {
        AppSettingsError::Io(format!(
            "World Machine could not finish temporary settings {}: {error}",
            temp.display()
        ))
    })?;
    file.sync_all().map_err(|error| {
        AppSettingsError::Io(format!(
            "World Machine could not sync temporary settings {}: {error}",
            temp.display()
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            Self {
                root: env::temp_dir().join(format!(
                    "world-machine-app-settings-{}-{nonce}-{sequence}",
                    std::process::id()
                )),
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn absent_file_loads_defaults() {
        let fixture = Fixture::new();
        assert_eq!(load(&fixture.root).unwrap(), AppSettings::empty());
    }

    #[test]
    fn save_load_round_trip() {
        let fixture = Fixture::new();
        let settings = AppSettings {
            version: SETTINGS_VERSION,
            pi_program: Some(PathBuf::from("/usr/local/bin/pi")),
            world_voice: false,
            world_voice_source: None,
            ambient_sound: false,
            sound_levels: Default::default(),
            language: None,
            text_scale: None,
            increase_contrast: None,
            strip: StripSettings::default(),
        };
        save(&fixture.root, &settings).unwrap();
        assert_eq!(load(&fixture.root).unwrap(), settings);
    }

    #[test]
    fn malformed_json_is_not_overwritten() {
        let fixture = Fixture::new();
        fs::create_dir_all(&fixture.root).unwrap();
        let path = settings_path(&fixture.root);
        fs::write(&path, "{not-json").unwrap();
        assert!(matches!(
            save_pi_program(&fixture.root, PathBuf::from("/new/pi")),
            Err(AppSettingsError::Malformed(_))
        ));
        assert_eq!(fs::read_to_string(path).unwrap(), "{not-json");
    }

    #[test]
    fn unsupported_version_is_rejected_without_data_loss() {
        let fixture = Fixture::new();
        fs::create_dir_all(&fixture.root).unwrap();
        let path = settings_path(&fixture.root);
        let contents = r#"{"version":2,"pi_program":null}"#;
        fs::write(&path, contents).unwrap();
        assert!(matches!(
            save_world_voice(&fixture.root, true),
            Err(AppSettingsError::UnsupportedVersion(2))
        ));
        assert_eq!(fs::read_to_string(path).unwrap(), contents);
    }

    #[test]
    fn relative_paths_are_rejected() {
        let mut settings = AppSettings::empty();
        settings.pi_program = Some(PathBuf::from("bin/pi"));
        assert!(matches!(
            settings.validate(),
            Err(AppSettingsError::InvalidPath { .. })
        ));
    }

    #[test]
    fn a_settings_file_from_an_older_app_still_reads() {
        // Fields an older app wrote, such as its analyst runtime paths, are
        // ignored rather than costing somebody their settings.
        let fixture = Fixture::new();
        fs::create_dir_all(&fixture.root).unwrap();
        fs::write(
            settings_path(&fixture.root),
            r#"{"version":1,"node_program":"/saved/node","pi_program":"/saved/pi","provider":"x"}"#,
        )
        .unwrap();
        let loaded = load(&fixture.root).unwrap();
        assert_eq!(loaded.pi_program, Some(PathBuf::from("/saved/pi")));
        assert!(
            !loaded.world_voice,
            "a voice nobody asked for was switched on"
        );
    }

    #[test]
    fn a_voice_needs_both_a_switch_and_something_behind_it() {
        // Any half missing means there is nothing to tell a World, so the app
        // passes it nothing and the World reads as it always has.
        let mut settings = AppSettings::empty();
        assert_eq!(settings.configured_voice(None), None);

        settings.world_voice = true;
        assert_eq!(
            settings.configured_voice(None),
            None,
            "a voice was claimed with nothing to write with"
        );

        settings.pi_program = Some(PathBuf::from("/saved/pi"));
        assert_eq!(
            settings.configured_voice(None),
            Some(ConfiguredVoice::Program("/saved/pi".to_string())),
            "a settings file from before sources existed must keep using its program"
        );

        settings.world_voice = false;
        assert_eq!(
            settings.configured_voice(None),
            None,
            "a World was given a voice its observer had switched off"
        );
    }

    #[test]
    fn a_key_voice_is_the_stored_key_and_never_the_program() {
        let mut settings = AppSettings::empty();
        settings.world_voice = true;
        settings.world_voice_source = Some(VoiceSource::Key);
        settings.pi_program = Some(PathBuf::from("/saved/pi"));

        assert_eq!(
            settings.configured_voice(None),
            None,
            "a key voice was claimed with no key stored"
        );
        assert_eq!(
            settings.configured_voice(Some("   ".into())),
            None,
            "whitespace was accepted as a key"
        );
        assert_eq!(
            settings.configured_voice(Some("sk-ant-test".into())),
            Some(ConfiguredVoice::Key("sk-ant-test".to_string())),
            "the key voice fell back to the program that happens to be configured"
        );
    }

    #[test]
    fn the_way_a_voice_reaches_a_model_survives_being_written_down() {
        let fixture = Fixture::new();
        save_pi_program(&fixture.root, PathBuf::from("/saved/pi")).unwrap();
        assert_eq!(load(&fixture.root).unwrap().world_voice_source, None);

        save_world_voice_source(&fixture.root, VoiceSource::Key).unwrap();
        let stored = load(&fixture.root).unwrap();
        assert_eq!(stored.world_voice_source, Some(VoiceSource::Key));
        assert_eq!(
            stored.pi_program,
            Some(PathBuf::from("/saved/pi")),
            "choosing a key threw away the program"
        );

        save_world_voice_source(&fixture.root, VoiceSource::Program).unwrap();
        assert_eq!(
            load(&fixture.root).unwrap().world_voice_source,
            Some(VoiceSource::Program)
        );
    }

    #[test]
    fn a_voice_is_off_until_it_is_turned_on_and_stays_where_it_is_put() {
        let fixture = Fixture::new();
        save_pi_program(&fixture.root, PathBuf::from("/saved/pi")).unwrap();
        assert!(!load(&fixture.root).unwrap().world_voice);

        save_world_voice(&fixture.root, true).unwrap();
        let on = load(&fixture.root).unwrap();
        assert!(on.world_voice);
        assert_eq!(
            on.pi_program,
            Some(PathBuf::from("/saved/pi")),
            "turning the voice on lost the program it needs"
        );

        save_world_voice(&fixture.root, false).unwrap();
        assert!(!load(&fixture.root).unwrap().world_voice);
        // Off is the default, so it leaves no trace in the file.
        let written = fs::read_to_string(settings_path(&fixture.root)).unwrap();
        assert!(!written.contains("world_voice"), "{written}");
    }

    #[test]
    fn concurrent_updates_do_not_drop_a_field() {
        let fixture = Fixture::new();
        let root = Arc::new(fixture.root.clone());
        let barrier = Arc::new(Barrier::new(3));

        let voice_root = Arc::clone(&root);
        let voice_barrier = Arc::clone(&barrier);
        let voice = thread::spawn(move || {
            voice_barrier.wait();
            save_world_voice(voice_root.as_ref(), true).unwrap();
        });

        let pi_root = Arc::clone(&root);
        let pi_barrier = Arc::clone(&barrier);
        let pi = thread::spawn(move || {
            pi_barrier.wait();
            save_pi_program(pi_root.as_ref(), PathBuf::from("/concurrent/pi")).unwrap();
        });

        barrier.wait();
        voice.join().unwrap();
        pi.join().unwrap();

        let settings = load(root.as_ref()).unwrap();
        assert!(settings.world_voice);
        assert_eq!(settings.pi_program, Some(PathBuf::from("/concurrent/pi")));
    }

    #[test]
    fn where_a_strip_goes_is_kept_and_left_out_until_chosen() {
        let fixture = Fixture::new();
        save(&fixture.root, &AppSettings::empty()).unwrap();
        let written = fs::read_to_string(settings_path(&fixture.root)).unwrap();
        assert!(!written.contains("strip"));
        let strip = StripSettings {
            top: true,
            always_on_top: true,
            display: Some("display-2".into()),
        };
        save_strip(&fixture.root, strip.clone()).unwrap();
        assert_eq!(load(&fixture.root).unwrap().strip, strip);
    }

    #[test]
    fn save_replaces_target_without_leaving_temp_file() {
        let fixture = Fixture::new();
        let mut settings = AppSettings::empty();
        settings.pi_program = Some(PathBuf::from("/first/pi"));
        save(&fixture.root, &settings).unwrap();
        settings.pi_program = Some(PathBuf::from("/second/pi"));
        save(&fixture.root, &settings).unwrap();
        assert_eq!(load(&fixture.root).unwrap(), settings);
        let mut entries: Vec<_> = fs::read_dir(&fixture.root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        entries.sort();
        assert_eq!(entries, vec![SETTINGS_FILE_NAME]);
    }
}
