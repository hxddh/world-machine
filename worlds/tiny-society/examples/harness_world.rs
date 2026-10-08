//! Writes a harbour lived by a warm player for a given number of days into
//! a World Machine library, for the release screenshot harness
//! (`scripts/release-shots.sh`): the first answer every day, and a plot
//! built (or a deed done) every third day, as `warm_days` plays it.
//!
//! ```bash
//! cargo run --release -p tiny-society --example harness_world -- <library-dir> <id> <days>
//! ```

use std::error::Error;
use std::path::PathBuf;

use tiny_society::{tiny_society_registration, TINY_SOCIETY_PACK_ID};
use world_host::WorldRegistry;
use world_library::{WorldDocumentId, WorldLibrary};
use world_projection::ProjectionIntent::InvokeCommand;

const PASS: &str = "tiny-society.let-day-pass";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: harness_world <library-dir> <id> <days>";
    let root = PathBuf::from(args.next().ok_or(usage)?);
    let id = args.next().ok_or(usage)?;
    let days: usize = args.next().ok_or(usage)?.parse()?;
    let library = WorldLibrary::new(root);
    let mut registry = WorldRegistry::new();
    registry.register(tiny_society_registration())?;
    let mut session = registry.create(TINY_SOCIETY_PACK_ID)?;
    let mut snapshot = session.snapshot();
    for day in 1..=days {
        if let Some(answer) = snapshot.commands.iter().find(|command| {
            command.question.is_some() && command.unavailable.is_none() && command.id != PASS
        }) {
            if let Ok(next) = session.handle(InvokeCommand(answer.id.clone())) {
                snapshot = next;
            }
        }
        if day % 3 == 0 {
            let plot = snapshot
                .canvas
                .plots
                .iter()
                .flat_map(|plot| plot.offers.iter())
                .find(|offer| offer.unavailable.is_none())
                .map(|offer| offer.command.clone());
            let deed = plot.or_else(|| {
                snapshot
                    .commands
                    .iter()
                    .filter(|command| command.unavailable.is_none())
                    .find(|command| {
                        command
                            .hand
                            .as_ref()
                            .is_some_and(|hand| hand.verb != "Undo")
                    })
                    .map(|command| command.id.clone())
            });
            if let Some(deed) = deed {
                // The day passes next, and its snapshot is the one kept.
                let _ = session.handle(InvokeCommand(deed));
            }
        }
        snapshot = session.handle(InvokeCommand(PASS.into()))?;
    }
    let archive = session.archive()?.ok_or("an archive")?;
    let id = WorldDocumentId::new(&id)?;
    library.save(&id, &archive)?;
    library.set_display_title(&id, Some(snapshot.title.as_str()))?;
    library.describe(&id, &snapshot)?;
    println!("{} · {}", library.path(&id).display(), snapshot.title);
    Ok(())
}
