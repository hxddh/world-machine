//! The two ages of Harbour Town the screenshot run photographs.
//!
//! They live here rather than in `examples/demo_world.rs` so the test below
//! can hold them to what the pictures need. A screenshot that has quietly
//! stopped showing the thing it is named after is worse than a missing one:
//! it goes on standing as evidence.

/// Late enough that the bakery has shut and Sea Finch has gone — the place
/// the canvas exists to draw. A town where nothing has gone wrong yet is a
/// row of identical open shops.
pub const DEMO_LATE_PERIODS: u64 = 80;

/// Early enough that the town is still living, which is the only condition
/// under which a *return* can be photographed at all: a return shades the
/// stretch the reader was away, and by [`DEMO_LATE_PERIODS`] Harbour Town has
/// come to rest, so an absence there spans no events and draws a flat line at
/// zero.
pub const DEMO_EARLY_PERIODS: u64 = 24;

/// How long the screenshot run backdates the observer's clock, in periods.
/// Two wall-clock days at six hours to the period.
pub const DEMO_ABSENCE_PERIODS: u64 = 8;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tiny_society_registration, TINY_SOCIETY_PACK_ID};
    use world_host::WorldRegistry;

    fn registry() -> WorldRegistry {
        let mut registry = WorldRegistry::new();
        registry
            .register(tiny_society_registration())
            .expect("Tiny Society registers");
        registry
    }

    /// What the return shot is supposed to contain. Checked here because the
    /// runner cannot: a macOS screenshot of a World that turned out to have
    /// nothing to shade looks exactly like one of a World that does.
    #[test]
    fn the_demo_town_has_something_to_shade_when_you_come_back() {
        let mut town = registry()
            .create(TINY_SOCIETY_PACK_ID)
            .expect("a town opens");
        town.advance_background(DEMO_EARLY_PERIODS)
            .expect("the town runs");
        let left_at = town.snapshot().world_time;

        town.advance_background(DEMO_ABSENCE_PERIODS)
            .expect("the town runs on while the reader is away");
        let returned = town.snapshot();

        let briefing = returned.briefing.expect("coming back gives a briefing");
        let since = briefing
            .since_world_time
            .expect("the briefing says where the absence began");
        assert!(
            since <= left_at,
            "the absence starts no later than the reader left: {since} against {left_at}"
        );

        let fortune = returned.fortune.expect("Harbour Town reports a figure");
        let band: Vec<i64> = fortune
            .history
            .iter()
            .filter(|point| point.world_time >= since)
            .map(|point| point.value)
            .collect();

        // Three separate ways of being a picture of nothing, and the reason
        // all three are checked: this test passed at 80 periods when it only
        // counted readings, because a town that has stopped still has a band
        // and still has readings in it — all of them zero.
        assert!(
            band.len() >= 8,
            "a band of {} reading(s) is not a shape anyone can read",
            band.len()
        );
        assert!(
            fortune.value > 0,
            "a return to a town where nothing is changing hands draws a flat \
             line along the bottom, which shades nothing legible"
        );
        assert!(
            band.iter().any(|value| *value != band[0]),
            "the readings across the absence are all {}, so the shading covers \
             a straight line and proves nothing about where it falls",
            band[0]
        );

        assert!(
            !briefing.items.is_empty(),
            "a return with nothing in it is not worth photographing"
        );
    }

    /// The other half of the pair, and the reason there are two: the World the
    /// place shots use has nothing left to report, so it cannot stand in for
    /// the one above.
    #[test]
    fn the_late_town_has_come_to_rest_and_so_cannot_show_an_absence() {
        let mut town = registry()
            .create(TINY_SOCIETY_PACK_ID)
            .expect("a town opens");
        town.advance_background(DEMO_LATE_PERIODS)
            .expect("the town runs");
        town.advance_background(DEMO_ABSENCE_PERIODS)
            .expect("the town sits still while the reader is away");

        let fortune = town
            .snapshot()
            .fortune
            .expect("Harbour Town reports a figure");
        assert_eq!(
            fortune.value, 0,
            "a town at rest has no money changing hands, which is why the \
             return shot needs a younger one"
        );
    }
}
