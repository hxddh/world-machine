//! Words a player reads over years, measured as a player meets them: what
//! is said too often, what is told again and again, what is sad too soon,
//! what shows a template's seams, and what is the engine's word rather
//! than the World's.
//!
//! A Pack plays its five players ([`crate::players`]) with a [`Words`]
//! watching each day, then holds every player to the bars:
//! - no line said more than [`MOST_SAID_IN_YEAR_THREE`] times in year three;
//! - nothing told more than [`MOST_TOLD_IN_A_WEEK`] times in any seven days,
//!   across the cards that ask about it, the beats of the film a returning
//!   player is shown, the notes left and written, and the moment strips;
//! - nothing sad or unkind in days 1 to [`KIND_DAYS`];
//! - no seam in anything the World can tell, by day 1,080;
//! - no engine word ([`ENGINE_WORDS`]) in anything a player reads.

use crate::seams;
use std::collections::{BTreeMap, BTreeSet};
use world_core::{EventId, Value, World};
use world_projection::{BriefingProjection, ProjectionSnapshot, SelectionId};

/// The most any one line may be said in year three.
pub const MOST_SAID_IN_YEAR_THREE: usize = 8;

/// The most anything may be told in any seven days.
pub const MOST_TOLD_IN_A_WEEK: usize = 2;

/// The first days, in which nothing sad or unkind is said.
pub const KIND_DAYS: usize = 5;

/// Once a week the player is away for the last [`AWAY_DAYS`] days of it,
/// and shown the film of what happened when they come back. A moment's
/// strip from those days is shown on their return only if the film did
/// not tell it.
pub const RETURNS_EVERY: usize = 7;
pub const AWAY_DAYS: usize = 2;

/// The engine's words, which a player should never read: the names of
/// its parts and its ledgers rather than the World's own words for them.
pub const ENGINE_WORDS: &[&str] = &[
    "payroll reserve",
    "exhausted its",
    "reserve",
    "payroll",
    "revenue",
    "entity",
    "component",
    "projection",
    "snapshot",
    "storylet",
    "null",
    "undefined",
    "todo",
];

/// Engine words inside a World's own words, where they are allowed.
const ENGINE_WORDS_IN_PLACE: &[(&str, &str)] = &[];

/// The engine word in `line`, if it has one, as a whole word (or a dotted
/// key: "Lives.opinion.430").
pub fn engine_word(line: &str) -> Option<&'static str> {
    let lower = line.to_lowercase();
    // A dotted key: a word, a dot, a word, with no space between.
    let dotted = lower.split_whitespace().any(|word| {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric());
        let parts = word.split('.').collect::<Vec<_>>();
        parts.len() >= 3 && parts.iter().all(|part| !part.is_empty())
    });
    if dotted {
        return Some("dotted key");
    }
    let words = lower
        .split(|c: char| !(c.is_alphanumeric() || c == '.' || c == '\''))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    ENGINE_WORDS.iter().copied().find(|engine| {
        let engine: &str = engine;
        if engine.contains(' ') {
            return lower.contains(engine);
        }
        words.iter().any(|word| {
            let word = word.trim_end_matches('.');
            (word == engine || word.strip_suffix('s') == Some(engine))
                && !ENGINE_WORDS_IN_PLACE
                    .iter()
                    .any(|(of, ok)| *of == engine && word.contains(ok))
        })
    })
}

/// One time something was told.
#[derive(Clone, Debug)]
pub struct Telling {
    pub day: usize,
    /// "card", "film", "note" or "strip".
    pub how: &'static str,
    pub what: String,
}

/// What a player's years brought in words.
#[derive(Default)]
pub struct Words {
    pub label: String,
    /// Words that are sad or unkind, for this World.
    unkind: Vec<&'static str>,
    /// Lines said each year, and how often.
    pub said: [BTreeMap<String, usize>; 3],
    /// Every line met, for seams.
    seen: BTreeSet<String>,
    pub seams: BTreeSet<String>,
    /// Sad or unkind lines in the first days, with their day.
    pub sad: BTreeSet<String>,
    /// Every telling of each thing, by what it is about.
    pub tellings: BTreeMap<String, Vec<Telling>>,
    moments_seen: BTreeSet<String>,
    letters_seen: BTreeSet<SelectionId>,
    keepsakes_seen: BTreeSet<SelectionId>,
    /// Engine words met.
    pub engine: BTreeSet<String>,
    /// Moment strips that show neither two people nor a place the town has.
    pub strips: BTreeSet<String>,
    /// The events count as the player went away.
    went_away_at: usize,
    /// Strips of moments that came while the player was away.
    away_strips: Vec<(usize, EventId, String)>,
}

/// What `event` and its causes are about: a storylet, a festival, or
/// someone's news.
pub fn topic(world: &World, id: EventId) -> Option<String> {
    let mut at = world.event(id)?;
    for _ in 0..5 {
        if let Some(Value::Text(storylet)) = at.payload.get("storylet") {
            return Some(storylet.clone());
        }
        if let Some(Value::Text(festival)) = at.payload.get("festival") {
            return Some(format!("festival {festival}"));
        }
        if lives::is_news(at) && at.kind != "letter_written" && at.kind != "keepsake_left" {
            let others = at
                .payload
                .values()
                .filter_map(|value| match value {
                    Value::Entity(entity) => Some(format!("{entity:?}")),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            let which = match at.payload.get("situation") {
                Some(Value::Text(which)) => which.clone(),
                _ => String::new(),
            };
            return Some(format!("{} {which} {:?} {others:?}", at.kind, at.actor));
        }
        let cause = at.caused_by.first()?;
        at = world.event(*cause)?;
    }
    None
}

impl Words {
    pub fn new(label: impl Into<String>, unkind: &[&'static str]) -> Self {
        Self {
            label: label.into(),
            unkind: unkind.to_vec(),
            ..Self::default()
        }
    }

    fn tell(&mut self, world: &World, day: usize, how: &'static str, id: EventId, what: &str) {
        if let Some(topic) = topic(world, id) {
            self.tellings.entry(topic).or_default().push(Telling {
                day,
                how,
                what: what.to_string(),
            });
        }
    }

    /// Watches one day: `began` is how the day looked as it began (what
    /// the player read and heard), `world` and `after` how it stands once
    /// it has passed. `events_before` is how many Events the World had as
    /// the day began; `told` is everything the World can tell, when it was
    /// asked for today; `returned` the film a player returning now would
    /// be shown, when they would.
    #[allow(clippy::too_many_arguments)]
    pub fn watch(
        &mut self,
        day: usize,
        began: &ProjectionSnapshot,
        events_before: usize,
        world: &World,
        after: &ProjectionSnapshot,
        told: Option<Vec<String>>,
        returned: impl FnOnce(usize) -> Option<BriefingProjection>,
    ) {
        // What was said aloud as the day began.
        let now = began.world_time;
        let year = ((day - 1) / 360).min(2);
        for voice in &began.voices {
            let said_now = began
                .timeline
                .items
                .iter()
                .any(|item| item.id == voice.moment && item.world_time == now);
            if said_now {
                *self.said[year].entry(voice.line.clone()).or_default() += 1;
            }
        }
        // Everything readable, for seams, sadness and the engine's words.
        let lines = told.unwrap_or_else(|| seams::readable(after, true));
        let mut read = seams::readable(began, true);
        read.extend(lines.iter().cloned());
        if day <= KIND_DAYS {
            for line in &seams::readable(began, true) {
                let lower = line.to_lowercase();
                if let Some(word) = self.unkind.iter().find(|word| lower.contains(*word)) {
                    self.sad
                        .insert(format!("{}: day {day} [{word}] {line}", self.label));
                }
            }
        }
        for line in &read {
            if self.seen.contains(line) {
                continue;
            }
            if let Some(word) = engine_word(line) {
                self.engine
                    .insert(format!("{}: [{word}] {line}", self.label));
            }
        }
        let label = self.label.clone();
        seams::note_seams(&label, read, &mut self.seen, &mut self.seams);
        // Cards: each question that came up today.
        let events = world.events();
        for event in &events[events_before.min(events.len())..] {
            if event.kind == "situation_arose" {
                let what = format!("card {}", event.kind);
                self.tell(world, day, "card", event.id, &what);
            }
        }
        // Strips: each moment new today, kept for the return when the
        // player is away.
        let week_day = day % RETURNS_EVERY;
        let away = week_day == 0 || week_day > RETURNS_EVERY - AWAY_DAYS;
        if week_day == RETURNS_EVERY - AWAY_DAYS + 1 {
            self.went_away_at = events_before;
        }
        for moment in &after.moments {
            if self.moments_seen.insert(moment.id.clone()) {
                if let Some(event) = moment.event {
                    if away {
                        self.away_strips.push((day, event, moment.title.clone()));
                    } else {
                        self.tell(world, day, "strip", event, &moment.title);
                    }
                }
            }
        }
        // Notes: each letter and keepsake new today.
        for letter in &after.letters {
            if self.letters_seen.insert(letter.moment) {
                if let SelectionId::Event(event) = letter.moment {
                    self.tell(world, day, "note", event, &letter.note);
                }
            }
        }
        for kept in &after.keepsakes {
            if self.keepsakes_seen.insert(kept.moment) {
                if let SelectionId::Event(event) = kept.moment {
                    self.tell(world, day, "note", event, &kept.note);
                }
            }
        }
        // The film the player is shown coming back.
        if week_day == 0 {
            let mut filmed = BTreeSet::new();
            if let Some(briefing) = returned(self.went_away_at) {
                for beat in briefing.beats() {
                    if let Some(SelectionId::Event(event)) = beat.selection {
                        filmed.insert(event);
                        self.tell(world, day, "film", event, &beat.title);
                    }
                }
            }
            for (on, event, title) in std::mem::take(&mut self.away_strips) {
                if !filmed.contains(&event) {
                    self.tell(world, on.max(day), "strip", event, &title);
                }
            }
        }
    }

    /// Holds each of `moments` to what a strip must show: what it is
    /// named for (a storm's rain) or at least two people, and only places
    /// the town (`now`) has.
    pub fn look_at_strips(
        &mut self,
        moments: &[world_projection::Moment],
        now: &ProjectionSnapshot,
    ) {
        let places = now
            .canvas
            .items
            .iter()
            .map(|item| item.id)
            .collect::<BTreeSet<_>>();
        for moment in moments {
            // A storm's strip shows the storm itself.
            let shows_its_thing = moment.kind == world_projection::MomentKind::Storm
                && moment.panels[1]
                    .props
                    .contains(&world_projection::Prop::Rain);
            if moment.cast().len() < 2 && !shows_its_thing {
                self.strips.insert(format!(
                    "{}: strip {:?} (day {}) shows one person",
                    self.label, moment.title, moment.day
                ));
            }
            for panel in &moment.panels {
                if let Some(place) = panel.place.filter(|place| !places.contains(place)) {
                    self.strips.insert(format!(
                        "{}: strip {:?} draws {place:?}, which the town does not have",
                        self.label, moment.title
                    ));
                }
            }
        }
    }

    /// Lines said more than the bar allows in year three, most first.
    pub fn said_too_often(&self) -> Vec<(String, usize)> {
        let mut over = self.said[2]
            .iter()
            .filter(|(_, n)| **n > MOST_SAID_IN_YEAR_THREE)
            .map(|(line, n)| (line.clone(), *n))
            .collect::<Vec<_>>();
        over.sort_by_key(|(line, n)| (std::cmp::Reverse(*n), line.clone()));
        over
    }

    /// Everything told more than the bar allows in some seven days, with
    /// the tellings of the worst week.
    pub fn told_too_often(&self) -> Vec<String> {
        let mut over = Vec::new();
        for (topic, tellings) in &self.tellings {
            let mut worst: Option<&[Telling]> = None;
            for (at, first) in tellings.iter().enumerate() {
                let week = tellings[at..]
                    .iter()
                    .take_while(|telling| telling.day < first.day + 7)
                    .count();
                if week > MOST_TOLD_IN_A_WEEK && worst.is_none_or(|worst| week > worst.len()) {
                    worst = Some(&tellings[at..at + week]);
                }
            }
            if let Some(worst) = worst {
                over.push(format!(
                    "{}: {topic} told {} times from day {}: {}",
                    self.label,
                    worst.len(),
                    worst[0].day,
                    worst
                        .iter()
                        .map(|telling| format!(
                            "{} d{} {:?}",
                            telling.how, telling.day, telling.what
                        ))
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
        }
        over
    }

    /// Prints the numbers, and returns what failed the bars.
    pub fn report(&self) -> Vec<String> {
        let p = &self.label;
        for (year, said) in self.said.iter().enumerate() {
            let total = said.values().sum::<usize>();
            let most = said.values().max().copied().unwrap_or(0);
            eprintln!(
                "WORDS {p} year {} said {total} distinct {} most {most}",
                year + 1,
                said.len()
            );
        }
        let mut top = self.said[2].iter().collect::<Vec<_>>();
        top.sort_by_key(|(line, n)| (std::cmp::Reverse(**n), (*line).clone()));
        for (line, n) in top.iter().take(8) {
            eprintln!("WORDS {p} year 3 top {n} {line}");
        }
        let told = self.told_too_often();
        eprintln!(
            "WORDS {p} told more than {MOST_TOLD_IN_A_WEEK} times in 7 days: {} of {} things",
            told.len(),
            self.tellings.len()
        );
        eprintln!("WORDS {p} seams {}", self.seams.len());
        eprintln!("WORDS {p} sad in days 1-{KIND_DAYS} {}", self.sad.len());
        eprintln!("WORDS {p} engine words {}", self.engine.len());
        eprintln!("WORDS {p} strips at fault {}", self.strips.len());
        let mut failed = Vec::new();
        for (line, n) in self.said_too_often() {
            failed.push(format!("{p}: said {n} times in year 3: {line}"));
        }
        failed.extend(told);
        failed.extend(self.seams.iter().cloned());
        failed.extend(self.sad.iter().cloned());
        failed.extend(self.engine.iter().cloned());
        failed.extend(self.strips.iter().cloned());
        failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engines_words_are_found() {
        for line in [
            "Anchor Pub exhausted its payroll reserve",
            "Lives.opinion.430 5",
            "The storylet fired",
            "Cash reserve 256",
        ] {
            assert!(engine_word(line).is_some(), "{line}");
        }
        for line in [
            "Mara's bread went out on the morning ferry.",
            "A ticket for the ferry, please.",
            "Leo walked with a stick.",
            "Mr. Tanaka sold the last of it.",
            "The bakery could not pay its people this week.",
        ] {
            assert_eq!(engine_word(line), None, "{line}");
        }
    }
}
