//! Ground that joins the buildings (1.0 P5, the v0.30 review: the place
//! "lacks ground that joins the buildings"; it stood on its lawn like a
//! catalogue): before each building on land, a dooryard trodden bare; from
//! its door a worn path down to the spine people walk along; and between
//! neighbours standing in a row, a lane worn from one door to the next.
//!
//! Laid on the land under everything standing, in the path's own ink for
//! the place (gravel by the water, concrete on a street, packed snow on
//! the ice, a paler dust on a rust plain), and seeded by the stage, so the
//! same place always has the same yards. Presentation only.

use super::*;

/// One piece of worn ground, in stage pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Worn {
    /// A dooryard: an ellipse trodden bare, centred at (`x`, `y`).
    Yard { x: f32, y: f32, rx: f32, ry: f32 },
    /// A path from (`x0`, `y0`) at a door down to (`x1`, `y1`) on the spine,
    /// `w0` wide at the door and `w1` at the spine.
    Path {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        w0: f32,
        w1: f32,
    },
    /// A lane from one dooryard (`x0`, `y0`) to the next (`x1`, `y1`),
    /// sagging a little toward the viewer between them.
    Lane {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        w: f32,
    },
}

/// Whether `building` stands on land and has a door to wear ground before:
/// not a tree or a bridge, nothing on the water's line.
fn has_a_yard(frame: &Frame, building: &BuildingPaint, spine: f32) -> bool {
    !matches!(building.shape, MarkShape::Tree | MarkShape::Bridge)
        && building.base < spine - frame.figure_h * 0.25
}

/// The worn ground of `frame`, its spine at stage `spine`: a dooryard and
/// a path for every building on land, and a lane between neighbours in a
/// row (their feet within half a figure of each other, no more than three
/// of the wider and six figures apart).
pub(super) fn worn_ground(frame: &Frame, spine: f32) -> Vec<Worn> {
    let p = frame.figure_h;
    let mut standing = frame
        .buildings
        .iter()
        .filter(|building| has_a_yard(frame, building, spine))
        .collect::<Vec<_>>();
    standing.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.index.cmp(&b.index)));
    let mut out = Vec::new();
    for building in &standing {
        let seed = painter::hash2(building.x as i32, building.base as i32, 0x7a2d);
        let depth = (building.h / (p * COTTAGE)).clamp(0.4, 1.6);
        let (x, y) = (building.x, building.base + p * 0.08 * depth);
        out.push(Worn::Yard {
            x,
            y,
            rx: (building.w * 0.42).max(p * 0.5),
            ry: p * 0.2 * depth,
        });
        let lean = ((seed % 100) as f32 / 100.0 - 0.5) * p * 1.4;
        out.push(Worn::Path {
            x0: x,
            y0: y,
            x1: x + lean,
            y1: spine + 2.0,
            w0: p * 0.26 * depth,
            w1: p * 0.42,
        });
    }
    for pair in standing.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let gap = b.x - a.x;
        if (a.base - b.base).abs() > p * 0.5 || gap > a.w.max(b.w) * 3.0 + p * 6.0 {
            continue;
        }
        let depth = (a.h / (p * COTTAGE)).clamp(0.4, 1.6);
        out.push(Worn::Lane {
            x0: a.x,
            y0: a.base + p * 0.1 * depth,
            x1: b.x,
            y1: b.base + p * 0.1 * depth,
            w: p * 0.18 * depth,
        });
    }
    out
}

/// Paints the worn ground of `frame` lying between stage `from` and `to`,
/// in `ink`, its spine at `spine`.
pub(super) fn paint_worn_ground(
    canvas: &mut Canvas,
    frame: &Frame,
    (from, to): (f32, f32),
    ink: Hsla,
    spine: f32,
) {
    let reach = frame.figure_h * 3.0;
    for worn in worn_ground(frame, spine) {
        match worn {
            Worn::Yard { x, y, rx, ry } => {
                if x + rx < from - reach || x - rx > to + reach {
                    continue;
                }
                canvas.soft(x, y, rx, ry, ry * 0.9, ink.opacity(0.62));
            }
            Worn::Path {
                x0,
                y0,
                x1,
                y1,
                w0,
                w1,
            } => {
                if x0.max(x1) < from - reach || x0.min(x1) > to + reach {
                    continue;
                }
                let mut path = Shape::new();
                path.move_to(x0 - w0 / 2.0, y0)
                    .curve_to(
                        x1 - w1 / 2.0,
                        y1,
                        x0 - w0 / 2.0 + (x1 - x0) * 0.15,
                        (y0 + y1) / 2.0,
                    )
                    .line_to(x1 + w1 / 2.0, y1)
                    .curve_to(
                        x0 + w0 / 2.0,
                        y0,
                        x0 + w0 / 2.0 + (x1 - x0) * 0.55,
                        (y0 + y1) / 2.0,
                    )
                    .close();
                canvas.fill(&path, ink.opacity(0.6));
                // Walked paler down its middle.
                let mut middle = Shape::new();
                middle
                    .move_to(x0, y0)
                    .curve_to(x1, y1, x0 + (x1 - x0) * 0.35, (y0 + y1) / 2.0);
                canvas.stroke(&middle, w0 * 0.35, art::shade(ink, 0.08).opacity(0.45));
            }
            Worn::Lane { x0, y0, x1, y1, w } => {
                if x1 < from - reach || x0 > to + reach {
                    continue;
                }
                let sag = (x1 - x0).abs() * 0.04;
                let mut lane = Shape::new();
                lane.move_to(x0, y0)
                    .curve_to(x1, y1, (x0 + x1) / 2.0, (y0 + y1) / 2.0 + sag);
                canvas.stroke(&lane, w, ink.opacity(0.5));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every building on land stands on a dooryard with a path from it
    /// down to the spine, and neighbours in a row are joined by a lane
    /// (the first day by the water, and every lived place the fixtures
    /// keep).
    #[test]
    fn the_ground_joins_every_building_to_the_spine_and_its_neighbours() {
        for json in [
            include_str!("../../tests/fixtures/rest/first-day.json"),
            include_str!("../../tests/fixtures/start-dust.json"),
            include_str!("../../tests/fixtures/start-street.json"),
            include_str!("../../tests/fixtures/rest/three-years.json"),
        ] {
            let wire: world_pack_protocol::ProjectionSnapshotWire =
                serde_json::from_str(json).expect("a wire snapshot");
            let snapshot = ProjectionSnapshot::try_from(wire).expect("a snapshot");
            let stage = stage_at(&snapshot, 1100.0, 848.0, Clock::at(12));
            let living = living(
                &stage,
                &snapshot,
                0.0,
                Daylight::Day,
                &Default::default(),
                None,
            );
            let frame = frame(
                &snapshot,
                &stage,
                &living,
                Camera::whole(&stage),
                0.0,
                Daylight::Day,
                &Glows::new(),
                1.0,
            );
            let spine = frame.feet - frame.figure_h * 0.25;
            let worn = worn_ground(&frame, spine);
            let on_land = frame
                .buildings
                .iter()
                .filter(|building| has_a_yard(&frame, building, spine))
                .collect::<Vec<_>>();
            assert!(!on_land.is_empty(), "{}: buildings on land", snapshot.title);
            for building in &on_land {
                let yard = worn.iter().any(|worn| {
                    matches!(worn, Worn::Yard { x, rx, .. }
                        if (x - building.x).abs() < 1.0 && *rx >= building.w * 0.4)
                });
                let path = worn.iter().any(|worn| {
                    matches!(worn, Worn::Path { x0, y0, y1, .. }
                        if (x0 - building.x).abs() < 1.0
                            && *y0 >= building.base
                            && *y1 >= spine)
                });
                assert!(
                    yard && path,
                    "{}: building {} unjoined",
                    snapshot.title,
                    building.index
                );
            }
            let lanes = worn
                .iter()
                .filter(|worn| matches!(worn, Worn::Lane { .. }))
                .count();
            if on_land.len() >= 3 {
                assert!(lanes >= 1, "{}: no lane between neighbours", snapshot.title);
            }
        }
    }
}
