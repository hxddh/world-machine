mod comparison;
mod macos;

pub use comparison::StrategyComparisonView;
pub use macos::ProjectionView;
pub use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub trait ProjectionController {
    fn snapshot(&self) -> ProjectionSnapshot;

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, String>;
}
