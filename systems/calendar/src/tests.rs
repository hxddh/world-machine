use super::*;

const NOTES: EntityId = EntityId::new(100);
const SQUARE: EntityId = EntityId::new(50);
const ANN: EntityId = EntityId::new(1);
const MOOD: &str = "mood";

const FESTIVALS: &[Festival] = &[
    Festival {
        id: "fair",
        name: "Summer Fair",
        day: 5,
        prepare: 3,
        at: SQUARE,
        speaker: ANN,
        shape: "stall",
        harvest: false,
        nears: "{name} is {days} days away.",
        getting_ready: "Only {days} days to the fair!",
        told: ["A grand {name}", "A fine {name}", "A thin {name}"],
        said: ["Wonderful!", "Nice enough.", "Nobody came."],
    },
    Festival {
        id: "harvest",
        name: "Harvest Home",
        day: 9,
        prepare: 0,
        at: SQUARE,
        speaker: ANN,
        shape: "tent",
        harvest: true,
        nears: "",
        getting_ready: "",
        told: ["A rich harvest", "A fair harvest", "A poor harvest"],
        said: ["Look at it all!", "Enough to go round.", "Hardly anything."],
    },
];

fn almanac(_: &WorldState) -> Almanac {
    Almanac {
        notes: NOTES,
        first: 300,
        room: 4,
        period: 10,
        year: 12,
        seasons: ["spring", "summer", "autumn", "winter"],
        festivals: FESTIVALS,
        people: |_| vec![ANN],
        festive: |state| integer(state, SQUARE, MOOD).unwrap_or(0),
        grown: |state| integer(state, SQUARE, "grown").unwrap_or(0),
        held: |state, festival, turnout, grown| {
            if festival.harvest {
                vec![StateChange::SetComponent {
                    entity: SQUARE,
                    key: "stores".into(),
                    value: (grown * 10).into(),
                }]
            } else if turnout == Turnout::Grand {
                vec![StateChange::SetComponent {
                    entity: SQUARE,
                    key: MOOD.into(),
                    value: (integer(state, SQUARE, MOOD).unwrap_or(0) + 1).into(),
                }]
            } else {
                Vec::new()
            }
        },
    }
}

fn world(mood: i64, grown: i64) -> (World, ActionRegistry) {
    let mut state = WorldState::default();
    state
        .seed_entity(
            Entity::new(ANN, "person")
                .with_component("name", "Ann")
                .with_component("lives.traits", "warm steady")
                .with_component(lives::Need::Company.key(), 50_i64),
        )
        .unwrap();
    state
        .seed_entity(
            Entity::new(SQUARE, "place")
                .with_component("name", "Square")
                .with_component(MOOD, mood)
                .with_component("grown", grown),
        )
        .unwrap();
    let mut actions = ActionRegistry::new();
    register_actions(&mut actions, almanac).unwrap();
    (World::new(state), actions)
}

fn live_days(world: &mut World, actions: &ActionRegistry, days: u64) -> Vec<String> {
    let mut told = Vec::new();
    for _ in 0..days {
        let events = tick(world, actions, &almanac(world.state())).unwrap();
        for id in events {
            told.extend(world.event(id).and_then(told_of));
        }
        let next = world.world_time() + 10;
        world.advance_to(actions, next).unwrap();
    }
    told
}

fn told_of(event: &Event) -> Option<String> {
    told(event)
}

#[test]
fn people_get_ready_and_each_day_comes_once_a_year() {
    let (mut world, actions) = world(7, 0);
    let told = live_days(&mut world, &actions, 24);
    assert_eq!(
        told,
        [
            "Summer Fair is 3 days away.",
            "A grand Summer Fair",
            "A rich harvest",
            "Summer Fair is 3 days away.",
            "A grand Summer Fair",
            "A rich harvest",
        ]
    );
    // It can't be held twice, or on another day.
    let again = ActionRequest::new("calendar_holds").arg("festival", "fair");
    assert!(world.execute(&actions, &again).is_err());
    assert_eq!(world.replay().unwrap().state(), world.state());
}

#[test]
fn how_a_day_goes_follows_the_place_and_a_harvest_what_was_grown() {
    for (mood, grown, fair, harvest) in [
        (0, 0, "A thin Summer Fair", "A poor harvest"),
        (3, 0, "A fine Summer Fair", "A fair harvest"),
        (0, 6, "A thin Summer Fair", "A rich harvest"),
    ] {
        let (mut world, actions) = world(mood, grown);
        let told = live_days(&mut world, &actions, 10);
        assert!(told.contains(&fair.to_string()), "{told:?}");
        assert!(told.contains(&harvest.to_string()), "{told:?}");
        assert_eq!(
            integer(world.state(), SQUARE, "stores"),
            Some(grown * 10),
            "{told:?}"
        );
    }
}

#[test]
fn a_festival_puts_something_up_for_the_day_and_brings_people_together() {
    let (mut world, actions) = world(7, 0);
    live_days(&mut world, &actions, 5);
    let before = lives::lack(world.state(), ANN, lives::Need::Company);
    live_days(&mut world, &actions, 1);
    let stall = world
        .state()
        .entities()
        .find(|entity| entity.kind == "fixture")
        .expect("a stall for the fair");
    assert_eq!(stall.component("shape"), Some(&Value::Text("stall".into())));
    assert_eq!(stall.component("until"), Some(&Value::Integer(6)));
    assert_eq!(
        lives::lack(world.state(), ANN, lives::Need::Company),
        before - 15
    );
    let (festival, days) = upcoming(world.state(), &almanac(world.state())).unwrap();
    assert_eq!((festival.id, days), ("harvest", 3));
    assert_eq!(
        coming_up(world.state(), &almanac(world.state()), 7).as_deref(),
        Some("Harvest Home in 3 days")
    );
    assert_eq!(
        season_name(world.state(), &almanac(world.state())),
        "autumn"
    );
}
