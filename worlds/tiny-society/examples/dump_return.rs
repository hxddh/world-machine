//! Print what a return actually looks like, at several lengths of absence.
//!
//! ```bash
//! cargo run -p tiny-society --example dump_return
//! ```
//!
//! Designing the return screen means reading the exact strings the World emits
//! — not a summary of them, and not copy written to flatter the design. This
//! prints the briefing, the offers on the table, the residents and the most
//! recent history side by side at 20, 40, 60 and 80 periods away, which is the
//! only way to see things like a briefing whose last line is "6 more things
//! happened" or a history that is five identical rent payments in a row.
use std::error::Error;
use tiny_society::{tiny_society_registration, TINY_SOCIETY_PACK_ID};
use world_host::WorldRegistry;

/// Where the branches take their choice: just after the bakery shuts.
const FORK_STEP: usize = 27;

fn main() -> Result<(), Box<dyn Error>> {
    let mut registry = WorldRegistry::new();
    registry.register(tiny_society_registration())?;
    for horizon in [20usize, 40, 60, 80] {
        let mut town = registry.create(TINY_SOCIETY_PACK_ID)?;
        town.advance_background(horizon as u64)?;
        let s = town.snapshot();
        println!(
            "\n===== after {horizon} periods · world time {} =====",
            s.world_time
        );
        if let Some(b) = &s.briefing {
            println!("-- briefing: {} / {}", b.eyebrow, b.title);
            for i in &b.items {
                println!("   [{:?}] {}\n        {}", i.kind, i.title, i.detail);
            }
        }
        println!("-- commands ({})", s.commands.len());
        for c in &s.commands {
            println!("   {} :: {}\n        {}", c.id, c.title, c.detail);
        }
        println!("-- collection: {}", s.collection.title);
        for i in s.collection.items.iter().take(9) {
            println!("   {} · {}", i.title, i.subtitle);
        }
        println!("-- timeline");
        for i in s.timeline.items.iter().take(5) {
            println!("   t={} {} · {}", i.world_time, i.title, i.subtitle);
        }
    }

    // A fortune line: how many people are in work and how much money is in the
    // town, sampled every few periods, on the trunk and on each branch. A
    // graph of this is the only way to see that the slope turned.
    // A fortune line for the trunk and for each branch of the bakery choice.
    //
    // Summing cash out of a rendered subtitle is a hack, and it is the only
    // way to get this today: no Pack reports a health figure, so there is
    // nothing to plot but a string the collection panel happens to print. A
    // World ought to say what its own fortune is, once per period, the way a
    // chess engine reports centipawns.
    println!("\n===== SERIES =====");
    for (label, command) in [
        ("trunk", None),
        ("reopen", Some(tiny_society::REOPEN_BAKERY_COMMAND)),
        ("lean", Some(tiny_society::LEAN_REOPEN_BAKERY_COMMAND)),
    ] {
        let mut town = registry.create(TINY_SOCIETY_PACK_ID)?;
        print!("{label}");
        for step in 0..60 {
            town.advance_background(2)?;
            // The fork. A swallowed failure here would make the branch equal
            // the trunk silently and the whole comparison a lie, so it is
            // loud.
            if step == FORK_STEP {
                if let Some(id) = command {
                    town.handle(world_projection::ProjectionIntent::InvokeCommand(
                        id.to_owned(),
                    ))
                    .unwrap_or_else(|error| {
                        panic!("{label}: the World refused {id} at the fork: {error}")
                    });
                }
            }
            let s = town.snapshot();
            let working = s
                .collection
                .items
                .iter()
                .filter(|i| !i.subtitle.contains("unemployed"))
                .count();
            let cash: i64 = s
                .collection
                .items
                .iter()
                .filter_map(|i| {
                    i.subtitle
                        .rsplit("cash ")
                        .next()
                        .and_then(|c| c.trim().parse::<i64>().ok())
                })
                .sum();
            print!(" {}:{}:{}", s.world_time, working, cash);
        }
        println!();
    }

    // Whether a World keeps producing events, or only keeps producing time.
    //
    // This is what found it: Harbour Town emits its last event, #346, at world
    // time 775, and then runs to 3000 without recording anything at all. Not
    // an equilibrium — a halt.
    println!("\n===== IS IT ALIVE =====");
    let mut town = registry.create(TINY_SOCIETY_PACK_ID)?;
    for mark in [40u64, 80, 120, 200, 300] {
        while town.snapshot().world_time < mark * 10 {
            town.advance_background(1)?;
        }
        let s = town.snapshot();
        let last = s.timeline.items.first();
        println!(
            "t={} · timeline items {} · newest: {}",
            s.world_time,
            s.timeline.items.len(),
            last.map(|i| format!("t={} {} · {}", i.world_time, i.title, i.subtitle))
                .unwrap_or_default()
        );
        if let Some(b) = &s.briefing {
            let beats = b
                .items
                .iter()
                .filter(|i| i.kind == world_projection::BriefingItemKind::Beat)
                .count();
            println!(
                "      briefing beats: {beats} · commands: {}",
                s.commands.len()
            );
        }
    }
    Ok(())
}
