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
use crate::mark;
use crate::pointers::{self, Pointer};
use gpui::{canvas, Focusable, Hsla, KeyDownEvent, Role, Stateful};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};
use world_projection::MarkShape;

mod asking;
mod bubbles;
mod camera;
mod controls;
mod dock;
mod hud;
mod quiet;
mod render_world;
#[cfg(test)]
mod tests;
mod widgets;
pub use bubbles::*;
pub(crate) use dock::*;
pub(crate) use widgets::*;

/// How tall the bar a World window puts above this view is.
pub(crate) const CHROME: f32 = 52.0;
/// How wide the card and the drawer are.
const CARD_WIDTH: f32 = 560.0;
pub(crate) const DRAWER_WIDTH: f32 = 380.0;
/// How wide the card of someone being asked is.
const ASKING_WIDTH: f32 = 300.0;
/// How far the card of someone being asked keeps from the window's edges.
const ASKING_CLEAR: f32 = 12.0;
/// How often a living World redraws while its window is in front.
/// How often a window behind others checks whether it has come to the
/// front again. In front, it draws at the display's own rate.
const FRAME: Duration = Duration::from_millis(250);
/// How a World window keeps drawing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Pace {
    /// In front and moving freely: at the display's own rate.
    EveryFrame,
    /// In front with Reduce Motion on: a few times a second
    /// ([`FRAME`]), enough for what still moves.
    Ticking,
    /// Behind other windows: nothing drawn, the clock only looking now
    /// and then whether the window has come to the front.
    Waiting,
    /// Nobody can see it (covered, minimised, or on a sleeping display):
    /// no frames and no clock at all, until it can be seen again.
    Asleep,
}

/// How a World window whose frames can (`visible`) or cannot be seen,
/// in front (`active`) or not, with Reduce Motion (`still`) or not, keeps
/// drawing.
pub(crate) fn pace(visible: bool, active: bool, still: bool) -> Pace {
    match (visible, active, still) {
        (false, ..) => Pace::Asleep,
        (true, false, _) => Pace::Waiting,
        (true, true, true) => Pace::Ticking,
        (true, true, false) => Pace::EveryFrame,
    }
}

/// How long each thing someone says stays over them, and how long a beat
/// of a return plays before the next.
const LINE_SECONDS: f32 = 4.6;
pub(super) const BEAT_SECONDS: f32 = 5.2;
const ANSWER_SECONDS: f32 = 9.0;
/// How many letters the drawer shows, newest first.
pub(crate) const LETTERS_SHOWN: usize = 12;
/// How long a keepsake handed over in front of the player stays up.
const GIFT_SECONDS: f32 = 5.0;
/// Where a word that something was saved hangs: under the row of handles,
/// clear of the cards at the bottom.
const TOAST_TOP: f32 = 64.0;
#[cfg(test)]
pub(crate) use super::glide::CAMERA_SECONDS;
use super::glide::{Bound, Glide};
/// How many of today's exchanges with someone their card shows.
const CONVERSATION_SHOWN: usize = 3;
/// How long a gauge takes to slide to where a turn left it.
const GAUGE_SECONDS: f32 = 0.9;
/// How long something new takes to rise.
const RISE_SECONDS: f32 = 1.1;
/// How much one press of the zoom control (or + and −) zooms, and the
/// closest the camera goes.
const ZOOM_STEP: f32 = 1.25;
const ZOOM_MOST: f32 = diorama::ZOOM_MOST;
/// How wide a pointer is.
const HINT_WIDTH: f32 = 280.0;
/// The shortest stage the zoom control is pointed at on: shorter, the
/// pointer beside it could reach down to the card.
const ZOOM_HINT_ROOM: f32 = 520.0;

/// Where a zoom leaves the camera: how close, where along the place it
/// looks, and the point it zoomed on.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Zoomed {
    zoom: f32,
    pan: f32,
    on: (f32, f32),
}

/// `factor` times closer than `before` from `camera`, around the screen
/// point `at` (the point under it stays under it): none if the zoom is at
/// the end of its travel that way. Coming within a hair of rest or of the
/// overview (one window), it stops there.
fn zoomed(
    stage: &Stage,
    camera: Camera,
    before: f32,
    factor: f32,
    at: (f32, f32),
) -> Option<Zoomed> {
    let (x, y) = camera.stage_point(stage, at.0, at.1);
    let mut zoom = (before * factor).clamp(Camera::least(stage), diorama::ZOOM_MOST);
    let rest = Camera::rest_zoom(stage);
    if (zoom - 1.0).abs() < 0.02 {
        zoom = 1.0;
    } else if (zoom / rest - 1.0).abs() < 0.02 {
        zoom = rest;
    }
    if (zoom - before).abs() <= f32::EPSILON {
        return None;
    }
    let keep = camera.zoom / zoom;
    Some(Zoomed {
        zoom,
        pan: x - (x - camera.x) * keep,
        on: (x - (x - camera.x) * keep, y - (y - camera.y) * keep),
    })
}

/// The view wherever the player has zoomed (`zoom`; none at rest, never
/// zoomed) and panned to (`pan`, the whole place's middle if never),
/// around the point zoomed on.
fn free_view(stage: &Stage, zoom: Option<f32>, pan: Option<f32>, on: (f32, f32)) -> Camera {
    let x = pan.unwrap_or(Camera::whole(stage).x);
    match zoom {
        None => Camera::rest(stage, x),
        Some(zoom) => Camera::around(
            stage,
            zoom,
            x,
            if zoom > 1.0 { on.1 } else { stage.height / 2.0 },
        ),
    }
}

/// Where the zoom control sits, down the left of a stage `height` tall:
/// below the stakes, well above the card.
fn zoom_top(height: f32) -> f32 {
    (height * 0.38).max(96.0)
}

/// A rectangle on the stage: left, top, width, height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Area {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
}

impl Area {
    pub(crate) fn overlaps(&self, other: &Area) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }
}

/// Which way a pointer's little arrow points: at a handle above it, the
/// card below it, or the zoom control to its left.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Caret {
    Up,
    Down,
    Left,
}

/// The scene a return beat plays, as the World staged it.
pub(crate) type BeatScene = std::rc::Rc<world_projection::SceneScript>;

/// Presentation state: what the player is looking at, never what the
/// World is.
#[derive(Default)]
pub(crate) struct Looking {
    /// The language the World's words on screen were put in, so a change
    /// of language shows the open World anew.
    pub(crate) shown_in: Option<world_i18n::Language>,
    pub(crate) started: Option<Instant>,
    /// When this window first showed the World's own moment (as it opened,
    /// or as the player chose where it begins), until the player's next
    /// turn: a question waits while the first thing said then is heard.
    pub(crate) opening: Option<Instant>,
    pub(crate) turn_at: Option<Instant>,
    pub(crate) card: usize,
    /// Which answer on a question's card the player is leaning toward.
    pub(crate) answer: usize,
    pub(crate) card_back: bool,
    pub(crate) drawer: bool,
    pub(crate) asking: Option<SelectionId>,
    pub(crate) answered: Option<(usize, Instant)>,
    pub(crate) beat_at: Option<Instant>,
    /// The scene the return film's beat plays, asked for once a beat (by
    /// the World's revision and the beat's index): none for a beat that is
    /// no recorded Event, or one the World stages no scene for.
    pub(crate) scene: Option<((u64, usize), Option<BeatScene>)>,
    /// The World as that scene draws it ([`diorama::staging::scene_snapshot`]),
    /// made once with it: none when it is the World as it stands.
    pub(crate) scene_world: Option<std::rc::Rc<ProjectionSnapshot>>,
    /// Where the return film's card docked for a beat, kept through it.
    pub(crate) film_dock: Option<((u64, usize), Dock)>,
    /// A test's clock for the beat: how far into it, whatever the wall
    /// clock says.
    #[cfg(test)]
    pub(crate) beat_clock: Option<f32>,
    /// The camera's glide toward what it is sent to look at.
    pub(crate) glide: Glide,
    pub(crate) focus: Option<gpui::FocusHandle>,
    /// The clock that keeps a living World moving; dropped (stopping it)
    /// while the window cannot be seen.
    pub(crate) clock: Option<gpui::Task<()>>,
    /// Keeps the frames drawn while things move at the player's cap.
    pub(crate) pacer: crate::frames::Pacer,
    /// The camera is gliding and wants the next frame, asked for once the
    /// frame it is in has been worked out (at the player's cap).
    pub(crate) glide_wants: bool,
    /// Hears when the window is covered, minimised, or shown again.
    pub(crate) visibility: Option<gpui::Subscription>,
    /// The last chapter whose ending card the player has turned past.
    pub(crate) chapter_read: Option<u32>,
    /// The last year whose closing card the player has turned past.
    pub(crate) year_read: Option<u32>,
    /// What the player is doing with their own hands, if anything.
    pub(crate) hands: Option<Hands>,
    /// Where the player types what they say to someone.
    pub(crate) say: Option<gpui::Entity<crate::text_input::TextInput>>,
    /// When the player last said something, so the answer shows over the
    /// person's head for a while.
    pub(crate) said_at: Option<Instant>,
    /// Words the player said that a language model is still hearing: to
    /// whom, and what. The window keeps drawing; the person is thinking.
    pub(crate) listening: Option<(SelectionId, String)>,
    /// How close the player has zoomed in with the wheel, and on which
    /// stage point; 1 is one window of the place (the overview), and 0 the
    /// camera at rest (never zoomed: v0.30 S1c).
    pub(crate) zoom: f32,
    pub(crate) zoom_on: (f32, f32),
    /// How tall the stage was when last laid out: the camera at rest is a
    /// function of it (0 before the first frame).
    pub(crate) stage_h: f32,
    /// Where along a panorama the player has panned to: the stage point
    /// the view is centred on at rest. `None` is the middle of the place.
    pub(crate) pan: Option<f32>,
    /// A pan by dragging under way: where the pointer went down, where the
    /// view was centred then, and whether it has moved far enough to be a
    /// drag rather than a click.
    pub(crate) drag: Option<(f32, f32, bool)>,
    /// When a drag last ended, so the click that ends it opens nothing.
    pub(crate) dragged_at: Option<Instant>,
    /// Where the camera looked last frame, to turn the pointer into a
    /// stage point.
    pub(crate) camera_now: Option<Camera>,
    /// Who the player last clicked on, and when: they wave.
    pub(crate) poked: Option<(SelectionId, Instant)>,
    /// The line last given a babble, so each is heard once as it appears.
    pub(crate) babbled: Option<(SelectionId, String)>,
    /// How many keepsakes the player held last frame, and when a new one
    /// was handed over while they watched.
    pub(crate) keepsakes_seen: Option<usize>,
    pub(crate) gift_at: Option<Instant>,
    /// A photograph being taken (everything but the scene hidden), and
    /// when the last was saved.
    pub(crate) photographing: bool,
    pub(crate) photo_saved: Option<Instant>,
    /// A postcard being taken: the paper and caption drawn around the
    /// scene while the photograph is taken, and when the last was saved.
    pub(crate) postcard: Option<crate::postcard::Postcard>,
    pub(crate) postcard_saved: Option<Instant>,
    /// When the last postcard could not be saved.
    pub(crate) postcard_failed: Option<Instant>,
    /// The gentle pointer showing now, and since when.
    pub(crate) pointer: Option<(Pointer, Instant)>,
    /// The player's mark on the place: plots, a design, a name.
    pub(crate) marking: super::marking::Marking,
    /// When the last pointer went away.
    pub(crate) pointer_gone: Option<Instant>,
    /// The drawer's leaf open, the years folded or opened against their
    /// usual way, and the kind of work shown.
    pub(crate) leaf: super::drawer::Leaf,
    pub(crate) years: BTreeSet<u32>,
    pub(crate) kind: Option<usize>,
    /// The favour shown when the window last looked (`None` until it has
    /// looked once), and what it did since: asked, so the camera turns
    /// to whoever asked; or done, so their thanks show.
    pub(crate) favour_seen: Option<Option<world_projection::Favour>>,
    pub(crate) favour_beat: Option<(super::arrival::FavourBeat, Instant)>,
    /// Whether the scene had been painted last frame: the welcome waits
    /// for it.
    pub(crate) painted: bool,
    /// Whoever the window opened on to welcome the player, and where it
    /// put the camera for them: its pan and its zoom.
    pub(crate) welcome_pan: Option<(SelectionId, f32, f32)>,
    /// Whose line the window opened on, which is said first: the window
    /// opens where the most people are round whoever speaks
    /// ([`super::arrival::opening_voice`]).
    pub(crate) opener: Option<SelectionId>,
    /// Where the opening view is centred at noon, for a stage of this
    /// width and height: the framing every hour opens on where its own
    /// would leave a building against the window's edge. Kept for the
    /// World's revision it was framed on: a turn that builds or moves
    /// something frames the noon stage again.
    pub(crate) noon_view: Option<(u64, (f32, f32), f32)>,
    /// A farewell showing over the World, and whether the camera has gone
    /// to whoever says goodbye.
    pub(crate) farewell: Option<super::farewell::Farewell>,
    pub(crate) farewell_panned: bool,
    /// When the player last moved the mouse or pressed a key: the
    /// interface quietens after a while without them (`quiet`).
    pub(crate) touched: Option<Instant>,
    pub(crate) farewell_speaker: Option<SelectionId>,
    /// Whether the first day's free thing has been offered in this window,
    /// and whether the offer is up now: the hands open on it, the card
    /// held back until it is placed or put away.
    pub(crate) free_offered: bool,
    pub(crate) free_offer: bool,
    /// While someone is being talked to (and the camera has moved in on
    /// them), where it was: so putting them away leaves the camera there.
    pub(crate) held: Option<Camera>,
    /// The hour this view last drew its scene at: its own held hour, or
    /// the clock's (v0.30 S4: one window asleep, another awake).
    pub(crate) drawn_at_hour: Option<u32>,
    /// This sitting: how many days it may still pass, its morning read, its
    /// goodnight, and whether the World sleeps (v0.30 S4). UI state only.
    pub(crate) sitting: super::sitting::Sitting,
}

/// Something the player's hands can make or do, for the build card.
#[derive(Clone, Debug)]
pub(crate) struct HandThing {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) cost: Option<String>,
    pub(crate) possible: bool,
    pub(crate) reason: Option<String>,
    pub(crate) shape: MarkShape,
    pub(crate) command: String,
}

/// How a thing of the hands is drawn on its tile: by the name its deed
/// gives it ("hands.build.bench.12" is a bench), else as its verb makes
/// things (a garden planted, bunting hung, a parcel given).
pub(crate) fn hand_shape(command: &str, verb: &str) -> MarkShape {
    use MarkShape::*;
    // The thing comes after the verb: "tiny-society.hand.build.bench.12".
    let parts = command.split('.').collect::<Vec<_>>();
    let verb_id = verb.to_lowercase();
    let thing = parts
        .iter()
        .position(|part| *part == verb_id)
        .and_then(|at| parts.get(at + 1))
        .copied()
        .unwrap_or(command);
    let named = [
        ("bench", Bench),
        ("picnic", Bench),
        ("lantern", Lantern),
        ("lamp", Lantern),
        ("stall", Stall),
        ("flag", Flag),
        ("bunting", Bunting),
        ("tree", Tree),
        ("well", Well),
        ("swing", Swing),
        ("fountain", Fountain),
        ("signpost", Signpost),
        ("birdhouse", Birdhouse),
        ("statue", Statue),
        ("postbox", Postbox),
        ("boat", Boat),
        ("flowerbox", Planter),
        ("planter", Planter),
        ("tent", Tent),
        ("rover", Rover),
        ("dome", Dome),
    ];
    if let Some((_, shape)) = named.iter().find(|(name, _)| thing.contains(name)) {
        return *shape;
    }
    match verb {
        "Plant" => Garden,
        "Decorate" => Bunting,
        "Build" => Parcel,
        _ => Parcel,
    }
}

/// The player's hands: which verb they picked, and what they are about to
/// put, give or invite, if they have picked it. While both are set, the
/// scene lights up where it can go.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Hands {
    pub(crate) verb: Option<String>,
    /// What they are placing, or `*` for anyone at all (a gift, an
    /// invitation).
    pub(crate) thing: Option<String>,
}

/// The verbs the player's hands can do, in the order they are offered.
const VERBS: [&str; 6] = ["Build", "Decorate", "Plant", "Move", "Give", "Invite"];

/// Verbs done to someone rather than somewhere.
pub(crate) fn to_someone(verb: &str) -> bool {
    matches!(verb, "Give" | "Invite")
}

/// What a row in the hands picker stands for: the kind of thing for what
/// is made, and the very fixture for a move, so two benches stay two.
pub(crate) fn which(
    command: &world_projection::ProjectionCommand,
    hand: &world_projection::Hand,
) -> String {
    match (hand.verb.as_str(), command.id.rsplit_once('.')) {
        ("Move", Some((fixture, _place))) => fixture.to_string(),
        _ => hand.thing.clone(),
    }
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

/// The time of year, in the drawer: the season, and what is coming up in
/// the next few days. At rest the scene's colours say the season.
pub(crate) fn coming_label(snapshot: &ProjectionSnapshot) -> Option<String> {
    let calendar = snapshot.calendar.as_ref()?;
    match (&calendar.season, &calendar.coming) {
        (Some(season), Some(coming)) => Some(format!("{season} · {coming}")),
        (Some(season), None) => Some(season.clone()),
        (None, coming) => coming.clone(),
    }
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
        text.push(crate::i18n::moment_label(snapshot, snapshot.world_time));
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
    // The morning's main question first (v0.30 S4), then whoever asks.
    cards.sort_by_key(|card| {
        let command = &snapshot.commands[card[0]];
        (
            !super::sitting::is_days_question(snapshot, command),
            command.asker.is_none(),
        )
    });
    cards
}

/// The most words a World window may show at rest.
pub const RESTING_WORD_LIMIT: usize = 40;

/// How many words a World window shows at rest, the bar above it included:
/// the World's name, when its next day comes ("Waits for you · a new
/// sol in 6 h"), and everything `resting_text` lists.
pub fn words_at_rest(snapshot: &ProjectionSnapshot) -> usize {
    let bar = [
        snapshot.title.clone(),
        "Waits for you · a new day in 6 h".to_string(),
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

pub(crate) fn label_of(snapshot: &ProjectionSnapshot, id: SelectionId) -> Option<String> {
    snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.label.clone())
}

/// A phrase as it starts a line: "a pressed flower" as "A pressed flower".
pub(crate) fn capitalized(phrase: &str) -> String {
    let mut chars = phrase.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

pub(crate) fn first_name(name: &str) -> String {
    name.split_whitespace().next().unwrap_or(name).to_string()
}

/// How long a question's card waits as a World opens: while the first
/// thing said is heard, so a newcomer is greeted before they are asked
/// anything. Nothing waits when the first card asks nothing, or nobody
/// speaks.
pub(crate) fn question_waits(snapshot: &ProjectionSnapshot) -> f32 {
    let asks = card_order(snapshot)
        .first()
        .and_then(|card| snapshot.commands.get(card[0]))
        .is_some_and(|command| command.question.is_some());
    match voices_now(snapshot).first() {
        Some(voice) if asks => LINE_SECONDS * speech_pages(&voice.line).len() as f32,
        _ => 0.0,
    }
}

/// How long the greeting as a World opens lasts: the first thing said,
/// every page of it, and never less than a question waits.
pub(crate) fn greeting_seconds(snapshot: &ProjectionSnapshot) -> f32 {
    voices_now(snapshot)
        .first()
        .map_or(0.0, |voice| {
            LINE_SECONDS * speech_pages(&voice.line).len() as f32
        })
        .max(question_waits(snapshot))
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

/// Whether a selection is a home or a work the Pack stood on the canvas
/// (ids from 900,000,000 up): part of the place, with nothing to inspect.
fn part_of_the_place(selection: SelectionId) -> bool {
    selection
        .stable_key()
        .strip_prefix("entity-")
        .and_then(|id| id.parse::<u64>().ok())
        .is_some_and(|id| id >= 900_000_000)
}

/// The box on the stage around whatever a return film's beat is about.
fn beat_box(
    snapshot: &ProjectionSnapshot,
    stage: &Stage,
    beat: &BriefingItem,
) -> Option<(f32, f32, f32, f32)> {
    let targets = beat_targets(snapshot, beat);
    let frames = |targets: &std::collections::BTreeSet<SelectionId>| {
        snapshot
            .canvas
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| targets.contains(&item.id))
            .filter_map(|(index, _)| stage.frame_of(index))
            .collect::<Vec<_>>()
    };
    let mut boxes = frames(&targets);
    // Someone not out on the scene just now (indoors at that hour) is
    // looked for where they are.
    if boxes.is_empty() {
        let at = snapshot
            .canvas
            .items
            .iter()
            .filter(|item| targets.contains(&item.id))
            .filter_map(|item| item.at)
            .collect();
        boxes = frames(&at);
    }
    let (x0, y0, x1, y1) = boxes.iter().fold(
        (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
        |(x0, y0, x1, y1), (x, y, w, h)| (x0.min(*x), y0.min(*y), x1.max(x + w), y1.max(y + h)),
    );
    (!boxes.is_empty()).then_some((x0, y0, x1 - x0, y1 - y0))
}

/// With `WORLD_MACHINE_FRAME_LOG` set, how long the scene takes on the
/// window's thread (working out its still layers and drawing everything):
/// every two seconds, the mean and the longest frame, and how long was
/// spent painting still layers (off this thread, unless painting is
/// synchronous). Measuring only.
fn frame_log(took: Duration) {
    use std::cell::RefCell;
    thread_local! {
        static LOG: RefCell<(Option<Instant>, u32, Duration, Duration)> =
            const { RefCell::new((None, 0, Duration::ZERO, Duration::ZERO)) };
    }
    if std::env::var_os("WORLD_MACHINE_FRAME_LOG").is_none() {
        return;
    }
    LOG.with(|log| {
        let mut log = log.borrow_mut();
        let since = *log.0.get_or_insert_with(Instant::now);
        log.1 += 1;
        log.2 += took;
        log.3 = log.3.max(took);
        if since.elapsed() > Duration::from_secs(2) {
            let (painted, paints) = crate::painter::painting_time();
            eprintln!(
                "frame: {} frames, mean {:.2} ms, longest {:.2} ms; still layers: {} painted in {:.1} ms",
                log.1,
                log.2.as_secs_f64() * 1000.0 / log.1.max(1) as f64,
                log.3.as_secs_f64() * 1000.0,
                paints,
                painted.as_secs_f64() * 1000.0,
            );
            *log = (Some(Instant::now()), 0, Duration::ZERO, Duration::ZERO);
        }
    });
}

/// How someone looks: the figure the app draws for them, their Pack's own
/// drawing if it ships one, and how they feel.
#[derive(Clone, Debug)]
pub(crate) struct Likeness {
    pub(crate) figure: Figure,
    pub(crate) drawing: Option<world_projection::Drawing>,
    pub(crate) mood: world_projection::Mood,
}

pub(crate) fn likeness_of(snapshot: &ProjectionSnapshot, id: SelectionId) -> Likeness {
    let item = snapshot.canvas.items.iter().find(|item| item.id == id);
    let figure = Figure::of(&id.stable_key(), item.and_then(|item| item.look));
    Likeness {
        // A baby's or a small child's portrait is the app's own young face,
        // never a grown-up's drawing (its hat, its beard) made small.
        drawing: item
            .and_then(|item| snapshot.drawing_of(item))
            .filter(|_| !young(&figure))
            .cloned(),
        figure,
        mood: item.and_then(|item| item.mood).unwrap_or_default(),
    }
}

/// Whether someone is a baby or a small child, drawn as the app draws the
/// young rather than in a grown-up's drawing.
pub(crate) fn young(figure: &Figure) -> bool {
    matches!(figure.age, crate::age::Age::Baby | crate::age::Age::Child) && !figure.bird
}

/// Whether a mouth is open now, for someone speaking: in bursts of a few
/// syllables, then a pause.
fn mouth_open() -> bool {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs_f32() % 60.0)
        .unwrap_or(0.0);
    seconds % 6.0 < 2.4 && (seconds * 5.0).fract() < 0.55
}

/// Someone's face, drawn as they are on the scene: the same drawing, the
/// same mood, and speaking when `talking`.
pub(crate) fn portrait(likeness: Likeness, side: f32, talking: bool) -> Div {
    div()
        .flex_shrink_0()
        .size(px(side))
        .rounded(px(side * 0.24))
        .overflow_hidden()
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    art::paint_likeness(
                        window,
                        bounds,
                        &likeness.figure,
                        likeness.drawing.as_ref(),
                        likeness.mood,
                        talking && mouth_open(),
                    )
                },
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

/// The noon framing kept in `cached`, if it still answers for a window of
/// `size` coming out from stage `x` on the World's `revision`: none once a
/// turn has changed the place (what was built or moved is framed again),
/// the window has another size, or `x` has gone half a window from it.
fn kept_noon_view(
    cached: Option<(u64, (f32, f32), f32)>,
    revision: u64,
    size: (f32, f32),
    x: f32,
) -> Option<f32> {
    let (framed, kept, centre) = cached?;
    (framed == revision && kept == size && (centre - x).abs() < size.0 * 0.5).then_some(centre)
}
