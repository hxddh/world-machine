//! Write a Harbour Town World into a World Machine library directory, so the
//! desktop app can be launched against it for screenshots and manual testing.
//!
//! ```bash
//! cargo run -p tiny-society --example demo_world -- /path/to/Worlds
//! ```
//!
//! Tiny Society is the Pack that tells the canvas where things are, so this is
//! the only World in the library that the window can draw as a place rather
//! than as a list. Screenshots taken without it say nothing about that drawing.
//!
//! The World is deterministic: the same run always produces the same archive.

use std::env;
use std::error::Error;
use std::path::PathBuf;

use tiny_society::{tiny_society_registration, TINY_SOCIETY_PACK_ID};
use world_host::WorldRegistry;
use world_library::{WorldDocumentId, WorldLibrary};

fn main() -> Result<(), Box<dyn Error>> {
    let root = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: demo_world <library-directory>")?;
    let library = WorldLibrary::new(root);
    let mut registry = WorldRegistry::new();
    registry.register(tiny_society_registration())?;

    // Far enough in that the bakery has shut and Sea Finch has gone — the
    // state the picture exists to show. A town where nothing has gone wrong
    // yet is a row of identical open shops, and proves nothing about whether
    // a shut shop reads as shut.
    let mut town = registry.create(TINY_SOCIETY_PACK_ID)?;
    town.advance_background(80)?;

    let snapshot = town.snapshot();
    let placed = snapshot
        .canvas
        .items
        .iter()
        .filter(|item| item.at.is_some())
        .count();
    if placed == 0 {
        return Err("this World would be drawn as a list, which is the thing \
                    the screenshot is meant to check"
            .into());
    }

    let archive = town
        .archive()?
        .ok_or("a Tiny Society session always has an archive")?;
    let id = WorldDocumentId::new("harbour-town")?;
    library.save(&id, &archive)?;
    println!(
        "{} · World time {} · {} · {placed} things placed",
        library.path(&id).display(),
        snapshot.world_time,
        snapshot.title
    );
    // Printed so a screenshot run's log says what the picture should contain;
    // a town where everything is Working means the shot proves less than it
    // looks like it does.
    for item in &snapshot.canvas.items {
        println!(
            "  {:?} {} · at {:?} · {:?}",
            item.kind, item.label, item.at, item.state
        );
    }
    Ok(())
}
