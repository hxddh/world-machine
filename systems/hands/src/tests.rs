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

const WORKS: &[Thing] = &[
    Thing {
        id: "bandstand",
        name: "Bandstand",
        verb: Verb::Build,
        shape: "bandstand",
        cost: 40,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "boathouse",
        name: "Boathouse",
        verb: Verb::Build,
        shape: "boathouse",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
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
        works: WORKS,
        plots: |_| {
            vec![
                Plot {
                    id: "p1".into(),
                    at: QUAY,
                    offers: vec!["bandstand", "boathouse"],
                },
                Plot {
                    id: "p2".into(),
                    at: SQUARE,
                    offers: vec!["bandstand"],
                },
            ]
        },
        wears: |state, id| {
            (text(state, id, "hands.thing") == Some("boathouse") || id == SQUARE).then_some("sail")
        },
        naming: |state, id| made(state).contains(&id).then(|| "work".to_string()),
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

#[test]
fn only_this_periods_count_of_deeds_is_kept() {
    let (mut world, registry) = world(1_000);
    for _ in 0..5 {
        let deed = deeds(&world, &kit(world.state()))
            .into_iter()
            .find(|deed| deed.unavailable.is_none() && deed.verb == Verb::Build)
            .unwrap();
        world.execute(&registry, &do_request(&deed.key)).unwrap();
        pass(&mut world, &registry);
    }
    let counts = |world: &World| {
        world
            .state()
            .entity(NOTES)
            .unwrap()
            .components
            .keys()
            .filter(|key| key.starts_with(DONE))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(counts(&world).len(), 1, "{:?}", counts(&world));
    // What is left for today is counted as before.
    let deed = deeds(&world, &kit(world.state()))
        .into_iter()
        .find(|deed| deed.unavailable.is_none() && deed.verb == Verb::Build)
        .unwrap();
    world.execute(&registry, &do_request(&deed.key)).unwrap();
    assert_eq!(left_this_period(world.state(), &kit(world.state())), 1);
    assert_eq!(
        counts(&world),
        vec![done_key(period(world.state(), &kit(world.state())))]
    );
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn a_plot_takes_one_work_the_place_finishes() {
    let (mut world, registry) = world(100);
    let plots = plot_deeds(&world, &kit(world.state()));
    assert_eq!(plots.len(), 2);
    assert_eq!(plots[0].1.len(), 2);
    let key = plot_key("bandstand", "p1");
    assert_eq!(plots[0].1[0].key, key);
    // Not offered there, or not a plot at all: refused.
    assert!(world
        .execute(&registry, &do_request(&plot_key("boathouse", "p2")))
        .is_err());
    assert!(world
        .execute(&registry, &do_request(&plot_key("boathouse", "p9")))
        .is_err());
    let began = world.execute(&registry, &do_request(&key)).unwrap().clone();
    assert_eq!(began.kind, "built_by_hand");
    let work = made(world.state())[0];
    assert!(building(world.state(), work));
    assert_eq!(plot_of(world.state(), work), Some("p1"));
    assert_eq!(integer(world.state(), FUND, "cash"), Some(60));
    // The plot is taken, and the bandstand is built once: p2 has nothing
    // left to offer.
    let plots = plot_deeds(&world, &kit(world.state()));
    assert!(plots.iter().all(|(_, deeds)| deeds.is_empty()));
    assert!(world
        .execute(&registry, &do_request(&plot_key("bandstand", "p2")))
        .is_err());
    // It takes none of the room for what is made by hand, and is not moved.
    assert!(deeds(&world, &kit(world.state()))
        .iter()
        .all(|deed| deed.verb != Verb::Move));
    for _ in 0..3 {
        pass(&mut world, &registry);
    }
    let finished = world
        .events()
        .iter()
        .find(|event| event.kind == "plot_finished")
        .expect("the place finished it");
    assert_eq!(finished.caused_by, vec![began.id]);
    assert!(!building(world.state(), work));
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn designs_and_names_are_checked_and_kept() {
    let (mut world, registry) = world(100);
    world
        .execute(&registry, &do_request(&plot_key("boathouse", "p1")))
        .unwrap();
    let boathouse = made(world.state())[0];
    let cells = "01".repeat(128);
    let design = format!("{cells}:8a");
    // Only what can wear one, and only a whole design.
    let bench = world
        .execute(&registry, &do_request(&format!("build.bench.{}", QUAY.0)))
        .unwrap()
        .targets[0];
    assert!(world
        .execute(&registry, &design_request(bench, &design))
        .is_err());
    for broken in [
        format!("{cells}:8"),
        format!("{cells}:88"),
        format!("{}:8a", &cells[1..]),
        cells.clone(),
    ] {
        assert!(world
            .execute(&registry, &design_request(boathouse, &broken))
            .is_err());
    }
    let painted = world
        .execute(&registry, &design_request(boathouse, &design))
        .unwrap()
        .kind
        .clone();
    assert_eq!(painted, "designed");
    assert_eq!(pattern_of(world.state(), boathouse), Some(design.as_str()));
    // Names: tidied, one line, one to twenty-four letters.
    for bad in ["", "   ", "two\nlines", &"x".repeat(25)] {
        assert!(world
            .execute(&registry, &name_request(boathouse, bad))
            .is_err());
    }
    assert!(world.execute(&registry, &name_request(ANN, "Bo")).is_err());
    let named_event = world
        .execute(&registry, &name_request(boathouse, "  Old Reliable "))
        .unwrap()
        .clone();
    assert_eq!(
        named_event.payload.get("told"),
        Some(&Value::Text("You named the work Old Reliable".into()))
    );
    assert_eq!(name(world.state(), boathouse), "Old Reliable");
    assert!(named(world.state(), boathouse));
    assert_eq!(was_called(world.state(), boathouse), Some("Boathouse"));
    world
        .execute(&registry, &name_request(boathouse, "Second Thoughts"))
        .unwrap();
    assert_eq!(was_called(world.state(), boathouse), Some("Boathouse"));
    // Finished, it keeps the name it was given.
    for _ in 0..3 {
        pass(&mut world, &registry);
    }
    assert!(!building(world.state(), boathouse));
    assert_eq!(name(world.state(), boathouse), "Second Thoughts");
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn a_named_plant_grows_and_keeps_its_name() {
    let (mut world, registry) = world(100);
    world
        .execute(&registry, &do_request(&format!("plant.garden.{}", QUAY.0)))
        .unwrap();
    let garden = made(world.state())[0];
    world
        .execute(&registry, &name_request(garden, "Ann's Patch"))
        .unwrap();
    for _ in 0..10 {
        pass(&mut world, &registry);
    }
    assert_eq!(name(world.state(), garden), "Ann's Patch");
    assert_eq!(text(world.state(), garden, "shape"), Some("garden"));
    let grew = world
        .events()
        .iter()
        .filter(|event| event.kind == "plant_grew")
        .count();
    assert_eq!(grew, 2, "each stage once");
}
