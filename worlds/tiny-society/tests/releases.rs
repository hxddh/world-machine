//! The harbours each release saves, from World Machine 1.0 on, open,
//! replay and play on (see `world_pack_testkit::release` and
//! docs/SAVES.md). A release adds its own with
//! `WORLD_PACK_WRITE_RELEASE=v1.0.0 cargo test -p tiny-society --test releases -- --ignored`.

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    registry
}

world_pack_testkit::release_saves!(
    registry(),
    tiny_society::TINY_SOCIETY_PACK_ID,
    &[],
    "tiny-society.let-day-pass",
    "harbour",
);
