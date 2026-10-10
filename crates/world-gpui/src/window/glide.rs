//! The camera's glide from where it is to what it is sent to look at, kept
//! apart from the window so its timing can be tested on a clock the test
//! holds: every step takes `now`, never reads the wall clock itself.
//!
//! A glide to a new subject (a return beat, someone being talked to) is
//! held where it set off until what it will see there is painted, or for a
//! moment at most; then it glides for [`CAMERA_SECONDS`]. Whatever waits for
//! the camera (the film's words, the talk card) waits for [`Glide::arrived`]
//! on its own subject. Until it has arrived, the glide asks for the next
//! frame itself: a window in front but not active draws only when asked
//! (v0.29 round 2: beats 1 and 7, already painted ahead, asked for no
//! frame, so the camera stayed on the last beat and its words never came).

use crate::diorama::Camera;
use std::time::{Duration, Instant};
use world_projection::SelectionId;

/// How long the camera takes to move.
pub(crate) const CAMERA_SECONDS: f32 = 0.9;
/// The longest a glide waits where it set off for what it will see to be
/// painted.
pub(crate) const CAMERA_HOLD_SECONDS: f32 = 1.2;

/// What the camera is sent to look at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bound {
    /// A beat of the return film, by its index.
    Beat(usize),
    /// Whoever is being talked to.
    Asking(SelectionId),
    /// Wherever the player has zoomed and panned to.
    Free,
}

/// Where the camera set off from, where it is going, and since when.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Glide {
    pub(crate) from: Option<Camera>,
    pub(crate) to: Option<Camera>,
    /// When the glide set off (or, held, now).
    pub(crate) at: Option<Instant>,
    /// Since when a glide to a new subject has been held where it set off,
    /// waiting for what it will see there to be painted.
    pub(crate) hold: Option<Instant>,
    /// What it is bound for.
    pub(crate) bound: Option<Bound>,
}

fn seconds(from: Option<Instant>, now: Instant) -> f32 {
    from.map(|at| now.saturating_duration_since(at).as_secs_f32())
        .unwrap_or(f32::MAX)
}

/// Two views close enough that going from one to the other is no move.
fn near(a: Camera, b: Camera) -> bool {
    (a.x - b.x).abs() < 4.0
        && (a.y - b.y).abs() < 4.0
        && (a.zoom - b.zoom).abs() < 0.01
        && (a.fold - b.fold).abs() < 0.01
}

impl Glide {
    /// Where the camera is at `now`.
    pub(crate) fn current(&self, now: Instant, whole: Camera) -> Camera {
        match (self.from, self.to) {
            // Held where it set off, it stays there exactly.
            (Some(from), Some(_)) if self.hold.is_some() => from,
            (Some(from), Some(to)) => from.toward(to, seconds(self.at, now) / CAMERA_SECONDS),
            _ => whole,
        }
    }

    /// One frame: the camera is sent to `target`, for `bound`, at `now`.
    /// `snap` puts it there at once (a drag, the window's first frame);
    /// `ready` says whether all the camera will see from a view is painted
    /// (`None` until a frame heading there has been drawn). Returns where
    /// the camera is, and whether it wants the next frame drawn.
    pub(crate) fn step(
        &mut self,
        target: Camera,
        bound: Bound,
        now: Instant,
        whole: Camera,
        snap: bool,
        ready: impl Fn(Camera) -> Option<bool>,
    ) -> (Camera, bool) {
        let current = self.current(now, whole);
        if self.to.is_none() || snap {
            let new_subject = self.bound != Some(bound);
            // There at once: arrived, nothing to glide.
            *self = Glide {
                from: Some(target),
                to: Some(target),
                at: Some(now - Duration::from_secs_f32(CAMERA_SECONDS)),
                hold: None,
                bound: Some(bound),
            };
            return (target, new_subject);
        }
        let new_subject = self.bound != Some(bound);
        if self.to != Some(target) {
            if bound == Bound::Free {
                // The wheel follows the hand at once.
                self.from = Some(current);
                self.to = Some(target);
                self.at = Some(now - Duration::from_secs_f32(CAMERA_SECONDS * 0.7));
                self.hold = None;
            } else if !new_subject {
                // The same subject, shifted a little (someone stepped
                // aside): the glide goes on toward where it is now, its
                // clock and any hold kept, so a subject that moves never
                // holds the camera for ever.
                self.to = Some(target);
            } else if near(current, target) && self.hold.is_none() {
                // Already in view: no move, so whatever waits for the
                // camera shows at once.
                self.from = Some(target);
                self.to = Some(target);
                self.at = Some(now - Duration::from_secs_f32(CAMERA_SECONDS));
            } else {
                self.from = Some(current);
                self.to = Some(target);
                self.at = Some(now);
                // A hold already running keeps its start.
                self.hold = self.hold.or(Some(now));
            }
        }
        self.bound = Some(bound);
        // A glide somewhere new sets off only once what the camera will
        // see there is painted (or after a moment at most): it never lands
        // on the rough painting (the v0.29 art director's Find).
        if let Some(held) = self.hold {
            let painted = ready(target).unwrap_or(false);
            if painted || seconds(Some(held), now) >= CAMERA_HOLD_SECONDS {
                self.hold = None;
            }
            // The glide's clock starts when it sets off.
            self.at = Some(now);
        }
        let current = self.current(now, whole);
        // A new subject wants one more frame, so whatever waits for the
        // camera sees it arrived; a hold or a glide wants every frame.
        let wants = new_subject || self.hold.is_some() || self.moving(now);
        (current, wants)
    }

    /// Whether the camera is still on its way (held or gliding) at `now`.
    pub(crate) fn moving(&self, now: Instant) -> bool {
        self.hold.is_some() || seconds(self.at, now) <= CAMERA_SECONDS
    }

    /// Whether the camera has arrived at `bound` by `now`: sent there, not
    /// held, and at the end of its glide (landed, so words shown then are
    /// over their own subject).
    pub(crate) fn arrived(&self, bound: Bound, now: Instant) -> bool {
        self.bound == Some(bound) && self.hold.is_none() && seconds(self.at, now) >= CAMERA_SECONDS
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn view(x: f32) -> Camera {
        Camera {
            zoom: 1.6,
            x,
            y: 300.0,
            fold: 0.0,
        }
    }

    /// How painting goes in a run: ready at once (painted ahead), after a
    /// while, or late (after the hold has timed out).
    #[derive(Clone, Copy, Debug)]
    pub(crate) enum Paint {
        Ahead,
        After(f32),
        /// Only after the hold has given up waiting.
        Late,
    }

    /// A return film played as the app plays it, on a clock the test
    /// holds: each frame, first whether the words show (the window's
    /// bookkeeping comes before the camera: the camera arrived at the
    /// beat, and the frame drawn before this one showed the view painted),
    /// then the camera's step. A window in front but not active draws a
    /// frame only when one was asked for (by the glide, by painting still
    /// to do, or by words still waiting) or on a key; a slow display draws
    /// one every `frame` seconds. Next is pressed every `press` seconds.
    /// Returns, for each beat, when its words first showed after Next, and
    /// whether they showed on its subject.
    pub(crate) fn play(
        subjects: &[Camera],
        whole: Camera,
        paint: Paint,
        frame: f32,
        press: f32,
    ) -> Vec<(f32, bool)> {
        let start = Instant::now();
        let mut glide = Glide::default();
        let mut shown = Vec::new();
        let mut clock = 0.0_f32;
        // Whether the frame drawn last showed what the camera saw painted.
        let mut painted_seen = true;
        let at = |s: f32| start + Duration::from_secs_f32(s);
        for (beat, subject) in subjects.iter().enumerate() {
            let pressed = clock;
            let target = *subject;
            // When what the camera will see there is painted: at once if
            // it is already in view.
            let in_view = beat > 0 && subjects[beat - 1] == target;
            let painted_at = pressed
                + match paint {
                    _ if in_view => 0.0,
                    Paint::Ahead => 0.0,
                    Paint::After(s) => s,
                    Paint::Late => CAMERA_HOLD_SECONDS + 0.4,
                };
            let mut first = None;
            let mut wants = true; // the key press draws a frame
            while clock < pressed + press {
                // Nothing asks: the next frame is the next key press,
                // unless painting is still to do.
                if !wants && clock >= painted_at {
                    break;
                }
                let now = at(clock);
                let ready =
                    |to: Camera| -> Option<bool> { (to == target).then_some(clock >= painted_at) };
                // The window's bookkeeping, then the camera.
                let words = glide.arrived(Bound::Beat(beat), now) && painted_seen;
                let (camera, more) =
                    glide.step(target, Bound::Beat(beat), now, whole, false, ready);
                if words && first.is_none() {
                    first = Some((clock - pressed, near(camera, target)));
                }
                // What this frame shows: painted, unless the camera has
                // moved off where it was and its destination is not yet.
                painted_seen = clock >= painted_at || glide.hold.is_some();
                // Words still waiting ask for the next frame themselves.
                wants = more || !words;
                clock += frame;
            }
            shown.push(first.unwrap_or((f32::MAX, false)));
            clock = pressed + press;
        }
        shown
    }

    /// Every beat's words show within 2.5 s of Next, on the beat's own
    /// subject, however painting goes, however slowly the display draws,
    /// and whether or not the window asks for frames by itself; a subject
    /// already in view shows its words at once.
    #[test]
    fn every_beat_of_the_return_film_speaks_on_its_subject_within_two_and_a_half_seconds() {
        // The film's subjects along the place; beats 3 and 4 are about the
        // same thing.
        let subjects = [300.0, 1800.0, 600.0, 2400.0, 2400.0, 900.0, 2000.0, 400.0].map(view);
        let shown = film_runs(&subjects, view(1000.0));
        // Beat 4 is about what beat 3 was: its words at once.
        for shown in shown {
            assert!(shown[4].0 <= 0.25 * 2.0 + 0.02, "{shown:?}");
        }
    }

    /// The film played several times over `subjects`, every way painting
    /// and the display can go, each beat's words held to 2.5 s after Next
    /// on its own subject. Returns each run's timings.
    pub(crate) fn film_runs(subjects: &[Camera], whole: Camera) -> Vec<Vec<(f32, bool)>> {
        let mut runs = Vec::new();
        for paint in [
            Paint::Ahead,
            Paint::After(0.4),
            Paint::After(1.2),
            Paint::Late,
        ] {
            for frame in [1.0 / 60.0, 0.25] {
                for press in [4.0, 6.0] {
                    for run in 0..3 {
                        let frame = frame + run as f32 * 0.003;
                        let shown = play(subjects, whole, paint, frame, press);
                        for (beat, (after, on_subject)) in shown.iter().enumerate() {
                            assert!(
                                *after <= 2.5 && *on_subject,
                                "{paint:?}, a frame every {frame}s, run {run}: beat {beat} \
                                 spoke at {after}s, on its subject: {on_subject}"
                            );
                        }
                        runs.push(shown);
                    }
                }
            }
        }
        runs
    }

    /// The talk card waits for the camera to land on whoever it is for,
    /// and shows as soon as it has.
    #[test]
    fn the_talk_card_waits_for_the_landing() {
        let start = Instant::now();
        let mut glide = Glide::default();
        let whole = view(1000.0);
        glide.step(view(300.0), Bound::Free, start, whole, false, |_| {
            Some(true)
        });
        let who = SelectionId::from_stable_key("entity-1").expect("an id");
        let mut landed = None;
        for i in 0..240 {
            let t = 2.0 + i as f32 / 60.0;
            let now = start + Duration::from_secs_f32(t);
            if glide.arrived(Bound::Asking(who), now) {
                landed = Some(t - 2.0);
                break;
            }
            glide.step(view(1800.0), Bound::Asking(who), now, whole, false, |_| {
                Some(true)
            });
        }
        let landed = landed.expect("it lands");
        assert!(
            (CAMERA_SECONDS * 0.8..=CAMERA_SECONDS + 0.1).contains(&landed),
            "the card waited {landed}s"
        );
    }

    /// The words of a new beat never show for a frame over the last beat's
    /// subject before the camera has been sent on.
    #[test]
    fn a_new_beat_never_speaks_over_the_last_subject() {
        let start = Instant::now();
        let mut glide = Glide::default();
        let whole = view(1000.0);
        let ready = |_: Camera| Some(true);
        glide.step(view(300.0), Bound::Beat(0), start, whole, false, ready);
        let later = start + Duration::from_secs(4);
        assert!(glide.arrived(Bound::Beat(0), later));
        assert!(!glide.arrived(Bound::Beat(1), later), "not sent there yet");
        let (_, wants) = glide.step(view(1800.0), Bound::Beat(1), later, whole, false, ready);
        assert!(wants, "a glide asks for its frames");
        assert!(!glide.arrived(Bound::Beat(1), later), "on its way");
    }

    /// A subject that keeps shifting a little never holds the camera where
    /// it set off: the glide goes on and lands.
    #[test]
    fn a_shifting_subject_never_holds_the_camera_for_ever() {
        let start = Instant::now();
        let mut glide = Glide::default();
        let whole = view(1000.0);
        glide.step(view(300.0), Bound::Beat(0), start, whole, false, |_| {
            Some(true)
        });
        let mut landed = None;
        for i in 1..400 {
            let t = i as f32 / 60.0;
            let now = start + Duration::from_secs_f32(t);
            // Never painted where it is heading: each frame's target is
            // new to the painter.
            let target = view(1800.0 + i as f32 * 0.5);
            let words = glide.arrived(Bound::Beat(1), now);
            glide.step(target, Bound::Beat(1), now, whole, false, |_| None);
            if words {
                landed = Some(t);
                break;
            }
        }
        let landed = landed.expect("the camera lands");
        assert!(
            landed <= CAMERA_HOLD_SECONDS + CAMERA_SECONDS + 0.05,
            "{landed}"
        );
    }
}
