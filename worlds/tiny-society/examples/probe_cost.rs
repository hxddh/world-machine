//! Does a World get more expensive the longer it lives?
//!
//! ```bash
//! cargo run --release -p tiny-society --example probe_cost
//! ```
//!
//! A World that never ends only works if returning to an old one costs what
//! returning to a new one costs. History is replayed from events, so the
//! question is whether advancing and projecting stay flat as the log grows.

use std::error::Error;
use std::time::Instant;
use world_host::WorldRegistry;

fn main() -> Result<(), Box<dyn Error>> {
    let mut registry = WorldRegistry::new();
    registry.register(tiny_society::tiny_society_registration())?;
    let mut world = registry.create(tiny_society::TINY_SOCIETY_PACK_ID)?;
    let mut visit = 0usize;
    for milestone in [50usize, 100, 200, 400] {
        while visit < milestone {
            world.advance_background(1)?;
            visit += 1;
        }
        let start = Instant::now();
        for _ in 0..10 {
            world.advance_background(1)?;
            visit += 1;
        }
        let advance = start.elapsed() / 10;
        let start = Instant::now();
        let snapshot = world.snapshot();
        let projection = start.elapsed();
        println!(
            "visit {visit:>3} · {:>5} events · advance one visit {advance:?} · project {projection:?}",
            snapshot.timeline.items.len()
        );
    }
    Ok(())
}
