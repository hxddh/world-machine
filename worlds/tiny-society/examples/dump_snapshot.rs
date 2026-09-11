//! Print a real World's projection as JSON, so a design can be worked out
//! against what this app actually produces rather than against invented
//! content.
//!
//! ```bash
//! cargo run -p tiny-society --example dump_snapshot -- 28 4 > snapshot.json
//! ```
//!
//! The first argument is how many background periods to advance, which is how
//! a World of a given age is obtained. The second is how many periods pass
//! between visits: without it the World is advanced in one jump and the
//! briefing covers the whole run, which is not a thing the app ever shows.
//! Pass it to get the snapshot a visitor actually sees on the last return.
//!
//! No voice is configured, so the text in the output is the table-written text
//! every observer sees by default.

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
    let stride = env::args()
        .nth(2)
        .map(|value| value.parse::<u64>())
        .transpose()?
        .filter(|stride| *stride > 0)
        .unwrap_or(periods.max(1));

    let mut registry = WorldRegistry::new();
    registry.register(tiny_society_registration())?;
    let mut session = registry.create(TINY_SOCIETY_PACK_ID)?;
    let mut snapshot = session.snapshot();
    let mut advanced = 0;
    while advanced < periods {
        let step = stride.min(periods - advanced);
        snapshot = session.advance_background(step)?;
        advanced += step;
    }

    let wire = ProjectionSnapshotWire::from(&snapshot);
    println!("{}", serde_json::to_string_pretty(&wire)?);
    Ok(())
}
