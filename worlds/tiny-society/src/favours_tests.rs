//! Talk with a goal in the harbour: a favour asked, done by talking, and
//! thanked; one let lapse; and every kind of player asked one early.

use crate::{story, TinySocietyBranch};
use conversation::favour::{self, Kind};
use world_core::{Event, Value, World};
use world_pack_testkit::players::{self, PLAYERS};
use world_projection::SelectionId;

fn pass(branch: &mut TinySocietyBranch) {
    branch
        .invoke_projection_command(story::WAIT_COMMAND)
        .unwrap();
}

fn day(branch: &TinySocietyBranch) -> u64 {
    branch.world().world_time() / crate::persistence::WORLD_DAY_TICKS + 1
}

fn asks(world: &World) -> Vec<&Event> {
    world.events_of_kind(&[favour::ASKED])
}

fn kind_of(event: &Event) -> Kind {
    match event.payload.get("favour") {
        Some(Value::Text(kind)) => Kind::from_id(kind).unwrap(),
        _ => panic!("a favour says what it is"),
    }
}

/// A new harbour, played until someone asks a favour: never in the first
/// two days.
fn asked() -> TinySocietyBranch {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    while asks(branch.world()).is_empty() {
        assert!(
            branch.projection_snapshot().favour.is_none(),
            "day {}",
            day(&branch)
        );
        pass(&mut branch);
        assert!(day(&branch) <= 10, "no favour by day 10");
    }
    assert!(day(&branch) >= 3, "asked on day {}", day(&branch));
    branch
}

/// What a player says to do a favour of `kind` asked by `asker`.
fn words(kind: Kind, asker: &str) -> String {
    match kind {
        Kind::AskAfter => "How are you doing?".into(),
        Kind::Invite => "Fancy a walk with me?".into(),
        Kind::CheerUp => "You're doing a great job, you know.".into(),
        Kind::Sorry => format!("{asker} says sorry."),
    }
}

#[test]
fn a_favour_is_asked_done_by_talking_and_thanked() {
    let mut branch = asked();
    let ask = asks(branch.world())[0].clone();
    let asker = ask.actor.unwrap();
    let whom = ask.targets[0];
    let state = branch.world().state();
    let (asker_name, whom_name) = (
        lives::first_name(state, asker),
        lives::first_name(state, whom),
    );
    // The asker says it aloud, and the drawer keeps a note of it.
    let snapshot = branch.projection_snapshot();
    let line = favour::said(state, &ask).expect("asked aloud").1;
    assert!(line.contains(&whom_name), "{line}");
    assert!(
        snapshot
            .voices
            .iter()
            .any(|voice| voice.speaker == SelectionId::Entity(asker) && voice.line == line),
        "{:?}",
        snapshot.voices
    );
    let shown = snapshot.favour.clone().expect("a note in the drawer");
    assert!(!shown.done);
    assert_eq!(shown.asker, SelectionId::Entity(asker));
    assert!(
        shown
            .note
            .starts_with(&format!("{asker_name} asked you to")),
        "{}",
        shown.note
    );
    assert!(shown.hint.contains(&whom_name), "{}", shown.hint);

    // Talking to someone else does nothing for it.
    let other = crate::story::people_in(branch.world().state())
        .into_iter()
        .find(|person| *person != whom && *person != asker)
        .unwrap();
    let events = branch.say(other, "Hello there!").unwrap();
    assert_eq!(events.len(), 1, "only the exchange");

    // Talking to whom it is for does it.
    let kind = kind_of(&ask);
    let regard = lives::regard(branch.world().state(), asker);
    let events = branch.say(whom, &words(kind, &asker_name)).unwrap();
    let world = branch.world();
    let spoken = world.event(events[0]).unwrap();
    let reply = match spoken.payload.get("reply") {
        Some(Value::Text(reply)) => reply.clone(),
        _ => panic!("answered"),
    };
    let done = world
        .events_of_kind(&[favour::DONE])
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{kind:?} not done by {reply:?}"));
    assert!(events.contains(&done.id));
    // Caused by the ask and by the words, and thanked by the asker.
    assert!(done.caused_by.contains(&ask.id), "{:?}", done.caused_by);
    assert!(done.caused_by.contains(&spoken.id), "{:?}", done.caused_by);
    assert_eq!(done.actor, Some(asker));
    assert!(lives::regard(world.state(), asker) > regard);
    let snapshot = branch.projection_snapshot();
    let thanks = favour::said(world.state(), done).unwrap().1;
    assert!(thanks.contains("Thank"), "{thanks}");
    assert!(snapshot
        .voices
        .iter()
        .any(|voice| voice.speaker == SelectionId::Entity(asker) && voice.line == thanks));
    assert!(snapshot.favour.as_ref().is_some_and(|favour| favour.done));
    // Whom it was for could tell who sent the player, except a kindness,
    // which speaks for itself.
    if kind != Kind::CheerUp {
        assert!(reply.contains(&asker_name), "{reply}");
    }
    // It is done once: saying it again does nothing more.
    let again = branch.say(whom, &words(kind, &asker_name)).unwrap();
    assert_eq!(again.len(), 1);
    // The next day the note is gone.
    pass(&mut branch);
    assert!(branch.projection_snapshot().favour.is_none());
    // And the World replays to where it stands without asking anyone.
    let replayed = branch.world().replay().unwrap();
    assert_eq!(replayed.state(), branch.world().state());
}

#[test]
fn every_kind_of_favour_can_be_done() {
    // Played on until each kind has been asked once, doing each.
    let mut branch = asked();
    let mut done = std::collections::BTreeSet::new();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..200 {
        let open = favour::open(
            branch.world().state(),
            &crate::speech::kit(branch.world().state()),
        );
        if let Some(open) = open {
            if seen.insert(open.kind.id()) {
                let name = lives::first_name(branch.world().state(), open.asker);
                let before = branch.world().events_of_kind(&[favour::DONE]).len();
                // An invitation may be turned down by someone worn out;
                // then it is tried again the next day.
                branch.say(open.whom, &words(open.kind, &name)).unwrap();
                if branch.world().events_of_kind(&[favour::DONE]).len() > before {
                    done.insert(open.kind.id());
                } else {
                    seen.remove(open.kind.id());
                }
            }
        }
        if done.len() == Kind::ALL.len() {
            break;
        }
        pass(&mut branch);
    }
    assert_eq!(done.len(), Kind::ALL.len(), "{done:?}");
}

#[test]
fn a_favour_not_done_lapses_quietly() {
    let mut branch = asked();
    let asked_on = day(&branch);
    for _ in 0..favour::OPEN_PERIODS {
        pass(&mut branch);
        assert!(branch.projection_snapshot().favour.is_some());
    }
    pass(&mut branch);
    // Gone from the drawer, with nothing recorded and nothing held against
    // the player.
    let snapshot = branch.projection_snapshot();
    assert!(snapshot.favour.is_none(), "day {}", day(&branch));
    let world = branch.world();
    assert!(world.events_of_kind(&[favour::DONE]).is_empty());
    assert!(favour::open(world.state(), &crate::speech::kit(world.state())).is_none());
    assert_eq!(asks(world).len(), 1);
    // The next is asked a week or so after the last, by someone else.
    while asks(branch.world()).len() < 2 {
        pass(&mut branch);
        assert!(day(&branch) <= asked_on + 12, "day {}", day(&branch));
    }
    let world = branch.world();
    let [first, second] = [asks(world)[0], asks(world)[1]];
    assert_ne!(first.actor, second.actor);
    assert!(day(&branch) >= asked_on + 6);
}

#[test]
fn five_players_see_a_favour_within_ten_days() {
    for player in PLAYERS {
        let branch = TinySocietyBranch::new_world().unwrap();
        let played = players::play(format!("{player:?}"), player, 10, branch);
        let asked = asks(&played.world);
        let first = asked
            .first()
            .unwrap_or_else(|| panic!("{player:?}: no favour in 10 days"));
        let on = first.world_time / crate::persistence::WORLD_DAY_TICKS + 1;
        eprintln!(
            "{player:?}: day {on}, {} favours in 10 days; first: {:?}",
            asked.len(),
            favour::said(played.world.state(), first).map(|(_, line)| line)
        );
        assert!((3..=10).contains(&on), "{player:?}: day {on}");
    }
}
