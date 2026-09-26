//! The World window as a place you look into: the scene fills it, and the
//! few things a player acts on float over it. The stakes are a slim strip
//! over the sky. Your turn is one card at a time rising from the bottom
//! edge. Anyone can be clicked and asked something. Coming back plays a
//! short film, one beat at a time, the camera going to each. Everything
//! else (who is who, what happened, how things stand) waits in a drawer
//! that opens with ⌘I.

use super::*;
use crate::art::{self, Figure};
use crate::diorama::{self, Camera, Glows, Stage};
use gpui::{canvas, Hsla, KeyDownEvent};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

/// How tall the bar a World window puts above this view is.
const CHROME: f32 = 52.0;
/// How wide the card and the drawer are.
const CARD_WIDTH: f32 = 560.0;
const DRAWER_WIDTH: f32 = 360.0;
/// How often a living World redraws while its window is in front.
const FRAME: Duration = Duration::from_millis(40);
/// How long each thing someone says stays over them, and how long a beat
/// of a return plays before the next.
const LINE_SECONDS: f32 = 4.6;
const BEAT_SECONDS: f32 = 5.2;
const ANSWER_SECONDS: f32 = 9.0;
/// How long the camera takes to move.
const CAMERA_SECONDS: f32 = 0.9;
/// How long a gauge takes to slide to where a turn left it.
const GAUGE_SECONDS: f32 = 0.9;
/// How long something new takes to rise.
const RISE_SECONDS: f32 = 1.1;

/// Presentation state: what the player is looking at, never what the
/// World is.
#[derive(Default)]
pub(crate) struct Looking {
    pub(crate) started: Option<Instant>,
    pub(crate) turn_at: Option<Instant>,
    pub(crate) card: usize,
    /// Which answer on a question's card the player is leaning toward.
    pub(crate) answer: usize,
    pub(crate) card_back: bool,
    pub(crate) drawer: bool,
    pub(crate) asking: Option<SelectionId>,
    pub(crate) answered: Option<(usize, Instant)>,
    pub(crate) beat_at: Option<Instant>,
    pub(crate) camera_from: Option<Camera>,
    pub(crate) camera_to: Option<Camera>,
    pub(crate) camera_at: Option<Instant>,
    pub(crate) focus: Option<gpui::FocusHandle>,
    pub(crate) ticking: bool,
    /// The last chapter whose ending card the player has turned past.
    pub(crate) chapter_read: Option<u32>,
}

/// The chapter that has just ended, if the player has not turned past its
/// card yet: one that closed within the last period.
pub(crate) fn chapter_just_ended(
    snapshot: &ProjectionSnapshot,
    read: Option<u32>,
) -> Option<&world_projection::Chapter> {
    let chapter = snapshot.chapters.last()?;
    if read.is_some_and(|read| read >= chapter.number) {
        return None;
    }
    let length = snapshot
        .calendar
        .as_ref()
        .map_or(1, |calendar| calendar.length);
    let ended = snapshot
        .timeline
        .items
        .iter()
        .find(|item| Some(item.id) == chapter.moment)?
        .world_time;
    (ended + length >= snapshot.world_time).then_some(chapter)
}

fn since(at: Option<Instant>) -> f32 {
    at.map(|at| at.elapsed().as_secs_f32()).unwrap_or(f32::MAX)
}

/// The words a World window shows at rest, besides the bar above it: the
/// gauges' names, the moment, the card, one thing somebody says, and the
/// names of whoever speaks or asks. Counted by a test, which holds the
/// window to showing a place rather than a page.
pub(crate) fn resting_text(snapshot: &ProjectionSnapshot) -> Vec<String> {
    let mut text = snapshot
        .gauges
        .iter()
        .map(|gauge| gauge.label.clone())
        .collect::<Vec<_>>();
    if snapshot.world_time > 0 {
        text.push(snapshot.moment_label(snapshot.world_time));
    }
    let first = card_order(snapshot).into_iter().next();
    let asking_question = first
        .as_ref()
        .and_then(|card| snapshot.commands.get(card[0]))
        .is_some_and(|command| command.question.is_some());
    if let Some(card) = first.filter(|_| !is_beginning(snapshot)) {
        let command = &snapshot.commands[card[0]];
        let faces = asker_faces(snapshot, command.asker)
            .iter()
            .map(|name| first_name(name))
            .collect::<Vec<_>>();
        text.push(faces.join(" & "));
        match &command.question {
            // A question's card: who asks, what they ask, and every answer.
            Some(question) => {
                text.push(question.prompt.clone());
                for index in &card {
                    text.push(snapshot.commands[*index].title.clone());
                }
                text.push("More".to_string());
            }
            None => {
                text.push(command.title.clone());
                text.extend(["More".to_string(), "Choose".to_string()]);
            }
        }
        text.extend(faces);
    }
    // While someone is asking, the scene is quiet: the question is the one
    // thing said.
    if asking_question && !is_beginning(snapshot) {
        return text;
    }
    if let Some(voice) = voices_now(snapshot).first() {
        text.push(voice.line.clone());
        if let Some(name) = label_of(snapshot, voice.speaker) {
            text.push(first_name(&name));
        }
    }
    text
}

/// The cards choices come up on, in order: each question with its
/// answers, and each choice that answers no question on its own; whatever
/// somebody asks first, letting time pass last, each group in the Pack's
/// own order.
pub(crate) fn card_order(snapshot: &ProjectionSnapshot) -> Vec<Vec<usize>> {
    let mut cards = snapshot.cards();
    cards.sort_by_key(|card| snapshot.commands[card[0]].asker.is_none());
    cards
}

/// The most words a World window may show at rest.
pub const RESTING_WORD_LIMIT: usize = 40;

/// How many words a World window shows at rest, the bar above it included:
/// the World's name, when it next moves ("Keeps going without you · next
/// sol in 6 h"), Branch and What if…, and everything `resting_text` lists.
pub fn words_at_rest(snapshot: &ProjectionSnapshot) -> usize {
    let bar = [
        snapshot.title.clone(),
        "Keeps going without you · next day in 6 h".to_string(),
        "Branch".to_string(),
        "What if…".to_string(),
    ];
    word_count(&bar) + word_count(&resting_text(snapshot))
}

/// How much of a World window the scene takes: all of it below the bar,
/// with everything else floating over it.
pub fn scene_share(window_height: f32) -> f32 {
    ((window_height - CHROME) / window_height).clamp(0.0, 1.0)
}

/// Words in a list of texts: runs with at least one letter in them.
pub(crate) fn word_count(texts: &[String]) -> usize {
    texts
        .iter()
        .flat_map(|text| text.split_whitespace())
        .filter(|word| word.chars().any(char::is_alphabetic))
        .count()
}

fn label_of(snapshot: &ProjectionSnapshot, id: SelectionId) -> Option<String> {
    snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.label.clone())
}

fn first_name(name: &str) -> String {
    name.split_whitespace().next().unwrap_or(name).to_string()
}

/// What was said at the latest moment, the story before the everyday.
pub(crate) fn voices_now(snapshot: &ProjectionSnapshot) -> Vec<&world_projection::Voice> {
    let time_of = |moment: SelectionId| {
        snapshot
            .timeline
            .items
            .iter()
            .find(|item| item.id == moment)
            .map(|item| (item.world_time, item.routine))
    };
    let mut voices = snapshot
        .voices
        .iter()
        .filter_map(|voice| {
            let (time, routine) = time_of(voice.moment)?;
            (time == snapshot.world_time).then_some((routine, voice))
        })
        .collect::<Vec<_>>();
    voices.sort_by_key(|(routine, _)| *routine);
    voices.into_iter().map(|(_, voice)| voice).collect()
}

fn figure_of(snapshot: &ProjectionSnapshot, id: SelectionId) -> Figure {
    let look = snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.id == id)
        .and_then(|item| item.look);
    Figure::of(&id.stable_key(), look)
}

/// Someone's face, drawn as they are on the scene.
pub(crate) fn portrait(figure: Figure, side: f32) -> Div {
    div()
        .flex_shrink_0()
        .size(px(side))
        .rounded(px(side * 0.24))
        .overflow_hidden()
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| art::paint_portrait(window, bounds, &figure),
            )
            .size_full(),
        )
}

/// The people a choice concerns, as scene ids: the asker, or both ends of
/// the relationship it is about.
fn asker_ids(snapshot: &ProjectionSnapshot, asker: Option<SelectionId>) -> Vec<SelectionId> {
    let Some(asker) = asker else {
        return Vec::new();
    };
    if let Some(link) = snapshot
        .canvas
        .links
        .iter()
        .find(|link| link.selection == Some(asker))
    {
        return vec![link.from, link.to];
    }
    if snapshot.canvas.items.iter().any(|item| item.id == asker) {
        vec![asker]
    } else {
        Vec::new()
    }
}

/// A speech bubble over someone: what they say, with a tail pointing down
/// at them. `x`, `y` is the top of their head, on screen.
fn bubble(
    key: String,
    line: String,
    x: f32,
    y: f32,
    stage: &Stage,
    opacity: f32,
    strong: bool,
) -> Div {
    const ROOM: f32 = 300.0;
    // Keep the bubble inside the stage; on a stage too narrow for it,
    // centre it (clamp would panic with its bounds the wrong way round).
    let (low, high) = (ROOM / 2.0 + 8.0, stage.width - ROOM / 2.0 - 8.0);
    let x = if high >= low {
        x.clamp(low, high)
    } else {
        stage.width / 2.0
    };
    let ink: Hsla = color(tokens::TEXT).into();
    let ground: Hsla = gpui::white();
    let tail = canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let o = bounds.origin;
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            let ox = f32::from(o.x);
            let oy = f32::from(o.y);
            art::polygon(
                window,
                &[(ox, oy), (ox + w, oy), (ox + w / 2.0, oy + h)],
                ground,
            );
        },
    )
    .w(px(14.0))
    .h(px(8.0));
    let _ = key;
    div()
        .absolute()
        .left(px(x - ROOM / 2.0))
        .top(px(y - 150.0))
        .w(px(ROOM))
        .h(px(150.0))
        .flex()
        .flex_col()
        .justify_end()
        .items_center()
        .opacity(opacity)
        .child(
            div()
                .max_w(px(ROOM))
                .px_3()
                .py_2()
                .rounded_xl()
                .bg(ground)
                .shadow_md()
                .text_sm()
                .when(strong, |text| text.font_weight(FontWeight::MEDIUM))
                .text_color(ink)
                .child(line),
        )
        .child(tail)
}

impl ProjectionView {
    fn stage_size(&self, window: &Window) -> (f32, f32) {
        let size = window.viewport_size();
        (f32::from(size.width), f32::from(size.height) - CHROME)
    }

    /// Start the clock that keeps a living World moving while its window is
    /// in front, once.
    fn keep_living(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.looking.ticking {
            return;
        }
        self.looking.ticking = true;
        cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor().timer(FRAME).await;
            let alive = this.update_in(cx, |_, window, cx| {
                if window.is_window_active() {
                    cx.notify();
                }
            });
            if alive.is_err() {
                break;
            }
        })
        .detach();
    }

    fn cue(&mut self, cue: crate::Cue) {
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
        self.looking.card = 0;
        self.looking.answer = 0;
        self.looking.card_back = false;
        self.looking.asking = None;
        self.looking.answered = None;
    }

    fn cycle_card(&mut self, by: isize, cx: &mut Context<Self>) {
        let count = card_order(&self.snapshot).len();
        if count == 0 {
            return;
        }
        self.looking.card = (self.looking.card as isize + by).rem_euclid(count as isize) as usize;
        self.looking.answer = 0;
        self.looking.card_back = false;
        self.cue(crate::Cue::Flip);
        cx.notify();
    }

    /// The answers on the card in front, if it is a question.
    fn card_answers(&self) -> Vec<usize> {
        let cards = card_order(&self.snapshot);
        cards
            .get(self.looking.card.min(cards.len().saturating_sub(1)))
            .cloned()
            .unwrap_or_default()
    }

    /// Leans toward the next or previous answer on a question's card, or
    /// turns to the next card when there is only one answer.
    fn lean(&mut self, by: isize, cx: &mut Context<Self>) {
        let answers = self.card_answers().len();
        if answers < 2 {
            self.cycle_card(by, cx);
            return;
        }
        self.looking.answer =
            (self.looking.answer as isize + by).rem_euclid(answers as isize) as usize;
        cx.notify();
    }

    fn lean_to(&mut self, answer: usize, cx: &mut Context<Self>) {
        if self.looking.answer != answer {
            self.looking.answer = answer;
            cx.notify();
        }
    }

    fn choose_card(&mut self, cx: &mut Context<Self>) {
        if self.retelling.is_some() || is_beginning(&self.snapshot) {
            return;
        }
        if let Some(command) = self.card_command() {
            let id = command.id.clone();
            self.invoke_command(id, cx);
        }
    }

    fn toggle_drawer(&mut self, cx: &mut Context<Self>) {
        self.looking.drawer = !self.looking.drawer;
        cx.notify();
    }

    fn ask(&mut self, who: SelectionId, cx: &mut Context<Self>) {
        self.looking.asking = Some(who);
        self.looking.answered = None;
        self.selected = Some(who);
        cx.notify();
    }

    fn answer(&mut self, talk: usize, cx: &mut Context<Self>) {
        self.looking.answered = Some((talk, Instant::now()));
        cx.notify();
    }

    fn look_away(&mut self, cx: &mut Context<Self>) {
        self.looking.asking = None;
        self.looking.answered = None;
        cx.notify();
    }

    fn on_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let command = event.keystroke.modifiers.platform || event.keystroke.modifiers.control;
        match key {
            "i" if command => self.toggle_drawer(cx),
            "escape" => {
                if self.looking.asking.is_some() {
                    self.look_away(cx);
                } else if self.looking.drawer {
                    self.toggle_drawer(cx);
                }
            }
            _ if self.retelling.is_some() => {
                if matches!(key, "right" | "enter" | "space") {
                    self.step_film(cx);
                }
            }
            "left" => self.lean(-1, cx),
            "right" => self.lean(1, cx),
            "up" => self.cycle_card(-1, cx),
            "down" => self.cycle_card(1, cx),
            "enter" => self.choose_card(cx),
            "space" => {
                self.looking.card_back = !self.looking.card_back;
                cx.notify();
            }
            _ => {}
        }
    }

    fn step_film(&mut self, cx: &mut Context<Self>) {
        self.looking.beat_at = Some(Instant::now());
        self.step_retelling(cx);
    }

    /// Where the camera is now, moving toward where it was last sent.
    fn camera(&mut self, stage: &Stage) -> Camera {
        let whole = Camera::whole(stage);
        // A return looks at whatever its beat is about.
        let target = self
            .current_beat()
            .map(|beat| beat_targets(&self.snapshot, beat))
            .and_then(|targets| {
                let boxes = self
                    .snapshot
                    .canvas
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| targets.contains(&item.id))
                    .filter_map(|(index, _)| stage.frame_of(index))
                    .collect::<Vec<_>>();
                let (x0, y0, x1, y1) = boxes.iter().fold(
                    (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
                    |(x0, y0, x1, y1), (x, y, w, h)| {
                        (x0.min(*x), y0.min(*y), x1.max(x + w), y1.max(y + h))
                    },
                );
                (!boxes.is_empty()).then(|| Camera::on(stage, (x0, y0, x1 - x0, y1 - y0)))
            })
            .unwrap_or(whole);
        let current = match (self.looking.camera_from, self.looking.camera_to) {
            (Some(from), Some(to)) => {
                from.toward(to, since(self.looking.camera_at) / CAMERA_SECONDS)
            }
            _ => whole,
        };
        if self.looking.camera_to != Some(target) {
            self.looking.camera_from = Some(current);
            self.looking.camera_to = Some(target);
            self.looking.camera_at = Some(Instant::now());
        }
        current
    }

    /// What the scene lights up now, and in what colour.
    fn glows(&self) -> Glows {
        let mut glows = Glows::new();
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

    fn card_command(&self) -> Option<&ProjectionCommand> {
        if self.controller.is_none() || is_beginning(&self.snapshot) || self.retelling.is_some() {
            return None;
        }
        let answers = self.card_answers();
        let answer = answers.get(self.looking.answer.min(answers.len().saturating_sub(1)))?;
        self.snapshot.commands.get(*answer)
    }

    /// The whole World window: the scene, and over it the gauges, the
    /// moment, your card, whoever is being asked, and the drawer.
    pub(crate) fn render_world(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        self.keep_living(window, cx);
        let focus = self
            .looking
            .focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        if self.looking.started.is_none() {
            self.looking.started = Some(Instant::now());
            window.focus(&focus, cx);
        }
        let seconds = since(self.looking.started);
        let (width, height) = self.stage_size(window);
        let stage = diorama::stage(&self.snapshot, width, height);
        let daylight = scene::daylight_now();

        // A return plays by itself, one beat after another.
        if self.retelling.is_some() {
            if self.looking.beat_at.is_none() {
                self.looking.beat_at = Some(Instant::now());
            }
            if since(self.looking.beat_at) > BEAT_SECONDS {
                self.looking.beat_at = Some(Instant::now());
                self.step_retelling(cx);
            }
        }
        let camera = self.camera(&stage);

        // Who is needed where they are, and who is talking.
        let speaking = voices_now(&self.snapshot);
        let answered = self
            .looking
            .answered
            .filter(|(_, at)| at.elapsed().as_secs_f32() < ANSWER_SECONDS)
            .and_then(|(index, at)| Some((self.snapshot.talks.get(index)?, at)));
        let beat_voice = self
            .current_beat()
            .and_then(|beat| beat.selection)
            .and_then(|moment| self.snapshot.voice_at(moment));
        let line_slot = (seconds / LINE_SECONDS) as usize;
        let line = if let Some((talk, at)) = answered {
            Some((talk.who, talk.answer.clone(), since(Some(at)), true))
        } else if self.retelling.is_some() {
            beat_voice.map(|voice| {
                (
                    voice.speaker,
                    voice.line.clone(),
                    since(self.looking.beat_at),
                    true,
                )
            })
        } else if speaking.is_empty()
            || self
                .card_command()
                .is_some_and(|command| command.question.is_some())
        {
            // While someone asks a question, the card says it and the scene
            // keeps quiet.
            None
        } else {
            let voice = speaking[line_slot % speaking.len()];
            Some((
                voice.speaker,
                voice.line.clone(),
                seconds % LINE_SECONDS,
                false,
            ))
        };
        let card_people = self
            .card_command()
            .map(|command| asker_ids(&self.snapshot, command.asker))
            .unwrap_or_default();
        let mut pinned = card_people.iter().copied().collect::<BTreeSet<_>>();
        pinned.extend(self.looking.asking);
        pinned.extend(line.as_ref().map(|(who, ..)| *who));
        pinned.extend(self.glows().keys().copied());
        let before = self.before_turn.as_ref().map(|before| {
            (
                diorama::stage(before, width, height),
                before,
                since(self.looking.turn_at) / diorama::WALK_SECONDS,
            )
        });
        let living = diorama::living(
            &stage,
            &self.snapshot,
            seconds,
            daylight,
            &pinned,
            before
                .as_ref()
                .map(|(stage, snapshot, progress)| (stage, *snapshot, *progress)),
        );
        let grew = self
            .before_turn
            .as_ref()
            .is_some_and(|before| before.canvas.marks.len() < self.snapshot.canvas.marks.len());
        let rising = if grew {
            (since(self.looking.turn_at) / RISE_SECONDS).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let frame = diorama::frame(
            &self.snapshot,
            &stage,
            &living,
            camera,
            seconds,
            daylight,
            &self.glows(),
            rising,
        );

        let mut root = div()
            .id("world-stage")
            .relative()
            .size_full()
            .overflow_hidden()
            .track_focus(&focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| this.on_key(event, cx)))
            .child({
                let frame = frame.clone();
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| diorama::paint(&frame, bounds, window),
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full()
            })
            // Clicking the open ground puts away whatever is open.
            .child(
                div()
                    .id("world-ground")
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .on_click(cx.listener(|this, _, _, cx| this.look_away(cx))),
            );

        // Buildings: named when pointed at, opening the drawer on a click.
        for spot in &stage.buildings {
            let item = &self.snapshot.canvas.items[spot.index];
            let (x, base) = camera.at(&stage, spot.x, spot.y);
            let w = stage.building_w * camera.zoom;
            let h = stage.building_h * camera.zoom;
            let selection = item.id;
            let group = SharedString::from(format!("place-{}", selection.stable_key()));
            let named = frame_glows(&frame, spot.index);
            root = root.child(
                div()
                    .id(SharedString::from(format!(
                        "stage-{}",
                        selection.stable_key()
                    )))
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
                        tag.opacity(0.0)
                            .group_hover(group, |style| style.opacity(1.0))
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.select(selection, cx);
                        this.looking.drawer = true;
                    })),
            );
        }
        // People: click to ask them something.
        let mut heads = Vec::new();
        for person in &frame.people {
            let item = &self.snapshot.canvas.items[person.index];
            let selection = item.id;
            let w = person.height * 0.7;
            let group = SharedString::from(format!("person-{}", selection.stable_key()));
            let named = card_people.contains(&selection)
                || self.looking.asking == Some(selection)
                || line.as_ref().is_some_and(|(who, ..)| *who == selection)
                || person.glow.is_some();
            heads.push((selection, person.x, person.y - person.height * 1.08));
            root = root.child(
                div()
                    .id(SharedString::from(format!(
                        "stage-{}",
                        selection.stable_key()
                    )))
                    .group(group.clone())
                    .absolute()
                    .left(px(person.x - w / 2.0 - 20.0))
                    .top(px(person.y - person.height))
                    .w(px(w + 40.0))
                    .h(px(person.height + 24.0))
                    .cursor_pointer()
                    .flex()
                    .flex_col()
                    .justify_end()
                    .items_center()
                    .child(name_tag(first_name(&item.label)).when(!named, |tag| {
                        tag.opacity(0.0)
                            .group_hover(group, |style| style.opacity(1.0))
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.ask(selection, cx);
                    })),
            );
        }
        // Whoever is talking, over their head.
        if let Some((who, text, age, strong)) = line {
            if let Some((_, x, y)) = heads.iter().find(|(id, ..)| *id == who) {
                let fade = if strong {
                    (age / 0.25).clamp(0.0, 1.0)
                } else {
                    ((age / 0.35).min((LINE_SECONDS - age) / 0.45)).clamp(0.0, 1.0)
                };
                root = root.child(bubble(
                    format!("line-{}", who.stable_key()),
                    text,
                    *x,
                    *y,
                    &stage,
                    fade,
                    strong,
                ));
            }
        }

        root = root.child(self.render_hud(cx));
        if let Some(beginning) = self.render_beginning(width >= 760.0, cx) {
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
            let card = if self.retelling.is_some() {
                self.render_retelling(cx)
            } else if let Some(chapter) = self.render_chapter_end(cx) {
                Some(chapter)
            } else {
                self.render_card(cx)
            };
            if let Some(card) = card {
                root = root.child(
                    bottom_card(card, room)
                        .when(self.looking.drawer, |card| card.right(px(DRAWER_WIDTH))),
                );
            }
        }
        if let Some((who, x, y)) = self
            .looking
            .asking
            .and_then(|who| heads.iter().find(|(id, ..)| *id == who).copied())
        {
            root = root.child(self.render_asking(who, x, y, &stage, cx));
        }
        if self.looking.drawer {
            root = root.child(self.render_drawer(cx));
        }
        if let Some(status) = self.render_status() {
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
        root.into_any_element()
    }

    /// Over the sky: the stakes on the left, and on the right the moment
    /// and the drawer's handle.
    fn render_hud(&self, cx: &mut Context<Self>) -> Div {
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
            right = right.child(
                pill()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.snapshot.moment_label(self.snapshot.world_time)),
            );
        }
        right = right.child(
            pill()
                .id("drawer-handle")
                .cursor_pointer()
                .hover(|style| style.bg(color(tokens::SURFACE)))
                .child(drawer_glyph().size(px(16.0)))
                .on_click(cx.listener(|this, _, _, cx| this.toggle_drawer(cx))),
        );
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
            .child(gauges)
            .child(right)
    }

    /// Your turn: one card, whoever asks it large, one line, and the ways
    /// to leaf through the others and choose.
    fn render_card(&self, cx: &mut Context<Self>) -> Option<Div> {
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
                .size(px(64.0))
                .rounded_full()
                .bg(color(tokens::ACCENT_SOFT))
                .p(px(16.0))
                .child(clock_glyph().size_full())
        } else {
            let mut stack = div()
                .relative()
                .flex_shrink_0()
                .h(px(64.0))
                .w(px(64.0 + 34.0 * (people.len().min(2) - 1) as f32));
            for (position, person) in people.iter().take(2).enumerate().rev() {
                stack = stack.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(position as f32 * 34.0))
                        .rounded(px(16.0))
                        .border_2()
                        .border_color(color(tokens::SURFACE))
                        .child(portrait(figure_of(&self.snapshot, *person), 60.0)),
                );
            }
            stack
        };
        let mut text = div().flex_1().min_w(px(0.0)).flex().flex_col().gap_1();
        if !names.is_empty() {
            text = text.child(ui::caption(names.join(" & ")));
        }
        // A question's card says what is asked; a lone choice says itself.
        text = text.child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(
            match &question {
                Some(question) => question.prompt.clone(),
                None => command.title.clone(),
            },
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
                    .text_sm()
                    .text_color(color(tokens::TEXT_SECONDARY))
                    .cursor_pointer()
                    .hover(|style| style.text_color(color(tokens::ACCENT_TEXT)))
                    .child(if self.looking.card_back {
                        "Less"
                    } else {
                        "More"
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.looking.card_back = !this.looking.card_back;
                        cx.notify();
                    })),
            )
            .when(question.is_none(), |actions| {
                actions.child(
                    ui::button("card-choose", "Choose", ButtonKind::Primary).on_click(
                        cx.listener(move |this, _, _, cx| this.invoke_command(id.clone(), cx)),
                    ),
                )
            });
        // Every answer to the question, on the card itself: pointing at one
        // leans toward it and the gauges show what it would move; clicking
        // it answers.
        let leaned = self.looking.answer.min(answers.len().saturating_sub(1));
        let mut replies = div().flex().flex_col().gap_2();
        if question.is_some() {
            for (position, answer) in answers.iter().enumerate() {
                let Some(reply) = self.snapshot.commands.get(*answer) else {
                    continue;
                };
                let id = reply.id.clone();
                let chosen = position == leaned;
                replies = replies.child(
                    div()
                        .id(SharedString::from(format!("answer-{position}")))
                        .px_4()
                        .py_2()
                        .rounded_xl()
                        .border_1()
                        .cursor_pointer()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .border_color(color(if chosen {
                            tokens::ACCENT
                        } else {
                            tokens::BORDER
                        }))
                        .bg(color(if chosen {
                            tokens::ACCENT_SOFT
                        } else {
                            tokens::SURFACE
                        }))
                        .text_color(color(if chosen {
                            tokens::ACCENT_TEXT
                        } else {
                            tokens::TEXT
                        }))
                        .hover(|style| style.border_color(color(tokens::ACCENT)))
                        .child(reply.title.clone())
                        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                            if *hovered {
                                this.lean_to(position, cx);
                            }
                        }))
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.invoke_command(id.clone(), cx)),
                        ),
                );
            }
        }

        let card = div()
            .p_5()
            .rounded_2xl()
            .bg(color(tokens::SURFACE))
            .shadow_lg()
            .border_1()
            .border_color(color(tokens::BORDER))
            .flex()
            .flex_col()
            .gap_4()
            .child(div().flex().items_center().gap_4().child(face).child(text))
            .when(question.is_some(), |card| card.child(replies))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(dots)
                    .child(actions),
            );
        Some(div().child(ui::arrive(
            card,
            format!("card-{}-{index}", self.revision()),
            0,
        )))
    }

    /// A chapter's ending, as a card of its own: its title, how it went,
    /// and the way on into the next.
    fn render_chapter_end(&self, cx: &mut Context<Self>) -> Option<Div> {
        self.controller.as_ref()?;
        let chapter = chapter_just_ended(&self.snapshot, self.looking.chapter_read)?;
        let number = chapter.number;
        let card = div()
            .p_6()
            .rounded_2xl()
            .bg(color(tokens::SURFACE))
            .shadow_lg()
            .border_1()
            .border_color(color(tokens::BORDER))
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .child(ui::caption(format!("Chapter {number} ends")))
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

    /// The story so far, chapter by chapter, and what the World is building,
    /// for the drawer.
    pub(crate) fn render_chapters(&self) -> Option<Div> {
        if self.snapshot.chapters.is_empty() && self.snapshot.goals.is_empty() {
            return None;
        }
        let mut book = div().flex().flex_col().gap_4();
        if !self.snapshot.goals.is_empty() {
            let mut goals = div()
                .flex()
                .flex_col()
                .gap_2()
                .child(ui::section_label("Building".to_string()));
            for goal in &self.snapshot.goals {
                let mut pips = div().flex().gap_1();
                for part in 0..goal.parts {
                    pips = pips.child(div().size(px(7.0)).rounded_full().bg(color(
                        if part < goal.done {
                            tokens::ACCENT
                        } else {
                            tokens::BORDER_STRONG
                        },
                    )));
                }
                goals = goals.child(
                    div()
                        .px_3()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().text_sm().child(goal.label.clone()))
                        .child(pips),
                );
            }
            book = book.child(goals);
        }
        if !self.snapshot.chapters.is_empty() {
            let mut chapters = div()
                .flex()
                .flex_col()
                .gap_3()
                .child(ui::section_label(format!(
                    "Chapters · {}",
                    self.snapshot.chapters.len()
                )));
            for chapter in self.snapshot.chapters.iter().rev() {
                chapters = chapters.child(
                    div()
                        .px_3()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(ui::caption(format!("Chapter {}", chapter.number)))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(chapter.title.clone()),
                        )
                        .child(ui::detail(chapter.summary.clone())),
                );
            }
            book = book.child(chapters);
        }
        Some(book)
    }

    /// What someone can be asked, beside them: their questions, and once
    /// one is asked, their answer, and what they ask for if they do.
    fn render_asking(
        &self,
        who: SelectionId,
        x: f32,
        head: f32,
        stage: &Stage,
        cx: &mut Context<Self>,
    ) -> Div {
        const WIDTH: f32 = 300.0;
        let name = label_of(&self.snapshot, who).unwrap_or_default();
        let detail = self
            .snapshot
            .canvas
            .items
            .iter()
            .find(|item| item.id == who)
            .map(|item| capitalize(&item.detail))
            .unwrap_or_default();
        let mut card = div()
            .id("asking")
            .w(px(WIDTH))
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(portrait(figure_of(&self.snapshot, who), 40.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .child(ui::row_title(name.clone()))
                            .child(ui::caption(detail)),
                    )
                    .child(arrow_button(
                        "asking-close",
                        "×",
                        cx.listener(|this, _, _, cx| this.look_away(cx)),
                    )),
            );
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
                    .child(talk.question.clone())
                    .on_click(cx.listener(move |this, _, _, cx| this.answer(index, cx))),
            );
            if asked {
                card = card.child(
                    div()
                        .px_3()
                        .text_sm()
                        .text_color(color(tokens::TEXT))
                        .child(format!("“{}”", talk.answer)),
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
        card = card.child(
            div()
                .id("ask-more")
                .pt_1()
                .text_xs()
                .text_color(color(tokens::TEXT_SECONDARY))
                .cursor_pointer()
                .hover(|style| style.text_color(color(tokens::ACCENT_TEXT)))
                .child(format!("More about {}", first_name(&name)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.selected = Some(who);
                    this.looking.drawer = true;
                    cx.notify();
                })),
        );
        // Beside them, on whichever side has room, level with their head.
        let left = if x + 40.0 + WIDTH < stage.width - 12.0 {
            x + 40.0
        } else {
            (x - 40.0 - WIDTH).max(12.0)
        };
        let top = (head + 8.0).clamp(64.0, (stage.height - 360.0).max(64.0));
        div()
            .absolute()
            .left(px(left))
            .top(px(top))
            .child(ui::arrive(card, format!("asking-{}", who.stable_key()), 0))
    }

    /// Everything a page used to show, one ⌘I away: a closer look at what
    /// is selected, what happened, how things stand, who is here, history.
    fn render_drawer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = div().flex().flex_col().gap_6().px_2().py_4();
        body = body.child(
            div()
                .px_3()
                .flex()
                .items_center()
                .justify_between()
                .child(ui::heading(self.snapshot.title.clone()))
                .child(arrow_button(
                    "drawer-close",
                    "×",
                    cx.listener(|this, _, _, cx| this.toggle_drawer(cx)),
                )),
        );
        // The story so far comes first: what the World is building and the
        // chapters it has closed.
        for part in [
            self.render_chapters(),
            self.render_closer_look(cx),
            self.render_story(cx),
            self.render_standing(cx),
            self.render_cast(cx),
            self.render_history(cx),
        ]
        .into_iter()
        .flatten()
        {
            body = body.child(div().px_1().child(part));
        }
        div()
            .id("world-drawer")
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .w(px(DRAWER_WIDTH))
            .bg(color(tokens::SIDEBAR))
            .border_l_1()
            .border_color(color(tokens::BORDER))
            .shadow_lg()
            .overflow_y_scroll()
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(body)
    }
}

fn frame_glows(frame: &diorama::Frame, index: usize) -> bool {
    frame
        .people
        .iter()
        .any(|person| person.index == index && person.glow.is_some())
}

/// A name under something on the scene.
fn name_tag(name: String) -> Div {
    div()
        .mt_1()
        .px_2()
        .py(px(1.0))
        .rounded_full()
        .bg(gpui::white().opacity(0.88))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(color(tokens::TEXT))
        .whitespace_nowrap()
        .child(name)
}

/// A soft pill over the sky.
fn pill() -> Div {
    div()
        .px_3()
        .py(px(6.0))
        .rounded_full()
        .bg(color(tokens::SURFACE).opacity(0.86))
        .shadow_sm()
        .text_sm()
        .text_color(color(tokens::TEXT))
        .flex()
        .items_center()
        .gap_2()
}

/// A gauge as a pill: its name and a short bar, and while a choice is on
/// the table which way the choice would move it and where it would end.
fn hud_gauge(gauge: &world_projection::Gauge, shown: f32, by: Option<i32>) -> Div {
    const BAR: f32 = 64.0;
    let fill: Hsla = color(scene::tone_token(gauge.tone)).into();
    let now = shown.clamp(0.0, 1.0);
    let then = by.map(|by| (now + by as f32 / 1000.0).clamp(0.0, 1.0));
    let mut bar = div()
        .relative()
        .w(px(BAR))
        .h(px(6.0))
        .rounded_full()
        .bg(color(tokens::BORDER))
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .h_full()
                .w(px(BAR * now))
                .rounded_full()
                .bg(fill),
        );
    if let Some(then) = then {
        let (from, to) = if then >= now {
            (now, then)
        } else {
            (then, now)
        };
        bar = bar.child(
            div()
                .absolute()
                .top_0()
                .left(px(BAR * from))
                .h_full()
                .w(px(BAR * (to - from)))
                .rounded_full()
                .bg(color(tokens::ACCENT).opacity(0.55)),
        );
    }
    let mut pill = pill()
        .child(
            div()
                .text_xs()
                .text_color(color(tokens::TEXT_SECONDARY))
                .child(gauge.label.clone()),
        )
        .child(bar);
    if let Some(by) = by.filter(|by| *by != 0) {
        pill = pill.child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(tokens::ACCENT_TEXT))
                .child(scene::movement_arrows(by)),
        );
    }
    pill
}

/// The drawer's handle: three short lines.
fn drawer_glyph() -> gpui::Canvas<()> {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let ink: Hsla = color(tokens::TEXT_SECONDARY).into();
            let x = f32::from(bounds.origin.x);
            let y = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            for row in 0..3 {
                art::rect(
                    window,
                    x + 1.0,
                    y + h * (0.22 + 0.28 * row as f32),
                    w - 2.0,
                    2.0,
                    1.0,
                    ink,
                );
            }
        },
    )
}

fn arrow_button(
    id: &'static str,
    glyph: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .size(px(26.0))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .text_base()
        .text_color(color(tokens::TEXT_SECONDARY))
        .cursor_pointer()
        .hover(|style| {
            style
                .bg(color(tokens::ROW_HOVER))
                .text_color(color(tokens::TEXT))
        })
        .child(glyph)
        .on_click(on_click)
}

/// A card floating at the bottom of the World, centred.
fn bottom_card(card: impl IntoElement, width: f32) -> Div {
    div()
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .pb_5()
        .px_4()
        .flex()
        .justify_center()
        .child(div().w(px(CARD_WIDTH.min(width - 32.0))).child(card))
}
