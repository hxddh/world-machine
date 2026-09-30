//! The harbour's key beats as three-panel moments: weddings, births,
//! comings of age, farewells and deaths, storms weathered, works opened,
//! a festival's best night, and newcomers settling in. Each is read from
//! the recorded Events alone, so the same history always gives the same
//! moments; captions are the harbour's own voice.

use crate::{HARBOR, PUB};
use chronicle::{fill, text, Beat};
use std::collections::{BTreeMap, BTreeSet};
use world_core::{EntityId, Event, StateChange, Value, World};
use world_projection::{BookEntry, MarkShape, Moment, MomentKind, Mood, Prop, SelectionId};

use Mood::{Content, Happy, Sad, Thinking};

/// The days of a season, for each season's best festival night.
const SEASON_DAYS: u64 = crate::story::SEASON_DAYS;
const DAY: u64 = crate::persistence::WORLD_DAY_TICKS;

/// The words of each kind of moment: its title, and the lines before and
/// after the moment itself, a few of each, one picked by the Event. Each
/// names who and what, from what the Event recorded: `{a}` and `{b}` (the
/// first two people drawn), `{place}`, `{season}`, and per kind `{trade}`,
/// `{festival}`, `{work}`, `{age}` or `{count}`.
struct Voice {
    title: &'static str,
    before: &'static [&'static str],
    after: &'static [&'static str],
    moods: [Mood; 3],
    /// What each panel shows besides its people, whatever its words.
    props: [&'static [Prop]; 3],
}

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

fn is_person(world: &World, id: EntityId) -> bool {
    world
        .state()
        .entity(id)
        .is_some_and(|entity| entity.kind == "resident")
}

/// Who a moment is about: its subject, whom it concerned, and whom it
/// joined them to, the people only.
fn people(world: &World, event: &Event) -> Vec<EntityId> {
    let mut seen = BTreeSet::new();
    chronicle::entity(event, "who")
        .into_iter()
        .chain(event.actor)
        .chain(event.targets.iter().copied())
        .chain(event.changes.iter().filter_map(|change| match change {
            StateChange::SetComponent {
                key,
                value: Value::Entity(other),
                ..
            } if key == "years.married" => Some(*other),
            StateChange::SetComponent {
                entity,
                key,
                value: Value::Bool(true),
            } if key == lives::SETTLED => Some(*entity),
            StateChange::CreateEntity(entity) => Some(entity.id),
            _ => None,
        }))
        .filter(|id| is_person(world, *id) && seen.insert(*id))
        .collect()
}

fn told(world: &World, event: &Event) -> Option<String> {
    crate::projection::narrated_title(world, event)
        .or_else(|| lives::told(event))
        .or_else(|| text(event, "told").map(str::to_string))
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Everyone a moment draws: those its captions name, at most four.
const MOST_CAST: usize = 4;

fn beat<'a>(
    world: &World,
    event: &'a Event,
    kind: MomentKind,
    voice: &Voice,
    mut cast: Vec<EntityId>,
    place: EntityId,
    slots: &[(&str, &str)],
) -> Option<Beat<'a>> {
    let state = world.state();
    // Someone without a name cannot be told of.
    cast.retain(|id| !lives::first_name(state, *id).is_empty());
    let count = cast.len().to_string();
    cast.truncate(MOST_CAST);
    let name = |at: usize| {
        cast.get(at)
            .map(|id| lives::first_name(state, *id))
            .unwrap_or_default()
    };
    let (a, b) = (name(0), name(1));
    let season = chronicle::season_at(event.world_time, DAY, crate::almanac::YEAR_DAYS);
    let place_name = state
        .entity(place)
        .map(world_projection::entity_title)
        .unwrap_or_default();
    let trade = text(event, "trade")
        .map(str::to_string)
        .or_else(|| {
            event.changes.iter().find_map(|change| match change {
                StateChange::SetComponent {
                    entity,
                    key,
                    value: Value::Text(job),
                } if Some(entity) == cast.first()
                    && key == society_basic::JOB
                    && job != "retired" =>
                {
                    Some(job.replace('_', " "))
                }
                _ => None,
            })
        })
        .unwrap_or_default();
    let age = match event.payload.get("age") {
        Some(Value::Integer(age)) => age.to_string(),
        _ => String::new(),
    };
    // A name nobody has is left unfilled, so no line is told without it.
    let mut all = [("a", a.as_str()), ("b", b.as_str())]
        .into_iter()
        .filter(|(_, name)| !name.is_empty())
        .collect::<Vec<_>>();
    all.extend([
        ("season", season),
        ("place", place_name.as_str()),
        ("count", count.as_str()),
    ]);
    all.extend_from_slice(slots);
    // A trade is told as "a fisher"; one that would want "an" is left
    // to the lines that do not name it.
    let trade = trade.replace('_', " ");
    if !trade.is_empty() && !trade.starts_with(['a', 'e', 'i', 'o', 'u']) {
        all.push(("trade", &trade));
    }
    if !age.is_empty() {
        all.push(("age", &age));
    }
    // The first of the lines whose every slot can be filled, from the one
    // the Event picks; none, and the moment is told without the panel's
    // own words.
    let line = |options: &[&str], seed: u64| {
        let turn = (0..options.len()).map(|step| options[(seed as usize + step) % options.len()]);
        // Lines that name who first; the others only when nobody can be.
        turn.clone()
            .filter(|option| option.contains("{a}"))
            .chain(turn)
            .map(|option| fill(option, &all))
            .find(|line| !line.contains('{'))
    };
    let seed = event.id.0;
    // A work is shown finished, whatever its last part was.
    let moment = match slots.iter().find(|(slot, _)| *slot == "work") {
        Some((_, work)) => format!("Finished at last: {work}."),
        None => format!("{}.", told(world, event)?.trim_end_matches('.')),
    };
    let captions = [
        line(voice.before, seed)?,
        moment,
        line(voice.after, seed / 3)?,
    ];
    // Whoever the captions name is drawn too, up to four.
    let everyone = state
        .entities()
        .map(|entity| entity.id)
        .filter(|id| is_person(world, *id))
        .collect::<Vec<_>>();
    let cast = chronicle::named_in(world, &captions, cast, everyone, MOST_CAST);
    Some(Beat {
        event,
        kind,
        title: capitalized(&fill(voice.title, &all)),
        cast,
        place: Some(place),
        props: [0, 1, 2].map(|at| chronicle::props_for(&captions[at], PROP_WORDS, voice.props[at])),
        captions,
        moods: voice.moods.map(Some),
    })
}

/// Where a moment happens: where the Event itself put whom it is about,
/// or the harbour. Read from the Event, never from how things
/// stand now, so a moment is told the same way however long ago it was.
fn where_of(event: &Event, who: Option<&EntityId>) -> EntityId {
    event
        .changes
        .iter()
        .find_map(|change| match change {
            StateChange::SetComponent {
                entity,
                key,
                value: Value::Entity(place),
            } if Some(entity) == who && key == "location" => Some(*place),
            _ => None,
        })
        .unwrap_or(HARBOR)
}

fn creates_someone(world: &World, event: &Event) -> Option<EntityId> {
    event.changes.iter().find_map(|change| match change {
        StateChange::CreateEntity(entity)
            if entity.kind == "resident" && is_person(world, entity.id) =>
        {
            Some(entity.id)
        }
        _ => None,
    })
}

fn goes(event: &Event) -> bool {
    event.changes.iter().any(|change| {
        matches!(change, StateChange::SetComponent { key, value: Value::Bool(true), .. }
            if key == lives::GONE)
    })
}

/// Every key beat of the harbour's history.
fn beats(world: &World) -> Vec<Beat<'_>> {
    let mut beats = Vec::new();
    // Storms weathered.
    for event in world.events_of_kind(crate::legends::storm_kinds()) {
        let cast = people(world, event);
        beats.extend(beat(
            world,
            event,
            MomentKind::Storm,
            &STORM,
            cast,
            HARBOR,
            &[],
        ));
    }
    // Lives: births, comings of age, retirements and deaths.
    for event in world.events_of_kind(&["born", "came_of_age", "retired", "died"]) {
        let cast = people(world, event);
        let place = where_of(event, cast.first());
        let (kind, voice) = match event.kind.as_str() {
            "born" => (MomentKind::Birth, &BIRTH),
            "came_of_age" => (MomentKind::ComingOfAge, &GROWN),
            "retired" => (MomentKind::Farewell, &RETIRED),
            _ => (MomentKind::Death, &DEATH),
        };
        beats.extend(beat(world, event, kind, voice, cast, place, &[]));
    }
    // The harbour's turning years.
    for event in world.events_of_kind(&["year_turned"]) {
        let cast = people(world, event);
        let place = where_of(event, cast.first());
        let the_beat = text(event, "beat").unwrap_or_default();
        let (kind, voice) = if the_beat == "grown" {
            (MomentKind::ComingOfAge, &GROWN)
        } else if matches!(the_beat, "hands" | "quay") {
            (MomentKind::Farewell, &RETIRED)
        } else if the_beat.starts_with("later_") && cast.len() >= 2 && !goes(event) {
            if creates_someone(world, event).is_some() {
                (MomentKind::Other, &ARRIVAL)
            } else {
                (MomentKind::Wedding, &WEDDING)
            }
        } else if goes(event) {
            (MomentKind::Farewell, &FAREWELL)
        } else if creates_someone(world, event).is_some() {
            (MomentKind::Other, &ARRIVAL)
        } else if the_beat == "settled" {
            (MomentKind::Other, &SETTLED)
        } else if the_beat == "teacher" {
            (MomentKind::Other, &TEACHER)
        } else {
            continue;
        };
        beats.extend(beat(world, event, kind, voice, cast, place, &[]));
    }
    // Between people: a couple's party, a newcomer staying, someone leaving.
    for event in world.events_of_kind(&[
        "situation_answered",
        "situation_lapsed",
        "ivo_arrived",
        "ada_arrived",
    ]) {
        let cast = people(world, event);
        let answer = text(event, "answer").unwrap_or_default();
        let (kind, voice, place) = match text(event, "kind") {
            Some("sweet") if answer == "ask" => {
                (MomentKind::Other, &COUPLE, where_of(event, cast.first()))
            }
            Some("party") if matches!(answer, "party" | "potluck") => {
                (MomentKind::Other, &PARTY, PUB)
            }
            Some("leaving") if answer != "stay" && goes(event) => {
                (MomentKind::Farewell, &FAREWELL, HARBOR)
            }
            _ => match creates_someone(world, event) {
                Some(newcomer) => {
                    let mut cast = cast.clone();
                    cast.retain(|id| *id != newcomer);
                    cast.insert(0, newcomer);
                    beats.extend(beat(
                        world,
                        event,
                        MomentKind::Other,
                        &ARRIVAL,
                        cast,
                        HARBOR,
                        &[],
                    ));
                    continue;
                }
                None => continue,
            },
        };
        beats.extend(beat(world, event, kind, voice, cast, place, &[]));
    }
    // Works opened: the Event that put each one's last part in place.
    let deck = crate::story::deck();
    for finished in storylets::finished_goals(world, deck) {
        let Some((id, _)) = finished.event else {
            continue;
        };
        let (Some(event), Some(label)) = (world.event(id), crate::story::goal_label(finished.goal))
        else {
            continue;
        };
        let cast = people(world, event);
        beats.extend(beat(
            world,
            event,
            MomentKind::WorkOpened,
            &WORK,
            cast,
            HARBOR,
            &[("work", &world_projection::lowered(label, &[]))],
        ));
    }
    // A festival's best night: the first time each is held to a full
    // turnout, and any night better attended than every one before it that
    // season. Read from what came before only, so it is never taken back.
    let mut first = BTreeSet::new();
    let mut best: BTreeMap<u64, usize> = BTreeMap::new();
    let mut nights = Vec::new();
    for event in world.events_of_kind(&["festival_held"]) {
        let grand = text(event, "turnout") == Some("grand");
        let season = event.world_time / DAY / SEASON_DAYS;
        let crowd = event.changes.len();
        let record = best.get(&season).is_none_or(|most| crowd > *most);
        if record {
            best.insert(season, crowd);
        }
        if (grand && first.insert(text(event, "festival").unwrap_or_default())) || record {
            nights.push(event);
        }
    }
    for event in nights {
        let name = text(event, "name").unwrap_or_default().to_string();
        let place = event.targets.first().copied().unwrap_or(HARBOR);
        let cast = people(world, event);
        beats.extend(beat(
            world,
            event,
            MomentKind::Festival,
            &FESTIVAL,
            cast,
            place,
            &[("festival", &name)],
        ));
    }
    beats
}

/// Every moment of the harbour's history, at most one a day, oldest
/// first.
pub(crate) fn moments(world: &World) -> Vec<Moment> {
    let mut seen = BTreeSet::new();
    let beats = beats(world)
        .into_iter()
        .filter(|beat| seen.insert(beat.event.id))
        .collect();
    chronicle::moments(beats, crate::persistence::WORLD_DAY_TICKS)
}

/// One moment, by its id.
pub(crate) fn moment(world: &World, id: &str) -> Option<Moment> {
    moments(world).into_iter().find(|moment| moment.id == id)
}

/// The book's page for each moment, with its cast for faces.
pub(crate) fn book_entries(moments: &[Moment]) -> Vec<BookEntry> {
    moments
        .iter()
        .map(|moment| BookEntry {
            shelf: "Moments".into(),
            name: moment.title.clone(),
            found: true,
            shape: Some(MarkShape::Flag),
            hint: String::new(),
            moment: Some(moment.id.clone()),
            cast: moment
                .cast()
                .into_iter()
                .chain(moment.panels[1].place)
                .collect::<Vec<SelectionId>>(),
        })
        .collect()
}
