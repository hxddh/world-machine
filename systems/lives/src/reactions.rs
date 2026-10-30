//! What people do about what happens: answering a situation, reacting to
//! what the player made, warming, greeting, remembering a year ago,
//! leaving a keepsake, meeting at a gathering, and the season's turn.

use crate::*;

/// Someone answers a situation, or it runs out.
pub(crate) struct Answers(pub(crate) fn(&WorldState) -> Cast);

impl Action for Answers {
    fn name(&self) -> &'static str {
        "lives_situation_answered"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let key = arg_text(request, "situation")?;
        let answer = arg_text(request, "answer")?;
        let candidate = Candidate::parse(key)
            .ok_or_else(|| ActionError::Invalid("no such situation".into()))?;
        if open_map(state, &cast, key).is_none() {
            return Err(ActionError::Invalid("not open".into()));
        }
        let lapsed = answer == "lapse";
        if !lapsed {
            let offered = compose(state, &cast, &candidate);
            let given = offered
                .answers
                .iter()
                .find(|offered| offered.id == answer)
                .ok_or_else(|| ActionError::Invalid(format!("{answer} is not an answer")))?;
            if let Some(why) = &given.unavailable {
                return Err(ActionError::Invalid(why.clone()));
            }
        }
        let (mut moves, told, said) = outcome(state, &cast, &candidate, answer)?;
        // At a door, they answer in their own words: the first reply to
        // the warmer answer, the second to the other.
        let said = match (door_scene(&cast, &candidate), lapsed) {
            (Some(scene), false) => {
                let warmer = matches!(answer, "stay" | "keep" | "go" | "word" | "help");
                let words = words_for(state, &cast, &candidate);
                fill_owned(scene.replies[usize::from(!warmer)], &words)
            }
            _ => said,
        };
        let heard = Heard::of(state, &cast);
        let said = personal(
            state,
            candidate.a,
            &said,
            &heard,
            mix(&[candidate.a.0, period(state, &cast), 5]),
        );
        remember_saying(&mut moves, &heard, &said);
        closing(state, &cast, &candidate, &mut moves);
        let mut draft = EventDraft::new(if lapsed {
            "situation_lapsed"
        } else {
            "situation_answered"
        });
        draft.actor = Some(candidate.a);
        draft.targets = candidate
            .b
            .filter(|b| state.entity(*b).is_some())
            .into_iter()
            .collect();
        draft.payload.insert("situation".into(), key.into());
        draft
            .payload
            .insert("kind".into(), candidate.kind.id().into());
        draft.payload.insert("answer".into(), answer.into());
        draft.payload.insert("told".into(), told.into());
        draft.payload.insert("said".into(), said.into());
        // The answer in the words it was offered in, where they name a
        // friend the record keeps no other trace of: a letter cites it
        // later, when the friend may be another.
        if let Some(label) = answer_label(candidate.kind.id(), answer)
            .filter(|label| !lapsed && label.contains("{friend}"))
        {
            let offered = fill_owned(label, &words_for(state, &cast, &candidate));
            draft.payload.insert("offered".into(), offered.into());
        }
        if lapsed {
            draft.payload.insert("lapsed".into(), true.into());
        } else if candidate.kind == Kind::Keepsake {
            if let Some((_, what)) = words_for(state, &cast, &candidate)
                .into_iter()
                .find(|(slot, _)| *slot == "keepsake")
            {
                draft.payload.insert("keepsake".into(), what.into());
            }
        }
        draft.changes = moves.changes;
        Ok(draft)
    }
}

/// Someone near a deed of the player's says what they make of it.
pub(crate) struct Reacts(pub(crate) fn(&WorldState) -> Cast);

impl Action for Reacts {
    fn name(&self) -> &'static str {
        "lives_reacts"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let who = arg_entity(request, "who")?;
        let deed = arg_text(request, "deed")?;
        let thing = arg_text(request, "thing")?;
        let place = arg_entity(request, "place")?;
        if !enrolled(state, who) || gone(state, who) {
            return Err(ActionError::Invalid("nobody to react".into()));
        }
        let words = [
            ("thing", thing.to_lowercase()),
            ("place", name(state, place)),
            ("settlement", cast.settlement.to_string()),
        ];
        let pool: &[&str] = match deed {
            "built_by_hand" => &[
                "A {thing}! Just what {place} needed.",
                "You built that? Well, look at it.",
                "A {thing} by {place}. I'll be using that.",
                "Another {thing}? The {settlement} won't know itself.",
                "Straight and true, that {thing}. Who taught you?",
                "I watched you build that {thing}. Not bad at all.",
            ],
            "decorated_by_hand" => &[
                "{thing} up at {place}! Cheers the place right up.",
                "Oh, that's pretty. Was that you?",
                "{thing} all over {place}. Is it someone's birthday?",
                "You've a good eye. {place} looks like a party.",
            ],
            "planted_by_hand" => &[
                "You planted that? I'll keep an eye on it.",
                "Something growing by {place}. About time.",
                "Water it at dusk, not at noon. Trust me.",
                "Green by {place}! The bees will find it.",
            ],
            "moved_by_hand" => &[
                "Better there, I think.",
                "Oh, you've moved it. It suits there.",
                "The {thing} by {place} now? Yes, I see it.",
                "Moved the {thing}? It catches the light there.",
                "I'd got used to the {thing} where it was. This is better.",
                "The {thing} looks at home by {place}.",
                "You'll wear that {thing} out, moving it about.",
                "Now the {thing}'s by {place}, I'll sit there.",
                "Round and round goes the {thing}. I like it here.",
                "The {thing}'s found its spot by {place}, I reckon.",
                "Someone's fond of moving that {thing}.",
                "By {place}, the {thing}? Suits it.",
                "I nearly walked into the {thing}. It's moved!",
                "The {thing} again? {place} it is, then.",
            ],
            _ => return Err(ActionError::Invalid(format!("nothing to say about {deed}"))),
        };
        let heard = Heard::of(state, &cast);
        // "Landing lights" are not "a landing lights" nor "that landing
        // lights": a thing named in the plural keeps to the lines that fit.
        let plural = thing.ends_with('s') && !thing.ends_with("ss");
        let options = pool
            .iter()
            .filter(|line| {
                !plural
                    || !["A {thing}", "Another {thing}", "that {thing}", "{thing}'s"]
                        .iter()
                        .any(|single| line.contains(single))
            })
            .map(|line| {
                fill_owned(
                    line,
                    &words
                        .iter()
                        .map(|(k, v)| (*k, v.clone()))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        let said = pick_line(&options, &heard, who);
        let said = match (cast.voice)(who) {
            Some(voice) => restyle(voice, &said, mix(&[who.0, heard.now, 41])),
            None => said,
        };
        let mut moves = Moves::default();
        remember_saying(&mut moves, &heard, &said);
        if deed != "moved_by_hand" {
            moves.regard(state, who, 2);
        }
        let mut draft = EventDraft::new("reacted");
        draft.actor = Some(who);
        draft.targets = vec![who];
        draft.payload.insert("deed".into(), deed.into());
        draft.payload.insert("thing".into(), thing.into());
        draft
            .payload
            .insert("place".into(), name(state, place).into());
        draft.payload.insert("said".into(), said.into());
        draft.changes = moves.changes;
        Ok(draft)
    }
}

/// Someone warming to the player shares a small moment with them: the
/// first of their five scenes, said unasked.
pub(crate) struct Warms(pub(crate) fn(&WorldState) -> Cast);

impl Action for Warms {
    fn name(&self) -> &'static str {
        "lives_warms"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let who = arg_entity(request, "who")?;
        if !enrolled(state, who) || gone(state, who) {
            return Err(ActionError::Invalid("nobody to warm".into()));
        }
        if door_opened(state, who, Kind::Warming).is_some() {
            return Err(ActionError::Invalid("already warmed".into()));
        }
        let candidate = Candidate {
            kind: Kind::Warming,
            a: who,
            b: None,
            topic: 0,
        };
        let words = words_for(state, &cast, &candidate);
        let scene = scene_of((cast.voice)(who), (cast.traits)(who), 0);
        let said = fill_owned(scene.prompt, &words);
        let mut moves = Moves::default();
        moves.set(who, &door_key(Kind::Warming), period(state, &cast) as i64);
        moves.regard(state, who, 2);
        let heard = Heard::of(state, &cast);
        remember_saying(&mut moves, &heard, &said);
        let mut draft = EventDraft::new("warmed");
        draft.actor = Some(who);
        draft.targets = vec![who];
        draft.payload.insert(
            "told".into(),
            format!("{} shared a quiet moment with you", first_name(state, who)).into(),
        );
        draft.payload.insert("said".into(), said.into());
        draft.changes = moves.changes;
        Ok(draft)
    }
}

/// Someone comes over to say hello to a new player.
pub(crate) struct Greets(pub(crate) fn(&WorldState) -> Cast);

impl Action for Greets {
    fn name(&self) -> &'static str {
        "lives_greets"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let who = arg_entity(request, "who")?;
        if state.entity(who).is_none() || gone(state, who) {
            return Err(ActionError::Invalid("nobody to say hello".into()));
        }
        let words = [
            ("name", first_name(state, who)),
            ("settlement", cast.settlement.to_string()),
        ];
        let lines: &[&str] = if arg_text(request, "later").is_ok() {
            &MEETINGS
        } else {
            &GREETINGS
        };
        // A Pack may give the place's own hello, in its own words.
        let own = arg_text(request, "line")
            .ok()
            .filter(|line| !line.is_empty());
        let said = own
            .or_else(|| pick(lines, mix(&[who.0, 7])).copied())
            .map(|line| fill_owned(line, &words))
            .unwrap_or_default();
        let mut draft = EventDraft::new("greeted");
        draft.actor = Some(who);
        draft.targets = vec![who];
        draft.payload.insert(
            "told".into(),
            format!("{} came over to say hello", first_name(state, who)).into(),
        );
        draft.payload.insert("said".into(), said.into());
        Ok(draft)
    }
}

/// How someone looks back on a day a year ago.
pub(crate) const A_YEAR_AGO: [&str; 6] = [
    "A year ago today, {told}. Feels like yesterday.",
    "A year ago today, {told}. Where does the time go?",
    "Remember? A year ago today, {told}.",
    "A year ago today, {told}. I still think about it.",
    "A year ago today, {told}. We've come a long way.",
    "Would you believe it? A year ago today, {told}.",
];

/// Someone remembers what happened a year ago today.
pub(crate) struct Remembers;

impl Action for Remembers {
    fn name(&self) -> &'static str {
        "lives_remembers_year"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let who = arg_entity(request, "who")?;
        let then = arg_text(request, "then")?;
        if state.entity(who).is_none() || gone(state, who) || then.is_empty() {
            return Err(ActionError::Invalid("nobody to remember it".into()));
        }
        let seed = mix(&[who.0, state.world_time(), 61]);
        let said = pick(&A_YEAR_AGO, seed)
            .map(|line| fill_owned(line, &[("told", quiet::inside(then))]))
            .unwrap_or_default();
        let mut draft = EventDraft::new("year_remembered");
        draft.actor = Some(who);
        draft.targets = vec![who];
        draft.payload.insert(
            "told".into(),
            format!("{} remembered a year ago today", first_name(state, who)).into(),
        );
        draft.payload.insert("said".into(), said.into());
        Ok(draft)
    }
}

/// On a day a year after something worth telling happened, someone who
/// was there remembers it. At most one a day.
pub fn remember_a_year(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    year: u64,
) -> Result<Option<EventId>, WorldError> {
    let Some((who, told)) = remembered(world, cast, year) else {
        return Ok(None);
    };
    let request = ActionRequest::new("lives_remembers_year")
        .actor(who)
        .arg("who", Value::Entity(who))
        .arg("then", told);
    Ok(world.execute(actions, &request).ok().map(|event| event.id))
}

/// Who remembers what of the day `year` periods ago. Events are recorded
/// in time order, so that day's are found by looking, not by reading the
/// whole history.
pub(crate) fn remembered(world: &World, cast: &Cast, year: u64) -> Option<(EntityId, String)> {
    let (from, to) = a_year_ago(world, cast, year)?;
    let events = world.events();
    let first = events.partition_point(|event| event.world_time < from);
    let last = first + events[first..].partition_point(|event| event.world_time < to);
    memory_of(world, cast, &events[first..last])
}

/// The world times of the day `year` periods ago, if there was one.
pub(crate) fn a_year_ago(world: &World, cast: &Cast, year: u64) -> Option<(u64, u64)> {
    let now = period(world.state(), cast);
    if year == 0 || now < year {
        return None;
    }
    let span = cast.period.max(1);
    let from = (now - year) * span;
    Some((from, from + span))
}

/// Who remembers what, of the events of a day a year ago.
pub(crate) fn memory_of<'a>(
    world: &World,
    cast: &Cast,
    day: impl IntoIterator<Item = &'a Event>,
) -> Option<(EntityId, String)> {
    let state = world.state();
    let now = period(state, cast);
    day.into_iter()
        // Something worth remembering: not a rest on a bench, a day's
        // errands or a letter, but friendships, answers, the year's turns,
        // festivals, things made and outings.
        .filter(|event| {
            matches!(
                event.kind.as_str(),
                "bond_changed"
                    | "situation_answered"
                    | "year_turned"
                    | "suggestion_held"
                    | "guest_visited"
                    | "festival_held"
                    | "built_by_hand"
                    | "decorated_by_hand"
                    | "planted_by_hand"
            )
        })
        .filter_map(|event| {
            let told = match event.payload.get("told") {
                Some(Value::Text(told)) if !told.is_empty() => told.clone(),
                _ => return None,
            };
            let who = event
                .actor
                .into_iter()
                .chain(event.targets.iter().copied())
                .find(|person| {
                    state
                        .entity(*person)
                        .is_some_and(|entity| entity.kind == "resident")
                        && !gone(state, *person)
                })?;
            Some((who, told))
        })
        .max_by_key(|(who, told)| mix(&[who.0, told.len() as u64, now]))
}

/// Someone leaves the player something while they are away.
pub(crate) struct LeavesKeepsake(pub(crate) fn(&WorldState) -> Cast);

impl Action for LeavesKeepsake {
    fn name(&self) -> &'static str {
        "lives_leaves_keepsake"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let who = arg_entity(request, "who")?;
        let why = arg_text(request, "why")?;
        if !enrolled(state, who) || gone(state, who) {
            return Err(ActionError::Invalid("nobody to leave it".into()));
        }
        let words = [
            ("gathering", name(state, cast.gathering)),
            ("settlement", cast.settlement.to_string()),
            ("unit", cast.unit.to_string()),
        ];
        let welcome = arg_text(request, "welcome").is_ok();
        if arg_text(request, "letter").is_ok() {
            // A letter on a quiet day, for the letter box: a letter is
            // not a keepsake.
            if why.trim().is_empty() {
                return Err(ActionError::Invalid("an empty letter".into()));
            }
            let first = first_name(state, who);
            let told = format!("{first} wrote to you");
            let mut draft = EventDraft::new("letter_written");
            draft.actor = Some(who);
            draft.targets = vec![who];
            draft.payload.insert("told".into(), told.into());
            draft.payload.insert("said".into(), why.into());
            draft.payload.insert("letter".into(), true.into());
            draft.changes.push(StateChange::SetComponent {
                entity: who,
                key: WROTE.into(),
                value: (integer(state, who, WROTE).unwrap_or(0) + 1).into(),
            });
            draft
                .changes
                .extend(quiet::letter_notes(state, &cast, who, request));
            return Ok(draft);
        }
        // Something of the Pack's own choosing, given in person.
        if let Ok(what) = arg_text(request, "what") {
            let mut draft = EventDraft::new("keepsake_left");
            draft.actor = Some(who);
            draft.targets = vec![who];
            draft.payload.insert("keepsake".into(), what.into());
            draft.payload.insert(
                "told".into(),
                format!("{} gave you {what}", first_name(state, who)).into(),
            );
            draft.payload.insert("said".into(), why.into());
            return Ok(draft);
        }
        // What is left for the player goes round everything there is to
        // leave, so each is found in turn.
        let left = integer(state, cast.notes, LEFT).unwrap_or(0);
        let what = if welcome {
            pick(&WELCOME, mix(&[who.0, period(state, &cast), 31])).copied()
        } else {
            LEFT_FOR_YOU
                .get(left.max(0) as usize % LEFT_FOR_YOU.len())
                .copied()
        }
        .map(|what| fill_owned(what, &words))
        .unwrap_or_default();
        let season = arg_text(request, "season")
            .ok()
            .and_then(|season| season.parse::<i64>().ok());
        let note = if welcome {
            format!("For your first day in the {}. Welcome.", cast.settlement)
        } else if let Some(season) = season {
            // A different word each season, so a year of them never
            // repeats one.
            let note = SEASON_NOTES[season.rem_euclid(SEASON_NOTES.len() as i64) as usize];
            format!("{note} {why}").trim().to_string()
        } else if why.is_empty() {
            "Missed you round here.".to_string()
        } else {
            format!("I kept this for you while you were away. {why}")
        };
        let told = if welcome {
            format!("{} gave you {what} to welcome you", first_name(state, who))
        } else {
            format!("{} left you {what}", first_name(state, who))
        };
        let mut draft = EventDraft::new("keepsake_left");
        draft.actor = Some(who);
        draft.targets = vec![who];
        if !welcome {
            draft.changes.push(StateChange::SetComponent {
                entity: cast.notes,
                key: LEFT.into(),
                value: (left + 1).into(),
            });
            if let Some(season) = season {
                draft.changes.push(StateChange::SetComponent {
                    entity: cast.notes,
                    key: SEASON_GIFT.into(),
                    value: season.into(),
                });
            }
        }
        draft.payload.insert("keepsake".into(), what.into());
        draft.payload.insert("told".into(), told.into());
        draft.payload.insert("said".into(), note.into());
        Ok(draft)
    }
}

/// The person nearest a deed of the player's, to say what they make of
/// it: someone there now, else whoever works or lives there, else the
/// place's host.
pub(crate) fn reactor(state: &WorldState, cast: &Cast, place: EntityId) -> Option<EntityId> {
    let people = cast_ids(state)
        .into_iter()
        .filter(|p| enrolled(state, *p) && !gone(state, *p))
        .collect::<Vec<_>>();
    people
        .iter()
        .copied()
        .find(|p| at(state, *p) == Some(place))
        .or_else(|| {
            people.iter().copied().find(|p| {
                (cast.work)(state, *p) == Some(place) || (cast.home)(state, *p) == Some(place)
            })
        })
        .or_else(|| people.contains(&cast.host).then_some(cast.host))
        .or_else(|| people.first().copied())
}

/// Someone says what they make of the player's deed: something made,
/// put up, planted or moved. Nothing for gifts and invitations, which
/// their own Systems answer.
pub fn react_to(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    deed: EventId,
) -> Result<Option<EventId>, WorldError> {
    let Some(event) = world.event(deed) else {
        return Ok(None);
    };
    if !matches!(
        event.kind.as_str(),
        "built_by_hand" | "decorated_by_hand" | "planted_by_hand" | "moved_by_hand"
    ) {
        return Ok(None);
    }
    let (Some(thing), Some(place)) = (event.targets.first(), event.targets.last()) else {
        return Ok(None);
    };
    let state = world.state();
    let Some(who) = reactor(state, cast, *place) else {
        return Ok(None);
    };
    let request = ActionRequest::new("lives_reacts")
        .actor(who)
        .caused_by(deed)
        .arg("who", Value::Entity(who))
        .arg("deed", event.kind.clone())
        .arg("thing", name(state, *thing))
        .arg("place", Value::Entity(*place));
    let first = !happened(world, "reacted") && keepsakes(world).is_empty();
    let reaction = world.execute(actions, &request).ok().map(|event| event.id);
    // The player's first deed earns them something to keep at once, from
    // whoever saw it: their first minutes end with a keepsake in hand.
    if let (true, Some(reaction)) = (first, reaction) {
        let welcome = ActionRequest::new("lives_leaves_keepsake")
            .actor(who)
            .caused_by(reaction)
            .arg("who", Value::Entity(who))
            .arg("why", "")
            .arg("welcome", "first deed");
        let _ = world.execute(actions, &welcome);
    }
    Ok(reaction)
}

/// A new player is greeted: whoever welcomes strangers comes over, says
/// who they are, and suggests something to make. Once per World.
pub fn greet(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
) -> Result<Option<EventId>, WorldError> {
    greet_saying(world, actions, cast, "")
}

/// [`greet`], in the place's own words: `line` is what the one who comes
/// over says (`{name}` and `{settlement}` filled in), or empty for one of
/// the System's own hellos.
pub fn greet_saying(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    line: &str,
) -> Result<Option<EventId>, WorldError> {
    if happened(world, "greeted") {
        return Ok(None);
    }
    let people = (cast.people)(world);
    let state = world.state();
    let Some(who) = std::iter::once(cast.host)
        .chain(people)
        .find(|person| state.entity(*person).is_some() && !gone(state, *person))
    else {
        return Ok(None);
    };
    let mut request = ActionRequest::new("lives_greets")
        .actor(who)
        .arg("who", Value::Entity(who));
    if !line.is_empty() {
        request = request.arg("line", line);
    }
    Ok(world.execute(actions, &request).ok().map(|event| event.id))
}

/// On the player's return, someone who thinks well of them leaves them
/// something to keep, with a line about what happened while they were
/// away (`why`, in the World's words, or empty).
pub fn leave_keepsake(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    why: &str,
) -> Result<Option<EventId>, WorldError> {
    if !room_for_keepsake(world, cast) {
        return Ok(None);
    }
    let state = world.state();
    let now = period(state, cast);
    let Some(who) = cast_ids(state)
        .into_iter()
        .filter(|p| enrolled(state, *p) && !gone(state, *p))
        .max_by_key(|p| (regard(state, *p), mix(&[p.0, now])))
    else {
        return Ok(None);
    };
    let request = ActionRequest::new("lives_leaves_keepsake")
        .actor(who)
        .arg("who", Value::Entity(who))
        .arg("why", why);
    Ok(world.execute(actions, &request).ok().map(|event| event.id))
}

/// Two people fond of each other, neither walking out with anyone, get
/// together at a gathering the player made happen: the fondest such pair.
pub(crate) struct Matches(pub(crate) fn(&WorldState) -> Cast);

/// How fond of each other two people must be to get together at a
/// gathering.
pub(crate) const FOND_AT_A_GATHERING: i64 = 40;

impl Action for Matches {
    fn name(&self) -> &'static str {
        "lives_matched"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let at = arg_text(request, "at")?;
        let people = cast_ids(state)
            .into_iter()
            .filter(|person| enrolled(state, *person) && partner(state, *person).is_none())
            .filter(|person| !heartsore(state, *person, period(state, &cast)))
            .collect::<Vec<_>>();
        let now = period(state, &cast);
        let (a, b) = people
            .iter()
            .flat_map(|a| people.iter().map(move |b| (*a, *b)))
            .filter(|(a, b)| a.0 < b.0 && !(cast.kept)(*a, *b) && !(cast.kept)(*b, *a))
            .filter(|(a, b)| generations::may_court(state, &cast, *a, *b))
            .map(|(a, b)| {
                let fond = opinion(state, a, b).min(opinion(state, b, a));
                (fond, mix(&[a.0, b.0, now]), a, b)
            })
            .filter(|(fond, ..)| *fond >= FOND_AT_A_GATHERING)
            .max()
            .map(|(_, _, a, b)| (a, b))
            .ok_or_else(|| ActionError::Invalid("nobody to get together".into()))?;
        let (an, bn) = (first_name(state, a), first_name(state, b));
        let heard = Heard::of(state, &cast);
        let said = pick_line(
            &[
                format!("Who'd have thought? {bn} and me, after {at}."),
                format!("{bn} asked me to dance at {at}. I said yes to more than the dance."),
                format!("It was {at} that did it. {bn} and me."),
            ],
            &heard,
            a,
        );
        let mut moves = Moves::default();
        walk_out(state, &mut moves, a, b);
        remember_saying(&mut moves, &heard, &said);
        for (x, y) in [(a, b), (b, a)] {
            moves.set(x, &bond_key(y), "partners");
        }
        let mut draft = EventDraft::new("bond_changed");
        draft.actor = Some(a);
        draft.targets = vec![b];
        draft.payload.insert("bond".into(), "got_together".into());
        draft.payload.insert(
            "told".into(),
            format!("{an} and {bn} got together at {at}").into(),
        );
        draft.payload.insert("said".into(), said.into());
        draft.changes = moves.changes;
        Ok(draft)
    }
}

/// At a gathering the player made happen, the two fondest of each other
/// who are not yet walking out with anyone may get together: `at` is what
/// the gathering is called ("the music night").
pub fn match_at(
    world: &mut World,
    actions: &ActionRegistry,
    at: &str,
) -> Result<Option<EventId>, WorldError> {
    let request = ActionRequest::new("lives_matched").arg("at", at);
    match world.execute(actions, &request) {
        Ok(event) => Ok(Some(event.id)),
        Err(WorldError::Action(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Someone gives the player something of the Pack's own choosing to keep,
/// with a word about it, if the week has room for it.
pub fn give_keepsake(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    who: EntityId,
    what: &str,
    said: &str,
) -> Result<Option<EventId>, WorldError> {
    if !room_for_keepsake(world, cast) {
        return Ok(None);
    }
    let request = ActionRequest::new("lives_leaves_keepsake")
        .actor(who)
        .arg("who", Value::Entity(who))
        .arg("why", said)
        .arg("what", what);
    Ok(world.execute(actions, &request).ok().map(|event| event.id))
}

/// As each season turns, whoever thinks best of the player leaves them
/// something for it: the next of the things there are to leave.
pub fn season_turns(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    season: u64,
) -> Result<Option<EventId>, WorldError> {
    let now = period(world.state(), cast);
    // Once a season, on its first day with room in the week.
    let this_season = now / season.max(1);
    let given = integer(world.state(), cast.notes, SEASON_GIFT).unwrap_or(0);
    if season == 0
        || this_season == 0
        || given >= this_season as i64
        || !room_for_keepsake(world, cast)
    {
        return Ok(None);
    }
    let state = world.state();
    let Some(who) = cast_ids(state)
        .into_iter()
        .filter(|p| enrolled(state, *p) && !gone(state, *p))
        .max_by_key(|p| (regard(state, *p), mix(&[p.0, now, 67])))
    else {
        return Ok(None);
    };
    let request = ActionRequest::new("lives_leaves_keepsake")
        .actor(who)
        .arg("who", Value::Entity(who))
        .arg("why", "")
        .arg("season", this_season.to_string());
    Ok(world.execute(actions, &request).ok().map(|event| event.id))
}
