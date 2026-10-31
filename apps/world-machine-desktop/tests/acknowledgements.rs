//! The third-party notices the app shows (`acknowledgements.tsv`, written
//! by `scripts/generate-acknowledgements.py`) are up to date with
//! `Cargo.lock`: every third-party crate the lock names is listed at its
//! version, and nothing the lock no longer has. A dependency changed
//! without running the script again fails here.

use std::collections::BTreeSet;
use std::path::Path;
use world_machine_desktop::notices;

/// Every crate with a source (crates.io or git) in a `Cargo.lock`, as
/// name and version. Workspace members have no source.
fn locked(lock: &str) -> BTreeSet<(String, String)> {
    let mut found = BTreeSet::new();
    for block in lock.split("[[package]]").skip(1) {
        let field = |key: &str| {
            block.lines().find_map(|line| {
                line.strip_prefix(key)
                    .and_then(|rest| rest.strip_prefix(" = \""))
                    .and_then(|rest| rest.strip_suffix('"'))
                    .map(str::to_string)
            })
        };
        if field("source").is_none() {
            continue;
        }
        if let (Some(name), Some(version)) = (field("name"), field("version")) {
            found.insert((name, version));
        }
    }
    found
}

fn listed(text: &str) -> BTreeSet<(String, String)> {
    notices::parse_acknowledgements(text)
        .into_iter()
        .map(|notice| (notice.name, notice.version))
        .collect()
}

#[test]
fn the_notices_list_every_crate_in_cargo_lock() {
    let lock =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock"))
            .expect("the workspace's Cargo.lock");
    let locked = locked(&lock);
    let listed = listed(notices::ACKNOWLEDGEMENTS);
    assert!(
        locked.len() > 100,
        "{} crates read from Cargo.lock",
        locked.len()
    );
    let missing: Vec<_> = locked.difference(&listed).collect();
    let gone: Vec<_> = listed.difference(&locked).collect();
    assert!(
        missing.is_empty() && gone.is_empty(),
        "apps/world-machine-desktop/acknowledgements.tsv is stale; run \
         python3 scripts/generate-acknowledgements.py\n\
         in Cargo.lock but not listed: {missing:?}\n\
         listed but not in Cargo.lock: {gone:?}"
    );
    // Every crate says under what licence it is used.
    for notice in notices::acknowledgements() {
        assert!(
            !notice.licence.is_empty() && notice.licence != "unknown",
            "{} {} has no licence",
            notice.name,
            notice.version
        );
    }
}

/// The check finds a stale list: a crate added to the lock, or one taken
/// out of it.
#[test]
fn a_stale_list_is_found() {
    let lock = "version = 4\n\n[[package]]\nname = \"ours\"\nversion = \"1.0.0\"\n\n\
                [[package]]\nname = \"serde\"\nversion = \"1.0.1\"\n\
                source = \"registry+https://github.com/rust-lang/crates.io-index\"\n";
    assert_eq!(
        locked(lock),
        BTreeSet::from([("serde".to_string(), "1.0.1".to_string())]),
        "a workspace member is not third-party"
    );
    let stale = "# header\nserde\t1.0.0\tMIT OR Apache-2.0\thttps://serde.rs\n";
    assert_ne!(locked(lock), listed(stale));
}
