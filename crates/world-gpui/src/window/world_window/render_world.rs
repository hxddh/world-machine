//! The whole World window drawn: the scene, and over it the gauges, the
//! moment, the card, whoever is being asked, and the drawer.

use super::*;

impl ProjectionView {
    /// The whole World window: the scene, and over it the gauges, the
    /// moment, your card, whoever is being asked, and the drawer.
    pub(crate) fn render_world(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        self.watch_visibility(window, cx);
        self.keep_living(window, cx);
        // In front, the World is drawn at the display's own rate (60 or
        // 120 times a second), or at the player's frame cap; with Reduce
        // Motion on, a few times a second is enough for what still moves;
        // covered, not at all.
        if pace(
            window.is_visible(),
            window.is_window_active(),
            cx.reduce_motion(),
        ) == Pace::EveryFrame
        {
            self.request_frame(window, cx);
        }
        let focus = self
            .looking
            .focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        if self.looking.started.is_none() {
            let now = Instant::now();
            self.looking.started = Some(now);
            if !is_beginning(&self.snapshot) {
                self.looking.opening = Some(now);
            }
            window.focus(&focus, cx);
        } else if window.focused(cx).is_none() {
            // Nothing holds the keyboard (a tip was dismissed, a field
            // closed): the scene takes it back, so keys are never lost.
            window.focus(&focus, cx);
        }
        // Reduce Motion holds the scene still: nobody wanders, bobs or
        // turns, and the clouds and the sea stay where they are.
        let still = cx.reduce_motion();
        let seconds = if still {
            0.0
        } else {
            since(self.looking.started)
        };
        let (width, height) = self.stage_size(window);
        let mut stage = diorama::stage(&self.snapshot, width, height);
        if still {
            // Nobody strolls across the place: they are simply there.
            stage.routes.clear();
        }
        let daylight = scene::daylight_now();

        let film_painted = self.keep_books(window, &stage, cx);
        // The beat's scene, as far in as its clock (held at its start until
        // the camera is there and the place painted): its cast put on the
        // stage where it has them, everyone else about their day.
        // With Reduce Motion on, it is held at one moment, its payoff in
        // view: nobody walks, no camera glides, no prop fades in jumps a
        // few frames a second. The beat still lasts as long as its scene.
        let scene_world = self.scene_world();
        let staged = self.played_scene().map(|script| {
            let world = scene_world.as_deref().unwrap_or(&self.snapshot);
            if still {
                diorama::staging::stage_still(&script, &stage, world)
            } else {
                let t = self.beat_clock().min(script.seconds);
                diorama::staging::stage_scene(&script, &stage, world, t)
            }
        });
        // A return beat's card docks once for the whole of its scene, clear
        // of what the scene is about all through it.
        if let (Some(index), Some(script)) = (self.retelling, self.played_scene()) {
            let key = (self.revision, index);
            if self.looking.film_dock.is_none_or(|(at, _)| at != key) {
                let room = width
                    - if self.looking.drawer {
                        DRAWER_WIDTH
                    } else {
                        0.0
                    };
                let world = scene_world.as_deref().unwrap_or(&self.snapshot);
                let dock = scene_dock(&script, &stage, world, (room, height));
                self.looking.film_dock = Some((key, dock));
            }
        }
        if let Some(staged) = &staged {
            stage = diorama::staging::cast_on(&stage, staged);
        }
        let camera = self.camera(
            &stage,
            window,
            staged.as_ref().and_then(|s| s.camera),
            cx.reduce_motion(),
        );
        if std::mem::take(&mut self.looking.glide_wants) {
            self.request_frame(window, cx);
        }

        // Who is needed where they are, and who is talking: whoever the
        // window opened on first.
        let mut speaking = voices_now(&self.snapshot);
        if let Some(at) = self
            .looking
            .opener
            .and_then(|who| speaking.iter().position(|voice| voice.speaker == who))
        {
            speaking.rotate_left(at);
        }
        let answered = self.looking.answered.and_then(|(index, at)| {
            let talk = self.snapshot.talks.get(index)?;
            (at.elapsed().as_secs_f32() < answer_seconds(&talk.answer)).then_some((talk, at))
        });
        let beat_voice = self
            .current_beat()
            .and_then(|beat| beat.selection)
            .and_then(|moment| self.snapshot.voice_at(moment));
        let said = self.looking.said_at.and_then(|at| {
            let who = self.looking.asking?;
            let exchange = self.snapshot.exchanges_with(who).last()?;
            (at.elapsed().as_secs_f32() < answer_seconds(&exchange.answer))
                .then_some((exchange, at))
        });
        // What is said now, by whom, how long ago it began, whether it
        // answers the player, and how long it stays.
        let line = if let Some((who, text)) = self.farewell_line() {
            let text = ui::t(text).to_string();
            let length = LINE_SECONDS * speech_pages(&text).len() as f32;
            // Said again and again while the farewell shows.
            let age = since(self.looking.started) % (length + LINE_SECONDS);
            (age < length).then_some((who, text, age, true, length))
        } else if let Some((exchange, at)) = said {
            Some((
                exchange.who,
                exchange.answer.clone(),
                since(Some(at)),
                true,
                answer_seconds(&exchange.answer),
            ))
        } else if let Some((talk, at)) = answered {
            Some((
                talk.who,
                talk.answer.clone(),
                since(Some(at)),
                true,
                answer_seconds(&talk.answer),
            ))
        } else if let Some((who, text, age)) = self.favour_line() {
            let length = LINE_SECONDS * speech_pages(&text).len() as f32;
            Some((who, text, age, true, length))
        } else if let Some(staged) = staged.as_ref().filter(|_| self.retelling.is_some()) {
            // A scene says only what its script records, by whoever said it.
            staged
                .bubble
                .clone()
                .filter(|_| film_painted)
                .map(|(who, text, age)| (who, text, age, true, f32::MAX))
        } else if self.retelling.is_some() {
            beat_voice.filter(|_| film_painted).map(|voice| {
                (
                    voice.speaker,
                    voice.line.clone(),
                    since(self.looking.beat_at),
                    true,
                    f32::MAX,
                )
            })
        } else if speaking.is_empty()
            || !self.looking.painted
            || self
                .card_command()
                .is_some_and(|command| command.question.is_some())
        {
            // While someone asks a question, the card says it and the scene
            // keeps quiet.
            None
        } else {
            // Each speaker in turn, a long line given a turn for each of
            // its pages.
            let lengths = speaking
                .iter()
                .map(|voice| LINE_SECONDS * speech_pages(&voice.line).len() as f32)
                .collect::<Vec<_>>();
            let round = lengths.iter().sum::<f32>();
            // As a World opens, from the first thing said.
            let mut left = since(self.looking.opening.or(self.looking.started)) % round;
            let mut chosen = None;
            for (voice, length) in speaking.iter().zip(&lengths) {
                if left < *length {
                    chosen = Some((voice, *length));
                    break;
                }
                left -= length;
            }
            chosen.map(|(voice, length)| {
                (
                    voice.speaker,
                    voice.line.clone(),
                    left.min(length),
                    false,
                    length,
                )
            })
        };
        // At most two lines at a time: a long line is said a page at a
        // time, each page for a line's while.
        let line = line.map(|(who, text, age, strong, length)| {
            let pages = speech_pages(&text);
            let page = ((age / LINE_SECONDS) as usize).min(pages.len() - 1);
            let page_age = if strong {
                age
            } else {
                age - page as f32 * LINE_SECONDS
            };
            let last = page + 1 == pages.len();
            let fade = if strong {
                (age / 0.25).clamp(0.0, 1.0)
            } else {
                let fade_in = if page == 0 { page_age / 0.35 } else { 1.0 };
                let fade_out = if last { (length - age) / 0.45 } else { 1.0 };
                fade_in.min(fade_out).clamp(0.0, 1.0)
            };
            (who, pages[page].clone(), fade, strong)
        });
        // A new line is heard in its speaker's voice as it appears.
        if let Some((who, text, ..)) = &line {
            let heard = self
                .looking
                .babbled
                .as_ref()
                .is_some_and(|(before, words)| before == who && words == text);
            if !heard {
                self.looking.babbled = Some((*who, text.clone()));
                self.cue(crate::babble(*who, text));
            }
        }
        let card_people = self
            .card_command()
            .map(|command| asker_ids(&self.snapshot, command.asker))
            .unwrap_or_default();
        let mut pinned = card_people.iter().copied().collect::<BTreeSet<_>>();
        pinned.extend(self.looking.asking);
        pinned.extend(line.as_ref().map(|(who, ..)| *who));
        pinned.extend(self.glows().keys().copied());
        let before = self.before_turn.as_ref().filter(|_| !still).map(|before| {
            (
                diorama::stage(before, width, height),
                before,
                since(self.looking.turn_at) / diorama::WALK_SECONDS,
            )
        });
        // Everyone drawn as the scene has them look, and whoever of its
        // cast has since gone drawn too: the World as the scene draws it.
        let drawn = scene_world.as_deref().unwrap_or(&self.snapshot);
        let mut living = diorama::living_staged(
            &stage,
            drawn,
            seconds,
            daylight,
            &pinned,
            before
                .as_ref()
                .map(|(stage, snapshot, progress)| (stage, *snapshot, *progress)),
            staged.as_ref(),
        );
        let poked = self
            .looking
            .poked
            .map(|(who, at)| [(who, at.elapsed().as_secs_f32())].into_iter().collect())
            .unwrap_or_default();
        diorama::wave(&mut living, &stage, drawn, &poked, still);
        let grew = self
            .before_turn
            .as_ref()
            .is_some_and(|before| before.canvas.marks.len() < self.snapshot.canvas.marks.len());
        let rising = if grew {
            (since(self.looking.turn_at) / RISE_SECONDS).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let mut frame = diorama::frame(
            drawn,
            &stage,
            &living,
            camera,
            seconds,
            daylight,
            &self.glows(),
            rising,
        )
        .stilled(still);
        if let Some(staged) = &staged {
            frame = frame.dressed(staged.dress());
        }
        // Where the camera is going: its look is baked ahead.
        if let Some(to) = self.looking.glide.to {
            frame = frame.heading_to(to);
        }
        frame.bounce(&self.snapshot, &poked);
        // What a turn just built rises into place.
        if let Some(before) = &self.before_turn {
            let known = before
                .canvas
                .items
                .iter()
                .map(|item| item.id)
                .collect::<BTreeSet<_>>();
            let fresh = self
                .snapshot
                .canvas
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    item.kind != world_projection::CanvasItemKind::Actor
                        && !known.contains(&item.id)
                })
                .map(|(index, _)| index)
                .collect::<BTreeSet<_>>();
            if !fresh.is_empty() {
                frame.arrive(&fresh, since(self.looking.turn_at) / RISE_SECONDS);
            }
        }
        // Whoever is speaking is drawn talking.
        if let Some((speaker, ..)) = &line {
            let drawn = scene_world.as_deref().unwrap_or(&self.snapshot);
            for person in &mut frame.people {
                if drawn.canvas.items.get(person.index).map(|item| item.id) == Some(*speaker)
                    && person.stance != world_projection::Stance::Walking
                {
                    person.stance = world_projection::Stance::Talking;
                }
            }
        }
        // Everyone nearby turns to whoever is speaking.
        if let Some((speaker, _, fade, _)) = &line {
            if let Some(index) = scene_world
                .as_deref()
                .unwrap_or(&self.snapshot)
                .canvas
                .items
                .iter()
                .position(|item| item.id == *speaker)
            {
                frame.listen(index, *fade);
            }
        }

        self.dress_frame(&mut frame);

        let mut root = div()
            .id("world-stage")
            .relative()
            .size_full()
            .overflow_hidden()
            .track_focus(&focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.touched(cx);
                this.on_key(event, window, cx)
            }))
            .on_scroll_wheel(
                cx.listener(|this, event: &gpui::ScrollWheelEvent, window, cx| {
                    this.touched(cx);
                    this.on_wheel(event, window, cx)
                }),
            )
            .on_pinch(cx.listener(|this, event: &gpui::PinchEvent, window, cx| {
                this.on_pinch(event, window, cx)
            }))
            // Dragging pans along the place.
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                    this.touched(cx);
                    this.drag_from(f32::from(event.position.x))
                }),
            )
            .on_mouse_move(
                cx.listener(|this, event: &gpui::MouseMoveEvent, window, cx| {
                    this.touched(cx);
                    if event.pressed_button == Some(gpui::MouseButton::Left) {
                        this.drag_to(f32::from(event.position.x), window, cx);
                    } else if this.looking.drag.is_some() {
                        this.drag_end();
                    }
                }),
            )
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _: &gpui::MouseUpEvent, _, _| this.drag_end()),
            )
            .child({
                // The scene: the living stage, its looks baked off this
                // thread as the camera needs them, everyone in depth.
                let started = Instant::now();
                let scene =
                    diorama::stage_view::living_scene(frame.clone(), &self.snapshot, window);
                let planned = started.elapsed();
                scene.child(
                    canvas(|_, _, _| (), move |_, _, _, _| frame_log(planned))
                        .absolute()
                        .size_0(),
                )
            })
            // Clicking the open ground puts away whatever is open.
            .child(
                div()
                    .id("world-ground")
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .on_click(cx.listener(|this, event: &gpui::ClickEvent, window, cx| {
                        if this.just_dragged() || this.on_a_control(event.position(), window) {
                            return;
                        }
                        if !this.place_on_ground(event.position(), window, cx) {
                            let marking = &mut this.looking.marking;
                            if marking.design.is_none() {
                                marking.card = None;
                                marking.offers = None;
                                marking.naming = None;
                                marking.plot_focus = None;
                            }
                            this.look_away(cx);
                        }
                    })),
            );

        // Nothing is named before it is painted (the art bible's §6).
        let painted = diorama::painted(window);
        // Buildings: named when pointed at, opening the drawer on a click.
        for spot in &stage.buildings {
            if !stage.shows(spot.index, camera.zoom) {
                continue;
            }
            let item = &self.snapshot.canvas.items[spot.index];
            let (x, base) = camera.at(&stage, spot.x, spot.y);
            let w = spot.w * camera.zoom;
            let h = stage.height_of(spot, item.shape.unwrap_or_default()) * camera.zoom;
            let selection = item.id;
            let group = SharedString::from(format!("place-{}", selection.stable_key()));
            let named = frame_glows(&frame, spot.index);
            root = root.child(
                div()
                    .id(SharedString::from(format!(
                        "stage-{}",
                        selection.stable_key()
                    )))
                    .role(Role::Button)
                    .aria_label(label_of(&self.snapshot, selection).unwrap_or_default())
                    .group(group.clone())
                    .absolute()
                    .left(px(x - w / 2.0))
                    .top(px(base - h - 28.0))
                    .w(px(w))
                    .h(px(h + 28.0))
                    .cursor_pointer()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(name_tag(item.label.clone()).when(!named, |tag| {
                        tag.opacity(0.0).when(painted, |tag| {
                            tag.group_hover(group, |style| style.opacity(1.0))
                        })
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        if this.place_at(selection, cx) {
                            return;
                        }
                        if this.open_mark_card(selection, cx) {
                            return;
                        }
                        this.looking.poked = Some((selection, Instant::now()));
                        // A home or a work is part of the place, not
                        // something the World records: it springs and says
                        // its name, and opens nothing.
                        if !part_of_the_place(selection) {
                            this.select(selection, cx);
                            this.open_drawer();
                        } else {
                            cx.notify();
                        }
                    })),
            );
        }
        // Plots to build on, and things with a card of their own.
        root = root.children(self.mark_targets(&stage, camera, cx));
        // People: click to ask them something. Each can be pointed at
        // only as far as half way to a neighbour, so a click finds the
        // person nearest the pointer and two are never named at once.
        let mut heads = Vec::new();
        let mut bodies = Vec::new();
        let standing = frame
            .people
            .iter()
            .map(|person| (person.x, person.y, person.height))
            .collect::<Vec<_>>();
        let reach = diorama::reach_of(&standing);
        // Named on hover, or while speaking; never two names over one
        // another, and none before the scene is painted.
        let drawn = scene_world.as_deref().unwrap_or(&self.snapshot);
        let speaker_x = line.as_ref().and_then(|(who, ..)| {
            frame
                .people
                .iter()
                .find(|person| {
                    drawn.canvas.items.get(person.index).map(|item| item.id) == Some(*who)
                })
                .map(|person| person.x)
        });
        for (nth, (person, reach)) in frame.people.iter().zip(reach).enumerate() {
            let Some(item) = drawn.canvas.items.get(person.index) else {
                continue;
            };
            let selection = item.id;
            let group = SharedString::from(format!("person-{}", selection.stable_key()));
            let speaking = line.as_ref().is_some_and(|(who, ..)| *who == selection);
            let named = painted && speaking;
            let hover =
                painted && !speaking && speaker_x.is_none_or(|x| (x - person.x).abs() > NAME_ROOM);
            heads.push((selection, person.x, person.y - person.height * 1.08));
            bodies.push((selection, person.x, person.y - person.height, person.y));
            // Someone a scene brings back who is gone from the World now:
            // drawn, and spoken over, but nobody to ask anything.
            if person.index >= self.snapshot.canvas.items.len() {
                continue;
            }
            root = root.child(
                div()
                    .id(SharedString::from(format!(
                        "stage-{}",
                        selection.stable_key()
                    )))
                    .role(Role::Button)
                    .aria_label(label_of(&self.snapshot, selection).unwrap_or_default())
                    .group(group.clone())
                    .absolute()
                    .left(px(person.x - reach))
                    .top(px(person.y - person.height))
                    .w(px(reach * 2.0))
                    .h(px(person.height + 24.0))
                    .cursor_pointer()
                    .flex()
                    .flex_col()
                    .justify_end()
                    .items_center()
                    .child({
                        // A name stands at their feet, in one box whether
                        // centred or slid aside. Whoever speaks has theirs
                        // where it covers nobody: under their feet, slid
                        // aside, or beside them; or, nowhere being clear, not
                        // at all (v0.30 S1d round 2). Only theirs is placed.
                        let name = first_name(&item.label);
                        let w = diorama::name_w(&name, crate::text_scale());
                        let h_name = diorama::NAME_H * crate::text_scale().max(1.0);
                        let under = person.x - w / 2.0;
                        let spot = if named {
                            diorama::name_spot(&standing, nth, (w, h_name))
                        } else {
                            Some((under, person.y))
                        };
                        let (left, top) = spot.unwrap_or((under, person.y));
                        let tag = name_tag(name)
                            .mt_0()
                            .when(named && spot.is_none(), |tag| tag.opacity(0.0))
                            .when(!named, |tag| {
                                tag.opacity(0.0).when(hover, |tag| {
                                    tag.group_hover(group, |style| style.opacity(1.0))
                                })
                            });
                        let place = div().absolute().top(px(top - (person.y - person.height)));
                        if (left - under).abs() < 0.5 {
                            place.left_0().right_0().flex().justify_center().child(tag)
                        } else if left > person.x {
                            place.left(px(left - (person.x - reach))).child(tag)
                        } else {
                            place.right(px(person.x + reach - (left + w))).child(tag)
                        }
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        if this.place_at(selection, cx) {
                            return;
                        }
                        this.ask(selection, cx);
                    })),
            );
        }
        // Whom a favour is for: a mark over their head until it is done.
        if let Some((_, x, _, feet)) = super::arrival::marked(&self.snapshot)
            .filter(|_| self.looking.painted && self.retelling.is_none())
            .and_then(|whom| bodies.iter().find(|(id, ..)| *id == whom))
            .filter(|(_, x, top, feet)| {
                // Only where they stand in view, drawn big enough to be
                // seen: never a mark over empty ground.
                on_screen(*x, *feet, (width, height)) && feet - top >= 12.0
            })
        {
            let hint = self
                .snapshot
                .favour
                .as_ref()
                .map(|favour| favour.hint.clone())
                .unwrap_or_default();
            root = root.child(favour_mark(*x, *feet, hint, seconds));
        }
        // Where a line said now goes: over its speaker, clear of the
        // interface; and so no pointer covers it.
        let asker_x = card_people
            .iter()
            .find_map(|who| heads.iter().find(|(id, ..)| id == who))
            .map(|(_, x, _)| *x)
            .filter(|_| self.retelling.is_none());
        let asking = self
            .looking
            .asking
            .and_then(|who| heads.iter().find(|(id, ..)| *id == who).copied());
        let crowd = self.crowd(&bodies, height);
        let mut interface = self.interface_areas((width, height), asker_x, asking, &crowd);
        // Nor over whoever a card is about, or the person being talked to.
        for who in card_people.iter().chain(self.looking.asking.iter()) {
            if line.as_ref().is_some_and(|(speaker, ..)| speaker == who) {
                continue;
            }
            if let Some((_, x, top, feet)) = bodies.iter().find(|(id, ..)| id == who) {
                interface.push(Area {
                    x: x - 14.0,
                    y: *top,
                    w: 28.0,
                    h: feet - top,
                });
            }
        }
        // The words as the text system measures them, in even lines.
        let laid = line
            .as_ref()
            .map(|(_, text, _, strong)| lay_bubble(window, text, *strong));
        let speaker = line
            .as_ref()
            .and_then(|(who, ..)| heads.iter().find(|(id, ..)| id == who).copied());
        let seen = speaker.filter(|(_, x, y)| on_screen(*x, *y, (width, height)));
        let placed = seen.zip(laid.as_ref()).and_then(|((_, x, y), (_, w, h))| {
            place_bubbles(&[(x, y, *w, *h)], &interface, (width, height)).pop()
        });
        // A speaker off the screen speaks from its edge, with their face.
        let tab = match (speaker, seen, laid.as_ref()) {
            (Some((_, x, y)), None, Some((_, w, h))) => {
                Some(place_tab(x, y, (*w, *h), &interface, (width, height)))
            }
            _ => None,
        };
        let pointing = self.point(
            width,
            height,
            placed
                .map(|placed| placed.whole())
                .or(tab.map(|(area, _)| area)),
        );
        // Whoever is talking, over their head, once there is a place
        // under them to talk in.
        if let (Some((who, _, fade, strong)), Some((rows, ..)), true) = (&line, laid, painted) {
            if let Some(placed) = placed {
                root = root.child(bubble(rows, placed, *fade, *strong));
            } else if let Some((area, left)) = tab {
                root = root.child(edge_tab(
                    rows,
                    area,
                    left,
                    likeness_of(&self.snapshot, *who),
                    *fade,
                    *strong,
                ));
            }
        }

        // Something handed over while the player watches is shown for a
        // moment, with a chime, before it goes into the drawer.
        let held = self.snapshot.keepsakes.len();
        if self.looking.keepsakes_seen.is_some_and(|seen| held > seen) {
            self.looking.gift_at = Some(Instant::now());
            self.cue(crate::Cue::Keepsake);
        }
        self.looking.keepsakes_seen = Some(held);
        if let Some((keepsake, at)) = self
            .looking
            .gift_at
            .filter(|at| at.elapsed().as_secs_f32() < GIFT_SECONDS)
            .and_then(|at| Some((self.snapshot.keepsakes.last()?, at)))
        {
            let age = at.elapsed().as_secs_f32();
            let from = label_of(&self.snapshot, keepsake.from).unwrap_or_default();
            // It rises in on a spring and fades away at the end.
            let fading = ((GIFT_SECONDS - age) / 0.6).clamp(0.0, 1.0);
            let gift = world_theme::reading(|| {
                div()
                    .id("gift-shown")
                    .role(gpui::Role::Status)
                    .aria_label(format!(
                        "{} {}",
                        crate::i18n::gave_you(&from),
                        keepsake.what
                    ))
                    .max_w(px(420.0))
                    .px_4()
                    .py_2()
                    .rounded_xl()
                    .bg(color(tokens::SURFACE))
                    .shadow_md()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(ui::caption(crate::i18n::gave_you(&from)))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(color(tokens::TEXT))
                            .child(crate::wrap::text(capitalized(&keepsake.what))),
                    )
            });
            root = root.child(
                div()
                    .absolute()
                    .top(px(72.0))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .opacity(fading)
                    .child(ui::spring_in(
                        gift,
                        format!("gift-{held}"),
                        cx.reduce_motion(),
                    )),
            );
        }
        if let Some(card) = &self.looking.postcard {
            root = root.child(postcard_paper(card, width, height));
        }
        if self.looking.photographing {
            return root.into_any_element();
        }
        let saved = [
            (self.looking.photo_saved, "Photo saved to Pictures"),
            (self.looking.postcard_saved, "Postcard saved to Pictures"),
            (self.looking.postcard_failed, "Couldn't save the postcard"),
        ]
        .into_iter()
        .filter_map(|(at, words)| Some((at?, words)))
        .max_by_key(|(at, _)| *at);
        // What was just saved says so over everything else, cards and
        // papers included, so it is always read (v0.31: the postcard's
        // word was hidden behind the card at the bottom): it hangs under
        // the handles, clear of the cards (or, under the sitting's paper
        // hanging from the top, at the bottom), and added last it is drawn
        // last.
        let low = self.sitting_covers();
        let toast = saved
            .filter(|(at, _)| at.elapsed().as_secs_f32() < 2.5)
            .map(|(at, words)| {
                let age = at.elapsed().as_secs_f32();
                div()
                    .absolute()
                    .when(low, |toast| toast.bottom(px(24.0)))
                    .when(!low, |toast| toast.top(px(TOAST_TOP)))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .opacity((age / 0.2).min((2.5 - age) / 0.5).clamp(0.0, 1.0))
                    .child(
                        pill()
                            .debug_selector(|| "saved-toast".into())
                            .occlude()
                            .child(ui::t(words)),
                    )
            });
        if let Some(farewell) = lit(|| self.render_farewell(cx)) {
            // A farewell has the place to itself: no card, no handles.
            return root.child(farewell).children(toast).into_any_element();
        }
        if let Some(sitting) = lit(|| self.render_sitting(cx)) {
            // The morning and the Goodnight card each have the place to
            // themselves: no card, no handles.
            return root.child(sitting).children(toast).into_any_element();
        }
        // The World asleep is the town at night, under its handles: the
        // drawer, a photo and a postcard stay open (1.0 P2).
        if let Some(night) = lit(|| self.render_night(cx)) {
            root = root.child(night);
        }
        // The handles, the gauges and the zoom quieten at rest (1.0 P5).
        let (chrome, fading) = self.chrome_now(pointing.is_some(), still);
        if fading {
            self.request_frame(window, cx);
        }
        root = root.child(self.render_hud(pointing, cx).opacity(chrome));
        if !is_beginning(&self.snapshot) && self.retelling.is_none() {
            let hint = pointing
                .filter(|(pointer, _)| *pointer == Pointer::Zoom)
                .map(|(pointer, age)| self.pointer_hint(pointer, age, Caret::Left, cx));
            root = root.child(
                div()
                    .absolute()
                    .left(px(16.0))
                    .top(px(zoom_top(height)))
                    .flex()
                    .items_center()
                    .gap_1()
                    .opacity(chrome)
                    .child(self.render_zoom(cx))
                    .children(hint),
            );
        }
        // What the player reads (the cards, the drawer, the pages) keeps
        // the window's own light at night; what hangs over the scene (the
        // handles, the zoom, names and speech) is the night's.
        if let Some(beginning) = lit(|| self.render_beginning(width >= 760.0, cx)) {
            root = root.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .p_6()
                    .flex()
                    .justify_center()
                    .child(div().w_full().max_w(px(1040.0)).child(beginning)),
            );
        } else {
            // The card stays in view beside an open drawer.
            let room = width
                - if self.looking.drawer {
                    DRAWER_WIDTH
                } else {
                    0.0
                };
            // The player's mark, while it is open, has the stage to itself.
            let marking = &self.looking.marking;
            // One card at a time: while someone is being talked to, theirs.
            let card = if self.moment_up()
                || self.looking.asking.is_some()
                || self.reading.page.is_some()
                || marking.design.is_some()
                || marking.card.is_some()
                || marking.offers.is_some()
            {
                None
            } else if self.retelling.is_some() {
                lit(|| self.render_retelling(cx)).filter(|_| film_painted)
            } else if let Some(chapter) = lit(|| self.render_chapter_end(cx)) {
                Some(chapter)
            } else if let Some(year) = lit(|| self.render_year_end(cx)) {
                Some(year)
            } else {
                // The evening has its own card: the day is done, and
                // Goodnight is the player's to say.
                lit(|| self.render_evening(cx)).or_else(|| lit(|| self.render_card(cx)))
            };
            // A pointer at the cards sits just above them, never on them.
            let card = card.map(|card| match pointing {
                Some((Pointer::Cards, age)) if self.retelling.is_none() => div()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap_1()
                    .child(self.pointer_hint(Pointer::Cards, age, Caret::Down, cx))
                    .child(card),
                _ => card,
            });
            if let Some(card) = card {
                // The card stands aside from whoever asks, so they and the
                // quay they stand on stay in view above and beside it.
                let asker_x = card_people
                    .iter()
                    .find_map(|who| heads.iter().find(|(id, ..)| id == who))
                    .map(|(_, x, _)| *x)
                    .filter(|_| self.retelling.is_none());
                // A return beat's card stays where it first docked for the
                // whole beat, however its scene's people move.
                let dock = match self.retelling {
                    Some(index) => {
                        let key = (self.revision, index);
                        match self.looking.film_dock {
                            Some((at, dock)) if at == key => dock,
                            _ => {
                                let dock = card_dock(room, asker_x, &crowd);
                                self.looking.film_dock = Some((key, dock));
                                dock
                            }
                        }
                    }
                    None => card_dock(room, asker_x, &crowd),
                };
                root = root.child(
                    bottom_card(card, dock)
                        .when(self.looking.drawer, |card| card.right(px(DRAWER_WIDTH))),
                );
            }
        }
        // The talk card waits for the camera to land on whoever it is
        // for, as the film's words do (v0.29 round 2: Find's card opened
        // over the old view, its speaker not yet on screen).
        let landed = |who: SelectionId| {
            crate::painter::synchronous()
                || self.retelling.is_some()
                || self
                    .looking
                    .glide
                    .arrived(Bound::Asking(who), Instant::now())
        };
        if let Some((who, x, y)) = self
            .looking
            .asking
            .filter(|who| landed(*who))
            .and_then(|who| heads.iter().find(|(id, ..)| *id == who).copied())
        {
            root = root.child(lit_one(|| self.render_asking(who, x, y, &stage, cx)));
        }
        if let Some(offers) = lit(|| self.render_offers(&stage, camera, cx)) {
            root = root.child(offers);
        }
        if let Some(card) = lit(|| self.render_mark_card(&stage, camera, cx)) {
            root = root.child(card);
        }
        let design_target = self.looking.marking.design.as_ref().and_then(|designing| {
            let index = self
                .snapshot
                .canvas
                .items
                .iter()
                .position(|item| item.id == designing.target)?;
            let (x, _, w, _) = stage.frame_of(index)?;
            Some(camera.at(&stage, x + w / 2.0, 0.0).0)
        });
        if let Some(design) = lit(|| self.render_design(width, height, design_target, cx)) {
            root = root.child(design);
        }
        if let Some(hands) = lit(|| self.render_hands(cx)) {
            root = root.child(hands);
        }
        if self.looking.drawer {
            root = root.child(world_theme::reading(|| self.render_drawer(cx)));
        }
        if let Some(strip) = lit(|| self.render_moment_up(width, height, window, cx)) {
            root = root.child(strip);
        }
        if let Some(page) = lit(|| self.render_page(width, height, cx)) {
            root = root.child(page);
        }
        if let Some(status) = lit(|| self.render_status()) {
            root = root.child(
                div()
                    .absolute()
                    .top(px(64.0))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(status.max_w(px(520.0)).shadow_md()),
            );
        }
        root.children(toast).into_any_element()
    }
}
