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
    pocket_universe_registration, BOLD_PATH_COMMAND, HOLD_PRESSURE_COMMAND,
    POCKET_UNIVERSE_PACK_ID, ROOTED_POSTURE_COMMAND, SEED_1980S_TOWN_COMMAND,
    SEED_MARS_COLONY_COMMAND, SHARED_PROJECT_COMMAND,
};
use world_host::{WorldRegistry, WorldSession};
use world_library::{WorldDocumentId, WorldLibrary};
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

    // A town that has only just started, so Home shows two Worlds at
    // different stages.
    let mut town = registry.create(POCKET_UNIVERSE_PACK_ID)?;
    invoke(town.as_mut(), SEED_1980S_TOWN_COMMAND)?;
    town.advance_background(1)?;
    save(&library, "maple-street-1987", town.as_ref())?;

    Ok(())
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
    println!(
        "{} · World time {} · {}",
        library.path(&id).display(),
        session.snapshot().world_time,
        session.snapshot().title
    );
    Ok(())
}
