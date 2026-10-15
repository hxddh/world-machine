//! A sitting, in the window (v0.30 S4, "one day a day"). The host says how
//! many of the World's days this sitting may still pass ([`Allowance`]),
//! and writes down each one passed; the window keeps the rest, as
//! presentation only:
//!
//! - the morning a new sitting opens on, from the World's own record (its
//!   letter, its main question, what was tended overnight, a find), read
//!   once and put away with "Begin the day";
//! - the day's main question weighted, first among the cards;
//! - once the sitting has passed every day it may, a Goodnight card; and
//!   then the World asleep: dimmed, the sky held at night with everyone
//!   home, and no card, until the next sitting opens it again.
//!
//! None of it is World truth: the World only ever hears that a day passed,
//! as it always has, from the day's own card.

use super::world_window::{first_name, label_of, scene_paper};
use super::ProjectionView;
use crate::ui::{self, ButtonKind};
use gpui::{
    div, px, Context, Div, FontWeight, InteractiveElement, ParentElement, Role,
    StatefulInteractiveElement, Styled,
};
use world_projection::{Morning, ProjectionCommand, ProjectionSnapshot, SelectionId};
use world_theme::tokens;

/// How many of the World's days this sitting may still pass, of how many
/// it may pass in all: the host's to say (from its ledger of sittings),
/// never the World's.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Allowance {
    pub left: u32,
    pub of: u32,
}

/// What a host does when one of the sitting's days passes: writes it down.
pub type SpendDay = std::rc::Rc<dyn Fn()>;

/// The hour the sky is held at while the World sleeps: everyone is home.
pub(crate) const ASLEEP_HOUR: u32 = 23;

/// How much the sleeping World is dimmed, over the night it is held at.
const ASLEEP_VEIL: f32 = 0.33;

/// The ink of the asleep note's line: the text's own, at least 4.5:1 on
/// the night's paper.
fn asleep_ink() -> gpui::Rgba {
    ui::color(tokens::TEXT)
}

/// The sitting as the window keeps it.
#[derive(Default)]
pub(crate) struct Sitting {
    /// `None` when the host gives no allowance: days pass as they always
    /// have, without end (tests, previews, a World looked at elsewhere).
    pub(crate) allowance: Option<Allowance>,
    pub(crate) spend: Option<SpendDay>,
    /// The Goodnight card is up.
    pub(crate) goodnight: bool,
    /// The World sleeps, with the sky held at night while it does.
    pub(crate) asleep: Option<Asleep>,
    /// The morning read, by its moment: put away with "Begin the day".
    pub(crate) morning_read: Option<SelectionId>,
    /// What the host does once the morning is read: writes it down, so it
    /// shows once however often the World is opened that day.
    pub(crate) on_read: Option<ReadMorning>,
}

/// What a host does once a morning (by its moment's stable key) is read.
pub type ReadMorning = std::rc::Rc<dyn Fn(&str)>;

impl Sitting {
    /// Whether the sitting has passed every day it may.
    pub(crate) fn spent(&self) -> bool {
        self.allowance.is_some_and(|allowance| allowance.left == 0)
    }
}

/// The sky held at night while the World sleeps, let go with it (its
/// window closed, or the sleep ended).
pub(crate) struct Asleep;

#[cfg(not(test))]
static SLEEPERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
thread_local! {
    /// In tests, counted on the sleeping test's thread alone, as the held
    /// hour is.
    static SLEEPERS: std::sync::atomic::AtomicUsize = const { std::sync::atomic::AtomicUsize::new(0) };
}

/// Changes how many views sleep, returning how many did before.
fn sleepers(by: isize) -> usize {
    use std::sync::atomic::Ordering::Relaxed;
    let change = |count: &std::sync::atomic::AtomicUsize| {
        if by >= 0 {
            count.fetch_add(by as usize, Relaxed)
        } else {
            count
                .fetch_update(Relaxed, Relaxed, |now| Some(now.saturating_sub(1)))
                .unwrap_or(0)
        }
    };
    #[cfg(test)]
    return SLEEPERS.with(change);
    #[cfg(not(test))]
    change(&SLEEPERS)
}

impl Asleep {
    /// The sky held at night for as long as any view sleeps: a view
    /// rebuilt while asleep (its page asked again) holds it before the
    /// one it replaces lets go, so the night never flickers to the clock.
    fn hold() -> Self {
        if sleepers(1) == 0 {
            crate::scene::hold_hour(Some(ASLEEP_HOUR));
        }
        Asleep
    }
}

impl Drop for Asleep {
    fn drop(&mut self) {
        if sleepers(-1) == 1 {
            crate::scene::hold_hour(None);
        }
    }
}

/// Whether `command` answers the morning's main question.
pub(crate) fn is_days_question(snapshot: &ProjectionSnapshot, command: &ProjectionCommand) -> bool {
    let Some(asked) = snapshot
        .morning
        .as_ref()
        .and_then(|morning| morning.question.as_ref())
    else {
        return false;
    };
    command
        .question
        .as_ref()
        .is_some_and(|question| question.id == asked.id)
}

impl ProjectionView {
    /// Gives the sitting `allowance`, and what to do as each of its days
    /// passes (`spend`). A sitting with nothing left opens asleep.
    pub fn with_allowance(mut self, allowance: Allowance, spend: impl Fn() + 'static) -> Self {
        self.looking.sitting.allowance = Some(allowance);
        self.looking.sitting.spend = Some(std::rc::Rc::new(spend));
        if allowance.left == 0 {
            self.fall_asleep();
        }
        self
    }

    /// The morning already read this sitting, by its moment's stable key,
    /// and what to do once one is read (`on_read`): so a World opened again
    /// the same day does not show its morning twice.
    pub fn with_morning_read(
        mut self,
        read: Option<&str>,
        on_read: impl Fn(&str) + 'static,
    ) -> Self {
        self.looking.sitting.morning_read = read.and_then(SelectionId::from_stable_key);
        self.looking.sitting.on_read = Some(std::rc::Rc::new(on_read));
        self
    }

    /// How many days the sitting may still pass, if the host said.
    pub fn allowance(&self) -> Option<Allowance> {
        self.looking.sitting.allowance
    }

    /// Whether the Goodnight card is up.
    pub fn goodnight_shown(&self) -> bool {
        self.looking.sitting.goodnight
    }

    /// What the World is doing, for the title bar: heading home while the
    /// Goodnight card is up, asleep once it is said (or opened asleep),
    /// and otherwise waiting for the player.
    pub fn rest(&self) -> crate::i18n::Rest {
        use crate::i18n::Rest;
        if self.looking.sitting.goodnight {
            Rest::HeadingHome
        } else if self.looking.sitting.asleep.is_some() {
            Rest::Asleep
        } else {
            Rest::Waits
        }
    }

    /// Whether the World sleeps until the next sitting.
    pub fn asleep(&self) -> bool {
        self.looking.sitting.asleep.is_some()
    }

    /// Whether a command that lets a day pass may go ahead: not while the
    /// World sleeps, and not once the sitting is spent, when the Goodnight
    /// card comes up instead.
    pub(crate) fn day_may_pass(&mut self) -> bool {
        if self.looking.sitting.asleep.is_some() {
            return false;
        }
        if self.looking.sitting.spent() {
            self.say_goodnight();
            return false;
        }
        true
    }

    /// A day of the sitting passed: written down by the host, and the
    /// Goodnight card up if it was the last.
    pub(crate) fn day_passed(&mut self) {
        let sitting = &mut self.looking.sitting;
        let Some(allowance) = sitting.allowance.as_mut() else {
            return;
        };
        allowance.left = allowance.left.saturating_sub(1);
        if let Some(spend) = &sitting.spend {
            spend();
        }
        if allowance.left == 0 {
            self.say_goodnight();
        }
    }

    /// The Goodnight card comes up over the place, everything else put
    /// away.
    fn say_goodnight(&mut self) {
        self.looking.sitting.goodnight = true;
        self.put_away_for_the_night();
    }

    fn put_away_for_the_night(&mut self) {
        self.looking.asking = None;
        self.looking.drawer = false;
        self.looking.hands = None;
        self.looking.pointer = None;
        self.looking.favour_beat = None;
        // A chapter just ended is read in the morning, not over the night.
        if let Some(chapter) = self.snapshot.chapters.last() {
            self.looking.chapter_read = Some(chapter.number);
        }
        self.mark_morning_read();
    }

    /// This day's morning is read: kept here, and by the host.
    fn mark_morning_read(&mut self) {
        let Some(moment) = self.snapshot.morning.as_ref().map(|morning| morning.moment) else {
            return;
        };
        if self.looking.sitting.morning_read == Some(moment) {
            return;
        }
        self.looking.sitting.morning_read = Some(moment);
        if let Some(on_read) = &self.looking.sitting.on_read {
            on_read(&moment.stable_key());
        }
    }

    /// Whether the sitting covers the place: the morning, the Goodnight
    /// card or the World asleep. Nothing under it is chosen.
    pub(crate) fn sitting_covers(&self) -> bool {
        self.looking.sitting.goodnight
            || self.looking.sitting.asleep.is_some()
            || self.morning_shown().is_some()
    }

    /// The World sleeps until the next sitting: dimmed, the sky held at
    /// night with everyone home.
    pub(crate) fn fall_asleep(&mut self) {
        self.looking.sitting.goodnight = false;
        self.put_away_for_the_night();
        if self.looking.sitting.asleep.is_none() {
            self.looking.sitting.asleep = Some(Asleep::hold());
        }
    }

    /// The morning to show: this day's, until it is read, and never over
    /// a goodnight, a sleep, a farewell or a return being told.
    pub(crate) fn morning_shown(&self) -> Option<&Morning> {
        let sitting = &self.looking.sitting;
        if sitting.goodnight
            || sitting.asleep.is_some()
            || self.looking.farewell.is_some()
            || self.retelling.is_some()
        {
            return None;
        }
        let morning = self.snapshot.morning.as_ref()?;
        (sitting.morning_read != Some(morning.moment)).then_some(morning)
    }

    /// "Begin the day": the morning is put away, and the day's main
    /// question is the card in front.
    pub(crate) fn begin_the_day(&mut self) {
        self.mark_morning_read();
        self.looking.card = 0;
        self.looking.answer = self.first_available_answer();
    }

    /// What the sitting shows over the place, when it shows anything: the
    /// Goodnight card, the World asleep, or the morning.
    pub(crate) fn render_sitting(&self, cx: &mut Context<Self>) -> Option<Div> {
        if self.looking.sitting.goodnight {
            return Some(self.render_goodnight(cx));
        }
        if self.looking.sitting.asleep.is_some() {
            return Some(self.render_asleep());
        }
        self.render_morning(cx)
    }

    fn render_goodnight(&self, cx: &mut Context<Self>) -> Div {
        let paper = div()
            .id("goodnight")
            .role(Role::Dialog)
            .aria_label(ui::t("Goodnight"))
            .debug_selector(|| "goodnight".into())
            .occlude()
            .w(px(460.0))
            .max_w_full()
            .p_6()
            .rounded_2xl()
            .bg(scene_paper())
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .shadow_lg()
            .flex()
            .flex_col()
            .gap_3()
            .child(ui::caption("That's the day"))
            .child(ui::heading("Goodnight"))
            .child(ui::body(
                "Everyone's heading home. Come back tomorrow for the morning: a letter, today's choice, something looked after, and a find for your book.",
            ))
            .child(ui::caption(
                "No hurry. The town will wait for you.",
            ))
            .child(
                div().pt_2().flex().justify_end().child(
                    ui::button("goodnight-sleep", "Goodnight", ButtonKind::Primary)
                        .debug_selector(|| "goodnight-sleep".into())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.fall_asleep();
                            cx.notify();
                        })),
                ),
            );
        hanging(paper)
    }

    fn render_asleep(&self) -> Div {
        let note = div()
            .id("asleep")
            .role(Role::Status)
            .debug_selector(|| "asleep".into())
            .px_4()
            .py_3()
            .rounded_2xl()
            // Opaque, so its words keep their contrast over the night.
            .bg(scene_paper())
            .shadow_sm()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .child(ui::row_title("Asleep until tomorrow"))
            .child(
                div()
                    .text_xs()
                    .text_color(asleep_ink())
                    .child(crate::wrap::text(ui::t(
                        "Everyone's home. Come back tomorrow for the morning.",
                    ))),
            );
        // The place dimmed under the night it is held at (lightly, so the
        // night's own blues still show), and the note hanging from the
        // top, clear of everyone's doors.
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .bg(gpui::hsla(0.63, 0.35, 0.06, ASLEEP_VEIL))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top_0()
                    .px_6()
                    .pt_3()
                    .flex()
                    .justify_center()
                    .child(note),
            )
    }

    fn render_morning(&self, cx: &mut Context<Self>) -> Option<Div> {
        let morning = self.morning_shown()?;
        let mut parts = div().flex().flex_col().gap_3();
        if let Some(letter) = &morning.letter {
            let from = label_of(&self.snapshot, letter.from)
                .map(|name| first_name(&name))
                .unwrap_or_default();
            parts = parts.child(
                part("morning-letter")
                    .child(ui::section_label(crate::i18n::fill(
                        "A letter from {name}",
                        &[("name", &world_i18n::tr_owned(&from))],
                    )))
                    .child(
                        div()
                            .text_sm()
                            .italic()
                            .line_height(gpui::relative(1.5))
                            .text_color(ui::color(tokens::TEXT))
                            .child(crate::wrap::text(ui::t(letter.note.clone()))),
                    ),
            );
        }
        if let Some(question) = &morning.question {
            parts = parts.child(
                part("morning-question")
                    .child(ui::section_label("Today's choice"))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ui::color(tokens::TEXT))
                            .child(crate::wrap::text(ui::t(question.prompt.clone()))),
                    ),
            );
        }
        if let Some(tended) = &morning.tended {
            parts = parts.child(
                part("morning-tended")
                    .child(ui::section_label("Looked after"))
                    .child(ui::body(tended.line.clone())),
            );
        }
        if let Some(find) = &morning.find {
            parts = parts.child(
                part("morning-find")
                    .child(ui::section_label("For your book"))
                    .child(ui::body(find.line.clone())),
            );
        }
        let paper = div()
            .id("morning")
            .role(Role::Dialog)
            .aria_label(ui::t("Good morning"))
            .debug_selector(|| "morning".into())
            .occlude()
            .w(px(520.0))
            .max_w_full()
            .p_6()
            .rounded_2xl()
            .bg(scene_paper())
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .shadow_lg()
            .flex()
            .flex_col()
            .gap_3()
            .child(ui::heading("Good morning"))
            .child(parts)
            .child(
                div().pt_2().flex().justify_end().child(
                    ui::button("morning-begin", "Begin the day", ButtonKind::Primary)
                        .debug_selector(|| "morning-begin".into())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.begin_the_day();
                            cx.notify();
                        })),
                ),
            );
        Some(hanging(paper))
    }
}

/// One part of the morning, named for its tests.
fn part(name: &'static str) -> Div {
    div()
        .debug_selector(move || name.into())
        .flex()
        .flex_col()
        .gap_1()
}

/// A paper hanging from the top of the place, over the sky, so the people
/// stay in sight under it.
fn hanging(paper: impl gpui::IntoElement) -> Div {
    div()
        .absolute()
        .left_0()
        .right_0()
        .top_0()
        .px_6()
        .pt_3()
        .flex()
        .justify_center()
        .child(paper)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Modifiers, TestAppContext, VisualTestContext};
    use std::cell::Cell;
    use std::rc::Rc;
    use world_projection::{CommandRole, Letter, MorningLine, ProjectionIntent, Question};

    const DAY: &str = "world.let-day-pass";

    fn command(id: &str, title: &str) -> ProjectionCommand {
        ProjectionCommand {
            id: id.into(),
            title: title.into(),
            detail: String::new(),
            effects: Vec::new(),
            scenery: None,
            asker: None,
            moves: Vec::new(),
            question: None,
            unavailable: None,
            hand: None,
            preview: None,
            role: None,
        }
    }

    fn day_card() -> ProjectionCommand {
        ProjectionCommand {
            role: Some(CommandRole::PassesTime),
            ..command(DAY, "Let the day pass")
        }
    }

    fn answer(question: &Question, id: &str, title: &str, asker: SelectionId) -> ProjectionCommand {
        ProjectionCommand {
            question: Some(question.clone()),
            asker: Some(asker),
            ..command(id, title)
        }
    }

    /// The three-year town with only the day's card on offer.
    pub(crate) fn town() -> ProjectionSnapshot {
        let mut snapshot = crate::diorama::tests::real_1082();
        snapshot.briefing = None;
        snapshot.morning = None;
        snapshot.commands = vec![day_card()];
        snapshot
    }

    /// The town on a morning: two questions (the second the day's main
    /// one), the day's card, and a morning with all four of its parts.
    pub(crate) fn morning_town() -> ProjectionSnapshot {
        let mut snapshot = town();
        let someone = snapshot
            .canvas
            .items
            .iter()
            .find(|item| item.kind == world_projection::CanvasItemKind::Actor)
            .map(|item| item.id)
            .expect("someone in the town");
        let small = Question {
            id: "small".into(),
            prompt: "Paint the gate blue?".into(),
        };
        let main = Question {
            id: "main".into(),
            prompt: "Should the old boat be mended or sold?".into(),
        };
        snapshot.commands = vec![
            answer(&small, "small.yes", "Yes", someone),
            answer(&small, "small.no", "No", someone),
            answer(&main, "main.mend", "Mend it", someone),
            answer(&main, "main.sell", "Sell it", someone),
            day_card(),
        ];
        snapshot.morning = Some(Morning {
            moment: someone,
            letter: Some(Letter {
                from: someone,
                note: "I wanted to tell you: the beans you planted came up. Come by soon.".into(),
                moment: someone,
            }),
            question: Some(main),
            tended: Some(MorningLine {
                line: "Overnight, someone watered your herb bed by the mill.".into(),
                about: None,
            }),
            find: Some(MorningLine {
                line: "You found a brass button by the mill.".into(),
                about: None,
            }),
        });
        snapshot
    }

    /// A World whose days pass one at a time, counting how many it was
    /// asked to pass.
    struct Days {
        snapshot: ProjectionSnapshot,
        passed: Rc<Cell<u32>>,
        /// Every other command the World was asked to carry out.
        chosen: Rc<Cell<u32>>,
    }

    impl crate::ProjectionController for Days {
        fn snapshot(&self) -> ProjectionSnapshot {
            self.snapshot.clone()
        }

        fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, String> {
            if intent == ProjectionIntent::InvokeCommand(DAY.into()) {
                self.passed.set(self.passed.get() + 1);
                self.snapshot.world_time += 1;
                // A new day has no morning until a sitting begins on it.
                self.snapshot.morning = None;
            } else if matches!(intent, ProjectionIntent::InvokeCommand(_)) {
                self.chosen.set(self.chosen.get() + 1);
            }
            Ok(self.snapshot.clone())
        }
    }

    struct Opened {
        view: gpui::Entity<ProjectionView>,
        cx: VisualTestContext,
        passed: Rc<Cell<u32>>,
        spent: Rc<Cell<u32>>,
        chosen: Rc<Cell<u32>>,
    }

    /// A World window on `snapshot`, with `allowance` if any: what the
    /// World passed, and what the host wrote down as spent.
    fn open(
        cx: &mut TestAppContext,
        snapshot: ProjectionSnapshot,
        allowance: Option<Allowance>,
    ) -> Opened {
        open_read(cx, snapshot, allowance, None, |_| {})
    }

    /// As [`open`], with the morning already read (`read`, a moment's
    /// stable key) and what the host does once one is.
    fn open_read(
        cx: &mut TestAppContext,
        snapshot: ProjectionSnapshot,
        allowance: Option<Allowance>,
        read: Option<String>,
        on_read: impl Fn(&str) + 'static,
    ) -> Opened {
        let passed = Rc::new(Cell::new(0));
        let spent = Rc::new(Cell::new(0));
        let chosen = Rc::new(Cell::new(0));
        let (world_passed, host_spent, world_chose) =
            (passed.clone(), spent.clone(), chosen.clone());
        let window = cx.add_window(move |_, _| {
            let view = ProjectionView::controlled(Days {
                snapshot,
                passed: world_passed,
                chosen: world_chose,
            })
            .with_morning_read(read.as_deref(), on_read);
            let mut view = match allowance {
                Some(allowance) => view.with_allowance(allowance, move || {
                    host_spent.set(host_spent.get() + 1);
                }),
                None => view,
            };
            view.looking.opening = None;
            view
        });
        let view = window.root(cx).expect("the World");
        let cx = VisualTestContext::from_window(window.into(), cx);
        cx.simulate_resize(gpui::size(px(1100.0), px(800.0)));
        cx.run_until_parked();
        Opened {
            view,
            cx,
            passed,
            spent,
            chosen,
        }
    }

    fn press_enter(view: &gpui::Entity<ProjectionView>, cx: &mut VisualTestContext) {
        cx.update(|window, app| view.update(app, |view, cx| view.enter(window, cx)));
        cx.run_until_parked();
    }

    fn pass_day(view: &gpui::Entity<ProjectionView>, cx: &mut VisualTestContext) {
        view.update(cx, |view, cx| view.invoke_command(DAY.into(), cx));
        cx.run_until_parked();
    }

    #[gpui::test]
    fn a_spent_sitting_says_goodnight_and_the_world_sleeps_until_the_next(cx: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        let Opened {
            view,
            mut cx,
            passed,
            spent,
            ..
        } = open(cx, town(), Some(Allowance { left: 1, of: 1 }));
        let cx = &mut cx;
        assert!(cx.debug_bounds("bottom-card").is_some(), "the day's card");
        assert!(cx.debug_bounds("goodnight").is_none());
        view.read_with(cx, |view, _| {
            assert_eq!(view.rest(), crate::i18n::Rest::Waits)
        });
        pass_day(&view, cx);
        assert_eq!((passed.get(), spent.get()), (1, 1), "one day, written down");
        view.read_with(cx, |view, _| {
            assert!(view.goodnight_shown());
            assert_eq!(
                view.rest(),
                crate::i18n::Rest::HeadingHome,
                "never asleep while the town is still up"
            );
            assert!(!view.asleep());
            assert_eq!(view.allowance(), Some(Allowance { left: 0, of: 1 }));
        });
        assert!(cx.debug_bounds("goodnight").is_some(), "the Goodnight card");
        assert!(cx.debug_bounds("bottom-card").is_none(), "no card under it");
        // "Goodnight": the World sleeps, at night, everyone home.
        let sleep = cx.debug_bounds("goodnight-sleep").expect("Goodnight");
        cx.simulate_click(sleep.center(), Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert!(view.asleep());
            assert_eq!(
                view.rest(),
                crate::i18n::Rest::Asleep,
                "asleep once it is said"
            );
        });
        assert!(cx.debug_bounds("asleep").is_some(), "asleep");
        assert!(cx.debug_bounds("goodnight").is_none());
        assert!(cx.debug_bounds("bottom-card").is_none(), "no card asleep");
        assert_eq!(crate::scene::pinned_hour(), Some(ASLEEP_HOUR));
        // Nothing lets another day pass.
        pass_day(&view, cx);
        assert_eq!(
            (passed.get(), spent.get()),
            (1, 1),
            "no day past the allowance"
        );
        // Let go (as when the window closes), the clock has the sky again.
        view.update(cx, |view, _| view.looking.sitting.asleep = None);
        assert!(crate::scene::pinned_hour().is_none());
        crate::scene::set_clock(None);
    }

    #[gpui::test]
    fn a_first_sitting_passes_three_days_then_says_goodnight(cx: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        let Opened {
            view,
            mut cx,
            passed,
            spent,
            ..
        } = open(cx, town(), Some(Allowance { left: 3, of: 3 }));
        let cx = &mut cx;
        for day in 1..=3 {
            view.read_with(cx, |view, _| assert!(!view.goodnight_shown(), "day {day}"));
            pass_day(&view, cx);
        }
        assert_eq!((passed.get(), spent.get()), (3, 3));
        view.read_with(cx, |view, _| assert!(view.goodnight_shown()));
        // Choosing the day's card again only brings the card up again.
        view.update(cx, |view, _| view.looking.sitting.goodnight = false);
        pass_day(&view, cx);
        assert_eq!(passed.get(), 3);
        view.read_with(cx, |view, _| assert!(view.goodnight_shown()));
        crate::scene::set_clock(None);
    }

    #[gpui::test]
    fn a_sitting_with_nothing_left_opens_asleep(cx: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        let Opened {
            view,
            mut cx,
            passed,
            ..
        } = open(cx, town(), Some(Allowance { left: 0, of: 1 }));
        let cx = &mut cx;
        view.read_with(cx, |view, _| {
            assert!(view.asleep());
            assert_eq!(view.rest(), crate::i18n::Rest::Asleep, "opened asleep");
            assert!(!view.goodnight_shown());
        });
        assert!(cx.debug_bounds("asleep").is_some());
        pass_day(&view, cx);
        assert_eq!(passed.get(), 0);
        view.update(cx, |view, _| view.looking.sitting.asleep = None);
        crate::scene::set_clock(None);
    }

    #[gpui::test]
    fn without_an_allowance_days_pass_as_they_always_have(cx: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        let Opened {
            view,
            mut cx,
            passed,
            spent,
            ..
        } = open(cx, town(), None);
        let cx = &mut cx;
        for _ in 0..5 {
            pass_day(&view, cx);
        }
        assert_eq!((passed.get(), spent.get()), (5, 0));
        view.read_with(cx, |view, _| {
            assert!(!view.goodnight_shown() && !view.asleep());
        });
        assert!(cx.debug_bounds("bottom-card").is_some());
        assert!(crate::scene::pinned_hour().is_none());
        crate::scene::set_clock(None);
    }

    #[gpui::test]
    fn the_morning_is_read_once_and_its_question_leads_the_cards(cx: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        let snapshot = morning_town();
        // The day's main question leads the cards, though asked second.
        let order = super::super::world_window::card_order(&snapshot);
        assert_eq!(snapshot.commands[order[0][0]].id, "main.mend");
        let Opened { view, mut cx, .. } = open(cx, snapshot, Some(Allowance { left: 1, of: 1 }));
        let cx = &mut cx;
        for part in [
            "morning",
            "morning-letter",
            "morning-question",
            "morning-tended",
            "morning-find",
        ] {
            assert!(cx.debug_bounds(part).is_some(), "{part}");
        }
        assert!(
            cx.debug_bounds("bottom-card").is_none(),
            "the morning first"
        );
        let begin = cx.debug_bounds("morning-begin").expect("Begin the day");
        cx.simulate_click(begin.center(), Modifiers::none());
        cx.run_until_parked();
        assert!(cx.debug_bounds("morning").is_none(), "read once");
        view.read_with(cx, |view, _| {
            let card = view.card_command().expect("a card in front");
            assert_eq!(
                card.question.as_ref().map(|question| question.id.as_str()),
                Some("main"),
                "the day's question in front"
            );
            assert!(is_days_question(&view.snapshot, card));
        });
        crate::scene::set_clock(None);
    }

    #[gpui::test]
    fn enter_chooses_nothing_under_the_morning_the_goodnight_or_the_sleep(cx: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        let Opened {
            view,
            mut cx,
            passed,
            chosen,
            ..
        } = open(cx, morning_town(), Some(Allowance { left: 1, of: 1 }));
        let cx = &mut cx;
        // Under the morning: the day's question is in front, and hidden.
        press_enter(&view, cx);
        assert_eq!(chosen.get(), 0, "nothing chosen under the morning");
        view.update(cx, |view, cx| {
            view.begin_the_day();
            cx.notify();
        });
        cx.run_until_parked();
        // Shown, Enter answers it (the scene's own way of choosing).
        press_enter(&view, cx);
        assert_eq!(chosen.get(), 1, "the card in front, once shown");
        // The day passes; under the Goodnight card and asleep, nothing.
        pass_day(&view, cx);
        assert_eq!(passed.get(), 1);
        view.read_with(cx, |view, _| assert!(view.goodnight_shown()));
        press_enter(&view, cx);
        assert_eq!(chosen.get(), 1, "nothing chosen under the Goodnight card");
        view.update(cx, |view, cx| {
            view.fall_asleep();
            cx.notify();
        });
        cx.run_until_parked();
        press_enter(&view, cx);
        assert_eq!(chosen.get(), 1, "nothing chosen asleep");
        assert_eq!(passed.get(), 1);
        view.update(cx, |view, _| view.looking.sitting.asleep = None);
        crate::scene::set_clock(None);
    }

    #[gpui::test]
    fn a_morning_read_is_written_down_and_not_shown_again(app: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        let moment = morning_town().morning.unwrap().moment.stable_key();
        let read = Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
        let kept = read.clone();
        let Opened { view, mut cx, .. } = open_read(
            app,
            morning_town(),
            Some(Allowance { left: 1, of: 1 }),
            None,
            move |key| kept.borrow_mut().push(key.to_string()),
        );
        let cx = &mut cx;
        assert!(cx.debug_bounds("morning").is_some());
        let begin = cx.debug_bounds("morning-begin").expect("Begin the day");
        cx.simulate_click(begin.center(), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(*read.borrow(), vec![moment.clone()], "written down once");
        drop(view);
        // Opened again that day, the morning read before is not shown.
        let Opened { mut cx, .. } = open_read(
            app,
            morning_town(),
            Some(Allowance { left: 1, of: 1 }),
            Some(moment),
            |_| {},
        );
        assert!(cx.debug_bounds("morning").is_none());
        assert!(cx.debug_bounds("bottom-card").is_some());
        crate::scene::set_clock(None);
    }

    #[test]
    fn a_view_rebuilt_asleep_keeps_the_night_until_the_last_lets_go() {
        let old = Asleep::hold();
        assert_eq!(crate::scene::pinned_hour(), Some(ASLEEP_HOUR));
        // The page asked again: the new view sleeps before the old goes.
        let new = Asleep::hold();
        drop(old);
        assert_eq!(
            crate::scene::pinned_hour(),
            Some(ASLEEP_HOUR),
            "still night"
        );
        drop(new);
        assert_eq!(crate::scene::pinned_hour(), None);
    }

    #[test]
    fn the_asleep_note_reads_at_four_and_a_half_to_one_on_the_night_paper() {
        fn luminance(colour: gpui::Rgba) -> f32 {
            let channel = |c: f32| {
                if c <= 0.039_28 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(colour.r) + 0.7152 * channel(colour.g) + 0.0722 * channel(colour.b)
        }
        let ink = asleep_ink();
        let night = crate::scene::daylight_at(ASLEEP_HOUR);
        let paper = super::super::world_window::paper_at(night, false);
        let (a, b) = (luminance(ink), luminance(paper));
        let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
        assert!(ratio >= 4.5, "{ratio}");
    }
}
