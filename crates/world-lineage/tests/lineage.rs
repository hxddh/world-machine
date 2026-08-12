use world_document::{WorldBranchCause, WorldDocument, WorldLineage, WorldParent};
use world_library::{WorldDocumentId, WorldLibrary};
use world_lineage::{build_index, LineageError, LineageIndex, LineageRecord};
use world_persistence::{WorldArchive, WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};

fn id(value: &str) -> WorldDocumentId {
    WorldDocumentId::new(value).unwrap()
}

fn record(value: &str) -> LineageRecord {
    LineageRecord {
        id: id(value),
        pack: WorldPackRef::new("world-machine.lineage-test", "1"),
        world_time: 10,
        event_count: 3,
        lineage: None,
    }
}

fn strategy_child(value: &str, parent: Option<&str>) -> LineageRecord {
    let mut record = record(value);
    record.world_time = 30;
    record.event_count = 8;
    record.lineage = Some(WorldLineage {
        parent: WorldParent {
            document: parent.map(str::to_owned),
            pack: WorldPackRef::new("world-machine.lineage-test", "1"),
            world_time: 10,
            event_count: 3,
        },
        branch: WorldBranchCause::Strategy {
            choice_id: "test.choose".into(),
            choice_title: "Choose".into(),
            horizon: 20,
        },
    });
    record
}

#[test]
fn builds_roots_children_and_branch_metadata() {
    let index = build_index([
        record("source"),
        strategy_child("future-a", Some("source")),
        strategy_child("future-b", Some("source.world")),
    ])
    .unwrap();

    assert_eq!(index.roots(), &[id("source")]);
    assert!(index.detached().is_empty());
    assert_eq!(
        index.node(&id("source")).unwrap().children,
        vec![id("future-a"), id("future-b")]
    );
    let future = index.node(&id("future-a")).unwrap();
    assert_eq!(future.parent.as_ref().unwrap().resolved, Some(id("source")));
    assert!(matches!(
        future.branch,
        Some(WorldBranchCause::Strategy { horizon: 20, .. })
    ));
}

#[test]
fn keeps_missing_or_external_parents_detached_instead_of_promoting_them_to_roots() {
    let index = build_index([
        record("local-root"),
        strategy_child("external-child", Some("External.world")),
        strategy_child("unknown-child", None),
    ])
    .unwrap();

    assert_eq!(index.roots(), &[id("local-root")]);
    assert_eq!(
        index.detached(),
        &[id("external-child"), id("unknown-child")]
    );
}

#[test]
fn exact_document_ids_win_before_suffix_normalization() {
    let index = build_index([
        record("source"),
        record("source.world"),
        strategy_child("future", Some("source.world")),
    ])
    .unwrap();

    assert_eq!(
        index
            .node(&id("future"))
            .unwrap()
            .parent
            .as_ref()
            .unwrap()
            .resolved,
        Some(id("source.world"))
    );
}

#[test]
fn ambiguous_normalized_parent_stays_detached() {
    let index = build_index([
        record("source"),
        record("source.world"),
        strategy_child("future", Some("source.world.json")),
    ])
    .unwrap();

    assert_eq!(index.detached(), &[id("future")]);
    assert_eq!(
        index
            .node(&id("future"))
            .unwrap()
            .parent
            .as_ref()
            .unwrap()
            .resolved,
        None
    );
}

#[test]
fn same_named_document_from_different_pack_stays_detached() {
    let mut unrelated = record("source");
    unrelated.pack = WorldPackRef::new("world-machine.other-pack", "1");

    let index = build_index([
        unrelated,
        strategy_child("future", Some("source.world")),
    ])
    .unwrap();

    assert_eq!(index.detached(), &[id("future")]);
    assert!(index.node(&id("source")).unwrap().children.is_empty());
    assert_eq!(
        index
            .node(&id("future"))
            .unwrap()
            .parent
            .as_ref()
            .unwrap()
            .resolved,
        None
    );
}

#[test]
fn rejects_duplicate_document_ids() {
    let error = build_index([record("same"), record("same")]).unwrap_err();
    assert!(matches!(
        error,
        LineageError::DuplicateDocumentId(ref duplicate) if duplicate == &id("same")
    ));
}

#[test]
fn rejects_resolved_parent_cycles() {
    let index = build_index([
        strategy_child("a", Some("b")),
        strategy_child("b", Some("a")),
    ]);

    assert!(matches!(index, Err(LineageError::Cycle(_))));
}

#[test]
fn child_lists_are_deterministically_sorted() {
    let index = build_index([
        record("root"),
        strategy_child("z-child", Some("root")),
        strategy_child("a-child", Some("root")),
    ])
    .unwrap();

    assert_eq!(
        index.node(&id("root")).unwrap().children,
        vec![id("a-child"), id("z-child")]
    );
}

#[test]
fn library_loader_reads_persisted_lineage_metadata() {
    let root =
        std::env::temp_dir().join(format!("world-machine-lineage-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let library = WorldLibrary::new(root.clone());
    let source = record("source");
    let future = strategy_child("future", Some("source"));

    let source_document = WorldDocument::new(archive(source.pack, source.world_time));
    let future_document = WorldDocument::new(archive(future.pack, future.world_time))
        .with_lineage(future.lineage.unwrap());

    library
        .save_document(&id("source"), &source_document)
        .unwrap();
    library
        .save_document(&id("future"), &future_document)
        .unwrap();

    let index = LineageIndex::from_library(&library).unwrap();

    assert_eq!(index.roots(), &[id("source")]);
    assert_eq!(
        index.node(&id("source")).unwrap().children,
        vec![id("future")]
    );
    let _ = std::fs::remove_dir_all(root);
}

fn archive(pack: WorldPackRef, world_time: u64) -> WorldArchive {
    WorldArchive {
        format: WORLD_ARCHIVE_FORMAT.into(),
        format_version: WORLD_ARCHIVE_VERSION,
        pack,
        world_time,
        events: Vec::new(),
        pending: Vec::new(),
    }
}
