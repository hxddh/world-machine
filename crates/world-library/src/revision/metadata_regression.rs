use crate::{DurableWorldSession, LibraryError, WorldDocumentId, WorldLibrary};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use world_document::{
    WorldBranchCause, WorldDocument, WorldDocumentMetadata, WorldLineage, WorldParent,
};
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};
use world_persistence::{WorldArchive, WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
use world_projection::{
    BriefingItem, BriefingProjection, ProjectionCapabilities, ProjectionCommand, ProjectionIntent,
    ProjectionSnapshot,
};

const MOCK_PACK: &str = "world-machine.metadata-regression";

struct MockSession {
    count: u64,
}

impl WorldSession for MockSession {
    fn pack(&self) -> WorldPackRef {
        WorldPackRef::new(MOCK_PACK, "1")
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        ProjectionSnapshot {
            title: format!("Metadata Mock {}", self.count),
            world_time: self.count,
            capabilities: ProjectionCapabilities { fork: false },
            briefing: Some(BriefingProjection {
                eyebrow: "Metadata".into(),
                title: "Current state".into(),
                items: vec![BriefingItem {
                    selection: None,
                    title: format!("State {}", self.count),
                    detail: format!("Durable summary {}", self.count),
                }],
            }),
            commands: vec![ProjectionCommand {
                id: "mock.advance".into(),
                title: "Advance".into(),
                detail: "Advance the metadata regression World".into(),
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
            _ => Err(HostError::session("unsupported metadata regression intent")),
        }
    }

    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
        self.count += periods;
        Ok(self.snapshot())
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        Ok(Some(archive(self.count)))
    }
}

fn registry() -> WorldRegistry {
    let mut registry = WorldRegistry::new();
    registry
        .register(
            WorldRegistration::new(
                WorldDescriptor {
                    pack: WorldPackRef::new(MOCK_PACK, "1"),
                    title: "Metadata Regression".into(),
                    description: "Document metadata transaction regression".into(),
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

fn archive(world_time: u64) -> WorldArchive {
    WorldArchive {
        format: WORLD_ARCHIVE_FORMAT.into(),
        format_version: WORLD_ARCHIVE_VERSION,
        pack: WorldPackRef::new(MOCK_PACK, "1"),
        world_time,
        events: Vec::new(),
        pending: Vec::new(),
    }
}

fn lineage(label: &str) -> WorldLineage {
    WorldLineage {
        parent: WorldParent {
            document: Some("parent-world".into()),
            pack: WorldPackRef::new(MOCK_PACK, "1"),
            world_time: 4,
            event_count: 0,
        },
        branch: WorldBranchCause::Fork {
            label: Some(label.into()),
        },
    }
}

fn document(world_time: u64, label: &str) -> WorldDocument {
    WorldDocument::new(archive(world_time))
        .with_display_title(format!("Metadata Mock {world_time}"))
        .with_display_summary(format!("Original summary {world_time}"))
        .with_lineage(lineage(label))
}

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    env::temp_dir().join(format!(
        "world-machine-metadata-regression-{}-{nonce}-{label}",
        process::id()
    ))
}

#[test]
fn interactive_edits_preserve_document_metadata() {
    let root = temp_root("handle");
    let library = WorldLibrary::new(root.clone());
    let registry = registry();
    let id = WorldDocumentId::new("child").unwrap();
    let source = document(5, "interactive");
    library.create_from_document(id.clone(), &source).unwrap();
    let mut session = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();

    session
        .handle(
            ProjectionIntent::InvokeCommand("mock.advance".into()),
            &registry,
            &library,
        )
        .unwrap();

    assert_eq!(session.snapshot().world_time, 6);
    assert_eq!(session.metadata().lineage, source.metadata.lineage);
    assert_eq!(
        session.metadata().display_title.as_deref(),
        Some("Metadata Mock 6")
    );
    assert_eq!(
        session.metadata().display_summary.as_deref(),
        Some("State 6 · Durable summary 6")
    );
    let stored = library.load_document(&id).unwrap().unwrap();
    assert_eq!(stored.archive.world_time, 6);
    assert_eq!(stored.metadata.lineage, source.metadata.lineage);
    assert_eq!(
        stored.metadata.display_title.as_deref(),
        Some("Metadata Mock 6")
    );
    assert_eq!(
        stored.metadata.display_summary.as_deref(),
        Some("State 6 · Durable summary 6")
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn background_progression_preserves_document_metadata() {
    let root = temp_root("background");
    let library = WorldLibrary::new(root.clone());
    let registry = registry();
    let id = WorldDocumentId::new("child").unwrap();
    let source = document(5, "background");
    library.create_from_document(id.clone(), &source).unwrap();
    let mut session = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();

    session.advance_background(3, &registry, &library).unwrap();

    assert_eq!(session.snapshot().world_time, 8);
    assert_eq!(session.metadata().lineage, source.metadata.lineage);
    assert_eq!(
        session.metadata().display_title.as_deref(),
        Some("Metadata Mock 8")
    );
    assert_eq!(
        session.metadata().display_summary.as_deref(),
        Some("State 8 · Durable summary 8")
    );
    let stored = library.load_document(&id).unwrap().unwrap();
    assert_eq!(stored.metadata.lineage, source.metadata.lineage);
    assert_eq!(
        stored.metadata.display_title.as_deref(),
        Some("Metadata Mock 8")
    );
    assert_eq!(
        stored.metadata.display_summary.as_deref(),
        Some("State 8 · Durable summary 8")
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn reload_replaces_metadata_only_after_the_replacement_world_opens() {
    let root = temp_root("reload");
    let library = WorldLibrary::new(root.clone());
    let registry = registry();
    let id = WorldDocumentId::new("child").unwrap();
    let first = document(5, "first");
    let second = document(9, "second");
    library.create_from_document(id.clone(), &first).unwrap();
    let mut session = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
    library.save_document(&id, &second).unwrap();

    let snapshot = session.reload(&registry, &library).unwrap();

    assert_eq!(snapshot.world_time, 9);
    assert_eq!(session.metadata(), &second.metadata);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn import_and_export_preserve_document_metadata() {
    let root = temp_root("portable");
    let source = WorldLibrary::new(root.join("source"));
    let target = WorldLibrary::new(root.join("target"));
    let source_id = WorldDocumentId::new("source").unwrap();
    let imported_id = WorldDocumentId::new("imported").unwrap();
    let external = root.join("Portable.world");
    let document = document(11, "portable");
    source
        .create_from_document(source_id.clone(), &document)
        .unwrap();

    source.export_file(&source_id, &external).unwrap();
    target.import_file(imported_id.clone(), &external).unwrap();

    assert_eq!(target.load_document(&imported_id).unwrap(), Some(document));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn metadata_only_external_changes_participate_in_revision_conflicts() {
    let root = temp_root("metadata-conflict");
    let library = WorldLibrary::new(root.clone());
    let registry = registry();
    let id = WorldDocumentId::new("child").unwrap();
    let first = document(5, "first");
    let mut second = first.clone();
    second.metadata = WorldDocumentMetadata {
        display_title: first.metadata.display_title.clone(),
        display_summary: first.metadata.display_summary.clone(),
        lineage: Some(lineage("second")),
    };
    library.create_from_document(id.clone(), &first).unwrap();
    let mut session = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
    library.save_document(&id, &second).unwrap();

    assert!(matches!(
        session.handle(
            ProjectionIntent::InvokeCommand("mock.advance".into()),
            &registry,
            &library,
        ),
        Err(LibraryError::DocumentChanged(path)) if path == library.path(&id)
    ));
    assert_eq!(session.snapshot().world_time, 5);
    assert_eq!(session.metadata(), &first.metadata);
    let _ = fs::remove_dir_all(root);
}
