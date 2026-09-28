use pocket_universe::{PocketUniverse, SEED_MARS_COLONY_COMMAND};
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
            .all(|(item, _)| !item.title.contains(" times") && !item.title.contains(" updates")),
        "a return says what happened, not how many times the routine ran"
    );

    Ok(())
}
