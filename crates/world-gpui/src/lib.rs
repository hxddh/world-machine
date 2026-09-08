mod macos;

pub use macos::ProjectionView;
pub use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub trait ProjectionController {
    fn snapshot(&self) -> ProjectionSnapshot;

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, String>;
}

/// A light-palette colour adapted to the current appearance.
pub(crate) fn theme_rgb(hex: u32) -> gpui::Rgba {
    gpui::rgb(world_theme::adapt(hex))
}
