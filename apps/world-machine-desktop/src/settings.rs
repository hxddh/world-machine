//! The Settings window.
//!
//! One place to say how this app's Worlds should speak and sound, and how
//! the app is shown.
//!
//! The window is deliberately plain and says what is true rather than what is
//! encouraging: which way the voice reaches a model, that a key is the only way
//! a World's contents leave this Mac, and what happens when it is switched off.

use gpui::{
    div, prelude::*, px, size, App, AppContext, Bounds, Context, Entity, IntoElement,
    PathPromptOptions, Render, SharedString, Styled, Window, WindowBounds, WindowOptions,
};
use world_gpui::ui;
use world_machine_desktop::ambience;
use world_machine_desktop::app_settings::{self, VoiceSource};
use world_machine_desktop::key_store;
use world_theme::tokens;

use crate::diagnostics;
use world_gpui::text_input::{self as text_field, TextInput};

pub fn open(cx: &mut App) {
    text_field::bind_keys(cx);
    let bounds = Bounds::centered(None, size(px(600.0), px(640.0)), cx);
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

struct SettingsView {
    key_input: Entity<TextInput>,
    /// Whether a key is stored, read once when the window opens and after every
    /// change. The key itself is never held here.
    key_stored: bool,
    voice_on: bool,
    sound_on: bool,
    /// The level of each sound, in `ambience::Channel::ALL` order.
    levels: [u8; 4],
    /// The language chosen ("en", "zh-Hans"), or `None` to follow the Mac.
    language: Option<String>,
    /// How large text is drawn, in percent.
    text_scale: u32,
    /// More contrast, fewer or as the Mac says.
    contrast: Option<bool>,
    source: VoiceSource,
    program: Option<String>,
    status: Option<SharedString>,
}

impl SettingsView {
    fn new(cx: &mut Context<Self>) -> Self {
        let key_input = cx.new(|cx| TextInput::new("Paste an API key…", cx));
        cx.observe(&key_input, |_, _, cx| cx.notify()).detach();
        let mut view = Self {
            key_input,
            key_stored: false,
            voice_on: false,
            sound_on: false,
            levels: ambience::Channel::ALL.map(ambience::Channel::default_level),
            language: None,
            text_scale: 100,
            contrast: None,
            source: VoiceSource::Program,
            program: None,
            status: None,
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
            .and_then(|root| app_settings::load(&root).ok());
        match settings {
            Some(settings) => {
                self.voice_on = settings.world_voice;
                self.sound_on = settings.ambient_sound;
                self.levels = ambience::Channel::ALL.map(|channel| settings.sound_level(channel));
                self.language = settings.language.clone();
                self.text_scale = settings.text_scale.unwrap_or(100);
                self.contrast = settings.increase_contrast;
                self.source = settings.world_voice_source.unwrap_or_default();
                self.program = settings.pi_program.map(|path| path.display().to_string());
            }
            None => {
                self.voice_on = false;
                self.sound_on = false;
                self.levels = ambience::Channel::ALL.map(ambience::Channel::default_level);
                self.language = None;
                self.text_scale = 100;
                self.contrast = None;
                self.source = VoiceSource::Program;
                self.program = None;
            }
        }
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

    fn set_voice(&mut self, on: bool, cx: &mut Context<Self>) {
        self.apply(
            move || {
                let root =
                    app_settings::application_support_root().map_err(|error| error.to_string())?;
                app_settings::save_world_voice(&root, on).map_err(|error| error.to_string())
            },
            cx,
        );
    }

    fn set_sound(&mut self, on: bool, cx: &mut Context<Self>) {
        self.apply(
            move || {
                let root =
                    app_settings::application_support_root().map_err(|error| error.to_string())?;
                app_settings::save_ambient_sound(&root, on).map_err(|error| error.to_string())
            },
            cx,
        );
        ambience::set_enabled(self.sound_on);
    }

    /// Shows every window the way the player now asks.
    fn show_as_chosen(&mut self, cx: &mut Context<Self>) {
        let settings = app_settings::application_support_root()
            .ok()
            .and_then(|root| app_settings::load(&root).ok());
        world_machine_desktop::display::apply(settings.as_ref());
        cx.refresh_windows();
    }

    fn set_language(&mut self, language: Option<String>, cx: &mut Context<Self>) {
        self.apply(
            move || {
                let root =
                    app_settings::application_support_root().map_err(|error| error.to_string())?;
                app_settings::save_language(&root, language).map_err(|error| error.to_string())
            },
            cx,
        );
        self.show_as_chosen(cx);
    }

    fn set_text_scale(&mut self, percent: u32, cx: &mut Context<Self>) {
        self.apply(
            move || {
                let root =
                    app_settings::application_support_root().map_err(|error| error.to_string())?;
                app_settings::save_text_scale(&root, percent).map_err(|error| error.to_string())
            },
            cx,
        );
        self.show_as_chosen(cx);
    }

    fn set_contrast(&mut self, on: Option<bool>, cx: &mut Context<Self>) {
        self.apply(
            move || {
                let root =
                    app_settings::application_support_root().map_err(|error| error.to_string())?;
                app_settings::save_increase_contrast(&root, on).map_err(|error| error.to_string())
            },
            cx,
        );
        self.show_as_chosen(cx);
    }

    fn set_level(&mut self, channel: ambience::Channel, percent: u8, cx: &mut Context<Self>) {
        self.apply(
            move || {
                let root =
                    app_settings::application_support_root().map_err(|error| error.to_string())?;
                app_settings::save_sound_level(&root, channel, percent)
                    .map_err(|error| error.to_string())
            },
            cx,
        );
        for (channel, level) in ambience::Channel::ALL.into_iter().zip(self.levels) {
            ambience::set_level(channel, level);
        }
    }

    fn set_source(&mut self, source: VoiceSource, cx: &mut Context<Self>) {
        self.apply(
            move || {
                let root =
                    app_settings::application_support_root().map_err(|error| error.to_string())?;
                app_settings::save_world_voice_source(&root, source)
                    .map_err(|error| error.to_string())
            },
            cx,
        );
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
                this.apply(
                    move || {
                        let root = app_settings::application_support_root()
                            .map_err(|error| error.to_string())?;
                        app_settings::save_pi_program(&root, path)
                            .map_err(|error| error.to_string())
                    },
                    cx,
                )
            });
        })
        .detach();
    }

    fn forget_key(&mut self, cx: &mut Context<Self>) {
        self.apply(key_store::clear, cx);
    }

    /// What is true right now, as a few words and how settled it is.
    fn state(&self) -> (&'static str, String) {
        if !self.voice_on {
            return (
                "off",
                "Off. Worlds read from the app's own copy, and nothing leaves this Mac.".into(),
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

/// One of two ways the voice can reach a model: a tile that says what it is
/// and the one fact that matters about it, selected like a radio button.
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

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        window.set_rem_size(gpui::px(world_gpui::rem_size()));
        window.set_window_title("World Machine Settings");

        let on = self.voice_on;
        let source = self.source;

        let voice = group().child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(ui::row_title("Let Worlds speak for themselves"))
                        .child(ui::caption(
                            "Coming back, a World tells you what happened in its own words.",
                        )),
                )
                .child(
                    switch("world-voice-switch", on)
                        .aria_label(ui::t("Let Worlds speak for themselves"))
                        .on_click(cx.listener(move |this, _, _, cx| this.set_voice(!on, cx))),
                ),
        );

        let mut page = div()
            .size_full()
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .bg(ui::color(tokens::WINDOW))
            .text_color(ui::color(tokens::TEXT))
            .child(ui::page_title("World voice"))
            .child(voice);

        if on {
            let program_fact = match self.program.as_deref() {
                Some(program) => format!("Stays on this Mac · {}", program_name(program)),
                None => "Stays on this Mac · choose the program below".into(),
            };
            let key_fact = if self.key_stored {
                "Each return is sent to your provider · key stored".to_string()
            } else {
                "Each return is sent to your provider".to_string()
            };
            let mut sources = group()
                .child(ui::section_label("Where the voice comes from"))
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .child(
                            source_tile(
                                "world-voice-program",
                                "⌂",
                                "A program on this Mac",
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
                        ),
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
                sources = sources
                    .child(div().w_full().child(self.key_input.clone()))
                    .child(actions)
                    .child(ui::caption(
                        "The key goes into your login keychain, never into a file.",
                    ));
            }
            page = page.child(sources);
        }

        let (state, sentence) = self.state();
        let dot = match state {
            "ready" => tokens::SUCCESS,
            "incomplete" => tokens::WARNING,
            _ => tokens::TEXT_TERTIARY,
        };
        page = page.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().size(px(8.0)).rounded_full().bg(ui::color(dot)))
                .child(ui::body(sentence)),
        );
        if let Some(status) = self.status.clone() {
            page = page.child(
                div()
                    .text_sm()
                    .text_color(ui::color(tokens::DANGER))
                    .child(status),
            );
        }
        let sound_on = self.sound_on;
        let mut mixer = group();
        for (index, channel) in ambience::Channel::ALL.into_iter().enumerate() {
            let (name, what) = match channel {
                ambience::Channel::Music => (
                    "Music",
                    "Chords, a morning tune and an evening one, by the hour and the weather",
                ),
                ambience::Channel::Ambience => ("Landscape", "Wind and the hum of the place"),
                ambience::Channel::Voices => ("Voices", "Everyone's own babble as they speak"),
                ambience::Channel::Interface => (
                    "Ticks and bells",
                    "Cards turning, turns passing, things built",
                ),
            };
            let level = self.levels[index];
            let mut steps = div().flex_shrink_0().flex().items_end().gap_1();
            for step in 0..=4_u8 {
                let percent = step * 25;
                let lit = percent <= level && level > 0;
                steps = steps.child(
                    div()
                        .id(("sound-level", usize::from(step) + index * 5))
                        .w(px(22.0))
                        .h(px(8.0 + f32::from(step) * 3.0))
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(ui::color(if lit {
                            tokens::ACCENT
                        } else {
                            tokens::BORDER_STRONG
                        }))
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.set_level(channel, percent, cx)),
                        ),
                );
            }
            mixer = mixer.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(ui::row_title(name))
                            .child(ui::caption(what)),
                    )
                    .child(steps),
            );
        }
        page.child(ui::caption(
            "Only an API key sends anything off this Mac: what a World has recorded, once per return. Worlds already open keep the voice they opened with.",
        ))
        .child(div().pt_4().child(ui::page_title("Sound")))
        .child(
            group().child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(ui::row_title("Sound"))
                            .child(ui::caption(
                                "The World in front plays its music and its landscape's sound, everyone speaks in a voice of their own, and cards, turns and building each have a small sound.",
                            )),
                    )
                    .child(
                        switch("ambient-sound-switch", sound_on)
                            .aria_label(ui::t("Sound"))
                            .on_click(cx.listener(move |this, _, _, cx| this.set_sound(!sound_on, cx))),
                    ),
            ),
        )
        .when(sound_on, |page| page.child(mixer))
        .child(div().pt_4().child(ui::page_title("Display")))
        .child(self.render_display(cx))
    }
}

impl SettingsView {
    /// Language, text size and contrast.
    fn render_display(&self, cx: &mut Context<Self>) -> gpui::Div {
        let chip = |id: SharedString, label: String, selected: bool| {
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
        };
        let row = |title: &'static str, what: &'static str| {
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap_1()
                .child(ui::row_title(title))
                .child(ui::caption(what))
        };
        let mut languages = div().flex().flex_wrap().gap_2();
        for (index, (id, label)) in [
            (None, ui::t("Follow the Mac").to_string()),
            (Some("en"), "English".to_string()),
            (Some("zh-Hans"), "简体中文".to_string()),
        ]
        .into_iter()
        .enumerate()
        {
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
        for percent in [100_u32, 125, 150, 175, 200] {
            sizes = sizes.child(
                chip(
                    SharedString::from(format!("text-size-{percent}")),
                    format!("{percent}%"),
                    self.text_scale == percent,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.set_text_scale(percent, cx))),
            );
        }
        let contrast_on = self
            .contrast
            .unwrap_or_else(world_machine_desktop::display::system_increase_contrast);
        group()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(row("Language", "The app and the Worlds that come with it."))
                    .child(languages),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(row(
                        "Text size",
                        "Everything written, from cards to speech.",
                    ))
                    .child(sizes),
            )
            .child(
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
            )
    }
}
