//! The place's legends: the life of anyone who lived in it, and of its
//! places and the things made in it, told line by line from what the
//! World recorded, each line with what brought it about.

use chronicle::{text, Answered, Teller};
use std::sync::OnceLock;
use world_core::{EntityId, Event, World};
use world_projection::{Legend, SelectionId};

/// The place, as the chronicle reads it.
pub(crate) struct Place<'a>(pub(crate) &'a World);

/// The storylets that are a storm the place weathers.
const STORMS: [&str; 1] = ["weather"];

/// Kinds that concern someone without always changing them.
const NAMING: &[&str] = &[
    "born",
    "came_of_age",
    "left_home",
    "retired",
    "died",
    "heirloom_passed",
    "memorial_placed",
    "anniversary_kept",
    "year_turned",
    "bond_changed",
    "situation_answered",
    "situation_lapsed",
    "greeted",
    "keepsake_left",
    "gift_given",
    "invited_out",
    "built_by_hand",
    "decorated_by_hand",
    "planted_by_hand",
    "festival_held",
];

/// The everyday round and the World's bookkeeping, never a line.
const EVERYDAY: &[&str] = &[
    "lived",
    "enjoyed",
    "first_mentioned",
    "year_remembered",
    "fixture_passed",
    "festival_nears",
    "situation_arose",
    "situation_came_up",
    "chapter_ended",
    "chapter_turning",
    "reacted",
    "moved_by_hand",
    "lines_forgotten",
    "warmed",
    "life_began",
    "plant_grew",
    "undone_by_hand",
    "letter_written",
    "story_began",
    "bond_settled",
    "spoken",
];

pub(crate) fn storm_kinds() -> &'static [&'static str] {
    static KINDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    KINDS.get_or_init(|| {
        let mut kinds = crate::story::outcome_kinds(&STORMS);
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
            // The places and things each place begins with, named as
            // written wherever a line begins with one.
            .chain([
                "Ares",
                "Aurora",
                "Fish Vault",
                "Hydroponics",
                "Icebridge",
                "K-88",
                "Kestrel",
                "Maple",
                "Night Bus",
            ])
            .collect()
    })
}

/// Whether someone is one of the place's people: the pair, and whoever
/// came or was born to live there, gone or not.
pub(crate) fn is_person(world: &World, id: EntityId) -> bool {
    let state = world.state();
    state.entity(id).is_some()
        && ([crate::SLOT_B, crate::SLOT_E, crate::story::NEWCOMER].contains(&id)
            || crate::life::newcomers(state).contains(&id))
}

fn involves(event: &Event, who: EntityId) -> bool {
    event.actor == Some(who) || event.targets.contains(&who)
}

/// The first time a festival was held, for a place's legend.
fn first_held(world: &World, event: &Event) -> bool {
    let festival = text(event, "festival");
    world_projection::latest_before(world, &["festival_held"], event.id, |held| {
        text(held, "festival") == festival
    })
    .is_none()
}

impl Teller for Place<'_> {
    fn world(&self) -> &World {
        self.0
    }

    fn day_length(&self) -> u64 {
        crate::BACKGROUND_PERIOD
    }

    fn names(&self) -> &[&'static str] {
        names()
    }

    fn title(&self, subject: EntityId) -> Option<String> {
        let entity = self.0.state().entity(subject)?;
        Some(if is_person(self.0, subject) {
            lives::name(self.0.state(), subject)
        } else {
            crate::story::fill(self.0, &world_projection::entity_title(entity))
        })
    }

    fn line(&self, subject: EntityId, event: &Event) -> Option<String> {
        let world = self.0;
        let kind = event.kind.as_str();
        if EVERYDAY.contains(&kind) {
            return None;
        }
        if chronicle::LIFE_BEATS.contains(&kind) {
            return lives::told(event)
                .or_else(|| text(event, "told").map(str::to_string))
                .map(|told| crate::plots::renamed(self.0.state(), event, told));
        }
        let person = is_person(world, subject);
        match kind {
            "bond_changed" | "situation_answered" | "situation_lapsed" | "greeted"
            | "keepsake_left" | "year_turned"
                if person && !involves(event, subject) =>
            {
                // A year's turn that changed them is theirs too.
                (kind == "year_turned")
                    .then(|| lives::told(event))
                    .flatten()
            }
            "festival_held" if person || !first_held(world, event) => None,
            _ if chronicle::DEEDS.contains(&kind) => text(event, "told").map(str::to_string),
            _ if text(event, "storylet").is_some() && person && !involves(event, subject) => None,
            _ => crate::story::told(world, event),
        }
    }

    fn naming_kinds(&self) -> &[&'static str] {
        NAMING
    }

    fn storylet_answer(&self, event: &Event) -> Option<Answered> {
        Some(match crate::story::answer_words(self.0, event)? {
            Some(words) if crate::story::answer_refuses(event) => Answered::Refused(words),
            Some(words) => Answered::Said(words),
            None => Answered::Lapsed,
        })
    }

    fn cause_words(&self, cause: &Event) -> Option<String> {
        storm_kinds()
            .contains(&cause.kind.as_str())
            .then(|| "after the storm".to_string())
    }

    fn storm_kinds(&self) -> &[&'static str] {
        storm_kinds()
    }

    fn is_person(&self, id: EntityId) -> bool {
        is_person(self.0, id)
    }
}

/// The legend of a person, place or thing.
pub(crate) fn legend(world: &World, subject: SelectionId) -> Option<Legend> {
    let teller = Place(world);
    chronicle::legend(&teller, subject).map(|legend| chronicle::filled_life(&teller, legend))
}
