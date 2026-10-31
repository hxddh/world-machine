//! A sitting, in the window (v0.30 S4, "one day a day"). The host says how
//! many of the World's days this sitting may still pass ([`Allowance`]),
//! and writes down each one passed; the window keeps the rest, as
//! presentation only:
//!
//! - the morning a new sitting opens on, from the World's own record (its
//!   letter, its main question, what was tended overnight, a find), read
//!   once and put away with "Begin the day";
//! - the day's main question weighted, first among the cards;
//! - once the sitting has passed every day it may, a Goodnight card; put
//!   off with "Not yet", the evening is the player's for as long as they
//!   like (looking around, talking, building), with no card that passes a
//!   day, until they say Goodnight (1.0 P2: the day is not a timer);
//! - then the World asleep: dimmed, the sky held at night with everyone
//!   home, until the next sitting opens it again. Asleep is not locked
//!   (1.0 P2): the drawer (story, letters, finds, the book), a photo, a
//!   postcard and the town at night stay open; only passing a day waits;
//! - with "Play freely" chosen in Settings, Goodnight offers "Sleep now",
//!   and the host begins the next sitting at once.
//!
//! None of it is World truth: the World only ever hears that a day passed,
//! as it always has, from the day's own card.

use super::world_window::{first_name, label_of, scene_paper};
use super::ProjectionView;
use crate::ui::{self, ButtonKind};
use gpui::{
    div, prelude::FluentBuilder, px, Context, Div, FontWeight, InteractiveElement, ParentElement,
    Role, StatefulInteractiveElement, Styled,
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

/// What a host does when the player chooses "Sleep now" (with "Play
/// freely" on): begins the next sitting at once, and opens the World on
/// it. Called once the view has fallen asleep, outside its own update.
pub type SleepNow = std::rc::Rc<dyn Fn(&mut gpui::App)>;

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
    /// What the host does for "Sleep now"; `None` where it cannot begin a
    /// sitting at once, and Goodnight is all there is.
    pub(crate) sleep_now: Option<SleepNow>,
}

/// What a host does once a morning (by its moment's stable key) is read.
pub type ReadMorning = std::rc::Rc<dyn Fn(&str)>;

impl Sitting {
    /// Whether the sitting has passed every day it may.
    pub(crate) fn spent(&self) -> bool {
        self.allowance.is_some_and(|allowance| allowance.left == 0)
    }
}

/// The World asleep: its view draws its own scene at night
/// ([`ProjectionView::held_hour`]), every other window keeping the clock.
pub(crate) struct Asleep;

impl Asleep {
    fn hold() -> Self {
        Asleep
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

    /// What to do for "Sleep now", offered on Goodnight while the player
    /// plays freely: the host begins the next sitting at once.
    pub fn with_sleep_now(mut self, sleep_now: impl Fn(&mut gpui::App) + 'static) -> Self {
        self.looking.sitting.sleep_now = Some(std::rc::Rc::new(sleep_now));
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
    /// Goodnight card is up or the evening is played out, asleep once
    /// Goodnight is said (or opened asleep), and otherwise waiting for the
    /// player.
    pub fn rest(&self) -> crate::i18n::Rest {
        use crate::i18n::Rest;
        if self.looking.sitting.goodnight || self.evening() {
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

    /// Whether the evening is being played out: the sitting's days all
    /// passed, the Goodnight card put off, and the World not yet asleep.
    pub fn evening(&self) -> bool {
        let sitting = &self.looking.sitting;
        sitting.spent() && !sitting.goodnight && sitting.asleep.is_none()
    }

    /// Whether "Sleep now" is offered: the player plays freely, and the
    /// host can begin the next sitting at once.
    pub(crate) fn sleep_now_offered(&self) -> bool {
        crate::ui::play_freely() && self.looking.sitting.sleep_now.is_some()
    }

    /// "Not yet": the Goodnight card put away, and the evening the
    /// player's for as long as they like. Nothing in it passes a day.
    pub(crate) fn stay_up(&mut self) {
        self.looking.sitting.goodnight = false;
    }

    /// Goodnight said, from its card (or Enter on it): the World sleeps;
    /// playing freely, the next sitting begins at once.
    pub(crate) fn goodnight_said(&mut self, cx: &mut Context<Self>) {
        if self.sleep_now_offered() {
            self.sleep_now(cx);
        } else {
            self.fall_asleep();
        }
        cx.notify();
    }

    /// "Sleep now": the World falls asleep, and the host begins the next
    /// sitting at once. Only once the sitting has passed every day it may.
    pub(crate) fn sleep_now(&mut self, cx: &mut Context<Self>) {
        if !self.looking.sitting.spent() {
            return;
        }
        self.fall_asleep();
        if let Some(sleep_now) = self.looking.sitting.sleep_now.clone() {
            cx.defer(move |cx| sleep_now(cx));
        }
        cx.notify();
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
    pub(crate) fn say_goodnight(&mut self) {
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

    /// Whether the sitting covers the place: the morning or the Goodnight
    /// card. Nothing under it is chosen. A World asleep covers nothing
    /// (1.0 P2): it is the town at night, its drawer and pictures open.
    pub(crate) fn sitting_covers(&self) -> bool {
        self.looking.sitting.goodnight || self.morning_shown().is_some()
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

    /// What the sitting shows over the place, when it has the place to
    /// itself: the Goodnight card, or the morning.
    pub(crate) fn render_sitting(&self, cx: &mut Context<Self>) -> Option<Div> {
        if self.looking.sitting.goodnight {
            return Some(self.render_goodnight(cx));
        }
        self.render_morning(cx)
    }

    /// The World asleep, over the scene and under the handles: the night's
    /// veil and its note.
    pub(crate) fn render_night(&self, cx: &mut Context<Self>) -> Option<Div> {
        self.looking.sitting.asleep.as_ref()?;
        Some(self.render_asleep(cx))
    }

    /// The evening's card, where the day's cards stood: the day is done,
    /// and Goodnight is the player's to say.
    pub(crate) fn render_evening(&self, cx: &mut Context<Self>) -> Option<Div> {
        if !self.evening() || self.controller.is_none() {
            return None;
        }
        let card = div()
            .id("evening")
            .role(Role::Group)
            .aria_label(ui::t("Evening"))
            .debug_selector(|| "evening".into())
            .px_4()
            .py_3()
            .rounded_2xl()
            .bg(scene_paper())
            .shadow_lg()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(ui::row_title("Evening"))
                    .child(ui::caption(
                        "The day is done. Look around, talk and build as long as you like.",
                    )),
            )
            .child(
                ui::button("evening-goodnight", "Goodnight", ButtonKind::Primary)
                    .debug_selector(|| "evening-goodnight".into())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.say_goodnight();
                        cx.notify();
                    })),
            );
        Some(div().child(card))
    }

    fn render_goodnight(&self, cx: &mut Context<Self>) -> Div {
        let freely = self.sleep_now_offered();
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
            .child(ui::body(if freely {
                "Everyone's heading home. Sleep now and the next morning comes at once: a letter, today's choice, something looked after, and a find for your book."
            } else {
                "Everyone's heading home. Come back tomorrow for the morning: a letter, today's choice, something looked after, and a find for your book."
            }))
            .child(ui::caption(
                "No hurry. The town will wait for you.",
            ))
            .child(
                div()
                    .pt_2()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        ui::button("goodnight-not-yet", "Not yet", ButtonKind::Secondary)
                            .debug_selector(|| "goodnight-not-yet".into())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.stay_up();
                                cx.notify();
                            })),
                    )
                    .child(
                        ui::button(
                            "goodnight-sleep",
                            if freely { "Sleep now" } else { "Goodnight" },
                            ButtonKind::Primary,
                        )
                        .debug_selector(|| "goodnight-sleep".into())
                        .on_click(cx.listener(|this, _, _, cx| this.goodnight_said(cx))),
                    ),
            );
        hanging(paper)
    }

    fn render_asleep(&self, cx: &mut Context<Self>) -> Div {
        let freely = self.sleep_now_offered();
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
            .child(ui::row_title(if freely {
                "Asleep"
            } else {
                "Asleep until tomorrow"
            }))
            .child(
                div()
                    .text_xs()
                    .text_color(asleep_ink())
                    .child(crate::wrap::text(ui::t(if freely {
                        "Everyone's home. Sleep now and the next morning comes at once."
                    } else {
                        "Everyone's home. Come back tomorrow for the morning; the book, letters and pictures are still yours."
                    }))),
            )
            .when(freely, |note| {
                note.child(
                    div().pt_2().child(
                        ui::button("asleep-sleep-now", "Sleep now", ButtonKind::Primary)
                            .debug_selector(|| "asleep-sleep-now".into())
                            .on_click(cx.listener(|this, _, _, cx| this.sleep_now(cx))),
                    ),
                )
            });
        // The place dimmed under the night it is held at (lightly, so the
        // night's own blues still show; and never taking a click, so the
        // town at night stays open), and the note at the foot of the
        // place, where the day's card stood, clear of the handles above.
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
                    .bottom_0()
                    .px_6()
                    .pb_4()
                    .flex()
                    .justify_center()
                    .child(note.occlude()),
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
            // One height every morning, however much it brings, so "Begin
            // the day" is always in the same place (v0.31: it moved with
            // the card). In rems, so larger text gets a taller paper; what
            // does not fit scrolls.
            .h(gpui::rems(MORNING_REMS))
            // Never taller than the stage, so "Begin the day" stays on
            // screen in a small window with large text; the parts scroll.
            .max_h_full()
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
            .child(
                div()
                    .id("morning-parts")
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .child(parts),
            )
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

/// How tall the morning's paper stands, in rems: room for a letter of
/// three lines and the other three parts, with "Begin the day" under them.
const MORNING_REMS: f32 = 27.0;

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
        .bottom_0()
        .px_6()
        .py_3()
        .flex()
        .justify_center()
        .items_start()
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
        view.read_with(cx, |view, _| {
            assert_eq!(view.held_hour(), Some(ASLEEP_HOUR));
            assert_eq!(
                view.looking.drawn_at_hour,
                Some(ASLEEP_HOUR),
                "drawn at night"
            );
        });
        assert!(
            crate::scene::pinned_hour().is_none(),
            "no other window's sky"
        );
        // Nothing lets another day pass.
        pass_day(&view, cx);
        assert_eq!(
            (passed.get(), spent.get()),
            (1, 1),
            "no day past the allowance"
        );
        // Woken (a new sitting), the view has the clock's sky again.
        view.update(cx, |view, _| view.looking.sitting.asleep = None);
        view.read_with(cx, |view, _| assert_eq!(view.held_hour(), None));
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

    /// Enter presses the sitting's one button (v0.31 play: a keyboard
    /// player was stuck under "Begin the day" and under Goodnight): on the
    /// morning it begins the day, choosing nothing under it; on the
    /// Goodnight card the World sleeps. And "Begin the day" stands in the
    /// same place on a morning of four parts and on one of a single line.
    #[gpui::test]
    fn enter_begins_the_day_and_says_goodnight_from_a_button_that_stays_put(
        app: &mut TestAppContext,
    ) {
        crate::scene::set_clock(Some(12));
        let Opened {
            view,
            mut cx,
            passed,
            chosen,
            ..
        } = open(app, morning_town(), Some(Allowance { left: 1, of: 1 }));
        let cx = &mut cx;
        let full = cx.debug_bounds("morning-begin").expect("Begin the day");
        press_enter(&view, cx);
        assert_eq!(chosen.get(), 0, "nothing chosen under the morning");
        view.read_with(cx, |view, _| assert!(view.morning_shown().is_none()));
        assert!(cx.debug_bounds("morning").is_none(), "the morning put away");
        assert!(cx.debug_bounds("bottom-card").is_some(), "the day's card");
        pass_day(&view, cx);
        assert_eq!(passed.get(), 1);
        assert!(cx.debug_bounds("goodnight").is_some());
        press_enter(&view, cx);
        view.read_with(cx, |view, _| {
            assert!(view.asleep(), "Enter says Goodnight");
            assert!(!view.goodnight_shown());
        });
        press_enter(&view, cx);
        assert_eq!((passed.get(), chosen.get()), (1, 0), "nothing asleep");
        view.update(cx, |view, _| view.looking.sitting.asleep = None);
        // A morning of one short letter: the button where it was.
        let mut short = morning_town();
        if let Some(morning) = short.morning.as_mut() {
            morning.question = None;
            morning.tended = None;
            morning.find = None;
            if let Some(letter) = morning.letter.as_mut() {
                letter.note = "Hello.".into();
            }
        }
        let Opened { mut cx, .. } = open(app, short, Some(Allowance { left: 1, of: 1 }));
        let one = cx.debug_bounds("morning-begin").expect("Begin the day");
        assert_eq!(one, full, "Begin the day stays put");
        crate::scene::set_clock(None);
    }

    /// A chapter that ended overnight never takes the morning's Enter
    /// (v0.31 release shots: on the first sitting's festival morning Enter
    /// marked the chapter read under the paper, and the morning stayed):
    /// Enter begins the day, and the chapter's card is there to read after.
    #[gpui::test]
    fn enter_begins_the_day_with_a_chapter_just_ended(app: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        let mut snapshot = morning_town();
        let moment = SelectionId::from_stable_key("event-900").unwrap();
        snapshot
            .timeline
            .items
            .push(world_projection::TimelineItem {
                id: moment,
                world_time: snapshot.world_time,
                title: "Spring Clean".into(),
                subtitle: String::new(),
                caused_by: Vec::new(),
                routine: false,
            });
        snapshot.chapters.push(world_projection::Chapter {
            number: 1,
            title: "The first spring".into(),
            summary: "The place came together.".into(),
            moment: Some(moment),
        });
        let Opened {
            view,
            mut cx,
            chosen,
            ..
        } = open(app, snapshot, Some(Allowance { left: 1, of: 1 }));
        let cx = &mut cx;
        assert!(cx.debug_bounds("morning").is_some(), "the morning is up");
        press_enter(&view, cx);
        assert!(cx.debug_bounds("morning").is_none(), "Enter began the day");
        assert_eq!(chosen.get(), 0, "nothing chosen under the morning");
        view.read_with(cx, |view, _| {
            assert!(
                super::super::world_window::chapter_just_ended(
                    &view.snapshot,
                    view.looking.chapter_read
                )
                .is_some(),
                "the chapter is still there to read"
            )
        });
        press_enter(&view, cx);
        view.read_with(cx, |view, _| {
            assert_eq!(view.looking.chapter_read, Some(1), "then read with Enter")
        });
        crate::scene::set_clock(None);
    }

    /// A moment that came overnight never takes the morning's Enter
    /// (v0.31 release shots: on the first sitting's fifth morning Enter put
    /// away the strip waiting under the paper, unseen, and the morning
    /// stayed): Enter begins the day, and the strip is still up after.
    #[gpui::test]
    fn enter_begins_the_day_with_a_moment_waiting_under_it(app: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        let snapshot = morning_town();
        let moment = world_projection::Moment {
            id: "moment-1".into(),
            title: "Spring Clean".into(),
            ..Default::default()
        };
        let Opened { view, mut cx, .. } = open(app, snapshot, Some(Allowance { left: 1, of: 1 }));
        let cx = &mut cx;
        view.update(cx, |view, cx| {
            view.reading.shown = Some((moment, std::time::Instant::now()));
            cx.notify();
        });
        cx.run_until_parked();
        assert!(cx.debug_bounds("morning").is_some(), "the morning is up");
        // The key as the window gets it, through its key handler.
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        assert!(cx.debug_bounds("morning").is_none(), "Enter began the day");
        view.read_with(cx, |view, _| {
            assert!(view.moment_up(), "the moment is still there to see")
        });
        crate::scene::set_clock(None);
    }

    /// With text at 200% in an 800-high window, the morning's paper is
    /// never taller than the stage: "Begin the day" stays on screen to
    /// click, and what does not fit scrolls.
    #[gpui::test]
    fn begin_the_day_is_on_screen_with_the_largest_text(app: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        crate::with_text_scale(200, || {
            let Opened { mut cx, .. } =
                open(app, morning_town(), Some(Allowance { left: 1, of: 1 }));
            let begin = cx.debug_bounds("morning-begin").expect("Begin the day");
            assert!(
                begin.bottom() <= px(800.0),
                "Begin the day below the window: {begin:?}"
            );
        });
        crate::scene::set_clock(None);
    }

    /// The word that a postcard was saved is never under a card (v0.31
    /// play: it showed behind the day's card): with the day's card up, and
    /// under the morning's paper, it is shown clear of the card.
    #[gpui::test]
    fn the_saved_word_is_never_hidden_by_a_card(app: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        for snapshot in [town(), morning_town()] {
            let Opened { view, mut cx, .. } = open(app, snapshot, None);
            let cx = &mut cx;
            view.update(cx, |view, cx| {
                view.looking.postcard_saved = Some(std::time::Instant::now());
                cx.notify();
            });
            cx.run_until_parked();
            let toast = cx.debug_bounds("saved-toast").expect("the saved word");
            for name in ["bottom-card", "morning"] {
                if let Some(card) = cx.debug_bounds(name) {
                    assert!(!toast.intersects(&card), "{name}: {toast:?} and {card:?}");
                }
            }
        }
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

    #[gpui::test]
    fn one_window_asleep_leaves_another_at_its_own_hour(cx: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        let Opened {
            view: asleep,
            cx: mut asleep_cx,
            ..
        } = open(cx, town(), Some(Allowance { left: 0, of: 1 }));
        let Opened {
            view: awake,
            cx: mut awake_cx,
            ..
        } = open(cx, town(), None);
        // Both drawn again, in either order: each at its own hour.
        for _ in 0..2 {
            asleep.update(&mut asleep_cx, |_, cx| cx.notify());
            asleep_cx.run_until_parked();
            awake.update(&mut awake_cx, |_, cx| cx.notify());
            awake_cx.run_until_parked();
        }
        asleep.read_with(&asleep_cx, |view, _| {
            assert_eq!(view.looking.drawn_at_hour, Some(ASLEEP_HOUR))
        });
        awake.read_with(&awake_cx, |view, _| {
            assert_eq!(view.held_hour(), None);
            assert_eq!(view.looking.drawn_at_hour, Some(12), "the clock's own noon");
        });
        assert!(
            awake_cx.debug_bounds("bottom-card").is_some(),
            "awake, with its card"
        );
        assert!(crate::scene::pinned_hour().is_none());
        crate::scene::set_clock(None);
    }

    /// The evening is the player's (1.0 P2): "Not yet" puts the Goodnight
    /// card away, and the place stays open (the drawer, people, what the
    /// hands make) with no card that passes a day, until the player says
    /// Goodnight. Nothing says it for them.
    #[gpui::test]
    fn not_yet_plays_the_evening_out_until_goodnight_is_chosen(app: &mut TestAppContext) {
        crate::scene::set_clock(Some(20));
        let Opened {
            view,
            mut cx,
            passed,
            spent,
            chosen,
        } = open(app, morning_town(), Some(Allowance { left: 1, of: 1 }));
        let cx = &mut cx;
        view.update(cx, |view, cx| {
            view.begin_the_day();
            cx.notify();
        });
        pass_day(&view, cx);
        assert!(cx.debug_bounds("goodnight").is_some());
        let not_yet = cx.debug_bounds("goodnight-not-yet").expect("Not yet");
        cx.simulate_click(not_yet.center(), Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert!(view.evening() && !view.goodnight_shown() && !view.asleep());
            assert_eq!(view.rest(), crate::i18n::Rest::HeadingHome);
            assert!(view.card_command().is_none(), "no World card at evening");
        });
        assert!(cx.debug_bounds("goodnight").is_none());
        assert!(cx.debug_bounds("evening").is_some(), "the evening's card");
        assert!(cx.debug_bounds("photo-handle").is_some(), "the handles");
        // Time goes by, and nothing says Goodnight but the player.
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(3600));
        cx.run_until_parked();
        view.read_with(cx, |view, _| assert!(view.evening()));
        // Looking around: the drawer opens, and what passes no day is done.
        view.update(cx, |view, cx| view.toggle_drawer(cx));
        cx.run_until_parked();
        assert!(cx.debug_bounds("drawer-leaves").is_some(), "the drawer");
        view.update(cx, |view, cx| view.toggle_drawer(cx));
        view.update(cx, |view, cx| view.invoke_command("small.yes".into(), cx));
        cx.run_until_parked();
        assert_eq!(chosen.get(), 1, "a deed of the evening");
        // The day's card passes no day: the Goodnight card comes back.
        pass_day(&view, cx);
        assert_eq!((passed.get(), spent.get()), (1, 1));
        view.read_with(cx, |view, _| assert!(view.goodnight_shown()));
        view.update(cx, |view, _| view.stay_up());
        cx.run_until_parked();
        // Enter on the evening's card brings up the Goodnight card; Enter
        // on that says it.
        press_enter(&view, cx);
        view.read_with(cx, |view, _| assert!(view.goodnight_shown()));
        press_enter(&view, cx);
        view.read_with(cx, |view, _| assert!(view.asleep()));
        assert_eq!((passed.get(), spent.get()), (1, 1));
        view.update(cx, |view, _| view.looking.sitting.asleep = None);
        crate::scene::set_clock(None);
    }

    /// Asleep is not locked (1.0 P2): the drawer (letters, finds, the
    /// book), a photo, a postcard and the town at night stay open; only
    /// passing a day waits, and Enter passes none.
    #[gpui::test]
    fn asleep_the_drawer_and_pictures_stay_open_and_no_day_passes(app: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        let Opened {
            view,
            mut cx,
            passed,
            chosen,
            ..
        } = open(app, morning_town(), Some(Allowance { left: 0, of: 1 }));
        let cx = &mut cx;
        view.read_with(cx, |view, _| {
            assert!(view.asleep());
            assert_eq!(view.held_hour(), Some(ASLEEP_HOUR), "the town at night");
        });
        assert!(cx.debug_bounds("asleep").is_some(), "the asleep note");
        assert!(cx.debug_bounds("morning").is_none());
        assert!(cx.debug_bounds("bottom-card").is_none(), "no day's card");
        for handle in ["photo-handle", "postcard-handle"] {
            assert!(cx.debug_bounds(handle).is_some(), "{handle}");
        }
        assert!(
            cx.debug_bounds("asleep-sleep-now").is_none(),
            "no Sleep now unless playing freely"
        );
        let drawer = cx
            .debug_bounds(
                format!(
                    "named: {}",
                    ui::t("The drawer: story, letters, keepsakes and the book (⌘I)")
                )
                .leak(),
            )
            .expect("the drawer's handle");
        cx.simulate_click(drawer.center(), Modifiers::none());
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("drawer-leaves").is_some(),
            "the drawer opens"
        );
        view.read_with(cx, |view, _| assert!(view.looking.drawer));
        view.update(cx, |view, cx| view.toggle_drawer(cx));
        cx.run_until_parked();
        // Nobody is asked, nothing is made, and no day passes.
        let someone = morning_town().morning.unwrap().letter.unwrap().from;
        view.update(cx, |view, cx| view.ask(someone, cx));
        cx.simulate_keystrokes("h");
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert!(view.looking.asking.is_none() && view.looking.hands.is_none())
        });
        press_enter(&view, cx);
        cx.simulate_keystrokes("enter");
        pass_day(&view, cx);
        view.update(cx, |view, cx| view.invoke_command("small.yes".into(), cx));
        cx.run_until_parked();
        assert_eq!((passed.get(), chosen.get()), (0, 0), "nothing asleep");
        view.read_with(cx, |view, _| assert!(view.asleep()));
        view.update(cx, |view, _| view.looking.sitting.asleep = None);
        crate::scene::set_clock(None);
    }

    /// Playing freely (1.0 P2): Goodnight offers "Sleep now", which puts
    /// the World to sleep and asks the host for the next sitting at once;
    /// asleep, the note offers it too. Not playing freely, Goodnight is as
    /// it was, and the host is asked for nothing.
    #[gpui::test]
    fn playing_freely_goodnight_offers_sleep_now(app: &mut TestAppContext) {
        crate::scene::set_clock(Some(12));
        for freely in [true, false] {
            crate::ui::set_thread_play_freely(Some(freely));
            let Opened {
                view,
                mut cx,
                passed,
                ..
            } = open(app, town(), Some(Allowance { left: 1, of: 1 }));
            let cx = &mut cx;
            let slept = Rc::new(Cell::new(0));
            let asked = slept.clone();
            view.update(cx, |view, _| {
                view.looking.sitting.sleep_now = Some(Rc::new(move |_| asked.set(asked.get() + 1)));
            });
            // Not before the day is done.
            view.update(cx, |view, cx| view.sleep_now(cx));
            cx.run_until_parked();
            assert_eq!(slept.get(), 0, "a day still to play");
            view.read_with(cx, |view, _| assert!(!view.asleep()));
            pass_day(&view, cx);
            view.read_with(cx, |view, _| {
                assert!(view.goodnight_shown());
                assert_eq!(view.sleep_now_offered(), freely);
            });
            let sleep = cx.debug_bounds("goodnight-sleep").expect("its button");
            cx.simulate_click(sleep.center(), Modifiers::none());
            cx.run_until_parked();
            view.read_with(cx, |view, _| assert!(view.asleep()));
            assert_eq!(slept.get(), u32::from(freely), "freely: {freely}");
            assert_eq!(passed.get(), 1, "no day passed by sleeping");
            // Asleep, playing freely, the note offers it again; Enter
            // never asks for it.
            press_enter(&view, cx);
            assert_eq!(slept.get(), u32::from(freely));
            match cx.debug_bounds("asleep-sleep-now") {
                Some(again) => {
                    assert!(freely);
                    cx.simulate_click(again.center(), Modifiers::none());
                    cx.run_until_parked();
                    assert_eq!(slept.get(), 2);
                }
                None => assert!(!freely),
            }
            view.update(cx, |view, _| view.looking.sitting.asleep = None);
        }
        crate::ui::set_thread_play_freely(None);
        crate::scene::set_clock(None);
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
