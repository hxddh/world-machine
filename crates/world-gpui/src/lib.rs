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
}
