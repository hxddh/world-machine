//! The stories a World tells about itself, on paper over the scene:
//! someone's legend, the three panels of a moment, and the almanac of a
//! year. Presentation only: each is asked of the World and shown as told.

use super::world_window::{chapter_just_ended, greeting_seconds, likeness_of, portrait, Likeness};
use super::*;
use crate::panels::{self, Beat, Occasion, PanelScene};
use crate::postcard::{INK, INK_SOFT, PAPER};
use gpui::{canvas, KeyDownEvent, Role, ScrollHandle, Stateful};
use std::collections::BTreeSet;
use std::time::Instant;
use world_projection::{
    Almanac, Legend, LegendLine, Moment, MomentKind, PanelBeat, StoryPage, StoryRequest,
};

/// A story open on the page.
#[derive(Clone, Debug)]
pub(crate) enum Story {
    Legend(Legend),
    /// A year in review, with its best moment when the World could tell it.
    Almanac(Almanac, Option<Moment>),
    Moment(Moment),
}

/// What the stories need the window to keep: presentation only.
#[derive(Default)]
pub(crate) struct Reading {
    /// The page open now, and the pages it was opened from.
    pub(crate) page: Option<Story>,
    pub(crate) back: Vec<Story>,
    /// Which line of a legend the keyboard is on.
    pub(crate) line: usize,
    pub(crate) scroll: ScrollHandle,
    /// The moments the player has seen, or had the chance to: none shows
    /// twice. Unset until the window first draws.
    pub(crate) seen: Option<BTreeSet<String>>,
    /// A moment just come, shown as a strip over the scene, and since when.
    pub(crate) shown: Option<(Moment, Instant)>,
    /// When a strip was last saved as a picture, and whether one is being
    /// saved now.
    pub(crate) saved: Option<Instant>,
    pub(crate) saving: bool,
}

/// The kind of a moment as the painter draws it.
pub(crate) fn occasion(kind: MomentKind) -> Occasion {
    match kind {
        MomentKind::Wedding => Occasion::Wedding,
        MomentKind::Birth => Occasion::Birth,
        MomentKind::ComingOfAge => Occasion::ComingOfAge,
        MomentKind::Farewell => Occasion::Farewell,
        MomentKind::Death => Occasion::Death,
        MomentKind::Storm => Occasion::Storm,
        MomentKind::WorkOpened => Occasion::WorkOpened,
        MomentKind::Festival => Occasion::Festival,
        MomentKind::Other => Occasion::Other,
    }
}

pub(crate) fn beat(beat: PanelBeat) -> Beat {
    match beat {
        PanelBeat::Before => Beat::Before,
        PanelBeat::Moment => Beat::Moment,
        PanelBeat::After => Beat::After,
    }
}

/// What each beat is called under its panel, for a screen reader.
pub(crate) fn beat_name(beat: PanelBeat) -> &'static str {
    match beat {
        PanelBeat::Before => "Before",
        PanelBeat::Moment => "The moment",
        PanelBeat::After => "After",
    }
}

/// The scene of each of a moment's panels, as the scene draws its place
/// and people now.
pub(crate) fn panel_scenes(snapshot: &ProjectionSnapshot, moment: &Moment) -> Vec<PanelScene> {
    moment
        .panels
        .iter()
        .enumerate()
        .map(|(index, panel)| {
            let mut scene = panels::panel_scene(
                snapshot,
                occasion(moment.kind),
                beat(panel.beat),
                panel.place,
                &panel.cast,
                panel.mood,
                &format!("{}-{index}", moment.id),
            );
            scene.props = panel.props.clone();
            scene
        })
        .collect()
}

/// How a moment's strip sits over a stage `width` by `height`: its box,
/// and each panel's size, three side by side with room for a caption
/// under each.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StripLayout {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) panel_w: f32,
    pub(crate) panel_h: f32,
    pub(crate) gap: f32,
    pub(crate) pad: f32,
}

/// Room under the panels for the title above and captions below.
const STRIP_TITLE: f32 = 64.0;
const STRIP_CAPTION: f32 = 64.0;
const STRIP_FOOT: f32 = 48.0;

pub(crate) fn strip_layout(width: f32, height: f32) -> StripLayout {
    let pad = (width * 0.025).clamp(16.0, 28.0);
    let gap = (width * 0.014).clamp(10.0, 18.0);
    let room_w = (width - 48.0).max(240.0);
    let room_h = (height - 64.0).max(200.0);
    // Panels are four by three, and no larger than looks well.
    let by_width = (room_w - 2.0 * pad - 2.0 * gap) / 3.0;
    let by_height =
        (room_h - 2.0 * pad - STRIP_TITLE - STRIP_CAPTION * crate::text_scale() - STRIP_FOOT) * 4.0
            / 3.0;
    let panel_w = by_width.min(by_height).clamp(96.0, 300.0);
    let panel_h = (panel_w * 0.75).round();
    let panel_w = panel_w.round();
    let w = 3.0 * panel_w + 2.0 * gap + 2.0 * pad;
    let h = 2.0 * pad + STRIP_TITLE + panel_h + STRIP_CAPTION * crate::text_scale() + STRIP_FOOT;
    StripLayout {
        x: ((width - w) / 2.0).max(0.0).round(),
        // A little above the middle, the way a picture is hung.
        y: ((height - h) * 0.42).max(8.0).round(),
        w,
        h,
        panel_w,
        panel_h,
        gap,
        pad,
    }
}

/// A legend's lines by day, oldest first: each day once, with its lines
/// in the order they happened.
pub(crate) fn legend_days(lines: &[LegendLine]) -> Vec<(u32, Vec<&LegendLine>)> {
    let mut days: Vec<(u32, Vec<&LegendLine>)> = Vec::new();
    for line in lines {
        match days.last_mut() {
            Some((day, lines)) if *day == line.day => lines.push(line),
            _ => days.push((line.day, vec![line])),
        }
    }
    days
}

/// How long someone has been part of the place, from the first day their
/// legend tells to `today`: in years once it has been a year (when the
/// World says how long a year is); under a year in seasons, when the
/// World has them, or months; in days only when the World counts no years.
pub(crate) fn time_here(first: u32, today: u32, year: Option<u64>, seasons: bool) -> String {
    let days = u64::from(today.saturating_sub(first) + 1);
    let Some(year) = year.filter(|year| *year > 0) else {
        return if days == 1 {
            "A day in the town".to_string()
        } else {
            format!("{days} days in the town")
        };
    };
    if days >= year {
        return match days / year {
            1 => "A year in the town".to_string(),
            years => format!("{years} years in the town"),
        };
    }
    if seasons {
        return match days * 4 / year {
            0 => "New to the town".to_string(),
            1 => "A season in the town".to_string(),
            2 => "Two seasons in the town".to_string(),
            _ => "Three seasons in the town".to_string(),
        };
    }
    match days * 12 / year {
        0 => "New to the town".to_string(),
        1 => "A month in the town".to_string(),
        months => format!("{months} months in the town"),
    }
}

/// The almanac's lists that have anything in them, with their headings,
/// in the order the page reads them.
pub(crate) fn almanac_sections(almanac: &Almanac) -> Vec<(&'static str, &[String])> {
    [
        ("Came to live here", almanac.arrived.as_slice()),
        ("Moved away", almanac.left.as_slice()),
        ("Born", almanac.born.as_slice()),
        ("Died", almanac.died.as_slice()),
        ("Built", almanac.built.as_slice()),
    ]
    .into_iter()
    .filter(|(_, names)| !names.is_empty())
    .collect()
}

/// The words a legend line is read out as: its day, what happened, and
/// why.
pub(crate) fn line_label(unit: &str, line: &LegendLine) -> String {
    let because = line
        .because
        .as_deref()
        .map(|because| format!(" ({because})"))
        .unwrap_or_default();
    format!(
        "{}: {}{because}",
        crate::i18n::day_label(unit, line.day),
        line.text
    )
}

/// The size of a moment's panel as painted, and its picture if it has
/// arrived from the painter's threads: asked for off the window's thread
/// the first time, never waited for.
fn panel_picture(
    window: &mut Window,
    scene: &PanelScene,
    key: u64,
    w: f32,
    h: f32,
) -> Option<(std::sync::Arc<gpui::RenderImage>, Instant)> {
    let dpr = window.scale_factor().max(1.0);
    match crate::painter::ready(key) {
        Some(crate::painter::Ready::Image(image, at)) => Some((image, at)),
        Some(crate::painter::Ready::Empty) => None,
        None => {
            let scene = scene.clone();
            crate::painter::want(
                window,
                key,
                crate::painter::synchronous(),
                Box::new(move || panels::paint_panel(&scene, w, h, dpr)),
            );
            match crate::painter::ready(key) {
                Some(crate::painter::Ready::Image(image, at)) => Some((image, at)),
                _ => None,
            }
        }
    }
}

/// What a panel's picture is kept under: the moment, which panel, how it
/// is drawn now (the people's looks change as they age), and its size.
fn panel_key(moment: &str, index: usize, scene: &PanelScene, w: f32, h: f32, dpr: f32) -> u64 {
    let mut key = crate::painter::Key::new("moment-panel");
    key.add(moment).add(index).float(w).float(h).float(dpr);
    for who in &scene.cast {
        key.add(format!("{:?}", who.figure));
    }
    key.add(format!("{:?}", scene.pram))
        .add(scene.season.map(|s| s as u8))
        .add(
            scene
                .place
                .as_ref()
                .and_then(|place| place.wears.as_ref())
                .map(|(wear, motif)| (*wear, motif.key())),
        );
    key.finish()
}

/// A moment's panel, drawn as it arrives: paper until then, then the
/// picture fading in (at once with Reduce Motion).
pub(crate) fn panel_view(
    moment: &str,
    index: usize,
    scene: PanelScene,
    w: f32,
    h: f32,
    label: String,
) -> Stateful<Div> {
    let moment = moment.to_string();
    div()
        .id(SharedString::from(format!("panel-{moment}-{index}")))
        .role(Role::Figure)
        .aria_label(label)
        .w(px(w))
        .h(px(h))
        .flex_shrink_0()
        .rounded(px(3.0))
        .overflow_hidden()
        .border_1()
        .border_color(gpui::rgb(INK_SOFT).opacity(0.35))
        .bg(gpui::rgb(0xe8e0cc))
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, cx| {
                    let dpr = window.scale_factor().max(1.0);
                    let key = panel_key(&moment, index, &scene, w, h, dpr);
                    let Some((image, at)) = panel_picture(window, &scene, key, w, h) else {
                        window.request_animation_frame();
                        return;
                    };
                    let fade = if cx.reduce_motion() || crate::painter::synchronous() {
                        1.0
                    } else {
                        (at.elapsed().as_secs_f32() / 0.3).clamp(0.0, 1.0)
                    };
                    if fade < 1.0 {
                        window.request_animation_frame();
                    }
                    let _ = window.paint_image(
                        bounds,
                        bounds,
                        gpui::Corners::default(),
                        image,
                        0,
                        false,
                    );
                    if fade < 1.0 {
                        window.paint_quad(gpui::fill(
                            bounds,
                            gpui::rgb(0xe8e0cc).opacity(1.0 - fade),
                        ));
                    }
                },
            )
            .size_full(),
        )
}

/// Whether every panel of `moment` has been painted at `layout`'s size,
/// asking for any that has not.
pub(crate) fn panels_painted(
    snapshot: &ProjectionSnapshot,
    moment: &Moment,
    layout: StripLayout,
    window: &mut Window,
) -> bool {
    let dpr = window.scale_factor().max(1.0);
    let (w, h) = (layout.panel_w, layout.panel_h);
    let mut painted = true;
    for (index, scene) in panel_scenes(snapshot, moment).iter().enumerate() {
        let key = panel_key(&moment.id, index, scene, w, h, dpr);
        painted &= panel_picture(window, scene, key, w, h).is_some();
    }
    painted
}

/// A moment as a strip: its title, three panels side by side, and the
/// caption of each beneath it, on paper.
pub(crate) fn moment_strip(
    snapshot: &ProjectionSnapshot,
    moment: &Moment,
    layout: StripLayout,
) -> Stateful<Div> {
    let scenes = panel_scenes(snapshot, moment);
    let mut row = div().flex().gap(px(layout.gap));
    for (index, (panel, scene)) in moment.panels.iter().zip(scenes).enumerate() {
        let label = format!("{}: {}", ui::t(beat_name(panel.beat)), panel.caption);
        row = row.child(
            div()
                .w(px(layout.panel_w))
                .flex()
                .flex_col()
                .gap_2()
                .child(panel_view(
                    &moment.id,
                    index,
                    scene,
                    layout.panel_w,
                    layout.panel_h,
                    label,
                ))
                .child(
                    div()
                        .text_sm()
                        .italic()
                        .line_height(relative(1.4))
                        .text_color(gpui::rgb(INK))
                        .line_clamp(3)
                        .child(panel.caption.clone()),
                ),
        );
    }
    let day = snapshot_day_label(snapshot, moment.day);
    div()
        .id(SharedString::from(format!("moment-{}", moment.id)))
        .role(Role::Group)
        .aria_label(format!("{}. {day}", moment.title))
        .w(px(layout.w))
        .p(px(layout.pad))
        .flex()
        .flex_col()
        .gap_3()
        .rounded(px(4.0))
        .bg(gpui::rgb(PAPER))
        .shadow_lg()
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .gap_4()
                .child(
                    div()
                        .text_xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(gpui::rgb(INK))
                        .line_clamp(1)
                        .child(moment.title.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_sm()
                        .text_color(gpui::rgb(INK_SOFT))
                        .child(day),
                ),
        )
        .child(row)
}

/// "Day 412", in the World's own unit.
pub(crate) fn snapshot_day_label(snapshot: &ProjectionSnapshot, day: u32) -> String {
    let unit = snapshot
        .calendar
        .as_ref()
        .map_or("Day", |calendar| calendar.unit.as_str());
    crate::i18n::day_label(unit, day)
}

/// The day the World is on now.
fn today(snapshot: &ProjectionSnapshot) -> u32 {
    let length = snapshot
        .calendar
        .as_ref()
        .map_or(1, |calendar| calendar.length.max(1));
    world_projection::day_of(snapshot.world_time, length)
}

/// How long a year of the World is, in its days, when it says.
fn year_days(snapshot: &ProjectionSnapshot) -> Option<u64> {
    snapshot
        .calendar
        .as_ref()
        .and_then(|calendar| calendar.year)
}

/// A pill of paper with words on it, for the page's controls.
fn page_button(id: &'static str, label: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(ui::t(label))
        .flex_shrink_0()
        .px_3()
        .py(px(5.0))
        .rounded_full()
        .text_sm()
        .cursor_pointer()
        .text_color(gpui::rgb(INK))
        .border_1()
        .border_color(gpui::rgb(INK_SOFT).opacity(0.45))
        .hover(|style| style.bg(gpui::rgb(0xece3cc)))
        .child(ui::t(label))
}

/// One line of a legend: its day in the margin (only on a day's first
/// line), what happened, and what caused it set softly beneath. `current`
/// is the line the keyboard is on.
pub(crate) fn legend_row(
    index: usize,
    unit: &str,
    day: Option<u32>,
    line: &LegendLine,
    current: bool,
) -> Stateful<Div> {
    div()
        .id(SharedString::from(format!("legend-line-{index}")))
        .role(Role::ListItem)
        .aria_label(line_label(unit, line))
        .aria_selected(current)
        .mx_6()
        .flex()
        .gap_4()
        .py_2()
        .px_2()
        .rounded(px(4.0))
        .when(current, |row| row.bg(gpui::rgb(0xefe6d0)))
        .child(
            div()
                .w(px(72.0))
                .flex_shrink_0()
                .pt(px(3.0))
                .text_xs()
                .text_right()
                .text_color(gpui::rgb(INK_SOFT))
                .child(
                    day.map(|day| crate::i18n::day_label(unit, day))
                        .unwrap_or_default(),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_base()
                        .line_height(relative(1.45))
                        .text_color(gpui::rgb(INK))
                        .child(line.text.clone()),
                )
                .children(line.because.as_ref().map(|because| {
                    div()
                        .text_xs()
                        .italic()
                        .line_height(relative(1.45))
                        .text_color(gpui::rgb(INK_SOFT).opacity(0.9))
                        .child(because.clone())
                })),
        )
}

impl ProjectionView {
    /// Asks the World for a story; `None` when it has none to tell.
    fn tell(&mut self, request: StoryRequest) -> Option<StoryPage> {
        self.controller.as_mut()?.story(request)
    }

    /// Opens a page, keeping the one open now to go back to.
    fn open_page(&mut self, story: Story, cx: &mut Context<Self>) {
        if let Some(open) = self.reading.page.take() {
            self.reading.back.push(open);
        }
        self.reading.page = Some(story);
        self.reading.line = 0;
        self.reading.scroll = ScrollHandle::new();
        self.looking.asking = None;
        self.cue(crate::Cue::Flip);
        cx.notify();
    }

    /// Someone's legend, or a place's or work's, as the World tells it.
    pub(crate) fn open_legend(&mut self, subject: SelectionId, cx: &mut Context<Self>) {
        match self.tell(StoryRequest::Legend(subject)) {
            Some(StoryPage::Legend(legend)) if !legend.lines.is_empty() => {
                self.open_page(Story::Legend(legend), cx)
            }
            _ => {
                self.status = Some(ui::t("No story to tell yet").to_string());
                self.status_is_error = false;
                cx.notify();
            }
        }
    }

    /// A moment the book keeps: from the snapshot if it is one of the
    /// latest, else asked of the World.
    pub(crate) fn open_moment(&mut self, id: &str, cx: &mut Context<Self>) {
        let moment = self
            .snapshot
            .moments
            .iter()
            .find(|moment| moment.id == id)
            .cloned()
            .or_else(|| match self.tell(StoryRequest::Moment(id.to_string())) {
                Some(StoryPage::Moment(moment)) => Some(moment),
                _ => None,
            });
        if let Some(moment) = moment {
            self.open_page(Story::Moment(moment), cx);
        }
    }

    /// A year's almanac, with its best moment.
    pub(crate) fn open_almanac(&mut self, year: u32, cx: &mut Context<Self>) {
        let almanac = self
            .snapshot
            .almanac
            .clone()
            .filter(|almanac| almanac.year == year)
            .or_else(|| match self.tell(StoryRequest::Almanac(year)) {
                Some(StoryPage::Almanac(almanac)) => Some(almanac),
                _ => None,
            });
        let Some(almanac) = almanac else {
            self.status = Some(ui::t("No story to tell yet").to_string());
            self.status_is_error = false;
            cx.notify();
            return;
        };
        let best = almanac.best.as_deref().and_then(|id| {
            self.snapshot
                .moments
                .iter()
                .find(|moment| moment.id == id)
                .cloned()
                .or_else(|| match self.tell(StoryRequest::Moment(id.to_string())) {
                    Some(StoryPage::Moment(moment)) => Some(moment),
                    _ => None,
                })
        });
        self.open_page(Story::Almanac(almanac, best), cx);
    }

    pub(crate) fn close_page(&mut self, cx: &mut Context<Self>) {
        self.reading.page = self.reading.back.pop();
        self.reading.line = 0;
        self.reading.scroll = ScrollHandle::new();
        cx.notify();
    }

    pub(crate) fn close_all_pages(&mut self, cx: &mut Context<Self>) {
        self.reading.page = None;
        self.reading.back.clear();
        cx.notify();
    }

    /// The keys a page answers: up and down along its lines, Escape or
    /// Backspace back to where it was opened from. `true` when the page
    /// took the key.
    pub(crate) fn page_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = event.keystroke.key.as_str();
        if let Some((moment, _)) = &self.reading.shown {
            match key {
                "escape" | "enter" | "space" => {
                    self.dismiss_moment(cx);
                    return true;
                }
                "s" => {
                    let moment = moment.clone();
                    self.save_moment(moment, window, cx);
                    return true;
                }
                _ => return false,
            }
        }
        let Some(page) = &self.reading.page else {
            return false;
        };
        let lines = match page {
            Story::Legend(legend) => legend.lines.len(),
            Story::Almanac(almanac, _) => almanac_sections(almanac).len(),
            Story::Moment(_) => 0,
        };
        match key {
            "escape" | "backspace" => self.close_page(cx),
            "down" | "j" => {
                self.reading.line = (self.reading.line + 1).min(lines.saturating_sub(1));
                self.reading.scroll.scroll_to_item(self.reading.line + 1);
                cx.notify();
            }
            "up" | "k" => {
                self.reading.line = self.reading.line.saturating_sub(1);
                self.reading.scroll.scroll_to_item(self.reading.line + 1);
                cx.notify();
            }
            "home" => {
                self.reading.line = 0;
                self.reading.scroll.scroll_to_item(0);
                cx.notify();
            }
            "end" => {
                self.reading.line = lines.saturating_sub(1);
                self.reading.scroll.scroll_to_item(self.reading.line + 1);
                cx.notify();
            }
            "s" => {
                if let Some(Story::Moment(moment)) = self.reading.page.clone() {
                    self.save_moment(moment, window, cx);
                }
            }
            _ => {}
        }
        true
    }

    /// Whether a moment strip may come up now: not over a return being
    /// told, a place being chosen, the greeting, the end of a chapter,
    /// someone being talked to, or a page being read. A question's card
    /// waits behind it.
    fn moment_may_show(&self) -> bool {
        self.controller.is_some()
            && self.retelling.is_none()
            && !is_beginning(&self.snapshot)
            && self.looking.asking.is_none()
            && self.reading.page.is_none()
            && !self.looking.photographing
            && self.looking.hands.is_none()
            && chapter_just_ended(&self.snapshot, self.looking.chapter_read).is_none()
            && !self
                .looking
                .opening
                .is_some_and(|at| at.elapsed().as_secs_f32() < greeting_seconds(&self.snapshot))
    }

    /// Brings up a moment that has just come, once: the newest one not
    /// seen yet, when nothing else is being shown. The moments a World
    /// already had when its window opened are in the book, not shown.
    pub(crate) fn notice_moments(&mut self) {
        let ids = self
            .snapshot
            .moments
            .iter()
            .map(|moment| moment.id.clone())
            .collect::<BTreeSet<_>>();
        let Some(seen) = self.reading.seen.as_ref() else {
            self.reading.seen = Some(ids);
            return;
        };
        if self.reading.shown.is_some() {
            self.reading.seen.get_or_insert_default().extend(ids);
            return;
        }
        // A moment the film of a return has just told is not told again
        // as a strip.
        let filmed = self
            .snapshot
            .briefing
            .as_ref()
            .filter(|briefing| briefing.returned)
            .map(|briefing| {
                briefing
                    .beats()
                    .iter()
                    .filter_map(|beat| match beat.selection {
                        Some(SelectionId::Event(event)) => Some(event),
                        _ => None,
                    })
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        let fresh = self
            .snapshot
            .moments
            .iter()
            .rev()
            .find(|moment| {
                !seen.contains(&moment.id)
                    && !moment.event.is_some_and(|event| filmed.contains(&event))
            })
            .cloned();
        let Some(fresh) = fresh else {
            return;
        };
        if !self.moment_may_show() {
            return;
        }
        self.reading.seen.get_or_insert_default().extend(ids);
        self.reading.shown = Some((fresh, Instant::now()));
        self.cue(crate::Cue::Keepsake);
    }

    /// Whether a moment strip is up over the scene.
    pub(crate) fn moment_up(&self) -> bool {
        self.reading.shown.is_some()
    }

    pub(crate) fn dismiss_moment(&mut self, cx: &mut Context<Self>) {
        self.reading.shown = None;
        cx.notify();
    }

    /// Saves the strip on screen to Pictures, as a postcard is saved: the
    /// strip's own area of the window, named after the moment.
    fn save_moment(&mut self, moment: Moment, window: &mut Window, cx: &mut Context<Self>) {
        if self.reading.saving {
            return;
        }
        let stem =
            crate::postcard::moment_file_stem(&self.snapshot.title, &moment.title, moment.day);
        let bounds = window.bounds();
        let viewport = window.viewport_size();
        // The window's own title bar, and the World's bar under it.
        let bars = (f32::from(bounds.size.height) - f32::from(viewport.height)).max(0.0)
            + super::world_window::CHROME;
        let (width, height) = (
            f32::from(viewport.width),
            (f32::from(viewport.height) - super::world_window::CHROME).max(1.0),
        );
        let layout = strip_layout(width, height);
        let region = gpui::Bounds::new(
            gpui::point(
                bounds.origin.x + px(layout.x),
                bounds.origin.y + px(bars + layout.y),
            ),
            gpui::size(px(layout.w), px(layout.h - STRIP_FOOT)),
        );
        self.reading.saving = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let saved = cx
                .background_executor()
                .spawn(async move { super::world_window::save_picture(region, &stem, false) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.reading.saving = false;
                if saved {
                    this.reading.saved = Some(Instant::now());
                    this.cue(crate::Cue::Flip);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The strip of a moment just come, over the scene, with a way to put
    /// it away and to keep it as a picture.
    pub(crate) fn render_moment_up(
        &self,
        width: f32,
        height: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let (moment, _) = self.reading.shown.as_ref()?;
        let layout = strip_layout(width, height);
        // The strip comes up once its panels are painted, never as blank
        // paper waiting for its pictures.
        if !panels_painted(&self.snapshot, moment, layout, window) {
            window.request_animation_frame();
            return None;
        }
        let strip = moment_strip(&self.snapshot, moment, layout)
            .role(Role::Dialog)
            .child(self.strip_foot(moment.clone(), cx));
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(
                    div()
                        .id("moment-scrim")
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .bg(gpui::black().opacity(0.18))
                        .on_click(cx.listener(|this, _, _, cx| {
                            cx.stop_propagation();
                            this.dismiss_moment(cx)
                        })),
                )
                .child(
                    div()
                        .id("moment-up")
                        .absolute()
                        .left(px(layout.x))
                        .top(px(layout.y))
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .child(ui::spring_in(
                            strip,
                            format!("moment-up-{}", moment.id),
                            cx.reduce_motion(),
                        )),
                ),
        )
    }

    /// Under a strip: that it is kept in the book, and what can be done.
    fn strip_foot(&self, moment: Moment, cx: &mut Context<Self>) -> Div {
        let saved = self
            .reading
            .saved
            .is_some_and(|at| at.elapsed().as_secs_f32() < 2.5);
        let keep = moment.clone();
        div()
            .pt_1()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .when(!self.reading.saving, |foot| {
                foot.child(
                    div()
                        .text_xs()
                        .text_color(gpui::rgb(INK_SOFT))
                        .child(ui::t(if saved {
                            "Saved to Pictures"
                        } else {
                            "Kept in the book"
                        })),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            page_button("moment-save", "Save as a picture (S)").on_click(
                                cx.listener(move |this, _, window, cx| {
                                    this.save_moment(keep.clone(), window, cx)
                                }),
                            ),
                        )
                        .child(
                            page_button("moment-close", "Close (Esc)").on_click(cx.listener(
                                |this, _, _, cx| {
                                    if this.reading.shown.is_some() {
                                        this.dismiss_moment(cx)
                                    } else {
                                        this.close_page(cx)
                                    }
                                },
                            )),
                        ),
                )
            })
    }

    /// The page open now, over the scene: a legend, an almanac, or a
    /// moment from the book.
    pub(crate) fn render_page(
        &self,
        width: f32,
        height: f32,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let page = self.reading.page.as_ref()?;
        if let Story::Moment(moment) = page {
            let layout = strip_layout(width, height);
            let strip = moment_strip(&self.snapshot, moment, layout)
                .role(Role::Dialog)
                .child(self.strip_foot(moment.clone(), cx));
            return Some(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .bg(gpui::black().opacity(0.18))
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .absolute()
                            .left(px(layout.x))
                            .top(px(layout.y))
                            .child(strip),
                    ),
            );
        }
        let page_w = (width - 48.0).clamp(280.0, 600.0);
        let page_h = (height - 48.0).max(240.0);
        let (label, body) = match page {
            Story::Legend(legend) => (legend.title.clone(), self.legend_body(legend, cx)),
            Story::Almanac(almanac, best) => (
                almanac.title.clone(),
                self.almanac_body(almanac, best.as_ref(), page_w, cx),
            ),
            Story::Moment(_) => unreachable!(),
        };
        let back = !self.reading.back.is_empty();
        let top = div()
            .flex()
            .justify_between()
            .items_center()
            .child(if back {
                page_button("page-back", "Back")
                    .on_click(cx.listener(|this, _, _, cx| this.close_page(cx)))
                    .into_any_element()
            } else {
                div().into_any_element()
            })
            .child(
                page_button("page-close", "Close (Esc)")
                    .on_click(cx.listener(|this, _, _, cx| this.close_all_pages(cx))),
            );
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .bg(gpui::black().opacity(0.22))
                .flex()
                .justify_center()
                .items_center()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .id("story-page")
                        .role(Role::Article)
                        .aria_label(label)
                        .w(px(page_w))
                        .max_h(px(page_h))
                        .rounded(px(4.0))
                        .bg(gpui::rgb(PAPER))
                        .shadow_lg()
                        .relative()
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(div().px_6().pt_4().child(top))
                        .child(
                            div()
                                .id("story-page-body")
                                .flex_shrink(1.0)
                                .min_h(px(0.0))
                                .overflow_y_scroll()
                                .track_scroll(&self.reading.scroll)
                                .role(Role::List)
                                .aria_label(ui::t("Lines of the story"))
                                .children(body),
                        )
                        // A long page fades out at its foot rather than
                        // stopping at a hard edge.
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .bottom_0()
                                .h(px(48.0))
                                .bg(gpui::linear_gradient(
                                    180.0,
                                    gpui::linear_color_stop(gpui::rgb(PAPER).opacity(0.0), 0.0),
                                    gpui::linear_color_stop(gpui::rgb(PAPER), 0.9),
                                )),
                        ),
                ),
        )
    }

    /// A legend's page: the subject's portrait at their age now, their
    /// name, how long they have been here, and their life line by line,
    /// each day once, with what caused each line set softly beneath it.
    fn legend_body(&self, legend: &Legend, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let snapshot = &self.snapshot;
        let item = snapshot
            .canvas
            .items
            .iter()
            .find(|item| item.id == legend.subject);
        let unit = snapshot
            .calendar
            .as_ref()
            .map_or("Day", |calendar| calendar.unit.as_str());
        let first = legend.lines.first().map_or(1, |line| line.day);
        let last = legend.lines.last().map_or(first, |line| line.day);
        let here = item.is_some();
        let span = if here {
            format!(
                "{} · {}",
                crate::i18n::here_since(unit, first),
                ui::t(time_here(
                    first,
                    today(snapshot),
                    year_days(snapshot),
                    snapshot
                        .calendar
                        .as_ref()
                        .is_some_and(|calendar| calendar.season.is_some())
                ))
            )
        } else {
            format!(
                "{} – {}",
                crate::i18n::day_label(unit, first),
                crate::i18n::day_label(unit, last)
            )
        };
        let face = match item {
            Some(item) if item.kind != CanvasItemKind::Actor => {
                let drawing = snapshot.drawing_of(item).cloned();
                let palette = crate::art::Palette::of(&item.id.stable_key(), false);
                let shape = item.shape.unwrap_or_default();
                div()
                    .size(px(96.0))
                    .rounded(px(12.0))
                    .bg(gpui::rgb(0xe9dfc8))
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, window, _| {
                                let x = f32::from(bounds.origin.x) + 48.0;
                                let base = f32::from(bounds.origin.y) + 88.0;
                                match &drawing {
                                    Some(drawing) => {
                                        let h = (72.0_f32).min(76.0 / drawing.aspect.max(0.3));
                                        crate::art::paint_drawing(
                                            window,
                                            x,
                                            base,
                                            h * drawing.aspect,
                                            h,
                                            drawing,
                                            &crate::art::Inks::of_place(&palette),
                                            world_projection::Stance::Standing,
                                            world_projection::Mood::Content,
                                            0.0,
                                            0.0,
                                            1.0,
                                        );
                                    }
                                    None => crate::art::paint_building(
                                        window, x, base, 64.0, 56.0, shape, &palette,
                                    ),
                                }
                            },
                        )
                        .size_full(),
                    )
                    .into_any_element()
            }
            Some(item) => portrait(likeness_of(snapshot, item.id), 96.0, false).into_any_element(),
            None => portrait(
                Likeness {
                    figure: crate::art::Figure::of(&legend.subject.stable_key(), None),
                    drawing: None,
                    mood: Default::default(),
                },
                96.0,
                false,
            )
            .into_any_element(),
        };
        let header = div()
            .id("legend-portrait")
            .role(Role::Image)
            .aria_label(legend.title.clone())
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .pb_6()
            .child(face)
            .child(
                div()
                    .id("legend-name")
                    .role(Role::Heading)
                    .text_2xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_center()
                    .text_color(gpui::rgb(INK))
                    .child(legend.title.clone()),
            )
            .child(div().text_sm().text_color(gpui::rgb(INK_SOFT)).child(span));
        let mut lines = vec![header.px_8().pt_2().into_any_element()];
        let mut index = 0;
        for (day, day_lines) in legend_days(&legend.lines) {
            for (nth, line) in day_lines.into_iter().enumerate() {
                let current = index == self.reading.line;
                let row = index;
                lines.push(
                    legend_row(index, unit, (nth == 0).then_some(day), line, current)
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.reading.line = row;
                            cx.notify();
                        }))
                        .into_any_element(),
                );
                index += 1;
            }
        }
        lines.push(div().h(px(56.0)).into_any_element());
        lines
    }

    /// An almanac's page: the year's title, who came and left, who was
    /// born and died, what was built, and the year's best moment in its
    /// panels.
    fn almanac_body(
        &self,
        almanac: &Almanac,
        best: Option<&Moment>,
        page_w: f32,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        let mut body = div().px_8().pt_2().pb_6().flex().flex_col().gap_6().child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(gpui::rgb(INK_SOFT))
                        .child(ui::t(format!("The almanac · Year {}", almanac.year))),
                )
                .child(
                    div()
                        .id("almanac-title")
                        .role(Role::Heading)
                        .text_2xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_center()
                        .text_color(gpui::rgb(INK))
                        .child(almanac.title.clone()),
                ),
        );
        let sections = almanac_sections(almanac);
        if sections.is_empty() {
            body = body.child(
                div()
                    .text_center()
                    .italic()
                    .text_color(gpui::rgb(INK_SOFT))
                    .child(ui::t("A quiet year.")),
            );
        }
        let mut list = vec![body.into_any_element()];
        for (index, (heading, names)) in sections.into_iter().enumerate() {
            let current = index == self.reading.line;
            let row = index;
            let heading = ui::t(heading).to_string();
            list.push(
                div()
                    .id(SharedString::from(format!("almanac-{index}")))
                    .mx_6()
                    .my_1()
                    .role(Role::ListItem)
                    .aria_label(format!("{heading}: {}", names.join(", ")))
                    .aria_selected(current)
                    .flex()
                    .gap_4()
                    .px_2()
                    .py_1()
                    .rounded(px(4.0))
                    .when(current, |row| row.bg(gpui::rgb(0xefe6d0)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.reading.line = row;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(120.0))
                            .flex_shrink_0()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(gpui::rgb(INK_SOFT))
                            .child(heading),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .text_base()
                            .line_height(relative(1.5))
                            .text_color(gpui::rgb(INK))
                            .child(names.join(" · ")),
                    )
                    .into_any_element(),
            );
        }
        if let Some(best) = best {
            let inner = page_w - 64.0;
            let panel_w = ((inner - 24.0) / 3.0).floor();
            let panel_h = (panel_w * 0.75).round();
            let scenes = panel_scenes(&self.snapshot, best);
            let mut row = div().flex().gap(px(12.0));
            for (index, (panel, scene)) in best.panels.iter().zip(scenes).enumerate() {
                row = row.child(
                    div()
                        .w(px(panel_w))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(panel_view(
                            &best.id,
                            index,
                            scene,
                            panel_w,
                            panel_h,
                            format!("{}: {}", ui::t(beat_name(panel.beat)), panel.caption),
                        ))
                        .child(
                            div()
                                .text_xs()
                                .italic()
                                .line_clamp(3)
                                .text_color(gpui::rgb(INK))
                                .child(panel.caption.clone()),
                        ),
                );
            }
            let id = best.id.clone();
            list.push(
                div()
                    .id("almanac-best")
                    .mx_8()
                    .mt_6()
                    .role(Role::Group)
                    .aria_label(format!("{}: {}", ui::t("The year's moment"), best.title))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| this.open_moment(&id, cx)))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(gpui::rgb(INK_SOFT))
                            .child(format!("{} · {}", ui::t("The year's moment"), best.title)),
                    )
                    .child(row)
                    .into_any_element(),
            );
        }
        list.push(div().h(px(40.0)).into_any_element());
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::accessible;

    struct Told(ProjectionSnapshot);

    impl crate::ProjectionController for Told {
        fn snapshot(&self) -> ProjectionSnapshot {
            self.0.clone()
        }

        fn handle(&mut self, _: ProjectionIntent) -> Result<ProjectionSnapshot, String> {
            Ok(self.0.clone())
        }
    }

    fn moment(id: &str) -> Moment {
        Moment {
            id: id.into(),
            day: 3,
            kind: MomentKind::Wedding,
            title: "A wedding".into(),
            ..Default::default()
        }
    }

    /// A moment already in the World as its window opens is in the book,
    /// not brought up; one that comes later is brought up once, but not
    /// while the World is still greeting the player, and never twice.
    #[test]
    fn a_new_moment_comes_up_once_and_never_over_the_greeting() {
        let mut snapshot = ProjectionSnapshot {
            moments: vec![moment("old")],
            ..Default::default()
        };
        snapshot
            .timeline
            .items
            .push(world_projection::TimelineItem {
                id: SelectionId::from_stable_key("event-1").unwrap(),
                world_time: 0,
                title: "The boats came in".into(),
                subtitle: String::new(),
                caused_by: Vec::new(),
                routine: false,
            });
        snapshot.voices.push(world_projection::Voice {
            moment: SelectionId::from_stable_key("event-1").unwrap(),
            speaker: SelectionId::from_stable_key("entity-1").unwrap(),
            line: "Morning! The boats are in.".into(),
        });
        let mut view = ProjectionView::controlled(Told(snapshot.clone()));
        view.notice_moments();
        assert!(!view.moment_up(), "what was there already is not shown");
        snapshot.moments.push(moment("new"));
        view.snapshot = snapshot;
        // The greeting is still being said.
        view.looking.opening = Some(Instant::now());
        view.notice_moments();
        assert!(!view.moment_up(), "never over the greeting");
        view.looking.opening = None;
        view.notice_moments();
        assert_eq!(
            view.reading
                .shown
                .as_ref()
                .map(|(moment, _)| moment.id.as_str()),
            Some("new")
        );
        view.reading.shown = None;
        view.notice_moments();
        assert!(!view.moment_up(), "shown once");
    }

    fn line(day: u32, text: &str, because: Option<&str>) -> LegendLine {
        LegendLine {
            day,
            text: text.into(),
            because: because.map(str::to_string),
            ..Default::default()
        }
    }

    #[test]
    fn a_legend_reads_by_day_each_day_once() {
        let lines = [
            line(3, "Came to the harbour", Some("after the storm")),
            line(3, "Took a room over the bakery", None),
            line(40, "Married Leo", Some("because you said “Throw a party”")),
        ];
        let days = legend_days(&lines);
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].0, 3);
        assert_eq!(days[0].1.len(), 2);
        assert_eq!(days[1].1[0].text, "Married Leo");
        assert_eq!(
            line_label("Day", &lines[0]),
            "Day 3: Came to the harbour (after the storm)"
        );
    }

    #[test]
    fn time_in_the_town_is_told_in_years_when_the_world_counts_them() {
        assert_eq!(time_here(1, 1, Some(120), true), "New to the town");
        assert_eq!(time_here(1, 40, Some(120), true), "A season in the town");
        assert_eq!(time_here(1, 70, Some(120), true), "Two seasons in the town");
        assert_eq!(
            time_here(1, 100, Some(120), true),
            "Three seasons in the town"
        );
        assert_eq!(time_here(1, 20, Some(120), false), "2 months in the town");
        assert_eq!(time_here(1, 10, Some(120), false), "A month in the town");
        assert_eq!(time_here(1, 130, Some(120), true), "A year in the town");
        assert_eq!(time_here(1, 360, Some(120), true), "3 years in the town");
        assert_eq!(time_here(1, 1080, None, true), "1080 days in the town");
    }

    #[test]
    fn an_almanac_shows_only_what_the_year_had() {
        let almanac = Almanac {
            year: 2,
            title: "The harbour's second year".into(),
            arrived: vec!["Ada".into()],
            born: vec!["Pip".into(), "Wren".into()],
            built: vec!["The bandstand".into()],
            ..Default::default()
        };
        let sections = almanac_sections(&almanac);
        let headings = sections
            .iter()
            .map(|(heading, _)| *heading)
            .collect::<Vec<_>>();
        assert_eq!(headings, ["Came to live here", "Born", "Built"]);
        assert!(almanac_sections(&Almanac::default()).is_empty());
    }

    /// The strip fits the stage at any usual size, its three panels four
    /// by three and never too small to see.
    #[test]
    fn a_moment_strip_fits_the_stage() {
        for (width, height) in [
            (1440.0, 848.0),
            (1100.0, 700.0),
            (800.0, 560.0),
            (520.0, 420.0),
        ] {
            let layout = strip_layout(width, height);
            assert!(
                layout.x >= 0.0 && layout.x + layout.w <= width + 0.5,
                "{width}: {layout:?}"
            );
            assert!(
                layout.y >= 0.0 && layout.y + layout.h <= height + 0.5,
                "{height}: {layout:?}"
            );
            assert!(layout.panel_w >= 96.0 && layout.panel_w <= 300.0);
            assert!((layout.panel_h / layout.panel_w - 0.75).abs() < 0.01);
            let inner = 3.0 * layout.panel_w + 2.0 * layout.gap + 2.0 * layout.pad;
            assert!((inner - layout.w).abs() < 0.5);
        }
    }

    #[test]
    fn stories_are_named_for_a_screen_reader() {
        let (role, node) = accessible(&legend_row(
            0,
            "Day",
            Some(40),
            &line(40, "Married Leo", Some("after the harvest")),
            true,
        ));
        assert_eq!(role, Some(Role::ListItem));
        assert_eq!(
            node.label(),
            Some("Day 40: Married Leo (after the harvest)")
        );
        assert_eq!(node.is_selected(), Some(true));
        let snapshot = ProjectionSnapshot::default();
        let moment = Moment {
            id: "wedding-40".into(),
            day: 40,
            kind: MomentKind::Wedding,
            title: "Mara and Leo marry".into(),
            ..Default::default()
        };
        let scenes = panel_scenes(&snapshot, &moment);
        assert_eq!(scenes.len(), 3);
        let (role, node) = accessible(&panel_view(
            "wedding-40",
            1,
            scenes[1].clone(),
            120.0,
            90.0,
            "The moment: They said yes".into(),
        ));
        assert_eq!(role, Some(Role::Figure));
        assert_eq!(node.label(), Some("The moment: They said yes"));
        let (role, node) = accessible(&moment_strip(
            &snapshot,
            &moment,
            strip_layout(900.0, 600.0),
        ));
        assert_eq!(role, Some(Role::Group));
        assert_eq!(node.label(), Some("Mara and Leo marry. Day 40"));
        let (role, node) = accessible(&page_button("page-close", "Close (Esc)"));
        assert_eq!(role, Some(Role::Button));
        assert_eq!(node.label(), Some("Close (Esc)"));
    }

    /// Every word the stories show is in each of the app's catalogs.
    #[test]
    fn every_story_word_is_translated() {
        for catalog in crate::i18n::app_catalogs() {
            for words in [
                "Their story",
                "Its story",
                "No story to tell yet",
                "Here since",
                "Before",
                "The moment",
                "After",
                "Kept in the book",
                "Saved to Pictures",
                "Save as a picture (S)",
                "Close (Esc)",
                "Back",
                "Lines of the story",
                "The year's moment",
                "A quiet year.",
                "Who came and went, and what was built",
                "Came to live here",
                "Moved away",
                "Born",
                "Died",
                "Built",
                "A year in the town",
                "A day in the town",
                "New to the town",
                "A season in the town",
                "Two seasons in the town",
                "Three seasons in the town",
                "A month in the town",
            ] {
                assert!(
                    catalog.exact(words).is_some(),
                    "no translation for {words:?}"
                );
            }
            for beat in PanelBeat::ALL {
                assert!(catalog.exact(beat_name(beat)).is_some());
            }
            for (heading, _) in almanac_sections(&Almanac {
                arrived: vec!["a".into()],
                left: vec!["a".into()],
                born: vec!["a".into()],
                died: vec!["a".into()],
                built: vec!["a".into()],
                ..Default::default()
            }) {
                assert!(catalog.exact(heading).is_some(), "{heading}");
            }
        }
        let [chinese, japanese] = crate::i18n::app_catalogs();
        for (english, zh, ja) in [
            ("3 years in the town", "在镇上 3 年", "町で過ごした3年"),
            ("12 days in the town", "在镇上 12 天", "町で過ごした12日"),
            ("5 months in the town", "在镇上 5 个月", "町で過ごした5か月"),
            ("The almanac · Year 2", "年鉴 · 第 2 年", "年鑑 · 2年目"),
        ] {
            assert_eq!(chinese.translate(english).as_deref(), Some(zh));
            assert_eq!(japanese.translate(english).as_deref(), Some(ja));
        }
    }
}
