use super::*;
use std::collections::BTreeSet;

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
        people,
        stays: |id| id.0 <= 2,
        traits: |_| None,
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
    register_actions(&mut registry, cast).unwrap();
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
                if let Some(answer) = situation.answers.iter().find(|a| a.unavailable.is_none()) {
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

#[test]
#[ignore]
fn show_what_comes_up() {
    let (world, came_up) = play(240, true);
    let mut kinds = BTreeMap::<String, usize>::new();
    for (_, key) in &came_up {
        *kinds
            .entry(key.split('.').next().unwrap().to_string())
            .or_default() += 1;
    }
    eprintln!("{kinds:?} total {}", came_up.len());
    let bonds = world
        .events()
        .iter()
        .filter(|e| e.kind == "bond_changed")
        .count();
    eprintln!("bonds {bonds}");
    for p in PEOPLE {
        let id = EntityId::new(p);
        eprintln!(
            "{p}: lacks {:?} regard {:?}",
            Need::ALL.map(|n| lack(world.state(), id, n)),
            integer(world.state(), id, REGARD)
        );
    }
    for e in world.events().iter().filter(|e| e.kind == "lived").take(12) {
        eprintln!("{} | {}", told(e).unwrap(), said(e).unwrap().1);
    }
}
