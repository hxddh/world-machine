//! A situation opening, and what each answer to it does to the people in
//! it.

use crate::*;

/// A situation comes up.
pub(crate) struct Opens(pub(crate) fn(&WorldState) -> Cast);

impl Action for Opens {
    fn name(&self) -> &'static str {
        "lives_situation_opens"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let key = arg_text(request, "situation")?;
        let candidate = Candidate::parse(key)
            .ok_or_else(|| ActionError::Invalid("no such situation".into()))?;
        if open_map(state, &cast, key).is_some() {
            return Err(ActionError::Invalid("already open".into()));
        }
        let mut situation = compose(state, &cast, &candidate);
        // Put in words nobody has used lately; at a door, in their own.
        let heard = Heard::of(state, &cast);
        let words = words_for(state, &cast, &candidate);
        let prompts = match door_scene(&cast, &candidate) {
            Some(_) => vec![situation.prompt.clone()],
            None => script(candidate.kind)
                .prompts
                .iter()
                .map(|prompt| fill_owned(prompt, &words))
                .collect::<Vec<_>>(),
        };
        // Each place starts its round of ways to put it somewhere of its
        // own, so no two places open on the same words.
        let place: u64 = format!("{} {}", cast.settlement, cast.unit)
            .bytes()
            .fold(1, |hash, byte| {
                hash.wrapping_mul(31).wrapping_add(u64::from(byte))
            });
        let start = (place % prompts.len().max(1) as u64) as usize;
        let prompts = prompts[start..]
            .iter()
            .chain(&prompts[..start])
            .cloned()
            .collect::<Vec<_>>();
        situation.prompt = match prompts.iter().find(|prompt| !heard.lately(prompt)) {
            Some(prompt) => prompt.clone(),
            None => personal(
                state,
                candidate.a,
                &situation.prompt,
                &heard,
                mix(&[candidate.a.0, 9]),
            ),
        };
        // Nothing is asked more than six times a year.
        if heard
            .when(&situation.prompt)
            .is_some_and(|at| heard.now.saturating_sub(at) < ASKED_APART_PERIODS)
        {
            return Err(ActionError::Invalid("asked lately".into()));
        }
        let mut moves = Moves::default();
        remember_saying(&mut moves, &heard, &situation.prompt);
        let mut draft = EventDraft::new("situation_came_up");
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
        draft
            .payload
            .insert("told".into(), situation.told.clone().into());
        draft
            .payload
            .insert("said".into(), situation.prompt.clone().into());
        let mut map = BTreeMap::new();
        map.insert(
            "opened".to_string(),
            Value::from(period(state, &cast) as i64),
        );
        map.insert("prompt".to_string(), Value::from(situation.prompt));
        // A stranger at the door keeps the name they gave, whoever else
        // comes to stay while they wait.
        if candidate.kind == Kind::Visitor {
            if let Some(stranger) = stranger_name(state, &cast, candidate.topic) {
                map.insert("stranger".to_string(), Value::from(stranger));
            }
        }
        draft.changes.push(StateChange::SetComponent {
            entity: cast.notes,
            key: open_key(key),
            value: Value::Map(map),
        });
        draft.changes.push(StateChange::SetComponent {
            entity: cast.notes,
            key: format!("lives.kind.{}", candidate.kind.id()),
            value: (period(state, &cast) as i64).into(),
        });
        draft.changes.extend(moves.changes);
        Ok(draft)
    }
}

/// What an answer does, how it is told, and what the asker says.
pub(crate) fn outcome(
    state: &WorldState,
    cast: &Cast,
    candidate: &Candidate,
    answer: &str,
) -> Result<(Moves, String, String), ActionError> {
    let words = words_for(state, cast, candidate);
    let w = |template: &str| fill_owned(template, &words);
    let a = candidate.a;
    let b = candidate.b;
    let mut moves = Moves::default();
    let spend = |moves: &mut Moves| -> Result<(), ActionError> {
        let fund = cast
            .fund
            .ok_or_else(|| ActionError::Invalid("no fund".into()))?;
        if integer(state, fund.entity, fund.key).unwrap_or(0) < fund.amount {
            return Err(ActionError::Invalid("not enough in the fund".into()));
        }
        moves.add(
            state,
            fund.entity,
            fund.key,
            -fund.amount,
            i64::MIN / 4,
            i64::MAX / 4,
        );
        Ok(())
    };
    let people = cast_ids(state);
    let heard = Heard::of(state, cast);
    let say = |options: &[&str]| {
        let options = options.iter().map(|option| w(option)).collect::<Vec<_>>();
        pick_line(&options, &heard, a)
    };
    // Where an answer sends people shows on the scene.
    let go = |moves: &mut Moves, who: EntityId, place: EntityId| {
        if state.entity(who).is_some() {
            moves.set(who, AT, Value::Entity(place));
        }
    };
    let (gathering, quiet) = (cast.gathering, cast.quiet);
    let now_period = period(state, cast) as i64;
    let work_of = |who: EntityId| (cast.work)(state, who).unwrap_or(gathering);
    let (told, said) = match (candidate.kind, answer) {
        (Kind::Feud, "mend") => {
            let b = b.unwrap_or(a);
            moves.opinion(state, a, b, 35);
            moves.opinion(state, b, a, 35);
            moves.regard(state, a, 5);
            moves.regard(state, b, 5);
            go(&mut moves, a, gathering);
            go(&mut moves, b, gathering);
            (
                w("{a} and {b} talked it through"),
                w("We talked, {b} and me. It's better."),
            )
        }
        (Kind::Feud, "side") => {
            let b = b.unwrap_or(a);
            moves.regard(state, a, 12);
            moves.regard(state, b, -15);
            moves.opinion(state, b, a, -10);
            go(&mut moves, a, gathering);
            go(&mut moves, b, quiet);
            (
                w("You took {a}'s side against {b}"),
                w("Thank you. Someone sees it."),
            )
        }
        // Left to themselves, time takes a little of the edge off.
        (Kind::Feud, "leave") => {
            let b = b.unwrap_or(a);
            moves.opinion(state, a, b, 5);
            moves.opinion(state, b, a, 5);
            go(&mut moves, a, quiet);
            (
                w("{a} and {b} were left to sort it out"),
                w("Fine. I'll manage {b} myself."),
            )
        }
        (Kind::Feud, "lapse") => {
            let b = b.unwrap_or(a);
            moves.opinion(state, a, b, -10);
            moves.opinion(state, b, a, -10);
            go(&mut moves, a, quiet);
            (
                w("{a} and {b} stopped speaking"),
                w("Not a word from {b} since."),
            )
        }
        (Kind::Sweet, "ask") => {
            let b = b.unwrap_or(a);
            let free = partner(state, a).is_none() && partner(state, b).is_none();
            let yes =
                free && (opinion(state, b, a) >= 15 || mix(&[a.0, b.0, 99]).is_multiple_of(2));
            if yes {
                walk_out(state, &mut moves, a, b);
                moves.regard(state, a, 8);
                go(&mut moves, a, quiet);
                go(&mut moves, b, quiet);
                (
                    w("{a} and {b} started walking out together"),
                    w("{b} said yes!"),
                )
            } else {
                moves.opinion(state, a, b, -15);
                moves.lack(state, a, Need::Company, 20);
                go(&mut moves, a, quiet);
                (
                    w("{b} turned {a} down, kindly"),
                    w("{b} said no. Kindly, but no."),
                )
            }
        }
        // Told to wait, they spend time as friends first; told to wait a
        // second time, someone who has come to like them back is asked
        // in their own good time.
        (Kind::Sweet, "wait") => {
            let b = b.unwrap_or(a);
            let waited = entity_of(state, a, COURTING) == Some(b);
            let free = partner(state, a).is_none() && partner(state, b).is_none();
            if free && waited && opinion(state, b, a) >= 30 {
                walk_out(state, &mut moves, a, b);
                go(&mut moves, a, quiet);
                go(&mut moves, b, quiet);
                (
                    w("{a} took {a_their} time, and {a} and {b} started walking out together"),
                    w("We took it slowly, {b} and me. Worth the wait."),
                )
            } else {
                moves.set(a, COURTING, Value::Entity(b));
                moves.set(a, TOLD_TO_WAIT, true);
                moves.opinion(state, a, b, 6);
                moves.opinion(state, b, a, 6);
                go(&mut moves, a, gathering);
                go(&mut moves, b, gathering);
                (
                    w("{a} kept it to {a_themselves}, and spent more time with {b}"),
                    say(&[
                        "Maybe you're right. Maybe.",
                        "I'll wait. For now.",
                        "Best not rush it.",
                        "Not yet, then.",
                    ]),
                )
            }
        }
        // Nobody to ask, they work up the courage on their own, the second
        // time it comes to them: only someone as fond of them as they are
        // says yes.
        (Kind::Sweet, "lapse") => {
            let b = b.unwrap_or(a);
            let free = partner(state, a).is_none() && partner(state, b).is_none();
            let waited = entity_of(state, a, COURTING) == Some(b);
            // Told to wait, they wait to be told otherwise.
            let told = matches!(
                state
                    .entity(a)
                    .and_then(|entity| entity.component(TOLD_TO_WAIT)),
                Some(Value::Bool(true))
            );
            if free
                && waited
                && !told
                && opinion(state, b, a) >= ASKS_ALONE
                && opinion(state, a, b) >= ASKS_ALONE
            {
                walk_out(state, &mut moves, a, b);
                go(&mut moves, a, quiet);
                go(&mut moves, b, quiet);
                (
                    w("{a} asked {b} out on {a_their} own, and {b} said yes"),
                    w("I asked {b} myself in the end. Yes, {b} said!"),
                )
            } else {
                moves.set(a, COURTING, Value::Entity(b));
                moves.opinion(state, a, b, -5);
                moves.lack(state, a, Need::Company, 10);
                go(&mut moves, a, gathering);
                (
                    w("{a} never quite said anything to {b}"),
                    say(&[
                        "Never found the words.",
                        "I nearly said something. Nearly.",
                        "Some things stay unsaid.",
                        "It'll pass. I expect.",
                    ]),
                )
            }
        }
        (Kind::Learn, "teach") => {
            let b = b.unwrap_or(a);
            moves.lack(state, a, Need::Purpose, -50);
            moves.opinion(state, a, b, 18);
            moves.opinion(state, b, a, 10);
            moves.lack(state, b, Need::Rest, 15);
            moves.lack(state, b, Need::Purpose, -20);
            moves.regard(state, a, 8);
            moves.unhelped(state, a, -2);
            // Learning from one, they do not ask to learn from another for
            // a while.
            moves.set(
                cast.notes,
                &rest_key(&own_family(Kind::Learn, a)),
                now_period,
            );
            go(&mut moves, a, work_of(b));
            (w("{b} began teaching {a}"), w("{b}'s a patient teacher."))
        }
        (Kind::Learn, "not_now") | (Kind::Learn, "lapse") => {
            moves.lack(state, a, Need::Purpose, 10);
            moves.regard(state, a, -5);
            moves.unhelped(state, a, 1);
            // Told not now, they do not ask to learn from anyone else for
            // a while either.
            moves.set(
                cast.notes,
                &rest_key(&own_family(Kind::Learn, a)),
                now_period,
            );
            go(&mut moves, a, quiet);
            (
                w("{a} was told not now"),
                say(&[
                    "Another time, then.",
                    "Maybe next month.",
                    "I'll ask again. Later.",
                    "Fair enough.",
                ]),
            )
        }
        (Kind::Short, "fund") => {
            spend(&mut moves)?;
            moves.lack(state, a, Need::Money, -55);
            moves.regard(state, a, 15);
            moves.unhelped(state, a, -2);
            go(&mut moves, a, quiet);
            (
                w("The {settlement} tided {a} over"),
                w("I'll pay it back, every bit."),
            )
        }
        (Kind::Short, "friend") => {
            let friend = best_friend(state, a)
                .ok_or_else(|| ActionError::Invalid("no friend to ask".into()))?;
            moves.lack(state, a, Need::Money, -45);
            moves.lack(state, friend, Need::Money, 15);
            moves.opinion(state, a, friend, 15);
            moves.opinion(state, friend, a, -3);
            moves.unhelped(state, a, -2);
            go(&mut moves, a, work_of(friend));
            (
                w("{friend} helped {a} out"),
                w("{friend} came through for me."),
            )
        }
        (Kind::Short, "jobs") => {
            moves.lack(state, a, Need::Money, -30);
            moves.lack(state, a, Need::Rest, 15);
            moves.regard(state, a, 4);
            moves.unhelped(state, a, -1);
            go(&mut moves, a, gathering);
            (
                w("{a} did odd jobs round the {settlement}"),
                say(&[
                    "Odd jobs pay. Not much, but they pay.",
                    "Painted three gates and a shed. It adds up.",
                    "Tired, but the {unit} is paid for.",
                ]),
            )
        }
        (Kind::Short, "manage") | (Kind::Short, "lapse") => {
            moves.regard(state, a, -8);
            moves.unhelped(state, a, 1);
            moves.lack(state, a, Need::Money, 5);
            go(&mut moves, a, work_of(a));
            (
                w("{a} scraped by"),
                say(&[
                    "Right. I'll manage.",
                    "I'll find a way.",
                    "Tighten the belt, then.",
                    "Somehow. I'll manage.",
                ]),
            )
        }
        (Kind::Lonely, "invite") => {
            moves.lack(state, a, Need::Company, -55);
            moves.unhelped(state, a, -2);
            let met = people
                .iter()
                .copied()
                .filter(|o| *o != a)
                .max_by_key(|o| (lack(state, *o, Need::Company), mix(&[a.0, o.0])));
            moves.regard(state, a, 6);
            go(&mut moves, a, gathering);
            if let Some(met) = met {
                go(&mut moves, met, gathering);
            }
            match met {
                Some(met) => {
                    moves.opinion(state, a, met, 15);
                    moves.opinion(state, met, a, 12);
                    moves.lack(state, met, Need::Company, -25);
                    let met_name = name(state, met);
                    (
                        format!(
                            "{} spent an evening at {} with {met_name}",
                            name(state, a),
                            name(state, cast.gathering)
                        ),
                        format!("{met_name} and I got talking. Lovely evening."),
                    )
                }
                None => (
                    w("{a} spent an evening at {gathering}"),
                    w("That did me good."),
                ),
            }
        }
        (Kind::Lonely, "space") | (Kind::Lonely, "lapse") => {
            moves.lack(state, a, Need::Company, 8);
            moves.regard(state, a, -3);
            moves.unhelped(state, a, 1);
            go(&mut moves, a, quiet);
            (
                w("{a} kept to {a_themselves}"),
                say(&[
                    "Maybe tomorrow.",
                    "I'm fine on my own. Mostly.",
                    "Quiet suits me. For now.",
                    "Another evening, perhaps.",
                ]),
            )
        }
        (Kind::Worn, "rest") => {
            moves.lack(state, a, Need::Rest, -65);
            moves.lack(state, a, Need::Money, 10);
            moves.regard(state, a, 8);
            moves.unhelped(state, a, -2);
            go(&mut moves, a, quiet);
            (
                w("{a} took a day off"),
                say(&[
                    "Slept till noon. Bliss.",
                    "A whole day off. I'd forgotten what that was.",
                    "Rested. Human again.",
                ]),
            )
        }
        (Kind::Worn, "push") => {
            moves.lack(state, a, Need::Rest, 10);
            moves.lack(state, a, Need::Money, -20);
            moves.regard(state, a, -6);
            moves.unhelped(state, a, 1);
            go(&mut moves, a, work_of(a));
            (
                w("{a} pushed on"),
                say(&[
                    "One more push, then.",
                    "Fine. Back to it.",
                    "I'll keep going. For now.",
                ]),
            )
        }
        (Kind::Worn, "lapse") => {
            moves.lack(state, a, Need::Rest, -35);
            moves.regard(state, a, -4);
            moves.unhelped(state, a, 1);
            go(&mut moves, a, quiet);
            (
                w("{a} took the day anyway"),
                say(&[
                    "I needed that. Sorry.",
                    "Had to stop. Couldn't go on.",
                    "Took the day. Don't be cross.",
                ]),
            )
        }
        (Kind::Party, "party") => {
            spend(&mut moves)?;
            let b = b.unwrap_or(a);
            for p in &people {
                moves.lack(state, *p, Need::Company, -30);
                moves.opinion(state, *p, a, 5);
                moves.opinion(state, *p, b, 5);
                moves.regard(state, *p, 3);
                go(&mut moves, *p, gathering);
            }
            moves.set(a, CELEBRATED, period(state, cast) as i64);
            (
                w("The whole {settlement} came to {a} and {b}'s party at {gathering}"),
                w("Best night in years!"),
            )
        }
        (Kind::Party, "potluck") => {
            let b = b.unwrap_or(a);
            for p in &people {
                moves.lack(state, *p, Need::Company, -15);
                moves.opinion(state, *p, a, 2);
                moves.opinion(state, *p, b, 2);
                go(&mut moves, *p, gathering);
            }
            moves.set(a, CELEBRATED, period(state, cast) as i64);
            (
                w("Everyone brought a dish to {a} and {b}'s do at {gathering}"),
                w("So many dishes, and not one the same!"),
            )
        }
        (Kind::Party, "quiet") | (Kind::Party, "lapse") => {
            moves.set(a, CELEBRATED, period(state, cast) as i64);
            go(&mut moves, a, quiet);
            if let Some(b) = b {
                go(&mut moves, b, quiet);
            }
            (w("{a} and {b} kept it small"), w("Just us two. Perfect."))
        }
        // Nobody answered the door: a stranger stays on only where people
        // are glad of the place, and moves on from one they are not.
        (Kind::Visitor, "lapse") if !welcoming(state) => (
            w("{b} waited a day for an answer, then moved on with {way_out}"),
            w("Shame. {b} seemed nice."),
        ),
        (Kind::Visitor, "welcome") | (Kind::Visitor, "lapse") => {
            let visitors = cast
                .visitors
                .ok_or_else(|| ActionError::Invalid("nobody comes here".into()))?;
            // Whoever was at the door takes the next room free, should
            // someone else have come to stay while they waited.
            let visitor = b
                .filter(|visitor| state.entity(*visitor).is_none())
                .or_else(|| next_visitor(state, cast))
                .ok_or_else(|| ActionError::Invalid("no room for anyone".into()))?;
            let seed = mix(&[visitor.0, candidate.topic, 17]);
            let visitor_name =
                stranger_at_door(state, cast, candidate).unwrap_or_else(|| "A stranger".into());
            let visitor_name = visitor_name.as_str();
            let (_, job) = pick(visitors.trades, seed / 7)
                .copied()
                .unwrap_or(("traveller", "traveller"));
            let mut newcomer =
                Entity::new(visitor, visitors.kind).with_component("name", visitor_name);
            for (key, value) in (visitors.components)(visitor_name, job) {
                newcomer = newcomer.with_component(key, value);
            }
            newcomer = newcomer
                .with_component("lives.newcomer", true)
                .with_component(AT, Value::Entity(gathering));
            moves.changes.push(StateChange::CreateEntity(newcomer));
            // Asked to stay, the host is glad of the player for it; a
            // stranger who stays on unasked owes the player nothing.
            if answer != "lapse" {
                moves.regard(state, a, 5);
            }
            if answer == "lapse" {
                (
                    w("{b}, a {trade} from {origin}, stayed on, and nobody minded"),
                    w("Looks like {b}'s staying. Fine by me."),
                )
            } else {
                (
                    w("{b}, a {trade} from {origin}, came to stay"),
                    w("Plenty of room here. Welcome, {b}."),
                )
            }
        }
        (Kind::Visitor, "decline") => (w("{b} moved on with {way_out}"), w("Maybe another time.")),
        (Kind::Leaving, "stay") => {
            moves.regard(state, a, 30);
            for need in Need::ALL {
                moves.lack(state, a, need, -20);
            }
            moves.unhelped(state, a, -20);
            go(&mut moves, a, gathering);
            (
                w("{a} was asked to stay, and did"),
                w("All right. I'll stay. For now."),
            )
        }
        (Kind::Leaving, "go") | (Kind::Leaving, "lapse") => {
            moves.set(a, GONE, true);
            if let Some(p) = partner(state, a) {
                moves.unset(state, p, PARTNER);
                moves.set(p, PARTED, now_period);
                moves.lack(state, p, Need::Company, 40);
            }
            moves.unset(state, a, PARTNER);
            (w("{a} left the {settlement}"), w("I'll write. I promise."))
        }
        (Kind::RoughPatch, "talk") => {
            let b = b.unwrap_or(a);
            moves.opinion(state, a, b, 25);
            moves.opinion(state, b, a, 25);
            go(&mut moves, a, gathering);
            go(&mut moves, b, gathering);
            (
                w("{a} and {b} talked it out"),
                w("We're all right, {b} and me."),
            )
        }
        (Kind::RoughPatch, "apart") | (Kind::RoughPatch, "lapse") => {
            let b = b.unwrap_or(a);
            moves.unset(state, a, PARTNER);
            moves.unset(state, b, PARTNER);
            moves.set(a, PARTED, now_period);
            moves.set(b, PARTED, now_period);
            moves.opinion(state, a, b, -10);
            moves.opinion(state, b, a, -10);
            moves.lack(state, a, Need::Company, 30);
            moves.lack(state, b, Need::Company, 30);
            go(&mut moves, a, quiet);
            go(&mut moves, b, work_of(b));
            (
                w("{a} and {b} went their separate ways"),
                w("It's for the best. I think."),
            )
        }
        (Kind::Warming, "stay") => {
            moves.set(a, &door_key(Kind::Warming), now_period);
            moves.regard(state, a, 6);
            moves.lack(state, a, Need::Company, -15);
            (w("{a} shared a quiet moment with you"), w("That was nice."))
        }
        (Kind::Warming, "wave") => {
            moves.set(a, &door_key(Kind::Warming), now_period);
            moves.regard(state, a, 2);
            (w("{a} waved you on your way"), w("See you, then."))
        }
        // A moment that passes has passed: the friendship goes on to the
        // next door without it.
        (Kind::Warming, "lapse") => {
            moves.set(a, &door_key(Kind::Warming), now_period);
            let said = pick(&MISSED, mix(&[a.0, 3])).copied().unwrap_or("Oh well.");
            (w("{a} looked for you"), w(said))
        }
        (Kind::Invite, "go") => {
            moves.set(a, &door_key(Kind::Invite), now_period);
            moves.regard(state, a, 8);
            moves.lack(state, a, Need::Company, -20);
            moves.lack(state, a, Need::Purpose, -10);
            (
                w("{a} took you somewhere of theirs"),
                w("I'm glad you came."),
            )
        }
        (Kind::Invite, "another") => {
            moves.set(a, &door_key(Kind::Invite), now_period);
            moves.regard(state, a, 2);
            (
                w("{a} asked you along, another time"),
                w("Another time, then."),
            )
        }
        (Kind::Invite, "lapse") => {
            moves.set(a, &door_key(Kind::Invite), now_period);
            let said = pick(&MISSED, mix(&[a.0, 9]))
                .copied()
                .unwrap_or("Never mind.");
            (w("{a} went on {a_their} own"), w(said))
        }
        (Kind::Confide, "keep") => {
            moves.set(a, &door_key(Kind::Confide), now_period);
            moves.regard(state, a, 10);
            moves.lack(state, a, Need::Company, -20);
            (
                w("{a} told you something {a_they}'d never told anyone"),
                w("Thank you. That's a weight off."),
            )
        }
        (Kind::Confide, "share") => {
            moves.set(a, &door_key(Kind::Confide), now_period);
            moves.regard(state, a, 5);
            moves.lack(state, a, Need::Purpose, -15);
            (
                w("{a} told you a secret, and you told {a_them} to share it"),
                w("Maybe you're right. Maybe I will."),
            )
        }
        (Kind::Confide, "lapse") => {
            let said = pick(&UNSAID, mix(&[a.0, now_period as u64, 13]))
                .copied()
                .unwrap_or("Never mind. It was nothing.");
            (w("{a} almost told you something"), w(said))
        }
        (Kind::Favour, "word") => {
            moves.set(a, &door_key(Kind::Favour), now_period);
            let friend = best_friend(state, a).unwrap_or(a);
            moves.regard(state, friend, 12);
            moves.regard(state, a, 3);
            (
                w("{a} put in a good word for you with {friend}"),
                w("I told {friend} what you're like. They'll see."),
            )
        }
        (Kind::Favour, "help") => {
            moves.set(a, &door_key(Kind::Favour), now_period);
            moves.regard(state, a, 3);
            // Whoever is worst off, besides the one doing the favour.
            let helped = people
                .iter()
                .copied()
                .filter(|p| *p != a && enrolled(state, *p) && !gone(state, *p))
                .max_by_key(|p| {
                    let worst = Need::ALL.iter().map(|n| lack(state, *p, *n)).max();
                    (worst.unwrap_or(0), std::cmp::Reverse(p.0))
                });
            match helped {
                Some(helped) => {
                    let need = Need::ALL
                        .into_iter()
                        .max_by_key(|n| lack(state, helped, *n))
                        .unwrap_or(Need::Company);
                    moves.lack(state, helped, need, -25);
                    moves.regard(state, helped, 5);
                    moves.opinion(state, helped, a, 10);
                    let helped_name = name(state, helped);
                    (
                        w("{a} helped {helped} out, as a favour to you")
                            .replace("{helped}", &helped_name),
                        w("Consider it done."),
                    )
                }
                None => (w("{a} offered you a favour"), w("Consider it done.")),
            }
        }
        (Kind::Favour, "nothing") => {
            moves.set(a, &door_key(Kind::Favour), now_period);
            moves.regard(state, a, 5);
            (
                w("{a} offered you a favour, and you asked for nothing"),
                w("Then I owe you twice."),
            )
        }
        (Kind::Favour, "lapse") => (w("{a}'s offer went unanswered"), w("Another time, then.")),
        (Kind::Keepsake, "keep") => {
            moves.set(a, &door_key(Kind::Keepsake), now_period);
            moves.regard(state, a, 5);
            (w("{a} gave you {keepsake}"), w("It suits you, having it."))
        }
        (Kind::Keepsake, "show") => {
            moves.set(a, &door_key(Kind::Keepsake), now_period);
            moves.regard(state, a, 3);
            for p in &people {
                if *p != a && enrolled(state, *p) {
                    moves.regard(state, *p, 2);
                }
            }
            (
                w("{a} gave you {keepsake}, and you put it where all can see"),
                w("Everyone's asking about it!"),
            )
        }
        (Kind::Keepsake, "lapse") => (
            w("{a} meant to give you something"),
            w("I'll find another moment."),
        ),
        (Kind::Cold, "sorry") => {
            moves.regard(state, a, 15);
            (
                w("You made your peace with {a}"),
                w("...Fine. Apology accepted."),
            )
        }
        // Left be, a grudge cools a little on its own; ignored, it keeps.
        (Kind::Cold, "leave") => {
            moves.regard(state, a, 3);
            go(&mut moves, a, quiet);
            (w("You left {a} be, and it cooled a little"), w("Suits me."))
        }
        (Kind::Cold, "lapse") => {
            go(&mut moves, a, quiet);
            (w("{a} kept {a_their} distance from you"), w("Suits me."))
        }
        _ => return Err(ActionError::Invalid(format!("no answer {answer}"))),
    };
    Ok((moves, told, said))
}

/// Two people start walking out together.
pub(crate) fn walk_out(state: &WorldState, moves: &mut Moves, a: EntityId, b: EntityId) {
    moves.set(a, PARTNER, Value::Entity(b));
    moves.set(b, PARTNER, Value::Entity(a));
    for who in [a, b] {
        moves.unset(state, who, COURTING);
        moves.unset(state, who, TOLD_TO_WAIT);
    }
    moves.opinion(state, a, b, 15);
    moves.opinion(state, b, a, 20);
    moves.lack(state, a, Need::Company, -40);
    moves.lack(state, b, Need::Company, -40);
}

/// Whether people are glad of the place: their goodwill toward it, on
/// the whole, is more than everyday life gives on its own.
pub(crate) fn welcoming(state: &WorldState) -> bool {
    let people = cast_ids(state);
    let total = people
        .iter()
        .map(|person| integer(state, *person, REGARD).unwrap_or(0))
        .sum::<i64>();
    total >= 8 * people.len() as i64
}

pub(crate) fn closing(state: &WorldState, cast: &Cast, candidate: &Candidate, moves: &mut Moves) {
    moves.unset(state, cast.notes, &open_key(&candidate.key()));
    moves.set(
        cast.notes,
        &rest_key(&candidate.family()),
        period(state, cast) as i64,
    );
}
