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

/// How tall the bar a World window puts above this view is.
pub(crate) const CHROME: f32 = 52.0;
/// How wide the card and the drawer are.
const CARD_WIDTH: f32 = 560.0;
pub(crate) const DRAWER_WIDTH: f32 = 380.0;
/// How wide the card of someone being asked is.
const ASKING_WIDTH: f32 = 300.0;
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
const BEAT_SECONDS: f32 = 5.2;
const ANSWER_SECONDS: f32 = 9.0;
/// How many letters the drawer shows, newest first.
pub(crate) const LETTERS_SHOWN: usize = 12;
/// How long a keepsake handed over in front of the player stays up.
const GIFT_SECONDS: f32 = 5.0;
/// How long the camera takes to move.
pub(crate) const CAMERA_SECONDS: f32 = 0.9;
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

/// How tall the row of gauges and handles over the sky is, generously.
const HUD_ROOM: f32 = 64.0;
/// How tall the card at the foot of the stage is, generously.
const CARD_ROOM: f32 = 250.0;
/// The zoom control's size: two buttons, one over the other.
const ZOOM_SIZE: (f32, f32) = (36.0, 68.0);
/// How far a bubble's tail reaches down to the head it speaks for.
const TAIL: f32 = 8.0;
/// How much room is kept between a bubble and whatever it must not
/// cover, and the stage's edge.
const BUBBLE_CLEAR: f32 = 6.0;

/// How big a speech bubble showing `page` is drawn, generously: its width
/// and its height, the tail below it included.
pub(crate) fn bubble_size(page: &str) -> (f32, f32) {
    let scale = crate::text_scale();
    let widest = page.lines().map(text_width).max().unwrap_or(0) as f32;
    let rows = page.lines().count().max(1) as f32;
    let w = (widest * 7.4 * scale + 30.0).clamp(56.0, BUBBLE_ROOM);
    (w, rows * 20.0 * scale + 18.0 + TAIL)
}

/// A speech bubble as placed on the stage: the box its words are in, and
/// its tail, from the box's foot at `tail` down to the head at (`head`,
/// `foot`): straight down, or slanting where the head is too near the
/// stage's edge for the box to be right over it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placed {
    pub(crate) area: Area,
    pub(crate) tail: f32,
    pub(crate) head: f32,
    pub(crate) foot: f32,
}

impl Placed {
    /// The box around the tail.
    pub(crate) fn stalk(&self) -> Area {
        let bottom = self.area.y + self.area.h;
        Area {
            x: self.tail.min(self.head) - 7.0,
            y: bottom,
            w: (self.tail - self.head).abs() + 14.0,
            h: (self.foot - bottom).max(0.0),
        }
    }

    /// What the bubble covers: its box, and its tail.
    pub(crate) fn parts(&self) -> [Area; 2] {
        [self.area, self.stalk()]
    }

    /// The whole of it: box and tail.
    pub(crate) fn whole(&self) -> Area {
        Area {
            h: (self.foot - self.area.y).max(self.area.h),
            ..self.area
        }
    }
}

impl Area {
    /// How much of this lies over `other`, in square pixels.
    fn over(&self, other: &Area) -> f32 {
        let w = (self.x + self.w).min(other.x + other.w) - self.x.max(other.x);
        let h = (self.y + self.h).min(other.y + other.h) - self.y.max(other.y);
        w.max(0.0) * h.max(0.0)
    }

    /// The same, with `by` more room all round.
    fn grown(&self, by: f32) -> Area {
        Area {
            x: self.x - by,
            y: self.y - by,
            w: self.w + by * 2.0,
            h: self.h + by * 2.0,
        }
    }
}

/// Places speech bubbles, each `(x, y, w, h)` over a head at `x`, `y`
/// (the bubble `w` by `h`, its tail included), one after another on a
/// stage `width` wide: each right over its head if it can be, else moved
/// along or lifted higher (its tail still pointing down at the head), so
/// that it never covers the interface (`interface`: the gauges and
/// handles, the zoom control, the card) or a bubble placed before it, and
/// stays on the stage. Where there is no room anywhere, it goes where it
/// covers least.
pub(crate) fn place_bubbles(
    bubbles: &[(f32, f32, f32, f32)],
    interface: &[Area],
    (width, _height): (f32, f32),
) -> Vec<Placed> {
    let mut placed: Vec<Placed> = Vec::new();
    for (index, &(x, y, w, h)) in bubbles.iter().enumerate() {
        let w = w.min(width - BUBBLE_CLEAR * 2.0).max(1.0);
        // Among others speaking close by, a bubble leans away from them
        // from the start, so theirs have room over their own heads.
        let near = bubbles
            .iter()
            .enumerate()
            .filter(|(other, (ox, oy, ..))| {
                *other != index && (ox - x).abs() < w && (oy - y).abs() < h * 2.0
            })
            .map(|(_, (ox, ..))| *ox)
            .collect::<Vec<_>>();
        let away = if near.is_empty() {
            0.0
        } else {
            let middle = near.iter().sum::<f32>() / near.len() as f32;
            if x < middle {
                -1.0
            } else if x > middle {
                1.0
            } else {
                0.0
            }
        };
        // As far along as a bubble goes: its head under its corner.
        let edge = (0.5 - 16.0 / w).max(0.0);
        let shifts = if away == 0.0 {
            [0.0, -0.25, 0.25, -edge, edge]
        } else {
            [edge, 0.25, 0.0, -0.25, -edge].map(|shift| shift * away)
        };
        let boxed = h - TAIL;
        // What is taken: the interface, the bubbles placed, and over the
        // heads of those still to place, where their tails will go.
        let taken = interface
            .iter()
            .copied()
            .chain(placed.iter().flat_map(|other| other.parts()))
            .chain(bubbles[index + 1..].iter().map(|(ox, oy, ..)| Area {
                x: ox - 10.0,
                y: 0.0,
                w: 20.0,
                h: *oy,
            }))
            .collect::<Vec<_>>();
        let mut best: Option<(f32, Placed)> = None;
        'search: for lift in [0.0, 16.0, 32.0, 56.0, 88.0, 128.0, 176.0, 232.0] {
            for shift in shifts {
                let left = (x - w / 2.0 + shift * w)
                    .min(width - BUBBLE_CLEAR - w)
                    .max(BUBBLE_CLEAR);
                // The tail leaves the box clear of its rounded corners,
                // slanting to a head too near the edge; a box moved along
                // keeps its tail straight down.
                let tail = x.clamp(left + 14.0, left + w - 14.0);
                if (tail - x).abs() > 0.5 && shift != 0.0 {
                    continue;
                }
                let top = y - h - lift;
                if top < BUBBLE_CLEAR {
                    continue;
                }
                let candidate = Placed {
                    area: Area {
                        x: left,
                        y: top,
                        w,
                        h: boxed,
                    },
                    tail,
                    head: x,
                    foot: y,
                };
                let covered = candidate
                    .parts()
                    .iter()
                    .flat_map(|part| {
                        taken
                            .iter()
                            .map(|other| part.grown(BUBBLE_CLEAR).over(other))
                    })
                    .sum::<f32>();
                if best.is_none_or(|(least, _)| covered < least) {
                    best = Some((covered, candidate));
                }
                if covered <= 0.0 {
                    break 'search;
                }
            }
        }
        // Too near the top, or too near an edge, for any of that: right
        // over the head, as low as it can go.
        let chosen = best.map(|(_, placed)| placed).unwrap_or_else(|| {
            let left =
                (x - w / 2.0).clamp(BUBBLE_CLEAR, (width - BUBBLE_CLEAR - w).max(BUBBLE_CLEAR));
            Placed {
                area: Area {
                    x: left,
                    y: (y - h).max(0.0),
                    w,
                    h: boxed.min(y.max(1.0)),
                },
                tail: x.clamp(left + 14.0, left + w - 14.0),
                head: x,
                foot: y,
            }
        });
        placed.push(chosen);
    }
    placed
}

/// Where each pointer is drawn on a stage `width` by `height`, generously:
/// under its handle in the top right, beside the zoom control, or just
/// above the card.
pub(crate) fn hint_area(pointer: Pointer, width: f32, height: f32) -> Area {
    match pointer {
        Pointer::Hands | Pointer::Drawer | Pointer::Letters | Pointer::Strip => Area {
            x: width - 16.0 - HINT_WIDTH - 60.0,
            y: 40.0,
            w: HINT_WIDTH + 60.0,
            h: 140.0,
        },
        Pointer::Zoom => Area {
            x: 16.0,
            y: zoom_top(height) - 20.0,
            w: 48.0 + HINT_WIDTH,
            h: 110.0,
        },
        Pointer::Cards => Area {
            x: width / 2.0 - CARD_WIDTH / 2.0,
            y: height - 460.0,
            w: CARD_WIDTH,
            h: 160.0,
        },
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
    pub(crate) camera_from: Option<Camera>,
    pub(crate) camera_to: Option<Camera>,
    pub(crate) camera_at: Option<Instant>,
    pub(crate) focus: Option<gpui::FocusHandle>,
    /// The clock that keeps a living World moving; dropped (stopping it)
    /// while the window cannot be seen.
    pub(crate) clock: Option<gpui::Task<()>>,
    /// Hears when the window is covered, minimised, or shown again.
    pub(crate) visibility: Option<gpui::Subscription>,
    /// The last chapter whose ending card the player has turned past.
    pub(crate) chapter_read: Option<u32>,
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
    /// stage point; 1 is the whole place.
    pub(crate) zoom: f32,
    pub(crate) zoom_on: (f32, f32),
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
    /// put the camera for them.
    pub(crate) welcome_pan: Option<(SelectionId, f32)>,
    /// A farewell showing over the World, and whether the camera has gone
    /// to whoever says goodbye.
    pub(crate) farewell: Option<super::farewell::Farewell>,
    pub(crate) farewell_panned: bool,
    pub(crate) farewell_speaker: Option<SelectionId>,
    /// Whether the first day's free thing has been offered in this window,
    /// and whether the offer is up now: the hands open on it, the card
    /// held back until it is placed or put away.
    pub(crate) free_offered: bool,
    pub(crate) free_offer: bool,
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
    cards.sort_by_key(|card| snapshot.commands[card[0]].asker.is_none());
    cards
}

/// The most words a World window may show at rest.
pub const RESTING_WORD_LIMIT: usize = 40;

/// How many words a World window shows at rest, the bar above it included:
/// the World's name, when it next moves ("Keeps going without you · next
/// sol in 6 h"), and everything `resting_text` lists.
pub fn words_at_rest(snapshot: &ProjectionSnapshot) -> usize {
    let bar = [
        snapshot.title.clone(),
        "Keeps going without you · next day in 6 h".to_string(),
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

/// How wide a speech bubble's line is, in the width of a Latin letter;
/// a Chinese or Japanese character takes two.
pub(crate) const BUBBLE_LINE: usize = 36;

/// How wide a bubble line is at the text size the player chose: larger
/// text, fewer letters to a line.
fn bubble_line() -> usize {
    ((BUBBLE_LINE as f32 / crate::text_scale()) as usize).max(14)
}

fn text_width(text: &str) -> usize {
    text.chars()
        .map(|character| {
            if !character.is_ascii() && character.is_alphanumeric() && !character.is_alphabetic()
                || ('\u{2e80}'..='\u{9fff}').contains(&character)
                || ('\u{ff00}'..='\u{ffef}').contains(&character)
                || ('\u{3000}'..='\u{303f}').contains(&character)
            {
                2
            } else {
                1
            }
        })
        .sum()
}

/// Punctuation that closes a phrase, and so never begins a line (the
/// Chinese and Japanese line-breaking rule, kinsoku shori).
pub(crate) fn never_starts_a_line(character: char) -> bool {
    matches!(
        character,
        '，' | '。'
            | '、'
            | '！'
            | '？'
            | '：'
            | '；'
            | '」'
            | '』'
            | '）'
            | '】'
            | '》'
            | '〉'
            | '〕'
            | '”'
            | '’'
            | '…'
            | '・'
            | '～'
            | 'ー'
            | ','
            | '.'
            | '!'
            | '?'
            | ':'
            | ';'
            | ')'
            | ']'
    )
}

/// Punctuation that opens a phrase, and so never ends a line.
fn never_ends_a_line(character: char) -> bool {
    matches!(
        character,
        '「' | '『' | '（' | '【' | '《' | '〈' | '〔' | '“' | '‘' | '(' | '['
    )
}

/// A line cut into bubble lines: broken between words, or anywhere in a
/// language written without spaces, but never before a closing mark or
/// after an opening one: the character before a comma goes down to the
/// next line with it.
fn bubble_rows(line: &str) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    let mut row = String::new();
    for word in line.split_whitespace() {
        // A word too wide for a line (or a run of Chinese) is cut where
        // it has to be.
        let mut pieces = Vec::new();
        let mut piece = String::new();
        for character in word.chars() {
            if text_width(&piece) + text_width(&character.to_string()) > bubble_line() {
                let mut carried = Vec::new();
                if never_starts_a_line(character) {
                    // Closing marks run together (。」); the word they
                    // close goes down with them.
                    while piece.chars().last().is_some_and(never_starts_a_line) {
                        carried.extend(piece.pop());
                    }
                    carried.extend(piece.pop());
                }
                while piece.chars().last().is_some_and(never_ends_a_line) {
                    carried.extend(piece.pop());
                }
                if piece.is_empty() {
                    // Nothing left to break before: keep it all on one line.
                    piece.extend(carried.into_iter().rev());
                } else {
                    pieces.push(std::mem::take(&mut piece));
                    piece.extend(carried.into_iter().rev());
                }
            }
            piece.push(character);
        }
        pieces.push(piece);
        for piece in pieces {
            let wide = text_width(&row) + usize::from(!row.is_empty()) + text_width(&piece);
            if !row.is_empty() && wide > bubble_line() {
                rows.push(std::mem::take(&mut row));
            }
            if !row.is_empty() {
                row.push(' ');
            }
            row.push_str(&piece);
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

/// Where a sentence ends: after its closing mark (and any closing quote
/// or bracket after it).
fn ends_a_sentence(character: char) -> bool {
    matches!(character, '.' | '!' | '?' | '…' | '。' | '！' | '？')
}

/// A mark that closes a quotation or an aside: after a sentence's end,
/// it belongs to that sentence (`?"` and `。」`).
fn closes(character: char) -> bool {
    matches!(
        character,
        '"' | '\'' | ')' | ']' | '”' | '’' | '」' | '』' | '）' | '】' | '》' | '〉'
    )
}

/// A line cut into its sentences, each with its closing marks; a
/// language written without spaces is cut after its full stops too.
pub(crate) fn sentences(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut sentence = String::new();
    let characters = line.chars().collect::<Vec<_>>();
    // The mark that ended the sentence, while closing marks follow it.
    let mut ended: Option<char> = None;
    for (index, character) in characters.iter().copied().enumerate() {
        sentence.push(character);
        if ends_a_sentence(character) {
            ended = Some(character);
        } else if !(ended.is_some() && closes(character)) {
            ended = None;
        }
        let Some(mark) = ended else {
            continue;
        };
        let next = characters.get(index + 1).copied();
        // More marks to come: `?!`, `。」`.
        if next.is_some_and(|next| ends_a_sentence(next) || closes(next)) {
            continue;
        }
        // "Mr. Lark" and "3.5" are not the end of anything: a full stop
        // ends a sentence written with spaces only before a space.
        let spaced = !matches!(mark, '。' | '！' | '？');
        if spaced && next.is_some_and(|next| !next.is_whitespace()) {
            continue;
        }
        ended = None;
        let done = std::mem::take(&mut sentence);
        if !done.trim().is_empty() {
            out.push(done.trim().to_string());
        }
    }
    if !sentence.trim().is_empty() {
        out.push(sentence.trim().to_string());
    }
    out
}

/// How many bubble lines a page shows at most: two, or for one sentence
/// too long for two, as many as it takes, up to this, so no sentence is
/// ever cut off and finished on the next page.
const PAGE_MOST: usize = 4;

/// A line cut into pages for a speech bubble. A page holds whole
/// sentences, as many as fit in two bubble lines; a sentence too long for
/// two lines has a page of its own, as tall as it needs (up to
/// [`PAGE_MOST`] lines; only a sentence longer still goes on over pages).
/// So a page never stops in the middle of a sentence and leaves the
/// reader waiting for its end.
pub fn speech_pages(line: &str) -> Vec<String> {
    let joined = |a: &str, b: &str| {
        if a.is_empty() {
            b.to_string()
        } else if a
            .chars()
            .last()
            .is_some_and(|c| c.is_ascii() || c.is_whitespace())
            || b.chars().next().is_some_and(|c| c.is_ascii())
        {
            format!("{a} {b}")
        } else {
            format!("{a}{b}")
        }
    };
    let mut pages: Vec<Vec<String>> = Vec::new();
    let mut page = String::new();
    for sentence in sentences(line) {
        let together = joined(&page, &sentence);
        if bubble_rows(&together).len() <= 2 {
            page = together;
            continue;
        }
        if !page.is_empty() {
            pages.push(bubble_rows(&std::mem::take(&mut page)));
        }
        let rows = bubble_rows(&sentence);
        if rows.len() <= 2 {
            page = sentence;
        } else {
            // A long sentence on its own page, as tall as it needs; one
            // longer than a bubble can hold goes on over pages.
            for chunk in rows.chunks(PAGE_MOST) {
                pages.push(chunk.to_vec());
            }
        }
    }
    if !page.is_empty() {
        pages.push(bubble_rows(&page));
    }
    if pages.is_empty() {
        return vec![String::new()];
    }
    pages.into_iter().map(|rows| rows.join("\n")).collect()
}

/// How long an answer to the player stays over its speaker: long enough
/// to read every page of it.
fn answer_seconds(answer: &str) -> f32 {
    ANSWER_SECONDS.max(LINE_SECONDS * speech_pages(answer).len() as f32 + 2.0)
}

/// How wide a speech bubble can be.
const BUBBLE_ROOM: f32 = 300.0;

/// A speech bubble: what someone says, where [`place_bubbles`] put it,
/// with a tail pointing down at their head.
fn bubble(line: String, placed: Placed, opacity: f32, strong: bool) -> Div {
    let area = placed.whole();
    let stalk = placed.stalk();
    let ink: Hsla = color(tokens::TEXT).into();
    // White by day; after dark the night's own card, never a glare.
    let ground: Hsla = color(tokens::SURFACE).into();
    // Lifted clear of something, the tail reaches down the further, and
    // the narrower; to a head near the edge it slants.
    let (tail, head) = (placed.tail - stalk.x, placed.head - stalk.x);
    let half = if stalk.h > TAIL * 2.0 { 5.0 } else { 7.0 };
    let tail_shape = canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            let h = f32::from(bounds.size.height);
            art::polygon(
                window,
                &[
                    (ox + tail - half, oy - 1.0),
                    (ox + tail + half, oy - 1.0),
                    (ox + head, oy + h),
                ],
                ground,
            );
        },
    )
    .absolute()
    .left(px(stalk.x - area.x))
    .top(px(placed.area.h))
    .w(px(stalk.w))
    .h(px(stalk.h));
    div()
        .absolute()
        .left(px(area.x))
        .top(px(area.y))
        .w(px(area.w))
        .h(px(area.h))
        .opacity(opacity)
        .child(
            div()
                .absolute()
                .left_0()
                .bottom(px(stalk.h))
                .w_full()
                .px_3()
                .py_2()
                .rounded_xl()
                .bg(ground)
                .shadow_md()
                .text_sm()
                .text_center()
                .when(strong, |text| text.font_weight(FontWeight::MEDIUM))
                .text_color(ink)
                .child(line),
        )
        .child(tail_shape)
}

impl ProjectionView {
    pub(crate) fn stage_size(&self, window: &Window) -> (f32, f32) {
        let size = window.viewport_size();
        (f32::from(size.width), f32::from(size.height) - CHROME)
    }

    /// Start the clock that keeps a living World moving while its window is
    /// in front, once, unless nobody can see the window.
    fn keep_living(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    /// Listens, once, for the window being covered or shown: covered, the
    /// clock stops; shown again, the World is drawn at once, and that frame
    /// starts the clock and the pace again.
    fn watch_visibility(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
    }

    fn cycle_card(&mut self, by: isize, cx: &mut Context<Self>) {
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
    fn turn_card(&mut self, cx: &mut Context<Self>) {
        self.looking.card_back = !self.looking.card_back;
        pointers::used(Pointer::Cards);
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
        if self.looking.asking != Some(who) {
            if let Some(input) = &self.looking.say {
                input.update(cx, |input, cx| input.clear(cx));
            }
            self.looking.said_at = None;
        }
        if self.looking.say.is_none() && self.snapshot.capabilities.talk {
            self.looking.say =
                Some(cx.new(|cx| crate::text_input::TextInput::new(ui::t("Say something…"), cx)));
        }
        self.looking.asking = Some(who);
        self.looking.answered = None;
        self.looking.poked = Some((who, Instant::now()));
        self.selected = Some(who);
        cx.notify();
    }

    /// Whether the player is typing what they say to someone.
    fn typing(&self, window: &Window, cx: &Context<Self>) -> bool {
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
    fn say(&mut self, cx: &mut Context<Self>) {
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
                    this.status = Some(format!("The moment passed before {name} could answer."));
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

    /// Says `words` to `who` as if the player had typed them: the person
    /// card's quick reply.
    fn say_for_them(&mut self, who: SelectionId, words: String, cx: &mut Context<Self>) {
        self.ask(who, cx);
        if let Some(input) = self.looking.say.clone() {
            input.update(cx, |input, cx| input.set_text(words, cx));
            self.say(cx);
        }
    }

    /// The welcome waits for the town to be painted: until it is, the
    /// World's opening moment has not begun, and nobody speaks. As it
    /// opens, the camera stands where whoever welcomes the player stands.
    fn notice_paint(&mut self, stage: &Stage, painted: bool) {
        let was = std::mem::replace(&mut self.looking.painted, painted);
        if !painted {
            if self.looking.opening.is_some() {
                self.looking.opening = Some(Instant::now());
            }
            return;
        }
        // Kept on the welcomer while the window settles its size, until the
        // player pans away.
        if let Some((who, at)) = self.looking.welcome_pan {
            if self.looking.pan != Some(at) {
                self.looking.welcome_pan = None;
            } else if let Some(x) = self.stage_x(stage, who) {
                let x = super::arrival::best_view(stage, x);
                self.looking.pan = Some(x);
                self.looking.welcome_pan = Some((who, x));
            }
            return;
        }
        if was || self.retelling.is_some() || self.looking.pan.is_some() {
            return;
        }
        if let Some((who, x)) = voices_now(&self.snapshot).first().and_then(|voice| {
            self.stage_x(stage, voice.speaker)
                .map(|x| (voice.speaker, super::arrival::best_view(stage, x)))
        }) {
            self.looking.pan = Some(x);
            self.looking.welcome_pan = Some((who, x));
        }
    }

    /// On a World's first day, once the welcome has been heard, the
    /// player's hands open on something free to place (the first card,
    /// which asks for money, waits until it is placed or put away): the
    /// toy before the ledger, well inside the first 20 seconds.
    fn notice_free_offer(&mut self) {
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

    /// Where along the stage someone stands, by the middle of their
    /// figure.
    fn stage_x(&self, stage: &Stage, who: SelectionId) -> Option<f32> {
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
    fn notice_favour(&mut self, stage: &Stage) {
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
    fn favour_line(&self) -> Option<(SelectionId, String, f32)> {
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
    fn finish_saying(
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
                self.status = Some(format!("Couldn't say that: {error}"));
                self.status_is_error = true;
                false
            }
        };
        cx.notify();
        said
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

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
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
                } else if (self.view_zoom() - 1.0).abs() > 0.01 {
                    self.looking.zoom = 1.0;
                    cx.notify();
                }
            }
            _ if self.retelling.is_some() => {
                if matches!(key, "right" | "enter" | "space") {
                    self.step_film(cx);
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
    fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use super::arrival::{enter_does, passes_day, EnterContext, EnterDoes};
        if self.controller.is_some() {
            if let Some(number) = chapter_just_ended(&self.snapshot, self.looking.chapter_read)
                .map(|chapter| chapter.number)
            {
                self.looking.chapter_read = Some(number);
                cx.notify();
                return;
            }
        }
        let card = self.card_command();
        let context = EnterContext {
            asking: self.looking.asking.is_some(),
            covered: self.looking.drawer
                || self.looking.hands.is_some()
                || self.looking.marking.card.is_some(),
            card_shown: card.is_some(),
            card_passes_day: card.is_some_and(|command| passes_day(&command.id)),
            scene_focused: self
                .looking
                .focus
                .as_ref()
                .is_some_and(|focus| focus.is_focused(window)),
        };
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
                    framed.zoom.min(1.35),
                    framed.x,
                    framed.y,
                ))
            })
            // Otherwise wherever the player has zoomed and panned to.
            .unwrap_or_else(|| {
                let zoom = self.view_zoom();
                Camera::around(
                    stage,
                    zoom,
                    self.looking.pan.unwrap_or(whole.x),
                    if zoom > 1.0 {
                        self.looking.zoom_on.1
                    } else {
                        stage.height / 2.0
                    },
                )
            });
        let current = match (self.looking.camera_from, self.looking.camera_to) {
            (Some(from), Some(to)) => {
                from.toward(to, since(self.looking.camera_at) / CAMERA_SECONDS)
            }
            _ => whole,
        };
        // A drag moves the view with the hand, at once.
        if self.looking.drag.is_some_and(|(.., moved)| moved) {
            self.looking.camera_from = Some(target);
            self.looking.camera_to = Some(target);
            self.looking.camera_at = Some(Instant::now());
            self.looking.camera_now = Some(target);
            return target;
        }
        if self.looking.camera_to != Some(target) {
            self.looking.camera_from = Some(current);
            self.looking.camera_to = Some(target);
            // The wheel follows the hand at once; everything else glides.
            let wheel = self.looking.asking.is_none() && self.current_beat().is_none();
            self.looking.camera_at = Some(if wheel {
                Instant::now() - Duration::from_secs_f32(CAMERA_SECONDS * 0.7)
            } else {
                Instant::now()
            });
        }
        self.looking.camera_now = Some(current);
        current
    }

    /// The wheel zooms in on the place around the pointer, and back out;
    /// a sideways scroll (or the wheel with shift held) pans along the
    /// place.
    fn on_wheel(
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

    /// How close the player has zoomed: 1 is one window of the place.
    fn view_zoom(&self) -> f32 {
        if self.looking.zoom <= 0.0 {
            1.0
        } else {
            self.looking.zoom
        }
    }

    /// Whether the arrow keys pan: always with shift, and otherwise along a
    /// panorama whenever there are no answers to lean between.
    fn pans_with_arrows(&self, event: &KeyDownEvent) -> bool {
        let wide = self.snapshot.canvas.width.unwrap_or(1.0) > 1.01 || self.view_zoom() > 1.01;
        wide && (event.keystroke.modifiers.shift || self.card_answers().len() < 2)
    }

    /// Pans by `screens` window widths.
    fn pan_by_screens(&mut self, screens: f32, window: &Window, cx: &mut Context<Self>) {
        let (width, _) = self.stage_size(window);
        self.pan_by(screens * width, window, cx);
    }

    /// Pans the view by `dx` window pixels along the place.
    fn pan_by(&mut self, dx: f32, window: &Window, cx: &mut Context<Self>) {
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
    fn drag_from(&mut self, x: f32) {
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
    fn drag_to(&mut self, x: f32, window: &Window, cx: &mut Context<Self>) {
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
    fn drag_end(&mut self) {
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
    fn on_pinch(&mut self, event: &gpui::PinchEvent, window: &Window, cx: &mut Context<Self>) {
        if event.delta != 0.0 {
            self.zoom_by(1.0 + event.delta, Some(event.position), window, cx);
        }
    }

    /// Zooms by `factor` on the stage point under `at` (a place in the
    /// window), or on the middle of what is in view.
    fn zoom_by(
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
        let (x, y) = camera.stage_point(&stage, x, y);
        let before = self.view_zoom();
        let mut zoom = (before * factor).clamp(Camera::least(&stage), diorama::ZOOM_MOST);
        if (zoom - 1.0).abs() < 0.02 {
            zoom = 1.0;
        }
        if (zoom - before).abs() > f32::EPSILON {
            // The point under the pointer stays under it.
            let keep = camera.zoom / zoom;
            self.looking.zoom = zoom;
            self.looking.pan = Some(x - (x - camera.x) * keep);
            self.looking.zoom_on = (x - (x - camera.x) * keep, y - (y - camera.y) * keep);
            pointers::used(Pointer::Zoom);
            cx.notify();
        }
    }

    /// What the scene lights up now, and in what colour.
    fn glows(&self) -> Glows {
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
    fn hand_targets(&self) -> Option<Vec<SelectionId>> {
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
    fn deed_at(&self, target: SelectionId) -> Option<String> {
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
    fn place_on_ground(
        &mut self,
        position: gpui::Point<gpui::Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(hands) = self.looking.hands.as_ref() else {
            return false;
        };
        if !matches!(
            hands.verb.as_deref(),
            Some("Build" | "Decorate" | "Plant" | "Move")
        ) || hands.thing.as_deref().is_none_or(|thing| thing == "*")
        {
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
        let spot = diorama::ground_spot(&stage, x);
        self.looking.hands = None;
        self.invoke_command(format!("{deed}@{spot}"), cx);
        true
    }

    /// The player's last thing made or moved, which they can take back
    /// this period: the command, and what it says.
    fn undo_command(&self) -> Option<(String, String)> {
        self.snapshot
            .deeds()
            .find(|(_, command, hand)| hand.verb == "Undo" && command.unavailable.is_none())
            .map(|(_, command, _)| (command.id.clone(), command.title.clone()))
    }

    fn undo(&mut self, cx: &mut Context<Self>) {
        if let Some((command, _)) = self.undo_command() {
            self.invoke_command(command, cx);
        }
    }

    /// A photograph of the scene: the chrome is hidden for a frame, the
    /// window's picture is saved to Pictures, and the chrome comes back.
    fn take_photo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
                    this.cue(crate::Cue::Flip);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn toggle_hands(&mut self, cx: &mut Context<Self>) {
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

    fn card_command(&self) -> Option<&ProjectionCommand> {
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
        // 120 times a second); with Reduce Motion on, a few times a second
        // is enough for what still moves; covered, not at all.
        if pace(
            window.is_visible(),
            window.is_window_active(),
            cx.reduce_motion(),
        ) == Pace::EveryFrame
        {
            window.request_animation_frame();
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
        self.notice_paint(&stage, diorama::painted(window));
        self.notice_free_offer();
        self.notice_favour(&stage);
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
            let speaker = if self.stage_x(&stage, who).is_some() {
                Some(who)
            } else {
                out
            };
            if let Some(speaker) = speaker {
                self.looking.farewell_speaker = Some(speaker);
                self.looking.pan = self.stage_x(&stage, speaker);
            }
        }
        let camera = self.camera(&stage);
        self.notice_moments();

        // Who is needed where they are, and who is talking.
        let speaking = voices_now(&self.snapshot);
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
        } else if self.retelling.is_some() {
            beat_voice.map(|voice| {
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
        let mut living = diorama::living(
            &stage,
            &self.snapshot,
            seconds,
            daylight,
            &pinned,
            before
                .as_ref()
                .map(|(stage, snapshot, progress)| (stage, *snapshot, *progress)),
        );
        let poked = self
            .looking
            .poked
            .map(|(who, at)| [(who, at.elapsed().as_secs_f32())].into_iter().collect())
            .unwrap_or_default();
        diorama::wave(&mut living, &stage, &self.snapshot, &poked, still);
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
            &self.snapshot,
            &stage,
            &living,
            camera,
            seconds,
            daylight,
            &self.glows(),
            rising,
        )
        .stilled(still);
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
            for person in &mut frame.people {
                if self
                    .snapshot
                    .canvas
                    .items
                    .get(person.index)
                    .map(|item| item.id)
                    == Some(*speaker)
                    && person.stance != world_projection::Stance::Walking
                {
                    person.stance = world_projection::Stance::Talking;
                }
            }
        }
        // Everyone nearby turns to whoever is speaking.
        if let Some((speaker, _, fade, _)) = &line {
            if let Some(index) = self
                .snapshot
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
            .on_key_down(
                cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    this.on_key(event, window, cx)
                }),
            )
            .on_scroll_wheel(
                cx.listener(|this, event: &gpui::ScrollWheelEvent, window, cx| {
                    this.on_wheel(event, window, cx)
                }),
            )
            .on_pinch(cx.listener(|this, event: &gpui::PinchEvent, window, cx| {
                this.on_pinch(event, window, cx)
            }))
            // Dragging pans along the place.
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                    this.drag_from(f32::from(event.position.x))
                }),
            )
            .on_mouse_move(
                cx.listener(|this, event: &gpui::MouseMoveEvent, window, cx| {
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
                // The scene: its still layers painted off this thread
                // and faded in, what moves drawn live.
                let started = Instant::now();
                let scene = diorama::scene(frame.clone(), window);
                let planned = started.elapsed();
                scene.child(
                    canvas(
                        |_, _, _| (),
                        move |_, _, _, _| frame_log(planned + crate::painter::take_frame_time()),
                    )
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
                        if this.just_dragged() {
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
        let reach = diorama::reach_of(
            &frame
                .people
                .iter()
                .map(|person| (person.x, person.y, person.height))
                .collect::<Vec<_>>(),
        );
        // Named on hover, or while speaking; never two names over one
        // another, and none before the scene is painted.
        let speaker_x = line.as_ref().and_then(|(who, ..)| {
            frame
                .people
                .iter()
                .find(|person| self.snapshot.canvas.items[person.index].id == *who)
                .map(|person| person.x)
        });
        for (person, reach) in frame.people.iter().zip(reach) {
            let item = &self.snapshot.canvas.items[person.index];
            let selection = item.id;
            let group = SharedString::from(format!("person-{}", selection.stable_key()));
            let speaking = line.as_ref().is_some_and(|(who, ..)| *who == selection);
            let named = painted && speaking;
            let hover =
                painted && !speaking && speaker_x.is_none_or(|x| (x - person.x).abs() > NAME_ROOM);
            heads.push((selection, person.x, person.y - person.height * 1.08));
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
                    .child(name_tag(first_name(&item.label)).when(!named, |tag| {
                        tag.opacity(0.0).when(hover, |tag| {
                            tag.group_hover(group, |style| style.opacity(1.0))
                        })
                    }))
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
        if let Some((_, x, y)) = super::arrival::marked(&self.snapshot)
            .filter(|_| self.looking.painted && self.retelling.is_none())
            .and_then(|whom| heads.iter().find(|(id, ..)| *id == whom))
        {
            let hint = self
                .snapshot
                .favour
                .as_ref()
                .map(|favour| favour.hint.clone())
                .unwrap_or_default();
            root = root.child(favour_mark(*x, *y, hint, seconds));
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
        let interface = self.interface_areas((width, height), asker_x, asking);
        let placed = line.as_ref().and_then(|(who, text, ..)| {
            let (_, x, y) = heads.iter().find(|(id, ..)| id == who)?;
            let (w, h) = bubble_size(text);
            place_bubbles(&[(*x, *y, w, h)], &interface, (width, height)).pop()
        });
        let pointing = self.point(width, height, placed.map(|placed| placed.whole()));
        // Whoever is talking, over their head, once there is a place
        // under them to talk in.
        if let (Some((_, text, fade, strong)), Some(placed), true) = (line, placed, painted) {
            root = root.child(bubble(text, placed, fade, strong));
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
                    .aria_label(format!("{from} gave you {}", keepsake.what))
                    .max_w(px(420.0))
                    .px_4()
                    .py_2()
                    .rounded_xl()
                    .bg(color(tokens::SURFACE))
                    .shadow_md()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(ui::caption(format!("{from} gave you")))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(color(tokens::TEXT))
                            .child(capitalized(&keepsake.what)),
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
        ]
        .into_iter()
        .filter_map(|(at, words)| Some((at?, words)))
        .max_by_key(|(at, _)| *at);
        if let Some((at, words)) = saved.filter(|(at, _)| at.elapsed().as_secs_f32() < 2.5) {
            let age = at.elapsed().as_secs_f32();
            root = root.child(
                div()
                    .absolute()
                    .bottom(px(24.0))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .opacity((age / 0.2).min((2.5 - age) / 0.5).clamp(0.0, 1.0))
                    .child(pill().child(ui::t(words))),
            );
        }
        if let Some(farewell) = lit(|| self.render_farewell(cx)) {
            // A farewell has the place to itself: no card, no handles.
            return root.child(farewell).into_any_element();
        }
        root = root.child(self.render_hud(pointing, cx));
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
            let card = if self.moment_up()
                || self.reading.page.is_some()
                || marking.design.is_some()
                || marking.card.is_some()
                || marking.offers.is_some()
            {
                None
            } else if self.retelling.is_some() {
                lit(|| self.render_retelling(cx))
            } else if let Some(chapter) = lit(|| self.render_chapter_end(cx)) {
                Some(chapter)
            } else {
                lit(|| self.render_card(cx))
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
                root = root.child(
                    bottom_card(card, card_dock(room, asker_x))
                        .when(self.looking.drawer, |card| card.right(px(DRAWER_WIDTH))),
                );
            }
        }
        if let Some((who, x, y)) = self
            .looking
            .asking
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
        root.into_any_element()
    }

    /// Whether the player's hands have anything to do here.
    fn has_deeds(&self) -> bool {
        self.controller.is_some()
            && self.retelling.is_none()
            && !is_beginning(&self.snapshot)
            && self.snapshot.deeds().next().is_some()
    }

    /// The gentle pointer to show now, and how long it has shown: it keeps
    /// to the app's record (each once, none at what has been used), waits
    /// while anything else is going on, and never covers a line being said.
    fn point(&mut self, width: f32, height: f32, spoken: Option<Area>) -> Option<(Pointer, f32)> {
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
    fn pointer_hint(
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
    fn render_zoom(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let closest = self.view_zoom() >= ZOOM_MOST - 0.01;
        let least = 1.0 / self.snapshot.canvas.width.unwrap_or(1.0).max(1.0);
        let whole = self.view_zoom() <= least + 0.01;
        div()
            .id("zoom")
            .role(Role::Group)
            .aria_label(ui::t("Zoom"))
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
    fn render_hud(&self, pointing: Option<(Pointer, f32)>, cx: &mut Context<Self>) -> Div {
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
            right = right.child(pill().font_weight(FontWeight::SEMIBOLD).child(
                crate::i18n::moment_label(&self.snapshot, self.snapshot.world_time),
            ));
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
            .filter(|_| self.controller.is_some() && self.retelling.is_none())
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
                    .child(ui::t(format!("Find {name}")))
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
            .child(gauges)
            .child(right)
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
    fn render_hands(&self, cx: &mut Context<Self>) -> Option<Div> {
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
        // A question's card says what is asked; a lone choice says itself.
        text = text.child(div().text_base().font_weight(FontWeight::SEMIBOLD).child(
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
            .border_1()
            .border_color(color(tokens::BORDER))
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
    fn render_chapter_end(&self, cx: &mut Context<Self>) -> Option<Div> {
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
            .role(Role::Group)
            .aria_label(ui::t(format!("Asking {name}")))
            .w(px(ASKING_WIDTH))
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
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.open_legend(who, cx)),
                                ),
                        )
                    })
                    .child(arrow_button(
                        "asking-close",
                        "×",
                        "Close",
                        cx.listener(|this, _, _, cx| this.look_away(cx)),
                    )),
            );
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
                            .child(crate::i18n::quoted(&words))
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
                            .child(crate::i18n::quoted(thanks)),
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
                    .child(talk.question.clone())
                    .on_click(cx.listener(move |this, _, _, cx| this.answer(index, cx))),
            );
            if asked {
                card = card.child(
                    div()
                        .px_3()
                        .text_sm()
                        .text_color(color(tokens::TEXT))
                        .child(crate::i18n::quoted(&talk.answer)),
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
            card = card.child(
                div()
                    .pt_1()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().min_w(px(0.0)).child(input))
                    .child(
                        ui::button("say", "Say", ButtonKind::Secondary)
                            .on_click(cx.listener(|this, _, _, cx| this.say(cx))),
                    ),
            );
        }
        card = card.child(
            div()
                .id("ask-more")
                .role(Role::Button)
                .pt_1()
                .text_xs()
                .text_color(color(tokens::TEXT_SECONDARY))
                .cursor_pointer()
                .hover(|style| style.text_color(color(tokens::ACCENT_TEXT)))
                .child(ui::t(format!("More about {}", first_name(&name))))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.selected = Some(who);
                    this.open_drawer();
                    cx.notify();
                })),
        );
        let area = self.asking_area(who, x, head, (stage.width, stage.height));
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
    fn asking_area(
        &self,
        who: SelectionId,
        x: f32,
        head: f32,
        (width, height): (f32, f32),
    ) -> Area {
        let left = if x + 40.0 + ASKING_WIDTH < width - 12.0 {
            x + 40.0
        } else {
            (x - 40.0 - ASKING_WIDTH).max(12.0)
        };
        let talked = self
            .snapshot
            .exchanges_with(who)
            .count()
            .min(CONVERSATION_SHOWN) as f32;
        let tall = 360.0 + talked * 96.0;
        Area {
            x: left,
            y: (head + 8.0).clamp(64.0, (height - tall).max(64.0)),
            w: ASKING_WIDTH,
            h: tall,
        }
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
        let (from, to) = if is_beginning(&self.snapshot) {
            (0.0, width)
        } else {
            card_dock(room, asker_x).span(room)
        };
        areas.push(Area {
            x: from,
            y: height - CARD_MARGIN - CARD_ROOM,
            w: to - from,
            h: CARD_ROOM + CARD_MARGIN,
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

    /// What the player and someone said to each other today, latest last:
    /// the player's words small and to the right, the answer in quotes.
    fn render_conversation(&self, who: SelectionId, cx: &mut Context<Self>) -> Div {
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
                );
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
                            .child(words.clone()),
                    ),
                )
                .child(
                    div()
                        .px_1()
                        .text_sm()
                        .text_color(color(tokens::TEXT_TERTIARY))
                        .child(thinking_dots(self.looking.started)),
                );
        }
        conversation
    }
}

fn frame_glows(frame: &diorama::Frame, index: usize) -> bool {
    frame
        .people
        .iter()
        .any(|person| person.index == index && person.glow.is_some())
}

/// Something the player reads (a card, a page), built in the window's own
/// light rather than the night's (see [`world_theme::reading`]), its
/// words too.
fn lit<E: Styled>(build: impl FnOnce() -> Option<E>) -> Option<E> {
    world_theme::reading(|| build().map(|surface| surface.text_color(color(tokens::TEXT))))
}

/// [`lit`], for something always there.
fn lit_one<E: Styled>(build: impl FnOnce() -> E) -> E {
    world_theme::reading(|| build().text_color(color(tokens::TEXT)))
}

/// How far from a speaker, on screen, someone's name would sit over the
/// speaker's: their name is not shown on hover.
const NAME_ROOM: f32 = 72.0;

/// A name under something on the scene.
/// The mark over whom a favour is for: a small lantern-gold drop that
/// bobs gently, named for a screen reader by how the favour is done.
pub(crate) fn favour_mark(x: f32, head: f32, hint: String, seconds: f32) -> Stateful<Div> {
    const SIDE: f32 = 14.0;
    let bob = (seconds * 2.4).sin() * 2.0;
    div()
        .id("favour-mark")
        .role(Role::Image)
        .aria_label(hint)
        .debug_selector(|| "favour-mark".into())
        .absolute()
        .left(px(x - SIDE / 2.0))
        .top(px(head - SIDE - 14.0 + bob))
        .size(px(SIDE))
        .rounded_full()
        .border_2()
        .border_color(color(tokens::SURFACE))
        .bg(color(tokens::ACCENT))
        .shadow_md()
}

pub(crate) fn name_tag(name: String) -> Div {
    div()
        .mt_1()
        .px_2()
        .py(px(1.0))
        .rounded_full()
        .bg(scene_paper().opacity(0.88))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(color(tokens::TEXT))
        .whitespace_nowrap()
        .child(name)
}

/// The paper that cards and buttons over the scene are cut from: white
/// by day, and after dusk a dimmer, warmer sheet, as if lit by a lamp, so
/// nothing glares over a night sky. The dark appearance has its own.
pub(crate) fn scene_paper() -> gpui::Rgba {
    // The tests' pictures keep to the day's paper whatever the clock says,
    // unless an hour is pinned.
    if cfg!(test) && scene::pinned_hour().is_none() {
        return paper_at(scene::Daylight::Day, world_theme::is_dark());
    }
    paper_at(scene::daylight_now(), world_theme::is_dark())
}

pub(crate) fn paper_at(daylight: scene::Daylight, dark: bool) -> gpui::Rgba {
    if dark {
        return color(tokens::SURFACE);
    }
    match daylight {
        scene::Daylight::Night => gpui::rgb(0xd8cfbd),
        scene::Daylight::Dusk => gpui::rgb(0xece3d2),
        _ => color(tokens::SURFACE),
    }
}

/// A soft pill over the sky.
fn pill() -> Div {
    div()
        .px_3()
        .py(px(6.0))
        .rounded_full()
        .bg(scene_paper().opacity(0.86))
        .shadow_sm()
        .text_sm()
        .text_color(color(tokens::TEXT))
        .flex()
        .items_center()
        .gap_2()
}

/// A gauge as a pill: its name and a short bar, and while a choice is on
/// the table which way the choice would move it and where it would end.
/// How bright (as the eye has it, 0 to 1) a colour on the interface may
/// be after dark, so it is never brighter than the night sky.
const NIGHT_BRIGHTNESS: f32 = 0.24;

/// A colour dimmed, if it must be, to be no brighter than the night sky:
/// its hue and its saturation kept.
pub(crate) fn at_night(colour: Hsla) -> Hsla {
    let rgba: gpui::Rgba = colour.into();
    let luma = 0.2126 * rgba.r + 0.7152 * rgba.g + 0.0722 * rgba.b;
    if luma <= NIGHT_BRIGHTNESS {
        return colour;
    }
    let by = NIGHT_BRIGHTNESS / luma;
    gpui::Rgba {
        r: rgba.r * by,
        g: rgba.g * by,
        b: rgba.b * by,
        a: rgba.a,
    }
    .into()
}

fn hud_gauge(gauge: &world_projection::Gauge, shown: f32, by: Option<i32>) -> Stateful<Div> {
    const BAR: f32 = 64.0;
    // After dark the fill keeps its colour but not its glow: no brighter
    // than the night sky it hangs in.
    let night = |colour: Hsla| {
        if scene::daylight_now() == scene::Daylight::Night {
            at_night(colour)
        } else {
            colour
        }
    };
    let fill: Hsla = night(color(scene::tone_token(gauge.tone)).into());
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
                .bg(night(color(tokens::ACCENT).into()).opacity(0.55)),
        );
    }
    // The reading in the World's own words, a number where it counts
    // one ("11,485"), so a cost can be weighed against it.
    let mut pill = pill()
        .id(SharedString::from(format!("gauge-{}", gauge.id)))
        .role(Role::Meter)
        .aria_label(format!("{}: {}", gauge.label, gauge.reading))
        .child(
            div()
                .text_xs()
                .text_color(color(tokens::TEXT_SECONDARY))
                .child(gauge.label.clone()),
        )
        .child(bar)
        .when(!gauge.reading.trim().is_empty(), |pill| {
            pill.child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color(tokens::TEXT))
                    .child(gauge.reading.clone()),
            )
        });
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

/// The hands' handle: a plus, pale on the accent while open.
fn plus_glyph(open: bool) -> gpui::Canvas<()> {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let ink: Hsla = if open {
                color(tokens::SURFACE).into()
            } else {
                color(tokens::TEXT_SECONDARY).into()
            };
            let x = f32::from(bounds.origin.x);
            let y = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            art::rect(window, x + w / 2.0 - 1.0, y + 2.0, 2.0, h - 4.0, 1.0, ink);
            art::rect(window, x + 2.0, y + h / 2.0 - 1.0, w - 4.0, 2.0, 1.0, ink);
        },
    )
}

/// The strip's handle: a long, low window with a sliver of land in it.
fn strip_glyph() -> gpui::Canvas<()> {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let ink: Hsla = color(tokens::TEXT_SECONDARY).into();
            let x = f32::from(bounds.origin.x);
            let y = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            // The screen's edge, and the strip along its foot.
            art::rect(window, x + 1.0, y + 2.0, w - 2.0, 1.5, 0.5, ink);
            art::rect(window, x + 1.0, y + 2.0, 1.5, h - 4.0, 0.5, ink);
            art::rect(window, x + w - 2.5, y + 2.0, 1.5, h - 4.0, 0.5, ink);
            art::rect(
                window,
                x + 1.0,
                y + h * 0.62,
                w - 2.0,
                h * 0.38 - 2.0,
                1.0,
                ink,
            );
        },
    )
}

/// A pointer's little arrow, in the accent colour.
fn caret(direction: Caret) -> gpui::Canvas<()> {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let ink: Hsla = color(tokens::ACCENT).into();
            let x = f32::from(bounds.origin.x);
            let y = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            let points = match direction {
                Caret::Up => [(x, y + h), (x + w, y + h), (x + w / 2.0, y)],
                Caret::Down => [(x, y), (x + w, y), (x + w / 2.0, y + h)],
                Caret::Left => [(x + w, y), (x + w, y + h), (x, y + h / 2.0)],
            };
            art::polygon(window, &points, ink);
        },
    )
}

/// A gentle pointer at `pointer`'s thing: what it is and how to reach it,
/// with a little arrow toward it and a way to put it away. A screen reader
/// hears it as a status as it arrives.
pub(crate) fn pointer_hint(
    pointer: Pointer,
    direction: Caret,
    dismiss: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> Stateful<Div> {
    let words = ui::t(pointer.words());
    let note = div()
        .w(px(HINT_WIDTH))
        .flex()
        .items_start()
        .gap_2()
        .px_3()
        .py_2()
        .rounded_lg()
        .bg(color(tokens::SURFACE))
        .border_1()
        .border_color(color(tokens::ACCENT))
        .shadow_md()
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .text_sm()
                .text_color(color(tokens::TEXT))
                .child(words.clone()),
        )
        .child(
            ui::named(
                div().id(SharedString::from(format!(
                    "pointer-dismiss-{}",
                    pointer.key()
                ))),
                "Dismiss this hint",
            )
            .role(Role::Button)
            .flex_shrink_0()
            .size(px(20.0))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(color(tokens::TEXT_SECONDARY))
            .cursor_pointer()
            .hover(|style| style.bg(color(tokens::ROW_HOVER)))
            .child("×")
            .on_click(dismiss),
        );
    let hint = div()
        .id(SharedString::from(format!("pointer-{}", pointer.key())))
        .role(Role::Status)
        .aria_label(words)
        .debug_selector(|| format!("pointer-{}", pointer.key()))
        .flex();
    match direction {
        Caret::Up => hint
            .flex_col()
            .items_end()
            .child(caret(direction).w(px(14.0)).h(px(8.0)).mr(px(13.0)))
            .child(note),
        Caret::Down => hint
            .flex_col()
            .items_start()
            .child(note)
            .child(caret(direction).w(px(14.0)).h(px(8.0)).ml(px(24.0))),
        Caret::Left => hint
            .flex_row()
            .items_center()
            .child(caret(direction).w(px(8.0)).h(px(14.0)))
            .child(note),
    }
}

/// One of the zoom control's two buttons, greyed at the end of its travel.
pub(crate) fn zoom_button(
    id: &'static str,
    glyph: &'static str,
    name: &'static str,
    spent: bool,
) -> Stateful<Div> {
    ui::named(div().id(id), name)
        .role(Role::Button)
        .size(px(28.0))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .text_base()
        .font_weight(FontWeight::MEDIUM)
        .text_color(color(if spent {
            tokens::TEXT_TERTIARY
        } else {
            tokens::TEXT_SECONDARY
        }))
        .when(!spent, |button| {
            button
                .cursor_pointer()
                .hover(|style| style.bg(color(tokens::ROW_HOVER)))
        })
        .child(glyph)
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

/// One entry in a list a screen reader reads out whole: a letter, a
/// keepsake, a chapter, a thing in the book.
pub(crate) fn list_entry(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    div().id(id).role(Role::ListItem).aria_label(label)
}

/// One of the hands' verbs, as a tab: chosen or not.
pub(crate) fn verb_tab(verb: &'static str, chosen: bool) -> Stateful<Div> {
    labelled_tab(verb, verb, chosen)
}

/// A verb's tab under another name.
pub(crate) fn labelled_tab(verb: &'static str, label: &'static str, chosen: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(format!("hands-verb-{verb}")))
        .role(Role::Tab)
        .aria_label(ui::t(label))
        .aria_selected(chosen)
        .px_2()
        .py(px(3.0))
        .rounded_full()
        .text_sm()
        .cursor_pointer()
        .when(chosen, |tab| {
            tab.bg(color(tokens::ACCENT))
                .text_color(color(tokens::SURFACE))
        })
        .when(!chosen, |tab| {
            tab.text_color(color(tokens::TEXT_SECONDARY))
                .hover(|style| style.bg(color(tokens::ROW_SELECTED)))
        })
        .child(ui::t(label))
}

/// One answer to the question on the card, the `position`th of `count`:
/// a button, leaned toward (`chosen`) or not. One that cannot be chosen
/// now is shown greyed so the player sees what it would have been, with
/// why not when they lean on it; a screen reader hears why at once.
fn answer_button(
    position: usize,
    count: usize,
    title: String,
    chosen: bool,
    unavailable: Option<&str>,
) -> Stateful<Div> {
    let row = div()
        .id(SharedString::from(format!("answer-{position}")))
        .role(Role::Button)
        .aria_label(title.clone())
        .aria_position_in_set(position + 1)
        .aria_size_of_set(count)
        .px_3()
        .py(px(5.0))
        .rounded_lg()
        .border_1()
        .text_sm();
    if let Some(reason) = unavailable {
        let row = row
            .when(!reason.is_empty(), |row| {
                row.aria_description(ui::t(format!("Not now: {reason}")))
            })
            .border_color(color(tokens::BORDER))
            .text_color(color(tokens::TEXT_TERTIARY))
            .flex()
            .justify_between()
            .gap_3()
            .child(title);
        return if chosen && !reason.is_empty() {
            row.child(div().text_xs().child(reason.to_string()))
        } else {
            row
        };
    }
    row.cursor_pointer()
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
        .child(title)
}

pub(crate) fn arrow_button(
    id: &'static str,
    glyph: &'static str,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    ui::named(div().id(id), label)
        .role(Role::Button)
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

/// Where the card floats at the bottom of the World: centred, or docked
/// to one side with the width it takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dock {
    pub(crate) side: DockSide,
    pub(crate) w: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DockSide {
    Centre,
    Left,
    Right,
}

/// How far the card keeps from the stage's edges.
const CARD_MARGIN: f32 = 16.0;
/// How far a docked card keeps from the middle of the stage.
const CARD_CLEAR: f32 = 64.0;

/// Where the card goes on a stage `width` wide, with whoever asks standing
/// at `asker_x` on screen: on the far side from them, no wider than half
/// the stage less its margins, so the half they stand in stays clear; with
/// nobody asking, centred.
pub(crate) fn card_dock(width: f32, asker_x: Option<f32>) -> Dock {
    let full = (CARD_WIDTH * crate::text_scale().sqrt()).min(width - CARD_MARGIN * 2.0);
    match asker_x {
        Some(x) if width >= 720.0 => Dock {
            side: if x > width / 2.0 {
                DockSide::Left
            } else {
                DockSide::Right
            },
            // Clear of the middle by more than anyone's shoulders, so
            // someone standing right at it is still in the open half.
            w: full.min(width / 2.0 - CARD_CLEAR),
        },
        _ => Dock {
            side: DockSide::Centre,
            w: full,
        },
    }
}

impl Dock {
    /// The span of the stage the card covers, left to right.
    pub(crate) fn span(&self, width: f32) -> (f32, f32) {
        match self.side {
            DockSide::Left => (CARD_MARGIN, CARD_MARGIN + self.w),
            DockSide::Right => (width - CARD_MARGIN - self.w, width - CARD_MARGIN),
            DockSide::Centre => (width / 2.0 - self.w / 2.0, width / 2.0 + self.w / 2.0),
        }
    }
}

/// A card floating at the bottom of the World, where `dock` puts it.
fn bottom_card(card: impl IntoElement, dock: Dock) -> Div {
    let row = div()
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .pb_4()
        .px(px(CARD_MARGIN))
        .flex();
    match dock.side {
        DockSide::Centre => row.justify_center(),
        DockSide::Left => row.justify_start(),
        DockSide::Right => row.justify_end(),
    }
    .child(
        div()
            .debug_selector(|| "bottom-card".into())
            .w(px(dock.w))
            .child(card),
    )
}

/// How someone stands with the player: five small marks, as many filled as
/// they are warm towards them, and the words for it.
fn standing_row(standing: &world_projection::Standing) -> Div {
    let filled = (standing.level.clamp(-2, 2) + 3) as usize;
    let tone = match standing.level {
        1.. => tokens::SUCCESS,
        0 => tokens::ACCENT,
        _ => tokens::DANGER,
    };
    let mut marks = div().flex().items_center().gap(px(3.0));
    for index in 0..5 {
        marks = marks.child(
            div()
                .w(px(6.0))
                .h(px(6.0))
                .rounded_full()
                .bg(color(if index < filled { tone } else { tokens::BORDER })),
        );
    }
    div().flex().items_center().gap_2().child(marks).child(
        div()
            .text_xs()
            .text_color(color(tokens::TEXT_SECONDARY))
            .child(standing.words.clone()),
    )
}

/// A person thinking: one to three dots, turning over while they do.
fn thinking_dots(since: Option<Instant>) -> String {
    let beat = since.map_or(0, |since| since.elapsed().as_millis() / 400) % 3;
    ".".repeat(beat as usize + 1)
}

/// Saves the screen area `bounds` covers as a picture in the player's
/// Pictures, in a World Machine folder, named after the World and the
/// time. Whether it was saved.
fn save_photo(bounds: gpui::Bounds<gpui::Pixels>, title: &str) -> bool {
    save_picture(bounds, title, true)
}

/// Saves the screen area `bounds` covers to Pictures/World Machine as
/// `<name>.png`, with the time after the name when `stamped` (a photo) or
/// only when a picture of that name is already there (a postcard, which is
/// named after its day).
pub(crate) fn save_picture(bounds: gpui::Bounds<gpui::Pixels>, title: &str, stamped: bool) -> bool {
    let Some(home) = std::env::var_os("HOME") else {
        return false;
    };
    let folder = std::path::Path::new(&home)
        .join("Pictures")
        .join("World Machine");
    if std::fs::create_dir_all(&folder).is_err() {
        return false;
    }
    let name = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let stamp = chrono::Local::now().format("%Y-%m-%d at %H.%M.%S");
    let plain = folder.join(format!("{}.png", name.trim()));
    let path = if stamped || plain.exists() {
        folder.join(format!("{} {stamp}.png", name.trim()))
    } else {
        plain
    };
    let region = format!(
        "{},{},{},{}",
        f32::from(bounds.origin.x).round(),
        f32::from(bounds.origin.y).round(),
        f32::from(bounds.size.width).round(),
        f32::from(bounds.size.height).round()
    );
    std::process::Command::new("/usr/sbin/screencapture")
        .arg("-x")
        .arg("-R")
        .arg(region)
        .arg(&path)
        .status()
        .is_ok_and(|status| status.success())
        && path.is_file()
}

/// The paper of a postcard around a scene `width` by `height`: an even
/// border, and at the foot a band with the caption, who said it, and the
/// World and the day.
pub(crate) fn postcard_paper(card: &crate::postcard::Postcard, width: f32, height: f32) -> Div {
    let layout = crate::postcard::postcard_layout(width, height);
    let paper = gpui::rgb(crate::postcard::PAPER);
    let ink = gpui::rgb(crate::postcard::INK);
    let soft = gpui::rgb(crate::postcard::INK_SOFT);
    let edge = |div: Div| div.absolute().bg(paper);
    div()
        .absolute()
        .top_0()
        .left_0()
        .w(px(width))
        .h(px(height))
        .child(edge(div()).top_0().left_0().right_0().h(px(layout.border)))
        .child(edge(div()).top_0().bottom_0().left_0().w(px(layout.border)))
        .child(
            edge(div())
                .top_0()
                .bottom_0()
                .right_0()
                .w(px(layout.border)),
        )
        // A hairline where the picture meets the paper, as if pasted on.
        .child(
            div()
                .absolute()
                .top(px(layout.border))
                .left(px(layout.border))
                .w(px(layout.scene_width))
                .h(px(layout.scene_height))
                .border_1()
                .border_color(gpui::rgb(crate::postcard::INK_SOFT).opacity(0.35)),
        )
        .child(
            edge(div())
                .bottom_0()
                .left_0()
                .right_0()
                .h(px(layout.band))
                .px(px(layout.border * 1.5))
                .flex()
                .flex_col()
                .justify_center()
                .gap_1()
                .child(
                    div()
                        .text_color(ink)
                        .text_lg()
                        .italic()
                        .line_clamp(2)
                        .child(card.printed_caption()),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap_4()
                        .text_sm()
                        .text_color(soft)
                        .child(card.signature())
                        .child(div().flex_shrink_0().child(ui::t("World Machine"))),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::super::drawer::tests::a_full_drawer;
    use super::*;
    use crate::ui::accessible;

    #[test]
    fn a_world_nobody_can_see_asks_for_no_frames_and_keeps_no_clock() {
        for active in [false, true] {
            for still in [false, true] {
                assert_eq!(pace(false, active, still), Pace::Asleep);
            }
        }
        assert_eq!(pace(true, false, false), Pace::Waiting);
        assert_eq!(pace(true, false, true), Pace::Waiting);
        assert_eq!(pace(true, true, true), Pace::Ticking);
        assert_eq!(pace(true, true, false), Pace::EveryFrame);
    }

    /// Through GPUI's test window: covered, the World's clock stops;
    /// shown again, it is drawn at once and the clock starts again.
    #[gpui::test]
    fn a_covered_world_stops_its_clock_and_draws_when_shown(cx: &mut gpui::TestAppContext) {
        use gpui::{VisualTestContext, WindowVisibility};
        let window = cx.add_window(|_, _| ProjectionView::new(ProjectionSnapshot::default()));
        let view = window.root(cx).expect("the World");
        let cx = &mut VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        let ticking =
            |cx: &mut VisualTestContext| view.read_with(cx, |view, _| view.looking.clock.is_some());
        assert!(ticking(cx), "a World in view keeps its clock");

        cx.simulate_visibility_change(WindowVisibility::Hidden);
        cx.run_until_parked();
        assert!(!ticking(cx), "covered, the clock stops");
        cx.executor().advance_clock(FRAME * 40);
        cx.run_until_parked();
        assert!(!ticking(cx), "and nothing starts it while covered");

        cx.simulate_visibility_change(WindowVisibility::Visible);
        cx.run_until_parked();
        assert!(ticking(cx), "shown, it is drawn and the clock starts again");
    }

    fn someone() -> SelectionId {
        SelectionId::from_stable_key("entity-7").expect("an entity key")
    }

    /// As a World opens on a question, the card waits while the first
    /// thing said (a hello) is heard; a card that asks nothing does not.
    #[test]
    fn a_question_waits_for_the_first_thing_said_as_a_world_opens() {
        use world_projection::{Question, TimelineItem, Voice};
        let moment = SelectionId::from_stable_key("event-3").expect("an event key");
        let command = |question: Option<Question>| ProjectionCommand {
            id: "ask".into(),
            title: "Timber for the pier".into(),
            detail: String::new(),
            effects: Vec::new(),
            scenery: None,
            asker: Some(someone()),
            moves: Vec::new(),
            question,
            unavailable: None,
            hand: None,
            preview: None,
        };
        let mut snapshot = ProjectionSnapshot {
            world_time: 20,
            voices: vec![Voice {
                moment,
                speaker: someone(),
                line: "Oh, a new face! I'm Leo. Welcome to the harbour.".into(),
            }],
            commands: vec![command(Some(Question {
                id: "pier".into(),
                prompt: "Will you find timber for the pier?".into(),
            }))],
            ..ProjectionSnapshot::default()
        };
        snapshot.timeline.items.push(TimelineItem {
            id: moment,
            world_time: 20,
            title: "Leo came over to say hello".into(),
            subtitle: String::new(),
            caused_by: Vec::new(),
            routine: false,
        });
        assert_eq!(question_waits(&snapshot), LINE_SECONDS);
        snapshot.commands = vec![command(None)];
        assert_eq!(question_waits(&snapshot), 0.0, "only a question waits");
    }

    /// The answers on a question's card are buttons in a set, named by
    /// what they say; one that cannot be chosen now says why.
    #[test]
    fn answers_and_verbs_are_named_controls() {
        let (role, node) = accessible(&answer_button(1, 3, "Stay home".into(), true, None));
        assert_eq!(role, Some(Role::Button));
        assert_eq!(node.label(), Some("Stay home"));
        assert_eq!(node.position_in_set(), Some(2));
        assert_eq!(node.size_of_set(), Some(3));
        let (role, node) = accessible(&answer_button(
            0,
            3,
            "Lend the boat".into(),
            false,
            Some("The boat is out"),
        ));
        assert_eq!(role, Some(Role::Button));
        assert_eq!(node.description(), Some("Not now: The boat is out"));
        let (role, node) = accessible(&verb_tab("Plant", true));
        assert_eq!(role, Some(Role::Tab));
        assert_eq!(node.label(), Some("Plant"));
        assert_eq!(node.is_selected(), Some(true));
        let (_, other) = accessible(&verb_tab("Give", false));
        assert_eq!(other.is_selected(), Some(false));
    }

    /// A place the tests can find a control by, as `ui::named` marks it.
    fn named(name: &str) -> &'static str {
        Box::leak(format!("named: {name}").into_boxed_str())
    }

    fn tip(name: &str) -> &'static str {
        Box::leak(format!("tip: {name}").into_boxed_str())
    }

    /// Through GPUI's test window: every icon on the World (the hands,
    /// the strip, the drawer, the zoom control, a card's arrows) shows its
    /// name when the pointer rests on it, after a short delay and not at
    /// once, and a screen reader hears the same name.
    #[gpui::test]
    fn icons_show_their_names_when_the_pointer_rests_on_them(cx: &mut gpui::TestAppContext) {
        use gpui::{Modifiers, VisualTestContext};
        let mut snapshot = a_full_drawer();
        let choice = |id: &str| ProjectionCommand {
            hand: None,
            ..deed(id, "Mend")
        };
        snapshot.commands = vec![
            deed("build", "Build"),
            deed("give", "Give"),
            choice("pier"),
            choice("nets"),
        ];
        let window = cx.add_window(move |_, _| {
            let mut view =
                ProjectionView::controlled(Still(snapshot.clone())).with_strip(|_, _| {});
            view.looking.opening = None;
            view
        });
        let cx = &mut VisualTestContext::from_window(window.into(), cx);
        cx.simulate_resize(gpui::size(px(1100.0), px(800.0)));
        cx.run_until_parked();
        for name in [
            "Make something (H)",
            "Show as a strip along the edge of the screen (⌥⌘S)",
            "The drawer: story, letters, keepsakes and the book (⌘I)",
            "Zoom in (+)",
            "Zoom out (−)",
            "Previous card (↑)",
            "Next card (↓)",
        ] {
            let bounds = cx
                .debug_bounds(named(name))
                .unwrap_or_else(|| panic!("{name} is drawn"));
            cx.simulate_mouse_move(bounds.center(), None, Modifiers::none());
            cx.run_until_parked();
            assert!(cx.debug_bounds(tip(name)).is_none(), "{name}: not at once");
            cx.executor()
                .advance_clock(ui::TIP_DELAY + Duration::from_millis(50));
            cx.run_until_parked();
            assert!(
                cx.debug_bounds(tip(name)).is_some(),
                "{name}: shown on hover"
            );
            // Away from it, it goes.
            cx.simulate_mouse_move(gpui::point(px(540.0), px(400.0)), None, Modifiers::none());
            cx.run_until_parked();
        }
    }

    /// After dusk the paper over the scene dims, so no card glares over a
    /// night sky; by day it is the usual white, and the dark appearance
    /// keeps its own.
    #[test]
    fn cards_over_a_night_scene_are_a_dimmer_paper() {
        let bright = |colour: gpui::Rgba| colour.r + colour.g + colour.b;
        let day = paper_at(scene::Daylight::Day, false);
        let dusk = paper_at(scene::Daylight::Dusk, false);
        let night = paper_at(scene::Daylight::Night, false);
        assert!(bright(night) < bright(dusk) && bright(dusk) < bright(day));
        assert!(bright(night) > 2.2, "still paper, dark text still reads");
        assert_eq!(
            paper_at(scene::Daylight::Night, true),
            paper_at(scene::Daylight::Day, true)
        );
    }

    /// A thing of the hands is drawn as what it is, by the name its deed
    /// gives it, and as its verb makes things when nothing matches.
    #[test]
    fn a_thing_of_the_hands_is_drawn_as_itself() {
        assert_eq!(
            hand_shape("tiny-society.hand.build.bench.12", "Build"),
            MarkShape::Bench
        );
        assert_eq!(
            hand_shape("tiny-society.hand.build.rowboat.3", "Build"),
            MarkShape::Boat
        );
        assert_eq!(
            hand_shape("tiny-society.hand.plant.herbs.3", "Plant"),
            MarkShape::Garden
        );
        assert_eq!(
            hand_shape("tiny-society.hand.plant.apple_tree.3", "Plant"),
            MarkShape::Tree
        );
        assert_eq!(
            hand_shape("hands.decorate.bunting.1", "Decorate"),
            MarkShape::Bunting
        );
    }

    /// A gauge says its reading, a number where the World counts one.
    #[test]
    fn a_gauge_shows_its_number() {
        let gauge = world_projection::Gauge {
            id: "money".into(),
            label: "Money in town".into(),
            value: 0.4,
            reading: "11,485".into(),
            tone: world_projection::Tone::Good,
        };
        let (role, node) = accessible(&hud_gauge(&gauge, 0.4, None));
        assert_eq!(role, Some(Role::Meter));
        assert_eq!(node.label(), Some("Money in town: 11,485"));
    }

    /// Every name an icon shows on hover is in each of the app's catalogs.
    #[test]
    fn every_icon_name_is_translated() {
        for catalog in crate::i18n::app_catalogs() {
            for name in [
                "Make something (H)",
                "Show as a strip along the edge of the screen (⌥⌘S)",
                "The drawer: story, letters, keepsakes and the book (⌘I)",
                "Zoom",
                "Zoom in (+)",
                "Zoom out (−)",
                "Previous card (↑)",
                "Next card (↓)",
                "Close the drawer",
                "Close",
                "Close the strip",
                "Put the letter away",
                "Dismiss this hint",
                "Turn the card over: what this would change (Space)",
                "Turn the card back (Space)",
            ] {
                assert!(catalog.exact(name).is_some(), "no translation for {name:?}");
            }
        }
    }

    /// A World whose snapshot never changes, for the window tests.
    struct Still(ProjectionSnapshot);

    impl crate::ProjectionController for Still {
        fn snapshot(&self) -> ProjectionSnapshot {
            self.0.clone()
        }

        fn handle(&mut self, _: ProjectionIntent) -> Result<ProjectionSnapshot, String> {
            Ok(self.0.clone())
        }
    }

    /// A deed for the hands: something to `verb`, here and now.
    fn deed(id: &str, verb: &str) -> ProjectionCommand {
        ProjectionCommand {
            id: id.into(),
            title: format!("{verb} something"),
            detail: String::new(),
            effects: Vec::new(),
            scenery: None,
            asker: None,
            moves: Vec::new(),
            question: None,
            unavailable: None,
            hand: Some(world_projection::Hand {
                verb: verb.into(),
                thing: "Bench".into(),
                cost: None,
                at: Some(someone()),
            }),
            preview: None,
        }
    }

    /// The icon-like controls are named buttons, and the tooltip itself is
    /// a tooltip to a screen reader.
    #[test]
    fn icon_controls_and_their_tips_have_roles() {
        let (role, node) = accessible(&zoom_button("zoom-in", "+", "Zoom in (+)", false));
        assert_eq!(role, Some(Role::Button));
        assert_eq!(node.label(), Some("Zoom in (+)"));
        let (role, node) = accessible(&ui::named(div().id("x"), "Close the drawer"));
        assert_eq!(role, None, "naming adds no role of its own");
        assert_eq!(node.label(), Some("Close the drawer"));
        let tip = gpui::div()
            .id("tip")
            .role(Role::Tooltip)
            .aria_label("Zoom in (+)");
        assert_eq!(accessible(&tip).0, Some(Role::Tooltip));
    }

    /// A pointer is a status a screen reader hears as it arrives, saying
    /// what it points at, with a named button that puts it away.
    #[test]
    fn a_pointer_is_a_status_with_a_way_to_dismiss_it() {
        for pointer in Pointer::ORDER {
            for caret in [Caret::Up, Caret::Down, Caret::Left] {
                let (role, node) = accessible(&pointer_hint(pointer, caret, |_, _, _| {}));
                assert_eq!(role, Some(Role::Status));
                assert_eq!(node.label(), Some(pointer.words()));
            }
        }
    }

    /// The v0.24 bar: the question card never covers whoever asks it. On
    /// the day-1,082 harbour at 1100 by 900 and 1440 by 900, for everyone
    /// out on the quay at noon, looked at from anywhere along the place,
    /// the card docked for them as asker covers no part of their figure.
    #[test]
    fn the_card_never_covers_its_asker() {
        use crate::diorama::{self, Camera, Glows};
        let snapshot = crate::diorama::tests::harbour_1082();
        for (width, window_h) in [(1100.0_f32, 900.0_f32), (1440.0, 900.0)] {
            let height = window_h - 52.0;
            let stage = diorama::stage_at(&snapshot, width, height, diorama::Clock::at(12));
            let lives = diorama::living(
                &stage,
                &snapshot,
                0.0,
                crate::scene::Daylight::Day,
                &Default::default(),
                None,
            );
            let mut askers = 0;
            for pan in 0..=20 {
                let camera =
                    Camera::around(&stage, 1.0, stage.width * pan as f32 / 20.0, height / 2.0);
                let frame = diorama::frame(
                    &snapshot,
                    &stage,
                    &lives,
                    camera,
                    0.0,
                    crate::scene::Daylight::Day,
                    &Glows::new(),
                    1.0,
                );
                for person in &frame.people {
                    if person.x < 0.0 || person.x > width {
                        continue;
                    }
                    askers += 1;
                    let dock = card_dock(width, Some(person.x));
                    let (left, right) = dock.span(width);
                    let half = person.height * 0.4;
                    assert!(
                        person.x + half <= left || person.x - half >= right,
                        "{width}: someone at {} covered by a card at {left}..{right}",
                        person.x
                    );
                    assert!(dock.w >= 400.0, "{width}: a card {} wide", dock.w);
                }
            }
            assert!(askers > 50, "{askers} askers seen");
        }
    }

    /// Where each pointer goes keeps clear of the card and of the speech
    /// over anyone standing in the middle of the ground, on a usual window.
    #[test]
    fn pointers_keep_off_the_card() {
        let (width, height) = (1100.0, 748.0);
        // The card: centred at the bottom, as tall as a question with
        // three answers.
        let card = Area {
            x: width / 2.0 - CARD_WIDTH / 2.0,
            y: height - 20.0 - 280.0,
            w: CARD_WIDTH,
            h: 300.0,
        };
        for pointer in Pointer::ORDER {
            assert!(
                !hint_area(pointer, width, height).overlaps(&card),
                "{pointer:?} would cover the card"
            );
        }
        let apart = Area {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        };
        let touching = Area { x: 5.0, ..apart };
        let beside = Area { x: 10.0, ..apart };
        assert!(apart.overlaps(&touching));
        assert!(!apart.overlaps(&beside));
    }

    struct Arriving(bool);

    impl Render for Arriving {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(ui::spring_in(
                div().debug_selector(|| "arriving".into()).size(px(40.0)),
                "arriving",
                self.0,
            ))
        }
    }

    /// A card springs up from a little below its place, and with Reduce
    /// Motion on (GPUI's setting, and ours) it is in its place from the
    /// first frame.
    #[gpui::test]
    fn a_card_springs_into_place_unless_motion_is_reduced(cx: &mut gpui::TestAppContext) {
        use gpui::VisualTestContext;
        let top = |still: bool, reduce: bool, cx: &mut gpui::TestAppContext| {
            cx.update(|cx| cx.set_reduce_motion(reduce));
            let window = cx.add_window(move |_, _| Arriving(still));
            let cx = &mut VisualTestContext::from_window(window.into(), cx);
            cx.run_until_parked();
            f32::from(cx.debug_bounds("arriving").expect("drawn").origin.y)
        };
        let moving = top(false, false, cx);
        assert!(moving > 10.0, "starts below its place: {moving}");
        assert_eq!(top(false, true, cx), 0.0, "GPUI's Reduce Motion");
        assert_eq!(top(true, false, cx), 0.0, "our own");
    }

    /// A page of speech ends where a sentence ends, never in the middle
    /// of one: whole sentences share a page while they fit two lines, and
    /// a sentence too long for two has a page of its own, taller. Nothing
    /// is lost, in English or in Chinese.
    #[test]
    fn a_page_of_speech_never_stops_in_the_middle_of_a_sentence() {
        let lines = [
            "Here we are again. Last time: Evan took a week's work on the mainland. A week's carpentry on the mainland. Should I go?",
            "The boats came in late again, and the market will be quiet tomorrow because the whole of the north quay is still under repair. Come by!",
            "Is it 3.5 miles to the lighthouse? \"Nearer four,\" she said. We walked anyway.",
            "今天港口的船都回来了，大家都很高兴。晚上我们在码头一起吃饭吧，你也来吗？好！",
        ];
        for line in lines {
            let wanted = sentences(line);
            assert!(wanted.len() >= 2, "{wanted:?}");
            let pages = speech_pages(line);
            let flat = |text: &str| text.split_whitespace().collect::<String>();
            // Every page is whole sentences: the sentences, in order, make
            // up the pages exactly.
            let mut left = wanted.iter();
            for page in &pages {
                let mut made = String::new();
                while flat(&made) != flat(page) {
                    let sentence = left
                        .next()
                        .unwrap_or_else(|| panic!("{page:?} stops mid-sentence in {pages:?}"));
                    made.push_str(sentence);
                    assert!(
                        flat(page).starts_with(&flat(&made)),
                        "{page:?} stops mid-sentence ({made:?}) in {pages:?}"
                    );
                }
                // Two lines, or one long sentence as tall as it needs.
                let rows = page.lines().count();
                assert!(
                    rows <= 2 || sentences(page).len() == 1 && rows <= PAGE_MOST,
                    "{page:?}"
                );
            }
            assert!(left.next().is_none(), "nothing is lost: {pages:?}");
        }
        assert_eq!(
            sentences("Mara said so. \"Really?\" Yes."),
            vec!["Mara said so.", "\"Really?\"", "Yes."]
        );
    }

    /// The v0.25 bar for speech: a bubble never covers the interface (the
    /// gauges and handles, the zoom control, the card) nor another bubble,
    /// and stays on the stage, wherever its speaker stands: beside the
    /// zoom control, under the handles, over the card, at either edge.
    #[test]
    fn speech_bubbles_never_cover_the_interface_or_each_other() {
        let (width, height) = (1100.0, 760.0);
        let interface = [
            Area {
                x: 0.0,
                y: 0.0,
                w: width,
                h: HUD_ROOM,
            },
            Area {
                x: 16.0,
                y: zoom_top(height),
                w: ZOOM_SIZE.0,
                h: ZOOM_SIZE.1,
            },
            Area {
                x: 270.0,
                y: height - CARD_MARGIN - CARD_ROOM,
                w: CARD_WIDTH,
                h: CARD_ROOM + CARD_MARGIN,
            },
        ];
        let long = "The boats came in late again, and the market will be quiet. Come by!";
        let pages = speech_pages(long);
        let (w, h) = bubble_size(&pages[0]);
        let check = |heads: &[(f32, f32)]| {
            let bubbles = heads
                .iter()
                .map(|(x, y)| (*x, *y, w, h))
                .collect::<Vec<_>>();
            let placed = place_bubbles(&bubbles, &interface, (width, height));
            assert_eq!(placed.len(), bubbles.len());
            for (index, bubble) in placed.iter().enumerate() {
                let area = bubble.whole();
                assert!(
                    area.x >= 0.0
                        && area.y >= 0.0
                        && area.x + area.w <= width
                        && area.y + area.h <= height,
                    "{heads:?}: bubble {index} leaves the stage: {area:?}"
                );
                for mine in bubble.parts() {
                    for part in &interface {
                        assert!(
                            !mine.overlaps(part),
                            "{heads:?}: bubble {index} {mine:?} covers {part:?}"
                        );
                    }
                    for (other, them) in placed.iter().enumerate().take(index) {
                        for theirs in them.parts() {
                            assert!(
                                !mine.overlaps(&theirs),
                                "{heads:?}: bubbles {other} and {index} cover each other"
                            );
                        }
                    }
                }
                // Its tail points down at its speaker's head, from over it.
                let (x, y) = heads[index];
                assert_eq!((bubble.head, bubble.foot), (x, y));
                assert!(bubble.area.y + bubble.area.h < y);
            }
        };
        // A speaker anywhere along the stage: by the zoom control, at
        // either edge, just over the card, under the handles.
        for head in [
            (70.0, zoom_top(height) + 90.0),
            (60.0, zoom_top(height) + 20.0),
            (8.0, 220.0),
            (width - 8.0, 400.0),
            (550.0, height - CARD_MARGIN - CARD_ROOM - 4.0),
            (700.0, HUD_ROOM + h + 30.0),
        ] {
            check(&[head]);
        }
        // Someone answering the player, beside the card that asks them,
        // while someone else speaks; three speaking at once, side by side.
        check(&[(500.0, 420.0), (560.0, 430.0)]);
        check(&[(420.0, 420.0), (480.0, 420.0), (540.0, 420.0)]);
        // With room, a bubble is right over its speaker.
        let alone = place_bubbles(&[(550.0, 400.0, w, h)], &interface, (width, height));
        assert_eq!(alone[0].area.x, 550.0 - w / 2.0);
        assert_eq!(alone[0].whole().y + alone[0].whole().h, 400.0);
    }

    /// Chinese is cut by width, but a closing mark (，。！？」) never
    /// starts a line, and an opening one (「) never ends one: the
    /// character before goes down with it, and every line still fits.
    #[test]
    fn chinese_lines_never_start_with_closing_punctuation() {
        // Eighteen characters fill a bubble line; each mark here falls
        // just where the line would otherwise break.
        let lines = [
            "今天港口的船都回来了大家都很高兴我们，晚上在码头一起吃饭吧你也来吗？",
            "今天港口的船都回来了大家都很高兴我们。」他笑着说今天港口的船都回来了。",
            "今天港口的船都回来了大家都很高兴我「们」今天港口的船都回来了大家都很！",
        ];
        for line in lines {
            let pages = speech_pages(line);
            let rows = pages
                .iter()
                .flat_map(|page| page.lines())
                .collect::<Vec<_>>();
            assert!(rows.len() >= 2, "{rows:?}");
            for row in &rows {
                let first = row.chars().next().expect("a row is never empty");
                assert!(!never_starts_a_line(first), "{row:?} starts with {first}");
                assert!(!row.ends_with('「'), "{row:?} ends opening a quote");
                assert!(row.chars().count() <= 18, "{row:?} is too wide");
            }
            assert_eq!(rows.concat(), line, "nothing is lost");
        }
    }
}
