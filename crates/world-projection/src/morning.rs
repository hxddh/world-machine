//! The morning a sitting opens on (v0.30 S4).

use crate::{Letter, Question, SelectionId};

/// The morning a sitting opens on, in the World's words. Presentation
/// only: each part is an Event in the World's record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Morning {
    /// The moment the morning came, by its Event.
    pub moment: SelectionId,
    /// A letter written to the player overnight.
    pub letter: Option<Letter>,
    /// The day's main question: one the commands put (by its id), which a
    /// screen shows first and weighted.
    pub question: Option<Question>,
    /// Something tended overnight.
    pub tended: Option<MorningLine>,
    /// Something that turned up for the book.
    pub find: Option<MorningLine>,
}

/// A line of the morning, and who or what it is about, if anyone on the
/// scene.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorningLine {
    pub line: String,
    pub about: Option<SelectionId>,
}
