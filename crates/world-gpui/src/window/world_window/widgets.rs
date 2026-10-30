//! The small pieces the World window is drawn from: marks and name tags
//! on the scene, the paper, gauges and glyphs, pointers' hints, tabs and
//! buttons, photos and postcards saved to Pictures.

use super::*;

/// What the card says while a model writes someone's answer: residents'
/// words are an AI's when the World voice is on.
pub(crate) const VOICE_NOTICE: &str =
    "World voice is on: an AI model is writing this answer. The World decides what happens.";
/// Under an answer a model wrote.
pub(crate) const VOICED_LABEL: &str = "Written by AI";
/// The command that reports a line, as `conversation::report` reads it.
pub(super) const REPORT_COMMAND: &str = "conversation.report:";

pub(super) fn frame_glows(frame: &diorama::Frame, index: usize) -> bool {
    frame
        .people
        .iter()
        .any(|person| person.index == index && person.glow.is_some())
}

/// Something the player reads (a card, a page), built in the window's own
/// light rather than the night's (see [`world_theme::reading`]), its
/// words too.
pub(super) fn lit<E: Styled>(build: impl FnOnce() -> Option<E>) -> Option<E> {
    world_theme::reading(|| build().map(|surface| surface.text_color(color(tokens::TEXT))))
}

/// [`lit`], for something always there.
pub(super) fn lit_one<E: Styled>(build: impl FnOnce() -> E) -> E {
    world_theme::reading(|| build().text_color(color(tokens::TEXT)))
}

/// How far from a speaker, on screen, someone's name would sit over the
/// speaker's: their name is not shown on hover.
pub(super) const NAME_ROOM: f32 = 72.0;

/// A name under something on the scene.
/// The mark beside whom a favour is for: a small lantern-gold flag on a
/// stick, standing on the ground at their feet with its own soft shadow,
/// its head bobbing gently; named for a screen reader by how the favour
/// is done.
pub(crate) fn favour_mark(x: f32, feet: f32, hint: String, seconds: f32) -> Stateful<Div> {
    const SIDE: f32 = 12.0;
    const TALL: f32 = 30.0;
    const BESIDE: f32 = 14.0;
    let bob = (seconds * 2.4).sin() * 1.5;
    let left = x + BESIDE - SIDE / 2.0;
    div()
        .id("favour-mark")
        .role(Role::Image)
        .aria_label(hint)
        .debug_selector(|| "favour-mark".into())
        .absolute()
        .left(px(left - 4.0))
        .top(px(feet - TALL - SIDE))
        .w(px(SIDE + 8.0))
        .h(px(TALL + SIDE + 4.0))
        // The shadow it casts on the ground.
        .child(
            div()
                .absolute()
                .left(px(0.0))
                .top(px(TALL + SIDE - 2.0))
                .w(px(SIDE + 8.0))
                .h(px(5.0))
                .rounded_full()
                .bg(gpui::black().opacity(0.22)),
        )
        // The stick, from the ground up.
        .child(
            div()
                .absolute()
                .left(px(SIDE / 2.0 + 3.0))
                .top(px(SIDE + bob))
                .w(px(2.0))
                .h(px(TALL - bob))
                .bg(color(tokens::TEXT_SECONDARY)),
        )
        // Its head.
        .child(
            div()
                .absolute()
                .left(px(4.0))
                .top(px(bob))
                .size(px(SIDE))
                .rounded_full()
                .border_2()
                .border_color(color(tokens::SURFACE))
                .bg(color(tokens::ACCENT))
                .shadow_sm(),
        )
}

pub(crate) fn name_tag(name: String) -> Div {
    div()
        .mt_1()
        .px_2()
        .py(px(1.0))
        .rounded_full()
        .bg(scene_paper().opacity(0.88))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(color(tokens::TEXT))
        .whitespace_nowrap()
        .child(name)
}

/// The paper that cards and buttons over the scene are cut from: white
/// by day, and after dusk a dimmer, warmer sheet, as if lit by a lamp, so
/// nothing glares over a night sky. The dark appearance has its own.
pub(crate) fn scene_paper() -> gpui::Rgba {
    // The tests' pictures keep to the day's paper whatever the clock says,
    // unless an hour is pinned.
    if cfg!(test) && scene::pinned_hour().is_none() {
        return paper_at(scene::Daylight::Day, world_theme::is_dark());
    }
    paper_at(scene::daylight_now(), world_theme::is_dark())
}

pub(crate) fn paper_at(daylight: scene::Daylight, dark: bool) -> gpui::Rgba {
    if dark {
        return color(tokens::SURFACE);
    }
    match daylight {
        scene::Daylight::Night => gpui::rgb(0xd8cfbd),
        scene::Daylight::Dusk => gpui::rgb(0xece3d2),
        _ => color(tokens::SURFACE),
    }
}

/// A soft pill over the sky.
pub(super) fn pill() -> Div {
    div()
        .px_3()
        .py(px(6.0))
        .rounded_full()
        .bg(scene_paper().opacity(0.86))
        .shadow_sm()
        .text_sm()
        .text_color(color(tokens::TEXT))
        .flex()
        .items_center()
        .gap_2()
}

/// A gauge as a pill: its name and a short bar, and while a choice is on
/// the table which way the choice would move it and where it would end.
/// How bright (as the eye has it, 0 to 1) a colour on the interface may
/// be after dark, so it is never brighter than the night sky.
pub(super) const NIGHT_BRIGHTNESS: f32 = 0.24;

/// A colour dimmed, if it must be, to be no brighter than the night sky:
/// its hue and its saturation kept.
pub(crate) fn at_night(colour: Hsla) -> Hsla {
    let rgba: gpui::Rgba = colour.into();
    let luma = 0.2126 * rgba.r + 0.7152 * rgba.g + 0.0722 * rgba.b;
    if luma <= NIGHT_BRIGHTNESS {
        return colour;
    }
    let by = NIGHT_BRIGHTNESS / luma;
    gpui::Rgba {
        r: rgba.r * by,
        g: rgba.g * by,
        b: rgba.b * by,
        a: rgba.a,
    }
    .into()
}

pub(super) fn hud_gauge(
    gauge: &world_projection::Gauge,
    shown: f32,
    by: Option<i32>,
) -> Stateful<Div> {
    const BAR: f32 = 64.0;
    // After dark the fill keeps its colour but not its glow: no brighter
    // than the night sky it hangs in.
    let night = |colour: Hsla| {
        if scene::daylight_now() == scene::Daylight::Night {
            at_night(colour)
        } else {
            colour
        }
    };
    let fill: Hsla = night(color(scene::tone_token(gauge.tone)).into());
    let now = shown.clamp(0.0, 1.0);
    let then = by.map(|by| (now + by as f32 / 1000.0).clamp(0.0, 1.0));
    let mut bar = div()
        .relative()
        .w(px(BAR))
        .h(px(6.0))
        .rounded_full()
        .bg(color(tokens::BORDER))
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .h_full()
                .w(px(BAR * now))
                .rounded_full()
                .bg(fill),
        );
    if let Some(then) = then {
        let (from, to) = if then >= now {
            (now, then)
        } else {
            (then, now)
        };
        bar = bar.child(
            div()
                .absolute()
                .top_0()
                .left(px(BAR * from))
                .h_full()
                .w(px(BAR * (to - from)))
                .rounded_full()
                .bg(night(color(tokens::ACCENT).into()).opacity(0.55)),
        );
    }
    // The reading in the World's own words, a number where it counts
    // one ("11,485"), so a cost can be weighed against it.
    let mut pill = pill()
        .id(SharedString::from(format!("gauge-{}", gauge.id)))
        .role(Role::Meter)
        .aria_label(format!("{}: {}", gauge.label, gauge.reading))
        .child(
            div()
                .text_xs()
                .text_color(color(tokens::TEXT_SECONDARY))
                .child(gauge.label.clone()),
        )
        .child(bar)
        .when(!gauge.reading.trim().is_empty(), |pill| {
            pill.child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color(tokens::TEXT))
                    .child(gauge.reading.clone()),
            )
        });
    if let Some(by) = by.filter(|by| *by != 0) {
        pill = pill.child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(tokens::ACCENT_TEXT))
                .child(scene::movement_arrows(by)),
        );
    }
    pill
}

/// The hands' handle: a plus, pale on the accent while open.
pub(super) fn plus_glyph(open: bool) -> gpui::Canvas<()> {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let ink: Hsla = if open {
                color(tokens::SURFACE).into()
            } else {
                color(tokens::TEXT_SECONDARY).into()
            };
            let x = f32::from(bounds.origin.x);
            let y = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            art::rect(window, x + w / 2.0 - 1.0, y + 2.0, 2.0, h - 4.0, 1.0, ink);
            art::rect(window, x + 2.0, y + h / 2.0 - 1.0, w - 4.0, 2.0, 1.0, ink);
        },
    )
}

/// The strip's handle: a long, low window with a sliver of land in it.
pub(super) fn strip_glyph() -> gpui::Canvas<()> {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let ink: Hsla = color(tokens::TEXT_SECONDARY).into();
            let x = f32::from(bounds.origin.x);
            let y = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            // The screen's edge, and the strip along its foot.
            art::rect(window, x + 1.0, y + 2.0, w - 2.0, 1.5, 0.5, ink);
            art::rect(window, x + 1.0, y + 2.0, 1.5, h - 4.0, 0.5, ink);
            art::rect(window, x + w - 2.5, y + 2.0, 1.5, h - 4.0, 0.5, ink);
            art::rect(
                window,
                x + 1.0,
                y + h * 0.62,
                w - 2.0,
                h * 0.38 - 2.0,
                1.0,
                ink,
            );
        },
    )
}

/// A pointer's little arrow, in the accent colour.
pub(super) fn caret(direction: Caret) -> gpui::Canvas<()> {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let ink: Hsla = color(tokens::ACCENT).into();
            let x = f32::from(bounds.origin.x);
            let y = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            let points = match direction {
                Caret::Up => [(x, y + h), (x + w, y + h), (x + w / 2.0, y)],
                Caret::Down => [(x, y), (x + w, y), (x + w / 2.0, y + h)],
                Caret::Left => [(x + w, y), (x + w, y + h), (x, y + h / 2.0)],
            };
            art::polygon(window, &points, ink);
        },
    )
}

/// A gentle pointer at `pointer`'s thing: what it is and how to reach it,
/// with a little arrow toward it and a way to put it away. A screen reader
/// hears it as a status as it arrives.
pub(crate) fn pointer_hint(
    pointer: Pointer,
    direction: Caret,
    dismiss: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> Stateful<Div> {
    let words = ui::t(pointer.words());
    let note = div()
        .w(px(HINT_WIDTH))
        .flex()
        .items_start()
        .gap_2()
        .px_3()
        .py_2()
        .rounded_lg()
        .bg(color(tokens::SURFACE))
        .border_1()
        .border_color(color(tokens::ACCENT))
        .shadow_md()
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .text_sm()
                .text_color(color(tokens::TEXT))
                .child(crate::wrap::text(words.clone())),
        )
        .child(
            ui::named(
                div().id(SharedString::from(format!(
                    "pointer-dismiss-{}",
                    pointer.key()
                ))),
                "Dismiss this hint",
            )
            .role(Role::Button)
            .flex_shrink_0()
            .size(px(20.0))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(color(tokens::TEXT_SECONDARY))
            .cursor_pointer()
            .hover(|style| style.bg(color(tokens::ROW_HOVER)))
            .child("×")
            .on_click(dismiss),
        );
    let hint = div()
        .id(SharedString::from(format!("pointer-{}", pointer.key())))
        .role(Role::Status)
        .aria_label(words)
        .debug_selector(|| format!("pointer-{}", pointer.key()))
        .flex();
    match direction {
        Caret::Up => hint
            .flex_col()
            .items_end()
            .child(caret(direction).w(px(14.0)).h(px(8.0)).mr(px(13.0)))
            .child(note),
        Caret::Down => hint
            .flex_col()
            .items_start()
            .child(note)
            .child(caret(direction).w(px(14.0)).h(px(8.0)).ml(px(24.0))),
        Caret::Left => hint
            .flex_row()
            .items_center()
            .child(caret(direction).w(px(8.0)).h(px(14.0)))
            .child(note),
    }
}

/// One of the zoom control's two buttons, greyed at the end of its travel.
pub(crate) fn zoom_button(
    id: &'static str,
    glyph: &'static str,
    name: &'static str,
    spent: bool,
) -> Stateful<Div> {
    ui::named(div().id(id), name)
        .role(Role::Button)
        .size(px(28.0))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .text_base()
        .font_weight(FontWeight::MEDIUM)
        .text_color(color(if spent {
            tokens::TEXT_TERTIARY
        } else {
            tokens::TEXT_SECONDARY
        }))
        .when(!spent, |button| {
            button
                .cursor_pointer()
                .hover(|style| style.bg(color(tokens::ROW_HOVER)))
        })
        .child(glyph)
}

/// The drawer's handle: three short lines.
pub(super) fn drawer_glyph() -> gpui::Canvas<()> {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let ink: Hsla = color(tokens::TEXT_SECONDARY).into();
            let x = f32::from(bounds.origin.x);
            let y = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            for row in 0..3 {
                art::rect(
                    window,
                    x + 1.0,
                    y + h * (0.22 + 0.28 * row as f32),
                    w - 2.0,
                    2.0,
                    1.0,
                    ink,
                );
            }
        },
    )
}

/// One entry in a list a screen reader reads out whole: a letter, a
/// keepsake, a chapter, a thing in the book.
pub(crate) fn list_entry(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    div().id(id).role(Role::ListItem).aria_label(label)
}

/// One of the hands' verbs, as a tab: chosen or not.
pub(crate) fn verb_tab(verb: &'static str, chosen: bool) -> Stateful<Div> {
    labelled_tab(verb, verb, chosen)
}

/// A verb's tab under another name.
pub(crate) fn labelled_tab(verb: &'static str, label: &'static str, chosen: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(format!("hands-verb-{verb}")))
        .role(Role::Tab)
        .aria_label(ui::t(label))
        .aria_selected(chosen)
        .px_2()
        .py(px(3.0))
        .rounded_full()
        .text_sm()
        .cursor_pointer()
        .when(chosen, |tab| {
            tab.bg(color(tokens::ACCENT))
                .text_color(color(tokens::SURFACE))
        })
        .when(!chosen, |tab| {
            tab.text_color(color(tokens::TEXT_SECONDARY))
                .hover(|style| style.bg(color(tokens::ROW_SELECTED)))
        })
        .child(ui::t(label))
}

/// One answer to the question on the card, the `position`th of `count`:
/// a button, leaned toward (`chosen`) or not. One that cannot be chosen
/// now is shown greyed so the player sees what it would have been, with
/// why not when they lean on it; a screen reader hears why at once.
pub(super) fn answer_button(
    position: usize,
    count: usize,
    title: String,
    chosen: bool,
    unavailable: Option<&str>,
) -> Stateful<Div> {
    let row = div()
        .id(SharedString::from(format!("answer-{position}")))
        .role(Role::Button)
        .aria_label(title.clone())
        .aria_position_in_set(position + 1)
        .aria_size_of_set(count)
        .px_3()
        .py(px(5.0))
        .rounded_lg()
        .border_1()
        .text_sm();
    if let Some(reason) = unavailable {
        let row = row
            .when(!reason.is_empty(), |row| {
                row.aria_description(crate::i18n::fill(
                    "Not now: {reason}",
                    &[("reason", &world_i18n::tr_owned(reason))],
                ))
            })
            .border_color(color(tokens::BORDER))
            .text_color(color(tokens::TEXT_TERTIARY))
            .flex()
            .justify_between()
            .gap_3()
            .child(crate::wrap::text(title));
        return if chosen && !reason.is_empty() {
            row.child(div().text_xs().child(reason.to_string()))
        } else {
            row
        };
    }
    row.cursor_pointer()
        .font_weight(FontWeight::MEDIUM)
        .border_color(color(if chosen {
            tokens::ACCENT
        } else {
            tokens::BORDER
        }))
        .bg(color(if chosen {
            tokens::ACCENT_SOFT
        } else {
            tokens::SURFACE
        }))
        .text_color(color(if chosen {
            tokens::ACCENT_TEXT
        } else {
            tokens::TEXT
        }))
        .hover(|style| style.border_color(color(tokens::ACCENT)))
        .child(crate::wrap::text(title))
}

pub(crate) fn arrow_button(
    id: &'static str,
    glyph: &'static str,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    ui::named(div().id(id), label)
        .role(Role::Button)
        .size(px(26.0))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .text_base()
        .text_color(color(tokens::TEXT_SECONDARY))
        .cursor_pointer()
        .hover(|style| {
            style
                .bg(color(tokens::ROW_HOVER))
                .text_color(color(tokens::TEXT))
        })
        .child(glyph)
        .on_click(on_click)
}

/// How someone stands with the player: five small marks, as many filled as
/// they are warm towards them, and the words for it.
pub(super) fn standing_row(standing: &world_projection::Standing) -> Div {
    let filled = (standing.level.clamp(-2, 2) + 3) as usize;
    let tone = match standing.level {
        1.. => tokens::SUCCESS,
        0 => tokens::ACCENT,
        _ => tokens::DANGER,
    };
    let mut marks = div().flex().items_center().gap(px(3.0));
    for index in 0..5 {
        marks = marks.child(
            div()
                .w(px(6.0))
                .h(px(6.0))
                .rounded_full()
                .bg(color(if index < filled { tone } else { tokens::BORDER })),
        );
    }
    div().flex().items_center().gap_2().child(marks).child(
        div()
            .text_xs()
            .text_color(color(tokens::TEXT_SECONDARY))
            .child(standing.words.clone()),
    )
}

/// A person thinking: one to three dots, turning over while they do.
pub(super) fn thinking_dots(since: Option<Instant>) -> String {
    let beat = since.map_or(0, |since| since.elapsed().as_millis() / 400) % 3;
    ".".repeat(beat as usize + 1)
}

/// Saves the screen area `bounds` covers as a picture in the player's
/// Pictures, in a World Machine folder, named after the World and the
/// time. Whether it was saved.
pub(super) fn save_photo(bounds: gpui::Bounds<gpui::Pixels>, title: &str) -> bool {
    save_picture(bounds, title, true)
}

/// Saves the screen area `bounds` covers to Pictures/World Machine as
/// `<name>.png`, with the time after the name when `stamped` (a photo) or
/// only when a picture of that name is already there (a postcard, which is
/// named after its day).
pub(crate) fn save_picture(bounds: gpui::Bounds<gpui::Pixels>, title: &str, stamped: bool) -> bool {
    // Windows names the home folder USERPROFILE.
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return false;
    };
    let folder = std::path::Path::new(&home)
        .join("Pictures")
        .join("World Machine");
    if std::fs::create_dir_all(&folder).is_err() {
        return false;
    }
    let name = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let stamp = chrono::Local::now().format("%Y-%m-%d at %H.%M.%S");
    let plain = folder.join(format!("{}.png", name.trim()));
    let path = if stamped || plain.exists() {
        folder.join(format!("{} {stamp}.png", name.trim()))
    } else {
        plain
    };
    let region = format!(
        "{},{},{},{}",
        f32::from(bounds.origin.x).round(),
        f32::from(bounds.origin.y).round(),
        f32::from(bounds.size.width).round(),
        f32::from(bounds.size.height).round()
    );
    capture_region(&region, &path) && path.is_file()
}

/// Copies the screen area `region` ("x,y,width,height" in the window's
/// points) to the PNG at `path`. Not offscreen: GPUI renders a frame to an
/// image (`Window::render_to_image`, Metal and DirectX) only with its
/// `test-support` feature at the pinned revision, so until it does, this is
/// the operating system's own copy of the screen: `screencapture` on the
/// Mac, .NET's `CopyFromScreen` through Windows PowerShell on Windows
/// (DPI-unaware, so it takes the same points GPUI gives), ImageMagick's
/// `import` or `grim` on Linux, nothing elsewhere.
pub(super) fn capture_region(region: &str, path: &std::path::Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/usr/sbin/screencapture")
            .arg("-x")
            .arg("-R")
            .arg(region)
            .arg(path)
            .status()
            .is_ok_and(|status| status.success())
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let numbers = region
            .split(',')
            .filter_map(|part| part.parse::<f64>().ok())
            .map(|value| value.round() as i64)
            .collect::<Vec<_>>();
        let [x, y, width, height] = numbers[..] else {
            return false;
        };
        let target = path.to_string_lossy().replace('\'', "''");
        let script = format!(
            "Add-Type -AssemblyName System.Drawing; \
             $b = New-Object System.Drawing.Bitmap {width}, {height}; \
             $g = [System.Drawing.Graphics]::FromImage($b); \
             $g.CopyFromScreen({x}, {y}, 0, 0, $b.Size); \
             $b.Save('{target}', [System.Drawing.Imaging.ImageFormat]::Png)"
        );
        let system = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        std::process::Command::new(
            std::path::Path::new(&system)
                .join("System32")
                .join("WindowsPowerShell")
                .join("v1.0")
                .join("powershell.exe"),
        )
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .is_ok_and(|status| status.success())
    }
    // Linux (a preview build, `linux-window`): ImageMagick's `import` on
    // X11, `grim` on Wayland, whichever is there; nothing otherwise.
    #[cfg(target_os = "linux")]
    {
        let numbers = region
            .split(',')
            .filter_map(|part| part.parse::<f64>().ok())
            .map(|value| value.round() as i64)
            .collect::<Vec<_>>();
        let [x, y, width, height] = numbers[..] else {
            return false;
        };
        let ran = |program: &str, args: &[&std::ffi::OsStr]| {
            std::process::Command::new(program)
                .args(args)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        };
        let crop = format!("{width}x{height}+{x}+{y}");
        let geometry = format!("{x},{y} {width}x{height}");
        let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
        let x11 = || {
            ran(
                "import",
                &[
                    "-silent".as_ref(),
                    "-window".as_ref(),
                    "root".as_ref(),
                    "-crop".as_ref(),
                    crop.as_ref(),
                    "+repage".as_ref(),
                    path.as_os_str(),
                ],
            )
        };
        let grim = || {
            ran(
                "grim",
                &["-g".as_ref(), geometry.as_ref(), path.as_os_str()],
            )
        };
        let tries: [&dyn Fn() -> bool; 2] = if wayland {
            [&grim, &x11]
        } else {
            [&x11, &grim]
        };
        tries.iter().any(|capture| capture())
    }
    #[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
    {
        let _ = (region, path);
        false
    }
}

/// The paper of a postcard around a scene `width` by `height`: an even
/// border, and at the foot a band with the caption, who said it, and the
/// World and the day.
pub(crate) fn postcard_paper(card: &crate::postcard::Postcard, width: f32, height: f32) -> Div {
    let layout = crate::postcard::postcard_layout(width, height);
    let paper = gpui::rgb(crate::postcard::PAPER);
    let ink = gpui::rgb(crate::postcard::INK);
    let soft = gpui::rgb(crate::postcard::INK_SOFT);
    let edge = |div: Div| div.absolute().bg(paper);
    div()
        .absolute()
        .top_0()
        .left_0()
        .w(px(width))
        .h(px(height))
        .child(edge(div()).top_0().left_0().right_0().h(px(layout.border)))
        .child(edge(div()).top_0().bottom_0().left_0().w(px(layout.border)))
        .child(
            edge(div())
                .top_0()
                .bottom_0()
                .right_0()
                .w(px(layout.border)),
        )
        // A hairline where the picture meets the paper, as if pasted on.
        .child(
            div()
                .absolute()
                .top(px(layout.border))
                .left(px(layout.border))
                .w(px(layout.scene_width))
                .h(px(layout.scene_height))
                .border_1()
                .border_color(gpui::rgb(crate::postcard::INK_SOFT).opacity(0.35)),
        )
        .child(
            edge(div())
                .bottom_0()
                .left_0()
                .right_0()
                .h(px(layout.band))
                .px(px(layout.border * 1.5))
                .flex()
                .flex_col()
                .justify_center()
                .gap_1()
                .child(
                    div()
                        .text_color(ink)
                        .text_lg()
                        .italic()
                        .line_clamp(2)
                        .child(crate::wrap::text(card.printed_caption())),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap_4()
                        .text_sm()
                        .text_color(soft)
                        .child(card.signature())
                        .child(div().flex_shrink_0().child(ui::t("World Machine"))),
                ),
        )
}
