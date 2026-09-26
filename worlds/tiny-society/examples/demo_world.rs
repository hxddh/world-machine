//! Write a demonstration Tiny Society World into a World Machine library
//! directory, so the desktop app can be looked at with a second kind of World
//! beside Pocket Universe (`scripts/linux-preview.sh` uses it).
//!
//! ```bash
//! cargo run -p tiny-society --example demo_world -- /path/to/Worlds
//! ```

use std::env;
use std::error::Error;
use std::path::PathBuf;

use tiny_society::{tiny_society_registration, TINY_SOCIETY_PACK_ID};
use world_host::WorldRegistry;
use world_library::{WorldDocumentId, WorldLibrary};
use world_projection::ProjectionIntent;

fn main() -> Result<(), Box<dyn Error>> {
    let root = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: demo_world <library-directory>")?;
    let library = WorldLibrary::new(root);
    let mut registry = WorldRegistry::new();
    registry.register(tiny_society_registration())?;

    let mut harbour = registry.create(TINY_SOCIETY_PACK_ID)?;
    // Twenty days with somebody playing: each day the first answer to the
    // first question, then the day passes, so the harbour on screen has
    // been lived in and shows it.
    for _ in 0..20 {
        let snapshot = harbour.snapshot();
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|command| command.question.is_some())
        {
            harbour.handle(ProjectionIntent::InvokeCommand(answer.id.clone()))?;
        }
        harbour.handle(ProjectionIntent::InvokeCommand(
            "tiny-society.let-day-pass".into(),
        ))?;
    }
    let archive = harbour
        .archive()?
        .ok_or("Tiny Society sessions always have an archive")?;
    let id = WorldDocumentId::new("harbour-town")?;
    library.save(&id, &archive)?;
    library.set_display_title(&id, Some(harbour.snapshot().title.as_str()))?;
    library.describe(&id, &harbour.snapshot())?;
    println!(
        "{} · {}",
        library.path(&id).display(),
        harbour.snapshot().title
    );
    Ok(())
}
