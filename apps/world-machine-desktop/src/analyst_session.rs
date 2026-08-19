use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use world_analyst_client::{
    AnalystTurn, AnalystTurnClientError, AnalystTurnProcess, AnalystTurnProcessConfig,
};
use world_library::{LibraryError, WorldDocumentId, WorldLibrary};

static ANALYST_SNAPSHOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
    },
    LoadWorld {
        side: &'static str,
        id: WorldDocumentId,
        source: LibraryError,
    },
    SerializeArchive {
        side: &'static str,
        id: WorldDocumentId,
        message: String,
    },
    CreateSnapshotDir {
        path: PathBuf,
        source: io::Error,
    },
    WriteSnapshot {
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
            Self::MissingWorld { side, id } => {
                write!(f, "{side} analyst World {id} does not exist")
            }
            Self::LoadWorld { side, id, source } => {
                write!(f, "could not load {side} analyst World {id}: {source}")
            }
            Self::SerializeArchive { side, id, message } => write!(
                f,
                "could not serialize {side} analyst World {id} as an archive snapshot: {message}"
            ),
            Self::CreateSnapshotDir { path, source } => write!(
                f,
                "could not create analyst snapshot directory {}: {source}",
                path.display()
            ),
            Self::WriteSnapshot { side, path, source } => write!(
                f,
                "could not write {side} analyst archive snapshot {}: {source}",
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
            Self::LoadWorld { source, .. } => Some(source),
            Self::CreateSnapshotDir { source, .. } | Self::WriteSnapshot { source, .. } => {
                Some(source)
            }
            Self::Client(error) => Some(error),
            Self::SameWorld(_)
            | Self::MissingWorld { .. }
            | Self::SerializeArchive { .. }
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

struct ArchiveSnapshotPair {
    root: PathBuf,
    left_path: PathBuf,
    right_path: PathBuf,
}

impl ArchiveSnapshotPair {
    fn capture(
        library: &WorldLibrary,
        left: &WorldDocumentId,
        right: &WorldDocumentId,
    ) -> Result<Self, DesktopAnalystSessionError> {
        let left_json = load_archive_json(library, "left", left)?;
        let right_json = load_archive_json(library, "right", right)?;
        let root = create_snapshot_root()?;
        let pair = Self {
            left_path: root.join("left.world-archive.json"),
            right_path: root.join("right.world-archive.json"),
            root,
        };
        write_private_snapshot("left", &pair.left_path, &left_json)?;
        write_private_snapshot("right", &pair.right_path, &right_json)?;
        Ok(pair)
    }
}

impl Drop for ArchiveSnapshotPair {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

struct SessionCore<P: AnalystSessionProcess> {
    left: WorldDocumentId,
    right: WorldDocumentId,
    _archives: ArchiveSnapshotPair,
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

        let archives = ArchiveSnapshotPair::capture(library, &left, &right)?;
        let process_config =
            config.process_config(archives.left_path.clone(), archives.right_path.clone());
        let process = spawn(&process_config).map_err(DesktopAnalystSessionError::Spawn)?;

        Ok(Self {
            left,
            right,
            _archives: archives,
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

fn load_archive_json(
    library: &WorldLibrary,
    side: &'static str,
    id: &WorldDocumentId,
) -> Result<String, DesktopAnalystSessionError> {
    let archive = library
        .load(id)
        .map_err(|source| DesktopAnalystSessionError::LoadWorld {
            side,
            id: id.clone(),
            source,
        })?
        .ok_or_else(|| DesktopAnalystSessionError::MissingWorld {
            side,
            id: id.clone(),
        })?;
    archive
        .to_json_pretty()
        .map_err(|error| DesktopAnalystSessionError::SerializeArchive {
            side,
            id: id.clone(),
            message: error.to_string(),
        })
}

fn create_snapshot_root() -> Result<PathBuf, DesktopAnalystSessionError> {
    let base = std::env::temp_dir();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for _ in 0..32 {
        let sequence = ANALYST_SNAPSHOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = base.join(format!(
            "world-machine-analyst-{}-{now}-{sequence}",
            std::process::id()
        ));
        match create_private_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(DesktopAnalystSessionError::CreateSnapshotDir { path, source });
            }
        }
    }

    let path = base.join(format!(
        "world-machine-analyst-{}-{now}",
        std::process::id()
    ));
    Err(DesktopAnalystSessionError::CreateSnapshotDir {
        path,
        source: io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a unique analyst snapshot directory",
        ),
    })
}

fn create_private_dir(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

fn write_private_snapshot(
    side: &'static str,
    path: &Path,
    json: &str,
) -> Result<(), DesktopAnalystSessionError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file =
        options
            .open(path)
            .map_err(|source| DesktopAnalystSessionError::WriteSnapshot {
                side,
                path: path.to_path_buf(),
                source,
            })?;
    file.write_all(json.as_bytes())
        .map_err(|source| DesktopAnalystSessionError::WriteSnapshot {
            side,
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };
    use world_analyst_client::{AnalystRemoteError, AnalystRemoteErrorKind};
    use world_persistence::{
        WorldArchive, WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION,
    };

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
    fn start_binds_raw_archive_snapshots_and_cleans_them_on_drop() {
        let fixture = Fixture::new("bind");
        let shutdowns = Arc::new(AtomicUsize::new(0));
        let captured_paths = Arc::new(Mutex::new(None));
        let process_shutdowns = Arc::clone(&shutdowns);
        let process_paths = Arc::clone(&captured_paths);
        let document_left_path = fixture.library.path(&fixture.left);
        let document_right_path = fixture.library.path(&fixture.right);
        let expected_left_archive = fixture.left_archive.clone();
        let expected_right_archive = fixture.right_archive.clone();

        let session = SessionCore::start_with(
            &fixture.library,
            fixture.left.clone(),
            fixture.right.clone(),
            DesktopAnalystConfig::new("turn-host.mjs"),
            move |config| {
                assert_ne!(config.left_archive, document_left_path);
                assert_ne!(config.right_archive, document_right_path);
                let left_json = fs::read_to_string(&config.left_archive).unwrap();
                let right_json = fs::read_to_string(&config.right_archive).unwrap();
                assert_eq!(
                    WorldArchive::from_json(&left_json).unwrap(),
                    expected_left_archive
                );
                assert_eq!(
                    WorldArchive::from_json(&right_json).unwrap(),
                    expected_right_archive
                );
                *process_paths.lock().unwrap() =
                    Some((config.left_archive.clone(), config.right_archive.clone()));
                Ok(FakeProcess {
                    script: VecDeque::new(),
                    shutdowns: process_shutdowns,
                })
            },
        )
        .expect("valid analyst fixture should start");

        assert_eq!(session.left, fixture.left);
        assert_eq!(session.right, fixture.right);
        assert_eq!(session.state, DesktopAnalystState::Ready);
        let (left_snapshot, right_snapshot) = captured_paths.lock().unwrap().clone().unwrap();
        assert!(left_snapshot.exists());
        assert!(right_snapshot.exists());
        drop(session);
        assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
        assert!(!left_snapshot.exists());
        assert!(!right_snapshot.exists());
    }

    #[test]
    fn archive_snapshots_do_not_follow_later_library_changes() {
        let fixture = Fixture::new("stable");
        let shutdowns = Arc::new(AtomicUsize::new(0));
        let mut session = fixture.session_with(Vec::new(), Arc::clone(&shutdowns));
        let snapshot_before = fs::read_to_string(&session._archives.left_path).unwrap();

        let replacement = archive("replacement", 99);
        fixture.library.save(&fixture.left, &replacement).unwrap();
        let snapshot_after = fs::read_to_string(&session._archives.left_path).unwrap();
        assert_eq!(snapshot_before, snapshot_after);
        assert_eq!(
            WorldArchive::from_json(&snapshot_after).unwrap(),
            fixture.left_archive
        );

        session.close().unwrap();
        assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn spawn_failure_cleans_archive_snapshots() {
        let fixture = Fixture::new("spawn-failure");
        let captured_paths = Arc::new(Mutex::new(None));
        let process_paths = Arc::clone(&captured_paths);
        let result = SessionCore::<FakeProcess>::start_with(
            &fixture.library,
            fixture.left.clone(),
            fixture.right.clone(),
            DesktopAnalystConfig::new("turn-host.mjs"),
            move |config| {
                *process_paths.lock().unwrap() =
                    Some((config.left_archive.clone(), config.right_archive.clone()));
                Err("spawn failed".into())
            },
        );
        assert!(matches!(result, Err(DesktopAnalystSessionError::Spawn(_))));
        let (left_snapshot, right_snapshot) = captured_paths.lock().unwrap().clone().unwrap();
        assert!(!left_snapshot.exists());
        assert!(!right_snapshot.exists());
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
        assert!(matches!(
            same_result,
            Err(DesktopAnalystSessionError::SameWorld(_))
        ));
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
        assert!(matches!(
            missing_result,
            Err(DesktopAnalystSessionError::MissingWorld { side: "right", .. })
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

    fn archive(label: &str, world_time: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new(format!("test-{label}"), "1"),
            world_time,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    struct Fixture {
        root: PathBuf,
        library: WorldLibrary,
        left: WorldDocumentId,
        right: WorldDocumentId,
        left_archive: WorldArchive,
        right_archive: WorldArchive,
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
            let left_archive = archive("left", 1);
            let right_archive = archive("right", 2);
            library.save(&left, &left_archive).unwrap();
            library.save(&right, &right_archive).unwrap();
            Self {
                root,
                library,
                left,
                right,
                left_archive,
                right_archive,
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
