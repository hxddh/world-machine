pub mod age;
pub mod art;
pub mod brush;
pub mod design;
pub mod diorama;
#[cfg(test)]
mod golden;
pub mod hand;
pub mod i18n;
pub mod ladder;
mod macos;
pub mod mark;
#[cfg(test)]
mod offscreen;
pub mod painter;
pub mod panels;
pub mod pointers;
pub mod postcard;
pub mod scene;
pub mod setting;
pub mod strip;
pub mod text_input;
pub mod ui;
pub mod works;

pub use macos::{
    is_beginning, scene_share, speech_pages, words_at_rest, Farewell, FarewellAction,
    ProjectionView, RESTING_WORD_LIMIT,
};
pub use world_i18n::{set_language, Language};
pub use world_projection::{
    Ears, ProjectionIntent, ProjectionSnapshot, SelectionId, StoryPage, StoryRequest, VoiceHearing,
};

static TEXT_SCALE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(100);

/// How large text is drawn, as a percentage from 100 to 200.
pub fn set_text_scale(percent: u32) {
    TEXT_SCALE.store(
        percent.clamp(100, 200),
        std::sync::atomic::Ordering::Relaxed,
    );
}

pub fn text_scale() -> f32 {
    TEXT_SCALE.load(std::sync::atomic::Ordering::Relaxed) as f32 / 100.0
}

/// The size one rem is drawn at: 16 points, scaled as the player asked.
pub fn rem_size() -> f32 {
    16.0 * text_scale()
}

use std::time::Duration;

/// Work that asks a language model what the player's words to someone
/// mean, run off the window's thread: who heard them (the model's response,
/// with its judge's verdict beside it when there was a judge), or nothing.
pub type Listening = Box<dyn FnOnce() -> Option<world_projection::Ears> + Send>;

/// How long the window lets a model think before the World's own ears
/// answer instead.
pub const LISTEN_DEADLINE: Duration = Duration::from_secs(12);

/// Runs `listening` on a thread of its own and waits at most `deadline`
/// for it: the response, or nothing if it took longer or had none. A model
/// that never answers is left behind; nothing waits on it.
pub fn listen_within(listening: Listening, deadline: Duration) -> Option<world_projection::Ears> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(listening());
    });
    receiver.recv_timeout(deadline).ok().flatten()
}

pub trait ProjectionController {
    fn snapshot(&self) -> ProjectionSnapshot;

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, String>;

    /// When the World voice is on, the work of asking the model what the
    /// player's words to someone mean, for the window to run off its own
    /// thread before it says them with the response. `None`, the default,
    /// says them at once.
    fn listen(&mut self, _to: SelectionId, _words: &str) -> Option<Listening> {
        None
    }

    /// A small sound for something that just happened on screen, if the
    /// app plays any. Presentation only.
    fn cue(&mut self, _cue: Cue) {}

    /// A story the World tells when asked: someone's legend, a moment the
    /// book keeps, a year's almanac. `None` when the World has none to
    /// tell, or cannot be asked. Read only: asking changes nothing.
    fn story(&mut self, _request: StoryRequest) -> Option<StoryPage> {
        None
    }
}

/// The small sounds a World window can ask for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cue {
    /// A card turned over, or the next one brought up.
    Flip,
    /// A turn was made and the World moved on.
    Turn,
    /// Something new was built.
    Built,
    /// The drawer was opened, by the handle, a click on a place, or ⌘I.
    Drawer,
    /// Someone handed the player a keepsake, shown for a moment before it
    /// goes into the drawer.
    Keepsake,
    /// Someone says a line: a babble in their own voice, a syllable or so
    /// for every few letters, rising at the end of a question.
    Babble {
        /// Stable per person, so each always sounds like themselves.
        voice: u32,
        syllables: u8,
        question: bool,
    },
}

/// The babble under a line `text` said by `who`.
pub fn babble(who: SelectionId, text: &str) -> Cue {
    let voice = who
        .stable_key()
        .bytes()
        .fold(0x811c_9dc5_u32, |hash, byte| {
            (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
        });
    // A syllable for each group of vowels in a word, and one for every
    // other character of a language written without spaces.
    let mut syllables = 0_usize;
    let mut in_vowel = false;
    for character in text.chars() {
        if character.is_ascii_alphabetic() {
            let vowel = "aeiouyAEIOUY".contains(character);
            if vowel && !in_vowel {
                syllables += 1;
            }
            in_vowel = vowel;
        } else {
            in_vowel = false;
            if !character.is_ascii() && character.is_alphanumeric() {
                syllables += 1;
            }
        }
    }
    let trimmed = text.trim_end_matches(|character: char| {
        character.is_whitespace() || matches!(character, '"' | '\'' | '”' | '’' | ')')
    });
    Cue::Babble {
        voice,
        syllables: syllables.div_ceil(2).clamp(2, 12) as u8,
        question: trimmed.ends_with('?') || trimmed.ends_with('？'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_shows_at_most_two_lines_at_a_time() {
        let long = "Here we are again. Last time: Evan took a week's work on the mainland. A week's carpentry on the mainland. Should I go?";
        let pages = speech_pages(long);
        assert!(pages.len() >= 2, "{pages:?}");
        for page in &pages {
            assert!(page.lines().count() <= 2, "{page:?}");
            for line in page.lines() {
                assert!(line.chars().count() <= 36, "{line:?}");
            }
        }
        assert_eq!(pages.join(" ").replace('\n', " "), long, "nothing is lost");
        assert_eq!(speech_pages("Morning!"), vec!["Morning!".to_string()]);
        // A language without spaces is cut by width: two columns a
        // character.
        let chinese = "今天港口的船都回来了，大家都很高兴，晚上我们在码头一起吃饭吧，你也来吗？";
        for page in speech_pages(chinese) {
            for line in page.lines() {
                assert!(line.chars().count() <= 18, "{line:?}");
            }
        }
    }

    /// At twice the text size, a line a third longer than English (as
    /// many translations are) still shows at most two lines at a time
    /// (or one whole sentence, too long for two), each narrow enough for
    /// the bubble.
    #[test]
    fn a_long_translation_at_double_size_still_pages_in_twos() {
        let english =
            "Here we are again. Last time: Evan took a week's work on the mainland. Should I go?";
        let longer = format!("{english} {}", &english[..english.len() / 3]);
        set_text_scale(200);
        let pages = speech_pages(&longer);
        set_text_scale(100);
        assert!(pages.len() >= 3, "{pages:?}");
        for page in &pages {
            // Two lines, or one sentence too long for two on a taller
            // page of its own, never cut in the middle.
            let one_sentence = !page
                .trim_end_matches(['.', '?', '!'])
                .contains(['.', '?', '!']);
            assert!(
                page.lines().count() <= 2 || one_sentence && page.lines().count() <= 4,
                "{page:?}"
            );
            for line in page.lines() {
                assert!(line.chars().count() <= 18, "{line:?} is too wide at 200%");
            }
        }
        assert_eq!(pages.join(" ").replace('\n', " "), longer);
    }

    #[test]
    fn a_line_babbles_by_its_length_and_rises_as_a_question() {
        let mara = SelectionId::from_stable_key("entity-4").unwrap();
        let leo = SelectionId::from_stable_key("entity-5").unwrap();
        let short = babble(mara, "Hi!");
        let long = babble(
            mara,
            "The boats came in late again, and the market will be quiet.",
        );
        let (
            Cue::Babble {
                voice,
                syllables: few,
                question: false,
            },
            Cue::Babble {
                voice: same,
                syllables: many,
                ..
            },
        ) = (short, long)
        else {
            panic!("{short:?} {long:?}");
        };
        assert_eq!(voice, same, "one person, one voice");
        assert!(many > few);
        assert!(matches!(
            babble(mara, "Are you staying for supper?"),
            Cue::Babble { question: true, .. }
        ));
        assert!(matches!(
            babble(mara, "你今天好吗？"),
            Cue::Babble {
                question: true,
                syllables: 3,
                ..
            }
        ));
        assert_ne!(babble(mara, "Hello there"), babble(leo, "Hello there"));
    }
    use std::time::Instant;

    #[test]
    fn a_model_that_takes_too_long_is_not_waited_for() {
        let started = Instant::now();
        use world_projection::Ears;
        let slow: Listening = Box::new(|| {
            std::thread::sleep(Duration::from_secs(5));
            Some(Ears::Model("too late".into()))
        });
        assert_eq!(listen_within(slow, Duration::from_millis(100)), None);
        assert!(started.elapsed() < Duration::from_secs(2));

        let quick: Listening = Box::new(|| Some(Ears::Model("MEANING: greet".into())));
        assert_eq!(
            listen_within(quick, Duration::from_secs(5)),
            Some(Ears::Model("MEANING: greet".into()))
        );
        let silent: Listening = Box::new(|| None);
        assert_eq!(listen_within(silent, Duration::from_secs(5)), None);
    }
}
