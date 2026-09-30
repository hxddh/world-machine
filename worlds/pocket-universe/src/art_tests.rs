//! Each place in its own clothes: every work, plot work and made thing on
//! Ares, Maple Street and Icebridge has a drawing of its own, from that
//! place's family of drawings; no two share a silhouette; nothing from the
//! harbour stands anywhere; and each place's set is kept as a contact
//! sheet. A lived year in each place is checked on the scene itself.

use crate::places::Place;
use crate::{drawings, handwork, plots, town, PocketUniverse, NUDGE_COMMAND};
use std::collections::BTreeMap;
use world_gpui::art::Setting;
use world_gpui::works::{self, Art};
use world_projection::{CanvasItemKind, MarkShape, ProjectionSnapshot};

const PLACES: [(&str, Place, Setting, &str); 3] = [
    (
        crate::SEED_MARS_COLONY_COMMAND,
        Place::Ares,
        Setting::Mars,
        "ares",
    ),
    (
        crate::SEED_1980S_TOWN_COMMAND,
        Place::Maple,
        Setting::Street,
        "maple",
    ),
    (
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
        Place::Ice,
        Setting::Ice,
        "icebridge",
    ),
];

fn seeded(seed: &str) -> PocketUniverse {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    universe
}

/// Everything a place can put up: its works, its plots' works, and what is
/// made by hand, with name, shape, drawing, and whether it wears the
/// player's design (a flag, a sail, a sign or a quilt line, which keep the
/// shape the design is painted on).
fn everything(
    universe: &PocketUniverse,
    place: Place,
) -> Vec<(String, String, MarkShape, Option<&'static str>, bool)> {
    let world = universe.world();
    let state = world.state();
    let mut all = Vec::new();
    for work in town::catalog(world, place) {
        all.push((
            work.id.to_string(),
            work.label.clone(),
            work.shape,
            drawings::art_of_work(place, work.id),
            false,
        ));
    }
    let wearing = |thing: &str, shape: &str| {
        matches!(shape, "flag" | "bunting")
            || matches!(thing, "sign" | "rowboat" | "kayak")
            || thing.ends_with("_sign")
            || matches!(thing, "sailboat" | "sail_sled")
    };
    for thing in plots::works(state) {
        all.push((
            thing.id.to_string(),
            thing.name.to_string(),
            crate::story::fixture_shape(world, thing.shape),
            drawings::art_of(place, thing.id),
            wearing(thing.id, thing.shape),
        ));
    }
    for thing in handwork::kit(state).things {
        all.push((
            thing.id.to_string(),
            thing.name.to_string(),
            crate::story::fixture_shape(world, thing.shape),
            drawings::art_of(place, thing.id),
            wearing(thing.id, thing.shape),
        ));
    }
    all
}

/// The drawings two names share because they are the same thing: a
/// plot's work and a work the place builds of its own accord, or a thing
/// made by hand and its plot twin.
const SAME_THING: &[&str] = &[
    "kelp-racks",
    "bus-shelter",
    "treehouse",
    "bleachers",
    "aurora-seat",
    "fishing-hole",
    "moss-patch",
    "low-g-swing",
    "drinking-fountain",
    "street-gazebo",
    "hot-dog-stand",
];

#[test]
fn every_work_in_every_place_is_drawn_as_itself() {
    let mut missing = Vec::new();
    for (seed, place, setting, name) in PLACES {
        let universe = seeded(seed);
        let mut drawn_as: BTreeMap<String, String> = BTreeMap::new();
        let mut outlines: BTreeMap<u64, String> = BTreeMap::new();
        for (id, label, shape, art, wears) in everything(&universe, place) {
            let drawn = works::drawn(art, shape, setting);
            assert!(
                drawn.family().belongs_in(setting),
                "{name}: {id} ({label}) is drawn as {}",
                drawn.id()
            );
            if wears {
                continue;
            }
            let Some(key) = art else {
                missing.push(format!("{name}: {id} ({label}, {shape:?})"));
                continue;
            };
            let known = Art::from_key(key).unwrap_or_else(|| panic!("{name}: no drawing {key}"));
            if let Some(other) = drawn_as.insert(key.to_string(), label.clone()) {
                if !SAME_THING.contains(&key) && other != label {
                    missing.push(format!("{name}: {label} and {other} are both {key}"));
                }
            }
            let outline = works::silhouette(known);
            if let Some(other) = outlines.insert(outline, key.to_string()) {
                assert_eq!(other, key, "{name}: {key} and {other} share a silhouette");
            }
        }
        assert!(drawn_as.len() >= 60, "{name}: {} drawings", drawn_as.len());
    }
    assert!(
        missing.is_empty(),
        "with no drawing of their own: {missing:#?}"
    );
}

/// The ids of every drawing a snapshot's scene is drawn with: the Pack's
/// own, or the app's for its art and shape in its setting.
fn drawn_on(snapshot: &ProjectionSnapshot) -> Vec<(String, works::Family, String)> {
    let setting = Setting::from_key(snapshot.canvas.setting.as_deref());
    snapshot
        .canvas
        .items
        .iter()
        .filter(|item| item.kind != CanvasItemKind::Actor)
        .map(|item| match snapshot.drawing_of(item) {
            Some(drawing) => (
                drawing.id.clone(),
                if drawing.id.starts_with("harbour") {
                    works::Family::Harbour
                } else {
                    works::Family::Any
                },
                item.label.clone(),
            ),
            None => {
                let drawn =
                    works::drawn(item.art.as_deref(), item.shape.unwrap_or_default(), setting);
                (drawn.id(), drawn.family(), item.label.clone())
            }
        })
        .collect()
}

/// A builder's year in each place: the first answer each period and a hand
/// lent every third, on a plot when one is free.
fn a_builders_year(seed: &str, periods: u64) -> PocketUniverse {
    let mut universe = seeded(seed);
    for period in 0..periods {
        let snapshot = universe.projection_snapshot();
        if let Some(pick) = snapshot.commands.iter().find(|command| {
            command.question.is_some()
                && command.unavailable.is_none()
                && command.id != NUDGE_COMMAND
        }) {
            let _ = universe.invoke_projection_command(&pick.id.clone());
        }
        if period % 3 == 2 {
            let snapshot = universe.projection_snapshot();
            let deed = snapshot
                .canvas
                .plots
                .iter()
                .flat_map(|plot| plot.offers.iter())
                .find(|offer| offer.unavailable.is_none())
                .map(|offer| offer.command.clone())
                .or_else(|| {
                    snapshot
                        .commands
                        .iter()
                        .find(|command| {
                            command.unavailable.is_none()
                                && command
                                    .hand
                                    .as_ref()
                                    .is_some_and(|hand| hand.verb != "Undo")
                        })
                        .map(|command| command.id.clone())
                });
            if let Some(deed) = deed {
                let _ = universe.invoke_projection_command(&deed);
            }
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
    universe
}

#[test]
fn no_harbour_drawing_stands_in_any_place() {
    for (seed, _, setting, name) in PLACES {
        let universe = a_builders_year(seed, 60);
        let snapshot = universe.projection_snapshot();
        assert_eq!(
            Setting::from_key(snapshot.canvas.setting.as_deref()),
            setting,
            "{name}"
        );
        let drawn = drawn_on(&snapshot);
        assert!(drawn.len() >= 8, "{name}: {} things", drawn.len());
        for (id, family, label) in &drawn {
            assert!(
                family.belongs_in(setting) && !id.starts_with("harbour"),
                "{name}: {label} is drawn as the harbour's {id}"
            );
        }
        // Every plot's offers are drawn in the place's own clothes too.
        for plot in &snapshot.canvas.plots {
            for offer in &plot.offers {
                let drawn = works::drawn(offer.art.as_deref(), offer.shape, setting);
                assert!(
                    drawn.family().belongs_in(setting),
                    "{name}: {}",
                    offer.label
                );
            }
        }
    }
}

#[test]
fn every_place_matches_its_contact_sheet() {
    let update = std::env::var_os("WORLD_GPUI_UPDATE_GOLDEN").is_some();
    for (seed, place, _, name) in PLACES {
        let universe = seeded(seed);
        let mut arts = Vec::new();
        for (_, _, _, art, _) in everything(&universe, place) {
            if let Some(art) = art.and_then(Art::from_key) {
                if !arts.contains(&art) {
                    arts.push(art);
                }
            }
        }
        let sheet = works::contact_sheet(&arts, 12, 64);
        let image =
            image::RgbaImage::from_raw(sheet.width(), sheet.height(), sheet.data().to_vec())
                .expect("a picture");
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/golden/works-{name}.png"));
        if update || !path.exists() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            image.save(&path).unwrap();
            continue;
        }
        let kept = image::open(&path).unwrap().to_rgba8();
        assert_eq!(
            kept.dimensions(),
            image.dimensions(),
            "{name}: the sheet changed size"
        );
        let changed = kept
            .pixels()
            .zip(image.pixels())
            .filter(|(a, b)| a.0.iter().zip(b.0).any(|(x, y)| x.abs_diff(y) > 6))
            .count();
        assert!(
            (changed as f64) < (image.len() / 4) as f64 * 0.002,
            "{name}: {changed} pixels differ from {}",
            path.display()
        );
    }
}

/// Writes a lived snapshot of each place for the app's golden scenes, as
/// the wire carries it: `WORLD_GPUI_PLACE_FIXTURES=<dir>`.
#[test]
#[ignore]
fn write_place_fixtures() {
    let Some(dir) = std::env::var_os("WORLD_GPUI_PLACE_FIXTURES") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    for (seed, _, _, name) in PLACES {
        let universe = a_builders_year(seed, 90);
        let mut snapshot = universe.projection_snapshot();
        // Only what the scene is drawn from: its canvas, look and weather.
        snapshot.timeline = Default::default();
        snapshot.inspectors.clear();
        snapshot.why.clear();
        snapshot.moments.clear();
        snapshot.book.clear();
        snapshot.voices.clear();
        snapshot.talks.clear();
        snapshot.exchanges.clear();
        snapshot.letters.clear();
        snapshot.keepsakes.clear();
        snapshot.chapters.clear();
        snapshot.commands.clear();
        snapshot.briefing = None;
        let wire = world_pack_protocol::ProjectionSnapshotWire::from(&snapshot);
        let json = serde_json::to_string(&wire).unwrap();
        std::fs::write(dir.join(format!("place-{name}.json")), json).unwrap();
    }
}
