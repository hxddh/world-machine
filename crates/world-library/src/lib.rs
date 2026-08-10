use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use world_host::{HostError, WorldRegistry, WorldSession};
use world_persistence::{PersistenceError, WorldArchive, WorldPackRef};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

const WORLD_FILE_SUFFIX: &str = ".world.json";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WorldDocumentId(String);

impl WorldDocumentId {
    pub fn new(value: impl Into<String>) -> Result<Self, LibraryError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 128
            && value != "."
            && value != ".."
            && value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'));
        if !valid {
            return Err(LibraryError::InvalidDocumentId(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn file_name(&self) -> String {
        format!("{}{WORLD_FILE_SUFFIX}", self.0)
    }
}

impl fmt::Display for WorldDocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldDocumentSummary {
    pub id: WorldDocumentId,
    pub pack: WorldPackRef,
    pub world_time: u64,
    pub event_count: usize,
}

#[derive(Clone, Debug)]
pub struct WorldLibrary {
    root: PathBuf,
}

impl WorldLibrary {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, id: &WorldDocumentId) -> PathBuf {
        self.root.join(id.file_name())
    }

    pub fn save(&self, id: &WorldDocumentId, archive: &WorldArchive) -> Result<(), LibraryError> {
        let json = archive.to_json_pretty()?;
        atomic_write(&self.path(id), json.as_bytes())?;
        Ok(())
    }

    pub fn load(&self, id: &WorldDocumentId) -> Result<Option<WorldArchive>, LibraryError> {
        let path = self.path(id);
        let json = match fs::read_to_string(path) {
            Ok(json) => json,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(LibraryError::Io(error)),
        };
        Ok(Some(WorldArchive::from_json(&json)?))
    }

    pub fn list(&self) -> Result<Vec<WorldDocumentSummary>, LibraryError> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(LibraryError::Io(error)),
        };

        let mut documents = Vec::new();
        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let file_name = entry.file_name();
            let Some(file_name) = file_name.to_str() else {
                continue;
            };
            let Some(raw_id) = file_name.strip_suffix(WORLD_FILE_SUFFIX) else {
                continue;
            };
            let id = WorldDocumentId::new(raw_id)?;
            let Some(archive) = self.load(&id)? else {
                continue;
            };
            documents.push(WorldDocumentSummary {
                id,
                pack: archive.pack.clone(),
                world_time: archive.world_time,
                event_count: archive.events.len(),
            });
        }
        documents.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(documents)
    }
}

pub struct DurableWorldSession {
    document_id: WorldDocumentId,
    session: Box<dyn WorldSession>,
}

impl DurableWorldSession {
    pub fn create(
        document_id: WorldDocumentId,
        pack_id: &str,
        registry: &WorldRegistry,
        library: &WorldLibrary,
    ) -> Result<Self, LibraryError> {
        let session = registry.create(pack_id)?;
        let archive = required_archive(session.as_ref())?;
        library.save(&document_id, &archive)?;
        Ok(Self {
            document_id,
            session,
        })
    }

    pub fn open(
        document_id: WorldDocumentId,
        registry: &WorldRegistry,
        library: &WorldLibrary,
    ) -> Result<Self, LibraryError> {
        let archive = library
            .load(&document_id)?
            .ok_or_else(|| LibraryError::UnknownDocument(document_id.clone()))?;
        let session = registry.open_archive(&archive)?;
        Ok(Self {
            document_id,
            session,
        })
    }

    pub fn document_id(&self) -> &WorldDocumentId {
        &self.document_id
    }

    pub fn pack(&self) -> WorldPackRef {
        self.session.pack()
    }

    pub fn snapshot(&self) -> ProjectionSnapshot {
        self.session.snapshot()
    }

    pub fn handle(
        &mut self,
        intent: ProjectionIntent,
        registry: &WorldRegistry,
        library: &WorldLibrary,
    ) -> Result<ProjectionSnapshot, LibraryError> {
        let current_archive = required_archive(self.session.as_ref())?;
        let mut candidate = registry.open_archive(&current_archive)?;
        let snapshot = candidate.handle(intent)?;
        let next_archive = required_archive(candidate.as_ref())?;
        library.save(&self.document_id, &next_archive)?;
        self.session = candidate;
        Ok(snapshot)
    }
}

fn required_archive(session: &dyn WorldSession) -> Result<WorldArchive, LibraryError> {
    session
        .archive()?
        .ok_or_else(|| LibraryError::ArchiveUnsupported(session.pack().id))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("world document path has no parent directory"))?;
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::other("world document path has no file name"))?
        .to_string_lossy();
    let temp_path = path.with_file_name(format!(".{file_name}.tmp"));
    let mut file = File::create(&temp_path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(temp_path, path)?;
    Ok(())
}

#[derive(Debug)]
pub enum LibraryError {
    InvalidDocumentId(String),
    UnknownDocument(WorldDocumentId),
    ArchiveUnsupported(String),
    Io(io::Error),
    Persistence(PersistenceError),
    Host(HostError),
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDocumentId(id) => write!(f, "invalid World document id: {id}"),
            Self::UnknownDocument(id) => write!(f, "unknown World document: {id}"),
            Self::ArchiveUnsupported(pack) => {
                write!(f, "World Pack does not support durable archives: {pack}")
            }
            Self::Io(error) => error.fmt(f),
            Self::Persistence(error) => error.fmt(f),
            Self::Host(error) => error.fmt(f),
        }
    }
}

impl Error for LibraryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Persistence(error) => Some(error),
            Self::Host(error) => Some(error),
            Self::InvalidDocumentId(_)
            | Self::UnknownDocument(_)
            | Self::ArchiveUnsupported(_) => None,
        }
    }
}

impl From<io::Error> for LibraryError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<PersistenceError> for LibraryError {
    fn from(error: PersistenceError) -> Self {
        Self::Persistence(error)
    }
}

impl From<HostError> for LibraryError {
    fn from(error: HostError) -> Self {
        Self::Host(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::env;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_host::{WorldDescriptor, WorldRegistration};
    use world_persistence::{WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
    use world_projection::{ProjectionCapabilities, ProjectionCommand};

    const MOCK_PACK: &str = "world-machine.mock";

    struct MockSession {
        count: u64,
    }

    impl WorldSession for MockSession {
        fn pack(&self) -> WorldPackRef {
            WorldPackRef::new(MOCK_PACK, "1")
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: format!("Mock {}", self.count),
                world_time: self.count,
                capabilities: ProjectionCapabilities { fork: false },
                commands: vec![ProjectionCommand {
                    id: "mock.advance".into(),
                    title: "Advance".into(),
                    detail: "Advance the mock World".into(),
                }],
                ..ProjectionSnapshot::default()
            }
        }

        fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            match intent {
                ProjectionIntent::InvokeCommand(command) if command == "mock.advance" => {
                    self.count += 1;
                    Ok(self.snapshot())
                }
                _ => Err(HostError::Session("unsupported mock intent".into())),
            }
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            Ok(Some(mock_archive(self.count)))
        }
    }

    fn mock_archive(count: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new(MOCK_PACK, "1"),
            world_time: count,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn registry() -> WorldRegistry {
        let mut registry = WorldRegistry::new();
        registry
            .register(
                WorldRegistration::new(
                    WorldDescriptor {
                        pack: WorldPackRef::new(MOCK_PACK, "1"),
                        title: "Mock World".into(),
                        description: "Durable session test".into(),
                    },
                    || Ok(Box::new(MockSession { count: 0 })),
                )
                .with_archive_opener(|archive| {
                    Ok(Box::new(MockSession {
                        count: archive.world_time,
                    }))
                }),
            )
            .unwrap();
        registry
    }

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "world-machine-library-{}-{nonce}-{label}",
            process::id()
        ))
    }

    #[test]
    fn document_ids_reject_path_like_values() {
        assert!(WorldDocumentId::new("tiny-society-1").is_ok());
        for invalid in ["", ".", "..", "../escape", "has space", "a/b"] {
            assert!(matches!(
                WorldDocumentId::new(invalid),
                Err(LibraryError::InvalidDocumentId(_))
            ));
        }
    }

    #[test]
    fn library_saves_loads_and_lists_archives() {
        let root = temp_root("round-trip");
        let library = WorldLibrary::new(root.clone());
        let first = WorldDocumentId::new("first").unwrap();
        let second = WorldDocumentId::new("second").unwrap();

        library.save(&second, &mock_archive(9)).unwrap();
        library.save(&first, &mock_archive(4)).unwrap();

        assert_eq!(library.load(&first).unwrap().unwrap().world_time, 4);
        let documents = library.list().unwrap();
        assert_eq!(
            documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
        assert_eq!(documents[1].world_time, 9);
        assert_eq!(documents[1].event_count, 0);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn durable_session_round_trips_through_the_registry() {
        let root = temp_root("durable");
        let library = WorldLibrary::new(root.clone());
        let registry = registry();
        let id = WorldDocumentId::new("mock-document").unwrap();

        let mut session =
            DurableWorldSession::create(id.clone(), MOCK_PACK, &registry, &library).unwrap();
        assert_eq!(session.snapshot().title, "Mock 0");
        session
            .handle(
                ProjectionIntent::InvokeCommand("mock.advance".into()),
                &registry,
                &library,
            )
            .unwrap();
        assert_eq!(session.snapshot().title, "Mock 1");

        let reopened = DurableWorldSession::open(id, &registry, &library).unwrap();
        assert_eq!(reopened.snapshot().title, "Mock 1");
        assert_eq!(reopened.pack(), WorldPackRef::new(MOCK_PACK, "1"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_save_does_not_commit_the_candidate_session() {
        let root = temp_root("failed-save");
        let library = WorldLibrary::new(root.clone());
        let registry = registry();
        let id = WorldDocumentId::new("mock-document").unwrap();
        let mut session =
            DurableWorldSession::create(id, MOCK_PACK, &registry, &library).unwrap();

        fs::remove_dir_all(&root).unwrap();
        File::create(&root).unwrap();

        assert!(matches!(
            session.handle(
                ProjectionIntent::InvokeCommand("mock.advance".into()),
                &registry,
                &library,
            ),
            Err(LibraryError::Io(_))
        ));
        assert_eq!(session.snapshot().title, "Mock 0");

        let _ = fs::remove_file(root);
    }

    #[test]
    fn unknown_documents_are_reported() {
        let root = temp_root("missing");
        let library = WorldLibrary::new(root.clone());
        let registry = registry();
        let id = WorldDocumentId::new("missing").unwrap();

        assert!(matches!(
            DurableWorldSession::open(id, &registry, &library),
            Err(LibraryError::UnknownDocument(_))
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unrelated_files_are_ignored_when_listing() {
        let root = temp_root("unrelated");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("notes.txt"), "not a World").unwrap();
        let library = WorldLibrary::new(root.clone());

        assert_eq!(library.list().unwrap(), Vec::new());

        let _ = fs::remove_dir_all(root);
    }

    #[allow(dead_code)]
    fn _keep_btreemap_import_for_future_archive_metadata(_: BTreeMap<String, String>) {}
}
