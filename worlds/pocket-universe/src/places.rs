//! Each place's own rhythm: the days its years turn on, and which of its
//! own years it is.
//!
//! Ares lives by its supply windows, one every 78 sols, and counts a year
//! as two of them. Maple Street lives by the school year, which starts in
//! the autumn, and by New Year's Eve, when 1987 becomes 1988. Icebridge
//! lives by the ice: the thaw in spring and the freeze-up in autumn, and
//! counts its years from thaw to thaw. None of it is a calendar the others
//! share, so no two places turn on the same days.

use world_core::{Value, WorldState};

/// The three places a Pocket Universe can begin as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Place {
    Ares,
    Maple,
    Ice,
}

impl Place {
    pub(crate) fn of(state: &WorldState) -> Option<Place> {
        match state
            .entity(crate::UNIVERSE)
            .and_then(|universe| universe.component(crate::SEED))
        {
            Some(Value::Text(seed)) => match seed.as_str() {
                "mars-colony" => Some(Place::Ares),
                "1980s-town" => Some(Place::Maple),
                "penguin-civilization" => Some(Place::Ice),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

/// Which period of the World it is.
pub(crate) fn period(state: &WorldState) -> u64 {
    state.world_time() / crate::BACKGROUND_PERIOD
}

/// A day on which a place's year turns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Turn {
    /// Ares: a supply window opens and the shuttle comes.
    Window,
    /// Maple Street: the school year starts.
    School,
    /// Maple Street: New Year's Eve.
    NewYear,
    /// Icebridge: the ice breaks up.
    Thaw,
    /// Icebridge: the sea freezes over.
    Freeze,
}

impl Turn {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Turn::Window => "window",
            Turn::School => "school",
            Turn::NewYear => "new_year",
            Turn::Thaw => "thaw",
            Turn::Freeze => "freeze",
        }
    }

    pub(crate) fn from_id(id: &str) -> Option<Turn> {
        Some(match id {
            "window" => Turn::Window,
            "school" => Turn::School,
            "new_year" => Turn::NewYear,
            "thaw" => Turn::Thaw,
            "freeze" => Turn::Freeze,
            _ => return None,
        })
    }

    /// The first period it comes on, and how many periods until it comes
    /// again.
    fn schedule(self) -> (u64, u64) {
        match self {
            Turn::Window => (50, 78),
            Turn::School => (62, 120),
            Turn::NewYear => (100, 120),
            Turn::Thaw => (25, 120),
            Turn::Freeze => (85, 120),
        }
    }

    /// How many times it has come by `period`.
    pub(crate) fn count(self, period: u64) -> u64 {
        let (first, every) = self.schedule();
        if period < first {
            0
        } else {
            (period - first) / every + 1
        }
    }

    /// The period its `n`th coming (from 0) falls on.
    pub(crate) fn day(self, n: u64) -> u64 {
        let (first, every) = self.schedule();
        first + n * every
    }
}

/// The turns of a place's year.
pub(crate) fn turns(place: Place) -> &'static [Turn] {
    match place {
        Place::Ares => &[Turn::Window],
        Place::Maple => &[Turn::School, Turn::NewYear],
        Place::Ice => &[Turn::Thaw, Turn::Freeze],
    }
}

/// The turn that begins each of the place's own years.
pub(crate) fn year_turn(place: Place) -> Turn {
    match place {
        Place::Ares => Turn::Window,
        Place::Maple => Turn::School,
        Place::Ice => Turn::Thaw,
    }
}

/// Which of the place's own years it is at `period`, counting the first as
/// 0: on Ares a year is two supply windows, on Maple Street a school
/// year, on Icebridge a year from thaw to thaw.
pub(crate) fn era_at(place: Place, period: u64) -> u64 {
    let turned = year_turn(place).count(period);
    match place {
        Place::Ares => turned / 2,
        _ => turned,
    }
}

/// The period the place's current own year began on, if it is past its
/// first.
pub(crate) fn era_began(place: Place, period: u64) -> Option<u64> {
    let era = era_at(place, period);
    if era == 0 {
        return None;
    }
    let turn = year_turn(place);
    (0..turn.count(period))
        .map(|n| turn.day(n))
        .find(|day| era_at(place, *day) == era)
}

/// Which of its own years the place is in now.
pub(crate) fn era(state: &WorldState) -> u64 {
    Place::of(state).map_or(0, |place| era_at(place, period(state)))
}

/// Every turn that has come by `period`, in the order they came: the
/// turn, which coming of it this is (from 0), and the period it came on.
pub(crate) fn turned_by(place: Place, period: u64) -> Vec<(Turn, u64, u64)> {
    let mut all = turns(place)
        .iter()
        .flat_map(|turn| (0..turn.count(period)).map(move |n| (*turn, n, turn.day(n))))
        .collect::<Vec<_>>();
    all.sort_by_key(|(turn, n, day)| (*day, *turn, *n));
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No two places turn their years on the same days, and every place
    /// keeps turning for as long as it is kept.
    #[test]
    fn every_place_turns_on_days_of_its_own() {
        let days = |place| {
            turned_by(place, 1_080)
                .into_iter()
                .map(|(_, _, day)| day)
                .collect::<Vec<_>>()
        };
        let ares = days(Place::Ares);
        let maple = days(Place::Maple);
        let ice = days(Place::Ice);
        assert_ne!(ares, maple);
        assert_ne!(maple, ice);
        assert_ne!(ares, ice);
        for (place, days) in [("Ares", &ares), ("Maple", &maple), ("Ice", &ice)] {
            assert!(days.len() >= 12, "{place}: {days:?}");
            assert!(
                days.iter().any(|day| *day > 1_000),
                "{place} stops turning: {days:?}"
            );
        }
        // Six of each place's own years at least, in three of the player's.
        for place in [Place::Ares, Place::Maple, Place::Ice] {
            assert!(era_at(place, 1_080) >= 6, "{place:?}");
        }
    }
}
