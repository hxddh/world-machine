//! A World's tune: the key it plays in and its motif, both made from the
//! colours of its landscape, exactly as they were when the music was a set
//! of loops, so a World keeps the tune its player already knows.

/// The colours a World's sound is made from: sky top, sky bottom, far,
/// near, sun, each `0xRRGGBB`.
pub type Palette = [u32; 5];

/// A World's own tune: steps along the pentatonic scale from where it
/// starts, and how many beats each note lasts, four beats in all.
#[derive(Clone, Debug, PartialEq)]
pub struct Motif {
    pub steps: Vec<i32>,
    pub beats: Vec<f32>,
}

/// The key and the motif a World plays.
#[derive(Clone, Debug, PartialEq)]
pub struct Tune {
    /// MIDI note of the key's root, between C3 (48) and B3 (59).
    pub root: u32,
    pub motif: Motif,
    /// A number of the palette, to seed the tune's variations with.
    pub seed: u32,
}

impl Tune {
    /// The tune a palette plays: the same palette always the same tune.
    pub fn of(palette: Palette) -> Self {
        Self {
            root: root(palette),
            motif: motif(palette),
            seed: palette_seed(palette),
        }
    }
}

/// The small random source the tune has always been made with.
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

pub(crate) fn palette_seed(palette: Palette) -> u32 {
    palette.iter().fold(0x2545_f491_u32, |seed, colour| {
        seed.rotate_left(5) ^ colour.wrapping_mul(0x9e37_79b9)
    })
}

/// The hue of a colour, from 0 to 1; a grey has none.
pub(crate) fn hue(colour: u32) -> f32 {
    let bytes = [16, 8, 0].map(|shift| (colour >> shift) & 0xff);
    if bytes[0] == bytes[1] && bytes[1] == bytes[2] {
        return 0.0;
    }
    let [r, g, b] = bytes.map(|byte| byte as f32);
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

/// The key a palette plays in: a root between C3 and B3 set by the hue of
/// its ground, so a green valley and a red desert are in different keys.
pub fn root(palette: Palette) -> u32 {
    48 + ((hue(palette[3]) * 12.0) as u32).min(11)
}

/// The motif a World's palette plays: the same palette always the same
/// tune, and different landscapes different tunes.
pub fn motif(palette: Palette) -> Motif {
    let mut dice = Dice::new(palette_seed(palette).rotate_left(11) ^ 0x6d2b_79f5);
    // Four beats of six rhythms, all singable.
    const RHYTHMS: [&[f32]; 6] = [
        &[1.0, 0.5, 0.5, 1.0, 1.0],
        &[0.5, 0.5, 1.0, 0.5, 0.5, 1.0],
        &[1.5, 0.5, 1.0, 1.0],
        &[1.0, 1.0, 0.5, 0.5, 1.0],
        &[0.5, 0.5, 0.5, 0.5, 2.0],
        &[1.0, 0.5, 0.5, 0.5, 0.5, 1.0],
    ];
    let beats = RHYTHMS[dice.pick(RHYTHMS.len())].to_vec();
    let mut steps = Vec::with_capacity(beats.len());
    let mut at = 0_i32;
    for index in 0..beats.len() {
        if index > 0 {
            // Mostly steps, now and then a leap, never far from home.
            let leap = if dice.next() < 0.25 { 2 } else { 1 };
            let up = dice.next() < if at > 2 { 0.3 } else { 0.62 };
            at += if up { leap } else { -leap };
            at = at.clamp(-2, 5);
        }
        steps.push(at);
    }
    Motif { steps, beats }
}

/// Scale steps of the major pentatonic.
pub(crate) const PENTATONIC: [u32; 5] = [0, 2, 4, 7, 9];

/// A pentatonic step as a number of semitones above the root.
pub fn pentatonic(step: i32) -> f32 {
    let octave = step.div_euclid(PENTATONIC.len() as i32);
    let degree = step.rem_euclid(PENTATONIC.len() as i32) as usize;
    (octave * 12) as f32 + PENTATONIC[degree] as f32
}

/// The motif in one of the four places a phrase puts it: stated, answered
/// a step higher and coming home, turned upside down, and stated again,
/// ending on the root. Each note is its pentatonic step over the root and
/// its length in beats.
pub(crate) fn phrase_steps(motif: &Motif, bar: usize) -> Vec<(i32, f32)> {
    let last = motif.steps.len().saturating_sub(1);
    motif
        .steps
        .iter()
        .zip(&motif.beats)
        .enumerate()
        .map(|(index, (step, beats))| {
            let step = match bar % 4 {
                0 => *step,
                1 if index == last => 0,
                1 => step + 1,
                2 => -step + 2,
                _ if index == last => 0,
                _ => *step,
            };
            (step, *beats)
        })
        .collect()
}

/// [`phrase_steps`] with each step as semitones over the root.
pub fn phrase(motif: &Motif, bar: usize) -> Vec<(f32, f32)> {
    phrase_steps(motif, bar)
        .into_iter()
        .map(|(step, beats)| (pentatonic(step), beats))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HARBOUR: Palette = [0x9fd3ee, 0xf6ecd2, 0x7fa37a, 0x4e7d5b, 0xfff1c9];
    const MARS: Palette = [0xe7b089, 0xf5d9bd, 0xc2663f, 0x8a3a22, 0xfff3dc];

    #[test]
    fn each_world_has_a_tune_and_a_key_of_its_own() {
        assert_ne!(motif(HARBOUR), motif(MARS), "two landscapes, two tunes");
        assert_ne!(root(HARBOUR), root(MARS), "two landscapes, two keys");
        let tunes = (0..40_u32)
            .map(|index| {
                let colour = 0x10_2030 + index * 0x05_0b13;
                format!(
                    "{:?}",
                    motif([colour, colour ^ 0xffff, 0x7fa37a, colour >> 1, 0xfff1c9])
                )
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(tunes.len() >= 30, "{} tunes for 40 places", tunes.len());
        let tune = motif(HARBOUR);
        assert_eq!(tune.beats.iter().sum::<f32>(), 4.0, "four beats a bar");
        assert_eq!(Tune::of(HARBOUR), Tune::of(HARBOUR));
    }

    #[test]
    fn a_phrase_states_answers_turns_and_comes_home() {
        let tune = motif(HARBOUR);
        let first = phrase(&tune, 0);
        let last = phrase(&tune, 3);
        assert_eq!(first[..first.len() - 1], last[..last.len() - 1]);
        assert_eq!(last.last().unwrap().0, 0.0, "it ends on the root");
        assert_ne!(first, phrase(&tune, 2), "and is turned about between");
    }

    #[test]
    fn the_scale_is_the_major_pentatonic() {
        let steps = (-5..=5).map(pentatonic).collect::<Vec<_>>();
        assert_eq!(
            steps,
            [-12.0, -10.0, -8.0, -5.0, -3.0, 0.0, 2.0, 4.0, 7.0, 9.0, 12.0]
        );
    }
}
