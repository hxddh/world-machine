from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one match, found {count}")
    file.write_text(text.replace(old, new, 1))


replace_once(
    "worlds/pocket-universe/src/projection.rs",
    '''    Some(BriefingItem {
        selection: Some(SelectionId::Entity(UNIVERSE)),
        title: format!("World legacy · {}", legacy_label(&legacy)),
        detail: summary,
    })
}''',
    '''    let selection = world
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "world_legacy_formed")
        .map(|event| SelectionId::Event(event.id))
        .unwrap_or(SelectionId::Entity(UNIVERSE));
    Some(BriefingItem {
        selection: Some(selection),
        title: format!("World legacy · {}", legacy_label(&legacy)),
        detail: summary,
    })
}''',
)

replace_once(
    "worlds/pocket-universe/src/lib.rs",
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.13.0";',
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.13.1";',
)
replace_once(
    "worlds/pocket-universe/Cargo.toml",
    'version = "0.13.0"',
    'version = "0.13.1"',
)
replace_once(
    "apps/pocket-universe-pack/Cargo.toml",
    'version = "0.13.0"',
    'version = "0.13.1"',
)
replace_once(
    "apps/world-machine-desktop/src/included_packs.rs",
    'version: "0.13.0"',
    'version: "0.13.1"',
)
replace_once(
    "apps/world-machine-desktop/src/included_packs.rs",
    'assert_eq!(packs[0].pack.version, "0.13.0");',
    'assert_eq!(packs[0].pack.version, "0.13.1");',
)
