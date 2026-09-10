//! What this app tells the Worlds it launches about how to speak.
//!
//! A World speaks in its own words only when somebody has both turned the voice
//! on and told the app which local program to write with. Everything about that
//! decision lives here rather than in the main view: which settings are read,
//! what a Pack is told, and the rule that a Pack given nothing behaves exactly
//! as it always has.

use world_machine_desktop::analyst_settings::{self, ConfiguredVoice};
use world_machine_desktop::key_store;
use world_pack_process::ProcessPackSource;

/// Told to a Pack that should say what happened in its World's own words.
const VOICE_SETTING: &str = "WORLD_MACHINE_POCKET_UNIVERSE_VOICE";
const PROGRAM_SETTING: &str = "WORLD_MACHINE_PI_PROGRAM";
const KEY_SETTING: &str = "WORLD_MACHINE_ANTHROPIC_API_KEY";
const VOICE_LOCAL_MODEL: &str = "pi";
const VOICE_API: &str = "api";

/// The settings every Pack this app launches is given.
///
/// Empty unless the voice is on and the way it was told to reach a model is
/// actually configured: a preference with nothing behind it is not a setting
/// worth passing. Anything unreadable also reads as empty, because how a World
/// phrases itself is never worth failing to open it over.
pub(crate) fn pack_settings() -> Vec<(String, String)> {
    let Ok(root) = analyst_settings::application_support_root() else {
        return Vec::new();
    };
    let Ok(settings) = analyst_settings::load(&root) else {
        return Vec::new();
    };
    settings_for(settings.configured_voice(key_store::load()))
}

/// What a configured voice tells a Pack. Separated from reading the settings so
/// the mapping can be checked without a keychain or a settings file.
pub(crate) fn settings_for(voice: Option<ConfiguredVoice>) -> Vec<(String, String)> {
    match voice {
        None => Vec::new(),
        Some(ConfiguredVoice::Program(program)) => vec![
            (VOICE_SETTING.to_string(), VOICE_LOCAL_MODEL.to_string()),
            (PROGRAM_SETTING.to_string(), program),
        ],
        Some(ConfiguredVoice::Key(key)) => vec![
            (VOICE_SETTING.to_string(), VOICE_API.to_string()),
            (KEY_SETTING.to_string(), key),
        ],
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
    fn a_voice_with_nothing_behind_it_tells_a_pack_nothing() {
        assert!(settings_for(None).is_empty());
    }

    #[test]
    fn each_way_of_reaching_a_model_is_named_to_the_pack() {
        let program = settings_for(Some(ConfiguredVoice::Program("/usr/local/bin/pi".into())));
        assert_eq!(
            program,
            vec![
                (VOICE_SETTING.to_string(), "pi".to_string()),
                (PROGRAM_SETTING.to_string(), "/usr/local/bin/pi".to_string()),
            ]
        );

        let key = settings_for(Some(ConfiguredVoice::Key("sk-ant-test".into())));
        assert_eq!(
            key,
            vec![
                (VOICE_SETTING.to_string(), "api".to_string()),
                (KEY_SETTING.to_string(), "sk-ant-test".to_string()),
            ]
        );
        assert!(
            !key.iter().any(|(name, _)| name == PROGRAM_SETTING),
            "a key voice was also handed a program to run"
        );
    }
}
