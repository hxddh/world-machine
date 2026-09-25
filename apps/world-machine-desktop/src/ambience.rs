//! A World's quiet sound: wind whose brightness follows its sky and a low
//! hum pitched by its ground, rising and falling slowly. Presentation only,
//! like the sky: the World records none of it.
//!
//! The sound is made here from a World's scenery rather than shipped, so
//! every Pack that gives its World colours gets a sound of its own without
//! adding a file. It is off unless the player turns it on in Settings, and
//! plays only while its World's window is in front.

use std::sync::atomic::{AtomicBool, Ordering};

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
    let (r, g, b) = (channel(colour, 16), channel(colour, 8), channel(colour, 0));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max - min < f32::EPSILON {
        return 0.0;
    }
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

/// The small sounds a World window asks for: a soft tick as a card turns,
/// a two-toned bell as a turn passes, and a rising pair of notes when
/// something new is built. Short, quiet, and the same every time.
pub fn cue_samples(cue: world_gpui::Cue) -> Vec<i16> {
    let rate = SAMPLE_RATE as f32;
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
            // A short, soft tick: a little filtered noise that dies at once.
            let samples = (0.06 * rate) as usize;
            let mut seed: u32 = 0x1234_5679;
            let mut smooth = 0.0_f32;
            (0..samples)
                .map(|index| {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    let white = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
                    smooth += 0.35 * (white - smooth);
                    let t = index as f32 / rate;
                    let envelope = (-t / 0.012).exp();
                    ((smooth * envelope * 0.35).clamp(-1.0, 1.0) * i16::MAX as f32) as i16
                })
                .collect()
        }
        world_gpui::Cue::Turn => tone(&[(0.0, 659.25, 0.6), (0.0, 987.77, 0.3)], 0.9, 0.28, 0.32),
        world_gpui::Cue::Built => tone(
            &[(0.0, 523.25, 0.55), (0.16, 783.99, 0.55)],
            0.9,
            0.24,
            0.32,
        ),
    }
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

#[cfg(target_os = "macos")]
pub mod player {
    //! Plays one World's loop at a time with the system's own player, and
    //! stops it the moment it is no longer wanted.

    use super::{file_name, synthesize, wav, Palette};
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    struct Playing {
        owner: u64,
        palette: Palette,
        stop: Arc<AtomicBool>,
        child: Arc<Mutex<Option<Child>>>,
    }

    static PLAYING: Mutex<Option<Playing>> = Mutex::new(None);
    /// How often the player checks whether its loop has ended.
    const POLL: std::time::Duration = std::time::Duration::from_millis(100);

    fn sound_file(palette: Palette) -> Option<PathBuf> {
        let root = crate::analyst_settings::application_support_root().ok()?;
        let directory = root.join("Ambience");
        std::fs::create_dir_all(&directory).ok()?;
        let path = directory.join(file_name(palette));
        if !path.is_file() {
            std::fs::write(&path, wav(&synthesize(palette))).ok()?;
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

    /// The window `owner` is in front and shows a World in `palette`: play
    /// its sound, unless it already plays.
    pub fn claim(owner: u64, palette: Palette) {
        if !super::enabled() {
            return;
        }
        let Ok(mut playing) = PLAYING.lock() else {
            return;
        };
        if playing
            .as_ref()
            .is_some_and(|current| current.owner == owner && current.palette == palette)
        {
            return;
        }
        if let Some(previous) = playing.take() {
            halt(previous);
        }
        let Some(path) = sound_file(palette) else {
            return;
        };
        let stop = Arc::new(AtomicBool::new(false));
        let child = Arc::new(Mutex::new(None));
        let (thread_stop, thread_child) = (Arc::clone(&stop), Arc::clone(&child));
        std::thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                let spawned = Command::new("/usr/bin/afplay")
                    .arg("-v")
                    .arg("0.35")
                    .arg(&path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
                let Ok(spawned) = spawned else {
                    return;
                };
                if let Ok(mut slot) = thread_child.lock() {
                    *slot = Some(spawned);
                }
                // Poll rather than wait, so the lock is only ever held for
                // a moment and stopping never waits on a loop to finish.
                let finished = loop {
                    if thread_stop.load(Ordering::Relaxed) {
                        return;
                    }
                    let polled = thread_child
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
        *playing = Some(Playing {
            owner,
            palette,
            stop,
            child,
        });
    }

    /// The window `owner` closed or went behind: stop its sound if it is
    /// the one playing.
    pub fn release(owner: u64) {
        let Ok(mut playing) = PLAYING.lock() else {
            return;
        };
        if playing
            .as_ref()
            .is_some_and(|current| current.owner == owner)
        {
            if let Some(previous) = playing.take() {
                halt(previous);
            }
        }
    }

    /// Play a small sound once, if the player wants sound.
    pub fn cue(cue: world_gpui::Cue) {
        if !super::enabled() {
            return;
        }
        let Some(root) = crate::analyst_settings::application_support_root().ok() else {
            return;
        };
        let directory = root.join("Ambience");
        if std::fs::create_dir_all(&directory).is_err() {
            return;
        }
        let path = directory.join(format!("cue-{cue:?}.wav").to_lowercase());
        if !path.is_file() && std::fs::write(&path, wav(&super::cue_samples(cue))).is_err() {
            return;
        }
        let _ = Command::new("/usr/bin/afplay")
            .arg("-v")
            .arg("0.5")
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }

    /// Silence, whatever plays.
    pub fn stop() {
        if let Ok(mut playing) = PLAYING.lock() {
            if let Some(previous) = playing.take() {
                halt(previous);
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

    #[test]
    fn cues_are_short_quiet_and_fade_to_nothing() {
        for cue in [
            world_gpui::Cue::Flip,
            world_gpui::Cue::Turn,
            world_gpui::Cue::Built,
        ] {
            let samples = cue_samples(cue);
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
            assert_eq!(samples, cue_samples(cue));
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
