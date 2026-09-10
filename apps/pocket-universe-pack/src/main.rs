use pocket_universe::{
    pocket_universe_descriptor, pocket_universe_registration,
    pocket_universe_registration_with_voice,
};
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use world_host::WorldRegistration;
use world_pack_server::{manifest_for_current_exe, serve_stdio, write_current_exe_bundle};
use world_pi_rpc::{PiCommand, PiRpcRuntime, ProcessPiRpcTransport};

mod api_voice;
mod voice;

const MIND_ENV: &str = "WORLD_MACHINE_POCKET_UNIVERSE_MIND";
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
        env::var(MIND_ENV).ok().as_deref(),
        env::var(VOICE_ENV).ok().as_deref(),
        env::var(API_KEY_ENV).ok().as_deref(),
    )?;
    serve_stdio(selection.registration()?)?;
    Ok(())
}

/// What decides in this World, and what says what happened.
///
/// Two separate choices, because they cost wildly different amounts. The mind
/// runs twice per period — a week-long catch-up is fifty-six requests — and all
/// it ever does is pick one of two offered actions. The voice runs once per
/// return, however long the observer was away, and writes what they read. So
/// the combination worth having is the deterministic mind with a model voice:
/// the simulation stays instant and the prose is this World's own.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Selection {
    mind: Mind,
    voice: Voice,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mind {
    Deterministic,
    Pi,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Voice {
    None,
    /// A local program the observer already has.
    Pi,
    /// A key the observer gave the app. The only voice whose requests leave
    /// the machine, so it exists only when a key was actually handed over.
    Api(String),
}

impl Selection {
    /// Refuse anything unrecognised rather than guessing what was meant.
    fn from_env(
        mind: Option<&str>,
        voice: Option<&str>,
        key: Option<&str>,
    ) -> Result<Self, String> {
        let mind = match mind.unwrap_or("deterministic") {
            "deterministic" => Mind::Deterministic,
            "pi" => Mind::Pi,
            other => {
                return Err(format!(
                    "unsupported {MIND_ENV} value {other:?}; expected deterministic or pi"
                ))
            }
        };
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
            other => {
                return Err(format!(
                    "unsupported {VOICE_ENV} value {other:?}; expected none, pi or api"
                ))
            }
        };
        Ok(Self { mind, voice })
    }

    fn registration(self) -> Result<WorldRegistration, Box<dyn Error>> {
        let voice: Option<pocket_universe::NarratorFactory> = match self.voice {
            Voice::None => None,
            Voice::Pi => Some(narrator_factory(pi_command())),
            Voice::Api(key) => Some(Arc::new(move || {
                Box::new(api_voice::ApiNarrator::new(key.clone()))
            })),
        };
        Ok(match (self.mind, voice) {
            (Mind::Deterministic, None) => pocket_universe_registration(),
            (Mind::Deterministic, Some(voice)) => pocket_universe_registration_with_voice(
                || pocket_universe::PocketMind,
                "deterministic",
                voice,
            )?,
            (Mind::Pi, voice) => {
                let command = pi_command();
                pocket_universe_registration_with_voice(
                    move || PiRpcRuntime::new(ProcessPiRpcTransport::new(command.clone())),
                    "pi",
                    voice.unwrap_or_else(|| {
                        Arc::new(|| Box::new(pocket_universe::narrator::NoNarrator))
                    }),
                )?
            }
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
    fn what_decides_and_what_narrates_are_chosen_separately() {
        // The combination worth having: the simulation stays instant and the
        // prose is the World's own. It has to be reachable without paying for a
        // model mind, which runs twice a period against the voice's once a
        // return.
        let voiced = Selection::from_env(None, Some("pi"), None).unwrap();
        assert_eq!(voiced.mind, Mind::Deterministic);
        assert_eq!(voiced.voice, Voice::Pi);

        for (mind, voice, expected) in [
            (None, None, (Mind::Deterministic, Voice::None)),
            (Some("pi"), None, (Mind::Pi, Voice::None)),
            (Some("pi"), Some("pi"), (Mind::Pi, Voice::Pi)),
        ] {
            let selection = Selection::from_env(mind, voice, None).unwrap();
            assert_eq!(
                (selection.mind, selection.voice),
                expected,
                "{mind:?} {voice:?}"
            );
        }
    }

    #[test]
    fn a_key_is_what_makes_the_networked_voice_exist() {
        let with_key = Selection::from_env(None, Some("api"), Some("sk-ant-test")).unwrap();
        assert_eq!(with_key.voice, Voice::Api("sk-ant-test".into()));
        assert_eq!(
            with_key.mind,
            Mind::Deterministic,
            "a key must not drag the expensive mind along with it"
        );
        // Surrounding space is somebody pasting, not somebody meaning it.
        assert_eq!(
            Selection::from_env(None, Some("api"), Some("  sk-ant-test  "))
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
            let error = Selection::from_env(None, Some("api"), key).unwrap_err();
            assert!(error.contains(API_KEY_ENV), "{error}");
        }
    }

    #[test]
    fn a_world_with_nothing_configured_is_the_one_that_ships() {
        let shipped = Selection::from_env(None, None, None).unwrap();
        assert_eq!(shipped.mind, Mind::Deterministic);
        assert_eq!(shipped.voice, Voice::None);
        // And it builds a Pack without needing any program to exist.
        assert!(shipped.registration().is_ok());
    }

    #[test]
    fn an_unknown_setting_is_refused_rather_than_guessed() {
        let mind = Selection::from_env(Some("gpt"), None, None).unwrap_err();
        assert!(mind.contains(MIND_ENV), "{mind}");
        let voice = Selection::from_env(None, Some("yes"), None).unwrap_err();
        assert!(voice.contains(VOICE_ENV), "{voice}");
    }
}
