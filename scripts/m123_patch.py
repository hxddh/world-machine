from pathlib import Path

ROOT = Path('.')


def replace_once(path: str, old: str, new: str) -> None:
    target = ROOT / path
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f'{path}: expected exactly one match, found {count}')
    target.write_text(text.replace(old, new, 1))


projection_path = 'worlds/pocket-universe/src/projection.rs'
projection = (ROOT / projection_path).read_text()

old_return = '''    if let Some(since) = since_event_count.filter(|since| *since < world.events().len()) {
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
new_return = '''    if let Some(since) = since_event_count.filter(|since| *since < world.events().len()) {
        let events = &world.events()[since..];
        let mut items = return_digest_items(events);
        items.push(return_compass_item(world));
        extend_with_persistent_consequences(world, &mut items);
        return BriefingProjection {
            eyebrow: format!("Pocket Universe · {}", seed_label(seed_id(world))),
            title: "While you were away".into(),
            items,
        };
    }
'''
if projection.count(old_return) != 1:
    raise SystemExit(f'{projection_path}: return block match count was {projection.count(old_return)}')
projection = projection.replace(old_return, new_return, 1)

anchor = '''fn return_digest_items(events: &[Event]) -> Vec<BriefingItem> {
'''
helper = '''fn return_compass_item(world: &World) -> BriefingItem {
    let generation = integer_component(world, GENERATION).unwrap_or_default();
    let (relationship_choice_available, intervention_choice_available) =
        choice_state(world, generation);
    let posture_choice_available = posture_choice_state(world, generation);
    let legacy = text_component(world.state().entity(UNIVERSE), LEGACY, "forming");
    let available = commands(world, true);
    let nudge = available
        .iter()
        .find(|command| command.id == NUDGE_COMMAND);
    let shaping = available
        .iter()
        .filter(|command| command.id != NUDGE_COMMAND)
        .collect::<Vec<_>>();

    let title = if posture_choice_available {
        "Your turn · World direction"
    } else if relationship_choice_available && intervention_choice_available {
        "Your turn · Shape the world"
    } else if relationship_choice_available {
        "Your turn · Relationship"
    } else if intervention_choice_available {
        "Your turn · Future"
    } else if legacy != "forming" {
        "Next · Living legacy"
    } else {
        "Next · Continue"
    };

    let detail = match (shaping.is_empty(), nudge) {
        (true, Some(nudge)) => format!("Continue with ‘{}’. {}", nudge.title, nudge.detail),
        (false, Some(nudge)) => {
            let choices = shaping
                .iter()
                .map(|command| format!("‘{}’", command.title))
                .collect::<Vec<_>>()
                .join(" · ");
            format!(
                "Available now: {choices}. Or choose ‘{}’ and let current dynamics keep moving without a larger choice.",
                nudge.title
            )
        }
        (_, None) => "This World can continue from its current durable state.".into(),
    };

    BriefingItem {
        selection: None,
        title: title.into(),
        detail,
    }
}

'''
if projection.count(anchor) != 1:
    raise SystemExit(f'{projection_path}: return digest anchor match count was {projection.count(anchor)}')
projection = projection.replace(anchor, helper + anchor, 1)
(ROOT / projection_path).write_text(projection)

# Release metadata.
replace_once(
    'worlds/pocket-universe/src/lib.rs',
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.14.2";',
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.14.3";',
)
replace_once(
    'worlds/pocket-universe/Cargo.toml',
    'version = "0.14.2"',
    'version = "0.14.3"',
)
replace_once(
    'apps/pocket-universe-pack/Cargo.toml',
    'version = "0.14.2"',
    'version = "0.14.3"',
)

included_path = ROOT / 'apps/world-machine-desktop/src/included_packs.rs'
included = included_path.read_text()
if included.count('0.14.2') != 2:
    raise SystemExit(f'included_packs.rs: expected 2 version matches, found {included.count("0.14.2")}')
included_path.write_text(included.replace('0.14.2', '0.14.3'))

lock_path = ROOT / 'Cargo.lock'
lock = lock_path.read_text()
for package in ('pocket-universe', 'pocket-universe-pack'):
    old = f'[[package]]\nname = "{package}"\nversion = "0.14.2"'
    new = f'[[package]]\nname = "{package}"\nversion = "0.14.3"'
    if lock.count(old) != 1:
        raise SystemExit(f'Cargo.lock: expected one {package} block, found {lock.count(old)}')
    lock = lock.replace(old, new, 1)
lock_path.write_text(lock)

# End-to-end return compass regressions.
test_path = ROOT / 'worlds/pocket-universe/tests/return_compass.rs'
test_path.write_text(r'''use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, OUTWARD_POSTURE_COMMAND, SEED_MARS_COLONY_COMMAND,
    SHARED_PROJECT_COMMAND,
};
use std::error::Error;

#[test]
fn return_compass_names_every_current_relationship_action() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    let since = universe.world().events().len();
    universe.advance_periods(2)?;

    let snapshot = universe.projection_snapshot_since(Some(since));
    let briefing = snapshot
        .briefing
        .as_ref()
        .expect("returning Pocket Universe should expose a Briefing");
    let compass = briefing
        .items
        .iter()
        .find(|item| item.title == "Your turn · Relationship")
        .expect("the return digest should surface the currently open relationship choice");

    assert_eq!(snapshot.commands.len(), 3, "nudge plus the two relationship choices");
    for command in &snapshot.commands {
        assert!(
            compass.detail.contains(&command.title),
            "the return compass must be generated from the same current command titles: {}",
            command.title
        );
    }

    Ok(())
}

#[test]
fn return_compass_surfaces_all_simultaneously_open_shaping_choices() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    let since = universe.world().events().len();
    universe.advance_periods(3)?;

    let snapshot = universe.projection_snapshot_since(Some(since));
    let compass = snapshot
        .briefing
        .as_ref()
        .expect("returning Pocket Universe should expose a Briefing")
        .items
        .iter()
        .find(|item| item.title == "Your turn · Shape the world")
        .expect("relationship and intervention choices should be summarized together");

    assert_eq!(
        snapshot.commands.len(),
        5,
        "one nudge plus two relationship and two intervention choices should be open"
    );
    for command in &snapshot.commands {
        assert!(
            compass.detail.contains(&command.title),
            "every actually available action should appear in the return compass: {}",
            command.title
        );
    }

    Ok(())
}

#[test]
fn return_compass_explains_how_to_continue_a_living_legacy() -> Result<(), Box<dyn Error>> {
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
    let snapshot = universe.projection_snapshot_since(Some(since));
    let compass = snapshot
        .briefing
        .as_ref()
        .expect("returning Pocket Universe should expose a Briefing")
        .items
        .iter()
        .find(|item| item.title == "Next · Living legacy")
        .expect("a mature World should explain why another cycle is meaningful");

    assert_eq!(snapshot.commands.len(), 1, "a mature legacy has one continuation command");
    let continuation = &snapshot.commands[0];
    assert!(compass.detail.contains(&continuation.title));
    assert!(
        compass.detail.contains(&continuation.detail),
        "when continuation is the only action, the compass should reuse its semantic explanation"
    );

    Ok(())
}
''')
