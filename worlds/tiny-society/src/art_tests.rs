//! Every work drawn as itself: the harbour's works, what can be built on
//! its plots and what can be made by hand each have a drawing of their
//! own, no two share a silhouette, and the whole set is kept as a
//! contact sheet.

use crate::{drawings, plots, story, town, TinySociety};
use std::collections::BTreeMap;
use world_gpui::art::Setting;
use world_gpui::works::{self, Art, Drawn};
use world_projection::MarkShape;

/// Everything the harbour can put up, in the order a contact sheet shows
/// it: its works, its plots' works, then what is made by hand, each with
/// its name, its shape, its drawing if it has one, and whether it wears
/// the player's design (and so keeps the shape it is painted on).
fn everything() -> Vec<(String, String, MarkShape, Option<&'static str>, bool)> {
    let mut all = Vec::new();
    for work in town::catalog() {
        all.push((
            work.id.to_string(),
            work.label.to_string(),
            work.shape,
            drawings::art_of_work(work.id),
            false,
        ));
    }
    for thing in plots::works() {
        all.push((
            thing.id.to_string(),
            thing.name.to_string(),
            story::fixture_shape(thing.shape),
            drawings::art_of(thing.id),
            matches!(thing.shape, "flag" | "bunting" | "signpost"),
        ));
    }
    let society = TinySociety::new().unwrap();
    for thing in crate::handwork::kit(society.world().state()).things {
        all.push((
            thing.id.to_string(),
            thing.name.to_string(),
            story::fixture_shape(thing.shape),
            drawings::art_of(thing.id),
            matches!(thing.shape, "flag" | "bunting" | "signpost"),
        ));
    }
    all
}

/// Two names for the same thing: a plot's twin of a work, built by the
/// player or by the harbour of its own accord.
fn same_thing(a: &str, b: &str) -> bool {
    let words = |text: &str| {
        text.to_lowercase()
            .replace(" on the square", "")
            .replace(" for the winter", "")
            .trim_start_matches("a ")
            .to_string()
    };
    words(a) == words(b)
}

#[test]
fn every_work_is_drawn_as_itself_and_no_two_share_a_silhouette() {
    let mut drawn_as: BTreeMap<String, String> = BTreeMap::new();
    let mut outlines: BTreeMap<u64, String> = BTreeMap::new();
    for (id, name, shape, art, wears) in everything() {
        if wears {
            // A flag, a sign or a line of washing is the player's design.
            continue;
        }
        let key = art.unwrap_or_else(|| panic!("{id} ({name}) has no drawing of its own"));
        let known = Art::from_key(key).unwrap_or_else(|| panic!("{id}: no drawing {key}"));
        let drawn = works::drawn(Some(key), shape, Setting::Harbour);
        assert_eq!(drawn, Drawn::Art(known));
        assert!(known.family().belongs_in(Setting::Harbour), "{key}");
        if let Some(other) = drawn_as.insert(key.to_string(), name.clone()) {
            assert!(
                same_thing(&other, &name),
                "{name} and {other} are both drawn as {key}"
            );
        }
        let outline = works::silhouette(known);
        if let Some(other) = outlines.insert(outline, key.to_string()) {
            assert_eq!(other, key, "{key} and {other} share a silhouette");
        }
    }
    // The review's own: a clock tower, a telescope, a bandstand and the
    // statues are each themselves, not a lighthouse or a tent.
    for (work, key) in [
        ("harbour_clock", "harbour-clock"),
        ("telescope", "telescope"),
        ("bandstand", "bandstand"),
        ("fishers_statue", "fishers-statue"),
        ("lighthouse_paint", "lighthouse"),
    ] {
        assert_eq!(drawings::art_of_work(work), Some(key));
    }
    assert_eq!(drawings::art_of("clock_tower"), Some("clock-tower"));
    assert_eq!(drawings::art_of("market_cross"), Some("market-cross"));
    assert_eq!(drawings::art_of("statue"), Some("fisherman-statue"));
}

#[test]
fn every_work_matches_its_contact_sheet() {
    let mut arts = Vec::new();
    for (_, _, _, art, _) in everything() {
        if let Some(art) = art.and_then(Art::from_key) {
            if !arts.contains(&art) {
                arts.push(art);
            }
        }
    }
    let sheet = works::contact_sheet(&arts, 14, 64);
    let image = image::RgbaImage::from_raw(sheet.width(), sheet.height(), sheet.data().to_vec())
        .expect("a picture");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/works.png");
    if std::env::var_os("WORLD_GPUI_UPDATE_GOLDEN").is_some() || !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image.save(&path).unwrap();
        return;
    }
    let kept = image::open(&path).unwrap().to_rgba8();
    assert_eq!(
        kept.dimensions(),
        image.dimensions(),
        "the sheet changed size"
    );
    let changed = kept
        .pixels()
        .zip(image.pixels())
        .filter(|(a, b)| a.0.iter().zip(b.0).any(|(x, y)| x.abs_diff(y) > 6))
        .count();
    assert!(
        (changed as f64) < (image.len() / 4) as f64 * 0.002,
        "{changed} pixels differ from {}",
        path.display()
    );
}

/// What a warm builder puts up is drawn as what it is on the scene: every
/// finished work and everything built on a plot carries its drawing.
#[test]
fn what_a_builder_puts_up_carries_its_drawing() {
    let branch = crate::plots_tests::warm_builder(45, |_, _| {});
    let snapshot = branch.projection_snapshot();
    assert_eq!(snapshot.canvas.setting.as_deref(), Some("harbour"));
    let state = branch.world().state();
    let mut built = 0;
    for item in &snapshot.canvas.items {
        let world_projection::SelectionId::Entity(id) = item.id else {
            continue;
        };
        let Some(entity) = state.entity(id) else {
            continue;
        };
        let Some(world_core::Value::Text(thing)) = entity.component("hands.thing") else {
            continue;
        };
        let growing = matches!(entity.component("shape"), Some(world_core::Value::Text(shape)) if shape == "sprouts");
        if let Some(art) = drawings::art_of(thing).filter(|_| !growing) {
            assert_eq!(item.art.as_deref(), Some(art), "{}", item.label);
            built += 1;
        }
    }
    assert!(built >= 3, "{built} built things with their drawings");
    for plot in &snapshot.canvas.plots {
        for offer in &plot.offers {
            if let Some(art) = &offer.art {
                assert!(Art::from_key(art).is_some(), "{art}");
            }
        }
    }
}
