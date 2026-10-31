//! The interface at rest (1.0 P5, the v0.30 review: "an interface that
//! hides at rest; it covers 10–15% of the frame"): the handles, the gauges
//! and the zoom over the scene fade to a whisper once the player has
//! left the mouse and the keys alone for [`QUIET_AFTER`] seconds, and are
//! back at once on a move of the mouse, a key, the wheel or a click.
//! Never while a card, a menu or a pointer is up: what the player is
//! reading or choosing from keeps its handles. With Reduce Motion on, the
//! fade is not animated: the interface is simply quiet.
//!
//! Window state only, like the camera: nothing the World records.

use super::*;

/// How long without the mouse or the keys before the interface quietens.
pub(crate) const QUIET_AFTER: f32 = 4.0;

/// How long the interface takes to fade to quiet.
pub(crate) const QUIET_FADE: f32 = 0.8;

/// How much of the interface shows at rest: enough to find it again.
pub(crate) const QUIET_OPACITY: f32 = 0.18;

/// How much of the interface shows `idle` seconds after the player last
/// touched the mouse or the keys: all of it while something is `held`
/// open, or until [`QUIET_AFTER`]; then fading to [`QUIET_OPACITY`] over
/// [`QUIET_FADE`], or at once when everything is held `still`.
pub(crate) fn chrome_opacity(idle: f32, held: bool, still: bool) -> f32 {
    if held || idle < QUIET_AFTER {
        return 1.0;
    }
    if still {
        return QUIET_OPACITY;
    }
    let t = ((idle - QUIET_AFTER) / QUIET_FADE).clamp(0.0, 1.0);
    let eased = t * t * (3.0 - 2.0 * t);
    1.0 - (1.0 - QUIET_OPACITY) * eased
}

/// Whether the interface is still on its way to quiet `idle` seconds after
/// the player last touched anything: a frame is wanted only then, never
/// once it has settled (whole or quiet).
pub(crate) fn chrome_fading(idle: f32, held: bool, still: bool) -> bool {
    !held && !still && (QUIET_AFTER..QUIET_AFTER + QUIET_FADE).contains(&idle)
}

impl ProjectionView {
    /// The player touched the mouse or the keys: the interface is back.
    /// Asks for a frame only when it was quiet, so a moving mouse draws
    /// nothing more than it did.
    pub(super) fn touched(&mut self, cx: &mut Context<Self>) {
        let was = self.idle_seconds();
        self.looking.touched = Some(Instant::now());
        if was >= QUIET_AFTER {
            cx.notify();
        }
    }

    /// Seconds since the player last touched the mouse or the keys (or
    /// since the window opened).
    pub(super) fn idle_seconds(&self) -> f32 {
        since(self.looking.touched.or(self.looking.started))
    }

    /// Whether something open keeps the interface in full: a card to
    /// choose from, the hands, the drawer, someone being talked to, the
    /// player's mark, a page, a moment, a pointer, a photograph.
    pub(super) fn chrome_held(&self, pointing: bool) -> bool {
        let marking = &self.looking.marking;
        pointing
            || self.card_command().is_some()
            || self.looking.hands.is_some()
            || self.looking.drawer
            || self.looking.asking.is_some()
            || self.looking.photographing
            || self.reading.page.is_some()
            || self.moment_up()
            || marking.design.is_some()
            || marking.card.is_some()
            || marking.offers.is_some()
            || self.retelling.is_some()
    }

    /// How much of the interface shows this frame, and whether it is
    /// still fading (and wants another frame).
    pub(super) fn chrome_now(&self, pointing: bool, still: bool) -> (f32, bool) {
        let (idle, held) = (self.idle_seconds(), self.chrome_held(pointing));
        (
            chrome_opacity(idle, held, still),
            chrome_fading(idle, held, still),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_wanted_only_while_the_interface_fades() {
        // Whole, held or settled at quiet: no frame is asked for, so an
        // idle window keeps its frame cap and rests when inactive.
        for idle in [0.0, QUIET_AFTER - 0.01, QUIET_AFTER + QUIET_FADE, 600.0] {
            assert!(!chrome_fading(idle, false, false), "{idle}");
        }
        for idle in [QUIET_AFTER, QUIET_AFTER + QUIET_FADE / 2.0] {
            assert!(chrome_fading(idle, false, false), "{idle}");
            assert!(!chrome_fading(idle, true, false), "{idle} held");
            assert!(!chrome_fading(idle, false, true), "{idle} still");
        }
        // Settled at quiet, the opacity is below whole: that alone is no
        // reason for another frame.
        assert!(chrome_opacity(600.0, false, false) < 1.0);
    }

    #[test]
    fn the_interface_is_whole_until_left_alone() {
        for idle in [0.0, 1.0, QUIET_AFTER - 0.01] {
            assert_eq!(chrome_opacity(idle, false, false), 1.0, "{idle}");
            assert_eq!(chrome_opacity(idle, false, true), 1.0, "{idle}");
        }
    }

    #[test]
    fn the_interface_fades_to_quiet_and_stays_there() {
        let mut last = 1.0;
        let mut t = QUIET_AFTER;
        while t <= QUIET_AFTER + QUIET_FADE + 0.5 {
            let now = chrome_opacity(t, false, false);
            assert!(now <= last + 1e-6, "{t}: {now} after {last}");
            assert!(now >= QUIET_OPACITY - 1e-6);
            last = now;
            t += 0.05;
        }
        assert!((last - QUIET_OPACITY).abs() < 1e-5);
        // Half way through the fade, half way there.
        let half = chrome_opacity(QUIET_AFTER + QUIET_FADE / 2.0, false, false);
        assert!((half - (1.0 + QUIET_OPACITY) / 2.0).abs() < 1e-3, "{half}");
        assert_eq!(chrome_opacity(600.0, false, false), QUIET_OPACITY);
    }

    #[test]
    fn a_card_or_menu_open_keeps_the_interface() {
        for idle in [QUIET_AFTER, QUIET_AFTER + QUIET_FADE, 600.0] {
            assert_eq!(chrome_opacity(idle, true, false), 1.0);
            assert_eq!(chrome_opacity(idle, true, true), 1.0);
        }
    }

    #[test]
    fn with_reduce_motion_the_interface_goes_quiet_without_a_fade() {
        assert_eq!(chrome_opacity(QUIET_AFTER - 0.01, false, true), 1.0);
        assert_eq!(chrome_opacity(QUIET_AFTER, false, true), QUIET_OPACITY);
        assert_eq!(
            chrome_opacity(QUIET_AFTER + 0.01, false, true),
            QUIET_OPACITY
        );
    }
}
