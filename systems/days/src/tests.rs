use super::*;

const STRETCHES: [Stretch; 3] = [
    Stretch {
        id: "left",
        from: 0.0,
        to: 1.0,
    },
    Stretch {
        id: "middle",
        from: 1.0,
        to: 2.0,
    },
    Stretch {
        id: "right",
        from: 2.0,
        to: 3.0,
    },
];

#[test]
fn slots_are_never_shared_and_spill_into_the_nearest_stretch() {
    let mut street = Street::new(
        3.0,
        &STRETCHES,
        &[Row {
            pitch: 0.25,
            offset: 0.0,
        }],
    );
    assert_eq!(street.slots(0), 12);
    let mut taken = Vec::new();
    for key in 0..6 {
        let near = street.spread(1, key);
        taken.push(street.take(0, Some(1), near).unwrap());
    }
    // The middle holds four; the other two stand next door.
    assert_eq!(
        taken.iter().filter(|px| STRETCHES[1].holds(**px)).count(),
        4
    );
    let distinct = taken
        .iter()
        .map(|px| (px * 1000.0) as i64)
        .collect::<BTreeSet<_>>();
    assert_eq!(distinct.len(), taken.len());
    for _ in 6..12 {
        assert!(street.take(0, Some(0), 0.0).is_some());
    }
    assert_eq!(street.take(0, None, 1.5), None);
}

#[test]
fn couples_share_a_home_and_children_live_with_their_parents() {
    let people = [1_u64, 2, 3, 7, 9];
    let partner = |person| match person {
        2 => Some(7),
        7 => Some(2),
        3 => Some(40), // with someone who left
        _ => None,
    };
    let parent = |person| (person == 9).then_some(7);
    let homes = households(&people, partner, parent);
    assert_eq!(homes.get(&2), Some(&vec![2, 7, 9]));
    assert_eq!(homes.get(&3), Some(&vec![3]));
    assert_eq!(homes.len(), 3);
}

fn plan(seed: u64, festival: bool) -> Plan<u8> {
    Plan {
        home: 1,
        work: Some((2, false)),
        about: 3,
        evening: Some((4, true)),
        gathering: 5,
        festival,
        seed,
    }
}

#[test]
fn everyone_is_home_at_night_and_at_the_gathering_on_a_festival() {
    let mut out = 0;
    for seed in 0..200 {
        let day = day(&plan(seed, false));
        assert!(day
            .windows(2)
            .all(|pair| pair[0].from_hour < pair[1].from_hour));
        for hour in [22, 23, 0, 3, 6] {
            let stop = stop_at(&day, hour).unwrap();
            assert_eq!((stop.at, stop.inside), (1, true), "{seed} at {hour}");
        }
        assert_eq!(stop_at(&day, 12).unwrap().at, 2);
        out += usize::from(stop_at(&day, 19).unwrap().at == 4);
        let festival = day_of(seed);
        assert_eq!(stop_at(&festival, 20).unwrap().at, 5);
        assert!(!stop_at(&festival, 22).unwrap().inside);
        assert_eq!(stop_at(&festival, 23).unwrap().at, 1);
    }
    // Some go out of an evening, most stay in.
    assert!((40..=120).contains(&out), "{out}");
}

fn day_of(seed: u64) -> Vec<Stop<u8>> {
    day(&plan(seed, true))
}

#[test]
fn a_grown_child_with_a_partner_and_a_one_sided_partner_still_share_a_home() {
    let people = [1_u64, 2, 5, 6, 8];
    // 5 is 1 and 2's child, grown up and with 6; 8 thinks it is with 6.
    let partner = |person| match person {
        1 => Some(2),
        2 => Some(1),
        5 => Some(6),
        6 => Some(5),
        8 => Some(6),
        _ => None,
    };
    let parent = |person| (person == 5).then_some(1);
    let homes = households(&people, partner, parent);
    assert_eq!(homes.get(&1), Some(&vec![1, 2]));
    assert_eq!(homes.get(&5), Some(&vec![5, 6, 8]));
}

/// However the coins of their own fall, at most a third of a town is home
/// as work ends at five; the rest stay out until six, and the same people
/// keep the early evening.
#[test]
fn no_town_empties_at_five() {
    use world_core::EntityId;
    use world_projection::{CanvasItem, CanvasItemKind, RoutineStop, SelectionId};
    let home = SelectionId::Entity(EntityId::new(1_000));
    let work = SelectionId::Entity(EntityId::new(1_001));
    let stop = |from_hour, at, inside| RoutineStop {
        from_hour,
        at,
        inside,
    };
    let person = |id| CanvasItem {
        id: SelectionId::Entity(EntityId::new(id)),
        kind: CanvasItemKind::Actor,
        home: Some(home),
        day: vec![
            stop(0, home, true),
            stop(7, work, false),
            stop(KNOCK_OFF, home, true),
        ],
        ..Default::default()
    };
    let mut items = (1..=15).rev().map(person).collect::<Vec<_>>();
    town::stagger_homecomings(&mut items);
    let home_at_five = items
        .iter()
        .filter(|item| item.day.last().unwrap().from_hour == KNOCK_OFF)
        .map(|item| item.id)
        .collect::<Vec<_>>();
    assert_eq!(
        home_at_five,
        [5, 4, 3, 2, 1].map(|id| SelectionId::Entity(EntityId::new(id)))
    );
    assert!(items.iter().all(|item| item
        .day
        .windows(2)
        .all(|pair| pair[0].from_hour < pair[1].from_hour)));
}
