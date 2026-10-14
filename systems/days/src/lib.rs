//! The place as a street you walk along, and the days its people keep.
//!
//! Everything here is a pure function of who lives in a place and what
//! stands in it: where each home and each finished work stands, and where
//! each person is from one hour of the day to the next. None of it is
//! recorded, so a World saved before it existed gets the same street and
//! the same days as one begun today, and replaying a World never needs it.
//!
//! The System knows nothing of harbours or colonies. A Pack names its
//! stretches (the quay, the square), says which of its people share a home
//! and where each of them works, and asks for slots and days.

#![forbid(unsafe_code)]

pub mod festive;
pub mod town;

pub use town::Town;

use std::collections::{BTreeMap, BTreeSet};

/// A stretch of a place along its panorama: the quay, the square, the hill.
/// Positions are in screen-widths from the left edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stretch {
    pub id: &'static str,
    pub from: f32,
    pub to: f32,
}

impl Stretch {
    pub fn holds(&self, px: f32) -> bool {
        px >= self.from && px < self.to
    }

    pub fn width(&self) -> f32 {
        self.to - self.from
    }
}

/// A row of slots running the whole width of a place: one thing may stand
/// in each slot, `pitch` apart, the first `offset` from the left edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    pub pitch: f32,
    pub offset: f32,
}

/// A place's slots, and which are taken.
#[derive(Clone, Debug)]
pub struct Street {
    width: f32,
    stretches: Vec<Stretch>,
    rows: Vec<Row>,
    taken: BTreeSet<(usize, usize)>,
}

impl Street {
    pub fn new(width: f32, stretches: &[Stretch], rows: &[Row]) -> Self {
        Self {
            width,
            stretches: stretches.to_vec(),
            rows: rows.to_vec(),
            taken: BTreeSet::new(),
        }
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn stretches(&self) -> &[Stretch] {
        &self.stretches
    }

    /// The stretch a point lies in, by index.
    pub fn stretch_at(&self, px: f32) -> Option<usize> {
        self.stretches.iter().position(|stretch| stretch.holds(px))
    }

    /// The stretch with an id, by index.
    pub fn stretch(&self, id: &str) -> Option<usize> {
        self.stretches.iter().position(|stretch| stretch.id == id)
    }

    /// How many slots a row has.
    pub fn slots(&self, row: usize) -> usize {
        self.rows.get(row).map_or(0, |row| {
            if row.pitch <= 0.0 || row.offset >= self.width {
                0
            } else {
                ((self.width - row.offset) / row.pitch).floor() as usize
            }
        })
    }

    /// Where a slot stands.
    pub fn px(&self, row: usize, slot: usize) -> f32 {
        let row = self.rows[row];
        row.offset + (slot as f32 + 0.5) * row.pitch
    }

    /// How many slots are taken, in every row.
    pub fn taken(&self) -> usize {
        self.taken.len()
    }

    /// Takes the free slot of `row` nearest `near`, in the stretch `within`
    /// if it has one free, and otherwise the free slot nearest `near`
    /// anywhere. Nothing if the row is full.
    pub fn take(&mut self, row: usize, within: Option<usize>, near: f32) -> Option<f32> {
        self.take_where(row, within, near, |_| true)
    }

    /// [`Street::take`], among the slots whose place `ok` allows.
    pub fn take_where(
        &mut self,
        row: usize,
        within: Option<usize>,
        near: f32,
        ok: impl Fn(f32) -> bool,
    ) -> Option<f32> {
        let stretch = within.and_then(|at| self.stretches.get(at).copied());
        let free = |street: &Self, inside: bool| {
            (0..street.slots(row))
                .filter(|slot| !street.taken.contains(&(row, *slot)))
                .filter(|slot| ok(street.px(row, *slot)))
                .filter(|slot| !inside || stretch.is_some_and(|s| s.holds(street.px(row, *slot))))
                .min_by(|a, b| {
                    let da = (street.px(row, *a) - near).abs();
                    let db = (street.px(row, *b) - near).abs();
                    da.total_cmp(&db).then(a.cmp(b))
                })
        };
        let slot = free(self, true).or_else(|| free(self, false))?;
        self.taken.insert((row, slot));
        Some(self.px(row, slot))
    }

    /// Takes a slot in the first of `rows` with one free in the stretch
    /// `within`, and failing that the first with one free anywhere: the
    /// row taken, and where the slot stands.
    pub fn take_first(
        &mut self,
        rows: &[usize],
        within: Option<usize>,
        near: f32,
    ) -> Option<(usize, f32)> {
        if within.is_some() {
            for &row in rows {
                if self.free_in(row, within) {
                    return self.take(row, within, near).map(|px| (row, px));
                }
            }
        }
        rows.iter()
            .find_map(|&row| self.take(row, None, near).map(|px| (row, px)))
    }

    /// Keeps the free slots of `row` within `reach` of `px` clear: they are
    /// taken, so nothing is put in them after. How many were.
    pub fn keep_clear(&mut self, row: usize, px: f32, reach: f32) -> usize {
        let near = (0..self.slots(row))
            .filter(|slot| !self.taken.contains(&(row, *slot)))
            .filter(|slot| (self.px(row, *slot) - px).abs() < reach)
            .collect::<Vec<_>>();
        for slot in &near {
            self.taken.insert((row, *slot));
        }
        near.len()
    }

    /// Whether a row has a slot free in a stretch (or anywhere).
    pub fn free_in(&self, row: usize, within: Option<usize>) -> bool {
        let stretch = within.and_then(|at| self.stretches.get(at));
        (0..self.slots(row)).any(|slot| {
            !self.taken.contains(&(row, slot))
                && stretch.is_none_or(|stretch| stretch.holds(self.px(row, slot)))
        })
    }

    /// A point within a stretch, spread by `key` so that things asked for
    /// one after another do not all crowd its left edge.
    pub fn spread(&self, stretch: usize, key: u64) -> f32 {
        let Some(stretch) = self.stretches.get(stretch) else {
            return self.width / 2.0;
        };
        let fraction = (mix(&[key, 0x5ca1ab1e]) % 1000) as f32 / 1000.0;
        stretch.from + stretch.width() * (0.08 + 0.84 * fraction)
    }
}

/// A small, stable hash of a few numbers.
pub fn mix(parts: &[u64]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for part in parts {
        for byte in part.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash ^ (hash >> 29)
}

/// Who lives together: each household keyed by its first member (the
/// lowest key), couples sharing one and children living with a parent.
/// `partner` names whoever someone is with; `parent` names a parent of a
/// child. Anyone named who is not among `people` is left out.
pub fn households<K: Ord + Copy>(
    people: &[K],
    partner: impl Fn(K) -> Option<K>,
    parent: impl Fn(K) -> Option<K>,
) -> BTreeMap<K, Vec<K>> {
    let living = people.iter().copied().collect::<BTreeSet<_>>();
    // Who lives with whom, joined up: everyone with whoever they are with,
    // and a child who is with nobody with their parent. Each household is
    // known by its first member.
    let mut first: BTreeMap<K, K> = living.iter().map(|person| (*person, *person)).collect();
    fn root<K: Ord + Copy>(first: &BTreeMap<K, K>, mut person: K) -> K {
        while let Some(&up) = first.get(&person).filter(|up| **up != person) {
            person = up;
        }
        person
    }
    let mut join = |a: K, b: K| {
        let (a, b) = (root(&first, a), root(&first, b));
        let (low, high) = (a.min(b), a.max(b));
        first.insert(high, low);
    };
    for &person in &living {
        match partner(person).filter(|other| living.contains(other)) {
            Some(other) => join(person, other),
            None => {
                if let Some(parent) = parent(person).filter(|parent| living.contains(parent)) {
                    join(person, parent);
                }
            }
        }
    }
    let mut homes: BTreeMap<K, Vec<K>> = BTreeMap::new();
    for &person in &living {
        homes.entry(root(&first, person)).or_default().push(person);
    }
    homes
}

/// What someone is doing at a stop, as far as anyone looking on can tell.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Doing {
    /// At home, out of sight.
    #[default]
    Rest,
    /// At their work.
    Work,
    /// Keeping a stall or a counter.
    Keep,
    /// Sitting: on a bench, a wall, a step.
    Sit,
    /// Standing about with others: outside the pub, under a lamp.
    Gather,
    /// Carrying something from one place to another.
    Carry,
    /// Out and about with nothing in particular to do.
    Stroll,
    /// Dancing.
    Dance,
}

impl Doing {
    /// How someone doing this stands, as the scene draws them: `None`
    /// leaves it to the app (an idle moment, or nobody to see).
    pub fn stance(self) -> Option<world_projection::Stance> {
        use world_projection::Stance;
        match self {
            Doing::Rest | Doing::Stroll => None,
            Doing::Work => Some(Stance::Working),
            Doing::Keep | Doing::Gather => Some(Stance::Talking),
            Doing::Sit => Some(Stance::Sitting),
            Doing::Carry => Some(Stance::Carrying),
            Doing::Dance => Some(Stance::Dancing),
        }
    }
}

/// What a thing standing near someone is good for.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Use {
    /// Somewhere to sit: a bench, a wall.
    Seat,
    /// A light people gather under after dark.
    Light,
    /// A stall or a counter someone keeps.
    Stall,
    /// Something growing that wants tending.
    Planting,
}

/// Where someone is from one hour of the day on, what they are doing there
/// and with whom.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stop<K> {
    pub from_hour: u8,
    pub at: K,
    pub inside: bool,
    pub doing: Doing,
    /// Whom they are there with: someone whose day has the same stop from
    /// the same hour.
    pub with: Option<K>,
}

/// What a person's day is made of.
#[derive(Clone, Debug)]
pub struct Plan<K> {
    pub home: K,
    /// Where they spend the working day, and whether indoors there.
    pub work: Option<(K, bool)>,
    /// Where someone with no work goes by day.
    pub about: K,
    /// Where they might go of an evening, and whether indoors there.
    pub evening: Option<(K, bool)>,
    /// Where everyone gathers on a festival.
    pub gathering: K,
    pub festival: bool,
    /// Something of the person's own, so that no two keep quite the same
    /// hours.
    pub seed: u64,
    /// The day it is, in the World's own days: what someone does with an
    /// hour to spare changes from one day to the next.
    pub today: u64,
    /// The things near them they might use, nearest first.
    pub uses: Vec<(K, Use)>,
    /// What the record last says they did: where, doing what, and with
    /// whom. They do it again today, at its hour.
    pub last: Option<(K, Doing, Option<K>)>,
}

/// The earliest anyone knocks off work: five, or six for the rest.
pub const KNOCK_OFF: u8 = 17;

/// The hour everyone on a festival goes out to the gathering.
pub const FESTIVAL_FROM: u8 = 17;
/// The hour a festival ends and everyone goes home.
pub const FESTIVAL_UNTIL: u8 = 23;

/// The hour (or the one after) a load is carried over of a morning.
pub const CARRY_FROM: u8 = 10;
/// The hour (or the one after) someone sits down of an afternoon.
pub const SIT_FROM: u8 = 14;
/// The hour (or the one after) people gather under a light, or meet
/// someone of an evening.
pub const DUSK: u8 = 19;

/// A day being put together: each stop begins later than the one before,
/// and a stop begun at the same hour as the last replaces it.
struct Hours<K>(Vec<Stop<K>>);

impl<K: Copy> Hours<K> {
    fn push(&mut self, stop: Stop<K>) {
        match self.0.last() {
            Some(last) if last.from_hour == stop.from_hour => {
                self.0.pop();
                self.0.push(stop);
            }
            Some(last) if last.from_hour > stop.from_hour => {}
            _ => self.0.push(stop),
        }
    }
}

/// A person's day, sorted by hour: at home until seven or eight, at work
/// (or about the place) by day, out of an evening now and then, and home
/// again by nine or ten. On a festival everyone is at the gathering from
/// five until eleven, dancing.
///
/// What the record last says they did comes round again at its hour: a
/// load carried of a morning, a stall kept all day, a bench of an
/// afternoon, the pub or a dance of an evening. Someone they did it with
/// is there with them, from the same hour (that hour is the day's, not
/// either one's own). A bench near them gets sitters of an afternoon, and a
/// light gathers people after dark.
///
/// The day is a pure function of the plan: the same person on the same day
/// always keeps the same hours, and none of it is ever recorded.
pub fn day<K: Copy>(plan: &Plan<K>) -> Vec<Stop<K>> {
    let seed = mix(&[plan.seed, 0xda75]);
    // Today's coins: one of the person's own, and one every pair shares.
    let roll = mix(&[plan.seed, plan.today, 0x5e47]);
    let shared = mix(&[plan.today, 0x9a15]);
    let stop = |from_hour, at, inside, doing| Stop {
        from_hour,
        at,
        inside,
        doing,
        with: None,
    };
    let home = |from_hour| stop(from_hour, plan.home, true, Doing::Rest);
    let mut stops = Hours(vec![home(0)]);
    let rises = 7 + (seed % 2) as u8;
    let working = match plan.work {
        Some((work, inside)) => stop(rises, work, inside, Doing::Work),
        None => stop(rises + 2, plan.about, false, Doing::Stroll),
    };
    if plan.festival {
        stops.push(working);
        stops.push(stop(FESTIVAL_FROM, plan.gathering, false, Doing::Dance));
        stops.push(home(FESTIVAL_UNTIL));
        return stops.0;
    }

    // What the record last says, again today: at its place, out of doors,
    // with whoever they did it with, from an hour the two share.
    let last = plan.last.map(|(at, doing, with)| Stop {
        from_hour: 0,
        at,
        inside: false,
        doing,
        with,
    });
    let hour_of = |last: &Stop<K>, from: u8, own: u64| {
        from + if last.with.is_some() {
            shared % 2
        } else {
            own % 2
        } as u8
    };

    // The working day: a stall kept all day long; and for someone with no
    // work, a stall or a planting near them to see to, or the place to
    // wander.
    let working = match last.filter(|last| last.doing == Doing::Keep) {
        Some(keeps) => Stop {
            from_hour: rises,
            ..keeps
        },
        None if plan.work.is_none() => {
            let mut chores = vec![working];
            chores.extend(plan.uses.iter().filter_map(|(at, used)| match used {
                Use::Stall => Some(stop(rises + 2, *at, false, Doing::Keep)),
                Use::Planting => Some(stop(rises + 2, *at, false, Doing::Work)),
                _ => None,
            }));
            chores[((roll >> 3) % chores.len() as u64) as usize]
        }
        None => working,
    };
    let busy = working.doing != Doing::Stroll;
    stops.push(working);

    // A load carried over of a morning, and back to work after.
    if let Some(carry) = last.filter(|last| last.doing == Doing::Carry) {
        let from_hour = hour_of(&carry, CARRY_FROM, roll >> 5);
        stops.push(Stop { from_hour, ..carry });
        stops.push(Stop {
            from_hour: from_hour + 1,
            ..working
        });
    }

    // A sit of an afternoon: on the bench the record says, or the nearest
    // seat. Anyone without work to do sits a while; someone at work now
    // and then takes an hour's break.
    let seat = last.filter(|last| last.doing == Doing::Sit).or_else(|| {
        plan.uses
            .iter()
            .find(|(_, used)| *used == Use::Seat)
            .map(|(at, _)| stop(0, *at, false, Doing::Sit))
    });
    if let Some(seat) = seat {
        let sits = !busy || seat.with.is_some() || (roll >> 7) % 3 == 0;
        if sits {
            let from_hour = hour_of(&seat, SIT_FROM, roll >> 9);
            stops.push(Stop { from_hour, ..seat });
            stops.push(Stop {
                from_hour: from_hour + if busy { 1 } else { 2 },
                ..working
            });
        }
    }

    // The evening.
    let knocks_off = KNOCK_OFF + ((seed >> 3) % 2) as u8;
    let back = 21 + ((seed >> 11) % 2) as u8;
    let out =
        last.filter(|last| matches!(last.doing, Doing::Gather | Doing::Dance | Doing::Stroll));
    let light = plan
        .uses
        .iter()
        .find(|(_, used)| *used == Use::Light)
        .filter(|_| (roll >> 11) % 3 == 0)
        .map(|(at, _)| stop(0, *at, false, Doing::Gather));
    let goes_out = (seed >> 7) % 5 < 2;
    match (out, light) {
        // Out with someone: home first, and out together after dark until
        // an hour the two share.
        (Some(out), _) if out.with.is_some() => {
            stops.push(home(knocks_off));
            stops.push(Stop {
                from_hour: hour_of(&out, DUSK, 0),
                ..out
            });
            stops.push(home(21 + ((shared >> 3) % 2) as u8));
        }
        (Some(out), _) => {
            stops.push(Stop {
                from_hour: knocks_off + 1,
                ..out
            });
            stops.push(home(back));
        }
        (None, Some(light)) => {
            stops.push(home(knocks_off));
            stops.push(Stop {
                from_hour: DUSK + ((roll >> 13) % 2) as u8,
                ..light
            });
            stops.push(home(back.max(DUSK + 2)));
        }
        (None, None) => match plan.evening.filter(|_| goes_out) {
            Some((evening, inside)) => {
                stops.push(stop(knocks_off + 1, evening, inside, Doing::Gather));
                stops.push(home(back));
            }
            None => stops.push(home(knocks_off)),
        },
    }
    stops.0
}

/// The stop of a day that holds at `hour`: the last begun by then, or the
/// day's last carried over from the night before.
pub fn stop_at<K>(day: &[Stop<K>], hour: u8) -> Option<&Stop<K>> {
    day.iter()
        .rev()
        .find(|stop| stop.from_hour <= hour)
        .or_else(|| day.last())
}

#[cfg(test)]
mod tests;
