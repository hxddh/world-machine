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
        effect: Effect::Rest,
    },
    Thing {
        id: "bunting",
        name: "Bunting",
        verb: Verb::Decorate,
        shape: "bunting",
        cost: 10,
        lasts: Some(3),
        stages: &[],
        effect: Effect::None,
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
        effect: Effect::Harvest,
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
        enjoy: |_, who, effect| {
            vec![StateChange::SetComponent {
                entity: who,
                key: format!("enjoyed.{}", effect.id()),
                value: true.into(),
            }]
        },
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

#[test]
fn only_someone_living_here_can_be_given_to_or_invited() {
    let (mut world, registry) = world(100);
    for verb in ["give", "invite"] {
        for at in [SQUARE, FUND, EntityId::new(999)] {
            let key = format!("{verb}.someone.{}", at.0);
            assert!(
                world.execute(&registry, &do_request(&key)).is_err(),
                "{key} should be refused"
            );
        }
    }
    assert_eq!(
        world.state().entity(SQUARE).unwrap().component("glad"),
        None
    );
}

#[test]
fn what_you_make_can_stand_anywhere_along_the_ground_and_move_there() {
    let (mut world, registry) = world(100);
    let deed = deeds(&world, &kit(world.state()))
        .into_iter()
        .find(|deed| deed.verb == Verb::Build && deed.at == QUAY)
        .unwrap();
    world
        .execute(&registry, &do_request(&at_spot(&deed.key, 37)))
        .unwrap();
    let bench = made(world.state())[0];
    assert_eq!(integer(world.state(), bench, SPOT), Some(37));
    // A spot beyond the ground, or on a person, is refused.
    assert!(world
        .execute(&registry, &do_request(&format!("{}@140", deed.key)))
        .is_err());
    pass(&mut world, &registry);
    let moved = format!("move.{}.{}", bench.0, SQUARE.0);
    world
        .execute(&registry, &do_request(&at_spot(&moved, 80)))
        .unwrap();
    assert_eq!(integer(world.state(), bench, SPOT), Some(80));
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn what_you_just_made_or_moved_can_be_taken_back_the_same_period() {
    let (mut world, registry) = world(100);
    let deed = deeds(&world, &kit(world.state()))
        .into_iter()
        .find(|deed| deed.verb == Verb::Build && deed.at == QUAY)
        .unwrap();
    world.execute(&registry, &do_request(&deed.key)).unwrap();
    assert_eq!(integer(world.state(), FUND, "cash"), Some(70));
    assert_eq!(
        can_undo(world.state(), &kit(world.state())).as_deref(),
        Some("Take back the bench")
    );
    world.execute(&registry, &undo_request()).unwrap();
    assert!(made(world.state()).is_empty(), "the bench is gone again");
    assert_eq!(
        integer(world.state(), FUND, "cash"),
        Some(100),
        "and paid back"
    );
    assert_eq!(left_this_period(world.state(), &kit(world.state())), 2);
    assert!(
        world.execute(&registry, &undo_request()).is_err(),
        "once only"
    );
    // A move is taken back to where it stood; the next period, nothing is.
    world
        .execute(&registry, &do_request(&at_spot(&deed.key, 20)))
        .unwrap();
    let bench = made(world.state())[0];
    pass(&mut world, &registry);
    assert!(can_undo(world.state(), &kit(world.state())).is_none());
    world
        .execute(
            &registry,
            &do_request(&format!("move.{}.{}", bench.0, SQUARE.0)),
        )
        .unwrap();
    world.execute(&registry, &undo_request()).unwrap();
    let entity = world.state().entity(bench).unwrap();
    assert_eq!(entity.component("at"), Some(&Value::Entity(QUAY)));
    assert_eq!(integer(world.state(), bench, SPOT), Some(20));
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn people_use_what_you_make() {
    let (mut world, registry) = world(100);
    let build = deeds(&world, &kit(world.state()))
        .into_iter()
        .find(|deed| deed.verb == Verb::Build && deed.at == QUAY)
        .unwrap();
    world.execute(&registry, &do_request(&build.key)).unwrap();
    let plant = deeds(&world, &kit(world.state()))
        .into_iter()
        .find(|deed| deed.verb == Verb::Plant && deed.at == SQUARE)
        .unwrap();
    world.execute(&registry, &do_request(&plant.key)).unwrap();
    for _ in 0..16 {
        pass(&mut world, &registry);
    }
    let enjoyed = world
        .events()
        .iter()
        .filter(|event| event.kind == "enjoyed")
        .collect::<Vec<_>>();
    let rested = enjoyed
        .iter()
        .find(|event| event.payload.get("effect") == Some(&Value::Text("rest".into())))
        .expect("someone rests on the bench");
    assert_eq!(rested.actor, Some(ANN));
    assert!(said(rested).is_some(), "and says so");
    assert_eq!(
        world.state().entity(ANN).unwrap().component("enjoyed.rest"),
        Some(&Value::Bool(true))
    );
    // Once the garden has grown, what it grows is brought to the player.
    let harvest = enjoyed
        .iter()
        .find(|event| event.payload.get("effect") == Some(&Value::Text("harvest".into())))
        .expect("the garden gives");
    assert_eq!(harvest.payload.get("kept"), Some(&Value::Bool(true)));
    assert!(matches!(
        harvest.payload.get("keepsake"),
        Some(Value::Text(_))
    ));
    let grown_at = world
        .events()
        .iter()
        .filter(|event| event.kind == "plant_grew")
        .map(|event| event.world_time)
        .max()
        .unwrap();
    assert!(harvest.world_time > grown_at, "only once it has grown");
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}
