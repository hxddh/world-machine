//! Write Harbour Town into a World Machine library directory, so the desktop
//! app can be launched against it for screenshots and manual testing.
//!
//! ```bash
//! cargo run -p tiny-society --example demo_world -- /path/to/Worlds
//! ```
//!
//! Tiny Society is the Pack that tells the canvas where things are, so this is
//! the only World in the library that the window can draw as a place rather
//! than as a list. Screenshots taken without it say nothing about that drawing.
//!
//! It writes the town twice, at two ages, because one archive cannot show both
//! of the things the window has to get right:
//!
//! - **Late** (80 periods) is the place: the bakery shut, Sea Finch gone. A
//!   town where nothing has gone wrong yet is a row of identical open shops
//!   and proves nothing about whether a shut shop reads as shut.
//! - **Early** (24 periods) is the only one a *return* can be photographed on.
//!   A return shades the stretch you were away, and there is nothing to shade
//!   unless the World is still living: by 80 periods the town has come to
//!   rest, so an absence there spans no events and draws a flat line at zero.
//!
//! Both are deterministic: the same run always produces the same archives.

use std::env;
use std::error::Error;
use std::path::PathBuf;

use tiny_society::{
    tiny_society_registration, DEMO_EARLY_PERIODS, DEMO_LATE_PERIODS, TINY_SOCIETY_PACK_ID,
};
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

    write_town(&library, &registry, "harbour-town", DEMO_LATE_PERIODS)?;
    write_town(
        &library,
        &registry,
        "harbour-town-still-running",
        DEMO_EARLY_PERIODS,
    )?;
    Ok(())
}

fn write_town(
    library: &WorldLibrary,
    registry: &WorldRegistry,
    id: &str,
    periods: u64,
) -> Result<(), Box<dyn Error>> {
    let mut town = registry.create(TINY_SOCIETY_PACK_ID)?;
    town.advance_background(periods)?;

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
    let id = WorldDocumentId::new(id)?;
    library.save(&id, &archive)?;
    println!(
        "{} · {periods} periods · World time {} · {} · {placed} things placed",
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
