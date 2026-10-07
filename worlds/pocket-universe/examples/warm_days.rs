//! Play a warm player's place and write its snapshot, as the app receives
//! it over the wire, on the days asked: the first answer every day, and a
//! plot built (or a deed done) every third day. For looking at the place
//! as it grows, and for the scene's fixtures.
//!
//! ```bash
//! cargo run -p pocket-universe --example warm_days -- <out-dir> ares 1 7 14 30 360
//! ```
//!
//! With `WARM_TALLY=1` it also prints how often each storylet ended, and
//! how, over the whole run. Each day is written to `<out-dir>/{place}-day-<N>.json`.

use std::error::Error;
use std::path::PathBuf;

use pocket_universe::{pocket_universe_registration, POCKET_UNIVERSE_PACK_ID};
use world_host::WorldRegistry;
use world_pack_protocol::ProjectionSnapshotWire;
use world_projection::ProjectionIntent::InvokeCommand;

const PASS: &str = "pocket-universe.nudge";

fn seed_of(place: &str) -> Result<&'static str, Box<dyn Error>> {
    Ok(match place {
        "ares" => "pocket-universe.seed-mars-colony",
        "maple" => "pocket-universe.seed-1980s-town",
        "icebridge" => "pocket-universe.seed-penguin-civilization",
        _ => return Err("ares, maple or icebridge".into()),
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().ok_or("an output folder")?);
    let place = args.next().ok_or("a place")?;
    let days = args
        .map(|day| day.parse::<usize>())
        .collect::<Result<Vec<_>, _>>()?;
    let last = days.iter().copied().max().unwrap_or(1);
    std::fs::create_dir_all(&out)?;
    let mut registry = WorldRegistry::new();
    registry.register(pocket_universe_registration())?;
    let mut session = registry.create(POCKET_UNIVERSE_PACK_ID)?;
    session.handle(InvokeCommand(seed_of(&place)?.into()))?;
    let mut snapshot = session.snapshot();
    if days.contains(&0) {
        let wire = ProjectionSnapshotWire::from(&snapshot);
        std::fs::write(
            out.join(format!("{place}-day-0.json")),
            serde_json::to_string(&wire)?,
        )?;
    }
    for day in 1..=last {
        if let Some(answer) = snapshot.commands.iter().find(|command| {
            command.question.is_some() && command.unavailable.is_none() && command.id != PASS
        }) {
            if let Ok(next) = session.handle(InvokeCommand(answer.id.clone())) {
                snapshot = next;
            }
        }
        if day % 3 == 0 {
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
                let _ = session.handle(InvokeCommand(deed));
            }
        }
        snapshot = session.handle(InvokeCommand(PASS.into()))?;
        if days.contains(&day) {
            let wire = ProjectionSnapshotWire::from(&snapshot);
            std::fs::write(
                out.join(format!("{place}-day-{day}.json")),
                serde_json::to_string(&wire)?,
            )?;
            eprintln!("day {day}: {} items", snapshot.canvas.items.len());
        }
    }
    // `WARM_TALLY=1`: how often each storylet ended, and how.
    if std::env::var_os("WARM_TALLY").is_some() {
        let archive = session.archive()?.ok_or("an archive")?;
        let mut tally = std::collections::BTreeMap::<String, usize>::new();
        for event in &archive.events {
            if event.kind == "situation_arose" {
                continue;
            }
            if let Some(storylet) = event.payload.get("storylet") {
                *tally
                    .entry(format!("{storylet:?} {}", event.kind))
                    .or_default() += 1;
            }
        }
        let mut tally = tally.into_iter().collect::<Vec<_>>();
        tally.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        for (what, times) in tally {
            println!("{times:5} {what}");
        }
    }
    Ok(())
}
