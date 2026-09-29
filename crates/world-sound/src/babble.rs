//! Voices: a babble like Animal Crossing's under what someone says, in a
//! voice of their own, and before a question their own three notes.

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
        let at = STEPS.iter().position(|step| *step == first).unwrap_or(0);
        second = STEPS[(at + 2) % 6];
    }
    [(first, long(19)), (second, long(21)), (pick(27), 0.24)]
}

/// The motif, as samples: a soft chime a touch above their voice, and
/// where the babble after it begins. Each note rings on under the next
/// until it has died away, rather than being cut off.
fn chime(seed: u32, rate: f32) -> (Vec<f32>, usize) {
    let tau = std::f32::consts::TAU;
    let base = voice(seed).pitch * 4.0;
    // Long enough to fall below a thousandth of its strike.
    let ring = (0.75 * rate) as usize;
    let mut out = Vec::new();
    let mut at = 0;
    for (step, length) in motif_of(seed) {
        let frequency = base * 2.0_f32.powf(step / 12.0);
        out.resize(out.len().max(at + ring), 0.0);
        for offset in 0..ring {
            let t = offset as f32 / rate;
            let envelope = (t / 0.004).min(1.0) * (-t / 0.12).exp();
            let wave = (tau * frequency * t).sin() + 0.3 * (tau * frequency * 2.0 * t).sin();
            out[at + offset] += 0.45 * envelope * wave;
        }
        at += (length * rate) as usize;
    }
    // The last note rings on under the babble's first syllable.
    (out, at + (0.06 * rate) as usize)
}

/// A babble: `syllables` short vowel sounds in the voice numbered `seed`,
/// a touch of breath before some, falling a little over the line, or
/// rising at the end of a question, which its asker's three notes
/// announce. Mono samples at `rate`, the same for the same arguments.
pub fn babble(seed: u32, syllables: u8, question: bool, variation: u8, rate: f32) -> Vec<f32> {
    let voice = voice(seed);
    let syllables = syllables.clamp(1, 12) as usize;
    let length = voice.syllable_seconds;
    let gap = length * 0.25;
    let (lead, begins) = if question {
        chime(seed, rate)
    } else {
        (Vec::new(), 0)
    };
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
                if frequency > rate * 0.45 {
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
    let mut out = vec![0.0; lead.len().max(begins + samples.len())];
    for (at, sample) in lead.into_iter().enumerate() {
        out[at] += sample * 0.38;
    }
    for (at, sample) in samples.into_iter().enumerate() {
        out[begins + at] += sample * gain;
    }
    out.into_iter()
        .map(|sample| sample.clamp(-1.0, 1.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    #[test]
    fn someone_asking_is_announced_by_their_own_three_notes() {
        let motifs = (0..40_u32)
            .map(|person| format!("{:?}", motif_of(person.wrapping_mul(0x9e37_79b9) ^ 0x1234)))
            .collect::<std::collections::BTreeSet<_>>();
        assert!(motifs.len() >= 30, "{} motifs for 40 people", motifs.len());
        let seed = 0xabcd_1234;
        let [(first, _), (second, _), _] = motif_of(seed);
        assert_ne!(first, second, "a tune, not one note twice");
        let asked = babble(seed, 5, true, 0, RATE);
        let said = babble(seed, 5, false, 0, RATE);
        assert!(asked.len() > said.len() + RATE as usize / 3);
    }

    #[test]
    fn everyone_babbles_in_a_voice_of_their_own() {
        let mut voices = std::collections::BTreeSet::new();
        for person in 0..40_u32 {
            let seed = person.wrapping_mul(0x0100_0193) ^ 0x811c_9dc5;
            let heard = voice(seed);
            assert!(heard.pitch >= 150.0 && heard.pitch <= 370.0);
            let samples = babble(seed, 6, false, 0, RATE);
            assert_eq!(samples, babble(seed, 6, false, 0, RATE));
            voices.insert(format!("{samples:?}"));
        }
        assert_eq!(voices.len(), 40, "no two people sound the same");
        let short = babble(11, 2, false, 0, RATE);
        let long = babble(11, 9, false, 0, RATE);
        assert!(long.len() > short.len() * 3);
        assert_ne!(babble(11, 4, true, 0, RATE), babble(11, 4, false, 0, RATE));
        let peak = long.iter().fold(0.0_f32, |peak, s| peak.max(s.abs()));
        assert!(peak > 0.3 && peak < 0.5, "{peak}");
        // A question's chime rings on under its notes, never cut off.
        // A cut-off is one jump far larger than the jumps either side.
        let asked = babble(0xabcd_1234, 5, true, 0, RATE);
        // The chime alone: the babble's breaths after it are noise.
        let jumps = asked[..(0.45 * RATE) as usize]
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .collect::<Vec<_>>();
        for (index, around) in jumps.windows(3).enumerate() {
            assert!(
                around[1] <= 3.0 * around[0].max(around[2]) + 1e-3,
                "a cut at {index}: {around:?}"
            );
        }
    }
}
