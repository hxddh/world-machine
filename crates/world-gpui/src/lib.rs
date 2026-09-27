pub mod art;
pub mod diorama;
mod macos;
pub mod scene;
pub mod text_input;
pub mod ui;

pub use macos::{is_beginning, scene_share, words_at_rest, ProjectionView, RESTING_WORD_LIMIT};
pub use world_projection::{Ears, ProjectionIntent, ProjectionSnapshot, SelectionId};

use std::time::Duration;

/// Work that asks a language model what the player's words to someone
/// mean, run off the window's thread: the model's response, or nothing.
pub type Listening = Box<dyn FnOnce() -> Option<String> + Send>;

/// How long the window lets a model think before the World's own ears
/// answer instead.
pub const LISTEN_DEADLINE: Duration = Duration::from_secs(12);

/// Runs `listening` on a thread of its own and waits at most `deadline`
/// for it: the response, or nothing if it took longer or had none. A model
/// that never answers is left behind; nothing waits on it.
pub fn listen_within(listening: Listening, deadline: Duration) -> Option<String> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn a_model_that_takes_too_long_is_not_waited_for() {
        let started = Instant::now();
        let slow: Listening = Box::new(|| {
            std::thread::sleep(Duration::from_secs(5));
            Some("too late".into())
        });
        assert_eq!(listen_within(slow, Duration::from_millis(100)), None);
        assert!(started.elapsed() < Duration::from_secs(2));

        let quick: Listening = Box::new(|| Some("MEANING: greet".into()));
        assert_eq!(
            listen_within(quick, Duration::from_secs(5)).as_deref(),
            Some("MEANING: greet")
        );
        let silent: Listening = Box::new(|| None);
        assert_eq!(listen_within(silent, Duration::from_secs(5)), None);
    }
}
