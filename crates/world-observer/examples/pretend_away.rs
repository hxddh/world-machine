//! Backdates the observer clock of every World in a library, so the next
//! open plays as a return after `hours` away. For previews and demos.
//!
//! ```text
//! cargo run -p world-observer --example pretend_away -- <library-dir> <hours>
//! ```

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use world_observer::{CatchUpPolicy, ObserverKey, ObserverStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let library = PathBuf::from(
        args.next()
            .ok_or("usage: pretend_away <library-dir> <hours>")?,
    );
    let hours: u64 = args.next().ok_or("missing hours")?.parse()?;
    let root = library.parent().unwrap_or(&library).join("Observer");
    let store = ObserverStore::new(root);
    let then = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() - hours * 60 * 60;
    let policy = CatchUpPolicy::new(1, 1)?;
    for entry in std::fs::read_dir(&library)? {
        let path = entry?.path();
        let Some(id) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        // A stamp newer than `then` is rewound; no stamp is written as `then`.
        store.claim_due(&ObserverKey::new(format!("library:{id}"))?, then, policy)?;
        println!("{id}: last seen {hours}h ago");
    }
    Ok(())
}
