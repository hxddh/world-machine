//! Print the briefing a visitor actually sees, visit after visit.
//!
//! `dump_snapshot` advances a World in one jump, so its briefing covers the
//! whole run and the same few events keep reappearing. That is not how the app
//! is used: a visitor comes back every so often and is told what happened
//! since last time. This example reproduces that loop, which is the only
//! honest way to check whether returning to a World is worth doing.
//!
//! ```bash
//! cargo run -p tiny-society --example dump_visits -- 12 4
//! ```
//!
//! The arguments are how many visits to make and how many periods pass between
//! them.

use std::env;
use std::error::Error;

use tiny_society::{tiny_society_registration, TINY_SOCIETY_PACK_ID};
use world_host::WorldRegistry;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let visits = args
        .next()
        .unwrap_or_else(|| "12".to_string())
        .parse::<u64>()?;
    let stride = args
        .next()
        .unwrap_or_else(|| "4".to_string())
        .parse::<u64>()?;

    let mut registry = WorldRegistry::new();
    registry.register(tiny_society_registration())?;
    let mut session = registry.create(TINY_SOCIETY_PACK_ID)?;

    let mut silent = 0_u32;
    for visit in 1..=visits {
        let snapshot = session.advance_background(stride)?;
        let beats = snapshot
            .briefing
            .as_ref()
            .map(|briefing| {
                briefing
                    .beats()
                    .into_iter()
                    .map(|item| item.title.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if beats.is_empty() {
            silent += 1;
        }
        println!(
            "visit {visit:>2} · world time {:>4} · {} choice(s) · {}",
            snapshot.world_time,
            snapshot.commands.len(),
            if beats.is_empty() {
                "NOTHING TO REPORT".to_string()
            } else {
                beats.join(" / ")
            }
        );
    }
    println!("\n{silent} of {visits} visits had nothing to report.");
    Ok(())
}
