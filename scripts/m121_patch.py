from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    if old not in text:
        raise SystemExit(f"missing expected text in {path}: {old[:80]!r}")
    file.write_text(text.replace(old, new, 1))


def replace_all(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    if old not in text:
        raise SystemExit(f"missing expected text in {path}: {old!r}")
    file.write_text(text.replace(old, new))


projection = Path("worlds/pocket-universe/src/projection.rs")
text = projection.read_text()
start = text.index("fn legacy_consequence_item(world: &World) -> Option<BriefingItem> {")
end = text.index("\nfn legacy_label(legacy: &str) -> String {", start)
new_fn = '''fn legacy_consequence_item(world: &World) -> Option<BriefingItem> {
    let legacy = text_component(world.state().entity(UNIVERSE), LEGACY, "forming");
    if legacy == "forming" {
        return None;
    }

    let latest_reinforcement = world
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "legacy_reinforced");
    let summary = latest_reinforcement
        .and_then(|event| match event.payload.get("summary") {
            Some(Value::Text(summary)) => Some(summary.clone()),
            _ => None,
        })
        .unwrap_or_else(|| {
            text_component(
                world.state().entity(UNIVERSE),
                LEGACY_SUMMARY,
                "This World now carries a durable legacy from its earlier choices.",
            )
        });
    let selection = latest_reinforcement
        .map(|event| SelectionId::Event(event.id))
        .or_else(|| {
            world
                .events()
                .iter()
                .rev()
                .find(|event| event.kind == "world_legacy_formed")
                .map(|event| SelectionId::Event(event.id))
        })
        .unwrap_or(SelectionId::Entity(UNIVERSE));
    Some(BriefingItem {
        selection: Some(selection),
        title: format!("World legacy · {}", legacy_label(&legacy)),
        detail: summary,
    })
}
'''
projection.write_text(text[:start] + new_fn + text[end:])

replace_once(
    "worlds/pocket-universe/src/projection.rs",
    '            "world_legacy_formed" => "A world legacy formed".into(),\n',
    '            "world_legacy_formed" => "A world legacy formed".into(),\n            "legacy_reinforced" => "A legacy reinforced itself".into(),\n',
)

replace_once(
    "worlds/pocket-universe/tests/legacy_why.rs",
    "    let reinforced_snapshot = reopened.projection_snapshot();\n    let reinforced_why = reinforced_snapshot\n",
    '''    let reinforced_snapshot = reopened.projection_snapshot();
    let reinforced_legacy_item = reinforced_snapshot
        .briefing
        .as_ref()
        .expect("reinforced Pocket Universe should keep its Briefing")
        .items
        .iter()
        .find(|item| item.title == "World legacy · Ridge Network")
        .expect("the living legacy should remain visible in Briefing");
    assert_eq!(
        reinforced_legacy_item.selection,
        Some(SelectionId::Event(reinforced_event_id)),
        "after reinforcement, the persistent legacy should open its latest living event"
    );
    assert!(
        reinforced_legacy_item.detail.contains("Legacy cycle 1"),
        "the persistent legacy should describe its latest durable feedback cycle"
    );

    let reinforced_why = reinforced_snapshot
''',
)

replace_once(
    "worlds/pocket-universe/tests/legacy_why.rs",
    '''    let reopened_again = PocketUniverse::resume_archive(&reinforced_archive)?;
    assert_eq!(
        reopened_again
            .projection_snapshot()
            .why(reinforced_event_id),
        Some(reinforced_why),
        "archive/reopen should preserve the reinforcement explanation"
    );
''',
    '''    let reopened_again = PocketUniverse::resume_archive(&reinforced_archive)?;
    let reopened_again_snapshot = reopened_again.projection_snapshot();
    let reopened_again_legacy = reopened_again_snapshot
        .briefing
        .as_ref()
        .expect("reopened reinforced World should keep its Briefing")
        .items
        .iter()
        .find(|item| item.title == "World legacy · Ridge Network")
        .expect("reopened reinforced World should keep its living legacy entrypoint");
    assert_eq!(reopened_again_legacy, reinforced_legacy_item);
    assert_eq!(
        reopened_again_snapshot.why(reinforced_event_id),
        Some(reinforced_why),
        "archive/reopen should preserve the reinforcement explanation"
    );
''',
)

replace_all("worlds/pocket-universe/Cargo.toml", 'version = "0.14.0"', 'version = "0.14.1"')
replace_all("apps/pocket-universe-pack/Cargo.toml", 'version = "0.14.0"', 'version = "0.14.1"')
replace_once(
    "worlds/pocket-universe/src/lib.rs",
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.14.0";',
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.14.1";',
)
replace_all(
    "apps/world-machine-desktop/src/included_packs.rs",
    '"0.14.0"',
    '"0.14.1"',
)
