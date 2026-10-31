use super::*;
use lives::{Activity, At, Need, With};
use std::collections::BTreeSet;
use world_core::Entity;

const NOTES: EntityId = EntityId::new(100);
const GREEN: EntityId = EntityId::new(50);
const MILL: EntityId = EntityId::new(51);
const BELL: EntityId = EntityId::new(52);
const HERBS: EntityId = EntityId::new(60);
const PEOPLE: [u64; 4] = [1, 2, 3, 4];

const ACTIVITIES: &[Activity] = &[
    Activity {
        id: "shift",
        need: Need::Money,
        with: With::Workmate,
        at: At::Work,
        told: "{name} worked a long shift at {place} with {other}",
        said: &["Long shift with {other}.", "{place} was busy today."],
        gives: &[],
    },
    Activity {
        id: "drink",
        need: Need::Company,
        with: With::Friend,
        at: At::Gathering,
        told: "{name} had a drink with {other} at {place}",
        said: &[
            "A pint with {other}.",
            "{other} told the old stories again.",
        ],
        gives: &[],
    },
    Activity {
        id: "nap",
        need: Need::Rest,
        with: With::Alone,
        at: At::Quiet,
        told: "{name} dozed at {place}",
        said: &["Forty winks at {place}."],
        gives: &[],
    },
    Activity {
        id: "help",
        need: Need::Purpose,
        with: With::Anyone,
        at: At::Work,
        told: "{name} helped {other} at {place}",
        said: &["Gave {other} a hand."],
        gives: &[],
    },
];

const FINDS: &[Find] = &[
    Find::any("a blue button"),
    Find::any("a smooth stone"),
    Find::any("a brass bell"),
];

fn people(_: &World) -> Vec<EntityId> {
    PEOPLE.map(EntityId::new).to_vec()
}

fn work(state: &WorldState, person: EntityId) -> Option<EntityId> {
    match state.entity(person)?.component("works_at")? {
        Value::Entity(id) => Some(*id),
        _ => None,
    }
}

fn home(_: &WorldState, _: EntityId) -> Option<EntityId> {
    None
}

fn cast() -> Cast {
    Cast {
        notes: NOTES,
        period: 10,
        unit: "day",
        settlement: "village",
        short_of: "money",
        people,
        stays: |_| true,
        traits: |_| None,
        kept: |_, _| false,
        mood: |_| 0,
        work,
        home,
        gathering: BELL,
        quiet: GREEN,
        host: EntityId::new(1),
        activities: ACTIVITIES,
        topics: &["the fence", "a borrowed ladder"],
        outings: &["a walk", "a dance"],
        fund: None,
        visitors: None,
        most_people: 8,
        most_open: 2,
        voice: |_| None,
        kin: None,
        kind: lives::never,
        pronouns: lives::they_all,
    }
}

fn rite(finds: &'static [Find]) -> Rite {
    Rite {
        cast: |_| cast(),
        finds,
        season: no_seasons,
    }
}

/// The player plants herbs by the mill: a deed of their own hands, as
/// the hands System records one.
struct Plants;

impl Action for Plants {
    fn name(&self) -> &'static str {
        "test_plants"
    }

    fn evaluate(&self, _: &WorldState, _: &ActionRequest) -> Result<EventDraft, ActionError> {
        let mut draft = EventDraft::new("planted_by_hand");
        draft.targets = vec![HERBS, MILL];
        draft
            .payload
            .insert("told".into(), "You planted herbs by the mill".into());
        draft.changes = vec![StateChange::CreateEntity(
            Entity::new(HERBS, hands::FIXTURE)
                .with_component("name", "Herb bed")
                .with_component("at", Value::Entity(MILL))
                .with_component(hands::MADE, true),
        )];
        Ok(draft)
    }
}

fn world(finds: &'static [Find]) -> (World, ActionRegistry) {
    world_in(rite(finds))
}

fn world_in(rite: Rite) -> (World, ActionRegistry) {
    let mut state = WorldState::default();
    for (id, name, works) in [
        (1, "Ann", MILL),
        (2, "Ben", MILL),
        (3, "Cat", BELL),
        (4, "Dan", BELL),
    ] {
        state
            .seed_entity(
                Entity::new(EntityId::new(id), "person")
                    .with_component("name", name)
                    .with_component("works_at", Value::Entity(works)),
            )
            .unwrap();
    }
    for (id, name) in [(BELL, "the Bell"), (GREEN, "the green"), (MILL, "the mill")] {
        state
            .seed_entity(Entity::new(id, "place").with_component("name", name))
            .unwrap();
    }
    let mut actions = ActionRegistry::new();
    lives::register_actions(&mut actions, |_| cast()).unwrap();
    register_actions(&mut actions, rite).unwrap();
    actions.register(Plants).unwrap();
    actions.register(DayBegins).unwrap();
    (World::new(state), actions)
}

/// A day begins, as a Pack's calendar records it: the cause of the day's
/// morning.
struct DayBegins;

impl Action for DayBegins {
    fn name(&self) -> &'static str {
        "test_day_begins"
    }

    fn evaluate(&self, _: &WorldState, _: &ActionRequest) -> Result<EventDraft, ActionError> {
        Ok(EventDraft::new("day_began"))
    }
}

/// A day passes: it begins, and everyone lives it. Returns its beginning.
fn pass(world: &mut World, actions: &ActionRegistry) -> EventId {
    let target = world.world_time() + 10;
    world.advance_to(actions, target).unwrap();
    let day = world
        .execute(actions, &ActionRequest::new("test_day_begins"))
        .unwrap()
        .id;
    world.following(day, |world| {
        lives::tick(world, actions, &cast(), false).unwrap();
    });
    day
}

fn questions() -> Vec<Question> {
    (1..=40)
        .map(|n| Question {
            id: format!("q{n}"),
            prompt: format!("Question {n}?"),
        })
        .collect()
}

fn command(question: &Question) -> world_projection::ProjectionCommand {
    world_projection::ProjectionCommand {
        id: format!("{}.yes", question.id),
        title: "Yes".into(),
        detail: String::new(),
        effects: Vec::new(),
        scenery: None,
        moves: Vec::new(),
        asker: None,
        question: Some(question.clone()),
        unavailable: None,
        hand: None,
        preview: None,
        role: None,
    }
}

fn parts_of(world: &World, came: EventId) -> Vec<&Event> {
    world
        .events()
        .iter()
        .filter(|event| event.caused_by.contains(&came))
        .collect()
}

#[test]
fn a_worlds_first_day_has_no_morning_and_every_later_day_one() {
    let (mut world, actions) = world(FINDS);
    let rite = rite(FINDS);
    let first = pass(&mut world, &actions);
    assert_eq!(today(world.state(), &cast()), 1);
    assert_eq!(
        morning(&mut world, &actions, &rite, Some(first), &[]).unwrap(),
        None,
        "the first day opens on its welcome"
    );
    let second = pass(&mut world, &actions);
    let came = morning(&mut world, &actions, &rite, Some(second), &[])
        .unwrap()
        .expect("the second day's morning");
    let event = world.event(came).unwrap();
    assert_eq!(event.kind, MORNING_CAME);
    assert_eq!(
        event.caused_by,
        vec![second],
        "it follows the day's beginning"
    );
    assert!(morning_came_today(world.state(), &cast()));
    // The same day again (a second window, the app opened twice): nothing.
    assert_eq!(
        morning(&mut world, &actions, &rite, Some(second), &[]).unwrap(),
        None
    );
    assert_eq!(world.events_of_kind(&[MORNING_CAME]).len(), 1);
    // A morning is refused for any other day than this one.
    let wrong = ActionRequest::new(MORNING_ACTION).arg("day", 9_i64);
    assert!(world.execute(&actions, &wrong).is_err());
}

#[test]
fn a_morning_brings_a_letter_a_question_a_tending_and_a_find_all_from_it() {
    let (mut world, actions) = world(FINDS);
    let rite = rite(FINDS);
    // Days lived first, so there is news to write and a deed to tend.
    for _ in 0..12 {
        pass(&mut world, &actions);
    }
    world
        .execute(&actions, &ActionRequest::new("test_plants"))
        .unwrap();
    let day = pass(&mut world, &actions);
    let questions = questions();
    let came = morning(&mut world, &actions, &rite, Some(day), &questions)
        .unwrap()
        .unwrap();
    let parts = parts_of(&world, came);
    let kinds = parts
        .iter()
        .map(|event| event.kind.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        kinds,
        BTreeSet::from(["letter_written", QUESTION_PUT, TENDED, FOUND]),
        "{parts:#?}"
    );
    // What the player made is what is tended, and it is marked tended.
    let tended = parts.iter().find(|event| event.kind == TENDED).unwrap();
    assert!(tended.targets.contains(&HERBS), "{tended:?}");
    let line = told(tended).unwrap();
    assert!(line.contains("your herb bed by the mill"), "{line}");
    assert!(world
        .state()
        .entity(HERBS)
        .unwrap()
        .component(TENDED_ON)
        .is_some());
    // The day's main question is the first the World puts.
    let put = parts
        .iter()
        .find(|event| event.kind == QUESTION_PUT)
        .unwrap();
    assert_eq!(put.payload.get("question"), Some(&Value::Text("q1".into())));
    // The find is the Pack's first.
    let found = parts.iter().find(|event| event.kind == FOUND).unwrap();
    assert_eq!(
        found.payload.get("find"),
        Some(&Value::Text("a blue button".into()))
    );
    // As the window is shown it: every part, the question while it is put.
    let commands = questions.iter().map(command).collect::<Vec<_>>();
    let shown = shown(&world, &cast(), &commands).expect("this day's morning");
    assert_eq!(shown.moment, SelectionId::Event(came));
    assert!(shown.letter.is_some_and(|letter| !letter.note.is_empty()));
    assert_eq!(
        shown.question.map(|question| question.id),
        Some("q1".into())
    );
    assert_eq!(
        shown.tended.map(|tended| tended.about),
        Some(Some(SelectionId::Entity(HERBS)))
    );
    assert!(shown
        .find
        .is_some_and(|find| find.line.starts_with("You found a blue button by ")));
    // Answered (no longer put), the question is left off; the rest stays.
    let shown = super::shown(&world, &cast(), &commands[1..]).unwrap();
    assert!(shown.question.is_none());
    assert!(shown.find.is_some());
    // The next day, until its own morning, there is none to show.
    pass(&mut world, &actions);
    assert!(super::shown(&world, &cast(), &commands).is_none());
    // The book keeps the find.
    let book = book(&world);
    assert_eq!(book.len(), 1);
    assert_eq!(book[0].shelf, FINDS_SHELF);
    assert_eq!(book[0].name, "A blue button");
}

#[test]
fn no_mornings_ritual_repeats_a_line() {
    let (mut world, actions) = world(FINDS);
    let rite = rite(FINDS);
    let questions = questions();
    for _ in 0..3 {
        pass(&mut world, &actions);
    }
    world
        .execute(&actions, &ActionRequest::new("test_plants"))
        .unwrap();
    let mut lines = Vec::new();
    let mut mornings = 0;
    for _ in 0..30 {
        let day = pass(&mut world, &actions);
        let Some(came) = morning(&mut world, &actions, &rite, Some(day), &questions).unwrap()
        else {
            continue;
        };
        mornings += 1;
        for part in parts_of(&world, came) {
            let line = match part.kind.as_str() {
                "letter_written" => match part.payload.get("said") {
                    Some(Value::Text(said)) => said.clone(),
                    _ => continue,
                },
                QUESTION_PUT => match part.payload.get("question") {
                    Some(Value::Text(id)) => id.clone(),
                    _ => continue,
                },
                _ => told(part).unwrap(),
            };
            lines.push(line);
        }
        // Every morning tends something and puts a question.
        let kinds = parts_of(&world, came)
            .iter()
            .map(|event| event.kind.clone())
            .collect::<BTreeSet<_>>();
        assert!(
            kinds.contains(TENDED) && kinds.contains(QUESTION_PUT),
            "{kinds:?}"
        );
    }
    assert_eq!(mornings, 30);
    let unique = lines.iter().collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), lines.len(), "a line came again: {lines:#?}");
    // Three finds, each once, and then no more: never a find twice.
    assert_eq!(world.events_of_kind(&[FOUND]).len(), FINDS.len());
}

#[test]
fn a_morning_is_the_same_from_the_same_world_and_replays_without_rerunning() {
    let run = || {
        let (mut world, actions) = world(FINDS);
        let rite = rite(FINDS);
        for _ in 0..8 {
            let day = pass(&mut world, &actions);
            morning(&mut world, &actions, &rite, Some(day), &questions()).unwrap();
        }
        world
    };
    let (one, two) = (run(), run());
    assert_eq!(one.events(), two.events());
    // The record alone rebuilds the state: nothing is asked again.
    let replayed = one.replay().unwrap();
    assert_eq!(replayed.state(), one.state());
}

#[test]
fn a_part_the_world_refuses_is_left_out_and_the_rest_still_comes() {
    // No finds, and nothing put: the morning still brings what it can.
    let (mut world, actions) = world(&[]);
    let rite = rite(&[]);
    for _ in 0..6 {
        pass(&mut world, &actions);
    }
    let day = pass(&mut world, &actions);
    let came = morning(&mut world, &actions, &rite, Some(day), &[])
        .unwrap()
        .unwrap();
    let kinds = parts_of(&world, came)
        .iter()
        .map(|event| event.kind.clone())
        .collect::<BTreeSet<_>>();
    assert!(!kinds.contains(FOUND) && !kinds.contains(QUESTION_PUT));
    assert!(kinds.contains(TENDED), "{kinds:?}");
    // Nothing made by the player: the place itself is looked after.
    let tended = world.events_of_kind(&[TENDED])[0];
    assert!(told(tended).unwrap().starts_with("Overnight, "));
    // A find not among the Pack's is refused outright.
    let made_up = ActionRequest::new(FIND_ACTION)
        .arg("what", "a dragon")
        .arg("at", Value::Entity(MILL));
    assert!(world.execute(&actions, &made_up).is_err());
}

#[test]
fn a_thing_named_for_its_place_is_tended_without_saying_the_place_twice() {
    assert!(names_its_place("green flag", "the green"));
    assert!(!names_its_place("herb bed", "the mill"));
    assert!(!names_its_place("old bench", "The Bell"));
    assert!(
        !names_its_place("the bed", "The Bed"),
        "never by a word of three letters"
    );
    assert_eq!(
        placeless("Overnight, {who} watered your {thing} by {place}."),
        "Overnight, {who} watered your {thing}."
    );
    assert_eq!(
        placeless("Overnight, {who} dusted off your {thing} at {place}."),
        "Overnight, {who} dusted off your {thing}."
    );
}

#[test]
fn every_line_of_the_morning_has_its_slots() {
    for line in lines() {
        assert!(!line.trim().is_empty());
    }
    for line in TENDING_PLANTED
        .iter()
        .chain(&TENDING_BUILT)
        .chain(&TENDING_DECORATED)
    {
        for slot in ["{who}", "{thing}", "{place}"] {
            assert!(line.contains(slot), "{line}");
        }
    }
    for line in TENDING_PLACE {
        assert!(line.contains("{who}") && line.contains("{place}"), "{line}");
    }
}

/// Whoever looks after something of the player's overnight has crossed
/// their path: the `lives` System counts this kind so (v0.31 play: Mia
/// watered the player's flowers, then said "We haven't met properly").
#[test]
fn looking_after_the_players_things_crosses_their_path() {
    assert!(lives::CROSSED_KINDS.contains(&TENDED));
}

/// What is looked after moves round the place: never the same thing two
/// mornings running, the player's herbs given a rest between visits, and
/// the places of the town seen to in between.
#[test]
fn what_is_looked_after_moves_round_the_place() {
    let (mut world, actions) = world(FINDS);
    let rite = rite(FINDS);
    for _ in 0..3 {
        pass(&mut world, &actions);
    }
    world
        .execute(&actions, &ActionRequest::new("test_plants"))
        .unwrap();
    let mut seen = Vec::new();
    for _ in 0..24 {
        let day = pass(&mut world, &actions);
        let came = morning(&mut world, &actions, &rite, Some(day), &[])
            .unwrap()
            .unwrap();
        let tended = parts_of(&world, came)
            .into_iter()
            .find(|part| part.kind == TENDED)
            .expect("something looked after");
        seen.push((tended.targets[1], told(tended).unwrap()));
    }
    for two in seen.windows(2) {
        assert_ne!(two[0].0, two[1].0, "{seen:#?}");
    }
    let herbs = seen
        .iter()
        .enumerate()
        .filter(|(_, (thing, _))| *thing == HERBS)
        .map(|(at, _)| at)
        .collect::<Vec<_>>();
    assert!(herbs.len() >= 6, "the herbs had their turn: {seen:#?}");
    assert!(
        herbs
            .windows(2)
            .all(|two| two[1] - two[0] >= REST_MADE as usize),
        "{herbs:?}"
    );
    let places = seen
        .iter()
        .map(|(thing, _)| *thing)
        .filter(|thing| *thing != HERBS)
        .collect::<BTreeSet<_>>();
    assert_eq!(places, BTreeSet::from([BELL, GREEN, MILL]), "{seen:#?}");
}

const WINTER_FINDS: &[Find] = &[
    Find::at("a daisy", 0, "the mill"),
    Find::at("a snowball", 3, "the mill"),
    Find::at("an icicle", 3, "the green"),
    Find::at("a lost glove", 3, "the old forge"),
    Find::any("a smooth stone"),
];

/// In a World with seasons, a winter's morning looks after things the
/// winter's way, and turns up the winter's finds at their own places,
/// never a spring one while a winter one is left; one by a place the
/// World does not have waits for it.
#[test]
fn a_season_brings_its_own_tending_and_finds() {
    let rite = Rite {
        cast: |_| cast(),
        finds: WINTER_FINDS,
        season: |_| Some(3),
    };
    let (mut world, actions) = world_in(rite);
    for _ in 0..3 {
        pass(&mut world, &actions);
    }
    world
        .execute(&actions, &ActionRequest::new("test_plants"))
        .unwrap();
    let mut found = Vec::new();
    let mut lines = Vec::new();
    for _ in 0..5 {
        let day = pass(&mut world, &actions);
        let came = morning(&mut world, &actions, &rite, Some(day), &[])
            .unwrap()
            .unwrap();
        for part in parts_of(&world, came) {
            match part.kind.as_str() {
                FOUND => found.push((
                    part.payload.get("find").cloned(),
                    part.targets.first().copied(),
                )),
                TENDED => lines.push(told(part).unwrap()),
                _ => {}
            }
        }
    }
    let text = |what: &str| Some(Value::Text(what.into()));
    let mut winter = found[..2].to_vec();
    winter.sort_by_key(|(what, _)| format!("{what:?}"));
    assert_eq!(
        winter,
        vec![
            (text("a snowball"), Some(MILL)),
            (text("an icicle"), Some(GREEN))
        ]
    );
    // Then what any season brings, and only then the spring's; the glove
    // by a forge the World has not got never turns up.
    assert_eq!(found[2].0, text("a smooth stone"));
    assert_eq!(found[3], (text("a daisy"), Some(MILL)));
    assert_eq!(found.len(), 4, "{found:?}");
    // The herbs are looked after the winter's way.
    let herbs = lines
        .iter()
        .find(|line| line.contains("your herb bed"))
        .expect("the herbs looked after");
    let winter_ways = PLANTED_IN[3]
        .iter()
        .map(|line| {
            line.split("{who} ")
                .nth(1)
                .unwrap()
                .split(" your")
                .next()
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert!(winter_ways.iter().any(|way| herbs.contains(way)), "{herbs}");
}

/// Each find is told the next way in turn.
#[test]
fn finds_are_told_each_way_in_turn() {
    let (mut world, actions) = world(FINDS);
    let rite = rite(FINDS);
    pass(&mut world, &actions);
    let mut told_as = Vec::new();
    for _ in 0..3 {
        let day = pass(&mut world, &actions);
        let came = morning(&mut world, &actions, &rite, Some(day), &[])
            .unwrap()
            .unwrap();
        let found = parts_of(&world, came)
            .into_iter()
            .find(|part| part.kind == FOUND)
            .unwrap();
        told_as.push(told(found).unwrap());
    }
    assert!(told_as[0].starts_with("You found a blue button by "));
    assert!(told_as[1].starts_with("Down by "), "{told_as:?}");
    assert!(told_as[2].starts_with("You came across a brass bell near "));
}
