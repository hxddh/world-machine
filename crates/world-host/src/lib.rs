use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use world_integrity::{check_archive, ArchiveIntegrityError};
use world_persistence::{WorldArchive, WorldPackRef};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub trait WorldSession {
    fn pack(&self) -> WorldPackRef;
    fn snapshot(&self) -> ProjectionSnapshot;
    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError>;

    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
        let _ = periods;
        Ok(self.snapshot())
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        Ok(None)
    }
}

struct IntegrityCheckedSession {
    inner: Box<dyn WorldSession>,
}

impl IntegrityCheckedSession {
    fn new(inner: Box<dyn WorldSession>) -> Self {
        Self { inner }
    }
}

impl WorldSession for IntegrityCheckedSession {
    fn pack(&self) -> WorldPackRef {
        self.inner.pack()
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        self.inner.snapshot()
    }

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
        self.inner.handle(intent)
    }

    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
        self.inner.advance_background(periods)
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        let archive = self.inner.archive()?;
        if let Some(archive) = archive.as_ref() {
            check_archive(archive)?;
        }
        Ok(archive)
    }
}

fn integrity_checked(session: Box<dyn WorldSession>) -> Box<dyn WorldSession> {
    Box::new(IntegrityCheckedSession::new(session))
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

/// Supplies one coherent set of Pack registrations to a Host registry.
///
/// Sources only describe registrations. Filesystem discovery, process launch,
/// trust policy, and code loading belong outside the Host and can be added by
/// future source implementations without changing registry semantics.
pub trait WorldPackSource {
    fn registrations(&self) -> Result<Vec<WorldRegistration>, HostError>;
}

struct WorldFamily {
    active_version: String,
    registrations: BTreeMap<String, WorldRegistration>,
}

impl WorldFamily {
    fn new(version: String, registration: WorldRegistration) -> Self {
        let mut registrations = BTreeMap::new();
        registrations.insert(version.clone(), registration);
        Self {
            active_version: version,
            registrations,
        }
    }

    fn active(&self) -> &WorldRegistration {
        self.registrations
            .get(&self.active_version)
            .expect("WorldFamily active version must remain registered")
    }

    fn version(&self, version: &str) -> Option<&WorldRegistration> {
        self.registrations.get(version)
    }
}

#[derive(Default)]
pub struct WorldRegistry {
    families: BTreeMap<String, WorldFamily>,
}

impl WorldRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one Pack version and make it the active version used for new Worlds.
    /// Older registered versions remain available for exact archive restoration.
    pub fn register(&mut self, registration: WorldRegistration) -> Result<(), HostError> {
        self.register_batch([registration])
    }

    /// Install all registrations produced by one source as a single transaction.
    /// A source failure or any invalid/duplicate registration leaves the registry
    /// completely unchanged.
    pub fn install_source<S>(&mut self, source: &S) -> Result<(), HostError>
    where
        S: WorldPackSource + ?Sized,
    {
        let registrations = source.registrations()?;
        self.register_batch(registrations)
    }

    /// Atomically register a batch in iteration order. If several versions of
    /// one Pack id are present, the last version in the batch becomes active.
    /// Version strings remain opaque and are never semver-sorted by the Host.
    pub fn register_batch(
        &mut self,
        registrations: impl IntoIterator<Item = WorldRegistration>,
    ) -> Result<(), HostError> {
        let registrations = registrations.into_iter().collect::<Vec<_>>();
        let mut incoming = BTreeSet::new();

        for registration in &registrations {
            validate_descriptor(&registration.descriptor)?;
            let id = registration.descriptor.pack.id.clone();
            let version = registration.descriptor.pack.version.clone();
            let key = (id.clone(), version.clone());

            if !incoming.insert(key) || self.contains_exact(&id, &version) {
                return Err(HostError::DuplicateWorld(format!("{id}@{version}")));
            }
        }

        for registration in registrations {
            self.insert_validated(registration);
        }
        Ok(())
    }

    fn contains_exact(&self, id: &str, version: &str) -> bool {
        self.families
            .get(id)
            .is_some_and(|family| family.registrations.contains_key(version))
    }

    fn insert_validated(&mut self, registration: WorldRegistration) {
        let id = registration.descriptor.pack.id.clone();
        let version = registration.descriptor.pack.version.clone();

        if let Some(family) = self.families.get_mut(&id) {
            family.registrations.insert(version.clone(), registration);
            family.active_version = version;
        } else {
            self.families
                .insert(id, WorldFamily::new(version, registration));
        }
    }

    /// Active descriptors only. Historical compatible versions stay hidden from
    /// ordinary World creation/catalog surfaces.
    pub fn descriptors(&self) -> Vec<&WorldDescriptor> {
        self.families
            .values()
            .map(|family| &family.active().descriptor)
            .collect()
    }

    /// The active descriptor used for new Worlds with this Pack id.
    pub fn descriptor(&self, pack_id: &str) -> Option<&WorldDescriptor> {
        self.families
            .get(pack_id)
            .map(|family| &family.active().descriptor)
    }

    /// An exact registered Pack descriptor, including a historical compatible version.
    pub fn descriptor_for(&self, pack: &WorldPackRef) -> Option<&WorldDescriptor> {
        self.families
            .get(&pack.id)
            .and_then(|family| family.version(&pack.version))
            .map(|registration| &registration.descriptor)
    }

    /// All registered descriptors for a Pack id. Ordering is deterministic but
    /// version strings are opaque and are not interpreted as semantic versions.
    pub fn descriptors_for(&self, pack_id: &str) -> Vec<&WorldDescriptor> {
        self.families
            .get(pack_id)
            .map(|family| {
                family
                    .registrations
                    .values()
                    .map(|registration| &registration.descriptor)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Explicitly choose which already-registered version is used for new Worlds.
    pub fn activate(&mut self, pack: &WorldPackRef) -> Result<(), HostError> {
        let family = self
            .families
            .get_mut(&pack.id)
            .ok_or_else(|| HostError::UnknownWorld(pack.id.clone()))?;
        if !family.registrations.contains_key(&pack.version) {
            return Err(HostError::VersionMismatch {
                expected: family.active().descriptor.pack.clone(),
                found: pack.clone(),
            });
        }
        family.active_version = pack.version.clone();
        Ok(())
    }

    pub fn create(&self, pack_id: &str) -> Result<Box<dyn WorldSession>, HostError> {
        let family = self
            .families
            .get(pack_id)
            .ok_or_else(|| HostError::UnknownWorld(pack_id.into()))?;
        let session = family.active().create()?;
        Ok(integrity_checked(session))
    }

    pub fn create_exact(&self, pack: &WorldPackRef) -> Result<Box<dyn WorldSession>, HostError> {
        let family = self
            .families
            .get(&pack.id)
            .ok_or_else(|| HostError::UnknownWorld(pack.id.clone()))?;
        let registration =
            family
                .version(&pack.version)
                .ok_or_else(|| HostError::VersionMismatch {
                    expected: family.active().descriptor.pack.clone(),
                    found: pack.clone(),
                })?;
        let session = registration.create()?;
        Ok(integrity_checked(session))
    }

    pub fn open_archive(&self, archive: &WorldArchive) -> Result<Box<dyn WorldSession>, HostError> {
        check_archive(archive)?;
        let family = self
            .families
            .get(&archive.pack.id)
            .ok_or_else(|| HostError::UnknownWorld(archive.pack.id.clone()))?;
        let registration =
            family
                .version(&archive.pack.version)
                .ok_or_else(|| HostError::VersionMismatch {
                    expected: family.active().descriptor.pack.clone(),
                    found: archive.pack.clone(),
                })?;
        let session = registration.open_archive(archive)?;
        Ok(integrity_checked(session))
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
    PackSource(String),
    VersionMismatch {
        expected: WorldPackRef,
        found: WorldPackRef,
    },
    Session(String),
}

impl HostError {
    pub fn pack_source(error: impl fmt::Display) -> Self {
        Self::PackSource(error.to_string())
    }

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
            Self::PackSource(message) => write!(f, "world Pack source failed: {message}"),
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

        fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
            self.count = self
                .count
                .checked_add(periods as usize)
                .ok_or_else(|| HostError::Session("mock background overflow".into()))?;
            Ok(self.snapshot())
        }
    }

    struct InvalidArchiveSession;

    impl WorldSession for InvalidArchiveSession {
        fn pack(&self) -> WorldPackRef {
            WorldPackRef::new("broken.world", "1")
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot::default()
        }

        fn handle(&mut self, _intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            Err(HostError::Session(
                "unsupported invalid session intent".into(),
            ))
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            let mut archive = archive("broken.world", "1", 0);
            archive.format = "broken-world-format".into();
            Ok(Some(archive))
        }
    }

    struct StaticSource {
        registrations: Vec<(&'static str, &'static str)>,
    }

    impl WorldPackSource for StaticSource {
        fn registrations(&self) -> Result<Vec<WorldRegistration>, HostError> {
            Ok(self
                .registrations
                .iter()
                .map(|(id, version)| registration(id, version))
                .collect())
        }
    }

    struct FailingSource;

    impl WorldPackSource for FailingSource {
        fn registrations(&self) -> Result<Vec<WorldRegistration>, HostError> {
            Err(HostError::pack_source("source unavailable"))
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

    fn invalid_registration() -> WorldRegistration {
        WorldRegistration::new(
            WorldDescriptor {
                pack: WorldPackRef::new("invalid.world", ""),
                title: "Invalid World".into(),
                description: "A deliberately invalid descriptor".into(),
            },
            || {
                Ok(Box::new(MockSession {
                    pack: WorldPackRef::new("invalid.world", ""),
                    count: 0,
                }))
            },
        )
    }

    fn openable_registration(id: &str, version: &str) -> WorldRegistration {
        let opener_pack = WorldPackRef::new(id, version);
        registration(id, version).with_archive_opener(move |archive| {
            Ok(Box::new(MockSession {
                pack: opener_pack.clone(),
                count: archive.world_time as usize,
            }))
        })
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
        assert_eq!(session.advance_background(2).unwrap().title, "Mock 3");
    }

    #[test]
    fn source_installs_multiple_pack_families() {
        let source = StaticSource {
            registrations: vec![("alpha.world", "1"), ("beta.world", "1")],
        };
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();

        assert!(registry.descriptor("alpha.world").is_some());
        assert!(registry.descriptor("beta.world").is_some());
    }

    #[test]
    fn source_order_selects_active_version_without_interpreting_version_strings() {
        let source = StaticSource {
            registrations: vec![("mock.world", "2027"), ("mock.world", "10")],
        };
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();

        assert_eq!(registry.descriptors().len(), 1);
        assert_eq!(
            registry.descriptor("mock.world").unwrap().pack.version,
            "10"
        );
        assert!(registry
            .descriptor_for(&WorldPackRef::new("mock.world", "2027"))
            .is_some());
    }

    #[test]
    fn source_install_is_atomic_against_existing_duplicates() {
        let mut registry = WorldRegistry::new();
        registry
            .register(registration("existing.world", "1"))
            .unwrap();
        let source = StaticSource {
            registrations: vec![("new.world", "1"), ("existing.world", "1")],
        };

        assert!(matches!(
            registry.install_source(&source),
            Err(HostError::DuplicateWorld(id)) if id == "existing.world@1"
        ));
        assert!(registry.descriptor("new.world").is_none());
        assert!(registry.descriptor("existing.world").is_some());
    }

    #[test]
    fn batch_install_is_atomic_for_internal_duplicate_or_invalid_descriptor() {
        let mut registry = WorldRegistry::new();
        assert!(matches!(
            registry.register_batch([
                registration("new.world", "1"),
                registration("new.world", "1")
            ]),
            Err(HostError::DuplicateWorld(id)) if id == "new.world@1"
        ));
        assert!(registry.descriptor("new.world").is_none());

        assert!(matches!(
            registry.register_batch([registration("valid.world", "1"), invalid_registration()]),
            Err(HostError::InvalidDescriptor)
        ));
        assert!(registry.descriptor("valid.world").is_none());
    }

    #[test]
    fn failing_source_leaves_registry_unchanged() {
        let mut registry = WorldRegistry::new();
        registry.register(registration("keep.world", "1")).unwrap();

        assert!(matches!(
            registry.install_source(&FailingSource),
            Err(HostError::PackSource(message)) if message == "source unavailable"
        ));
        assert!(registry.descriptor("keep.world").is_some());
        assert_eq!(registry.descriptors().len(), 1);
    }

    #[test]
    fn pack_versions_coexist_and_latest_registration_is_active() {
        let mut registry = WorldRegistry::new();
        registry
            .register(openable_registration("mock.world", "1"))
            .unwrap();
        registry
            .register(openable_registration("mock.world", "2"))
            .unwrap();

        assert_eq!(registry.descriptors().len(), 1);
        assert_eq!(registry.descriptor("mock.world").unwrap().pack.version, "2");
        assert_eq!(registry.descriptors_for("mock.world").len(), 2);
        assert!(registry
            .descriptor_for(&WorldPackRef::new("mock.world", "1"))
            .is_some());

        assert_eq!(
            registry.create("mock.world").unwrap().pack(),
            WorldPackRef::new("mock.world", "2")
        );
        assert_eq!(
            registry
                .create_exact(&WorldPackRef::new("mock.world", "1"))
                .unwrap()
                .pack(),
            WorldPackRef::new("mock.world", "1")
        );

        let restored_v1 = registry
            .open_archive(&archive("mock.world", "1", 7))
            .unwrap();
        assert_eq!(restored_v1.pack(), WorldPackRef::new("mock.world", "1"));
        assert_eq!(restored_v1.snapshot().title, "Mock 7");
    }

    #[test]
    fn active_pack_version_can_be_selected_explicitly() {
        let mut registry = WorldRegistry::new();
        registry.register(registration("mock.world", "1")).unwrap();
        registry.register(registration("mock.world", "2")).unwrap();
        registry
            .activate(&WorldPackRef::new("mock.world", "1"))
            .unwrap();

        assert_eq!(registry.descriptor("mock.world").unwrap().pack.version, "1");
        assert_eq!(
            registry.create("mock.world").unwrap().pack(),
            WorldPackRef::new("mock.world", "1")
        );
    }

    #[test]
    fn registry_rejects_exact_duplicates_and_unknown_worlds() {
        let mut registry = WorldRegistry::new();
        registry.register(registration("mock.world", "1")).unwrap();
        registry.register(registration("mock.world", "2")).unwrap();

        assert!(matches!(
            registry.register(registration("mock.world", "2")),
            Err(HostError::DuplicateWorld(id)) if id == "mock.world@2"
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
    fn registry_sessions_reject_structurally_invalid_archive_output() {
        let registration = WorldRegistration::new(
            WorldDescriptor {
                pack: WorldPackRef::new("broken.world", "1"),
                title: "Broken World".into(),
                description: "Archive output gate regression".into(),
            },
            || Ok(Box::new(InvalidArchiveSession)),
        );
        let mut registry = WorldRegistry::new();
        registry.register(registration).unwrap();

        let session = registry.create("broken.world").unwrap();
        assert!(matches!(
            session.archive(),
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
