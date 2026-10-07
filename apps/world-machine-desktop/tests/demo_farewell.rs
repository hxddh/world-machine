//! The demo's farewell, read from a real harbour at its last day: a warm
//! newcomer's 21 days (the first answer every day, something made by hand
//! every third), then what the farewell recaps and who says goodbye.

use world_machine_desktop::demo;
use world_projection::ProjectionIntent;

#[test]
fn the_farewell_recaps_four_to_six_of_the_players_own_moments() {
    let registry = world_builtins::registry().unwrap();
    let mut session = registry.create(demo::PACK_ID).unwrap();
    let mut snapshot = session.snapshot();
    let length = snapshot.calendar.as_ref().expect("a calendar").length;
    let mut day = demo::day_of(snapshot.world_time, length);
    while day < demo::LAST_DAY {
        if let Some(answer) = snapshot.commands.iter().find(|command| {
            command.question.is_some()
                && command.unavailable.is_none()
                && command.id != demo::DAY_PASS_COMMAND
        }) {
            if let Ok(next) = session.handle(ProjectionIntent::InvokeCommand(answer.id.clone())) {
                snapshot = next;
            }
        }
        if day.is_multiple_of(3) {
            if let Some(deed) = snapshot
                .commands
                .iter()
                .find(|command| command.hand.is_some() && command.unavailable.is_none())
            {
                let _ = session.handle(ProjectionIntent::InvokeCommand(deed.id.clone()));
            }
        }
        snapshot = session
            .handle(ProjectionIntent::InvokeCommand(
                demo::DAY_PASS_COMMAND.into(),
            ))
            .unwrap();
        day = demo::day_of(snapshot.world_time, length);
    }
    // The next day is what the demo holds back.
    assert_eq!(
        demo::gate(demo::PACK_ID, Some(day), demo::DAY_PASS_COMMAND),
        demo::Gate::Ending
    );
    let recap = demo::recap(&snapshot);
    eprintln!("recap on day {day}: {recap:#?}");
    assert!(
        (demo::RECAP_LEAST..=demo::RECAP_MOST).contains(&recap.len()),
        "{} lines: {recap:?}",
        recap.len()
    );
    for line in &recap {
        assert!(!line.trim().is_empty());
        assert!(!line.contains('{'), "{line}");
    }
    let goodbye = demo::goodbye_from(&snapshot).expect("someone says goodbye");
    assert!(snapshot.canvas.items.iter().any(|item| item.id == goodbye));
}
