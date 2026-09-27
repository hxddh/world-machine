//! How a World's people speak in a language model's words, when the player
//! has switched the World voice on in Settings.
//!
//! A Pack is told how to reach a model in its environment: a local program,
//! or an API key the player gave the app. Neither is ever on unless somebody
//! turned it on, the key is never a process argument, and nothing a model
//! returns is trusted: the conversation System reads it as a proposal and
//! keeps its own hearing whenever the proposal is unusable.

use conversation::{Hearing, Listened};
pub use conversation::{Listener, OwnEars};
use std::io::Write as _;
use std::process::{Command, Stdio};
use world_pi_rpc::{PiCommand, PiRpcTransport, ProcessPiRpcTransport};

/// Told to a Pack whose World should speak in a model's words.
pub const VOICE_ENV: &str = "WORLD_MACHINE_POCKET_UNIVERSE_VOICE";
pub const PI_PROGRAM_ENV: &str = "WORLD_MACHINE_PI_PROGRAM";
pub const API_KEY_ENV: &str = "WORLD_MACHINE_ANTHROPIC_API_KEY";

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const MODEL: &str = "claude-opus-5";
/// An answer is a sentence or two; there is nothing longer worth keeping.
const MAX_TOKENS: u32 = 1024;
const EFFORT: &str = "low";
/// Somebody is waiting for the answer.
const TIMEOUT_SECONDS: u32 = 20;

/// Something that turns a prompt into a model's text, or nothing.
pub trait Completion: Send {
    fn complete(&mut self, prompt: &str) -> Option<String>;
}

/// How to reach a model, as a Pack is told in its environment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Voice {
    None,
    Pi(String),
    Api(String),
}

impl Voice {
    /// What the environment says. Anything unrecognised, or an API voice
    /// with no key, is an error rather than a silent fallback, so whoever
    /// asked for a voice learns why they did not get one.
    pub fn from_env(
        voice: Option<&str>,
        program: Option<&str>,
        key: Option<&str>,
    ) -> Result<Self, String> {
        match voice.unwrap_or("none") {
            "none" => Ok(Voice::None),
            "pi" => Ok(Voice::Pi(program.unwrap_or("pi").to_string())),
            "api" => key
                .map(str::trim)
                .filter(|key| !key.is_empty())
                .map(|key| Voice::Api(key.to_string()))
                .ok_or_else(|| format!("{VOICE_ENV}=api needs a key in {API_KEY_ENV}")),
            other => Err(format!(
                "unsupported {VOICE_ENV} value {other:?}; expected none, pi or api"
            )),
        }
    }

    /// Reads the process's own environment.
    pub fn from_process() -> Result<Self, String> {
        Self::from_env(
            std::env::var(VOICE_ENV).ok().as_deref(),
            std::env::var(PI_PROGRAM_ENV).ok().as_deref(),
            std::env::var(API_KEY_ENV).ok().as_deref(),
        )
    }

    /// A listener for each session: the model's if there is one, and the
    /// conversation System's own ears otherwise.
    pub fn listener_or_own_ears(&self) -> Box<dyn Listener> {
        self.listener().unwrap_or_else(|| Box::new(OwnEars))
    }

    /// A listener that speaks in the model's words, if there is a model.
    pub fn listener(&self) -> Option<Box<dyn Listener>> {
        match self {
            Voice::None => None,
            Voice::Pi(program) => Some(Box::new(ModelListener(PiCompletion(
                ProcessPiRpcTransport::new(PiCommand::decision_only(program.clone())),
            )))),
            Voice::Api(key) => Some(Box::new(ModelListener(ApiCompletion::new(key.clone())))),
        }
    }
}

/// A local program speaking the Pi RPC protocol.
pub struct PiCompletion<T>(pub T);

impl<T: PiRpcTransport + Send> Completion for PiCompletion<T> {
    fn complete(&mut self, prompt: &str) -> Option<String> {
        self.0.complete(prompt).ok()
    }
}

/// The Messages API, reached with the player's key through `curl`: the key
/// goes in on `curl`'s configuration on standard input, never as an
/// argument another process could read.
pub struct ApiCompletion {
    key: String,
}

impl ApiCompletion {
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

/// What a request is made of, apart from making it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiRequest {
    pub body: String,
    pub config: String,
    pub args: Vec<String>,
}

fn escape_config(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The request for one prompt: the key only in the configuration.
pub fn api_request(prompt: &str, key: &str, body_path: &str) -> ApiRequest {
    let body = serde_json::json!({
        "model": MODEL,
        "max_tokens": MAX_TOKENS,
        "output_config": { "effort": EFFORT },
        "messages": [{ "role": "user", "content": prompt }],
    })
    .to_string();
    let config = format!(
        "url = \"{ENDPOINT}\"\nheader = \"x-api-key: {}\"\nheader = \"anthropic-version: {API_VERSION}\"\nheader = \"content-type: application/json\"\ndata-binary = \"@{body_path}\"\n",
        escape_config(key)
    );
    ApiRequest {
        body,
        config,
        args: vec![
            "--config".into(),
            "-".into(),
            "--silent".into(),
            "--show-error".into(),
            "--max-time".into(),
            TIMEOUT_SECONDS.to_string(),
        ],
    }
}

/// The text of a Messages API reply; nothing for an error or a refusal.
pub fn reply_text(response: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(response).ok()?;
    let text = value
        .get("content")?
        .as_array()?
        .iter()
        .filter(|block| block.get("type").and_then(|kind| kind.as_str()) == Some("text"))
        .filter_map(|block| block.get("text").and_then(|text| text.as_str()))
        .collect::<Vec<_>>()
        .join("\n");
    (!text.trim().is_empty()).then_some(text)
}

impl Completion for ApiCompletion {
    fn complete(&mut self, prompt: &str) -> Option<String> {
        let body = BodyFile::write(prompt, &self.key)?;
        let request = api_request(prompt, &self.key, body.path.to_str()?);
        let mut child = Command::new("curl")
            .args(&request.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        child
            .stdin
            .take()?
            .write_all(request.config.as_bytes())
            .ok()?;
        let output = child.wait_with_output().ok()?;
        if !output.status.success() {
            return None;
        }
        reply_text(&String::from_utf8_lossy(&output.stdout))
    }
}

/// The request body on disk, readable only by this user, removed after.
struct BodyFile {
    path: std::path::PathBuf,
}

impl BodyFile {
    fn write(prompt: &str, key: &str) -> Option<Self> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "world-machine-voice-{}-{nonce}.json",
            std::process::id()
        ));
        let request = api_request(prompt, key, path.to_str()?);
        write_private(&path, request.body.as_bytes())?;
        Some(Self { path })
    }
}

impl Drop for BodyFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(unix)]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> Option<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .ok()?;
    file.write_all(bytes).ok()
}

#[cfg(not(unix))]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> Option<()> {
    std::fs::write(path, bytes).ok()
}

/// A listener that asks a model and reads its three lines back.
pub struct ModelListener<C>(pub C);

impl<C: Completion> Listener for ModelListener<C> {
    fn listen(&mut self, hearing: &Hearing) -> Option<Listened> {
        conversation::parse(&self.0.complete(&conversation::prompt(hearing))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_voice_is_only_what_the_environment_asks_for() {
        assert_eq!(Voice::from_env(None, None, None), Ok(Voice::None));
        assert_eq!(
            Voice::from_env(Some("pi"), Some("/usr/local/bin/pi"), None),
            Ok(Voice::Pi("/usr/local/bin/pi".into()))
        );
        assert!(Voice::from_env(Some("api"), None, Some("  ")).is_err());
        assert!(Voice::from_env(Some("shout"), None, None).is_err());
        assert!(Voice::None.listener().is_none());
    }

    #[test]
    fn the_key_is_never_an_argument_and_a_reply_gives_up_only_its_text() {
        let request = api_request("Hello", "sk-\"secret\\", "/tmp/body.json");
        assert!(!request.args.iter().any(|arg| arg.contains("secret")));
        assert!(!request.body.contains("secret"));
        assert!(request.config.contains("x-api-key: sk-\\\"secret\\\\"));
        assert_eq!(
            reply_text(r#"{"content":[{"type":"text","text":"MEANING: greet"}]}"#).as_deref(),
            Some("MEANING: greet")
        );
        assert_eq!(reply_text(r#"{"type":"error","error":{}}"#), None);
    }

    struct Canned(&'static str);

    impl Completion for Canned {
        fn complete(&mut self, _: &str) -> Option<String> {
            Some(self.0.into())
        }
    }

    #[test]
    fn a_model_listener_reads_its_three_lines() {
        let hearing = Hearing {
            name: "Mara".into(),
            settlement: "the harbour".into(),
            traits: vec!["warm".into()],
            facts: Vec::new(),
            people: Vec::new(),
            places: Vec::new(),
            words: "Hi!".into(),
            answer: "Hello.".into(),
        };
        let heard = ModelListener(Canned("MEANING: greet\nABOUT: none\nREPLY: Morning, love!"))
            .listen(&hearing)
            .unwrap();
        assert_eq!(heard.answer, "Morning, love!");
        assert!(ModelListener(Canned("no")).listen(&hearing).is_none());
    }
}
