from pathlib import Path


def replace_exact(path: str, old: str, new: str, expected: int = 1) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != expected:
        raise SystemExit(f"{path}: expected {expected} matches for {old!r}, found {count}")
    file.write_text(text.replace(old, new))


replace_exact(
    "worlds/pocket-universe/src/lib.rs",
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.12.0";',
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.13.0";',
)
replace_exact(
    "worlds/pocket-universe/Cargo.toml",
    'version = "0.12.0"',
    'version = "0.13.0"',
)
replace_exact(
    "apps/pocket-universe-pack/Cargo.toml",
    'version = "0.12.0"',
    'version = "0.13.0"',
)
replace_exact(
    "apps/world-machine-desktop/src/included_packs.rs",
    'version: "0.12.0"',
    'version: "0.13.0"',
    expected=2,
)
replace_exact(
    "apps/world-machine-desktop/src/included_packs.rs",
    'description: "Seed a tiny persistent world, let its inhabitants act, and watch relationships turn into durable consequences."',
    'description: "Seed a tiny persistent world, let its inhabitants act, and watch choices, relationships, and repeated behavior compound into durable legacies."',
)
