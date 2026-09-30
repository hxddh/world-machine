//! A World's sound, played live: its landscape, its music following the
//! hour, and a note for every act. Presentation only, like the sky: the
//! World records none of it, and replay never needs it.
//!
//! The sound itself is made by `world_sound`, a pure render graph that
//! knows nothing of devices. This module decides what the World in front
//! sounds like (which landscape, which tune, which sky), keeps the player's
//! switch and levels, and on a Mac feeds the graph to the sound device from
//! one output stream in this process. It is off unless the player turns it
//! on in Settings, and the landscape and music play only while their
//! World's window is in front.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

pub use world_sound::{Act, Levels, Palette, Place, Scene, Sky, Tune};

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Whether the player has turned sound on.
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Record the player's choice; turning it off fades whatever plays and
/// closes the output.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
    #[cfg(target_os = "macos")]
    if !on {
        player::stop();
    }
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

    /// The level a new install starts at: under the full default, so the
    /// first sound a new player hears is quiet; on the Settings mixer's
    /// own steps (each a quarter), so the mixer shows it as it is.
    pub fn gentle_level(self) -> u8 {
        match self {
            Channel::Music => 25,
            Channel::Ambience => 50,
            Channel::Voices => 50,
            Channel::Interface => 50,
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

/// Record the player's level for a channel; what plays follows at once.
pub fn set_level(channel: Channel, percent: u8) {
    LEVELS[slot(channel)].store(percent.min(100), Ordering::Relaxed);
    #[cfg(target_os = "macos")]
    player::levels_changed();
}

/// How loud to play a channel now: its full volume times the player's
/// level, or nothing with sound off.
pub fn volume(channel: Channel) -> f32 {
    if !enabled() {
        return 0.0;
    }
    channel.full() * f32::from(level(channel)) / 100.0
}

/// The mixer's gains for the player's levels. The graph's channels are
/// trimmed so a gain plays as loud as the same volume did when each sound
/// was a file played at that volume, so the levels players chose before
/// sound the same now.
pub fn levels() -> Levels {
    Levels {
        music: volume(Channel::Music),
        ambience: volume(Channel::Ambience),
        voices: volume(Channel::Voices),
        interface: volume(Channel::Interface),
    }
}

/// What kind of place a World is, for its landscape: Tiny Society's
/// harbour; each of Pocket Universe's three places by the name it gives
/// itself, or failing that by its colours; anything else an open place
/// with a drone of its own.
pub fn place(pack_id: &str, title: &str, palette: Palette) -> Place {
    if pack_id.ends_with("tiny-society") {
        return Place::Harbour;
    }
    if pack_id.ends_with("pocket-universe") {
        let title = title.to_lowercase();
        let named = |words: &[&str]| words.iter().any(|word| title.contains(word));
        if named(&["ares", "mars"]) {
            return Place::Dome;
        }
        if named(&["maple", "1987"]) {
            return Place::Street;
        }
        if named(&["icebridge", "penguin"]) {
            return Place::Ice;
        }
        // Renamed: the ground tells. Ice is pale, Mars red, the town at
        // night dark.
        let [_, _, _, near, _] = palette;
        let channel = |shift: u32| ((near >> shift) & 0xff) as f32 / 255.0;
        let (r, g, b) = (channel(16), channel(8), channel(0));
        let light = 0.299 * r + 0.587 * g + 0.114 * b;
        return if light > 0.7 {
            Place::Ice
        } else if r > g * 1.5 && r > b * 1.5 {
            Place::Dome
        } else {
            Place::Street
        };
    }
    Place::open(palette)
}

/// Everything the sound follows about a World in front: its place, its
/// tune, the hour on the player's clock, its sky, and a festival.
pub fn scene(
    pack_id: &str,
    title: &str,
    palette: Palette,
    hour: u32,
    sky: Sky,
    festival: bool,
) -> Scene {
    Scene {
        place: place(pack_id, title, palette),
        tune: Tune::of(palette),
        hour: (hour % 24) as f32,
        sky,
        festival,
    }
}

/// A World's weather as its sound hears it.
#[cfg(target_os = "macos")]
pub fn sky(weather: world_projection::Weather) -> Sky {
    match weather {
        world_projection::Weather::Clear => Sky::Clear,
        world_projection::Weather::Cloudy => Sky::Cloudy,
        world_projection::Weather::Fog => Sky::Fog,
        world_projection::Weather::Rain => Sky::Rain,
        world_projection::Weather::Snow => Sky::Snow,
        world_projection::Weather::Storm => Sky::Storm,
        world_projection::Weather::Dust => Sky::Dust,
    }
}

/// What a World window's cue plays: a card turning is paper, a turn made is
/// an answer, something built is wood, unless a keepsake just came, and a
/// line said is its speaker's babble.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Heard {
    Act(Act),
    Babble {
        voice: u32,
        syllables: u8,
        question: bool,
    },
}

pub fn heard(cue: world_gpui::Cue) -> Heard {
    match cue {
        world_gpui::Cue::Flip => Heard::Act(Act::Flip),
        world_gpui::Cue::Turn => Heard::Act(Act::Answer),
        world_gpui::Cue::Built => Heard::Act(Act::Place),
        world_gpui::Cue::Keepsake => Heard::Act(Act::Keepsake),
        world_gpui::Cue::Drawer => Heard::Act(Act::Drawer),
        world_gpui::Cue::Babble {
            voice,
            syllables,
            question,
        } => Heard::Babble {
            voice,
            syllables,
            question,
        },
    }
}

/// What a turn brought, as far as the sound cares: how many things are
/// built, keepsakes held and letters come. The desktop compares these
/// before and after a turn to hear a letter arrive or a keepsake given.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Tally {
    pub built: usize,
    pub keepsakes: usize,
    pub letters: usize,
}

/// Whether a turn from `before` to `after` brought a letter, heard just
/// after the answer.
pub fn letter_came(before: Tally, after: Tally) -> bool {
    after.letters > before.letters
}

#[cfg(target_os = "macos")]
pub mod player {
    //! One output stream for the whole app, opened the first time sound is
    //! wanted and closed when the player turns sound off. The window's
    //! thread sends it commands through a channel; its callback applies
    //! them and renders the graph straight into the device's buffer, so a
    //! click is heard in the next callback.

    use super::{enabled, levels, Act, Heard, Levels, Scene};
    use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::Mutex;
    use std::time::Duration;
    use world_sound::{Command, Engine, BUFFER_FRAMES};

    struct Output {
        commands: Sender<Command>,
        stop: Sender<()>,
        rate: f32,
    }

    struct State {
        output: Option<Output>,
        /// Opening the device failed; not tried again until sound is
        /// turned off and on.
        failed: bool,
        owner: Option<u64>,
        scene: Option<Scene>,
        levels: Levels,
        /// A line for the log, once, about the stream that opened.
        report: Option<String>,
    }

    static STATE: Mutex<State> = Mutex::new(State {
        output: None,
        failed: false,
        owner: None,
        scene: None,
        levels: Levels {
            music: 0.0,
            ambience: 0.0,
            voices: 0.0,
            interface: 0.0,
        },
        report: None,
    });

    /// The device's own delay, from a callback to its sound leaving the
    /// speaker, as the first callback measured it, in microseconds.
    static DEVICE_LATENCY_MICROS: AtomicU32 = AtomicU32::new(0);
    /// The frames the device actually asked for in its first callback.
    static CALLBACK_FRAMES: AtomicU32 = AtomicU32::new(0);
    /// The variation of the babble last played.
    static LAST_BABBLE: AtomicU8 = AtomicU8::new(0);

    /// The output, opened if it is not yet, while sound is on.
    fn output(state: &mut State) -> Option<&Output> {
        if !enabled() {
            return None;
        }
        if state.output.is_none() && !state.failed {
            match open() {
                Ok(output) => {
                    state.report = Some(format!(
                        "sound: {} Hz, asking for {BUFFER_FRAMES}-frame buffers ({:.1} ms)",
                        output.rate,
                        BUFFER_FRAMES as f32 / output.rate * 1000.0
                    ));
                    state.levels = Levels::default();
                    state.scene = None;
                    state.output = Some(output);
                }
                Err(error) => {
                    state.failed = true;
                    state.report = Some(format!("sound: could not open the output: {error}"));
                }
            }
        }
        state.output.as_ref()
    }

    fn send(state: &mut State, command: Command) {
        let sent = output(state).map(|output| output.commands.send(command).is_ok());
        if sent == Some(false) {
            // The stream's thread is gone (the device went away): open
            // afresh next time.
            state.output = None;
            state.scene = None;
        }
    }

    fn keep_levels(state: &mut State) {
        let now = levels();
        if state.output.is_some() && state.levels != now {
            state.levels = now;
            send(state, Command::Levels(now));
        }
    }

    /// The window `owner` is in front and shows `scene`: its landscape and
    /// music play, following any change within a couple of seconds.
    pub fn claim(owner: u64, scene: Scene) {
        let Ok(mut state) = STATE.lock() else {
            return;
        };
        if !enabled() {
            return;
        }
        if state.owner != Some(owner)
            || state.scene.as_ref() != Some(&scene)
            || state.output.is_none()
        {
            if output(&mut state).is_none() {
                return;
            }
            keep_levels(&mut state);
            state.owner = Some(owner);
            state.scene = Some(scene.clone());
            send(&mut state, Command::Scene(Some(scene)));
        } else {
            keep_levels(&mut state);
        }
    }

    /// The window `owner` closed or went behind: its landscape and music
    /// fade out, if they are the ones playing.
    pub fn release(owner: u64) {
        let Ok(mut state) = STATE.lock() else {
            return;
        };
        if state.owner == Some(owner) {
            state.owner = None;
            state.scene = None;
            if state.output.is_some() {
                send(&mut state, Command::Scene(None));
            }
        }
    }

    /// Something the player did: heard at once, if sound is on.
    pub fn act(act: Act) {
        if let Ok(mut state) = STATE.lock() {
            keep_levels(&mut state);
            send(&mut state, Command::Act(act));
        }
    }

    /// Something heard a moment from now, after the act that caused it.
    pub fn act_later(act: Act, seconds: f32) {
        if let Ok(mut state) = STATE.lock() {
            send(&mut state, Command::ActLater { act, seconds });
        }
    }

    /// Plays what a World window's cue asks for.
    pub fn cue(cue: world_gpui::Cue) {
        match super::heard(cue) {
            Heard::Act(act) => self::act(act),
            Heard::Babble {
                voice,
                syllables,
                question,
            } => {
                let Some((commands, rate)) = STATE.lock().ok().and_then(|mut state| {
                    keep_levels(&mut state);
                    output(&mut state).map(|output| (output.commands.clone(), output.rate))
                }) else {
                    return;
                };
                let variation = (LAST_BABBLE.load(Ordering::Relaxed) + 1) % 3;
                LAST_BABBLE.store(variation, Ordering::Relaxed);
                // A babble takes a few milliseconds to make: off the
                // window's thread, and never on the sound's.
                std::thread::spawn(move || {
                    let samples = world_sound::babble(voice, syllables, question, variation, rate);
                    let _ = commands.send(Command::Say {
                        samples,
                        place: 0.0,
                    });
                });
            }
        }
    }

    /// The player moved a level: what plays follows.
    pub fn levels_changed() {
        if let Ok(mut state) = STATE.lock() {
            keep_levels(&mut state);
        }
    }

    /// Silence: everything fades in a moment and the output closes.
    pub fn stop() {
        let Ok(mut state) = STATE.lock() else {
            return;
        };
        if let Some(output) = state.output.take() {
            let _ = output.commands.send(Command::Levels(Levels::default()));
            let _ = output.stop.send(());
        }
        state.owner = None;
        state.scene = None;
        state.failed = false;
    }

    /// A line for the log about the output, once it has something to say.
    pub fn take_report() -> Option<String> {
        let mut state = STATE.lock().ok()?;
        let latency = DEVICE_LATENCY_MICROS.load(Ordering::Relaxed);
        let frames = CALLBACK_FRAMES.load(Ordering::Relaxed);
        if let (Some(line), true) = (state.report.as_ref(), latency > 0 || state.failed) {
            let line = if state.failed {
                line.clone()
            } else {
                format!(
                    "{line}; the device calls for {frames} frames and plays them {:.1} ms later",
                    latency as f32 / 1000.0
                )
            };
            state.report = None;
            return Some(line);
        }
        None
    }

    /// Opens the default output device on a thread of its own, which keeps
    /// the stream alive until told to stop.
    fn open() -> Result<Output, String> {
        let (ready, answer) = mpsc::channel();
        std::thread::Builder::new()
            .name("world-sound".into())
            .spawn(move || run(ready))
            .map_err(|error| error.to_string())?;
        answer
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| "the sound device did not answer".to_string())?
    }

    fn run(ready: Sender<Result<Output, String>>) {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let Some(device) = host.default_output_device() else {
            let _ = ready.send(Err("there is no output device".into()));
            return;
        };
        let supported = match device.default_output_config() {
            Ok(supported) => supported,
            Err(error) => {
                let _ = ready.send(Err(error.to_string()));
                return;
            }
        };
        let rate = supported.sample_rate() as f32;
        let format = supported.sample_format();
        // Small buffers first, for a quick answer to a click; the
        // device's own if it will not have them.
        let small = match supported.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => {
                Some(cpal::BufferSize::Fixed(BUFFER_FRAMES.clamp(*min, *max)))
            }
            cpal::SupportedBufferSize::Unknown => None,
        };
        let mut last_error = String::new();
        for buffer_size in small.into_iter().chain([cpal::BufferSize::Default]) {
            let mut config = supported.config();
            config.buffer_size = buffer_size;
            let (commands, received) = mpsc::channel();
            let stream = match format {
                cpal::SampleFormat::F32 => build::<f32>(&device, config, received, rate),
                cpal::SampleFormat::I16 => build::<i16>(&device, config, received, rate),
                cpal::SampleFormat::I32 => build::<i32>(&device, config, received, rate),
                other => Err(format!("the device wants {other} samples")),
            };
            let stream = match stream.and_then(|stream| {
                stream.play().map_err(|error| error.to_string())?;
                Ok(stream)
            }) {
                Ok(stream) => stream,
                Err(error) => {
                    last_error = error;
                    continue;
                }
            };
            let (stop, stopped) = mpsc::channel::<()>();
            if ready
                .send(Ok(Output {
                    commands,
                    stop,
                    rate,
                }))
                .is_err()
            {
                return;
            }
            // Until sound is turned off or the app ends; then a moment for
            // the fade already sent to finish.
            let _ = stopped.recv();
            std::thread::sleep(Duration::from_millis(150));
            drop(stream);
            return;
        }
        let _ = ready.send(Err(last_error));
    }

    fn build<T>(
        device: &cpal::Device,
        config: cpal::StreamConfig,
        commands: Receiver<Command>,
        rate: f32,
    ) -> Result<cpal::Stream, String>
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        use cpal::traits::DeviceTrait;
        let channels = usize::from(config.channels.max(1));
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos() as u64);
        let mut engine = Engine::new(rate, seed);
        let mut scratch = vec![0.0_f32; 1024 * 2];
        let mut measured = false;
        device
            .build_output_stream(
                config,
                move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
                    if !measured {
                        measured = true;
                        let stamp = info.timestamp();
                        let delay = stamp
                            .playback
                            .checked_duration_since(stamp.callback)
                            .unwrap_or_default();
                        DEVICE_LATENCY_MICROS
                            .store(delay.as_micros().max(1) as u32, Ordering::Relaxed);
                        CALLBACK_FRAMES.store((data.len() / channels) as u32, Ordering::Relaxed);
                    }
                    while let Ok(command) = commands.try_recv() {
                        engine.apply(command);
                    }
                    let frames = data.len() / channels;
                    let mut done = 0;
                    while done < frames {
                        let count = (frames - done).min(scratch.len() / 2);
                        let rendered = &mut scratch[..count * 2];
                        engine.render(rendered);
                        let out = &mut data[done * channels..(done + count) * channels];
                        for (frame, stereo) in out.chunks_mut(channels).zip(rendered.chunks(2)) {
                            if channels == 1 {
                                frame[0] = T::from_sample((stereo[0] + stereo[1]) * 0.5);
                                continue;
                            }
                            frame[0] = T::from_sample(stereo[0]);
                            frame[1] = T::from_sample(stereo[1]);
                            for other in &mut frame[2..] {
                                *other = T::EQUILIBRIUM;
                            }
                        }
                        done += count;
                    }
                },
                |_| {},
                None,
            )
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HARBOUR: Palette = [0xa9cfe6, 0xe9f1ef, 0x7f9f8a, 0x2f6a86, 0xffe2a0];
    const MARS: Palette = [0xe7b089, 0xf5d9bd, 0xc2663f, 0x8a3a22, 0xfff3dc];
    const TOWN: Palette = [0x241d45, 0x6b4a7a, 0x3a2f55, 0x1b1630, 0xf4bf5c];
    const ICE: Palette = [0x14305a, 0x3f8f95, 0xa9cbdb, 0xe6f1f6, 0xb9f3d3];
    const POCKET: &str = "world-machine.pocket-universe";

    #[test]
    fn each_world_has_a_landscape_of_its_own() {
        assert_eq!(
            place("world-machine.tiny-society", "Harbour", HARBOUR),
            Place::Harbour
        );
        assert_eq!(place(POCKET, "Ares Pocket Colony", MARS), Place::Dome);
        assert_eq!(place(POCKET, "Maple Street · 1987", TOWN), Place::Street);
        assert_eq!(place(POCKET, "Icebridge Colony", ICE), Place::Ice);
        // Renamed, each is still known by its ground.
        assert_eq!(place(POCKET, "Home", MARS), Place::Dome);
        assert_eq!(place(POCKET, "Home", TOWN), Place::Street);
        assert_eq!(place(POCKET, "Home", ICE), Place::Ice);
        // Another Pack's World has wind and a drone of its own.
        assert!(matches!(
            place("someone.else", "Anywhere", MARS),
            Place::Open { .. }
        ));
        let noon = scene(POCKET, "Ares", MARS, 13, Sky::Dust, false);
        assert_eq!(noon.tune, Tune::of(MARS), "the World's own tune");
        assert_eq!(noon.hour, 13.0);
    }

    #[test]
    fn the_old_levels_map_onto_the_mixer() {
        set_enabled(true);
        for channel in Channel::ALL {
            set_level(channel, channel.default_level());
        }
        let mixed = levels();
        assert_eq!(mixed.music, volume(Channel::Music));
        assert_eq!(mixed.ambience, volume(Channel::Ambience));
        assert_eq!(mixed.voices, volume(Channel::Voices));
        assert_eq!(mixed.interface, volume(Channel::Interface));
        assert!((mixed.music - 0.18).abs() < 1e-6, "0.3 at 60%");
        set_level(Channel::Voices, 100);
        let full = volume(Channel::Voices);
        set_level(Channel::Voices, 50);
        assert!((volume(Channel::Voices) - full / 2.0).abs() < 1e-6);
        set_level(Channel::Voices, 0);
        assert_eq!(levels().voices, 0.0);
        set_level(Channel::Voices, Channel::Voices.default_level());

        // The same volume on the old files and the new mixer: an act at
        // the interface's default peaks where the old bell did (0.32 of
        // full at the player's volume), within a few decibels.
        let volume = volume(Channel::Interface);
        let mut graph = world_sound::Engine::new(48_000.0, 1);
        graph.apply(world_sound::Command::Levels(Levels {
            interface: volume,
            ..Levels::default()
        }));
        graph.apply(world_sound::Command::Act(Act::Answer));
        let peak = graph
            .render_seconds(1.0)
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        let old = 0.32 * volume;
        assert!(peak > old * 0.5 && peak < old * 2.0, "{peak} vs {old}");

        set_enabled(false);
        assert_eq!(levels(), Levels::default(), "sound off silences all");
    }

    #[test]
    fn every_cue_is_heard_as_its_act() {
        use world_gpui::Cue;
        assert_eq!(heard(Cue::Flip), Heard::Act(Act::Flip));
        assert_eq!(heard(Cue::Turn), Heard::Act(Act::Answer));
        assert_eq!(heard(Cue::Built), Heard::Act(Act::Place));
        assert_eq!(heard(Cue::Keepsake), Heard::Act(Act::Keepsake));
        assert_eq!(heard(Cue::Drawer), Heard::Act(Act::Drawer));
        let line = Cue::Babble {
            voice: 7,
            syllables: 5,
            question: true,
        };
        assert!(matches!(heard(line), Heard::Babble { voice: 7, .. }));
    }

    #[test]
    fn a_turn_hears_a_letter_only_when_one_came() {
        let before = Tally {
            built: 3,
            keepsakes: 1,
            letters: 0,
        };
        let after = Tally {
            built: 4,
            keepsakes: 2,
            letters: 1,
        };
        assert!(letter_came(before, after));
        assert!(!letter_came(before, before));
        assert!(!letter_came(after, before));
    }
}
