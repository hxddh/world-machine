//! The player's part in a World and the long arc of its years, in the
//! Pack's words: what the player is there, the pages of this year's arc
//! (filled as the year reaches each, or still to come), each year closed
//! with its credits, and what the World works toward over many years,
//! each counted.
//!
//! Everything here is read from recorded Events and the state they made;
//! nothing is written back. What a page, a year or an aim is, is the
//! Pack's to say: this module only holds the shapes.

use crate::SelectionId;

/// The player's book of a World: its title page, this year's pages, the
/// years it has closed and what it works toward.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Keeping {
    /// The book's title, on its title page.
    pub title: String,
    /// Who the player is in the World, in a sentence under the title.
    pub role: String,
    /// The year now being lived, counted from 1.
    pub year: u32,
    /// This year's pages, in the order the year fills them.
    pub pages: Vec<Page>,
    /// Every year closed so far, oldest first, each with its credits.
    pub closed: Vec<YearClosed>,
    /// What the World works toward over the years, each counted.
    pub aims: Vec<Aim>,
}

impl Keeping {
    /// The year closed most recently, if it closed less than `within` ago
    /// at `now` (world time): its page is shown on the day it closed.
    pub fn just_closed(&self, now: u64, within: u64) -> Option<&YearClosed> {
        self.closed
            .last()
            .filter(|closed| closed.at <= now && closed.at.saturating_add(within) > now)
    }
}

/// One page of a year's arc: a season's beat.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Page {
    /// "Summer: the Midsummer Fair".
    pub title: String,
    /// How it went, once filled; what will fill it, while still to come.
    pub line: String,
    pub filled: bool,
    /// The recorded moment that filled it: a timeline item.
    pub moment: Option<SelectionId>,
}

/// A year as it closed: its title and a roll of credits.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct YearClosed {
    /// The year closed, counted from 1.
    pub year: u32,
    pub title: String,
    /// The world time it closed at.
    pub at: u64,
    /// Everyone the year names and what each did, the player last.
    pub credits: Vec<Credit>,
    /// The recorded moment it closed at.
    pub moment: Option<SelectionId>,
}

/// One line of a year's credits: a name and a deed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Credit {
    pub name: String,
    pub deed: String,
    /// Who it names, to draw them by.
    pub who: Option<SelectionId>,
}

/// Something the World works toward over the years: a project of several
/// steps, a book to complete, someone's story.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Aim {
    /// Which kind of aim: "Projects", "To complete", "Hearts".
    pub group: String,
    pub label: String,
    /// How many steps are done, of how many.
    pub done: u32,
    pub of: u32,
    /// The next step, in the World's words; empty once it is done.
    pub next: String,
    /// Whom it is about, to draw them by.
    pub who: Option<SelectionId>,
}

impl Aim {
    pub fn finished(&self) -> bool {
        self.done >= self.of
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_year_just_closed_is_shown_on_the_day_it_closed_only() {
        let keeping = Keeping {
            closed: vec![YearClosed {
                year: 1,
                at: 1_190,
                ..YearClosed::default()
            }],
            ..Keeping::default()
        };
        assert!(keeping.just_closed(1_185, 10).is_none());
        assert_eq!(keeping.just_closed(1_190, 10).map(|y| y.year), Some(1));
        assert!(keeping.just_closed(1_199, 10).is_some());
        assert!(keeping.just_closed(1_200, 10).is_none());
        assert!(Keeping::default().just_closed(0, 10).is_none());
        let aim = Aim {
            done: 3,
            of: 3,
            ..Aim::default()
        };
        assert!(aim.finished());
    }
}
