//! The harbour's legends: the life of anyone who lived here, and of its
//! places and the things made in it, told line by line from what the
//! World recorded, each line with what brought it about.

use chronicle::{kit, text, Answered, Teller};
use std::sync::OnceLock;
use world_core::{EntityId, Event, World};
use world_projection::{Legend, SelectionId};

/// The harbour, as the chronicle reads it.
pub(crate) struct Harbour<'a>(pub(crate) &'a World);

/// The storylets that are a storm the harbour weathers.
const STORMS: [&str; 2] = ["great_storm", "storm_warning"];

/// Kinds of this World's own everyday round and bookkeeping, never a line.
const EVERYDAY: &[&str] = &[
    "work_shift_completed",
    "bread_purchased",
    "living_cost_paid",
    "catch_landed",
    "fish_sold",
    "counter_sale_recorded",
    "market_restocked",
    "spirits_settled",
    "payroll_shortfall",
    "living_cost_unmet",
];

pub(crate) fn storm_kinds() -> &'static [&'static str] {
    static KINDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    KINDS.get_or_init(|| {
        let mut kinds = crate::story::outcome_kinds(&STORMS);
        kinds.push("storm_started");
        kinds.sort_unstable();
        kinds.dedup();
        kinds
    })
}

pub(crate) fn names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| {
        crate::people_names()
            .into_iter()
            .flat_map(|name| name.split_whitespace())
            .chain(["Harbour Bakery", "Anchor Pub", "Island School", "Sea Finch"])
            .collect()
    })
}

fn is_person(world: &World, id: EntityId) -> bool {
    world
        .state()
        .entity(id)
        .is_some_and(|entity| entity.kind == "resident")
}

impl Teller for Harbour<'_> {
    fn world(&self) -> &World {
        self.0
    }

    fn day_length(&self) -> u64 {
        crate::persistence::WORLD_DAY_TICKS
    }

    fn names(&self) -> &[&'static str] {
        names()
    }

    fn title(&self, subject: EntityId) -> Option<String> {
        let entity = self.0.state().entity(subject)?;
        Some(match entity.kind.as_str() {
            "resident" => lives::name(self.0.state(), subject),
            _ => world_projection::entity_title(entity),
        })
    }

    fn line(&self, subject: EntityId, event: &Event) -> Option<String> {
        let world = self.0;
        let kind = event.kind.as_str();
        if kit::everyday(kind, EVERYDAY) {
            return None;
        }
        if chronicle::LIFE_BEATS.contains(&kind) {
            return lives::told(event)
                .or_else(|| text(event, "told").map(str::to_string))
                .map(|told| crate::plots::renamed(world.state(), event, told));
        }
        let person = is_person(world, subject);
        match kind {
            // A year's turn is someone's; the harbour's new festival is not.
            "year_turned" if person && text(event, "beat")?.starts_with("festival_") => None,
            // Among people, what happened between them or to them.
            "bond_changed" | "situation_answered" | "situation_lapsed" | "greeted"
            | "keepsake_left"
                if !kit::involves(event, subject) =>
            {
                None
            }
            // A festival is a place's, the first time it is held there.
            "festival_held" if person || !kit::first_held(world, event) => None,
            _ if chronicle::DEEDS.contains(&kind) => text(event, "told").map(str::to_string),
            // The storyteller's questions are the asker's, and the harbour's.
            _ if text(event, "storylet").is_some() && person && !kit::involves(event, subject) => {
                None
            }
            _ => crate::projection::narrated_title(world, event),
        }
    }

    fn naming_kinds(&self) -> &[&'static str] {
        kit::NAMING
    }

    fn storylet_answer(&self, event: &Event) -> Option<Answered> {
        Some(match crate::story::answer_words(event)? {
            Some(words) if crate::story::answer_refuses(event) => Answered::Refused(words.into()),
            Some(words) => Answered::Said(words.into()),
            None => Answered::Lapsed,
        })
    }

    fn cause_words(&self, cause: &Event) -> Option<String> {
        if cause.kind == "storm_started" {
            return Some("after the storm".into());
        }
        if !storm_kinds().contains(&cause.kind.as_str()) {
            return None;
        }
        Some(match text(cause, "storylet") {
            Some("great_storm") => "after the great storm".into(),
            _ => "after the storm".into(),
        })
    }

    fn storm_kinds(&self) -> &[&'static str] {
        storm_kinds()
    }

    fn is_person(&self, id: EntityId) -> bool {
        is_person(self.0, id)
    }
}

/// The legend of a person, place or thing in the harbour.
pub(crate) fn legend(world: &World, subject: SelectionId) -> Option<Legend> {
    kit::filled_legend(&Harbour(world), subject)
}
