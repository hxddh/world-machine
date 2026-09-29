//! A note for every act: what the player does answers at once, in the key
//! of the World's tune, softly, and never the same twice running.
//!
//! Each act has a sound of its own that says what it is (wood for placing
//! something, paper for a card, a bell for a letter, glass for a keepsake)
//! and a few notes of the World's pentatonic scale that put it in the
//! music's key. Each has [`VARIATIONS`] versions.

use crate::dsp::Dice;
use crate::tune::pentatonic;
use crate::voices::{Bus, Instrument, Note, Swish};

/// Something the player did that the World answers with a sound.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Act {
    /// A card was answered and the World moved on.
    Answer,
    /// Something was built, planted or placed.
    Place,
    /// A letter came.
    Letter,
    /// Someone handed over a keepsake.
    Keepsake,
    /// A card was turned over, or the next one brought up.
    Flip,
    /// The drawer was opened.
    Drawer,
}

impl Act {
    pub const ALL: [Act; 6] = [
        Act::Answer,
        Act::Place,
        Act::Letter,
        Act::Keepsake,
        Act::Flip,
        Act::Drawer,
    ];

    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

/// How many versions each act's sound has.
pub const VARIATIONS: u8 = 4;

/// The version to play next: any but the one played last.
pub(crate) fn next_variation(last: Option<u8>, dice: &mut Dice) -> u8 {
    match last {
        None => dice.pick(VARIATIONS as usize) as u8,
        Some(last) => {
            let other = dice.pick(VARIATIONS as usize - 1) as u8;
            if other >= last {
                other + 1
            } else {
                other
            }
        }
    }
}

/// One part of an act's sound: a note or a swish, `delay` seconds in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Part {
    Note {
        delay: f32,
        note: Note,
    },
    Swish {
        delay: f32,
        swish: Swish,
        place: f32,
    },
}

impl Part {
    pub(crate) fn delay(&self) -> f32 {
        match self {
            Part::Note { delay, .. } | Part::Swish { delay, .. } => *delay,
        }
    }
}

fn note(delay: f32, instrument: Instrument, pitch: f32, level: f32, place: f32) -> Part {
    Part::Note {
        delay,
        note: Note {
            instrument,
            pitch,
            level,
            length: 0.3,
            place,
            bus: Bus::Act,
        },
    }
}

fn swish(delay: f32, from: f32, to: f32, seconds: f32, level: f32, q: f32, swell: f32) -> Part {
    Part::Swish {
        delay,
        swish: Swish {
            from,
            to,
            seconds,
            level,
            q,
            swell,
        },
        place: 0.0,
    }
}

/// The sound of `act`, version `variation`, in the key whose root is MIDI
/// `root`: every pitch is on the World's pentatonic scale.
pub(crate) fn parts(act: Act, variation: u8, root: f32) -> Vec<Part> {
    let v = usize::from(variation % VARIATIONS);
    let high = root + 24.0;
    let step = pentatonic;
    match act {
        // A soft rising pair on the kalimba, settling on a chord tone,
        // over a low warm bell: the World heard you.
        Act::Answer => {
            let [first, second] = [[2, 4], [0, 2], [4, 5], [1, 4]][v];
            vec![
                note(0.0, Instrument::Kalimba, high + step(first), 0.5, -0.1),
                note(0.085, Instrument::Kalimba, high + step(second), 0.55, 0.1),
                note(
                    0.0,
                    Instrument::Bell,
                    root + 12.0 + step([0, 2, 0, 4][v]),
                    0.18,
                    0.0,
                ),
            ]
        }
        // Wood: a round low "plop" with a thump, and a small grace note
        // above as it settles.
        Act::Place => {
            let low = [0, 2, 4, -1][v];
            vec![
                swish(0.0, 260.0, 140.0, 0.03, 0.5, 0.8, 0.0),
                note(0.0, Instrument::Marimba, root + 12.0 + step(low), 0.7, 0.0),
                note(0.07, Instrument::Kalimba, high + step(low + 2), 0.2, 0.15),
            ]
        }
        // Paper drawn from an envelope, then two bells, falling: a letter.
        Act::Letter => {
            let [first, second] = [[4, 2], [5, 3], [7, 4], [3, 0]][v];
            vec![
                swish(0.0, 2600.0, 1400.0, 0.2, 0.3, 1.1, 0.06),
                note(0.14, Instrument::Bell, high + step(first), 0.4, -0.15),
                note(0.3, Instrument::Bell, high + step(second), 0.35, 0.15),
            ]
        }
        // Glass: a small rising arpeggio, the last note held longest.
        Act::Keepsake => {
            let shape = [[0, 1, 2, 4], [1, 2, 4, 5], [0, 2, 4, 5], [2, 3, 4, 6]][v];
            shape
                .iter()
                .enumerate()
                .map(|(index, degree)| {
                    note(
                        index as f32 * 0.075,
                        Instrument::Glass,
                        high + step(*degree),
                        if index == 3 { 0.42 } else { 0.3 },
                        -0.3 + 0.2 * index as f32,
                    )
                })
                .collect()
        }
        // Paper: a quick flick, and very faintly a high note.
        Act::Flip => {
            let bright = [4200.0, 3600.0, 5000.0, 3900.0][v];
            vec![
                swish(0.0, bright, bright * 0.7, 0.045, 1.8, 0.9, 0.0),
                note(
                    0.005,
                    Instrument::Kalimba,
                    high + 12.0 + step([0, 2, 4, 1][v]),
                    0.16,
                    0.2,
                ),
            ]
        }
        // A wooden drawer sliding, and a low knock as it stops.
        Act::Drawer => {
            let [from, to] = [
                [380.0, 900.0],
                [320.0, 760.0],
                [420.0, 1000.0],
                [350.0, 820.0],
            ][v];
            vec![
                swish(0.0, from, to, 0.22, 0.4, 1.3, 0.08),
                note(
                    0.2,
                    Instrument::Marimba,
                    root + step([0, 3, 2, -2][v]),
                    0.55,
                    0.0,
                ),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tune::PENTATONIC;

    #[test]
    fn every_act_is_in_the_key_of_the_tune() {
        for root in 48..60 {
            for act in Act::ALL {
                for variation in 0..VARIATIONS {
                    for part in parts(act, variation, root as f32) {
                        if let Part::Note { note, .. } = part {
                            let over = (note.pitch - root as f32).rem_euclid(12.0) as u32;
                            assert!(PENTATONIC.contains(&over), "{act:?} {variation}: {over}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn every_act_has_at_least_three_versions_and_never_repeats_the_last() {
        for act in Act::ALL {
            let versions = (0..VARIATIONS)
                .map(|variation| format!("{:?}", parts(act, variation, 55.0)))
                .collect::<std::collections::BTreeSet<_>>();
            assert!(versions.len() >= 3, "{act:?}");
        }
        let mut dice = Dice::new(3);
        let mut last = None;
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..200 {
            let next = next_variation(last, &mut dice);
            assert_ne!(Some(next), last);
            assert!(next < VARIATIONS);
            seen.insert(next);
            last = Some(next);
        }
        assert_eq!(seen.len(), VARIATIONS as usize, "all are heard in time");
    }
}
