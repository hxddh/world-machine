//! The window's side of a favour and of the first minute: plain logic,
//! so it can be tested without a window.
//!
//! - A favour asked turns the camera to whoever asked it while they say
//!   it, marks whom it is for, and offers its words as a quick reply on
//!   that person's card; once done, the asker's thanks show at once.
//! - The welcome waits for the town to be painted, and the camera opens
//!   on whoever says it.
//! - Enter passes a day only from the day's own card, shown and in front:
//!   never while someone's card is open, or anything else is.

use world_projection::{CommandRole, Favour, ProjectionCommand, ProjectionSnapshot, SelectionId};

/// How long the camera stays on whoever asked a favour, in seconds: long
/// enough to hear them ask it.
pub(crate) const ASK_SECONDS: f32 = 6.0;

/// How long the asker's thanks stay over their head, in seconds.
pub(crate) const THANKS_SECONDS: f32 = 6.0;

/// Something a favour did since the window last looked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FavourBeat {
    /// Someone asked the player a favour: look at them.
    Asked { asker: SelectionId },
    /// The player did it: the asker thanks them, in these words.
    Thanked { asker: SelectionId, line: String },
}

/// What changed in the favour shown, from `before` to `now`: a new one
/// asked, or one just done. Nothing for one that was already there, or
/// one that lapsed.
pub(crate) fn favour_beat(before: Option<&Favour>, now: Option<&Favour>) -> Option<FavourBeat> {
    let now = now?;
    let same = before.is_some_and(|before| before.asker == now.asker && before.whom == now.whom);
    if now.done {
        let thanked_before = same && before.is_some_and(|before| before.done);
        return (!thanked_before).then(|| FavourBeat::Thanked {
            asker: now.asker,
            line: now.thanks.clone(),
        });
    }
    (!same || before.is_some_and(|before| before.done))
        .then_some(FavourBeat::Asked { asker: now.asker })
}

/// Whom the scene marks: whom the open favour is for.
pub(crate) fn marked(snapshot: &ProjectionSnapshot) -> Option<SelectionId> {
    snapshot
        .favour
        .as_ref()
        .filter(|favour| !favour.done)
        .map(|favour| favour.whom)
}

/// The quick reply on `who`'s card: the open favour's words, when it is
/// for them.
pub(crate) fn quick_reply(snapshot: &ProjectionSnapshot, who: SelectionId) -> Option<&str> {
    snapshot
        .favour
        .as_ref()
        .filter(|favour| !favour.done && favour.whom == who && !favour.reply.trim().is_empty())
        .map(|favour| favour.reply.as_str())
}

/// The thanks to show on `who`'s card: the asker's own words, once the
/// favour they asked is done. Only on the asker's card, so nobody else
/// seems to be thanking the player (said over the asker's head as well).
pub(crate) fn thanks_on(
    snapshot: &ProjectionSnapshot,
    who: SelectionId,
) -> Option<(SelectionId, &str)> {
    snapshot
        .favour
        .as_ref()
        .filter(|favour| favour.done && favour.asker == who && !favour.thanks.trim().is_empty())
        .map(|favour| (favour.asker, favour.thanks.as_str()))
}

/// What the asker of a favour says as they ask it: their line at the
/// World's latest moment.
pub(crate) fn ask_line(snapshot: &ProjectionSnapshot, asker: SelectionId) -> Option<String> {
    super::world_window::voices_now(snapshot)
        .into_iter()
        .find(|voice| voice.speaker == asker)
        .map(|voice| voice.line.clone())
}

/// What the window knows when Enter is pressed outside a text field.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EnterContext {
    /// Someone's card is open.
    pub(crate) asking: bool,
    /// The drawer, the hands, a story or a design is open over the scene.
    pub(crate) covered: bool,
    /// A card is in front and shown (not waiting on a greeting).
    pub(crate) card_shown: bool,
    /// The card in front is the one that lets the day pass.
    pub(crate) card_passes_day: bool,
    /// The scene holds the keyboard: nothing else (a tip, a field) does.
    pub(crate) scene_focused: bool,
}

/// What Enter does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EnterDoes {
    /// Choose the answer leaned toward on the card in front.
    Choose,
    /// Go to the open card's words: the player meant to talk.
    Talk,
    Nothing,
}

/// What Enter does, outside a text field: on someone's open card, it goes
/// to their words; it chooses on the card in front only when that card is
/// shown and nothing covers it; and it lets a day pass only from the
/// day's own card with the scene holding the keyboard.
pub(crate) fn enter_does(context: EnterContext) -> EnterDoes {
    if context.asking {
        return EnterDoes::Talk;
    }
    if context.covered || !context.card_shown {
        return EnterDoes::Nothing;
    }
    if context.card_passes_day && !context.scene_focused {
        return EnterDoes::Nothing;
    }
    EnterDoes::Choose
}

/// Whether a command lets the day pass: its Pack says so, with its role.
/// Never guessed from the id.
pub(crate) fn passes_day(command: &ProjectionCommand) -> bool {
    command.role == Some(CommandRole::PassesTime)
}

/// The first day's free thing to place, as the hands pick it (its verb,
/// and the thing as `world_window::which` names it): something the
/// player's hands can place now that costs nothing, on a World's first
/// day. `None` on any later day, or with nothing free.
pub(crate) fn free_offer(snapshot: &ProjectionSnapshot) -> Option<(String, String)> {
    let length = snapshot.calendar.as_ref()?.length.max(1);
    if snapshot.world_time.div_ceil(length).max(1) != 1 {
        return None;
    }
    snapshot
        .deeds()
        .find(|(_, command, hand)| {
            command.unavailable.is_none()
                && hand.cost.is_none()
                && !super::world_window::to_someone(&hand.verb)
                && hand.verb != "Move"
                && hand.at.is_some()
        })
        .map(|(_, command, hand)| (hand.verb.clone(), super::world_window::which(command, hand)))
}

/// What the first screen shows when the view is centred at `centre`:
/// how many residents are out on it, and how many buildings stand on it
/// (most of each building in the window). The camera stops at the ends of
/// the place, so the centre is held to them first.
pub(crate) fn first_screen(stage: &crate::diorama::Stage, centre: f32) -> (usize, usize) {
    let half = stage.view_w / 2.0;
    let centre = centre.clamp(half, (stage.width - half).max(half));
    let residents = stage
        .people
        .iter()
        .filter(|spot| (spot.x - centre).abs() <= half - 8.0)
        .count();
    let buildings = stage
        .buildings
        .iter()
        .filter(|spot| (spot.x - centre).abs() + spot.w / 2.0 <= half)
        .count();
    (residents, buildings)
}

/// How good a first screen the window centred on stage `centre` makes: one
/// that cuts no building at its edges, then the focal cluster (four
/// residents and three buildings), then someone beside a building, then
/// as much of both as it can.
pub(crate) fn view_score(
    stage: &crate::diorama::Stage,
    centre: f32,
) -> (bool, bool, bool, usize, usize, std::cmp::Reverse<usize>) {
    let (residents, buildings) = first_screen(stage, centre);
    let cut = cut_at_the_edges(stage, centre);
    (
        cut == 0,
        residents >= 4 && buildings >= 3,
        residents >= 1 && buildings >= 1,
        residents.min(6) + buildings.min(5),
        residents + buildings,
        std::cmp::Reverse(cut),
    )
}

/// How many buildings the window centred on stage `centre` cuts at its
/// left or right edge: on a first screen, none (the art director's rule).
pub(crate) fn cut_at_the_edges(stage: &crate::diorama::Stage, centre: f32) -> usize {
    let half = stage.view_w / 2.0;
    let centre = centre.clamp(half, (stage.width - half).max(half));
    let (left, right) = (centre - half, centre + half);
    stage
        .buildings
        .iter()
        .filter(|spot| {
            let (from, to) = (spot.x - spot.w / 2.0, spot.x + spot.w / 2.0);
            (from < left && to > left) || (from < right && to > right)
        })
        .count()
}

/// Where to centre a view that must show someone standing at `x` (who
/// welcomes the player, say): of the centres that keep them well inside
/// the window, the one that opens on a place, the art bible's focal
/// cluster: at least four residents and three buildings if any centre
/// shows them, then as many of both as it can, then the nearest to them;
/// and before all of that, a view that cuts no building at its edges.
pub(crate) fn best_view(stage: &crate::diorama::Stage, x: f32) -> f32 {
    let half = stage.view_w / 2.0;
    let margin = (stage.view_w * 0.12).min(half);
    let reach = half - margin;
    let score = |centre: f32| view_score(stage, centre);
    let steps = 96;
    (0..=steps)
        .map(|step| x - reach + 2.0 * reach * step as f32 / steps as f32)
        .max_by(|a, b| {
            score(*a)
                .cmp(&score(*b))
                .then((b - x).abs().total_cmp(&(a - x).abs()))
        })
        .unwrap_or(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u64) -> SelectionId {
        SelectionId::from_stable_key(&format!("entity-{n}")).expect("an entity key")
    }

    fn favour(asker: u64, whom: u64, done: bool) -> Favour {
        Favour {
            asker: id(asker),
            whom: id(whom),
            note: "Noah asked you to tell Evan they're sorry.".into(),
            hint: "Tell Evan what Noah said.".into(),
            done,
            reply: if done {
                String::new()
            } else {
                "Noah says sorry, and means it.".into()
            },
            thanks: if done {
                "You told Evan? Thank you. I feel lighter already.".into()
            } else {
                String::new()
            },
        }
    }

    #[test]
    fn a_favour_asked_turns_to_its_asker_once_and_its_thanks_show_once() {
        let open = favour(1, 2, false);
        let done = favour(1, 2, true);
        assert_eq!(
            favour_beat(None, Some(&open)),
            Some(FavourBeat::Asked { asker: id(1) })
        );
        assert_eq!(favour_beat(Some(&open), Some(&open)), None, "once only");
        assert_eq!(
            favour_beat(Some(&open), Some(&done)),
            Some(FavourBeat::Thanked {
                asker: id(1),
                line: done.thanks.clone()
            })
        );
        assert_eq!(favour_beat(Some(&done), Some(&done)), None, "thanked once");
        // A lapsed favour goes quietly; the next one asked is a new beat.
        assert_eq!(favour_beat(Some(&open), None), None);
        let next = favour(3, 1, false);
        assert_eq!(
            favour_beat(Some(&done), Some(&next)),
            Some(FavourBeat::Asked { asker: id(3) })
        );
        assert_eq!(
            favour_beat(Some(&open), Some(&next)),
            Some(FavourBeat::Asked { asker: id(3) })
        );
    }

    #[test]
    fn the_target_is_marked_and_offered_the_words_and_then_the_thanks() {
        let mut snapshot = ProjectionSnapshot {
            favour: Some(favour(1, 2, false)),
            ..ProjectionSnapshot::default()
        };
        assert_eq!(marked(&snapshot), Some(id(2)));
        assert_eq!(
            quick_reply(&snapshot, id(2)),
            Some("Noah says sorry, and means it.")
        );
        assert_eq!(
            quick_reply(&snapshot, id(1)),
            None,
            "only on the target's card"
        );
        assert_eq!(thanks_on(&snapshot, id(2)), None);
        snapshot.favour = Some(favour(1, 2, true));
        assert_eq!(marked(&snapshot), None, "done, nobody is marked");
        assert_eq!(quick_reply(&snapshot, id(2)), None);
        assert_eq!(
            thanks_on(&snapshot, id(1)),
            Some((id(1), "You told Evan? Thank you. I feel lighter already.")),
            "on the asker's card"
        );
        assert_eq!(thanks_on(&snapshot, id(2)), None, "never on the target's");
    }

    #[test]
    fn enter_lets_a_day_pass_only_from_the_days_own_card_in_front() {
        let day = EnterContext {
            card_shown: true,
            card_passes_day: true,
            scene_focused: true,
            ..EnterContext::default()
        };
        assert_eq!(enter_does(day), EnterDoes::Choose);
        // Someone's card open: Enter is for talking, never for the day.
        assert_eq!(
            enter_does(EnterContext {
                asking: true,
                ..day
            }),
            EnterDoes::Talk
        );
        // A tip or anything else holding the keyboard: nothing passes.
        assert_eq!(
            enter_does(EnterContext {
                scene_focused: false,
                ..day
            }),
            EnterDoes::Nothing
        );
        // The drawer or the hands open, or the card not shown yet.
        for context in [
            EnterContext {
                covered: true,
                ..day
            },
            EnterContext {
                card_shown: false,
                ..day
            },
        ] {
            assert_eq!(enter_does(context), EnterDoes::Nothing);
        }
        // A question's answer is still chosen with Enter.
        assert_eq!(
            enter_does(EnterContext {
                card_passes_day: false,
                ..day
            }),
            EnterDoes::Choose
        );
        let command = |id: &str, role| ProjectionCommand {
            id: id.into(),
            role,
            ..ProjectionCommand::default()
        };
        assert!(passes_day(&command(
            "anything",
            Some(CommandRole::PassesTime)
        )));
        // An id that once would have been guessed is just an id.
        assert!(!passes_day(&command("tiny-society.let-day-pass", None)));
        assert!(!passes_day(&command("tiny-society.build-bench", None)));
    }

    /// A harbour with someone asking a favour of the player at its latest
    /// moment, for someone else on the scene: as it opens, and once done.
    fn favoured() -> (
        ProjectionSnapshot,
        ProjectionSnapshot,
        SelectionId,
        SelectionId,
    ) {
        use world_projection::{CanvasItemKind, TimelineItem, Voice};
        let mut open = crate::diorama::tests::harbour_1082();
        open.retelling_clear();
        // Two people standing out on the place, not indoors.
        crate::scene::pin_hour(Some(12));
        let stage = crate::diorama::stage_at(&open, 1100.0, 748.0, crate::diorama::Clock::at(12));
        let people = stage
            .people
            .iter()
            .map(|spot| open.canvas.items[spot.index].clone())
            .filter(|item| item.kind == CanvasItemKind::Actor)
            .map(|item| item.id)
            .collect::<Vec<_>>();
        let (asker, whom) = (people[0], people[people.len() / 2]);
        let moment = SelectionId::from_stable_key("event-999999").expect("an event key");
        open.timeline.items.push(TimelineItem {
            id: moment,
            world_time: open.world_time,
            title: "A favour asked".into(),
            subtitle: String::new(),
            caused_by: Vec::new(),
            routine: false,
        });
        open.voices.push(Voice {
            moment,
            speaker: asker,
            line: "Would you tell Evan I'm sorry? It would mean more coming from you.".into(),
        });
        open.capabilities.talk = true;
        open.favour = Some(Favour {
            asker,
            whom,
            ..favour(1, 2, false)
        });
        let mut done = open.clone();
        done.favour = Some(Favour {
            asker,
            whom,
            ..favour(1, 2, true)
        });
        (open, done, asker, whom)
    }

    trait Clear {
        fn retelling_clear(&mut self);
    }

    impl Clear for ProjectionSnapshot {
        /// No return to tell: the window opens straight on the place.
        fn retelling_clear(&mut self) {
            self.briefing = None;
        }
    }

    /// A World that records what it is asked and answers a word said with
    /// the favour done.
    struct Recording {
        now: ProjectionSnapshot,
        done: ProjectionSnapshot,
        heard: std::rc::Rc<std::cell::RefCell<Vec<world_projection::ProjectionIntent>>>,
    }

    impl crate::ProjectionController for Recording {
        fn snapshot(&self) -> ProjectionSnapshot {
            self.now.clone()
        }

        fn handle(
            &mut self,
            intent: world_projection::ProjectionIntent,
        ) -> Result<ProjectionSnapshot, String> {
            if matches!(intent, world_projection::ProjectionIntent::Say { .. }) {
                self.now = self.done.clone();
            }
            self.heard.borrow_mut().push(intent);
            Ok(self.now.clone())
        }
    }

    /// Through GPUI's test window, the v0.27 favour flow: asked, the camera
    /// turns to the asker and whom it is for is marked; "Find" opens their
    /// card with the favour's words as a quick reply; one click says them,
    /// and the asker's thanks are on the card at once, well inside 5 s.
    #[gpui::test]
    fn a_favour_is_seen_found_done_in_a_click_and_thanked_at_once(cx: &mut gpui::TestAppContext) {
        use gpui::{Modifiers, VisualTestContext};
        let (open, done, asker, whom) = favoured();
        let reply = open.favour.as_ref().unwrap().reply.clone();
        let heard = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let recording = Recording {
            now: open,
            done,
            heard: heard.clone(),
        };
        let window = cx.add_window(move |_, _| {
            let mut view = super::super::ProjectionView::controlled(recording);
            view.looking.opening = None;
            view
        });
        let view = window.root(cx).expect("the World");
        let cx = &mut VisualTestContext::from_window(window.into(), cx);
        cx.simulate_resize(gpui::size(gpui::px(1100.0), gpui::px(800.0)));
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert_eq!(
                view.looking
                    .favour_beat
                    .as_ref()
                    .map(|(beat, _)| beat.clone()),
                Some(FavourBeat::Asked { asker }),
                "the ask is noticed"
            );
            assert!(view.looking.pan.is_some(), "the camera turns to the asker");
        });
        assert!(
            cx.debug_bounds("favour-mark").is_some(),
            "whom it is for is marked"
        );
        let find = cx
            .debug_bounds("favour-handle")
            .expect("a way to find them");
        cx.simulate_click(find.center(), Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |view, _| assert_eq!(view.looking.asking, Some(whom)));
        // The camera goes to them (gliding on the wall clock), and their
        // card opens beside them.
        std::thread::sleep(std::time::Duration::from_secs_f32(
            super::super::world_window::CAMERA_SECONDS + 0.2,
        ));
        for _ in 0..4 {
            view.update(cx, |_, cx| cx.notify());
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(500));
            cx.run_until_parked();
        }
        let quick = cx.debug_bounds("favour-reply").expect("the quick reply");
        cx.simulate_click(quick.center(), Modifiers::none());
        cx.run_until_parked();
        // As a structured intent: the World does the favour it offered,
        // and nothing hears the words.
        let said = heard.borrow().iter().any(|intent| {
            matches!(intent, world_projection::ProjectionIntent::Say { to, words, ears }
                if *to == whom && *words == reply && *ears == world_projection::Ears::Offered)
        });
        assert!(
            said,
            "the quick reply is said to whom it is for: {:?}",
            heard.borrow()
        );
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(100));
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("favour-thanks").is_none(),
            "the target's card never holds the asker's thanks"
        );
        view.read_with(cx, |view, _| {
            assert_eq!(
                view.favour_line().map(|(who, ..)| who),
                Some(asker),
                "the asker says thank you at once, over their own head"
            );
        });
        assert!(
            cx.debug_bounds("favour-mark").is_none(),
            "done, nobody is marked"
        );
        view.read_with(cx, |view, _| {
            assert!(matches!(
                view.looking.favour_beat,
                Some((FavourBeat::Thanked { asker: thanker, .. }, _)) if thanker == asker
            ));
        });
    }

    /// Through GPUI's test window: Enter lets a day pass from the day's
    /// own card in front, but never while someone's card is open, where it
    /// goes to their words instead.
    #[gpui::test]
    fn enter_never_passes_a_day_from_someones_card(cx: &mut gpui::TestAppContext) {
        use gpui::VisualTestContext;
        let (open, _, _, whom) = favoured();
        let mut open = open;
        open.favour = None;
        open.commands = vec![world_projection::ProjectionCommand {
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
            role: Some(CommandRole::PassesTime),
        }];
        let heard = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let recording = Recording {
            now: open.clone(),
            done: open,
            heard: heard.clone(),
        };
        let window = cx.add_window(move |_, _| {
            let mut view = super::super::ProjectionView::controlled(recording);
            view.looking.opening = None;
            view
        });
        let view = window.root(cx).expect("the World");
        let cx = &mut VisualTestContext::from_window(window.into(), cx);
        cx.simulate_resize(gpui::size(gpui::px(1100.0), gpui::px(800.0)));
        cx.run_until_parked();
        let passes =
            |heard: &std::rc::Rc<std::cell::RefCell<Vec<world_projection::ProjectionIntent>>>| {
                heard
                    .borrow()
                    .iter()
                    .filter(|intent| {
                        matches!(intent, world_projection::ProjectionIntent::InvokeCommand(id)
                        if id == "tiny-society.let-day-pass")
                    })
                    .count()
            };
        // Someone's card open: Enter is for talking to them.
        view.update(cx, |view, cx| view.ask(whom, cx));
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        assert_eq!(passes(&heard), 0, "no day passed from someone's card");
        // Closed, with the scene holding the keyboard: the day passes.
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        view.read_with(cx, |view, _| assert_eq!(view.looking.asking, None));
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        assert_eq!(passes(&heard), 1, "the day's own card, in front");
    }

    /// A brand-new harbour on its first day, as its Pack sends it.
    fn first_day() -> ProjectionSnapshot {
        let json = include_str!("../../tests/fixtures/harbour-day-1.json");
        let wire: world_pack_protocol::ProjectionSnapshotWire =
            serde_json::from_str(json).expect("a wire snapshot");
        ProjectionSnapshot::try_from(wire).expect("a snapshot")
    }

    /// Through GPUI's test window, the v0.27 first minute: the camera opens
    /// on whoever welcomes the player, with at least four residents on the
    /// first screen; nobody speaks before the town is painted; and within
    /// 20 s the hands are open on something free to place, with the first
    /// card (which asks for money) held back until it is placed.
    #[gpui::test]
    fn the_first_minute_opens_on_the_welcome_and_a_free_toy(cx: &mut gpui::TestAppContext) {
        use gpui::VisualTestContext;
        crate::scene::pin_hour(Some(12));
        let snapshot = first_day();
        let welcomer = super::super::world_window::voices_now(&snapshot)
            .first()
            .map(|voice| voice.speaker)
            .expect("someone says welcome");
        let free = free_offer(&snapshot).expect("something free on day 1");
        let recording = Recording {
            now: snapshot.clone(),
            done: snapshot.clone(),
            heard: Default::default(),
        };
        let window = cx.add_window(move |_, _| super::super::ProjectionView::controlled(recording));
        let view = window.root(cx).expect("the World");
        let cx = &mut VisualTestContext::from_window(window.into(), cx);
        cx.simulate_resize(gpui::size(gpui::px(1100.0), gpui::px(800.0)));
        cx.run_until_parked();
        let (width, height) = (1100.0, 800.0 - super::super::world_window::CHROME);
        let stage = crate::diorama::stage(&snapshot, width, height);
        let pan = view
            .read_with(cx, |view, _| view.looking.pan)
            .expect("the camera goes to the welcome");
        let camera = crate::diorama::Camera::around(&stage, 1.0, pan, height / 2.0);
        let in_view = stage
            .people
            .iter()
            .filter(|spot| {
                let (x, _) = camera.at(&stage, spot.x, spot.y);
                (0.0..=width).contains(&x)
            })
            .map(|spot| snapshot.canvas.items[spot.index].id)
            .collect::<Vec<_>>();
        let name = |id: SelectionId| {
            super::super::world_window::label_of(&snapshot, id).unwrap_or_default()
        };
        assert!(
            in_view.contains(&welcomer),
            "the welcomer {} is in view: pan {pan}, on stage {:?}, in view {:?}",
            name(welcomer),
            stage
                .people
                .iter()
                .map(|spot| (name(snapshot.canvas.items[spot.index].id), spot.x))
                .collect::<Vec<_>>(),
            in_view.iter().map(|id| name(*id)).collect::<Vec<_>>()
        );
        eprintln!("first screen: {} residents", in_view.len());
        // Within 20 s: the free thing, before any card.
        // The window times the welcome by the wall clock, so each test
        // second also moves the opening a second into the past.
        for _ in 0..20 {
            cx.executor()
                .advance_clock(std::time::Duration::from_secs(1));
            view.update(cx, |view, cx| {
                if let Some(at) = view.looking.opening {
                    view.looking.opening = at.checked_sub(std::time::Duration::from_secs(1));
                }
                cx.notify();
            });
            cx.run_until_parked();
        }
        view.read_with(cx, |view, _| {
            assert!(view.looking.free_offer, "the free thing is offered");
            let hands = view.looking.hands.as_ref().expect("the hands are open");
            assert_eq!(hands.verb.as_deref(), Some(free.0.as_str()));
        });
        assert!(
            cx.debug_bounds("bottom-card").is_none(),
            "no card before the toy"
        );
        crate::scene::pin_hour(None);
    }

    /// The v0.28 bar, in the real window (1100 by 900, a stage 848 high
    /// under the bar): day 1 opens on the focal cluster, at least four
    /// residents and three buildings, with whoever welcomes the player.
    /// The camera does its part (`best_view`); the rest is the day-1
    /// composition (A1): where the fishers work stands on the water beside
    /// the pub, so the people at work and the pub's share a window.
    #[test]
    fn the_first_screen_opens_on_the_focal_cluster() {
        crate::scene::pin_hour(Some(12));
        let snapshot = first_day();
        let stage = crate::diorama::stage(&snapshot, 1100.0, 848.0);
        let welcomer = super::super::world_window::voices_now(&snapshot)[0].speaker;
        let index = snapshot
            .canvas
            .items
            .iter()
            .position(|item| item.id == welcomer)
            .unwrap();
        let (x, _, w, _) = stage.frame_of(index).expect("the welcomer is out");
        let centre = best_view(&stage, x + w / 2.0);
        let (residents, buildings) = first_screen(&stage, centre);
        let cut = cut_at_the_edges(&stage, centre);
        crate::scene::pin_hour(None);
        assert!(
            residents >= 4 && buildings >= 3,
            "{residents} residents and {buildings} buildings on the first screen"
        );
        assert_eq!(cut, 0, "no building is cut by the window's edge");
        let half = stage.view_w / 2.0;
        let shown = centre.clamp(half, stage.width - half);
        assert!(
            (x + w / 2.0 - shown).abs() <= half * 0.9,
            "the welcomer is on the first screen"
        );
    }
}
