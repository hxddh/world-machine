use std::error::Error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use world_analyst_client::{
    AnalystTurn, AnalystTurnClientError, AnalystTurnProcess, AnalystTurnProcessConfig,
};
use world_library::{WorldDocumentId, WorldLibrary};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesktopAnalystState {
    Ready,
    Answer { turn_index: usize },
    RecoverableError { message: String },
    FatalError { message: String },
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopAnalystConfig {
    pub node_program: PathBuf,
    pub turn_host_script: PathBuf,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub thinking: Option<String>,
    pub pi_program: Option<PathBuf>,
    pub analyst_program: Option<PathBuf>,
    pub timeout_ms: Option<u64>,
}

impl DesktopAnalystConfig {
    pub fn new(turn_host_script: impl Into<PathBuf>) -> Self {
        Self {
            node_program: PathBuf::from("node"),
            turn_host_script: turn_host_script.into(),
            provider: None,
            model: None,
            thinking: None,
            pi_program: None,
            analyst_program: None,
            timeout_ms: None,
        }
    }

    fn process_config(
        &self,
        left_archive: PathBuf,
        right_archive: PathBuf,
    ) -> AnalystTurnProcessConfig {
        AnalystTurnProcessConfig {
            node_program: self.node_program.clone(),
            turn_host_script: self.turn_host_script.clone(),
            left_archive,
            right_archive,
            provider: self.provider.clone(),
            model: self.model.clone(),
            thinking: self.thinking.clone(),
            pi_program: self.pi_program.clone(),
            analyst_program: self.analyst_program.clone(),
        }
    }
}

#[derive(Debug)]
pub enum DesktopAnalystSessionError {
    SameWorld(WorldDocumentId),
    MissingWorld {
        side: &'static str,
        id: WorldDocumentId,
        path: PathBuf,
    },
    InspectWorld {
        side: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Spawn(String),
    Client(AnalystTurnClientError),
    FatalSession(String),
    Closed,
    Shutdown(String),
}

impl fmt::Display for DesktopAnalystSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameWorld(id) => {
                write!(
                    f,
                    "analyst session requires two different Worlds; both are {id}"
                )
            }
            Self::MissingWorld { side, id, path } => write!(
                f,
                "{side} analyst World {id} does not exist at {}",
                path.display()
            ),
            Self::InspectWorld { side, path, source } => write!(
                f,
                "could not inspect {side} analyst World {}: {source}",
                path.display()
            ),
            Self::Spawn(message) => write!(f, "could not start analyst session: {message}"),
            Self::Client(error) => error.fmt(f),
            Self::FatalSession(message) => {
                write!(f, "analyst session is unavailable: {message}")
            }
            Self::Closed => write!(f, "analyst session is closed"),
            Self::Shutdown(message) => {
                write!(f, "could not close analyst session cleanly: {message}")
            }
        }
    }
}

impl Error for DesktopAnalystSessionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InspectWorld { source, .. } => Some(source),
            Self::Client(error) => Some(error),
            Self::SameWorld(_)
            | Self::MissingWorld { .. }
            | Self::Spawn(_)
            | Self::FatalSession(_)
            | Self::Closed
            | Self::Shutdown(_) => None,
        }
    }
}

trait AnalystSessionProcess {
    fn ask(
        &mut self,
        prompt: &str,
        timeout_ms: Option<u64>,
    ) -> Result<AnalystTurn, AnalystTurnClientError>;

    fn shutdown(self) -> Result<(), String>;
}

impl AnalystSessionProcess for AnalystTurnProcess {
    fn ask(
        &mut self,
        prompt: &str,
        timeout_ms: Option<u64>,
    ) -> Result<AnalystTurn, AnalystTurnClientError> {
        AnalystTurnProcess::ask(self, prompt, timeout_ms)
    }

    fn shutdown(self) -> Result<(), String> {
        AnalystTurnProcess::shutdown(self)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

struct SessionCore<P: AnalystSessionProcess> {
    left: WorldDocumentId,
    right: WorldDocumentId,
    left_path: PathBuf,
    right_path: PathBuf,
    process: Option<P>,
    state: DesktopAnalystState,
    turns: Vec<AnalystTurn>,
    timeout_ms: Option<u64>,
}

impl<P: AnalystSessionProcess> SessionCore<P> {
    fn start_with<F>(
        library: &WorldLibrary,
        left: WorldDocumentId,
        right: WorldDocumentId,
        config: DesktopAnalystConfig,
        spawn: F,
    ) -> Result<Self, DesktopAnalystSessionError>
    where
        F: FnOnce(&AnalystTurnProcessConfig) -> Result<P, String>,
    {
        if left == right {
            return Err(DesktopAnalystSessionError::SameWorld(left));
        }

        let left_path = required_world_path(library, "left", &left)?;
        let right_path = required_world_path(library, "right", &right)?;
        let process_config = config.process_config(left_path.clone(), right_path.clone());
        let process = spawn(&process_config).map_err(DesktopAnalystSessionError::Spawn)?;

        Ok(Self {
            left,
            right,
            left_path,
            right_path,
            process: Some(process),
            state: DesktopAnalystState::Ready,
            turns: Vec::new(),
            timeout_ms: config.timeout_ms,
        })
    }

    fn ask(&mut self, prompt: &str) -> Result<AnalystTurn, DesktopAnalystSessionError> {
        match &self.state {
            DesktopAnalystState::FatalError { message } => {
                return Err(DesktopAnalystSessionError::FatalSession(message.clone()));
            }
            DesktopAnalystState::Closed => return Err(DesktopAnalystSessionError::Closed),
            DesktopAnalystState::Ready
            | DesktopAnalystState::Answer { .. }
            | DesktopAnalystState::RecoverableError { .. } => {}
        }

        let process = self
            .process
            .as_mut()
            .ok_or(DesktopAnalystSessionError::Closed)?;
        match process.ask(prompt, self.timeout_ms) {
            Ok(turn) => {
                self.turns.push(turn.clone());
                self.state = DesktopAnalystState::Answer {
                    turn_index: self.turns.len() - 1,
                };
                Ok(turn)
            }
            Err(error) if error.is_session_fatal() => {
                let message = error.to_string();
                let _ = self.shutdown_process();
                self.state = DesktopAnalystState::FatalError {
                    message: message.clone(),
                };
                Err(DesktopAnalystSessionError::Client(error))
            }
            Err(error) => {
                self.state = DesktopAnalystState::RecoverableError {
                    message: error.to_string(),
                };
                Err(DesktopAnalystSessionError::Client(error))
            }
        }
    }

    fn close(&mut self) -> Result<(), DesktopAnalystSessionError> {
        if matches!(self.state, DesktopAnalystState::Closed) {
            return Ok(());
        }
        let result = self.shutdown_process();
        self.state = DesktopAnalystState::Closed;
        result.map_err(DesktopAnalystSessionError::Shutdown)
    }

    fn shutdown_process(&mut self) -> Result<(), String> {
        match self.process.take() {
            Some(process) => process.shutdown(),
            None => Ok(()),
        }
    }
}

impl<P: AnalystSessionProcess> Drop for SessionCore<P> {
    fn drop(&mut self) {
        let _ = self.shutdown_process();
    }
}

pub struct DesktopAnalystSession {
    inner: SessionCore<AnalystTurnProcess>,
}

impl DesktopAnalystSession {
    pub fn start(
        library: &WorldLibrary,
        left: WorldDocumentId,
        right: WorldDocumentId,
        config: DesktopAnalystConfig,
    ) -> Result<Self, DesktopAnalystSessionError> {
        let inner = SessionCore::start_with(library, left, right, config, |process_config| {
            AnalystTurnProcess::spawn(process_config).map_err(|error| error.to_string())
        })?;
        Ok(Self { inner })
    }

    pub fn left(&self) -> &WorldDocumentId {
        &self.inner.left
    }

    pub fn right(&self) -> &WorldDocumentId {
        &self.inner.right
    }

    pub fn left_path(&self) -> &Path {
        &self.inner.left_path
    }

    pub fn right_path(&self) -> &Path {
        &self.inner.right_path
    }

    pub fn state(&self) -> &DesktopAnalystState {
        &self.inner.state
    }

    pub fn turns(&self) -> &[AnalystTurn] {
        &self.inner.turns
    }

    pub fn ask(&mut self, prompt: &str) -> Result<AnalystTurn, DesktopAnalystSessionError> {
        self.inner.ask(prompt)
    }

    pub fn close(&mut self) -> Result<(), DesktopAnalystSessionError> {
        self.inner.close()
    }
}

fn required_world_path(
    library: &WorldLibrary,
    side: &'static str,
    id: &WorldDocumentId,
) -> Result<PathBuf, DesktopAnalystSessionError> {
    let path = library.path(id);
    match path.try_exists() {
        Ok(true) => Ok(path),
        Ok(false) => Err(DesktopAnalystSessionError::MissingWorld {
            side,
            id: id.clone(),
            path,
        }),
        Err(source) => Err(DesktopAnalystSessionError::InspectWorld { side, path, source }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::fs;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_analyst_client::{AnalystRemoteError, AnalystRemoteErrorKind};

    struct FakeProcess {
        script: VecDeque<Result<AnalystTurn, AnalystTurnClientError>>,
        shutdowns: Arc<AtomicUsize>,
    }

    impl AnalystSessionProcess for FakeProcess {
        fn ask(
            &mut self,
            _prompt: &str,
            _timeout_ms: Option<u64>,
        ) -> Result<AnalystTurn, AnalystTurnClientError> {
            self.script
                .pop_front()
                .expect("fake analyst process should have a scripted result")
        }

        fn shutdown(self) -> Result<(), String> {
            self.shutdowns.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn start_binds_two_existing_library_paths_once() {
        let fixture = Fixture::new("bind");
        let shutdowns = Arc::new(AtomicUsize::new(0));
        let expected_left = fixture.library.path(&fixture.left);
        let expected_right = fixture.library.path(&fixture.right);
        let process_shutdowns = Arc::clone(&shutdowns);

        let session = SessionCore::start_with(
            &fixture.library,
            fixture.left.clone(),
            fixture.right.clone(),
            DesktopAnalystConfig::new("turn-host.mjs"),
            move |config| {
                assert_eq!(config.left_archive, expected_left);
                assert_eq!(config.right_archive, expected_right);
                Ok(FakeProcess {
                    script: VecDeque::new(),
                    shutdowns: process_shutdowns,
                })
            },
        )
        .expect("valid analyst fixture should start");

        assert_eq!(session.left, fixture.left);
        assert_eq!(session.right, fixture.right);
        assert_eq!(session.left_path, fixture.library.path(&fixture.left));
        assert_eq!(session.right_path, fixture.library.path(&fixture.right));
        assert_eq!(session.state, DesktopAnalystState::Ready);
        drop(session);
        assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn start_rejects_same_or_missing_world_before_spawn() {
        let fixture = Fixture::new("invalid");
        let same_spawned = Arc::new(AtomicUsize::new(0));
        let same_counter = Arc::clone(&same_spawned);
        let same_result = SessionCore::<FakeProcess>::start_with(
            &fixture.library,
            fixture.left.clone(),
            fixture.left.clone(),
            DesktopAnalystConfig::new("turn-host.mjs"),
            move |_| {
                same_counter.fetch_add(1, Ordering::SeqCst);
                unreachable!()
            },
        );
        let error = match same_result {
            Ok(_) => panic!("same World pair unexpectedly started"),
            Err(error) => error,
        };
        assert!(matches!(error, DesktopAnalystSessionError::SameWorld(_)));
        assert_eq!(same_spawned.load(Ordering::SeqCst), 0);

        fs::remove_file(fixture.library.path(&fixture.right)).unwrap();
        let missing_spawned = Arc::new(AtomicUsize::new(0));
        let missing_counter = Arc::clone(&missing_spawned);
        let missing_result = SessionCore::<FakeProcess>::start_with(
            &fixture.library,
            fixture.left.clone(),
            fixture.right.clone(),
            DesktopAnalystConfig::new("turn-host.mjs"),
            move |_| {
                missing_counter.fetch_add(1, Ordering::SeqCst);
                unreachable!()
            },
        );
        let error = match missing_result {
            Ok(_) => panic!("missing World unexpectedly started"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            DesktopAnalystSessionError::MissingWorld { side: "right", .. }
        ));
        assert_eq!(missing_spawned.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn successful_asks_retain_turns_and_advance_answer_state() {
        let fixture = Fixture::new("answers");
        let shutdowns = Arc::new(AtomicUsize::new(0));
        let mut session = fixture.session_with(
            vec![Ok(turn("one")), Ok(turn("two"))],
            Arc::clone(&shutdowns),
        );

        assert_eq!(session.ask("first").unwrap().text.as_deref(), Some("one"));
        assert_eq!(session.state, DesktopAnalystState::Answer { turn_index: 0 });
        assert_eq!(session.ask("second").unwrap().text.as_deref(), Some("two"));
        assert_eq!(session.turns.len(), 2);
        assert_eq!(session.state, DesktopAnalystState::Answer { turn_index: 1 });

        session.close().unwrap();
        assert_eq!(session.state, DesktopAnalystState::Closed);
        assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn nonfatal_errors_are_recoverable_and_reuse_process() {
        let fixture = Fixture::new("recoverable");
        let shutdowns = Arc::new(AtomicUsize::new(0));
        let mut session = fixture.session_with(
            vec![Err(remote_command("busy")), Ok(turn("recovered"))],
            Arc::clone(&shutdowns),
        );

        assert!(matches!(
            session.ask("first").unwrap_err(),
            DesktopAnalystSessionError::Client(AnalystTurnClientError::RemoteCommand(_))
        ));
        assert!(matches!(
            session.state,
            DesktopAnalystState::RecoverableError { .. }
        ));
        assert_eq!(shutdowns.load(Ordering::SeqCst), 0);

        assert_eq!(
            session.ask("retry").unwrap().text.as_deref(),
            Some("recovered")
        );
        assert_eq!(shutdowns.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn fatal_client_error_closes_process_and_prevents_later_ask() {
        let fixture = Fixture::new("fatal");
        let shutdowns = Arc::new(AtomicUsize::new(0));
        let mut session = fixture.session_with(
            vec![Err(remote_fatal("transport ended"))],
            Arc::clone(&shutdowns),
        );

        assert!(matches!(
            session.ask("first").unwrap_err(),
            DesktopAnalystSessionError::Client(AnalystTurnClientError::RemoteFatal(_))
        ));
        assert!(matches!(
            session.state,
            DesktopAnalystState::FatalError { .. }
        ));
        assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
        assert!(matches!(
            session.ask("again").unwrap_err(),
            DesktopAnalystSessionError::FatalSession(_)
        ));
        assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn close_is_idempotent_and_drop_does_not_shutdown_twice() {
        let fixture = Fixture::new("close");
        let shutdowns = Arc::new(AtomicUsize::new(0));
        let mut session = fixture.session_with(Vec::new(), Arc::clone(&shutdowns));

        session.close().unwrap();
        session.close().unwrap();
        assert_eq!(session.state, DesktopAnalystState::Closed);
        drop(session);
        assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn main_view_source_does_not_own_raw_analyst_process_or_pi_protocol() {
        let main = include_str!("main.rs").to_ascii_lowercase();
        for forbidden in [
            "analystturnprocess",
            "childstdin",
            "childstdout",
            "agent_settled",
            "tool_execution_start",
            "pi_program",
        ] {
            assert!(
                !main.contains(forbidden),
                "main GPUI source contains analyst process/provider token {forbidden}"
            );
        }
    }

    fn turn(text: &str) -> AnalystTurn {
        AnalystTurn {
            request_id: "world-analyst-test".into(),
            text: Some(text.into()),
            tool_calls: Vec::new(),
            runtime_errors: Vec::new(),
        }
    }

    fn remote_command(message: &str) -> AnalystTurnClientError {
        AnalystTurnClientError::RemoteCommand(AnalystRemoteError {
            kind: AnalystRemoteErrorKind::Command,
            fatal: false,
            message: message.into(),
        })
    }

    fn remote_fatal(message: &str) -> AnalystTurnClientError {
        AnalystTurnClientError::RemoteFatal(AnalystRemoteError {
            kind: AnalystRemoteErrorKind::Transport,
            fatal: true,
            message: message.into(),
        })
    }

    struct Fixture {
        root: PathBuf,
        library: WorldLibrary,
        left: WorldDocumentId,
        right: WorldDocumentId,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "world-machine-m222-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&root).unwrap();
            let library = WorldLibrary::new(root.clone());
            let left = WorldDocumentId::new("left").unwrap();
            let right = WorldDocumentId::new("right").unwrap();
            fs::write(library.path(&left), "left").unwrap();
            fs::write(library.path(&right), "right").unwrap();
            Self {
                root,
                library,
                left,
                right,
            }
        }

        fn session_with(
            &self,
            script: Vec<Result<AnalystTurn, AnalystTurnClientError>>,
            shutdowns: Arc<AtomicUsize>,
        ) -> SessionCore<FakeProcess> {
            SessionCore::start_with(
                &self.library,
                self.left.clone(),
                self.right.clone(),
                DesktopAnalystConfig::new("turn-host.mjs"),
                move |_| {
                    Ok(FakeProcess {
                        script: script.into(),
                        shutdowns,
                    })
                },
            )
            .expect("valid fake analyst session should start")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
