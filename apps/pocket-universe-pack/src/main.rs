use pocket_universe::{
    pocket_universe_descriptor, pocket_universe_registration,
    pocket_universe_registration_with_voices,
};
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use world_host::WorldRegistration;
use world_pack_server::{manifest_for_current_exe, serve_stdio, write_current_exe_bundle};
use world_pi_rpc::{PiCommand, ProcessPiRpcTransport};

mod api_voice;
mod fm_voice;
mod voice;

const VOICE_ENV: &str = "WORLD_MACHINE_POCKET_UNIVERSE_VOICE";
const PI_PROGRAM_ENV: &str = "WORLD_MACHINE_PI_PROGRAM";
const API_KEY_ENV: &str = "WORLD_MACHINE_ANTHROPIC_API_KEY";

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    let descriptor = pocket_universe_descriptor();
    if args.len() == 1 && args[0] == "--print-manifest" {
        let manifest = manifest_for_current_exe(&descriptor)?;
        println!("{}", manifest.to_json_pretty()?);
        return Ok(());
    }
    if args.len() == 2 && args[0] == "--write-bundle" {
        let destination = PathBuf::from(&args[1]);
        write_current_exe_bundle(&descriptor, destination)?;
        return Ok(());
    }
    if !args.is_empty() {
        return Err("unsupported arguments; run without arguments as a Pack server, use --print-manifest, or use --write-bundle PATH"
            .to_string()
            .into());
    }

    let selection = Selection::from_env(
        env::var(VOICE_ENV).ok().as_deref(),
        env::var(API_KEY_ENV).ok().as_deref(),
    )?;
    serve_stdio(selection.registration()?)?;
    Ok(())
}

/// What says what happened in this World, and speaks for its people. It
/// runs once per return, however long the observer was away.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Selection {
    voice: Voice,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Voice {
    None,
    /// A local program the observer already has.
    Pi,
    /// A key the observer gave the app. The only voice whose requests leave
    /// the machine, so it exists only when a key was actually handed over.
    Api(String),
    /// The model built into macOS 27, through its `fm` program: offline,
    /// and chosen only once the app found it working.
    Fm,
}

impl Selection {
    /// Refuse anything unrecognised rather than guessing what was meant.
    fn from_env(voice: Option<&str>, key: Option<&str>) -> Result<Self, String> {
        let voice = match voice.unwrap_or("none") {
            "none" => Voice::None,
            "pi" => Voice::Pi,
            // A key that is missing or blank is not a quiet fallback to no
            // voice: somebody asked for this and would otherwise never learn
            // why their Worlds stayed silent.
            "api" => match key.map(str::trim).filter(|key| !key.is_empty()) {
                Some(key) => Voice::Api(key.to_owned()),
                None => return Err(format!("{VOICE_ENV}=api needs a key in {API_KEY_ENV}")),
            },
            "fm" => Voice::Fm,
            other => {
                return Err(format!(
                    "unsupported {VOICE_ENV} value {other:?}; expected none, pi, api or fm"
                ))
            }
        };
        Ok(Self { voice })
    }

    fn registration(self) -> Result<WorldRegistration, Box<dyn Error>> {
        // Whoever narrates also speaks for the people when the player talks
        // to them; nothing the model says is more than a proposal.
        let speaking = match &self.voice {
            Voice::None => world_voice::Voice::None,
            Voice::Pi => {
                world_voice::Voice::Pi(env::var(PI_PROGRAM_ENV).unwrap_or_else(|_| "pi".into()))
            }
            Voice::Api(key) => world_voice::Voice::Api(key.clone()),
            Voice::Fm => world_voice::Voice::Fm(world_voice::fm::PROGRAM.into()),
        };
        let ears: pocket_universe::ListenerFactory =
            Arc::new(move || speaking.listener_or_own_ears());
        let voice: Option<pocket_universe::NarratorFactory> = match self.voice {
            Voice::None => None,
            Voice::Pi => Some(narrator_factory(pi_command())),
            Voice::Api(key) => Some(Arc::new(move || {
                Box::new(api_voice::ApiNarrator::new(key.clone()))
            })),
            Voice::Fm => Some(Arc::new(|| {
                Box::new(fm_voice::FmNarrator::new(world_voice::fm::PROGRAM))
            })),
        };
        Ok(match voice {
            None => pocket_universe_registration(),
            Some(voice) => pocket_universe_registration_with_voices(voice, ears),
        })
    }
}

/// The locked-down local model process: no tools, no extensions, no session.
fn pi_command() -> PiCommand {
    PiCommand::decision_only(env::var(PI_PROGRAM_ENV).unwrap_or_else(|_| "pi".into()))
}

fn narrator_factory(command: PiCommand) -> pocket_universe::NarratorFactory {
    Arc::new(move || {
        Box::new(voice::PiNarrator::new(ProcessPiRpcTransport::new(
            command.clone(),
        )))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_voice_can_be_chosen() {
        for (voice, expected) in [
            (None, Voice::None),
            (Some("pi"), Voice::Pi),
            (Some("fm"), Voice::Fm),
        ] {
            let selection = Selection::from_env(voice, None).unwrap();
            assert_eq!(selection.voice, expected, "{voice:?}");
        }
    }

    #[test]
    fn a_key_is_what_makes_the_networked_voice_exist() {
        let with_key = Selection::from_env(Some("api"), Some("sk-ant-test")).unwrap();
        assert_eq!(with_key.voice, Voice::Api("sk-ant-test".into()));
        // Surrounding space is somebody pasting, not somebody meaning it.
        assert_eq!(
            Selection::from_env(Some("api"), Some("  sk-ant-test  "))
                .unwrap()
                .voice,
            Voice::Api("sk-ant-test".into())
        );
    }

    #[test]
    fn asking_for_a_networked_voice_without_a_key_says_so() {
        // Falling back to silence would leave somebody who asked for this
        // wondering why their Worlds never changed.
        for key in [None, Some(""), Some("   ")] {
            let error = Selection::from_env(Some("api"), key).unwrap_err();
            assert!(error.contains(API_KEY_ENV), "{error}");
        }
    }

    #[test]
    fn a_world_with_nothing_configured_is_the_one_that_ships() {
        let shipped = Selection::from_env(None, None).unwrap();
        assert_eq!(shipped.voice, Voice::None);
        // And it builds a Pack without needing any program to exist.
        assert!(shipped.registration().is_ok());
        // This Mac's own model builds one too, with no program checked yet.
        assert!(Selection::from_env(Some("fm"), None)
            .unwrap()
            .registration()
            .is_ok());
    }

    #[test]
    fn an_unknown_setting_is_refused_rather_than_guessed() {
        let voice = Selection::from_env(Some("yes"), None).unwrap_err();
        assert!(voice.contains(VOICE_ENV), "{voice}");
    }
}
