//! Legends, moments and almanacs across the boundary (v7): asked for with
//! `story`, the latest moments and a New Year's almanac in the snapshot,
//! and the book's moment and faces; an older Pack is never asked, and an
//! older app passes all of it over.

use world_core::{EntityId, EventId};
use world_pack_protocol::{
    decode_request, decode_response, encode_request, encode_response, PackRequest,
    PackRequestEnvelope, PackResponse, PackResponseEnvelope, ProjectionSnapshotWire, StoryPageWire,
    StoryRequestWire, PACK_PROTOCOL_VERSION_V6, PACK_PROTOCOL_VERSION_V7,
};
use world_projection::{
    Almanac, BookEntry, Calendar, Legend, LegendLine, Moment, MomentKind, Mood, Named, Panel,
    PanelBeat, ProjectionSnapshot, Prop, SelectionId, StoryPage, StoryRequest,
};

fn someone(id: u64) -> SelectionId {
    SelectionId::Entity(EntityId::new(id))
}

fn moment(id: u64) -> Moment {
    let panel = |beat, caption: &str| Panel {
        caption: caption.into(),
        cast: vec![someone(1), someone(2)],
        place: Some(someone(100)),
        mood: Some(Mood::Happy),
        beat,
        props: vec![Prop::Bunting, Prop::Bouquet],
    };
    Moment {
        id: format!("moment-{id}"),
        day: id as u32,
        kind: MomentKind::Wedding,
        title: "Ada and Leo's wedding".into(),
        panels: [
            panel(PanelBeat::Before, "They had been walking out a long while."),
            panel(
                PanelBeat::Moment,
                "Ada and Leo were married at the old chapel",
            ),
            panel(
                PanelBeat::After,
                "The harbour danced on the quay till late.",
            ),
        ],
        event: Some(EventId::new(id)),
    }
}

fn almanac() -> Almanac {
    Almanac {
        year: 2,
        title: "The harbour's year 2".into(),
        arrived: vec!["Rosa".into()],
        left: vec!["Tobias".into()],
        born: vec!["Alfie".into()],
        died: vec![],
        built: vec!["The new pier".into()],
        best: Some("moment-7".into()),
        cast: vec![
            Named {
                name: "Rosa".into(),
                who: someone(430),
            },
            Named {
                name: "The new pier".into(),
                who: someone(6),
            },
        ],
    }
}

#[test]
fn a_story_is_asked_for_and_told_only_from_v7() {
    let ask = PackRequest::Story {
        request: StoryRequestWire::from(&StoryRequest::Legend(someone(2))),
    };
    assert!(PackRequestEnvelope::for_version(PACK_PROTOCOL_VERSION_V6, 1, ask.clone()).is_err());
    let envelope = PackRequestEnvelope::for_version(PACK_PROTOCOL_VERSION_V7, 1, ask).unwrap();
    let decoded = decode_request(&encode_request(&envelope).unwrap()).unwrap();
    let PackRequest::Story { request } = decoded.request else {
        panic!("not a story");
    };
    assert_eq!(
        StoryRequest::from(request),
        StoryRequest::Legend(someone(2))
    );
    for request in [
        StoryRequest::Moment("moment-7".into()),
        StoryRequest::Almanac(3),
    ] {
        assert_eq!(
            StoryRequest::from(StoryRequestWire::from(&request)),
            request
        );
    }

    let legend = Legend {
        subject: someone(2),
        title: "Mara".into(),
        lines: vec![
            LegendLine {
                day: 3,
                text: "Mara reopened Harbor Bakery".into(),
                because: Some("because you said “Let's help”".into()),
                event: Some(EventId::new(40)),
                cause: Some(EventId::new(31)),
            },
            LegendLine {
                day: 9,
                text: "Mara and Noah made it up".into(),
                because: None,
                event: Some(EventId::new(90)),
                cause: None,
            },
        ],
    };
    for page in [
        StoryPage::Legend(legend),
        StoryPage::Moment(moment(7)),
        StoryPage::Almanac(almanac()),
    ] {
        let response = PackResponseEnvelope::for_version(
            PACK_PROTOCOL_VERSION_V7,
            1,
            PackResponse::Story {
                page: Some(StoryPageWire::from(&page)),
            },
        )
        .unwrap();
        let decoded = decode_response(&encode_response(&response).unwrap()).unwrap();
        let PackResponse::Story { page: Some(told) } = decoded.response else {
            panic!("no page");
        };
        assert_eq!(told.into_page(), Some(page));
    }
    let none = PackResponseEnvelope::new(1, PackResponse::Story { page: None });
    let decoded = decode_response(&encode_response(&none).unwrap()).unwrap();
    assert_eq!(decoded.response, PackResponse::Story { page: None });
}

#[test]
fn the_snapshot_carries_the_latest_three_moments_the_almanac_and_the_books_faces() {
    let snapshot = ProjectionSnapshot {
        title: "Harbour".into(),
        moments: (1..=5).map(moment).collect(),
        almanac: Some(almanac()),
        almanac_years: vec![1, 2],
        calendar: Some(Calendar {
            unit: "Day".into(),
            length: 10,
            year: Some(120),
            ..Calendar::default()
        }),
        book: vec![BookEntry {
            shelf: "Moments".into(),
            name: "Ada and Leo's wedding".into(),
            found: true,
            moment: Some("moment-5".into()),
            cast: vec![someone(1), someone(2)],
            ..BookEntry::default()
        }],
        ..ProjectionSnapshot::default()
    };
    let wire = ProjectionSnapshotWire::from(&snapshot);
    assert_eq!(wire.moments.len(), 3);
    let json = serde_json::to_string(&wire).unwrap();
    let back = ProjectionSnapshot::try_from(
        serde_json::from_str::<ProjectionSnapshotWire>(&json).unwrap(),
    )
    .unwrap();
    assert_eq!(back.moments, (3..=5).map(moment).collect::<Vec<_>>());
    assert_eq!(back.almanac, snapshot.almanac);
    assert_eq!(back.almanac_years, vec![1, 2]);
    assert_eq!(back.book, snapshot.book);
    assert_eq!(back.calendar.unwrap().year, Some(120));

    // An older Pack's snapshot has none of it, and reads as it always did.
    let mut older = serde_json::to_value(&wire).unwrap();
    let fields = older.as_object_mut().unwrap();
    for field in ["moments", "almanac", "almanac_years"] {
        assert!(fields.remove(field).is_some());
    }
    for entry in fields["book"].as_array_mut().unwrap() {
        let entry = entry.as_object_mut().unwrap();
        entry.remove("moment");
        entry.remove("cast");
    }
    fields["calendar"].as_object_mut().unwrap().remove("year");
    let older = ProjectionSnapshot::try_from(
        serde_json::from_value::<ProjectionSnapshotWire>(older).unwrap(),
    )
    .unwrap();
    assert!(older.moments.is_empty() && older.almanac.is_none());
    assert!(older.almanac_years.is_empty());
    assert_eq!(older.book[0].moment, None);
    assert_eq!(older.calendar.unwrap().year, None);
}

#[test]
fn a_moment_without_its_three_panels_is_not_drawn() {
    let mut wire = world_pack_protocol::MomentWire::from(&moment(4));
    wire.panels.pop();
    assert!(wire.clone().into_moment().is_none());
    let mut odd = world_pack_protocol::MomentWire::from(&moment(4));
    odd.kind = "coronation".into();
    odd.panels[0].mood = Some("bemused".into());
    let drawn = odd.into_moment().unwrap();
    assert_eq!(drawn.kind, MomentKind::Other);
    assert_eq!(drawn.panels[0].mood, None);
}

/// A panel's props cross the boundary by id, each one; an id this build
/// does not know is left out, and a panel from an older Pack has none.
#[test]
fn a_panels_props_cross_the_boundary() {
    for prop in Prop::ALL {
        assert_eq!(Prop::from_id(prop.id()), Some(prop));
    }
    let mut wire = world_pack_protocol::MomentWire::from(&moment(2));
    assert_eq!(wire.panels[1].props, vec!["bunting", "bouquet"]);
    let json = serde_json::to_string(&wire).unwrap();
    let back: world_pack_protocol::MomentWire = serde_json::from_str(&json).unwrap();
    assert_eq!(back.into_moment(), Some(moment(2)));

    wire.panels[0].props = vec!["ferry".into(), "a-zeppelin".into()];
    wire.panels[1].props.clear();
    let told = wire.into_moment().unwrap();
    assert_eq!(told.panels[0].props, vec![Prop::Ferry]);
    assert!(told.panels[1].props.is_empty());

    let mut older =
        serde_json::to_value(world_pack_protocol::MomentWire::from(&moment(3))).unwrap();
    for panel in older["panels"].as_array_mut().unwrap() {
        assert!(panel.as_object_mut().unwrap().remove("props").is_some());
    }
    let older: world_pack_protocol::MomentWire = serde_json::from_value(older).unwrap();
    assert!(older
        .into_moment()
        .unwrap()
        .panels
        .iter()
        .all(|panel| panel.props.is_empty()));
}
