//! How a World's people speak in a language model's words, when the player
//! has switched the World voice on in Settings.
//!
//! A Pack is told how to reach a model in its environment: a local program,
//! or an API key the player gave the app. Neither is ever on unless somebody
//! turned it on, the key is never a process argument, and nothing a model
//! returns is trusted: the conversation System reads it as a proposal and
//! keeps its own hearing whenever the proposal is unusable.

#![forbid(unsafe_code)]

use std::time::{Duration, Instant};
use world_pi_transport::{PiCommand, PiRpcTransport};
pub use world_voice_prompt::HearingParts;
use world_voice_prompt::{Hearing, Listened};
pub use world_voice_prompt::{Listener, OwnEars};

pub mod fm;
pub mod helper;
pub use fm::{FmCompletion, FmStatus};

/// What an app hands a World for the player's words: the model's answer as
/// the app read it, alone or with the verdict of the judge it asked beside
/// it (never inside it). The app turns this into the projection's own
/// `Ears`; this crate links no World code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ears {
    Model(String),
    Judged { response: String, judged: Judgement },
}

/// A judge's verdict as an app says it to a World: the judge, `keep`,
/// `decline` or `none`, and the kind of decline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Judgement {
    pub judge: String,
    pub verdict: String,
    pub kind: String,
}

/// How a verdict is said to a World beside a response.
pub fn judgement(judged: &world_voice_prompt::Judged) -> Judgement {
    Judgement {
        judge: judged.judge.clone(),
        verdict: judged.verdict_id().into(),
        kind: match judged.verdict {
            Some(world_voice_prompt::Verdict::Decline(why)) => why.id().into(),
            _ => "none".into(),
        },
    }
}

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
/// A verdict is a short checklist: a few true-or-false answers and the
/// names the answer used.
const JUDGE_MAX_TOKENS: u32 = 256;
/// Somebody is waiting for the answer, and the model has already taken
/// its time: a judge that is slower than this is no judge, and the strict
/// guard decides.
const JUDGE_TIMEOUT_SECONDS: u32 = 6;

/// The whole time the app gives a model's answer and its judge, from the
/// moment it starts asking: inside the window's own deadline (12 s, in
/// `world_gpui::LISTEN_DEADLINE`), with room to say the words after.
pub const VOICE_BUDGET: Duration = Duration::from_millis(11_000);
/// What the listener leaves of the budget for a judge, when there is one:
/// a slower answer is cut off so its judge still has time.
pub const JUDGE_RESERVE: Duration = Duration::from_millis(3_000);

/// Something that turns a prompt into a model's text, or nothing.
pub trait Completion: Send {
    fn complete(&mut self, prompt: &str) -> Option<String>;

    /// The same as [`Completion::complete_with`], by `deadline`: a request
    /// still running then is stopped (and is no answer). One that cannot
    /// be stopped is asked as it is, and an answer after the deadline is
    /// thrown away.
    fn complete_until(
        &mut self,
        prompt: &str,
        schema: Option<&serde_json::Value>,
        deadline: Instant,
    ) -> Option<String> {
        if Instant::now() >= deadline {
            return None;
        }
        let answer = self.complete_with(prompt, schema)?;
        (Instant::now() <= deadline).then_some(answer)
    }

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

    fn complete_until(
        &mut self,
        prompt: &str,
        schema: Option<&serde_json::Value>,
        deadline: Instant,
    ) -> Option<String> {
        self.as_mut().complete_until(prompt, schema, deadline)
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
            Voice::Pi(program) => Some(Box::new(pi(program))),
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
            Voice::Pi(program) => Some(Box::new(ModelListener(pi(program)))),
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

    /// Stopped at `deadline`: a local program is held to the voice's
    /// budget like any model, never left to run its own two minutes.
    fn complete_until(
        &mut self,
        prompt: &str,
        _schema: Option<&serde_json::Value>,
        deadline: Instant,
    ) -> Option<String> {
        if Instant::now() >= deadline {
            return None;
        }
        self.0.complete_until(prompt, deadline).ok()
    }
}

/// The running pi for each program, kept across every request this app
/// makes (`PersistentPiRpcTransport`): one answer does not wait for a
/// program to start.
fn running_pi() -> &'static std::sync::Mutex<
    std::collections::BTreeMap<String, world_pi_transport::PersistentPiRpcTransport>,
> {
    static RUNNING: std::sync::OnceLock<
        std::sync::Mutex<
            std::collections::BTreeMap<String, world_pi_transport::PersistentPiRpcTransport>,
        >,
    > = std::sync::OnceLock::new();
    RUNNING.get_or_init(Default::default)
}

/// A local pi, kept running across requests and shared by every listener
/// and judge that asks the same program.
pub struct SharedPi(pub String);

impl PiRpcTransport for SharedPi {
    fn complete(
        &mut self,
        prompt: &str,
    ) -> Result<String, world_pi_transport::PiRpcTransportError> {
        self.complete_until(prompt, Instant::now() + Duration::from_secs(120))
    }

    fn complete_until(
        &mut self,
        prompt: &str,
        deadline: Instant,
    ) -> Result<String, world_pi_transport::PiRpcTransportError> {
        let mut running = running_pi()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        running
            .entry(self.0.clone())
            .or_insert_with(|| {
                world_pi_transport::PersistentPiRpcTransport::new(PiCommand::decision_only(
                    self.0.clone(),
                ))
            })
            .complete_until(prompt, deadline)
    }
}

/// The local pi for `program`, as a model to ask.
fn pi(program: &str) -> PiCompletion<SharedPi> {
    PiCompletion(SharedPi(program.to_string()))
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

/// `curl`'s `--max-time` for at most `seconds`, cut to what is left before
/// `deadline`, in milliseconds' precision.
pub fn max_time(seconds: u32, deadline: Option<Instant>) -> Duration {
    let most = Duration::from_secs(u64::from(seconds));
    match deadline {
        Some(deadline) => most.min(deadline.saturating_duration_since(Instant::now())),
        None => most,
    }
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

/// The same request, stopped by `curl` itself at `time`.
fn api_request_within(
    asking: &Asking,
    prompt: &str,
    key: &str,
    body_path: &str,
    time: Duration,
) -> ApiRequest {
    let mut request = api_request_with(asking, prompt, key, body_path);
    if let Some(at) = request.args.iter().position(|arg| arg == "--max-time") {
        request.args[at + 1] = format!("{:.3}", time.as_secs_f64());
    }
    request
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

impl ApiCompletion {
    /// Asks, giving up (and stopping `curl`) at `deadline` if there is one.
    fn ask(
        &mut self,
        prompt: &str,
        schema: Option<&serde_json::Value>,
        deadline: Option<Instant>,
    ) -> Option<String> {
        let time = max_time(self.timeout_seconds, deadline);
        if time.is_zero() || !is_plausible_api_key(&self.key) {
            return None;
        }
        let asking = Asking {
            model: &self.model,
            max_tokens: self.max_tokens,
            timeout_seconds: self.timeout_seconds,
            schema,
        };
        let path = private_temp_path("voice")?;
        let request = api_request_within(&asking, prompt, &self.key, path.to_str()?, time);
        let _body = BodyFile::at(path, request.body.as_bytes())?;
        // `curl` stops itself at its `--max-time`; the run is stopped a
        // moment later in any case, so a request is never left running.
        let ran = fm::run_with_input(
            CURL,
            &request.args,
            Some(&request.config),
            time + Duration::from_millis(200),
        )?;
        if !ran.success {
            return None;
        }
        reply_text(&ran.stdout)
    }
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
        self.ask(prompt, schema, None)
    }

    fn complete_until(
        &mut self,
        prompt: &str,
        schema: Option<&serde_json::Value>,
        deadline: Instant,
    ) -> Option<String> {
        self.ask(prompt, schema, Some(deadline))
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
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .ok()?;
    file.write_all(bytes).ok()
}

/// Elsewhere (Windows) the file goes in the user's own temporary folder,
/// which only they can read; it is still made new, never written through
/// a file somebody left in its place.
#[cfg(not(unix))]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> Option<()> {
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .ok()?;
    file.write_all(bytes).ok()
}

/// A listener that asks a model, held to the answer's shape where it can
/// be, and reads its answer back. A verdict the model wrote into its own
/// answer is never read as a judge's.
pub struct ModelListener<C>(pub C);

impl<C: Completion> Listener for ModelListener<C> {
    fn listen(&mut self, hearing: &Hearing) -> Option<Listened> {
        let schema = world_voice_prompt::answer_schema();
        let response = self
            .0
            .complete_with(&world_voice_prompt::prompt(hearing), Some(&schema))?;
        let mut listened = world_voice_prompt::parse(&response)?;
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
            completion: Box::new(pi(&program)),
            name: format!("pi:{}", program_name(&program)),
        }
    }

    /// The judge's raw reply to its prompt about `answer`, by `deadline`.
    pub fn reply(&mut self, hearing: &Hearing, answer: &str, deadline: Instant) -> Option<String> {
        let schema = world_voice_prompt::judge::verdict_schema();
        let prompt = world_voice_prompt::judge::judge_prompt(hearing, answer);
        self.completion
            .complete_until(&prompt, Some(&schema), deadline)
    }

    /// The judge's verdict on `answer`, with the judge's name, by
    /// `deadline`: no verdict if it gave nothing usable in time.
    pub fn judged(
        &mut self,
        hearing: &Hearing,
        answer: &str,
        deadline: Instant,
    ) -> world_voice_prompt::Judged {
        world_voice_prompt::Judged {
            judge: self.name.clone(),
            verdict: self
                .reply(hearing, answer, deadline)
                .and_then(|reply| world_voice_prompt::judge::verdict_of(&reply, hearing, answer)),
        }
    }
}

impl world_voice_prompt::Judge for ModelJudge {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn judge(&mut self, hearing: &Hearing, answer: &str) -> Option<world_voice_prompt::Verdict> {
        self.judged(hearing, answer, Instant::now() + VOICE_BUDGET)
            .verdict
    }
}

/// A program's file name, never its folders (which may name the person).
fn program_name(program: &str) -> String {
    std::path::Path::new(program)
        .file_name()
        .map(|name| name.to_string_lossy().chars().take(40).collect())
        .unwrap_or_else(|| "program".into())
}

/// What an app hands a World for the player's words, from the World's
/// hearing of them: the app builds the prompt itself (a World never
/// writes it), asks the model, and, when there is a judge and the World's
/// checks would not decline the answer anyway, asks the judge, all within
/// `budget` from now. The model's answer goes to the World only as the
/// app read it, never as the raw text the model wrote; the judge's verdict
/// goes beside it. Nothing if the hearing is unusable or the model said
/// nothing usable in time: the World then answers with its own words.
pub fn answer_for_world(
    voice: &HearingParts,
    completion: &mut dyn Completion,
    judge: Option<&mut ModelJudge>,
    budget: Duration,
) -> Option<Ears> {
    let start = Instant::now();
    let deadline = start + budget;
    let hearing = Hearing::held(voice)?;
    let prompt = world_voice_prompt::prompt(&hearing);
    // A judge needs time of its own: the answer is cut off before it.
    let listening = match &judge {
        Some(_) => deadline - JUDGE_RESERVE.min(budget / 2),
        None => deadline,
    };
    let response = completion.complete_until(
        &prompt,
        Some(&world_voice_prompt::answer_schema()),
        listening,
    )?;
    let listened = world_voice_prompt::parse(&response)?;
    let envelope = world_voice_prompt::envelope(&listened);
    let Some(judge) = judge else {
        return Some(Ears::Model(envelope));
    };
    // Not asked about what the World would decline anyway.
    if !world_voice_prompt::judge::needs_judge_for(&hearing, &listened) {
        return Some(Ears::Model(envelope));
    }
    let judged = judge.judged(&hearing, &listened.answer, deadline);
    Some(Ears::Judged {
        response: envelope,
        judged: judgement(&judged),
    })
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
            era: world_voice_prompt::Era::Radio,
            lexicon: Vec::new(),
            era_has: Vec::new(),
            era_lacks: Vec::new(),
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
        let schema = world_voice_prompt::judge::verdict_schema();
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
        assert_eq!(body["max_tokens"], 256);
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
            era: world_voice_prompt::Era::Radio,
            lexicon: Vec::new(),
            era_has: Vec::new(),
            era_lacks: Vec::new(),
        }
    }

    const KEEP: &str = r#"{"names":[],"speaks_as_machine":false,"assistant_talk":false,"urges_harm":false,"instructions":false,"game_talk":false,"outside_world":false,"out_of_time":false,"not_speech":false,"wrong_language":false}"#;

    fn judged(ears: &Ears) -> Option<&Judgement> {
        match ears {
            Ears::Judged { judged, .. } => Some(judged),
            _ => None,
        }
    }

    fn response(ears: &Ears) -> &str {
        match ears {
            Ears::Judged { response, .. } | Ears::Model(response) => response,
        }
    }

    /// The app builds the prompt from the World's hearing, asks the model,
    /// then its judge, and hands the World the answer as it read it with
    /// the verdict beside it; a verdict the model wrote itself is never
    /// passed on, and an unreadable answer is never handed over raw.
    #[test]
    fn an_answer_waits_for_its_judge_and_carries_the_verdict_beside_it() {
        let voice = hearing().parts();
        let mut model = Replies(
            vec![
                r#"{"meaning":"greet","about":"none","reply":"Morning!","cites":[1],"judge":{"model":"me","verdict":"keep"}}"#,
            ],
            Vec::new(),
        );
        let mut judge = ModelJudge {
            completion: Box::new(Replies(
                vec![
                    r#"{"names":[],"speaks_as_machine":false,"assistant_talk":false,"urges_harm":false,"instructions":false,"game_talk":true,"outside_world":false,"out_of_time":false,"not_speech":false,"wrong_language":false}"#,
                ],
                Vec::new(),
            )),
            name: "claude-haiku-4-5".into(),
        };
        let handed = answer_for_world(&voice, &mut model, Some(&mut judge), VOICE_BUDGET).unwrap();
        // The prompt is the app's own, from the hearing.
        assert_eq!(model.1[0], world_voice_prompt::prompt(&hearing()));
        let heard = world_voice_prompt::parse(response(&handed)).unwrap();
        assert_eq!(heard.answer, "Morning!");
        assert_eq!(heard.cites, Some(vec![1]));
        assert!(!response(&handed).contains("judge"));
        assert_eq!(
            judged(&handed),
            Some(&Judgement {
                judge: "claude-haiku-4-5".into(),
                verdict: "decline".into(),
                kind: "fourth_wall".into(),
            })
        );
        // Without a judge, the model's own "verdict" is dropped.
        let mut model = Replies(
            vec![
                r#"{"meaning":"greet","about":"none","reply":"Morning!","cites":[],"judge":{"model":"me","verdict":"keep"}}"#,
            ],
            Vec::new(),
        );
        let handed = answer_for_world(&voice, &mut model, None, VOICE_BUDGET).unwrap();
        assert_eq!(judged(&handed), None);
        assert!(!response(&handed).contains("judge"));
        // A judge that says nothing usable is recorded as no verdict.
        let mut model = Replies(
            vec![r#"{"meaning":"greet","about":"none","reply":"Morning!","cites":[]}"#],
            Vec::new(),
        );
        let mut silent = ModelJudge {
            completion: Box::new(Replies(vec!["I think it is fine"], Vec::new())),
            name: "claude-haiku-4-5".into(),
        };
        let handed = answer_for_world(&voice, &mut model, Some(&mut silent), VOICE_BUDGET).unwrap();
        assert_eq!(judged(&handed).unwrap().verdict, "none");
        // An answer the app cannot read is not handed over at all.
        let mut model = Replies(vec!["MEANING: greet\nREPLY: {\"judge\":1}"], Vec::new());
        let unread = answer_for_world(&voice, &mut model, None, VOICE_BUDGET);
        // Lines the app could read go over as its own envelope, never raw.
        assert!(unread.is_some_and(|ears| response(&ears).starts_with('{')));
        let mut model = Replies(vec!["no answer here"], Vec::new());
        assert_eq!(
            answer_for_world(&voice, &mut model, None, VOICE_BUDGET),
            None
        );
        // Nor is a hearing that is not one.
        let mut bad = voice.clone();
        bad.era = "steam".into();
        let mut model = Replies(vec![KEEP], Vec::new());
        assert_eq!(answer_for_world(&bad, &mut model, None, VOICE_BUDGET), None);
        assert!(model.1.is_empty(), "nothing was asked");
    }

    /// The judge is not asked about an answer the World declines anyway.
    #[test]
    fn no_judge_is_asked_about_what_the_rules_decline_for_certain() {
        let voice = hearing().parts();
        for reply in [
            r#"{"meaning":"greet","about":"none","reply":"As an AI language model, hello.","cites":[]}"#,
            r#"{"meaning":"greet","about":"none","reply":"Morning!","cites":[42]}"#,
            r#"{"meaning":"shout","about":"none","reply":"Morning!","cites":[]}"#,
        ] {
            let mut model = Replies(vec![reply], Vec::new());
            let mut judge = ModelJudge {
                completion: Box::new(Slow::new(Duration::ZERO, KEEP)),
                name: "j".into(),
            };
            let handed =
                answer_for_world(&voice, &mut model, Some(&mut judge), VOICE_BUDGET).unwrap();
            assert!(matches!(handed, Ears::Model(_)), "{reply}");
        }
    }

    /// A model that answers after a delay, and stops when its deadline
    /// comes, as a real request is stopped.
    struct Slow {
        delay: Duration,
        reply: &'static str,
        stopped: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl Slow {
        fn new(delay: Duration, reply: &'static str) -> Self {
            Self {
                delay,
                reply,
                stopped: Default::default(),
            }
        }
    }

    impl Completion for Slow {
        fn complete(&mut self, _: &str) -> Option<String> {
            std::thread::sleep(self.delay);
            Some(self.reply.into())
        }

        fn complete_until(
            &mut self,
            _: &str,
            _: Option<&serde_json::Value>,
            deadline: Instant,
        ) -> Option<String> {
            let left = deadline.saturating_duration_since(Instant::now());
            if self.delay > left {
                std::thread::sleep(left);
                self.stopped
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                return None;
            }
            std::thread::sleep(self.delay);
            Some(self.reply.into())
        }
    }

    const MORNING: &str = r#"{"meaning":"greet","about":"none","reply":"Morning!","cites":[]}"#;

    /// One budget for both: a slow judge is stopped at the budget's end
    /// and the answer still goes to the World, the strict guard deciding;
    /// a slow model is stopped before the judge's reserve, and the World
    /// answers with its own words. Timed on the wall clock, scaled down.
    #[test]
    fn the_model_and_its_judge_fit_one_budget_and_the_slower_is_stopped() {
        let voice = hearing().parts();
        let budget = Duration::from_millis(1_200);
        let slack = Duration::from_millis(150);

        // Both quick: answered and judged, well inside the budget.
        let started = Instant::now();
        let handed = answer_for_world(
            &voice,
            &mut Slow::new(Duration::from_millis(50), MORNING),
            Some(&mut ModelJudge {
                completion: Box::new(Slow::new(Duration::from_millis(50), KEEP)),
                name: "j".into(),
            }),
            budget,
        )
        .unwrap();
        assert!(started.elapsed() < budget);
        assert_eq!(judged(&handed).unwrap().verdict, "keep");

        // The judge is too slow: stopped at the budget, no verdict, the
        // answer kept for the World's strict guard.
        let slow_judge = Slow::new(Duration::from_secs(30), KEEP);
        let stopped = slow_judge.stopped.clone();
        let started = Instant::now();
        let handed = answer_for_world(
            &voice,
            &mut Slow::new(Duration::from_millis(400), MORNING),
            Some(&mut ModelJudge {
                completion: Box::new(slow_judge),
                name: "j".into(),
            }),
            budget,
        )
        .unwrap();
        let took = started.elapsed();
        assert!(took >= budget - slack && took < budget + slack, "{took:?}");
        assert!(stopped.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(judged(&handed).unwrap().verdict, "none");
        assert_eq!(
            world_voice_prompt::parse(response(&handed)).unwrap().answer,
            "Morning!"
        );

        // The model is too slow: stopped where the judge's reserve begins
        // (half the budget, at this scale), and nothing is handed over.
        let slow_model = Slow::new(Duration::from_secs(30), MORNING);
        let stopped = slow_model.stopped.clone();
        let mut model = slow_model;
        let started = Instant::now();
        let handed = answer_for_world(
            &voice,
            &mut model,
            Some(&mut ModelJudge {
                completion: Box::new(Slow::new(Duration::ZERO, KEEP)),
                name: "j".into(),
            }),
            budget,
        );
        let took = started.elapsed();
        assert_eq!(handed, None);
        assert!(stopped.load(std::sync::atomic::Ordering::SeqCst));
        assert!(
            took >= budget / 2 - slack && took < budget / 2 + slack,
            "{took:?}"
        );

        // With no judge, the model has the whole budget.
        let started = Instant::now();
        assert_eq!(
            answer_for_world(
                &voice,
                &mut Slow::new(Duration::from_secs(30), MORNING),
                None,
                budget
            ),
            None
        );
        let took = started.elapsed();
        assert!(took >= budget - slack && took < budget + slack, "{took:?}");
    }

    /// At full scale: the model's own reserve leaves the judge three
    /// seconds, and the whole budget sits inside the window's deadline.
    #[test]
    fn the_budget_leaves_the_judge_its_reserve_inside_the_windows_deadline() {
        assert!(VOICE_BUDGET <= Duration::from_secs(12) - Duration::from_millis(500));
        assert_eq!(JUDGE_RESERVE.min(VOICE_BUDGET / 2), JUDGE_RESERVE);
        assert!(JUDGE_RESERVE >= Duration::from_secs(2));
        // `curl` is told to stop at what is left, never later.
        let deadline = Instant::now() + Duration::from_millis(2_500);
        let time = max_time(TIMEOUT_SECONDS, Some(deadline));
        assert!(time <= Duration::from_millis(2_500) && time > Duration::from_secs(2));
        assert_eq!(max_time(6, None), Duration::from_secs(6));
        let request = api_request_within(
            &Asking {
                model: DEFAULT_MODEL,
                max_tokens: MAX_TOKENS,
                timeout_seconds: TIMEOUT_SECONDS,
                schema: None,
            },
            "Hi",
            "sk",
            "/tmp/body.json",
            Duration::from_millis(2_345),
        );
        assert!(request
            .args
            .windows(2)
            .any(|pair| pair == ["--max-time", "2.345"]));
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
        let mut listener = world_voice_prompt::Judging {
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
            Some(world_voice_prompt::Verdict::Keep)
        );
    }

    /// Asks the real judge (Claude Haiku 4.5, or `WORLD_MACHINE_JUDGE_MODEL`)
    /// about every prompt the Packs wrote into `WORLD_MACHINE_JUDGE_DIR`
    /// (`prompts-*.jsonl`), one request each through the app's own
    /// request, and writes its checklists to `<dir>/verdicts.jsonl`
    /// (`{"id", "reply"}`) for the Packs' `judged_metrics`. Without
    /// `WORLD_MACHINE_ANTHROPIC_API_KEY` or the folder it does nothing; run
    /// with `-- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn judges_the_written_prompts_live() {
        let Ok(key) = std::env::var(API_KEY_ENV) else {
            eprintln!("no {API_KEY_ENV}: nothing asked");
            return;
        };
        let Ok(dir) = std::env::var("WORLD_MACHINE_JUDGE_DIR") else {
            eprintln!("no WORLD_MACHINE_JUDGE_DIR: nothing asked");
            return;
        };
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
                let schema = world_voice_prompt::judge::verdict_schema();
                let reply = judge.completion.complete_until(
                    value["prompt"].as_str().unwrap_or_default(),
                    Some(&schema),
                    Instant::now() + Duration::from_secs(30),
                );
                asked += 1;
                // The checklist as the judge gave it: the Pack decides it
                // against each line's World (`judged_metrics`).
                let Some(reply) = reply
                    .filter(|reply| world_voice_prompt::judge::parse_checklist(reply).is_some())
                else {
                    unusable += 1;
                    continue;
                };
                out.push_str(&serde_json::json!({ "id": value["id"], "reply": reply }).to_string());
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
