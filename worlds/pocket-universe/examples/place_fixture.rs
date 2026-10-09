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
//! made by hand) every third, and the period let pass. Each place is also
//! written as it starts (`start-<ground>.json`: `dust`, `street` or `ice`),
//! as a player first sees it. Only what the scene draws is kept. The same build always
//! writes the same files.

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
    for (name, ground, seed) in [
        ("ares", "dust", SEED_MARS_COLONY_COMMAND),
        ("maple", "street", SEED_1980S_TOWN_COMMAND),
        ("icebridge", "ice", SEED_PENGUIN_CIVILIZATION_COMMAND),
    ] {
        let mut universe = PocketUniverse::new()?;
        universe.invoke_projection_command(seed)?;
        write(&out, &format!("start-{ground}.json"), &universe)?;
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
        write(&out, &format!("place-{name}.json"), &universe)?;
    }
    Ok(())
}

/// Writes what the scene draws of `universe` (the place, its people and
/// its drawings, under a clear sky: the pictures do not change with the
/// day's weather) to `name` in `out`.
fn write(
    out: &std::path::Path,
    name: &str,
    universe: &PocketUniverse,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let path = out.join(name);
    std::fs::write(&path, serde_json::to_string(&wire)?)?;
    println!("{}", path.display());
    Ok(())
}
