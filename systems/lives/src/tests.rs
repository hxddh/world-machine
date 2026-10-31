use super::*;
use std::collections::{BTreeMap, BTreeSet};

const NOTES: EntityId = EntityId::new(100);
const PUB: EntityId = EntityId::new(50);
const QUAY: EntityId = EntityId::new(51);
const MILL: EntityId = EntityId::new(52);
const FUND: EntityId = EntityId::new(53);
const PEOPLE: [u64; 6] = [1, 2, 3, 4, 5, 6];

fn people(world: &World) -> Vec<EntityId> {
    let mut people = PEOPLE.map(EntityId::new).to_vec();
    people.extend(arrivals(world.state(), &cast()));
    people
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

const ACTIVITIES: &[Activity] = &[
    Activity {
        id: "shift",
        need: Need::Money,
        with: With::Workmate,
        at: At::Work,
        told: "{name} worked a long shift at {place} with {other}",
        said: &["Long shift with {other}.", "{place} was busy today."],
        gives: &[("cash", 5)],
    },
    Activity {
        id: "odd_job",
        need: Need::Money,
        with: With::Alone,
        at: At::Work,
        told: "{name} took an odd job",
        said: &["Odd jobs pay.", "A bit extra this week."],
        gives: &[("cash", 3)],
    },
    Activity {
        id: "nap",
        need: Need::Rest,
        with: With::Alone,
        at: At::Quiet,
        told: "{name} dozed at {place}",
        said: &["Forty winks at {place}.", "Lovely and quiet at {place}."],
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
        id: "cards",
        need: Need::Company,
        with: With::Anyone,
        at: At::Gathering,
        told: "{name} played cards with {other}",
        said: &["{other} cheats at cards.", "Won a hand off {other}!"],
        gives: &[],
    },
    Activity {
        id: "help",
        need: Need::Purpose,
        with: With::Anyone,
        at: At::Work,
        told: "{name} helped {other} at {place}",
        said: &["Gave {other} a hand.", "{other} needed help at {place}."],
        gives: &[],
    },
    Activity {
        id: "study",
        need: Need::Purpose,
        with: With::Alone,
        at: At::Quiet,
        told: "{name} read up on something new",
        said: &["Reading about the stars.", "Learning knots."],
        gives: &[],
    },
];

fn visitor(_: &str, job: &str) -> Vec<(String, Value)> {
    vec![
        ("job".into(), job.into()),
        ("works_at".into(), Value::Entity(MILL)),
    ]
}

fn cast() -> Cast {
    Cast {
        notes: NOTES,
        period: 10,
        unit: "day",
        settlement: "village",
        short_of: "money",
        people,
        stays: |id| id.0 <= 2,
        traits: |_| None,
        kept: |_, _| false,
        mood: |_| 0,
        work,
        home,
        gathering: PUB,
        quiet: QUAY,
        host: EntityId::new(1),
        activities: ACTIVITIES,
        topics: &[
            "the mill race",
            "a borrowed ladder",
            "the fence",
            "the vote",
            "a remark at the pub",
        ],
        outings: &["walk to the lighthouse", "a dance"],
        fund: Some(Fund {
            entity: FUND,
            key: "cash",
            name: "the village fund",
            amount: 20,
        }),
        visitors: Some(Visitors {
            first: 200,
            room: 4,
            names: &["Ada", "Bo", "Cy", "Di", "Ed"],
            trades: &[("miller", "miller"), ("weaver", "weaver")],
            origins: &["the hills", "the coast"],
            way_out: "the carrier's cart",
            components: visitor,
            kind: "person",
        }),
        most_people: 8,
        most_open: 2,
        voice: |_| None,
        kin: None,
        kind: crate::never,
        pronouns: crate::they_all,
    }
}

fn world() -> (World, ActionRegistry) {
    let mut state = WorldState::default();
    for (id, name, works) in [
        (1, "Ann", MILL),
        (2, "Ben", MILL),
        (3, "Cat", PUB),
        (4, "Dan", PUB),
        (5, "Eve", QUAY),
        (6, "Fin", QUAY),
    ] {
        state
            .seed_entity(
                Entity::new(EntityId::new(id), "person")
                    .with_component("name", name)
                    .with_component("works_at", Value::Entity(works))
                    .with_component("cash", 20_i64),
            )
            .unwrap();
    }
    for (id, name) in [(PUB, "the Bell"), (QUAY, "the quay"), (MILL, "the mill")] {
        state
            .seed_entity(Entity::new(id, "place").with_component("name", name))
            .unwrap();
    }
    state
        .seed_entity(Entity::new(FUND, "fund").with_component("cash", 1_000_i64))
        .unwrap();
    let mut registry = ActionRegistry::new();
    register_actions(&mut registry, |_| cast()).unwrap();
    (World::new(state), registry)
}

fn pass(world: &mut World, registry: &ActionRegistry) {
    let target = world.world_time() + 10;
    world.advance_to(registry, target).unwrap();
    tick(world, registry, &cast(), false).unwrap();
}

/// Plays `periods`, answering the first answer of whatever is open when
/// `answering`, and returns every situation key that came up.
fn play(periods: u64, answering: bool) -> (World, Vec<(u64, String)>) {
    let (mut world, registry) = world();
    let mut came_up = Vec::new();
    for _ in 0..periods {
        pass(&mut world, &registry);
        if answering {
            for situation in situations(&world, &cast()) {
                // Strangers are welcomed; anything else gets its first answer.
                let welcome = situation.answers.iter().find(|a| a.id == "welcome");
                if let Some(answer) =
                    welcome.or_else(|| situation.answers.iter().find(|a| a.unavailable.is_none()))
                {
                    world
                        .execute(&registry, &answer_request(&situation.key, answer.id))
                        .unwrap();
                }
            }
        }
    }
    for event in world.events() {
        if event.kind == "situation_came_up" {
            if let Some(Value::Text(key)) = event.payload.get("situation") {
                came_up.push((event.world_time / 10, key.clone()));
            }
        }
    }
    (world, came_up)
}

#[test]
fn everyone_lives_a_day_every_period() {
    let (world, _) = play(5, false);
    let now = world.world_time();
    let lived_today = world
        .events()
        .iter()
        .filter(|event| event.kind == "lived" && event.world_time == now)
        .filter_map(|event| event.actor)
        .collect::<BTreeSet<_>>();
    assert_eq!(lived_today.len(), PEOPLE.len());
    assert!(world
        .events()
        .iter()
        .filter(|event| event.kind == "lived")
        .all(|event| told(event).is_some_and(|told| !told.contains('{'))));
}

#[test]
fn people_become_friends_and_fall_out() {
    let (world, _) = play(120, true);
    let bonds = world
        .events()
        .iter()
        .filter(|event| event.kind == "bond_changed")
        .filter_map(|event| match event.payload.get("bond") {
            Some(Value::Text(bond)) => Some(bond.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert!(bonds.contains("became_friends"), "{bonds:?}");
    assert!(bonds.contains("fell_out"), "{bonds:?}");
}

#[test]
fn new_situations_keep_coming() {
    let (_, came_up) = play(240, true);
    let mut seen = BTreeSet::new();
    let mut fresh_by_month = vec![0; 8];
    for (period, key) in came_up {
        if seen.insert(key) {
            fresh_by_month[(period / 30).min(7) as usize] += 1;
        }
    }
    assert!(
        fresh_by_month[3..].iter().all(|fresh| *fresh >= 6),
        "{fresh_by_month:?}"
    );
}

#[test]
fn answers_are_recorded_and_the_world_replays_without_the_system() {
    let (world, came_up) = play(90, true);
    assert!(!came_up.is_empty());
    assert!(world
        .events()
        .iter()
        .any(|event| event.kind == "situation_answered"));
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn a_stranger_can_come_to_stay() {
    let (world, _) = play(60, true);
    assert!(!arrivals(world.state(), &cast()).is_empty());
}

#[test]
fn unanswered_situations_run_out() {
    let (world, _) = play(40, false);
    assert!(world
        .events()
        .iter()
        .any(|event| event.kind == "situation_lapsed"));
    assert!(open(world.state(), &cast()).len() <= cast().most_open);
}

/// Sets components outright, for putting a World where a test needs it.
struct Put(Vec<(EntityId, &'static str, Value)>);

impl Action for Put {
    fn name(&self) -> &'static str {
        "put_for_test"
    }

    fn evaluate(
        &self,
        _state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let mut draft = EventDraft::new("put_for_test");
        draft.changes = self
            .0
            .iter()
            .map(|(entity, key, value)| StateChange::SetComponent {
                entity: *entity,
                key: key.to_string(),
                value: value.clone(),
            })
            .collect();
        Ok(draft)
    }
}

/// Plays `periods` from a World where `put` has been set once everyone is
/// enrolled, answering the first answer offered, and returns the World
/// and every situation that came up.
fn play_from(
    put: Vec<(EntityId, &'static str, Value)>,
    periods: u64,
) -> (World, ActionRegistry, Vec<Candidate>) {
    let (mut world, mut registry) = world();
    registry.register(Put(put)).unwrap();
    pass(&mut world, &registry);
    let mut came_up = Vec::new();
    for _ in 0..periods {
        world
            .execute(&registry, &ActionRequest::new("put_for_test"))
            .unwrap();
        pass(&mut world, &registry);
        for situation in situations(&world, &cast()) {
            if let Some(answer) = situation.answers.iter().find(|a| a.unavailable.is_none()) {
                world
                    .execute(&registry, &answer_request(&situation.key, answer.id))
                    .unwrap();
            }
        }
    }
    for event in world.events() {
        if event.kind == "situation_came_up" {
            if let Some(Value::Text(key)) = event.payload.get("situation") {
                came_up.extend(Candidate::parse(key));
            }
        }
    }
    (world, registry, came_up)
}

#[test]
fn a_warm_friendship_opens_doors_once_each() {
    let cat = EntityId::new(3);
    let (world, _, came_up) = play_from(vec![(cat, REGARD, Value::Integer(80))], 90);
    let doors = came_up
        .iter()
        .filter(|c| {
            c.a == cat
                && matches!(
                    c.kind,
                    Kind::Warming | Kind::Confide | Kind::Invite | Kind::Favour | Kind::Keepsake
                )
        })
        .map(|c| c.kind)
        .collect::<Vec<_>>();
    assert!(door_opened(world.state(), cat, Kind::Warming).is_some());
    assert!(world
        .events()
        .iter()
        .any(|event| event.kind == "warmed" && event.actor == Some(cat)));
    assert_eq!(
        doors,
        vec![Kind::Confide, Kind::Invite, Kind::Favour, Kind::Keepsake],
        "each door once, in turn"
    );
    for kind in [
        Kind::Warming,
        Kind::Confide,
        Kind::Invite,
        Kind::Favour,
        Kind::Keepsake,
    ] {
        assert!(door_opened(world.state(), cat, kind).is_some(), "{kind:?}");
    }
    let given = keepsakes(&world);
    assert!(
        given.iter().any(|k| k.from == cat && !k.what.contains('{')),
        "{given:?}"
    );
    // A player who never answers anyone opens no doors at all.
    let (_, strangers) = play(90, false);
    assert!(
        !strangers.iter().any(|(_, key)| key.starts_with("confide")
            || key.starts_with("invite")
            || key.starts_with("favour")
            || key.starts_with("keepsake")),
        "{strangers:?}"
    );
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn a_grudge_closes_doors_and_shows() {
    let dan = EntityId::new(4);
    let (_, _, came_up) = play_from(
        vec![
            (dan, REGARD, Value::Integer(-60)),
            (dan, Need::Money.key(), Value::Integer(95)),
            (dan, Need::Company.key(), Value::Integer(95)),
        ],
        60,
    );
    let dans = came_up
        .iter()
        .filter(|c| c.a == dan)
        .map(|c| c.kind)
        .collect::<Vec<_>>();
    assert!(
        !dans.iter().any(|kind| kind.asks_for_help()),
        "someone with a grudge asked for help: {dans:?}"
    );
    assert!(dans.contains(&Kind::Cold), "{dans:?}");
}

#[test]
fn someone_says_what_they_make_of_what_you_made() {
    struct Made;
    impl Action for Made {
        fn name(&self) -> &'static str {
            "made_for_test"
        }
        fn evaluate(
            &self,
            _state: &WorldState,
            _request: &ActionRequest,
        ) -> Result<EventDraft, ActionError> {
            let bench = EntityId::new(300);
            let mut draft = EventDraft::new("built_by_hand");
            draft.targets = vec![bench, PUB];
            draft.changes.push(StateChange::CreateEntity(
                Entity::new(bench, "fixture").with_component("name", "Bench"),
            ));
            Ok(draft)
        }
    }
    let (mut world, mut registry) = world();
    registry.register(Made).unwrap();
    pass(&mut world, &registry);
    let deed = world
        .execute(&registry, &ActionRequest::new("made_for_test"))
        .unwrap()
        .id;
    let reacted = react_to(&mut world, &registry, &cast(), deed)
        .unwrap()
        .expect("someone reacts");
    let event = world.event(reacted).unwrap();
    let (who, line) = said(event).expect("they say something");
    assert!(event.caused_by.contains(&deed));
    assert!(
        at(world.state(), who) == Some(PUB) || work(world.state(), who) == Some(PUB),
        "{who:?} is not by the Bell"
    );
    assert!(!line.contains('{'), "{line}");
}

#[test]
fn a_return_brings_a_keepsake_from_someone_who_likes_you() {
    let eve = EntityId::new(5);
    let (mut world, registry, _) = play_from(vec![(eve, REGARD, Value::Integer(90))], 2);
    let left = leave_keepsake(
        &mut world,
        &registry,
        &cast(),
        "The fair was the best in years.",
    )
    .unwrap()
    .expect("a keepsake");
    let event = world.event(left).unwrap();
    assert_eq!(event.actor, Some(eve));
    let kept = keepsakes(&world);
    let last = kept.last().unwrap();
    assert_eq!(last.from, eve);
    assert!(last.note.contains("best in years"), "{last:?}");
    assert!(!last.what.contains('{'));
}

/// What the book, the letter box and the daily round find through the
/// World's index of its history is what reading every event finds, on
/// every period of a long World, with letters and keepsakes along the
/// way, and again on the World rebuilt from its history.
#[test]
fn the_index_finds_what_reading_every_event_finds() {
    let (mut world, registry) = world();
    greet(&mut world, &registry, &cast()).unwrap();
    for day in 0..200 {
        pass(&mut world, &registry);
        if day % 9 == 0 {
            leave_keepsake(&mut world, &registry, &cast(), "").unwrap();
        }
        remember_a_year(&mut world, &registry, &cast(), 60).unwrap();
        scanned::compare(&world, &cast()).unwrap();
    }
    assert!(!keepsakes(&world).is_empty());
    assert!(!letters(&world).is_empty());
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
    scanned::compare(&replayed, &cast()).unwrap();
    assert_eq!(keepsakes(&replayed), keepsakes(&world));
    assert_eq!(letters(&replayed), letters(&world));
    assert_eq!(met(&replayed), met(&world));
}

/// What was said is noted one entry a period, not one a line, and only
/// for as long as it is remembered.
#[test]
fn what_was_said_is_noted_a_period_at_a_time() {
    let (mut world, _) = play(200, true);
    let notes = world.state().entity(NOTES).unwrap();
    let keys = |prefix: &str| {
        notes
            .components
            .keys()
            .filter(|key| key.starts_with(prefix))
            .count()
    };
    let periods = keys(SAID);
    assert!(
        periods > 0 && periods as u64 <= HEARD_PERIODS + 30,
        "{periods}"
    );
    let heard = Heard::of(world.state(), &cast());
    assert!(!heard.said.is_empty());
    // Each period's lines are kept in order, to be looked up.
    assert!(heard.said.iter().all(|(_, lines)| lines.is_sorted()));
    assert!(heard.said.windows(2).all(|pair| pair[0].0 > pair[1].0));
    for hash in [
        0,
        1,
        (1 << (6 * SAID_CODE)) - 1,
        short_hash::<SAID_CODE>(line_hash("Long shift with Ben.")),
    ] {
        assert_eq!(code_hash(&hash_code::<SAID_CODE>(hash)), hash);
    }
    // Said again now, the latest counts, and is noted once.
    let line = "A line said lately.";
    let now = heard.now;
    let mut moves = Moves::default();
    remember_saying(&mut moves, &heard, line);
    remember_saying(&mut moves, &heard, line);
    assert_eq!(moves.changes.len(), 1);
    let state = applied(world.state(), moves.changes);
    assert_eq!(Heard::of(&state, &cast()).when(line), Some(now));
    assert!(Heard::of(&state, &cast()).lately(line));
    // Everything noted is forgotten in time.
    let registry = world_registry();
    for _ in 0..HEARD_PERIODS + 31 {
        let target = world.world_time() + 10;
        world.advance_to(&registry, target).unwrap();
        let _ = world.execute(&registry, &ActionRequest::new("lives_forget"));
    }
    let notes = world.state().entity(NOTES).unwrap();
    assert!(!notes.components.keys().any(|key| key.starts_with(SAID)));
}

/// A trouble's rest that has run out is let go of when lines are forgotten,
/// as nothing would come up any differently without it; one still running
/// is kept.
#[test]
fn rests_that_have_run_out_are_let_go_of() {
    let (world, _) = play(40, true);
    let now = period(world.state(), &cast()) as i64;
    let rests = [
        ("lives.rest.feud.1.2", now - 40, false),
        ("lives.rest.feud.1.3", now - 39, true),
        ("lives.rest.party.1.2", now - 59, true),
        ("lives.rest.learn.2", now - 30, false),
        ("lives.rest.confide.1", now - 11, true),
    ];
    let state = applied(
        world.state(),
        rests
            .iter()
            .map(|(key, at, _)| StateChange::SetComponent {
                entity: NOTES,
                key: (*key).into(),
                value: (*at).into(),
            })
            .collect(),
    );
    let mut world = World::from_history(state, &[]).unwrap();
    world
        .execute(&world_registry(), &ActionRequest::new("lives_forget"))
        .unwrap();
    let notes = world.state().entity(NOTES).unwrap();
    for (key, _, kept) in rests {
        assert_eq!(notes.component(key).is_some(), kept, "{key}");
    }
}

/// A state with some changes made to it, as an event would make them.
fn applied(state: &WorldState, changes: Vec<StateChange>) -> WorldState {
    let event = Event {
        id: EventId::new(1),
        kind: "noted".into(),
        world_time: state.world_time(),
        actor: None,
        targets: Vec::new(),
        caused_by: Vec::new(),
        payload: Default::default(),
        changes,
    };
    World::from_history(state.clone(), &[event])
        .unwrap()
        .state()
        .clone()
}

fn world_registry() -> ActionRegistry {
    world().1
}

/// When a line was last said, and whether lately, looked up by number
/// among the periods still remembered, is what reading every note's codes
/// finds; and a line filled in is filled in as filling every slot does.
#[test]
fn heard_is_what_reading_every_note_finds() {
    let (world, _) = play(200, true);
    let mut lines = world
        .events()
        .iter()
        .filter_map(|event| match event.payload.get("said") {
            Some(Value::Text(said)) => Some(said.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert!(lines.len() > 50, "{}", lines.len());
    let names = ["Ann", "Ben", "the mill", ""];
    for activity in cast().activities {
        for line in activity.said.iter().chain([&activity.told]) {
            for (a, b) in names.iter().zip(names.iter().rev()) {
                let words = [("place", *a), ("other", *b), ("friend", *a), ("name", *b)];
                let filled = fill(line, &words);
                assert_eq!(filled, fill_every_slot(line, &words));
                lines.insert(filled);
            }
        }
    }
    let now = period(world.state(), &cast());
    {
        let heard = Heard::of(world.state(), &cast());
        assert!(heard.said.iter().any(|(at, _)| now - at >= HEARD_PERIODS));
        let (mut found, mut recent) = (0, 0);
        for line in &lines {
            let when = heard.when_by_reading_every_note(line);
            assert_eq!(heard.when(line), when, "{line}");
            let lately = when.is_some_and(|at| now.saturating_sub(at) < HEARD_PERIODS);
            assert_eq!(heard.lately(line), lately, "{line}");
            found += usize::from(when.is_some());
            recent += usize::from(lately);
        }
        assert!(found > recent && recent > 0, "{found} {recent}");
    }
}

#[test]
fn codes_are_sorted_as_sorting_sorts_them() {
    let mut seed = 7_u64;
    let numbers = (0..5_000)
        .map(|at| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            // Some alike at the top, some at the ends of the range.
            match at % 50 {
                0 => 0,
                1 => u64::MAX,
                2 => seed & 0xffff,
                _ => seed,
            }
        })
        .collect::<Vec<_>>();
    let mut sorted = numbers.clone();
    sorted.sort_unstable();
    assert_eq!(sort_codes(numbers), sorted);
    assert!(sort_codes(Vec::new()).is_empty());
}

// ---- Generations ------------------------------------------------------------

const KIN_WORDS: KinWords = KinWords {
    born: &["{a} and {b} had a baby: {name}"],
    born_said: &["Welcome, {name}."],
    came_of_age: &["{name} came of age as a {trade}, a trade of {their} own"],
    came_of_age_said: &["Grown!"],
    left_home: &["{name} moved out"],
    left_home_said: &["My own door."],
    retired: &["{name} retired"],
    retired_said: &["Feet up."],
    died: &["{name} died peacefully, at {age}"],
    died_said: &["We'll miss {name}."],
    heirloom: &["{name}'s {heirloom} passed to {heir}"],
    heirloom_said: &["I'll keep the {heirloom}."],
    memorial_by_player: &["You put up {memorial}"],
    memorial_by_town: &["The village put up {memorial}"],
    memorial_said: &["For {name}."],
    anniversary: &["{heir} remembered {name}"],
    anniversary_said: &["A year since {name}."],
    remember: &[
        "I miss {name}.",
        "{name} would have liked this.",
        "Thinking of {name}.",
    ],
};

/// Ann is ninety, Ben seventeen, Cat and Dan a couple of thirty.
fn test_age(_: &WorldState, id: EntityId) -> Option<u64> {
    Some(match id.0 {
        1 => 90,
        2 => 17,
        3 | 4 => 30,
        5 => 66,
        _ => return None,
    })
}

static TEST_KIN: Kin = Kin {
    year: 12,
    age_at_start: test_age,
    born_at: |_, _| None,
    youngest: 20,
    spread: 30,
    child_at: 2,
    teen_at: 13,
    grown_at: 18,
    elder_at: 65,
    grey_at: 55,
    stoop_at: 75,
    fertile_until: 44,
    retire_at: 67,
    frail_at: 80,
    first_child: 300,
    room: 20,
    names: &["Pip", "Wren"],
    kind: "person",
    newborn: |_, _, _| vec![("job".into(), Value::from("child"))],
    job_key: "job",
    retired_job: "retired",
    learning: &["child", "student"],
    trades: &[("miller", "miller")],
    keeps: |_| false,
    births_now: |_| true,
    busy: |_, _| false,
    birth_odds: 3_000,
    frailty: 400,
    apart: 2,
    most_children: 2,
    heirlooms: &["pocket watch"],
    memorials: &[
        Memorial {
            shape: "bench",
            named: "{name}'s bench",
        },
        Memorial {
            shape: "stone",
            named: "{name}'s stone",
        },
    ],
    first_memorial: 400,
    memorial_at: QUAY,
    memorial_wait: 3,
    mourning: 12,
    words: &KIN_WORDS,
};

fn kin_people(world: &World) -> Vec<EntityId> {
    let mut people = people(world);
    people.extend(grown_here(world.state(), &cast()));
    people
}

fn kin_cast() -> Cast {
    Cast {
        people: kin_people,
        // Everyone stays, so the old grow old here.
        stays: |id| id.0 <= 6,
        kin: Some(&TEST_KIN),
        ..cast()
    }
}

fn kin_world() -> (World, ActionRegistry) {
    let (world, _) = world();
    let mut state = world.state().clone();
    for id in PEOPLE {
        let entity = state.entity(EntityId::new(id)).unwrap().clone();
        let entity = entity.with_component("job", if id == 2 { "student" } else { "miller" });
        // Ben is "he" by his own say.
        let entity = if id == 2 {
            entity.with_component(PRONOUNS, "he")
        } else {
            entity
        };
        state = {
            let mut next = WorldState::default();
            for other in state.entities() {
                next.seed_entity(if other.id == entity.id {
                    entity.clone()
                } else {
                    other.clone()
                })
                .unwrap();
            }
            next
        };
    }
    let mut registry = ActionRegistry::new();
    register_actions(&mut registry, |_| kin_cast()).unwrap();
    (World::new(state), registry)
}

fn kin_pass(world: &mut World, registry: &ActionRegistry) {
    let target = world.world_time() + 10;
    world.advance_to(registry, target).unwrap();
    tick(world, registry, &kin_cast(), false).unwrap();
}

fn of_kind<'a>(world: &'a World, kind: &str) -> Vec<&'a Event> {
    world.events().iter().filter(|e| e.kind == kind).collect()
}

#[test]
fn ages_come_from_who_people_are_and_go_up_with_the_year() {
    let (mut world, registry) = kin_world();
    let cast = kin_cast();
    let state = world.state();
    assert_eq!(age_of(state, &cast, &TEST_KIN, EntityId::new(1)), 90);
    assert_eq!(age_of(state, &cast, &TEST_KIN, EntityId::new(3)), 30);
    // Someone the Pack does not name is given an age from who they are,
    // the same every time.
    let six = age_of(state, &cast, &TEST_KIN, EntityId::new(6));
    assert!((20..50).contains(&six), "{six}");
    let (stage, grey, stoop) = looks_of(state, &cast, EntityId::new(1)).unwrap();
    assert_eq!((stage, grey, stoop), (Stage::Elder, true, true));
    assert_eq!(
        looks_of(state, &cast, EntityId::new(2)).unwrap().0,
        Stage::Teen
    );
    for _ in 0..TEST_KIN.year {
        kin_pass(&mut world, &registry);
    }
    let state = world.state();
    assert_eq!(age_of(state, &cast, &TEST_KIN, EntityId::new(3)), 31);
    assert_eq!(age_of(state, &cast, &TEST_KIN, EntityId::new(6)), six + 1);
}

/// Cat and Dan, together and fond of each other, in a World of kin.
fn fond_couple() -> (WorldState, ActionRegistry, EntityId, EntityId) {
    let (mut world, registry) = kin_world();
    let (cat, dan) = (EntityId::new(3), EntityId::new(4));
    kin_pass(&mut world, &registry);
    // Cat and Dan get together, and are fond of each other.
    let mut changes = Vec::new();
    for (x, y) in [(cat, dan), (dan, cat)] {
        changes.push(StateChange::SetComponent {
            entity: x,
            key: PARTNER.into(),
            value: Value::Entity(y),
        });
        changes.push(StateChange::SetComponent {
            entity: x,
            key: opinion_key(y),
            value: 90.into(),
        });
    }
    let mut state = world.state().clone();
    for change in changes {
        if let StateChange::SetComponent { entity, key, value } = change {
            let e = state
                .entity(entity)
                .unwrap()
                .clone()
                .with_component(key, value);
            let mut next = WorldState::default();
            for other in state.entities() {
                next.seed_entity(if other.id == entity {
                    e.clone()
                } else {
                    other.clone()
                })
                .unwrap();
            }
            state = next;
        }
    }
    (state, registry, cat, dan)
}

/// Births never stop silently: with every id for a child taken, the next
/// birth fails loudly in a test, in debug and in release alike, rather than
/// quietly never happening.
#[test]
#[should_panic(expected = "no room for another child")]
fn a_full_block_of_children_fails_loudly() {
    let (mut state, registry, _, _) = fond_couple();
    for id in TEST_KIN.children().ids() {
        state
            .seed_entity(world_core::Entity::new(id, "stone"))
            .unwrap();
    }
    assert_eq!(state.room_left_in(TEST_KIN.children()), 0);
    let mut world = World::new(state);
    for _ in 0..200 {
        kin_pass(&mut world, &registry);
    }
}

#[test]
fn a_fond_couple_may_have_a_child_who_takes_after_both() {
    let (state, registry, cat, dan) = fond_couple();
    let mut world = World::new(state);
    let mut born = None;
    for _ in 0..200 {
        kin_pass(&mut world, &registry);
        // Nobody may force it: an unfond couple has none.
        if let Some(event) = of_kind(&world, "born").first() {
            born = Some((*event).clone());
            break;
        }
    }
    let born = born.expect("a fond couple had a child in 200 periods");
    let child = born.targets[0];
    let state = world.state();
    assert_eq!(parents(state, child), vec![cat, dan]);
    assert_eq!(lives_with(state, child), Some(cat));
    assert!(told(&born).is_some_and(|told| told.contains("Cat") && !told.contains('{')));
    // A trait from each parent, joining their life only once grown.
    let nature = text(state, child, generations::NATURE).unwrap().to_string();
    assert!(!nature.is_empty());
    assert!(!kin_people(&world).contains(&child));
    assert_eq!(looks_of(state, &kin_cast(), child).unwrap().0, Stage::Baby);
    // They cannot have another straight away.
    assert!(generations::birth_odds(state, &kin_cast(), cat, dan).is_none());
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn someone_comes_of_age_takes_a_trade_and_the_old_retire() {
    let (mut world, registry) = kin_world();
    for _ in 0..(TEST_KIN.year * 2 + 2) {
        kin_pass(&mut world, &registry);
    }
    let came = of_kind(&world, "came_of_age");
    assert_eq!(came.len(), 1, "Ben comes of age once");
    assert_eq!(came[0].actor, Some(EntityId::new(2)));
    assert_eq!(text(world.state(), EntityId::new(2), "job"), Some("miller"));
    // His own trade, as his pronouns have it (v0.31: "Mia is grown now,
    // with a trade of their own").
    assert_eq!(
        told(came[0]).as_deref(),
        Some("Ben came of age as a miller, a trade of his own")
    );
    // Eve turns 67 and retires in time.
    for _ in 0..60 {
        kin_pass(&mut world, &registry);
    }
    assert!(of_kind(&world, "retired")
        .iter()
        .any(|event| event.actor == Some(EntityId::new(5))));
    assert_eq!(
        text(world.state(), EntityId::new(5), "job"),
        Some("retired")
    );
}

#[test]
fn a_death_is_gentle_and_remembered() {
    let (mut world, registry) = kin_world();
    let ann = EntityId::new(1);
    for _ in 0..40 {
        kin_pass(&mut world, &registry);
        if gone(world.state(), ann) {
            break;
        }
    }
    let died = of_kind(&world, "died");
    assert_eq!(died.len(), 1, "Ann, at ninety, dies in her sleep");
    let died = died[0].clone();
    assert_eq!(died.payload.get("who"), Some(&Value::Entity(ann)));
    assert!(told(&died).is_some_and(|told| told.contains("peacefully")));
    // Her heirloom goes to someone close, because of it.
    let heirloom = of_kind(&world, "heirloom_passed");
    assert_eq!(heirloom.len(), 1);
    assert_eq!(heirloom[0].caused_by, vec![died.id]);
    let heir = heirloom[0].actor.unwrap();
    assert_eq!(
        text(world.state(), heir, generations::HEIRLOOM),
        Some("pocket watch")
    );
    // The player may place her bench; left alone, the village does.
    assert_eq!(awaiting_memorial(world.state()), vec![ann]);
    for _ in 0..TEST_KIN.memorial_wait + 1 {
        kin_pass(&mut world, &registry);
    }
    let memorial = of_kind(&world, "memorial_placed");
    assert_eq!(memorial.len(), 1);
    assert_eq!(
        memorial[0].payload.get("by"),
        Some(&Value::Text("town".into()))
    );
    assert_eq!(memorial[0].caused_by, vec![died.id]);
    assert!(awaiting_memorial(world.state()).is_empty());
    // Those close speak of her for a season, and a year on someone keeps
    // the day.
    for _ in 0..TEST_KIN.year {
        kin_pass(&mut world, &registry);
    }
    assert!(world
        .events()
        .iter()
        .any(|event| event.payload.get("remembers") == Some(&Value::Entity(ann))));
    assert_eq!(of_kind(&world, "anniversary_kept").len(), 1);
    // Nobody who is gone lives a day.
    assert!(!world.events().iter().any(|event| event.kind == "lived"
        && event.actor == Some(ann)
        && event.world_time > died.world_time));
    assert_eq!(world.replay().unwrap().state(), world.state());
}

#[test]
fn the_player_places_a_memorial_where_they_choose() {
    let (mut world, registry) = kin_world();
    let ann = EntityId::new(1);
    while !gone(world.state(), ann) {
        kin_pass(&mut world, &registry);
    }
    let event = world
        .execute(&registry, &memorial_request(ann, true, Some(37)))
        .unwrap()
        .clone();
    assert_eq!(event.payload.get("by"), Some(&Value::Text("player".into())));
    let Some(Value::Entity(memorial)) = event.payload.get("memorial") else {
        panic!("no memorial");
    };
    let bench = world.state().entity(*memorial).unwrap();
    assert_eq!(
        bench.component(generations::SPOT),
        Some(&Value::Integer(37))
    );
    assert_eq!(
        bench.component("name"),
        Some(&Value::Text("Ann's bench".into()))
    );
    // Only once.
    assert!(world
        .execute(&registry, &memorial_request(ann, true, None))
        .is_err());
}

#[test]
fn friendships_and_feuds_hold_and_change_one_at_a_time() {
    let (world, _) = play(400, true);
    let changes = world
        .events()
        .iter()
        .filter(|event| event.kind == "bond_changed")
        .collect::<Vec<_>>();
    assert!(!changes.is_empty());
    for pair in changes.windows(2) {
        let gap = (pair[1].world_time - pair[0].world_time) / 10;
        assert!(gap as i64 >= BOND_GAP, "{gap} periods apart");
    }
    for event in &changes {
        assert!(matches!(event.payload.get("because"), Some(Value::Text(why)) if !why.is_empty()));
    }
}

/// An Event as a test's history records it.
fn recorded(id: u64, kind: &str, world_time: u64, actor: Option<EntityId>) -> Event {
    Event {
        id: EventId::new(id),
        kind: kind.into(),
        world_time,
        actor,
        targets: actor.into_iter().collect(),
        caused_by: Vec::new(),
        payload: Default::default(),
        changes: Vec::new(),
    }
}

fn with(mut event: Event, key: &str, value: impl Into<Value>) -> Event {
    event.payload.insert(key.into(), value.into());
    event
}

/// What the player did is found by walking back along what caused
/// something, a few steps, and only ever from what the record names.
#[test]
fn a_players_deed_is_found_back_along_what_it_caused() {
    let (world, _) = world();
    let ann = EntityId::new(1);
    let deed = with(
        recorded(1, "built_by_hand", 10, None),
        "told",
        "You built a bench by the mill",
    );
    let mut reacted = with(recorded(2, "reacted", 10, Some(ann)), "told", "Ann saw it");
    reacted.caused_by = vec![deed.id];
    let mut kept = with(
        recorded(3, "keepsake_left", 20, Some(ann)),
        "told",
        "Ann left you a shell",
    );
    kept.caused_by = vec![reacted.id];
    let alone = with(recorded(4, "lived", 30, Some(ann)), "told", "Ann dozed");
    let answered = with(
        with(
            recorded(5, "situation_answered", 40, Some(ann)),
            "kind",
            "feud",
        ),
        "answer",
        "mend",
    );
    let history = [deed, reacted, kept, alone, answered];
    let world = World::from_history(world.state().clone(), &history).unwrap();
    let event = |id: u64| world.event(EventId::new(id)).unwrap();
    let built = Act::Deed("you built a bench by the mill".into());
    assert_eq!(
        player_act(&world, event(3)),
        Some((EventId::new(1), built.clone()))
    );
    assert_eq!(player_act(&world, event(4)), None);
    // An answer is the player's own act.
    assert_eq!(
        player_act(&world, event(5)),
        Some((EventId::new(5), Act::Said("Make peace".into())))
    );
    // Cited the same for the same seed, in one of two ways.
    assert_eq!(
        cited(&built, 0),
        "All because you built a bench by the mill."
    );
    assert_eq!(
        cited(&built, 1),
        "And to think it started when you built a bench by the mill."
    );
    assert_eq!(
        cited(&Act::Said("Make peace".into()), 0),
        "All because you said “Make peace”."
    );
}

/// A letter whose news the player brought about says what they did, and
/// names it as the letter's cause; news nobody brought about cites
/// nothing.
#[test]
fn a_letter_cites_what_the_player_did() {
    let (start, _) = world();
    let (ann, ben) = (EntityId::new(1), EntityId::new(2));
    let mut answered = with(
        with(
            with(
                recorded(1, "situation_answered", 30, Some(ann)),
                "kind",
                "sweet",
            ),
            "answer",
            "ask",
        ),
        "told",
        "Ann and Ben started walking out together",
    );
    answered.targets = vec![ann, ben];
    let world = World::from_history(start.state().clone(), &[answered]).unwrap();
    let (body, request) = quiet::letter_body(&world, &cast(), ben).unwrap();
    assert!(body.contains("you said “Go for it”."), "{body}");
    assert!(
        body.contains("Ann and I started walking out together"),
        "{body}"
    );
    assert_eq!(request.caused_by, vec![EventId::new(1)]);
    // The same history, the same letter.
    assert_eq!(quiet::letter_body(&world, &cast(), ben).unwrap().0, body);

    let mut bond = with(
        recorded(1, "bond_changed", 30, Some(ann)),
        "told",
        "Ann and Ben became firm friends",
    );
    bond.targets = vec![ben];
    let world = World::from_history(start.state().clone(), &[bond]).unwrap();
    let (body, request) = quiet::letter_body(&world, &cast(), ben).unwrap();
    assert!(
        !body.contains("All because") && !body.contains("started when"),
        "{body}"
    );
    assert!(request.caused_by.is_empty());

    // News that already says what the player did is not told it twice.
    let deed = with(
        recorded(1, "built_by_hand", 20, None),
        "told",
        "You built a bench by the mill",
    );
    let mut drawn = with(
        recorded(2, "drawn_here", 30, Some(ann)),
        "told",
        "Ann came because of the bench you built",
    );
    drawn.caused_by = vec![deed.id];
    let world = World::from_history(start.state().clone(), &[deed, drawn]).unwrap();
    let (body, request) = quiet::letter_body(&world, &cast(), ben).unwrap();
    assert!(body.contains("the bench you built"), "{body}");
    assert!(!body.contains("All because") && !body.contains("started when"));
    // Not told twice, but still the letter's cause.
    assert_eq!(request.caused_by, vec![EventId::new(1)]);

    // A line that only speaks to the player tells nothing of what they
    // chose: the answer is cited all the same.
    let mut answered = with(
        with(
            with(
                recorded(1, "situation_answered", 30, Some(ann)),
                "kind",
                "confide",
            ),
            "answer",
            "keep",
        ),
        "told",
        "Ann told you something she'd never told anyone",
    );
    answered.targets = vec![ann];
    // Long enough after to be looked back on.
    let later = recorded(2, "lived", 30 + 1_000 * cast().period, Some(ben));
    let world = World::from_history(start.state().clone(), &[answered, later]).unwrap();
    let (body, request) = quiet::letter_body(&world, &cast(), ann).unwrap();
    assert!(body.contains("you said “Keep it between you”."), "{body}");
    assert_eq!(request.caused_by, vec![EventId::new(1)]);
}

/// An answer that named a friend is cited with the friend it named, kept
/// in the record when it was given; one from before, which kept no friend,
/// is cited by nothing it cannot fill.
#[test]
fn an_answer_naming_a_friend_is_cited_with_that_friend() {
    let (world, _) = world();
    let ann = EntityId::new(1);
    let short = |id: u64| {
        with(
            with(
                recorded(id, "situation_answered", 30, Some(ann)),
                "kind",
                "short",
            ),
            "answer",
            "friend",
        )
    };
    let history = [with(short(1), "offered", "Ask Ben"), short(2)];
    let world = World::from_history(world.state().clone(), &history).unwrap();
    let event = |id: u64| world.event(EventId::new(id)).unwrap();
    assert_eq!(
        player_act(&world, event(1)),
        Some((EventId::new(1), Act::Said("Ask Ben".into())))
    );
    assert_eq!(player_act(&world, event(2)), None);
}

/// Whether a line already tells what the player did: by the deed's doing
/// or the answer's words, never by a passing "you".
#[test]
fn a_line_tells_a_deed_by_its_doing() {
    let built = Act::Deed("you built a bench by the mill".into());
    assert!(crate::retold::tells(
        "Ann came because of the bench you built",
        &built
    ));
    assert!(!crate::retold::tells("Ann told you something", &built));
    let said = Act::Said("Throw a party".into());
    assert!(crate::retold::tells(
        "They threw a party, as you said: throw a party",
        &said
    ));
    assert!(!crate::retold::tells("Thank you for everything", &said));
}

/// He, she or they: as the Pack says, unless someone carries their own;
/// and a line about someone says it in their words.
#[test]
fn a_situation_speaks_of_someone_in_their_pronouns() {
    let (world, _) = world();
    let (ann, ben) = (EntityId::new(1), EntityId::new(2));
    let cast = Cast {
        pronouns: |_, id| {
            if id.0 == 2 {
                Pronouns::He
            } else {
                Pronouns::She
            }
        },
        ..cast()
    };
    let learn = Candidate {
        kind: Kind::Learn,
        a: ann,
        b: Some(ben),
        topic: 0,
    };
    let words = words_for(world.state(), &cast, &learn);
    let filled = |template: &str| fill_owned(template, &words);
    assert_eq!(
        filled("{b} makes it look easy. Could I learn from {b_them}?"),
        "Ben makes it look easy. Could I learn from him?"
    );
    assert_eq!(
        filled("{a} confided {a_they} {a_were} sweet on {b}"),
        "Ann confided she was sweet on Ben"
    );
    assert_eq!(filled("{a} kept to {a_themselves}"), "Ann kept to herself");
    // Every way of asking it fills each slot it has.
    for prompt in script(Kind::Learn).prompts {
        assert!(!filled(prompt).contains(['{', '}']), "{prompt}");
    }
    assert!(!compose(world.state(), &cast, &learn).prompt.contains('{'));
    // A Pack that says no more speaks of everyone as "they", as before.
    let words = words_for(world.state(), &crate::tests::cast(), &learn);
    assert_eq!(
        fill_owned("Could I learn from {b_them}?", &words),
        "Could I learn from them?"
    );
    // Someone's own pronouns come before the Pack's.
    let state = applied(
        world.state(),
        vec![StateChange::SetComponent {
            entity: ben,
            key: PRONOUNS.into(),
            value: "they".into(),
        }],
    );
    assert_eq!(pronouns(&state, &cast, ben), Pronouns::They);
    assert_eq!(pronouns(&state, &cast, ann), Pronouns::She);
}

/// The same kind of day is told of the same person at most five times a
/// year; the record keeps every one, and a new year tells it again.
#[test]
fn the_same_story_is_told_of_someone_at_most_five_times_a_year() {
    let (world, _) = world();
    let (ann, ben) = (EntityId::new(1), EntityId::new(2));
    let year = year_length(&cast());
    assert_eq!(year, 365 * 10);
    let day = |id: u64, at: u64, who: EntityId, activity: &str| {
        with(
            with(recorded(id, "lived", at, Some(who)), "activity", activity),
            "told",
            format!("{} did {activity}", who.0),
        )
    };
    let mut history = (1..=7)
        .map(|n| day(n, n * 10, ann, "odd_job"))
        .collect::<Vec<_>>();
    history.push(day(8, 80, ann, "nap"));
    history.push(day(9, 90, ben, "odd_job"));
    history.push(day(10, year + 10, ann, "odd_job"));
    let world = World::from_history(world.state().clone(), &history).unwrap();
    let capped = (1..=10)
        .filter(|id| told_enough(&world, world.event(EventId::new(*id)).unwrap(), year))
        .collect::<Vec<_>>();
    assert_eq!(capped, vec![6, 7]);
    assert_eq!(world.events().len(), 10);
    assert_eq!(MOST_TOLD_A_YEAR, 5);
}

/// The year's counts are kept in one short note with what someone last
/// did ("odd_job|0|01"), a character a kind of day in the Pack's order.
#[test]
fn a_years_days_are_kept_in_one_note_with_what_was_last_done() {
    let cast = cast();
    let mut state = WorldState::default();
    state
        .seed_entity(Entity::new(EntityId::new(7), "person").with_component("lives.did", "shift"))
        .unwrap();
    let person = EntityId::new(7);
    // Nothing counted yet this year: the first.
    let lived = crate::retold::Days::of(&state, person, 2).lived(&cast, "odd_job");
    assert_eq!(lived.nth, 1);
    assert_eq!(lived.note, "odd_job|2|01");
    // Kept as the note: read back, and counted on.
    let kept = Entity::new(EntityId::new(8), "person").with_component("lives.did", lived.note);
    state.seed_entity(kept).unwrap();
    let person = EntityId::new(8);
    assert_eq!(crate::retold::last_did(&state, person), "odd_job");
    let nap = crate::retold::Days::of(&state, person, 2).lived(&cast, "nap");
    assert_eq!((nap.nth, nap.note.as_str()), (1, "nap|2|011"));
    // A new year starts the count again.
    let next = crate::retold::Days::of(&state, person, 3).lived(&cast, "shift");
    assert_eq!((next.nth, next.note.as_str()), (1, "shift|3|1"));
}

/// Which time this year a day is told is counted when it is lived and kept
/// on the person, so a World resumed from a checkpoint, its earlier Events
/// gone, caps the year as one that kept them all.
#[test]
fn a_day_carries_which_time_this_year_it_was_told() {
    let (played, _) = play(200, true);
    let year = year_length(&cast());
    let mut counted = BTreeMap::<(EntityId, String, u64), i64>::new();
    let mut days = 0;
    for event in played.events().iter().filter(|event| event.kind == "lived") {
        let (Some(who), Some(Value::Text(activity))) = (event.actor, event.payload.get("activity"))
        else {
            continue;
        };
        let nth = counted
            .entry((who, activity.clone(), event.world_time / year))
            .or_default();
        *nth += 1;
        assert_eq!(event.payload.get("told_nth"), Some(&Value::Integer(*nth)));
        days += 1;
    }
    assert!(days > 0);
    // Alone in the record, a sixth telling is still the sixth.
    let (start, _) = world();
    let ann = EntityId::new(1);
    let sixth = with(
        with(recorded(1, "lived", 60, Some(ann)), "activity", "odd_job"),
        "told_nth",
        6_i64,
    );
    let fifth = with(
        with(recorded(2, "lived", 70, Some(ann)), "activity", "nap"),
        "told_nth",
        5_i64,
    );
    let resumed = World::from_history(start.state().clone(), &[sixth, fifth]).unwrap();
    let event = |id: u64| resumed.event(EventId::new(id)).unwrap();
    assert!(told_enough(&resumed, event(1), year));
    assert!(!told_enough(&resumed, event(2), year));
    // A World from before, upgraded mid-year: the days it lived before
    // they were counted add to the count of those after.
    let before = |id: u64| {
        with(
            recorded(id, "lived", id * 10, Some(ann)),
            "activity",
            "odd_job",
        )
    };
    let after = |id: u64, nth: i64| {
        with(
            with(
                recorded(id, "lived", id * 10, Some(ann)),
                "activity",
                "odd_job",
            ),
            "told_nth",
            nth,
        )
    };
    let history = [
        before(1),
        before(2),
        before(3),
        after(4, 1),
        after(5, 2),
        after(6, 3),
    ];
    let upgraded = World::from_history(start.state().clone(), &history).unwrap();
    let capped = (1..=6)
        .filter(|id| told_enough(&upgraded, upgraded.event(EventId::new(*id)).unwrap(), year))
        .collect::<Vec<_>>();
    assert_eq!(capped, vec![6]);
}
