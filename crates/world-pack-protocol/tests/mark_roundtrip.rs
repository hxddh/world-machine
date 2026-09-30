//! The player's mark crosses the wire: plots and their offers, a design on
//! a flag, a name to give and how the town built something, exactly; and a
//! Pack or host from before any of it sends and reads none.

use world_core::EntityId;
use world_pack_protocol::{CanvasItemWire, PlotWire, ProjectionSnapshotWire};
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasProjection, Design, Designable, MarkShape, Naming, Plot,
    PlotOffer, ProjectionSnapshot, SelectionId, Variant, Wears, PATTERN_CELLS,
};

fn design() -> Design {
    let cells = (0..PATTERN_CELLS)
        .map(|at| char::from_digit((at % 3) as u32, 16).unwrap())
        .collect::<String>();
    Design::new(&cells, &[2, 0, 9]).unwrap()
}

fn snapshot() -> ProjectionSnapshot {
    ProjectionSnapshot {
        canvas: CanvasProjection {
            items: vec![CanvasItem {
                id: SelectionId::Entity(EntityId::new(700)),
                kind: CanvasItemKind::Object,
                label: "Flagpole".into(),
                px: Some(1.25),
                pattern: Some(design().pattern()),
                design: Some(Designable {
                    command: "pack.design.700".into(),
                    wears: Wears::Flag,
                }),
                naming: Some(Naming {
                    command: "pack.name.700".into(),
                    proposals: vec!["Ada".into(), "Brin".into(), "Cato".into()],
                }),
                variant: Some(Variant {
                    colour: [200, 58, 50],
                    flip: true,
                    join_left: false,
                    join_right: true,
                }),
                ..Default::default()
            }],
            plots: vec![Plot {
                id: "p3".into(),
                px: 0.58,
                row: 4,
                district: "quay".into(),
                offers: vec![
                    PlotOffer {
                        command: "pack.hand.plot.bandstand.p3".into(),
                        label: "Bandstand".into(),
                        shape: MarkShape::Tent,
                        cost: Some("60".into()),
                        unavailable: None,
                        art: None,
                    },
                    PlotOffer {
                        command: "pack.hand.plot.boathouse.p3".into(),
                        label: "Boathouse".into(),
                        shape: MarkShape::House,
                        cost: None,
                        unavailable: Some("That's enough for today".into()),
                        art: None,
                    },
                ],
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
fn plots_designs_names_and_variants_come_back_the_same() {
    let snapshot = snapshot();
    let back = across(&snapshot);
    assert_eq!(back.canvas.plots, snapshot.canvas.plots);
    assert_eq!(back.canvas.items, snapshot.canvas.items);
    let pattern = back.canvas.items[0].pattern.as_ref().unwrap();
    assert_eq!(Design::from_pattern(pattern).unwrap(), design());
}

#[test]
fn a_pack_from_before_sends_none_and_is_read_as_none() {
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
    for field in ["plots", "pattern", "design", "naming", "variant"] {
        assert!(!json.contains(&format!("\"{field}\"")), "{field} in {json}");
    }
    assert_eq!(across(&plain).canvas, plain.canvas);
}

#[test]
fn a_broken_design_or_plot_is_dropped_not_shown() {
    let mut wire = ProjectionSnapshotWire::from(&snapshot());
    let item: &mut CanvasItemWire = &mut wire.canvas.items[0];
    // A colour not in the palette, and a cell past the design's colours.
    item.pattern.as_mut().unwrap().palette[0] = [1, 2, 3];
    item.design.as_mut().unwrap().wears = "hat".into();
    item.naming
        .as_mut()
        .unwrap()
        .proposals
        .push("\u{202E}\u{2066}".into());
    wire.canvas.plots.push(PlotWire {
        id: " ".into(),
        px: 1.0,
        row: 0,
        district: String::new(),
        offers: Vec::new(),
    });
    let back = ProjectionSnapshot::try_from(wire).unwrap();
    let item = &back.canvas.items[0];
    assert!(item.pattern.is_none());
    assert!(item.design.is_none());
    // A proposal of nothing but hidden controls is no name at all.
    assert_eq!(item.naming.as_ref().unwrap().proposals.len(), 3);
    assert_eq!(back.canvas.plots.len(), 1);
}
