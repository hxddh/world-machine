pub mod art;
pub mod diorama;
mod macos;
pub mod scene;
pub mod ui;

pub use macos::{is_beginning, scene_share, words_at_rest, ProjectionView, RESTING_WORD_LIMIT};
pub use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub trait ProjectionController {
    fn snapshot(&self) -> ProjectionSnapshot;

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, String>;

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
