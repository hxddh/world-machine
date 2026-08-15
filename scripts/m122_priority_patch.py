from pathlib import Path

projection_path = Path('worlds/pocket-universe/src/projection.rs')
projection = projection_path.read_text()
old = '''    groups
        .into_iter()
        .take(3)
        .map(|(event, occurrences)| return_item(event, occurrences))
        .collect()
}

fn extend_with_persistent_consequences(world: &World, items: &mut Vec<BriefingItem>) {
'''
new = '''    groups.sort_by_key(|(event, _)| return_digest_priority(event.kind.as_str()));
    groups
        .into_iter()
        .take(3)
        .map(|(event, occurrences)| return_item(event, occurrences))
        .collect()
}

fn return_digest_priority(kind: &str) -> u8 {
    match kind {
        "universe_seeded"
        | "universe_intervened"
        | "relationship_steered"
        | "partnership_formed"
        | "relationship_fractured"
        | "world_legacy_formed" => 0,
        _ => 1,
    }
}

fn extend_with_persistent_consequences(world: &World, items: &mut Vec<BriefingItem>) {
'''
if projection.count(old) != 1:
    raise SystemExit(f'projection milestone insertion expected one match, found {projection.count(old)}')
projection_path.write_text(projection.replace(old, new, 1))

test_path = Path('worlds/pocket-universe/tests/return_digest.rs')
test = test_path.read_text()
addition = r'''

#[test]
fn return_digest_promotes_a_milestone_above_later_routine_churn() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    universe.advance_periods(1)?;
    universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    universe.advance_periods(3)?;
    universe.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;
    universe.advance_periods(2)?;

    let since = universe.world().events().len();
    universe.advance_periods(2)?;
    let legacy_formed = universe
        .world()
        .events()
        .iter()
        .skip(since)
        .find(|event| event.kind == "world_legacy_formed")
        .expect("the first unseen period should form the legacy");
    assert!(
        universe
            .world()
            .events()
            .iter()
            .skip(since)
            .any(|event| event.kind == "legacy_reinforced"),
        "a later unseen period should add routine legacy feedback after the milestone"
    );

    let returned = universe.projection_snapshot_since(Some(since));
    let event_items = returned
        .briefing
        .as_ref()
        .expect("returning World should keep its Briefing")
        .items
        .iter()
        .filter_map(|item| match item.selection {
            Some(SelectionId::Event(event)) => Some(event),
            _ => None,
        })
        .take(3)
        .collect::<Vec<_>>();
    assert_eq!(event_items.len(), 3, "the return digest remains bounded");
    assert!(
        event_items.contains(&legacy_formed.id),
        "a durable milestone should stay in the bounded digest even when later routine event kinds exist"
    );

    Ok(())
}
'''
if 'fn return_digest_promotes_a_milestone_above_later_routine_churn' in test:
    raise SystemExit('milestone regression already exists')
test_path.write_text(test.rstrip() + addition + '\n')
