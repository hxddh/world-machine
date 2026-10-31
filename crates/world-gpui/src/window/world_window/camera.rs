//! Where the camera looks, and the player's hands on the scene: the
//! camera's glide, zooming and panning (wheel, keys, dragging, a
//! pinch), what glows, placing and undoing a deed, photos and
//! postcards, and the books the window keeps as it paints.

use super::*;

impl ProjectionView {
    /// Where the camera is now, moving toward where it was last sent.
    pub(super) fn camera(
        &mut self,
        stage: &Stage,
        window: &Window,
        scene: Option<Camera>,
    ) -> Camera {
        let whole = Camera::whole(stage);
        // Whoever was being talked to is put away (Esc, the close button,
        // the drawer, a page): the camera stays where it is, never jumping
        // back to wherever the player had looked before.
        if self.looking.asking.is_none() && self.current_beat().is_none() {
            if let Some(held) = self.looking.held.take() {
                let now = self.looking.camera_now.unwrap_or(held);
                self.looking.zoom = now.zoom;
                self.looking.pan = Some(now.x);
                self.looking.zoom_on = (now.x, now.y);
            }
        } else if self.looking.asking.is_some() && self.current_beat().is_none() {
            self.looking.held = self.looking.camera_now.or(Some(whole));
        }
        // A return looks where its beat's scene looks, or else at
        // whatever its beat is about.
        let target = scene
            .filter(|_| self.current_beat().is_some())
            .or_else(|| {
                self.current_beat()
                    .and_then(|beat| beat_box(&self.snapshot, stage, beat))
                    .map(|subject| Camera::on(stage, subject))
            })
            // Talking to someone, the camera moves in on them.
            .or_else(|| {
                let who = self.looking.asking?;
                let index = self
                    .snapshot
                    .canvas
                    .items
                    .iter()
                    .position(|item| item.id == who)?;
                let (x, y, w, h) = stage.frame_of(index)?;
                let framed = Camera::on(stage, (x - w, y, w * 3.0, h));
                Some(Camera::around(
                    stage,
                    framed.zoom.min(Camera::rest_zoom(stage) * 1.15),
                    framed.x,
                    framed.y,
                ))
            })
            // Otherwise wherever the player has zoomed and panned to.
            .unwrap_or_else(|| self.free_target(stage));
        let bound = match (self.retelling, self.looking.asking) {
            (Some(index), _) if self.current_beat().is_some() => Bound::Beat(index),
            (_, Some(who)) => Bound::Asking(who),
            _ => Bound::Free,
        };
        // A drag moves the view with the hand, at once; and the window's
        // first frame is where the camera is meant to be, never a glide
        // there from somewhere else (the welcome's view, a keeper's).
        let snap = self.looking.drag.is_some_and(|(.., moved)| moved);
        let (current, wants) =
            self.looking
                .glide
                .step(target, bound, Instant::now(), whole, snap, |to| {
                    diorama::heading_ready(window, to)
                });
        // A glide asks for its own frames: a window in front but not
        // active draws only when asked, and a glide whose destination was
        // painted ahead asks the painter for nothing (v0.29 round 2's
        // silent beats).
        if wants {
            window.request_animation_frame();
        }
        self.looking.camera_now = Some(current);
        current
    }

    /// The wheel zooms in on the place around the pointer, and back out;
    /// a sideways scroll (or the wheel with shift held) pans along the
    /// place.
    pub(super) fn on_wheel(
        &mut self,
        event: &gpui::ScrollWheelEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let delta = event.delta.pixel_delta(px(16.0));
        let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
        let sideways = if event.modifiers.shift { dy } else { dx };
        if sideways.abs() > dy.abs() * 0.8 || (event.modifiers.shift && dy != 0.0) {
            self.pan_by(-sideways, window, cx);
            return;
        }
        if dy == 0.0 {
            return;
        }
        self.zoom_by(1.0 + dy * 0.004, Some(event.position), window, cx);
    }

    /// How close the player has zoomed: 1 is one window of the place (the
    /// overview), and never zoomed, the camera at rest.
    pub(super) fn view_zoom(&self) -> f32 {
        if self.looking.zoom <= 0.0 {
            if self.looking.stage_h > 0.0 {
                diorama::rest_zoom(diorama::figure_height(self.looking.stage_h))
            } else {
                1.0
            }
        } else {
            self.looking.zoom
        }
    }

    /// Whether the arrow keys pan: always with shift, and otherwise along a
    /// panorama whenever there are no answers to lean between.
    pub(super) fn pans_with_arrows(&self, event: &KeyDownEvent) -> bool {
        let wide = self.snapshot.canvas.width.unwrap_or(1.0) > 1.01 || self.view_zoom() > 1.01;
        wide && (event.keystroke.modifiers.shift || self.card_answers().len() < 2)
    }

    /// Pans by `screens` window widths.
    pub(super) fn pan_by_screens(&mut self, screens: f32, window: &Window, cx: &mut Context<Self>) {
        let (width, _) = self.stage_size(window);
        self.pan_by(screens * width, window, cx);
    }

    /// Pans the view by `dx` window pixels along the place.
    pub(super) fn pan_by(&mut self, dx: f32, window: &Window, cx: &mut Context<Self>) {
        let (width, height) = self.stage_size(window);
        let stage = diorama::stage(&self.snapshot, width, height);
        let camera = self
            .looking
            .camera_now
            .unwrap_or_else(|| Camera::whole(&stage));
        let rest = self.looking.pan.unwrap_or(camera.x);
        let zoom = self.view_zoom();
        let next = Camera::around(&stage, zoom, rest + dx / camera.zoom, camera.y).x;
        if (next - rest).abs() > 0.01 {
            self.looking.pan = Some(next);
            cx.notify();
        }
    }

    /// The pointer went down on the scene: perhaps the start of a drag.
    pub(super) fn drag_from(&mut self, x: f32) {
        let centre = self
            .looking
            .camera_now
            .map(|camera| camera.x)
            .or(self.looking.pan);
        if let Some(centre) = centre {
            self.looking.drag = Some((x, centre, false));
        }
    }

    /// The pointer moved with the button down: the view follows it.
    pub(super) fn drag_to(&mut self, x: f32, window: &Window, cx: &mut Context<Self>) {
        let Some((from, centre, moved)) = self.looking.drag else {
            return;
        };
        let moved = moved || (x - from).abs() > 4.0;
        self.looking.drag = Some((from, centre, moved));
        if !moved {
            return;
        }
        let (width, height) = self.stage_size(window);
        let stage = diorama::stage(&self.snapshot, width, height);
        let zoom = self.view_zoom();
        let y = self
            .looking
            .camera_now
            .map_or(stage.height / 2.0, |camera| camera.y);
        let next = Camera::around(&stage, zoom, centre - (x - from) / zoom, y).x;
        self.looking.pan = Some(next);
        cx.notify();
    }

    /// The pointer came up: a drag, if there was one, is over.
    pub(super) fn drag_end(&mut self) {
        if self.looking.drag.take().is_some_and(|(.., moved)| moved) {
            self.looking.dragged_at = Some(Instant::now());
        }
    }

    /// Whether a click is only the end of a drag.
    pub(crate) fn just_dragged(&self) -> bool {
        self.looking
            .dragged_at
            .is_some_and(|at| at.elapsed() < Duration::from_millis(250))
    }

    /// A pinch on a trackpad zooms in on the place between the fingers,
    /// and back out.
    pub(super) fn on_pinch(
        &mut self,
        event: &gpui::PinchEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if event.delta != 0.0 {
            self.zoom_by(1.0 + event.delta, Some(event.position), window, cx);
        }
    }

    /// Zooms by `factor` on the stage point under `at` (a place in the
    /// window), or on the middle of what is in view.
    pub(super) fn zoom_by(
        &mut self,
        factor: f32,
        at: Option<gpui::Point<gpui::Pixels>>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let (width, height) = self.stage_size(window);
        let stage = diorama::stage(&self.snapshot, width, height);
        let camera = self
            .looking
            .camera_now
            .unwrap_or_else(|| Camera::whole(&stage));
        let (x, y) = at.map_or((width / 2.0, height / 2.0), |at| {
            (f32::from(at.x), f32::from(at.y) - CHROME)
        });
        if self.zoom_at(&stage, camera, factor, (x, y)) {
            pointers::used(Pointer::Zoom);
            cx.notify();
        }
    }

    /// Zooms `factor` times closer from `camera` around the screen point
    /// `at`; whether it moved (not at the end of the zoom's travel).
    pub(super) fn zoom_at(
        &mut self,
        stage: &Stage,
        camera: Camera,
        factor: f32,
        at: (f32, f32),
    ) -> bool {
        let before = self.view_zoom();
        let Some(zoomed) = zoomed(stage, camera, before, factor, at) else {
            return false;
        };
        // Back at rest it is as never zoomed (framed roofs to water); out
        // to the overview, it frames the place around where it was, as
        // v0.29 opened on it.
        let rest = Camera::rest_zoom(stage);
        self.looking.zoom = if zoomed.zoom == rest {
            0.0
        } else {
            zoomed.zoom
        };
        self.looking.pan = Some(if zoomed.zoom <= 1.0 && before > 1.0 {
            self.overview_view(stage, zoomed.pan)
        } else {
            zoomed.pan
        });
        self.looking.zoom_on = zoomed.on;
        true
    }

    /// Wherever the player has zoomed and panned to.
    pub(super) fn free_target(&self, stage: &Stage) -> Camera {
        free_view(
            stage,
            (self.looking.zoom > 0.0).then_some(self.looking.zoom),
            self.looking.pan,
            self.looking.zoom_on,
        )
    }

    /// What the scene lights up now, and in what colour.
    pub(super) fn glows(&self) -> Glows {
        let mut glows = Glows::new();
        // Where the player's hands can put what they picked.
        if let Some(targets) = self.hand_targets() {
            let colour: Hsla = color(tokens::ACCENT).into();
            for target in targets {
                glows.insert(target, colour);
            }
            return glows;
        }
        if let Some(beat) = self.current_beat() {
            let colour: Hsla = color(scene::tone_token(beat.tone)).into();
            for target in beat_targets(&self.snapshot, beat) {
                glows.insert(target, colour);
            }
            return glows;
        }
        if let Some(asking) = self.looking.asking {
            glows.insert(asking, color(tokens::ACCENT).into());
            return glows;
        }
        if let Some(whom) = super::arrival::marked(&self.snapshot) {
            glows.insert(whom, color(tokens::ACCENT).into());
        }
        if let Some(command) = self.card_command() {
            let colour: Hsla = color(tokens::ACCENT).into();
            for target in command
                .effects
                .iter()
                .filter_map(|effect| effect.target)
                .chain(asker_ids(&self.snapshot, command.asker))
            {
                glows.insert(target, colour);
            }
        }
        glows
    }

    /// While the player is placing something, everywhere it can go.
    pub(super) fn hand_targets(&self) -> Option<Vec<SelectionId>> {
        let hands = self.looking.hands.as_ref()?;
        let (verb, thing) = (hands.verb.as_deref()?, hands.thing.as_deref()?);
        Some(
            self.snapshot
                .deeds()
                .filter(|(_, command, hand)| {
                    command.unavailable.is_none()
                        && hand.verb == verb
                        && (thing == "*" || which(command, hand) == thing)
                })
                .filter_map(|(_, _, hand)| hand.at)
                .collect(),
        )
    }

    /// The deed that puts what the player picked at `target`, if there is
    /// one.
    pub(super) fn deed_at(&self, target: SelectionId) -> Option<String> {
        let hands = self.looking.hands.as_ref()?;
        let (verb, thing) = (hands.verb.as_deref()?, hands.thing.as_deref()?);
        self.snapshot
            .deeds()
            .find(|(_, command, hand)| {
                command.unavailable.is_none()
                    && hand.verb == verb
                    && (thing == "*" || which(command, hand) == thing)
                    && hand.at == Some(target)
            })
            .map(|(_, command, _)| command.id.clone())
    }

    /// Clicking something on the scene while placing does the deed there,
    /// and puts the player's hands away.
    pub(crate) fn place_at(&mut self, target: SelectionId, cx: &mut Context<Self>) -> bool {
        let Some(deed) = self.deed_at(target) else {
            return false;
        };
        self.looking.hands = None;
        self.invoke_command(deed, cx);
        true
    }

    /// Clicking open ground while placing puts what the player picked right
    /// there: counted with the nearest place, standing at the spot along
    /// the ground that was clicked.
    pub(crate) fn place_on_ground(
        &mut self,
        position: gpui::Point<gpui::Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.placing() {
            return false;
        }
        let (width, height) = self.stage_size(window);
        let stage = diorama::stage(&self.snapshot, width, height);
        let camera = self
            .looking
            .camera_now
            .unwrap_or_else(|| Camera::whole(&stage));
        let (x, _) = camera.stage_point(
            &stage,
            f32::from(position.x),
            f32::from(position.y) - CHROME,
        );
        self.place_hands_at(&stage, x, cx)
    }

    /// Puts what the player's hands hold at stage point `x` along the
    /// ground: counted with the nearest place it can go by, standing at
    /// that spot. Whether it was put.
    pub(crate) fn place_hands_at(&mut self, stage: &Stage, x: f32, cx: &mut Context<Self>) -> bool {
        let targets = self.hand_targets().unwrap_or_default();
        let nearest = stage
            .buildings
            .iter()
            .filter(|spot| targets.contains(&self.snapshot.canvas.items[spot.index].id))
            .min_by(|a, b| (a.x - x).abs().total_cmp(&(b.x - x).abs()));
        let Some(nearest) = nearest else {
            return false;
        };
        let Some(deed) = self.deed_at(self.snapshot.canvas.items[nearest.index].id) else {
            return false;
        };
        let spot = diorama::ground_spot(stage, x);
        self.looking.hands = None;
        self.invoke_command(format!("{deed}@{spot}"), cx);
        true
    }

    /// Whether a window point is on the interface over the scene (the
    /// row of gauges and handles, the zoom): a click there is never a
    /// click on the ground.
    pub(super) fn on_a_control(
        &self,
        position: gpui::Point<gpui::Pixels>,
        window: &Window,
    ) -> bool {
        let (_, height) = self.stage_size(window);
        let (x, y) = (f32::from(position.x), f32::from(position.y) - CHROME);
        let zoom = Area {
            x: 16.0 - BUBBLE_CLEAR,
            y: zoom_top(height) - BUBBLE_CLEAR,
            w: ZOOM_SIZE.0 + BUBBLE_CLEAR * 2.0,
            h: ZOOM_SIZE.1 + BUBBLE_CLEAR * 2.0,
        };
        let inside = |area: &Area| {
            x >= area.x && x <= area.x + area.w && y >= area.y && y <= area.y + area.h
        };
        y < HUD_ROOM || inside(&zoom)
    }

    /// Whether the player's hands hold something to put on the ground.
    pub(crate) fn placing(&self) -> bool {
        self.looking.hands.as_ref().is_some_and(|hands| {
            matches!(
                hands.verb.as_deref(),
                Some("Build" | "Decorate" | "Plant" | "Move")
            ) && hands.thing.as_deref().is_some_and(|thing| thing != "*")
        })
    }

    /// The player's last thing made or moved, which they can take back
    /// this period: the command, and what it says.
    pub(super) fn undo_command(&self) -> Option<(String, String)> {
        self.snapshot
            .deeds()
            .find(|(_, command, hand)| hand.verb == "Undo" && command.unavailable.is_none())
            .map(|(_, command, _)| (command.id.clone(), command.title.clone()))
    }

    pub(super) fn undo(&mut self, cx: &mut Context<Self>) {
        if let Some((command, _)) = self.undo_command() {
            self.invoke_command(command, cx);
        }
    }

    /// A photograph of the scene: the chrome is hidden for a frame, the
    /// window's picture is saved to Pictures, and the chrome comes back.
    pub(super) fn take_photo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.looking.photographing {
            return;
        }
        self.looking.photographing = true;
        let bounds = window.bounds();
        let title = self.snapshot.title.clone();
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(180))
                .await;
            let saved = cx
                .background_executor()
                .spawn(async move { save_photo(bounds, &title) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.looking.photographing = false;
                if saved {
                    this.looking.photo_saved = Some(Instant::now());
                    this.cue(crate::Cue::Flip);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// A postcard: the moment picked in History (or the scene now) with
    /// what a resident said then as its caption, printed on paper around
    /// the scene and saved to Pictures like a photograph. Only this view's
    /// own area is saved, without the window's bars.
    pub(crate) fn take_postcard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.looking.photographing {
            return;
        }
        let moment = self.selected.filter(|selected| {
            self.snapshot
                .timeline
                .items
                .iter()
                .any(|item| item.id == *selected)
        });
        let card = crate::postcard::postcard(&self.snapshot, moment);
        let stem = card.file_stem();
        self.looking.postcard = Some(card);
        self.looking.photographing = true;
        let bounds = window.bounds();
        let viewport = window.viewport_size();
        // The window's own title bar is what its frame has beyond its
        // content; the World's bar sits under it.
        let bars = (f32::from(bounds.size.height) - f32::from(viewport.height)).max(0.0) + CHROME;
        let region = gpui::Bounds::new(
            gpui::point(bounds.origin.x, bounds.origin.y + px(bars)),
            gpui::size(
                bounds.size.width,
                (bounds.size.height - px(bars)).max(px(1.0)),
            ),
        );
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(180))
                .await;
            let saved = cx
                .background_executor()
                .spawn(async move { save_picture(region, &stem, false) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.looking.photographing = false;
                this.looking.postcard = None;
                if saved {
                    this.looking.postcard_saved = Some(Instant::now());
                    this.looking.postcard_failed = None;
                    this.cue(crate::Cue::Flip);
                } else {
                    this.looking.postcard_failed = Some(Instant::now());
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn toggle_hands(&mut self, cx: &mut Context<Self>) {
        self.looking.hands = match self.looking.hands {
            Some(_) => None,
            None => {
                pointers::used(Pointer::Hands);
                let verb = VERBS
                    .into_iter()
                    .find(|verb| self.snapshot.deeds().any(|(_, _, hand)| hand.verb == *verb))
                    .map(str::to_string);
                Some(Hands {
                    thing: verb
                        .as_deref()
                        .filter(|verb| to_someone(verb))
                        .map(|_| "*".into()),
                    verb,
                })
            }
        };
        self.looking.asking = None;
        self.looking.marking.offers = None;
        cx.notify();
    }

    pub(crate) fn card_command(&self) -> Option<&ProjectionCommand> {
        if self.controller.is_none() || is_beginning(&self.snapshot) || self.retelling.is_some() {
            return None;
        }
        // The first day's free thing comes before any card.
        if self.looking.free_offer {
            return None;
        }
        if self
            .looking
            .opening
            .is_some_and(|at| since(Some(at)) < question_waits(&self.snapshot))
        {
            return None;
        }
        let answers = self.card_answers();
        let answer = answers.get(self.looking.answer.min(answers.len().saturating_sub(1)))?;
        self.snapshot.commands.get(*answer)
    }

    /// The window's bookkeeping for the frame about to be drawn, kept out
    /// of `render_world` so drawing only reads: the return film's beats,
    /// what has been painted, the first day's free offer, a favour's beats,
    /// the farewell's camera and the moments not yet seen. Returns whether
    /// what the camera sees is painted, which the film's words wait for.
    pub(super) fn keep_books(
        &mut self,
        window: &mut Window,
        stage: &Stage,
        cx: &mut Context<Self>,
    ) -> bool {
        // A return plays by itself, one beat after another. A beat begins
        // only on a painted scene: while what the camera sees is not
        // painted (the window opening, the camera arriving somewhere new),
        // its clock is held and its words wait.
        // Nor while the camera is still on its way to the beat (held until
        // what it will see is painted, then gliding): the words never show
        // over the last beat's subject (v0.29's catch over the school).
        let arrived = crate::painter::synchronous()
            || self.retelling.is_some_and(|index| {
                self.looking
                    .glide
                    .arrived(Bound::Beat(index), Instant::now())
            });
        let film_painted = diorama::view_painted(window) && arrived;
        // A beat that is a recorded Event plays its scene, for as long as
        // the scene plays; any other beat is told for a beat's while.
        let beat_seconds = self.beat_seconds();
        if self.retelling.is_some() {
            if self.looking.beat_at.is_none() || !film_painted {
                self.looking.beat_at = Some(Instant::now());
                // Words still waiting ask for the next frame themselves:
                // whether the view is painted is known only from the frame
                // drawn before, and a window in front but not active draws
                // only when asked (v0.29 round 2: a beat landed on painted
                // ground, nothing asked for another frame, and its words
                // never came).
                window.request_animation_frame();
            }
            if self.beat_clock() > beat_seconds {
                self.step_retelling(cx);
            }
        }
        self.notice_paint(stage, diorama::painted(window));
        self.notice_free_offer();
        self.notice_favour(stage);
        // A farewell: the camera goes to whoever says goodbye, or, if they
        // are indoors at that hour, to whoever is out on the place, who says
        // it instead.
        if let Some((who, _)) = self
            .farewell_line()
            .filter(|_| !self.looking.farewell_panned)
        {
            self.looking.farewell_panned = true;
            let out = stage
                .people
                .first()
                .and_then(|spot| self.snapshot.canvas.items.get(spot.index))
                .map(|item| item.id);
            let speaker = if self.stage_x(stage, who).is_some() {
                Some(who)
            } else {
                out
            };
            if let Some(speaker) = speaker {
                self.looking.farewell_speaker = Some(speaker);
                self.looking.pan = self.stage_x(stage, speaker);
            }
        }
        self.notice_moments();
        film_painted
    }
}
