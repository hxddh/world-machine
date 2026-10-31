//! What floats over the scene: the gentle pointers, the zoom control,
//! the gauges and handles over the sky, the player's hands, the card,
//! and a chapter's ending.

use super::*;

impl ProjectionView {
    /// Whether the player's hands have anything to do here.
    pub(super) fn has_deeds(&self) -> bool {
        self.controller.is_some()
            && self.retelling.is_none()
            && !is_beginning(&self.snapshot)
            && !self.asleep()
            && self.snapshot.deeds().next().is_some()
    }

    /// The gentle pointer to show now, and how long it has shown: it keeps
    /// to the app's record (each once, none at what has been used), waits
    /// while anything else is going on, and never covers a line being said.
    pub(super) fn point(
        &mut self,
        width: f32,
        height: f32,
        spoken: Option<Area>,
    ) -> Option<(Pointer, f32)> {
        let record = pointers::record()?;
        let quiet = self.controller.is_some()
            && self.retelling.is_none()
            && !is_beginning(&self.snapshot)
            && self.looking.hands.is_none()
            && self.looking.asking.is_none()
            && !self.looking.drawer
            && !self.looking.photographing
            && chapter_just_ended(&self.snapshot, self.looking.chapter_read).is_none()
            && self
                .looking
                .gift_at
                .is_none_or(|at| since(Some(at)) > GIFT_SECONDS)
            && self
                .looking
                .opening
                .is_none_or(|at| since(Some(at)) > question_waits(&self.snapshot) + LINE_SECONDS);
        let clear = |pointer: Pointer| {
            spoken.is_none_or(|spoken| !hint_area(pointer, width, height).overlaps(&spoken))
        };
        if let Some((pointer, at)) = self.looking.pointer {
            let age = since(Some(at));
            if !record.used.contains(&pointer) && age < pointers::LASTS && quiet {
                return clear(pointer).then_some((pointer, age));
            }
            self.looking.pointer = None;
            self.looking.pointer_gone = Some(Instant::now());
        }
        let mut offered = BTreeSet::new();
        if self.has_deeds() {
            offered.insert(Pointer::Hands);
        }
        if card_order(&self.snapshot).len() > 1 && self.card_command().is_some() {
            offered.insert(Pointer::Cards);
        }
        offered.insert(Pointer::Drawer);
        if height >= ZOOM_HINT_ROOM {
            offered.insert(Pointer::Zoom);
        }
        if !self.snapshot.letters.is_empty() {
            offered.insert(Pointer::Letters);
        }
        if self.strip.is_some() {
            offered.insert(Pointer::Strip);
        }
        offered.retain(|pointer| clear(*pointer));
        let now = pointers::Now {
            open_for: since(self.looking.started),
            since_last: self.looking.pointer_gone.map(|at| since(Some(at))),
            quiet,
            offered,
        };
        let pointer = pointers::next(&record, &now)?;
        pointers::shown(pointer);
        self.looking.pointer = Some((pointer, Instant::now()));
        Some((pointer, 0.0))
    }

    /// A pointer as it shows, `age` seconds in: rising in on a spring and
    /// fading at the end, or simply there with Reduce Motion on.
    pub(super) fn pointer_hint(
        &self,
        pointer: Pointer,
        age: f32,
        caret: Caret,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let still = cx.reduce_motion();
        let fade = if still {
            1.0
        } else {
            ((pointers::LASTS - age) / 0.6).clamp(0.0, 1.0)
        };
        let hint = pointer_hint(
            pointer,
            caret,
            cx.listener(|this, _, _, cx| {
                cx.stop_propagation();
                this.looking.pointer = None;
                this.looking.pointer_gone = Some(Instant::now());
                cx.notify();
            }),
        )
        .opacity(fade);
        ui::spring_in(hint, format!("pointer-{}", pointer.key()), still).into_any_element()
    }

    /// The zoom control: closer, and back out, beside the wheel and a
    /// pinch.
    pub(super) fn render_zoom(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let closest = self.view_zoom() >= ZOOM_MOST - 0.01;
        let least = 1.0 / self.snapshot.canvas.width.unwrap_or(1.0).max(1.0);
        let whole = self.view_zoom() <= least + 0.01;
        div()
            .id("zoom")
            .role(Role::Group)
            .aria_label(ui::t("Zoom"))
            // A click on the zoom is never a click on the ground under it
            // (it once placed the free bench).
            .occlude()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .p_1()
            .rounded_full()
            .bg(scene_paper().opacity(0.86))
            .shadow_sm()
            .child(
                zoom_button("zoom-in", "+", "Zoom in (+)", closest)
                    .aria_keyshortcuts("+")
                    .on_click(
                        cx.listener(|this, _, window, cx| {
                            this.zoom_by(ZOOM_STEP, None, window, cx)
                        }),
                    ),
            )
            .child(
                zoom_button("zoom-out", "−", "Zoom out (−)", whole)
                    .aria_keyshortcuts("-")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.zoom_by(1.0 / ZOOM_STEP, None, window, cx)
                    })),
            )
    }

    /// Over the sky: the stakes on the left, and on the right the moment
    /// and the drawer's handle.
    pub(super) fn render_hud(
        &self,
        pointing: Option<(Pointer, f32)>,
        cx: &mut Context<Self>,
    ) -> Div {
        let previewing = self.card_command();
        let mut gauges = div().flex().flex_wrap().gap_2();
        for gauge in &self.snapshot.gauges {
            let by = previewing.and_then(|command| {
                command
                    .moves
                    .iter()
                    .find(|step| step.gauge == gauge.id)
                    .map(|step| step.by)
            });
            // A gauge a turn moved slides from where it stood.
            let before = self
                .before_turn
                .as_ref()
                .and_then(|before| before.gauges.iter().find(|old| old.id == gauge.id))
                .map(|old| old.value);
            let settle = (since(self.looking.turn_at) / GAUGE_SECONDS).clamp(0.0, 1.0);
            let shown = match before {
                Some(old) => old + (gauge.value - old) * (settle * settle * (3.0 - 2.0 * settle)),
                None => gauge.value,
            };
            gauges = gauges.child(hud_gauge(gauge, shown, by));
        }
        let mut right = div().flex().items_center().gap_2();
        if self.snapshot.world_time > 0 {
            // Named for the part of the day the sky shows.
            let dark = matches!(scene::daylight_now(), scene::Daylight::Night);
            right = right.child(
                pill()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(crate::i18n::now_label(&self.snapshot, dark)),
            );
        }
        // A pointer at one of these handles hangs just under it.
        let hint_under = |handle: Stateful<Div>, pointers: &[Pointer], cx: &mut Context<Self>| {
            let hint = pointing
                .filter(|(pointer, _)| pointers.contains(pointer))
                .map(|(pointer, age)| self.pointer_hint(pointer, age, Caret::Up, cx));
            div()
                .relative()
                .child(handle)
                .when_some(hint, |anchor, hint| {
                    anchor.child(div().absolute().top(px(40.0)).right_0().child(hint))
                })
        };
        // The player's own hands: build, plant, give, invite.
        let has_deeds = self.has_deeds();
        if has_deeds {
            let open = self.looking.hands.is_some();
            let handle = ui::named(pill().id("hands-handle"), "Make something (H)")
                .role(Role::Button)
                .aria_keyshortcuts("h")
                .aria_expanded(open)
                .cursor_pointer()
                .when(open, |pill| pill.bg(color(tokens::ACCENT)))
                .hover(|style| style.bg(color(tokens::SURFACE)))
                .child(plus_glyph(open).size(px(16.0)))
                .on_click(cx.listener(|this, _, _, cx| this.toggle_hands(cx)));
            right = right.child(hint_under(handle, &[Pointer::Hands], cx));
        }
        if let (true, Some((_, title))) = (has_deeds, self.undo_command()) {
            right = right.child(
                pill()
                    .id("undo-handle")
                    .role(Role::Button)
                    .aria_label(ui::t(format!("{title} (⌘Z)")))
                    .cursor_pointer()
                    .hover(|style| style.bg(color(tokens::SURFACE)))
                    .child(format!("↶ {}", ui::t(title.clone())))
                    .on_click(cx.listener(|this, _, _, cx| this.undo(cx))),
            );
        }
        if self.controller.is_some() && self.retelling.is_none() && !is_beginning(&self.snapshot) {
            right = right.child(
                pill()
                    .id("photo-handle")
                    .debug_selector(|| "photo-handle".into())
                    .role(Role::Button)
                    .aria_label(ui::t("Save a photo of the scene (P)"))
                    .cursor_pointer()
                    .hover(|style| style.bg(color(tokens::SURFACE)))
                    .child(ui::t("Photo"))
                    .on_click(cx.listener(|this, _, window, cx| this.take_photo(window, cx))),
            );
            right = right.child(
                pill()
                    .id("postcard-handle")
                    .debug_selector(|| "postcard-handle".into())
                    .role(Role::Button)
                    .aria_label(ui::t(
                        "Save a postcard of the moment picked in History, or of now (C)",
                    ))
                    .cursor_pointer()
                    .hover(|style| style.bg(color(tokens::SURFACE)))
                    .child(ui::t("Postcard"))
                    .on_click(cx.listener(|this, _, window, cx| this.take_postcard(window, cx))),
            );
        }
        if let Some(show) = self.strip.clone().filter(|_| {
            self.controller.is_some() && self.retelling.is_none() && !is_beginning(&self.snapshot)
        }) {
            let handle = ui::named(
                pill().id("strip-handle"),
                "Show as a strip along the edge of the screen (⌥⌘S)",
            )
            .role(Role::Button)
            .aria_keyshortcuts("Alt+Meta+S")
            .cursor_pointer()
            .hover(|style| style.bg(color(tokens::SURFACE)))
            .child(strip_glyph().w(px(18.0)).h(px(16.0)))
            .on_click(move |_, window, cx| {
                pointers::used(Pointer::Strip);
                show(window, cx);
            });
            right = right.child(hint_under(handle, &[Pointer::Strip], cx));
        }
        // A favour open: who it is for, found in one click (their card
        // opens, with its words to say, and the camera goes to them).
        if let Some(favour) = self
            .snapshot
            .favour
            .as_ref()
            .filter(|favour| !favour.done)
            .filter(|_| self.controller.is_some() && self.retelling.is_none() && !self.asleep())
        {
            let whom = favour.whom;
            let name = label_of(&self.snapshot, whom)
                .map(|name| first_name(&name))
                .unwrap_or_default();
            right = right.child(
                pill()
                    .id("favour-handle")
                    .role(Role::Button)
                    .aria_label(favour.note.clone())
                    .debug_selector(|| "favour-handle".into())
                    .cursor_pointer()
                    .hover(|style| style.bg(color(tokens::SURFACE)))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(div().size(px(8.0)).rounded_full().bg(color(tokens::ACCENT)))
                    .child(crate::i18n::find_whom(&name))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.ask(whom, cx)
                    })),
            );
        }
        let handle = ui::named(
            pill().id("drawer-handle"),
            "The drawer: story, letters, keepsakes and the book (⌘I)",
        )
        .role(Role::Button)
        .aria_keyshortcuts("Meta+I")
        .aria_expanded(self.looking.drawer)
        .cursor_pointer()
        .hover(|style| style.bg(color(tokens::SURFACE)))
        .child(drawer_glyph().size(px(16.0)))
        .on_click(cx.listener(|this, _, _, cx| this.toggle_drawer(cx)));
        right = right.child(hint_under(handle, &[Pointer::Drawer, Pointer::Letters], cx));
        div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .p_4()
            .flex()
            .items_start()
            .justify_between()
            .gap_4()
            // The gauges keep to their row; a handle added on the right
            // (a chip to take something back) wraps the handles, never
            // the gauges into a column.
            .child(gauges.flex_shrink_0())
            .child(right.flex_1().min_w(px(0.0)).flex_wrap().justify_end())
    }

    /// What the player's hands can make or do with `verb`, each thing
    /// once: its name (a second of the same told apart by a number), what
    /// it costs, and why it cannot be done now if it cannot.
    pub(crate) fn hand_things(&self, verb: &str) -> Vec<HandThing> {
        let mut things = Vec::<HandThing>::new();
        for (_, command, hand) in self
            .snapshot
            .deeds()
            .filter(|(_, _, hand)| hand.verb == verb)
        {
            let key = which(command, hand);
            if things.iter().any(|thing| thing.key == key) {
                continue;
            }
            let possible = self.snapshot.deeds().any(|(_, other_command, other)| {
                other.verb == verb
                    && which(other_command, other) == key
                    && other_command.unavailable.is_none()
            });
            // Two benches the player made are two tiles: the second one is
            // told apart by a number.
            let same_name = self
                .snapshot
                .deeds()
                .filter(|(_, _, other)| other.verb == verb && other.thing == hand.thing)
                .map(|(_, other_command, other)| which(other_command, other))
                .fold(Vec::<String>::new(), |mut keys, other| {
                    if !keys.contains(&other) {
                        keys.push(other);
                    }
                    keys
                });
            let label = match same_name.iter().position(|other| *other == key) {
                Some(index) if index > 0 => format!("{} {}", hand.thing, index + 1),
                _ => hand.thing.clone(),
            };
            things.push(HandThing {
                key,
                label,
                cost: hand.cost.clone(),
                reason: (!possible).then(|| command.unavailable.clone()).flatten(),
                possible,
                shape: hand_shape(&command.id, verb),
                command: command.id.clone(),
            });
        }
        things
    }

    /// The hand verbs there is anything to do with, in their order.
    pub(crate) fn hand_verbs(&self) -> Vec<&'static str> {
        VERBS
            .into_iter()
            .filter(|verb| self.snapshot.deeds().any(|(_, _, hand)| hand.verb == *verb))
            .collect()
    }

    /// The town's purse, as the World counts it, for weighing a cost: its
    /// gauge named "money", read as a number.
    pub(crate) fn purse(&self) -> Option<String> {
        self.snapshot
            .gauges
            .iter()
            .find(|gauge| gauge.id == "money")
            .map(|gauge| format!("{} · {}", gauge.label, gauge.reading))
    }

    /// The player's hands, open: the same drawn card a plot opens, with
    /// the verbs as tabs, and for the one picked what can be made, each
    /// as it would stand with what it costs; then where it goes or who it
    /// is for.
    pub(super) fn render_hands(&self, cx: &mut Context<Self>) -> Option<Div> {
        let hands = self.looking.hands.clone()?;
        let mut tabs = ui::region("hands-verbs", Role::TabList, "What to do")
            .flex()
            .flex_wrap()
            .gap_1();
        for verb in self.hand_verbs() {
            let chosen = hands.verb.as_deref() == Some(verb);
            tabs = tabs.child(verb_tab(verb, chosen).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.looking.hands = Some(Hands {
                        verb: Some(verb.to_string()),
                        thing: to_someone(verb).then(|| "*".to_string()),
                    });
                    cx.notify();
                },
            )));
        }
        let mut body = div().flex().flex_col().gap_2();
        let mut wide = false;
        match (hands.verb.as_deref(), hands.thing.as_deref()) {
            (Some(verb), Some(thing)) => {
                let named = self
                    .snapshot
                    .deeds()
                    .find(|(_, command, hand)| hand.verb == verb && which(command, hand) == thing)
                    .map(|(_, _, hand)| hand.thing.clone())
                    .unwrap_or_else(|| thing.to_string());
                let hint = match (verb, thing) {
                    ("Give", "*") => ui::t("Choose who to give a present to.").to_string(),
                    (_, "*") => ui::t("Choose who to invite out.").to_string(),
                    ("Move", _) => crate::i18n::choose_where(&named, true),
                    _ => crate::i18n::choose_where(&named, false),
                };
                if self.looking.free_offer {
                    body = body.child(ui::caption("On the house, to welcome you."));
                }
                body = body.child(div().text_sm().text_color(color(tokens::TEXT)).child(hint));
                // The first day's free thing opened by itself: it says how
                // to place it, and how to leave it for later, so the day's
                // card is never out of reach (v0.31's soft lock).
                if self.looking.free_offer {
                    body = body
                        .child(ui::caption(
                            "Click where you would like it in the scene. Or press Enter to do it later.",
                        ))
                        .child(
                            div().flex().justify_end().child(
                                ui::button("hands-later", "Later", ui::ButtonKind::Secondary)
                                    .debug_selector(|| "hands-later".into())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.put_free_offer_away(cx);
                                    })),
                            ),
                        );
                }
                if self
                    .hand_targets()
                    .is_some_and(|targets| targets.is_empty())
                {
                    if let Some(reason) = self
                        .snapshot
                        .deeds()
                        .find(|(_, command, hand)| {
                            hand.verb == verb && (thing == "*" || which(command, hand) == thing)
                        })
                        .and_then(|(_, command, _)| command.unavailable.clone())
                    {
                        body = body.child(
                            div()
                                .text_xs()
                                .text_color(color(tokens::WARNING))
                                .child(reason),
                        );
                    }
                }
                if !to_someone(verb) {
                    let verb = verb.to_string();
                    body = body.child(
                        div()
                            .id("hands-back")
                            .role(Role::Button)
                            .aria_label(ui::t("Something else"))
                            .text_sm()
                            .text_color(color(tokens::ACCENT_TEXT))
                            .cursor_pointer()
                            .child(ui::t("Something else"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.looking.hands = Some(Hands {
                                    verb: Some(verb.clone()),
                                    thing: None,
                                });
                                cx.notify();
                            })),
                    );
                }
            }
            (Some(verb), None) => {
                wide = true;
                let mut grid = div()
                    .id("hands-things")
                    .role(Role::List)
                    .aria_label(ui::t("Make something"))
                    .max_h(px(430.0))
                    .overflow_y_scroll()
                    .flex()
                    .flex_wrap()
                    .gap_2();
                for thing in self.hand_things(verb) {
                    let (verb, key) = (verb.to_string(), thing.key.clone());
                    let tile = super::marking::offer_tile(
                        SharedString::from(format!("hands-thing-{}", thing.command)),
                        &thing.label,
                        thing
                            .reason
                            .clone()
                            .or(thing.cost.clone())
                            .unwrap_or_default(),
                        thing.possible,
                        false,
                        thing.shape,
                        None,
                    );
                    grid = grid.child(if thing.possible {
                        tile.on_click(cx.listener(move |this, _, _, cx| {
                            this.looking.hands = Some(Hands {
                                verb: Some(verb.clone()),
                                thing: Some(key.clone()),
                            });
                            cx.notify();
                        }))
                    } else {
                        tile
                    });
                }
                body = body.child(grid);
                if verb == "Build" && !mark::plots_of(&self.snapshot).is_empty() {
                    body = body.child(ui::caption(
                        "Bigger things are built on a plot: choose one in the scene.",
                    ));
                }
            }
            _ => {}
        }
        let columns = if wide { 3.0 } else { 2.0 };
        let width = columns * super::marking::OFFER_W + (columns - 1.0) * 8.0 + 32.0 + 6.0;
        Some(
            div()
                .absolute()
                .top(px(64.0))
                .right(px(16.0))
                .w(px(width))
                .child(
                    div()
                        .id("hands")
                        .role(Role::Group)
                        .aria_label(ui::t("Make something"))
                        .p_4()
                        .rounded_xl()
                        .bg(scene_paper())
                        .border_1()
                        .border_color(color(tokens::BORDER))
                        .flex()
                        .flex_col()
                        .gap_3()
                        .shadow_lg()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(ui::row_title("Make something"))
                                .children(self.purse().map(ui::caption)),
                        )
                        .child(tabs)
                        .child(body),
                ),
        )
    }

    /// Your turn: one card, whoever asks it large, one line, and the ways
    /// to leaf through the others and choose.
    pub(super) fn render_card(&self, cx: &mut Context<Self>) -> Option<Div> {
        let command = self.card_command()?;
        let count = card_order(&self.snapshot).len();
        let index = self.looking.card.min(count - 1);
        let answers = self.card_answers();
        let question = command.question.clone();
        let people = asker_ids(&self.snapshot, command.asker);
        let names = asker_faces(&self.snapshot, command.asker)
            .iter()
            .map(|name| first_name(name))
            .collect::<Vec<_>>();
        let face = if people.is_empty() {
            div()
                .flex_shrink_0()
                .size(px(44.0))
                .rounded_full()
                .bg(color(tokens::ACCENT_SOFT))
                .p(px(11.0))
                .child(clock_glyph().size_full())
        } else {
            let mut stack = div()
                .relative()
                .flex_shrink_0()
                .h(px(44.0))
                .w(px(44.0 + 26.0 * (people.len().min(2) - 1) as f32));
            for (position, person) in people.iter().take(2).enumerate().rev() {
                stack = stack.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(position as f32 * 26.0))
                        .rounded(px(12.0))
                        .border_2()
                        .border_color(color(tokens::SURFACE))
                        .child(portrait(
                            likeness_of(&self.snapshot, *person),
                            40.0,
                            position == 0,
                        )),
                );
            }
            stack
        };
        let mut text = div().flex_1().min_w(px(0.0)).flex().flex_col().gap(px(2.0));
        // Who asks, with the card's way round (the dots, More) beside it:
        // one line for both keeps the card low over the quay.
        let names_line = names.join(" & ");
        // The morning's main question says so, over what it asks (v0.30 S4).
        let weighty = super::sitting::is_days_question(&self.snapshot, command);
        if weighty {
            text = text.child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color(tokens::ACCENT_TEXT))
                    .child(ui::t("Today's choice")),
            );
        }
        // A question's card says what is asked; a lone choice says itself.
        text = text.child(div().text_base().font_weight(FontWeight::SEMIBOLD).child(
            crate::wrap::text(match &question {
                Some(question) => question.prompt.clone(),
                None => command.title.clone(),
            }),
        ));
        if self.looking.card_back {
            if !command.detail.is_empty() {
                text = text.child(ui::detail(command.detail.clone()));
            }
            let mut chips = div().pt_1().flex().flex_wrap().gap_1();
            for step in &command.moves {
                if let Some(gauge) = self
                    .snapshot
                    .gauges
                    .iter()
                    .find(|gauge| gauge.id == step.gauge)
                {
                    chips = chips.child(move_chip(&gauge.label, step.by));
                }
            }
            for effect in &command.effects {
                if command.moves.is_empty()
                    || !matches!(effect.change, EffectChange::Up | EffectChange::Down)
                {
                    chips = chips.child(effect_chip(effect));
                }
            }
            text = text.child(chips);
        }
        let mut dots = div().flex().items_center().gap(px(6.0));
        if count > 1 {
            dots = dots.child(arrow_button(
                "card-previous",
                "‹",
                "Previous card (↑)",
                cx.listener(|this, _, _, cx| this.cycle_card(-1, cx)),
            ));
            for step in 0..count {
                dots = dots.child(
                    div()
                        .size(px(if step == index { 8.0 } else { 6.0 }))
                        .rounded_full()
                        .bg(color(if step == index {
                            tokens::ACCENT
                        } else {
                            tokens::BORDER_STRONG
                        })),
                );
            }
            dots = dots.child(arrow_button(
                "card-next",
                "›",
                "Next card (↓)",
                cx.listener(|this, _, _, cx| this.cycle_card(1, cx)),
            ));
        }
        let id = command.id.clone();
        let actions = div()
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .id("card-more")
                    .role(Role::Button)
                    .aria_label(ui::t(if self.looking.card_back {
                        "Less"
                    } else {
                        "More"
                    }))
                    .aria_expanded(self.looking.card_back)
                    .aria_keyshortcuts("Space")
                    .tooltip(ui::tip(if self.looking.card_back {
                        "Turn the card back (Space)"
                    } else {
                        "Turn the card over: what this would change (Space)"
                    }))
                    .tooltip_show_delay(ui::TIP_DELAY)
                    .text_sm()
                    .text_color(color(tokens::TEXT_SECONDARY))
                    .cursor_pointer()
                    .hover(|style| style.text_color(color(tokens::ACCENT_TEXT)))
                    .child(ui::t(if self.looking.card_back {
                        "Less"
                    } else {
                        "More"
                    }))
                    .on_click(cx.listener(|this, _, _, cx| this.turn_card(cx))),
            )
            .when(
                question.is_none() && command.unavailable.is_none(),
                |actions| {
                    actions.child(
                        ui::button("card-choose", "Choose", ButtonKind::Primary).on_click(
                            cx.listener(move |this, _, _, cx| this.invoke_command(id.clone(), cx)),
                        ),
                    )
                },
            );
        // Every answer to the question, on the card itself: pointing at one
        // leans toward it and the gauges show what it would move; clicking
        // it answers.
        let leaned = self.looking.answer.min(answers.len().saturating_sub(1));
        let mut replies = ui::region("answers", Role::Group, "Answers")
            .flex()
            .flex_col()
            .gap(px(4.0));
        if question.is_some() {
            for (position, answer) in answers.iter().enumerate() {
                let Some(reply) = self.snapshot.commands.get(*answer) else {
                    continue;
                };
                let id = reply.id.clone();
                let chosen = position == leaned;
                let row = answer_button(
                    position,
                    answers.len(),
                    reply.title.clone(),
                    chosen,
                    reply.unavailable.as_deref(),
                )
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    if *hovered {
                        this.lean_to(position, cx);
                    }
                }));
                replies = replies.child(if reply.unavailable.is_some() {
                    row
                } else {
                    row.on_click(
                        cx.listener(move |this, _, _, cx| this.invoke_command(id.clone(), cx)),
                    )
                });
            }
        }

        let prompt = match &question {
            Some(question) => question.prompt.clone(),
            None => command.title.clone(),
        };
        let card = div()
            .id("turn-card")
            .role(Role::Group)
            .aria_label(if names.is_empty() {
                prompt.to_string()
            } else {
                format!("{}: {prompt}", names.join(" & "))
            })
            .px_4()
            .py_3()
            .rounded_2xl()
            .bg(scene_paper())
            .shadow_lg()
            // The day's question is weighted: a heavier edge in the accent.
            .when(weighty, |card| {
                card.border_2().border_color(color(tokens::ACCENT))
            })
            .when(!weighty, |card| {
                card.border_1().border_color(color(tokens::BORDER))
            })
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(div().min_w(px(0.0)).child(ui::caption(names_line)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(dots)
                            .child(actions),
                    ),
            )
            .child(div().flex().items_center().gap_3().child(face).child(text))
            .when(question.is_some(), |card| card.child(replies));
        Some(div().child(ui::spring_in(
            card,
            format!("card-{}-{index}", self.revision()),
            cx.reduce_motion(),
        )))
    }

    /// A chapter's ending, as a card of its own: its title, how it went,
    /// and the way on into the next.
    pub(super) fn render_chapter_end(&self, cx: &mut Context<Self>) -> Option<Div> {
        self.controller.as_ref()?;
        let chapter = chapter_just_ended(&self.snapshot, self.looking.chapter_read)?;
        let number = chapter.number;
        let card = div()
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
            .child(ui::caption(crate::i18n::fill(
                "Chapter {number} ends",
                &[("number", &number.to_string())],
            )))
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(chapter.title.clone()),
            )
            .child(
                div()
                    .text_center()
                    .text_color(color(tokens::TEXT_SECONDARY))
                    .child(chapter.summary.clone()),
            )
            .child(
                div().pt_2().child(
                    ui::button(
                        "chapter-next",
                        format!("Begin chapter {}", number + 1),
                        ButtonKind::Primary,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.looking.chapter_read = Some(number);
                        cx.notify();
                    })),
                ),
            );
        Some(div().child(ui::arrive(card, format!("chapter-{number}"), 0)))
    }
}
