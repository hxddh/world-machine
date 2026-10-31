//! A hero palette per place (1.0 P5): every place the shipped Packs make
//! (the harbour, and each of Pocket Universe's three) is its own to look
//! at, at noon, at dusk and at night: its sky, its land, its spine and its
//! lamplight, as the scene lights them, far from every other place's.
//! Read from the places' own `PlaceLook` and scenery through the
//! renderer's colours (`Frame::hero_colours`), nothing drawn.

use world_gpui::diorama::{self, Camera, Clock};
use world_gpui::scene::Daylight;
use world_projection::ProjectionSnapshot;

/// Each shipped place as a player first opens it.
fn places() -> Vec<(&'static str, ProjectionSnapshot)> {
    let mut registry = world_builtins::registry().unwrap();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    let mut out = Vec::new();
    let harbour = registry.create(tiny_society::TINY_SOCIETY_PACK_ID).unwrap();
    out.push(("harbour", harbour.snapshot()));
    for (name, seed) in [
        ("Ares", pocket_universe::SEED_MARS_COLONY_COMMAND),
        ("Maple Street", pocket_universe::SEED_1980S_TOWN_COMMAND),
        (
            "Icebridge",
            pocket_universe::SEED_PENGUIN_CIVILIZATION_COMMAND,
        ),
    ] {
        let mut session = registry
            .create(pocket_universe::POCKET_UNIVERSE_PACK_ID)
            .unwrap();
        let snapshot = session
            .handle(world_projection::ProjectionIntent::InvokeCommand(
                seed.into(),
            ))
            .unwrap();
        out.push((name, snapshot));
    }
    out
}

/// The place's hero colours at `hour`, as the World window lights them.
fn colours(snapshot: &ProjectionSnapshot, daylight: Daylight, hour: f32) -> [[f32; 3]; 7] {
    let stage = diorama::stage_at(snapshot, 1100.0, 848.0, Clock::at(hour as u8));
    let living = diorama::living(&stage, snapshot, 0.0, daylight, &Default::default(), None);
    diorama::frame(
        snapshot,
        &stage,
        &living,
        Camera::whole(&stage),
        0.0,
        daylight,
        &diorama::Glows::new(),
        1.0,
    )
    .at_hour(hour)
    .hero_colours()
}

/// How far apart two palettes are: the root of the summed squares of
/// their colours' differences, in 0-to-255 steps.
fn apart(a: &[[f32; 3]; 7], b: &[[f32; 3]; 7]) -> f32 {
    a.iter()
        .zip(b)
        .flat_map(|(a, b)| (0..3).map(move |c| ((a[c] - b[c]) * 255.0).powi(2)))
        .sum::<f32>()
        .sqrt()
}

/// Every two places are at least 60 steps apart over their seven hero
/// colours at noon, at dusk and at night; and each place's own glow (its
/// lamps and lit windows) is not every other place's.
#[test]
fn every_place_has_a_hero_palette_of_its_own() {
    let places = places();
    for (daylight, hour) in [
        (Daylight::Day, 13.0),
        (Daylight::Dusk, 19.5),
        (Daylight::Night, 22.0),
    ] {
        let palettes = places
            .iter()
            .map(|(name, snapshot)| (*name, colours(snapshot, daylight, hour)))
            .collect::<Vec<_>>();
        for (n, (a, pa)) in palettes.iter().enumerate() {
            for (b, pb) in &palettes[n + 1..] {
                let d = apart(pa, pb);
                eprintln!("{daylight:?}: {a} and {b} {d:.0} apart");
                assert!(d >= 60.0, "{daylight:?}: {a} and {b} only {d:.0} apart");
            }
        }
    }
    let glows = places
        .iter()
        .map(|(name, snapshot)| (*name, colours(snapshot, Daylight::Night, 22.0)[6]))
        .collect::<Vec<_>>();
    for (n, (a, ga)) in glows.iter().enumerate() {
        for (b, gb) in &glows[n + 1..] {
            let d = (0..3)
                .map(|c| ((ga[c] - gb[c]) * 255.0).powi(2))
                .sum::<f32>()
                .sqrt();
            assert!(d >= 12.0, "{a} and {b} glow alike ({d:.0} apart)");
        }
    }
}
