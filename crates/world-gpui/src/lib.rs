mod macos;
pub mod scene;
pub mod ui;

pub use macos::ProjectionView;
pub use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub trait ProjectionController {
    fn snapshot(&self) -> ProjectionSnapshot;

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, String>;
}
