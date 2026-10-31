//! The morning a sitting opens on, and the player's book (P3b: their
//! part in the World, this year's pages, the years closed with their
//! credits, and what the World works toward), as they cross the boundary.
//! Every field past the few that name a thing is optional both ways, and
//! what an app cannot show is left out rather than refused.

use crate::{LetterWire, QuestionWire, SelectionIdWire};
use serde::{Deserialize, Serialize};

/// The most pages, closed years, credits or aims one book carries.
pub const MOST_BOOK_ITEMS: usize = 200;

/// The morning a sitting opens on, as it crosses the boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MorningWire {
    pub moment: SelectionIdWire,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub letter: Option<LetterWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<QuestionWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tended: Option<MorningLineWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub find: Option<MorningLineWire>,
}

/// A line of the morning, as it crosses the boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MorningLineWire {
    pub line: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<SelectionIdWire>,
}

impl From<&world_projection::Morning> for MorningWire {
    fn from(morning: &world_projection::Morning) -> Self {
        let line = |line: &world_projection::MorningLine| MorningLineWire {
            line: line.line.clone(),
            about: line.about.map(Into::into),
        };
        Self {
            moment: morning.moment.into(),
            letter: morning.letter.as_ref().map(|letter| LetterWire {
                from: letter.from.into(),
                note: letter.note.clone(),
                moment: letter.moment.into(),
            }),
            question: morning.question.as_ref().map(|question| QuestionWire {
                id: question.id.clone(),
                prompt: question.prompt.clone(),
            }),
            tended: morning.tended.as_ref().map(line),
            find: morning.find.as_ref().map(line),
        }
    }
}

impl From<MorningWire> for world_projection::Morning {
    /// Every part with no words to show is left out.
    fn from(morning: MorningWire) -> Self {
        let line = |line: MorningLineWire| {
            (!line.line.trim().is_empty()).then(|| world_projection::MorningLine {
                line: line.line,
                about: line.about.map(Into::into),
            })
        };
        Self {
            moment: morning.moment.into(),
            letter: morning
                .letter
                .filter(|letter| !letter.note.trim().is_empty())
                .map(|letter| world_projection::Letter {
                    from: letter.from.into(),
                    note: letter.note,
                    moment: letter.moment.into(),
                }),
            question: morning
                .question
                .filter(|question| {
                    !question.id.trim().is_empty() && !question.prompt.trim().is_empty()
                })
                .map(|question| world_projection::Question {
                    id: question.id,
                    prompt: question.prompt,
                }),
            tended: morning.tended.and_then(line),
            find: morning.find.and_then(line),
        }
    }
}

/// The player's book, as it crosses the boundary.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KeepingWire {
    pub title: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub year: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<PageWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub closed: Vec<YearClosedWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aims: Vec<AimWire>,
}

/// One page of a year's arc, as it crosses the boundary.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PageWire {
    pub title: String,
    #[serde(default)]
    pub line: String,
    #[serde(default)]
    pub filled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moment: Option<SelectionIdWire>,
}

/// A year closed, with its credits, as it crosses the boundary.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct YearClosedWire {
    pub year: u32,
    pub title: String,
    #[serde(default)]
    pub at: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credits: Vec<CreditWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moment: Option<SelectionIdWire>,
}

/// One line of a year's credits, as it crosses the boundary.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CreditWire {
    pub name: String,
    #[serde(default)]
    pub deed: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub who: Option<SelectionIdWire>,
}

/// Something the World works toward, as it crosses the boundary.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AimWire {
    #[serde(default)]
    pub group: String,
    pub label: String,
    #[serde(default)]
    pub done: u32,
    #[serde(default)]
    pub of: u32,
    #[serde(default)]
    pub next: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub who: Option<SelectionIdWire>,
}

impl From<&world_projection::Keeping> for KeepingWire {
    fn from(keeping: &world_projection::Keeping) -> Self {
        Self {
            title: keeping.title.clone(),
            role: keeping.role.clone(),
            year: keeping.year,
            pages: keeping
                .pages
                .iter()
                .take(MOST_BOOK_ITEMS)
                .map(|page| PageWire {
                    title: page.title.clone(),
                    line: page.line.clone(),
                    filled: page.filled,
                    moment: page.moment.map(Into::into),
                })
                .collect(),
            // The latest years, if there are more than a book carries.
            closed: keeping.closed[keeping.closed.len().saturating_sub(MOST_BOOK_ITEMS)..]
                .iter()
                .map(|closed| YearClosedWire {
                    year: closed.year,
                    title: closed.title.clone(),
                    at: closed.at,
                    credits: closed
                        .credits
                        .iter()
                        .take(MOST_BOOK_ITEMS)
                        .map(|credit| CreditWire {
                            name: credit.name.clone(),
                            deed: credit.deed.clone(),
                            who: credit.who.map(Into::into),
                        })
                        .collect(),
                    moment: closed.moment.map(Into::into),
                })
                .collect(),
            aims: keeping
                .aims
                .iter()
                .take(MOST_BOOK_ITEMS)
                .map(|aim| AimWire {
                    group: aim.group.clone(),
                    label: aim.label.clone(),
                    done: aim.done,
                    of: aim.of,
                    next: aim.next.clone(),
                    who: aim.who.map(Into::into),
                })
                .collect(),
        }
    }
}

impl From<KeepingWire> for world_projection::Keeping {
    /// Anything with no words to show is left out, and nothing is counted
    /// as more done than it takes.
    fn from(keeping: KeepingWire) -> Self {
        let worded = |text: &str| !text.trim().is_empty();
        Self {
            title: keeping.title,
            role: keeping.role,
            year: keeping.year,
            pages: keeping
                .pages
                .into_iter()
                .filter(|page| worded(&page.title))
                .take(MOST_BOOK_ITEMS)
                .map(|page| world_projection::Page {
                    title: page.title,
                    line: page.line,
                    filled: page.filled,
                    moment: page.moment.map(Into::into),
                })
                .collect(),
            closed: keeping
                .closed
                .into_iter()
                .filter(|closed| worded(&closed.title))
                .take(MOST_BOOK_ITEMS)
                .map(|closed| world_projection::YearClosed {
                    year: closed.year,
                    title: closed.title,
                    at: closed.at,
                    credits: closed
                        .credits
                        .into_iter()
                        .filter(|credit| worded(&credit.name))
                        .take(MOST_BOOK_ITEMS)
                        .map(|credit| world_projection::Credit {
                            name: credit.name,
                            deed: credit.deed,
                            who: credit.who.map(Into::into),
                        })
                        .collect(),
                    moment: closed.moment.map(Into::into),
                })
                .collect(),
            aims: keeping
                .aims
                .into_iter()
                .filter(|aim| worded(&aim.label) && aim.of > 0)
                .take(MOST_BOOK_ITEMS)
                .map(|aim| world_projection::Aim {
                    group: aim.group,
                    label: aim.label,
                    done: aim.done.min(aim.of),
                    of: aim.of,
                    next: aim.next,
                    who: aim.who.map(Into::into),
                })
                .collect(),
        }
    }
}
