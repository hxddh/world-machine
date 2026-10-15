//! Backdates the observer clock and the sitting ledger of every World in a
//! library, so the next open plays as a new sitting after `hours` away
//! (at least a day). For previews and demos.
//!
//! ```text
//! cargo run -p world-observer --example pretend_away -- <library-dir> <hours>
//! ```

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use world_observer::{local_day, CatchUpPolicy, ObserverKey, ObserverStore, SittingLedger};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let library = PathBuf::from(
        args.next()
            .ok_or("usage: pretend_away <library-dir> <hours>")?,
    );
    let hours: u64 = args.next().ok_or("missing hours")?.parse()?;
    let root = library.parent().unwrap_or(&library).join("Observer");
    let store = ObserverStore::new(root);
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let then = now - hours * 60 * 60;
    let policy = CatchUpPolicy::new(1, 1)?;
    // The sitting is counted in days of the player's own, on their own
    // calendar; UTC's is at most a day off it, so a return of any length is
    // put a day further back than its hours.
    let today = local_day(now as i64, 0);
    let days = (hours / 24) as i64 + 1;
    for entry in std::fs::read_dir(&library)? {
        let path = entry?.path();
        let Some(id) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let key = ObserverKey::new(format!("library:{id}"))?;
        // A stamp newer than `then` is rewound; no stamp is written as `then`.
        store.claim_due(&key, then, policy)?;
        let ledger = store.ledger(&key)?;
        store.set_ledger(
            &key,
            &SittingLedger {
                sittings: ledger.sittings.max(1),
                began_on: Some(today - days),
                ..ledger
            },
        )?;
        println!("{id}: last seen {hours}h ago");
    }
    Ok(())
}
