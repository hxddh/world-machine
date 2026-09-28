//! What this app tells the Worlds it launches about how to speak.
//!
//! A World speaks in its own words only when somebody has both turned the voice
//! on and told the app how to reach a model: a local program, an API key, or
//! the model built into macOS (`/usr/bin/fm`). Everything about that
//! decision lives here rather than in the main view: which settings are read,
//! what a Pack is told, and the rule that a Pack given nothing behaves exactly
//! as it always has.

use world_machine_desktop::app_settings::{self, ConfiguredVoice};
use world_machine_desktop::key_store;
use world_pack_process::ProcessPackSource;

/// Told to a Pack that should say what happened in its World's own words.
const VOICE_SETTING: &str = "WORLD_MACHINE_POCKET_UNIVERSE_VOICE";
const PROGRAM_SETTING: &str = "WORLD_MACHINE_PI_PROGRAM";
const KEY_SETTING: &str = "WORLD_MACHINE_ANTHROPIC_API_KEY";
const MODEL_SETTING: &str = ::world_voice::MODEL_ENV;
const VOICE_LOCAL_MODEL: &str = "pi";
const VOICE_API: &str = "api";
const VOICE_ON_DEVICE: &str = "fm";

/// The settings every Pack this app launches is given.
///
/// Empty unless the voice is on and the way it was told to reach a model is
/// actually configured: a preference with nothing behind it is not a setting
/// worth passing. Anything unreadable also reads as empty, because how a World
/// phrases itself is never worth failing to open it over.
pub(crate) fn pack_settings() -> Vec<(String, String)> {
    let Ok(root) = app_settings::application_support_root() else {
        return Vec::new();
    };
    let Ok(settings) = app_settings::load(&root) else {
        return Vec::new();
    };
    let model = ::world_voice::model_or(settings.voice_model.as_deref());
    settings_for(settings.configured_voice(key_store::load()), &model)
}

/// What a configured voice tells a Pack, and which Claude model a key asks.
/// Separated from reading the settings so the mapping can be checked without
/// a keychain or a settings file.
///
/// This Mac's own model needs nothing but its name: a Pack runs `fm` itself
/// and keeps its own words whenever it gives no answer.
pub(crate) fn settings_for(voice: Option<ConfiguredVoice>, model: &str) -> Vec<(String, String)> {
    match voice {
        None => Vec::new(),
        Some(ConfiguredVoice::OnDevice) => {
            vec![(VOICE_SETTING.to_string(), VOICE_ON_DEVICE.to_string())]
        }
        Some(ConfiguredVoice::Program(program)) => vec![
            (VOICE_SETTING.to_string(), VOICE_LOCAL_MODEL.to_string()),
            (PROGRAM_SETTING.to_string(), program),
        ],
        Some(ConfiguredVoice::Key(key)) => vec![
            (VOICE_SETTING.to_string(), VOICE_API.to_string()),
            (KEY_SETTING.to_string(), key),
            (MODEL_SETTING.to_string(), model.to_string()),
        ],
    }
}

/// Whether the World voice is switched on at all, read without touching
/// the keychain, so a window can tell cheaply whether to ask a model.
pub(crate) fn voice_on() -> bool {
    app_settings::application_support_root()
        .ok()
        .and_then(|root| app_settings::load(&root).ok())
        .is_some_and(|settings| settings.world_voice)
}

/// Asks the configured model `prompt`, the way a Pack would: its response,
/// or nothing if no model is configured or it had nothing to say. Reads
/// the key, so it belongs off the window's thread.
pub(crate) fn ask_model(prompt: &str) -> Option<String> {
    let root = app_settings::application_support_root().ok()?;
    let settings = app_settings::load(&root).ok()?;
    let voice = settings.configured_voice(key_store::load());
    // This Mac's own model is asked only once it has been found working; its
    // probe is run here, off the window's thread, at most once a run.
    if voice == Some(ConfiguredVoice::OnDevice) && !::world_voice::fm::status().is_ready() {
        return None;
    }
    let mut completion =
        model_for(voice)?.completion_with_model(settings.voice_model.as_deref())?;
    completion.complete(prompt)
}

/// The model a configured voice reaches.
pub(crate) fn model_for(voice: Option<ConfiguredVoice>) -> Option<::world_voice::Voice> {
    match voice? {
        ConfiguredVoice::Program(program) => Some(::world_voice::Voice::Pi(program)),
        ConfiguredVoice::Key(key) => Some(::world_voice::Voice::Api(key)),
        ConfiguredVoice::OnDevice => Some(::world_voice::Voice::Fm(
            ::world_voice::fm::PROGRAM.to_string(),
        )),
    }
}

/// Hand every Pack in this source the same settings.
///
/// A setting the Pack layer refuses is a mistake in this app rather than
/// anything the observer did, so the Worlds are installed without it instead of
/// leaving somebody unable to open anything.
pub(crate) fn with_settings(
    source: ProcessPackSource,
    settings: Vec<(String, String)>,
) -> ProcessPackSource {
    if settings.is_empty() {
        return source;
    }
    let packs = source
        .packs()
        .iter()
        .cloned()
        .map(|pack| pack.with_settings(settings.clone()))
        .collect::<Result<Vec<_>, _>>();
    match packs {
        Ok(packs) => ProcessPackSource::from_packs(packs),
        Err(_) => source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_asks_the_same_model_its_worlds_are_told_about() {
        assert_eq!(model_for(None), None);
        assert_eq!(
            model_for(Some(ConfiguredVoice::Program("/usr/local/bin/pi".into()))),
            Some(::world_voice::Voice::Pi("/usr/local/bin/pi".into()))
        );
        assert_eq!(
            model_for(Some(ConfiguredVoice::Key("sk-ant-test".into()))),
            Some(::world_voice::Voice::Api("sk-ant-test".into()))
        );
        assert_eq!(
            model_for(Some(ConfiguredVoice::OnDevice)),
            Some(::world_voice::Voice::Fm("/usr/bin/fm".into()))
        );
    }

    #[test]
    fn this_macs_own_model_is_named_to_a_pack_with_nothing_else() {
        assert_eq!(
            settings_for(Some(ConfiguredVoice::OnDevice), "claude-sonnet-5"),
            vec![(VOICE_SETTING.to_string(), "fm".to_string())]
        );
    }

    #[test]
    fn a_voice_with_nothing_behind_it_tells_a_pack_nothing() {
        assert!(settings_for(None, "claude-sonnet-5").is_empty());
    }

    #[test]
    fn each_way_of_reaching_a_model_is_named_to_the_pack() {
        let program = settings_for(
            Some(ConfiguredVoice::Program("/usr/local/bin/pi".into())),
            "claude-sonnet-5",
        );
        assert_eq!(
            program,
            vec![
                (VOICE_SETTING.to_string(), "pi".to_string()),
                (PROGRAM_SETTING.to_string(), "/usr/local/bin/pi".to_string()),
            ]
        );

        let key = settings_for(
            Some(ConfiguredVoice::Key("sk-ant-test".into())),
            "claude-opus-5-5",
        );
        assert_eq!(
            key,
            vec![
                (VOICE_SETTING.to_string(), "api".to_string()),
                (KEY_SETTING.to_string(), "sk-ant-test".to_string()),
                (
                    "WORLD_MACHINE_VOICE_MODEL".to_string(),
                    "claude-opus-5-5".to_string()
                ),
            ]
        );
        assert!(
            !key.iter().any(|(name, _)| name == PROGRAM_SETTING),
            "a key voice was also handed a program to run"
        );
    }
}
