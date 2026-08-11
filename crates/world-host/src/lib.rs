use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use world_integrity::{check_archive, ArchiveIntegrityError};
use world_persistence::{WorldArchive, WorldPackRef};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub trait WorldSession {
    fn pack(&self) -> WorldPackRef;
    fn snapshot(&self) -> ProjectionSnapshot;
    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError>;

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        Ok(None)
    }
}

pub type SessionFactory =
    Box<dyn Fn() -> Result<Box<dyn WorldSession>, HostError> + Send + Sync + 'static>;
pub type ArchiveOpener =
    Box<dyn Fn(&WorldArchive) -> Result<Box<dyn WorldSession>, HostError> + Send + Sync + 'static>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldDescriptor {
    pub pack: WorldPackRef,
    pub title: String,
    pub description: String,
}

pub struct WorldRegistration {
    pub descriptor: WorldDescriptor,
    factory: SessionFactory,
    opener: Option<ArchiveOpener>,
}

impl WorldRegistration {
    pub fn new(
        descriptor: WorldDescriptor,
        factory: impl Fn() -> Result<Box<dyn WorldSession>, HostError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            descriptor,
            factory: Box::new(factory),
            opener: None,
        }
    }

    pub fn with_archive_opener(
        mut self,
        opener: impl Fn(&WorldArchive) -> Result<Box<dyn WorldSession>, HostError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.opener = Some(Box::new(opener));
        self
    }

    fn create(&self) -> Result<Box<dyn WorldSession>, HostError> {
        (self.factory)()
    }

    fn open_archive(&self, archive: &WorldArchive) -> Result<Box<dyn WorldSession>, HostError> {
        let opener = self
            .opener
            .as_ref()
            .ok_or_else(|| HostError::ArchiveOpenUnsupported(self.descriptor.pack.id.clone()))?;
        opener(archive)
    }
}

#[derive(Default)]
pub struct WorldRegistry {
    registrations: BTreeMap<String, WorldRegistration>,
}

impl WorldRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, registration: WorldRegistration) -> Result<(), HostError> {
        validate_descriptor(&registration.descriptor)?;
        let id = registration.descriptor.pack.id.clone();
        if self.registrations.contains_key(&id) {
            return Err(HostError::DuplicateWorld(id));
        }
        self.registrations.insert(id, registration);
        Ok(())
    }

    pub fn descriptors(&self) -> Vec<&WorldDescriptor> {
        self.registrations
            .values()
            .map(|registration| &registration.descriptor)
            .collect()
    }

    pub fn descriptor(&self, pack_id: &str) -> Option<&WorldDescriptor> {
        self.registrations
            .get(pack_id)
            .map(|registration| &registration.descriptor)
    }

    pub fn create(&self, pack_id: &str) -> Result<Box<dyn WorldSession>, HostError> {
        self.registrations
            .get(pack_id)
            .ok_or_else(|| HostError::UnknownWorld(pack_id.into()))?
            .create()
    }

    pub fn open_archive(&self, archive: &WorldArchive) -> Result<Box<dyn WorldSession>, HostError> {
        check_archive(archive)?;
        let registration = self
            .registrations
            .get(&archive.pack.id)
            .ok_or_else(|| HostError::UnknownWorld(archive.pack.id.clone()))?;
        if registration.descriptor.pack.version != archive.pack.version {
            return Err(HostError::VersionMismatch {
                expected: registration.descriptor.pack.clone(),
                found: archive.pack.clone(),
            });
        }
        registration.open_archive(archive)
    }
}

fn validate_descriptor(descriptor: &WorldDescriptor) -> Result<(), HostError> {
    if descriptor.pack.id.trim().is_empty()
        || descriptor.pack.version.trim().is_empty()
        || descriptor.title.trim().is_empty()
    {
        return Err(HostError::InvalidDescriptor);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostError {
    InvalidDescriptor,
    DuplicateWorld(String),
    UnknownWorld(String),
    ArchiveOpenUnsupported(String),
    ArchiveIntegrity(ArchiveIntegrityError),
    VersionMismatch {
        expected: WorldPackRef,
        found: WorldPackRef,
    },
    Session(String),
}

impl HostError {
    pub fn session(error: impl fmt::Display) -> Self {
        Self::Session(error.to_string())
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor => write!(f, "world descriptor is incomplete"),
            Self::DuplicateWorld(id) => write!(f, "world is already registered: {id}"),
            Self::UnknownWorld(id) => write!(f, "unknown world: {id}"),
            Self::ArchiveOpenUnsupported(id) => {
                write!(f, "world does not support archive opening: {id}")
            }
            Self::ArchiveIntegrity(error) => {
                write!(f, "world archive failed integrity check: {error}")
            }
            Self::VersionMismatch { expected, found } => write!(
                f,
                "world version mismatch: host has {}@{}, archive requires {}@{}",
                expected.id, expected.version, found.id, found.version
            ),
            Self::Session(message) => message.fmt(f),
        }
    }
}

impl Error for HostError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ArchiveIntegrity(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ArchiveIntegrityError> for HostError {
    fn from(error: ArchiveIntegrityError) -> Self {
        Self::ArchiveIntegrity(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_projection::{ProjectionCapabilities, ProjectionCommand};

    struct MockSession {
        pack: WorldPackRef,
        count: usize,
    }

    impl WorldSession for MockSession {
        fn pack(&self) -> WorldPackRef {
            self.pack.clone()
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: format!("Mock {}", self.count),
                capabilities: ProjectionCapabilities { fork: false },
                commands: vec![ProjectionCommand {
                    id: "mock.advance".into(),
                    title: "Advance".into(),
                    detail: "Advance the mock world".into(),
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
    }

    fn registration(id: &str, version: &str) -> WorldRegistration {
        let pack = WorldPackRef::new(id, version);
        let factory_pack = pack.clone();
        WorldRegistration::new(
            WorldDescriptor {
                pack,
                title: "Mock World".into(),
                description: "A host registry test world".into(),
            },
            move || {
                Ok(Box::new(MockSession {
                    pack: factory_pack.clone(),
                    count: 0,
                }))
            },
        )
    }

    fn archive(id: &str, version: &str, world_time: u64) -> WorldArchive {
        WorldArchive {
            format: world_persistence::WORLD_ARCHIVE_FORMAT.into(),
            format_version: world_persistence::WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new(id, version),
            world_time,
            events: vec![],
            pending: vec![],
        }
    }

    #[test]
    fn registry_lists_and_creates_sessions() {
        let mut registry = WorldRegistry::new();
        registry.register(registration("mock.world", "1")).unwrap();

        assert_eq!(registry.descriptors().len(), 1);
        assert_eq!(registry.descriptors()[0].pack.id, "mock.world");

        let mut session = registry.create("mock.world").unwrap();
        assert_eq!(session.pack(), WorldPackRef::new("mock.world", "1"));
        assert_eq!(session.snapshot().title, "Mock 0");
        assert_eq!(
            session
                .handle(ProjectionIntent::InvokeCommand("mock.advance".into()))
                .unwrap()
                .title,
            "Mock 1"
        );
    }

    #[test]
    fn registry_rejects_duplicates_and_unknown_worlds() {
        let mut registry = WorldRegistry::new();
        registry.register(registration("mock.world", "1")).unwrap();

        assert!(matches!(
            registry.register(registration("mock.world", "1")),
            Err(HostError::DuplicateWorld(_))
        ));
        assert!(matches!(
            registry.create("missing.world"),
            Err(HostError::UnknownWorld(_))
        ));
        assert!(matches!(
            registry.open_archive(&archive("missing.world", "1", 0)),
            Err(HostError::UnknownWorld(_))
        ));
    }

    #[test]
    fn archive_dispatch_checks_pack_version_before_opening() {
        let mut registry = WorldRegistry::new();
        registry.register(registration("mock.world", "2")).unwrap();

        assert!(matches!(
            registry.open_archive(&archive("mock.world", "1", 0)),
            Err(HostError::VersionMismatch { .. })
        ));
    }

    #[test]
    fn archive_open_requires_an_explicit_opener() {
        let mut registry = WorldRegistry::new();
        registry.register(registration("mock.world", "1")).unwrap();

        assert!(matches!(
            registry.open_archive(&archive("mock.world", "1", 0)),
            Err(HostError::ArchiveOpenUnsupported(id)) if id == "mock.world"
        ));
    }

    #[test]
    fn archive_integrity_is_checked_before_pack_opener() {
        let mut registry = WorldRegistry::new();
        registry
            .register(registration("mock.world", "1").with_archive_opener(|_| {
                panic!("Pack opener must not receive a structurally invalid archive")
            }))
            .unwrap();
        let mut invalid = archive("mock.world", "1", 0);
        invalid.format = "broken-world-format".into();

        assert!(matches!(
            registry.open_archive(&invalid),
            Err(HostError::ArchiveIntegrity(
                ArchiveIntegrityError::UnsupportedFormat(_)
            ))
        ));
    }

    #[test]
    fn registered_archive_opener_receives_the_archive() {
        let pack = WorldPackRef::new("mock.world", "1");
        let opener_pack = pack.clone();
        let mut registry = WorldRegistry::new();
        registry
            .register(
                registration("mock.world", "1").with_archive_opener(move |archive| {
                    Ok(Box::new(MockSession {
                        pack: opener_pack.clone(),
                        count: archive.world_time as usize,
                    }))
                }),
            )
            .unwrap();

        let session = registry
            .open_archive(&archive("mock.world", "1", 7))
            .unwrap();
        assert_eq!(session.snapshot().title, "Mock 7");
        assert_eq!(session.pack(), pack);
    }
}
