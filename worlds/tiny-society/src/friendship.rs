//! The v0.15 bar in the harbour: a friendship opens doors and a grudge
//! closes them, people remember what the player did, what the player made
//! is named as theirs, and every return brings something to keep.

use crate::{speech, story, TinySociety, TinySocietyBranch, HARBOR, LEO, MARA};
use world_core::{EntityId, Value};
use world_projection::{ProjectionCommand, SelectionId};

fn opened() -> TinySocietyBranch {
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    branch
        .invoke_projection_command(story::WAIT_COMMAND)
        .unwrap();
    branch
}

fn next_day(branch: &mut TinySocietyBranch) {
    branch
        .invoke_projection_command(story::WAIT_COMMAND)
        .unwrap();
}

fn commands(branch: &TinySocietyBranch) -> Vec<ProjectionCommand> {
    branch.projection_snapshot().commands
}

/// The first answer that can be given to each question someone is asking.
fn first_answers(branch: &TinySocietyBranch, asker: EntityId) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    commands(branch)
        .into_iter()
        .filter(|command| {
            command.asker == Some(SelectionId::Entity(asker)) && command.unavailable.is_none()
        })
        .filter_map(|command| {
            let question = command.question.as_ref()?.id.clone();
            seen.insert(question).then_some(command.id)
        })
        .collect()
}

/// The situations people put to the player, by kind and asker.
fn came_up(events: &[world_core::Event]) -> Vec<(String, EntityId)> {
    events
        .iter()
        .filter(|event| event.kind == "situation_came_up")
        .filter_map(|event| match (event.payload.get("kind"), event.actor) {
            (Some(Value::Text(kind)), Some(who)) => Some((kind.clone(), who)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_warm_friendship_opens_doors_once_each() {
    let mut branch = opened();
    for _ in 0..180 {
        branch.say(MARA, "your bread is wonderful").unwrap();
        let gift = commands(&branch).into_iter().find(|command| {
            command.unavailable.is_none()
                && command.hand.as_ref().is_some_and(|hand| {
                    hand.verb == "Give" && hand.at == Some(SelectionId::Entity(MARA))
                })
        });
        if let Some(gift) = gift {
            branch.invoke_projection_command(&gift.id).unwrap();
        }
        for answer in first_answers(&branch, MARA) {
            branch.invoke_projection_command(&answer).unwrap();
        }
        next_day(&mut branch);
    }
    let world = branch.world();
    let doors = came_up(world.events())
        .into_iter()
        .filter(|(kind, who)| {
            *who == MARA && matches!(kind.as_str(), "confide" | "invite" | "favour" | "keepsake")
        })
        .map(|(kind, _)| kind)
        .collect::<Vec<_>>();
    assert_eq!(
        doors,
        vec!["confide", "invite", "favour", "keepsake"],
        "{doors:?}"
    );
    // Each in Mara's own words, which nobody else says.
    let prompts = world
        .events()
        .iter()
        .filter(|event| {
            matches!(event.kind.as_str(), "situation_came_up" | "warmed")
                && event.actor == Some(MARA)
        })
        .filter_map(|event| match event.payload.get("said") {
            Some(Value::Text(said)) => Some(said.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for scene in crate::voices::MARA_VOICE.scenes {
        let opening = scene.prompt.split('{').next().unwrap();
        assert!(
            prompts.iter().any(|prompt| prompt.starts_with(opening)),
            "{opening:?} never said: {prompts:?}"
        );
    }
    assert!(lives::keepsakes(world).iter().any(|kept| kept.from == MARA));
    let standing = speech::standing_of(world, MARA).unwrap();
    assert_eq!(standing.level, 2, "{standing:?}");
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn a_grudge_closes_doors_and_shows() {
    let mut branch = opened();
    let mut soured = None;
    for _ in 0..60 {
        branch.say(LEO, "you're useless").unwrap();
        next_day(&mut branch);
        if soured.is_none() && lives::regard(branch.world().state(), LEO) <= lives::GRUDGE {
            soured = Some(branch.world().events().len());
        }
    }
    let soured = soured.expect("insults every day sour Leo on the player");
    let later = came_up(&branch.world().events()[soured..])
        .into_iter()
        .filter(|(_, who)| *who == LEO)
        .map(|(kind, _)| kind)
        .collect::<Vec<_>>();
    assert!(
        !later.iter().any(|kind| matches!(
            kind.as_str(),
            "short" | "lonely" | "worn" | "learn" | "sweet"
        )),
        "Leo asked for help while holding a grudge: {later:?}"
    );
    assert!(later.iter().any(|kind| kind == "cold"), "{later:?}");
}

#[test]
fn people_remember_how_you_answered_them() {
    let mut branch = opened();
    let asked = commands(&branch)
        .into_iter()
        .find(|command| {
            command.unavailable.is_none()
                && story::parse_command(&command.id).is_some()
                && command.question.is_some()
        })
        .expect("a question on the first day");
    let Some(SelectionId::Entity(asker)) = asked.asker else {
        panic!("asked by someone");
    };
    branch.invoke_projection_command(&asked.id).unwrap();
    for day in 1..=3 {
        next_day(&mut branch);
        let world = branch.world();
        let recalled = conversation::recollection(world, &speech::kit(world.state()), asker);
        if recalled
            .as_deref()
            .is_some_and(|line| line.contains(asked.title.trim_end_matches('.')))
        {
            return;
        }
        assert!(day < 3, "never brought up: {recalled:?}");
    }
}

#[test]
fn someone_sees_what_you_made_and_remembers_it() {
    let mut branch = opened();
    let build = commands(&branch)
        .into_iter()
        .find(|command| {
            command.unavailable.is_none()
                && command
                    .hand
                    .as_ref()
                    .is_some_and(|hand| hand.verb == "Build")
        })
        .expect("something to build");
    let thing = build.hand.as_ref().unwrap().thing.to_lowercase();
    branch.invoke_projection_command(&build.id).unwrap();
    let reacted = branch
        .world()
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "reacted")
        .expect("someone says what they make of it");
    let who = reacted.actor.unwrap();
    next_day(&mut branch);
    let world = branch.world();
    let recalled =
        conversation::recollection(world, &speech::kit(world.state()), who).unwrap_or_default();
    assert!(
        recalled.contains(&format!("{thing} you built")),
        "{recalled:?}"
    );
}

#[test]
fn what_you_made_is_named_as_yours() {
    let mut branch = opened();
    let bench = commands(&branch)
        .into_iter()
        .find(|command| {
            command.unavailable.is_none()
                && command.hand.as_ref().is_some_and(|hand| {
                    hand.verb == "Build" && hand.at == Some(SelectionId::Entity(HARBOR))
                })
        })
        .expect("something to build on the quay");
    branch.invoke_projection_command(&bench.id).unwrap();
    for _ in 0..120 {
        next_day(&mut branch);
    }
    let world = branch.world();
    let text = |event: &world_core::Event, key: &str| match event.payload.get(key) {
        Some(Value::Text(text)) => text.clone(),
        _ => String::new(),
    };
    assert!(
        world
            .events()
            .iter()
            .filter(|event| event.kind == "festival_held")
            .any(|event| text(event, "told").contains("you made")),
        "no festival on the quay was told with what you made"
    );
    let first_chapter = world
        .events()
        .iter()
        .find(|event| event.kind == "chapter_ended")
        .expect("a chapter closed");
    assert!(
        text(first_chapter, "summary").contains("You built"),
        "{}",
        text(first_chapter, "summary")
    );
}

/// Every return ends on something someone left or wrote for the player.
#[test]
fn every_return_brings_a_keepsake_or_a_letter() {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(crate::tiny_society_registration())
        .unwrap();
    let mut session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
    for periods in 1..=7 {
        let snapshot = session.advance_background(periods).unwrap();
        let beats = snapshot
            .briefing
            .as_ref()
            .map(|briefing| {
                briefing
                    .items
                    .iter()
                    .filter(|item| item.kind == world_projection::BriefingItemKind::Beat)
                    .map(|item| item.title.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let last = beats.last().cloned().unwrap_or_default();
        assert!(
            last.contains(" left you ") || last.contains(" wrote to you"),
            "a return of {periods} ended on {last:?}: {beats:?}"
        );
    }
}

/// A suggestion's outcome follows from the people and the weather: a
/// picnic in the rain draws fewer than one in sunshine, a dance indoors
/// does not care, and the World replays the same without deciding again.
#[test]
fn a_suggestion_follows_from_people_and_weather_and_replays() {
    let mut society = crate::TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    for _ in 0..20 {
        branch
            .invoke_projection_command(crate::story::WAIT_COMMAND)
            .unwrap();
    }
    let actions = crate::build_action_registry().unwrap();
    let came = |fair: bool, idea: &str| {
        let mut world = branch.world().clone();
        let event = world
            .execute(actions, &lives::suggestion_request(idea, fair))
            .unwrap();
        let came = match event.payload.get("came") {
            Some(world_core::Value::Integer(came)) => *came,
            _ => 0,
        };
        let replayed = world.replay().unwrap();
        assert_eq!(replayed.state(), world.state(), "replays the same");
        came
    };
    assert!(came(true, "picnic") > came(false, "picnic"));
    assert_eq!(came(true, "dance"), came(false, "dance"));
    // Once suggested, not again for a while.
    let mut world = branch.world().clone();
    world
        .execute(actions, &lives::suggestion_request("market", true))
        .unwrap();
    assert!(world
        .execute(actions, &lives::suggestion_request("market", true))
        .is_err());
}

/// A guest from another World brings a letter and something to keep, as
/// Actions in the World they visit; the World they came from is only read,
/// and nothing about it changes.
#[test]
fn a_guest_from_another_world_visits_and_writes_nothing_back() {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(crate::tiny_society_registration())
        .unwrap();
    let mut home = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
    for _ in 0..5 {
        home.handle(world_projection::ProjectionIntent::InvokeCommand(
            crate::story::WAIT_COMMAND.into(),
        ))
        .unwrap();
    }
    let mut away = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
    for _ in 0..3 {
        away.handle(world_projection::ProjectionIntent::InvokeCommand(
            crate::story::WAIT_COMMAND.into(),
        ))
        .unwrap();
    }
    let before = away.archive().unwrap().unwrap();
    let guest = world_projection::Guest::from_snapshot(&away.snapshot()).expect("someone to visit");
    let letters = home.snapshot().letters.len();
    let after = home
        .handle(world_projection::ProjectionIntent::Host(guest.clone()))
        .unwrap();
    assert_eq!(
        after.letters.len(),
        letters + 1,
        "the guest's letter is in the box"
    );
    assert!(after
        .letters
        .last()
        .is_some_and(|letter| letter.note.contains(&guest.letter)));
    assert_eq!(
        away.archive().unwrap().unwrap(),
        before,
        "the World the guest came from is unchanged"
    );
    // The visit replays like anything else.
    let archive = home.archive().unwrap().unwrap();
    let reopened = registry.open_archive(&archive).unwrap();
    assert_eq!(reopened.snapshot().letters, after.letters);
}
