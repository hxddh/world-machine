//! A World's history retold: legends of its people, places and works, its
//! key beats as three-panel moments, and a year's almanac.
//!
//! This System decides nothing and records nothing. It reads the Events a
//! World has recorded, through the history's indexes, and tells them again
//! in the World's own words, with each line's cause: first the causes the
//! Events recorded (`caused_by`), then what the record itself shows caused
//! it (the question a choice answered, the festival it happened at, the
//! pair's last trouble), always naming an Event that was recorded. A World
//! from before this System existed is told from the Events it already has.
//!
//! It knows the Systems it tells of by their Event kinds (`lives`'
//! situations and life beats, `storylets`' questions, `calendar`'s
//! festivals, `hands`' deeds); a Pack says the rest through [`Teller`].

pub mod kit;

use std::collections::BTreeSet;
use world_core::{EntityId, Event, EventId, StateChange, Value, World};
use world_projection::{
    cause_in_words, day_of, latest_before, life_events, lowered, one_a_day, Almanac, Legend,
    LegendLine, Moment, MomentKind, Mood, Named, Panel, PanelBeat, Prop, SelectionId,
};

/// How a question of the Pack's own storyteller was settled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Answered {
    /// The player chose an answer, offered in these words.
    Said(String),
    /// The player turned it down, in these words.
    Refused(String),
    /// Nobody answered in time.
    Lapsed,
}

/// What a Pack tells this System about its World.
pub trait Teller {
    fn world(&self) -> &World;
    /// How much world time a day is.
    fn day_length(&self) -> u64;
    /// The names lines can begin with, kept as written after "after".
    fn names(&self) -> &[&'static str];
    /// What a subject is called at the head of its legend; `None` for
    /// something this World does not tell the life of.
    fn title(&self, subject: EntityId) -> Option<String>;
    /// The line `event` makes in `subject`'s legend, in the World's words;
    /// `None` when it is no part of their story.
    fn line(&self, subject: EntityId, event: &Event) -> Option<String>;
    /// Kinds of Event that can concern someone without changing them, so a
    /// legend looks them up by kind as well.
    fn naming_kinds(&self) -> &[&'static str];
    /// How one of the Pack's own storyteller's questions was settled, when
    /// `event` settles one.
    fn storylet_answer(&self, event: &Event) -> Option<Answered> {
        let _ = event;
        None
    }
    /// The Pack's own words for a cause ("after the great storm"), tried
    /// before the shared ones.
    fn cause_words(&self, cause: &Event) -> Option<String> {
        let _ = cause;
        None
    }
    /// Kinds of Event that are a storm weathered.
    fn storm_kinds(&self) -> &[&'static str] {
        &[]
    }
    /// Whether someone is one of the World's people, whose legend is a
    /// life rather than a place's history.
    fn is_person(&self, id: EntityId) -> bool;
}

/// A text field of an Event's payload.
pub fn text<'a>(event: &'a Event, key: &str) -> Option<&'a str> {
    match event.payload.get(key) {
        Some(Value::Text(text)) if !text.is_empty() => Some(text),
        _ => None,
    }
}

/// An entity field of an Event's payload.
pub fn entity(event: &Event, key: &str) -> Option<EntityId> {
    match event.payload.get(key) {
        Some(Value::Entity(id)) => Some(*id),
        _ => None,
    }
}

/// `template` with each `{slot}` filled.
pub fn fill(template: &str, slots: &[(&str, &str)]) -> String {
    // Each slot in turn, as replacing them one after another would: a slot
    // not in the text (and none can be once no brace is left) leaves it as
    // it is, so it is not copied.
    let mut text = template.to_string();
    let mut pattern = String::new();
    for (slot, value) in slots {
        if !text.contains('{') {
            break;
        }
        pattern.clear();
        pattern.push('{');
        pattern.push_str(slot);
        pattern.push('}');
        if text.contains(pattern.as_str()) {
            text = text.replace(pattern.as_str(), value);
        }
    }
    text
}

/// The season a moment falls in, lower case ("spring"), for a World whose
/// year is `year_days` days of `day_length`, spring first.
pub fn season_at(world_time: u64, day_length: u64, year_days: u64) -> &'static str {
    let day_of_year = world_time / day_length.max(1) % year_days.max(1);
    let quarter = (year_days / 4).max(1);
    ["spring", "summer", "autumn", "winter"][(day_of_year / quarter).min(3) as usize]
}

/// One of `options`, the same one for the same `seed` every time.
pub fn pick<'a>(seed: u64, options: &[&'a str]) -> &'a str {
    let mixed = seed
        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .rotate_left(29)
        .wrapping_mul(0xbf58_476d_1ce4_e5b9);
    options[(mixed % options.len().max(1) as u64) as usize]
}

/// The kinds of `lives`' life beats, as the lives System records them.
pub const LIFE_BEATS: [&str; 8] = [
    "born",
    "came_of_age",
    "left_home",
    "retired",
    "died",
    "heirloom_passed",
    "memorial_placed",
    "anniversary_kept",
];

/// The kinds of `hands`' deeds: something the player did by hand.
pub const DEEDS: [&str; 5] = [
    "built_by_hand",
    "decorated_by_hand",
    "planted_by_hand",
    "gift_given",
    "invited_out",
];

/// Where a `lives` situation was answered or let lapse.
const SETTLED: [&str; 2] = ["situation_answered", "situation_lapsed"];

/// A day, and a few days, in world time.
fn days(teller: &impl Teller, count: u64) -> u64 {
    teller.day_length().max(1) * count
}

fn name(teller: &impl Teller, id: EntityId) -> String {
    lives::name(teller.world().state(), id)
}

/// The words of the player's answer to a `lives` situation, as it was
/// offered: "Make peace", "Back Leo".
fn situation_answer(teller: &impl Teller, event: &Event) -> Option<String> {
    let label = lives::answer_label(text(event, "kind")?, text(event, "answer")?)?;
    let a = event.actor.map(|a| name(teller, a)).unwrap_or_default();
    let b = event
        .targets
        .iter()
        .find(|b| Some(**b) != event.actor)
        .map(|b| name(teller, *b))
        .unwrap_or_default();
    let label = fill(label, &[("a", &a), ("b", &b)]);
    if label.contains('{') {
        return None;
    }
    Some(label)
}

/// "because you said “Make peace”", or, for an answer whose words named
/// someone the record does not, "because you answered Rosa".
fn answered_words(teller: &impl Teller, event: &Event) -> Option<String> {
    if let Some(words) = situation_answer(teller, event) {
        return Some(you_said(&words));
    }
    lives::answer_label(text(event, "kind")?, text(event, "answer")?)?;
    Some(format!(
        "because you answered {}",
        name(teller, event.actor?)
    ))
}

/// "because you said “Make peace”"
pub fn you_said(words: &str) -> String {
    format!("because you said “{words}”")
}

/// The words for a recorded cause, if it is one a player would name.
pub fn cause_words(teller: &impl Teller, cause: &Event) -> Option<String> {
    if let Some(words) = teller.cause_words(cause) {
        return Some(words);
    }
    let told = || text(cause, "told").map(|told| lowered(told, teller.names()));
    match cause.kind.as_str() {
        "situation_answered" => answered_words(teller, cause),
        "festival_held" => text(cause, "name").map(|name| format!("at {name}")),
        // A day two people spent together, which a change between them
        // came of.
        "lived" if !cause.targets.is_empty() => Some(
            if text(cause, "told").is_some_and(|told| told.ends_with("they had words")) {
                "after they had words".into()
            } else {
                "after a day spent together".into()
            },
        ),
        // A life beat is named by whose it was, in words of its own, so the
        // words stay whole in any language.
        "died" | "came_of_age" | "retired" | "left_home" => {
            let who = name(
                teller,
                entity(cause, "who").or(cause.targets.first().copied())?,
            );
            Some(match cause.kind.as_str() {
                "died" => format!("after {who} died"),
                "came_of_age" => format!("after {who} came of age"),
                "retired" => format!("after {who} retired"),
                _ => format!("after {who} left home"),
            })
        }
        kind if DEEDS.contains(&kind) => told().map(|told| format!("after {told}")),
        _ if text(cause, "storylet").is_some() => match teller.storylet_answer(cause)? {
            Answered::Said(words) => Some(you_said(&words)),
            Answered::Refused(_) | Answered::Lapsed => None,
        },
        _ => None,
    }
}

/// The question an answer settled, and the words of the answer: a `lives`
/// situation's, or one of the Pack's storyteller's.
fn own_answer(teller: &impl Teller, event: &Event) -> Option<(EventId, String)> {
    let world = teller.world();
    if SETTLED.contains(&event.kind.as_str()) {
        let key = text(event, "situation")?;
        let asked = latest_before(world, &["situation_came_up"], event.id, |asked| {
            text(asked, "situation") == Some(key)
        })?;
        // What the player let pass is never given as a reason.
        if event.kind == "situation_lapsed" {
            return None;
        }
        let words = answered_words(teller, event)?;
        return Some((asked.id, words));
    }
    let storylet = text(event, "storylet")?;
    if event.kind == "situation_arose" {
        return None;
    }
    let words = match teller.storylet_answer(event)? {
        Answered::Said(words) => you_said(&words),
        Answered::Refused(_) | Answered::Lapsed => return None,
    };
    let asked = latest_before(world, &["situation_arose"], event.id, |asked| {
        text(asked, "storylet") == Some(storylet)
    })?;
    Some((asked.id, words))
}

/// The two people an Event is between: who acted and whom it concerned,
/// or whom the one who acted was joined to by it.
fn pair(event: &Event) -> Option<(EntityId, EntityId)> {
    let a = event.actor?;
    if let Some(b) = event.targets.iter().find(|b| **b != a) {
        return Some((a, *b));
    }
    event.changes.iter().find_map(|change| match change {
        StateChange::SetComponent {
            entity,
            key,
            value: Value::Entity(b),
        } if *entity == a && *b != a && (key.ends_with("married") || key.ends_with("partner")) => {
            Some((a, *b))
        }
        _ => None,
    })
}

fn involves(event: &Event, who: EntityId) -> bool {
    event.actor == Some(who) || event.targets.contains(&who)
}

/// What last passed between the pair an Event is between: the latest of
/// their troubles or hopes the player answered, within a month, or at any
/// time when they were joined for good.
fn between_pair(teller: &impl Teller, event: &Event) -> Option<(EventId, String)> {
    if SETTLED.contains(&event.kind.as_str()) || text(event, "storylet").is_some() {
        return None;
    }
    let (a, b) = pair(event)?;
    let joined = event.targets.is_empty();
    let since = event.world_time.saturating_sub(days(teller, 30));
    let settled = latest_before(
        teller.world(),
        &["situation_answered"],
        event.id,
        |settled| {
            // Only what was between them as a pair: a quarrel, a courtship,
            // a rough patch, a celebration; not lessons or favours.
            matches!(
                text(settled, "kind"),
                Some("feud" | "rough" | "sweet" | "party")
            ) && involves(settled, a)
                && involves(settled, b)
                && (joined || settled.world_time >= since)
                && answered_words(teller, settled).is_some()
        },
    );
    let settled = settled?;
    Some((settled.id, answered_words(teller, settled)?))
}

/// The festival held earlier on the same day, for what happened among
/// people at it.
fn festival_that_day(teller: &impl Teller, event: &Event) -> Option<(EventId, String)> {
    if !lives::is_life(event) || event.kind == "situation_came_up" {
        return None;
    }
    let length = teller.day_length();
    let day = day_of(event.world_time, length);
    let held = latest_before(teller.world(), &["festival_held"], event.id, |held| {
        day_of(held.world_time, length) == day
    })?;
    Some((held.id, cause_words(teller, held)?))
}

/// Why someone else took up a want: the player turned it down, or left
/// its asker waiting. Only here is what the player let pass a reason,
/// since it is the whole of this one.
fn taken_up_because(teller: &impl Teller, event: &Event) -> Option<(EventId, String)> {
    if event.payload.get("taken_up") != Some(&Value::Bool(true)) {
        return None;
    }
    let cause = teller.world().event(*event.caused_by.first()?)?;
    let words = match teller.storylet_answer(cause)? {
        Answered::Said(words) => you_said(&words),
        Answered::Refused(words) => format!("after you said “{words}”"),
        Answered::Lapsed => format!("after {} was left waiting", name(teller, cause.actor?)),
    };
    Some((cause.id, words))
}

/// What brought `event` about, as a player would say it, and the recorded
/// Event that says so; `None` when the record does not show.
pub fn because(teller: &impl Teller, event: &Event) -> Option<(EventId, String)> {
    taken_up_because(teller, event)
        // What the player said to a pair comes before the day they spent.
        .or_else(|| {
            (event.kind == "bond_changed")
                .then(|| between_pair(teller, event))
                .flatten()
        })
        .or_else(|| cause_in_words(teller.world(), event, 4, |cause| cause_words(teller, cause)))
        .or_else(|| own_answer(teller, event))
        .or_else(|| between_pair(teller, event))
        .or_else(|| festival_that_day(teller, event))
}

/// The most lines a legend keeps: the weightiest, told in day order.
pub const MOST_LEGEND_LINES: usize = 20;
/// The most lines of one sort a legend keeps (friends made, quarrels,
/// lessons), the earliest.
const MOST_OF_A_SORT: usize = 3;

/// Answers to a `lives` situation that turn it down or leave it be: never
/// part of anyone's life story.
const TURNED_DOWN: [&str; 9] = [
    "decline", "not_now", "wait", "space", "another", "wave", "nothing", "leave", "lapse",
];

/// How much a beat weighs in a life, the weightiest first kept.
fn weight(event: &Event, subject: EntityId) -> u32 {
    if creates(event, subject) {
        return 95;
    }
    match event.kind.as_str() {
        "born" | "died" => 100,
        "came_of_age" | "retired" | "left_home" | "year_turned" => 90,
        "heirloom_passed" | "memorial_placed" => 70,
        "bond_changed" => match text(event, "bond") {
            Some("partners") => 85,
            Some("fell_out") => 55,
            Some("made_up") => 50,
            Some("drifted") => 25,
            _ => 45,
        },
        "situation_answered" => match text(event, "kind") {
            Some("visitor" | "leaving" | "sweet") => 85,
            Some("party") => 75,
            Some("rough" | "feud") => 60,
            Some("short" | "learn" | "confide") => 50,
            _ => 40,
        },
        "greeted" => 45,
        "anniversary_kept" | "keepsake_left" => 30,
        "festival_held" => 35,
        kind if DEEDS.contains(&kind) => 45,
        _ if text(event, "storylet").is_some() => 55,
        _ => 40,
    }
}

fn creates(event: &Event, who: EntityId) -> bool {
    event
        .changes
        .iter()
        .any(|change| matches!(change, StateChange::CreateEntity(entity) if entity.id == who))
}

/// Whether a line is the subject's own: their life beat, their arrival,
/// or a line that names them. Other people's news is not.
fn own(teller: &impl Teller, subject: EntityId, event: &Event, text: &str) -> bool {
    if entity(event, "who") == Some(subject) || creates(event, subject) {
        return true;
    }
    let first = lives::first_name(teller.world().state(), subject);
    text.split(|c: char| !c.is_alphanumeric())
        .any(|word| !first.is_empty() && word == first)
}

/// Whether an Event is a question turned down, let lapse or left be.
fn turned_down(teller: &impl Teller, event: &Event) -> bool {
    match event.kind.as_str() {
        "situation_lapsed" | "situation_came_up" | "situation_arose" => true,
        "situation_answered" => {
            text(event, "answer").is_some_and(|answer| TURNED_DOWN.contains(&answer))
        }
        _ if text(event, "storylet").is_some() => matches!(
            teller.storylet_answer(event),
            Some(Answered::Refused(_) | Answered::Lapsed)
        ),
        _ => false,
    }
}

/// Where someone worked, from their first day there: one line a place,
/// however many days they worked it.
fn work_lines(
    teller: &impl Teller,
    subject: EntityId,
    events: &[&Event],
) -> Vec<(u32, String, EventId)> {
    let state = teller.world().state();
    let who = lives::first_name(state, subject);
    let mut seen = BTreeSet::new();
    events
        .iter()
        .filter(|event| {
            event.kind == "lived"
                && event.actor == Some(subject)
                && text(event, "activity") == Some("shift")
        })
        .filter_map(|event| {
            let place = entity(event, "place")?;
            seen.insert(place).then_some(())?;
            let day = day_of(event.world_time, teller.day_length());
            let at = state.entity(place).map(world_projection::entity_title)?;
            Some((70, format!("{who} worked at {at} from day {day}"), event.id))
        })
        .collect()
}

/// The legend of a person, place or work: the weightiest beats of its life
/// the World tells, at most [`MOST_LEGEND_LINES`], each once and in day
/// order, each with its cause when that cause is a beat of its own and
/// not the line before's.
pub fn legend(teller: &impl Teller, subject: SelectionId) -> Option<Legend> {
    let SelectionId::Entity(id) = subject else {
        return None;
    };
    let title = teller.title(id)?;
    let world = teller.world();
    let person = teller.is_person(id);
    let events = life_events(world, id, teller.naming_kinds());
    let mut seen = BTreeSet::new();
    let mut kept = Vec::new();
    // Beats of one sort are told a few times, not every time: the first
    // three friends made, the first three quarrels.
    let mut sorts = std::collections::BTreeMap::<(String, String), usize>::new();
    for event in &events {
        if turned_down(teller, event) {
            continue;
        }
        let Some(text) = teller.line(id, event) else {
            continue;
        };
        if person && !own(teller, id, event, &text) {
            continue;
        }
        if !seen.insert(text.clone()) {
            continue;
        }
        let sort = (
            event.kind.clone(),
            crate::text(event, "bond")
                .or(crate::text(event, "kind"))
                .unwrap_or_default()
                .to_string(),
        );
        let told = sorts.entry(sort).or_default();
        *told += 1;
        if *told > MOST_OF_A_SORT {
            continue;
        }
        kept.push((weight(event, id), text, event.id));
    }
    if person {
        kept.extend(work_lines(teller, id, &events));
    }
    // The weightiest first, the earlier on a tie; then told in day order.
    kept.sort_by_key(|(weight, _, event)| (std::cmp::Reverse(*weight), *event));
    kept.truncate(MOST_LEGEND_LINES);
    kept.sort_by_key(|(_, _, event)| *event);
    let mut lines: Vec<LegendLine> = Vec::new();
    for (_, text, event) in kept {
        let event = world.event(event)?;
        let mut found = because(teller, event);
        // The same cause twice running is said once.
        if lines.last().and_then(|line| line.cause) == found.as_ref().map(|(cause, _)| *cause) {
            found = None;
        }
        lines.push(LegendLine {
            day: day_of(event.world_time, teller.day_length()),
            text,
            cause: found.as_ref().map(|(cause, _)| *cause),
            because: found.map(|(_, because)| because),
            event: Some(event.id),
        });
    }
    Some(Legend {
        subject,
        title,
        lines,
    })
}

/// The fewest lines a life is told in once there is enough of it: a
/// person who has lived a year is never two lines long.
pub const FEWEST_LIFE_LINES: usize = 5;

/// A person's legend with too few lines of its own, told fuller from what
/// happened around them while they were here: each festival the place held
/// in their life, the first time it came round for them ("Bo's first
/// Lantern Night"), until the life has [`FEWEST_LIFE_LINES`]. Every line
/// is a festival the World recorded; nothing is made up.
pub fn filled_life(teller: &impl Teller, mut legend: Legend) -> Legend {
    let SelectionId::Entity(id) = legend.subject else {
        return legend;
    };
    if !teller.is_person(id) || legend.lines.len() >= FEWEST_LIFE_LINES {
        return legend;
    }
    let world = teller.world();
    let state = world.state();
    let first = lives::first_name(state, id);
    let Some(since) = legend.lines.first().map(|line| line.day) else {
        return legend;
    };
    if first.is_empty() {
        return legend;
    }
    let born_here = world
        .events_of_kind(&["born"])
        .iter()
        .any(|event| entity(event, "who") == Some(id) || creates(event, id));
    // Until they left or died, if they did.
    let until = world
        .events_of_kind(&["died"])
        .iter()
        .filter(|event| entity(event, "who") == Some(id))
        .map(|event| day_of(event.world_time, teller.day_length()))
        .min()
        .unwrap_or(u32::MAX);
    let told = legend
        .lines
        .iter()
        .filter_map(|line| line.event)
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut more = Vec::new();
    for event in world.events_of_kind(&["festival_held"]) {
        let day = day_of(event.world_time, teller.day_length());
        if day <= since || day > until || told.contains(&event.id) {
            continue;
        }
        let Some(festival) = text(event, "name") else {
            continue;
        };
        if !seen.insert(festival.to_string()) {
            continue;
        }
        let here = if born_here { "" } else { " here" };
        more.push(LegendLine {
            day,
            text: format!("{first}'s first {festival}{here}"),
            because: None,
            event: Some(event.id),
            cause: None,
        });
        if legend.lines.len() + more.len() >= FEWEST_LIFE_LINES {
            break;
        }
    }
    legend.lines.extend(more);
    legend.lines.sort_by_key(|line| (line.day, line.event));
    legend
}

/// A key beat, as a Pack describes it: which Event, what kind, who and
/// where, and the three captions in the World's voice.
#[derive(Clone, Debug)]
pub struct Beat<'a> {
    pub event: &'a Event,
    pub kind: MomentKind,
    pub title: String,
    /// The one it is about first.
    pub cast: Vec<EntityId>,
    pub place: Option<EntityId>,
    /// Before, the moment, after.
    pub captions: [String; 3],
    pub moods: [Option<Mood>; 3],
    /// What else each panel shows, before, the moment and after: the
    /// ferry at a farewell, the bunting at a wedding.
    pub props: [Vec<Prop>; 3],
}

impl Beat<'_> {
    pub fn into_moment(self, day_length: u64) -> Moment {
        let cast = self
            .cast
            .iter()
            .copied()
            .map(SelectionId::Entity)
            .collect::<Vec<_>>();
        let place = self.place.map(SelectionId::Entity);
        let [before, moment, after] = self.captions;
        let [props_before, props_moment, props_after] = self.props;
        let panel =
            |caption: String, beat: PanelBeat, mood: Option<Mood>, props: Vec<Prop>| Panel {
                caption,
                cast: cast.clone(),
                place,
                mood,
                beat,
                props,
            };
        Moment {
            id: moment_id(self.event.id),
            day: day_of(self.event.world_time, day_length),
            kind: self.kind,
            title: self.title,
            panels: [
                panel(before, PanelBeat::Before, self.moods[0], props_before),
                panel(moment, PanelBeat::Moment, self.moods[1], props_moment),
                panel(after, PanelBeat::After, self.moods[2], props_after),
            ],
            event: Some(self.event.id),
        }
    }
}

/// What a panel shows besides its people: the props its caption's own
/// words name (`words`, each a word or phrase in lower case and the prop
/// it shows), after `always`, each once. A caption that says "the ferry"
/// shows the ferry, so what is drawn is what is told.
pub fn props_for(caption: &str, words: &[(&str, Prop)], always: &[Prop]) -> Vec<Prop> {
    let lower = caption.to_lowercase();
    let mut props: Vec<Prop> = Vec::new();
    for prop in always.iter().copied().chain(
        words
            .iter()
            .filter(|(word, _)| lower.contains(word))
            .map(|(_, prop)| *prop),
    ) {
        if !props.contains(&prop) {
            props.push(prop);
        }
    }
    props
}

/// `cast`, and then whoever of `people` the captions name, up to `most`,
/// so everyone a moment's words name is drawn in it.
pub fn named_in(
    world: &World,
    captions: &[String],
    mut cast: Vec<EntityId>,
    people: impl IntoIterator<Item = EntityId>,
    most: usize,
) -> Vec<EntityId> {
    let state = world.state();
    let words = captions
        .iter()
        .flat_map(|caption| caption.split(|c: char| !c.is_alphanumeric()))
        .collect::<BTreeSet<_>>();
    for person in people {
        if cast.len() >= most {
            break;
        }
        if !cast.contains(&person) && words.contains(lives::first_name(state, person).as_str()) {
            cast.push(person);
        }
    }
    cast.truncate(most);
    cast
}

/// The people a caption may name, each with their first name, worked out
/// once for all of a history's beats (see [`named_among`]).
pub struct Folk {
    people: Vec<(EntityId, String)>,
    by_name: std::collections::BTreeMap<String, Vec<usize>>,
    /// The first bytes names start with, so most words are passed over
    /// without being looked up.
    starts: [bool; 256],
}

impl Folk {
    pub fn new(world: &World, people: impl IntoIterator<Item = EntityId>) -> Self {
        let state = world.state();
        let people = people
            .into_iter()
            .map(|person| (person, lives::first_name(state, person)))
            .collect::<Vec<_>>();
        let mut by_name = std::collections::BTreeMap::<String, Vec<usize>>::new();
        for (at, (_, name)) in people.iter().enumerate() {
            by_name.entry(name.clone()).or_default().push(at);
        }
        let mut starts = [false; 256];
        for name in by_name.keys() {
            if let Some(first) = name.bytes().next() {
                starts[usize::from(first)] = true;
            }
        }
        Self {
            people,
            by_name,
            starts,
        }
    }
}

/// [`named_in`], with the people and their names found once: `cast`, and
/// then whoever of `folk` the captions name, up to `most`.
pub fn named_among(
    folk: &Folk,
    captions: &[String],
    mut cast: Vec<EntityId>,
    most: usize,
) -> Vec<EntityId> {
    let mut named = vec![false; folk.people.len()];
    for word in captions
        .iter()
        .flat_map(|caption| caption.split(|c: char| !c.is_alphanumeric()))
    {
        // A word no name starts as is no name (the empty word is looked up,
        // for someone with no name at all).
        if word
            .bytes()
            .next()
            .is_some_and(|first| !folk.starts[usize::from(first)])
        {
            continue;
        }
        if let Some(people) = folk.by_name.get(word) {
            for at in people {
                named[*at] = true;
            }
        }
    }
    for (at, (person, _)) in folk.people.iter().enumerate() {
        if cast.len() >= most {
            break;
        }
        if named[at] && !cast.contains(person) {
            cast.push(*person);
        }
    }
    cast.truncate(most);
    cast
}

/// A moment's id, from the Event it shows: the same for as long as the
/// history is.
pub fn moment_id(event: EventId) -> String {
    format!("moment-{event}")
}

/// How much a kind of beat weighs when a year's best moment is chosen,
/// the weightiest first.
pub fn rank(kind: MomentKind) -> u32 {
    match kind {
        MomentKind::Death => 0,
        MomentKind::Birth => 1,
        MomentKind::Wedding => 2,
        MomentKind::ComingOfAge => 3,
        MomentKind::Farewell => 4,
        MomentKind::WorkOpened => 5,
        MomentKind::Storm => 6,
        MomentKind::Festival => 7,
        MomentKind::Other => 8,
    }
}

/// Every moment of a history, at most one a day, oldest first. Of a
/// day's beats the first recorded is kept, so a moment once shown is never
/// taken back by something later the same day.
pub fn moments(beats: Vec<Beat<'_>>, day_length: u64) -> Vec<Moment> {
    one_a_day(
        beats
            .into_iter()
            .map(|beat| beat.into_moment(day_length))
            .collect(),
        |_| 0,
    )
}

/// One of a World's years, in world time: from `from`, up to but not
/// including `to`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Year {
    /// Counted from 1.
    pub number: u32,
    pub from: u64,
    pub to: u64,
}

impl Year {
    /// Year `number` of a World whose years are `year_days` days of
    /// `day_length`.
    pub fn of(number: u32, year_days: u64, day_length: u64) -> Self {
        let length = year_days * day_length;
        let from = u64::from(number.max(1) - 1) * length;
        Self {
            number: number.max(1),
            from,
            to: from + length,
        }
    }

    pub fn holds(&self, world_time: u64) -> bool {
        world_time >= self.from && world_time < self.to
    }
}

/// The Events of these kinds recorded in `year`, oldest first.
pub fn in_year<'a>(world: &'a World, kinds: &[&str], year: Year) -> Vec<&'a Event> {
    world
        .events_of_kind(kinds)
        .into_iter()
        .filter(|event| year.holds(event.world_time))
        .collect()
}

/// Whoever of `everyone` came to live in the World in `year`: the Event that
/// made them was recorded then, and was not a birth.
fn arrived(world: &World, everyone: &[EntityId], year: Year) -> Vec<EntityId> {
    let index = world.history_index();
    everyone
        .iter()
        .copied()
        .filter(|who| {
            index
                .changes_of(*who)
                .first()
                .and_then(|id| world.event(*id))
                .is_some_and(|made| {
                    year.holds(made.world_time)
                        && made.kind != "born"
                        && made.changes.iter().any(|change| {
                            matches!(change, StateChange::CreateEntity(entity) if entity.id == *who)
                        })
                })
        })
        .collect()
}

/// Whoever of `everyone` left the World in `year` without dying.
fn left(world: &World, everyone: &[EntityId], year: Year) -> Vec<EntityId> {
    let index = world.history_index();
    let state = world.state();
    everyone
        .iter()
        .copied()
        .filter(|who| lives::gone(state, *who))
        .filter(|who| {
            index
                .changes_of(*who)
                .iter()
                .rev()
                .filter_map(|id| world.event(*id))
                .find(|event| {
                    event.changes.iter().any(|change| {
                        matches!(change, StateChange::SetComponent { entity, key, value: Value::Bool(true) }
                            if entity == who && key == lives::GONE)
                    })
                })
                .is_some_and(|went| year.holds(went.world_time) && went.kind != "died")
        })
        .collect()
}

/// Whose life beat of this kind each Event of it is.
fn whose(event: &Event) -> Option<EntityId> {
    entity(event, "who").or_else(|| event.targets.first().copied())
}

/// A year in review: who came and left, who was born and died, what was
/// built (as the Pack names it), and the year's best moment.
pub fn almanac(
    teller: &impl Teller,
    year: Year,
    title: String,
    everyone: &[EntityId],
    built: Vec<Named>,
    moments: &[Moment],
) -> Almanac {
    let world = teller.world();
    let mut cast = Vec::new();
    let mut named = |ids: Vec<EntityId>| {
        ids.into_iter()
            .map(|id| {
                let name = name(teller, id);
                cast.push(Named {
                    name: name.clone(),
                    who: SelectionId::Entity(id),
                });
                name
            })
            .collect::<Vec<_>>()
    };
    let arrived = named(arrived(world, everyone, year));
    let left = named(left(world, everyone, year));
    let born = named(
        in_year(world, &["born"], year)
            .into_iter()
            .filter_map(whose)
            .collect(),
    );
    let died = named(
        in_year(world, &["died"], year)
            .into_iter()
            .filter_map(whose)
            .collect(),
    );
    let built_names = built.iter().map(|named| named.name.clone()).collect();
    cast.extend(built);
    let best = moments
        .iter()
        .filter(|moment| {
            moment
                .event
                .and_then(|id| world.event(id))
                .is_some_and(|event| year.holds(event.world_time))
        })
        .min_by_key(|moment| (rank(moment.kind), moment.day))
        .map(|moment| moment.id.clone());
    Almanac {
        year: year.number,
        title,
        arrived,
        left,
        born,
        died,
        built: built_names,
        best,
        cast,
    }
}

#[cfg(test)]
mod tests;
