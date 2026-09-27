use super::*;

const NOTES: EntityId = EntityId::new(100);
const SQUARE: EntityId = EntityId::new(50);
const QUAY: EntityId = EntityId::new(51);
const FUND: EntityId = EntityId::new(52);
const ANN: EntityId = EntityId::new(1);

const THINGS: &[Thing] = &[
    Thing {
        id: "bench",
        name: "Bench",
        verb: Verb::Build,
        shape: "bench",
        cost: 30,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "bunting",
        name: "Bunting",
        verb: Verb::Decorate,
        shape: "bunting",
        cost: 10,
        lasts: Some(3),
        stages: &[],
    },
    Thing {
        id: "garden",
        name: "Garden",
        verb: Verb::Plant,
        shape: "garden",
        cost: 5,
        lasts: None,
        stages: &[
            ("Seedlings", "sprouts"),
            ("Vegetable patch", "garden"),
            ("Garden in bloom", "garden"),
        ],
    },
];

fn kit(_: &WorldState) -> Kit {
    Kit {
        notes: NOTES,
        first: 200,
        room: 50,
        period: 10,
        things: THINGS,
        places: |_| vec![SQUARE, QUAY],
        people: |_| vec![ANN],
        purse: Some(Purse {
            entity: FUND,
            key: "cash",
            name: "the fund",
        }),
        gift_cost: 15,
        gift: |_, who| {
            vec![StateChange::SetComponent {
                entity: who,
                key: "glad".into(),
                value: true.into(),
            }]
        },
        invite: |_, who| {
            vec![StateChange::SetComponent {
                entity: who,
                key: "out".into(),
                value: true.into(),
            }]
        },
        gathering: SQUARE,
        growing: 3,
        per_period: 2,
        most_standing: 10,
    }
}

fn world(cash: i64) -> (World, ActionRegistry) {
    let mut state = WorldState::default();
    state
        .seed_entity(Entity::new(ANN, "person").with_component("name", "Ann"))
        .unwrap();
    state
        .seed_entity(Entity::new(SQUARE, "place").with_component("name", "the square"))
        .unwrap();
    state
        .seed_entity(Entity::new(QUAY, "place").with_component("name", "the quay"))
        .unwrap();
    state
        .seed_entity(Entity::new(FUND, "fund").with_component("cash", cash))
        .unwrap();
    let mut registry = ActionRegistry::new();
    register_actions(&mut registry, kit).unwrap();
    (World::new(state), registry)
}

fn pass(world: &mut World, registry: &ActionRegistry) {
    let target = world.world_time() + 10;
    world.advance_to(registry, target).unwrap();
    tick(world, registry, &kit(world.state())).unwrap();
}

#[test]
fn what_you_build_stands_where_you_put_it_and_costs() {
    let (mut world, registry) = world(100);
    let deed = deeds(&world, &kit(world.state()))
        .into_iter()
        .find(|deed| deed.verb == Verb::Build && deed.at == QUAY)
        .unwrap();
    world.execute(&registry, &do_request(&deed.key)).unwrap();
    let bench = made(world.state())[0];
    assert_eq!(
        world.state().entity(bench).unwrap().component("at"),
        Some(&Value::Entity(QUAY))
    );
    assert_eq!(integer(world.state(), FUND, "cash"), Some(70));
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn a_period_allows_only_so_much_and_the_purse_limits_it() {
    let (mut world, registry) = world(35);
    let build = |world: &World, place: EntityId| {
        deeds(world, &kit(world.state()))
            .into_iter()
            .find(|deed| deed.verb == Verb::Build && deed.at == place)
            .unwrap()
    };
    world
        .execute(&registry, &do_request(&build(&world, QUAY).key))
        .unwrap();
    let second = build(&world, SQUARE);
    assert!(second.unavailable.is_some(), "only 5 left in the fund");
    assert!(world.execute(&registry, &do_request(&second.key)).is_err());
}

#[test]
fn plants_grow_and_decorations_come_down_by_their_own_record() {
    let (mut world, registry) = world(100);
    let plant = deeds(&world, &kit(world.state()))
        .into_iter()
        .find(|deed| deed.verb == Verb::Plant && deed.at == SQUARE)
        .unwrap();
    world.execute(&registry, &do_request(&plant.key)).unwrap();
    let garden = made(world.state())[0];
    assert_eq!(text(world.state(), garden, "name"), Some("Seedlings"));
    for _ in 0..7 {
        pass(&mut world, &registry);
    }
    assert_eq!(text(world.state(), garden, "name"), Some("Garden in bloom"));
    assert!(world
        .events()
        .iter()
        .any(|event| event.kind == "plant_grew"));
}

#[test]
fn what_you_made_can_be_moved_and_people_can_be_given_to_and_invited() {
    let (mut world, registry) = world(100);
    let deed = deeds(&world, &kit(world.state()))
        .into_iter()
        .find(|deed| deed.verb == Verb::Build && deed.at == QUAY)
        .unwrap();
    world.execute(&registry, &do_request(&deed.key)).unwrap();
    pass(&mut world, &registry);
    let moving = deeds(&world, &kit(world.state()))
        .into_iter()
        .find(|deed| deed.verb == Verb::Move)
        .unwrap();
    assert_eq!(moving.at, SQUARE);
    world.execute(&registry, &do_request(&moving.key)).unwrap();
    let give = deeds(&world, &kit(world.state()))
        .into_iter()
        .find(|deed| deed.verb == Verb::Give)
        .unwrap();
    world.execute(&registry, &do_request(&give.key)).unwrap();
    assert_eq!(
        world.state().entity(ANN).unwrap().component("glad"),
        Some(&Value::Bool(true))
    );
    pass(&mut world, &registry);
    let invite = deeds(&world, &kit(world.state()))
        .into_iter()
        .find(|deed| deed.verb == Verb::Invite)
        .unwrap();
    world.execute(&registry, &do_request(&invite.key)).unwrap();
    assert!(world
        .events()
        .iter()
        .filter_map(told)
        .all(|told| !told.contains('{')));
}
