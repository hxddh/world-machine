//! Steam: a spike behind the `steam` feature of the desktop app only.
//!
//! With the feature off (every build so far) this is a no-op that says so.
//! With it on, [`start`] starts the Steamworks API when the app is run by
//! Steam: the app id comes from `WORLD_MACHINE_STEAM_APP_ID` (Steam's own
//! `steam_appid.txt` beside the executable works too, for a developer build
//! launched outside Steam), and Steam's callbacks are run on a thread of
//! their own four times a second, which costs nothing measurable in the
//! background. Nothing in the kernel, a System or a Pack knows Steam exists:
//! what Steam offers the game (achievements, Cloud, the overlay) belongs to
//! this app, never to World truth.
//!
//! What is left before a Steam build ships (see docs/RELEASE_SIGNING.md):
//!
//! - **Linking.** The `steamworks-sys` crate carries Valve's redistributable
//!   library (`steam_api64.dll`, `libsteam_api.dylib`), which the app links
//!   against and has to ship beside the executable. The crates are MIT or
//!   Apache-2.0; the library is under Valve's Steamworks SDK Access
//!   Agreement, which a store build accepts.
//! - **The overlay on macOS** injects itself into the process, so a Mac
//!   build signed with the hardened runtime needs the entitlements
//!   `com.apple.security.cs.disable-library-validation` and
//!   `com.apple.security.cs.allow-dyld-environment-variables`, and
//!   `libsteam_api.dylib` in `Contents/MacOS` (or `Frameworks`) signed with
//!   the app. Steam also requires notarized Mac builds, which waits on the
//!   Apple Developer enrolment.
//! - **Cloud** should sync the Worlds folder only, never the settings (which
//!   hold machine paths) or the logs.

/// The environment variable a Steam build reads its app id from.
pub const APP_ID_ENV: &str = "WORLD_MACHINE_STEAM_APP_ID";

/// What became of Steam at startup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Built without the `steam` feature.
    NotBuilt,
    /// Built with it, and Steam is running this app.
    Running {
        app_id: u32,
        language: String,
        overlay: bool,
    },
    /// Built with it, but Steam could not be started (not running, not
    /// owned, no app id): the game plays on without it.
    Unavailable(String),
}

impl Status {
    /// One line for the log.
    pub fn describe(&self) -> String {
        match self {
            Status::NotBuilt => "steam · not built in".to_string(),
            Status::Running {
                app_id,
                language,
                overlay,
            } => format!(
                "steam · app {app_id} · language {language} · overlay {}",
                if *overlay { "on" } else { "off" }
            ),
            Status::Unavailable(why) => format!("steam · unavailable: {why}"),
        }
    }
}

/// The app id named by `value` (the environment variable's), if any: a
/// positive whole number, or an error saying what was wrong with it.
pub fn app_id_from(value: Option<&str>) -> Result<Option<u32>, String> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match value.parse::<u32>() {
        Ok(0) | Err(_) => Err(format!("{APP_ID_ENV} is not a Steam app id: {value:?}")),
        Ok(id) => Ok(Some(id)),
    }
}

/// Starts Steam if this build has it, and says what happened. Never fails
/// the app: a Steam build run without Steam plays on.
pub fn start() -> Status {
    imp::start()
}

#[cfg(not(feature = "steam"))]
mod imp {
    pub(super) fn start() -> super::Status {
        super::Status::NotBuilt
    }
}

#[cfg(feature = "steam")]
mod imp {
    use super::{app_id_from, Status, APP_ID_ENV};
    use std::time::Duration;

    pub(super) fn start() -> Status {
        let app_id = match app_id_from(std::env::var(APP_ID_ENV).ok().as_deref()) {
            Ok(app_id) => app_id,
            Err(why) => return Status::Unavailable(why),
        };
        let client = match app_id {
            Some(app_id) => steamworks::Client::init_app(steamworks::AppId(app_id)),
            // Steam sets the app id itself when it launches the game, and a
            // developer build reads steam_appid.txt.
            None => steamworks::Client::init(),
        };
        let client = match client {
            Ok(client) => client,
            Err(error) => return Status::Unavailable(error.to_string()),
        };
        let status = Status::Running {
            app_id: client.utils().app_id().0,
            language: client.apps().current_game_language(),
            overlay: client.utils().is_overlay_enabled(),
        };
        // The client lives as long as the app; its callbacks are run here.
        let _ = std::thread::Builder::new()
            .name("steam-callbacks".into())
            .spawn(move || loop {
                client.run_callbacks();
                std::thread::sleep(Duration::from_millis(250));
            });
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_app_id_is_a_positive_whole_number_or_nothing() {
        assert_eq!(app_id_from(None), Ok(None));
        assert_eq!(app_id_from(Some("  ")), Ok(None));
        assert_eq!(app_id_from(Some("480")), Ok(Some(480)));
        assert_eq!(app_id_from(Some(" 3141590 ")), Ok(Some(3_141_590)));
        assert!(app_id_from(Some("0")).is_err());
        assert!(app_id_from(Some("-5")).is_err());
        assert!(app_id_from(Some("tiny")).is_err());
    }

    #[cfg(not(feature = "steam"))]
    #[test]
    fn without_the_feature_steam_is_a_no_op() {
        assert_eq!(start(), Status::NotBuilt);
        assert_eq!(start().describe(), "steam · not built in");
    }
}
