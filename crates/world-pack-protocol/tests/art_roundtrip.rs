//! What a place looks like crosses the wire: its setting, and the drawing
//! each thing on it and each plot offer is drawn as, exactly; a Pack from
//! before sends none; and a key that is not a plain name is read as none.

use world_core::EntityId;
use world_pack_protocol::ProjectionSnapshotWire;
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasProjection, MarkShape, Plot, PlotOffer, ProjectionSnapshot,
    SelectionId,
};

fn snapshot() -> ProjectionSnapshot {
    ProjectionSnapshot {
        canvas: CanvasProjection {
            setting: Some("mars".into()),
            items: vec![CanvasItem {
                id: SelectionId::Entity(EntityId::new(910_000_004)),
                kind: CanvasItemKind::Place,
                label: "An observatory dome".into(),
                shape: Some(MarkShape::Dome),
                art: Some("observatory-dome".into()),
                ..Default::default()
            }],
            plots: vec![Plot {
                id: "p1".into(),
                px: 0.4,
                row: 3,
                district: "pad".into(),
                offers: vec![PlotOffer {
                    command: "pack.hand.plot.hab.p1".into(),
                    label: "Hab module".into(),
                    shape: MarkShape::House,
                    cost: Some("40".into()),
                    unavailable: None,
                    art: Some("hab-module".into()),
                }],
            }],
            ..CanvasProjection::default()
        },
        ..ProjectionSnapshot::default()
    }
}

fn across(snapshot: &ProjectionSnapshot) -> ProjectionSnapshot {
    let wire = ProjectionSnapshotWire::from(snapshot);
    let json = serde_json::to_string(&wire).unwrap();
    let back: ProjectionSnapshotWire = serde_json::from_str(&json).unwrap();
    ProjectionSnapshot::try_from(back).unwrap()
}

#[test]
fn the_setting_and_every_art_key_come_back_the_same() {
    let snapshot = snapshot();
    assert_eq!(across(&snapshot).canvas, snapshot.canvas);
}

#[test]
fn a_pack_from_before_sends_no_setting_or_art_and_is_read_as_none() {
    let plain = ProjectionSnapshot {
        canvas: CanvasProjection {
            items: vec![CanvasItem {
                label: "Bench".into(),
                ..Default::default()
            }],
            ..CanvasProjection::default()
        },
        ..ProjectionSnapshot::default()
    };
    let json = serde_json::to_string(&ProjectionSnapshotWire::from(&plain)).unwrap();
    for field in ["setting", "art"] {
        assert!(!json.contains(&format!("\"{field}\"")), "{field} in {json}");
    }
    // And a snapshot written before either field existed reads the same.
    let back = across(&plain);
    assert_eq!(back.canvas.setting, None);
    assert_eq!(back.canvas.items[0].art, None);
}

#[test]
fn a_key_that_is_not_a_plain_name_is_read_as_none() {
    let mut wire = ProjectionSnapshotWire::from(&snapshot());
    wire.canvas.setting = Some("Mars\u{202e}".into());
    wire.canvas.items[0].art = Some("x".repeat(200));
    wire.canvas.plots[0].offers[0].art = Some("hab module".into());
    let json = serde_json::to_string(&wire).unwrap();
    let back: ProjectionSnapshotWire = serde_json::from_str(&json).unwrap();
    let back = ProjectionSnapshot::try_from(back).unwrap();
    assert_eq!(back.canvas.setting, None);
    assert_eq!(back.canvas.items[0].art, None);
    assert_eq!(back.canvas.plots[0].offers[0].art, None);
}
