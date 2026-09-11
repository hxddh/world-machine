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
    Ok(())
}
