from pathlib import Path

ROOT = Path('.')


def replace_once(path: str, old: str, new: str) -> None:
    target = ROOT / path
    text = target.read_text()
    if text.count(old) != 1:
        raise SystemExit(f'{path}: expected exactly one match, found {text.count(old)}')
    target.write_text(text.replace(old, new, 1))


def replace_all_checked(path: str, old: str, new: str, expected: int) -> None:
    target = ROOT / path
    text = target.read_text()
    if text.count(old) != expected:
        raise SystemExit(f'{path}: expected {expected} matches, found {text.count(old)}')
    target.write_text(text.replace(old, new))


projection_path = 'worlds/pocket-universe/src/projection.rs'
projection = (ROOT / projection_path).read_text()

old_since = '''    if let Some(since) = since_event_count.filter(|since| *since < world.events().len()) {
        let events = &world.events()[since..];
        let mut items = events
            .iter()
            .rev()
            .filter(|event| event.kind != "agent_decision_recorded")
            .take(3)
            .map(return_item)
            .collect::<Vec<_>>();
        items.extend(persistent_consequence_items(world));
        return BriefingProjection {
            eyebrow: format!("Pocket Universe · {}", seed_label(seed_id(world))),
            title: "While you were away".into(),
            items,
        };
    }
'''
new_since = '''    if let Some(since) = since_event_count.filter(|since| *since < world.events().len()) {
        let events = &world.events()[since..];
        let mut items = return_digest_items(events);
        extend_with_persistent_consequences(world, &mut items);
        return BriefingProjection {
            eyebrow: format!("Pocket Universe · {}", seed_label(seed_id(world))),
            title: "While you were away".into(),
            items,
        };
    }
'''
if projection.count(old_since) != 1:
    raise SystemExit(f'{projection_path}: return block match count was {projection.count(old_since)}')
projection = projection.replace(old_since, new_since, 1)

old_return = '''fn return_item(event: &Event) -> BriefingItem {
    let detail = ["change", "summary"]
        .into_iter()
        .find_map(|key| match event.payload.get(key) {
            Some(Value::Text(value)) => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_else(|| event.kind.replace('_', " "));
    BriefingItem {
        selection: Some(SelectionId::Event(event.id)),
        title: match event.kind.as_str() {
            "universe_grew" => "The world moved".into(),
            "universe_intervened" => "Your choice took hold".into(),
            "universe_seeded" => "A world began".into(),
            "agent_cared_for_world" => "Someone cared for the world".into(),
            "agent_explored_world" => "Someone explored beyond routine".into(),
            "relationship_shifted" => "Their relationship changed".into(),
            "relationship_steered" => "You steered their relationship".into(),
            "partnership_formed" => "A partnership formed".into(),
            "relationship_fractured" => "Their relationship fractured".into(),
            "world_legacy_formed" => "A world legacy formed".into(),
            "legacy_reinforced" => "A legacy reinforced itself".into(),
            _ => event.kind.replace('_', " "),
        },
        detail,
    }
}
'''
new_return = '''fn return_digest_items(events: &[Event]) -> Vec<BriefingItem> {
    let mut groups = Vec::<(&Event, usize)>::new();
    for event in events
        .iter()
        .rev()
        .filter(|event| event.kind != "agent_decision_recorded")
    {
        if let Some((_, count)) = groups
            .iter_mut()
            .find(|(latest, _)| latest.kind == event.kind)
        {
            *count += 1;
        } else {
            groups.push((event, 1));
        }
    }

    groups
        .into_iter()
        .take(3)
        .map(|(event, occurrences)| return_item(event, occurrences))
        .collect()
}

fn extend_with_persistent_consequences(world: &World, items: &mut Vec<BriefingItem>) {
    let represented_events = items
        .iter()
        .filter_map(|item| match item.selection.as_ref() {
            Some(SelectionId::Event(event)) => Some(*event),
            _ => None,
        })
        .collect::<Vec<_>>();
    items.extend(
        persistent_consequence_items(world)
            .into_iter()
            .filter(|item| match item.selection.as_ref() {
                Some(SelectionId::Event(event)) => !represented_events.contains(event),
                _ => true,
            }),
    );
}

fn return_item(event: &Event, occurrences: usize) -> BriefingItem {
    let detail = ["change", "summary"]
        .into_iter()
        .find_map(|key| match event.payload.get(key) {
            Some(Value::Text(value)) => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_else(|| event.kind.replace('_', " "));
    let base_title: String = match event.kind.as_str() {
        "universe_grew" => "The world moved".into(),
        "universe_intervened" => "Your choice took hold".into(),
        "universe_seeded" => "A world began".into(),
        "agent_cared_for_world" => "Someone cared for the world".into(),
        "agent_explored_world" => "Someone explored beyond routine".into(),
        "relationship_shifted" => "Their relationship changed".into(),
        "relationship_steered" => "You steered their relationship".into(),
        "partnership_formed" => "A partnership formed".into(),
        "relationship_fractured" => "Their relationship fractured".into(),
        "world_legacy_formed" => "A world legacy formed".into(),
        "legacy_reinforced" => "A legacy reinforced itself".into(),
        _ => event.kind.replace('_', " "),
    };
    let title = if occurrences <= 1 {
        base_title
    } else {
        match event.kind.as_str() {
            "universe_grew" => format!("The world moved · {occurrences} cycles"),
            "legacy_reinforced" => {
                format!("A legacy reinforced itself · {occurrences} cycles")
            }
            "relationship_shifted" => {
                format!("Their relationship changed · {occurrences} times")
            }
            "agent_cared_for_world" => {
                format!("Someone cared for the world · {occurrences} times")
            }
            "agent_explored_world" => {
                format!("Someone explored beyond routine · {occurrences} times")
            }
            _ => format!("{base_title} · {occurrences} updates"),
        }
    };
    BriefingItem {
        selection: Some(SelectionId::Event(event.id)),
        title,
        detail,
    }
}
'''
if projection.count(old_return) != 1:
    raise SystemExit(f'{projection_path}: return_item match count was {projection.count(old_return)}')
projection = projection.replace(old_return, new_return, 1)
(ROOT / projection_path).write_text(projection)

# Release metadata.
replace_once(
    'worlds/pocket-universe/src/lib.rs',
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.14.1";',
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.14.2";',
)
replace_once(
    'worlds/pocket-universe/Cargo.toml',
    'version = "0.14.1"',
    'version = "0.14.2"',
)
replace_once(
    'apps/pocket-universe-pack/Cargo.toml',
    'version = "0.14.1"',
    'version = "0.14.2"',
)
replace_all_checked(
    'apps/world-machine-desktop/src/included_packs.rs',
    '0.14.1',
    '0.14.2',
    2,
)
replace_all_checked('Cargo.lock', 'version = "0.14.1"', 'version = "0.14.2"', 2)

# End-to-end return digest regression.
test_path = ROOT / 'worlds/pocket-universe/tests/return_digest.rs'
test_path.write_text(r'''use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, OUTWARD_POSTURE_COMMAND, SEED_MARS_COLONY_COMMAND,
    SHARED_PROJECT_COMMAND,
};
use std::collections::BTreeSet;
use std::error::Error;
use world_projection::SelectionId;

#[test]
fn return_digest_groups_repeated_event_kinds() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    let since = universe.world().events().len();

    universe.advance_periods(3)?;
    let snapshot = universe.projection_snapshot_since(Some(since));
    let briefing = snapshot
        .briefing
        .as_ref()
        .expect("Pocket Universe should expose a return Briefing");
    assert_eq!(briefing.title, "While you were away");

    let event_items = briefing
        .items
        .iter()
        .filter_map(|item| match item.selection {
            Some(SelectionId::Event(event)) => Some((item, event)),
            _ => None,
        })
        .take(3)
        .collect::<Vec<_>>();
    assert_eq!(event_items.len(), 3, "the return digest stays bounded");

    let kinds = event_items
        .iter()
        .map(|(_, event_id)| {
            universe
                .world()
                .events()
                .iter()
                .find(|event| event.id == *event_id)
                .expect("return item should select a real event")
                .kind
                .clone()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        kinds.len(),
        event_items.len(),
        "repeated event kinds should collapse to their latest semantic update"
    );
    assert!(
        event_items
            .iter()
            .any(|(item, _)| item.title.contains("· 3 times")),
        "the digest should say how often a repeated change happened while away"
    );

    Ok(())
}

#[test]
fn return_digest_does_not_repeat_the_same_living_legacy_event() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    universe.advance_periods(1)?;
    universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    universe.advance_periods(3)?;
    universe.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;
    universe.advance_periods(3)?;

    let since = universe.world().events().len();
    universe.advance_periods(1)?;
    let reinforced = universe
        .world()
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "legacy_reinforced")
        .expect("the next period should reinforce the legacy");

    let returned = universe.projection_snapshot_since(Some(since));
    let returned_items = &returned
        .briefing
        .as_ref()
        .expect("returning World should keep its Briefing")
        .items;
    let represented = returned_items
        .iter()
        .filter(|item| item.selection == Some(SelectionId::Event(reinforced.id)))
        .count();
    assert_eq!(
        represented, 1,
        "the latest reinforcement should not be repeated as both a return event and persistent consequence"
    );

    let live = universe.projection_snapshot();
    assert!(
        live.briefing
            .as_ref()
            .expect("live World should keep its Briefing")
            .items
            .iter()
            .any(|item| item.title == "World legacy · Ridge Network"),
        "deduplication is return-only; the persistent living Legacy remains on the normal Briefing"
    );

    Ok(())
}
''')
