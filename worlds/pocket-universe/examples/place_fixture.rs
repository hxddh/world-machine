//! Writes each Pocket Universe place, as its Pack sends it over the wire,
//! for the scene's place pictures and the key art
//! (`crates/world-gpui/tests/fixtures/place-<place>.json`).
//!
//! ```bash
//! cargo run -p pocket-universe --example place_fixture -- crates/world-gpui/tests/fixtures
//! ```
//!
//! Each place is played as a builder plays it for its first ninety
//! periods: the first answer each period, something built on a plot (or
//! made by hand) every third, and the period let pass. Only what the scene
//! draws is kept. The same build always writes the same files.

use pocket_universe::{
    PocketUniverse, NUDGE_COMMAND, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use world_pack_protocol::ProjectionSnapshotWire;

const PERIODS: u64 = 90;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "crates/world-gpui/tests/fixtures".into()),
    );
    for (name, seed) in [
        ("ares", SEED_MARS_COLONY_COMMAND),
        ("maple", SEED_1980S_TOWN_COMMAND),
        ("icebridge", SEED_PENGUIN_CIVILIZATION_COMMAND),
    ] {
        let mut universe = PocketUniverse::new()?;
        universe.invoke_projection_command(seed)?;
        for period in 1..=PERIODS {
            let snapshot = universe.projection_snapshot();
            if let Some(answer) = snapshot.commands.iter().find(|command| {
                command.question.is_some()
                    && command.unavailable.is_none()
                    && command.id != NUDGE_COMMAND
            }) {
                let _ = universe.invoke_projection_command(&answer.id.clone());
            }
            if period % 3 == 0 {
                let snapshot = universe.projection_snapshot();
                let plot = snapshot
                    .canvas
                    .plots
                    .iter()
                    .flat_map(|plot| plot.offers.iter())
                    .find(|offer| offer.unavailable.is_none())
                    .map(|offer| offer.command.clone());
                let deed = plot.or_else(|| {
                    snapshot
                        .commands
                        .iter()
                        .filter(|command| command.unavailable.is_none())
                        .find(|command| {
                            command
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
            universe.invoke_projection_command(NUDGE_COMMAND)?;
        }
        // Only what the scene draws: the place, its people and its
        // drawings, under a clear sky (the pictures do not change with the
        // day's weather).
        let shown = universe.projection_snapshot();
        let kept = world_projection::ProjectionSnapshot {
            title: shown.title,
            world_time: shown.world_time,
            capabilities: shown.capabilities,
            canvas: shown.canvas,
            collection: shown.collection,
            scenery: shown.scenery,
            calendar: shown.calendar,
            gauges: shown.gauges,
            goals: shown.goals,
            drawings: shown.drawings,
            ..Default::default()
        };
        let wire = ProjectionSnapshotWire::from(&kept);
        let path = out.join(format!("place-{name}.json"));
        std::fs::write(&path, serde_json::to_string(&wire)?)?;
        println!("{}", path.display());
    }
    Ok(())
}
