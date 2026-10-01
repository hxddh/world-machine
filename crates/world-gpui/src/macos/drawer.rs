//! The drawer, kept as a book: a few leaves to turn between rather than
//! one long list. The story (the almanac of every year gone, and the
//! chapters by year, each with a short summary), what the World has
//! built (by season, told apart by kind), what the player keeps (letters,
//! and keepsakes counted once), the book of everything to find, and the
//! rest. Presentation only: everything here is read from the snapshot.

use super::world_window::{
    arrow_button, capitalized, coming_label, first_name, label_of, likeness_of, list_entry,
    portrait, Likeness, DRAWER_WIDTH, LETTERS_SHOWN,
};
use super::*;
use crate::art;
use crate::postcard::{INK, INK_SOFT, PAPER};
use gpui::{canvas, Hsla, Role, Stateful};
use world_projection::{BookEntry, Goal, MarkShape};

/// A leaf of the drawer's book.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Leaf {
    /// The almanacs and the chapters.
    #[default]
    Story,
    /// What the World has built and is building.
    Built,
    /// Letters and keepsakes.
    Kept,
    /// Everything there is to find.
    Found,
    /// A closer look, how things stand, who is here, and the history.
    More,
}

impl Leaf {
    pub(crate) const ALL: [Leaf; 5] = [
        Leaf::Story,
        Leaf::Built,
        Leaf::Kept,
        Leaf::Found,
        Leaf::More,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Leaf::Story => "The story",
            Leaf::Built => "Built",
            Leaf::Kept => "Kept",
            Leaf::Found => "The book",
            Leaf::More => "More",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Leaf::Story => "story",
            Leaf::Built => "built",
            Leaf::Kept => "kept",
            Leaf::Found => "found",
            Leaf::More => "more",
        }
    }
}

/// The drawer's paper and inks, in the current appearance.
fn ink(hex: u32) -> gpui::Rgba {
    gpui::rgb(world_theme::adapt(hex))
}

const RULE: u32 = 0xe2d5ba;

/// What a favour's note says under it once it is done.
pub(crate) const FAVOUR_DONE: &str = "Done, and thanked.";
const TINT: u32 = 0xede2c9;

/// The kinds of thing a World builds, as the drawer groups them, by the
/// shape each is drawn in.
pub(crate) const KINDS: [&str; 6] = [
    "Buildings",
    "By the water",
    "Lights",
    "Gardens",
    "On the square",
    "Small things",
];

pub(crate) fn kind_of(shape: MarkShape) -> usize {
    use MarkShape::*;
    match shape {
        House | Dome | Tower | Shop | Stall | Tent => 0,
        Pier | Bridge | Boat => 1,
        Lamp | Lantern => 2,
        Tree | Garden | Planter | Sprouts => 3,
        Fountain | Statue | Well | Bench | Swing | Bunting | Flag => 4,
        Signpost | Birdhouse | Postbox | Parcel | Rover => 5,
    }
}

const SEASONS: [&str; 4] = ["Spring", "Summer", "Autumn", "Winter"];

/// The day of the World now.
fn today(snapshot: &ProjectionSnapshot) -> u32 {
    let length = snapshot
        .calendar
        .as_ref()
        .map_or(1, |calendar| calendar.length.max(1));
    world_projection::day_of(snapshot.world_time, length)
}

fn year_length(snapshot: &ProjectionSnapshot) -> Option<u32> {
    snapshot
        .calendar
        .as_ref()
        .and_then(|calendar| calendar.year)
        .filter(|year| *year > 0)
        .map(|year| year as u32)
}

/// Which of the World's years a day falls in, counted from 1, when it
/// counts years.
pub(crate) fn year_of(snapshot: &ProjectionSnapshot, day: u32) -> Option<u32> {
    year_length(snapshot).map(|year| day.saturating_sub(1) / year + 1)
}

/// The season a day falls in, by name, when the World names its seasons
/// the four usual ways: worked back from the season it says it is today.
pub(crate) fn season_of(snapshot: &ProjectionSnapshot, day: u32) -> Option<&'static str> {
    let year = year_length(snapshot)?;
    let now = snapshot.calendar.as_ref()?.season.as_deref()?;
    let named = SEASONS.iter().position(|season| *season == now)?;
    let quarter = |day: u32| (day.saturating_sub(1) % year) * 4 / year;
    let back = (quarter(today(snapshot)) + 4 - quarter(day)) % 4;
    Some(SEASONS[(named + 4 - back as usize) % 4])
}

/// Every year whose almanac can be asked for now, oldest first: the
/// World's own list, or, from a Pack that sends none, every year that has
/// ended by the World's calendar.
pub(crate) fn almanac_years(snapshot: &ProjectionSnapshot) -> Vec<u32> {
    let mut years = snapshot.almanac_years.clone();
    if years.is_empty() {
        if let Some(year) = year_length(snapshot) {
            years = (1..=today(snapshot).saturating_sub(1) / year).collect();
        }
    }
    if let Some(almanac) = &snapshot.almanac {
        if !years.contains(&almanac.year) {
            years.push(almanac.year);
        }
    }
    years.sort_unstable();
    years.dedup();
    years
}

/// A summary cut to a sentence or two, for a list of chapters.
pub(crate) fn short_summary(text: &str) -> String {
    const MOST: usize = 110;
    let mut out = String::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        let end = rest
            .char_indices()
            .find(|(_, c)| matches!(c, '.' | '!' | '?' | '。' | '！' | '？'))
            .map_or(rest.len(), |(at, c)| at + c.len_utf8());
        let sentence = &rest[..end];
        if !out.is_empty() && out.chars().count() + sentence.chars().count() > MOST {
            out.push('…');
            return out;
        }
        out.push_str(sentence);
        rest = &rest[end..];
        if out.chars().count() >= MOST / 2 && !rest.trim().is_empty() {
            out.push_str(if rest.starts_with(' ') { " …" } else { "…" });
            return out;
        }
    }
    out
}

/// A keepsake, counted once however many times it was given.
pub(crate) struct Kept<'a> {
    pub(crate) what: &'a str,
    pub(crate) note: &'a str,
    pub(crate) from: Vec<String>,
    pub(crate) times: usize,
}

/// The keepsakes, each thing once, newest first: how many times it came,
/// from whom, and the newest word given with it.
pub(crate) fn kept_once(snapshot: &ProjectionSnapshot) -> Vec<Kept<'_>> {
    let mut kept: Vec<Kept> = Vec::new();
    for keepsake in snapshot.keepsakes.iter().rev() {
        let from = label_of(snapshot, keepsake.from).map(|name| first_name(&name));
        let key = keepsake.what.trim().to_lowercase();
        match kept
            .iter_mut()
            .find(|seen| seen.what.trim().to_lowercase() == key)
        {
            Some(seen) => {
                seen.times += 1;
                if let Some(from) = from.filter(|from| !seen.from.contains(from)) {
                    seen.from.push(from);
                }
            }
            None => kept.push(Kept {
                what: &keepsake.what,
                note: &keepsake.note,
                from: from.into_iter().collect(),
                times: 1,
            }),
        }
    }
    kept
}

/// "Mara", "Mara and Leo", "Mara, Leo and 2 others".
fn names_list(names: &[String]) -> String {
    match names {
        [] => ui::t("a friend").to_string(),
        [one] => one.clone(),
        [one, two] => crate::i18n::two_names(one, two),
        [one, two, rest @ ..] => crate::i18n::names_and_others(one, two, rest.len()),
    }
}

/// A work the World has finished, and when.
pub(crate) struct Finished<'a> {
    pub(crate) goal: &'a Goal,
    pub(crate) day: Option<u32>,
}

/// What the World has finished, with the day each was, newest first.
pub(crate) fn finished(snapshot: &ProjectionSnapshot) -> Vec<Finished<'_>> {
    let mut done = snapshot
        .goals
        .iter()
        .filter(|goal| goal.finished())
        .map(|goal| {
            let name = goal.label.trim().to_lowercase();
            let day = snapshot
                .canvas
                .items
                .iter()
                .find(|item| item.label.trim().to_lowercase() == name)
                .and_then(|item| item.built);
            Finished { goal, day }
        })
        .collect::<Vec<_>>();
    done.sort_by_key(|done| std::cmp::Reverse(done.day.unwrap_or(0)));
    done
}

/// Things finished, grouped by the season (or year) they were finished
/// in, newest first; each group sorted by kind.
pub(crate) fn by_season<'a>(
    snapshot: &ProjectionSnapshot,
    done: Vec<Finished<'a>>,
) -> Vec<(Option<u32>, Option<&'static str>, Vec<Finished<'a>>)> {
    let mut groups: Vec<(Option<u32>, Option<&'static str>, Vec<Finished>)> = Vec::new();
    for work in done {
        let year = work.day.and_then(|day| year_of(snapshot, day));
        let season = work.day.and_then(|day| season_of(snapshot, day));
        match groups.last_mut() {
            Some((y, s, list)) if *y == year && *s == season => list.push(work),
            _ => groups.push((year, season, vec![work])),
        }
    }
    for (_, _, list) in &mut groups {
        list.sort_by_key(|work| (kind_of(work.goal.shape), work.goal.label.clone()));
    }
    groups
}

impl ProjectionView {
    fn leaf_has_content(&self, leaf: Leaf) -> bool {
        let snapshot = &self.snapshot;
        match leaf {
            Leaf::Story => !snapshot.chapters.is_empty() || !almanac_years(snapshot).is_empty(),
            Leaf::Built => !snapshot.goals.is_empty(),
            Leaf::Kept => !snapshot.letters.is_empty() || !snapshot.keepsakes.is_empty(),
            Leaf::Found => !snapshot.book.is_empty(),
            Leaf::More => true,
        }
    }

    /// The leaves there is something on, in order.
    pub(crate) fn leaves(&self) -> Vec<Leaf> {
        Leaf::ALL
            .into_iter()
            .filter(|leaf| self.leaf_has_content(*leaf))
            .collect()
    }

    /// The leaf the drawer is open at: the one chosen, if it has anything.
    pub(crate) fn open_leaf(&self) -> Leaf {
        let leaves = self.leaves();
        if leaves.contains(&self.looking.leaf) {
            self.looking.leaf
        } else {
            leaves.first().copied().unwrap_or_default()
        }
    }

    pub(crate) fn turn_to(&mut self, leaf: Leaf, cx: &mut Context<Self>) {
        self.looking.leaf = leaf;
        self.looking.drawer = true;
        cx.notify();
    }

    /// Whether a year's part of a leaf is open: the latest is unless
    /// folded, an older one only once opened.
    fn year_open(&self, year: Option<u32>, latest: Option<u32>) -> bool {
        let Some(year) = year else {
            return true;
        };
        let toggled = self.looking.years.contains(&year);
        (Some(year) == latest) != toggled
    }

    /// The years of the almanac, as a shelf of little books: the one a
    /// new year has just brought marked new.
    pub(crate) fn render_almanacs(&self, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let years = almanac_years(&self.snapshot);
        if years.is_empty() {
            return None;
        }
        let heading = ui::t("The almanac").to_string();
        let fresh = self.snapshot.almanac.as_ref().map(|almanac| almanac.year);
        let mut shelf = div().flex().flex_wrap().gap_2();
        for year in years.iter().rev().copied() {
            let title = self
                .snapshot
                .almanac
                .as_ref()
                .filter(|almanac| almanac.year == year)
                .map(|almanac| almanac.title.clone());
            let label = crate::i18n::year_label(year);
            shelf = shelf.child(
                div()
                    .id(SharedString::from(format!("almanac-{year}")))
                    .role(Role::ListItem)
                    .aria_label(match &title {
                        Some(title) => format!("{heading} · {label}: {title}"),
                        None => format!("{heading} · {label}"),
                    })
                    .w(px(92.0))
                    .h(px(64.0))
                    .px_2()
                    .py_2()
                    .rounded(px(3.0))
                    .bg(ink(0xf9f4e8))
                    .border_1()
                    .border_color(ink(RULE))
                    .border_l_4()
                    .shadow_sm()
                    .cursor_pointer()
                    .hover(|style| style.bg(ink(TINT)))
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .text_color(ink(INK_SOFT))
                            .child(ui::t("Almanac")),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ink(INK))
                            .child(label),
                    )
                    .when(fresh == Some(year), |book| {
                        book.child(
                            div()
                                .text_xs()
                                .text_color(color(tokens::ACCENT_TEXT))
                                .child(ui::t("New")),
                        )
                    })
                    .on_click(cx.listener(move |this, _, _, cx| this.open_almanac(year, cx))),
            );
        }
        Some(
            ui::region("drawer-almanac", Role::List, heading.clone())
                .flex()
                .flex_col()
                .gap_2()
                .child(leaf_heading(heading))
                .child(div().px_3().child(shelf))
                .child(
                    div()
                        .px_3()
                        .text_xs()
                        .text_color(ink(INK_SOFT))
                        .child(ui::t("Each year, who came and went and what was built (Y)")),
                ),
        )
    }

    /// The story so far, chapter by chapter, by year: this year open,
    /// the years before folded to a line each.
    pub(crate) fn render_chapters(&self, cx: Option<&mut Context<Self>>) -> Option<Stateful<Div>> {
        if self.snapshot.chapters.is_empty() {
            return None;
        }
        let length = self
            .snapshot
            .calendar
            .as_ref()
            .map_or(1, |calendar| calendar.length.max(1));
        let when = |chapter: &world_projection::Chapter| {
            let moment = chapter.moment?;
            let item = self
                .snapshot
                .timeline
                .items
                .iter()
                .find(|item| item.id == moment)?;
            year_of(
                &self.snapshot,
                world_projection::day_of(item.world_time, length),
            )
        };
        // Chapters in years, newest first. A chapter whose moment has left
        // the timeline takes the year of the one before it; the first few,
        // before any can be dated, are kept together as the earlier ones.
        let mut known = self.snapshot.chapters.iter().map(when).collect::<Vec<_>>();
        for at in 1..known.len() {
            if known[at].is_none() {
                known[at] = known[at - 1];
            }
        }
        let mut years: Vec<(Option<u32>, Vec<&world_projection::Chapter>)> = Vec::new();
        for (chapter, year) in self.snapshot.chapters.iter().zip(known) {
            match years.iter_mut().find(|(y, _)| *y == year) {
                Some((_, list)) => list.push(chapter),
                None => years.push((year, vec![chapter])),
            }
        }
        years.sort_by_key(|(year, _)| *year);
        years.reverse();
        let latest = years.first().and_then(|(year, _)| *year);
        let heading = format!("{} · {}", ui::t("Chapters"), self.snapshot.chapters.len());
        let mut story = ui::region("drawer-story", Role::Group, "The story so far")
            .flex()
            .flex_col()
            .gap_3()
            .child(leaf_heading(heading));
        let weak = cx.map(|cx| cx.entity().downgrade());
        for (year, chapters) in years {
            let key = year.unwrap_or(0);
            let open = match year {
                Some(_) => self.year_open(year, latest),
                None if latest.is_none() => true,
                None => self.looking.years.contains(&0),
            };
            let name = year
                .map(crate::i18n::year_label)
                .unwrap_or_else(|| ui::t("Earlier").to_string());
            let mut list = ui::region(
                SharedString::from(format!("chapters-{key}")),
                Role::List,
                name.clone(),
            )
            .flex()
            .flex_col()
            .gap_1();
            if year.is_some() || latest.is_some() {
                let count = crate::i18n::chapters_count(chapters.len());
                let mut fold = div()
                    .id(SharedString::from(format!("year-{key}")))
                    .role(Role::Button)
                    .aria_expanded(open)
                    .aria_label(format!("{name} · {count}"))
                    .mx_3()
                    .py_1()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(ink(RULE))
                    .cursor_pointer()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ink(INK))
                            .child(name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(ink(INK_SOFT))
                            .child(format!("{count}  {}", if open { "▾" } else { "▸" })),
                    );
                if let Some(weak) = weak.clone() {
                    fold = fold.on_click(move |_, _, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            if !this.looking.years.remove(&key) {
                                this.looking.years.insert(key);
                            }
                            cx.notify();
                        });
                    });
                }
                list = list.child(fold);
            }
            if open {
                for chapter in chapters.into_iter().rev() {
                    list = list.child(chapter_entry(chapter));
                }
            }
            story = story.child(list);
        }
        Some(story)
    }

    /// What the World is building now, and what it has built, by season,
    /// with a count of each kind.
    pub(crate) fn render_builds(&self, cx: Option<&mut Context<Self>>) -> Option<Stateful<Div>> {
        if self.snapshot.goals.is_empty() {
            return None;
        }
        let snapshot = &self.snapshot;
        let mut section = ui::region("drawer-goals", Role::Group, "Building")
            .flex()
            .flex_col()
            .gap_4();
        let under_way = snapshot
            .goals
            .iter()
            .filter(|goal| !goal.finished())
            .collect::<Vec<_>>();
        if !under_way.is_empty() {
            let mut list = ui::region("goals-under-way", Role::List, "Under way")
                .flex()
                .flex_col()
                .gap_1()
                .child(leaf_heading(ui::t("Under way").to_string()));
            for goal in under_way {
                let mut pips = div().flex().gap_1();
                for part in 0..goal.parts {
                    pips = pips.child(div().size(px(7.0)).rounded_full().bg(if part < goal.done {
                        color(tokens::ACCENT)
                    } else {
                        ink(RULE)
                    }));
                }
                let spoken = crate::i18n::parts_done(goal.done, goal.parts);
                list = list.child(
                    list_entry(
                        SharedString::from(format!("goal-{}", goal.id)),
                        format!("{}: {spoken}", goal.label),
                    )
                    .mx_3()
                    .py_1()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(work_icon(snapshot, goal).w(px(30.0)).h(px(24.0)))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .text_sm()
                            .text_color(ink(INK))
                            .child(goal.label.clone()),
                    )
                    .child(pips),
                );
            }
            section = section.child(list);
        }
        let done = finished(snapshot);
        if done.is_empty() {
            return Some(section);
        }
        // The kinds, each with how many: a way to see only one.
        let mut counts = [0usize; KINDS.len()];
        for work in &done {
            counts[kind_of(work.goal.shape)] += 1;
        }
        let chosen = self.looking.kind.filter(|kind| counts[*kind] > 0);
        let weak = cx.map(|cx| cx.entity().downgrade());
        let mut kinds = div()
            .id("built-kinds")
            .role(Role::TabList)
            .aria_label(ui::t("Kinds"))
            .mx_3()
            .flex()
            .flex_wrap()
            .gap_1();
        let all = crate::i18n::all_count(done.len());
        for (index, label) in std::iter::once((None, all)).chain(
            KINDS
                .iter()
                .enumerate()
                .filter(|(kind, _)| counts[*kind] > 0)
                .map(|(kind, name)| (Some(kind), format!("{} {}", ui::t(*name), counts[kind]))),
        ) {
            let on = chosen == index;
            let mut chip = div()
                .id(SharedString::from(format!("kind-{}", index.unwrap_or(9))))
                .role(Role::Tab)
                .aria_selected(on)
                .aria_label(label.clone())
                .px_2()
                .py(px(2.0))
                .rounded_full()
                .text_xs()
                .cursor_pointer()
                .border_1()
                .border_color(ink(RULE))
                .when(on, |chip| chip.bg(ink(INK)).text_color(ink(PAPER)))
                .when(!on, |chip| {
                    chip.text_color(ink(INK_SOFT))
                        .hover(|style| style.bg(ink(TINT)))
                })
                .child(label);
            if let Some(weak) = weak.clone() {
                chip = chip.on_click(move |_, _, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        this.looking.kind = index;
                        cx.notify();
                    });
                });
            }
            kinds = kinds.child(chip);
        }
        let heading = crate::i18n::built_count(done.len());
        let mut built = ui::region("goals-built", Role::Group, heading.clone())
            .flex()
            .flex_col()
            .gap_3()
            .child(leaf_heading(heading))
            .child(kinds);
        let done = done
            .into_iter()
            .filter(|work| chosen.is_none_or(|kind| kind_of(work.goal.shape) == kind))
            .collect::<Vec<_>>();
        let groups = by_season(snapshot, done);
        let latest = groups.first().and_then(|(year, ..)| *year);
        let mut last_year = None;
        for (year, season, works) in groups {
            // A year folded shows as one line, under its first season.
            if !self.year_open(year, latest) {
                if last_year == Some(year) {
                    continue;
                }
                last_year = Some(year);
                if let Some(year) = year {
                    let mut fold = div()
                        .id(SharedString::from(format!("built-year-{year}")))
                        .role(Role::Button)
                        .aria_expanded(false)
                        .mx_3()
                        .py_1()
                        .flex()
                        .justify_between()
                        .border_b_1()
                        .border_color(ink(RULE))
                        .cursor_pointer()
                        .text_sm()
                        .text_color(ink(INK))
                        .child(crate::i18n::year_label(year))
                        .child(div().text_xs().text_color(ink(INK_SOFT)).child("▸"));
                    if let Some(weak) = weak.clone() {
                        fold = fold.on_click(move |_, _, cx| {
                            let _ = weak.update(cx, |this, cx| {
                                if !this.looking.years.remove(&year) {
                                    this.looking.years.insert(year);
                                }
                                cx.notify();
                            });
                        });
                    }
                    built = built.child(fold);
                }
                continue;
            }
            last_year = Some(year);
            let label = match (season, year) {
                (Some(season), Some(year)) => crate::i18n::season_in_year(season, year),
                (None, Some(year)) => crate::i18n::year_label(year),
                _ => ui::t("Added to what stands").to_string(),
            };
            let mut grid = div().mx_3().flex().flex_wrap().gap_x_2().gap_y_1();
            for work in works {
                grid = grid.child(
                    list_entry(
                        SharedString::from(format!("built-{}", work.goal.id)),
                        work.goal.label.clone(),
                    )
                    .w(px((DRAWER_WIDTH - 56.0) / 2.0))
                    .py_1()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(work_icon(snapshot, work.goal).w(px(28.0)).h(px(22.0)))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .text_xs()
                            .line_height(relative(1.3))
                            .text_color(ink(INK))
                            .child(work.goal.label.clone()),
                    ),
                );
            }
            built = built.child(
                ui::region(
                    SharedString::from(format!("built-{label}")),
                    Role::List,
                    label.clone(),
                )
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .mx_3()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink(INK_SOFT))
                        .child(label),
                )
                .child(grid),
            );
        }
        Some(section.child(built))
    }

    /// What people have given the player to keep, newest first, each thing
    /// once: what it is, how many times and from whom, and the newest word
    /// said with it.
    pub(crate) fn render_keepsakes(&self) -> Option<Stateful<Div>> {
        if self.snapshot.keepsakes.is_empty() {
            return None;
        }
        let kept = kept_once(&self.snapshot);
        let heading = format!("{} · {}", ui::t("Keepsakes"), kept.len());
        let mut list = ui::region("drawer-keepsakes", Role::List, heading.clone())
            .flex()
            .flex_col()
            .gap_1()
            .child(leaf_heading(heading));
        for (index, keepsake) in kept.iter().enumerate() {
            let from = crate::i18n::from_whom(&names_list(&keepsake.from));
            let what = capitalized(keepsake.what);
            let said = if keepsake.note.is_empty() {
                String::new()
            } else {
                format!(": “{}”", keepsake.note)
            };
            list = list.child(
                list_entry(
                    SharedString::from(format!("keepsake-{index}")),
                    format!("{what}. {from}{said}"),
                )
                .mx_3()
                .py_2()
                .border_b_1()
                .border_color(ink(RULE))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(ink(INK))
                                .child(what),
                        )
                        .when(keepsake.times > 1, |row| {
                            row.child(
                                div()
                                    .flex_shrink_0()
                                    .text_xs()
                                    .text_color(ink(INK_SOFT))
                                    .child(format!("× {}", keepsake.times)),
                            )
                        }),
                )
                .child(div().text_xs().text_color(ink(INK_SOFT)).child(from))
                .when(!keepsake.note.is_empty(), |row| {
                    row.child(
                        div()
                            .text_xs()
                            .italic()
                            .text_color(ink(INK_SOFT))
                            .line_clamp(2)
                            .child(format!("“{}”", keepsake.note)),
                    )
                }),
            );
        }
        Some(list)
    }

    /// The letter box: what people have written the player, newest first.
    pub(crate) fn render_letters(&self) -> Option<Stateful<Div>> {
        if self.snapshot.letters.is_empty() {
            return None;
        }
        let heading = format!("{} · {}", ui::t("Letters"), self.snapshot.letters.len());
        let mut letters = ui::region("drawer-letters", Role::List, heading.clone())
            .flex()
            .flex_col()
            .gap_2()
            .child(leaf_heading(heading));
        for (index, letter) in self
            .snapshot
            .letters
            .iter()
            .enumerate()
            .rev()
            .take(LETTERS_SHOWN)
        {
            let from = crate::i18n::from_whom(
                &label_of(&self.snapshot, letter.from)
                    .map(|name| first_name(&name))
                    .unwrap_or_else(|| ui::t("a friend").to_string()),
            );
            letters = letters.child(
                list_entry(
                    SharedString::from(format!("letter-{index}")),
                    format!("{from}: {}", letter.note),
                )
                .mx_3()
                .px_3()
                .py_2()
                .rounded(px(2.0))
                .bg(ink(0xfbf7ee))
                .border_1()
                .border_color(ink(RULE))
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_xs().text_color(ink(INK_SOFT)).child(from))
                .when(!letter.note.is_empty(), |card| {
                    card.child(
                        div()
                            .text_sm()
                            .line_height(relative(1.45))
                            .text_color(ink(INK))
                            .line_clamp(4)
                            .child(letter.note.clone()),
                    )
                }),
            );
        }
        Some(letters)
    }

    /// A favour someone asked of the player, as a quiet note tucked under
    /// the drawer's title: who asked for what, and how talk can do it; once
    /// done, the same note with its dot gone quiet and a word of thanks.
    pub(crate) fn render_favour_note(&self) -> Option<Stateful<Div>> {
        let favour = self.snapshot.favour.as_ref()?;
        let under = if favour.done {
            ui::t(FAVOUR_DONE).to_string()
        } else {
            favour.hint.clone()
        };
        Some(
            ui::region("drawer-favour", Role::Article, favour.note.clone())
                .mx_4()
                .mb_2()
                .px_3()
                .py_2()
                .rounded(px(2.0))
                .bg(ink(0xfbf7ee))
                .border_1()
                .border_color(ink(RULE))
                .flex()
                .items_start()
                .gap_2()
                .child(
                    // A dab of paint, like the mark on a note left out.
                    div()
                        .mt(px(5.0))
                        .size(px(7.0))
                        .flex_shrink_0()
                        .rounded_full()
                        .when(favour.done, |dot| dot.bg(ink(RULE)))
                        .when(!favour.done, |dot| dot.bg(color(tokens::ACCENT))),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .min_w(px(0.0))
                        .child(
                            div()
                                .text_sm()
                                .line_height(relative(1.35))
                                .text_color(ink(if favour.done { INK_SOFT } else { INK }))
                                .line_clamp(2)
                                .child(favour.note.clone()),
                        )
                        .when(!under.is_empty(), |note| {
                            note.child(
                                div()
                                    .text_xs()
                                    .italic()
                                    .text_color(ink(INK_SOFT))
                                    .line_clamp(1)
                                    .child(under),
                            )
                        }),
                ),
        )
    }

    /// The book of everything to find: a shelf each for keepsakes, people,
    /// things made and festival days, what has been found drawn in colour
    /// and what is still to come as a silhouette with a hint.
    pub(crate) fn render_book(
        &self,
        open: Option<gpui::WeakEntity<Self>>,
    ) -> Option<Stateful<Div>> {
        let book = &self.snapshot.book;
        if book.is_empty() {
            return None;
        }
        let found = book.iter().filter(|entry| entry.found).count();
        let heading = format!(
            "{} · {}",
            ui::t("Book"),
            crate::i18n::of_count(found, book.len())
        );
        let mut section = ui::region("drawer-book", Role::Group, heading.clone())
            .flex()
            .flex_col()
            .gap_4()
            .child(leaf_heading(heading));
        let mut shelves = Vec::<&str>::new();
        for entry in book {
            if !shelves.contains(&entry.shelf.as_str()) {
                shelves.push(&entry.shelf);
            }
        }
        for shelf in shelves {
            let entries = book
                .iter()
                .filter(|entry| entry.shelf == shelf)
                .collect::<Vec<_>>();
            let found = entries.iter().filter(|entry| entry.found).count();
            let mut grid = div().flex().flex_wrap().gap_2();
            // A long shelf shows its newest found and what is still to find,
            // a page's worth, and says how many more it keeps.
            const SHELF: usize = 18;
            let hidden = entries.len().saturating_sub(SHELF);
            for (index, entry) in entries.iter().enumerate().skip(hidden) {
                let look = entry
                    .found
                    .then(|| book_look(&self.snapshot, entry))
                    .flatten();
                let look = match (&entry.moment, entry.found) {
                    (Some(id), true) => self
                        .snapshot
                        .moments
                        .iter()
                        .find(|moment| &moment.id == id)
                        .and_then(|moment| {
                            let scene = super::stories::panel_scenes(&self.snapshot, moment)
                                .into_iter()
                                .nth(1)?;
                            Some(BookLook::Moment(moment.id.clone(), Box::new(scene)))
                        })
                        .or(look),
                    _ => look,
                };
                let mut tile = book_tile(entry, index, look);
                // A moment opens in its panels; someone, somewhere or
                // something opens their story.
                if let (Some(open), true) = (open.clone(), entry.found) {
                    let moment = entry.moment.clone();
                    let subject = entry.cast.first().copied();
                    if moment.is_some() || subject.is_some() {
                        tile = tile.cursor_pointer().on_click(move |_, _, cx| {
                            let _ = open.update(cx, |this, cx| match (&moment, subject) {
                                (Some(moment), _) => this.open_moment(moment, cx),
                                (None, Some(subject)) => this.open_legend(subject, cx),
                                _ => {}
                            });
                        });
                    }
                }
                grid = grid.child(tile);
            }
            let heading = format!(
                "{} · {}",
                ui::t(shelf.to_string()),
                crate::i18n::of_count(found, entries.len())
            );
            if hidden > 0 {
                grid = grid.child(
                    div()
                        .w(px(TILE_W))
                        .h(px(PICTURE_H + 40.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .text_center()
                        .text_color(ink(INK_SOFT))
                        .child(crate::i18n::more_kept(hidden)),
                );
            }
            section = section.child(
                ui::region(
                    SharedString::from(format!("shelf-{shelf}")),
                    Role::List,
                    heading.clone(),
                )
                .px_3()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink(INK_SOFT))
                        .child(heading),
                )
                .child(grid),
            );
        }
        Some(section)
    }

    /// The drawer: the World's name and the leaves of its book, one open.
    pub(crate) fn render_drawer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let leaf = self.open_leaf();
        let mut tabs = div()
            .id("drawer-leaves")
            .role(Role::TabList)
            .aria_label(ui::t("The drawer"))
            .px_4()
            .flex()
            .gap_4()
            .border_b_1()
            .border_color(ink(RULE));
        for each in self.leaves() {
            let on = each == leaf;
            tabs = tabs.child(
                div()
                    .id(SharedString::from(format!("leaf-{}", each.id())))
                    .role(Role::Tab)
                    .aria_selected(on)
                    .aria_label(ui::t(each.name()))
                    .pb_2()
                    .pt_1()
                    .text_sm()
                    .cursor_pointer()
                    .border_b_2()
                    .when(on, |tab| {
                        tab.border_color(color(tokens::ACCENT))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ink(INK))
                    })
                    .when(!on, |tab| {
                        tab.border_color(gpui::transparent_black())
                            .text_color(ink(INK_SOFT))
                            .hover(|style| style.text_color(ink(INK)))
                    })
                    .child(ui::t(each.name()))
                    .on_click(cx.listener(move |this, _, _, cx| this.turn_to(each, cx))),
            );
        }
        let header = div()
            .px_4()
            .pt_4()
            .pb_2()
            .flex()
            .items_start()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ink(INK))
                            .child(self.snapshot.title.clone()),
                    )
                    .children(
                        coming_label(&self.snapshot)
                            .map(|coming| div().text_xs().text_color(ink(INK_SOFT)).child(coming)),
                    ),
            )
            .child(arrow_button(
                "drawer-close",
                "×",
                "Close the drawer",
                cx.listener(|this, _, _, cx| this.toggle_drawer(cx)),
            ));
        let parts = match leaf {
            Leaf::Story => vec![
                self.render_almanacs(cx).map(IntoElement::into_any_element),
                self.render_chapters(Some(&mut *cx))
                    .map(IntoElement::into_any_element),
            ],
            Leaf::Built => vec![self
                .render_builds(Some(&mut *cx))
                .map(IntoElement::into_any_element)],
            Leaf::Kept => vec![
                self.render_letters().map(IntoElement::into_any_element),
                self.render_keepsakes().map(IntoElement::into_any_element),
            ],
            Leaf::Found => vec![self
                .render_book(Some(cx.entity().downgrade()))
                .map(IntoElement::into_any_element)],
            Leaf::More => vec![
                self.render_closer_look(cx)
                    .map(IntoElement::into_any_element),
                self.render_story(cx).map(IntoElement::into_any_element),
                self.render_standing(cx).map(IntoElement::into_any_element),
                self.render_cast(cx).map(IntoElement::into_any_element),
                self.render_history(cx).map(IntoElement::into_any_element),
            ],
        };
        let mut body = div().flex().flex_col().gap_6().px_1().py_4();
        for part in parts.into_iter().flatten() {
            body = body.child(div().px_1().child(part));
        }
        div()
            .id("world-drawer")
            .role(Role::Complementary)
            .aria_label(ui::t("The drawer"))
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .w(px(DRAWER_WIDTH))
            .bg(ink(PAPER))
            .border_l_1()
            .border_color(ink(RULE))
            .shadow_lg()
            .flex()
            .flex_col()
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(header)
            .children(self.render_favour_note())
            .child(tabs)
            .child(
                div()
                    .id(SharedString::from(format!("drawer-leaf-{}", leaf.id())))
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .child(body),
            )
    }
}

/// A leaf's section heading, small and quiet in the paper's ink.
fn leaf_heading(text: impl Into<SharedString>) -> Div {
    div()
        .px_3()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(ink(INK_SOFT))
        .child(ui::t(text))
}

/// One chapter: its number in the margin, its title, and a sentence or
/// two of how it went.
fn chapter_entry(chapter: &world_projection::Chapter) -> Stateful<Div> {
    let summary = short_summary(&chapter.summary);
    list_entry(
        SharedString::from(format!("chapter-{}", chapter.number)),
        format!(
            "{}: {}. {}",
            crate::i18n::chapter_label(chapter.number),
            chapter.title,
            chapter.summary
        ),
    )
    .mx_3()
    .py_2()
    .flex()
    .gap_3()
    .border_b_1()
    .border_color(ink(RULE))
    .child(
        div()
            .w(px(26.0))
            .flex_shrink_0()
            .text_right()
            .text_lg()
            .line_height(relative(1.1))
            .text_color(ink(INK_SOFT).opacity(0.7))
            .child(chapter.number.to_string()),
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
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(ink(INK))
                    .child(chapter.title.clone()),
            )
            .when(!summary.is_empty(), |text| {
                text.child(
                    div()
                        .text_xs()
                        .line_height(relative(1.45))
                        .text_color(ink(INK_SOFT))
                        .child(summary),
                )
            }),
    )
}

/// A work as the scene draws it, small: its own drawing when the scene
/// has one, else its shape.
fn work_icon(snapshot: &ProjectionSnapshot, goal: &Goal) -> gpui::Canvas<()> {
    let name = goal.label.trim().to_lowercase();
    let item = snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.label.trim().to_lowercase() == name);
    let drawing = item.and_then(|item| snapshot.drawing_of(item)).cloned();
    let key = item.map_or_else(|| goal.id.clone(), |item| item.id.stable_key());
    let shape = goal.shape;
    let finished = goal.finished();
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let x = f32::from(bounds.origin.x) + f32::from(bounds.size.width) / 2.0;
            let base = f32::from(bounds.origin.y) + f32::from(bounds.size.height);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            let palette = art::Palette::of(&key, false);
            match (&drawing, finished) {
                (Some(drawing), true) => {
                    let tall = h.min(w / drawing.aspect.max(0.3));
                    art::paint_drawing(
                        window,
                        x,
                        base,
                        tall * drawing.aspect,
                        tall,
                        drawing,
                        &art::Inks::of_place(&palette),
                        world_projection::Stance::Standing,
                        world_projection::Mood::Content,
                        0.0,
                        0.0,
                        1.0,
                    );
                }
                (None, true) => art::paint_building(window, x, base, w * 0.8, h, shape, &palette),
                (_, false) => {
                    let soft: Hsla = ink(INK_SOFT).into();
                    crate::ui::paint_mark(
                        window,
                        bounds,
                        shape,
                        soft.opacity(0.55),
                        soft.opacity(0.3),
                    );
                }
            }
        },
    )
}

/// How a found entry of the book is drawn: someone as the scene draws
/// them, or a place or thing in its own drawing.
#[derive(Clone)]
pub(crate) enum BookLook {
    Someone(Likeness),
    /// A moment kept in the book: its middle panel, as painted.
    Moment(String, Box<crate::panels::PanelScene>),
    Something {
        drawing: Option<world_projection::Drawing>,
        palette: art::Palette,
    },
}

/// The book entry's own drawing, found by its name on the scene: the
/// person, place or thing it is. Someone no longer on the scene is still
/// drawn as themselves, in the colours their name gives them.
pub(crate) fn book_look(snapshot: &ProjectionSnapshot, entry: &BookEntry) -> Option<BookLook> {
    let name = entry.name.trim().to_lowercase();
    let item = snapshot.canvas.items.iter().find(|item| {
        let label = item.label.trim().to_lowercase();
        !label.is_empty() && (label == name || label.split_whitespace().next() == Some(&name))
    });
    match (item, entry.shape) {
        (Some(item), _) if item.kind == world_projection::CanvasItemKind::Actor => {
            Some(BookLook::Someone(likeness_of(snapshot, item.id)))
        }
        (Some(item), _) => Some(BookLook::Something {
            drawing: snapshot.drawing_of(item).cloned(),
            palette: art::Palette::of(&item.id.stable_key(), false),
        }),
        (None, None) => Some(BookLook::Someone(Likeness {
            figure: art::Figure::of(&entry.name, None),
            drawing: None,
            mood: world_projection::Mood::default(),
        })),
        (None, Some(_)) => None,
    }
}

/// A tile's caption, cut at a word to two short lines: the whole name is
/// what a screen reader hears, and what the entry opens to.
pub(crate) fn tile_caption(text: &str) -> String {
    // In half-widths: a Chinese character is as wide as two letters.
    const MOST: usize = 26;
    let width = |text: &str| {
        text.chars()
            .map(|c| if c.is_ascii() { 1 } else { 2 })
            .sum::<usize>()
    };
    if width(text) <= MOST {
        return text.to_string();
    }
    let mut out = String::new();
    if !text.contains(' ') {
        for c in text.chars() {
            if width(&out) + 2 > MOST - 2 {
                break;
            }
            out.push(c);
        }
        return format!("{out}…");
    }
    for word in text.split(' ') {
        if width(&out) + width(word) + 1 > MOST - 1 {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    format!("{}…", out.trim_end_matches([',', '.', ';', ':']))
}

/// The size of a book tile, and of the picture in it: the picture is cut
/// to its frame, so a figure never reaches down over the caption.
pub(crate) const TILE_W: f32 = 100.0;
pub(crate) const PICTURE_W: f32 = 84.0;
pub(crate) const PICTURE_H: f32 = 60.0;

/// One entry of the book: drawn in colour with its name once found, in
/// its own drawing (the person's or the thing's) when there is one, and a
/// silhouette with a hint until then. The picture sits in its own frame
/// above the caption and is cut to it.
pub(crate) fn book_tile(entry: &BookEntry, index: usize, look: Option<BookLook>) -> Stateful<Div> {
    let found = entry.found;
    let shape = entry.shape;
    let key = entry.name.clone();
    let picture: gpui::AnyElement = match look.clone() {
        Some(BookLook::Someone(likeness)) if found => div()
            .size_full()
            .flex()
            .justify_center()
            .child(portrait(likeness, PICTURE_H, false))
            .into_any_element(),
        _ => canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let x = f32::from(bounds.origin.x) + f32::from(bounds.size.width) / 2.0;
                let base = f32::from(bounds.origin.y) + f32::from(bounds.size.height) - 4.0;
                let h = f32::from(bounds.size.height) - 8.0;
                let w = h * 1.1;
                let shadow: Hsla = ink(INK_SOFT).opacity(0.35).into();
                match (&look, shape, found) {
                    (Some(BookLook::Moment(id, scene)), _, true) => {
                        let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                        let dpr = window.scale_factor().max(1.0);
                        let mut key = crate::painter::Key::new("book-moment");
                        key.add(id).float(dpr).float(w).float(h);
                        let key = key.finish();
                        match crate::painter::ready(key) {
                            Some(crate::painter::Ready::Image(image, _)) => {
                                let _ = window.paint_image(
                                    bounds,
                                    bounds,
                                    gpui::Corners::all(px(3.0)),
                                    image,
                                    0,
                                    false,
                                );
                            }
                            Some(crate::painter::Ready::Empty) => {}
                            None => {
                                let scene = (**scene).clone();
                                crate::painter::want(
                                    window,
                                    key,
                                    crate::painter::synchronous(),
                                    Box::new(move || crate::panels::paint_panel(&scene, w, h, dpr)),
                                );
                                window.request_animation_frame();
                            }
                        }
                    }
                    (
                        Some(BookLook::Something {
                            drawing: Some(drawing),
                            palette,
                        }),
                        _,
                        true,
                    ) => {
                        let tall = h.min(PICTURE_W * 0.8 / drawing.aspect.max(0.3));
                        art::paint_drawing(
                            window,
                            x,
                            base,
                            tall * drawing.aspect,
                            tall,
                            drawing,
                            &art::Inks::of_place(palette),
                            world_projection::Stance::Standing,
                            world_projection::Mood::Content,
                            0.0,
                            0.0,
                            1.0,
                        );
                    }
                    (look, Some(shape), true) => {
                        let palette = match look {
                            Some(BookLook::Something { palette, .. }) => *palette,
                            _ => art::Palette::of(&key, false),
                        };
                        art::paint_building(window, x, base, w, h * 0.9, shape, &palette);
                    }
                    (_, Some(shape), false) => {
                        crate::ui::paint_mark(
                            window,
                            gpui::Bounds::new(
                                gpui::point(px(x - w / 2.0), px(base - h * 0.9)),
                                gpui::size(px(w), px(h * 0.9)),
                            ),
                            shape,
                            shadow,
                            shadow,
                        );
                    }
                    (_, None, _) => {
                        // Someone not met yet: a head and shoulders.
                        let colour: Hsla = if found { art::hex(0x7a8fb0) } else { shadow };
                        let r = h * 0.2;
                        art::circle(window, x, base - h * 0.62, r, colour);
                        art::rect(
                            window,
                            x - h * 0.3,
                            base - h * 0.38,
                            h * 0.6,
                            h * 0.38,
                            h * 0.2,
                            colour,
                        );
                    }
                }
            },
        )
        .size_full()
        .into_any_element(),
    };
    let label = if found {
        capitalized(&entry.name)
    } else {
        format!("{}: {}", ui::t("Not found yet"), entry.hint)
    };
    list_entry(SharedString::from(format!("book-{index}")), label)
        .w(px(TILE_W))
        .p_1()
        .rounded(px(4.0))
        .when(found, |tile| {
            tile.bg(ink(0xfbf7ee)).border_1().border_color(ink(RULE))
        })
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .child(
            div()
                .id(SharedString::from(format!("book-picture-{index}")))
                .w(px(PICTURE_W))
                .h(px(PICTURE_H))
                .flex_shrink_0()
                .rounded(px(3.0))
                .overflow_hidden()
                .child(picture),
        )
        .child(
            div()
                .w(px(TILE_W - 8.0))
                .overflow_hidden()
                .text_xs()
                .text_center()
                .line_height(relative(1.3))
                .text_color(ink(if found { INK } else { INK_SOFT }))
                .child(tile_caption(&if found {
                    capitalized(&entry.name)
                } else {
                    entry.hint.clone()
                })),
        )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ui::accessible;
    use world_projection::{Calendar, Chapter, Keepsake, Letter};

    fn someone() -> SelectionId {
        SelectionId::from_stable_key("entity-7").expect("an entity key")
    }

    fn wire(json: &str) -> ProjectionSnapshot {
        let wire: world_pack_protocol::ProjectionSnapshotWire =
            serde_json::from_str(json).expect("a wire snapshot");
        ProjectionSnapshot::try_from(wire).expect("a snapshot")
    }

    /// A real harbour on its 358th day: 34 chapters, 38 keepsakes, 100
    /// letters.
    fn day_358() -> ProjectionSnapshot {
        wire(include_str!("../../tests/fixtures/harbour-day-358.json"))
    }

    /// The same harbour three years on: 65 works.
    fn day_1082() -> ProjectionSnapshot {
        // Only what the scene draws is kept there; the rest from day 358.
        let mut base: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/harbour-day-358.json"))
                .expect("json");
        let later: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/harbour-day-1082.json"))
                .expect("json");
        for (key, value) in later.as_object().expect("an object") {
            base[key] = value.clone();
        }
        wire(&base.to_string())
    }

    pub(crate) fn a_full_drawer() -> ProjectionSnapshot {
        let mut snapshot = ProjectionSnapshot {
            letters: vec![Letter {
                from: someone(),
                note: "The pier is mended.".into(),
                moment: someone(),
            }],
            keepsakes: vec![
                Keepsake {
                    from: someone(),
                    what: "a pressed flower".into(),
                    note: "From the harbour".into(),
                    moment: someone(),
                },
                Keepsake {
                    from: someone(),
                    what: "A pressed flower".into(),
                    note: "Another".into(),
                    moment: someone(),
                },
            ],
            chapters: vec![Chapter {
                number: 1,
                title: "The storm".into(),
                summary: "The town came through.".into(),
                moment: None,
            }],
            book: vec![
                BookEntry {
                    shelf: "Keepsakes".into(),
                    name: "a pressed flower".into(),
                    found: true,
                    shape: None,
                    hint: String::new(),
                    ..Default::default()
                },
                BookEntry {
                    shelf: "Keepsakes".into(),
                    name: "a shell".into(),
                    found: false,
                    shape: None,
                    hint: "Someone by the sea".into(),
                    ..Default::default()
                },
            ],
            ..ProjectionSnapshot::default()
        };
        snapshot.canvas.items.push(world_projection::CanvasItem {
            id: someone(),
            kind: world_projection::CanvasItemKind::Actor,
            label: "Mara Quinn".into(),
            ..Default::default()
        });
        snapshot
    }

    /// Each part of the drawer is a region a screen reader can find by
    /// name, and what is in it is read out entry by entry.
    #[test]
    fn the_drawer_sections_are_named_lists() {
        let view = ProjectionView::new(a_full_drawer());
        let named = |part: Option<Stateful<Div>>| {
            let (role, node) = accessible(&part.expect("a section"));
            (role, node.label().map(str::to_string))
        };
        assert_eq!(
            named(view.render_letters()),
            (Some(Role::List), Some("Letters · 1".into()))
        );
        // The same keepsake given twice is kept once.
        assert_eq!(
            named(view.render_keepsakes()),
            (Some(Role::List), Some("Keepsakes · 1".into()))
        );
        assert_eq!(
            named(view.render_book(None)),
            (Some(Role::Group), Some("Book · 1 of 2".into()))
        );
        assert_eq!(
            named(view.render_chapters(None)),
            (Some(Role::Group), Some("The story so far".into()))
        );
        let snapshot = a_full_drawer();
        let (role, found) = accessible(&book_tile(&snapshot.book[0], 0, None));
        assert_eq!(role, Some(Role::ListItem));
        assert_eq!(found.label(), Some("A pressed flower"));
        let (_, missing) = accessible(&book_tile(&snapshot.book[1], 1, None));
        assert_eq!(missing.label(), Some("Not found yet: Someone by the sea"));
        assert_eq!(
            view.leaves(),
            vec![Leaf::Story, Leaf::Kept, Leaf::Found, Leaf::More]
        );
    }

    /// A keepsake given four times is one entry that says so, with
    /// everyone who gave it.
    #[test]
    fn keepsakes_are_counted_once() {
        let snapshot = day_358();
        let kept = kept_once(&snapshot);
        assert!(kept.len() < snapshot.keepsakes.len());
        let mut names = kept
            .iter()
            .map(|kept| kept.what.trim().to_lowercase())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), kept.len(), "each thing once");
        let scarf = kept
            .iter()
            .find(|kept| kept.what.contains("scarf"))
            .expect("the scarf");
        assert_eq!(scarf.times, 4);
        assert_eq!(
            kept.iter().map(|kept| kept.times).sum::<usize>(),
            snapshot.keepsakes.len()
        );
    }

    /// A tile's caption fits under its picture: two short lines at most.
    #[test]
    fn a_tile_caption_fits_its_tile() {
        assert_eq!(
            tile_caption("A note under the door"),
            "A note under the door"
        );
        let long = tile_caption("A jar of something homemade from Rosa's kitchen");
        assert!(long.chars().count() <= 26, "{long}");
        assert!(long.ends_with('…'));
        assert!(
            tile_caption("一张在你离开时拍的港口照片，边角已经卷起来了")
                .chars()
                .count()
                <= 14
        );
    }

    /// A chapter's summary is a sentence or two, never a paragraph.
    #[test]
    fn a_chapter_is_summed_up_in_a_sentence_or_two() {
        let long = "The school planted a garden. The Anchor made its music night a monthly thing. When times were hard, the harbour shared what it had. Leo and Emma became firm friends.";
        let short = short_summary(long);
        assert!(short.chars().count() <= 115, "{short}");
        assert!(short.starts_with("The school planted a garden."));
        assert!(short.ends_with('…'));
        assert_eq!(short_summary("Quiet."), "Quiet.");
        assert_eq!(
            short_summary("码头修好了。大家都来了。风很大，但船都回来了。孩子们在沙滩上跑。"),
            "码头修好了。大家都来了。风很大，但船都回来了。孩子们在沙滩上跑。"
        );
        for chapter in &day_358().chapters {
            assert!(short_summary(&chapter.summary).chars().count() <= 115);
        }
    }

    /// At day 358, the chapters fold by year: only this year's are
    /// listed at first, the year before is one line.
    #[test]
    fn the_story_opens_at_this_year() {
        let snapshot = day_358();
        let year = snapshot.calendar.as_ref().and_then(|c| c.year).unwrap() as u32;
        let now = year_of(&snapshot, today(&snapshot)).unwrap();
        assert!(
            now >= 2,
            "a harbour into its later years ({year}-day years)"
        );
        let length = snapshot.calendar.as_ref().unwrap().length;
        let this_year = snapshot
            .chapters
            .iter()
            .filter(|chapter| {
                chapter
                    .moment
                    .and_then(|moment| {
                        snapshot
                            .timeline
                            .items
                            .iter()
                            .find(|item| item.id == moment)
                    })
                    .map(|item| world_projection::day_of(item.world_time, length))
                    .and_then(|day| year_of(&snapshot, day))
                    == Some(now)
            })
            .count();
        assert!(this_year < snapshot.chapters.len());
    }

    /// Almanacs: the World's own list, or every year gone by its calendar,
    /// and none before the first New Year.
    #[test]
    fn every_year_gone_has_its_almanac() {
        let mut snapshot = ProjectionSnapshot {
            calendar: Some(Calendar {
                unit: "Day".into(),
                length: 10,
                season: Some("Spring".into()),
                coming: None,
                festival_today: false,
                year: Some(120),
            }),
            ..ProjectionSnapshot::default()
        };
        snapshot.world_time = 10 * 119;
        assert!(almanac_years(&snapshot).is_empty(), "not yet a year");
        snapshot.world_time = 10 * 121;
        assert_eq!(almanac_years(&snapshot), vec![1]);
        snapshot.world_time = 10 * 362;
        assert_eq!(almanac_years(&snapshot), vec![1, 2, 3]);
        snapshot.almanac_years = vec![1, 2];
        assert_eq!(almanac_years(&snapshot), vec![1, 2]);
    }

    /// The seasons are worked back from the one it is today.
    #[test]
    fn a_day_is_put_in_its_season() {
        let snapshot = ProjectionSnapshot {
            world_time: 10 * 125,
            calendar: Some(Calendar {
                unit: "Day".into(),
                length: 10,
                season: Some("Spring".into()),
                coming: None,
                festival_today: false,
                year: Some(120),
            }),
            ..ProjectionSnapshot::default()
        };
        assert_eq!(season_of(&snapshot, 1), Some("Spring"));
        assert_eq!(season_of(&snapshot, 31), Some("Summer"));
        assert_eq!(season_of(&snapshot, 100), Some("Winter"));
        assert_eq!(year_of(&snapshot, 100), Some(1));
        assert_eq!(year_of(&snapshot, 121), Some(2));
    }

    /// Three years of works read as seasons of a few each, never one list
    /// of sixty-five, and every kind of work has its group.
    #[test]
    fn works_are_grouped_by_season_and_kind() {
        let snapshot = day_1082();
        let done = finished(&snapshot);
        assert!(done.len() >= 40, "{}", done.len());
        let dated = done.iter().filter(|work| work.day.is_some()).count();
        // What was added to a work already standing has no day of its own.
        assert!(dated * 4 >= done.len() * 3, "{dated} of {}", done.len());
        let groups = by_season(&snapshot, done);
        assert!(groups.len() >= 6, "{} groups", groups.len());
        let largest = groups
            .iter()
            .filter(|(year, ..)| year.is_some())
            .map(|(.., works)| works.len())
            .max()
            .unwrap();
        assert!(largest <= 12, "a season of {largest}");
        for (_, _, works) in &groups {
            let kinds = works
                .iter()
                .map(|work| kind_of(work.goal.shape))
                .collect::<Vec<_>>();
            let mut sorted = kinds.clone();
            sorted.sort();
            assert_eq!(kinds, sorted, "kinds together");
        }
    }
}
