//! A period of lives: who joins the World, what each person does about
//! the need that presses hardest, with whom and where, what they say
//! about it, and the lines remembered so that nobody says the same thing
//! twice while it is still remembered.

use crate::*;

/// The people the System looks after: the Pack's, less anyone gone.
pub(crate) fn living(world: &World, cast: &Cast) -> Vec<EntityId> {
    (cast.people)(world)
        .into_iter()
        .filter(|id| world.state().entity(*id).is_some() && !gone(world.state(), *id))
        .collect()
}

pub(crate) fn arg_entity(request: &ActionRequest, name: &str) -> Result<EntityId, ActionError> {
    match request.args.get(name) {
        Some(Value::Entity(id)) => Ok(*id),
        Some(Value::Integer(id)) => Ok(EntityId::new(*id as u64)),
        _ => Err(ActionError::Invalid(format!("missing {name}"))),
    }
}

pub(crate) fn arg_text<'a>(request: &'a ActionRequest, name: &str) -> Result<&'a str, ActionError> {
    match request.args.get(name) {
        Some(Value::Text(value)) => Ok(value),
        _ => Err(ActionError::Invalid(format!("missing {name}"))),
    }
}

/// Changes that move integers, keeping track of what the same event has
/// already moved them to.
#[derive(Default)]
pub(crate) struct Moves {
    pub(crate) values: BTreeMap<(EntityId, String), i64>,
    pub(crate) changes: Vec<StateChange>,
}

impl Moves {
    pub(crate) fn add(
        &mut self,
        state: &WorldState,
        entity: EntityId,
        key: &str,
        by: i64,
        min: i64,
        max: i64,
    ) {
        if state.entity(entity).is_none() {
            return;
        }
        let current = *self
            .values
            .entry((entity, key.to_string()))
            .or_insert_with(|| integer(state, entity, key).unwrap_or(0));
        let next = current.saturating_add(by).clamp(min, max);
        self.values.insert((entity, key.to_string()), next);
        self.changes.push(StateChange::SetComponent {
            entity,
            key: key.to_string(),
            value: next.into(),
        });
    }

    pub(crate) fn lack(&mut self, state: &WorldState, person: EntityId, need: Need, by: i64) {
        self.add(state, person, need.key(), by, 0, 100);
    }

    pub(crate) fn opinion(&mut self, state: &WorldState, a: EntityId, b: EntityId, by: i64) {
        if a != b {
            self.add(state, a, &opinion_key(b), by, -100, 100);
        }
    }

    pub(crate) fn regard(&mut self, state: &WorldState, person: EntityId, by: i64) {
        self.add(state, person, REGARD, by, -100, 100);
    }

    /// One more hard time someone was left to face alone, or (with a
    /// negative `by`) some help through one.
    pub(crate) fn unhelped(&mut self, state: &WorldState, person: EntityId, by: i64) {
        self.add(state, person, UNHELPED, by, 0, 20);
    }

    pub(crate) fn set(&mut self, entity: EntityId, key: &str, value: impl Into<Value>) {
        self.changes.push(StateChange::SetComponent {
            entity,
            key: key.to_string(),
            value: value.into(),
        });
    }

    pub(crate) fn unset(&mut self, state: &WorldState, entity: EntityId, key: &str) {
        if state
            .entity(entity)
            .is_some_and(|entity| entity.component(key).is_some())
        {
            self.changes.push(StateChange::RemoveComponent {
                entity,
                key: key.to_string(),
            });
        }
    }
}

/// A fresh start for someone joining the System's life: how short they
/// are of each need, their traits, what they think of everyone.
pub(crate) fn enrolment(
    state: &WorldState,
    cast: &Cast,
    person: EntityId,
    others: &[EntityId],
) -> Moves {
    let mut moves = Moves::default();
    let seed = mix(&[person.0, 7]);
    let inherited = text(state, person, generations::NATURE)
        .filter(|nature| !nature.is_empty())
        .map(str::to_string);
    let traits = (cast.traits)(person)
        .map(|[a, b]| format!("{a} {b}"))
        .or(inherited)
        .unwrap_or_else(|| {
            let a = TRAITS[(seed % TRAITS.len() as u64) as usize];
            let b = TRAITS[((seed / 11 + 3) % TRAITS.len() as u64) as usize];
            if a == b {
                a.to_string()
            } else {
                format!("{a} {b}")
            }
        });
    moves.set(person, TRAITS_KEY, traits);
    for (index, need) in Need::ALL.iter().enumerate() {
        let start = 10 + (mix(&[person.0, index as u64]) % 40) as i64;
        moves.set(person, need.key(), start);
    }
    moves.set(person, REGARD, 10_i64);
    for other in others {
        if *other == person {
            continue;
        }
        for (a, b) in [(person, *other), (*other, person)] {
            if integer(state, a, &opinion_key(b)).is_none() {
                let start = (mix(&[a.0, b.0]) % 36) as i64 - 10;
                moves.set(a, &opinion_key(b), start);
            }
        }
    }
    moves
}

/// The System begins, or someone new joins in.
pub(crate) struct Enrols(pub(crate) fn(&WorldState) -> Cast);

impl Action for Enrols {
    fn name(&self) -> &'static str {
        "lives_enrol"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let person = arg_entity(request, "person")?;
        if state.entity(person).is_none() {
            return Err(ActionError::Invalid("nobody by that id".into()));
        }
        if enrolled(state, person) {
            return Err(ActionError::Invalid("already living here".into()));
        }
        let others = match request.args.get("others") {
            Some(Value::List(others)) => others
                .iter()
                .filter_map(|value| match value {
                    Value::Entity(id) => Some(*id),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let mut draft = EventDraft::new("life_began");
        draft.targets = vec![person];
        if state.entity(cast.notes).is_none() {
            draft.changes.push(StateChange::CreateEntity(
                Entity::new(cast.notes, "lives").with_component("name", "Lives"),
            ));
        }
        draft
            .changes
            .extend(enrolment(state, &cast, person, &others).changes);
        Ok(draft)
    }
}

/// Where someone does an activity.
pub(crate) fn place_for(state: &WorldState, cast: &Cast, person: EntityId, at: At) -> EntityId {
    match at {
        At::Work => (cast.work)(state, person).unwrap_or(cast.gathering),
        At::Home => (cast.home)(state, person).unwrap_or(cast.quiet),
        At::Gathering => cast.gathering,
        At::Quiet => cast.quiet,
    }
}

/// Who someone does an activity with, if anyone.
pub(crate) fn company_for(
    state: &WorldState,
    cast: &Cast,
    person: EntityId,
    with: With,
    others: &[EntityId],
    seed: u64,
) -> Option<EntityId> {
    let others = others
        .iter()
        .copied()
        .filter(|other| *other != person)
        .collect::<Vec<_>>();
    if others.is_empty() {
        return None;
    }
    match with {
        With::Alone => None,
        With::Friend => {
            // Mostly whoever they like best, sometimes their partner, now
            // and then someone else entirely.
            if seed.is_multiple_of(5) {
                return pick(&others, seed / 5).copied();
            }
            if let Some(partner) = partner(state, person).filter(|p| others.contains(p)) {
                if seed.is_multiple_of(3) {
                    return Some(partner);
                }
            }
            others
                .iter()
                .copied()
                .max_by_key(|other| (opinion(state, person, *other), mix(&[seed, other.0])))
        }
        With::Anyone => pick(&others, seed).copied(),
        With::Workmate => {
            let work = (cast.work)(state, person);
            let mates = others
                .iter()
                .copied()
                .filter(|other| work.is_some() && (cast.work)(state, *other) == work)
                .collect::<Vec<_>>();
            pick(if mates.is_empty() { &others } else { &mates }, seed).copied()
        }
    }
}

/// The need pressing hardest on someone, with a little chance in it.
pub(crate) fn pressing(state: &WorldState, person: EntityId, seed: u64) -> Need {
    Need::ALL
        .iter()
        .copied()
        .max_by_key(|need| {
            lack(state, person, *need) * 4 + (mix(&[seed, *need as u64]) % 60) as i64
        })
        .unwrap_or(Need::Company)
}

/// How much each need grows in a period, by someone's nature.
pub(crate) fn drift(traits: &[String], need: Need) -> i64 {
    let mut by = 9;
    for t in traits {
        by += match (t.as_str(), need) {
            ("sociable", Need::Company) | ("restless", Need::Purpose) => 4,
            ("shy", Need::Company) | ("steady", Need::Purpose) => -3,
            ("thrifty", Need::Money) | ("dreamy", Need::Purpose) => 3,
            ("generous", Need::Money) => 2,
            ("proud", Need::Purpose) => 2,
            ("steady", Need::Rest) => -2,
            ("restless", Need::Rest) => 3,
            _ => 0,
        };
    }
    by
}

/// Things someone might add about how their life stands: their partner,
/// someone they are not speaking to, a friend, what they are short of.
pub(crate) fn tails(state: &WorldState, person: EntityId, seed: u64) -> Vec<String> {
    let others = cast_ids(state);
    let mut tails = Vec::new();
    if let Some(p) = partner(state, person) {
        tails.push(format!(" {} sends their love.", name(state, p)));
        tails.push(format!(" Home to {} now.", name(state, p)));
    }
    if let Some(foe) = others
        .iter()
        .copied()
        .filter(|o| *o != person && opinion(state, person, *o) <= -30)
        .min_by_key(|o| opinion(state, person, *o))
    {
        tails.push(format!(" Still not speaking to {}.", name(state, foe)));
        tails.push(format!(" Kept well clear of {}.", name(state, foe)));
    }
    if let Some(friend) = others
        .iter()
        .copied()
        .filter(|o| *o != person && opinion(state, person, *o) >= 40)
        .max_by_key(|o| mix(&[seed, o.0]))
    {
        tails.push(format!(" {} is a gem.", name(state, friend)));
        tails.push(format!(" Seeing {} later.", name(state, friend)));
    }
    if lack(state, person, Need::Rest) >= 70 {
        tails.push(" I could sleep for a week.".into());
    }
    if lack(state, person, Need::Money) >= 70 {
        tails.push(" Things are tight, mind.".into());
    }
    if lack(state, person, Need::Company) >= 70 {
        tails.push(" Quiet without anyone to talk to.".into());
    }
    tails
}

/// A line as someone says it: as it is if nobody has said it lately,
/// otherwise with something of their own added, so the same words do not
/// come round from everyone.
pub(crate) fn personal(
    state: &WorldState,
    person: EntityId,
    base: &str,
    heard: &Heard,
    seed: u64,
) -> String {
    let said_lately = |line: &str| heard.lately(line);
    if !said_lately(base) {
        return base.to_string();
    }
    let options = tails(state, person, seed)
        .into_iter()
        .map(|tail| format!("{base}{tail}"))
        .collect::<Vec<_>>();
    options
        .iter()
        .find(|line| !said_lately(line))
        .cloned()
        .unwrap_or_else(|| base.to_string())
}

/// What someone says about what they did, with a word about how things
/// stand with the people they care about.
pub(crate) fn saying(
    state: &WorldState,
    cast: &Cast,
    person: EntityId,
    activity: &Activity,
    words: &[(&str, &str)],
    seed: u64,
    heard: &Heard,
) -> (String, String) {
    // Something not said lately, if there is anything; otherwise whatever
    // was said longest ago. A line about a friend needs a friend.
    let friend = best_friend(state, person).map(|friend| name(state, friend));
    let mut all_words = words.to_vec();
    if let Some(friend) = &friend {
        all_words.push(("friend", friend.as_str()));
    }
    // Someone with a voice of their own says one of their own lines about
    // half the time, and anything else in their own words.
    let voice = (cast.voice)(person);
    let lines = match voice {
        Some(voice) if (seed / 3).is_multiple_of(2) => own_lines(voice)
            .into_iter()
            .filter(|line| friend.is_some() || !line.contains("{friend}"))
            .map(|line| fill(&line, &all_words))
            .collect::<Vec<_>>(),
        _ => activity
            .said
            .iter()
            .filter(|line| friend.is_some() || !line.contains("{friend}"))
            .map(|line| fill(line, &all_words))
            .collect::<Vec<_>>(),
    };
    let fresh = lines
        .iter()
        .filter(|line| !heard.lately(line))
        .collect::<Vec<_>>();
    let base = if fresh.is_empty() {
        lines
            .iter()
            .min_by_key(|line| heard.when(line))
            .cloned()
            .unwrap_or_default()
    } else {
        pick(&fresh, seed)
            .map(|line| (*line).clone())
            .unwrap_or_default()
    };
    let tails = tails(state, person, seed);
    let tail = (seed / 7)
        .is_multiple_of(2)
        .then(|| pick(&tails, seed / 17).cloned())
        .flatten();
    let line = format!("{base}{}", tail.unwrap_or_default());
    if heard.lately(&line) {
        (personal(state, person, &base, heard, seed), base)
    } else {
        (line, base)
    }
}

/// Everyone the System has taken in and who has not left, for words about
/// them; who is living here now is the Pack's to say.
pub(crate) fn cast_ids(state: &WorldState) -> Vec<EntityId> {
    state
        .entities()
        .filter(|entity| entity.component(TRAITS_KEY).is_some())
        .filter(|entity| !gone(state, entity.id))
        .map(|entity| entity.id)
        .collect()
}

/// Someone gets on with their day.
pub(crate) struct Lives(pub(crate) fn(&WorldState) -> Cast);

impl Action for Lives {
    fn name(&self) -> &'static str {
        "lives_day"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let person = arg_entity(request, "person")?;
        if !enrolled(state, person) || gone(state, person) {
            return Err(ActionError::Invalid("not living here".into()));
        }
        let others = match request.args.get("others") {
            Some(Value::List(others)) => others
                .iter()
                .filter_map(|value| match value {
                    Value::Entity(id) if enrolled(state, *id) => Some(*id),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let now = period(state, &cast);
        let mood = (cast.mood)(state).clamp(-10, 10);
        let seed = mix(&[now, person.0, 11, (mood + 10) as u64]);
        // Two days in five, whoever has work goes to it, unless they are
        // dead on their feet; the rest are theirs.
        let working = (cast.work)(state, person).is_some()
            && (now + person.0) % 5 < 2
            && lack(state, person, Need::Rest) < 85;
        let need = if working {
            Need::Money
        } else {
            pressing(state, person, seed)
        };
        let last = retold::last_did(state, person);
        // Rather than say the same again, people do something else: an
        // activity is fresh if it still has something to say that nobody
        // said lately.
        let heard = Heard::of(state, &cast);
        let friend = best_friend(state, person).map(|friend| name(state, friend));
        let fresh = |activity: &&Activity| {
            let place = name(state, place_for(state, &cast, person, activity.at));
            let other = company_for(state, &cast, person, activity.with, &others, seed / 13)
                .map(|other| name(state, other))
                .unwrap_or_default();
            activity.said.iter().any(|line| {
                if line.contains("{friend}") && friend.is_none() {
                    return false;
                }
                let filled = fill(
                    line,
                    &[
                        ("place", place.as_str()),
                        ("other", other.as_str()),
                        ("friend", friend.as_deref().unwrap_or("")),
                    ],
                );
                !heard.lately(&filled)
            })
        };
        let all = cast
            .activities
            .iter()
            .filter(|activity| activity.need == need && activity.id != last)
            .collect::<Vec<_>>();
        let fresh_ones = all.iter().copied().filter(fresh).collect::<Vec<_>>();
        let choices = if fresh_ones.is_empty() {
            all
        } else {
            fresh_ones
        };
        let Some(activity) = pick(&choices, seed / 3).copied() else {
            return Err(ActionError::Invalid("nothing to do".into()));
        };
        let with = company_for(state, &cast, person, activity.with, &others, seed / 13);
        if activity.with != With::Alone && with.is_none() {
            // Nobody to do it with: a quiet day instead.
            return Err(ActionError::Invalid("nobody about".into()));
        }
        let place = place_for(state, &cast, person, activity.at);
        let (me, place_name) = (name(state, person), name(state, place));
        let other_name = with.map(|w| name(state, w)).unwrap_or_default();
        let words = [
            ("name", me.as_str()),
            ("other", other_name.as_str()),
            ("place", place_name.as_str()),
        ];

        let mut moves = Moves::default();
        let traits = traits(state, person);
        for n in Need::ALL {
            let by = if n == need {
                -(30 + (seed % 20) as i64)
            } else {
                drift(&traits, n)
            };
            moves.lack(state, person, n, by);
        }
        let mut quarrel = false;
        if let Some(other) = with {
            let fit = compatibility(state, person, other);
            let chance = mix(&[seed, other.0, 3]) % 100;
            // People who grate on each other sometimes have words, and
            // anyone at the end of their tether snaps more easily.
            let frayed = Need::ALL
                .iter()
                .any(|need| lack(state, person, *need) >= 70 || lack(state, other, *need) >= 80);
            // A sour mood in the place shortens tempers; a bright one
            // lengthens them.
            let temper = (18 - fit * 4).max(5) + if frayed { 12 } else { 0 } - mood;
            quarrel = chance < temper.max(2) as u64 && !(cast.kind)(state);
            let by = if quarrel {
                -(10 + (chance % 12) as i64)
            } else {
                3 + fit * 2 + (chance % 6) as i64
            };
            moves.opinion(state, person, other, by);
            moves.opinion(state, other, person, by - if quarrel { 2 } else { 1 });
            if need == Need::Company {
                moves.lack(state, other, Need::Company, -15);
            }
        }
        // Every so often, what someone feels about the people they have not
        // spent time with fades a little toward nothing.
        if now % 5 == person.0 % 5 {
            for other in &others {
                if Some(*other) == with || *other == person {
                    continue;
                }
                let view = opinion(state, person, *other);
                if view.abs() >= 3 {
                    moves.opinion(state, person, *other, -3 * view.signum());
                }
            }
        }
        for (key, by) in activity.gives {
            moves.add(state, person, key, *by, i64::MIN / 4, i64::MAX / 4);
        }
        // An ordinary day, with no words had, mends a little of how someone
        // feels about the place.
        if !quarrel && integer(state, person, REGARD).unwrap_or(0) < 10 {
            moves.regard(state, person, 1);
        }
        // What they did, and which time this year this kind of day is told
        // of them, this one included: counted on them, so a World resumed
        // from a checkpoint, its earlier Events not kept, counts the year
        // all the same.
        let year = state.world_time() / year_length(&cast).max(1);
        let lived = retold::Days::of(state, person, year).lived(&cast, activity.id);
        moves.set(person, retold::DID, lived.note);
        let nth = lived.nth;
        moves.set(person, AT, Value::Entity(place));

        let mut told = fill(activity.told, &words);
        let (mut said, base) = saying(state, &cast, person, activity, &words, seed, &heard);
        if quarrel {
            told.push_str(", and they had words");
            // Said in words nobody has used lately, like any other line:
            // over something else, if every way of saying this one was.
            let quarrel_over = |topic: &str| {
                let mut options = [
                    format!("{other_name} and I had words over {topic}."),
                    format!("Don't mention {topic} to {other_name}. Just don't."),
                    format!("{other_name} and I fell out over {topic}. It'll blow over."),
                    format!("Me and {other_name}, shouting about {topic}. Silly."),
                ];
                options.rotate_left((seed / 23 % 4) as usize);
                options
            };
            let first = pick(cast.topics, seed / 19).copied().unwrap_or("nothing");
            let (topic, line) = (0..cast.topics.len().clamp(1, 6))
                .filter_map(|offset| {
                    let topic = pick(cast.topics, seed / 19 + offset as u64)
                        .copied()
                        .unwrap_or("nothing");
                    let line = quarrel_over(topic)
                        .into_iter()
                        .find(|line| !heard.lately(line))?;
                    Some((topic, line))
                })
                .next()
                .unwrap_or_else(|| (first, quarrel_over(first)[0].clone()));
            if let Some(other) = with {
                moves.set(person, ROW, format!("{other}:{topic}"));
                moves.set(other, ROW, format!("{person}:{topic}"));
            }
            said = line;
        }
        // Someone close to one who died lately speaks of them.
        let mut remembers = None;
        if !quarrel {
            if let Some((who, lines)) = generations::remembering(state, &cast, person, seed / 31) {
                if let Some(line) = lines.iter().find(|line| !heard.lately(line)) {
                    said = line.clone();
                    remembers = Some(who);
                }
            }
        }
        if let Some(voice) = (cast.voice)(person) {
            said = restyle(voice, &said, seed / 29);
        }
        remember_saying(&mut moves, &heard, &said);
        remember_saying(&mut moves, &heard, &base);
        let mut draft = EventDraft::new("lived");
        draft.payload.insert(TOLD_NTH.into(), Value::Integer(nth));
        if let Some(who) = remembers {
            draft.payload.insert("remembers".into(), Value::Entity(who));
        }
        draft.actor = Some(person);
        draft.targets = with.into_iter().collect();
        draft.payload.insert("activity".into(), activity.id.into());
        draft.payload.insert("place".into(), Value::Entity(place));
        draft.payload.insert("told".into(), told.into());
        draft.payload.insert("said".into(), said.into());
        draft.changes = moves.changes;
        Ok(draft)
    }
}

/// How many periods a line is remembered for: nobody says the same thing
/// again, and nobody else says it either, until it has been forgotten.
pub(crate) const HEARD_PERIODS: u64 = 90;

/// Periods between two askings of the same question: a year of days holds
/// no more than six.
pub const ASKED_APART_PERIODS: u64 = 61;

/// When lines were last said. Each period's lines are kept together in
/// one note, `lives.lines.<period>`, as a short hash of each line written
/// in [`SAID_CODE`] characters, so the notes hold one entry a period
/// rather than one a line.
#[derive(Clone)]
pub(crate) struct Heard<'a> {
    pub(crate) state: &'a WorldState,
    pub(crate) notes: EntityId,
    pub(crate) now: u64,
    /// Each period's note, the latest first: the period, and the short
    /// hashes of the lines said in it, in order, so a line is looked up
    /// rather than looked for.
    pub(crate) said: Vec<(u64, &'a [[u8; SAID_CODE]])>,
    /// The same, as one list in order of the lines' codes: see
    /// [`Heard::latest`].
    pub(crate) latest: std::cell::OnceCell<Vec<u64>>,
}

/// Where a period's lines are noted.
pub(crate) const SAID: &str = "lives.lines.";
/// How many characters a line's short hash takes: six bits each, so 36
/// bits. With a few thousand lines remembered at once, another line is
/// taken for one of them about once in twenty million askings: over three
/// years of a harbour's days, about one chance in ten that it ever
/// happens, and then only a line said a little differently.
pub(crate) const SAID_CODE: usize = 6;
pub(crate) const SAID_DIGITS: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Each period's note under `prefix`, the latest first, read as codes of
/// `N` characters.
pub(crate) fn said_notes<'a, const N: usize>(
    state: &'a WorldState,
    notes: EntityId,
    prefix: &str,
) -> Vec<(u64, &'a [[u8; N]])> {
    let mut said = Vec::new();
    if let Some(notes) = state.entity(notes) {
        for (key, value) in notes
            .components
            .range::<str, _>((
                std::ops::Bound::Included(prefix),
                std::ops::Bound::Unbounded,
            ))
            .take_while(|(key, _)| key.starts_with(prefix))
        {
            let at = key.strip_prefix(prefix).and_then(|at| at.parse().ok());
            if let (Some(at), Value::Text(lines)) = (at, value) {
                said.push((at, lines.as_bytes().as_chunks::<N>().0));
            }
        }
    }
    said.sort_unstable_by_key(|(at, _)| std::cmp::Reverse(*at));
    said
}

/// Each code character's place among [`SAID_DIGITS`] in the order of the
/// characters themselves (not of the digits they stand for), so a code
/// read as a number of these sorts as its characters do.
pub(crate) const SAID_RANKS: [u8; 256] = {
    let mut ranks = [0; 256];
    let mut rank = 0;
    let mut byte = 0;
    while byte < 256 {
        let mut digit = 0;
        while digit < 64 {
            if SAID_DIGITS[digit] as usize == byte {
                ranks[byte] = rank;
                rank += 1;
            }
            digit += 1;
        }
        byte += 1;
    }
    ranks
};

/// A code as one number of six bits a character, ordered as the codes'
/// characters are.
pub(crate) fn code_number<const N: usize>(code: &[u8; N]) -> u64 {
    code.iter().fold(0, |number, digit| {
        (number << 6) | u64::from(SAID_RANKS[usize::from(*digit)])
    })
}

/// Bits of a [`Heard::latest`] entry that hold the period, below the code.
pub(crate) const PERIOD_BITS: u32 = 64 - 6 * SAID_CODE as u32;

/// Numbers whose top bits are evenly spread (codes are hashes), in order:
/// dealt by their top bits into small piles, each put in order.
pub(crate) fn sort_codes(numbers: Vec<u64>) -> Vec<u64> {
    const PILE_BITS: u32 = 10;
    let pile = |number: u64| (number >> (64 - PILE_BITS)) as usize;
    let mut starts = vec![0_usize; (1 << PILE_BITS) + 1];
    for number in &numbers {
        starts[pile(*number) + 1] += 1;
    }
    for at in 1..starts.len() {
        starts[at] += starts[at - 1];
    }
    let mut next = starts.clone();
    let mut sorted = vec![0; numbers.len()];
    for number in numbers {
        let at = &mut next[pile(number)];
        sorted[*at] = number;
        *at += 1;
    }
    for piles in starts.windows(2) {
        sorted[piles[0]..piles[1]].sort_unstable();
    }
    sorted
}

/// The latest period whose note holds `code`.
pub(crate) fn noted_in<const N: usize>(said: &[(u64, &[[u8; N]])], code: &[u8; N]) -> Option<u64> {
    said.iter()
        .find(|(_, lines)| lines.binary_search(code).is_ok())
        .map(|(at, _)| *at)
}

impl<'a> Heard<'a> {
    pub(crate) fn of(state: &'a WorldState, cast: &Cast) -> Self {
        Heard {
            state,
            notes: cast.notes,
            now: period(state, cast),
            said: said_notes(state, cast.notes, SAID),
            latest: std::cell::OnceCell::new(),
        }
    }

    /// Every line noted in every period, as one number each, its code
    /// above the period, in order: made the first time a line is looked
    /// up, so that each line after is one search rather than one a period.
    pub(crate) fn latest(&self) -> &[u64] {
        self.latest.get_or_init(|| {
            let latest = self
                .said
                .iter()
                .flat_map(|(at, lines)| {
                    lines
                        .iter()
                        .map(move |line| (code_number(line) << PERIOD_BITS) | at)
                })
                .collect::<Vec<_>>();
            sort_codes(latest)
        })
    }

    /// The period a line was last said in, if it is remembered.
    pub(crate) fn when(&self, line: &str) -> Option<u64> {
        let hash = line_hash(line);
        let code = hash_code::<SAID_CODE>(short_hash::<SAID_CODE>(hash));
        // A period too late to share a number with its code (hundreds of
        // thousands of years of days) is looked for note by note.
        if self
            .said
            .first()
            .is_some_and(|(at, _)| *at >> PERIOD_BITS != 0)
        {
            noted_in(&self.said, &code)
        } else {
            let number = code_number(&code);
            let latest = self.latest();
            // The last entry for the code holds its latest period.
            let after = latest.partition_point(|entry| entry >> PERIOD_BITS <= number);
            after
                .checked_sub(1)
                .map(|last| latest[last])
                .filter(|entry| entry >> PERIOD_BITS == number)
                .map(|entry| entry & ((1 << PERIOD_BITS) - 1))
        }
    }

    pub(crate) fn lately(&self, line: &str) -> bool {
        self.when(line)
            .is_some_and(|at| self.now.saturating_sub(at) < HEARD_PERIODS)
    }

    /// [`Self::when`] as it was first written, reading every note's codes
    /// as strings: the reference the faster reading is checked against.
    #[cfg(test)]
    pub(crate) fn when_by_reading_every_note(&self, line: &str) -> Option<u64> {
        fn noted<const N: usize>(said: &[(u64, &[[u8; N]])], code: &[u8; N]) -> Option<u64> {
            said.iter()
                .find(|(_, lines)| lines.binary_search(code).is_ok())
                .map(|(at, _)| *at)
        }
        let hash = line_hash(line);
        noted(
            &self.said,
            &hash_code::<SAID_CODE>(short_hash::<SAID_CODE>(hash)),
        )
    }
}

pub(crate) fn line_hash(line: &str) -> u64 {
    line.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A line's short hash in `N` characters: the best-mixed bits of its full
/// one.
pub(crate) fn short_hash<const N: usize>(line_hash: u64) -> u64 {
    line_hash >> (64 - 6 * N)
}

pub(crate) fn hash_code<const N: usize>(hash: u64) -> [u8; N] {
    let mut code = [0; N];
    for (at, digit) in code.iter_mut().enumerate() {
        *digit = SAID_DIGITS[((hash >> (6 * (N - 1 - at))) & 63) as usize];
    }
    code
}

/// A short hash from its characters, the other way from [`hash_code`].
#[cfg(test)]
pub(crate) fn code_hash(code: &[u8]) -> u64 {
    code.iter().fold(0, |hash, digit| {
        let value = SAID_DIGITS
            .iter()
            .position(|known| known == digit)
            .unwrap_or(0) as u64;
        (hash << 6) | value
    })
}

pub(crate) fn said_key(period: u64) -> String {
    format!("{SAID}{period}")
}

/// The period a note of lines is for.
pub(crate) fn said_period(key: &str) -> Option<u64> {
    key.strip_prefix(SAID)?.parse().ok()
}

/// Notes that a line was said this period: added to this period's note,
/// once, whether it is already in the World's notes or earlier in the
/// same event.
pub(crate) fn remember_saying(moves: &mut Moves, heard: &Heard, said: &str) {
    let key = said_key(heard.now);
    let code = hash_code::<SAID_CODE>(short_hash::<SAID_CODE>(line_hash(said)));
    // Kept in order, so the note is looked up rather than read through.
    let noted = |lines: &mut String| {
        let at = lines
            .as_bytes()
            .as_chunks::<SAID_CODE>()
            .0
            .binary_search(&code);
        if let Err(at) = at {
            lines.insert_str(
                at * SAID_CODE,
                std::str::from_utf8(&code).expect("the digits are ASCII"),
            );
        }
    };
    let pending = moves.changes.iter_mut().find_map(|change| match change {
        StateChange::SetComponent {
            entity,
            key: noted,
            value: Value::Text(lines),
        } if *entity == heard.notes && *noted == key => Some(lines),
        _ => None,
    });
    if let Some(lines) = pending {
        noted(lines);
        return;
    }
    let mut lines = text(heard.state, heard.notes, &key)
        .unwrap_or_default()
        .to_string();
    let before = lines.len();
    noted(&mut lines);
    if lines.len() != before {
        moves.set(heard.notes, &key, lines);
    }
}

/// One of several ways to say something, whichever was said longest ago.
pub(crate) fn pick_line(options: &[String], heard: &Heard, person: EntityId) -> String {
    options
        .iter()
        .min_by_key(|line| (heard.when(line), mix(&[person.0, line.len() as u64])))
        .cloned()
        .unwrap_or_default()
}

/// How two people stand, from what each thinks of the other and how they
/// stood before: a friendship or a feud, once made, holds through an
/// ordinary bad week or good one, and ends only when it has truly gone.
pub(crate) fn standing(state: &WorldState, a: EntityId, b: EntityId, was: &str) -> &'static str {
    let (ab, ba) = (opinion(state, a, b), opinion(state, b, a));
    if partner(state, a) == Some(b) {
        return "partners";
    }
    let feud = ab <= -25 || ba <= -25;
    let friends = ab >= 32 && ba >= 32;
    match was {
        "friends" if ab <= -45 || ba <= -45 => "foes",
        "friends" if ab >= 10 || ba >= 10 => "friends",
        "foes" if ab < 5 || ba < 5 => "foes",
        "foes" if friends => "friends",
        "friends" | "foes" => "",
        _ if feud => "foes",
        _ if friends => "friends",
        _ => "",
    }
}
