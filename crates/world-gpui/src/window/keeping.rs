//! The player's own Book, as the drawer keeps it and as a year closes:
//! its title page (what the World calls the player), this year's pages
//! filled or still to come, each year closed with its credits, and what
//! the World works toward, each counted. Presentation only: everything
//! here is read from the snapshot's `keeping`, in the World's own words.

use super::drawer::{ink, leaf_heading, RULE};
use super::world_window::{list_entry, scene_paper};
use super::ProjectionView;
use crate::postcard::{INK, INK_SOFT};
use crate::ui::{self, color, ButtonKind};
use gpui::{
    div, prelude::FluentBuilder, px, relative, Context, Div, FontWeight, InteractiveElement,
    ParentElement, Role, SharedString, Stateful, StatefulInteractiveElement, Styled,
};
use world_projection::{Aim, Keeping, Page, YearClosed};
use world_theme::tokens;

/// How many credits a closed year's card rolls before it says how many
/// more the drawer keeps.
const CREDITS_SHOWN: usize = 14;

/// A mark for a page or a step: filled in, or an outline still to come.
fn pip(filled: bool) -> Div {
    div()
        .size(px(8.0))
        .flex_shrink_0()
        .rounded_full()
        .border_1()
        .border_color(color(tokens::ACCENT))
        .when(filled, |pip| pip.bg(color(tokens::ACCENT)))
}

fn page_entry(index: usize, page: &Page) -> Stateful<Div> {
    list_entry(
        SharedString::from(format!("page-{index}")),
        format!("{}. {}", page.title, page.line),
    )
    .mx_3()
    .py_1()
    .flex()
    .items_start()
    .gap_3()
    .child(div().pt(px(5.0)).child(pip(page.filled)))
    .child(
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(ink(if page.filled { INK } else { INK_SOFT }))
                    .child(page.title.clone()),
            )
            .when(!page.line.is_empty(), |text| {
                text.child(
                    div()
                        .text_xs()
                        .line_height(relative(1.45))
                        .text_color(ink(INK_SOFT))
                        .child(page.line.clone()),
                )
            }),
    )
}

/// A year's credits, a name and a deed a line.
fn credits(year: &YearClosed, most: usize) -> Div {
    let mut roll = div().flex().flex_col().gap(px(2.0));
    for credit in year.credits.iter().take(most) {
        roll = roll.child(
            div()
                .flex()
                .gap_2()
                .text_xs()
                .child(
                    div()
                        .w(px(84.0))
                        .flex_shrink_0()
                        .text_right()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink(INK))
                        .child(credit.name.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .text_color(ink(INK_SOFT))
                        .child(credit.deed.clone()),
                ),
        );
    }
    let more = year.credits.len().saturating_sub(most);
    if more > 0 {
        roll = roll.child(
            div()
                .text_xs()
                .text_center()
                .text_color(ink(INK_SOFT))
                .child(crate::i18n::more_kept(more)),
        );
    }
    roll
}

fn aim_entry(index: usize, aim: &Aim) -> Stateful<Div> {
    let count = crate::i18n::of_count(aim.done as usize, aim.of as usize);
    let mut pips = div().flex().gap_1();
    for step in 0..aim.of.min(12) {
        pips = pips.child(pip(step < aim.done));
    }
    list_entry(
        SharedString::from(format!("aim-{index}")),
        format!("{}: {count}. {}", aim.label, aim.next),
    )
    .mx_3()
    .py_1()
    .flex()
    .flex_col()
    .gap(px(2.0))
    .child(
        div()
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .text_sm()
                    .text_color(ink(INK))
                    .child(aim.label.clone()),
            )
            .child(pips)
            .child(
                div()
                    .w(px(52.0))
                    .text_right()
                    .text_xs()
                    .text_color(ink(INK_SOFT))
                    .child(count),
            ),
    )
    .when(!aim.next.is_empty(), |entry| {
        entry.child(
            div()
                .text_xs()
                .text_color(ink(INK_SOFT))
                .child(aim.next.clone()),
        )
    })
}

/// The Book's title page and this year's pages, then the years it has
/// closed: the latest with its credits, the ones before by title.
pub(crate) fn title_page(keeping: &Keeping) -> Stateful<Div> {
    let mut page = ui::region("drawer-keeping", Role::Group, keeping.title.clone())
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .px_3()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink(INK))
                        .child(keeping.title.clone()),
                )
                .when(!keeping.role.is_empty(), |page| {
                    page.child(
                        div()
                            .text_xs()
                            .text_color(ink(INK_SOFT))
                            .child(keeping.role.clone()),
                    )
                }),
        );
    if !keeping.pages.is_empty() {
        let filled = keeping.pages.iter().filter(|page| page.filled).count();
        let heading = format!(
            "{} · {}",
            crate::i18n::year_label(keeping.year.max(1)),
            crate::i18n::of_count(filled, keeping.pages.len())
        );
        let mut list = ui::region("keeping-pages", Role::List, heading.clone())
            .flex()
            .flex_col()
            .gap_1()
            .child(leaf_heading(heading));
        for (index, each) in keeping.pages.iter().enumerate() {
            list = list.child(page_entry(index, each));
        }
        page = page.child(list);
    }
    if let Some(latest) = keeping.closed.last() {
        let mut years = ui::region("keeping-years", Role::List, latest.title.clone())
            .flex()
            .flex_col()
            .gap_2()
            .child(leaf_heading(latest.title.clone()))
            .child(div().px_3().child(credits(latest, usize::MAX)));
        for year in keeping.closed.iter().rev().skip(1) {
            years = years.child(
                list_entry(
                    SharedString::from(format!("year-closed-{}", year.year)),
                    year.title.clone(),
                )
                .mx_3()
                .py_1()
                .border_t_1()
                .border_color(ink(RULE))
                .text_xs()
                .text_color(ink(INK_SOFT))
                .child(year.title.clone()),
            );
        }
        page = page.child(years);
    }
    page
}

/// What the World works toward, by kind, each with how far it has come.
pub(crate) fn aims(keeping: &Keeping) -> Option<Stateful<Div>> {
    if keeping.aims.is_empty() {
        return None;
    }
    let mut section = ui::region("drawer-aims", Role::Group, "Working toward")
        .flex()
        .flex_col()
        .gap_4();
    let mut groups = Vec::<&str>::new();
    for aim in &keeping.aims {
        if !groups.contains(&aim.group.as_str()) {
            groups.push(&aim.group);
        }
    }
    let mut index = 0;
    for group in groups {
        let of_group = keeping
            .aims
            .iter()
            .filter(|aim| aim.group == group)
            .collect::<Vec<_>>();
        let done = of_group.iter().filter(|aim| aim.finished()).count();
        let heading = format!(
            "{} · {}",
            group,
            crate::i18n::of_count(done, of_group.len())
        );
        let mut list = ui::region(
            SharedString::from(format!("aims-{group}")),
            Role::List,
            heading.clone(),
        )
        .flex()
        .flex_col()
        .gap_1()
        .child(leaf_heading(heading));
        for aim in of_group {
            list = list.child(aim_entry(index, aim));
            index += 1;
        }
        section = section.child(list);
    }
    Some(section)
}

impl ProjectionView {
    /// The year that has just closed, if its card has not been turned past:
    /// shown on the day it closed.
    pub(crate) fn year_just_closed(&self) -> Option<&YearClosed> {
        let length = self
            .snapshot
            .calendar
            .as_ref()
            .map_or(1, |calendar| calendar.length.max(1));
        self.snapshot
            .keeping
            .as_ref()?
            .just_closed(self.snapshot.world_time, length)
            .filter(|closed| self.looking.year_read.is_none_or(|read| read < closed.year))
    }

    /// A year's end, as a card of its own: its title, its credits rolled,
    /// and the way on into the next year.
    pub(crate) fn render_year_end(&self, cx: &mut Context<Self>) -> Option<Div> {
        self.controller.as_ref()?;
        let closed = self.year_just_closed()?;
        let year = closed.year;
        let card = div()
            .id("year-end")
            .role(Role::Group)
            .aria_label(closed.title.clone())
            .debug_selector(|| "year-end".into())
            .p_6()
            .rounded_2xl()
            .bg(scene_paper())
            .shadow_lg()
            .border_1()
            .border_color(color(tokens::BORDER))
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .child(ui::caption("The year closes in the Book"))
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(closed.title.clone()),
            )
            .child(div().w(px(360.0)).child(credits(closed, CREDITS_SHOWN)))
            .child(div().pt_2().child(
                ui::button("year-next", "Begin the new year", ButtonKind::Primary).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.looking.year_read = Some(year);
                        cx.notify();
                    }),
                ),
            ));
        Some(div().child(ui::arrive(card, format!("year-end-{year}"), 0)))
    }
}

#[cfg(test)]
mod tests {
    use super::super::drawer::Leaf;
    use super::*;
    use crate::ui::accessible;
    use world_projection::{Calendar, Credit, ProjectionSnapshot};

    fn book() -> Keeping {
        Keeping {
            title: "The Book".into(),
            role: "You keep it.".into(),
            year: 2,
            pages: vec![Page {
                title: "Spring: a beginning".into(),
                line: "Everyone came.".into(),
                filled: true,
                moment: None,
            }],
            closed: vec![YearClosed {
                year: 1,
                title: "The end of year 1".into(),
                at: 100,
                credits: vec![Credit {
                    name: "You".into(),
                    deed: "Made 3 things by hand".into(),
                    who: None,
                }],
                moment: None,
            }],
            aims: vec![Aim {
                group: "Projects".into(),
                label: "The tower".into(),
                done: 2,
                of: 5,
                next: "Paint it".into(),
                who: None,
            }],
        }
    }

    /// The Book opens the story leaf and its aims the built one, each a
    /// region a screen reader can find; the year's card shows on the day
    /// it closed until it is turned past.
    #[test]
    fn the_book_has_its_title_page_its_aims_and_a_years_card() {
        let snapshot = ProjectionSnapshot {
            world_time: 105,
            calendar: Some(Calendar {
                unit: "Day".into(),
                length: 10,
                ..Calendar::default()
            }),
            keeping: Some(book()),
            ..ProjectionSnapshot::default()
        };
        let mut view = ProjectionView::new(snapshot);
        assert_eq!(view.leaves(), vec![Leaf::Story, Leaf::Built]);
        let keeping = book();
        let (role, node) = accessible(&title_page(&keeping));
        assert_eq!((role, node.label()), (Some(Role::Group), Some("The Book")));
        let (role, node) = accessible(&aims(&keeping).unwrap());
        assert_eq!(
            (role, node.label()),
            (Some(Role::Group), Some("Working toward"))
        );
        assert_eq!(view.year_just_closed().map(|year| year.year), Some(1));
        view.looking.year_read = Some(1);
        assert!(view.year_just_closed().is_none());
    }
}
