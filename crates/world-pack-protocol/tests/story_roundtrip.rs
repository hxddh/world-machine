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
