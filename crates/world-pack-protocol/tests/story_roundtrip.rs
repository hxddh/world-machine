use world_core::EventId;
use world_pack_protocol::{ChapterWire, GoalWire, ProjectionSnapshotWire, SelectionIdWire};
use world_projection::{Chapter, Goal, MarkShape, ProjectionSnapshot, SelectionId};

#[test]
fn goals_and_chapters_cross_the_wire_and_come_back_the_same() {
    let snapshot = ProjectionSnapshot {
        goals: vec![Goal {
            id: "pier".into(),
            label: "The new pier".into(),
            shape: MarkShape::Bridge,
            done: 2,
            parts: 3,
        }],
        chapters: vec![Chapter {
            number: 1,
            title: "A bright summer".into(),
            summary: "The harbour kept its bakery.".into(),
            moment: Some(SelectionId::Event(EventId::new(7))),
        }],
        ..ProjectionSnapshot::default()
    };
    let wire = ProjectionSnapshotWire::from(&snapshot);
    let json = serde_json::to_string(&wire).unwrap();
    let back: ProjectionSnapshotWire = serde_json::from_str(&json).unwrap();
    let back = ProjectionSnapshot::try_from(back).unwrap();
    assert_eq!(back.goals, snapshot.goals);
    assert_eq!(back.chapters, snapshot.chapters);
}

#[test]
fn a_pack_that_sends_no_story_is_read_as_having_none() {
    let json = serde_json::to_string(&ProjectionSnapshotWire::default()).unwrap();
    assert!(!json.contains("goals") && !json.contains("chapters"));
    let back: ProjectionSnapshotWire = serde_json::from_str(&json).unwrap();
    assert!(back.goals.is_empty() && back.chapters.is_empty());
}

#[test]
fn a_goal_without_parts_or_a_name_and_a_chapter_without_a_title_are_dropped() {
    let wire = ProjectionSnapshotWire {
        goals: vec![
            GoalWire {
                id: "empty".into(),
                label: "Nothing".into(),
                shape: Default::default(),
                done: 0,
                parts: 0,
            },
            GoalWire {
                id: "nameless".into(),
                label: " ".into(),
                shape: Default::default(),
                done: 0,
                parts: 2,
            },
            GoalWire {
                id: "overdone".into(),
                label: "A lamp".into(),
                shape: Default::default(),
                done: 9,
                parts: 2,
            },
        ],
        chapters: vec![ChapterWire {
            number: 1,
            title: "".into(),
            summary: "Nothing".into(),
            moment: Some(SelectionIdWire::Event { id: 1 }),
        }],
        ..ProjectionSnapshotWire::default()
    };
    let snapshot = ProjectionSnapshot::try_from(wire).unwrap();
    assert_eq!(snapshot.goals.len(), 1);
    assert_eq!(snapshot.goals[0].done, 2, "no more done than it takes");
    assert!(snapshot.chapters.is_empty());
}

#[test]
fn keepsakes_cross_the_wire_and_an_older_pack_sends_none() {
    use world_core::EntityId;
    use world_projection::Keepsake;
    let snapshot = ProjectionSnapshot {
        keepsakes: vec![Keepsake {
            from: SelectionId::Entity(EntityId::new(2)),
            what: "a pressed flower from the Harbor".into(),
            note: "I kept this for you while you were away.".into(),
            moment: SelectionId::Event(EventId::new(9)),
        }],
        ..ProjectionSnapshot::default()
    };
    let wire = ProjectionSnapshotWire::from(&snapshot);
    let json = serde_json::to_string(&wire).unwrap();
    let back: ProjectionSnapshotWire = serde_json::from_str(&json).unwrap();
    let back = ProjectionSnapshot::try_from(back).unwrap();
    assert_eq!(back.keepsakes, snapshot.keepsakes);

    let older = serde_json::to_string(&ProjectionSnapshotWire::default()).unwrap();
    assert!(!older.contains("keepsakes"));
    let back: ProjectionSnapshotWire = serde_json::from_str(&older).unwrap();
    assert!(back.keepsakes.is_empty());
}

#[test]
fn a_book_and_where_things_stand_cross_the_wire_and_an_older_pack_sends_neither() {
    use world_projection::{BookEntry, CanvasItem, CanvasItemKind, MarkShape};
    let snapshot = ProjectionSnapshot {
        book: vec![
            BookEntry {
                shelf: "Made".into(),
                name: "Well".into(),
                found: true,
                shape: Some(MarkShape::Well),
                hint: String::new(),
            },
            BookEntry {
                shelf: "People".into(),
                name: "Rosa".into(),
                found: false,
                shape: None,
                hint: "Might come to stay".into(),
            },
        ],
        canvas: world_projection::CanvasProjection {
            items: vec![CanvasItem {
                id: SelectionId::Entity(world_core::EntityId::new(700)),
                kind: CanvasItemKind::Object,
                label: "Fountain".into(),
                detail: String::new(),
                x: 0.5,
                y: 0.5,
                changes: Vec::new(),
                shape: Some(MarkShape::Fountain),
                at: None,
                look: None,
                drawing: None,
                stance: None,
                standing: None,
                mood: None,
                spot: Some(0.4),
            }],
            ..Default::default()
        },
        ..ProjectionSnapshot::default()
    };
    let wire = ProjectionSnapshotWire::from(&snapshot);
    let json = serde_json::to_string(&wire).unwrap();
    let back = ProjectionSnapshot::try_from(
        serde_json::from_str::<ProjectionSnapshotWire>(&json).unwrap(),
    )
    .unwrap();
    assert_eq!(back.book, snapshot.book);
    assert_eq!(back.canvas.items[0].spot, Some(0.4));
    assert_eq!(back.canvas.items[0].shape, Some(MarkShape::Fountain));

    let older = serde_json::to_string(&ProjectionSnapshotWire::default()).unwrap();
    assert!(!older.contains("book"));
    let back: ProjectionSnapshotWire = serde_json::from_str(&older).unwrap();
    assert!(back.book.is_empty());
}
