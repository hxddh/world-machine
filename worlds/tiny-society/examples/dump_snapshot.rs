//! Print a real World's projection as JSON, so a design can be worked out
//! against what this app actually produces rather than against invented
//! content.
//!
//! ```bash
//! cargo run -p tiny-society --example dump_snapshot -- 4 > snapshot.json
//! ```
//!
//! The argument is how many background periods to advance first, which is how
//! a World of a given age is obtained. No voice is configured, so the text in
//! the output is the table-written text every observer sees by default.

use std::env;
use std::error::Error;

use tiny_society::{tiny_society_registration, TINY_SOCIETY_PACK_ID};
use world_host::WorldRegistry;
use world_pack_protocol::ProjectionSnapshotWire;

fn main() -> Result<(), Box<dyn Error>> {
    let periods = env::args()
        .nth(1)
        .unwrap_or_else(|| "4".to_string())
        .parse::<u64>()?;

    let mut registry = WorldRegistry::new();
    registry.register(tiny_society_registration())?;
    let mut session = registry.create(TINY_SOCIETY_PACK_ID)?;
    if periods > 0 {
        session.advance_background(periods)?;
    }

    let snapshot = session.snapshot();
    let wire = ProjectionSnapshotWire::from(&snapshot);
    println!("{}", serde_json::to_string_pretty(&wire)?);
    Ok(())
}
