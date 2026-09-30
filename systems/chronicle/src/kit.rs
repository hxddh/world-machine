//! Key beats chosen from a World's history and told as three-panel moments:
//! weddings, births, comings of age, farewells and deaths, storms weathered,
//! works opened, a festival's best night, and newcomers settling in.
//!
//! Which Events are beats, and how each is cast, placed and captioned, is
//! the same in every World; a Pack gives its own words (a [`Voice`] for each
//! kind of moment), its people, and its own turning years through
//! [`ChroniclePack`]. Everything is read from the recorded Events alone, so the
//! same history always gives the same moments.

use crate::{fill, text, Beat, Teller, Year};
use std::collections::{BTreeMap, BTreeSet};
use world_core::{EntityId, Event, EventId, StateChange, Value, World};
use world_projection::{
    Almanac, BookEntry, MarkShape, Moment, MomentKind, Mood, Named, Prop, SelectionId,
};

/// The words of each kind of moment: its title, and the lines before and
/// after the moment itself, a few of each, one picked by the Event. Each
/// names who and what, from what the Event recorded: `{a}` and `{b}` (the
/// first two people drawn), `{place}`, `{season}`, and per kind `{trade}`,
/// `{festival}`, `{work}`, `{age}` or `{count}`.
pub struct Voice {
    pub title: &'static str,
    pub before: &'static [&'static str],
    pub after: &'static [&'static str],
    pub moods: [Mood; 3],
    /// What each panel shows besides its people, whatever its words.
    pub props: [&'static [Prop]; 3],
}

/// A Pack's voice for every kind of moment the kit chooses itself.
pub struct Voices {
    pub wedding: &'static Voice,
    pub couple: &'static Voice,
    pub party: &'static Voice,
    pub birth: &'static Voice,
    pub grown: &'static Voice,
    pub retired: &'static Voice,
    pub farewell: &'static Voice,
    pub death: &'static Voice,
    pub storm: &'static Voice,
    pub work: &'static Voice,
    pub festival: &'static Voice,
    pub arrival: &'static Voice,
}

/// What a Pack tells the kit about its World's moments.
pub trait ChroniclePack {
    /// Where a moment happens when the Event puts it nowhere else.
    fn home(&self) -> EntityId;
    /// Where a couple's party is held.
    fn party_place(&self) -> EntityId;
    /// World time in a day, days in a season and in a year.
    fn day(&self) -> u64;
    fn season_days(&self) -> u64;
    fn year_days(&self) -> u64;
    /// Words a caption can say that a panel shows.
    fn prop_words(&self) -> &'static [(&'static str, Prop)];
    fn voices(&self) -> &Voices;
    /// Whether an entity is one of the World's people.
    fn is_person(&self, world: &World, id: EntityId) -> bool;
    /// Whether a component joining two people is a marriage.
    fn is_married_key(&self, key: &str) -> bool;
    /// Whether a component is someone's trade.
    fn is_trade_key(&self, key: &str) -> bool;
    /// The Pack's own telling of an Event, before `lives`' and the
    /// Event's own recorded words.
    fn told(&self, world: &World, event: &Event) -> Option<String>;
    /// The kinds of Event that are a storm weathered.
    fn storm_kinds(&self) -> &[&'static str];
    /// The kinds of Event that happen between people: questions answered
    /// or lapsed, and anyone arriving.
    fn meeting_kinds(&self) -> &[&'static str];
    /// What a turning year was, if it is a moment.
    fn turning_year(
        &self,
        world: &World,
        event: &Event,
        cast: &[EntityId],
    ) -> Option<(MomentKind, &'static Voice)>;
    /// Every work opened: the Event that put its last part in place, when,
    /// and the work's name, in the order the works were finished.
    fn works_opened(&self, world: &World) -> Vec<(EventId, u64, String)>;
    /// How someone leaves the World or comes to it, drawn at a farewell
    /// and an arrival besides what the captions name.
    fn transport(&self, _world: &World) -> Option<Prop> {
        None
    }
}

/// Who a moment is about: its subject, whom it concerned, and whom it
/// joined them to, the people only.
pub fn people(pack: &impl ChroniclePack, world: &World, event: &Event) -> Vec<EntityId> {
    let mut seen = BTreeSet::new();
    crate::entity(event, "who")
        .into_iter()
        .chain(event.actor)
        .chain(event.targets.iter().copied())
        .chain(event.changes.iter().filter_map(|change| match change {
            StateChange::SetComponent {
                key,
                value: Value::Entity(other),
                ..
            } if pack.is_married_key(key) => Some(*other),
            StateChange::SetComponent {
                entity,
                key,
                value: Value::Bool(true),
            } if key == lives::SETTLED => Some(*entity),
            StateChange::CreateEntity(entity) => Some(entity.id),
            _ => None,
        }))
        .filter(|id| pack.is_person(world, *id) && seen.insert(*id))
        .collect()
}

fn told(pack: &impl ChroniclePack, world: &World, event: &Event) -> Option<String> {
    pack.told(world, event)
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

/// One beat, told in `voice`: `None` when its lines cannot be told.
#[allow(clippy::too_many_arguments)]
pub fn beat<'a>(
    pack: &impl ChroniclePack,
    world: &World,
    event: &'a Event,
    kind: MomentKind,
    voice: &Voice,
    cast: Vec<EntityId>,
    place: EntityId,
    slots: &[(&str, &str)],
) -> Option<Beat<'a>> {
    let everyone = crate::Folk::new(world, everyone(pack, world));
    beat_among(
        pack, world, event, kind, voice, cast, place, slots, &everyone,
    )
}

/// Everyone in the World, in id order: whom a caption may name.
fn everyone(pack: &impl ChroniclePack, world: &World) -> Vec<EntityId> {
    world
        .state()
        .entities()
        .map(|entity| entity.id)
        .filter(|id| pack.is_person(world, *id))
        .collect()
}

/// [`beat`], with the World's people found once for all of its beats.
#[allow(clippy::too_many_arguments)]
fn beat_among<'a>(
    pack: &impl ChroniclePack,
    world: &World,
    event: &'a Event,
    kind: MomentKind,
    voice: &Voice,
    mut cast: Vec<EntityId>,
    place: EntityId,
    slots: &[(&str, &str)],
    everyone: &crate::Folk,
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
    let season = crate::season_at(event.world_time, pack.day(), pack.year_days());
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
                } if Some(entity) == cast.first() && pack.is_trade_key(key) && job != "retired" => {
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
        None => format!("{}.", told(pack, world, event)?.trim_end_matches('.')),
    };
    let captions = [
        line(voice.before, seed)?,
        moment,
        line(voice.after, seed / 3)?,
    ];
    // Whoever the captions name is drawn too, up to four.
    let cast = crate::named_among(everyone, &captions, cast, MOST_CAST);
    let mut props =
        [0, 1, 2].map(|at| crate::props_for(&captions[at], pack.prop_words(), voice.props[at]));
    // Someone leaving goes, and someone new comes, the way people travel
    // here.
    if let Some(transport) = pack.transport(world) {
        if voice.title == pack.voices().farewell.title {
            props[1].push(transport);
        } else if voice.title == pack.voices().arrival.title {
            props[0].push(transport);
        }
    }
    Some(Beat {
        event,
        kind,
        title: capitalized(&fill(voice.title, &all)),
        cast,
        place: Some(place),
        props,
        captions,
        moods: voice.moods.map(Some),
    })
}

/// Where a moment happens: where the Event itself put whom it is about,
/// or the Pack's home. Read from the Event, never from how things stand
/// now, so a moment is told the same way however long ago it was.
pub fn where_of(pack: &impl ChroniclePack, event: &Event, who: Option<&EntityId>) -> EntityId {
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
        .unwrap_or(pack.home())
}

/// Whoever an Event brought into the World.
pub fn creates_someone(
    pack: &impl ChroniclePack,
    world: &World,
    event: &Event,
) -> Option<EntityId> {
    event.changes.iter().find_map(|change| match change {
        StateChange::CreateEntity(entity) if pack.is_person(world, entity.id) => Some(entity.id),
        _ => None,
    })
}

/// Whether an Event saw someone leave for good.
pub fn goes(event: &Event) -> bool {
    event.changes.iter().any(|change| {
        matches!(change, StateChange::SetComponent { key, value: Value::Bool(true), .. }
            if key == lives::GONE)
    })
}

/// Whether an Event married two people.
pub fn married(pack: &impl ChroniclePack, event: &Event) -> bool {
    event.changes.iter().any(|change| {
        matches!(change, StateChange::SetComponent { key, value: Value::Entity(_), .. }
            if pack.is_married_key(key))
    })
}

/// Every key beat of the World's history.
pub fn beats<'a>(pack: &impl ChroniclePack, world: &'a World) -> Vec<Beat<'a>> {
    let voices = pack.voices();
    let everyone = crate::Folk::new(world, everyone(pack, world));
    let mut beats = Vec::new();
    // Storms weathered.
    for event in world.events_of_kind(pack.storm_kinds()) {
        let cast = people(pack, world, event);
        beats.extend(beat_among(
            pack,
            world,
            event,
            MomentKind::Storm,
            voices.storm,
            cast,
            pack.home(),
            &[],
            &everyone,
        ));
    }
    // Lives: births, comings of age, retirements and deaths.
    for event in world.events_of_kind(&["born", "came_of_age", "retired", "died"]) {
        let cast = people(pack, world, event);
        let place = where_of(pack, event, cast.first());
        let (kind, voice) = match event.kind.as_str() {
            "born" => (MomentKind::Birth, voices.birth),
            "came_of_age" => (MomentKind::ComingOfAge, voices.grown),
            "retired" => (MomentKind::Farewell, voices.retired),
            _ => (MomentKind::Death, voices.death),
        };
        beats.extend(beat_among(
            pack,
            world,
            event,
            kind,
            voice,
            cast,
            place,
            &[],
            &everyone,
        ));
    }
    // The World's turning years.
    for event in world.events_of_kind(&["year_turned"]) {
        let cast = people(pack, world, event);
        let place = where_of(pack, event, cast.first());
        let Some((kind, voice)) = pack.turning_year(world, event, &cast) else {
            continue;
        };
        beats.extend(beat_among(
            pack,
            world,
            event,
            kind,
            voice,
            cast,
            place,
            &[],
            &everyone,
        ));
    }
    // Between people: a couple's party, a newcomer staying, someone leaving.
    for event in world.events_of_kind(pack.meeting_kinds()) {
        let cast = people(pack, world, event);
        let answer = text(event, "answer").unwrap_or_default();
        let (kind, voice, place) = match text(event, "kind") {
            Some("sweet") if answer == "ask" => (
                MomentKind::Other,
                voices.couple,
                where_of(pack, event, cast.first()),
            ),
            Some("party") if matches!(answer, "party" | "potluck") => {
                (MomentKind::Other, voices.party, pack.party_place())
            }
            Some("leaving") if answer != "stay" && goes(event) => {
                (MomentKind::Farewell, voices.farewell, pack.home())
            }
            _ => match creates_someone(pack, world, event) {
                Some(newcomer) => {
                    let mut cast = cast.clone();
                    cast.retain(|id| *id != newcomer);
                    cast.insert(0, newcomer);
                    beats.extend(beat_among(
                        pack,
                        world,
                        event,
                        MomentKind::Other,
                        voices.arrival,
                        cast,
                        pack.home(),
                        &[],
                        &everyone,
                    ));
                    continue;
                }
                None => continue,
            },
        };
        beats.extend(beat_among(
            pack,
            world,
            event,
            kind,
            voice,
            cast,
            place,
            &[],
            &everyone,
        ));
    }
    // Works opened: the Event that put each one's last part in place.
    for (id, _, label) in pack.works_opened(world) {
        let Some(event) = world.event(id) else {
            continue;
        };
        let cast = people(pack, world, event);
        beats.extend(beat_among(
            pack,
            world,
            event,
            MomentKind::WorkOpened,
            voices.work,
            cast,
            pack.home(),
            &[("work", &world_projection::lowered(&label, &[]))],
            &everyone,
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
        let season = event.world_time / pack.day() / pack.season_days();
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
        let place = event.targets.first().copied().unwrap_or(pack.home());
        let cast = people(pack, world, event);
        beats.extend(beat_among(
            pack,
            world,
            event,
            MomentKind::Festival,
            voices.festival,
            cast,
            place,
            &[("festival", &name)],
            &everyone,
        ));
    }
    beats
}

/// Every moment of the World's history, at most one a day, oldest first.
///
/// Worked out once for as long as the World stands as it does (a Pack's
/// moments are told by its type alone), since a snapshot, its book and its
/// almanac all read them.
pub fn moments_of<P: ChroniclePack + 'static>(pack: &P, world: &World) -> Vec<Moment> {
    world
        .as_it_stands(|| KeptMoments::<P>(all_moments(pack, world), std::marker::PhantomData))
        .0
        .clone()
}

/// A Pack's moments of a World, as it stands.
struct KeptMoments<P>(Vec<Moment>, std::marker::PhantomData<fn() -> P>);

fn all_moments(pack: &impl ChroniclePack, world: &World) -> Vec<Moment> {
    let mut seen = BTreeSet::new();
    let beats = beats(pack, world)
        .into_iter()
        .filter(|beat| seen.insert(beat.event.id))
        .collect();
    crate::moments(beats, pack.day())
}

/// One moment, by its id.
pub fn moment<P: ChroniclePack + 'static>(pack: &P, world: &World, id: &str) -> Option<Moment> {
    moments_of(pack, world)
        .into_iter()
        .find(|moment| moment.id == id)
}

/// The book's page for each moment, with its cast for faces.
pub fn book_entries(moments: &[Moment]) -> Vec<BookEntry> {
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

/// Which of the World's years a moment falls in, counting the first as 1.
pub fn year_of(pack: &impl ChroniclePack, world_time: u64) -> u64 {
    world_time / pack.day() / pack.year_days() + 1
}

/// What was built in `year`: the works finished, each with whoever asked
/// for it, and what the player made by hand, each as itself.
fn built(pack: &impl ChroniclePack, world: &World, year: Year) -> Vec<Named> {
    let mut built = Vec::new();
    for (id, at, label) in pack.works_opened(world) {
        let (true, Some(event)) = (year.holds(at), world.event(id)) else {
            continue;
        };
        built.push(Named {
            name: label,
            who: SelectionId::Entity(event.actor.unwrap_or(pack.home())),
        });
    }
    for event in crate::in_year(world, &["built_by_hand"], year) {
        let Some(thing) = event.targets.first() else {
            continue;
        };
        let Some(entity) = world.state().entity(*thing) else {
            continue;
        };
        built.push(Named {
            name: world_projection::entity_title(entity),
            who: SelectionId::Entity(*thing),
        });
    }
    built
}

/// The almanac of `year`, once it has ended, under `title`.
pub fn almanac_page(
    pack: &impl ChroniclePack,
    teller: &impl Teller,
    year: u32,
    title: String,
    moments: &[Moment],
) -> Option<Almanac> {
    let world = teller.world();
    if year == 0 || u64::from(year) >= year_of(pack, world.world_time()) {
        return None;
    }
    let span = Year::of(year, pack.year_days(), pack.day());
    Some(crate::almanac(
        teller,
        span,
        title,
        &everyone(pack, world),
        built(pack, world, span),
        moments,
    ))
}

/// Every year whose almanac can be asked for now: each one that has
/// ended, oldest first. A year's page is kept from its New Year on.
pub fn almanac_years(pack: &impl ChroniclePack, world: &World) -> Vec<u32> {
    let now = year_of(pack, world.world_time());
    (1..now)
        .filter_map(|year| u32::try_from(year).ok())
        .collect()
}

/// On New Year's day, the year just ended, whose page is delivered.
pub fn year_just_ended(pack: &impl ChroniclePack, world: &World) -> Option<u32> {
    let day = world.world_time() / pack.day();
    let year = year_of(pack, world.world_time());
    if year < 2 || !day.is_multiple_of(pack.year_days()) {
        return None;
    }
    u32::try_from(year - 1).ok()
}

fn entry(
    shelf: &str,
    name: String,
    found: bool,
    shape: Option<MarkShape>,
    hint: String,
) -> BookEntry {
    BookEntry {
        shelf: shelf.into(),
        name,
        found,
        shape,
        hint,
        moment: None,
        cast: Vec::new(),
    }
}

/// The book of everything to find: keepsakes, letters, firsts, people met,
/// things made by hand and works built, and festival days, found or still
/// a silhouette, in shelf order. The Pack gives its people (`cast`), what
/// its hands can make (`kit`), its works (`goals`), its calendar
/// (`almanac`) and how a fixture's shape is drawn (`shape`).
pub fn book(
    world: &World,
    cast: &lives::Cast,
    kit: &hands::Kit,
    goals: &[world_projection::Goal],
    almanac: &calendar::Almanac,
    shape_of: impl Fn(&str) -> MarkShape,
) -> Vec<BookEntry> {
    let mut book = Vec::new();
    // Keepsakes: whatever anyone living here could give, and what a garden
    // grows.
    let kept = lives::keepsakes(world)
        .into_iter()
        .map(|kept| kept.what)
        .collect::<BTreeSet<_>>();
    let possible =
        lives::possible_keepsakes(world, cast)
            .into_iter()
            .chain(hands::PRODUCE.iter().map(|what| {
                (
                    (*what).to_string(),
                    "Grown in a garden you planted".to_string(),
                )
            }));
    for (what, hint) in possible {
        let found = kept.contains(&what);
        book.push(entry(
            "Keepsakes",
            what,
            found,
            Some(MarkShape::Parcel),
            hint,
        ));
    }
    // Letters: one from everyone who lives here.
    for (name, wrote) in lives::letter_writers(world, cast) {
        book.push(entry(
            "Letters",
            name,
            wrote,
            Some(MarkShape::Parcel),
            "Comes on a quiet day".into(),
        ));
    }
    // Firsts: each small first of a quiet day, once it has come.
    for first in lives::firsts(world) {
        book.push(entry(
            "Firsts",
            first,
            true,
            None,
            "Comes on a quiet day".into(),
        ));
    }
    // People: everyone here, and strangers who might come to stay.
    let met = lives::met(world);
    for (name, person) in lives::people_to_meet(world, cast) {
        let found = person.is_some_and(|person| met.contains(&person));
        let hint = if person.is_some() {
            "Say hello".to_string()
        } else {
            "Might come to stay".to_string()
        };
        let mut met = entry("People", name, found, None, hint);
        // Someone met is drawn with their own face.
        met.cast = person
            .filter(|_| found)
            .map(SelectionId::Entity)
            .into_iter()
            .collect();
        book.push(met);
    }
    // Things made with the player's own hands.
    let made = hands::ever_made(world);
    for thing in kit.things {
        let found = made.contains(thing.id);
        let shape = thing.stages.last().map_or(thing.shape, |(_, shape)| *shape);
        book.push(entry(
            "Made",
            thing.name.into(),
            found,
            Some(shape_of(shape)),
            format!("{} it with your own hands", thing.verb.word()),
        ));
    }
    // What the place is building towards, once each is finished.
    for goal in goals {
        let found = goal.done >= goal.parts;
        book.push(entry(
            "Made",
            goal.label.clone(),
            found,
            Some(goal.shape),
            "Built by the whole place, a part at a time".into(),
        ));
    }
    // Festival days, once each has been held.
    let held = calendar::held(world);
    for festival in almanac.festivals {
        let shape = if festival.shape.is_empty() {
            MarkShape::Flag
        } else {
            shape_of(festival.shape)
        };
        book.push(entry(
            "Days",
            festival.name.into(),
            held.contains(festival.id),
            Some(shape),
            "Comes round once a year".into(),
        ));
    }
    book
}

/// Kinds that concern someone without always changing them: a legend
/// names whom they concern.
pub const NAMING: &[&str] = &[
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

/// Kinds of every World's everyday round and bookkeeping, never a line of
/// a legend.
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
];

/// Whether a kind is the everyday round, never a line: every World's, or
/// one of the Pack's own `extra` kinds.
pub fn everyday(kind: &str, extra: &[&str]) -> bool {
    EVERYDAY.contains(&kind) || extra.contains(&kind)
}

/// Whether an Event was done by or to `who`.
pub fn involves(event: &Event, who: EntityId) -> bool {
    event.actor == Some(who) || event.targets.contains(&who)
}

/// The first time a festival was held, for a place's legend.
pub fn first_held(world: &World, event: &Event) -> bool {
    let festival = text(event, "festival");
    world_projection::latest_before(world, &["festival_held"], event.id, |held| {
        text(held, "festival") == festival
    })
    .is_none()
}

/// The legend of a person, place or thing, its life filled in.
pub fn filled_legend(
    teller: &impl Teller,
    subject: SelectionId,
) -> Option<world_projection::Legend> {
    crate::legend(teller, subject).map(|legend| crate::filled_life(teller, legend))
}
