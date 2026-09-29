//! The small pieces every sound is made of: a seeded random source, a cheap
//! sine, filters, smoothers, noise and a room. All plain arithmetic in a
//! fixed order, so the same seed renders the same samples.

use std::f32::consts::{FRAC_PI_4, PI, TAU};

/// Added to what idle filters hear, so that their state settles at a
/// normal tiny number instead of sinking into subnormal floats, which some
/// processors work on a hundred times more slowly. Far below hearing.
const NOT_DENORMAL: f32 = 1e-20;

/// A small deterministic random source (xorshift64*).
#[derive(Clone, Debug)]
pub(crate) struct Dice(u64);

impl Dice {
    pub(crate) fn new(seed: u64) -> Self {
        // Spread the seed so that neighbouring seeds start far apart.
        let mut mixed = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        Self((mixed ^ (mixed >> 31)) | 1)
    }

    pub(crate) fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    /// From 0 up to, not including, 1.
    pub(crate) fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1_u32 << 24) as f32
    }

    /// From -1 up to 1.
    pub(crate) fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    pub(crate) fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    pub(crate) fn chance(&mut self, probability: f32) -> bool {
        self.unit() < probability
    }

    pub(crate) fn pick(&mut self, count: usize) -> usize {
        ((self.unit() * count as f32) as usize).min(count.saturating_sub(1))
    }
}

/// `sin(2π·turns)`, for any `turns` in [0, 1], to about a thousandth: a
/// parabola with one correction, far cheaper than `f32::sin` and
/// identical on every machine.
#[inline]
pub(crate) fn sine(turns: f32) -> f32 {
    // Centred on zero: sin(2π(t - ½)) = -sin(2πt).
    let t = turns - 0.5;
    let y = 8.0 * t - 16.0 * t * t.abs();
    -(0.225 * (y * y.abs() - y) + y)
}

/// A phase in turns, kept in [0, 1).
#[inline]
pub(crate) fn advance(phase: &mut f32, step: f32) {
    *phase += step;
    if *phase >= 1.0 {
        *phase -= 1.0;
    }
}

/// Equal-power gains for a place from -1 (left) to 1 (right).
pub(crate) fn pan(position: f32) -> (f32, f32) {
    let angle = (position.clamp(-1.0, 1.0) + 1.0) * FRAC_PI_4;
    (angle.cos(), angle.sin())
}

/// Hertz at a MIDI note number.
pub(crate) fn midi(note: f32) -> f32 {
    440.0 * 2.0_f32.powf((note - 69.0) / 12.0)
}

/// The per-sample multiplier that takes a level to 1/e in `seconds`.
pub(crate) fn decay(seconds: f32, rate: f32) -> f32 {
    if seconds <= 0.0 {
        0.0
    } else {
        (-1.0 / (seconds * rate)).exp()
    }
}

/// Glides toward a target with a time constant, one step per call.
#[derive(Clone, Debug)]
pub(crate) struct Smoother {
    pub(crate) value: f32,
    pub(crate) target: f32,
    coefficient: f32,
}

impl Smoother {
    /// `seconds` is the time constant at `calls_per_second` steps.
    pub(crate) fn new(value: f32, seconds: f32, calls_per_second: f32) -> Self {
        Self {
            value,
            target: value,
            coefficient: 1.0 - decay(seconds, calls_per_second),
        }
    }

    #[inline]
    pub(crate) fn step(&mut self) -> f32 {
        self.value += self.coefficient * (self.target - self.value);
        if (self.value - self.target).abs() < 1e-6 {
            self.value = self.target;
        }
        self.value
    }
}

/// A one-pole low-pass filter.
#[derive(Clone, Debug, Default)]
pub(crate) struct OnePole {
    state: f32,
    coefficient: f32,
}

impl OnePole {
    pub(crate) fn new(cutoff: f32, rate: f32) -> Self {
        let mut filter = Self::default();
        filter.set(cutoff, rate);
        filter
    }

    pub(crate) fn set(&mut self, cutoff: f32, rate: f32) {
        self.coefficient = 1.0 - (-TAU * cutoff / rate).exp();
    }

    #[inline]
    pub(crate) fn low(&mut self, input: f32) -> f32 {
        self.state += self.coefficient * (input + NOT_DENORMAL - self.state);
        self.state
    }

    #[inline]
    pub(crate) fn high(&mut self, input: f32) -> f32 {
        input - self.low(input)
    }
}

/// A state-variable filter (the trapezoidal form), stable while its
/// cutoff moves.
#[derive(Clone, Debug, Default)]
pub(crate) struct Svf {
    ic1: f32,
    ic2: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Svf {
    pub(crate) fn new(cutoff: f32, q: f32, rate: f32) -> Self {
        let mut filter = Self::default();
        filter.set(cutoff, q, rate);
        filter
    }

    pub(crate) fn set(&mut self, cutoff: f32, q: f32, rate: f32) {
        let g = (PI * cutoff.clamp(10.0, rate * 0.45) / rate).tan();
        self.set_g(g, q);
    }

    /// With `g` already worked out, cheaply: `π·cutoff/rate` is close
    /// enough to `tan` below a few kilohertz.
    #[inline]
    pub(crate) fn set_g(&mut self, g: f32, q: f32) {
        self.k = 1.0 / q.max(0.1);
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    /// One sample in; low-pass and band-pass out.
    #[inline]
    pub(crate) fn run(&mut self, input: f32) -> (f32, f32) {
        let v3 = input + NOT_DENORMAL - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        (v2, v1)
    }

    #[inline]
    pub(crate) fn band(&mut self, input: f32) -> f32 {
        self.run(input).1
    }

    #[inline]
    pub(crate) fn low(&mut self, input: f32) -> f32 {
        self.run(input).0
    }
}

/// Pink noise: white noise shaped to fall 3 dB an octave, like wind and
/// water (Paul Kellet's economical filter).
#[derive(Clone, Debug)]
pub(crate) struct Pink {
    dice: Dice,
    b: [f32; 3],
}

impl Pink {
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            dice: Dice::new(seed),
            b: [0.0; 3],
        }
    }

    #[inline]
    pub(crate) fn white(&mut self) -> f32 {
        self.dice.signed()
    }

    /// Pink noise, at about the same loudness as the white.
    #[inline]
    pub(crate) fn next(&mut self) -> f32 {
        let white = self.dice.signed();
        self.b[0] = 0.997_65 * self.b[0] + white * 0.099_046;
        self.b[1] = 0.963 * self.b[1] + white * 0.296_516_4;
        self.b[2] = 0.57 * self.b[2] + white * 1.052_691_3;
        (self.b[0] + self.b[1] + self.b[2] + white * 0.1848) * 0.2
    }
}

/// A feedback comb with a damped loop, for the room.
#[derive(Clone, Debug)]
struct Comb {
    buffer: Vec<f32>,
    at: usize,
    store: f32,
}

impl Comb {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length.max(1)],
            at: 0,
            store: 0.0,
        }
    }

    #[inline]
    fn run(&mut self, input: f32, feedback: f32, damp: f32) -> f32 {
        let output = self.buffer[self.at];
        self.store = output * (1.0 - damp) + self.store * damp;
        self.buffer[self.at] = input + NOT_DENORMAL + self.store * feedback;
        self.at += 1;
        if self.at == self.buffer.len() {
            self.at = 0;
        }
        output
    }
}

#[derive(Clone, Debug)]
struct AllPass {
    buffer: Vec<f32>,
    at: usize,
}

impl AllPass {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length.max(1)],
            at: 0,
        }
    }

    #[inline]
    fn run(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[self.at];
        let output = delayed - input;
        self.buffer[self.at] = input + NOT_DENORMAL + delayed * 0.5;
        self.at += 1;
        if self.at == self.buffer.len() {
            self.at = 0;
        }
        output
    }
}

/// A small warm room (after Freeverb): four damped combs and two
/// all-passes a side, the right a little longer than the left so the
/// room is wide.
#[derive(Clone, Debug)]
pub(crate) struct Room {
    combs: [Vec<Comb>; 2],
    passes: [Vec<AllPass>; 2],
    feedback: f32,
    damp: f32,
}

impl Room {
    pub(crate) fn new(rate: f32) -> Self {
        let scale = rate / 44_100.0;
        let side = |spread: usize| {
            let combs = [1116, 1188, 1277, 1356]
                .iter()
                .map(|length| Comb::new(((length + spread) as f32 * scale) as usize))
                .collect();
            let passes = [556, 441]
                .iter()
                .map(|length| AllPass::new(((length + spread) as f32 * scale) as usize))
                .collect();
            (combs, passes)
        };
        let (left_combs, left_passes) = side(0);
        let (right_combs, right_passes) = side(23);
        Self {
            combs: [left_combs, right_combs],
            passes: [left_passes, right_passes],
            feedback: 0.8,
            damp: 0.42,
        }
    }

    /// The room's answer to one stereo sample sent into it.
    #[inline]
    pub(crate) fn run(&mut self, left: f32, right: f32) -> (f32, f32) {
        let input = (left + right) * 0.5 * 0.1;
        let mut out = [0.0_f32; 2];
        for (side, sum) in out.iter_mut().enumerate() {
            for comb in &mut self.combs[side] {
                *sum += comb.run(input, self.feedback, self.damp);
            }
            for pass in &mut self.passes[side] {
                *sum = pass.run(*sum);
            }
        }
        (out[0], out[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cheap_sine_is_a_sine() {
        let mut worst = 0.0_f32;
        for index in 0..=1000 {
            let turns = index as f32 / 1000.0;
            worst = worst.max((sine(turns) - (TAU * turns).sin()).abs());
        }
        assert!(worst < 2e-3, "{worst}");
    }

    #[test]
    fn dice_are_repeatable_and_fair() {
        let mut one = Dice::new(7);
        let mut two = Dice::new(7);
        let mut sum = 0.0;
        for _ in 0..10_000 {
            let value = one.unit();
            assert_eq!(value, two.unit());
            assert!((0.0..1.0).contains(&value));
            sum += value;
        }
        assert!((sum / 10_000.0 - 0.5).abs() < 0.02);
        assert_ne!(Dice::new(7).next_u32(), Dice::new(8).next_u32());
    }

    #[test]
    fn panning_keeps_the_power() {
        for position in [-1.0, -0.3, 0.0, 0.5, 1.0] {
            let (left, right) = pan(position);
            assert!((left * left + right * right - 1.0).abs() < 1e-5);
        }
        assert!(pan(-1.0).0 > 0.99 && pan(1.0).1 > 0.99);
    }
}
