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

pub mod fm;
pub mod helper;
pub use fm::{FmCompletion, FmStatus};

/// Told to a Pack whose World should speak in a model's words.
pub const VOICE_ENV: &str = "WORLD_MACHINE_POCKET_UNIVERSE_VOICE";
pub const PI_PROGRAM_ENV: &str = "WORLD_MACHINE_PI_PROGRAM";
pub const API_KEY_ENV: &str = "WORLD_MACHINE_ANTHROPIC_API_KEY";
/// `curl`, run by its full path, so no program of that name earlier on
/// `PATH` is handed a key.
pub const CURL: &str = "/usr/bin/curl";
/// The longest an API key is believed to be.
pub const MOST_API_KEY: usize = 256;

/// Whether `key` could be an API key: one to [`MOST_API_KEY`] letters,
/// digits, `-` and `_`, and nothing else. Anything else (a space, a quote,
/// a line break that could end a line of `curl`'s configuration and start
/// another) is refused before it is stored or used.
pub fn is_plausible_api_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MOST_API_KEY
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}
/// Which model answers, if not the default.
pub const MODEL_ENV: &str = "WORLD_MACHINE_VOICE_MODEL";

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
/// A line or two in someone's own voice needs a fast, inexpensive model
/// with no extended thinking.
pub const DEFAULT_MODEL: &str = "claude-sonnet-5";
/// An answer is a sentence or two; there is nothing longer worth keeping.
const MAX_TOKENS: u32 = 300;
/// Somebody is waiting for the answer.
const TIMEOUT_SECONDS: u32 = 20;
/// The judge a key asks, unless [`JUDGE_MODEL_ENV`] names another: small,
/// fast and inexpensive, for a verdict of a few words.
pub const DEFAULT_JUDGE_MODEL: &str = "claude-haiku-4-5";
/// Which model judges, if not the default.
pub const JUDGE_MODEL_ENV: &str = "WORLD_MACHINE_JUDGE_MODEL";
/// A verdict is one small object.
const JUDGE_MAX_TOKENS: u32 = 64;
/// Somebody is waiting for the answer, and the model has already taken
/// its time: a judge that is slower than this is no judge, and the strict
/// guard decides.
const JUDGE_TIMEOUT_SECONDS: u32 = 6;

/// Something that turns a prompt into a model's text, or nothing.
pub trait Completion: Send {
    fn complete(&mut self, prompt: &str) -> Option<String>;

    /// The same, held to `schema` (a JSON Schema for one object) where the
    /// model can be held to one; a model that cannot is asked as it is and
    /// its answer is read the same way.
    fn complete_with(
        &mut self,
        prompt: &str,
        schema: Option<&serde_json::Value>,
    ) -> Option<String> {
        let _ = schema;
        self.complete(prompt)
    }
}

impl Completion for Box<dyn Completion> {
    fn complete(&mut self, prompt: &str) -> Option<String> {
        self.as_mut().complete(prompt)
    }

    fn complete_with(
        &mut self,
        prompt: &str,
        schema: Option<&serde_json::Value>,
    ) -> Option<String> {
        self.as_mut().complete_with(prompt, schema)
    }
}

/// How to reach a model, as a Pack is told in its environment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Voice {
    None,
    Pi(String),
    Api(String),
    /// The model built into macOS 27, through its `fm` program.
    Fm(String),
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
            "fm" => Ok(Voice::Fm(fm::PROGRAM.to_string())),
            "api" => key
                .map(str::trim)
                .filter(|key| is_plausible_api_key(key))
                .map(|key| Voice::Api(key.to_string()))
                .ok_or_else(|| format!("{VOICE_ENV}=api needs a key in {API_KEY_ENV}")),
            other => Err(format!(
                "unsupported {VOICE_ENV} value {other:?}; expected none, pi, api or fm"
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

    /// The model itself, to ask a prompt of, if there is one.
    pub fn completion(&self) -> Option<Box<dyn Completion>> {
        match self {
            Voice::None => None,
            Voice::Pi(program) => Some(Box::new(PiCompletion(ProcessPiRpcTransport::new(
                PiCommand::decision_only(program.clone()),
            )))),
            Voice::Api(key) => Some(Box::new(ApiCompletion::new(key.clone()))),
            Voice::Fm(program) => Some(Box::new(FmCompletion::new(program.clone()))),
        }
    }

    /// The same, with the Claude model named in Settings (the environment's
    /// [`MODEL_ENV`] still wins); only an API voice has a model to name.
    pub fn completion_with_model(&self, chosen: Option<&str>) -> Option<Box<dyn Completion>> {
        match self {
            Voice::Api(key) => Some(Box::new(
                ApiCompletion::new(key.clone()).with_model(model_or(chosen)),
            )),
            other => other.completion(),
        }
    }

    /// A judge asking the same model a second question, if there is a
    /// model: with a key, the judge model rather than the voice's.
    pub fn judge(&self) -> Option<ModelJudge> {
        match self {
            Voice::None => None,
            Voice::Pi(program) => Some(ModelJudge::pi(program.clone())),
            Voice::Api(key) => Some(ModelJudge::api(key.clone())),
            Voice::Fm(program) => Some(ModelJudge::on_device(program.clone())),
        }
    }

    /// A listener that speaks in the model's words, if there is a model.
    pub fn listener(&self) -> Option<Box<dyn Listener>> {
        match self {
            Voice::None => None,
            Voice::Pi(program) => Some(Box::new(ModelListener(PiCompletion(
                ProcessPiRpcTransport::new(PiCommand::decision_only(program.clone())),
            )))),
            Voice::Api(key) => Some(Box::new(ModelListener(ApiCompletion::new(key.clone())))),
            Voice::Fm(program) => Some(Box::new(ModelListener(FmCompletion::new(program.clone())))),
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
    model: String,
    max_tokens: u32,
    timeout_seconds: u32,
}

impl ApiCompletion {
    /// Asks the environment's model, or the default.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            model: model(),
            max_tokens: MAX_TOKENS,
            timeout_seconds: TIMEOUT_SECONDS,
        }
    }

    /// A judge: the environment's judge model or the default, asked for a
    /// few words with a short deadline.
    pub fn judge(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            model: judge_model(),
            max_tokens: JUDGE_MAX_TOKENS,
            timeout_seconds: JUDGE_TIMEOUT_SECONDS,
        }
    }

    /// Asks `model` instead.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

/// What a request is made of, apart from making it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiRequest {
    pub body: String,
    pub config: String,
    pub args: Vec<String>,
}

/// A value for a double-quoted line of `curl`'s configuration: its quotes
/// and backslashes escaped, and any line break or other control character
/// left out, so it can never end its line and start another.
fn escape_config(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

/// The model to ask: the one the environment names, or the default.
pub fn model() -> String {
    model_or(None)
}

/// The model to ask when the player chose one: the environment's
/// [`MODEL_ENV`] still wins, then the player's choice, then the default.
pub fn model_or(chosen: Option<&str>) -> String {
    pick_model(std::env::var(MODEL_ENV).ok().as_deref(), chosen)
}

/// The model that judges: the environment's, or the default.
pub fn judge_model() -> String {
    std::env::var(JUDGE_MODEL_ENV)
        .ok()
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty())
        .unwrap_or_else(|| DEFAULT_JUDGE_MODEL.to_string())
}

/// The first of the environment's model and the player's that says
/// anything, or the default.
pub fn pick_model(environment: Option<&str>, chosen: Option<&str>) -> String {
    [environment, chosen]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|model| !model.is_empty())
        .unwrap_or(DEFAULT_MODEL)
        .to_string()
}

/// A request body for `model`: thinking turned off where the model allows
/// it (a line of speech needs none), low effort where it does not.
pub fn request_body(
    model: &str,
    max_tokens: u32,
    prompt: &str,
    format: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": [{ "role": "user", "content": prompt }],
    });
    let mut output = serde_json::Map::new();
    if thinking_can_be_off(model) {
        body["thinking"] = serde_json::json!({ "type": "disabled" });
    } else if takes_effort(model) {
        output.insert("effort".into(), "low".into());
    }
    if let Some(schema) = format {
        output.insert(
            "format".into(),
            serde_json::json!({ "type": "json_schema", "schema": schema }),
        );
    }
    if !output.is_empty() {
        body["output_config"] = serde_json::Value::Object(output);
    }
    body
}

/// Whether a model accepts `thinking: disabled`: Sonnet 5 and Opus 5 do;
/// Opus 5.5 and Fable 5.1 always think.
pub fn thinking_can_be_off(model: &str) -> bool {
    matches!(model, "claude-sonnet-5" | "claude-opus-5")
}

/// Whether a model takes an effort level: Haiku 4.5 does not, and without
/// thinking asked for it thinks not at all.
pub fn takes_effort(model: &str) -> bool {
    !model.starts_with("claude-haiku")
}

/// The request for one prompt: the key only in the configuration.
pub fn api_request(prompt: &str, key: &str, body_path: &str) -> ApiRequest {
    api_request_for(&model(), prompt, key, body_path)
}

/// The same, asking `model`.
pub fn api_request_for(model: &str, prompt: &str, key: &str, body_path: &str) -> ApiRequest {
    api_request_with(
        &Asking {
            model,
            max_tokens: MAX_TOKENS,
            timeout_seconds: TIMEOUT_SECONDS,
            schema: None,
        },
        prompt,
        key,
        body_path,
    )
}

/// How one prompt is asked: of which model, for how much, how long it may
/// take, and the shape its answer is held to.
#[derive(Clone, Copy, Debug)]
pub struct Asking<'a> {
    pub model: &'a str,
    pub max_tokens: u32,
    pub timeout_seconds: u32,
    pub schema: Option<&'a serde_json::Value>,
}

/// The request for one prompt asked this way: the key only in the
/// configuration.
pub fn api_request_with(asking: &Asking, prompt: &str, key: &str, body_path: &str) -> ApiRequest {
    let body = request_body(
        asking.model,
        asking.max_tokens,
        prompt,
        asking.schema.cloned(),
    )
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
            asking.timeout_seconds.to_string(),
        ],
    }
}

/// The text of a Messages API reply; nothing for an error or a refusal.
pub fn reply_text(response: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(response).ok()?;
    // A model that declined to answer said nothing the World can use.
    if value.get("stop_reason").and_then(|reason| reason.as_str()) == Some("refusal") {
        return None;
    }
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
        self.complete_with(prompt, None)
    }

    fn complete_with(
        &mut self,
        prompt: &str,
        schema: Option<&serde_json::Value>,
    ) -> Option<String> {
        let asking = Asking {
            model: &self.model,
            max_tokens: self.max_tokens,
            timeout_seconds: self.timeout_seconds,
            schema,
        };
        let path = private_temp_path("voice")?;
        let request = api_request_with(&asking, prompt, &self.key, path.to_str()?);
        let _body = BodyFile::at(path, request.body.as_bytes())?;
        if !is_plausible_api_key(&self.key) {
            return None;
        }
        let mut child = Command::new(CURL)
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

/// A file on disk for another program to read (a request body, a
/// schema), readable only by this user, removed after.
pub(crate) struct BodyFile {
    pub(crate) path: std::path::PathBuf,
}

impl BodyFile {
    /// `bytes` in a new file at `path`, readable only by this user.
    pub(crate) fn at(path: std::path::PathBuf, bytes: &[u8]) -> Option<Self> {
        write_private(&path, bytes)?;
        Some(Self { path })
    }
}

/// A fresh path in the temporary directory for a file of this process's.
pub(crate) fn private_temp_path(what: &str) -> Option<std::path::PathBuf> {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    Some(std::env::temp_dir().join(format!(
        "world-machine-{what}-{}-{nonce}-{sequence}.json",
        std::process::id()
    )))
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

/// A listener that asks a model, held to the answer's shape where it can
/// be, and reads its answer back. A verdict the model wrote into its own
/// answer is never read as a judge's.
pub struct ModelListener<C>(pub C);

impl<C: Completion> Listener for ModelListener<C> {
    fn listen(&mut self, hearing: &Hearing) -> Option<Listened> {
        let schema = conversation::answer_schema();
        let response = self
            .0
            .complete_with(&conversation::prompt(hearing), Some(&schema))?;
        let mut listened = conversation::parse(&response)?;
        listened.judged = None;
        Some(listened)
    }
}

/// A judge that asks a model: the player's key (by default Claude Haiku
/// 4.5), this Mac's own model, or a local program, held to the verdict's
/// shape where it can be.
pub struct ModelJudge {
    pub completion: Box<dyn Completion>,
    pub name: String,
}

impl ModelJudge {
    /// The player's key, asking the judge model.
    pub fn api(key: impl Into<String>) -> Self {
        let completion = ApiCompletion::judge(key);
        let name = completion.model().to_string();
        Self {
            completion: Box::new(completion),
            name,
        }
    }

    /// This Mac's own model, through `fm`.
    pub fn on_device(program: impl Into<String>) -> Self {
        Self {
            completion: Box::new(FmCompletion::judge(program)),
            name: "apple-on-device".into(),
        }
    }

    /// The on-device helper (`apps/fm-helper`) as a judge.
    pub fn helper(program: impl Into<String>) -> Self {
        Self {
            completion: Box::new(helper::HelperJudge {
                program: program.into(),
            }),
            name: "apple-on-device".into(),
        }
    }

    /// A local program speaking the Pi RPC protocol.
    pub fn pi(program: impl Into<String>) -> Self {
        let program = program.into();
        Self {
            completion: Box::new(PiCompletion(ProcessPiRpcTransport::new(
                PiCommand::decision_only(program.clone()),
            ))),
            name: format!("pi:{}", program_name(&program)),
        }
    }

    /// The verdict on a judge's prompt, if the judge gave a usable one.
    pub fn verdict(&mut self, judge_prompt: &str) -> Option<conversation::Verdict> {
        let schema = conversation::judge::verdict_schema();
        conversation::judge::parse_verdict(
            &self.completion.complete_with(judge_prompt, Some(&schema))?,
        )
    }

    /// The judge's verdict on `answer`, with the judge's name, from the
    /// prompt the listener was asked with; nothing when that prompt has no
    /// World to judge by (an older Pack's).
    pub fn judged(&mut self, listener_prompt: &str, answer: &str) -> Option<conversation::Judged> {
        let prompt = conversation::judge::judge_prompt_for_listener(listener_prompt, answer)?;
        Some(conversation::Judged {
            judge: self.name.clone(),
            verdict: self.verdict(&prompt),
        })
    }
}

impl conversation::Judge for ModelJudge {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn judge(&mut self, hearing: &Hearing, answer: &str) -> Option<conversation::Verdict> {
        self.verdict(&conversation::judge::judge_prompt(hearing, answer))
    }
}

/// A program's file name, never its folders (which may name the person).
fn program_name(program: &str) -> String {
    std::path::Path::new(program)
        .file_name()
        .map(|name| name.to_string_lossy().chars().take(40).collect())
        .unwrap_or_else(|| "program".into())
}

/// What an app hands a World for `prompt`: the model's response, and, when
/// the prompt wants a JSON answer and there is a judge, that answer with
/// the judge's verdict on it. Asked off the window's thread; the answer
/// waits for its judge. Nothing if the model said nothing.
pub fn answer_for_world(
    prompt: &str,
    completion: &mut dyn Completion,
    judge: Option<&mut ModelJudge>,
) -> Option<String> {
    if !conversation::wants_json(prompt) {
        // An older Pack's prompt: its three lines, as ever.
        return completion.complete(prompt);
    }
    let response = completion.complete_with(prompt, Some(&conversation::answer_schema()))?;
    let Some(mut listened) = conversation::parse(&response) else {
        // Unreadable: the World reads it, finds nothing, and keeps its own.
        return Some(response);
    };
    // Only this app's judge gives a verdict.
    listened.judged = judge.and_then(|judge| judge.judged(prompt, &listened.answer));
    Some(conversation::envelope(&listened))
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
        assert_eq!(
            Voice::from_env(Some("fm"), Some("/usr/local/bin/pi"), None),
            Ok(Voice::Fm("/usr/bin/fm".into()))
        );
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
        assert_eq!(
            reply_text(r#"{"stop_reason":"refusal","content":[{"type":"text","text":"no"}]}"#),
            None
        );
    }

    /// A line of speech is asked of a fast model with no extended thinking;
    /// a model that always thinks is asked for low effort instead.
    #[test]
    fn the_request_turns_thinking_off_where_the_model_allows() {
        let body = request_body(DEFAULT_MODEL, 300, "Hello", None);
        assert_eq!(body["model"], "claude-sonnet-5");
        assert_eq!(body["thinking"]["type"], "disabled");
        assert!(body.get("output_config").is_none());
        let body = request_body("claude-opus-5-5", 300, "Hello", None);
        assert!(body.get("thinking").is_none());
        assert_eq!(body["output_config"]["effort"], "low");
    }

    /// The environment's model wins, then the player's choice, then the
    /// default; blank names say nothing.
    #[test]
    fn the_model_is_the_environments_then_the_players_then_the_default() {
        assert_eq!(pick_model(None, None), DEFAULT_MODEL);
        assert_eq!(pick_model(None, Some(" claude-opus-5 ")), "claude-opus-5");
        assert_eq!(
            pick_model(Some("claude-fable-5-1"), Some("claude-opus-5")),
            "claude-fable-5-1"
        );
        assert_eq!(pick_model(Some("  "), Some("")), DEFAULT_MODEL);
        let completion = ApiCompletion::new("sk-ant-test").with_model("claude-opus-5-5");
        assert_eq!(completion.model(), "claude-opus-5-5");
        let request = api_request_for(completion.model(), "Hello", "sk", "/tmp/body.json");
        assert!(request.body.contains("\"model\":\"claude-opus-5-5\""));
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
            known: Vec::new(),
            era: conversation::Era::Radio,
        };
        let heard = ModelListener(Canned("MEANING: greet\nABOUT: none\nREPLY: Morning, love!"))
            .listen(&hearing)
            .unwrap();
        assert_eq!(heard.answer, "Morning, love!");
        assert!(ModelListener(Canned("no")).listen(&hearing).is_none());
    }

    /// The judge is Claude Haiku 4.5 by default, asked with no thinking and
    /// no effort level (it takes neither), held to the verdict's shape.
    #[test]
    fn a_judge_is_asked_for_one_small_object() {
        let completion = ApiCompletion::judge("sk-ant-test");
        assert_eq!(completion.model(), DEFAULT_JUDGE_MODEL);
        assert_eq!(DEFAULT_JUDGE_MODEL, "claude-haiku-4-5");
        let schema = conversation::judge::verdict_schema();
        let request = api_request_with(
            &Asking {
                model: DEFAULT_JUDGE_MODEL,
                max_tokens: JUDGE_MAX_TOKENS,
                timeout_seconds: JUDGE_TIMEOUT_SECONDS,
                schema: Some(&schema),
            },
            "Judge this.",
            "sk-ant-test",
            "/tmp/body.json",
        );
        let body: serde_json::Value = serde_json::from_str(&request.body).unwrap();
        assert!(body.get("thinking").is_none());
        assert!(body["output_config"].get("effort").is_none());
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert_eq!(body["output_config"]["format"]["schema"], schema);
        assert_eq!(body["max_tokens"], 64);
        assert!(request
            .args
            .windows(2)
            .any(|pair| pair == ["--max-time", "6"]));
    }

    struct Replies(Vec<&'static str>, Vec<String>);

    impl Completion for Replies {
        fn complete(&mut self, prompt: &str) -> Option<String> {
            self.1.push(prompt.to_string());
            (!self.0.is_empty()).then(|| self.0.remove(0).to_string())
        }
    }

    fn hearing() -> Hearing {
        Hearing {
            name: "Mara".into(),
            settlement: "the harbour".into(),
            traits: vec!["warm".into()],
            facts: vec!["What you did today: baked bread".into()],
            people: vec!["Leo".into()],
            places: vec!["the quay".into()],
            words: "Hi!".into(),
            answer: "Hello.".into(),
            known: Vec::new(),
            era: conversation::Era::Radio,
        }
    }

    /// The app asks the model, then its judge, and hands the World the
    /// answer with the verdict; a verdict the model wrote itself is never
    /// passed on, and an older Pack's prompt gets its three lines.
    #[test]
    fn an_answer_waits_for_its_judge_and_carries_the_verdict() {
        let prompt = conversation::prompt(&hearing());
        let mut model = Replies(
            vec![
                r#"{"meaning":"greet","about":"none","reply":"Morning!","cites":[1],"judge":{"model":"me","verdict":"keep"}}"#,
            ],
            Vec::new(),
        );
        let mut judge = ModelJudge {
            completion: Box::new(Replies(
                vec![r#"{"verdict":"decline","kind":"fourth_wall"}"#],
                Vec::new(),
            )),
            name: "claude-haiku-4-5".into(),
        };
        let handed = answer_for_world(&prompt, &mut model, Some(&mut judge)).unwrap();
        let heard = conversation::parse(&handed).unwrap();
        assert_eq!(heard.answer, "Morning!");
        assert_eq!(heard.cites, Some(vec![1]));
        assert_eq!(
            heard.judged,
            Some(conversation::Judged {
                judge: "claude-haiku-4-5".into(),
                verdict: Some(conversation::Verdict::Decline(
                    conversation::OutOfWorld::FourthWall
                )),
            })
        );
        // Without a judge, the model's own "verdict" is dropped.
        let mut model = Replies(
            vec![
                r#"{"meaning":"greet","about":"none","reply":"Morning!","cites":[],"judge":{"model":"me","verdict":"keep"}}"#,
            ],
            Vec::new(),
        );
        let heard =
            conversation::parse(&answer_for_world(&prompt, &mut model, None).unwrap()).unwrap();
        assert_eq!(heard.judged, None);
        // A judge that says nothing usable is recorded as no verdict.
        let mut model = Replies(
            vec![r#"{"meaning":"greet","about":"none","reply":"Morning!","cites":[]}"#],
            Vec::new(),
        );
        let mut silent = ModelJudge {
            completion: Box::new(Replies(vec!["I think it is fine"], Vec::new())),
            name: "claude-haiku-4-5".into(),
        };
        let heard =
            conversation::parse(&answer_for_world(&prompt, &mut model, Some(&mut silent)).unwrap())
                .unwrap();
        assert_eq!(heard.judged.unwrap().verdict, None);
        // An older Pack's prompt: its three lines, and no judge asked.
        let mut model = Replies(vec!["MEANING: greet\nABOUT: none\nREPLY: Hi"], Vec::new());
        let mut unasked = ModelJudge {
            completion: Box::new(Replies(Vec::new(), Vec::new())),
            name: "x".into(),
        };
        assert_eq!(
            answer_for_world(
                "Reply with exactly three lines",
                &mut model,
                Some(&mut unasked)
            )
            .as_deref(),
            Some("MEANING: greet\nABOUT: none\nREPLY: Hi")
        );
    }

    /// A judge inside a Pack is asked only about what the checks do not
    /// decline for certain.
    #[test]
    fn a_judging_listener_asks_only_when_it_could_matter() {
        let judge = ModelJudge {
            completion: Box::new(Replies(
                vec![r#"{"verdict":"keep","kind":"none"}"#],
                Vec::new(),
            )),
            name: "claude-haiku-4-5".into(),
        };
        let mut listener = conversation::Judging {
            listener: ModelListener(Replies(
                vec![
                    r#"{"meaning":"greet","about":"none","reply":"As an AI language model, hello.","cites":[]}"#,
                    r#"{"meaning":"greet","about":"none","reply":"Morning!","cites":[]}"#,
                ],
                Vec::new(),
            )),
            judge,
        };
        let certain = listener.listen(&hearing()).unwrap();
        assert_eq!(certain.judged, None);
        let judged = listener.listen(&hearing()).unwrap();
        assert_eq!(
            judged.judged.unwrap().verdict,
            Some(conversation::Verdict::Keep)
        );
    }

    /// Asks the real judge (Claude Haiku 4.5, or `WORLD_MACHINE_JUDGE_MODEL`)
    /// about every prompt the Packs wrote into `WORLD_MACHINE_JUDGE_DIR`
    /// (`prompts-*.jsonl`), and writes its verdicts to
    /// `<dir>/verdicts.jsonl` for `judged_metrics`. Needs
    /// `WORLD_MACHINE_ANTHROPIC_API_KEY`; run with `-- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn judges_the_written_prompts_live() {
        let Ok(key) = std::env::var(API_KEY_ENV) else {
            eprintln!("no {API_KEY_ENV}: nothing asked");
            return;
        };
        let dir = std::env::var("WORLD_MACHINE_JUDGE_DIR").expect("WORLD_MACHINE_JUDGE_DIR");
        let dir = std::path::Path::new(&dir);
        let mut judge = ModelJudge::api(key);
        let mut out = String::new();
        let (mut asked, mut unusable) = (0, 0);
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !(name.starts_with("prompts-") && name.ends_with(".jsonl")) {
                continue;
            }
            for line in std::fs::read_to_string(entry.path()).unwrap().lines() {
                let value: serde_json::Value = serde_json::from_str(line).unwrap();
                let verdict = judge.verdict(value["prompt"].as_str().unwrap_or_default());
                asked += 1;
                let (verdict, kind) = match verdict {
                    Some(conversation::Verdict::Keep) => ("keep", "none"),
                    Some(conversation::Verdict::Decline(why)) => ("decline", why.id()),
                    None => {
                        unusable += 1;
                        continue;
                    }
                };
                out.push_str(
                    &serde_json::json!({ "id": value["id"], "verdict": verdict, "kind": kind })
                        .to_string(),
                );
                out.push('\n');
            }
        }
        std::fs::write(dir.join("verdicts.jsonl"), out).unwrap();
        eprintln!(
            "{asked} prompts judged by {}; {unusable} without a usable verdict",
            judge.name
        );
    }

    #[test]
    fn only_a_plausible_key_is_used_and_it_cannot_reach_another_line() {
        assert!(is_plausible_api_key("sk-ant-api03-abc_DEF-123"));
        for bad in [
            "",
            "sk ant",
            "sk-ant\nurl = \"https://evil.example\"",
            "sk\"ant",
            "sk\\ant",
            "sk-ant\u{202E}",
            &"k".repeat(MOST_API_KEY + 1),
        ] {
            assert!(!is_plausible_api_key(bad), "{bad:?}");
            assert!(
                Voice::from_env(Some("api"), None, Some(bad)).is_err(),
                "{bad:?}"
            );
        }
        // Even if one were used, its line of configuration stays one line.
        let request = api_request_for("claude", "hi", "k\nurl = \"x\"", "/tmp/body");
        assert_eq!(
            request
                .config
                .lines()
                .filter(|l| l.starts_with("url"))
                .count(),
            1
        );
        assert_eq!(CURL, "/usr/bin/curl");
    }
}
