mod revision;

use revision::DocumentRevision;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use world_document::{DocumentError, WorldDocument, WorldDocumentMetadata};
use world_host::{HostError, WorldRegistry, WorldSession};
use world_persistence::{PersistenceError, WorldArchive, WorldPackRef};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub const WORLD_DOCUMENT_SUFFIX: &str = ".world";
pub const LEGACY_WORLD_DOCUMENT_SUFFIX: &str = ".world.json";

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
        format!("{}{WORLD_DOCUMENT_SUFFIX}", self.0)
    }

    fn legacy_file_name(&self) -> String {
        format!("{}{LEGACY_WORLD_DOCUMENT_SUFFIX}", self.0)
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
    pub display_title: Option<String>,
    pub display_summary: Option<String>,
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

    fn legacy_path(&self, id: &WorldDocumentId) -> PathBuf {
        self.root.join(id.legacy_file_name())
    }

    pub fn contains(&self, id: &WorldDocumentId) -> Result<bool, LibraryError> {
        Ok(self.path(id).try_exists()? || self.legacy_path(id).try_exists()?)
    }

    /// Save an archive while preserving document metadata that is already
    /// attached to this Library World. New documents start with empty metadata.
    pub fn save(&self, id: &WorldDocumentId, archive: &WorldArchive) -> Result<(), LibraryError> {
        let metadata = self
            .load_document(id)?
            .map(|document| document.metadata)
            .unwrap_or_default();
        let document = WorldDocument {
            archive: archive.clone(),
            metadata,
        };
        self.save_document_with_revision(id, &document)?;
        Ok(())
    }

    pub fn save_document(
        &self,
        id: &WorldDocumentId,
        document: &WorldDocument,
    ) -> Result<(), LibraryError> {
        self.save_document_with_revision(id, document)?;
        Ok(())
    }

    fn save_document_with_revision(
        &self,
        id: &WorldDocumentId,
        document: &WorldDocument,
    ) -> Result<DocumentRevision, LibraryError> {
        let revision = write_document_file(&self.path(id), document)?;
        let _ = fs::remove_file(self.legacy_path(id));
        Ok(revision)
    }

    pub fn load(&self, id: &WorldDocumentId) -> Result<Option<WorldArchive>, LibraryError> {
        Ok(self.load_document(id)?.map(|document| document.archive))
    }

    pub fn load_document(
        &self,
        id: &WorldDocumentId,
    ) -> Result<Option<WorldDocument>, LibraryError> {
        Ok(self
            .load_document_with_revision(id)?
            .map(|(document, _revision)| document))
    }

    fn load_document_with_revision(
        &self,
        id: &WorldDocumentId,
    ) -> Result<Option<(WorldDocument, DocumentRevision)>, LibraryError> {
        for path in [self.path(id), self.legacy_path(id)] {
            match read_document_file_with_revision(&path) {
                Ok(value) => return Ok(Some(value)),
                Err(LibraryError::Io(error)) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    }

    fn current_revision(
        &self,
        id: &WorldDocumentId,
    ) -> Result<Option<DocumentRevision>, LibraryError> {
        for path in [self.path(id), self.legacy_path(id)] {
            if let Some(revision) = revision_if_exists(&path)? {
                return Ok(Some(revision));
            }
        }
        Ok(None)
    }

    fn document_modified_time(
        &self,
        id: &WorldDocumentId,
    ) -> Result<Option<SystemTime>, LibraryError> {
        for path in [self.path(id), self.legacy_path(id)] {
            match fs::metadata(path) {
                Ok(metadata) => return Ok(Some(metadata.modified()?)),
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(LibraryError::Io(error)),
            }
        }
        Ok(None)
    }

    /// List Library Worlds with the most recently persisted document first.
    /// File modification time is Library browsing metadata only; it is never
    /// written into World state or used by replay. Ties are ordered by stable
    /// document id so the result remains deterministic for equal timestamps.
    pub fn list(&self) -> Result<Vec<WorldDocumentSummary>, LibraryError> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(LibraryError::Io(error)),
        };

        let mut ids = Vec::new();
        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let file_name = entry.file_name();
            let Some(file_name) = file_name.to_str() else {
                continue;
            };
            let raw_id = if let Some(raw_id) = file_name.strip_suffix(WORLD_DOCUMENT_SUFFIX) {
                raw_id
            } else if let Some(raw_id) = file_name.strip_suffix(LEGACY_WORLD_DOCUMENT_SUFFIX) {
                raw_id
            } else {
                continue;
            };
            ids.push(WorldDocumentId::new(raw_id)?);
        }
        ids.sort();
        ids.dedup();

        let mut documents = Vec::new();
        for id in ids {
            let Some(document) = self.load_document(&id)? else {
                continue;
            };
            let modified = self.document_modified_time(&id)?.unwrap_or(UNIX_EPOCH);
            documents.push((modified, summary(id, &document)));
        }
        documents.sort_by(|(left_modified, left), (right_modified, right)| {
            right_modified
                .cmp(left_modified)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(documents
            .into_iter()
            .map(|(_modified, document)| document)
            .collect())
    }

    pub fn import_file(
        &self,
        id: WorldDocumentId,
        source: &Path,
    ) -> Result<WorldDocumentSummary, LibraryError> {
        if self.contains(&id)? {
            return Err(LibraryError::DocumentAlreadyExists(id));
        }
        let document = read_document_file(source)?;
        self.save_document(&id, &document)?;
        Ok(summary(id, &document))
    }

    pub fn export_file(
        &self,
        id: &WorldDocumentId,
        destination: &Path,
    ) -> Result<(), LibraryError> {
        if destination.try_exists()? {
            return Err(LibraryError::ExportDestinationExists(
                destination.to_path_buf(),
            ));
        }
        let document = self
            .load_document(id)?
            .ok_or_else(|| LibraryError::UnknownDocument(id.clone()))?;
        write_document_file(destination, &document)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorldDocumentTarget {
    Library(WorldDocumentId),
    File(PathBuf),
}

impl WorldDocumentTarget {
    pub fn library_document_id(&self) -> Option<&WorldDocumentId> {
        match self {
            Self::Library(id) => Some(id),
            Self::File(_) => None,
        }
    }

    pub fn file_path(&self) -> Option<&Path> {
        match self {
            Self::Library(_) => None,
            Self::File(path) => Some(path),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Library(id) => id.to_string(),
            Self::File(path) => path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
                .unwrap_or_else(|| path.display().to_string()),
        }
    }

    fn load_with_revision(
        &self,
        library: &WorldLibrary,
    ) -> Result<(WorldDocument, DocumentRevision), LibraryError> {
        match self {
            Self::Library(id) => library
                .load_document_with_revision(id)?
                .ok_or_else(|| LibraryError::UnknownDocument(id.clone())),
            Self::File(path) => read_document_file_with_revision(path),
        }
    }

    fn current_revision(
        &self,
        library: &WorldLibrary,
    ) -> Result<Option<DocumentRevision>, LibraryError> {
        match self {
            Self::Library(id) => library.current_revision(id),
            Self::File(path) => revision_if_exists(path),
        }
    }

    fn conflict_path(&self, library: &WorldLibrary) -> PathBuf {
        match self {
            Self::Library(id) => library.path(id),
            Self::File(path) => path.clone(),
        }
    }

    fn verify_revision(
        &self,
        expected: DocumentRevision,
        library: &WorldLibrary,
    ) -> Result<(), LibraryError> {
        let current = self.current_revision(library)?;
        if current == Some(expected) {
            Ok(())
        } else {
            Err(LibraryError::DocumentChanged(self.conflict_path(library)))
        }
    }

    fn persist(
        &self,
        document: &WorldDocument,
        library: &WorldLibrary,
    ) -> Result<DocumentRevision, LibraryError> {
        match self {
            Self::Library(id) => library.save_document_with_revision(id, document),
            Self::File(path) => write_document_file(path, document),
        }
    }
}

pub struct DurableWorldSession {
    target: WorldDocumentTarget,
    revision: DocumentRevision,
    metadata: WorldDocumentMetadata,
    session: Box<dyn WorldSession>,
}

impl DurableWorldSession {
    pub fn create(
        document_id: WorldDocumentId,
        pack_id: &str,
        registry: &WorldRegistry,
        library: &WorldLibrary,
    ) -> Result<Self, LibraryError> {
        if library.contains(&document_id)? {
            return Err(LibraryError::DocumentAlreadyExists(document_id));
        }
        let session = registry.create(pack_id)?;
        let snapshot = session.snapshot();
        let archive = required_archive(session.as_ref())?;
        let mut document = WorldDocument::new(archive);
        document.metadata.display_title = snapshot_display_title(&snapshot);
        document.metadata.display_summary = snapshot_display_summary(&snapshot);
        let revision = library.save_document_with_revision(&document_id, &document)?;
        Ok(Self {
            target: WorldDocumentTarget::Library(document_id),
            revision,
            metadata: document.metadata,
            session,
        })
    }

    pub fn open(
        document_id: WorldDocumentId,
        registry: &WorldRegistry,
        library: &WorldLibrary,
    ) -> Result<Self, LibraryError> {
        let (document, revision) = library
            .load_document_with_revision(&document_id)?
            .ok_or_else(|| LibraryError::UnknownDocument(document_id.clone()))?;
        let session = registry.open_archive(&document.archive)?;
        Ok(Self {
            target: WorldDocumentTarget::Library(document_id),
            revision,
            metadata: document.metadata,
            session,
        })
    }

    pub fn open_file(path: PathBuf, registry: &WorldRegistry) -> Result<Self, LibraryError> {
        let (document, revision) = read_document_file_with_revision(&path)?;
        let session = registry.open_archive(&document.archive)?;
        Ok(Self {
            target: WorldDocumentTarget::File(path),
            revision,
            metadata: document.metadata,
            session,
        })
    }

    pub fn import_file(
        document_id: WorldDocumentId,
        source: &Path,
        registry: &WorldRegistry,
        library: &WorldLibrary,
    ) -> Result<Self, LibraryError> {
        if library.contains(&document_id)? {
            return Err(LibraryError::DocumentAlreadyExists(document_id));
        }
        let document = read_document_file(source)?;
        let session = registry.open_archive(&document.archive)?;
        let revision = library.save_document_with_revision(&document_id, &document)?;
        Ok(Self {
            target: WorldDocumentTarget::Library(document_id),
            revision,
            metadata: document.metadata,
            session,
        })
    }

    pub fn target(&self) -> &WorldDocumentTarget {
        &self.target
    }

    pub fn document_id(&self) -> Option<&WorldDocumentId> {
        self.target.library_document_id()
    }

    pub fn file_path(&self) -> Option<&Path> {
        self.target.file_path()
    }

    pub fn display_name(&self) -> String {
        self.target.display_name()
    }

    pub fn pack(&self) -> WorldPackRef {
        self.session.pack()
    }

    pub fn snapshot(&self) -> ProjectionSnapshot {
        self.session.snapshot()
    }

    pub fn metadata(&self) -> &WorldDocumentMetadata {
        &self.metadata
    }

    pub fn reload(
        &mut self,
        registry: &WorldRegistry,
        library: &WorldLibrary,
    ) -> Result<ProjectionSnapshot, LibraryError> {
        let (document, revision) = self.target.load_with_revision(library)?;
        let replacement = registry.open_archive(&document.archive)?;
        let snapshot = replacement.snapshot();

        let mut metadata = document.metadata;
        if metadata.display_summary.is_none() {
            metadata.display_summary = snapshot_display_summary(&snapshot);
        }
        self.revision = revision;
        self.metadata = metadata;
        self.session = replacement;
        Ok(snapshot)
    }

    pub fn handle(
        &mut self,
        intent: ProjectionIntent,
        registry: &WorldRegistry,
        library: &WorldLibrary,
    ) -> Result<ProjectionSnapshot, LibraryError> {
        self.target.verify_revision(self.revision, library)?;

        let current_archive = required_archive(self.session.as_ref())?;
        let mut candidate = registry.open_archive(&current_archive)?;
        let snapshot = candidate.handle(intent)?;
        let next_archive = required_archive(candidate.as_ref())?;
        let mut next_metadata = self.metadata.clone();
        if let Some(title) = snapshot_display_title(&snapshot) {
            next_metadata.display_title = Some(title);
        }
        next_metadata.display_summary = snapshot_display_summary(&snapshot);
        let next_document = WorldDocument {
            archive: next_archive,
            metadata: next_metadata.clone(),
        };

        self.target.verify_revision(self.revision, library)?;
        let next_revision = self.target.persist(&next_document, library)?;

        self.revision = next_revision;
        self.metadata = next_metadata;
        self.session = candidate;
        Ok(snapshot)
    }
}

fn snapshot_display_title(snapshot: &ProjectionSnapshot) -> Option<String> {
    let title = snapshot.title.trim();
    (!title.is_empty()).then(|| title.to_owned())
}

const DISPLAY_SUMMARY_MAX_CHARS: usize = 220;

pub fn snapshot_display_summary(snapshot: &ProjectionSnapshot) -> Option<String> {
    let item = snapshot.briefing.as_ref()?.items.first()?;
    let title = normalize_summary_text(&item.title);
    let detail = normalize_summary_text(&item.detail);
    let summary = match (title.is_empty(), detail.is_empty()) {
        (true, true) => return None,
        (false, true) => title,
        (true, false) => detail,
        (false, false) if title == detail => title,
        (false, false) => format!("{title} · {detail}"),
    };
    Some(truncate_summary(summary))
}

fn normalize_summary_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_summary(value: String) -> String {
    if value.chars().count() <= DISPLAY_SUMMARY_MAX_CHARS {
        return value;
    }
    let mut compact = value
        .chars()
        .take(DISPLAY_SUMMARY_MAX_CHARS - 1)
        .collect::<String>();
    compact.push('…');
    compact
}

fn summary(id: WorldDocumentId, document: &WorldDocument) -> WorldDocumentSummary {
    WorldDocumentSummary {
        id,
        pack: document.archive.pack.clone(),
        display_title: document.metadata.display_title.clone(),
        display_summary: document.metadata.display_summary.clone(),
        world_time: document.archive.world_time,
        event_count: document.archive.events.len(),
    }
}

fn read_document_file(path: &Path) -> Result<WorldDocument, LibraryError> {
    Ok(read_document_file_with_revision(path)?.0)
}

fn read_document_file_with_revision(
    path: &Path,
) -> Result<(WorldDocument, DocumentRevision), LibraryError> {
    let json = fs::read_to_string(path)?;
    let revision = DocumentRevision::from_bytes(json.as_bytes());
    let document = WorldDocument::from_json(&json)?;
    Ok((document, revision))
}

#[cfg(test)]
fn read_archive_file(path: &Path) -> Result<WorldArchive, LibraryError> {
    Ok(read_document_file(path)?.archive)
}

fn revision_if_exists(path: &Path) -> Result<Option<DocumentRevision>, LibraryError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(DocumentRevision::from_bytes(&bytes))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(LibraryError::Io(error)),
    }
}

fn write_document_file(
    path: &Path,
    document: &WorldDocument,
) -> Result<DocumentRevision, LibraryError> {
    let json = document.to_json_pretty()?;
    let revision = DocumentRevision::from_bytes(json.as_bytes());
    atomic_write(path, json.as_bytes())?;
    Ok(revision)
}

#[cfg(test)]
fn write_archive_file(
    path: &Path,
    archive: &WorldArchive,
) -> Result<DocumentRevision, LibraryError> {
    write_document_file(path, &WorldDocument::new(archive.clone()))
}

fn required_archive(session: &dyn WorldSession) -> Result<WorldArchive, LibraryError> {
    session
        .archive()?
        .ok_or_else(|| LibraryError::ArchiveUnsupported(session.pack().id))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
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
    DocumentAlreadyExists(WorldDocumentId),
    ExportDestinationExists(PathBuf),
    DocumentChanged(PathBuf),
    ArchiveUnsupported(String),
    Io(io::Error),
    Document(DocumentError),
    Persistence(PersistenceError),
    Host(HostError),
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDocumentId(id) => write!(f, "invalid World document id: {id}"),
            Self::UnknownDocument(id) => write!(f, "unknown World document: {id}"),
            Self::DocumentAlreadyExists(id) => write!(f, "World document already exists: {id}"),
            Self::ExportDestinationExists(path) => {
                write!(f, "export destination already exists: {}", path.display())
            }
            Self::DocumentChanged(path) => write!(
                f,
                "World document changed on disk since it was opened: {}",
                path.display()
            ),
            Self::ArchiveUnsupported(pack) => {
                write!(f, "World Pack does not support durable archives: {pack}")
            }
            Self::Io(error) => error.fmt(f),
            Self::Document(error) => error.fmt(f),
            Self::Persistence(error) => error.fmt(f),
            Self::Host(error) => error.fmt(f),
        }
    }
}

impl Error for LibraryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Document(error) => Some(error),
            Self::Persistence(error) => Some(error),
            Self::Host(error) => Some(error),
            Self::InvalidDocumentId(_)
            | Self::UnknownDocument(_)
            | Self::DocumentAlreadyExists(_)
            | Self::ExportDestinationExists(_)
            | Self::DocumentChanged(_)
            | Self::ArchiveUnsupported(_) => None,
        }
    }
}

impl From<io::Error> for LibraryError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<DocumentError> for LibraryError {
    fn from(error: DocumentError) -> Self {
        Self::Document(error)
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
    use std::env;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_host::{WorldDescriptor, WorldRegistration};
    use world_persistence::{WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
    use world_projection::{
        BriefingItem, BriefingProjection, ProjectionCapabilities, ProjectionCommand,
    };

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
                briefing: Some(BriefingProjection {
                    eyebrow: "Mock".into(),
                    title: "Current mock state".into(),
                    items: vec![BriefingItem {
                        selection: None,
                        title: format!("Count {}", self.count),
                        detail: format!("Current durable count {}", self.count),
                    }],
                }),
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
    fn new_documents_use_world_extension() {
        let root = temp_root("extension");
        let library = WorldLibrary::new(root.clone());
        let id = WorldDocumentId::new("portable").unwrap();

        library.save(&id, &mock_archive(2)).unwrap();

        assert_eq!(
            library.path(&id).file_name().unwrap().to_string_lossy(),
            "portable.world"
        );
        assert!(library.path(&id).is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_world_json_is_read_and_migrated_on_next_save() {
        let root = temp_root("legacy");
        fs::create_dir_all(&root).unwrap();
        let library = WorldLibrary::new(root.clone());
        let id = WorldDocumentId::new("legacy").unwrap();
        let legacy_path = root.join("legacy.world.json");
        fs::write(&legacy_path, mock_archive(3).to_json_pretty().unwrap()).unwrap();

        assert_eq!(library.load(&id).unwrap().unwrap().world_time, 3);
        assert_eq!(library.list().unwrap().len(), 1);

        library.save(&id, &mock_archive(4)).unwrap();
        assert_eq!(library.load(&id).unwrap().unwrap().world_time, 4);
        assert!(library.path(&id).is_file());
        assert!(!legacy_path.exists());

        let _ = fs::remove_dir_all(root);
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
    fn library_lists_most_recently_persisted_world_first() {
        let root = temp_root("recent-first");
        let library = WorldLibrary::new(root.clone());
        let older = WorldDocumentId::new("older").unwrap();
        let recent = WorldDocumentId::new("recent").unwrap();

        library.save(&older, &mock_archive(1)).unwrap();
        library.save(&recent, &mock_archive(2)).unwrap();
        let older_file = File::options()
            .write(true)
            .open(library.path(&older))
            .unwrap();
        older_file
            .set_times(
                fs::FileTimes::new().set_modified(UNIX_EPOCH + std::time::Duration::from_secs(10)),
            )
            .unwrap();
        let recent_file = File::options()
            .write(true)
            .open(library.path(&recent))
            .unwrap();
        recent_file
            .set_times(
                fs::FileTimes::new().set_modified(UNIX_EPOCH + std::time::Duration::from_secs(20)),
            )
            .unwrap();

        assert_eq!(
            library
                .list()
                .unwrap()
                .iter()
                .map(|document| document.id.as_str())
                .collect::<Vec<_>>(),
            vec!["recent", "older"]
        );

        library.save(&older, &mock_archive(3)).unwrap();
        assert_eq!(
            library
                .list()
                .unwrap()
                .iter()
                .map(|document| document.id.as_str())
                .collect::<Vec<_>>(),
            vec!["older", "recent"]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn equal_library_modification_times_fall_back_to_document_id() {
        let root = temp_root("recent-tie");
        let library = WorldLibrary::new(root.clone());
        let beta = WorldDocumentId::new("beta").unwrap();
        let alpha = WorldDocumentId::new("alpha").unwrap();

        library.save(&beta, &mock_archive(2)).unwrap();
        library.save(&alpha, &mock_archive(1)).unwrap();
        let tied = UNIX_EPOCH + std::time::Duration::from_secs(30);
        for id in [&alpha, &beta] {
            File::options()
                .write(true)
                .open(library.path(id))
                .unwrap()
                .set_times(fs::FileTimes::new().set_modified(tied))
                .unwrap();
        }

        assert_eq!(
            library
                .list()
                .unwrap()
                .iter()
                .map(|document| document.id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "beta"]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn export_and_import_round_trip_a_portable_world_file() {
        let root = temp_root("portable-round-trip");
        let source_root = root.join("source");
        let target_root = root.join("target");
        let source = WorldLibrary::new(source_root);
        let target = WorldLibrary::new(target_root);
        let source_id = WorldDocumentId::new("source").unwrap();
        let imported_id = WorldDocumentId::new("imported").unwrap();
        let external = root.join("Shared World.world");
        let archive = mock_archive(11);

        source.save(&source_id, &archive).unwrap();
        source.export_file(&source_id, &external).unwrap();
        let summary = target.import_file(imported_id.clone(), &external).unwrap();

        assert_eq!(summary.id, imported_id);
        assert_eq!(summary.world_time, 11);
        assert_eq!(target.load(&summary.id).unwrap().unwrap(), archive);
        assert!(external.is_file());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn export_refuses_to_replace_an_existing_destination() {
        let root = temp_root("export-existing");
        let library = WorldLibrary::new(root.join("library"));
        let id = WorldDocumentId::new("source").unwrap();
        let destination = root.join("existing.world");
        library.save(&id, &mock_archive(1)).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(&destination, "keep me").unwrap();

        assert!(matches!(
            library.export_file(&id, &destination),
            Err(LibraryError::ExportDestinationExists(path)) if path == destination
        ));
        assert_eq!(fs::read_to_string(&destination).unwrap(), "keep me");

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
        assert_eq!(
            library.list().unwrap()[0].display_title.as_deref(),
            Some("Mock 0")
        );
        assert_eq!(
            library.list().unwrap()[0].display_summary.as_deref(),
            Some("Count 0 · Current durable count 0")
        );
        assert_eq!(session.target(), &WorldDocumentTarget::Library(id.clone()));
        session
            .handle(
                ProjectionIntent::InvokeCommand("mock.advance".into()),
                &registry,
                &library,
            )
            .unwrap();
        assert_eq!(session.snapshot().title, "Mock 1");
        assert_eq!(
            library.list().unwrap()[0].display_title.as_deref(),
            Some("Mock 1")
        );
        assert_eq!(
            library.list().unwrap()[0].display_summary.as_deref(),
            Some("Count 1 · Current durable count 1")
        );

        let reopened = DurableWorldSession::open(id, &registry, &library).unwrap();
        assert_eq!(reopened.snapshot().title, "Mock 1");
        assert_eq!(reopened.pack(), WorldPackRef::new(MOCK_PACK, "1"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn snapshot_display_summary_uses_the_first_briefing_item_and_compacts_whitespace() {
        let snapshot = ProjectionSnapshot {
            briefing: Some(BriefingProjection {
                eyebrow: "Test".into(),
                title: "Today".into(),
                items: vec![BriefingItem {
                    selection: None,
                    title: "  Ridge   Network ".into(),
                    detail: "  Routes   now   persist.  ".into(),
                }],
            }),
            ..ProjectionSnapshot::default()
        };

        assert_eq!(
            snapshot_display_summary(&snapshot).as_deref(),
            Some("Ridge Network · Routes now persist.")
        );
    }

    #[test]
    fn snapshot_display_summary_is_bounded_for_library_cards() {
        let snapshot = ProjectionSnapshot {
            briefing: Some(BriefingProjection {
                eyebrow: "Test".into(),
                title: "Today".into(),
                items: vec![BriefingItem {
                    selection: None,
                    title: "State".into(),
                    detail: "x".repeat(400),
                }],
            }),
            ..ProjectionSnapshot::default()
        };
        let summary = snapshot_display_summary(&snapshot).unwrap();

        assert_eq!(summary.chars().count(), DISPLAY_SUMMARY_MAX_CHARS);
        assert!(summary.ends_with('…'));
    }

    #[test]
    fn snapshot_display_title_ignores_blank_titles() {
        let mut snapshot = ProjectionSnapshot {
            title: "   ".into(),
            ..ProjectionSnapshot::default()
        };
        assert_eq!(snapshot_display_title(&snapshot), None);

        snapshot.title = "  A Living World  ".into();
        assert_eq!(
            snapshot_display_title(&snapshot).as_deref(),
            Some("A Living World")
        );
    }

    #[test]
    fn stale_library_session_can_reload_and_continue() {
        let root = temp_root("library-conflict");
        let library = WorldLibrary::new(root.clone());
        let registry = registry();
        let id = WorldDocumentId::new("shared").unwrap();
        let mut first =
            DurableWorldSession::create(id.clone(), MOCK_PACK, &registry, &library).unwrap();
        let mut stale = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();

        first
            .handle(
                ProjectionIntent::InvokeCommand("mock.advance".into()),
                &registry,
                &library,
            )
            .unwrap();

        assert!(matches!(
            stale.handle(
                ProjectionIntent::InvokeCommand("mock.advance".into()),
                &registry,
                &library,
            ),
            Err(LibraryError::DocumentChanged(path)) if path == library.path(&id)
        ));
        assert_eq!(stale.snapshot().title, "Mock 0");

        let reloaded = stale.reload(&registry, &library).unwrap();
        assert_eq!(reloaded.title, "Mock 1");
        stale
            .handle(
                ProjectionIntent::InvokeCommand("mock.advance".into()),
                &registry,
                &library,
            )
            .unwrap();
        assert_eq!(stale.snapshot().title, "Mock 2");
        assert_eq!(library.load(&id).unwrap().unwrap().world_time, 2);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn open_file_updates_the_original_world_document() {
        let root = temp_root("open-file");
        let external = root.join("Shared World.world");
        let library = WorldLibrary::new(root.join("library"));
        let registry = registry();
        fs::create_dir_all(&root).unwrap();
        write_archive_file(&external, &mock_archive(3)).unwrap();

        let mut session = DurableWorldSession::open_file(external.clone(), &registry).unwrap();
        assert_eq!(
            session.target(),
            &WorldDocumentTarget::File(external.clone())
        );
        assert_eq!(session.file_path(), Some(external.as_path()));
        assert!(session.document_id().is_none());
        for _ in 0..2 {
            session
                .handle(
                    ProjectionIntent::InvokeCommand("mock.advance".into()),
                    &registry,
                    &library,
                )
                .unwrap();
        }

        assert_eq!(session.snapshot().title, "Mock 5");
        assert_eq!(read_archive_file(&external).unwrap().world_time, 5);
        assert!(library.list().unwrap().is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn external_conflict_can_reload_and_continue() {
        let root = temp_root("external-conflict");
        let external = root.join("Shared World.world");
        let library = WorldLibrary::new(root.join("library"));
        let registry = registry();
        fs::create_dir_all(&root).unwrap();
        write_archive_file(&external, &mock_archive(3)).unwrap();
        let mut session = DurableWorldSession::open_file(external.clone(), &registry).unwrap();

        write_archive_file(&external, &mock_archive(9)).unwrap();

        assert!(matches!(
            session.handle(
                ProjectionIntent::InvokeCommand("mock.advance".into()),
                &registry,
                &library,
            ),
            Err(LibraryError::DocumentChanged(path)) if path == external
        ));
        assert_eq!(session.snapshot().title, "Mock 3");

        let reloaded = session.reload(&registry, &library).unwrap();
        assert_eq!(reloaded.title, "Mock 9");
        session
            .handle(
                ProjectionIntent::InvokeCommand("mock.advance".into()),
                &registry,
                &library,
            )
            .unwrap();
        assert_eq!(session.snapshot().title, "Mock 10");
        assert_eq!(read_archive_file(&external).unwrap().world_time, 10);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_reload_does_not_replace_the_live_session() {
        let root = temp_root("failed-reload");
        let external = root.join("Shared World.world");
        let library = WorldLibrary::new(root.join("library"));
        let registry = registry();
        fs::create_dir_all(&root).unwrap();
        write_archive_file(&external, &mock_archive(3)).unwrap();
        let mut session = DurableWorldSession::open_file(external.clone(), &registry).unwrap();

        let mut unsupported = mock_archive(9);
        unsupported.pack = WorldPackRef::new("world-machine.missing", "1");
        write_archive_file(&external, &unsupported).unwrap();

        assert!(matches!(
            session.reload(&registry, &library),
            Err(LibraryError::Host(HostError::UnknownWorld(_)))
        ));
        assert_eq!(session.snapshot().title, "Mock 3");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn open_file_validates_pack_without_copying_the_document() {
        let root = temp_root("open-file-unsupported");
        let external = root.join("Unsupported.world");
        fs::create_dir_all(&root).unwrap();
        let mut unsupported = mock_archive(5);
        unsupported.pack = WorldPackRef::new("world-machine.missing", "1");
        write_archive_file(&external, &unsupported).unwrap();
        let before = fs::read_to_string(&external).unwrap();

        assert!(matches!(
            DurableWorldSession::open_file(external.clone(), &registry()),
            Err(LibraryError::Host(HostError::UnknownWorld(_)))
        ));
        assert_eq!(fs::read_to_string(&external).unwrap(), before);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn durable_import_validates_pack_before_writing_to_library() {
        let root = temp_root("validated-import");
        let library = WorldLibrary::new(root.join("library"));
        let registry = registry();
        let external = root.join("incoming.world");
        let id = WorldDocumentId::new("incoming").unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(&external, mock_archive(7).to_json_pretty().unwrap()).unwrap();

        let imported =
            DurableWorldSession::import_file(id.clone(), &external, &registry, &library).unwrap();
        assert_eq!(imported.snapshot().title, "Mock 7");
        assert!(library.contains(&id).unwrap());

        let bad_external = root.join("unsupported.world");
        let bad_id = WorldDocumentId::new("unsupported").unwrap();
        let mut unsupported = mock_archive(1);
        unsupported.pack = WorldPackRef::new("world-machine.missing", "1");
        fs::write(&bad_external, unsupported.to_json_pretty().unwrap()).unwrap();

        assert!(matches!(
            DurableWorldSession::import_file(bad_id.clone(), &bad_external, &registry, &library),
            Err(LibraryError::Host(HostError::UnknownWorld(_)))
        ));
        assert!(!library.contains(&bad_id).unwrap());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn duplicate_document_creation_is_rejected() {
        let root = temp_root("duplicate");
        let library = WorldLibrary::new(root.clone());
        let registry = registry();
        let id = WorldDocumentId::new("same").unwrap();
        DurableWorldSession::create(id.clone(), MOCK_PACK, &registry, &library).unwrap();

        assert!(matches!(
            DurableWorldSession::create(id.clone(), MOCK_PACK, &registry, &library),
            Err(LibraryError::DocumentAlreadyExists(existing)) if existing == id
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_save_does_not_commit_the_candidate_session() {
        let root = temp_root("failed-save");
        let library = WorldLibrary::new(root.clone());
        let registry = registry();
        let id = WorldDocumentId::new("mock-document").unwrap();
        let mut session = DurableWorldSession::create(id, MOCK_PACK, &registry, &library).unwrap();

        fs::remove_dir_all(&root).unwrap();
        File::create(&root).unwrap();

        assert!(matches!(
            session.handle(
                ProjectionIntent::InvokeCommand("mock.advance".into()),
                &registry,
                &library,
            ),
            Err(LibraryError::DocumentChanged(_)) | Err(LibraryError::Io(_))
        ));
        assert_eq!(session.snapshot().title, "Mock 0");

        let _ = fs::remove_file(root);
    }

    #[test]
    fn failed_external_save_does_not_commit_the_candidate_session() {
        let root = temp_root("failed-external-save");
        let external_dir = root.join("external");
        let external = external_dir.join("portable.world");
        let library = WorldLibrary::new(root.join("library"));
        let registry = registry();
        fs::create_dir_all(&external_dir).unwrap();
        write_archive_file(&external, &mock_archive(2)).unwrap();
        let mut session = DurableWorldSession::open_file(external.clone(), &registry).unwrap();

        fs::remove_file(&external).unwrap();
        fs::remove_dir(&external_dir).unwrap();
        File::create(&external_dir).unwrap();

        assert!(matches!(
            session.handle(
                ProjectionIntent::InvokeCommand("mock.advance".into()),
                &registry,
                &library,
            ),
            Err(LibraryError::DocumentChanged(_)) | Err(LibraryError::Io(_))
        ));
        assert_eq!(session.snapshot().title, "Mock 2");

        let _ = fs::remove_file(external_dir);
        let _ = fs::remove_dir_all(root);
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
}
