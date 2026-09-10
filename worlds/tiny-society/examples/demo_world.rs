//! Write a demonstration Tiny Society World into a World Machine library
//! directory, beside the Pocket Universe ones, so the desktop app can be
//! launched against a library that exercises more than one Pack.
//!
//! ```bash
//! cargo run -p tiny-society --example demo_world -- /path/to/Worlds
//! ```
//!
//! This exists because a screenshot of a Pocket Universe World cannot show
//! anything about relations: that Pack models a relationship as an entity and
//! declares no `Relation` at all. Tiny Society does declare them, so a World
//! from it is the only place the canvas has edges to draw, and the only place
//! a screenshot can tell whether they are drawn correctly.
//!
//! The World is deterministic: the same commands always produce the same
//! archive, so screenshots taken from it are reproducible.

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

    // A town left to itself for a while, so its people have had time to form
    // the relationships the canvas draws.
    let mut town = registry.create(TINY_SOCIETY_PACK_ID)?;
    town.advance_background(4)?;

    let archive = town
        .archive()?
        .ok_or("Tiny Society sessions always have an archive")?;
    let id = WorldDocumentId::new("harbour-town")?;
    library.save(&id, &archive)?;
    println!(
        "{} · World time {} · {}",
        library.path(&id).display(),
        town.snapshot().world_time,
        town.snapshot().title
    );
    Ok(())
}
