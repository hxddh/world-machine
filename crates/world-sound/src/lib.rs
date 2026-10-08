//! World Machine's sound, as a pure render graph: a World's landscape, its
//! music following the hour note by note, and a note for every act, mixed
//! in stereo into a buffer. Nothing here touches a sound device or a file;
//! the app feeds the graph to its output, and tests render it into memory.
//!
//! It is presentation only, like the picture: nothing a World records
//! depends on it, and replay never needs it. Everything is made from the
//! World's colours, its hour, its sky and a seed, so the same seed and the
//! same commands always render the same samples.
//!
//! The mix has four channels, each with the player's level: music, the
//! landscape, voices and the interface's acts. [`Engine::apply`] takes a
//! [`Command`]; [`Engine::render`] fills interleaved stereo samples.

#![forbid(unsafe_code)]

mod acts;
mod babble;
mod dsp;
mod landscape;
mod music;
mod tune;
mod voices;
mod wav;

pub use acts::{Act, VARIATIONS};
pub use babble::{babble, motif_of, voice, Voice};
pub use landscape::{beds, daylight, Bed};
pub use music::{layers, Layers};
pub use tune::{motif, pentatonic, phrase, root, Motif, Palette, Tune};
pub use wav::wav;

use dsp::{pan, Dice, Room, Smoother};
use voices::{Bus, Pool, Tone, BUSES};

/// The sample rate tests render at, and the one the app asks for first.
pub const RATE: u32 = 48_000;

/// How many frames the app asks the device for in each callback: 5.3 ms
/// at 48 kHz. An act waits at most one callback to be heard, plus one
/// callback in flight to the device.
pub const BUFFER_FRAMES: u32 = 256;

/// The graph renders in slices this many frames long; the music and the
/// landscape change their settings once a slice.
const SLICE: usize = 16;

/// The sky over a World, as far as its sound cares.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum Sky {
    #[default]
    Clear,
    Cloudy,
    Fog,
    Rain,
    Snow,
    Storm,
    Dust,
}

/// What kind of place a World is, which decides its landscape's beds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    /// A harbour town: the sea, gulls, rain on roofs, wind.
    Harbour,
    /// A colony under a dome on a dusty world: its hum, wind and grit.
    Dome,
    /// A street in a town: cars passing, an arcade, rain, wind.
    Street,
    /// Ice: wind, creaking, penguins, the sea far off.
    Ice,
    /// Anywhere else: wind and a drone at `hum` hertz.
    Open { hum: f32 },
}

impl Place {
    /// An open place with its drone pitched by the hue of its ground, as
    /// every World's sound was before it had a landscape of its own.
    pub fn open(palette: Palette) -> Self {
        Place::Open {
            hum: 55.0 * (1.0 + tune::hue(palette[3])),
        }
    }
}

/// Everything the sound follows about the World in front.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub place: Place,
    pub tune: Tune,
    /// The hour on the player's clock, 0 to 24, with its minutes.
    pub hour: f32,
    pub sky: Sky,
    /// A festival is held today.
    pub festival: bool,
}

/// How loud each channel plays, as a gain: 0 is silent, and 1 is the
/// channel at full.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Levels {
    pub music: f32,
    pub ambience: f32,
    pub voices: f32,
    pub interface: f32,
}

impl Levels {
    fn gains(self) -> [f32; 4] {
        [self.music, self.ambience, self.voices, self.interface]
    }
}

/// What the app tells the graph.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// The World in front, or none: the music and landscape fade to it.
    Scene(Option<Scene>),
    /// The player's levels.
    Levels(Levels),
    /// Something the player did, heard at once.
    Act(Act),
    /// Something heard a moment from now, so it does not sound on top of
    /// the act that caused it.
    ActLater { act: Act, seconds: f32 },
    /// Someone speaking: mono samples at the graph's rate, placed at
    /// `place` (-1 left to 1 right), on the voices channel.
    Say { samples: Vec<f32>, place: f32 },
}

/// Someone's babble, playing.
struct Said {
    samples: Vec<f32>,
    at: usize,
    left: f32,
    right: f32,
}

/// The whole mix.
pub struct Engine {
    rate: f32,
    clock: u64,
    dice: Dice,
    seed: u64,
    notes: u64,
    scene: Option<Scene>,
    channels: [Smoother; 4],
    music: music::Music,
    land: landscape::Landscape,
    pool: Pool,
    said: Vec<Said>,
    room: Room,
    later: Vec<(u64, Act)>,
    parts: Vec<(u64, acts::Part)>,
    last: [Option<u8>; Act::ALL.len()],
    buses: [[Vec<f32>; 2]; BUSES],
    ambience: [Vec<f32>; 2],
    voices: [Vec<f32>; 2],
    limit: f32,
    limit_release: f32,
}

/// How each channel is trimmed so that at a gain of 1 it is as loud as its
/// sound was when it was played from files: music peaking near 0.4, the
/// landscape near 0.35, an act near 0.35, and a babble as it was, which
/// was mono and so is lifted by the pan's 3 dB here.
const TRIM: [f32; 4] = [0.45, 0.35, std::f32::consts::SQRT_2, 0.6];

/// A few milliseconds in and out of a babble, whatever its first and last
/// samples, so it can never click.
const SAY_FADE_SECONDS: f32 = 0.003;

impl Engine {
    /// A silent graph at `rate` samples a second, its choices seeded by
    /// `seed`. Nothing sounds until it has levels, and a scene or an act.
    pub fn new(rate: f32, seed: u64) -> Self {
        let control = rate / SLICE as f32;
        let slice = || [vec![0.0; SLICE], vec![0.0; SLICE]];
        Self {
            rate,
            clock: 0,
            dice: Dice::new(seed ^ 0x6163_7473),
            seed,
            notes: 0,
            scene: None,
            channels: std::array::from_fn(|_| Smoother::new(0.0, 0.03, control)),
            music: music::Music::new(rate, seed, control),
            land: landscape::Landscape::new(rate, seed, control),
            pool: Pool::new(40),
            said: Vec::with_capacity(8),
            room: Room::new(rate),
            later: Vec::with_capacity(16),
            parts: Vec::with_capacity(64),
            last: [None; Act::ALL.len()],
            buses: std::array::from_fn(|_| slice()),
            ambience: slice(),
            voices: slice(),
            limit: 1.0,
            limit_release: 1.0 - dsp::decay(0.25, rate),
        }
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    /// How many frames have been rendered.
    pub fn clock(&self) -> u64 {
        self.clock
    }

    pub fn apply(&mut self, command: Command) {
        match command {
            Command::Scene(scene) => {
                self.music.set_scene(scene.as_ref(), self.clock);
                self.land.set_scene(scene.as_ref());
                self.scene = scene;
            }
            Command::Levels(levels) => {
                for (channel, gain) in self.channels.iter_mut().zip(levels.gains()) {
                    channel.target = gain.clamp(0.0, 2.0);
                }
            }
            Command::Act(act) => self.act(act),
            Command::ActLater { act, seconds } => {
                if self.later.len() < self.later.capacity() {
                    let at = self.clock + (seconds.max(0.0) * self.rate) as u64;
                    self.later.push((at, act));
                }
            }
            Command::Say { samples, place } => {
                // Eight people at once is a crowd; a ninth waits its turn
                // rather than cutting anyone off.
                if self.said.len() == self.said.capacity() {
                    return;
                }
                let (left, right) = pan(place);
                self.said.push(Said {
                    samples,
                    at: 0,
                    left,
                    right,
                });
            }
        }
    }

    fn act(&mut self, act: Act) {
        let slot = &mut self.last[act.index()];
        let variation = acts::next_variation(*slot, &mut self.dice);
        *slot = Some(variation);
        let root = self
            .scene
            .as_ref()
            .map_or(55.0, |scene| scene.tune.root as f32);
        for part in acts::parts(act, variation, root) {
            let at = self.clock + (part.delay() * self.rate) as u64;
            if part.delay() <= 0.0 {
                self.start(part);
            } else if self.parts.len() < self.parts.capacity() {
                self.parts.push((at, part));
            }
        }
    }

    fn start(&mut self, part: acts::Part) {
        self.notes = self.notes.wrapping_add(1);
        let seed = self.seed ^ self.notes.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        let tone = match part {
            acts::Part::Note { note, .. } => Tone::note(note, self.rate, seed),
            acts::Part::Swish { swish, place, .. } => {
                Tone::swish(swish, place, Bus::Act, self.rate, seed)
            }
        };
        self.pool.start(tone, self.rate);
    }

    /// Fills `out` with interleaved stereo samples, left then right.
    pub fn render(&mut self, out: &mut [f32]) {
        for frames in out.chunks_mut(SLICE * 2) {
            self.render_slice(frames);
        }
    }

    /// Renders `seconds` of sound, interleaved stereo.
    pub fn render_seconds(&mut self, seconds: f32) -> Vec<f32> {
        let mut out = vec![0.0; (seconds * self.rate) as usize * 2];
        self.render(&mut out);
        out
    }

    fn render_slice(&mut self, out: &mut [f32]) {
        let frames = out.len() / 2;
        let until = self.clock + frames as u64;

        // What is due in this slice starts at its first sample.
        let mut index = 0;
        while index < self.later.len() {
            if self.later[index].0 < until {
                let (_, act) = self.later.swap_remove(index);
                self.act(act);
            } else {
                index += 1;
            }
        }
        let mut index = 0;
        while index < self.parts.len() {
            if self.parts[index].0 < until {
                let (_, part) = self.parts.swap_remove(index);
                self.start(part);
            } else {
                index += 1;
            }
        }
        let (rate, seed) = (self.rate, self.seed);
        let (pool, notes) = (&mut self.pool, &mut self.notes);
        self.music.advance(until, |note| {
            *notes = notes.wrapping_add(1);
            pool.start(
                Tone::note(note, rate, seed ^ notes.wrapping_mul(0x9e37_79b9_7f4a_7c15)),
                rate,
            );
        });

        for side in self
            .buses
            .iter_mut()
            .flatten()
            .chain(self.ambience.iter_mut())
            .chain(self.voices.iter_mut())
        {
            side[..frames].fill(0.0);
        }
        self.pool.render(&mut self.buses, frames);
        {
            let [left, right] = &mut self.ambience;
            self.land.render(&mut left[..frames], &mut right[..frames]);
        }
        let fade = (SAY_FADE_SECONDS * self.rate).max(1.0);
        let [voice_left, voice_right] = &mut self.voices;
        self.said.retain_mut(|said| {
            let length = said.samples.len();
            for frame in 0..frames {
                if said.at >= length {
                    return false;
                }
                let edge = (said.at.min(length - 1 - said.at) as f32 / fade).min(1.0);
                let value = said.samples[said.at] * edge;
                voice_left[frame] += value * said.left;
                voice_right[frame] += value * said.right;
                said.at += 1;
            }
            said.at < length
        });

        let [music, ambience, voices, interface] = {
            let gains = self.channels.each_mut().map(Smoother::step);
            std::array::from_fn::<f32, 4, _>(|index| gains[index] * TRIM[index])
        };
        let layers = self.music.gains.each_mut().map(Smoother::step);
        for frame in 0..frames {
            let mut dry = [0.0_f32; 2];
            let mut send = [0.0_f32; 2];
            for side in 0..2 {
                let tune = self.buses[Bus::Pad as usize][side][frame] * layers[0]
                    + self.buses[Bus::Day as usize][side][frame] * layers[1]
                    + self.buses[Bus::Evening as usize][side][frame] * layers[2]
                    + self.buses[Bus::Festival as usize][side][frame] * layers[3];
                let tune = tune * music;
                let place = self.ambience[side][frame] * ambience;
                let voice = self.voices[side][frame] * voices;
                let act = self.buses[Bus::Act as usize][side][frame] * interface;
                dry[side] = tune + place + voice + act;
                send[side] = tune * 0.45 + place * 0.08 + voice * 0.12 + act * 0.3;
            }
            let (wet_left, wet_right) = self.room.run(send[0], send[1]);
            let left = dry[0] + wet_left * 0.5;
            let right = dry[1] + wet_right * 0.5;
            // A gentle limiter: instant down, slow back up, so nothing
            // ever reaches full scale.
            let peak = left.abs().max(right.abs()) * self.limit;
            if peak > 0.9 {
                self.limit *= 0.9 / peak;
            } else {
                self.limit += self.limit_release * (1.0 - self.limit);
            }
            out[frame * 2] = (left * self.limit).clamp(-1.0, 1.0);
            out[frame * 2 + 1] = (right * self.limit).clamp(-1.0, 1.0);
        }
        self.clock = until;
    }

    /// How many notes and swishes are sounding.
    pub fn sounding(&self) -> usize {
        self.pool.len()
    }

    /// How loud each landscape bed plays now, in [`Bed::ALL`] order.
    pub fn bed_levels(&self) -> [f32; Bed::ALL.len()] {
        self.land.levels()
    }

    /// How loud each music layer plays now: chords, day, evening, festival.
    pub fn music_levels(&self) -> [f32; 4] {
        self.music.gains.each_ref().map(|gain| gain.value)
    }

    /// How many notes of the day layer are sounding.
    pub fn day_notes(&self) -> usize {
        self.pool.sounding(Bus::Day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_graph_is_silent() {
        let mut engine = Engine::new(RATE as f32, 1);
        // Far below anything a device can play: the filters' idle hum.
        assert!(engine
            .render_seconds(0.5)
            .iter()
            .all(|sample| sample.abs() < 1e-9));
    }
}
