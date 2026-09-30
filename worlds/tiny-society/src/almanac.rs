//! The harbour's year: its festivals and the days that simply arrive, and
//! how each goes depends on the harbour as it stands.

use crate::story::{MOOD, STORY};
use crate::{BAKERY, EMMA, EVAN, HARBOR, JONAS, LEO, MARA, MIA, NOAH, PUB, SCHOOL, SOFIA};
use calendar::{Almanac, Festival, Turnout};
use society_basic::CASH;
use world_core::{StateChange, Value, WorldState};

/// Days in the harbour's year.
pub(crate) const YEAR_DAYS: u64 = crate::story::SEASON_DAYS * 4;

pub(crate) const FESTIVALS: &[Festival] = &[
    Festival {
        id: "spring_clean",
        name: "Spring Clean",
        day: 5,
        prepare: 3,
        at: HARBOR,
        speaker: SOFIA,
        shape: "flag",
        harvest: false,
        nears: "Spring Clean is {days} days off. Brooms are coming out.",
        getting_ready: "Spring Clean in {days} days. I've a list as long as my arm.",
        told: [
            "The whole harbour turned out for Spring Clean",
            "The harbour had its Spring Clean",
            "Spring Clean came, and hardly anyone lifted a broom",
        ],
        said: [
            "Look at it shine!",
            "Cleaner than it was, anyway.",
            "Just me and a broom, then.",
        ],
    },
    Festival {
        id: "swallows",
        name: "the swallows",
        day: 13,
        prepare: 0,
        at: HARBOR,
        speaker: NOAH,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "The swallows came back to the harbour",
            "The swallows came back to the harbour",
            "The swallows came back to the harbour",
        ],
        said: [
            "Swallows are back. Summer's on its way.",
            "Swallows are back. Summer's on its way.",
            "Swallows are back. Summer's on its way.",
        ],
    },
    Festival {
        id: "blossom_walk",
        name: "the Blossom Walk",
        day: 22,
        prepare: 4,
        at: SCHOOL,
        speaker: EMMA,
        shape: "lantern",
        harvest: false,
        nears: "The Blossom Walk is {days} days away. The children are making lanterns.",
        getting_ready: "The children have made paper blossoms for the walk.",
        told: [
            "The Blossom Walk wound through the whole harbour",
            "The children led the Blossom Walk",
            "The Blossom Walk was a handful of children in the drizzle",
        ],
        said: [
            "Every family came out!",
            "The children loved it.",
            "Only the parents came.",
        ],
    },
    Festival {
        id: "boat_blessing",
        name: "the Boat Blessing",
        day: 28,
        prepare: 5,
        at: HARBOR,
        speaker: JONAS,
        shape: "bunting",
        harvest: false,
        nears: "The Boat Blessing is {days} days off. Hulls are getting a fresh coat.",
        getting_ready: "Painting Sea Finch for the Blessing.",
        told: [
            "Every boat was blessed with the whole harbour watching",
            "The boats were blessed for the season",
            "The Boat Blessing was a quiet word over empty water",
        ],
        said: [
            "Never seen so many on the quay.",
            "Safe seas, all of us.",
            "Blessed, I suppose.",
        ],
    },
    Festival {
        id: "mackerel",
        name: "the mackerel run",
        day: 38,
        prepare: 0,
        at: HARBOR,
        speaker: JONAS,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "The mackerel came in off the point",
            "The mackerel came in off the point",
            "The mackerel came in off the point",
        ],
        said: [
            "Mackerel's running! Get the lines out.",
            "Mackerel's running! Get the lines out.",
            "Mackerel's running! Get the lines out.",
        ],
    },
    Festival {
        id: "midsummer_fair",
        name: "the Midsummer Fair",
        day: 44,
        prepare: 7,
        at: HARBOR,
        speaker: SOFIA,
        shape: "stall",
        harvest: false,
        nears: "The Midsummer Fair is a week away. Stalls are going up.",
        getting_ready: "A week to the Fair. I'm making jam day and night.",
        told: [
            "The Midsummer Fair filled the quay from end to end",
            "The Midsummer Fair came and went",
            "The Midsummer Fair was three stalls and a gull",
        ],
        said: [
            "Sold out by noon!",
            "A decent day's trade.",
            "Hardly worth setting up.",
        ],
    },
    Festival {
        id: "lantern_night",
        name: "Lantern Night",
        day: 53,
        prepare: 4,
        at: PUB,
        speaker: LEO,
        shape: "lantern",
        harvest: false,
        nears: "Lantern Night is {days} days off. Leo's stringing lights.",
        getting_ready: "Lantern Night soon. Every lamp in the pub's getting polished.",
        told: [
            "Lantern Night lit the harbour until dawn",
            "The harbour had its Lantern Night",
            "Lantern Night was a few lamps and an early night",
        ],
        said: [
            "Best night of the year.",
            "A fine night.",
            "Might as well have stayed in.",
        ],
    },
    Festival {
        id: "geese",
        name: "the geese",
        day: 62,
        prepare: 0,
        at: HARBOR,
        speaker: NOAH,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "The geese flew south over the harbour",
            "The geese flew south over the harbour",
            "The geese flew south over the harbour",
        ],
        said: [
            "Geese going over. Autumn, then.",
            "Geese going over. Autumn, then.",
            "Geese going over. Autumn, then.",
        ],
    },
    Festival {
        id: "harvest_home",
        name: "Harvest Home",
        day: 70,
        prepare: 7,
        at: BAKERY,
        speaker: MARA,
        shape: "stall",
        harvest: true,
        nears: "Harvest Home is a week away. What's been grown is nearly in.",
        getting_ready: "Harvest Home in a week. I'll bake with whatever comes in.",
        told: [
            "Harvest Home was a feast of everything the harbour grew",
            "Harvest Home brought in enough to go round",
            "Harvest Home came, and there was little to gather",
        ],
        said: [
            "Look at it all! I'll be baking for a week.",
            "Enough to go round.",
            "Hardly anything grew this year.",
        ],
    },
    Festival {
        id: "apple_pressing",
        name: "the Apple Pressing",
        day: 78,
        prepare: 3,
        at: PUB,
        speaker: LEO,
        shape: "tent",
        harvest: false,
        nears: "The Apple Pressing is {days} days off. Barrels are being scrubbed.",
        getting_ready: "Getting the press ready. Cider by the weekend.",
        told: [
            "The Apple Pressing ran into the night",
            "The Apple Pressing filled a few barrels",
            "The Apple Pressing was Leo and one sack of apples",
        ],
        said: [
            "Best cider in years.",
            "It'll do.",
            "Sour, and not much of it.",
        ],
    },
    Festival {
        id: "bonfire_night",
        name: "Bonfire Night",
        day: 84,
        prepare: 5,
        at: HARBOR,
        speaker: EVAN,
        shape: "lantern",
        harvest: false,
        nears: "Bonfire Night is {days} days off. Driftwood's piling up on the beach.",
        getting_ready: "Building the bonfire. Biggest one yet.",
        told: [
            "The Bonfire Night blaze could be seen from the next bay",
            "The harbour gathered round the bonfire",
            "Bonfire Night fizzled out in the damp",
        ],
        said: ["Did you see it go up?", "A good fire.", "Wouldn't catch."],
    },
    Festival {
        id: "first_frost",
        name: "the first frost",
        day: 95,
        prepare: 0,
        at: HARBOR,
        speaker: NOAH,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "The first frost silvered the nets",
            "The first frost silvered the nets",
            "The first frost silvered the nets",
        ],
        said: [
            "Frost on the nets. Winter's here.",
            "Frost on the nets. Winter's here.",
            "Frost on the nets. Winter's here.",
        ],
    },
    Festival {
        id: "school_play",
        name: "the school play",
        day: 100,
        prepare: 5,
        at: SCHOOL,
        speaker: EMMA,
        shape: "flag",
        harvest: false,
        nears: "The school play is {days} days away. Lines are being learned.",
        getting_ready: "Rehearsals every afternoon. Mia has the lead.",
        told: [
            "The school play had the whole harbour on its feet",
            "The children put on their play",
            "The school play went on to half an empty room",
        ],
        said: [
            "Standing ovation!",
            "They remembered most of their lines.",
            "Poor things, playing to empty chairs.",
        ],
    },
    Festival {
        id: "midwinter_feast",
        name: "the Midwinter Feast",
        day: 106,
        prepare: 7,
        at: PUB,
        speaker: LEO,
        shape: "lantern",
        harvest: false,
        nears: "The Midwinter Feast is a week away. The pub's getting ready.",
        getting_ready: "A week to the Feast. I've ordered twice what we need.",
        told: [
            "The Midwinter Feast had the whole harbour round one table",
            "The harbour shared its Midwinter Feast",
            "The Midwinter Feast was a few of us and a cold pie",
        ],
        said: [
            "Everyone came. Everyone.",
            "Good food, good company.",
            "Too much pie, not enough people.",
        ],
    },
    Festival {
        id: "year_end_swim",
        name: "the Year's End Swim",
        day: 117,
        prepare: 2,
        at: HARBOR,
        speaker: MIA,
        shape: "flag",
        harvest: false,
        nears: "The Year's End Swim is {days} days off. Who's brave enough?",
        getting_ready: "I'm doing the swim this year. Don't laugh.",
        told: [
            "Half the harbour ran into the sea for the Year's End Swim",
            "A brave few did the Year's End Swim",
            "Nobody but Mia went in for the Year's End Swim",
        ],
        said: [
            "Freezing! Best thing ever!",
            "Cold, but I did it.",
            "Just me, then. Brr.",
        ],
    },
];

/// How ready the harbour is to celebrate: its spirits and what the player
/// has made, decorations counting double.
fn festive(state: &WorldState) -> i64 {
    let mood = match state.entity(STORY).and_then(|story| story.component(MOOD)) {
        Some(Value::Integer(mood)) => *mood,
        _ => 0,
    };
    let made = calendar::decorated(state);
    2 + mood + made
}

/// How many gardens the player planted have grown.
pub(crate) use calendar::gardens_grown as grown;

fn held(state: &WorldState, festival: &Festival, turnout: Turnout, grown: i64) -> Vec<StateChange> {
    let mut changes = Vec::new();
    let by = match turnout {
        Turnout::Grand => 1,
        Turnout::Fine => 0,
        Turnout::Thin => -1,
    };
    if let (true, Some(story)) = (by != 0, state.entity(STORY)) {
        let mood = match story.component(MOOD) {
            Some(Value::Integer(mood)) => *mood,
            _ => 0,
        };
        changes.push(StateChange::SetComponent {
            entity: STORY,
            key: MOOD.into(),
            value: (mood + by).clamp(-5, 5).into(),
        });
    }
    if festival.harvest && grown > 0 {
        if let Some(fund) = state.entity(HARBOR) {
            let cash = match fund.component(CASH) {
                Some(Value::Integer(cash)) => *cash,
                _ => 0,
            };
            changes.push(StateChange::SetComponent {
                entity: HARBOR,
                key: CASH.into(),
                value: (cash + 20 * grown).into(),
            });
        }
    }
    changes
}

pub(crate) fn almanac(state: &WorldState) -> Almanac {
    Almanac {
        notes: world_core::EntityId::new(1001),
        first: 1010,
        room: 20,
        period: crate::persistence::WORLD_DAY_TICKS,
        year: YEAR_DAYS,
        seasons: ["Spring", "Summer", "Autumn", "Winter"],
        festivals: crate::years::festivals(state, FESTIVALS),
        people: crate::story::people_in,
        festive,
        grown,
        held,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dated day comes round at least every two weeks, all year.
    #[test]
    fn a_dated_day_at_least_every_fourteen_days() {
        let mut days = FESTIVALS
            .iter()
            .map(|festival| festival.day)
            .collect::<Vec<_>>();
        days.sort_unstable();
        let wrap = days[0] + YEAR_DAYS - days[days.len() - 1];
        let widest = days
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .chain([wrap])
            .max()
            .unwrap();
        assert!(widest <= 14, "{widest} days without a dated day");
    }
}
