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
                ..BookEntry::default()
            },
            BookEntry {
                shelf: "People".into(),
                name: "Rosa".into(),
                found: false,
                shape: None,
                hint: "Might come to stay".into(),
                ..BookEntry::default()
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
                px: None,
                home: None,
                day: Vec::new(),
                built: None,
                ..Default::default()
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

#[test]
fn a_guest_crosses_the_wire_and_too_long_a_letter_is_cut() {
    use world_pack_protocol::{ProjectionIntentWire, MOST_GUEST_TEXT};
    use world_projection::{Guest, ProjectionIntent};
    let guest = Guest {
        name: "Nia".into(),
        from: "Ares Station".into(),
        letter: "Dust storm cleared. Thought of you.".into(),
        gift: "a postcard of Ares Station".into(),
        ..Guest::default()
    };
    let wire = ProjectionIntentWire::from(ProjectionIntent::Host(guest.clone()));
    let json = serde_json::to_string(&wire).unwrap();
    let back: ProjectionIntentWire = serde_json::from_str(&json).unwrap();
    assert_eq!(ProjectionIntent::from(back), ProjectionIntent::Host(guest));

    let long = ProjectionIntentWire::Host {
        name: "Nia".into(),
        from: "Ares".into(),
        letter: "x".repeat(MOST_GUEST_TEXT * 3),
        gift: String::new(),
        look: None,
        drawing: None,
        line: Some("y".repeat(MOST_GUEST_TEXT * 2)),
    };
    let ProjectionIntent::Host(cut) = ProjectionIntent::from(long) else {
        panic!("a guest");
    };
    assert_eq!(cut.letter.chars().count(), MOST_GUEST_TEXT);
    assert_eq!(
        cut.line.map(|line| line.chars().count()),
        Some(MOST_GUEST_TEXT)
    );
}

/// A friend's resident crosses with how they look, their own drawing and
/// a line from their World; a guest without them reads as before, and an
/// older guest's JSON, which has none of them, still reads.
#[test]
fn a_friends_resident_crosses_the_wire_with_their_drawing_and_line() {
    use world_pack_protocol::ProjectionIntentWire;
    use world_projection::{
        AgeStage, Carry, DrawPart, Drawing, Guest, Ink, Look, ProjectionIntent, Stance,
    };
    let guest = Guest {
        name: "Piko".into(),
        from: "Icebridge".into(),
        letter: "The ice sang all night.".into(),
        gift: "a postcard of Icebridge".into(),
        look: Some(Look {
            clothes: Some(0x223344),
            carries: Some(Carry::Fish),
            bird: true,
            age: Some(AgeStage::Elder),
            ..Look::default()
        }),
        drawing: Some(Drawing::new(
            "penguin",
            0.6,
            vec![
                DrawPart::ellipse(0.0, 0.45, 0.3, 0.45, Ink::Colour(0x1b1d24)),
                DrawPart::ellipse(0.0, 0.4, 0.2, 0.35, Ink::Colour(0xf4f1ea)).tone(0.1),
                DrawPart::line((0.2, 0.5), (0.4, 0.7), 0.05, Ink::Colour(0x1b1d24))
                    .only(&[Stance::Waving]),
            ],
        )),
        line: Some("Mind the thin ice by the Fish Vault.".into()),
    };
    let wire = ProjectionIntentWire::from(ProjectionIntent::Host(guest.clone()));
    let json = serde_json::to_string(&wire).unwrap();
    let back: ProjectionIntentWire = serde_json::from_str(&json).unwrap();
    assert_eq!(ProjectionIntent::from(back), ProjectionIntent::Host(guest));

    let plain = Guest {
        name: "Nia".into(),
        from: "Ares".into(),
        letter: "Hello.".into(),
        gift: String::new(),
        ..Guest::default()
    };
    let json = serde_json::to_string(&ProjectionIntentWire::from(ProjectionIntent::Host(
        plain.clone(),
    )))
    .unwrap();
    assert!(!json.contains("look") && !json.contains("drawing") && !json.contains("line"));
    let older = r#"{"type":"host","name":"Nia","from":"Ares","letter":"Hello."}"#;
    let back: ProjectionIntentWire = serde_json::from_str(older).unwrap();
    assert_eq!(ProjectionIntent::from(back), ProjectionIntent::Host(plain));
}

/// The place a snapshot draws: a panorama in districts, a season on the
/// ground, and people with homes and days, and works with the day they were
/// built.
fn a_place() -> ProjectionSnapshot {
    use world_projection::{
        CanvasItem, CanvasItemKind, District, GroundCover, RoutineStop, Season,
    };
    let person = SelectionId::Entity(world_core::EntityId::new(1));
    let home = SelectionId::Entity(world_core::EntityId::new(900_000_001));
    let pub_ = SelectionId::Entity(world_core::EntityId::new(400));
    let item = |id, kind, px| CanvasItem {
        id,
        kind,
        label: "Somewhere".into(),
        detail: String::new(),
        x: 0.5,
        y: 0.5,
        changes: Vec::new(),
        shape: None,
        at: None,
        look: None,
        drawing: None,
        stance: None,
        standing: None,
        mood: None,
        spot: None,
        px: Some(px),
        home: None,
        day: Vec::new(),
        built: None,
        ..Default::default()
    };
    let mut resident = item(person, CanvasItemKind::Actor, 1.2);
    resident.home = Some(home);
    resident.day = vec![
        RoutineStop {
            from_hour: 0,
            at: home,
            inside: true,
        },
        RoutineStop {
            from_hour: 8,
            at: pub_,
            inside: false,
        },
        RoutineStop {
            from_hour: 21,
            at: home,
            inside: true,
        },
    ];
    let mut work = item(
        SelectionId::Event(EventId::new(42)),
        CanvasItemKind::Object,
        2.7,
    );
    work.built = Some(117);
    work.shape = Some(MarkShape::Fountain);
    ProjectionSnapshot {
        canvas: world_projection::CanvasProjection {
            items: vec![
                resident,
                item(home, CanvasItemKind::Place, 1.1),
                item(pub_, CanvasItemKind::Place, 1.7),
                work,
            ],
            width: Some(4.5),
            districts: vec![
                District {
                    id: "quay".into(),
                    label: "The quay and the lighthouse".into(),
                    from: 0.0,
                    to: 1.5,
                },
                District {
                    id: "square".into(),
                    label: "The square".into(),
                    from: 1.5,
                    to: 3.0,
                },
            ],
            season: Some(Season::Winter),
            ground: Some(GroundCover::Snow),
            ice: true,
            ..Default::default()
        },
        ..ProjectionSnapshot::default()
    }
}

#[test]
fn the_place_crosses_the_wire_and_comes_back_the_same() {
    let snapshot = a_place();
    let wire = ProjectionSnapshotWire::from(&snapshot);
    let json = serde_json::to_string(&wire).unwrap();
    for field in [
        "\"width\"",
        "\"districts\"",
        "\"season\":\"winter\"",
        "\"ground\":\"snow\"",
        "\"ice\":true",
        "\"px\"",
        "\"home\"",
        "\"day\"",
        "\"from_hour\"",
        "\"built\":117",
    ] {
        assert!(json.contains(field), "{field} in {json}");
    }
    let back = ProjectionSnapshot::try_from(
        serde_json::from_str::<ProjectionSnapshotWire>(&json).unwrap(),
    )
    .unwrap();
    assert_eq!(back.canvas, snapshot.canvas);
}

#[test]
fn a_pack_that_sends_no_place_is_one_screen_with_nobody_keeping_hours() {
    let mut snapshot = a_place();
    snapshot.canvas.items.truncate(3);
    for item in &mut snapshot.canvas.items {
        item.px = None;
        item.home = None;
        item.day.clear();
    }
    snapshot.canvas.width = None;
    snapshot.canvas.districts.clear();
    snapshot.canvas.season = None;
    snapshot.canvas.ground = None;
    snapshot.canvas.ice = false;
    let json = serde_json::to_string(&ProjectionSnapshotWire::from(&snapshot)).unwrap();
    for field in [
        "width",
        "districts",
        "season",
        "ground",
        "ice",
        "\"px\"",
        "\"home\"",
        "\"day\"",
        "built",
    ] {
        assert!(!json.contains(field), "{field} in {json}");
    }
    let back = ProjectionSnapshot::try_from(
        serde_json::from_str::<ProjectionSnapshotWire>(&json).unwrap(),
    )
    .unwrap();
    assert_eq!(back.canvas, snapshot.canvas);
}

#[test]
fn a_newer_pack_s_unknown_season_and_ground_and_bad_numbers_are_read_safely() {
    let json = serde_json::to_string(&ProjectionSnapshotWire::from(&a_place()))
        .unwrap()
        .replace("\"winter\"", "\"monsoon\"")
        .replace("\"snow\"", "\"lava\"")
        .replace("\"width\":4.5", "\"width\":400.0")
        .replace("\"from_hour\":8", "\"from_hour\":30");
    let back = ProjectionSnapshot::try_from(
        serde_json::from_str::<ProjectionSnapshotWire>(&json).unwrap(),
    )
    .unwrap();
    assert_eq!(back.canvas.season, None);
    assert_eq!(back.canvas.ground, None);
    assert_eq!(
        back.canvas.width,
        Some(world_pack_protocol::MOST_CANVAS_WIDTH)
    );
    // An hour past the day's end is left out; the rest keep their order.
    let day = &back.canvas.items[0].day;
    assert_eq!(
        day.iter().map(|stop| stop.from_hour).collect::<Vec<_>>(),
        [0, 21]
    );
}

#[test]
fn the_place_goes_to_an_app_on_every_protocol_it_speaks() {
    use world_pack_protocol::{PackResponse, PackResponseEnvelope, PACK_PROTOCOL_VERSION};
    for version in 1..=PACK_PROTOCOL_VERSION {
        let envelope = PackResponseEnvelope::for_version(
            version,
            1,
            PackResponse::Snapshot {
                snapshot: ProjectionSnapshotWire::from(&a_place()),
            },
        )
        .unwrap();
        let json = serde_json::to_string(&envelope).unwrap();
        let back: PackResponseEnvelope = serde_json::from_str(&json).unwrap();
        back.validate().unwrap();
    }
}
