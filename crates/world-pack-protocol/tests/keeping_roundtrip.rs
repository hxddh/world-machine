//! The player's book (P3b) crosses the boundary whole, and an older Pack's
//! snapshot, which sends none, reads as it always did.

use world_core::{EntityId, EventId};
use world_pack_protocol::ProjectionSnapshotWire;
use world_projection::{Aim, Credit, Keeping, Page, ProjectionSnapshot, SelectionId, YearClosed};

fn keeping() -> Keeping {
    Keeping {
        title: "The Book".into(),
        role: "You keep it.".into(),
        year: 2,
        pages: vec![
            Page {
                title: "Spring: a beginning".into(),
                line: "Everyone came.".into(),
                filled: true,
                moment: Some(SelectionId::Event(EventId::new(4))),
            },
            Page {
                title: "Summer: a fair".into(),
                line: "The fair will fill this page".into(),
                filled: false,
                moment: None,
            },
        ],
        closed: vec![YearClosed {
            year: 1,
            title: "The end of year 1".into(),
            at: 1_190,
            credits: vec![
                Credit {
                    name: "Ada".into(),
                    deed: "Led the fair".into(),
                    who: Some(SelectionId::Entity(EntityId::new(3))),
                },
                Credit {
                    name: "You".into(),
                    deed: "Made 3 things by hand".into(),
                    who: None,
                },
            ],
            moment: Some(SelectionId::Event(EventId::new(9))),
        }],
        aims: vec![Aim {
            group: "Projects".into(),
            label: "The tower".into(),
            done: 2,
            of: 5,
            next: "Paint it".into(),
            who: None,
        }],
    }
}

#[test]
fn the_book_crosses_whole_and_an_older_pack_sends_none() {
    let snapshot = ProjectionSnapshot {
        title: "A place".into(),
        keeping: Some(keeping()),
        ..ProjectionSnapshot::default()
    };
    let wire = ProjectionSnapshotWire::from(&snapshot);
    let json = serde_json::to_string(&wire).unwrap();
    let back = ProjectionSnapshot::try_from(
        serde_json::from_str::<ProjectionSnapshotWire>(&json).unwrap(),
    )
    .unwrap();
    assert_eq!(back.keeping, Some(keeping()));

    let mut older = serde_json::to_value(&wire).unwrap();
    assert!(older.as_object_mut().unwrap().remove("keeping").is_some());
    let older = ProjectionSnapshot::try_from(
        serde_json::from_value::<ProjectionSnapshotWire>(older).unwrap(),
    )
    .unwrap();
    assert_eq!(older.keeping, None);
}

#[test]
fn a_book_with_nothing_to_show_is_left_out_and_counts_are_kept_honest() {
    let mut keeping = keeping();
    keeping.pages[1].title = "  ".into();
    keeping.aims[0].done = 9;
    keeping.aims.push(Aim {
        label: "Nothing to do".into(),
        of: 0,
        ..Aim::default()
    });
    let snapshot = ProjectionSnapshot {
        keeping: Some(keeping),
        ..ProjectionSnapshot::default()
    };
    let wire = serde_json::to_string(&ProjectionSnapshotWire::from(&snapshot)).unwrap();
    let back = ProjectionSnapshot::try_from(
        serde_json::from_str::<ProjectionSnapshotWire>(&wire).unwrap(),
    )
    .unwrap()
    .keeping
    .unwrap();
    assert_eq!(back.pages.len(), 1);
    assert_eq!(back.aims.len(), 1);
    assert_eq!((back.aims[0].done, back.aims[0].of), (5, 5));
    assert!(back.aims[0].finished());
}
