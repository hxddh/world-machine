//! The bars the sound is held to: no clicks, answers within a few
//! milliseconds, cheap to render, the same for the same seed, following
//! the hour and the sky within two seconds, and a landscape for each place.

use std::time::Instant;
use world_sound::{
    babble, Act, Bed, Command, Engine, Levels, Place, Scene, Sky, Tune, BUFFER_FRAMES, RATE,
};

const HARBOUR: [u32; 5] = [0xa9cfe6, 0xe9f1ef, 0x7f9f8a, 0x2f6a86, 0xffe2a0];
const MARS: [u32; 5] = [0xe7b089, 0xf5d9bd, 0xc2663f, 0x8a3a22, 0xfff3dc];
const TOWN: [u32; 5] = [0x241d45, 0x6b4a7a, 0x3a2f55, 0x1b1630, 0xf4bf5c];
const ICE: [u32; 5] = [0x14305a, 0x3f8f95, 0xa9cbdb, 0xe6f1f6, 0xb9f3d3];

const FULL: Levels = Levels {
    music: 1.0,
    ambience: 1.0,
    voices: 1.0,
    interface: 1.0,
};

fn scene(place: Place, palette: [u32; 5], hour: f32, sky: Sky, festival: bool) -> Scene {
    Scene {
        place,
        tune: Tune::of(palette),
        hour,
        sky,
        festival,
    }
}

fn harbour(hour: f32, sky: Sky) -> Scene {
    scene(Place::Harbour, HARBOUR, hour, sky, false)
}

fn engine(seed: u64, levels: Levels, scene: Option<Scene>) -> Engine {
    let mut engine = Engine::new(RATE as f32, seed);
    engine.apply(Command::Levels(levels));
    engine.apply(Command::Scene(scene));
    engine
}

fn rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt()
}

fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0_f32, |peak, s| peak.max(s.abs()))
}

/// The largest jump between neighbouring samples on either side.
fn largest_step(samples: &[f32]) -> (f32, usize) {
    let mut worst = (0.0_f32, 0);
    for side in 0..2 {
        let mut previous = samples[side];
        for (index, sample) in samples.iter().skip(side).step_by(2).enumerate() {
            let step = (sample - previous).abs();
            if step > worst.0 {
                worst = (step, index);
            }
            previous = *sample;
        }
    }
    worst
}

/// Clicks: a jump between two samples that stands far out from the jumps
/// around it. Rain and paper move quickly all the time, and are not clicks;
/// a sound cut off or started mid-wave is a single jump where its
/// neighbours are small. Returns the worst jump's size, how many times the
/// jumps within a millisecond either side it is, and where it is.
fn worst_click(samples: &[f32], rate: f32) -> (f32, f32, usize) {
    let half = (rate / 1000.0) as usize;
    let mut worst = (0.0_f32, 0.0_f32, 0);
    for side in 0..2 {
        let signal = samples
            .iter()
            .skip(side)
            .step_by(2)
            .copied()
            .collect::<Vec<_>>();
        let jumps = signal
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .collect::<Vec<_>>();
        let mut energy = vec![0.0_f64; jumps.len() + 1];
        for (index, jump) in jumps.iter().enumerate() {
            energy[index + 1] = energy[index] + f64::from(jump * jump);
        }
        for (index, jump) in jumps.iter().enumerate() {
            if jump.abs() < 0.01 {
                continue;
            }
            let from = index.saturating_sub(half);
            let to = (index + half + 1).min(jumps.len());
            let around = energy[to] - energy[from] - f64::from(jump * jump);
            let local = (around / (to - from - 1) as f64).sqrt() as f32;
            let ratio = jump.abs() / local.max(1e-6);
            if ratio > worst.1 {
                worst = (jump.abs(), ratio, index);
            }
        }
    }
    worst
}

/// A minute of everything at once: every place in turn, every sky, a
/// festival, dusk, an act every 0.7 s and someone talking every 3 s.
fn busy_minute(seed: u64) -> Vec<f32> {
    let mut engine = engine(seed, FULL, Some(harbour(12.0, Sky::Clear)));
    let rate = RATE as f32;
    let changes: [(f32, Scene); 8] = [
        (8.0, harbour(12.0, Sky::Rain)),
        (16.0, harbour(12.0, Sky::Storm)),
        (24.0, scene(Place::Harbour, HARBOUR, 12.0, Sky::Clear, true)),
        (32.0, scene(Place::Dome, MARS, 15.0, Sky::Dust, false)),
        (38.0, scene(Place::Street, TOWN, 19.0, Sky::Cloudy, false)),
        (44.0, scene(Place::Street, TOWN, 21.0, Sky::Rain, true)),
        (50.0, scene(Place::Ice, ICE, 2.0, Sky::Snow, false)),
        (
            55.0,
            scene(Place::Open { hum: 70.0 }, HARBOUR, 7.0, Sky::Fog, false),
        ),
    ];
    let mut out = Vec::with_capacity(60 * RATE as usize * 2);
    let mut block = vec![0.0; BUFFER_FRAMES as usize * 2];
    let mut next_act = 0.3;
    let mut next_say = 1.0;
    let mut acts = Act::ALL.iter().cycle();
    while out.len() < 60 * RATE as usize * 2 {
        let now = out.len() as f32 / 2.0 / rate;
        for (at, change) in &changes {
            if now <= *at && *at < now + BUFFER_FRAMES as f32 / rate {
                engine.apply(Command::Scene(Some(change.clone())));
            }
        }
        if now >= next_act {
            engine.apply(Command::Act(*acts.next().unwrap()));
            next_act += 0.7;
        }
        if now >= next_say {
            let who = (now * 1000.0) as u32;
            engine.apply(Command::Say {
                samples: babble(who, 6, who.is_multiple_of(2), 0, rate),
                place: 0.2,
            });
            next_say += 3.0;
        }
        engine.render(&mut block);
        out.extend_from_slice(&block);
    }
    out
}

#[test]
fn a_busy_minute_never_clicks_clips_or_falls_silent() {
    let minute = busy_minute(7);
    assert!(minute.iter().all(|sample| sample.is_finite()));
    let (step, at) = largest_step(&minute);
    println!(
        "largest step between samples: {step:.4} at {:.3} s; peak {:.3}; rms {:.4}",
        at as f32 / RATE as f32,
        peak(&minute),
        rms(&minute)
    );
    let (jump, ratio, where_) = worst_click(&minute, RATE as f32);
    println!(
        "most click-like jump: {jump:.4}, {ratio:.1} times its surroundings, at {:.3} s",
        where_ as f32 / RATE as f32
    );
    // No jump stands out eight times from the millisecond around it, and
    // none at all is large enough to be a gross fault. (Rain and spray in
    // a storm, with every channel at twice or more its fullest setting in
    // the app, move up to about 0.4 between samples, and that is hiss.)
    assert!(ratio < 8.0, "a click of {jump} at {where_}");
    assert!(step < 0.6, "a step of {step} at {at}");
    assert!(peak(&minute) < 0.95, "never at full scale");
    for (index, second) in minute.chunks(RATE as usize * 2).enumerate() {
        assert!(
            rms(second) > 0.003,
            "second {index} is silent: {}",
            rms(second)
        );
    }
}

#[test]
fn the_same_seed_renders_the_same_samples() {
    let one = busy_minute(11);
    assert_eq!(one, busy_minute(11), "deterministic given a seed");
    let other = busy_minute(12);
    assert_ne!(one, other, "and another seed plays another way");
}

#[test]
fn an_act_is_heard_within_a_few_milliseconds() {
    // With everything else playing, an act lands in the next callback.
    for act in Act::ALL {
        let mut quiet = engine(3, FULL, None);
        let rate = RATE as f32;
        quiet.render_seconds(0.2);
        quiet.apply(Command::Act(act));
        let heard = quiet.render_seconds(0.05);
        let first = heard
            .iter()
            .position(|sample| sample.abs() > 1e-3)
            .map(|index| index as f32 / 2.0 / rate)
            .unwrap_or(f32::MAX);
        println!(
            "{act:?}: first heard {:.2} ms after its callback",
            first * 1000.0
        );
        assert!(first < 0.004, "{act:?}: {first}");
    }
    // The worst case from a click to the device: the rest of the callback
    // being rendered, the next one, and the note's first milliseconds.
    let buffer = BUFFER_FRAMES as f32 / RATE as f32;
    let worst = 2.0 * buffer + 0.004;
    println!(
        "buffer {BUFFER_FRAMES} frames = {:.2} ms; worst case in the graph {:.2} ms",
        buffer * 1000.0,
        worst * 1000.0
    );
    assert!(worst < 0.030);
}

#[test]
fn every_act_is_different_each_time_and_in_the_key() {
    // Played twice running, an act never sounds the same.
    for act in Act::ALL {
        let mut graph = engine(9, FULL, None);
        let mut takes = Vec::new();
        for _ in 0..6 {
            graph.apply(Command::Act(act));
            takes.push(graph.render_seconds(1.2));
        }
        for pair in takes.windows(2) {
            let difference: f32 = pair[0]
                .iter()
                .zip(&pair[1])
                .map(|(a, b)| (a - b).abs())
                .sum();
            assert!(difference > 1.0, "{act:?} repeated itself");
        }
        let loud = peak(&takes[0]);
        assert!(loud > 0.05 && loud < 0.9, "{act:?}: {loud}");
    }
}

#[test]
fn the_players_levels_scale_their_channels() {
    let render = |levels: Levels| {
        let mut graph = engine(5, levels, None);
        graph.apply(Command::Act(Act::Answer));
        graph.render_seconds(1.0)
    };
    let full = rms(&render(FULL));
    let half = rms(&render(Levels {
        interface: 0.5,
        ..FULL
    }));
    assert!((half / full - 0.5).abs() < 0.02, "{half} / {full}");
    let off = render(Levels {
        interface: 0.0,
        ..FULL
    });
    assert!(peak(&off) < 1e-4, "a channel at nothing is silent");

    // Music off leaves the landscape; the landscape off leaves the music.
    let place = |levels: Levels| {
        let mut graph = engine(5, levels, Some(harbour(13.0, Sky::Clear)));
        graph.render_seconds(6.0)
    };
    let everything = rms(&place(FULL));
    let landscape = rms(&place(Levels { music: 0.0, ..FULL }));
    let music = rms(&place(Levels {
        ambience: 0.0,
        ..FULL
    }));
    println!("harbour at noon: all {everything:.4}, landscape {landscape:.4}, music {music:.4}");
    assert!(landscape > 0.005 && music > 0.005);
    assert!(everything > landscape && everything > music);
    let silent = place(Levels::default());
    assert!(peak(&silent) < 1e-4, "sound off is silence");
}

#[test]
fn a_change_of_hour_or_sky_is_heard_within_two_seconds() {
    let mut graph = engine(2, FULL, Some(harbour(13.0, Sky::Clear)));
    graph.render_seconds(10.0);
    let [_, day, evening, _] = graph.music_levels();
    assert!(day > 0.9 && evening < 0.05, "{day} {evening}");
    // A storm comes: the plucks go.
    graph.apply(Command::Scene(Some(harbour(13.0, Sky::Storm))));
    graph.render_seconds(2.0);
    let [_, day, _, _] = graph.music_levels();
    assert!(day < 0.05, "plucks after a storm came: {day}");
    let sea = graph.bed_levels()[Bed::Sea as usize];
    let rain = graph.bed_levels()[Bed::Rain as usize];
    assert!(rain > 0.6 && sea > 0.8, "rain {rain}, sea {sea}");
    // Evening falls: the hum takes over.
    graph.apply(Command::Scene(Some(harbour(21.0, Sky::Clear))));
    graph.render_seconds(2.0);
    let [_, day, evening, _] = graph.music_levels();
    assert!(day < 0.05 && evening > 0.9, "{day} {evening}");
}

#[test]
fn each_place_has_a_landscape_of_its_own() {
    let levels = |scene: Scene| world_sound::beds(&scene);
    let on = |levels: [f32; 11], bed: Bed| levels[bed as usize] > 0.1;
    let noon = levels(harbour(13.0, Sky::Clear));
    assert!(on(noon, Bed::Sea) && on(noon, Bed::Gulls) && on(noon, Bed::Wind));
    assert!(!on(noon, Bed::Rain) && !on(noon, Bed::Crowd));
    let rain = levels(harbour(13.0, Sky::Rain));
    assert!(on(rain, Bed::Rain) && rain[Bed::Gulls as usize] < noon[Bed::Gulls as usize]);
    let storm = levels(harbour(13.0, Sky::Storm));
    assert!(
        storm[Bed::Sea as usize] > noon[Bed::Sea as usize],
        "the swell follows the sky"
    );
    assert!(
        !on(levels(harbour(1.0, Sky::Clear)), Bed::Gulls),
        "no gulls at night"
    );
    let festival = levels(scene(Place::Harbour, HARBOUR, 17.0, Sky::Clear, true));
    assert!(on(festival, Bed::Crowd), "a crowd on a festival day");

    let mars = levels(scene(Place::Dome, MARS, 13.0, Sky::Dust, false));
    assert!(on(mars, Bed::Hum) && on(mars, Bed::Grit) && on(mars, Bed::Wind));
    assert!(!on(mars, Bed::Sea) && !on(mars, Bed::Gulls));
    let street = levels(scene(Place::Street, TOWN, 18.0, Sky::Clear, false));
    assert!(on(street, Bed::Traffic) && on(street, Bed::Arcade));
    let ice = levels(scene(Place::Ice, ICE, 13.0, Sky::Snow, false));
    assert!(on(ice, Bed::Creak) && on(ice, Bed::Penguins) && on(ice, Bed::Wind));

    // And each sounds: different, and never silent.
    let mut heard = Vec::new();
    for (place, palette) in [
        (Place::Harbour, HARBOUR),
        (Place::Dome, MARS),
        (Place::Street, TOWN),
        (Place::Ice, ICE),
        (Place::Open { hum: 66.0 }, HARBOUR),
    ] {
        let mut graph = engine(
            4,
            Levels {
                ambience: 1.0,
                ..Levels::default()
            },
            Some(scene(place, palette, 13.0, Sky::Clear, false)),
        );
        graph.render_seconds(3.0);
        let sound = graph.render_seconds(20.0);
        let loudness = rms(&sound);
        println!("{place:?}: rms {loudness:.4}, peak {:.3}", peak(&sound));
        assert!(
            loudness > 0.004 && peak(&sound) < 0.9,
            "{place:?}: {loudness}"
        );
        heard.push(sound);
    }
    for (index, one) in heard.iter().enumerate() {
        for other in &heard[index + 1..] {
            assert_ne!(one, other);
        }
    }
}

#[test]
fn rendering_the_full_mix_costs_little() {
    // The busiest place, a festival, rain, and acts and voices over it.
    let mut graph = engine(
        8,
        FULL,
        Some(scene(Place::Harbour, HARBOUR, 17.0, Sky::Rain, true)),
    );
    graph.render_seconds(2.0);
    let rate = RATE as f32;
    let seconds = 10;
    let mut block = vec![0.0; BUFFER_FRAMES as usize * 2];
    let blocks = seconds * RATE as usize / BUFFER_FRAMES as usize;
    let started = Instant::now();
    for index in 0..blocks {
        if index % 60 == 0 {
            graph.apply(Command::Act(Act::ALL[index / 60 % Act::ALL.len()]));
        }
        if index % 300 == 0 {
            graph.apply(Command::Say {
                samples: babble(index as u32, 8, false, 0, rate),
                place: -0.2,
            });
        }
        graph.render(&mut block);
    }
    let share = started.elapsed().as_secs_f32() / seconds as f32;
    println!(
        "rendering 1 s of the full mix took {:.2} ms ({:.2}% of real time)",
        share * 1000.0,
        share * 100.0
    );
    // Release builds are held to 5%; unoptimised ones only to real time.
    let bar = if cfg!(debug_assertions) { 0.5 } else { 0.05 };
    assert!(share < bar, "{share}");
}

#[test]
fn a_world_leaving_fades_rather_than_stops() {
    let mut graph = engine(6, FULL, Some(harbour(13.0, Sky::Clear)));
    let playing = graph.render_seconds(4.0);
    graph.apply(Command::Scene(None));
    let leaving = graph.render_seconds(0.2);
    let joined = [&playing[..], &leaving[..]].concat();
    let (_, ratio, _) = worst_click(&joined[joined.len() - 20_000..], RATE as f32);
    assert!(ratio < 8.0, "{ratio}");
    let later = {
        graph.render_seconds(8.0);
        graph.render_seconds(1.0)
    };
    assert!(rms(&later) < 1e-3, "gone: {}", rms(&later));
}

/// Writes each place, at the app's default levels with acts over it, and
/// each act alone, to listen to:
/// `WORLD_MACHINE_SOUNDS=dir cargo test -p world-sound --release --test bars write_sounds -- --ignored`
#[test]
#[ignore]
fn write_sounds() {
    let Ok(directory) = std::env::var("WORLD_MACHINE_SOUNDS") else {
        return;
    };
    let directory = std::path::Path::new(&directory);
    // Settings' defaults: 60, 70, 70 and 80 percent of each channel's full.
    let defaults = Levels {
        music: 0.3 * 0.6,
        ambience: 0.35 * 0.7,
        voices: 0.45 * 0.7,
        interface: 0.5 * 0.8,
    };
    for (name, scene) in [
        ("harbour-morning", harbour(9.0, Sky::Clear)),
        ("harbour-rain", harbour(14.0, Sky::Rain)),
        (
            "harbour-festival-evening",
            scene(Place::Harbour, HARBOUR, 20.0, Sky::Clear, true),
        ),
        ("harbour-night", harbour(23.0, Sky::Clear)),
        (
            "mars-dust",
            scene(Place::Dome, MARS, 13.0, Sky::Dust, false),
        ),
        (
            "maple-street-evening",
            scene(Place::Street, TOWN, 19.0, Sky::Clear, false),
        ),
        (
            "icebridge-snow",
            scene(Place::Ice, ICE, 11.0, Sky::Snow, false),
        ),
    ] {
        let mut graph = engine(1, defaults, Some(scene));
        let mut out = Vec::new();
        for (second, act) in (0..40).zip(Act::ALL.iter().cycle()) {
            if second % 5 == 4 {
                graph.apply(Command::Act(*act));
            }
            out.extend(graph.render_seconds(1.0));
        }
        std::fs::write(
            directory.join(format!("{name}.wav")),
            world_sound::wav(&out, RATE),
        )
        .unwrap();
    }
    let mut graph = engine(1, defaults, None);
    let mut out = Vec::new();
    for act in Act::ALL {
        for _ in 0..3 {
            graph.apply(Command::Act(act));
            out.extend(graph.render_seconds(1.0));
        }
    }
    std::fs::write(directory.join("acts.wav"), world_sound::wav(&out, RATE)).unwrap();
}
