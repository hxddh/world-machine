//! Backdates the sitting ledger every World in a library carries in its
//! file (host metadata, never its history), so the next open plays as a new
//! sitting after `hours` away (at least a day). For previews and demos; run
//! it while the app is closed.
//!
//! ```text
//! cargo run -p world-library --example pretend_away -- <library-dir> <hours>
//! cargo run -p world-library --example pretend_away -- --show <library-dir> sittings|began|days|morning
//! ```
//!
//! `--show` prints one field of the ledger of the one World in the
//! library (or `?<n> ledgers` when there are more), for the release-shots
//! harness.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use world_document::{DocumentSittings, WorldDocument};

/// The Worlds in `library`: its `.world` files (a `.world.bak` beside one
/// is not a World of its own).
fn worlds(library: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(library)? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("world") {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

fn show(library: &Path, field: &str) -> Result<(), Box<dyn std::error::Error>> {
    let worlds = worlds(library)?;
    let ledgers = worlds
        .iter()
        .map(|path| {
            Ok(WorldDocument::from_bytes(&std::fs::read(path)?)?
                .metadata
                .host_sittings)
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    match ledgers.as_slice() {
        [] => {}
        [ledger] => match field {
            "sittings" => println!("{}", ledger.sittings),
            "began" => println!(
                "{}",
                ledger.began_on.map_or("-".into(), |day| day.to_string())
            ),
            "days" => println!("{}", ledger.days_passed),
            "morning" => {
                if let Some(moment) = &ledger.morning_read {
                    println!("{moment}");
                }
            }
            other => return Err(format!("no ledger field {other}").into()),
        },
        many => println!("?{} ledgers", many.len()),
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("--show") {
        let [_, library, field] = args.as_slice() else {
            return Err("usage: pretend_away --show <library-dir> <field>".into());
        };
        return show(Path::new(library), field);
    }
    let usage = "usage: pretend_away <library-dir> <hours>";
    let hours: u64 = args.pop().ok_or(usage)?.parse()?;
    let library = PathBuf::from(args.pop().ok_or(usage)?);
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
    // The sitting is counted in days of the player's own, on their own
    // calendar; UTC's is at most a day off it, so a return of any length is
    // put a day further back than its hours.
    let today = now.div_euclid(24 * 60 * 60);
    let days = (hours / 24) as i64 + 1;
    for path in worlds(&library)? {
        let mut document = WorldDocument::from_bytes(&std::fs::read(&path)?)?;
        let ledger = document.metadata.host_sittings.take().unwrap_or_default();
        document.metadata.host_sittings = Some(DocumentSittings {
            sittings: ledger.sittings.max(1),
            began_on: Some(today - days),
            ..ledger
        });
        std::fs::write(&path, document.to_bytes()?)?;
        println!(
            "{}: last sat with {hours}h ago",
            path.file_stem().unwrap_or_default().to_string_lossy()
        );
    }
    Ok(())
}
