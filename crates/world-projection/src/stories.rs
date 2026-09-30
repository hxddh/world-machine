//! Stories a World tells about itself, asked for rather than sent every
//! time: a legend (the life of one person, place or work, line by line,
//! each naming its cause), a moment (a key beat told in three panels) and
//! an almanac (a year in review).
//!
//! Everything here is read from recorded Events and never written back:
//! the words are the World's, and what a line says caused it is a cause
//! the World recorded. What counts as a cause, a moment or a year is the
//! Pack's to say; this module only holds the shapes, and the few readings
//! of a history that any World shares.

use crate::{Mood, SelectionId};
use std::collections::{BTreeSet, VecDeque};
use world_core::{EntityId, Event, EventId, World};

/// The life of one person, place or work, as the World tells it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Legend {
    pub subject: SelectionId,
    /// Whose life it is, in the World's words ("Mara, the baker").
    pub title: String,
    /// Oldest first.
    pub lines: Vec<LegendLine>,
}

/// One line of a legend.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LegendLine {
    /// The World's day it happened on, counted from 1.
    pub day: u32,
    /// What happened, in the World's words.
    pub text: String,
    /// What brought it about, as a player would say it: "because you said
    /// “Throw a party”", "after the great storm", "at the Harvest Home".
    /// Never an id or an engine word.
    pub because: Option<String>,
    /// The Event the line tells.
    pub event: Option<EventId>,
    /// The recorded Event `because` names, whenever there is a `because`.
    pub cause: Option<EventId>,
}

/// What kind of beat a moment is, so the painter can pick its poses.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MomentKind {
    Wedding,
    Birth,
    ComingOfAge,
    Farewell,
    Death,
    Storm,
    WorkOpened,
    Festival,
    #[default]
    Other,
}

impl MomentKind {
    pub const ALL: [MomentKind; 9] = [
        MomentKind::Wedding,
        MomentKind::Birth,
        MomentKind::ComingOfAge,
        MomentKind::Farewell,
        MomentKind::Death,
        MomentKind::Storm,
        MomentKind::WorkOpened,
        MomentKind::Festival,
        MomentKind::Other,
    ];

    pub fn id(self) -> &'static str {
        match self {
            MomentKind::Wedding => "wedding",
            MomentKind::Birth => "birth",
            MomentKind::ComingOfAge => "coming_of_age",
            MomentKind::Farewell => "farewell",
            MomentKind::Death => "death",
            MomentKind::Storm => "storm",
            MomentKind::WorkOpened => "work_opened",
            MomentKind::Festival => "festival",
            MomentKind::Other => "other",
        }
    }

    /// The kind with this id; an id this build does not know is `Other`.
    pub fn from_id(id: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|kind| kind.id() == id)
            .unwrap_or(MomentKind::Other)
    }
}

/// Which of a moment's three panels.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PanelBeat {
    Before,
    #[default]
    Moment,
    After,
}

impl PanelBeat {
    pub const ALL: [PanelBeat; 3] = [PanelBeat::Before, PanelBeat::Moment, PanelBeat::After];

    pub fn id(self) -> &'static str {
        match self {
            PanelBeat::Before => "before",
            PanelBeat::Moment => "moment",
            PanelBeat::After => "after",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|beat| beat.id() == id)
    }
}

/// Something a moment's panel shows besides its people and place, so the
/// picture tells the event its caption names: the ferry that takes someone
/// away, the bunting at a wedding, the cradle at a birth. The app draws
/// each in the look of the World's setting (a ferry by the harbour, a
/// shuttle on Mars); a prop it has no drawing for is left out.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Prop {
    /// A boat at the quay that takes someone away or brings them.
    Ferry,
    /// A lander on its pad, for leaving or coming to a colony.
    Shuttle,
    /// A bus at the stop, for leaving or coming to a town.
    Bus,
    /// A sled on the ice, for leaving or coming over the ice.
    Sled,
    /// One bag, packed.
    Suitcase,
    /// A string of little flags overhead.
    Bunting,
    /// Flowers carried in the hand.
    Bouquet,
    /// A cradle, or a baby wrapped up.
    Cradle,
    /// A lamp lit in a window.
    Lamp,
    /// A wreath of flowers laid down.
    Wreath,
    /// Scaffolding and a ladder round what is going up.
    Scaffold,
    /// A ribbon across something new, to be cut.
    Ribbon,
    /// Rain slanting down and a dark sky.
    Rain,
    /// The tools of a trade.
    Tools,
    /// A long table laid for a party.
    Table,
    /// A bench to sit on.
    Bench,
}

impl Prop {
    pub const ALL: [Prop; 16] = [
        Prop::Ferry,
        Prop::Shuttle,
        Prop::Bus,
        Prop::Sled,
        Prop::Suitcase,
        Prop::Bunting,
        Prop::Bouquet,
        Prop::Cradle,
        Prop::Lamp,
        Prop::Wreath,
        Prop::Scaffold,
        Prop::Ribbon,
        Prop::Rain,
        Prop::Tools,
        Prop::Table,
        Prop::Bench,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Prop::Ferry => "ferry",
            Prop::Shuttle => "shuttle",
            Prop::Bus => "bus",
            Prop::Sled => "sled",
            Prop::Suitcase => "suitcase",
            Prop::Bunting => "bunting",
            Prop::Bouquet => "bouquet",
            Prop::Cradle => "cradle",
            Prop::Lamp => "lamp",
            Prop::Wreath => "wreath",
            Prop::Scaffold => "scaffold",
            Prop::Ribbon => "ribbon",
            Prop::Rain => "rain",
            Prop::Tools => "tools",
            Prop::Table => "table",
            Prop::Bench => "bench",
        }
    }

    /// The prop with this id; one this build does not know is none.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|prop| prop.id() == id)
    }
}

/// One panel of a moment: who is in it, where, how they feel, what else
/// it shows, and the World's caption beneath.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Panel {
    pub caption: String,
    /// Who is drawn, the one the panel is about first.
    pub cast: Vec<SelectionId>,
    /// Where it happens, drawn from the scene's own art.
    pub place: Option<SelectionId>,
    pub mood: Option<Mood>,
    pub beat: PanelBeat,
    /// What else the panel shows, so it shows its event: a ferry at a
    /// farewell, bunting at a wedding. Empty for a panel of people alone.
    pub props: Vec<Prop>,
}

/// A key beat of a World's history told as three panels: before, the
/// moment itself, and after.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Moment {
    /// Stable for as long as the history is: the same history always gives
    /// the same moments, with the same ids.
    pub id: String,
    /// The World's day it happened on, counted from 1.
    pub day: u32,
    pub kind: MomentKind,
    pub title: String,
    pub panels: [Panel; 3],
    /// The recorded Event the moment shows.
    pub event: Option<EventId>,
}

impl Moment {
    /// Everyone drawn in any of its panels, each once, in the order they
    /// first appear.
    pub fn cast(&self) -> Vec<SelectionId> {
        let mut seen = BTreeSet::new();
        self.panels
            .iter()
            .flat_map(|panel| panel.cast.iter().copied())
            .filter(|who| seen.insert(*who))
            .collect()
    }
}

/// Someone or something an almanac names, and who or what they are, so
/// the page can be drawn with faces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Named {
    pub name: String,
    pub who: SelectionId,
}

/// A year in review, written at New Year: who came and left, who was born
/// and died, what was built, and the year's best moment.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Almanac {
    /// The year it looks back on, counted from 1.
    pub year: u32,
    /// The World's words for the page's heading ("The harbour's third year").
    pub title: String,
    pub arrived: Vec<String>,
    pub left: Vec<String>,
    pub born: Vec<String>,
    pub died: Vec<String>,
    pub built: Vec<String>,
    /// The id of the year's best moment.
    pub best: Option<String>,
    /// Everyone and everything the page names, with what each is, in the
    /// order the page names them.
    pub cast: Vec<Named>,
}

impl Almanac {
    /// Whether the year left nothing to tell.
    pub fn is_empty(&self) -> bool {
        self.arrived.is_empty()
            && self.left.is_empty()
            && self.born.is_empty()
            && self.died.is_empty()
            && self.built.is_empty()
            && self.best.is_none()
    }
}

/// What a player can ask a World to tell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoryRequest {
    /// The life of a person, place or work.
    Legend(SelectionId),
    /// One moment, by its id, as the book keeps it.
    Moment(String),
    /// The almanac of one year, counted from 1.
    Almanac(u32),
}

/// What a World tells when asked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoryPage {
    Legend(Legend),
    Moment(Moment),
    Almanac(Almanac),
}

/// The most moments a snapshot carries: the latest. The book keeps them
/// all, and any one can be asked for.
pub const MOST_MOMENTS_IN_SNAPSHOT: usize = 3;

/// The latest of `moments` a snapshot carries, oldest first.
pub fn latest_moments(moments: &[Moment]) -> Vec<Moment> {
    moments[moments.len().saturating_sub(MOST_MOMENTS_IN_SNAPSHOT)..].to_vec()
}

/// No more than one moment a day: of the moments falling on one day, the
/// one `rank` puts first (lowest), the earlier on a tie. Oldest first.
pub fn one_a_day(mut moments: Vec<Moment>, rank: impl Fn(&Moment) -> u32) -> Vec<Moment> {
    moments.sort_by_key(|moment| (moment.day, rank(moment), moment.event, moment.id.clone()));
    moments.dedup_by_key(|moment| moment.day);
    moments
}

/// The Events of one entity's life, oldest first: every Event that changed
/// it (from the history's index of what each Event touched), and every
/// Event of these `kinds` that names it as who acted or whom it concerned
/// (from the index of Events by kind), each once. Nothing else is read.
pub fn life_events<'a>(world: &'a World, entity: EntityId, kinds: &[&str]) -> Vec<&'a Event> {
    let index = world.history_index();
    let mut ids = index.changes_of(entity).to_vec();
    for kind in kinds {
        ids.extend(index.of_kind(kind).iter().copied().filter(|id| {
            world
                .event(*id)
                .is_some_and(|event| event.actor == Some(entity) || event.targets.contains(&entity))
        }));
    }
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter().filter_map(|id| world.event(id)).collect()
}

/// What caused `event`, in words: its recorded causes, nearest first and
/// then theirs, up to `depth` steps back, until one `phrase` can say. The
/// cause returned is always a recorded Event of this World.
pub fn cause_in_words(
    world: &World,
    event: &Event,
    depth: usize,
    phrase: impl Fn(&Event) -> Option<String>,
) -> Option<(EventId, String)> {
    let mut seen = BTreeSet::from([event.id]);
    let mut queue = event
        .caused_by
        .iter()
        .map(|id| (*id, 1))
        .collect::<VecDeque<_>>();
    while let Some((id, steps)) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        let Some(cause) = world.event(id) else {
            continue;
        };
        if let Some(words) = phrase(cause) {
            return Some((cause.id, words));
        }
        if steps < depth {
            queue.extend(cause.caused_by.iter().map(|id| (*id, steps + 1)));
        }
    }
    None
}

/// The latest Event of one of these `kinds` recorded before `before` for
/// which `fits` holds, found through the index of Events by kind, reading
/// back from `before` rather than from the start.
pub fn latest_before<'a>(
    world: &'a World,
    kinds: &[&str],
    before: EventId,
    fits: impl Fn(&Event) -> bool,
) -> Option<&'a Event> {
    let index = world.history_index();
    kinds
        .iter()
        .filter_map(|kind| {
            let ids = index.of_kind(kind);
            let end = ids.partition_point(|id| *id < before);
            ids[..end]
                .iter()
                .rev()
                .filter_map(|id| world.event(*id))
                .find(|event| fits(event))
        })
        .max_by_key(|event| event.id)
}

/// The World's day a moment falls on, counted from 1, when a day is
/// `day_length` of world time.
pub fn day_of(world_time: u64, day_length: u64) -> u32 {
    let day = world_time.div_ceil(day_length.max(1)).max(1);
    u32::try_from(day).unwrap_or(u32::MAX)
}

/// Whoever asked the question the player has just answered goes over to
/// what the answer put on the scene (the first length of pier, a dome going
/// up), so an answer is seen as well as told. `is_answer` says which Events
/// are the Pack's answers; the asker is the answer's actor. Read from the
/// answer's own Events, for this moment only; nothing is recorded.
pub fn go_to_what_was_answered(
    world: &World,
    items: &mut [crate::CanvasItem],
    is_answer: impl Fn(&Event) -> bool,
) {
    let now = world.world_time();
    let Some(answer) = world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time == now)
        .find(|event| is_answer(event))
    else {
        return;
    };
    let Some(asker) = answer.actor else {
        return;
    };
    // What the answer made, or what came of it at once.
    let made = world
        .events()
        .iter()
        .filter(|event| {
            event.world_time == now
                && (event.id == answer.id || event.caused_by.contains(&answer.id))
        })
        .flat_map(|event| event.changes.iter())
        .filter_map(|change| match change {
            world_core::StateChange::CreateEntity(entity) => Some(SelectionId::Entity(entity.id)),
            _ => None,
        })
        .find(|made| items.iter().any(|item| item.id == *made));
    let Some(made) = made else {
        return;
    };
    if let Some(item) = items
        .iter_mut()
        .find(|item| item.id == SelectionId::Entity(asker))
    {
        item.at = Some(made);
        item.day.clear();
    }
}

/// A line's text with its first letter lowered, to follow a word such as
/// "after": "after the whole harbour mended Sea Finch". A line that starts
/// with a name keeps it as it is.
pub fn lowered(text: &str, names: &[&str]) -> String {
    if names.iter().any(|name| {
        text.starts_with(name)
            && text[name.len()..]
                .chars()
                .next()
                .is_none_or(|next| !next.is_alphanumeric())
    }) {
        return text.to_string();
    }
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_core::{Entity, WorldState};

    fn event(id: u64, kind: &str, day: u64, caused_by: &[u64]) -> Event {
        Event {
            id: EventId::new(id),
            kind: kind.into(),
            world_time: day,
            actor: Some(EntityId::new(1)),
            targets: vec![],
            caused_by: caused_by.iter().copied().map(EventId::new).collect(),
            payload: Default::default(),
            changes: vec![],
        }
    }

    fn world(events: &[Event]) -> World {
        let mut state = WorldState::default();
        state
            .seed_entity(Entity::new(EntityId::new(1), "person"))
            .unwrap();
        World::from_history(state, events).unwrap()
    }

    #[test]
    fn a_cause_is_found_nearest_first_and_is_always_recorded() {
        let history = [
            event(1, "storm", 1, &[]),
            event(2, "answer", 2, &[1]),
            event(3, "fell_out", 3, &[2, 99]),
        ];
        let world = world(&history);
        let said = |event: &Event| (event.kind == "storm").then(|| "after the storm".to_string());
        assert_eq!(
            cause_in_words(&world, &history[2], 3, said),
            Some((EventId::new(1), "after the storm".into()))
        );
        assert_eq!(cause_in_words(&world, &history[2], 1, said), None);
    }

    #[test]
    fn the_latest_before_reads_back_from_the_line() {
        let history = [
            event(1, "asked", 1, &[]),
            event(2, "asked", 2, &[]),
            event(3, "answered", 3, &[]),
            event(4, "asked", 4, &[]),
        ];
        let world = world(&history);
        let found = latest_before(&world, &["asked"], EventId::new(3), |_| true).unwrap();
        assert_eq!(found.id, EventId::new(2));
    }

    #[test]
    fn one_moment_a_day_keeps_the_first_by_rank() {
        let moment = |id: &str, day, kind| Moment {
            id: id.into(),
            day,
            kind,
            ..Moment::default()
        };
        let kept = one_a_day(
            vec![
                moment("a", 2, MomentKind::Festival),
                moment("b", 2, MomentKind::Wedding),
                moment("c", 1, MomentKind::Storm),
            ],
            |moment| u32::from(moment.kind != MomentKind::Wedding),
        );
        assert_eq!(
            kept.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            ["c", "b"]
        );
        assert_eq!(latest_moments(&kept).len(), 2);
    }

    #[test]
    fn a_line_after_a_word_keeps_its_names() {
        assert_eq!(lowered("The harbour met", &["Leo"]), "the harbour met");
        assert_eq!(lowered("Leo met Mara", &["Leo"]), "Leo met Mara");
        assert_eq!(lowered("Leonard met", &["Leo"]), "leonard met");
        assert_eq!(day_of(0, 10), 1);
        assert_eq!(day_of(10, 10), 1);
        assert_eq!(day_of(11, 10), 2);
    }
}
