//! A World's music, made here from its colours, the hour and the weather
//! rather than shipped: presentation only, like its sound, and never
//! anything the World records.
//!
//! Every World has three layers in one key. A soft base of chords is always
//! there. A day layer of light plucked notes comes up with the morning and
//! fades by evening; an evening layer, a slow low tune, takes over as the
//! light goes. Each of the day's 24 hours has its own mix, so the music
//! moves through the day the way Animal Crossing's does, and weather thins
//! it: rain takes most of the plucks away, a storm leaves only the chords
//! and a few low notes. A composer's stems can replace the layers later
//! without anything else changing.

use crate::ambience::{Palette, SAMPLE_RATE};

/// How long one loop lasts: four chords of six seconds, which is a whole
/// number of beats at both tempos.
pub const LOOP_SECONDS: u32 = 24;
const CHORD_SECONDS: f32 = 6.0;
/// How long a note may ring past the end of the loop; it is folded back
/// onto the start so the loop joins without a seam.
const TAIL_SECONDS: f32 = 3.0;

/// How wet the sky is, as far as the music cares.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Sky {
    Clear,
    /// Cloud, fog: a little quieter.
    Grey,
    /// Rain, snow, dust: most of the plucks go.
    Wet,
    /// Wind and lightning: only the chords and a few low notes.
    Storm,
}

/// When the music plays: the hour on the player's clock (0 to 23) and the
/// sky over the World.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Moment {
    pub hour: u32,
    pub sky: Sky,
}

/// How loud each layer is at a moment: the base, the day layer and the
/// evening layer, each from 0 to 1.
pub fn layers(moment: Moment) -> [f32; 3] {
    let hour = (moment.hour % 24) as f32;
    // Distance round the clock, so 23 is next to 0.
    let near = |centre: f32, width: f32| {
        let apart = (hour - centre).abs();
        let apart = apart.min(24.0 - apart);
        (1.0 - (apart / width).powi(2)).max(0.0)
    };
    let day = near(13.0, 7.5);
    let evening = near(21.0, 6.0).max(near(1.0, 4.5) * 0.6);
    let (base, day_weather, evening_weather) = match moment.sky {
        Sky::Clear => (1.0, 1.0, 1.0),
        Sky::Grey => (0.9, 0.7, 1.0),
        Sky::Wet => (0.85, 0.35, 0.85),
        Sky::Storm => (0.7, 0.0, 0.4),
    };
    [base, day * day_weather, evening * evening_weather]
}

/// A small deterministic random source.
struct Dice(u32);

impl Dice {
    fn new(seed: u32) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32
    }

    fn pick(&mut self, count: usize) -> usize {
        ((self.next() * count as f32) as usize).min(count - 1)
    }
}

fn midi(note: f32) -> f32 {
    440.0 * 2.0_f32.powf((note - 69.0) / 12.0)
}

fn palette_seed(palette: Palette) -> u32 {
    palette.iter().fold(0x2545_f491_u32, |seed, colour| {
        seed.rotate_left(5) ^ colour.wrapping_mul(0x9e37_79b9)
    })
}

/// The key a palette plays in: a root between C3 and B3 set by the hue of
/// its ground, so a green valley and a red desert are in different keys.
pub fn root(palette: Palette) -> u32 {
    let [_, _, _, near, _] = palette;
    let (r, g, b) = (
        ((near >> 16) & 0xff) as f32,
        ((near >> 8) & 0xff) as f32,
        (near & 0xff) as f32,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let hue = if max - min < f32::EPSILON {
        0.0
    } else if max == r {
        ((g - b) / (max - min)).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / (max - min) + 2.0) / 6.0
    } else {
        ((r - g) / (max - min) + 4.0) / 6.0
    };
    48 + ((hue * 12.0) as u32).min(11)
}

/// Scale steps of the major pentatonic, and the four chords a loop walks
/// through (as scale degrees of the major scale): I, vi, IV, V, or in the
/// evening I, iii, vi, IV, which sits lower and softer.
const PENTATONIC: [u32; 5] = [0, 2, 4, 7, 9];
const DAY_CHORDS: [[u32; 3]; 4] = [[0, 4, 7], [9, 12, 16], [5, 9, 12], [7, 11, 14]];
const EVENING_CHORDS: [[u32; 3]; 4] = [[0, 4, 7], [4, 7, 11], [9, 12, 16], [5, 9, 12]];

/// One note in the loop: when it starts, its pitch, how long it rings,
/// how loud, and its colour.
struct Note {
    start: f32,
    pitch: f32,
    length: f32,
    level: f32,
    voice: Voice,
}

#[derive(Clone, Copy)]
enum Voice {
    /// Slow in, slow out, a soft octave above.
    Pad,
    /// A quick strike dying away, like a kalimba.
    Pluck,
    /// A soft, round tune with a slow vibrato.
    Hum,
}

fn score(palette: Palette, moment: Moment) -> Vec<Note> {
    let [base, day, evening] = layers(moment);
    let root = root(palette) as f32;
    let mut dice = Dice::new(palette_seed(palette) ^ moment.hour.wrapping_mul(0x85eb_ca6b));
    let chords = if evening > day {
        EVENING_CHORDS
    } else {
        DAY_CHORDS
    };
    // Brisk by day, slower in the evening and at night.
    let beat = if day >= 0.5 { 0.75 } else { 1.0 };
    let mut notes = Vec::new();
    for (index, chord) in chords.iter().enumerate() {
        let start = index as f32 * CHORD_SECONDS;
        for (voice, step) in chord.iter().enumerate() {
            notes.push(Note {
                start,
                pitch: root - 12.0 + *step as f32,
                length: CHORD_SECONDS + 1.5,
                level: base * if voice == 0 { 0.34 } else { 0.22 },
                voice: Voice::Pad,
            });
        }
    }
    let beats = (LOOP_SECONDS as f32 / beat) as usize;
    // The day layer: a note on some beats and some half beats, from the
    // pentatonic scale two octaves up, walking rather than leaping.
    if day > 0.02 {
        let mut step = dice.pick(PENTATONIC.len());
        for half in 0..beats * 2 {
            let on_beat = half % 2 == 0;
            let chance = if on_beat { 0.55 } else { 0.2 } * day.sqrt();
            if dice.next() >= chance {
                continue;
            }
            step = (step + PENTATONIC.len() + dice.pick(3) - 1) % PENTATONIC.len();
            let octave = if dice.next() < 0.2 { 36.0 } else { 24.0 };
            notes.push(Note {
                start: half as f32 * beat / 2.0,
                pitch: root + octave + PENTATONIC[step] as f32,
                length: 1.2,
                level: day * if on_beat { 0.5 } else { 0.34 },
                voice: Voice::Pluck,
            });
        }
    }
    // The evening layer: a slow tune an octave up, two beats a note, some
    // held longer, always on a note of the chord under it or next to one.
    if evening > 0.02 {
        let mut at = 0.0;
        while at < LOOP_SECONDS as f32 - 0.01 {
            let length = beat * if dice.next() < 0.35 { 4.0 } else { 2.0 };
            let chord = chords[((at / CHORD_SECONDS) as usize).min(3)];
            if dice.next() < 0.8 * evening.sqrt() {
                let tone = chord[dice.pick(3)] as f32 + if dice.next() < 0.25 { 2.0 } else { 0.0 };
                notes.push(Note {
                    start: at,
                    pitch: root + 12.0 + tone,
                    length: length + 0.6,
                    level: evening * 0.28,
                    voice: Voice::Hum,
                });
            }
            at += length;
        }
    }
    notes
}

/// The music for a World in `palette` at `moment`, as 16-bit samples of a
/// loop `LOOP_SECONDS` long. The same palette and moment always make the
/// same music.
pub fn compose(palette: Palette, moment: Moment) -> Vec<i16> {
    let rate = SAMPLE_RATE as f32;
    let samples = (LOOP_SECONDS * SAMPLE_RATE) as usize;
    let mut mix = vec![0.0_f32; samples + (TAIL_SECONDS * rate) as usize];
    let tau = std::f32::consts::TAU;
    for note in score(palette, moment) {
        let frequency = midi(note.pitch);
        let first = (note.start * rate) as usize;
        let count = ((note.length * rate) as usize).min(mix.len() - first);
        for offset in 0..count {
            let t = offset as f32 / rate;
            let (envelope, wave) = match note.voice {
                Voice::Pad => {
                    let attack = (t / 1.2).min(1.0);
                    let release = ((note.length - t) / 1.5).clamp(0.0, 1.0);
                    let wave = (tau * frequency * t).sin()
                        + 0.25 * (tau * frequency * 2.0 * t).sin()
                        + 0.08 * (tau * frequency * 3.003 * t).sin();
                    (attack * attack * release, wave)
                }
                Voice::Pluck => {
                    let envelope = (t / 0.003).min(1.0) * (-t / 0.28).exp();
                    let wave = (tau * frequency * t).sin()
                        + 0.35 * (-t / 0.08).exp() * (tau * frequency * 3.0 * t).sin()
                        + 0.12 * (tau * frequency * 5.4 * t).sin() * (-t / 0.05).exp();
                    (envelope, wave)
                }
                Voice::Hum => {
                    let attack = (t / 0.25).min(1.0);
                    let release = ((note.length - t) / 0.5).clamp(0.0, 1.0);
                    let vibrato = 1.0 + 0.004 * (tau * 5.0 * t).sin() * (t / 0.8).min(1.0);
                    let phase = tau * frequency * vibrato * t;
                    let wave =
                        phase.sin() + 0.18 * (2.0 * phase).sin() + 0.06 * (3.0 * phase).sin();
                    (attack * release, wave)
                }
            };
            mix[first + offset] += note.level * envelope * wave;
        }
    }
    // Whatever rings past the end comes round to the start.
    let tail = mix.split_off(samples);
    for (index, sample) in tail.into_iter().enumerate() {
        mix[index] += sample;
    }
    let peak = mix
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    let gain = if peak > 0.0 { 0.42 / peak } else { 0.0 };
    mix.into_iter()
        .map(|sample| ((sample * gain).clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

/// A short stable name for the music of a palette at a moment.
pub fn file_name(palette: Palette, moment: Moment) -> String {
    let sky = match moment.sky {
        Sky::Clear => 0,
        Sky::Grey => 1,
        Sky::Wet => 2,
        Sky::Storm => 3,
    };
    let key = palette
        .iter()
        .chain([&(moment.hour % 24), &sky])
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, value| {
            (hash ^ u64::from(*value)).wrapping_mul(0x0100_0000_01b3)
        });
    format!("music-{key:016x}.wav")
}

#[cfg(test)]
mod tests {
    use super::*;

    const HARBOUR: Palette = [0x9fd3ee, 0xf6ecd2, 0x7fa37a, 0x4e7d5b, 0xfff1c9];
    const MARS: Palette = [0xe7b089, 0xf5d9bd, 0xc2663f, 0x8a3a22, 0xfff3dc];

    fn at(hour: u32, sky: Sky) -> Moment {
        Moment { hour, sky }
    }

    /// Loudness of a loop, as a root mean square.
    fn loudness(samples: &[i16]) -> f64 {
        let sum: f64 = samples.iter().map(|s| f64::from(*s).powi(2)).sum();
        (sum / samples.len() as f64).sqrt()
    }

    /// How much of the loop is quick change: the day layer's plucks are
    /// high and short, so they show here and the chords do not.
    fn brightness(samples: &[i16]) -> f64 {
        let change: f64 = samples
            .windows(2)
            .map(|pair| (f64::from(pair[1]) - f64::from(pair[0])).powi(2))
            .sum();
        change / samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>()
    }

    #[test]
    fn every_hour_and_every_sky_has_music_of_its_own() {
        let mut heard = std::collections::BTreeSet::new();
        for hour in 0..24 {
            let music = compose(HARBOUR, at(hour, Sky::Clear));
            assert_eq!(music.len(), (LOOP_SECONDS * SAMPLE_RATE) as usize);
            assert!(heard.insert(music), "hour {hour} sounds like another");
        }
        let noon = compose(HARBOUR, at(12, Sky::Clear));
        for sky in [Sky::Grey, Sky::Wet, Sky::Storm] {
            assert_ne!(noon, compose(HARBOUR, at(12, sky)), "{sky:?}");
            assert_ne!(
                file_name(HARBOUR, at(12, Sky::Clear)),
                file_name(HARBOUR, at(12, sky))
            );
        }
        assert_ne!(noon, compose(MARS, at(12, Sky::Clear)));
        assert_ne!(root(HARBOUR), root(MARS), "two landscapes, two keys");
        assert_eq!(
            noon,
            compose(HARBOUR, at(12, Sky::Clear)),
            "the same every time"
        );
    }

    #[test]
    fn the_day_layer_rises_with_the_morning_and_the_evening_layer_takes_over() {
        let [base, day, evening] = layers(at(13, Sky::Clear));
        assert!(base == 1.0 && day > 0.9 && evening == 0.0);
        let [_, day, evening] = layers(at(21, Sky::Clear));
        assert!(day == 0.0 && evening > 0.9);
        let [_, day, evening] = layers(at(3, Sky::Clear));
        assert!(day == 0.0 && evening < 0.6);
        // Every hour moves at least one layer from the hour before.
        for hour in 0..24 {
            let now = layers(at(hour, Sky::Clear));
            let next = layers(at(hour + 1, Sky::Clear));
            assert!(now != next || now[1] + now[2] < 0.01, "{hour}");
        }
        // Noon's plucks make it brighter than midnight's slow tune.
        let noon = compose(HARBOUR, at(12, Sky::Clear));
        let night = compose(HARBOUR, at(22, Sky::Clear));
        assert!(brightness(&noon) > brightness(&night) * 1.2);
    }

    #[test]
    fn weather_thins_the_music() {
        for hour in [9, 13, 20] {
            let mut last = f32::MAX;
            for sky in [Sky::Clear, Sky::Grey, Sky::Wet, Sky::Storm] {
                let [base, day, evening] = layers(at(hour, sky));
                let total = base + day + evening;
                assert!(total <= last, "{hour} {sky:?}");
                last = total;
            }
        }
        let [_, day, _] = layers(at(12, Sky::Storm));
        assert_eq!(day, 0.0, "no plucks in a storm");
        let clear = compose(HARBOUR, at(12, Sky::Clear));
        let storm = compose(HARBOUR, at(12, Sky::Storm));
        assert!(brightness(&clear) > brightness(&storm));
    }

    #[test]
    fn it_is_quiet_never_silent_and_joins_itself() {
        for moment in [at(8, Sky::Clear), at(15, Sky::Wet), at(23, Sky::Storm)] {
            let music = compose(MARS, moment);
            let peak = music.iter().map(|s| s.unsigned_abs()).max().unwrap();
            assert!(peak < i16::MAX as u16 / 2, "{moment:?}: {peak}");
            assert!(loudness(&music) > 600.0, "{moment:?}");
            for second in music.chunks(SAMPLE_RATE as usize) {
                assert!(second.iter().any(|s| s.unsigned_abs() > 200), "{moment:?}");
            }
            // No jump where the end meets the start.
            let seam = (i32::from(music[0]) - i32::from(*music.last().unwrap())).abs();
            let step = music
                .windows(2)
                .map(|pair| (i32::from(pair[1]) - i32::from(pair[0])).abs())
                .max()
                .unwrap();
            assert!(
                seam <= step * 2,
                "{moment:?}: seam {seam}, largest step {step}"
            );
        }
    }

    /// Writes a few loops and voices to listen to:
    /// `WORLD_MACHINE_SOUNDS=dir cargo test -p world-machine-desktop --lib write_sounds -- --ignored`
    #[test]
    #[ignore]
    fn write_sounds() {
        let Ok(directory) = std::env::var("WORLD_MACHINE_SOUNDS") else {
            return;
        };
        let directory = std::path::Path::new(&directory);
        for (name, moment) in [
            ("08-clear", at(8, Sky::Clear)),
            ("13-clear", at(13, Sky::Clear)),
            ("13-rain", at(13, Sky::Wet)),
            ("20-clear", at(20, Sky::Clear)),
            ("23-storm", at(23, Sky::Storm)),
        ] {
            let bytes = crate::ambience::wav(&compose(HARBOUR, moment));
            std::fs::write(directory.join(format!("music-{name}.wav")), bytes).unwrap();
        }
        let mut voices = Vec::new();
        for (seed, question) in [(3_u32, false), (0x00c0_ffee, true), (0x7f10_2244, false)] {
            voices.extend(crate::ambience::babble(seed, 8, question, 0));
            voices.extend(std::iter::repeat_n(0, SAMPLE_RATE as usize / 3));
        }
        std::fs::write(directory.join("voices.wav"), crate::ambience::wav(&voices)).unwrap();
    }
}
