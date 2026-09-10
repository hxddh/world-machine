//! What this app tells the Worlds it launches about how to speak.
//!
//! A World speaks in its own words only when somebody has both turned the voice
//! on and told the app which local program to write with. Everything about that
//! decision lives here rather than in the main view: which settings are read,
//! what a Pack is told, and the rule that a Pack given nothing behaves exactly
//! as it always has.

use world_machine_desktop::analyst_settings;
use world_pack_process::ProcessPackSource;

/// Told to a Pack that should say what happened in its World's own words.
const VOICE_SETTING: &str = "WORLD_MACHINE_POCKET_UNIVERSE_VOICE";
const PROGRAM_SETTING: &str = "WORLD_MACHINE_PI_PROGRAM";
const VOICE_LOCAL_MODEL: &str = "pi";

/// The settings every Pack this app launches is given.
///
/// Empty unless the voice is on and a program is actually configured: a
/// preference with nothing to act on is not a setting worth passing. Anything
/// unreadable also reads as empty, because how a World phrases itself is never
/// worth failing to open it over.
pub(crate) fn pack_settings() -> Vec<(String, String)> {
    let Ok(root) = analyst_settings::application_support_root() else {
        return Vec::new();
    };
    let Ok(settings) = analyst_settings::load(&root) else {
        return Vec::new();
    };
    let Some(program) = settings.configured_voice_program() else {
        return Vec::new();
    };
    vec![
        (VOICE_SETTING.to_string(), VOICE_LOCAL_MODEL.to_string()),
        (PROGRAM_SETTING.to_string(), program),
    ]
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
