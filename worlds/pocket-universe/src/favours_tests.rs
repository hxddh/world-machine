//! Talk with a goal in every place: a favour asked, done by talking and
//! thanked, one let lapse, and every kind of player asked one early.

use crate::{PocketUniverse, NUDGE_COMMAND};
use conversation::favour::{self, Kind};
use world_core::{Event, Value, World};
use world_pack_testkit::players::{self, PLAYERS};
use world_projection::SelectionId;

const PLACES: [&str; 3] = [
    crate::SEED_MARS_COLONY_COMMAND,
    crate::SEED_1980S_TOWN_COMMAND,
    crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
];

fn period(world: &World) -> u64 {
    world.world_time() / crate::BACKGROUND_PERIOD
}

fn asks(world: &World) -> Vec<&Event> {
    world.events_of_kind(&[favour::ASKED])
}

fn begun(seed: &str) -> PocketUniverse {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    universe
}

/// A place, played until someone asks a favour: never in its first two
/// periods.
fn asked(seed: &str) -> PocketUniverse {
    let mut universe = begun(seed);
    let began = period(universe.world());
    while asks(universe.world()).is_empty() {
        assert!(universe.projection_snapshot().favour.is_none());
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        assert!(period(universe.world()) <= began + 10, "{seed}: none in 10");
    }
    assert!(
        period(universe.world()) >= began + 2,
        "{seed}: asked in period {}",
        period(universe.world()) - began
    );
    universe
}

fn words(kind: Kind, asker: &str) -> String {
    match kind {
        Kind::AskAfter => "How are you doing?".into(),
        Kind::Invite => "Fancy a walk with me?".into(),
        Kind::CheerUp => "You're doing a great job, you know.".into(),
        Kind::Sorry => format!("{asker} says sorry."),
    }
}

#[test]
fn in_every_place_a_favour_is_asked_done_by_talking_and_thanked() {
    for seed in PLACES {
        let mut universe = asked(seed);
        let ask = asks(universe.world())[0].clone();
        let asker = ask.actor.unwrap();
        let whom = ask.targets[0];
        let kind = match ask.payload.get("favour") {
            Some(Value::Text(kind)) => Kind::from_id(kind).unwrap(),
            _ => panic!("{seed}: what favour?"),
        };
        let snapshot = universe.projection_snapshot();
        let said = favour::said(universe.world().state(), &ask).unwrap().1;
        assert!(
            snapshot
                .voices
                .iter()
                .any(|voice| voice.speaker == SelectionId::Entity(asker) && voice.line == said),
            "{seed}: {said}"
        );
        let note = snapshot.favour.expect("a note in the drawer");
        assert!(!note.done && note.whom == SelectionId::Entity(whom));
        let name = lives::first_name(universe.world().state(), asker);
        let regard = lives::regard(universe.world().state(), asker);
        let spoken = universe.say(whom, &words(kind, &name)).unwrap();
        let world = universe.world();
        let done = world
            .events_of_kind(&[favour::DONE])
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("{seed}: {kind:?} not done"));
        assert!(done.caused_by.contains(&ask.id) && done.caused_by.contains(&spoken));
        assert!(lives::regard(world.state(), asker) > regard, "{seed}");
        let snapshot = universe.projection_snapshot();
        assert!(snapshot.favour.is_some_and(|favour| favour.done));
        let thanks = favour::said(world.state(), done).unwrap().1;
        assert!(snapshot.voices.iter().any(|voice| voice.line == thanks));
        let replayed = universe.world().replay().unwrap();
        assert_eq!(replayed.state(), universe.world().state(), "{seed}");
    }
}

#[test]
fn in_every_place_a_favour_not_done_lapses_quietly() {
    for seed in PLACES {
        let mut universe = asked(seed);
        for _ in 0..=favour::OPEN_PERIODS {
            universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        }
        assert!(universe.projection_snapshot().favour.is_none(), "{seed}");
        assert!(universe.world().events_of_kind(&[favour::DONE]).is_empty());
    }
}

#[test]
fn five_players_in_every_place_see_a_favour_within_ten_periods() {
    for seed in PLACES {
        for player in PLAYERS {
            let universe = begun(seed);
            let began = period(universe.world());
            let played = players::play(format!("{seed} {player:?}"), player, 10, universe);
            let first = asks(&played.world)
                .first()
                .map(|event| event.world_time / crate::BACKGROUND_PERIOD - began)
                .unwrap_or_else(|| panic!("{seed} {player:?}: no favour"));
            assert!((2..=10).contains(&first), "{seed} {player:?}: {first}");
        }
    }
}
