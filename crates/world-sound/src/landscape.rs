//! A World's landscape: beds of sound that fit the place, each fading in
//! and out with the hour, the sky and the day.
//!
//! Every bed is made as it plays, from noise and a few sines shaped by
//! filters, with its own seeded dice for when a gull calls or a car passes,
//! so nothing ever repeats as a loop does, and nothing ever joins itself
//! with a seam.

use crate::dsp::{advance, decay, pan, sine, Dice, OnePole, Pink, Smoother, Svf};
use crate::tune::pentatonic;
use crate::{Place, Scene, Sky};

/// The beds a landscape can be made of.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Bed {
    /// The sea's swell, rising and falling in waves.
    Sea,
    /// Gulls calling now and then, far and near.
    Gulls,
    /// Rain on roofs: a hush and the patter of drops.
    Rain,
    /// Wind, gusting, and in a storm a thin howl.
    Wind,
    /// A crowd's murmur, on a festival day.
    Crowd,
    /// A low hum: a dome's machinery, or a place's own drone.
    Hum,
    /// Dust and grit ticking against a dome.
    Grit,
    /// Cars passing on a street, and the town's low rumble.
    Traffic,
    /// A distant arcade game's jingles, through a door.
    Arcade,
    /// Ice creaking, and now and then singing as it settles.
    Creak,
    /// Penguins braying across the ice.
    Penguins,
}

impl Bed {
    pub const ALL: [Bed; 11] = [
        Bed::Sea,
        Bed::Gulls,
        Bed::Rain,
        Bed::Wind,
        Bed::Crowd,
        Bed::Hum,
        Bed::Grit,
        Bed::Traffic,
        Bed::Arcade,
        Bed::Creak,
        Bed::Penguins,
    ];
}

const BEDS: usize = Bed::ALL.len();

/// How light it is outside at `hour`: 0 at one in the morning, 1 at one in
/// the afternoon, smoothly between.
pub fn daylight(hour: f32) -> f32 {
    0.5 - 0.5 * (std::f32::consts::TAU * (hour - 1.0) / 24.0).cos()
}

/// Near `centre` o'clock, within `width` hours, round the clock.
fn near(hour: f32, centre: f32, width: f32) -> f32 {
    let apart = (hour.rem_euclid(24.0) - centre).abs();
    let apart = apart.min(24.0 - apart);
    (1.0 - (apart / width).powi(2)).max(0.0)
}

/// What a sky does to the landscape.
struct Weather {
    wind: f32,
    rain: f32,
    swell: f32,
    birds: f32,
    dust: f32,
}

fn weather(sky: Sky) -> Weather {
    let (wind, rain, swell, birds, dust) = match sky {
        Sky::Clear => (0.3, 0.0, 0.3, 1.0, 0.08),
        Sky::Cloudy => (0.42, 0.0, 0.45, 0.7, 0.08),
        Sky::Fog => (0.18, 0.0, 0.35, 0.45, 0.05),
        Sky::Rain => (0.5, 0.7, 0.6, 0.25, 0.0),
        Sky::Snow => (0.55, 0.0, 0.4, 0.2, 0.0),
        Sky::Storm => (1.0, 1.0, 1.0, 0.0, 0.3),
        Sky::Dust => (0.9, 0.0, 0.5, 0.3, 1.0),
    };
    Weather {
        wind,
        rain,
        swell,
        birds,
        dust,
    }
}

/// How loud each bed is in a scene, from 0 to 1, in [`Bed::ALL`] order.
pub fn beds(scene: &Scene) -> [f32; BEDS] {
    let mut levels = [0.0; BEDS];
    let mut set = |bed: Bed, level: f32| levels[bed as usize] = level.clamp(0.0, 1.0);
    let hour = scene.hour;
    let light = daylight(hour);
    let sky = weather(scene.sky);
    // A festival crowd gathers through the day and thins after dark.
    let crowd = if scene.festival {
        0.35 + 0.65 * near(hour, 16.0, 9.0)
    } else {
        0.0
    };
    match scene.place {
        Place::Harbour => {
            set(Bed::Sea, 0.55 + 0.45 * sky.swell);
            set(Bed::Gulls, light.powf(1.2) * sky.birds);
            set(Bed::Rain, sky.rain);
            set(Bed::Wind, 0.8 * sky.wind);
            set(Bed::Crowd, crowd);
        }
        Place::Dome => {
            set(Bed::Hum, 1.0);
            set(Bed::Wind, 0.35 + 0.45 * sky.wind);
            set(Bed::Grit, sky.dust);
            set(Bed::Crowd, 0.6 * crowd);
        }
        Place::Street => {
            set(Bed::Traffic, 0.2 + 0.8 * near(hour, 14.0, 10.0));
            set(Bed::Arcade, 0.25 + 0.75 * near(hour, 18.0, 7.0));
            set(Bed::Rain, sky.rain);
            set(Bed::Wind, 0.6 * sky.wind);
            set(Bed::Crowd, crowd);
        }
        Place::Ice => {
            set(Bed::Wind, 0.45 + 0.55 * sky.wind);
            set(Bed::Creak, 0.8);
            set(
                Bed::Penguins,
                if scene.festival {
                    1.0
                } else {
                    0.3 + 0.7 * light
                },
            );
            set(Bed::Sea, 0.3);
            set(Bed::Rain, 0.5 * sky.rain);
        }
        Place::Open { .. } => {
            set(Bed::Wind, 0.4 + 0.6 * sky.wind);
            set(Bed::Hum, 0.6);
            set(Bed::Rain, sky.rain);
            set(Bed::Crowd, 0.8 * crowd);
        }
    }
    levels
}

/// How loud each bed is made at full, so that at 1 they sit together.
const GAIN: [f32; BEDS] = [
    1.1,  // sea
    0.16, // gulls
    0.9,  // rain
    1.4,  // wind
    1.1,  // crowd
    0.1,  // hum
    0.6,  // grit
    0.9,  // traffic
    0.1,  // arcade
    0.5,  // creak
    0.05, // penguins
];

pub(crate) struct Landscape {
    rate: f32,
    levels: [Smoother; BEDS],
    sea: Sea,
    gulls: Gulls,
    rain: Drips,
    wind: Wind,
    crowd: Crowd,
    hum: Hum,
    grit: Drips,
    traffic: Traffic,
    arcade: Arcade,
    creak: Creak,
    penguins: Penguins,
}

impl Landscape {
    pub(crate) fn new(rate: f32, seed: u64, control_rate: f32) -> Self {
        let seed = |bed: Bed| seed ^ (0x6265_6400 + bed as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        Self {
            rate,
            levels: std::array::from_fn(|_| Smoother::new(0.0, 1.2, control_rate)),
            sea: Sea::new(rate, seed(Bed::Sea), control_rate),
            gulls: Gulls::new(rate, seed(Bed::Gulls)),
            rain: Drips::rain(rate, seed(Bed::Rain), control_rate),
            wind: Wind::new(rate, seed(Bed::Wind), control_rate),
            crowd: Crowd::new(rate, seed(Bed::Crowd)),
            hum: Hum::new(rate, seed(Bed::Hum)),
            grit: Drips::grit(seed(Bed::Grit), control_rate),
            traffic: Traffic::new(rate, seed(Bed::Traffic)),
            arcade: Arcade::new(rate, seed(Bed::Arcade)),
            creak: Creak::new(rate, seed(Bed::Creak)),
            penguins: Penguins::new(rate, seed(Bed::Penguins)),
        }
    }

    pub(crate) fn set_scene(&mut self, scene: Option<&Scene>) {
        let targets = scene.map_or([0.0; BEDS], beds);
        for (level, target) in self.levels.iter_mut().zip(targets) {
            level.target = target;
        }
        if let Some(scene) = scene {
            let sky = weather(scene.sky);
            self.sea.rough.target = sky.swell;
            self.wind.strength.target = match scene.place {
                Place::Dome => 0.3 + 0.4 * sky.wind,
                _ => sky.wind,
            };
            self.rain.intensity.target = sky.rain.max(0.3);
            self.grit.intensity.target = sky.dust.max(0.1);
            self.traffic.busy = near(scene.hour, 14.0, 10.0);
            self.arcade.root = scene.tune.root as f32;
            self.hum.set_pitch(
                match scene.place {
                    Place::Open { hum } => hum,
                    _ => 58.0,
                },
                self.rate,
            );
        }
    }

    /// How loud each bed plays now, in [`Bed::ALL`] order.
    pub(crate) fn levels(&self) -> [f32; BEDS] {
        std::array::from_fn(|index| self.levels[index].value)
    }

    /// Adds every bed that can be heard into the two sides.
    pub(crate) fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let rate = self.rate;
        for bed in Bed::ALL {
            let level = &mut self.levels[bed as usize];
            let gain = level.step();
            if gain < 1e-4 && level.target == 0.0 {
                continue;
            }
            let gain = gain * GAIN[bed as usize];
            match bed {
                Bed::Sea => self.sea.render(gain, left, right, rate),
                Bed::Gulls => self.gulls.render(gain, left, right, rate),
                Bed::Rain => self.rain.render(gain, left, right, rate),
                Bed::Wind => self.wind.render(gain, left, right, rate),
                Bed::Crowd => self.crowd.render(gain, left, right),
                Bed::Hum => self.hum.render(gain, left, right, rate),
                Bed::Grit => self.grit.render(gain, left, right, rate),
                Bed::Traffic => self.traffic.render(gain, left, right, rate),
                Bed::Arcade => self.arcade.render(gain, left, right, rate),
                Bed::Creak => self.creak.render(gain, left, right, rate),
                Bed::Penguins => self.penguins.render(gain, left, right, rate),
            }
        }
    }
}

/// The sea: waves of low noise that swell and draw back, each a few
/// seconds long, with a hiss where they break. Rougher weather makes them
/// bigger, brighter and closer together.
struct Sea {
    dice: Dice,
    noise: [Pink; 2],
    body: [Svf; 2],
    hiss: [OnePole; 2],
    at: f32,
    length: f32,
    peak: f32,
    rough: Smoother,
}

impl Sea {
    fn new(rate: f32, seed: u64, control_rate: f32) -> Self {
        Self {
            dice: Dice::new(seed),
            noise: [Pink::new(seed ^ 1), Pink::new(seed ^ 2)],
            body: [Svf::new(400.0, 0.7, rate), Svf::new(420.0, 0.7, rate)],
            hiss: [OnePole::new(1800.0, rate), OnePole::new(1900.0, rate)],
            at: 0.0,
            length: 8.0,
            peak: 0.8,
            rough: Smoother::new(0.3, 2.0, control_rate),
        }
    }

    fn render(&mut self, gain: f32, left: &mut [f32], right: &mut [f32], rate: f32) {
        let rough = self.rough.step();
        let u = (self.at / self.length).min(1.0);
        let wave = (std::f32::consts::PI * u.powf(0.7)).sin();
        let swell = 0.3 + self.peak * wave * wave;
        let cutoff = 180.0 + 900.0 * swell * (0.6 + 0.6 * rough);
        self.body[0].set(cutoff, 0.7, rate);
        self.body[1].set(cutoff * 1.12, 0.7, rate);
        let spray = swell * swell * swell * 0.08;
        for (out_left, out_right) in left.iter_mut().zip(right.iter_mut()) {
            let mut side = [0.0; 2];
            for (index, value) in side.iter_mut().enumerate() {
                let body = self.body[index].low(self.noise[index].next());
                let hiss = self.hiss[index].high(self.noise[index].white());
                *value = body * swell + hiss * spray;
            }
            *out_left += side[0] * gain;
            *out_right += side[1] * gain;
        }
        self.at += left.len() as f32 / rate;
        if self.at >= self.length {
            self.at = 0.0;
            self.length = self.dice.range(6.0, 11.0) / (0.7 + 0.6 * rough);
            self.peak = self.dice.range(0.55, 1.0);
        }
    }
}

/// Wind: noise through two moving band-passes, gusting, with a thin howl
/// in the strongest gusts.
struct Wind {
    dice: Dice,
    noise: [Pink; 2],
    band: [Svf; 2],
    howl: Svf,
    gust: Smoother,
    wander: Smoother,
    next_change: u32,
    strength: Smoother,
}

impl Wind {
    fn new(rate: f32, seed: u64, control_rate: f32) -> Self {
        Self {
            dice: Dice::new(seed),
            noise: [Pink::new(seed ^ 1), Pink::new(seed ^ 2)],
            band: [Svf::new(400.0, 1.4, rate), Svf::new(460.0, 1.4, rate)],
            howl: Svf::new(900.0, 10.0, rate),
            gust: Smoother::new(0.5, 1.2, control_rate),
            wander: Smoother::new(0.5, 3.0, control_rate),
            next_change: 0,
            strength: Smoother::new(0.3, 1.5, control_rate),
        }
    }

    fn render(&mut self, gain: f32, left: &mut [f32], right: &mut [f32], rate: f32) {
        if self.next_change == 0 {
            self.gust.target = self.dice.range(0.15, 1.0);
            self.wander.target = self.dice.unit();
            self.next_change = (self.dice.range(1.5, 5.0) * rate / left.len().max(1) as f32) as u32;
        }
        self.next_change -= 1;
        let gust = self.gust.step();
        let wander = self.wander.step();
        let strength = self.strength.step();
        let strong = 0.6 + 0.7 * strength;
        let cutoff = (200.0 + 600.0 * (0.4 * wander + 0.6 * gust)) * strong;
        self.band[0].set(cutoff, 1.4, rate);
        self.band[1].set(cutoff * 1.15, 1.4, rate);
        self.howl.set(700.0 + 700.0 * wander, 12.0, rate);
        let body = (0.2 + 0.8 * gust) * (0.5 + 0.5 * strength);
        let howl = gust * gust * strength * strength * 0.5;
        for (out_left, out_right) in left.iter_mut().zip(right.iter_mut()) {
            let a = self.noise[0].next();
            let b = self.noise[1].next();
            let whistle = self.howl.band(a + b) * howl;
            *out_left += (self.band[0].band(a) * body + whistle) * gain;
            *out_right += (self.band[1].band(b) * body + whistle) * gain;
        }
    }
}

/// One drop: a tiny ringing tone that dies at once.
#[derive(Clone, Copy, Debug, Default)]
struct Drop {
    phase: f32,
    step: f32,
    level: f32,
    decay: f32,
    left: f32,
    right: f32,
}

/// Drops: rain on roof tiles, or grit on a dome. A soft hush beneath, and
/// many tiny tones, each struck at random and dying at once.
struct Drips {
    dice: Dice,
    drops: [Drop; 24],
    /// How many a second at full, the pitch range, how long each rings,
    /// how loud, and how many are low knocks on wood.
    per_second: f32,
    pitch: (f32, f32),
    ring: (f32, f32),
    loud: (f32, f32),
    knocks: f32,
    hush: Option<(Pink, OnePole, OnePole)>,
    intensity: Smoother,
}

impl Drips {
    fn rain(rate: f32, seed: u64, control_rate: f32) -> Self {
        Self {
            dice: Dice::new(seed),
            drops: [Drop::default(); 24],
            per_second: 220.0,
            pitch: (1800.0, 5200.0),
            ring: (0.003, 0.012),
            loud: (0.01, 0.05),
            knocks: 0.15,
            hush: Some((
                Pink::new(seed ^ 7),
                OnePole::new(700.0, rate),
                OnePole::new(5500.0, rate),
            )),
            intensity: Smoother::new(0.7, 1.5, control_rate),
        }
    }

    fn grit(seed: u64, control_rate: f32) -> Self {
        Self {
            dice: Dice::new(seed),
            drops: [Drop::default(); 24],
            per_second: 400.0,
            pitch: (4500.0, 9000.0),
            ring: (0.0008, 0.002),
            loud: (0.01, 0.05),
            knocks: 0.0,
            hush: None,
            intensity: Smoother::new(0.1, 1.5, control_rate),
        }
    }

    fn render(&mut self, gain: f32, left: &mut [f32], right: &mut [f32], rate: f32) {
        let intensity = self.intensity.step();
        let chance = self.per_second * (0.15 + 0.85 * intensity) / rate;
        let hush_level = 0.12 * intensity;
        for (out_left, out_right) in left.iter_mut().zip(right.iter_mut()) {
            if self.dice.unit() < chance {
                if let Some(slot) = self.drops.iter_mut().find(|drop| drop.level < 1e-4) {
                    let knock = self.dice.chance(self.knocks);
                    let hertz = if knock {
                        self.dice.range(450.0, 1100.0)
                    } else {
                        self.dice.range(self.pitch.0, self.pitch.1)
                    };
                    let ring =
                        self.dice.range(self.ring.0, self.ring.1) * if knock { 2.0 } else { 1.0 };
                    let (l, r) = pan(self.dice.signed() * 0.9);
                    // Each starts at a zero crossing of its sine, so it
                    // never clicks however suddenly it comes.
                    *slot = Drop {
                        phase: 0.0,
                        step: hertz / rate,
                        level: self.dice.range(self.loud.0, self.loud.1),
                        decay: decay(ring, rate),
                        left: l,
                        right: r,
                    };
                }
            }
            let (mut sum_left, mut sum_right) = (0.0, 0.0);
            for drop in &mut self.drops {
                if drop.level < 1e-4 {
                    continue;
                }
                let value = drop.level * sine(drop.phase);
                advance(&mut drop.phase, drop.step);
                drop.level *= drop.decay;
                sum_left += value * drop.left;
                sum_right += value * drop.right;
            }
            if let Some((noise, high, low)) = self.hush.as_mut() {
                let hush = low.low(high.high(noise.next())) * hush_level;
                sum_left += hush;
                sum_right += hush;
            }
            *out_left += sum_left * gain;
            *out_right += sum_right * gain;
        }
    }
}

/// Gulls: now and then a call of a few rising and falling cries, somewhere
/// along the shore.
struct Gulls {
    dice: Dice,
    wait: f32,
    cries_left: u32,
    at: f32,
    length: f32,
    gap: f32,
    pitch: f32,
    phases: [f32; 3],
    rough: f32,
    left: f32,
    right: f32,
    distance: f32,
    soften: OnePole,
}

impl Gulls {
    fn new(rate: f32, seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        Self {
            wait: dice.range(2.0, 8.0),
            dice,
            cries_left: 0,
            at: 0.0,
            length: 0.2,
            gap: 0.15,
            pitch: 1200.0,
            phases: [0.0; 3],
            rough: 0.0,
            left: 0.7,
            right: 0.7,
            distance: 1.0,
            soften: OnePole::new(3500.0, rate),
        }
    }

    fn render(&mut self, gain: f32, left: &mut [f32], right: &mut [f32], rate: f32) {
        let tick = 1.0 / rate;
        for (out_left, out_right) in left.iter_mut().zip(right.iter_mut()) {
            if self.cries_left == 0 {
                self.wait -= tick;
                if self.wait <= 0.0 {
                    self.cries_left = 2 + self.dice.pick(3) as u32;
                    self.pitch = self.dice.range(1000.0, 1400.0);
                    self.distance = self.dice.range(0.3, 1.0);
                    (self.left, self.right) = pan(self.dice.signed() * 0.8);
                    self.at = 0.0;
                    self.length = self.dice.range(0.18, 0.32);
                    self.gap = self.dice.range(0.1, 0.25);
                    self.phases = [0.0; 3];
                }
                let quiet = self.soften.low(0.0);
                *out_left += quiet * gain;
                *out_right += quiet * gain;
                continue;
            }
            let mut value = 0.0;
            if self.at < self.length {
                let u = self.at / self.length;
                // Up quickly to the cry, then falling away.
                let bend = if u < 0.3 {
                    1.0 + 0.4 * u / 0.3
                } else {
                    1.4 - 0.6 * (u - 0.3) / 0.7
                };
                let hertz = self.pitch * bend;
                let shape = (std::f32::consts::PI * u.powf(0.6)).sin();
                advance(&mut self.rough, 38.0 * tick);
                let rough = 1.0 + 0.25 * sine(self.rough);
                value = (sine(self.phases[0])
                    + 0.5 * sine(self.phases[1])
                    + 0.2 * sine(self.phases[2]))
                    * shape
                    * shape
                    * rough
                    * self.distance;
                for (index, phase) in self.phases.iter_mut().enumerate() {
                    advance(phase, hertz * (index + 1) as f32 * tick);
                }
            }
            self.at += tick;
            if self.at >= self.length + self.gap {
                self.cries_left -= 1;
                self.at = 0.0;
                self.length = self.dice.range(0.15, 0.3);
                self.pitch *= self.dice.range(0.93, 1.05);
                self.phases = [0.0; 3];
                if self.cries_left == 0 {
                    self.wait = self.dice.range(5.0, 18.0);
                }
            }
            let value = self.soften.low(value);
            *out_left += value * self.left * gain;
            *out_right += value * self.right * gain;
        }
    }
}

/// One voice in a crowd: a murmur of vowels, on and off like speech.
struct Talker {
    noise: Pink,
    formant: Svf,
    gate: f32,
    target: f32,
    timer: u32,
    left: f32,
    right: f32,
}

/// A crowd's murmur: six voices talking over each other.
struct Crowd {
    dice: Dice,
    talkers: Vec<Talker>,
    glide: f32,
    rate: f32,
}

impl Crowd {
    fn new(rate: f32, seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        let talkers = (0..6)
            .map(|index| {
                let (left, right) = pan(-0.8 + 0.32 * index as f32);
                Talker {
                    noise: Pink::new(seed ^ (index + 11)),
                    formant: Svf::new(dice.range(380.0, 900.0), 3.0, rate),
                    gate: 0.0,
                    target: 0.0,
                    timer: 1,
                    left,
                    right,
                }
            })
            .collect();
        Self {
            dice,
            talkers,
            glide: 1.0 - decay(0.02, rate),
            rate,
        }
    }

    fn render(&mut self, gain: f32, left: &mut [f32], right: &mut [f32]) {
        for (out_left, out_right) in left.iter_mut().zip(right.iter_mut()) {
            let (mut sum_left, mut sum_right) = (0.0, 0.0);
            for talker in &mut self.talkers {
                talker.timer -= 1;
                if talker.timer == 0 {
                    // A syllable, or now and then a pause for breath.
                    let (target, seconds) = if self.dice.chance(0.12) {
                        (0.0, self.dice.range(0.3, 1.4))
                    } else {
                        (self.dice.range(0.3, 1.0), self.dice.range(0.06, 0.18))
                    };
                    talker.target = target;
                    talker.timer = ((seconds * self.rate) as u32).max(1);
                }
                talker.gate += self.glide * (talker.target - talker.gate);
                if talker.gate < 1e-6 && talker.target == 0.0 {
                    talker.gate = 0.0;
                }
                let value = talker.formant.band(talker.noise.next()) * talker.gate;
                sum_left += value * talker.left;
                sum_right += value * talker.right;
            }
            *out_left += sum_left * gain;
            *out_right += sum_right * gain;
        }
    }
}

/// A low hum with a slow beat in it, and air moving through vents.
struct Hum {
    phases: [f32; 4],
    steps: [f32; 4],
    swell: f32,
    vent: Pink,
    vent_low: OnePole,
}

impl Hum {
    fn new(rate: f32, seed: u64) -> Self {
        let mut hum = Self {
            phases: [0.0, 0.25, 0.5, 0.75],
            steps: [0.0; 4],
            swell: 0.0,
            vent: Pink::new(seed),
            vent_low: OnePole::new(260.0, rate),
        };
        hum.set_pitch(58.0, rate);
        hum
    }

    fn set_pitch(&mut self, hertz: f32, rate: f32) {
        // Changing only the step keeps the phase, so a new pitch never clicks.
        self.steps = [1.0, 2.0, 3.0, 2.009].map(|ratio| hertz * ratio / rate);
    }

    fn render(&mut self, gain: f32, left: &mut [f32], right: &mut [f32], rate: f32) {
        const WEIGHTS: [f32; 4] = [1.0, 0.55, 0.2, 0.4];
        let step = 0.07 / rate;
        for (out_left, out_right) in left.iter_mut().zip(right.iter_mut()) {
            let mut value = 0.0;
            for ((phase, step), weight) in self.phases.iter_mut().zip(self.steps).zip(WEIGHTS) {
                value += weight * sine(*phase);
                advance(phase, step);
            }
            advance(&mut self.swell, step);
            let value = value * (0.85 + 0.15 * sine(self.swell));
            let air = self.vent_low.low(self.vent.next()) * 3.0;
            *out_left += (value + air) * gain;
            *out_right += (value * 0.9 + air) * gain;
        }
    }
}

/// A car passing: an engine's low note and its tyres, coming from one
/// side, loudest in front, going to the other, its note falling as it
/// goes.
struct Car {
    at: f32,
    length: f32,
    way: f32,
    engine: f32,
    phases: [f32; 2],
    loud: f32,
}

struct Traffic {
    dice: Dice,
    car: Option<Car>,
    wait: f32,
    rumble: Pink,
    rumble_low: OnePole,
    tyres: Pink,
    tyre_band: Svf,
    busy: f32,
}

impl Traffic {
    fn new(rate: f32, seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        Self {
            wait: dice.range(1.0, 5.0),
            dice,
            car: None,
            rumble: Pink::new(seed ^ 3),
            rumble_low: OnePole::new(110.0, rate),
            tyres: Pink::new(seed ^ 5),
            tyre_band: Svf::new(750.0, 0.8, rate),
            busy: 0.5,
        }
    }

    fn render(&mut self, gain: f32, left: &mut [f32], right: &mut [f32], rate: f32) {
        let seconds = left.len() as f32 / rate;
        let (mut car_left, mut car_right, mut near, mut bend) = (0.0, 0.0, 0.0, 1.0);
        if let Some(car) = self.car.as_ref() {
            let u = (car.at / car.length).min(1.0);
            let x = car.way * (-1.0 + 2.0 * u);
            // In from nothing and out to nothing, loudest in front.
            let edges = (std::f32::consts::PI * u).sin();
            near = car.loud * edges * edges / (1.0 + 5.0 * x * x);
            (car_left, car_right) = pan(x * 0.85);
            bend = 1.0 - 0.035 * x * car.way;
        }
        for (out_left, out_right) in left.iter_mut().zip(right.iter_mut()) {
            let town = self.rumble_low.low(self.rumble.next()) * 0.9;
            let mut passing = 0.0;
            if let Some(car) = self.car.as_mut() {
                let engine = sine(car.phases[0]) + 0.5 * sine(car.phases[1]);
                advance(&mut car.phases[0], car.engine * bend / rate);
                advance(&mut car.phases[1], car.engine * 2.0 * bend / rate);
                passing = (engine * 0.25 + self.tyre_band.band(self.tyres.next()) * 1.4) * near;
            }
            *out_left += (town + passing * car_left) * gain;
            *out_right += (town + passing * car_right) * gain;
        }
        match self.car.as_mut() {
            Some(car) => {
                car.at += seconds;
                if car.at >= car.length {
                    self.car = None;
                    self.wait = self.dice.range(3.0, 14.0) / (0.3 + self.busy);
                }
            }
            None => {
                self.wait -= seconds;
                if self.wait <= 0.0 {
                    self.car = Some(Car {
                        at: 0.0,
                        length: self.dice.range(3.0, 6.0),
                        way: if self.dice.chance(0.5) { 1.0 } else { -1.0 },
                        engine: self.dice.range(42.0, 72.0),
                        phases: [0.0; 2],
                        loud: self.dice.range(0.5, 1.0),
                    });
                }
            }
        }
    }
}

/// An arcade across the street: now and then a game's little jingle in the
/// World's own key, soft, as if through an open door.
struct Arcade {
    dice: Dice,
    wait: f32,
    notes_left: u32,
    at: f32,
    length: f32,
    step: i32,
    way: i32,
    phases: [f32; 3],
    hertz: f32,
    root: f32,
    door: OnePole,
}

impl Arcade {
    fn new(rate: f32, seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        Self {
            wait: dice.range(2.0, 8.0),
            dice,
            notes_left: 0,
            at: 0.0,
            length: 0.1,
            step: 0,
            way: 1,
            phases: [0.0; 3],
            hertz: 440.0,
            root: 60.0,
            door: OnePole::new(1700.0, rate),
        }
    }

    fn next_note(&mut self) {
        if self.dice.chance(0.2) {
            self.way = -self.way;
        }
        self.step += self.way;
        if !(0..=7).contains(&self.step) {
            self.way = -self.way;
            self.step = self.step.clamp(0, 7);
        }
        self.hertz = crate::dsp::midi(self.root + 24.0 + pentatonic(self.step));
        self.at = 0.0;
    }

    fn render(&mut self, gain: f32, left: &mut [f32], right: &mut [f32], rate: f32) {
        let tick = 1.0 / rate;
        for (out_left, out_right) in left.iter_mut().zip(right.iter_mut()) {
            let mut value = 0.0;
            if self.notes_left > 0 {
                let envelope =
                    (self.at / 0.002).min(1.0) * ((self.length - self.at) / 0.01).clamp(0.0, 1.0);
                value = (sine(self.phases[0])
                    + sine(self.phases[1]) / 3.0
                    + sine(self.phases[2]) / 5.0)
                    * envelope;
                for (index, phase) in self.phases.iter_mut().enumerate() {
                    advance(phase, self.hertz * (2 * index + 1) as f32 * tick);
                }
                self.at += tick;
                if self.at >= self.length {
                    self.notes_left -= 1;
                    if self.notes_left == 0 {
                        self.wait = self.dice.range(5.0, 15.0);
                    } else {
                        self.next_note();
                    }
                }
            } else {
                self.wait -= tick;
                if self.wait <= 0.0 {
                    self.notes_left = 4 + self.dice.pick(7) as u32;
                    self.length = self.dice.range(0.08, 0.13);
                    self.step = self.dice.pick(5) as i32;
                    self.way = if self.dice.chance(0.5) { 1 } else { -1 };
                    self.next_note();
                }
            }
            let value = self.door.low(value);
            *out_left += value * 0.45 * gain;
            *out_right += value * 0.9 * gain;
        }
    }
}

/// Ice: a creak now and then, a slow stick-and-slip of pulses through a
/// ringing body, and rarely the high falling note ice sings as it settles.
struct Creak {
    dice: Dice,
    wait: f32,
    at: f32,
    length: f32,
    pulses: (f32, f32),
    pulse: f32,
    body: Svf,
    left: f32,
    right: f32,
    song_wait: f32,
    song_at: f32,
    song_phase: f32,
    song_hertz: f32,
}

impl Creak {
    fn new(rate: f32, seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        Self {
            wait: dice.range(2.0, 8.0),
            song_wait: dice.range(15.0, 40.0),
            dice,
            at: -1.0,
            length: 1.0,
            pulses: (15.0, 40.0),
            pulse: 0.0,
            body: Svf::new(400.0, 12.0, rate),
            left: 0.7,
            right: 0.7,
            song_at: -1.0,
            song_phase: 0.0,
            song_hertz: 1600.0,
        }
    }

    fn render(&mut self, gain: f32, left: &mut [f32], right: &mut [f32], rate: f32) {
        let tick = 1.0 / rate;
        let song_ring = decay(0.25, rate);
        for (out_left, out_right) in left.iter_mut().zip(right.iter_mut()) {
            let creak = if self.at >= 0.0 {
                let u = self.at / self.length;
                let per_second = self.pulses.0 + (self.pulses.1 - self.pulses.0) * u;
                self.pulse += per_second * tick;
                let strike = if self.pulse >= 1.0 {
                    self.pulse -= 1.0;
                    self.dice.range(0.6, 1.0)
                } else {
                    0.0
                };
                let shape = (std::f32::consts::PI * u).sin();
                let creak = self.body.band(strike + self.dice.signed() * 0.01) * shape * shape;
                self.at += tick;
                if self.at >= self.length {
                    self.at = -1.0;
                    self.wait = self.dice.range(6.0, 22.0);
                }
                creak
            } else {
                // The body keeps ringing out after the last pulse.
                let creak = self.body.band(0.0);
                self.wait -= tick;
                if self.wait <= 0.0 {
                    self.at = 0.0;
                    self.length = self.dice.range(0.5, 1.4);
                    self.pulses = if self.dice.chance(0.5) {
                        (self.dice.range(12.0, 20.0), self.dice.range(35.0, 60.0))
                    } else {
                        (self.dice.range(35.0, 60.0), self.dice.range(12.0, 20.0))
                    };
                    self.body.set(self.dice.range(250.0, 600.0), 12.0, rate);
                    (self.left, self.right) = pan(self.dice.signed() * 0.7);
                }
                creak
            };
            let mut song = 0.0;
            if self.song_at >= 0.0 {
                let level = (self.song_at / 0.003).min(1.0) * song_ring.powf(self.song_at * rate);
                song = sine(self.song_phase) * level * 0.15;
                advance(&mut self.song_phase, self.song_hertz * tick);
                // Falling from high to low, as far-off ice sings.
                self.song_hertz *= 1.0 - 2.5 * tick;
                self.song_at += tick;
                if self.song_at > 1.5 {
                    self.song_at = -1.0;
                    self.song_wait = self.dice.range(25.0, 60.0);
                }
            } else {
                self.song_wait -= tick;
                if self.song_wait <= 0.0 {
                    self.song_at = 0.0;
                    self.song_phase = 0.0;
                    self.song_hertz = self.dice.range(1300.0, 1900.0);
                }
            }
            *out_left += (creak * self.left + song * 0.6) * gain;
            *out_right += (creak * self.right + song) * gain;
        }
    }
}

/// Penguins: now and then one brays, a few buzzing honks rising and
/// falling, somewhere across the ice.
struct Penguins {
    dice: Dice,
    wait: f32,
    honks_left: u32,
    at: f32,
    length: f32,
    gap: f32,
    pitch: f32,
    phases: [f32; 5],
    flutter: f32,
    throat: Svf,
    far: OnePole,
    left: f32,
    right: f32,
    loud: f32,
}

impl Penguins {
    fn new(rate: f32, seed: u64) -> Self {
        let mut dice = Dice::new(seed);
        Self {
            wait: dice.range(2.0, 9.0),
            dice,
            honks_left: 0,
            at: 0.0,
            length: 0.25,
            gap: 0.08,
            pitch: 340.0,
            phases: [0.0; 5],
            flutter: 0.0,
            throat: Svf::new(900.0, 1.5, rate),
            far: OnePole::new(2600.0, rate),
            left: 0.7,
            right: 0.7,
            loud: 1.0,
        }
    }

    fn render(&mut self, gain: f32, left: &mut [f32], right: &mut [f32], rate: f32) {
        const WEIGHTS: [f32; 5] = [1.0, 0.7, 0.5, 0.35, 0.2];
        let tick = 1.0 / rate;
        for (out_left, out_right) in left.iter_mut().zip(right.iter_mut()) {
            let mut value = 0.0;
            if self.honks_left > 0 {
                if self.at < self.length {
                    let u = self.at / self.length;
                    let hertz = self.pitch * (1.0 + 0.3 * u - 0.4 * u * u);
                    let shape = (std::f32::consts::PI * u).sin();
                    advance(&mut self.flutter, 28.0 * tick);
                    let flutter = 1.0 + 0.3 * sine(self.flutter);
                    for (index, (phase, weight)) in self.phases.iter_mut().zip(WEIGHTS).enumerate()
                    {
                        value += weight * sine(*phase);
                        advance(phase, hertz * (index + 1) as f32 * tick);
                    }
                    value *= shape * flutter * self.loud;
                }
                self.at += tick;
                if self.at >= self.length + self.gap {
                    self.honks_left -= 1;
                    self.at = 0.0;
                    self.phases = [0.0; 5];
                    self.length = if self.honks_left == 1 {
                        self.dice.range(0.4, 0.6)
                    } else {
                        self.dice.range(0.15, 0.32)
                    };
                    self.gap = self.dice.range(0.05, 0.12);
                    if self.honks_left == 0 {
                        self.wait = self.dice.range(7.0, 20.0);
                    }
                }
            } else {
                self.wait -= tick;
                if self.wait <= 0.0 {
                    self.honks_left = 3 + self.dice.pick(4) as u32;
                    self.pitch = self.dice.range(280.0, 420.0);
                    self.loud = self.dice.range(0.35, 1.0);
                    self.at = 0.0;
                    self.length = self.dice.range(0.15, 0.3);
                    self.phases = [0.0; 5];
                    (self.left, self.right) = pan(self.dice.signed() * 0.8);
                }
            }
            let voiced = value * 0.4 + self.throat.band(value) * 1.4;
            let value = self.far.low(voiced);
            *out_left += value * self.left * gain;
            *out_right += value * self.right * gain;
        }
    }
}
