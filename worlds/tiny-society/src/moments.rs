//! The harbour's key beats as three-panel moments: weddings, births,
//! comings of age, farewells and deaths, storms weathered, works opened,
//! a festival's best night, and newcomers settling in. Each is read from
//! the recorded Events alone, so the same history always gives the same
//! moments; captions are the harbour's own voice.

use crate::{HARBOR, PUB};
use chronicle::kit::{self, creates_someone, goes, ChroniclePack, Voice, Voices};
use chronicle::text;
use world_core::{EntityId, Event, EventId, World};
use world_projection::{BookEntry, Moment, MomentKind, Mood, Prop};

use Mood::{Content, Happy, Sad, Thinking};

/// The days of a season, for each season's best festival night.
const SEASON_DAYS: u64 = crate::story::SEASON_DAYS;
const DAY: u64 = crate::persistence::WORLD_DAY_TICKS;

/// Words a caption can say that a panel shows: a caption that names the
/// ferry has the ferry drawn in it.
const PROP_WORDS: &[(&str, Prop)] = &[
    ("ferry", Prop::Ferry),
    ("bag", Prop::Suitcase),
    ("lamp stayed lit", Prop::Lamp),
    ("a light in the window", Prop::Lamp),
    ("chapel", Prop::Bouquet),
    ("danced", Prop::Bunting),
    ("tables", Prop::Table),
    ("storm", Prop::Rain),
    ("the wind got up", Prop::Rain),
    ("went dark", Prop::Rain),
    ("sit on the quay", Prop::Bench),
];

const WEDDING: Voice = Voice {
    title: "{a} and {b}'s wedding",
    before: &[
        "{a} and {b} had been walking out since {season}.",
        "{a} asked {b} on the quay, and {b} said yes before the question was done.",
    ],
    after: &[
        "{a} and {b} came out of the old chapel arm in arm.",
        "{a} and {b} danced on the quay till the lamps went out.",
    ],
    moods: [Content, Happy, Happy],
    props: [&[], &[Prop::Bunting, Prop::Bouquet], &[]],
};

const COUPLE: Voice = Voice {
    title: "{a} and {b} walk out",
    before: &[
        "{a} kept finding reasons to be wherever {b} was.",
        "{a} had been sweet on {b} since {season}.",
    ],
    after: &[
        "By the end of {season}, {a} and {b} were seen everywhere together.",
        "{b} smiled for a week, and {a} for longer.",
    ],
    moods: [Thinking, Happy, Happy],
    props: [&[], &[], &[]],
};

const SETTLED: Voice = Voice {
    title: "{a} settles for good",
    before: &[
        "{a} had been sleeping on borrowed beds since {season}.",
        "{a} had kept a bag packed by the door since {season}.",
    ],
    after: &[
        "By the end of {season}, all {count} of them had a door of their own.",
        "{a} unpacked the bag at last, and so did the others.",
    ],
    moods: [Thinking, Happy, Content],
    props: [&[Prop::Suitcase], &[], &[]],
};

const TEACHER: Voice = Voice {
    title: "{a} starts teaching",
    before: &[
        "{a} had been helping at {place} since {season}.",
        "{a} had been hearing the little ones read since {season}.",
    ],
    after: &[
        "By the end of {season}, {a} had the little ones reading.",
        "{a} walked home from {place} with chalk on every sleeve.",
    ],
    moods: [Thinking, Happy, Content],
    props: [&[], &[], &[]],
};

const PARTY: Voice = Voice {
    title: "{a} and {b}'s party",
    before: &[
        "{a} and {b} asked the whole harbour to {place}.",
        "{a} and {b} carried tables into {place} all afternoon.",
    ],
    after: &[
        "{a} and {b} danced at {place} till the lamps went out.",
        "Nobody left {place} before {a} and {b} did.",
    ],
    moods: [Content, Happy, Happy],
    props: [
        &[Prop::Table],
        &[Prop::Bunting, Prop::Table],
        &[Prop::Bunting],
    ],
};

const BIRTH: Voice = Voice {
    title: "{a} is born",
    before: &[
        "{b} waited all night for news of {a}.",
        "A lamp stayed lit in {b}'s window until morning.",
    ],
    after: &[
        "{a} slept through the whole harbour's visit.",
        "{b} carried {a} along the quay for everyone to see.",
    ],
    moods: [Thinking, Happy, Happy],
    props: [&[], &[Prop::Cradle], &[Prop::Cradle]],
};

const GROWN: Voice = Voice {
    title: "{a} comes of age",
    before: &[
        "{a} grew up running along the quay.",
        "{a} was a child at {place} once.",
    ],
    after: &[
        "{a} went to work as a {trade} the next morning.",
        "{a} came home from the first day as a {trade} too tired to talk.",
        "{a} went to work at {place} the next morning, grown at last.",
    ],
    moods: [Content, Happy, Content],
    props: [&[], &[], &[Prop::Tools]],
};

const RETIRED: Voice = Voice {
    title: "{a} hands it on",
    before: &[
        "{a} had worked as a {trade} every day for years.",
        "{a} knew every job at {place} by heart.",
    ],
    after: &[
        "{a} took the long way home along the quay, for once.",
        "{a} had time at last to sit on the quay in the {season} sun.",
    ],
    moods: [Thinking, Content, Happy],
    props: [&[Prop::Tools], &[], &[Prop::Bench]],
};

const FAREWELL: Voice = Voice {
    title: "{a}'s farewell",
    before: &[
        "{a} looked round {place} one more time.",
        "{a} packed one bag in {season} and left the rest behind.",
    ],
    after: &[
        "The ferry took {a} away in the {season} light.",
        "{a}'s window at {place} stayed dark that night.",
    ],
    moods: [Thinking, Sad, Content],
    props: [&[Prop::Suitcase], &[Prop::Ferry], &[]],
};

const DEATH: Voice = Voice {
    title: "In memory of {a}",
    before: &[
        "{a} lived to {age}, most of it by the harbour.",
        "Everyone knew {a}, and {a} knew everyone, for {age} years.",
    ],
    after: &[
        "{b} kept a light in the window for {a}.",
        "The boats stayed in the day after {a} died.",
    ],
    moods: [Content, Sad, Sad],
    props: [&[], &[Prop::Wreath], &[]],
};

const STORM: Voice = Voice {
    title: "The storm",
    before: &[
        "The {season} sky over the harbour went dark, and {a} watched it come.",
        "The wind got up in the afternoon, and {a} ran to warn the boats.",
        "The {season} sky over the harbour went dark.",
    ],
    after: &[
        "By morning {a} was out counting what the storm had left.",
        "By morning the harbour was out counting what the storm had left.",
        "{a} helped sweep the quay clear before breakfast.",
    ],
    moods: [Thinking, Thinking, Content],
    props: [&[Prop::Rain], &[Prop::Rain], &[]],
};

const WORK: Voice = Voice {
    title: "{work}",
    before: &[
        "{a} had been asking for {work} since {season}.",
        "{a} put in the first part of {work} with their own hands.",
    ],
    after: &[
        "{a} was the first to try out {work}.",
        "{a} walked round {work} twice, just to look.",
    ],
    moods: [Thinking, Happy, Happy],
    props: [&[Prop::Scaffold], &[Prop::Ribbon], &[]],
};

const FESTIVAL: Voice = Voice {
    title: "{festival}",
    before: &[
        "{a} spent all day getting {place} ready for {festival}.",
        "{a} was the first down to {place} for {festival}.",
    ],
    after: &[
        "{a} was the last to leave {place} after {festival}.",
        "{a} talked about {festival} for days.",
    ],
    moods: [Content, Happy, Happy],
    props: [&[Prop::Bunting], &[Prop::Bunting], &[]],
};

const ARRIVAL: Voice = Voice {
    title: "{a} comes to stay",
    before: &[
        "{a} came in on the ferry in {season}, with one bag.",
        "{a} was asking round the harbour for a room in {season}.",
    ],
    after: &[
        "By the end of {season}, everyone in the harbour knew {a}.",
        "{a} had a key to a door in the harbour before the week was out.",
    ],
    moods: [Thinking, Happy, Content],
    props: [&[], &[Prop::Suitcase], &[]],
};

const VOICES: Voices = Voices {
    wedding: &WEDDING,
    couple: &COUPLE,
    party: &PARTY,
    birth: &BIRTH,
    grown: &GROWN,
    retired: &RETIRED,
    farewell: &FAREWELL,
    death: &DEATH,
    storm: &STORM,
    work: &WORK,
    festival: &FESTIVAL,
    arrival: &ARRIVAL,
};

/// The harbour, as the moment kit reads it.
pub(crate) struct Harbour;

impl ChroniclePack for Harbour {
    fn home(&self) -> EntityId {
        HARBOR
    }
    fn party_place(&self) -> EntityId {
        PUB
    }
    fn day(&self) -> u64 {
        DAY
    }
    fn season_days(&self) -> u64 {
        SEASON_DAYS
    }
    fn year_days(&self) -> u64 {
        crate::almanac::YEAR_DAYS
    }
    fn prop_words(&self) -> &'static [(&'static str, Prop)] {
        PROP_WORDS
    }
    fn voices(&self) -> &Voices {
        &VOICES
    }
    fn is_person(&self, world: &World, id: EntityId) -> bool {
        world
            .state()
            .entity(id)
            .is_some_and(|entity| entity.kind == "resident")
    }
    fn is_married_key(&self, key: &str) -> bool {
        key == "years.married"
    }
    fn is_trade_key(&self, key: &str) -> bool {
        key == society_basic::JOB
    }
    fn told(&self, world: &World, event: &Event) -> Option<String> {
        crate::projection::narrated_title(world, event)
    }
    fn storm_kinds(&self) -> &[&'static str] {
        crate::legends::storm_kinds()
    }
    fn meeting_kinds(&self) -> &[&'static str] {
        &[
            "situation_answered",
            "situation_lapsed",
            "ivo_arrived",
            "ada_arrived",
        ]
    }
    /// The harbour's turning years.
    fn turning_year(
        &self,
        world: &World,
        event: &Event,
        cast: &[EntityId],
    ) -> Option<(MomentKind, &'static Voice)> {
        let the_beat = text(event, "beat").unwrap_or_default();
        Some(if the_beat == "grown" {
            (MomentKind::ComingOfAge, &GROWN)
        } else if matches!(the_beat, "hands" | "quay") {
            (MomentKind::Farewell, &RETIRED)
        } else if the_beat.starts_with("later_") && cast.len() >= 2 && !goes(event) {
            if creates_someone(self, world, event).is_some() {
                (MomentKind::Other, &ARRIVAL)
            } else {
                (MomentKind::Wedding, &WEDDING)
            }
        } else if goes(event) {
            (MomentKind::Farewell, &FAREWELL)
        } else if creates_someone(self, world, event).is_some() {
            (MomentKind::Other, &ARRIVAL)
        } else if the_beat == "settled" {
            (MomentKind::Other, &SETTLED)
        } else if the_beat == "teacher" {
            (MomentKind::Other, &TEACHER)
        } else {
            return None;
        })
    }
    fn works_opened(&self, world: &World) -> Vec<(EventId, u64, String)> {
        storylets::finished_goals(world, crate::story::deck())
            .into_iter()
            .filter_map(|finished| {
                let (id, at) = finished.event?;
                Some((id, at, crate::story::goal_label(finished.goal)?.to_string()))
            })
            .collect()
    }
}

/// Every moment of the harbour's history, at most one a day, oldest
/// first.
pub(crate) fn moments(world: &World) -> Vec<Moment> {
    kit::moments_of(&Harbour, world)
}

/// One moment, by its id.
pub(crate) fn moment(world: &World, id: &str) -> Option<Moment> {
    kit::moment(&Harbour, world, id)
}

/// The book's page for each moment, with its cast for faces.
pub(crate) fn book_entries(moments: &[Moment]) -> Vec<BookEntry> {
    kit::book_entries(moments)
}
