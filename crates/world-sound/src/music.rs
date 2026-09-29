//! The music, note by note: the World's motif in phrases over four chords,
//! with rests between, in three layers that follow the hour and the sky.
//!
//! A soft base of chords is always there. A day layer of plucked notes comes
//! up with the morning and fades by evening; an evening layer, the motif
//! hummed slowly, takes over as the light goes. Weather thins it: rain takes
//! most of the plucks away, a storm leaves only the chords and a few low
//! notes. A festival day brings bells and a quicker step.
//!
//! Every phrase is the motif stated, answered a step higher, turned about
//! and stated again, then a breath, so a player comes to know their World's
//! tune. What changes from phrase to phrase is chosen by rule: which notes
//! rest, which passing notes join, the octave, a phrase lifted a step, a
//! rhythm turned about, a breath of one bar or two. Notes lean a little
//! early or late together, and downbeats land heavier, the way a player
//! would play them.
//!
//! Each layer's loudness glides to where the hour and the sky put it in
//! about a second, and each beat decides its notes from them afresh, so a
//! change of hour or weather is heard within two seconds.

use crate::dsp::{Dice, Smoother};
use crate::tune::{pentatonic, phrase_steps, Tune};
use crate::voices::{Bus, Instrument, Note};
use crate::{Scene, Sky};

/// How loud each layer is: the chords, the day's plucks, the evening's
/// hum, and a festival's bells, each from 0 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layers {
    pub base: f32,
    pub day: f32,
    pub evening: f32,
    pub festival: f32,
}

/// The layers at `hour` (0 to 24, with its minutes) under `sky`.
pub fn layers(hour: f32, sky: Sky, festival: bool) -> Layers {
    let hour = hour.rem_euclid(24.0);
    // Distance round the clock, so 23 is next to 0.
    let near = |centre: f32, width: f32| {
        let apart = (hour - centre).abs();
        let apart = apart.min(24.0 - apart);
        (1.0 - (apart / width).powi(2)).max(0.0)
    };
    let day = near(13.0, 7.5);
    let evening = near(21.0, 6.0).max(near(1.0, 4.5) * 0.6);
    let (base, day_weather, evening_weather) = match sky {
        Sky::Clear => (1.0, 1.0, 1.0),
        Sky::Cloudy | Sky::Fog => (0.9, 0.7, 1.0),
        Sky::Rain | Sky::Snow | Sky::Dust => (0.85, 0.35, 0.85),
        Sky::Storm => (0.7, 0.0, 0.4),
    };
    let day = day * day_weather;
    let evening = evening * evening_weather;
    if festival {
        Layers {
            base,
            day: day.max(0.8 * day_weather.max(0.35)),
            evening: 0.0,
            festival: 1.0,
        }
    } else {
        Layers {
            base,
            day,
            evening,
            festival: 0.0,
        }
    }
}

/// The four chords a phrase walks through, as degrees of the major scale:
/// I, vi, IV, V by day, and I, iii, vi, IV in the evening, which sits
/// lower and softer.
const DAY_CHORDS: [[u32; 3]; 4] = [[0, 4, 7], [9, 12, 16], [5, 9, 12], [7, 11, 14]];
const EVENING_CHORDS: [[u32; 3]; 4] = [[0, 4, 7], [4, 7, 11], [9, 12, 16], [5, 9, 12]];

const PHRASE_BARS: u32 = 4;
const BEATS: u32 = 4;

/// A note in a bar's line: when it starts in beats from the bar's start,
/// its pentatonic step, and how many beats it lasts.
#[derive(Clone, Copy, Debug)]
struct LineNote {
    onset: f32,
    step: i32,
    beats: f32,
}

pub(crate) struct Music {
    rate: f32,
    dice: Dice,
    tune: Option<Tune>,
    /// A new tune waits for the next bar.
    fresh: bool,
    next_beat: u64,
    beat_samples: u64,
    beat: u32,
    bar: u32,
    breath: u32,
    phrases: u32,
    chords: [[u32; 3]; 4],
    octave: f32,
    lift: i32,
    /// A bar whose rhythm is turned about in this phrase.
    swap: Option<u32>,
    day_line: Vec<LineNote>,
    hum_line: Vec<LineNote>,
    /// Correlated timing: notes lean early or late together.
    walk: f32,
    queue: Vec<(u64, Note)>,
    /// Loudness of the chords, the day, the evening and the festival, at
    /// the control rate.
    pub(crate) gains: [Smoother; 4],
    pub(crate) layers: Layers,
}

impl Music {
    pub(crate) fn new(rate: f32, seed: u64, control_rate: f32) -> Self {
        Self {
            rate,
            dice: Dice::new(seed ^ 0x006d_7573_6963),
            tune: None,
            fresh: false,
            next_beat: 0,
            beat_samples: rate as u64,
            beat: 0,
            bar: 0,
            breath: 1,
            phrases: 0,
            chords: DAY_CHORDS,
            octave: 24.0,
            lift: 0,
            swap: None,
            day_line: Vec::with_capacity(16),
            hum_line: Vec::with_capacity(16),
            walk: 0.0,
            queue: Vec::with_capacity(128),
            gains: std::array::from_fn(|_| Smoother::new(0.0, 0.35, control_rate)),
            layers: Layers::default(),
        }
    }

    /// The scene now: a new tune starts a new phrase at the next bar; the
    /// hour and sky reach the layers at once.
    pub(crate) fn set_scene(&mut self, scene: Option<&Scene>, now: u64) {
        match scene {
            Some(scene) => {
                self.layers = layers(scene.hour, scene.sky, scene.festival);
                if self.tune.as_ref() != Some(&scene.tune) {
                    let starting = self.tune.is_none();
                    self.tune = Some(scene.tune.clone());
                    self.fresh = true;
                    if starting {
                        // Nothing playing: begin on the next beat.
                        self.next_beat = now;
                        self.beat = 0;
                        self.bar = 0;
                    }
                }
            }
            None => {
                self.tune = None;
                self.layers = Layers::default();
                self.queue.clear();
            }
        }
        let targets = [
            self.layers.base,
            self.layers.day,
            self.layers.evening,
            self.layers.festival,
        ];
        for (gain, target) in self.gains.iter_mut().zip(targets) {
            gain.target = target;
        }
    }

    /// Plans every beat that begins before `until`, and hands `start` each
    /// note due to begin before it.
    pub(crate) fn advance(&mut self, until: u64, mut start: impl FnMut(Note)) {
        while self.tune.is_some() && self.next_beat < until {
            let at = self.next_beat;
            self.on_beat(at);
        }
        let mut index = 0;
        while index < self.queue.len() {
            if self.queue[index].0 < until {
                start(self.queue.swap_remove(index).1);
            } else {
                index += 1;
            }
        }
    }

    fn on_beat(&mut self, at: u64) {
        if self.beat == 0 {
            if self.fresh {
                self.fresh = false;
                self.bar = 0;
            }
            if self.bar == 0 {
                self.start_phrase();
            }
            self.start_bar();
        }
        self.play_beat(at);
        self.beat += 1;
        if self.beat == BEATS {
            self.beat = 0;
            self.bar += 1;
            if self.bar >= PHRASE_BARS + self.breath {
                self.bar = 0;
            }
        }
        self.next_beat = at + self.beat_samples;
    }

    fn start_phrase(&mut self) {
        let layers = self.layers;
        self.chords = if layers.evening > layers.day && layers.festival == 0.0 {
            EVENING_CHORDS
        } else {
            DAY_CHORDS
        };
        // The third phrase in four is lifted a step: a sequence, as a
        // tune is sung again higher.
        self.lift = i32::from(self.phrases % 4 == 2);
        self.octave = if layers.day > layers.evening && self.dice.chance(0.3) {
            36.0
        } else {
            24.0
        };
        self.swap = if self.dice.chance(0.35) {
            Some(1 + self.dice.pick(2) as u32)
        } else {
            None
        };
        // At night the rests are longer.
        self.breath = if layers.day < 0.1 || self.dice.chance(0.4) {
            2
        } else {
            1
        };
        self.phrases = self.phrases.wrapping_add(1);
    }

    fn start_bar(&mut self) {
        let layers = self.layers;
        let seconds = if layers.festival > 0.0 {
            0.6
        } else if layers.day >= 0.5 {
            0.75
        } else {
            1.0
        };
        self.beat_samples = (seconds * self.rate) as u64;
        self.day_line.clear();
        self.hum_line.clear();
        let Some(tune) = self.tune.as_ref() else {
            return;
        };
        if self.bar >= PHRASE_BARS {
            return;
        }
        let mut notes = phrase_steps(&tune.motif, self.bar as usize);
        if self.swap == Some(self.bar) && notes.len() > 2 {
            // Two notes' lengths change places; the tune's shape stays.
            let at = 1 + self.dice.pick(notes.len() - 2);
            let (first, second) = (notes[at - 1].1, notes[at].1);
            notes[at - 1].1 = second;
            notes[at].1 = first;
        }
        let mut onset = 0.0;
        for (step, beats) in &notes {
            self.day_line.push(LineNote {
                onset,
                step: step + self.lift,
                beats: *beats,
            });
            onset += beats;
        }
        // The evening hums the motif at half speed over the first two bars.
        if self.bar < 2 {
            let mut onset = 0.0;
            for (step, beats) in phrase_steps(&tune.motif, 0) {
                let slow = onset * 2.0 - self.bar as f32 * BEATS as f32;
                if (0.0..BEATS as f32).contains(&slow) {
                    self.hum_line.push(LineNote {
                        onset: slow,
                        step,
                        beats: beats * 2.0,
                    });
                }
                onset += beats;
            }
        }
    }

    fn play_beat(&mut self, at: u64) {
        let Some(root) = self.tune.as_ref().map(|tune| tune.root as f32) else {
            return;
        };
        let rate = self.rate;
        let layers = self.layers;
        let beat = self.beat as f32;
        let bar = self.bar;
        let breathing = bar >= PHRASE_BARS;
        let chord = self.chords[(bar % PHRASE_BARS) as usize];
        let beat_seconds = self.beat_samples as f32 / self.rate;
        // A phrase swells to its second bar and settles at its end.
        let arc = [0.85, 1.0, 0.95, 0.8][(bar % PHRASE_BARS) as usize];
        // Everything sounds a moment after it is planned, so a note that
        // leans early still has somewhere to lean.
        let lead = 0.03 * rate;
        let mut queue = std::mem::take(&mut self.queue);
        let place_at = |onset: f32, walk: f32| {
            let seconds = (onset - beat) * beat_seconds + walk;
            at + (lead + seconds * rate).max(0.0) as u64
        };

        // The chords, on the bar's first beat: the whole triad through a
        // phrase; in its breath, the root and fifth by day, nothing by night.
        if self.beat == 0 {
            let length = BEATS as f32 * beat_seconds + 0.9;
            if !breathing {
                for (voice, step) in chord.iter().enumerate() {
                    queue.push((
                        place_at(beat, 0.0),
                        Note {
                            instrument: Instrument::Pad,
                            pitch: root - 12.0 + *step as f32,
                            level: if voice == 0 { 0.34 } else { 0.22 },
                            length,
                            place: [0.0, -0.35, 0.35][voice],
                            bus: Bus::Pad,
                        },
                    ));
                }
            } else if layers.day > 0.1 && bar == PHRASE_BARS {
                for (voice, step) in [0.0, 7.0].into_iter().enumerate() {
                    queue.push((
                        place_at(beat, 0.0),
                        Note {
                            instrument: Instrument::Pad,
                            pitch: root - 12.0 + step,
                            level: 0.2,
                            length: length * 1.5,
                            place: [-0.2, 0.2][voice],
                            bus: Bus::Pad,
                        },
                    ));
                }
            }
        }

        // The day layer: the motif over the chord's root, a note resting
        // now and then except where the tune is stated, and the hour's own
        // passing notes between.
        if layers.day > 0.02 && !breathing {
            let home = chord[0] as f32;
            let stated = bar == 0 || bar == PHRASE_BARS - 1;
            let rest = 0.1 + 0.5 * (1.0 - layers.day.sqrt());
            for index in 0..self.day_line.len() {
                let note = self.day_line[index];
                if note.onset < beat || note.onset >= beat + 1.0 {
                    continue;
                }
                if !stated && self.dice.chance(rest) {
                    continue;
                }
                self.walk = 0.85 * self.walk + 0.006 * self.dice.signed();
                let accent = if note.onset == 0.0 {
                    1.0
                } else if note.onset == 2.0 {
                    0.9
                } else if note.onset.fract() == 0.0 {
                    0.8
                } else {
                    0.7
                };
                let pitch = root + self.octave + home + pentatonic(note.step);
                let level = 0.5 * accent * arc * self.dice.range(0.92, 1.08);
                queue.push((
                    place_at(note.onset, self.walk.clamp(-0.02, 0.02)),
                    Note {
                        instrument: Instrument::Kalimba,
                        pitch,
                        level,
                        length: note.beats * beat_seconds,
                        place: ((pitch - root - 36.0) / 30.0).clamp(-0.5, 0.5),
                        bus: Bus::Day,
                    },
                ));
                if note.beats >= 1.0 && self.dice.chance(0.35 * layers.day) {
                    let onset = note.onset + note.beats / 2.0;
                    let pitch = root + self.octave + home + pentatonic(note.step + 1);
                    queue.push((
                        place_at(onset, self.walk.clamp(-0.02, 0.02)),
                        Note {
                            instrument: Instrument::Kalimba,
                            pitch,
                            level: level * 0.6,
                            length: beat_seconds,
                            place: ((pitch - root - 36.0) / 30.0).clamp(-0.5, 0.5),
                            bus: Bus::Day,
                        },
                    ));
                }
            }
        }

        // The evening layer: the motif hummed slowly, then long chord
        // tones, and in a breath now and then one long low note.
        if layers.evening > 0.02 {
            for note in self.hum_line.iter().copied() {
                if note.onset < beat || note.onset >= beat + 1.0 {
                    continue;
                }
                queue.push((
                    place_at(note.onset, 0.0),
                    Note {
                        instrument: Instrument::Hum,
                        pitch: root + 12.0 + pentatonic(note.step),
                        level: 0.3 * arc,
                        length: note.beats * beat_seconds + 0.4,
                        place: -0.15,
                        bus: Bus::Evening,
                    },
                ));
            }
            let chord_tones = (bar == 2 || bar == 3) && self.beat.is_multiple_of(2);
            let breath_note = breathing && self.beat == 0 && self.dice.chance(0.5);
            if (chord_tones && self.dice.chance(0.8 * layers.evening.sqrt())) || breath_note {
                let long = breath_note || self.dice.chance(0.35);
                let tone = if breath_note {
                    7
                } else {
                    chord[self.dice.pick(3)]
                };
                queue.push((
                    place_at(beat, 0.0),
                    Note {
                        instrument: Instrument::Hum,
                        pitch: root + 12.0 + tone as f32,
                        level: 0.26 * arc,
                        length: beat_seconds * if long { 4.0 } else { 2.0 } + 0.4,
                        place: 0.1,
                        bus: Bus::Evening,
                    },
                ));
            }
        }

        // A festival: the motif rung on bells as the phrase starts and
        // ends, and a soft brushed tap on every beat.
        if layers.festival > 0.0 {
            if !breathing && (bar == 0 || bar == PHRASE_BARS - 1) {
                let home = chord[0] as f32;
                for note in self.day_line.iter().copied() {
                    if note.onset < beat || note.onset >= beat + 1.0 {
                        continue;
                    }
                    queue.push((
                        place_at(note.onset, 0.0),
                        Note {
                            instrument: Instrument::Bell,
                            pitch: root + 36.0 + home + pentatonic(note.step),
                            level: 0.28,
                            length: 1.0,
                            place: 0.4,
                            bus: Bus::Festival,
                        },
                    ));
                }
            }
            queue.push((
                place_at(beat, 0.0),
                Note {
                    instrument: Instrument::Brush,
                    pitch: 60.0,
                    level: if self.beat.is_multiple_of(2) {
                        0.16
                    } else {
                        0.1
                    },
                    length: 0.1,
                    place: -0.3,
                    bus: Bus::Festival,
                },
            ));
        }
        self.queue = queue;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tune::phrase;
    use crate::{Place, Tune};

    const HARBOUR: crate::Palette = [0x9fd3ee, 0xf6ecd2, 0x7fa37a, 0x4e7d5b, 0xfff1c9];

    fn scene(hour: f32, sky: Sky) -> Scene {
        Scene {
            place: Place::Harbour,
            tune: Tune::of(HARBOUR),
            hour,
            sky,
            festival: false,
        }
    }

    /// Every note the music starts in `seconds`, with when it starts.
    fn notes(scene: &Scene, seconds: f32) -> Vec<(u64, Note)> {
        let rate = 48_000.0;
        let mut music = Music::new(rate, 5, rate / 16.0);
        music.set_scene(Some(scene), 0);
        let mut heard = Vec::new();
        let mut at = 0;
        while at < (seconds * rate) as u64 {
            at += 16;
            music.advance(at, |note| heard.push((at, note)));
        }
        heard
    }

    #[test]
    fn the_day_layer_plays_the_worlds_motif_note_by_note() {
        let tune = Tune::of(HARBOUR);
        let noon = scene(13.0, Sky::Clear);
        let heard = notes(&noon, 3.0);
        // The first bar: the motif stated, every note of it, in order,
        // with passing notes between it may have.
        let plucks = heard
            .iter()
            .filter(|(at, note)| note.bus == Bus::Day && *at < 3 * 48_000)
            .map(|(_, note)| note.pitch - tune.root as f32)
            .collect::<Vec<_>>();
        let written = phrase(&tune.motif, 0);
        let mut found = plucks.iter();
        for (pitch, _) in &written {
            assert!(
                found.any(|heard| (heard - 36.0 - pitch).abs() < 1e-3
                    || (heard - 24.0 - pitch).abs() < 1e-3),
                "{pitch} of {written:?} in {plucks:?}"
            );
        }
    }

    #[test]
    fn night_is_sparse_and_noon_is_not() {
        let count = |hour: f32| {
            notes(&scene(hour, Sky::Clear), 60.0)
                .iter()
                .filter(|(_, note)| note.bus != Bus::Pad)
                .count()
        };
        let (noon, night) = (count(13.0), count(3.0));
        assert!(noon > night * 2, "noon {noon}, night {night}");
        assert!(night > 5, "never silent for a whole minute: {night}");
        assert!(noon < 180, "gentle, not busy: {noon} a minute");
    }

    #[test]
    fn music_is_the_same_for_the_same_seed() {
        let noon = scene(13.0, Sky::Clear);
        assert_eq!(notes(&noon, 20.0), notes(&noon, 20.0));
    }

    #[test]
    fn the_day_layer_rises_with_the_morning_and_the_evening_takes_over() {
        let noon = layers(13.0, Sky::Clear, false);
        assert!(noon.base == 1.0 && noon.day > 0.9 && noon.evening == 0.0);
        let night = layers(21.0, Sky::Clear, false);
        assert!(night.day == 0.0 && night.evening > 0.9);
        let small_hours = layers(3.0, Sky::Clear, false);
        assert!(small_hours.day == 0.0 && small_hours.evening < 0.6);
        // It moves smoothly: no two minutes apart differ by much.
        for minute in 0..24 * 60 {
            let now = layers(minute as f32 / 60.0, Sky::Clear, false);
            let next = layers((minute + 1) as f32 / 60.0, Sky::Clear, false);
            assert!((now.day - next.day).abs() < 0.01);
            assert!((now.evening - next.evening).abs() < 0.01);
        }
    }

    #[test]
    fn weather_thins_the_music_and_a_storm_takes_the_plucks() {
        for hour in [9.0, 13.0, 20.0] {
            let mut last = f32::MAX;
            for sky in [Sky::Clear, Sky::Cloudy, Sky::Rain, Sky::Storm] {
                let layers = layers(hour, sky, false);
                let total = layers.base + layers.day + layers.evening;
                assert!(total <= last, "{hour} {sky:?}");
                last = total;
            }
        }
        assert_eq!(layers(12.0, Sky::Storm, false).day, 0.0);
        let festival = layers(22.0, Sky::Clear, true);
        assert!(festival.festival == 1.0 && festival.day >= 0.8);
    }
}
