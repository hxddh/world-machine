//! The Settings window as a plan: its sections, the controls in each, how
//! the keyboard walks and changes them, and the keyboard shortcuts it
//! lists. Pure presentation; `settings.rs` draws it and saves each change
//! through `app_settings`.

use crate::ambience::Channel;
use crate::shell::Step;

/// A section of Settings, in the order they are shown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Section {
    Sound,
    Display,
    Performance,
    Playing,
    Controls,
    Reports,
    Voice,
    Makers,
}

impl Section {
    pub const ALL: [Section; 8] = [
        Section::Sound,
        Section::Display,
        Section::Performance,
        Section::Playing,
        Section::Controls,
        Section::Reports,
        Section::Voice,
        Section::Makers,
    ];

    /// The section's heading, in English (the window translates it).
    pub fn title(self) -> &'static str {
        match self {
            Section::Sound => "Sound",
            Section::Display => "Display",
            Section::Performance => "Performance",
            Section::Playing => "Playing",
            Section::Controls => "Controls",
            Section::Reports => "Crash reports and support",
            Section::Voice => "AI voice (experimental)",
            Section::Makers => "For World makers",
        }
    }
}

/// A sound level the player sets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Level {
    Master,
    Channel(Channel),
}

impl Level {
    pub const ALL: [Level; 5] = [
        Level::Master,
        Level::Channel(Channel::Music),
        Level::Channel(Channel::Interface),
        Level::Channel(Channel::Ambience),
        Level::Channel(Channel::Voices),
    ];

    /// Its name and what it is, in English.
    pub fn words(self) -> (&'static str, &'static str) {
        match self {
            Level::Master => ("Master volume", "Every sound, together"),
            Level::Channel(Channel::Music) => (
                "Music",
                "Chords, a morning tune and an evening one, by the hour and the weather",
            ),
            Level::Channel(Channel::Interface) => (
                "Effects",
                "A note in the World's key for every card, letter and thing placed",
            ),
            Level::Channel(Channel::Ambience) => (
                "Ambient sound",
                "The sea, the wind, rain and the life of the place",
            ),
            Level::Channel(Channel::Voices) => ("Voices", "Everyone's own babble as they speak"),
        }
    }

    /// The element id its control is given.
    pub fn id(self) -> &'static str {
        match self {
            Level::Master => "level-master",
            Level::Channel(Channel::Music) => "level-music",
            Level::Channel(Channel::Interface) => "level-effects",
            Level::Channel(Channel::Ambience) => "level-ambient",
            Level::Channel(Channel::Voices) => "level-voices",
        }
    }
}

/// One thing in Settings the keyboard can reach.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Control {
    Sound,
    Level(Level),
    Language,
    TextSize,
    Contrast,
    ReduceMotion,
    FrameRate,
    BatterySaver,
    PlayFreely,
    CrashReports,
    Voice,
    VoiceSource,
    VoiceJudge,
    Developer,
}

/// How a control is changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    /// On or off: Enter, Space, Left or Right flips it.
    Switch,
    /// A level from 0 to 100: Left and Right step it.
    Level,
    /// One of a few: Left and Right choose the one before or after.
    Choice,
}

impl Control {
    pub fn section(self) -> Section {
        match self {
            Control::Sound | Control::Level(_) => Section::Sound,
            Control::Language | Control::TextSize | Control::Contrast | Control::ReduceMotion => {
                Section::Display
            }
            Control::FrameRate | Control::BatterySaver => Section::Performance,
            Control::PlayFreely => Section::Playing,
            Control::CrashReports => Section::Reports,
            Control::Voice | Control::VoiceSource | Control::VoiceJudge => Section::Voice,
            Control::Developer => Section::Makers,
        }
    }

    pub fn kind(self) -> Kind {
        match self {
            Control::Level(_) => Kind::Level,
            Control::Language
            | Control::TextSize
            | Control::FrameRate
            | Control::CrashReports
            | Control::VoiceSource
            | Control::VoiceJudge => Kind::Choice,
            _ => Kind::Switch,
        }
    }
}

/// The controls Settings shows, in order: the levels only while sound is
/// on, and where the voice comes from only while it is on.
pub fn controls(sound_on: bool, voice_on: bool) -> Vec<Control> {
    let mut controls = vec![Control::Sound];
    if sound_on {
        controls.extend(Level::ALL.map(Control::Level));
    }
    controls.extend([
        Control::Language,
        Control::TextSize,
        Control::Contrast,
        Control::ReduceMotion,
        Control::FrameRate,
        Control::BatterySaver,
        Control::PlayFreely,
        Control::CrashReports,
        Control::Voice,
    ]);
    if voice_on {
        controls.extend([Control::VoiceSource, Control::VoiceJudge]);
    }
    controls.push(Control::Developer);
    controls
}

/// What a key asks of Settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyDoes {
    /// To the control before or after.
    Move(Step),
    /// The focused control a step down (-1) or up (1).
    Adjust(i32),
    /// Flip the focused switch.
    Toggle,
    /// Close Settings.
    Close,
}

/// The key's meaning in Settings, by GPUI's key name: Up, Down and Tab walk
/// the controls, Left and Right change the one focused, Enter and Space
/// flip a switch, Esc closes.
pub fn key_does(key: &str, shift: bool) -> Option<KeyDoes> {
    match key {
        "up" => Some(KeyDoes::Move(Step::Back)),
        "down" => Some(KeyDoes::Move(Step::Forward)),
        "tab" if shift => Some(KeyDoes::Move(Step::Back)),
        "tab" => Some(KeyDoes::Move(Step::Forward)),
        "left" => Some(KeyDoes::Adjust(-1)),
        "right" => Some(KeyDoes::Adjust(1)),
        "enter" | "space" => Some(KeyDoes::Toggle),
        "escape" => Some(KeyDoes::Close),
        _ => None,
    }
}

/// Where the focus stands after a key: a move goes where it went; a change
/// that may have changed the controls (sound or the voice turned off)
/// stays on the control it was on wherever that now stands, and on the
/// nearest one if it is gone.
pub fn refocus<T: PartialEq>(does: KeyDoes, was: Option<&T>, at: usize, after: &[T]) -> usize {
    let at = match (does, was) {
        (KeyDoes::Move(_), _) | (_, None) => at,
        (_, Some(was)) => after.iter().position(|each| each == was).unwrap_or(at),
    };
    at.min(after.len().saturating_sub(1))
}

/// How far one step moves a level.
pub const LEVEL_STEP: u8 = 10;

/// A level one step up or down, onto the steps of ten: 25 goes to 30 up
/// and 20 down.
pub fn step_level(level: u8, up: bool) -> u8 {
    let level = level.min(100);
    if up {
        (level / LEVEL_STEP * LEVEL_STEP + LEVEL_STEP).min(100)
    } else if !level.is_multiple_of(LEVEL_STEP) {
        level / LEVEL_STEP * LEVEL_STEP
    } else {
        level.saturating_sub(LEVEL_STEP)
    }
}

/// The option `by` places from `current` in `options`, stopping at
/// either end (← and →); `by` 0 is the next one, round from the last to
/// the first (Enter).
pub fn choose<T: Copy + PartialEq>(options: &[T], current: T, by: i32) -> T {
    let Some(at) = options.iter().position(|option| *option == current) else {
        return options.first().copied().unwrap_or(current);
    };
    if by == 0 {
        return options[(at + 1) % options.len()];
    }
    let to = (at as i64 + i64::from(by)).clamp(0, options.len() as i64 - 1) as usize;
    options[to]
}

/// The keyboard, as Settings lists it: the keys and what they do, ⌘ on a
/// Mac (the window shows Ctrl elsewhere).
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("↑ ↓", "Choose a card"),
    ("Enter", "Take the card"),
    ("Space", "Turn the card over"),
    ("← →", "Look along the place"),
    ("+ −", "Look closer or further"),
    ("⌘I", "Open the drawer"),
    ("H", "Make something"),
    ("P", "Take a photo"),
    ("C", "Make a postcard"),
    ("⌘Z", "Undo"),
    ("Esc", "Put away what is open"),
    ("⌘,", "Settings"),
    ("⌥⌘S", "Show as strip"),
    ("⌘W", "Close the window"),
    ("⌘Q", "Quit"),
];

/// Every line Settings shows from this plan, in English: for the
/// translation check.
pub fn every_line() -> Vec<&'static str> {
    let mut lines: Vec<&'static str> = Section::ALL.iter().map(|s| s.title()).collect();
    for level in Level::ALL {
        let (name, what) = level.words();
        lines.extend([name, what]);
    }
    for rate in crate::app_settings::FrameRate::ALL {
        lines.push(rate.label());
    }
    lines.extend(SHORTCUTS.iter().map(|(_, what)| *what));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Up, Down and Tab leave the control they were on (P6a review: the
    /// focus was put back on it, and never left Sound); a change keeps the
    /// focus on its control as the list changes under it.
    #[test]
    fn the_keys_walk_the_controls_and_a_change_keeps_its_place() {
        let all = controls(true, true);
        let first = all[0];
        let moved = crate::shell::step(all.len(), 0, Step::Forward);
        assert_eq!(moved, 1);
        assert_eq!(
            refocus(KeyDoes::Move(Step::Forward), Some(&first), moved, &all),
            1
        );
        // Sound turned off: the controls after the levels move up, and the
        // focus stays on the same control.
        let fewer = controls(false, true);
        let language = all.iter().position(|c| *c == Control::Language).unwrap();
        let now = refocus(KeyDoes::Toggle, Some(&Control::Language), language, &fewer);
        assert_eq!(fewer[now], Control::Language);
        // A control that is gone: the nearest that is left.
        assert!(refocus(KeyDoes::Toggle, Some(&first), all.len() - 1, &fewer) < fewer.len());
    }

    #[test]
    fn the_controls_follow_the_sections_in_order() {
        for (sound, voice) in [(true, true), (false, false), (true, false)] {
            let controls = controls(sound, voice);
            let sections: Vec<Section> = controls.iter().map(|c| c.section()).collect();
            let mut last = 0;
            for section in sections {
                let at = Section::ALL.iter().position(|s| *s == section).unwrap();
                assert!(at >= last, "{section:?} out of order");
                last = at;
            }
            assert_eq!(
                controls.iter().any(|c| matches!(c, Control::Level(_))),
                sound
            );
            assert_eq!(controls.contains(&Control::VoiceSource), voice);
            // Every setting P6 asks for is reachable by the keyboard.
            for wanted in [
                Control::Language,
                Control::TextSize,
                Control::Contrast,
                Control::ReduceMotion,
                Control::FrameRate,
                Control::BatterySaver,
                Control::PlayFreely,
                Control::CrashReports,
                Control::Voice,
            ] {
                assert!(controls.contains(&wanted), "{wanted:?}");
            }
        }
        let levels: Vec<_> = controls(true, false)
            .into_iter()
            .filter(|c| c.kind() == Kind::Level)
            .collect();
        assert_eq!(levels.len(), 5, "master, music, effects, ambient, voices");
    }

    #[test]
    fn the_keyboard_walks_and_changes_settings() {
        assert_eq!(key_does("down", false), Some(KeyDoes::Move(Step::Forward)));
        assert_eq!(key_does("tab", true), Some(KeyDoes::Move(Step::Back)));
        assert_eq!(key_does("right", false), Some(KeyDoes::Adjust(1)));
        assert_eq!(key_does("space", false), Some(KeyDoes::Toggle));
        assert_eq!(key_does("escape", false), Some(KeyDoes::Close));
        assert_eq!(key_does("a", false), None);
        assert_eq!(step_level(25, true), 30);
        assert_eq!(step_level(25, false), 20);
        assert_eq!(step_level(100, true), 100);
        assert_eq!(step_level(0, false), 0);
        assert_eq!(step_level(60, false), 50);
        let rates = crate::app_settings::FrameRate::ALL;
        use crate::app_settings::FrameRate::*;
        assert_eq!(choose(&rates, Sixty, 1), Display);
        assert_eq!(choose(&rates, Display, 1), Display);
        assert_eq!(choose(&rates, Sixty, -1), Thirty);
        assert_eq!(choose(&rates, Thirty, -1), Thirty);
        assert_eq!(choose(&rates, Sixty, 0), Display, "Enter: the next");
        assert_eq!(choose(&rates, Display, 0), Thirty, "and round");
    }
}
