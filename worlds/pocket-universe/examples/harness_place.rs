//! Writes one Pocket Universe World at its very start (its first night,
//! sol or day, as the start screen's card begins it) into a World Machine
//! library directory, for the release screenshot harness
//! (`scripts/release-shots.sh`).
//!
//! ```bash
//! cargo run -p pocket-universe --example harness_place -- /path/to/Worlds maple
//! ```
//!
//! The place is `ares`, `maple` or `icebridge`.

use std::env;
use std::error::Error;
use std::path::PathBuf;

use pocket_universe::{
    pocket_universe_registration, POCKET_UNIVERSE_PACK_ID, SEED_1980S_TOWN_COMMAND,
    SEED_MARS_COLONY_COMMAND, SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use world_document::WorldDocument;
use world_host::WorldRegistry;
use world_library::{WorldDocumentId, WorldLibrary};
use world_projection::ProjectionIntent;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let root = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: harness_place <library-directory> <ares|maple|icebridge>")?;
    let place = args
        .next()
        .and_then(|place| place.into_string().ok())
        .ok_or("usage: harness_place <library-directory> <ares|maple|icebridge>")?;
    let seed = match place.as_str() {
        "ares" => SEED_MARS_COLONY_COMMAND,
        "maple" => SEED_1980S_TOWN_COMMAND,
        "icebridge" => SEED_PENGUIN_CIVILIZATION_COMMAND,
        other => return Err(format!("no place called {other}").into()),
    };
    let library = WorldLibrary::new(root);
    let mut registry = WorldRegistry::new();
    registry.register(pocket_universe_registration())?;
    let mut session = registry.create(POCKET_UNIVERSE_PACK_ID)?;
    session.handle(ProjectionIntent::InvokeCommand(seed.into()))?;
    let archive = session
        .archive()?
        .ok_or("Pocket Universe sessions always have an archive")?;
    let id = WorldDocumentId::new(format!("{place}-start"))?;
    library.save_document(&id, &WorldDocument::new(archive))?;
    let snapshot = session.snapshot();
    library.describe(&id, &snapshot)?;
    // Named as a start names it ("Maple Street · 1987"), not by its file.
    let title = snapshot.title.trim();
    if !title.is_empty() {
        library.set_display_title(&id, Some(title))?;
    }
    println!("{place}-start");
    Ok(())
}
