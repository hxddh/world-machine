//! Someone being asked, beside them: their questions, their answer and
//! what they ask for, the exchange so far, and the room it all takes
//! among the rest of the interface.

use super::*;

impl ProjectionView {
    /// What someone can be asked, beside them: their questions, and once
    /// one is asked, their answer, and what they ask for if they do.
    pub(super) fn render_asking(
        &self,
        who: SelectionId,
        x: f32,
        head: f32,
        stage: &Stage,
        cx: &mut Context<Self>,
    ) -> Div {
        let name = label_of(&self.snapshot, who).unwrap_or_default();
        let detail = self
            .snapshot
            .canvas
            .items
            .iter()
            .find(|item| item.id == who)
            .map(|item| capitalize(&item.detail))
            .unwrap_or_default();
        // Who they are stays at the top and where the player types at the
        // foot; what is between scrolls, so a long talk or large text never
        // takes the card past the window.
        let header = div()
            .flex()
            .items_center()
            .gap_3()
            .child(portrait(likeness_of(&self.snapshot, who), 40.0, false))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(ui::row_title(name.clone()))
                    .child(ui::caption(detail)),
            )
            .when(self.controller.is_some(), |row| {
                row.child(
                    ui::button("asking-story", "Their story", ButtonKind::Secondary)
                        .on_click(cx.listener(move |this, _, _, cx| this.open_legend(who, cx))),
                )
            })
            .child(arrow_button(
                "asking-close",
                "×",
                "Close",
                cx.listener(|this, _, _, cx| this.look_away(cx)),
            ));
        let mut card = div()
            .id("asking-body")
            .flex()
            .flex_col()
            .gap_2()
            .flex_shrink(1.0)
            .min_h(px(0.0))
            .overflow_y_scroll();
        let mut footer = Vec::new();
        if let Some(standing) = self
            .snapshot
            .canvas
            .items
            .iter()
            .find(|item| item.id == who)
            .and_then(|item| item.standing.as_ref())
        {
            card = card.child(standing_row(standing));
        }
        // A favour for them: what it is, and its words to say in one click;
        // once done, whoever asked it says thank you, here and at once.
        let talking = self.snapshot.capabilities.talk
            && self.controller.is_some()
            && self.retelling.is_none();
        if let (Some(reply), Some(favour), true) = (
            super::arrival::quick_reply(&self.snapshot, who),
            self.snapshot.favour.as_ref(),
            talking,
        ) {
            let words = reply.to_string();
            card = card.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(ui::caption(favour.note.clone()))
                    .child(
                        div()
                            .id("favour-reply")
                            .role(Role::Button)
                            .aria_label(words.clone())
                            .debug_selector(|| "favour-reply".into())
                            .px_3()
                            .py_2()
                            .rounded_lg()
                            .text_sm()
                            .cursor_pointer()
                            .border_1()
                            .border_color(color(tokens::ACCENT))
                            .bg(color(tokens::ACCENT_SOFT))
                            .text_color(color(tokens::ACCENT_TEXT))
                            .hover(|style| style.bg(color(tokens::ROW_HOVER)))
                            .child(crate::wrap::text(crate::i18n::quoted(&words)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.say_for_them(who, words.clone(), cx)
                            })),
                    ),
            );
        }
        if let Some((asker, thanks)) = super::arrival::thanks_on(&self.snapshot, who) {
            card = card.child(
                div()
                    .id("favour-thanks")
                    .debug_selector(|| "favour-thanks".into())
                    .flex()
                    .items_start()
                    .gap_2()
                    .px_2()
                    .py_2()
                    .rounded_lg()
                    .bg(color(tokens::ACCENT_SOFT))
                    .child(portrait(likeness_of(&self.snapshot, asker), 28.0, false))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .text_sm()
                            .text_color(color(tokens::TEXT))
                            .child(crate::wrap::text(crate::i18n::quoted(thanks))),
                    ),
            );
        }
        // Someone who can be named (a newborn): the name field, with the
        // names their parents propose.
        if self.controller.is_some()
            && self.retelling.is_none()
            && self
                .snapshot
                .canvas
                .items
                .iter()
                .any(|item| item.id == who && crate::mark::nameable(item))
        {
            match self
                .looking
                .marking
                .naming
                .clone()
                .filter(|(target, _)| *target == who)
            {
                Some((_, input)) => card = card.child(self.naming_block(who, input, cx)),
                None => {
                    card = card.child(div().flex().child(
                        ui::button("asking-name", "Name…", ButtonKind::Secondary).on_click(
                            cx.listener(move |this, _, window, cx| {
                                this.start_naming(who, window, cx)
                            }),
                        ),
                    ))
                }
            }
        }
        let answered = self.looking.answered.map(|(index, _)| index);
        for (index, talk) in self
            .snapshot
            .talks
            .iter()
            .enumerate()
            .filter(|(_, talk)| talk.who == who)
        {
            let asked = answered == Some(index);
            card = card.child(
                div()
                    .id(SharedString::from(format!("ask-{index}")))
                    .role(Role::Button)
                    .aria_label(talk.question.clone())
                    .px_3()
                    .py_2()
                    .rounded_lg()
                    .text_sm()
                    .cursor_pointer()
                    .bg(color(if asked {
                        tokens::ACCENT_SOFT
                    } else {
                        tokens::ROW_HOVER
                    }))
                    .text_color(color(if asked {
                        tokens::ACCENT_TEXT
                    } else {
                        tokens::TEXT
                    }))
                    .hover(|style| style.bg(color(tokens::ACCENT_SOFT)))
                    .child(crate::wrap::text(talk.question.clone()))
                    .on_click(cx.listener(move |this, _, _, cx| this.answer(index, cx))),
            );
            if asked {
                card = card.child(
                    div()
                        .px_3()
                        .text_sm()
                        .text_color(color(tokens::TEXT))
                        .child(crate::wrap::text(crate::i18n::quoted(&talk.answer))),
                );
                if let Some(command) = talk
                    .asks_for
                    .as_deref()
                    .and_then(|command| self.snapshot.command(command))
                    .filter(|_| self.controller.is_some() && self.retelling.is_none())
                {
                    let id = command.id.clone();
                    card = card.child(div().flex().justify_end().child(
                        ui::button("ask-grant", "Do it", ButtonKind::Primary).on_click(
                            cx.listener(move |this, _, _, cx| this.invoke_command(id.clone(), cx)),
                        ),
                    ));
                }
            }
        }
        if let Some(input) = self
            .looking
            .say
            .clone()
            .filter(|_| self.snapshot.capabilities.talk)
            .filter(|_| self.controller.is_some() && self.retelling.is_none())
        {
            card = card.child(self.render_conversation(who, cx));
            if let Some(openers) = self.render_openers(who, cx) {
                card = card.child(openers);
            }
            footer.push(
                div()
                    .pt_1()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().min_w(px(0.0)).child(input))
                    .child(
                        ui::button("say", "Say", ButtonKind::Secondary)
                            .on_click(cx.listener(|this, _, _, cx| this.say(cx))),
                    )
                    .into_any_element(),
            );
        }
        footer.push(
            div()
                .id("ask-more")
                .role(Role::Button)
                .pt_1()
                .text_xs()
                .text_color(color(tokens::TEXT_SECONDARY))
                .cursor_pointer()
                .hover(|style| style.text_color(color(tokens::ACCENT_TEXT)))
                .child(crate::i18n::more_about(&first_name(&name)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.selected = Some(who);
                    this.open_drawer();
                    cx.notify();
                }))
                .into_any_element(),
        );
        let area = self.asking_area(who, x, head, (stage.view_w, stage.height));
        let card = div()
            .id("asking")
            .role(Role::Group)
            .aria_label(crate::i18n::fill(
                "Asking {name}",
                &[("name", &world_i18n::tr_owned(&name))],
            ))
            .w(px(ASKING_WIDTH))
            .max_h(px((stage.height - ASKING_CLEAR - area.y).max(120.0)))
            .p_4()
            .rounded_xl()
            .bg(color(tokens::SURFACE))
            .shadow_lg()
            .border_1()
            .border_color(color(tokens::BORDER))
            .flex()
            .flex_col()
            .gap_2()
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(header)
            .child(card)
            .children(footer);
        div()
            .absolute()
            .left(px(area.x))
            .top(px(area.y))
            .child(ui::spring_in(
                card,
                format!("asking-{}", who.stable_key()),
                cx.reduce_motion(),
            ))
    }

    /// Where the card of someone being asked goes, generously: beside them
    /// on whichever side has room, level with their head at `x`, `head`,
    /// on a stage `width` by `height`. A conversation makes it taller; it
    /// rises to stay on screen.
    pub(super) fn asking_area(
        &self,
        who: SelectionId,
        x: f32,
        head: f32,
        (width, height): (f32, f32),
    ) -> Area {
        let talked = self
            .snapshot
            .exchanges_with(who)
            .count()
            .min(CONVERSATION_SHOWN);
        asking_place(
            x,
            head,
            (width, height),
            self.looking.drawer,
            talked,
            crate::text_scale(),
        )
    }

    /// What of the interface lies over a stage `width` by `height`,
    /// generously, for a speech bubble to keep clear of: the row of gauges
    /// and handles, the zoom control, the card at the foot (stood aside
    /// from whoever asks, at `asker_x`), an open drawer, and the card of
    /// whoever is being asked.
    pub(crate) fn interface_areas(
        &self,
        (width, height): (f32, f32),
        asker_x: Option<f32>,
        asking: Option<(SelectionId, f32, f32)>,
        crowd: &[(f32, f32)],
    ) -> Vec<Area> {
        let mut areas = vec![Area {
            x: 0.0,
            y: 0.0,
            w: width,
            h: HUD_ROOM,
        }];
        if !is_beginning(&self.snapshot) && self.retelling.is_none() {
            areas.push(Area {
                x: 16.0,
                y: zoom_top(height),
                w: ZOOM_SIZE.0,
                h: ZOOM_SIZE.1,
            });
        }
        let room = width
            - if self.looking.drawer {
                DRAWER_WIDTH
            } else {
                0.0
            };
        let film = self.retelling.and_then(|index| {
            self.looking
                .film_dock
                .filter(|(at, _)| *at == (self.revision, index))
                .map(|(_, dock)| dock)
        });
        let (from, to) = if is_beginning(&self.snapshot) {
            (0.0, width)
        } else {
            film.unwrap_or_else(|| card_dock(room, asker_x, crowd))
                .span(room)
        };
        let tall = self.card_room();
        areas.push(Area {
            x: from,
            y: height - CARD_MARGIN - tall,
            w: to - from,
            h: tall + CARD_MARGIN,
        });
        if self.looking.drawer {
            areas.push(Area {
                x: width - DRAWER_WIDTH,
                y: 0.0,
                w: DRAWER_WIDTH,
                h: height,
            });
        }
        if let Some((who, x, head)) = asking {
            areas.push(self.asking_area(who, x, head, (width, height)));
        }
        areas
    }

    /// How tall the card at the foot of the stage is, generously: a
    /// chapter's ending, with its summary, stands taller than a question
    /// (v0.29's chapter card cut a bubble placed over the question's room).
    pub(super) fn card_room(&self) -> f32 {
        let chapter = self.retelling.is_none()
            && chapter_just_ended(&self.snapshot, self.looking.chapter_read).is_some();
        if chapter {
            CHAPTER_ROOM * crate::text_scale()
        } else {
            CARD_ROOM
        }
    }

    /// The people whose figures reach down into the band the card at the
    /// foot of the stage covers: each where they stand, and half their
    /// width.
    pub(super) fn crowd(
        &self,
        bodies: &[(SelectionId, f32, f32, f32)],
        height: f32,
    ) -> Vec<(f32, f32)> {
        let top = height - CARD_MARGIN - self.card_room();
        bodies
            .iter()
            .filter(|(_, _, head, feet)| *feet > top && feet - head >= 8.0)
            .map(|(_, x, head, feet)| (*x, ((feet - head) * 0.4).max(6.0)))
            .collect()
    }

    /// What the player and someone said to each other today, latest last:
    /// the player's words small and to the right, the answer in quotes.
    pub(super) fn render_conversation(&self, who: SelectionId, cx: &mut Context<Self>) -> Div {
        const SHOWN: usize = CONVERSATION_SHOWN;
        let exchanges = self.snapshot.exchanges_with(who).collect::<Vec<_>>();
        let mut conversation = div().flex().flex_col().gap_1();
        let latest = exchanges.len().saturating_sub(1);
        for (index, exchange) in exchanges
            .iter()
            .enumerate()
            .skip(exchanges.len().saturating_sub(SHOWN))
        {
            conversation = conversation
                .child(
                    div().flex().justify_end().child(
                        div()
                            .max_w(px(220.0))
                            .px_2()
                            .py_1()
                            .rounded_lg()
                            .bg(color(tokens::ACCENT_SOFT))
                            .text_xs()
                            .text_color(color(tokens::ACCENT_TEXT))
                            .child(exchange.words.clone()),
                    ),
                )
                .child(
                    div()
                        .px_1()
                        .text_sm()
                        .text_color(color(tokens::TEXT))
                        .child(crate::i18n::quoted(&exchange.answer)),
                )
                .child(self.render_line_note(exchange, cx));
            if index == latest {
                if let Some(command) = exchange
                    .asks_for
                    .as_deref()
                    .and_then(|command| self.snapshot.command(command))
                {
                    let id = command.id.clone();
                    conversation = conversation.child(div().flex().justify_end().child(
                        ui::button("say-grant", "Do it", ButtonKind::Primary).on_click(
                            cx.listener(move |this, _, _, cx| this.invoke_command(id.clone(), cx)),
                        ),
                    ));
                }
            }
        }
        if let Some((_, words)) = self
            .looking
            .listening
            .as_ref()
            .filter(|(listening, _)| *listening == who)
        {
            conversation = conversation
                .child(
                    div().flex().justify_end().child(
                        div()
                            .max_w(px(220.0))
                            .px_2()
                            .py_1()
                            .rounded_lg()
                            .bg(color(tokens::ACCENT_SOFT))
                            .text_xs()
                            .text_color(color(tokens::ACCENT_TEXT))
                            .child(crate::wrap::text(words.clone())),
                    ),
                )
                .child(
                    div()
                        .px_1()
                        .text_sm()
                        .text_color(color(tokens::TEXT_TERTIARY))
                        .child(thinking_dots(self.looking.started)),
                )
                // Whoever answers with the World voice on is answered in a
                // model's words, and the player is told so as it happens.
                .child(
                    div()
                        .px_1()
                        .text_xs()
                        .text_color(color(tokens::TEXT_TERTIARY))
                        .child(crate::wrap::text(ui::t(VOICE_NOTICE))),
                );
        }
        conversation
    }

    /// Under an answer: that a model wrote it, when one did, and a way to
    /// report it ("report this line"), recorded in the World as an Event.
    pub(super) fn render_line_note(
        &self,
        exchange: &world_projection::Exchange,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut note = div()
            .px_1()
            .flex()
            .gap_2()
            .text_xs()
            .text_color(color(tokens::TEXT_TERTIARY));
        if exchange.voiced {
            note = note.child(ui::t(VOICED_LABEL));
        }
        if exchange.reported {
            return note.child(ui::t("Reported"));
        }
        let SelectionId::Event(spoken) = exchange.moment else {
            return note;
        };
        let command = format!("{REPORT_COMMAND}{}", spoken.0);
        note.child(
            div()
                .id(SharedString::from(format!("report-{}", spoken.0)))
                .role(Role::Button)
                .aria_label(ui::t("Report this line"))
                .cursor_pointer()
                .hover(|style| style.text_color(color(tokens::ACCENT_TEXT)))
                .child(ui::t("Report"))
                .on_click(
                    cx.listener(move |this, _, _, cx| this.invoke_command(command.clone(), cx)),
                ),
        )
    }

    /// What the player might open with to `who`, from what the World knows
    /// of them, until they have said something today: each sent as if
    /// typed.
    pub(super) fn render_openers(&self, who: SelectionId, cx: &mut Context<Self>) -> Option<Div> {
        if self.snapshot.exchanges_with(who).next().is_some() || self.looking.listening.is_some() {
            return None;
        }
        let openers = self
            .snapshot
            .openers
            .iter()
            .find(|openers| openers.who == who)?;
        let mut row = div().flex().flex_wrap().gap_1();
        for (index, line) in openers.lines.iter().enumerate() {
            let words = line.clone();
            row = row.child(
                div()
                    .id(SharedString::from(format!("opener-{index}")))
                    .role(Role::Button)
                    .aria_label(words.clone())
                    .px_2()
                    .py(px(2.0))
                    .rounded_lg()
                    .text_xs()
                    .cursor_pointer()
                    .bg(color(tokens::ROW_HOVER))
                    .text_color(color(tokens::TEXT_SECONDARY))
                    .hover(|style| style.bg(color(tokens::ACCENT_SOFT)))
                    .child(crate::wrap::text(words.clone()))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if let Some(input) = this.looking.say.clone() {
                            input.update(cx, |input, cx| input.set_text(words.clone(), cx));
                            let _ = window;
                            this.say(cx);
                        }
                    })),
            );
        }
        Some(row)
    }
}
