//! How the World window answers the player: its clock and its cue, the
//! card and its answers, asking someone and saying something to them,
//! what the window notices as it paints, and the keys.

use super::*;

impl ProjectionView {
    pub(crate) fn stage_size(&self, window: &Window) -> (f32, f32) {
        let size = window.viewport_size();
        (f32::from(size.width), f32::from(size.height) - CHROME)
    }

    /// Start the clock that keeps a living World moving while its window is
    /// in front, once, unless nobody can see the window.
    pub(super) fn keep_living(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.looking.clock.is_some() || !window.is_visible() {
            return;
        }
        self.looking.clock = Some(cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor().timer(FRAME).await;
            let alive = this.update_in(cx, |_, window, cx| {
                let still = cx.reduce_motion();
                if matches!(
                    pace(window.is_visible(), window.is_window_active(), still),
                    Pace::EveryFrame | Pace::Ticking
                ) {
                    cx.notify();
                }
            });
            if alive.is_err() {
                break;
            }
        }));
    }

    /// Asks for the next frame of something moving, at the player's frame
    /// cap ([`crate::frames`]): at the next refresh, or after the wait
    /// that keeps the frames at the cap. Uncapped (and when the painter
    /// draws synchronously, as in tests), every refresh.
    pub(crate) fn request_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cap = if crate::painter::synchronous() {
            None
        } else {
            crate::frames::frame_cap()
        };
        match self.looking.pacer.ask(Instant::now(), cap) {
            crate::frames::Ask::Now => window.request_animation_frame(),
            crate::frames::Ask::After(wait) => {
                cx.spawn_in(window, async move |this, cx| {
                    cx.background_executor().timer(wait).await;
                    let _ = this.update(cx, |this, cx| {
                        this.looking.pacer.fired();
                        cx.notify();
                    });
                })
                .detach();
            }
            crate::frames::Ask::Waiting => {}
        }
    }

    /// Listens, once, for the window being covered or shown: covered, the
    /// clock stops; shown again, the World is drawn at once, and that frame
    /// starts the clock and the pace again.
    pub(super) fn watch_visibility(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.looking.visibility.is_some() {
            return;
        }
        self.looking.visibility = Some(cx.observe_window_visibility(
            window,
            |this, visibility, _, cx| {
                if visibility.is_visible() {
                    cx.notify();
                } else {
                    this.looking.clock = None;
                }
            },
        ));
    }

    pub(crate) fn cue(&mut self, cue: crate::Cue) {
        if let Some(controller) = self.controller.as_mut() {
            controller.cue(cue);
        }
    }

    pub(crate) fn turn_landed(&mut self) {
        let built = self
            .before_turn
            .as_ref()
            .is_some_and(|before| before.canvas.marks.len() < self.snapshot.canvas.marks.len());
        self.cue(if built {
            crate::Cue::Built
        } else {
            crate::Cue::Turn
        });
        self.looking.turn_at = Some(Instant::now());
        // Choosing where a World begins opens it; any other turn is the
        // player's own, and nothing waits for it.
        self.looking.opening = self
            .before_turn
            .as_ref()
            .is_some_and(is_beginning)
            .then(Instant::now);
        self.looking.card = 0;
        self.looking.answer = self.first_available_answer();
        self.looking.card_back = false;
        self.looking.asking = None;
        self.looking.answered = None;
        // A turn goes back to the player's own view of the place.
        self.looking.held = None;
    }

    pub(super) fn cycle_card(&mut self, by: isize, cx: &mut Context<Self>) {
        let count = card_order(&self.snapshot).len();
        if count == 0 {
            return;
        }
        self.looking.card = (self.looking.card as isize + by).rem_euclid(count as isize) as usize;
        self.looking.answer = self.first_available_answer();
        self.looking.card_back = false;
        pointers::used(Pointer::Cards);
        self.cue(crate::Cue::Flip);
        cx.notify();
    }

    /// Turns the card in front over, to what a choice would change, or
    /// back.
    pub(super) fn turn_card(&mut self, cx: &mut Context<Self>) {
        self.looking.card_back = !self.looking.card_back;
        pointers::used(Pointer::Cards);
        cx.notify();
    }

    /// The answers on the card in front, if it is a question.
    pub(super) fn card_answers(&self) -> Vec<usize> {
        let cards = card_order(&self.snapshot);
        cards
            .get(self.looking.card.min(cards.len().saturating_sub(1)))
            .cloned()
            .unwrap_or_default()
    }

    /// The first answer on the card in front that can be chosen, to lean
    /// toward to begin with.
    pub(crate) fn first_available_answer(&self) -> usize {
        self.card_answers()
            .iter()
            .position(|index| {
                self.snapshot
                    .commands
                    .get(*index)
                    .is_some_and(|command| command.unavailable.is_none())
            })
            .unwrap_or(0)
    }

    /// Leans toward the next or previous answer on a question's card, or
    /// turns to the next card when there is only one answer.
    pub(super) fn lean(&mut self, by: isize, cx: &mut Context<Self>) {
        let answers = self.card_answers().len();
        if answers < 2 {
            self.cycle_card(by, cx);
            return;
        }
        self.looking.answer =
            (self.looking.answer as isize + by).rem_euclid(answers as isize) as usize;
        cx.notify();
    }

    pub(super) fn lean_to(&mut self, answer: usize, cx: &mut Context<Self>) {
        if self.looking.answer != answer {
            self.looking.answer = answer;
            cx.notify();
        }
    }

    pub(super) fn choose_card(&mut self, cx: &mut Context<Self>) {
        if self.retelling.is_some() || is_beginning(&self.snapshot) {
            return;
        }
        if let Some(command) = self
            .card_command()
            .filter(|command| command.unavailable.is_none())
        {
            let id = command.id.clone();
            self.invoke_command(id, cx);
        }
    }

    pub(crate) fn toggle_drawer(&mut self, cx: &mut Context<Self>) {
        if self.looking.drawer {
            self.looking.drawer = false;
        } else {
            self.open_drawer();
        }
        cx.notify();
    }

    /// Opens the drawer; with a letter in it, the letter box has been seen.
    pub(crate) fn open_drawer(&mut self) {
        if !self.looking.drawer {
            self.cue(crate::Cue::Drawer);
        }
        self.looking.drawer = true;
        pointers::used(Pointer::Drawer);
        if !self.snapshot.letters.is_empty() {
            pointers::used(Pointer::Letters);
        }
    }

    pub(crate) fn ask(&mut self, who: SelectionId, cx: &mut Context<Self>) {
        // Asleep, everyone is home for the night: nobody is asked.
        if self.asleep() {
            return;
        }
        if self.looking.asking != Some(who) {
            if let Some(input) = &self.looking.say {
                input.update(cx, |input, cx| input.clear(cx));
            }
            self.looking.said_at = None;
        }
        if self.looking.say.is_none() && self.snapshot.capabilities.talk {
            self.looking.say =
                Some(cx.new(|cx| crate::text_input::TextInput::new("Say something…", cx)));
        }
        self.looking.asking = Some(who);
        self.looking.answered = None;
        self.looking.poked = Some((who, Instant::now()));
        self.selected = Some(who);
        cx.notify();
    }

    /// Whether the player is typing what they say to someone.
    pub(super) fn typing(&self, window: &Window, cx: &Context<Self>) -> bool {
        self.looking.asking.is_some()
            && self
                .looking
                .say
                .as_ref()
                .is_some_and(|input| input.focus_handle(cx).is_focused(window))
    }

    /// Says what the player typed to whoever they are talking to. The World
    /// hears it and records the answer; no time passes. With the World
    /// voice on, a model hears it first, off the window's thread: the
    /// window keeps drawing while the person thinks.
    pub(super) fn say(&mut self, cx: &mut Context<Self>) {
        let (Some(who), Some(input)) = (self.looking.asking, self.looking.say.clone()) else {
            return;
        };
        let words = input.read(cx).text().trim().to_string();
        if words.is_empty() || self.retelling.is_some() || self.looking.listening.is_some() {
            return;
        }
        let Some(controller) = self.controller.as_mut() else {
            return;
        };
        let Some(listening) = controller.listen(who, &words) else {
            if self.finish_saying(who, words, Ears::World, cx) {
                input.update(cx, |input, cx| input.clear(cx));
            }
            return;
        };
        self.looking.listening = Some((who, words.clone()));
        input.update(cx, |input, cx| input.clear(cx));
        let started = self.revision;
        let heard = cx
            .background_executor()
            .spawn(async move { crate::listen_within(listening, crate::LISTEN_DEADLINE) });
        cx.spawn(async move |this, cx| {
            let response = heard.await;
            let _ = this.update(cx, |this, cx| {
                this.looking.listening = None;
                // The model heard the World as it stood when the words were
                // said. If a turn or a branch has changed it since, the
                // answer belongs to a moment that is gone.
                if this.revision != started {
                    let name = label_of(&this.snapshot, who)
                        .map(|name| first_name(&name).to_string())
                        .unwrap_or_else(|| "They".into());
                    this.status = Some(crate::i18n::fill(
                        "The moment passed before {name} could answer.",
                        &[("name", &world_i18n::tr_owned(&name))],
                    ));
                    this.status_is_error = false;
                    cx.notify();
                    return;
                }
                let ears = response.unwrap_or(Ears::Own);
                this.finish_saying(who, words, ears, cx);
            });
        })
        .detach();
        cx.notify();
    }

    /// Says the favour's quick reply to `who`, as the World offered it: a
    /// structured intent ([`Ears::Offered`]), so the click is done as the
    /// favour it offers and never goes through a hearing or a model, in
    /// whatever language the button reads.
    pub(super) fn say_for_them(&mut self, who: SelectionId, words: String, cx: &mut Context<Self>) {
        self.ask(who, cx);
        if self.retelling.is_some() || self.looking.listening.is_some() {
            return;
        }
        self.finish_saying(who, words, Ears::Offered, cx);
    }

    /// The welcome waits for the town to be painted: until it is, the
    /// World's opening moment has not begun, and nobody speaks. From the
    /// first frame, the camera stands where whoever welcomes the player
    /// stands.
    pub(super) fn notice_paint(&mut self, stage: &Stage, painted: bool) {
        self.looking.stage_h = stage.height;
        let was = std::mem::replace(&mut self.looking.painted, painted);
        if !painted && self.looking.opening.is_some() {
            self.looking.opening = Some(Instant::now());
        }
        // Kept on the welcome while the window settles its size, until the
        // player pans away.
        if let Some((_, at)) = self.looking.welcome_pan {
            if self.looking.pan != Some(at) {
                self.looking.welcome_pan = None;
            } else if let Some((who, x)) = self.rest_opening(stage) {
                let x = self.open_on(stage, who, x);
                self.looking.welcome_pan = Some((who, x));
            }
            return;
        }
        if was || self.retelling.is_some() || self.looking.pan.is_some() {
            return;
        }
        // The camera is where the welcome will be said before anything is
        // painted, at rest, on the most of the place's people round whoever
        // says it: the first painted frame is the busiest view, never a pan
        // away from somewhere else. With nobody speaking (the night), on
        // the most people anywhere: never on an empty road.
        if let Some((who, x)) = self.rest_opening(stage) {
            let x = self.open_on(stage, who, x);
            self.looking.welcome_pan = Some((who, x));
            self.looking.opener = Some(who);
        }
    }

    /// Opens the camera on `who`, centred on stage `x` at rest; or, on a
    /// World's first day where the place's landmarks cannot be met at rest,
    /// pulled back to hold them ([`super::super::first_frame`]). Where it
    /// is centred.
    fn open_on(&mut self, stage: &Stage, who: SelectionId, x: f32) -> f32 {
        let first = super::super::arrival::first_day(&self.snapshot)
            .then(|| self.stage_x(stage, who))
            .flatten()
            .and_then(|speaker| super::super::first_frame::first_frame(stage, speaker));
        match first {
            Some((zoom, centre)) => {
                self.looking.zoom = zoom;
                self.looking.zoom_on = (centre, Camera::rest_y(stage, zoom));
                self.looking.pan = Some(centre);
                centre
            }
            None => {
                self.looking.pan = Some(x);
                x
            }
        }
    }

    /// Where the window opens at rest ([`super::arrival::busiest_view`]),
    /// and who it opens on: whoever speaks, or the resident nearest the
    /// middle of it. None with nobody out.
    pub(super) fn rest_opening(&self, stage: &Stage) -> Option<(SelectionId, f32)> {
        let speaker =
            super::arrival::opening_voice(&self.snapshot, stage, |who| self.stage_x(stage, who))
                .map(|(voice, x)| (voice.speaker, x));
        let centre = super::arrival::busiest_view(stage, speaker.map(|(_, x)| x));
        let who = speaker.map(|(who, _)| who).or_else(|| {
            stage
                .people
                .iter()
                .min_by(|a, b| (a.x - centre).abs().total_cmp(&(b.x - centre).abs()))
                .and_then(|spot| Some(self.snapshot.canvas.items.get(spot.index)?.id))
        })?;
        Some((who, centre))
    }

    /// Where the overview (zoomed out to one window of the place) is
    /// centred, coming out from stage `x`: the best view around it, or,
    /// where that leaves a building against the window's edge, the nearest
    /// to how the place opens at noon around the same point (see
    /// [`super::arrival::framed_as_at_noon`]). v0.29's opening framing.
    pub(super) fn overview_view(&mut self, stage: &Stage, x: f32) -> f32 {
        let size = (stage.view_w, stage.height);
        let revision = self.revision;
        super::arrival::opening_view(stage, x, || {
            match kept_noon_view(self.looking.noon_view, revision, size, x) {
                Some(centre) => centre,
                None => {
                    let noon_stage =
                        diorama::stage_at(&self.snapshot, size.0, size.1, diorama::Clock::at(12));
                    let centre = super::arrival::best_view(&noon_stage, x);
                    self.looking.noon_view = Some((revision, size, centre));
                    centre
                }
            }
        })
    }

    /// On a World's first day, once the welcome has been heard, the
    /// player's hands open on something free to place (the first card,
    /// which asks for money, waits until it is placed or put away): the
    /// toy before the ledger, well inside the first 20 seconds.
    pub(super) fn notice_free_offer(&mut self) {
        if self.looking.free_offer && self.looking.hands.is_none() {
            self.looking.free_offer = false;
        }
        if self.looking.free_offered
            || self.controller.is_none()
            || self.retelling.is_some()
            || is_beginning(&self.snapshot)
            || !self.looking.painted
            || self.looking.hands.is_some()
            || self.looking.asking.is_some()
            || self.looking.farewell.is_some()
        {
            return;
        }
        let heard = self
            .looking
            .opening
            .is_none_or(|at| since(Some(at)) >= greeting_seconds(&self.snapshot));
        if !heard {
            return;
        }
        // Looked at once: a later day, or nothing free, offers nothing.
        self.looking.free_offered = true;
        let Some(hands) = super::arrival::free_offer(&self.snapshot) else {
            return;
        };
        self.looking.hands = Some(Hands {
            verb: Some(hands.0),
            thing: Some(hands.1),
        });
        self.looking.free_offer = true;
    }

    /// The first day's free thing, left for later: the hands are put away
    /// and the day's card comes up. It stays free in the hands all day.
    pub(crate) fn put_free_offer_away(&mut self, cx: &mut Context<Self>) {
        self.looking.hands = None;
        self.looking.free_offer = false;
        cx.notify();
    }

    /// Where along the stage someone stands, by the middle of their
    /// figure.
    pub(super) fn stage_x(&self, stage: &Stage, who: SelectionId) -> Option<f32> {
        let index = self
            .snapshot
            .canvas
            .items
            .iter()
            .position(|item| item.id == who)?;
        let (x, _, w, _) = stage.frame_of(index)?;
        Some(x + w / 2.0)
    }

    /// Notices what a favour did since the last frame: asked, and the
    /// camera turns to whoever asked while they say it; or done, and
    /// their thanks show at once. Not during a return's film, which has
    /// the camera; afterwards.
    pub(super) fn notice_favour(&mut self, stage: &Stage) {
        use super::arrival::{ask_line, favour_beat, FavourBeat};
        if self.retelling.is_some() {
            return;
        }
        let now = self.snapshot.favour.clone();
        let beat = match self.looking.favour_seen.replace(now.clone()) {
            Some(before) => favour_beat(before.as_ref(), now.as_ref()),
            // Opened on a favour asked at this very moment: shown too.
            None => favour_beat(None, now.as_ref()).filter(|beat| {
                matches!(beat, FavourBeat::Asked { asker } if ask_line(&self.snapshot, *asker).is_some())
            }),
        };
        let Some(beat) = beat else {
            return;
        };
        if let FavourBeat::Asked { asker } = &beat {
            if self.looking.asking.is_none() {
                if let Some(x) = self.stage_x(stage, *asker) {
                    self.looking.pan = Some(x);
                }
            }
        }
        self.looking.favour_beat = Some((beat, Instant::now()));
    }

    /// What a favour has someone say now, over their head: the ask, while
    /// the camera is on them, then the thanks once it is done. Whom, the
    /// line, and how long ago it began.
    pub(crate) fn favour_line(&self) -> Option<(SelectionId, String, f32)> {
        use super::arrival::{ask_line, FavourBeat, ASK_SECONDS, THANKS_SECONDS};
        let (beat, at) = self.looking.favour_beat.as_ref()?;
        let age = since(Some(*at));
        match beat {
            FavourBeat::Asked { asker } if age < ASK_SECONDS && self.looking.asking.is_none() => {
                Some((*asker, ask_line(&self.snapshot, *asker)?, age))
            }
            FavourBeat::Thanked { asker, line } if age < THANKS_SECONDS && !line.is_empty() => {
                Some((*asker, line.clone(), age))
            }
            _ => None,
        }
    }

    /// Records what the player said, heard by `ears`: whether it was.
    pub(super) fn finish_saying(
        &mut self,
        who: SelectionId,
        words: String,
        ears: Ears,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(controller) = self.controller.as_mut() else {
            return false;
        };
        let said = match controller.handle(ProjectionIntent::Say {
            to: who,
            words,
            ears,
        }) {
            Ok(snapshot) => {
                self.snapshot = snapshot;
                self.revision += 1;
                self.looking.opening = None;
                self.looking.answered = None;
                self.looking.said_at = Some(Instant::now());
                self.status = None;
                self.status_is_error = false;
                true
            }
            Err(error) => {
                self.status = Some(crate::i18n::fill(
                    "Couldn't say that: {error}",
                    &[("error", &world_i18n::tr_owned(&error.to_string()))],
                ));
                self.status_is_error = true;
                false
            }
        };
        cx.notify();
        said
    }

    pub(super) fn answer(&mut self, talk: usize, cx: &mut Context<Self>) {
        self.looking.answered = Some((talk, Instant::now()));
        cx.notify();
    }

    pub(super) fn look_away(&mut self, cx: &mut Context<Self>) {
        self.looking.asking = None;
        self.looking.answered = None;
        cx.notify();
    }

    pub(super) fn on_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.mark_key(event, window, cx) {
            return;
        }
        let key = event.keystroke.key.as_str();
        let command = event.keystroke.modifiers.platform || event.keystroke.modifiers.control;
        // While the player types, their keys are words, not moves.
        if self.typing(window, cx) {
            match key {
                "enter" => self.say(cx),
                "escape" => {
                    if let Some(focus) = &self.looking.focus {
                        window.focus(focus, cx);
                    }
                    self.look_away(cx);
                }
                _ => {}
            }
            return;
        }
        if self.page_key(event, window, cx) {
            return;
        }
        match key {
            "i" if command => self.toggle_drawer(cx),
            "z" if command => self.undo(cx),
            "h" if !command && self.retelling.is_none() => self.toggle_hands(cx),
            "p" if !command && self.retelling.is_none() => self.take_photo(window, cx),
            "c" if !command && self.retelling.is_none() => self.take_postcard(window, cx),
            // The stories: whoever is being talked to (or chosen), the
            // newest moment, and the year's almanac.
            "l" if !command && self.retelling.is_none() => {
                if let Some(who) = self
                    .looking
                    .asking
                    .or(self.selected)
                    .filter(|who| matches!(who, SelectionId::Entity(_)))
                {
                    self.open_legend(who, cx);
                }
            }
            "m" if !command && self.retelling.is_none() => {
                if let Some(id) = self.snapshot.moments.last().map(|moment| moment.id.clone()) {
                    self.open_moment(&id, cx);
                }
            }
            // Any year's almanac, once a year has ended: the newest.
            "y" if !command && self.retelling.is_none() => {
                if let Some(year) = super::drawer::almanac_years(&self.snapshot).last().copied() {
                    self.open_almanac(year, cx);
                }
            }
            "escape" => {
                if self.looking.hands.is_some() {
                    self.looking.hands = None;
                    cx.notify();
                } else if self.looking.asking.is_some() {
                    self.look_away(cx);
                } else if self.looking.drawer {
                    self.toggle_drawer(cx);
                }
                // Esc puts things away; it never moves the camera.
            }
            _ if self.retelling.is_some() => {
                if matches!(key, "right" | "enter" | "space") {
                    self.step_retelling(cx);
                }
            }
            "left" | "right" if self.pans_with_arrows(event) => {
                let by = if key == "left" { -1.0 } else { 1.0 };
                self.pan_by_screens(by * 0.35, window, cx);
            }
            "left" => self.lean(-1, cx),
            "right" => self.lean(1, cx),
            "up" => self.cycle_card(-1, cx),
            "down" => self.cycle_card(1, cx),
            "enter" => self.enter(window, cx),
            "space" => self.turn_card(cx),
            "=" | "+" => self.zoom_by(ZOOM_STEP, None, window, cx),
            "-" => self.zoom_by(1.0 / ZOOM_STEP, None, window, cx),
            _ => {}
        }
    }

    /// Enter, outside a text field: on someone's open card it goes to
    /// their words; on a chapter's ending it turns the page; on the card
    /// in front it chooses, and lets a day pass only from the day's own
    /// card with the scene holding the keyboard (see `arrival`).
    pub(crate) fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use super::arrival::{enter_does, passes_day, EnterContext, EnterDoes};
        // The sitting's paper in front takes Enter for its one button:
        // "Begin the day" on the morning, "Goodnight" on the Goodnight
        // card. Nothing under it is chosen.
        if self.looking.sitting.goodnight {
            self.goodnight_said(cx);
            return;
        }
        if self.morning_shown().is_some() && self.looking.asking.is_none() {
            self.begin_the_day();
            cx.notify();
            return;
        }
        // Asleep, Enter passes no day and chooses nothing (1.0 P2: the
        // night stays open, but only a new sitting brings the next day).
        if self.asleep() {
            return;
        }
        // A chapter's end is a card under the sitting's paper: read with
        // Enter once the paper is put away, never unseen beneath it.
        if self.controller.is_some() {
            if let Some(number) = chapter_just_ended(&self.snapshot, self.looking.chapter_read)
                .map(|chapter| chapter.number)
            {
                self.looking.chapter_read = Some(number);
                cx.notify();
                return;
            }
            // And so is a year's close, after any chapter's.
            if let Some(year) = self.year_just_closed().map(|closed| closed.year) {
                self.looking.year_read = Some(year);
                cx.notify();
                return;
            }
        }
        // The first day's free thing, opened by itself, never holds the
        // day back: Enter leaves it for later and the day's card comes up.
        if self.looking.free_offer && self.looking.hands.is_some() {
            self.put_free_offer_away(cx);
            return;
        }
        let card = self.card_command();
        let context = EnterContext {
            asking: self.looking.asking.is_some(),
            covered: self.looking.drawer
                || self.looking.hands.is_some()
                || self.looking.marking.card.is_some()
                // The morning, the Goodnight card and the World asleep hide
                // the cards under them (v0.30 S4).
                || self.sitting_covers(),
            card_shown: card.is_some(),
            card_passes_day: card.is_some_and(passes_day),
            scene_focused: self
                .looking
                .focus
                .as_ref()
                .is_some_and(|focus| focus.is_focused(window)),
        };
        // The evening's card in front: Enter brings up the Goodnight card,
        // as its button does, with nothing over it.
        if self.evening() && !context.asking && !context.covered {
            self.say_goodnight();
            cx.notify();
            return;
        }
        match enter_does(context) {
            EnterDoes::Choose => self.choose_card(cx),
            EnterDoes::Talk => {
                if let Some(input) = self.looking.say.clone() {
                    window.focus(&input.focus_handle(cx), cx);
                    cx.notify();
                }
            }
            EnterDoes::Nothing => {}
        }
    }
}
