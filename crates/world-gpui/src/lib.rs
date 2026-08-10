mod macos;

use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub use macos::ProjectionView;

pub trait ProjectionController {
    fn snapshot(&self) -> ProjectionSnapshot;

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, String>;
}
