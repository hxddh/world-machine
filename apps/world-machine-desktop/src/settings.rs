//! The Settings window.
//!
//! One place to say how this app's Worlds sound, how the app is shown and
//! paced, how it plays, what happens after a crash, and whether an AI
//! writes residents' answers (experimental, and off unless turned on).
//! Its sections and controls are planned in `settings_plan.rs`; the
//! keyboard walks them (↑ ↓ and Tab), changes the one focused (← → and
//! Enter), and Esc closes the window.
//!
//! The window is deliberately plain and says what is true rather than what is
//! encouraging: which way the voice reaches a model, that a key is the only way
//! a World's contents leave this computer, and what happens when it is
//! switched off.

use gpui::{
    div, prelude::*, px, size, App, AppContext, Bounds, Context, Entity, IntoElement,
    PathPromptOptions, Render, SharedString, Styled, Window, WindowBounds, WindowOptions,
};
use std::path::Path;
use world_gpui::ui;
use world_machine_desktop::ambience;
use world_machine_desktop::app_settings::{
    self, AppSettingsError, CrashReports, FrameRate, JudgeChoice, VoiceSource,
};
use world_machine_desktop::key_store;
use world_machine_desktop::settings_plan::{self, Control, KeyDoes, Kind, Level, Section};
use world_machine_desktop::shell;
use world_theme::tokens;

use crate::diagnostics;
use world_gpui::text_input::{self as text_field, TextInput};

pub fn open(cx: &mut App) {
    text_field::bind_keys(cx);
    let bounds = Bounds::centered(None, size(px(620.0), px(680.0)), cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |_, cx| cx.new(SettingsView::new),
    );
    if let Err(error) = opened {
        diagnostics::error(format!("could not open the Settings window: {error}"));
    }
}

/// The languages Settings offers, `None` following the system.
const LANGUAGES: [Option<&str>; 4] = [None, Some("en"), Some("zh-Hans"), Some("ja")];
/// The text sizes Settings offers, in percent.
const TEXT_SIZES: [u32; 5] = [100, 125, 150, 175, 200];
/// The crash report choices, as Settings offers them.
const CRASH_CHOICES: [CrashReports; 2] = [CrashReports::Ask, CrashReports::Never];

struct SettingsView {
    key_input: Entity<TextInput>,
    /// Whether a key is stored, read once when the window opens and after every
    /// change. The key itself is never held here.
    key_stored: bool,
    voice_on: bool,
    sound_on: bool,
    /// The master level every sound plays under.
    master: u8,
    /// The level of each sound, in `ambience::Channel::ALL` order.
    levels: [u8; 4],
    /// The language chosen ("en", "zh-Hans", "ja"), or `None` to follow the Mac.
    language: Option<String>,
    /// How large text is drawn, in percent.
    text_scale: u32,
    /// More contrast, fewer or as the Mac says.
    contrast: Option<bool>,
    /// Less motion, more or as the system says.
    reduce_motion: Option<bool>,
    /// How often a World's window draws while things move.
    frame_rate: FrameRate,
    battery_saver: bool,
    /// Whether a crash report is offered on the next launch.
    crash_reports: CrashReports,
    /// Whether the drawer shows the World's inner workings.
    developer: bool,
    /// Whether the player plays freely: Goodnight offers "Sleep now".
    play_freely: bool,
    source: VoiceSource,
    program: Option<String>,
    /// The Claude model chosen for an API key, if not the default.
    voice_model: Option<String>,
    model_input: Entity<TextInput>,
    /// Whether this Mac's own model (`/usr/bin/fm`) answers; `None` while
    /// that is still being found out, off this window's thread.
    on_device: Option<::world_voice::FmStatus>,
    /// Who gives each voice answer a second opinion.
    judge: JudgeChoice,
    status: Option<SharedString>,
    /// Holds the keyboard, so the arrows walk the settings and Esc closes.
    focus: gpui::FocusHandle,
    /// Which control the keyboard is on, in `settings_plan::controls` order.
    focused: usize,
}

impl SettingsView {
    fn new(cx: &mut Context<Self>) -> Self {
        let key_input = cx.new(|cx| TextInput::new("Paste an API key…", cx));
        cx.observe(&key_input, |_, _, cx| cx.notify()).detach();
        let model_input = cx.new(|cx| TextInput::new(::world_voice::DEFAULT_MODEL, cx));
        cx.observe(&model_input, |_, _, cx| cx.notify()).detach();
        // Asking `fm` once can take as long as loading its model, so it is
        // done in the background and remembered for the rest of the run.
        cx.spawn(async move |this, cx| {
            let status = cx
                .background_executor()
                .spawn(async { ::world_voice::fm::status().clone() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.on_device = Some(status);
                cx.notify();
            });
        })
        .detach();
        let mut view = Self {
            key_input,
            key_stored: false,
            voice_on: false,
            sound_on: false,
            master: 100,
            levels: ambience::Channel::ALL.map(ambience::Channel::default_level),
            language: None,
            text_scale: 100,
            contrast: None,
            reduce_motion: None,
            frame_rate: FrameRate::default(),
            battery_saver: false,
            crash_reports: CrashReports::Ask,
            developer: false,
            play_freely: false,
            source: VoiceSource::Program,
            program: None,
            voice_model: None,
            model_input,
            on_device: ::world_voice::fm::status_if_known().cloned(),
            judge: JudgeChoice::Off,
            status: None,
            focus: cx.focus_handle(),
            focused: 0,
        };
        view.reload();
        view
    }

    /// Read what is actually configured. Anything unreadable reads as nothing
    /// configured, which is the same as never having set it up.
    fn reload(&mut self) {
        self.key_stored = key_store::is_configured();
        let settings = app_settings::application_support_root()
            .ok()
            .and_then(|root| app_settings::load(&root).ok())
            .unwrap_or_else(app_settings::AppSettings::empty);
        self.voice_on = settings.world_voice;
        self.sound_on = settings.ambient_sound;
        self.master = settings.master_level();
        self.levels = ambience::Channel::ALL.map(|channel| settings.sound_level(channel));
        self.language = settings.language.clone();
        self.text_scale = settings.text_scale.unwrap_or(100);
        self.contrast = settings.increase_contrast;
        self.reduce_motion = settings.reduce_motion;
        self.frame_rate = settings.frame_rate();
        self.battery_saver = settings.battery_saver;
        self.crash_reports = settings.crash_reports;
        self.developer = settings.developer;
        self.play_freely = settings.play_freely;
        self.source = settings.world_voice_source.unwrap_or_default();
        self.program = settings
            .pi_program
            .as_ref()
            .map(|path| path.display().to_string());
        self.judge = settings.judge_choice(self.key_stored);
        self.voice_model = settings.voice_model;
    }

    fn apply<F>(&mut self, change: F, cx: &mut Context<Self>)
    where
        F: FnOnce() -> Result<(), String>,
    {
        match change() {
            Ok(()) => self.status = None,
            Err(error) => self.status = Some(error.into()),
        }
        self.reload();
        cx.notify();
    }

    /// Writes one change to the settings file, through `save`.
    fn save(
        &mut self,
        save: impl FnOnce(&Path) -> Result<(), AppSettingsError>,
        cx: &mut Context<Self>,
    ) {
        self.apply(
            move || {
                let root =
                    app_settings::application_support_root().map_err(|error| error.to_string())?;
                save(&root).map_err(|error| error.to_string())
            },
            cx,
        );
    }

    fn set_voice(&mut self, on: bool, cx: &mut Context<Self>) {
        self.save(|root| app_settings::save_world_voice(root, on), cx);
    }

    fn set_sound(&mut self, on: bool, cx: &mut Context<Self>) {
        self.save(|root| app_settings::save_ambient_sound(root, on), cx);
        ambience::set_enabled(self.sound_on);
    }

    /// Shows every window the way the player now asks.
    fn show_as_chosen(&mut self, cx: &mut Context<Self>) {
        let settings = app_settings::application_support_root()
            .ok()
            .and_then(|root| app_settings::load(&root).ok());
        world_machine_desktop::display::apply(settings.as_ref());
        cx.set_reduce_motion(world_machine_desktop::display::reduce_motion(
            settings.as_ref(),
        ));
        cx.refresh_windows();
    }

    fn set_language(&mut self, language: Option<String>, cx: &mut Context<Self>) {
        self.save(|root| app_settings::save_language(root, language), cx);
        self.show_as_chosen(cx);
    }

    fn set_text_scale(&mut self, percent: u32, cx: &mut Context<Self>) {
        self.save(|root| app_settings::save_text_scale(root, percent), cx);
        self.show_as_chosen(cx);
    }

    fn set_contrast(&mut self, on: Option<bool>, cx: &mut Context<Self>) {
        self.save(|root| app_settings::save_increase_contrast(root, on), cx);
        self.show_as_chosen(cx);
    }

    fn set_reduce_motion(&mut self, on: Option<bool>, cx: &mut Context<Self>) {
        self.save(|root| app_settings::save_reduce_motion(root, on), cx);
        self.show_as_chosen(cx);
    }

    fn set_frame_rate(&mut self, rate: FrameRate, cx: &mut Context<Self>) {
        self.save(|root| app_settings::save_frame_rate(root, rate), cx);
        self.show_as_chosen(cx);
    }

    fn set_battery_saver(&mut self, on: bool, cx: &mut Context<Self>) {
        self.save(|root| app_settings::save_battery_saver(root, on), cx);
        self.show_as_chosen(cx);
    }

    fn set_crash_reports(&mut self, choice: CrashReports, cx: &mut Context<Self>) {
        self.save(|root| app_settings::save_crash_reports(root, choice), cx);
    }

    fn set_developer(&mut self, on: bool, cx: &mut Context<Self>) {
        self.save(|root| app_settings::save_developer(root, on), cx);
        self.show_as_chosen(cx);
    }

    /// Play freely, or a new day with each day the player comes back. Open
    /// Worlds follow at once: their title bars and Goodnight cards redraw.
    fn set_play_freely(&mut self, on: bool, cx: &mut Context<Self>) {
        self.save(|root| app_settings::save_play_freely(root, on), cx);
        self.show_as_chosen(cx);
    }

    fn set_level(&mut self, level: Level, percent: u8, cx: &mut Context<Self>) {
        match level {
            Level::Master => {
                self.save(|root| app_settings::save_master_volume(root, percent), cx);
                ambience::set_master(self.master);
            }
            Level::Channel(channel) => {
                self.save(
                    |root| app_settings::save_sound_level(root, channel, percent),
                    cx,
                );
                for (channel, level) in ambience::Channel::ALL.into_iter().zip(self.levels) {
                    ambience::set_level(channel, level);
                }
            }
        }
    }

    fn level(&self, level: Level) -> u8 {
        match level {
            Level::Master => self.master,
            Level::Channel(channel) => ambience::Channel::ALL
                .iter()
                .position(|each| *each == channel)
                .map_or(0, |at| self.levels[at]),
        }
    }

    fn set_source(&mut self, source: VoiceSource, cx: &mut Context<Self>) {
        self.save(
            |root| app_settings::save_world_voice_source(root, source),
            cx,
        );
    }

    fn set_judge(&mut self, judge: JudgeChoice, cx: &mut Context<Self>) {
        self.save(|root| app_settings::save_voice_judge(root, judge), cx);
    }

    fn save_key(&mut self, cx: &mut Context<Self>) {
        let typed = self.key_input.read(cx).text().to_owned();
        self.apply(move || key_store::save(&typed), cx);
        // Never leave a key sitting in a field once it is stored.
        self.key_input.update(cx, |input, cx| input.clear(cx));
    }

    /// Asks which program on this Mac the voice writes with.
    fn choose_program(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose the voice program".into()),
        });
        cx.spawn(async move |this, cx| {
            let chosen = match picker.await {
                Ok(Ok(Some(mut paths))) => paths.pop(),
                Ok(Ok(None)) => None,
                Ok(Err(error)) => {
                    let message = format!("Could not choose a program: {error}");
                    let _ = this.update(cx, |this, cx| this.apply(|| Err(message), cx));
                    return;
                }
                Err(_) => return,
            };
            let Some(path) = chosen else {
                return;
            };
            let _ = this.update(cx, |this, cx| {
                this.save(|root| app_settings::save_pi_program(root, path), cx)
            });
        })
        .detach();
    }

    /// Keeps the Claude model typed in; an empty field goes back to the
    /// default.
    fn save_model(&mut self, cx: &mut Context<Self>) {
        let typed = self.model_input.read(cx).text().to_owned();
        self.save(|root| app_settings::save_voice_model(root, Some(typed)), cx);
        self.model_input.update(cx, |input, cx| input.clear(cx));
    }

    /// Whether this Mac's own model is offered: only where it answers, or
    /// where it was chosen before, so it can be chosen away from.
    fn offers_on_device(&self) -> bool {
        self.on_device
            .as_ref()
            .is_some_and(::world_voice::FmStatus::is_ready)
            || self.source == VoiceSource::OnDevice
    }

    /// The ways the voice can reach a model, as the window offers them.
    fn sources(&self) -> Vec<VoiceSource> {
        let mut sources = vec![VoiceSource::Program, VoiceSource::Key];
        if self.offers_on_device() {
            sources.push(VoiceSource::OnDevice);
        }
        sources
    }

    /// The judges, as the window offers them.
    fn judges(&self) -> Vec<JudgeChoice> {
        let mut judges = vec![JudgeChoice::Off, JudgeChoice::Cloud];
        if self.offers_on_device() {
            judges.push(JudgeChoice::OnDevice);
        }
        judges
    }

    fn forget_key(&mut self, cx: &mut Context<Self>) {
        self.apply(key_store::clear, cx);
    }

    fn contrast_on(&self) -> bool {
        self.contrast
            .unwrap_or_else(world_machine_desktop::display::system_increase_contrast)
    }

    fn reduce_motion_on(&self) -> bool {
        self.reduce_motion
            .unwrap_or_else(world_machine_desktop::display::system_reduce_motion)
    }

    /// The controls the keyboard walks now.
    fn controls(&self) -> Vec<Control> {
        settings_plan::controls(self.sound_on, self.voice_on)
    }

    /// Whether the keyboard is on `control`.
    fn is_focused(&self, control: Control) -> bool {
        self.controls().get(self.focused) == Some(&control)
    }

    /// Changes `control` a step (`by` -1 or 1), or flips it (`by` 0).
    fn change(&mut self, control: Control, by: i32, cx: &mut Context<Self>) {
        match control {
            Control::Sound => self.set_sound(!self.sound_on, cx),
            Control::Level(level) => {
                if by != 0 {
                    let now = settings_plan::step_level(self.level(level), by > 0);
                    self.set_level(level, now, cx);
                }
            }
            Control::Language => {
                let current = LANGUAGES
                    .iter()
                    .copied()
                    .find(|id| self.language.as_deref() == *id)
                    .unwrap_or(None);
                let chosen = settings_plan::choose(&LANGUAGES, current, by);
                self.set_language(chosen.map(str::to_string), cx);
            }
            Control::TextSize => {
                let chosen = settings_plan::choose(&TEXT_SIZES, self.text_scale, by);
                self.set_text_scale(chosen, cx);
            }
            Control::Contrast => self.set_contrast(Some(!self.contrast_on()), cx),
            Control::ReduceMotion => self.set_reduce_motion(Some(!self.reduce_motion_on()), cx),
            Control::FrameRate => {
                let chosen = settings_plan::choose(&FrameRate::ALL, self.frame_rate, by);
                self.set_frame_rate(chosen, cx);
            }
            Control::BatterySaver => self.set_battery_saver(!self.battery_saver, cx),
            Control::PlayFreely => self.set_play_freely(!self.play_freely, cx),
            Control::CrashReports => {
                let chosen = settings_plan::choose(&CRASH_CHOICES, self.crash_reports, by);
                self.set_crash_reports(chosen, cx);
            }
            Control::Voice => self.set_voice(!self.voice_on, cx),
            Control::VoiceSource => {
                let chosen = settings_plan::choose(&self.sources(), self.source, by);
                self.set_source(chosen, cx);
            }
            Control::VoiceJudge => {
                let chosen = settings_plan::choose(&self.judges(), self.judge, by);
                self.set_judge(chosen, cx);
            }
            Control::Developer => self.set_developer(!self.developer, cx),
        }
    }

    /// The keyboard: ↑ ↓ and Tab walk the settings, ← → change the one
    /// focused, Enter and Space flip a switch, Esc closes the window (or,
    /// from a text field, gives the keyboard back to the settings first).
    fn on_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let modifiers = keystroke.modifiers;
        if modifiers.control || modifiers.platform || modifiers.alt {
            return;
        }
        let key = keystroke.key.as_str();
        if !self.focus.is_focused(window) {
            // A text field has the keyboard: its keys are words.
            if key == "escape" {
                window.focus(&self.focus, cx);
                cx.stop_propagation();
                cx.notify();
            }
            return;
        }
        let Some(does) = settings_plan::key_does(key, modifiers.shift) else {
            return;
        };
        let controls = self.controls();
        let focused = controls.get(self.focused.min(controls.len().saturating_sub(1)));
        match (does, focused.copied()) {
            (KeyDoes::Close, _) => {
                window.remove_window();
                return;
            }
            (KeyDoes::Move(step), _) => {
                self.focused = shell::step(controls.len(), self.focused, step);
            }
            // Enter flips a switch, and moves a choice on to the next.
            (KeyDoes::Toggle, Some(control)) => {
                if control.kind() != Kind::Level {
                    self.change(control, 0, cx);
                }
            }
            (KeyDoes::Adjust(by), Some(control)) => {
                let by = if control.kind() == Kind::Switch {
                    0
                } else {
                    by
                };
                self.change(control, by, cx);
            }
            (_, None) => {}
        }
        // The controls can change under the focus (sound or the voice
        // turned off): a change stays on its control; a move goes on.
        self.focused = settings_plan::refocus(does, focused, self.focused, &self.controls());
        cx.stop_propagation();
        cx.notify();
    }

    /// What is true right now, as a few words and how settled it is.
    fn state(&self) -> (&'static str, String) {
        if !self.voice_on {
            return (
                "off",
                "Off. Worlds read from the app's own copy, and nothing leaves this computer."
                    .into(),
            );
        }
        match self.source {
            VoiceSource::Program => match self.program.as_deref() {
                Some(program) => ("ready", format!("On, through {}.", program_name(program))),
                None => (
                    "incomplete",
                    "On, but no program is chosen, so Worlds still read from the app's copy."
                        .into(),
                ),
            },
            VoiceSource::Key => {
                if self.key_stored {
                    ("ready", "On, through your API key.".into())
                } else {
                    (
                        "incomplete",
                        "On, but no key is stored, so Worlds still read from the app's copy."
                            .into(),
                    )
                }
            }
            VoiceSource::OnDevice => match &self.on_device {
                Some(::world_voice::FmStatus::Ready) => {
                    ("ready", "On, through this Mac's own model.".into())
                }
                None => ("incomplete", "On, checking this Mac's own model…".into()),
                Some(_) => (
                    "incomplete",
                    "On, but this Mac's own model is not available, so Worlds still read from the app's copy."
                        .into(),
                ),
            },
        }
    }
}

/// The file name of a program, which is what anyone recognises it by.
fn program_name(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
}

/// A switch drawn the way the system draws one: a track and a knob.
fn switch(id: &'static str, on: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .role(gpui::Role::Switch)
        .aria_toggled(if on {
            gpui::Toggled::True
        } else {
            gpui::Toggled::False
        })
        .flex_shrink_0()
        .w(px(38.0))
        .h(px(22.0))
        .p(px(2.0))
        .rounded_full()
        .cursor_pointer()
        .bg(ui::color(if on {
            tokens::ACCENT
        } else {
            tokens::BORDER_STRONG
        }))
        .flex()
        .when(on, |track| track.justify_end())
        .child(
            div()
                .size(px(18.0))
                .rounded_full()
                .bg(ui::color(tokens::SURFACE)),
        )
}

/// One of a few ways the voice can reach a model: a tile that says what it
/// is and the one fact that matters about it, selected like a radio button.
fn source_tile(
    id: &'static str,
    glyph: &'static str,
    title: &'static str,
    fact: String,
    selected: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_1()
        .min_w(px(0.0))
        .p_3()
        .rounded_lg()
        .cursor_pointer()
        .border_1()
        .border_color(ui::color(if selected {
            tokens::ACCENT
        } else {
            tokens::BORDER
        }))
        .bg(ui::color(if selected {
            tokens::ACCENT_SOFT
        } else {
            tokens::SURFACE
        }))
        .hover(|tile| tile.border_color(ui::color(tokens::ACCENT)))
        .role(gpui::Role::RadioButton)
        .aria_selected(selected)
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .size(px(28.0))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(ui::color(if selected {
                    tokens::ACCENT
                } else {
                    tokens::ROW_HOVER
                }))
                .text_color(ui::color(if selected {
                    tokens::ON_ACCENT
                } else {
                    tokens::TEXT_SECONDARY
                }))
                .text_sm()
                .child(glyph),
        )
        .child(ui::row_title(title))
        .child(ui::caption(fact))
}

/// A grouped block of settings, the way the system groups them.
fn group() -> gpui::Div {
    div()
        .w_full()
        .p_4()
        .rounded_lg()
        .border_1()
        .border_color(ui::color(tokens::BORDER))
        .bg(ui::color(tokens::SURFACE))
        .flex()
        .flex_col()
        .gap_3()
}

/// A section's heading over its group.
fn section_title(section: Section) -> gpui::Div {
    div().pt_2().child(ui::heading(section.title()))
}

/// A control's place, ringed in the accent while the keyboard is on it.
fn focus_ring(focused: bool) -> gpui::Div {
    div()
        .w_full()
        .p_1()
        .rounded_md()
        .border_2()
        .border_color(if focused {
            gpui::Hsla::from(ui::color(tokens::ACCENT))
        } else {
            gpui::transparent_black()
        })
}

/// A control's name and what it does.
fn row(title: &'static str, what: &'static str) -> gpui::Div {
    div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .gap_1()
        .child(ui::row_title(title))
        .child(ui::caption(what))
}

/// A choice among a few, drawn as a chip.
fn chip(id: SharedString, label: String, selected: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px_3()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .border_1()
        .border_color(ui::color(if selected {
            tokens::ACCENT
        } else {
            tokens::BORDER
        }))
        .bg(ui::color(if selected {
            tokens::ACCENT_SOFT
        } else {
            tokens::SURFACE
        }))
        .text_sm()
        .role(gpui::Role::RadioButton)
        .aria_selected(selected)
        .aria_label(label.clone())
        .child(label)
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        window.set_rem_size(gpui::px(world_gpui::rem_size()));
        window.set_window_title(&ui::t("World Machine Settings"));
        if window.focused(cx).is_none() {
            window.focus(&self.focus, cx);
        }

        let mut page = div()
            .id("settings-page")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .overflow_y_scroll()
            .size_full()
            .p_6()
            .flex()
            .flex_col()
            .gap_3()
            .bg(ui::color(tokens::WINDOW))
            .text_color(ui::color(tokens::TEXT))
            .child(ui::page_title("Settings"))
            .child(ui::caption(
                "↑ ↓ choose a setting, ← → change it, Enter turns it on or off, and Esc closes Settings.",
            ));
        if let Some(status) = self.status.clone() {
            page = page.child(
                div()
                    .text_sm()
                    .text_color(ui::color(tokens::DANGER))
                    .child(status),
            );
        }
        page.child(section_title(Section::Sound))
            .child(self.render_sound(cx))
            .child(section_title(Section::Display))
            .child(self.render_display(cx))
            .child(section_title(Section::Performance))
            .child(self.render_performance(cx))
            .child(section_title(Section::Playing))
            .child(self.render_playing(cx))
            .child(section_title(Section::Controls))
            .child(render_controls())
            .child(section_title(Section::Reports))
            .child(self.render_reports(cx))
            .child(section_title(Section::Voice))
            .child(self.render_voice(cx))
            .child(section_title(Section::Makers))
            .child(self.render_makers(cx))
    }
}

impl SettingsView {
    /// The sound switch, and while it is on the master level and each
    /// sound's own.
    fn render_sound(&self, cx: &mut Context<Self>) -> gpui::Div {
        let sound_on = self.sound_on;
        let mut sound = group().child(
            focus_ring(self.is_focused(Control::Sound)).child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(row(
                        "Sound",
                        "The World in front plays its music and its landscape's sound, everyone speaks in a voice of their own, and cards, turns and building each have a small sound.",
                    ))
                    .child(
                        switch("ambient-sound-switch", sound_on)
                            .aria_label(ui::t("Sound"))
                            .on_click(cx.listener(move |this, _, _, cx| this.set_sound(!sound_on, cx))),
                    ),
            ),
        );
        if !sound_on {
            return sound;
        }
        for level in Level::ALL {
            sound = sound.child(self.render_level(level, cx));
        }
        sound
    }

    /// One level: its name, − and +, ten steps to click, and the
    /// percentage.
    fn render_level(&self, level: Level, cx: &mut Context<Self>) -> gpui::Div {
        let (name, what) = level.words();
        let percent = self.level(level);
        let mut steps = div().flex_shrink_0().flex().items_end().gap(px(3.0));
        for step in 1..=10_u8 {
            let at = step * settings_plan::LEVEL_STEP;
            let lit = percent + settings_plan::LEVEL_STEP / 2 >= at && percent > 0;
            steps = steps.child(
                div()
                    .id(SharedString::from(format!("{}-{step}", level.id())))
                    .w(px(9.0))
                    .h(px(6.0 + f32::from(step) * 1.4))
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(ui::color(if lit {
                        tokens::ACCENT
                    } else {
                        tokens::BORDER_STRONG
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| this.set_level(level, at, cx))),
            );
        }
        let nudge = |id: &'static str, glyph: &'static str, up: bool| {
            div()
                .id(SharedString::from(format!("{}-{id}", level.id())))
                .flex_shrink_0()
                .size(px(22.0))
                .rounded_md()
                .border_1()
                .border_color(ui::color(tokens::BORDER_STRONG))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_sm()
                .role(gpui::Role::Button)
                .child(glyph)
                .on_click(cx.listener(move |this, _, _, cx| {
                    let now = settings_plan::step_level(this.level(level), up);
                    this.set_level(level, now, cx)
                }))
        };
        focus_ring(self.is_focused(Control::Level(level))).child(
            div()
                .id(level.id())
                .role(gpui::Role::Slider)
                .aria_label(ui::t(name))
                .flex()
                .items_center()
                .gap_3()
                .child(row(name, what))
                .child(nudge("down", "−", false))
                .child(steps)
                .child(nudge("up", "+", true))
                .child(
                    div()
                        .flex_shrink_0()
                        .w(px(40.0))
                        .text_sm()
                        .text_color(ui::color(tokens::TEXT_SECONDARY))
                        .child(format!("{percent}%")),
                ),
        )
    }

    /// Language, text size, contrast and motion.
    fn render_display(&self, cx: &mut Context<Self>) -> gpui::Div {
        let mut languages = div().flex().flex_wrap().gap_2();
        for (index, id) in LANGUAGES.into_iter().enumerate() {
            let label = match id {
                None => ui::t("Follow the system").to_string(),
                Some("en") => "English".to_string(),
                Some("zh-Hans") => "简体中文".to_string(),
                Some(_) => "日本語".to_string(),
            };
            let selected = self.language.as_deref() == id;
            let chosen = id.map(str::to_string);
            languages = languages.child(
                chip(
                    SharedString::from(format!("language-{index}")),
                    label,
                    selected,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.set_language(chosen.clone(), cx))),
            );
        }
        let mut sizes = div().flex().flex_wrap().gap_2();
        for percent in TEXT_SIZES {
            sizes = sizes.child(
                chip(
                    SharedString::from(format!("text-size-{percent}")),
                    format!("{percent}%"),
                    self.text_scale == percent,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.set_text_scale(percent, cx))),
            );
        }
        let contrast_on = self.contrast_on();
        let still = self.reduce_motion_on();
        group()
            .child(
                focus_ring(self.is_focused(Control::Language)).child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(row("Language", "The app and the Worlds that come with it."))
                        .child(languages),
                ),
            )
            .child(
                focus_ring(self.is_focused(Control::TextSize)).child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(row(
                            "Text size",
                            "Everything written, from cards to speech.",
                        ))
                        .child(sizes),
                ),
            )
            .child(
                focus_ring(self.is_focused(Control::Contrast)).child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(row(
                            "Increase contrast",
                            "Quiet text and outlines drawn stronger.",
                        ))
                        .child(
                            switch("contrast-switch", contrast_on)
                                .aria_label(ui::t("Increase contrast"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.set_contrast(Some(!contrast_on), cx)
                                })),
                        ),
                ),
            )
            .child(
                focus_ring(self.is_focused(Control::ReduceMotion)).child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(row(
                            "Reduce motion",
                            "The scene holds still: nobody strolls, the sea and the clouds stay put, and nothing glides.",
                        ))
                        .child(
                            switch("reduce-motion-switch", still)
                                .aria_label(ui::t("Reduce motion"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.set_reduce_motion(Some(!still), cx)
                                })),
                        ),
                ),
            )
    }

    /// The frame rate and the battery saver.
    fn render_performance(&self, cx: &mut Context<Self>) -> gpui::Div {
        let mut rates = div().flex().flex_wrap().gap_2();
        for rate in FrameRate::ALL {
            rates = rates.child(
                chip(
                    SharedString::from(format!("frame-rate-{}", rate.cap().unwrap_or(0))),
                    ui::t(rate.label()).to_string(),
                    self.frame_rate == rate,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.set_frame_rate(rate, cx))),
            );
        }
        let battery = self.battery_saver;
        group()
            .child(
                focus_ring(self.is_focused(Control::FrameRate)).child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(row(
                            "Frame rate",
                            "How often a World's window draws while things move. Fewer frames use less power.",
                        ))
                        .child(rates),
                ),
            )
            .child(
                focus_ring(self.is_focused(Control::BatterySaver)).child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(row(
                            "Battery saver",
                            "At most 30 frames a second, whatever the frame rate says, so a battery lasts longer.",
                        ))
                        .child(
                            switch("battery-saver-switch", battery)
                                .aria_label(ui::t("Battery saver"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.set_battery_saver(!battery, cx)
                                })),
                        ),
                ),
            )
    }

    /// Play freely.
    fn render_playing(&self, cx: &mut Context<Self>) -> gpui::Div {
        let freely_on = self.play_freely;
        group().child(
            focus_ring(self.is_focused(Control::PlayFreely)).child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(row(
                        "Play freely",
                        "Goodnight offers Sleep now, and the next day comes at once. Off, each day you come back brings one new day.",
                    ))
                    .child(
                        switch("play-freely-switch", freely_on)
                            .aria_label(ui::t("Play freely"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_play_freely(!freely_on, cx)
                            })),
                    ),
            ),
        )
    }

    /// Whether a crash report is offered, and where to write.
    fn render_reports(&self, cx: &mut Context<Self>) -> gpui::Div {
        let mut choices = div().flex().flex_wrap().gap_2();
        for choice in CRASH_CHOICES {
            let label = match choice {
                CrashReports::Ask => "Offer it to me",
                CrashReports::Never => "Don't ask",
            };
            choices = choices.child(
                chip(
                    SharedString::from(format!("crash-reports-{choice:?}")),
                    ui::t(label).to_string(),
                    self.crash_reports == choice,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.set_crash_reports(choice, cx))),
            );
        }
        group()
            .child(
                focus_ring(self.is_focused(Control::CrashReports)).child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(row(
                            "After a crash",
                            "A crash report is kept on this computer. Offered, it is shown to you once on the next launch; nothing is ever sent unless you send it.",
                        ))
                        .child(choices),
                ),
            )
            .child(crate::about::support_line())
    }

    /// The developer switch.
    fn render_makers(&self, cx: &mut Context<Self>) -> gpui::Div {
        let developer_on = self.developer;
        group().child(
            focus_ring(self.is_focused(Control::Developer)).child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(row(
                        "For World makers",
                        "The drawer shows each World's inner workings: its records and counters.",
                    ))
                    .child(
                        switch("developer-switch", developer_on)
                            .aria_label(ui::t("For World makers"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_developer(!developer_on, cx)
                            })),
                    ),
            ),
        )
    }

    /// The AI voice: experimental, off unless turned on, and while on where
    /// it comes from and who checks it.
    fn render_voice(&self, cx: &mut Context<Self>) -> gpui::Div {
        let on = self.voice_on;
        let source = self.source;
        let mut voice = group().child(
            focus_ring(self.is_focused(Control::Voice)).child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(row(
                        "Let an AI write residents' answers",
                        "Experimental, and off unless you turn it on. When it is on, an AI model writes what residents answer to what you say; the World still decides what happens.",
                    ))
                    .child(
                        switch("world-voice-switch", on)
                            .aria_label(ui::t("Let an AI write residents' answers"))
                            .on_click(cx.listener(move |this, _, _, cx| this.set_voice(!on, cx))),
                    ),
            ),
        );

        if on {
            let program_fact = match self.program.as_deref() {
                Some(program) => format!("Stays on this computer · {}", program_name(program)),
                None => "Stays on this computer · choose the program below".into(),
            };
            let key_fact = if self.key_stored {
                "What you say to someone is sent to your provider · key stored".to_string()
            } else {
                "What you say to someone is sent to your provider".to_string()
            };
            let mut sources = div()
                .flex()
                .flex_col()
                .gap_3()
                .child(ui::section_label("Where the voice comes from"))
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .child(
                            source_tile(
                                "world-voice-program",
                                "⌂",
                                "A program on this computer",
                                program_fact,
                                matches!(source, VoiceSource::Program),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_source(VoiceSource::Program, cx)
                            })),
                        )
                        .child(
                            source_tile(
                                "world-voice-key",
                                "↗",
                                "An API key",
                                key_fact,
                                matches!(source, VoiceSource::Key),
                            )
                            .on_click(
                                cx.listener(|this, _, _, cx| this.set_source(VoiceSource::Key, cx)),
                            ),
                        )
                        .when(self.offers_on_device(), |tiles| {
                            tiles.child(
                                source_tile(
                                    "world-voice-on-device",
                                    "◎",
                                    "This Mac's own model",
                                    "Stays on this Mac · free, and needs no key".to_string(),
                                    matches!(source, VoiceSource::OnDevice),
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| this.set_source(VoiceSource::OnDevice, cx),
                                )),
                            )
                        }),
                );
            if matches!(source, VoiceSource::Program) {
                let label = if self.program.is_some() {
                    "Choose another program…"
                } else {
                    "Choose program…"
                };
                sources = sources.child(
                    div().flex().gap_2().items_center().child(
                        ui::button(
                            "world-voice-choose-program",
                            label,
                            ui::ButtonKind::Secondary,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.choose_program(cx))),
                    ),
                );
            }
            if matches!(source, VoiceSource::Key) {
                let mut actions = div().flex().gap_2().items_center().child(
                    ui::button("world-voice-save-key", "Save key", ui::ButtonKind::Primary)
                        .on_click(cx.listener(|this, _, _, cx| this.save_key(cx))),
                );
                if self.key_stored {
                    actions = actions.child(
                        ui::button(
                            "world-voice-forget-key",
                            "Forget key",
                            ui::ButtonKind::Secondary,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.forget_key(cx))),
                    );
                }
                let model = ::world_voice::model_or(self.voice_model.as_deref());
                let model_note = if std::env::var(::world_voice::MODEL_ENV)
                    .is_ok_and(|model| !model.trim().is_empty())
                {
                    format!("Model: {model}, set by WORLD_MACHINE_VOICE_MODEL")
                } else {
                    format!("Model: {model}")
                };
                sources = sources
                    .child(div().w_full().child(self.key_input.clone()))
                    .child(actions)
                    .child(ui::caption(shell::key_store_line(shell::Os::current())))
                    .child(ui::section_label("Claude model"))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .child(self.model_input.clone()),
                            )
                            .child(
                                ui::button(
                                    "world-voice-save-model",
                                    "Save model",
                                    ui::ButtonKind::Secondary,
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.save_model(cx))),
                            ),
                    )
                    .child(ui::caption(model_note))
                    .child(ui::caption("An empty field goes back to the default."));
            }
            if matches!(source, VoiceSource::OnDevice) {
                sources = sources.child(ui::caption(
                    "Apple's model answers what people say to you, through /usr/bin/fm. When it cannot, they answer in the World's own words.",
                ));
            }
            voice = voice.child(focus_ring(self.is_focused(Control::VoiceSource)).child(sources));

            // A second opinion on each answer, before it is said.
            let judge = self.judge;
            let cloud_fact = if self.key_stored {
                format!(
                    "{} reads each answer with your key",
                    ::world_voice::judge_model()
                )
            } else {
                "Needs an API key, saved above".to_string()
            };
            let mut judges = div()
                .flex()
                .gap_3()
                .child(
                    source_tile(
                        "world-voice-judge-off",
                        "–",
                        "No second opinion",
                        "The World's own checks decide, strictly".to_string(),
                        judge == JudgeChoice::Off,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.set_judge(JudgeChoice::Off, cx))),
                )
                .child(
                    source_tile(
                        "world-voice-judge-cloud",
                        "↗",
                        "The key's small model",
                        cloud_fact,
                        judge == JudgeChoice::Cloud,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.set_judge(JudgeChoice::Cloud, cx))),
                );
            if self.offers_on_device() {
                judges = judges.child(
                    source_tile(
                        "world-voice-judge-on-device",
                        "◎",
                        "This Mac's own model",
                        "Stays on this Mac · free".to_string(),
                        judge == JudgeChoice::OnDevice,
                    )
                    .on_click(
                        cx.listener(|this, _, _, cx| this.set_judge(JudgeChoice::OnDevice, cx)),
                    ),
                );
            }
            voice = voice.child(
                focus_ring(self.is_focused(Control::VoiceJudge)).child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(ui::section_label("A second opinion"))
                        .child(judges)
                        .child(ui::caption(
                            "Before anyone says a model's answer, a second model checks that it keeps to the World. It can only turn an answer down; the World then answers in its own words, and the record keeps its verdict.",
                        )),
                ),
            );
        }

        let (state, sentence) = self.state();
        let dot = match state {
            "ready" => tokens::SUCCESS,
            "incomplete" => tokens::WARNING,
            _ => tokens::TEXT_TERTIARY,
        };
        voice
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().size(px(8.0)).rounded_full().bg(ui::color(dot)))
                    .child(ui::body(sentence)),
            )
            .child(ui::caption(
                "Only an API key sends anything off this computer: what you say to someone, and the facts of the World their answer needs. Worlds already open keep the voice they opened with.",
            ))
    }
}

/// The keyboard shortcuts, as this computer's keyboard has them. They
/// cannot be changed yet.
fn render_controls() -> gpui::Div {
    let mut list = group().gap_1().child(ui::caption(
        "The keyboard, wherever a World is open. Keys cannot be changed yet.",
    ));
    for (keys, what) in settings_plan::SHORTCUTS {
        list = list.child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .flex_shrink_0()
                        .w(px(96.0))
                        .text_sm()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(ui::keys_here(keys)),
                )
                .child(div().flex_1().min_w(px(0.0)).child(ui::body(*what))),
        );
    }
    list
}
