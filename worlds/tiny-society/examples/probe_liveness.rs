//! How long does this World go on having something to say?
//!
//! ```bash
//! cargo run -p PACK --example probe_liveness
//! ```
//!
//! Prints three figures, for `docs/PRODUCT_STUDY.md`:
//!
//! - **distinct choices** — how many different decisions the World will ever
//!   put on the table, across its whole life.
//! - **visits with a decision** — how often opening it asks you for something.
//! - **time to silence** — the last visit that produced a new event. After it
//!   the World is not settled into a rhythm; it is emitting nothing, and every
//!   return from then on opens a document that has not changed.
//!
//! The last figure is the one that matters. A story generator that can run out
//! is a story generator with a length, and this measures it.

use std::error::Error;
use world_host::WorldRegistry;

const VISITS: usize = 120;

fn probe(
    r: &WorldRegistry,
    name: &str,
    pack: &str,
    seed: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let mut world = r.create(pack)?;
    if let Some(command) = seed {
        // Some Worlds do nothing at all until they are seeded.
        world.handle(world_projection::ProjectionIntent::InvokeCommand(
            command.into(),
        ))?;
    }
    let mut with_a_decision = 0usize;
    let mut offered = Vec::<String>::new();
    let mut last_new_event = 0usize;
    let mut events = world.snapshot().timeline.items.len();
    for visit in 1..=VISITS {
        world.advance_background(1)?;
        let snapshot = world.snapshot();
        if !snapshot.commands.is_empty() {
            with_a_decision += 1;
            for command in &snapshot.commands {
                if !offered.contains(&command.id) {
                    offered.push(command.id.clone());
                }
            }
        }
        let now = snapshot.timeline.items.len();
        if now != events {
            last_new_event = visit;
            events = now;
        }
    }
    println!(
        "{name} · distinct choices {} · visits with a decision {with_a_decision}/{VISITS} · last new event at visit {last_new_event}",
        offered.len()
    );
    for id in &offered {
        println!("    {id}");
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut registry = WorldRegistry::new();
    registry.register(tiny_society::tiny_society_registration())?;
    probe(
        &registry,
        "Tiny Society",
        tiny_society::TINY_SOCIETY_PACK_ID,
        None,
    )
}
