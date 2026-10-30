//! The places each release saves, from World Machine 1.0 on, open, replay
//! and play on (see `world_pack_testkit::release` and docs/SAVES.md). A
//! release adds its own with
//! `WORLD_PACK_WRITE_RELEASE=v1.0.0 cargo test -p pocket-universe --test releases -- --ignored`.

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    registry
}

world_pack_testkit::release_saves!(
    registry(),
    pocket_universe::POCKET_UNIVERSE_PACK_ID,
    &[pocket_universe::SEED_1980S_TOWN_COMMAND],
    pocket_universe::NUDGE_COMMAND,
    "maple",
);
