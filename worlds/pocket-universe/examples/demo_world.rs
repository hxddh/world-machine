//! Write demonstration Worlds into a World Machine library directory so the
//! desktop app can be launched against a realistic first-run state, for
//! screenshots and manual testing.
//!
//! ```bash
//! cargo run -p pocket-universe --example demo_world -- /path/to/Worlds
//! ```
//!
//! The Worlds are deterministic: the same commands always produce the same
//! archives, so screenshots taken from them are reproducible.

use std::env;
use std::error::Error;
use std::path::PathBuf;

use pocket_universe::{
    pocket_universe_registration, BOLD_PATH_COMMAND, HOLD_PRESSURE_COMMAND, NUDGE_COMMAND,
    POCKET_UNIVERSE_PACK_ID, REACH_PRESSURE_COMMAND, ROOTED_POSTURE_COMMAND,
    SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND, SHARED_PROJECT_COMMAND,
};
use world_document::{WorldBranchCause, WorldDocument, WorldLineage, WorldParent};
use world_host::{WorldRegistry, WorldSession};
use world_library::{WorldDocumentId, WorldLibrary};
use world_persistence::WorldArchive;
use world_projection::ProjectionIntent;

fn main() -> Result<(), Box<dyn Error>> {
    let root = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: demo_world <library-directory>")?;
    let library = WorldLibrary::new(root);
    let mut registry = WorldRegistry::new();
    registry.register(pocket_universe_registration())?;

    // A Mars colony that has lived long enough for chapter three to open:
    // first intervention, partnership, rooted posture, legacy, then pressure.
    let mut colony = registry.create(POCKET_UNIVERSE_PACK_ID)?;
    invoke(colony.as_mut(), SEED_MARS_COLONY_COMMAND)?;
    colony.advance_background(3)?;
    invoke(colony.as_mut(), BOLD_PATH_COMMAND)?;
    invoke(colony.as_mut(), SHARED_PROJECT_COMMAND)?;
    colony.advance_background(3)?;
    invoke(colony.as_mut(), ROOTED_POSTURE_COMMAND)?;
    let mut periods = 0;
    while !colony
        .snapshot()
        .commands
        .iter()
        .any(|command| command.id == HOLD_PRESSURE_COMMAND)
    {
        colony.advance_background(1)?;
        periods += 1;
        if periods > 20 {
            return Err("the colony never reached its third chapter".into());
        }
    }
    save(&library, "ares-pocket-colony", colony.as_ref())?;

    // Two futures branched from the colony at its crisis, and a branch of a
    // branch, so the lineage graph has a family to draw.
    let colony_archive = colony
        .archive()?
        .ok_or("Pocket Universe sessions always have an archive")?;
    let held = branch(
        &registry,
        &library,
        &colony_archive,
        "ares-pocket-colony",
        ("ares-held", "Ares · Held on"),
        (
            HOLD_PRESSURE_COMMAND,
            "Rebuild the reclaimer from what Ares has",
        ),
        12,
    )?;
    branch(
        &registry,
        &library,
        &colony_archive,
        "ares-pocket-colony",
        ("ares-reached", "Ares · Reached out"),
        (REACH_PRESSURE_COMMAND, "Send Kestrel for a replacement"),
        8,
    )?;
    branch(
        &registry,
        &library,
        &held,
        "ares-held",
        ("ares-held-later", "Ares · Held on, years later"),
        (NUDGE_COMMAND, "Let time pass"),
        10,
    )?;

    // A town that has only just started, so Home shows two Worlds at
    // different stages.
    let mut town = registry.create(POCKET_UNIVERSE_PACK_ID)?;
    invoke(town.as_mut(), SEED_1980S_TOWN_COMMAND)?;
    town.advance_background(1)?;
    save(&library, "maple-street-1987", town.as_ref())?;

    Ok(())
}

/// Play `choice` from `from` for `periods`, and save the result as a World
/// that remembers where it came from.
fn branch(
    registry: &WorldRegistry,
    library: &WorldLibrary,
    from: &WorldArchive,
    parent: &str,
    (id, title): (&str, &str),
    (command, choice_title): (&str, &str),
    periods: u64,
) -> Result<WorldArchive, Box<dyn Error>> {
    let mut session = registry.open_archive(from)?;
    invoke(session.as_mut(), command)?;
    session.advance_background(periods)?;
    let archive = session
        .archive()?
        .ok_or("Pocket Universe sessions always have an archive")?;
    let document = WorldDocument::new(archive.clone())
        .with_display_title(title)
        .with_lineage(WorldLineage {
            parent: WorldParent {
                document: Some(parent.into()),
                pack: from.pack.clone(),
                world_time: from.world_time,
                event_count: from.events.len(),
            },
            branch: WorldBranchCause::Strategy {
                choice_id: command.into(),
                choice_title: choice_title.into(),
                horizon: periods,
            },
        });
    library.save_document(&WorldDocumentId::new(id)?, &document)?;
    library.describe(&WorldDocumentId::new(id)?, &session.snapshot())?;
    println!("{id} · branched from {parent} · {title}");
    Ok(archive)
}

fn invoke(session: &mut dyn WorldSession, command: &str) -> Result<(), Box<dyn Error>> {
    session.handle(ProjectionIntent::InvokeCommand(command.into()))?;
    Ok(())
}

fn save(
    library: &WorldLibrary,
    id: &str,
    session: &dyn WorldSession,
) -> Result<(), Box<dyn Error>> {
    let archive = session
        .archive()?
        .ok_or("Pocket Universe sessions always have an archive")?;
    let id = WorldDocumentId::new(id)?;
    library.save(&id, &archive)?;
    // The app names a World after its snapshot title when it saves one; do
    // the same, or Home lists every demonstration World as "Pocket Universe".
    library.set_display_title(&id, Some(session.snapshot().title.as_str()))?;
    library.describe(&id, &session.snapshot())?;
    println!(
        "{} · {} · {}",
        library.path(&id).display(),
        session
            .snapshot()
            .moment_label(session.snapshot().world_time),
        session.snapshot().title
    );
    Ok(())
}
