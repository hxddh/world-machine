use crate::{PiRpcEventParser, PiRpcProtocolError};
use serde_json::json;
use std::error::Error;
use std::fmt;
use std::io::{Read, Write as _};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub trait PiRpcTransport {
    fn complete(&mut self, prompt: &str) -> Result<String, PiRpcTransportError>;
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
            Self::ReaderPanicked | Self::Timeout { .. } | Self::NonZeroExit { .. } => None,
        }
    }
}

pub struct ProcessPiRpcTransport {
    command: PiCommand,
    request_sequence: u64,
    timeout: Duration,
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

impl PiRpcTransport for ProcessPiRpcTransport {
    fn complete(&mut self, prompt: &str) -> Result<String, PiRpcTransportError> {
        let request_id = format!("world-machine-{}", self.request_sequence);
        self.request_sequence += 1;

        let mut child = Command::new(&self.command.program)
            .args(&self.command.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(PiRpcTransportError::Spawn)?;

        let stdout = child.stdout.take().ok_or_else(|| {
            PiRpcTransportError::ReadOutput(std::io::Error::other("Pi stdout was not piped"))
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            PiRpcTransportError::ReadOutput(std::io::Error::other("Pi stderr was not piped"))
        })?;
        let stdout_reader = thread::spawn(move || read_all(stdout));
        let stderr_reader = thread::spawn(move || read_all(stderr));

        let request = json!({
            "id": request_id,
            "type": "prompt",
            "message": prompt,
        });
        {
            let stdin = child.stdin.as_mut().ok_or_else(|| {
                PiRpcTransportError::Stdin(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "Pi stdin was not piped",
                ))
            })?;
            writeln!(stdin, "{request}").map_err(PiRpcTransportError::Stdin)?;
        }
        drop(child.stdin.take());

        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().map_err(PiRpcTransportError::Poll)? {
                break status;
            }
            if started.elapsed() >= self.timeout {
                let _ = child.kill();
                let _ = child.wait();
                let _ = join_reader(stdout_reader);
                let _ = join_reader(stderr_reader);
                return Err(PiRpcTransportError::Timeout {
                    millis: self.timeout.as_millis(),
                });
            }
            thread::sleep(Duration::from_millis(10));
        };

        let stdout = join_reader(stdout_reader)?;
        let stderr = join_reader(stderr_reader)?;
        if !status.success() {
            return Err(PiRpcTransportError::NonZeroExit {
                code: status.code(),
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
            });
        }

        parse_stdout(&stdout).map_err(PiRpcTransportError::Protocol)
    }
}

fn read_all(mut reader: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn join_reader(
    reader: thread::JoinHandle<std::io::Result<Vec<u8>>>,
) -> Result<Vec<u8>, PiRpcTransportError> {
    reader
        .join()
        .map_err(|_| PiRpcTransportError::ReaderPanicked)?
        .map_err(PiRpcTransportError::ReadOutput)
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
