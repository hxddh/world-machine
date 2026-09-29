//! A World's quiet sound: wind whose brightness follows its sky and a low
//! hum pitched by its ground, rising and falling slowly. Presentation only,
//! like the sky: the World records none of it.
//!
//! The sound is made here from a World's scenery rather than shipped, so
//! every Pack that gives its World colours gets a sound of its own without
//! adding a file. It is off unless the player turns it on in Settings, and
//! plays only while its World's window is in front.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Samples per second of the made sound.
pub const SAMPLE_RATE: u32 = 22_050;
/// How long one loop lasts. The slow swell and the hum both fit it a whole
/// number of times, so it loops without a seam.
pub const LOOP_SECONDS: u32 = 16;
/// How long the wind crossfades into itself where the loop joins.
const JOIN_SECONDS: f32 = 1.0;

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Whether the player has turned ambient sound on.
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Record the player's choice; turning it off silences whatever plays.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
    #[cfg(target_os = "macos")]
    if !on {
        player::stop();
    }
}

/// The colours a sound is made from: sky top, sky bottom, far, near, sun.
pub type Palette = [u32; 5];

fn channel(colour: u32, shift: u32) -> f32 {
    ((colour >> shift) & 0xff) as f32 / 255.0
}

fn brightness(colour: u32) -> f32 {
    0.299 * channel(colour, 16) + 0.587 * channel(colour, 8) + 0.114 * channel(colour, 0)
}

fn hue(colour: u32) -> f32 {
    // A grey has no hue: all three channels are the same byte. Compared as
    // bytes, exactly, rather than as floats within some epsilon.
    let bytes = [16, 8, 0].map(|shift| (colour >> shift) & 0xff);
    if bytes[0] == bytes[1] && bytes[1] == bytes[2] {
        return 0.0;
    }
    let (r, g, b) = (channel(colour, 16), channel(colour, 8), channel(colour, 0));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let h = if max == r {
        ((g - b) / (max - min)).rem_euclid(6.0)
    } else if max == g {
        (b - r) / (max - min) + 2.0
    } else {
        (r - g) / (max - min) + 4.0
    };
    h / 6.0
}

/// A frequency near `target` that completes a whole number of cycles in one
/// loop, so the hum joins itself without a click.
fn loop_frequency(target: f32) -> f32 {
    (target * LOOP_SECONDS as f32).round().max(1.0) / LOOP_SECONDS as f32
}

/// The loop for a palette, as 16-bit samples. The same palette always makes
/// the same sound.
pub fn synthesize(palette: Palette) -> Vec<i16> {
    let [sky_top, sky_bottom, _far, near, _sun] = palette;
    let samples = (SAMPLE_RATE * LOOP_SECONDS) as usize;
    let rate = SAMPLE_RATE as f32;
    // A bright sky makes a brighter, airier wind; a dark one a low rumble.
    let sky = (brightness(sky_top) + brightness(sky_bottom)) / 2.0;
    let smoothing = 0.02 + 0.10 * sky;
    // The ground sets the hum, between a low A and the A above it.
    let hum = loop_frequency(55.0 * (1.0 + hue(near)));
    let fifth = loop_frequency(hum * 1.5);

    let mut seed: u32 = palette.iter().fold(0x2545_f491_u32, |seed, colour| {
        seed.rotate_left(5) ^ colour.wrapping_mul(0x9e37_79b9)
    }) | 1;
    let mut white = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    let join = (JOIN_SECONDS * rate) as usize;
    // Make a little extra wind so the end can fade into the start.
    let mut wind = Vec::with_capacity(samples + join);
    let (mut brown, mut filtered) = (0.0_f32, 0.0_f32);
    for _ in 0..samples + join {
        brown = (brown + white() * 0.02) * 0.998;
        filtered += smoothing * (brown - filtered);
        wind.push(filtered);
    }
    for index in 0..join {
        let fade = index as f32 / join as f32;
        wind[index] = wind[index] * fade + wind[samples + index] * (1.0 - fade);
    }
    wind.truncate(samples);
    let peak = wind
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    let wind_gain = if peak > 0.0 { 0.55 / peak } else { 0.0 };

    (0..samples)
        .map(|index| {
            let t = index as f32 / rate;
            // Swells twice a loop, never dropping to silence.
            let swell = 0.65 + 0.35 * (std::f32::consts::TAU * t * 2.0 / LOOP_SECONDS as f32).sin();
            let drone = 0.06 * (std::f32::consts::TAU * hum * t).sin()
                + 0.03 * (std::f32::consts::TAU * fifth * t).sin();
            let sample = (wind[index] * wind_gain * swell + drone) * 0.5;
            (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
        })
        .collect()
}

/// How many ways each small sound can be played. The player takes them in
/// turn, so the same one is never heard twice running.
pub const VARIATIONS: u8 = 3;

/// The variation to play after `last`: the next in turn.
pub fn next_variation(last: Option<u8>) -> u8 {
    last.map_or(0, |last| (last + 1) % VARIATIONS)
}

/// The small sounds a World window asks for: a soft tick as a card turns,
/// a two-toned bell as a turn passes, a rising pair of notes when something
/// new is built, each in `VARIATIONS` versions, and a babble under what
/// someone says. Short, quiet, and the same for the same variation.
pub fn cue_samples(cue: world_gpui::Cue, variation: u8) -> Vec<i16> {
    let rate = SAMPLE_RATE as f32;
    let variation = variation % VARIATIONS;
    let tone = |notes: &[(f32, f32, f32)], length: f32, decay: f32, level: f32| {
        let samples = (length * rate) as usize;
        (0..samples)
            .map(|index| {
                let t = index as f32 / rate;
                let mut sample = 0.0;
                for (start, frequency, weight) in notes {
                    if t >= *start {
                        let local = t - start;
                        let envelope = (-local / decay).exp() * (local / 0.004).min(1.0);
                        sample += weight
                            * envelope
                            * ((std::f32::consts::TAU * frequency * local).sin()
                                + 0.3 * (std::f32::consts::TAU * frequency * 2.0 * local).sin());
                    }
                }
                ((sample * level).clamp(-1.0, 1.0) * i16::MAX as f32) as i16
            })
            .collect::<Vec<_>>()
    };
    match cue {
        world_gpui::Cue::Flip => {
            // A short, soft tick: a little filtered noise that dies at once,
            // a shade brighter or duller each time.
            let samples = (0.06 * rate) as usize;
            let mut seed: u32 = 0x1234_5679 + u32::from(variation) * 0x0101_0101;
            let smoothing = [0.35, 0.28, 0.44][usize::from(variation)];
            let mut smooth = 0.0_f32;
            (0..samples)
                .map(|index| {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    let white = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
                    smooth += smoothing * (white - smooth);
                    let t = index as f32 / rate;
                    let envelope = (-t / 0.012).exp();
                    ((smooth * envelope * 0.35).clamp(-1.0, 1.0) * i16::MAX as f32) as i16
                })
                .collect()
        }
        world_gpui::Cue::Turn => {
            let [low, high] =
                [[659.25, 987.77], [587.33, 880.0], [783.99, 1174.66]][usize::from(variation)];
            tone(&[(0.0, low, 0.6), (0.0, high, 0.3)], 0.9, 0.28, 0.32)
        }
        world_gpui::Cue::Built => {
            let [first, second] =
                [[523.25, 783.99], [587.33, 880.0], [659.25, 1046.5]][usize::from(variation)];
            tone(&[(0.0, first, 0.55), (0.16, second, 0.55)], 0.9, 0.24, 0.32)
        }
        world_gpui::Cue::Babble {
            voice,
            syllables,
            question,
        } => babble(voice, syllables, question, variation),
    }
}

/// A voice of someone's own: how high they speak, how quickly, and how
/// bright or round their vowels are, all from their stable number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Voice {
    pub pitch: f32,
    pub syllable_seconds: f32,
    pub brightness: f32,
}

pub fn voice(seed: u32) -> Voice {
    let unit = |shift: u32| ((seed >> shift) & 0xff) as f32 / 255.0;
    Voice {
        // From a low murmur to a bright chirp.
        pitch: 150.0 * 2.0_f32.powf(unit(0) * 1.3),
        syllable_seconds: 0.065 + 0.035 * unit(8),
        brightness: 0.4 + 0.6 * unit(16),
    }
}

/// Where the two lowest resonances of each vowel sit (a, e, i, o, u).
const VOWELS: [(f32, f32); 5] = [
    (800.0, 1200.0),
    (500.0, 1900.0),
    (320.0, 2300.0),
    (500.0, 900.0),
    (350.0, 750.0),
];

/// A babble like Animal Crossing's: `syllables` short vowel sounds in the
/// voice numbered `seed`, a touch of breath before some, falling a little
/// over the line, or rising at the end of a question.
/// Someone's own three notes, rung softly before they ask the player
/// something, so a player hears who is asking before they read it: steps
/// of the pentatonic scale above their voice's pitch, and how long each
/// rings, the same person always the same.
pub fn motif_of(seed: u32) -> [(f32, f32); 3] {
    const STEPS: [f32; 6] = [0.0, 2.0, 4.0, 7.0, 9.0, 12.0];
    let pick = |shift: u32| STEPS[((seed.rotate_left(shift) ^ (seed >> 7)) % 6) as usize];
    let long = |shift: u32| if (seed >> shift) & 1 == 1 { 0.16 } else { 0.11 };
    let first = pick(3);
    let mut second = pick(11);
    if second == first {
        second = STEPS[(STEPS.iter().position(|step| *step == first).unwrap() + 2) % 6];
    }
    [(first, long(19)), (second, long(21)), (pick(27), 0.24)]
}

/// The motif, as samples: a soft chime a touch above their voice.
fn chime(seed: u32) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let tau = std::f32::consts::TAU;
    let base = voice(seed).pitch * 4.0;
    let mut out = Vec::new();
    for (step, length) in motif_of(seed) {
        let frequency = base * 2.0_f32.powf(step / 12.0);
        let count = (length * rate) as usize;
        let ring = (0.35 * rate) as usize;
        let start = out.len();
        out.resize(start + count.max(ring), 0.0);
        for offset in 0..ring {
            let t = offset as f32 / rate;
            let envelope = (t / 0.004).min(1.0) * (-t / 0.12).exp();
            let wave = (tau * frequency * t).sin() + 0.3 * (tau * frequency * 2.0 * t).sin();
            out[start + offset] += 0.45 * envelope * wave;
        }
        out.truncate(start + count);
    }
    out.extend(std::iter::repeat_n(0.0, (0.06 * rate) as usize));
    out
}

pub fn babble(seed: u32, syllables: u8, question: bool, variation: u8) -> Vec<i16> {
    let rate = SAMPLE_RATE as f32;
    let voice = voice(seed);
    let syllables = syllables.clamp(1, 12) as usize;
    let length = voice.syllable_seconds;
    let gap = length * 0.25;
    // A question is announced by the asker's own three notes.
    let lead = if question { chime(seed) } else { Vec::new() };
    let total = ((length + gap) * syllables as f32 * rate) as usize + (0.05 * rate) as usize;
    let mut samples = vec![0.0_f32; total];
    let mut dice = seed ^ (u32::from(variation) << 24) ^ 0x9e37_79b9 | 1;
    let mut roll = move || {
        dice ^= dice << 13;
        dice ^= dice >> 17;
        dice ^= dice << 5;
        dice as f32 / u32::MAX as f32
    };
    let tau = std::f32::consts::TAU;
    for syllable in 0..syllables {
        let start = ((length + gap) * syllable as f32 * rate) as usize;
        let (first, second) = VOWELS[(roll() * VOWELS.len() as f32) as usize % VOWELS.len()];
        let through = syllable as f32 / syllables.max(2) as f32;
        let mut pitch = voice.pitch * (1.06 - 0.12 * through) * (0.96 + 0.08 * roll());
        if question && syllable + 2 >= syllables {
            pitch *= if syllable + 1 == syllables {
                1.35
            } else {
                1.15
            };
        }
        let breath = roll() < 0.4;
        let count = (length * rate) as usize;
        let mut hiss = 0.0_f32;
        for offset in 0..count.min(total - start) {
            let t = offset as f32 / rate;
            let envelope = (t / 0.008).min(1.0) * ((length - t) / 0.02).clamp(0.0, 1.0);
            let mut sample = 0.0;
            // Harmonics of the pitch, loudest near the vowel's resonances.
            for harmonic in 1..=14 {
                let frequency = pitch * harmonic as f32;
                if frequency > rate / 2.0 {
                    break;
                }
                let near =
                    |centre: f32, width: f32| (-((frequency - centre) / width).powi(2)).exp();
                let weight = near(first, 130.0)
                    + voice.brightness * 0.7 * near(second, 220.0)
                    + 0.15 / harmonic as f32;
                sample += weight * (tau * frequency * t).sin();
            }
            if breath && t < 0.018 {
                let white = roll() * 2.0 - 1.0;
                hiss += 0.5 * (white - hiss);
                sample += 1.2 * hiss * (1.0 - t / 0.018);
            }
            samples[start + offset] += sample * envelope;
        }
    }
    let peak = samples
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    let gain = if peak > 0.0 { 0.38 / peak } else { 0.0 };
    lead.into_iter()
        .map(|sample| sample * 0.38)
        .chain(samples.into_iter().map(|sample| sample * gain))
        .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

/// The four sounds a player can set apart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channel {
    Music,
    Ambience,
    Voices,
    Interface,
}

impl Channel {
    pub const ALL: [Channel; 4] = [
        Channel::Music,
        Channel::Ambience,
        Channel::Voices,
        Channel::Interface,
    ];

    /// How loud the channel plays at full, before the player's level.
    fn full(self) -> f32 {
        match self {
            Channel::Music => 0.3,
            Channel::Ambience => 0.35,
            Channel::Voices => 0.45,
            Channel::Interface => 0.5,
        }
    }

    /// The level a channel starts at, as a percentage.
    pub fn default_level(self) -> u8 {
        match self {
            Channel::Music => 60,
            Channel::Ambience => 70,
            Channel::Voices => 70,
            Channel::Interface => 80,
        }
    }
}

static LEVELS: [AtomicU8; 4] = [
    AtomicU8::new(60),
    AtomicU8::new(70),
    AtomicU8::new(70),
    AtomicU8::new(80),
];

fn slot(channel: Channel) -> usize {
    Channel::ALL
        .iter()
        .position(|each| *each == channel)
        .unwrap_or(0)
}

/// The player's level for a channel, from 0 to 100.
pub fn level(channel: Channel) -> u8 {
    LEVELS[slot(channel)].load(Ordering::Relaxed)
}

/// Record the player's level for a channel.
pub fn set_level(channel: Channel, percent: u8) {
    LEVELS[slot(channel)].store(percent.min(100), Ordering::Relaxed);
}

/// How loud to play a channel now: its full volume times the player's
/// level, or nothing with sound off.
pub fn volume(channel: Channel) -> f32 {
    if !enabled() {
        return 0.0;
    }
    channel.full() * f32::from(level(channel)) / 100.0
}

/// A mono 16-bit WAV file holding `samples`.
pub fn wav(samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

/// A short stable name for a palette's sound file.
pub fn file_name(palette: Palette) -> String {
    let key = palette
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, colour| {
            (hash ^ u64::from(*colour)).wrapping_mul(0x0100_0000_01b3)
        });
    format!("{key:016x}.wav")
}

/// What one of a World's two loops plays: its landscape's sound, or its
/// music at a moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Loop {
    Ambience(Palette),
    Music(Palette, crate::music::Moment),
}

impl Loop {
    pub fn channel(self) -> Channel {
        match self {
            Loop::Ambience(_) => Channel::Ambience,
            Loop::Music(..) => Channel::Music,
        }
    }

    pub fn file_name(self) -> String {
        match self {
            Loop::Ambience(palette) => file_name(palette),
            Loop::Music(palette, moment) => crate::music::file_name(palette, moment),
        }
    }

    pub fn samples(self) -> Vec<i16> {
        match self {
            Loop::Ambience(palette) => synthesize(palette),
            Loop::Music(palette, moment) => crate::music::compose(palette, moment),
        }
    }
}

#[cfg(target_os = "macos")]
pub mod player {
    //! Plays one World's two loops at a time, its landscape and its music,
    //! with the system's own player, and stops them the moment they are no
    //! longer wanted. When the hour or the weather changes, the music moves
    //! to its new mix where the loop ends, so it never cuts off mid-phrase.

    use super::{volume, wav, Channel, Loop, Palette};
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    struct Playing {
        owner: u64,
        palette: Palette,
        wanted: Arc<Mutex<Loop>>,
        stop: Arc<AtomicBool>,
        child: Arc<Mutex<Option<Child>>>,
    }

    /// The ambience loop, then the music loop.
    static PLAYING: Mutex<[Option<Playing>; 2]> = Mutex::new([None, None]);
    /// The variation each small sound played last.
    static LAST: Mutex<[Option<u8>; 4]> = Mutex::new([None; 4]);
    /// How often the player checks whether its loop has ended.
    const POLL: std::time::Duration = std::time::Duration::from_millis(100);

    fn directory() -> Option<PathBuf> {
        let root = crate::app_settings::application_support_root().ok()?;
        let directory = root.join("Ambience");
        std::fs::create_dir_all(&directory).ok()?;
        Some(directory)
    }

    /// The file for a loop, made the first time it is wanted. Called off
    /// the window's thread, since music takes a moment to make.
    fn sound_file(sound: Loop) -> Option<PathBuf> {
        let path = directory()?.join(sound.file_name());
        if !path.is_file() {
            let partial = path.with_extension("part");
            std::fs::write(&partial, wav(&sound.samples())).ok()?;
            std::fs::rename(&partial, &path).ok()?;
        }
        Some(path)
    }

    fn halt(playing: Playing) {
        playing.stop.store(true, Ordering::Relaxed);
        if let Ok(mut child) = playing.child.lock() {
            if let Some(child) = child.as_mut() {
                let _ = child.kill();
            }
        }
    }

    fn start(wanted: Arc<Mutex<Loop>>, stop: Arc<AtomicBool>, child: Arc<Mutex<Option<Child>>>) {
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let Ok(sound) = wanted.lock().map(|sound| *sound) else {
                    return;
                };
                let level = volume(sound.channel());
                if level <= 0.0 {
                    // Turned right down: wait until it is turned up again.
                    std::thread::sleep(POLL * 5);
                    continue;
                }
                let Some(path) = sound_file(sound) else {
                    return;
                };
                let spawned = Command::new("/usr/bin/afplay")
                    .arg("-v")
                    .arg(format!("{level:.2}"))
                    .arg(&path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
                let Ok(spawned) = spawned else {
                    return;
                };
                if let Ok(mut slot) = child.lock() {
                    *slot = Some(spawned);
                }
                // Poll rather than wait, so the lock is only ever held for
                // a moment and stopping never waits on a loop to finish.
                let finished = loop {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }
                    let polled = child
                        .lock()
                        .ok()
                        .and_then(|mut slot| slot.as_mut().map(|child| child.try_wait()));
                    match polled {
                        Some(Ok(Some(status))) => break status,
                        Some(Ok(None)) => std::thread::sleep(POLL),
                        _ => return,
                    }
                };
                if !finished.success() {
                    return;
                }
            }
        });
    }

    /// The window `owner` is in front and shows a World in `palette` at
    /// `moment`: play its landscape and its music, unless they already
    /// play. A new moment is picked up where the music's loop ends.
    pub fn claim(owner: u64, palette: Palette, moment: crate::music::Moment) {
        if !super::enabled() {
            return;
        }
        let Ok(mut playing) = PLAYING.lock() else {
            return;
        };
        for (index, sound) in [Loop::Ambience(palette), Loop::Music(palette, moment)]
            .into_iter()
            .enumerate()
        {
            if let Some(current) = playing[index]
                .as_ref()
                .filter(|current| current.owner == owner && current.palette == palette)
            {
                if let Ok(mut wanted) = current.wanted.lock() {
                    *wanted = sound;
                }
                continue;
            }
            if let Some(previous) = playing[index].take() {
                halt(previous);
            }
            let wanted = Arc::new(Mutex::new(sound));
            let stop = Arc::new(AtomicBool::new(false));
            let child = Arc::new(Mutex::new(None));
            start(Arc::clone(&wanted), Arc::clone(&stop), Arc::clone(&child));
            playing[index] = Some(Playing {
                owner,
                palette,
                wanted,
                stop,
                child,
            });
        }
    }

    /// The window `owner` closed or went behind: stop its sound if it is
    /// the one playing.
    pub fn release(owner: u64) {
        let Ok(mut playing) = PLAYING.lock() else {
            return;
        };
        for slot in playing.iter_mut() {
            if slot.as_ref().is_some_and(|current| current.owner == owner) {
                if let Some(previous) = slot.take() {
                    halt(previous);
                }
            }
        }
    }

    /// Play a small sound once, if the player wants sound: the next of its
    /// variations, on the voices channel for a babble and the interface
    /// channel for the rest.
    pub fn cue(cue: world_gpui::Cue) {
        let (kind, channel) = match cue {
            world_gpui::Cue::Flip => (0, Channel::Interface),
            world_gpui::Cue::Turn => (1, Channel::Interface),
            world_gpui::Cue::Built => (2, Channel::Interface),
            world_gpui::Cue::Babble { .. } => (3, Channel::Voices),
        };
        let level = volume(channel);
        if level <= 0.0 {
            return;
        }
        let variation = match LAST.lock() {
            Ok(mut last) => {
                let next = super::next_variation(last[kind]);
                last[kind] = Some(next);
                next
            }
            Err(_) => 0,
        };
        std::thread::spawn(move || {
            let Some(directory) = directory() else {
                return;
            };
            let name = match cue {
                world_gpui::Cue::Babble {
                    voice,
                    syllables,
                    question,
                } => format!(
                    "babble-{voice:08x}-{syllables}-{}-{variation}.wav",
                    u8::from(question)
                ),
                other => format!("cue-{other:?}-{variation}.wav").to_lowercase(),
            };
            let path = directory.join(name);
            if !path.is_file()
                && std::fs::write(&path, wav(&super::cue_samples(cue, variation))).is_err()
            {
                return;
            }
            let _ = Command::new("/usr/bin/afplay")
                .arg("-v")
                .arg(format!("{level:.2}"))
                .arg(&path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        });
    }

    /// Silence, whatever plays.
    pub fn stop() {
        if let Ok(mut playing) = PLAYING.lock() {
            for slot in playing.iter_mut() {
                if let Some(previous) = slot.take() {
                    halt(previous);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARS: Palette = [0xe7b089, 0xf5d9bd, 0xc2663f, 0x8a3a22, 0xfff3dc];
    const NIGHT_TOWN: Palette = [0x241d45, 0x6b4a7a, 0x3a2f55, 0x1b1630, 0xf4bf5c];

    #[test]
    fn a_landscape_always_sounds_the_same_and_two_sound_different() {
        let mars = synthesize(MARS);
        assert_eq!(mars.len(), (SAMPLE_RATE * LOOP_SECONDS) as usize);
        assert_eq!(mars, synthesize(MARS));
        assert_ne!(mars, synthesize(NIGHT_TOWN));
        assert_ne!(file_name(MARS), file_name(NIGHT_TOWN));
    }

    #[test]
    fn it_is_quiet_and_never_silent() {
        let samples = synthesize(MARS);
        let peak = samples
            .iter()
            .map(|sample| sample.unsigned_abs())
            .max()
            .unwrap();
        assert!(
            peak < i16::MAX as u16 / 2,
            "a bed of sound, not a blast: {peak}"
        );
        let second = SAMPLE_RATE as usize;
        for window in samples.chunks(second) {
            assert!(window.iter().any(|sample| sample.unsigned_abs() > 200));
        }
    }

    #[test]
    fn the_hum_fits_the_loop_a_whole_number_of_times() {
        for target in [55.0, 61.3, 82.5, 109.9] {
            let cycles = loop_frequency(target) * LOOP_SECONDS as f32;
            assert!((cycles - cycles.round()).abs() < 1e-3);
        }
    }

    const CUES: [world_gpui::Cue; 4] = [
        world_gpui::Cue::Flip,
        world_gpui::Cue::Turn,
        world_gpui::Cue::Built,
        world_gpui::Cue::Babble {
            voice: 7,
            syllables: 5,
            question: false,
        },
    ];

    #[test]
    fn cues_are_short_quiet_and_fade_to_nothing() {
        for cue in CUES {
            for variation in 0..VARIATIONS {
                let samples = cue_samples(cue, variation);
                assert!(
                    !samples.is_empty() && samples.len() <= SAMPLE_RATE as usize,
                    "{cue:?}"
                );
                let peak = samples
                    .iter()
                    .map(|sample| sample.unsigned_abs())
                    .max()
                    .unwrap();
                assert!(peak > 500 && peak < i16::MAX as u16 / 2, "{cue:?}: {peak}");
                let tail = samples[samples.len() * 9 / 10..]
                    .iter()
                    .map(|sample| sample.unsigned_abs())
                    .max()
                    .unwrap();
                assert!(
                    tail < peak / 8,
                    "{cue:?} should have died away: {tail} of {peak}"
                );
                assert_eq!(samples, cue_samples(cue, variation));
            }
        }
    }

    #[test]
    fn every_small_sound_has_three_versions_never_heard_twice_running() {
        for cue in CUES {
            let versions = (0..VARIATIONS)
                .map(|variation| cue_samples(cue, variation))
                .collect::<std::collections::BTreeSet<_>>();
            assert!(versions.len() >= 3, "{cue:?}");
        }
        let mut last = None;
        for _ in 0..30 {
            let next = next_variation(last);
            assert_ne!(Some(next), last);
            assert!(next < VARIATIONS);
            last = Some(next);
        }
    }

    #[test]
    fn someone_asking_is_announced_by_their_own_three_notes() {
        let motifs = (0..40_u32)
            .map(|person| format!("{:?}", motif_of(person.wrapping_mul(0x9e37_79b9) ^ 0x1234)))
            .collect::<std::collections::BTreeSet<_>>();
        assert!(motifs.len() >= 30, "{} motifs for 40 people", motifs.len());
        let seed = 0xabcd_1234;
        let [(first, _), (second, _), _] = motif_of(seed);
        assert_ne!(first, second, "a tune, not one note twice");
        assert_eq!(
            motif_of(seed),
            motif_of(seed),
            "the same person, the same notes"
        );
        // It comes before the question's babble, and a plain line has none.
        let asked = babble(seed, 5, true, 0);
        let said = babble(seed, 5, false, 0);
        assert!(asked.len() > said.len() + SAMPLE_RATE as usize / 3);
    }

    #[test]
    fn everyone_babbles_in_a_voice_of_their_own() {
        use world_gpui::SelectionId;
        let line = "Morning! The boats are late again.";
        let mut voices = std::collections::BTreeSet::new();
        for id in 0..40 {
            let world_gpui::Cue::Babble { voice: seed, .. } = world_gpui::babble(
                SelectionId::from_stable_key(&format!("entity-{}", id + 1)).unwrap(),
                line,
            ) else {
                panic!("a line is a babble");
            };
            let heard = voice(seed);
            assert!(heard.pitch >= 150.0 && heard.pitch <= 370.0);
            voices.insert(babble(seed, 6, false, 0));
            assert_eq!(babble(seed, 6, false, 0), babble(seed, 6, false, 0));
        }
        assert_eq!(voices.len(), 40, "no two people sound the same");
        // A longer line is a longer babble, and a question rises.
        let short = babble(11, 2, false, 0);
        let long = babble(11, 9, false, 0);
        assert!(long.len() > short.len() * 3);
        assert_ne!(babble(11, 4, true, 0), babble(11, 4, false, 0));
    }

    #[test]
    fn a_level_scales_its_channel_and_sound_off_silences_all() {
        set_enabled(true);
        set_level(Channel::Voices, 100);
        let full = volume(Channel::Voices);
        set_level(Channel::Voices, 50);
        assert!((volume(Channel::Voices) - full / 2.0).abs() < 1e-6);
        set_level(Channel::Voices, 0);
        assert_eq!(volume(Channel::Voices), 0.0);
        set_level(Channel::Voices, Channel::Voices.default_level());
        set_enabled(false);
        for channel in Channel::ALL {
            assert_eq!(volume(channel), 0.0);
        }
    }

    #[test]
    fn the_file_is_a_mono_16_bit_wave() {
        let bytes = wav(&[0, 1, -1]);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(bytes.len(), 44 + 6);
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 1);
        assert_eq!(u16::from_le_bytes([bytes[34], bytes[35]]), 16);
    }
}
