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
        today: 40,
        uses: Vec::new(),
        last: None,
    }
}

/// The day as it was before stops said what anyone was doing: what a plan
/// with nothing near to use and nothing done last must still give, hour
/// for hour.
fn plain_day(plan: &Plan<u8>) -> Vec<(u8, u8, bool)> {
    let seed = mix(&[plan.seed, 0xda75]);
    let mut stops = vec![(0, plan.home, true)];
    let rises = 7 + (seed % 2) as u8;
    match plan.work {
        Some((work, inside)) => stops.push((rises, work, inside)),
        None => stops.push((rises + 2, plan.about, false)),
    }
    if plan.festival {
        stops.push((FESTIVAL_FROM, plan.gathering, false));
        stops.push((FESTIVAL_UNTIL, plan.home, true));
        return stops;
    }
    let knocks_off = KNOCK_OFF + ((seed >> 3) % 2) as u8;
    match plan.evening.filter(|_| (seed >> 7) % 5 < 2) {
        Some((evening, inside)) => {
            stops.push((knocks_off + 1, evening, inside));
            stops.push((21 + ((seed >> 11) % 2) as u8, plan.home, true));
        }
        None => stops.push((knocks_off, plan.home, true)),
    }
    stops
}

fn hours(day: &[Stop<u8>]) -> Vec<(u8, u8, bool)> {
    day.iter()
        .map(|stop| (stop.from_hour, stop.at, stop.inside))
        .collect()
}

const SEAT: u8 = 6;
const LAMP: u8 = 7;
const STALL: u8 = 8;
const MARKET: u8 = 9;

#[test]
fn with_nothing_near_and_nothing_done_the_day_keeps_its_old_hours() {
    for seed in 0..300 {
        for festival in [false, true] {
            let mut idle = plan(seed, festival);
            idle.work = None;
            for plan in [plan(seed, festival), idle] {
                let day = super::day(&plan);
                assert_eq!(hours(&day), plain_day(&plan), "{seed}");
                for stop in &day {
                    let expected = if stop.at == plan.home {
                        Doing::Rest
                    } else if stop.at == 5 {
                        Doing::Dance
                    } else if stop.at == 4 {
                        Doing::Gather
                    } else if plan.work.is_some() {
                        Doing::Work
                    } else {
                        Doing::Stroll
                    };
                    assert_eq!(stop.doing, expected);
                    assert_eq!(stop.with, None);
                }
            }
        }
    }
}

#[test]
fn a_bench_near_gets_sitters_of_an_afternoon() {
    let mut workers_sitting = 0;
    for seed in 0..300 {
        let mut idle = plan(seed, false);
        idle.work = None;
        idle.uses = vec![(SEAT, Use::Seat)];
        let day = super::day(&idle);
        // Anyone with nothing to do sits a while every afternoon.
        let sat = day
            .iter()
            .find(|stop| stop.doing == Doing::Sit)
            .unwrap_or_else(|| panic!("{seed}: {day:?}"));
        assert_eq!((sat.at, sat.inside), (SEAT, false));
        assert!((SIT_FROM..=SIT_FROM + 1).contains(&sat.from_hour));
        assert_eq!(stop_at(&day, 15).unwrap().doing, Doing::Sit);
        assert_eq!(stop_at(&day, 12).unwrap().doing, Doing::Stroll);

        let mut worker = plan(seed, false);
        worker.uses = vec![(SEAT, Use::Seat)];
        let day = super::day(&worker);
        if let Some(sat) = day.iter().position(|stop| stop.doing == Doing::Sit) {
            workers_sitting += 1;
            // An hour's break, and back to work.
            assert_eq!(day[sat + 1].at, 2);
            assert_eq!(day[sat + 1].from_hour, day[sat].from_hour + 1);
        }
        assert_eq!(stop_at(&day, 10).unwrap().doing, Doing::Work);
    }
    // Now and then someone at work takes a break on it.
    assert!((60..=140).contains(&workers_sitting), "{workers_sitting}");
}

#[test]
fn a_light_gathers_people_after_dark_and_a_stall_gets_a_keeper() {
    let (mut gathered, mut kept) = (0, 0);
    for seed in 0..300 {
        let mut someone = plan(seed, false);
        someone.work = None;
        someone.uses = vec![(LAMP, Use::Light), (STALL, Use::Stall)];
        let day = super::day(&someone);
        if let Some(lamp) = day.iter().find(|stop| stop.at == LAMP) {
            gathered += 1;
            assert_eq!(lamp.doing, Doing::Gather);
            assert!(lamp.from_hour >= DUSK && !lamp.inside);
        }
        if let Some(stall) = day.iter().find(|stop| stop.at == STALL) {
            kept += 1;
            assert_eq!(stall.doing, Doing::Keep);
        }
        // Whoever keeps it as their trade keeps it all day.
        let mut keeper = plan(seed, false);
        keeper.last = Some((STALL, Doing::Keep, None));
        let day = super::day(&keeper);
        for hour in 9..17 {
            assert_eq!(stop_at(&day, hour).unwrap().at, STALL, "{seed} at {hour}");
        }
        assert_eq!(stop_at(&day, 23).unwrap().at, 1);
    }
    assert!((60..=140).contains(&gathered), "{gathered}");
    assert!((60..=200).contains(&kept), "{kept}");
}

#[test]
fn a_load_is_carried_over_of_a_morning_and_work_goes_on_after() {
    for seed in 0..200 {
        let mut fisher = plan(seed, false);
        fisher.last = Some((MARKET, Doing::Carry, None));
        let day = super::day(&fisher);
        let carry = day
            .iter()
            .position(|stop| stop.doing == Doing::Carry)
            .unwrap();
        assert_eq!((day[carry].at, day[carry].inside), (MARKET, false));
        assert!((CARRY_FROM..=CARRY_FROM + 1).contains(&day[carry].from_hour));
        assert_eq!(day[carry + 1].at, 2);
        assert_eq!(day[carry + 1].doing, Doing::Work);
    }
}

#[test]
fn two_who_went_out_together_go_out_together_again_from_the_same_hour() {
    for today in 0..60 {
        for (doing, place) in [
            (Doing::Gather, 4),
            (Doing::Dance, 5),
            (Doing::Sit, SEAT),
            (Doing::Carry, MARKET),
        ] {
            let mut ann = plan(11, false);
            let mut ben = plan(12 + today, false);
            ben.work = None;
            ann.today = today;
            ben.today = today;
            ann.last = Some((place, doing, Some(100)));
            ben.last = Some((place, doing, Some(101)));
            let (ann, ben) = (super::day(&ann), super::day(&ben));
            let ann_stop = ann.iter().find(|stop| stop.with == Some(100)).unwrap();
            let ben_stop = ben.iter().find(|stop| stop.with == Some(101)).unwrap();
            assert_eq!(ann_stop.doing, doing);
            assert_eq!(
                (ann_stop.from_hour, ann_stop.at),
                (ben_stop.from_hour, ben_stop.at),
                "{doing:?} on day {today}"
            );
            for day in [&ann, &ben] {
                assert!(day
                    .windows(2)
                    .all(|pair| pair[0].from_hour < pair[1].from_hour));
                assert_eq!(stop_at(day, 23).unwrap().at, 1);
            }
        }
    }
}

#[test]
fn a_festival_is_the_same_whatever_is_near_or_was_done() {
    for seed in 0..200 {
        let mut busy = plan(seed, true);
        busy.uses = vec![(SEAT, Use::Seat), (LAMP, Use::Light), (STALL, Use::Stall)];
        busy.last = Some((MARKET, Doing::Carry, Some(3)));
        let festival = super::day(&busy);
        assert_eq!(hours(&festival), plain_day(&plan(seed, true)));
        assert_eq!(stop_at(&festival, 20).unwrap().doing, Doing::Dance);
        assert!(festival.iter().all(|stop| stop.with.is_none()));
    }
}

#[test]
fn the_same_person_on_the_same_day_keeps_the_same_hours() {
    let mut someone = plan(77, false);
    someone.work = None;
    someone.uses = vec![(SEAT, Use::Seat), (LAMP, Use::Light), (STALL, Use::Stall)];
    let mut days = BTreeSet::new();
    for today in 0..30 {
        someone.today = today;
        let first = super::day(&someone);
        assert_eq!(first, super::day(&someone.clone()));
        days.insert(hours(&first));
    }
    // ...and not quite the same hours every day.
    assert!(days.len() > 2, "{}", days.len());
}

#[test]
fn everyone_is_home_at_night_and_at_the_gathering_on_a_festival() {
    let mut out = 0;
    for seed in 0..200 {
        let day = super::day(&plan(seed, false));
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
    super::day(&plan(seed, true))
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
        stance: None,
        with: None,
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

/// A town's people carry what they are doing onto the scene: how they
/// stand at each stop out of doors, and whom with while the other is
/// there too.
#[test]
fn the_scene_learns_what_everyone_is_doing_and_with_whom() {
    use world_core::EntityId;
    use world_projection::{CanvasItem, CanvasItemKind, SelectionId, Stance};
    let id = |n| SelectionId::Entity(EntityId::new(n));
    let (home, pub_, bench, gone) = (id(1_000), id(1_001), id(1_002), id(1_003));
    let mut town = town::Town::new(3.0, &STRETCHES, town::PLOT_ROW_AT);
    for (at, px) in [(home, 0.5), (pub_, 1.5), (bench, 2.5)] {
        town.placed.insert(at, px);
    }
    let plan_of = |seed, last| Plan {
        home,
        work: None,
        about: pub_,
        evening: None,
        gathering: pub_,
        festival: false,
        seed,
        today: 3,
        uses: vec![(bench, Use::Seat), (gone, Use::Light)],
        last,
    };
    let person = |n| CanvasItem {
        id: id(n),
        kind: CanvasItemKind::Actor,
        ..Default::default()
    };
    let mut items = vec![person(1), person(2), person(3)];
    let lasts = [
        Some((pub_, Doing::Gather, Some(id(2)))),
        Some((pub_, Doing::Gather, Some(id(1)))),
        // With someone who is not here at all.
        Some((pub_, Doing::Gather, Some(id(9)))),
    ];
    for (item, (seed, last)) in items
        .iter_mut()
        .zip([(1, lasts[0]), (2, lasts[1]), (3, lasts[2])])
    {
        town.live(item, home, &plan_of(seed, last));
    }
    town::keep_company(&mut items);
    let out = |item: &CanvasItem| {
        *item
            .day
            .iter()
            .find(|stop| {
                stop.at == pub_ && stop.stance == Some(Stance::Talking) && stop.from_hour >= DUSK
            })
            .unwrap()
    };
    let (ann, ben, cal) = (out(&items[0]), out(&items[1]), out(&items[2]));
    assert_eq!(
        (ann.with, ben.with, cal.with),
        (Some(id(2)), Some(id(1)), None)
    );
    assert_eq!(ann.from_hour, ben.from_hour);
    for item in &items {
        // A bench on the scene gets them of an afternoon; a light that
        // found no room never does.
        assert!(item
            .day
            .iter()
            .any(|stop| stop.at == bench && stop.stance == Some(Stance::Sitting)));
        assert!(item.day.iter().all(|stop| stop.at != gone));
        // Nobody indoors is drawn doing anything.
        assert!(item
            .day
            .iter()
            .all(|stop| !stop.inside || stop.stance.is_none()));
    }
}

/// By day the town gathers (v0.30 S1c): at each hour from nine to four,
/// those out and about elsewhere come to one or two hubs near each other,
/// talking two by two, enough to fill a window at rest; keepers stay at
/// their stalls; nobody indoors comes out; no stretch holds more than its
/// share; a festival is left alone; and the same town gathers the same.
#[test]
fn the_town_gathers_at_hubs_by_day() {
    use world_core::EntityId;
    use world_projection::{
        CanvasItem, CanvasItemKind, MarkShape, RoutineStop, SelectionId, Stance,
    };
    let id = |n| SelectionId::Entity(EntityId::new(n));
    let place = |n: u64, px: f32, shape: MarkShape| CanvasItem {
        id: id(n),
        kind: CanvasItemKind::Place,
        px: Some(px),
        shape: Some(shape),
        ..Default::default()
    };
    let stop = |from_hour, at, inside, stance| RoutineStop {
        from_hour,
        at,
        inside,
        stance,
        with: None,
    };
    let mut items = vec![
        place(100, 0.3, MarkShape::House),
        place(101, 0.95, MarkShape::Shop),
        place(102, 1.1, MarkShape::Stall),
        place(103, 1.9, MarkShape::Shop),
        place(104, 2.6, MarkShape::House),
    ];
    // Ten people: two keep the stall, one works indoors, the rest are out
    // and about all over the place.
    for n in 1..=10_u64 {
        let (at, inside, stance) = match n {
            1 | 2 => (id(102), false, Some(Stance::Talking)),
            3 => (id(103), true, None),
            n => (id(100 + n % 5), false, Some(Stance::Working)),
        };
        items.push(CanvasItem {
            id: id(n),
            kind: CanvasItemKind::Actor,
            day: vec![
                stop(0, id(100), true, None),
                stop(8, at, inside, stance),
                stop(18, id(104), true, None),
            ],
            ..Default::default()
        });
    }
    let before = items.clone();
    let spans = STRETCHES.map(|stretch| (stretch.from, stretch.to));
    town::gather_at_hubs(&mut items, &spans, None);
    let mut again = before.clone();
    town::gather_at_hubs(&mut again, &spans, None);
    assert_eq!(items, again, "the same town gathers the same");
    let px = |at: SelectionId| {
        items
            .iter()
            .find(|item| item.id == at)
            .and_then(|item| item.px)
            .unwrap()
    };
    let people = || {
        items
            .iter()
            .filter(|item| item.kind == CanvasItemKind::Actor)
    };
    for hour in 9..=16 {
        let now = people()
            .map(|item| {
                (
                    item.id,
                    *world_projection::stop_at(&item.day, hour).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let out = now
            .iter()
            .filter(|(_, stop)| !stop.inside)
            .collect::<Vec<_>>();
        // Around the busiest spot: the stall and its keepers.
        let hub = px(id(102));
        let near = out
            .iter()
            .filter(|(_, stop)| (px(stop.at) - hub).abs() <= town::HUB_APART + town::HUB_REACH)
            .count();
        assert!(near >= 6, "{hour}:00: {near} near the hub");
        assert!(near <= town::HUB_MOST + 2, "{hour}:00: {near}, a crowd");
        // Keepers keep; indoors stays in.
        for (who, stop) in &now {
            if *who == id(1) || *who == id(2) {
                assert_eq!(stop.at, id(102));
            }
            if *who == id(3) {
                assert!(stop.inside);
            }
        }
        // Those who came talk, two by two.
        let came = now
            .iter()
            .filter(|(who, stop)| {
                let was = before
                    .iter()
                    .find(|item| item.id == *who)
                    .and_then(|item| world_projection::stop_at(&item.day, hour))
                    .unwrap();
                stop.at != was.at
            })
            .collect::<Vec<_>>();
        assert!(!came.is_empty(), "{hour}:00: nobody came");
        assert!(came
            .iter()
            .all(|(_, stop)| stop.stance == Some(Stance::Talking) && !stop.inside));
        // (At most one alone at each of the two hubs.)
        assert!(came.iter().filter(|(_, stop)| stop.with.is_some()).count() + 2 >= came.len());
        // No stretch holds more than its share.
        for stretch in &STRETCHES {
            let in_it = now
                .iter()
                .filter(|(_, stop)| stretch.holds(px(stop.at)))
                .count();
            assert!(in_it <= 6, "{hour}:00: {in_it} in {}", stretch.id);
        }
    }
    // Outside the hours, as it was.
    for hour in [7, 8, 17, 18, 22] {
        for (a, b) in people().zip(
            before
                .iter()
                .filter(|item| item.kind == CanvasItemKind::Actor),
        ) {
            assert_eq!(
                world_projection::stop_at(&a.day, hour).map(|stop| (stop.at, stop.inside)),
                world_projection::stop_at(&b.day, hour).map(|stop| (stop.at, stop.inside))
            );
        }
    }
    // A festival is left as it is.
    let mut festival = before.clone();
    for item in festival
        .iter_mut()
        .filter(|item| item.kind == CanvasItemKind::Actor)
    {
        item.day = vec![
            stop(0, id(100), true, None),
            stop(9, id(102), false, Some(Stance::Dancing)),
        ];
    }
    let kept = festival.clone();
    town::gather_at_hubs(&mut festival, &spans, None);
    assert_eq!(festival, kept);
    // Round whoever speaks today, when they are out: the gathering is
    // where they are.
    let mut round = before.clone();
    town::gather_at_hubs(&mut round, &spans, Some(id(9)));
    for hour in 9..=16 {
        let theirs = round
            .iter()
            .find(|item| item.id == id(9))
            .and_then(|item| world_projection::stop_at(&item.day, hour))
            .copied()
            .unwrap();
        // They come to the gathering, which is round them.
        assert_eq!(theirs.stance, Some(Stance::Talking), "{hour}:00");
        let near = round
            .iter()
            .filter(|item| item.kind == CanvasItemKind::Actor)
            .filter_map(|item| world_projection::stop_at(&item.day, hour))
            .filter(|stop| !stop.inside && (px(stop.at) - px(theirs.at)).abs() <= 0.3)
            .count();
        assert!(near >= 6, "{hour}:00: {near} round the speaker");
    }
}
