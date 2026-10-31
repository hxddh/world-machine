//! Prop scale on the stage (1.0 P5, the v0.30 review: "props at human
//! scale; benches are about twice too big"): the ladder's own tests hold
//! every drawing to its rung as it is painted (`ladder_tests`); these hold
//! every building and thing as the stage sizes it, in every fixture, to
//! the rung beside a grown-up standing at its depth.

use super::*;
use crate::ladder::Subject;

/// Every fixture the scene's tests keep, named by its file.
fn fixtures() -> Vec<(String, ProjectionSnapshot)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut paths = std::fs::read_dir(&dir)
        .expect("the fixtures")
        .chain(std::fs::read_dir(dir.join("rest")).expect("the rest fixtures"))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let json = std::fs::read_to_string(&path).ok()?;
            let wire: world_pack_protocol::ProjectionSnapshotWire =
                serde_json::from_str(&json).ok()?;
            let snapshot = ProjectionSnapshot::try_from(wire).ok()?;
            Some((path.file_stem()?.to_str()?.to_string(), snapshot))
        })
        .collect()
}

/// Whether a prop sized `got` on the stage stands at its rung: never
/// larger than the ladder's box for it beside a grown-up `p` tall, and
/// never under half of it (squeezed into a narrow gap, it keeps its
/// proportions a little smaller).
fn off_rung(subject: &Subject, got: (f32, f32), p: f32, room: f32) -> Option<String> {
    let Some(rung) = subject.rung() else {
        return Some("no rung".into());
    };
    let (w, h) = subject.sized(p, room)?;
    let (bigger, smaller) = (
        got.0 > w * 1.02 + 0.5 || got.1 > h * 1.02 + 0.5,
        got.1 < h * 0.5,
    );
    (bigger || smaller).then(|| {
        format!(
            "{:.2} by {:.2} P where its rung ({rung:?}) has {:.2} by {:.2}",
            got.0 / p,
            got.1 / p,
            w / p,
            h / p
        )
    })
}

/// Every thing and building on the stage, at the window's size, in every
/// fixture, has a rung and stands at it beside a grown-up at its depth:
/// a bench as high as a grown-up's hip, never a cottage's; nothing grown
/// past its rung, nothing shrunk to under half of it.
#[test]
fn every_prop_on_the_stage_stands_at_its_own_scale() {
    let mut wrong = Vec::new();
    for (name, snapshot) in fixtures() {
        let stage = stage_at(&snapshot, 1100.0, 848.0, Clock::at(12));
        let setting = art::Setting::from_key(snapshot.canvas.setting.as_deref());
        for spot in &stage.things {
            let item = &snapshot.canvas.items[spot.index];
            let p = stage.figure_h * spot.scale;
            let subject = match snapshot.drawing_of(item) {
                Some(drawing) => Subject::Drawing(drawing),
                None => Subject::Thing(
                    item.shape.unwrap_or(MarkShape::Parcel),
                    setting,
                    art_of(item),
                ),
            };
            let size = drawn_thing_size(&stage, &snapshot, spot, setting);
            if let Some(why) = off_rung(&subject, size, p, spot.w) {
                wrong.push(format!(
                    "{name}: thing {:?} {:?}: {why}",
                    item.shape, item.art
                ));
            }
        }
        for spot in &stage.buildings {
            let item = &snapshot.canvas.items[spot.index];
            let p = stage.figure_h * spot.scale;
            let subject = match snapshot.drawing_of(item) {
                Some(drawing) => Subject::Drawing(drawing),
                None => Subject::Building(item.shape.unwrap_or_default(), setting, art_of(item)),
            };
            let size = building_size(&stage, &snapshot, spot, setting);
            if let Some(why) = off_rung(&subject, size, p, spot.w) {
                wrong.push(format!(
                    "{name}: building {:?} {:?}: {why}",
                    item.shape, item.art
                ));
            }
        }
    }
    wrong.sort();
    wrong.dedup();
    assert!(
        wrong.is_empty(),
        "off the ladder on the stage:\n{}",
        wrong.join("\n")
    );
}
