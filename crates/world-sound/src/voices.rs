//! The instruments: every note is a few decaying partials (a modal model,
//! like a struck bar or a bell) with, for some, a breath of filtered noise.
//! They cost a few multiplications a sample, are exactly in tune, and die
//! away by themselves the way a real one does.

use crate::dsp::{advance, decay, midi, pan, sine, Dice, Svf};

/// Where a note is mixed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Bus {
    /// The music's soft chords.
    Pad = 0,
    /// The music's day layer: plucked notes.
    Day = 1,
    /// The music's evening layer: a hummed tune.
    Evening = 2,
    /// Festival bells and taps.
    Festival = 3,
    /// The note an act makes.
    Act = 4,
}

pub(crate) const BUSES: usize = 5;

/// One partial: a sine with its own level and decay.
#[derive(Clone, Copy, Debug, Default)]
struct Partial {
    phase: f32,
    step: f32,
    level: f32,
    decay: f32,
}

/// A breath of band-passed noise whose centre glides.
#[derive(Clone, Debug)]
struct Breath {
    level: f32,
    decay: f32,
    /// `π·cutoff/rate`, and how much it is multiplied by each sample.
    g: f32,
    glide: f32,
    /// Samples left to glide.
    gliding: u32,
    q: f32,
    filter: Svf,
    /// Samples before the breath swells to full, then falls away.
    swell: u32,
}

/// The instruments a note can be played on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Instrument {
    /// A kalimba: a soft tine, quick to speak, ringing a while.
    Kalimba,
    /// A small bell: inharmonic partials, a long ring.
    Bell,
    /// Glass: a high clear ring for something precious.
    Glass,
    /// A wooden bar, struck: short and round, a "plop".
    Marimba,
    /// Soft held chords: slow to come and slow to go.
    Pad,
    /// A hummed tune, with a slow vibrato.
    Hum,
    /// A soft brushed tap: noise only.
    Brush,
}

/// Something with a sound of its own that is not a note: paper, a drawer,
/// a thump.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Swish {
    /// Where its noise is centred as it starts and as it ends, in hertz.
    pub(crate) from: f32,
    pub(crate) to: f32,
    pub(crate) seconds: f32,
    pub(crate) level: f32,
    pub(crate) q: f32,
    /// How long it takes to swell, as a share of its length.
    pub(crate) swell: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct Tone {
    partials: [Partial; 4],
    count: usize,
    breath: Option<Breath>,
    age: u32,
    attack: u32,
    /// When the note is let go, in samples from its start.
    hold: u32,
    release: u32,
    left: f32,
    right: f32,
    pub(crate) bus: Bus,
    vibrato_phase: f32,
    vibrato_step: f32,
    vibrato_depth: f32,
    /// A small random source for the breath.
    dice: Dice,
}

/// An instrument's partials (ratio, level, ring), its attack and release
/// in seconds, and how long it holds, if it holds.
type Shape = (&'static [(f32, f32, f32)], f32, f32, Option<f32>);

/// Below this a partial is inaudible (about -70 dB) and is let go.
const QUIET: f32 = 3e-4;

/// The shortest fade any note may start or stop with: short enough to
/// keep a strike crisp, long enough never to click.
pub(crate) const FADE_SECONDS: f32 = 0.002;

/// A note to play: on `instrument` at MIDI `pitch`, `level` loud, placed
/// at `place` (-1 left to 1 right), held for `length` seconds where the
/// instrument holds, mixed into `bus`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Note {
    pub(crate) instrument: Instrument,
    pub(crate) pitch: f32,
    pub(crate) level: f32,
    pub(crate) length: f32,
    pub(crate) place: f32,
    pub(crate) bus: Bus,
}

impl Tone {
    pub(crate) fn note(note: Note, rate: f32, seed: u64) -> Self {
        let Note {
            instrument,
            pitch,
            level,
            length,
            place,
            bus,
        } = note;
        let frequency = midi(pitch);
        // Each partial: its ratio to the fundamental, level, and how long
        // it rings (to 1/e). Upper partials above the audible top are left
        // out, and so are ones that would fold back.
        let (shape, attack, release, hold): Shape = match instrument {
            Instrument::Kalimba => (
                &[(1.0, 1.0, 0.9), (3.0, 0.16, 0.16), (5.4, 0.07, 0.05)],
                0.003,
                0.0,
                None,
            ),
            Instrument::Bell => (
                &[
                    (1.0, 1.0, 1.5),
                    (2.76, 0.38, 0.6),
                    (5.40, 0.16, 0.25),
                    (8.93, 0.06, 0.1),
                ],
                0.002,
                0.0,
                None,
            ),
            Instrument::Glass => (
                &[(1.0, 1.0, 1.1), (2.0, 0.22, 0.5), (4.07, 0.1, 0.18)],
                0.002,
                0.0,
                None,
            ),
            Instrument::Marimba => (
                &[(1.0, 1.0, 0.32), (3.93, 0.24, 0.06), (9.24, 0.05, 0.02)],
                0.002,
                0.0,
                None,
            ),
            Instrument::Pad => (
                &[(1.0, 1.0, 0.0), (1.0035, 0.55, 0.0), (2.0, 0.2, 0.0)],
                1.2,
                1.6,
                Some(length),
            ),
            Instrument::Hum => (
                &[(1.0, 1.0, 0.0), (2.0, 0.18, 0.0), (3.0, 0.06, 0.0)],
                0.25,
                0.5,
                Some(length),
            ),
            Instrument::Brush => (&[], 0.001, 0.0, None),
        };
        let mut partials = [Partial::default(); 4];
        let mut count = 0;
        let mut total = 0.0;
        for (ratio, weight, ring) in shape {
            let hertz = frequency * ratio;
            if hertz > rate * 0.45 || count == partials.len() {
                continue;
            }
            partials[count] = Partial {
                phase: 0.0,
                step: hertz / rate,
                level: *weight,
                decay: if *ring > 0.0 { decay(*ring, rate) } else { 1.0 },
            };
            total += weight;
            count += 1;
        }
        // Loudness is the note's, not the number of its partials'.
        for partial in &mut partials[..count] {
            partial.level *= level / total.max(1.0);
        }
        let breath = match instrument {
            // A kalimba's thumb and a mallet's wood.
            Instrument::Kalimba => Some(Swish {
                from: (frequency * 4.0).min(6000.0),
                to: (frequency * 3.0).min(5000.0),
                seconds: 0.012,
                level: level * 0.25,
                q: 1.2,
                swell: 0.0,
            }),
            Instrument::Marimba => Some(Swish {
                from: (frequency * 2.0).min(3000.0),
                to: frequency,
                seconds: 0.02,
                level: level * 0.35,
                q: 0.9,
                swell: 0.0,
            }),
            Instrument::Brush => Some(Swish {
                from: 5200.0,
                to: 3800.0,
                seconds: 0.035,
                level,
                q: 0.7,
                swell: 0.0,
            }),
            _ => None,
        };
        let mut tone = Self {
            partials,
            count,
            breath: None,
            age: 0,
            attack: ((attack.max(FADE_SECONDS)) * rate) as u32,
            hold: hold.map_or(u32::MAX, |seconds| (seconds * rate) as u32),
            release: ((release.max(FADE_SECONDS)) * rate) as u32,
            left: 0.0,
            right: 0.0,
            bus,
            vibrato_phase: 0.0,
            vibrato_step: if instrument == Instrument::Hum {
                5.0 / rate
            } else {
                0.0
            },
            vibrato_depth: if instrument == Instrument::Hum {
                0.004
            } else {
                0.0
            },
            dice: Dice::new(seed),
        };
        (tone.left, tone.right) = pan(place);
        if let Some(swish) = breath {
            tone.breath = Some(Breath::of(swish, rate));
        }
        tone
    }

    /// A swish of noise and nothing else.
    pub(crate) fn swish(swish: Swish, place: f32, bus: Bus, rate: f32, seed: u64) -> Self {
        let mut tone = Self {
            partials: [Partial::default(); 4],
            count: 0,
            breath: Some(Breath::of(swish, rate)),
            age: 0,
            attack: (FADE_SECONDS * rate) as u32,
            hold: u32::MAX,
            release: (FADE_SECONDS * rate) as u32,
            left: 0.0,
            right: 0.0,
            bus,
            vibrato_phase: 0.0,
            vibrato_step: 0.0,
            vibrato_depth: 0.0,
            dice: Dice::new(seed),
        };
        (tone.left, tone.right) = pan(place);
        tone
    }

    /// Let go now, fading in `seconds`: for a note that must make room.
    pub(crate) fn let_go(&mut self, seconds: f32, rate: f32) {
        let release = (seconds.max(FADE_SECONDS) * rate) as u32;
        if self.age < self.hold || self.release > release {
            self.hold = self.age.min(self.hold);
            self.release = release;
        }
    }

    /// How loud it is now, roughly: to choose which note makes room.
    pub(crate) fn loudness(&self) -> f32 {
        let partials: f32 = self.partials[..self.count]
            .iter()
            .map(|partial| partial.level.abs())
            .sum();
        partials + self.breath.as_ref().map_or(0.0, |breath| breath.level)
    }

    /// Adds `left.len()` samples of the note into the two sides; false
    /// once it has died away.
    #[inline]
    pub(crate) fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> bool {
        let attack = self.attack.max(1) as f32;
        let release = self.release.max(1) as f32;
        for (out_left, out_right) in left.iter_mut().zip(right.iter_mut()) {
            let mut envelope = if self.age < self.attack {
                // An eased rise: gentle into the note, never a corner.
                let x = self.age as f32 / attack;
                x * x * (3.0 - 2.0 * x)
            } else {
                1.0
            };
            if self.age >= self.hold {
                let gone = (self.age - self.hold) as f32 / release;
                if gone >= 1.0 {
                    return false;
                }
                envelope *= 1.0 - gone;
            }
            let bend = if self.vibrato_depth > 0.0 {
                advance(&mut self.vibrato_phase, self.vibrato_step);
                let arrive = (self.age as f32 / (attack * 3.0)).min(1.0);
                1.0 + self.vibrato_depth * arrive * sine(self.vibrato_phase)
            } else {
                1.0
            };
            let mut sample = 0.0;
            for partial in &mut self.partials[..self.count] {
                sample += partial.level * sine(partial.phase);
                advance(&mut partial.phase, partial.step * bend);
                partial.level *= partial.decay;
            }
            if let Some(breath) = self.breath.as_mut() {
                sample += breath.next(self.age, &mut self.dice);
            }
            sample *= envelope;
            *out_left += sample * self.left;
            *out_right += sample * self.right;
            self.age = self.age.saturating_add(1);
        }
        // Partials and a breath that have died away are let go, before
        // they sink into subnormal floats and cost far more than they
        // are worth.
        let mut index = 0;
        while index < self.count {
            if self.partials[index].level.abs() < QUIET {
                self.count -= 1;
                self.partials.swap(index, self.count);
            } else {
                index += 1;
            }
        }
        if self
            .breath
            .as_ref()
            .is_some_and(|breath| breath.level < QUIET && self.age > breath.swell)
        {
            self.breath = None;
        }
        // A struck note ends when it has died away of itself. A held one
        // ends when its release is done, above.
        !(self.count == 0 && self.breath.is_none() && self.hold == u32::MAX)
    }
}

impl Breath {
    fn of(swish: Swish, rate: f32) -> Self {
        let samples = (swish.seconds.max(0.001) * rate).max(1.0);
        let g = std::f32::consts::PI * swish.from.clamp(20.0, rate * 0.4) / rate;
        let end = std::f32::consts::PI * swish.to.clamp(20.0, rate * 0.4) / rate;
        let swell = (swish.swell.clamp(0.0, 0.9) * samples) as u32;
        Self {
            // Swelling breaths start from silence and rise; the rest
            // start full and fall.
            level: swish.level,
            decay: decay(swish.seconds * (1.0 - swish.swell) * 0.2, rate),
            g,
            glide: (end / g).powf(1.0 / samples),
            gliding: samples as u32,
            q: swish.q,
            filter: Svf::default(),
            swell,
        }
    }

    #[inline]
    fn next(&mut self, age: u32, dice: &mut Dice) -> f32 {
        self.filter.set_g(self.g, self.q);
        if self.gliding > 0 {
            self.g *= self.glide;
            self.gliding -= 1;
        }
        let noise = dice.signed();
        let band = self.filter.band(noise);
        let shape = if age < self.swell {
            let x = age as f32 / self.swell.max(1) as f32;
            x * x
        } else {
            let level = self.level;
            self.level *= self.decay;
            return band * level;
        };
        band * self.level * shape
    }
}

/// The notes sounding now, a fixed number at most: the quietest makes room
/// for a new one, fading quickly rather than stopping.
pub(crate) struct Pool {
    tones: Vec<Tone>,
    capacity: usize,
}

impl Pool {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            tones: Vec::with_capacity(capacity + 8),
            capacity,
        }
    }

    pub(crate) fn start(&mut self, tone: Tone, rate: f32) {
        if self.tones.len() >= self.capacity {
            // The quietest note, not already fading, fades out fast; the
            // new one comes in beside it.
            if let Some(quietest) = self
                .tones
                .iter_mut()
                .filter(|tone| tone.hold == u32::MAX || tone.age < tone.hold)
                .min_by(|a, b| a.loudness().total_cmp(&b.loudness()))
            {
                quietest.let_go(0.008, rate);
            }
            if self.tones.len() >= self.tones.capacity() {
                return;
            }
        }
        self.tones.push(tone);
    }

    /// Adds `frames` of every note into its bus.
    pub(crate) fn render(&mut self, buses: &mut [[Vec<f32>; 2]; BUSES], frames: usize) {
        self.tones.retain_mut(|tone| {
            let [left, right] = &mut buses[tone.bus as usize];
            tone.render(&mut left[..frames], &mut right[..frames])
        });
    }

    pub(crate) fn len(&self) -> usize {
        self.tones.len()
    }

    pub(crate) fn sounding(&self, bus: Bus) -> usize {
        self.tones.iter().filter(|tone| tone.bus == bus).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    fn note(instrument: Instrument, pitch: f32, length: f32) -> Note {
        Note {
            instrument,
            pitch,
            level: 0.5,
            length,
            place: 0.0,
            bus: Bus::Act,
        }
    }

    fn render(mut tone: Tone, seconds: f32) -> Vec<f32> {
        let count = (seconds * RATE) as usize;
        let mut left = vec![0.0; count];
        let mut right = vec![0.0; count];
        tone.render(&mut left, &mut right);
        left.iter().zip(&right).map(|(l, r)| l + r).collect()
    }

    #[test]
    fn every_instrument_starts_softly_and_dies_away() {
        for instrument in [
            Instrument::Kalimba,
            Instrument::Bell,
            Instrument::Glass,
            Instrument::Marimba,
            Instrument::Pad,
            Instrument::Hum,
            Instrument::Brush,
        ] {
            let tone = Tone::note(note(instrument, 72.0, 1.0), RATE, 1);
            let samples = render(tone, 9.0);
            assert!(samples[0].abs() < 1e-3, "{instrument:?} starts at nothing");
            let peak = samples.iter().fold(0.0_f32, |peak, s| peak.max(s.abs()));
            assert!(peak > 0.05 && peak < 1.0, "{instrument:?}: {peak}");
            let tail = samples[samples.len() - 4800..]
                .iter()
                .fold(0.0_f32, |peak, s| peak.max(s.abs()));
            assert!(tail < peak * 0.01, "{instrument:?} dies away: {tail}");
            // A brush is noise, all quick change; the rest are tones.
            if instrument != Instrument::Brush {
                let jump = samples
                    .windows(2)
                    .fold(0.0_f32, |jump, pair| jump.max((pair[1] - pair[0]).abs()));
                assert!(jump < 0.12, "{instrument:?} never clicks: {jump}");
            }
        }
    }

    #[test]
    fn a_note_is_in_tune() {
        // Count rising zero crossings of a pad's A4 over a second.
        let tone = Tone::note(note(Instrument::Hum, 69.0, 3.0), RATE, 1);
        let samples = render(tone, 2.0);
        let second = &samples[RATE as usize / 2..RATE as usize * 3 / 2];
        let crossings = second
            .windows(2)
            .filter(|pair| pair[0] < 0.0 && pair[1] >= 0.0)
            .count();
        assert!((438..=442).contains(&crossings), "{crossings}");
    }

    #[test]
    fn a_full_pool_makes_room_without_a_click() {
        let mut pool = Pool::new(4);
        let mut buses: [[Vec<f32>; 2]; BUSES] =
            std::array::from_fn(|_| [vec![0.0; 64], vec![0.0; 64]]);
        for index in 0..12 {
            pool.start(
                Tone::note(note(Instrument::Pad, 60.0 + index as f32, 5.0), RATE, 1),
                RATE,
            );
            for bus in buses.iter_mut() {
                bus[0].fill(0.0);
                bus[1].fill(0.0);
            }
            pool.render(&mut buses, 64);
        }
        assert!(pool.len() <= 12);
    }
}
