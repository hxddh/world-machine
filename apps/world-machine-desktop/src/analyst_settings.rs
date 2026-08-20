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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopAnalystSettings {
    pub version: u32,
    pub node_program: Option<PathBuf>,
    pub pi_program: Option<PathBuf>,
}

impl DesktopAnalystSettings {
    pub fn empty() -> Self {
        Self {
            version: SETTINGS_VERSION,
            node_program: None,
            pi_program: None,
        }
    }

    pub fn validate(&self) -> Result<(), DesktopAnalystSettingsError> {
        if self.version != SETTINGS_VERSION {
            return Err(DesktopAnalystSettingsError::UnsupportedVersion(
                self.version,
            ));
        }
        for (field, path) in [
            ("Node", self.node_program.as_deref()),
            ("Pi", self.pi_program.as_deref()),
        ] {
            if let Some(path) = path {
                if !path.is_absolute() {
                    return Err(DesktopAnalystSettingsError::InvalidPath {
                        field,
                        path: path.to_path_buf(),
                    });
                }
            }
        }
        Ok(())
    }
}

impl Default for DesktopAnalystSettings {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopAnalystProgramSource {
    Environment,
    Persisted,
    Default,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopAnalystProgramSelection {
    pub program: PathBuf,
    pub source: DesktopAnalystProgramSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopAnalystProgramSelections {
    pub node: DesktopAnalystProgramSelection,
    pub pi: DesktopAnalystProgramSelection,
}

#[derive(Debug)]
pub enum DesktopAnalystSettingsError {
    Io(String),
    Malformed(String),
    UnsupportedVersion(u32),
    InvalidPath { field: &'static str, path: PathBuf },
}

impl fmt::Display for DesktopAnalystSettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => f.write_str(message),
            Self::Malformed(message) => {
                write!(f, "World Analyst settings are malformed: {message}")
            }
            Self::UnsupportedVersion(version) => write!(
                f,
                "World Analyst settings use unsupported format version {version}"
            ),
            Self::InvalidPath { field, path } => write!(
                f,
                "World Analyst {field} path must be absolute: {}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for DesktopAnalystSettingsError {}

pub fn application_support_root() -> Result<PathBuf, DesktopAnalystSettingsError> {
    let home = env::var_os("HOME").ok_or_else(|| {
        DesktopAnalystSettingsError::Io(
            "World Analyst could not locate the user's home directory".to_string(),
        )
    })?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("World Machine"))
}

pub fn settings_path(root: &Path) -> PathBuf {
    root.join(SETTINGS_FILE_NAME)
}

pub fn load(root: &Path) -> Result<DesktopAnalystSettings, DesktopAnalystSettingsError> {
    let path = settings_path(root);
    if !path.exists() {
        return Ok(DesktopAnalystSettings::empty());
    }
    let mut file = File::open(&path).map_err(|error| {
        DesktopAnalystSettingsError::Io(format!(
            "World Analyst could not read settings at {}: {error}",
            path.display()
        ))
    })?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|error| {
        DesktopAnalystSettingsError::Io(format!(
            "World Analyst could not read settings at {}: {error}",
            path.display()
        ))
    })?;
    let settings: DesktopAnalystSettings = serde_json::from_str(&contents)
        .map_err(|error| DesktopAnalystSettingsError::Malformed(error.to_string()))?;
    settings.validate()?;
    Ok(settings)
}

pub fn save(
    root: &Path,
    settings: &DesktopAnalystSettings,
) -> Result<(), DesktopAnalystSettingsError> {
    settings.validate()?;
    fs::create_dir_all(root).map_err(|error| {
        DesktopAnalystSettingsError::Io(format!(
            "World Analyst could not create settings directory {}: {error}",
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
        .map_err(|error| DesktopAnalystSettingsError::Malformed(error.to_string()))?;

    let write_result = write_temporary_settings(&temp, &payload);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }

    fs::rename(&temp, &target).map_err(|error| {
        let _ = fs::remove_file(&temp);
        DesktopAnalystSettingsError::Io(format!(
            "World Analyst could not replace settings at {}: {error}",
            target.display()
        ))
    })?;
    Ok(())
}

pub fn save_node_program(root: &Path, path: PathBuf) -> Result<(), DesktopAnalystSettingsError> {
    update_settings(root, move |settings| settings.node_program = Some(path))
}

pub fn save_pi_program(root: &Path, path: PathBuf) -> Result<(), DesktopAnalystSettingsError> {
    update_settings(root, move |settings| settings.pi_program = Some(path))
}

pub fn clear_node_program(root: &Path) -> Result<(), DesktopAnalystSettingsError> {
    update_settings(root, |settings| settings.node_program = None)
}

pub fn clear_pi_program(root: &Path) -> Result<(), DesktopAnalystSettingsError> {
    update_settings(root, |settings| settings.pi_program = None)
}

pub fn clear_programs(root: &Path) -> Result<(), DesktopAnalystSettingsError> {
    update_settings(root, |settings| {
        settings.node_program = None;
        settings.pi_program = None;
    })
}

fn update_settings(
    root: &Path,
    update: impl FnOnce(&mut DesktopAnalystSettings),
) -> Result<(), DesktopAnalystSettingsError> {
    let _guard = SETTINGS_UPDATE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut settings = load(root)?;
    update(&mut settings);
    save(root, &settings)
}

fn write_temporary_settings(
    temp: &Path,
    payload: &[u8],
) -> Result<(), DesktopAnalystSettingsError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(temp)
        .map_err(|error| {
            DesktopAnalystSettingsError::Io(format!(
                "World Analyst could not write temporary settings {}: {error}",
                temp.display()
            ))
        })?;
    file.write_all(payload).map_err(|error| {
        DesktopAnalystSettingsError::Io(format!(
            "World Analyst could not write temporary settings {}: {error}",
            temp.display()
        ))
    })?;
    file.write_all(b"\n").map_err(|error| {
        DesktopAnalystSettingsError::Io(format!(
            "World Analyst could not finish temporary settings {}: {error}",
            temp.display()
        ))
    })?;
    file.sync_all().map_err(|error| {
        DesktopAnalystSettingsError::Io(format!(
            "World Analyst could not sync temporary settings {}: {error}",
            temp.display()
        ))
    })?;
    Ok(())
}

pub fn selections(
    settings: &DesktopAnalystSettings,
    node_environment: Option<PathBuf>,
    pi_environment: Option<PathBuf>,
) -> DesktopAnalystProgramSelections {
    DesktopAnalystProgramSelections {
        node: select_program(
            node_environment,
            settings.node_program.clone(),
            PathBuf::from("node"),
        ),
        pi: select_program(
            pi_environment,
            settings.pi_program.clone(),
            PathBuf::from("pi"),
        ),
    }
}

fn select_program(
    environment: Option<PathBuf>,
    persisted: Option<PathBuf>,
    fallback: PathBuf,
) -> DesktopAnalystProgramSelection {
    if let Some(program) = environment {
        DesktopAnalystProgramSelection {
            program,
            source: DesktopAnalystProgramSource::Environment,
        }
    } else if let Some(program) = persisted {
        DesktopAnalystProgramSelection {
            program,
            source: DesktopAnalystProgramSource::Persisted,
        }
    } else {
        DesktopAnalystProgramSelection {
            program: fallback,
            source: DesktopAnalystProgramSource::Default,
        }
    }
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
                    "world-machine-m226-settings-{}-{nonce}-{sequence}",
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
        assert_eq!(
            load(&fixture.root).unwrap(),
            DesktopAnalystSettings::empty()
        );
    }

    #[test]
    fn save_load_round_trip() {
        let fixture = Fixture::new();
        let settings = DesktopAnalystSettings {
            version: SETTINGS_VERSION,
            node_program: Some(PathBuf::from("/opt/homebrew/bin/node")),
            pi_program: Some(PathBuf::from("/usr/local/bin/pi")),
        };
        save(&fixture.root, &settings).unwrap();
        assert_eq!(load(&fixture.root).unwrap(), settings);
    }

    #[test]
    fn relative_paths_are_rejected() {
        let mut settings = DesktopAnalystSettings::empty();
        settings.node_program = Some(PathBuf::from("bin/node"));
        assert!(matches!(
            settings.validate(),
            Err(DesktopAnalystSettingsError::InvalidPath { field: "Node", .. })
        ));
    }

    #[test]
    fn malformed_json_is_not_overwritten() {
        let fixture = Fixture::new();
        fs::create_dir_all(&fixture.root).unwrap();
        let path = settings_path(&fixture.root);
        fs::write(&path, "{not-json").unwrap();
        assert!(matches!(
            save_node_program(&fixture.root, PathBuf::from("/new/node")),
            Err(DesktopAnalystSettingsError::Malformed(_))
        ));
        assert_eq!(fs::read_to_string(path).unwrap(), "{not-json");
    }

    #[test]
    fn unsupported_version_is_rejected_without_data_loss() {
        let fixture = Fixture::new();
        fs::create_dir_all(&fixture.root).unwrap();
        let path = settings_path(&fixture.root);
        let contents = r#"{"version":2,"node_program":null,"pi_program":null}"#;
        fs::write(&path, contents).unwrap();
        assert!(matches!(
            clear_programs(&fixture.root),
            Err(DesktopAnalystSettingsError::UnsupportedVersion(2))
        ));
        assert_eq!(fs::read_to_string(path).unwrap(), contents);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let fixture = Fixture::new();
        fs::create_dir_all(&fixture.root).unwrap();
        fs::write(
            settings_path(&fixture.root),
            r#"{"version":1,"node_program":null,"pi_program":null,"provider":"x"}"#,
        )
        .unwrap();
        assert!(matches!(
            load(&fixture.root),
            Err(DesktopAnalystSettingsError::Malformed(_))
        ));
    }

    #[test]
    fn environment_beats_persisted_beats_default() {
        let settings = DesktopAnalystSettings {
            version: SETTINGS_VERSION,
            node_program: Some(PathBuf::from("/persisted/node")),
            pi_program: Some(PathBuf::from("/persisted/pi")),
        };
        let selected = selections(&settings, Some(PathBuf::from("/env/node")), None);
        assert_eq!(selected.node.program, PathBuf::from("/env/node"));
        assert_eq!(
            selected.node.source,
            DesktopAnalystProgramSource::Environment
        );
        assert_eq!(selected.pi.program, PathBuf::from("/persisted/pi"));
        assert_eq!(selected.pi.source, DesktopAnalystProgramSource::Persisted);

        let defaults = selections(&DesktopAnalystSettings::empty(), None, None);
        assert_eq!(defaults.node.program, PathBuf::from("node"));
        assert_eq!(defaults.node.source, DesktopAnalystProgramSource::Default);
        assert_eq!(defaults.pi.program, PathBuf::from("pi"));
        assert_eq!(defaults.pi.source, DesktopAnalystProgramSource::Default);
    }

    #[test]
    fn field_updates_preserve_each_other_and_clear_individually_or_together() {
        let fixture = Fixture::new();
        save_node_program(&fixture.root, PathBuf::from("/saved/node")).unwrap();
        save_pi_program(&fixture.root, PathBuf::from("/saved/pi")).unwrap();
        assert_eq!(
            load(&fixture.root).unwrap(),
            DesktopAnalystSettings {
                version: SETTINGS_VERSION,
                node_program: Some(PathBuf::from("/saved/node")),
                pi_program: Some(PathBuf::from("/saved/pi")),
            }
        );

        clear_node_program(&fixture.root).unwrap();
        let after_node_clear = load(&fixture.root).unwrap();
        assert_eq!(after_node_clear.node_program, None);
        assert_eq!(
            after_node_clear.pi_program,
            Some(PathBuf::from("/saved/pi"))
        );

        clear_programs(&fixture.root).unwrap();
        assert_eq!(
            load(&fixture.root).unwrap(),
            DesktopAnalystSettings::empty()
        );
    }

    #[test]
    fn concurrent_node_and_pi_updates_do_not_drop_a_field() {
        let fixture = Fixture::new();
        let root = Arc::new(fixture.root.clone());
        let barrier = Arc::new(Barrier::new(3));

        let node_root = Arc::clone(&root);
        let node_barrier = Arc::clone(&barrier);
        let node = thread::spawn(move || {
            node_barrier.wait();
            save_node_program(node_root.as_ref(), PathBuf::from("/concurrent/node")).unwrap();
        });

        let pi_root = Arc::clone(&root);
        let pi_barrier = Arc::clone(&barrier);
        let pi = thread::spawn(move || {
            pi_barrier.wait();
            save_pi_program(pi_root.as_ref(), PathBuf::from("/concurrent/pi")).unwrap();
        });

        barrier.wait();
        node.join().unwrap();
        pi.join().unwrap();

        let settings = load(root.as_ref()).unwrap();
        assert_eq!(
            settings.node_program,
            Some(PathBuf::from("/concurrent/node"))
        );
        assert_eq!(settings.pi_program, Some(PathBuf::from("/concurrent/pi")));
    }

    #[test]
    fn save_replaces_target_without_leaving_temp_file() {
        let fixture = Fixture::new();
        let mut settings = DesktopAnalystSettings::empty();
        settings.node_program = Some(PathBuf::from("/first/node"));
        save(&fixture.root, &settings).unwrap();
        settings.node_program = Some(PathBuf::from("/second/node"));
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
