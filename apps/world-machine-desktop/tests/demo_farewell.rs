//! The demo's farewell, read from a real harbour at its last day: a warm
//! newcomer's days to the last (the first answer every day, something new
//! made by hand every other day), then what the farewell recaps and who
//! says goodbye.

use world_machine_desktop::demo;
use world_projection::ProjectionIntent;

#[test]
fn the_farewell_recaps_four_to_six_of_the_players_own_deeds() {
    let registry = world_builtins::registry().unwrap();
    let mut session = registry.create(demo::PACK_ID).unwrap();
    let mut snapshot = session.snapshot();
    let length = snapshot.calendar.as_ref().expect("a calendar").length;
    let mut day = demo::day_of(snapshot.world_time, length);
    let pass = demo::day_pass(&snapshot).expect("a day to pass").id.clone();
    let mut made = Vec::new();
    while day < demo::last_day() {
        if let Some(answer) = snapshot.commands.iter().find(|command| {
            command.question.is_some() && command.unavailable.is_none() && command.id != pass
        }) {
            if let Ok(next) = session.handle(ProjectionIntent::InvokeCommand(answer.id.clone())) {
                snapshot = next;
            }
        }
        if day.is_multiple_of(2) {
            if let Some(deed) = snapshot.commands.iter().find(|command| {
                command.unavailable.is_none()
                    && command
                        .hand
                        .as_ref()
                        .is_some_and(|hand| hand.verb == "Build" && !made.contains(&hand.thing))
            }) {
                made.push(deed.hand.as_ref().unwrap().thing.clone());
                let _ = session.handle(ProjectionIntent::InvokeCommand(deed.id.clone()));
            }
        }
        snapshot = session
            .handle(ProjectionIntent::InvokeCommand(pass.clone()))
            .unwrap();
        day = demo::day_of(snapshot.world_time, length);
    }
    // The next day is what the demo holds back.
    assert_eq!(
        demo::gate(
            demo::PACK_ID,
            Some(day),
            demo::passes_time(&snapshot, &pass)
        ),
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
        // The player's own, never a season's or the weather's title, nor
        // a keepsake's note torn from it (v0.28).
        assert!(
            !snapshot
                .moments
                .iter()
                .any(|moment| line.trim_end_matches('.') == moment.title),
            "{line}"
        );
        assert!(
            !snapshot.keepsakes.iter().any(|kept| *line == kept.note),
            "{line}"
        );
        assert!(line.contains("you") || line.starts_with("You "), "{line}");
    }
    assert!(
        recap.iter().any(|line| line.starts_with("You ")),
        "something the player did: {recap:?}"
    );
    let goodbye = demo::goodbye_from(&snapshot).expect("someone says goodbye");
    assert!(snapshot.canvas.items.iter().any(|item| item.id == goodbye));
}
