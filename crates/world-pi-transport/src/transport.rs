use crate::{PiRpcEventParser, PiRpcProtocolError};
use serde_json::json;
use std::error::Error;
use std::fmt;
use std::io::Write as _;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub trait PiRpcTransport {
    fn complete(&mut self, prompt: &str) -> Result<String, PiRpcTransportError>;

    /// The same, by `deadline`: a request still running then is stopped,
    /// and is a [`PiRpcTransportError::Timeout`]. A transport that cannot
    /// be stopped is asked as it is, and an answer after the deadline is
    /// thrown away.
    fn complete_until(
        &mut self,
        prompt: &str,
        deadline: Instant,
    ) -> Result<String, PiRpcTransportError> {
        let answer = self.complete(prompt)?;
        if Instant::now() > deadline {
            return Err(PiRpcTransportError::Timeout { millis: 0 });
        }
        Ok(answer)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PiCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl Default for PiCommand {
    fn default() -> Self {
        Self::decision_only("pi")
    }
}

impl PiCommand {
    pub fn decision_only(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: vec![
                "--mode".into(),
                "rpc".into(),
                "--no-tools".into(),
                "--no-extensions".into(),
                "--no-skills".into(),
                "--no-prompt-templates".into(),
                "--no-themes".into(),
                "--no-session".into(),
                "--hide-cwd-in-prompt".into(),
            ],
        }
    }
}

#[derive(Debug)]
pub enum PiRpcTransportError {
    Spawn(std::io::Error),
    Stdin(std::io::Error),
    Poll(std::io::Error),
    Wait(std::io::Error),
    ReadOutput(std::io::Error),
    ReaderPanicked,
    Timeout { millis: u128 },
    Cancelled,
    NonZeroExit { code: Option<i32>, stderr: String },
    Protocol(PiRpcProtocolError),
}

impl fmt::Display for PiRpcTransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(error) => write!(f, "failed to start external Pi runtime: {error}"),
            Self::Stdin(error) => write!(f, "failed to write Pi RPC prompt: {error}"),
            Self::Poll(error) => write!(f, "failed to poll Pi RPC process: {error}"),
            Self::Wait(error) => write!(f, "failed while waiting for Pi RPC runtime: {error}"),
            Self::ReadOutput(error) => write!(f, "failed to read Pi RPC output: {error}"),
            Self::ReaderPanicked => write!(f, "Pi RPC output reader thread panicked"),
            Self::Timeout { millis } => {
                write!(f, "Pi RPC decision timed out after {millis}ms")
            }
            Self::Cancelled => write!(f, "Pi RPC request was cancelled"),
            Self::NonZeroExit { code, stderr } => write!(
                f,
                "Pi RPC process exited unsuccessfully ({code:?}): {}",
                excerpt(stderr)
            ),
            Self::Protocol(error) => error.fmt(f),
        }
    }
}

impl Error for PiRpcTransportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Spawn(error)
            | Self::Stdin(error)
            | Self::Poll(error)
            | Self::Wait(error)
            | Self::ReadOutput(error) => Some(error),
            Self::Protocol(error) => Some(error),
            Self::ReaderPanicked
            | Self::Timeout { .. }
            | Self::Cancelled
            | Self::NonZeroExit { .. } => None,
        }
    }
}

pub struct ProcessPiRpcTransport {
    command: PiCommand,
    request_sequence: u64,
    timeout: Duration,
    cancel: Option<world_run::Cancel>,
}

impl Default for ProcessPiRpcTransport {
    fn default() -> Self {
        Self::new(PiCommand::default())
    }
}

impl ProcessPiRpcTransport {
    pub fn new(command: PiCommand) -> Self {
        Self {
            command,
            request_sequence: 1,
            timeout: Duration::from_secs(120),
            cancel: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn command(&self) -> &PiCommand {
        &self.command
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl ProcessPiRpcTransport {
    /// Cancels every request this transport makes once `cancel` is set.
    pub fn with_cancel(mut self, cancel: world_run::Cancel) -> Self {
        self.cancel = Some(cancel);
        self
    }

    fn ask(&mut self, prompt: &str, deadline: Instant) -> Result<String, PiRpcTransportError> {
        let request_id = format!("world-machine-{}", self.request_sequence);
        self.request_sequence += 1;
        let request = json!({
            "id": request_id,
            "type": "prompt",
            "message": prompt,
        });
        let input = format!("{request}\n");
        let started = Instant::now();
        let ran = world_run::Run {
            program: &self.command.program,
            args: &self.command.args,
            input: Some(&input),
            deadline: Some(deadline),
            cancel: self.cancel.clone(),
        }
        .run()
        .map_err(|why| match why {
            world_run::NotRun::Spawn(error) => PiRpcTransportError::Spawn(error),
            world_run::NotRun::TimedOut => PiRpcTransportError::Timeout {
                millis: started.elapsed().as_millis(),
            },
            world_run::NotRun::Cancelled => PiRpcTransportError::Cancelled,
        })?;
        if !ran.success {
            return Err(PiRpcTransportError::NonZeroExit {
                code: ran.code,
                stderr: ran.stderr,
            });
        }
        parse_stdout(ran.stdout.as_bytes()).map_err(PiRpcTransportError::Protocol)
    }
}

impl PiRpcTransport for ProcessPiRpcTransport {
    fn complete(&mut self, prompt: &str) -> Result<String, PiRpcTransportError> {
        let deadline = Instant::now() + self.timeout;
        self.ask(prompt, deadline)
    }

    /// Held to whichever comes first, `deadline` or the transport's own
    /// timeout: a local program is stopped at the voice's budget, never
    /// left to run its full two minutes.
    fn complete_until(
        &mut self,
        prompt: &str,
        deadline: Instant,
    ) -> Result<String, PiRpcTransportError> {
        let deadline = deadline.min(Instant::now() + self.timeout);
        self.ask(prompt, deadline)
    }
}

/// A pi kept running across requests, so each answer does not wait for a
/// program to start: one request at a time, written as a line on its
/// standard input, its answer read until pi says the turn is over
/// (`agent_end`). A request past its deadline or cancelled stops pi; the
/// next starts it again. pi still keeps no session on disk
/// (`--no-session`), and is started afresh every [`MOST_REQUESTS`]
/// requests so what it remembers of earlier prompts stays small.
pub struct PersistentPiRpcTransport {
    command: PiCommand,
    timeout: Duration,
    request_sequence: u64,
    cancel: Option<world_run::Cancel>,
    live: Option<Live>,
}

/// The most requests one running pi answers before it is started again.
pub const MOST_REQUESTS: u32 = 8;

struct Live {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    lines: std::sync::mpsc::Receiver<String>,
    served: u32,
}

impl Live {
    fn stop(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl PersistentPiRpcTransport {
    pub fn new(command: PiCommand) -> Self {
        Self {
            command,
            timeout: Duration::from_secs(120),
            request_sequence: 1,
            cancel: None,
            live: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Cancels every request this transport makes once `cancel` is set.
    pub fn with_cancel(mut self, cancel: world_run::Cancel) -> Self {
        self.cancel = Some(cancel);
        self
    }

    /// Whether a pi is running now.
    pub fn is_running(&self) -> bool {
        self.live.is_some()
    }

    fn start(&self) -> Result<Live, PiRpcTransportError> {
        let mut child = Command::new(&self.command.program)
            .args(&self.command.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(PiRpcTransportError::Spawn)?;
        let stdin = child.stdin.take().ok_or_else(|| {
            PiRpcTransportError::Stdin(std::io::Error::other("Pi stdin was not piped"))
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            PiRpcTransportError::ReadOutput(std::io::Error::other("Pi stdout was not piped"))
        })?;
        let (send, lines) = std::sync::mpsc::channel();
        thread::spawn(move || {
            use std::io::BufRead as _;
            for line in std::io::BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        Ok(Live {
            child,
            stdin,
            lines,
            served: 0,
        })
    }

    fn ask(&mut self, prompt: &str, deadline: Instant) -> Result<String, PiRpcTransportError> {
        let started = Instant::now();
        let mut live = match self.live.take() {
            Some(live) => live,
            None => self.start()?,
        };
        let request_id = format!("world-machine-{}", self.request_sequence);
        self.request_sequence += 1;
        let request = json!({ "id": request_id, "type": "prompt", "message": prompt });
        if let Err(error) = writeln!(live.stdin, "{request}").and_then(|()| live.stdin.flush()) {
            live.stop();
            return Err(PiRpcTransportError::Stdin(error));
        }
        let mut parser = PiRpcEventParser::default();
        loop {
            if self
                .cancel
                .as_ref()
                .is_some_and(world_run::Cancel::is_cancelled)
            {
                live.stop();
                return Err(PiRpcTransportError::Cancelled);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                live.stop();
                return Err(PiRpcTransportError::Timeout {
                    millis: started.elapsed().as_millis(),
                });
            }
            match live.lines.recv_timeout(left.min(Duration::from_millis(25))) {
                Ok(line) if line.trim().is_empty() => {}
                Ok(line) => {
                    let ended = serde_json::from_str::<serde_json::Value>(&line)
                        .ok()
                        .and_then(|value| value.get("type")?.as_str().map(str::to_owned))
                        .is_some_and(|kind| kind == "agent_end");
                    if let Err(error) = parser.push_line(&line) {
                        live.stop();
                        return Err(PiRpcTransportError::Protocol(error));
                    }
                    if ended {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                // pi is gone: whatever it said is all there is.
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    live.stop();
                    return parser.finish().map_err(PiRpcTransportError::Protocol);
                }
            }
        }
        live.served += 1;
        if live.served >= MOST_REQUESTS {
            live.stop();
        } else {
            self.live = Some(live);
        }
        parser.finish().map_err(PiRpcTransportError::Protocol)
    }
}

impl Drop for PersistentPiRpcTransport {
    fn drop(&mut self) {
        if let Some(live) = self.live.take() {
            live.stop();
        }
    }
}

impl PiRpcTransport for PersistentPiRpcTransport {
    fn complete(&mut self, prompt: &str) -> Result<String, PiRpcTransportError> {
        let deadline = Instant::now() + self.timeout;
        self.ask(prompt, deadline)
    }

    fn complete_until(
        &mut self,
        prompt: &str,
        deadline: Instant,
    ) -> Result<String, PiRpcTransportError> {
        let deadline = deadline.min(Instant::now() + self.timeout);
        self.ask(prompt, deadline)
    }
}

fn parse_stdout(stdout: &[u8]) -> Result<String, PiRpcProtocolError> {
    let text = String::from_utf8_lossy(stdout);
    let mut parser = PiRpcEventParser::default();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        parser.push_line(line)?;
    }
    parser.finish()
}

fn excerpt(stderr: &str) -> String {
    const LIMIT: usize = 800;
    let trimmed = stderr.trim();
    if trimmed.chars().count() <= LIMIT {
        return trimmed.to_owned();
    }
    let prefix: String = trimmed.chars().take(LIMIT).collect();
    format!("{prefix}…")
}
