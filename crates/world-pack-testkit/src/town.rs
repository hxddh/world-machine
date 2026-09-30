//! Bars for a town laid out along its panorama: everyone has a home,
//! couples share one, every finished work stands in a spot of its own, the
//! place is out by day and home by night, and a festival gathers it.

use std::collections::BTreeMap;
use world_projection::{CanvasItemKind, ProjectionSnapshot, RoutineStop, SelectionId};

/// How a town stands, as its bars measure it.
#[derive(Debug)]
#[allow(dead_code)]
pub struct TownBars {
    pub people: usize,
    /// People drawn with no home on the scene.
    pub homeless: usize,
    /// Couples living apart.
    pub apart: usize,
    pub outside_at_22: usize,
    /// The largest share of people in one district at noon.
    pub busiest_at_noon: f64,
    pub at_noon: BTreeMap<String, usize>,
    /// The share of people in the district the town gathers in, at seven
    /// in the evening.
    pub gathered_in_evening: f64,
    pub festival: bool,
    /// Finished works, and those standing on the scene.
    pub finished: usize,
    pub standing: usize,
    /// Places and things standing in the same spot as another.
    pub shared_slots: usize,
}

/// What the Pack tells the bars about its town.
pub struct TownFacts<'a> {
    /// Whether a selection is one of the homes.
    pub is_home: &'a dyn Fn(SelectionId) -> bool,
    /// Someone's partner, if they have one.
    pub partner: &'a dyn Fn(SelectionId) -> Option<SelectionId>,
    /// The district the town gathers in of an evening.
    pub gathering: String,
    /// The labels of the works finished so far.
    pub finished: Vec<String>,
    pub festival: bool,
}

pub fn town_bars(snapshot: &ProjectionSnapshot, facts: &TownFacts<'_>) -> TownBars {
    let canvas = &snapshot.canvas;
    let people = canvas
        .items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Actor)
        .collect::<Vec<_>>();
    let homeless = people
        .iter()
        .filter(|person| {
            person
                .home
                .is_none_or(|home| !(facts.is_home)(home) || canvas.px_of(home).is_none())
        })
        .count();
    let apart = people
        .iter()
        .filter(|person| {
            (facts.partner)(person.id).is_some_and(|partner| {
                people
                    .iter()
                    .find(|other| other.id == partner)
                    .is_some_and(|other| other.home != person.home)
            })
        })
        .count();
    let district_of = |stop: &RoutineStop| {
        canvas
            .px_of(stop.at)
            .and_then(|px| canvas.district_at(px))
            .map(|district| district.id.clone())
            .unwrap_or_default()
    };
    let outside_at_22 = canvas
        .whereabouts(22)
        .iter()
        .filter(|(_, stop)| !stop.inside)
        .count();
    let noon = canvas.whereabouts(12);
    let mut at_noon: BTreeMap<String, usize> = BTreeMap::new();
    for (_, stop) in &noon {
        *at_noon.entry(district_of(stop)).or_default() += 1;
    }
    let busiest_at_noon =
        at_noon.values().copied().max().unwrap_or(0) as f64 / noon.len().max(1) as f64;
    let evening = canvas.whereabouts(19);
    let gathered_in_evening = evening
        .iter()
        .filter(|(_, stop)| district_of(stop) == facts.gathering)
        .count() as f64
        / evening.len().max(1) as f64;
    let standing = facts
        .finished
        .iter()
        .filter(|label| {
            canvas
                .items
                .iter()
                .any(|item| &item.label == *label && item.px.is_some())
        })
        .count();
    let mut spots = canvas
        .items
        .iter()
        .filter(|item| item.kind != CanvasItemKind::Actor)
        .filter_map(|item| item.px)
        .collect::<Vec<_>>();
    spots.sort_by(f32::total_cmp);
    let shared_slots = spots
        .windows(2)
        .filter(|pair| pair[1] - pair[0] < 0.01)
        .count();
    TownBars {
        people: people.len(),
        homeless,
        apart,
        outside_at_22,
        busiest_at_noon,
        at_noon,
        gathered_in_evening,
        festival: facts.festival,
        finished: facts.finished.len(),
        standing,
        shared_slots,
    }
}

/// Asserts a town's bars: noon spread over its stretches once it has at
/// least `spread_from` people (a handful cannot spread over three).
pub fn check_town_bars(label: &str, bars: &TownBars, spread_from: usize) {
    eprintln!("{label}: {bars:?}");
    assert_eq!(bars.homeless, 0, "{label}: {bars:?}");
    assert_eq!(bars.apart, 0, "{label}: {bars:?}");
    assert_eq!(bars.shared_slots, 0, "{label}: {bars:?}");
    assert_eq!(bars.standing, bars.finished, "{label}: {bars:?}");
    if bars.people >= spread_from {
        assert!(bars.busiest_at_noon <= 0.6, "{label}: {bars:?}");
    }
    if bars.festival {
        assert!(bars.gathered_in_evening > 0.5, "{label}: {bars:?}");
    } else {
        assert!(bars.outside_at_22 <= 3, "{label}: {bars:?}");
    }
}

/// Who is out of doors at an hour: the share of the people drawn on the
/// scene whose day has them outside then.
pub fn outdoors(snapshot: &ProjectionSnapshot, hour: u8) -> f32 {
    let people = snapshot
        .canvas
        .items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Actor)
        .collect::<Vec<_>>();
    let out = people
        .iter()
        .filter(|item| {
            let day = item
                .day
                .iter()
                .map(|stop| days::Stop {
                    from_hour: stop.from_hour,
                    at: stop.at,
                    inside: stop.inside,
                })
                .collect::<Vec<_>>();
            days::stop_at(&day, hour).is_some_and(|stop| !stop.inside)
        })
        .count();
    out as f32 / people.len().max(1) as f32
}
