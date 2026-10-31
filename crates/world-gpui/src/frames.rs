//! How often a World's window draws while things move: as often as the
//! display asks, or at most the player's frame cap (Settings' 30 or 60
//! frames a second, or the battery saver's 30).
//!
//! The window asks a [`Pacer`] before each frame it wants: the pacer says
//! draw at the next refresh, or after a wait that keeps the frames at the
//! cap, or nothing (a wait is already running). The pacer is pure and
//! clock-free; the window gives it the time.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// The cap, in frames a second; 0 is the display's own rate.
static CAP: AtomicU32 = AtomicU32::new(0);

/// The most frames a second a World window draws; `None` is as often as
/// the display refreshes.
pub fn set_frame_cap(cap: Option<u32>) {
    CAP.store(cap.unwrap_or(0), Ordering::Relaxed);
}

pub fn frame_cap() -> Option<u32> {
    match CAP.load(Ordering::Relaxed) {
        0 => None,
        cap => Some(cap),
    }
}

/// How close a cap may come to the display's own rate and still count as
/// it: a 60 cap on a 60 Hz display draws every refresh, never waiting.
pub const SLACK: Duration = Duration::from_millis(2);

/// Asks closer together than this are of one frame: no display refreshes
/// faster than 1,000 times a second.
const SAME_FRAME: Duration = Duration::from_millis(1);

/// What the window should do about the frame it wants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ask {
    /// Draw at the next refresh.
    Now,
    /// Draw once this long has passed.
    After(Duration),
    /// A wait is already running; its frame will come.
    Waiting,
}

/// One window's frame pacing. It learns the display's refresh from the
/// frames it lets through at once (the shortest time between two), and
/// waits only when the cap is under that rate: then each wait ends half a
/// refresh before the beat, so the frame lands on the refresh at the beat
/// however late a timer wakes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pacer {
    /// When the frame before was drawn, if it was let through at once.
    last_now: Option<Instant>,
    /// The display's refresh, as far as it has been seen.
    refresh: Option<Duration>,
    waiting: bool,
}

impl Pacer {
    /// The frame drawn at `now` wants the next, with the frames capped at
    /// `cap` a second (`None`: the display's rate).
    pub fn ask(&mut self, now: Instant, cap: Option<u32>) -> Ask {
        if self.waiting {
            if cap.is_some() {
                return Ask::Waiting;
            }
            self.waiting = false;
        }
        if let Some(last) = self.last_now {
            let between = now.saturating_duration_since(last);
            // Asked again within the same frame (the scene and a camera
            // glide both want the next): the same answer.
            if between < SAME_FRAME {
                return Ask::Now;
            }
            self.refresh = Some(self.refresh.map_or(between, |seen| seen.min(between)));
            self.last_now = None;
        }
        let interval = cap
            .filter(|cap| *cap > 0)
            .map(|cap| Duration::from_secs(1) / cap);
        match (interval, self.refresh) {
            // Under a cap below the display's rate, wait for the beat.
            (Some(interval), Some(refresh)) if interval > refresh + SLACK => {
                self.waiting = true;
                Ask::After(interval - refresh / 2)
            }
            // Uncapped, at or over the display's rate, or the rate not yet
            // seen: the next refresh.
            _ => {
                self.last_now = Some(now);
                Ask::Now
            }
        }
    }

    /// The wait asked for ended: its frame is the next one.
    pub fn fired(&mut self) {
        self.waiting = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// When a window asking for a frame after every frame draws in a
    /// second of a `hz` display, capped at `cap`: a frame let through at
    /// once draws on the next refresh; a wait's frame on the first refresh
    /// after its timer, which wakes `late` after it was due.
    fn frames_in_a_second(hz: u32, cap: Option<u32>, late: Duration) -> Vec<Duration> {
        let start = Instant::now();
        let refresh = Duration::from_secs(1) / hz;
        let mut pacer = Pacer::default();
        let mut drawn = Vec::new();
        let mut due: Option<Instant> = None;
        let mut wants = true;
        for tick in 0..hz {
            let now = start + refresh * tick;
            if let Some(at) = due {
                if now >= at + late {
                    pacer.fired();
                    due = None;
                    wants = true;
                }
            }
            if !wants {
                continue;
            }
            // The frame drawn now asks for the next.
            drawn.push(now - start);
            wants = false;
            match pacer.ask(now, cap) {
                Ask::Now => wants = true,
                Ask::After(wait) => due = Some(now + wait),
                Ask::Waiting => {}
            }
        }
        drawn
    }

    /// The time between frames once the pacer has seen the display: each
    /// gap, rounded to a tenth of a millisecond.
    fn gaps(frames: &[Duration]) -> Vec<u64> {
        frames
            .windows(2)
            .skip(2)
            .map(|pair| ((pair[1] - pair[0]).as_micros() as u64 + 50) / 100)
            .collect()
    }

    #[test]
    fn the_display_rate_draws_every_refresh() {
        let none = Duration::ZERO;
        assert_eq!(frames_in_a_second(60, None, none).len(), 60);
        assert_eq!(frames_in_a_second(120, None, none).len(), 120);
    }

    /// Capped under the display's rate, the frames come at the cap, evenly,
    /// even when the timer wakes a few milliseconds late: each wait ends
    /// half a refresh early, so its frame lands on the refresh at the beat.
    #[test]
    fn a_cap_holds_the_frames_at_it() {
        for late in [Duration::ZERO, Duration::from_millis(3)] {
            for (hz, cap) in [(120, 60), (120, 30), (60, 30)] {
                let frames = frames_in_a_second(hz, Some(cap), late);
                let count = frames.len() as u32;
                // The first two frames go at once, while the display's
                // rate is seen.
                assert!(
                    (cap..=cap + 1).contains(&count),
                    "{cap} on {hz} Hz, {late:?} late: {count} frames"
                );
                let beat = (1_000_000 / u64::from(cap) + 50) / 100;
                assert!(
                    gaps(&frames).iter().all(|gap| gap.abs_diff(beat) <= 1),
                    "{cap} on {hz} Hz, {late:?} late: {:?}",
                    gaps(&frames)
                );
            }
        }
        // A cap at or over the display's rate costs no frame.
        let none = Duration::ZERO;
        assert_eq!(frames_in_a_second(60, Some(60), none).len(), 60);
        assert_eq!(frames_in_a_second(60, Some(120), none).len(), 60);
        assert_eq!(frames_in_a_second(120, Some(120), none).len(), 120);
    }

    #[test]
    fn one_wait_at_a_time_and_none_without_a_cap() {
        let start = Instant::now();
        let refresh = Duration::from_millis(8);
        let mut pacer = Pacer::default();
        // The rate is not known yet: at once, twice, and then it is.
        assert_eq!(pacer.ask(start, Some(30)), Ask::Now);
        let next = start + refresh;
        match pacer.ask(next, Some(30)) {
            Ask::After(wait) => {
                assert_eq!(wait, Duration::from_secs(1) / 30 - refresh / 2)
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(pacer.ask(next, Some(30)), Ask::Waiting);
        pacer.fired();
        assert!(matches!(
            pacer.ask(start + Duration::from_millis(34), Some(30)),
            Ask::After(_)
        ));
        // Uncapped, a frame is always at once, and a pending wait is let go.
        assert_eq!(pacer.ask(next, None), Ask::Now);
        assert_eq!(pacer.ask(next, None), Ask::Now);
    }

    /// The scene and a camera glide both ask within one frame: one answer,
    /// and the display's rate is not mistaken for the gap between them.
    #[test]
    fn two_asks_in_one_frame_are_one() {
        let start = Instant::now();
        let refresh = Duration::from_secs(1) / 60;
        let mut pacer = Pacer::default();
        let mut now = start;
        for _ in 0..4 {
            assert_eq!(pacer.ask(now, Some(60)), Ask::Now);
            assert_eq!(
                pacer.ask(now + Duration::from_micros(200), Some(60)),
                Ask::Now
            );
            now += refresh;
        }
        assert_eq!(pacer.refresh, Some(refresh));
    }
}
