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
        let stretch = within.and_then(|at| self.stretches.get(at).copied());
        let free = |street: &Self, inside: bool| {
            (0..street.slots(row))
                .filter(|slot| !street.taken.contains(&(row, *slot)))
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

/// Where someone is from one hour of the day on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stop<K> {
    pub from_hour: u8,
    pub at: K,
    pub inside: bool,
}

/// What a person's day is made of.
#[derive(Clone, Copy, Debug)]
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
}

/// The hour everyone on a festival goes out to the gathering.
pub const FESTIVAL_FROM: u8 = 17;
/// The hour a festival ends and everyone goes home.
pub const FESTIVAL_UNTIL: u8 = 23;

/// A person's day, sorted by hour: at home until seven or eight, at work
/// (or about the place) by day, out of an evening now and then, and home
/// again by nine or ten. On a festival everyone is at the gathering from
/// five until eleven.
pub fn day<K: Copy>(plan: &Plan<K>) -> Vec<Stop<K>> {
    let seed = mix(&[plan.seed, 0xda75]);
    let home = |from_hour| Stop {
        from_hour,
        at: plan.home,
        inside: true,
    };
    let mut stops = vec![home(0)];
    let rises = 7 + (seed % 2) as u8;
    match plan.work {
        Some((work, inside)) => stops.push(Stop {
            from_hour: rises,
            at: work,
            inside,
        }),
        None => stops.push(Stop {
            from_hour: rises + 2,
            at: plan.about,
            inside: false,
        }),
    }
    if plan.festival {
        stops.push(Stop {
            from_hour: FESTIVAL_FROM,
            at: plan.gathering,
            inside: false,
        });
        stops.push(home(FESTIVAL_UNTIL));
        return stops;
    }
    let knocks_off = 17 + ((seed >> 3) % 2) as u8;
    let goes_out = (seed >> 7) % 5 < 2;
    match plan.evening.filter(|_| goes_out) {
        Some((evening, inside)) => {
            stops.push(Stop {
                from_hour: knocks_off + 1,
                at: evening,
                inside,
            });
            stops.push(home(21 + ((seed >> 11) % 2) as u8));
        }
        None => stops.push(home(knocks_off)),
    }
    stops
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
