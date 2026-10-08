//! A World's farewell: the place at dusk, the player's own moments
//! recapped from what the World recorded, a resident's goodbye over their
//! head, and a postcard to keep. Nothing else is shown with it: no card,
//! no chapter's end, no pointers. The host writes it (the demo's ending);
//! the window only shows it.

use super::world_window::{likeness_of, portrait, scene_paper};
use super::ProjectionView;
use crate::ui::{self, ButtonKind};
use gpui::{
    div, px, Context, Div, InteractiveElement, ParentElement, Role, StatefulInteractiveElement,
    Styled, Window,
};
use world_projection::SelectionId;
use world_theme::tokens;

/// What a host does when a button of the farewell is pressed.
pub type FarewellAction = std::rc::Rc<dyn Fn(&mut Window, &mut gpui::App)>;

/// A farewell, in the host's words (the window shows each through the
/// app's catalogs) and the World's own (the recap, already in the player's
/// language).
pub struct Farewell {
    pub title: String,
    /// What the recap is headed with: "What you did here".
    pub recap_title: String,
    /// The player's own moments, 4 to 6 of them, oldest first.
    pub recap: Vec<String>,
    /// Who says goodbye, and what they say.
    pub goodbye: Option<(SelectionId, String)>,
    pub body: String,
    pub kept: String,
    /// "Keep a postcard": a postcard of the place at dusk, saved.
    pub postcard: String,
    /// The way on, if the host offers one: "About the full app".
    pub more: Option<(String, FarewellAction)>,
    /// "Stay a while": the farewell is put away and the clock let go.
    pub stay: String,
    /// The hour the sky is held at while it shows.
    pub hour: u32,
}

/// A farewell let go (put away, or its window closed) lets the clock have
/// the sky again.
impl Drop for Farewell {
    fn drop(&mut self) {
        crate::scene::hold_hour(None);
    }
}

impl ProjectionView {
    /// Shows `farewell` over the World: the sky held at its hour, every
    /// card and pointer put away (a chapter just ended is taken as read),
    /// and the camera on whoever says goodbye.
    pub fn show_farewell(&mut self, farewell: Farewell) {
        let hour = farewell.hour;
        self.looking.asking = None;
        self.looking.drawer = false;
        self.looking.hands = None;
        self.looking.pointer = None;
        self.looking.favour_beat = None;
        // A postcard kept before is not this farewell's.
        self.looking.postcard_saved = None;
        self.looking.postcard_failed = None;
        if let Some(chapter) = self.snapshot.chapters.last() {
            self.looking.chapter_read = Some(chapter.number);
        }
        self.looking.farewell_panned = false;
        self.looking.farewell_speaker = None;
        // Any farewell before it is let go first, then the sky is held.
        self.looking.farewell = Some(farewell);
        crate::scene::hold_hour(Some(hour));
    }

    /// Whether a farewell is showing.
    pub fn farewell_shown(&self) -> bool {
        self.looking.farewell.is_some()
    }

    /// Puts the farewell away and lets the clock have the sky again.
    pub fn end_farewell(&mut self) {
        self.looking.farewell = None;
    }

    /// Who says goodbye, and the line, while the farewell shows: whoever
    /// the host asked, or, if they are indoors at that hour, whoever is out
    /// on the place (see `world_window`'s farewell camera).
    pub(crate) fn farewell_line(&self) -> Option<(SelectionId, String)> {
        let (who, line) = self.looking.farewell.as_ref()?.goodbye.clone()?;
        Some((self.looking.farewell_speaker.unwrap_or(who), line))
    }

    /// The farewell's paper: the recap, the goodbye, and the ways on.
    pub(crate) fn render_farewell(&self, cx: &mut Context<Self>) -> Option<Div> {
        let farewell = self.looking.farewell.as_ref()?;
        let mut recap = div().flex().flex_col().gap_1();
        for line in &farewell.recap {
            recap = recap.child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div()
                            .mt(px(7.0))
                            .size(px(6.0))
                            .flex_shrink_0()
                            .rounded_full()
                            .bg(ui::color(tokens::ACCENT)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .text_sm()
                            .text_color(ui::color(tokens::TEXT))
                            .child(crate::wrap::text(ui::t(line.clone()))),
                    ),
            );
        }
        let goodbye = self.farewell_line().map(|(who, line)| {
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(portrait(likeness_of(&self.snapshot, who), 36.0, false))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .text_sm()
                        .italic()
                        .text_color(ui::color(tokens::TEXT))
                        .child(crate::i18n::quoted(&ui::t(line))),
                )
        });
        // Whether the postcard was kept, said on the paper itself: the
        // window's own word for it would sit under the farewell.
        let kept = match (self.looking.postcard_saved, self.looking.postcard_failed) {
            _ if self.looking.photographing => Some("Keeping the postcard…"),
            (Some(saved), Some(failed)) if failed > saved => Some("Couldn't save the postcard"),
            (None, Some(_)) => Some("Couldn't save the postcard"),
            (Some(_), _) => Some("Postcard saved to Pictures"),
            (None, None) => None,
        };
        let more = farewell.more.as_ref().map(|(label, action)| {
            let action = action.clone();
            ui::button("farewell-more", label.clone(), ButtonKind::Secondary)
                .on_click(move |_, window, cx| action(window, cx))
        });
        let paper = div()
            .id("farewell")
            .role(Role::Dialog)
            .aria_label(ui::t(farewell.title.clone()))
            .debug_selector(|| "farewell".into())
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
            .child(ui::heading(farewell.title.clone()))
            .child(ui::caption(farewell.recap_title.clone()))
            .child(recap)
            .children(goodbye)
            .child(ui::body(farewell.body.clone()))
            .child(ui::caption(farewell.kept.clone()))
            .children(kept.map(|kept| {
                div()
                    .id("farewell-postcard-kept")
                    .debug_selector(|| "farewell-postcard-kept".into())
                    .role(Role::Status)
                    .text_sm()
                    .text_color(ui::color(tokens::ACCENT_TEXT))
                    .child(ui::t(kept))
            }))
            .child(
                div()
                    .pt_2()
                    .flex()
                    .flex_wrap()
                    .justify_end()
                    .gap_2()
                    .children(more)
                    .child(
                        ui::button(
                            "farewell-stay",
                            farewell.stay.clone(),
                            ButtonKind::Secondary,
                        )
                        .debug_selector(|| "farewell-stay".into())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.end_farewell();
                            cx.notify();
                        })),
                    )
                    .child(
                        ui::button(
                            "farewell-postcard",
                            farewell.postcard.clone(),
                            ButtonKind::Primary,
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.take_postcard(window, cx)),
                        ),
                    ),
            );
        Some(
            div()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .p_6()
                .flex()
                .justify_center()
                .child(paper),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn farewell(goodbye: Option<(SelectionId, String)>) -> Farewell {
        Farewell {
            title: "This is where the demo ends".into(),
            recap_title: "What you did here".into(),
            recap: vec![
                "Rosa came because of the pottery you built.".into(),
                "Mara and Emma's party".into(),
                "A bread cart".into(),
                "A hand-drawn map of the harbour".into(),
            ],
            goodbye,
            body: "It carries on from this evening.".into(),
            kept: "Your World is kept.".into(),
            postcard: "Keep a postcard".into(),
            more: None,
            stay: "Stay a while".into(),
            hour: 18,
        }
    }

    /// Through GPUI's test window, the v0.27 bar for the demo's ending: a
    /// farewell over the place at dusk, with its recap and goodbye, and no
    /// card or chapter's end under it; "Stay a while" lets the sky go.
    /// A World whose snapshot never changes.
    struct Still(world_projection::ProjectionSnapshot);

    impl crate::ProjectionController for Still {
        fn snapshot(&self) -> world_projection::ProjectionSnapshot {
            self.0.clone()
        }

        fn handle(
            &mut self,
            _: world_projection::ProjectionIntent,
        ) -> Result<world_projection::ProjectionSnapshot, String> {
            Ok(self.0.clone())
        }
    }

    /// Through GPUI's test window, the v0.27 bar for the demo's ending: a
    /// farewell over the place at dusk, with its recap and goodbye, and no
    /// card or chapter's end under it; "Stay a while" lets the sky go.
    #[gpui::test]
    fn a_farewell_shows_alone_at_dusk_and_stays_a_while(cx: &mut gpui::TestAppContext) {
        use gpui::{Modifiers, VisualTestContext};
        let mut snapshot = crate::diorama::tests::harbour_1082();
        snapshot.briefing = None;
        snapshot.commands = vec![world_projection::ProjectionCommand {
            id: "tiny-society.let-day-pass".into(),
            title: "Let the day pass".into(),
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
        }];
        // Someone out on the place at dusk, when the farewell is.
        let speaker =
            crate::diorama::stage_at(&snapshot, 1100.0, 748.0, crate::diorama::Clock::at(18))
                .people
                .first()
                .map(|spot| snapshot.canvas.items[spot.index].id)
                .expect("someone out on the place");
        let window = cx.add_window(move |_, _| {
            let mut view = ProjectionView::controlled(Still(snapshot));
            view.looking.opening = None;
            view
        });
        let view = window.root(cx).expect("the World");
        let cx = &mut VisualTestContext::from_window(window.into(), cx);
        cx.simulate_resize(gpui::size(px(1100.0), px(800.0)));
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("bottom-card").is_some(),
            "the day's card, before"
        );
        view.update(cx, |view, cx| {
            view.show_farewell(farewell(Some((speaker, "Come back when you can.".into()))));
            cx.notify();
        });
        cx.run_until_parked();
        assert!(cx.debug_bounds("farewell").is_some(), "the farewell shows");
        assert!(cx.debug_bounds("bottom-card").is_none(), "no card under it");
        assert_eq!(
            crate::scene::daylight_at(crate::scene::hour_now()),
            crate::scene::Daylight::Dusk,
            "at dusk"
        );
        view.read_with(cx, |view, _| {
            assert!(
                view.looking.pan.is_some(),
                "the camera on whoever says goodbye"
            );
        });
        let stay = cx.debug_bounds("farewell-stay").expect("Stay a while");
        cx.simulate_click(stay.center(), Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |view, _| assert!(!view.farewell_shown()));
        assert!(
            crate::scene::pinned_hour().is_none(),
            "the clock has the sky again"
        );
        assert!(
            cx.debug_bounds("bottom-card").is_some(),
            "and the day's card is back"
        );
    }
}
