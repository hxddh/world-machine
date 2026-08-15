use pocket_universe::{
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
