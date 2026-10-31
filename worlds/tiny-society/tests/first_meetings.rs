//! Greetings remember first meetings (v0.31 play): on day 4 Mia, who had
//! asked the player a favour and watered their wildflowers overnight, came
//! over to say "We haven't met properly. I'm Mia." Played through sittings
//! as the app plays them (a morning each, the day's question answered),
//! nobody says hello as to a stranger once they have written to the
//! player, asked them a favour, looked after something of theirs, or met
//! them any other way.

use std::collections::BTreeMap;
use tiny_society::TinySocietyBranch;
use world_projection::{CommandRole, SITTING_BEGINS_COMMAND};

#[test]
fn nobody_greets_the_player_as_a_stranger_after_meeting_them() {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    // A builder's first month: a sitting a day, the day's question answered
    // each way in turn, something planted on the first.
    let free = branch
        .projection_snapshot()
        .commands
        .into_iter()
        .find(|command| {
            command.unavailable.is_none()
                && command
                    .hand
                    .as_ref()
                    .is_some_and(|hand| hand.verb == "Plant" && hand.cost.is_none())
        })
        .map(|command| format!("{}@50", command.id));
    if let Some(free) = free {
        branch.invoke_projection_command(&free).unwrap();
    }
    for day in 0..30 {
        let _ = branch.invoke_projection_command(SITTING_BEGINS_COMMAND);
        let snapshot = branch.projection_snapshot();
        let answers = snapshot
            .commands
            .iter()
            .filter(|command| command.question.is_some() && command.unavailable.is_none())
            .map(|command| command.id.clone())
            .collect::<Vec<_>>();
        if !answers.is_empty() {
            let _ = branch.invoke_projection_command(&answers[day % answers.len()]);
        }
        let pass = branch
            .projection_snapshot()
            .commands
            .into_iter()
            .find(|command| command.role == Some(CommandRole::PassesTime))
            .expect("the day's card");
        branch.invoke_projection_command(&pass.id).unwrap();
    }
    // When each person first crossed the player's path or met them.
    const STRANGERS_HELLO: [&str; 4] = [
        "We haven't met properly.",
        "I've seen you about.",
        "I hear you're new round here.",
        "I've been meaning to say hello.",
    ];
    const MET: [&str; 5] = [
        "greeted",
        "warmed",
        "reacted",
        "situation_came_up",
        "keepsake_left",
    ];
    let mut first_met = BTreeMap::new();
    let mut wrong = Vec::new();
    let mut known_greeted = 0;
    for event in branch.world().events() {
        let Some(actor) = event.actor else {
            continue;
        };
        if event.kind == "greeted" {
            let said = match event.payload.get("said") {
                Some(world_core::Value::Text(said)) => said.clone(),
                _ => String::new(),
            };
            if let Some(met) = first_met.get(&actor) {
                known_greeted += 1;
                if STRANGERS_HELLO.iter().any(|line| said.contains(line)) {
                    wrong.push(format!(
                        "{actor:?} at {}, known since {met}: {said}",
                        event.world_time
                    ));
                }
            }
        }
        if lives::CROSSED_KINDS.contains(&event.kind.as_str()) || MET.contains(&event.kind.as_str())
        {
            first_met
                .entry(actor)
                .or_insert(format!("{} ({})", event.world_time, event.kind));
        }
    }
    assert!(known_greeted > 0, "someone known was greeted later on");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
