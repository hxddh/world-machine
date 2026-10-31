//! The situations put to the player: their kinds, who could be in one
//! now, the words each is told in, and the situations open.

use crate::*;

/// The kinds of situation this System composes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Feud,
    Sweet,
    Learn,
    Short,
    Lonely,
    Worn,
    Party,
    Visitor,
    Leaving,
    RoughPatch,
    /// Someone warming to the player shares a small moment with them.
    Warming,
    /// A friend tells the player something they have told nobody.
    Confide,
    /// A friend asks the player along somewhere of theirs.
    Invite,
    /// A close friend offers the player a favour.
    Favour,
    /// A close friend gives the player something to keep.
    Keepsake,
    /// Someone with a grudge against the player lets it show.
    Cold,
}

impl Kind {
    pub(crate) const ALL: [Kind; 16] = [
        Kind::Feud,
        Kind::Sweet,
        Kind::Learn,
        Kind::Short,
        Kind::Lonely,
        Kind::Worn,
        Kind::Party,
        Kind::Visitor,
        Kind::Leaving,
        Kind::RoughPatch,
        Kind::Warming,
        Kind::Confide,
        Kind::Invite,
        Kind::Favour,
        Kind::Keepsake,
        Kind::Cold,
    ];

    pub(crate) fn id(self) -> &'static str {
        match self {
            Kind::Feud => "feud",
            Kind::Sweet => "sweet",
            Kind::Learn => "learn",
            Kind::Short => "short",
            Kind::Lonely => "lonely",
            Kind::Worn => "worn",
            Kind::Party => "party",
            Kind::Visitor => "visitor",
            Kind::Leaving => "leaving",
            Kind::RoughPatch => "rough",
            Kind::Warming => "warming",
            Kind::Confide => "confide",
            Kind::Invite => "invite",
            Kind::Favour => "favour",
            Kind::Keepsake => "keepsake",
            Kind::Cold => "cold",
        }
    }

    /// Someone asking the player for help, which nobody with a grudge
    /// against them does.
    pub(crate) fn asks_for_help(self) -> bool {
        matches!(
            self,
            Kind::Sweet | Kind::Learn | Kind::Short | Kind::Lonely | Kind::Worn
        )
    }

    pub(crate) fn from_id(id: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.id() == id)
    }

    /// How many periods the same people's same trouble rests before it can
    /// come up again.
    pub(crate) fn rests(self) -> u64 {
        match self {
            Kind::Visitor => 0,
            Kind::Party => 60,
            Kind::Warming | Kind::Confide | Kind::Invite | Kind::Favour | Kind::Keepsake => 12,
            // Two people not speaking are asked about every season or so,
            // not every month.
            Kind::Feud => 40,
            Kind::Learn => 30,
            _ => 24,
        }
    }
}

/// A situation that could be put to the player now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub kind: Kind,
    /// Whose it is: they ask it.
    pub a: EntityId,
    /// Who else it is about, if anyone.
    pub b: Option<EntityId>,
    /// What it is over, as an index into the Pack's words.
    pub topic: u64,
}

impl Candidate {
    /// What the situation is known by: the same people's same trouble over
    /// the same thing is the same situation.
    pub fn key(&self) -> String {
        match self.b {
            Some(b) => format!("{}.{}.{}.{}", self.kind.id(), self.a, b, self.topic),
            None => format!("{}.{}.{}", self.kind.id(), self.a, self.topic),
        }
    }

    /// The same people's same kind of trouble, over whatever it is.
    pub(crate) fn family(&self) -> String {
        match self.b {
            Some(b) => format!(
                "{}.{}.{}",
                self.kind.id(),
                self.a.0.min(b.0),
                self.a.0.max(b.0)
            ),
            None => format!("{}.{}", self.kind.id(), self.a),
        }
    }

    pub(crate) fn parse(key: &str) -> Option<Candidate> {
        let parts = key.split('.').collect::<Vec<_>>();
        let kind = Kind::from_id(parts.first()?)?;
        let a = EntityId::new(parts.get(1)?.parse().ok()?);
        let (b, topic) = match parts.len() {
            4 => (
                Some(EntityId::new(parts[2].parse().ok()?)),
                parts[3].parse().ok()?,
            ),
            3 => (None, parts[2].parse().ok()?),
            _ => return None,
        };
        Some(Candidate { kind, a, b, topic })
    }
}

/// Someone's own kind of trouble, whoever else it is about: rested after
/// they are turned away, so they do not turn straight to someone else.
pub(crate) fn own_family(kind: Kind, a: EntityId) -> String {
    format!("{}.{a}", kind.id())
}

pub(crate) fn open_key(key: &str) -> String {
    format!("lives.open.{key}")
}

pub(crate) fn rest_key(family: &str) -> String {
    format!("lives.rest.{family}")
}

/// The situations open now: each one's key and the period it opened.
pub fn open(state: &WorldState, cast: &Cast) -> Vec<(String, i64)> {
    let Some(notes) = state.entity(cast.notes) else {
        return Vec::new();
    };
    notes
        .components
        .iter()
        .filter_map(|(key, value)| {
            let key = key.strip_prefix("lives.open.")?;
            match value {
                Value::Map(map) => match map.get("opened") {
                    Some(Value::Integer(at)) => Some((key.to_string(), *at)),
                    _ => None,
                },
                _ => None,
            }
        })
        .collect()
}

pub(crate) fn open_map<'a>(
    state: &'a WorldState,
    cast: &Cast,
    key: &str,
) -> Option<&'a BTreeMap<String, Value>> {
    match state.entity(cast.notes)?.component(&open_key(key))? {
        Value::Map(map) => Some(map),
        _ => None,
    }
}

/// How the place stands, as one number: everyone's goodwill toward it.
pub(crate) fn standing_of(state: &WorldState, people: &[EntityId]) -> u64 {
    people
        .iter()
        .map(|person| integer(state, *person, REGARD).unwrap_or(0))
        .sum::<i64>()
        .unsigned_abs()
}

pub(crate) fn next_visitor(state: &WorldState, cast: &Cast) -> Option<EntityId> {
    let visitors = cast.visitors?;
    state
        .free_id_in(world_core::IdBlock::new(visitors.first, visitors.room))
        .ok()
}

/// The situations the World could put to the player now, most pressing
/// first.
pub fn candidates(world: &World, cast: &Cast) -> Vec<Candidate> {
    let state = world.state();
    let people = living(world, cast)
        .into_iter()
        .filter(|id| enrolled(state, *id))
        .collect::<Vec<_>>();
    let now = period(state, cast);
    let mut found = Vec::new();
    let room = room_for_keepsake(world, cast);
    let topic = |a: EntityId, b: u64, salt: u64| mix(&[a.0, b, salt, now / 30]) % 1000;
    for &a in &people {
        for &b in &people {
            if a == b || (cast.kept)(a, b) || (cast.kept)(b, a) {
                continue;
            }
            let (ab, ba) = (opinion(state, a, b), opinion(state, b, a));
            if ab <= -30 && ab <= ba && partner(state, a) != Some(b) {
                found.push((
                    ab.abs() + 20,
                    Candidate {
                        kind: Kind::Feud,
                        a,
                        b: Some(b),
                        topic: topic(a, b.0, 1),
                    },
                ));
            }
            if ab >= 45
                && generations::may_court(state, cast, a, b)
                && partner(state, a).is_none()
                && partner(state, b).is_none()
                && !heartsore(state, a, now)
                && !heartsore(state, b, now)
            {
                found.push((
                    ab,
                    Candidate {
                        kind: Kind::Sweet,
                        a,
                        b: Some(b),
                        topic: topic(a, b.0, 2),
                    },
                ));
            }
            // A couple one of whom has cooled on the other has hit a rough
            // patch, long before they stop speaking.
            if partner(state, a) == Some(b) && a.0 < b.0 && (ab <= ROUGH || ba <= ROUGH) {
                found.push((
                    70,
                    Candidate {
                        kind: Kind::RoughPatch,
                        a,
                        b: Some(b),
                        topic: topic(a, b.0, 3),
                    },
                ));
            }
            if partner(state, a) == Some(b) && a.0 < b.0 && integer(state, a, CELEBRATED).is_none()
            {
                found.push((
                    60,
                    Candidate {
                        kind: Kind::Party,
                        a,
                        b: Some(b),
                        topic: topic(a, b.0, 4),
                    },
                ));
            }
            if lack(state, a, Need::Purpose) >= 55
                && ab >= 0
                && (cast.work)(state, a) != (cast.work)(state, b)
                && (cast.work)(state, b).is_some()
            {
                found.push((
                    lack(state, a, Need::Purpose) - 10,
                    Candidate {
                        kind: Kind::Learn,
                        a,
                        b: Some(b),
                        topic: topic(a, b.0, 5),
                    },
                ));
            }
        }
        let lacks = |need| lack(state, a, need);
        if lacks(Need::Money) >= 70 {
            found.push((
                lacks(Need::Money),
                Candidate {
                    kind: Kind::Short,
                    a,
                    b: None,
                    topic: topic(a, 0, 6),
                },
            ));
        }
        // Nobody brings their loneliness to a newcomer in the kind first
        // days a Pack keeps (`Cast::kind`); it waits until they are over.
        if lacks(Need::Company) >= 70 && !(cast.kind)(state) {
            found.push((
                lacks(Need::Company),
                Candidate {
                    kind: Kind::Lonely,
                    a,
                    b: None,
                    topic: topic(a, 0, 7),
                },
            ));
        }
        if lacks(Need::Rest) >= 80 {
            found.push((
                lacks(Need::Rest),
                Candidate {
                    kind: Kind::Worn,
                    a,
                    b: None,
                    topic: topic(a, 0, 8),
                },
            ));
        }
        // Doors a friendship opens, each once, in turn: a secret, then a
        // favour, then a keepsake some periods after. Anyone's real trouble
        // comes first.
        let regard = integer(state, a, REGARD).unwrap_or(0);
        let door = |kind| door_opened(state, a, kind);
        let door_candidate = |kind| Candidate {
            kind,
            a,
            b: None,
            topic: mix(&[a.0, 11]) % 1000,
        };
        // A door that has waited grows more pressing, so a friendship's
        // moments come round in weeks, not whenever nobody is in trouble.
        let waited = |before: Kind| door(before).map_or(0, |at| (now as i64 - at).clamp(0, 15));
        if regard >= WARM && door(Kind::Warming).is_some() && door(Kind::Confide).is_none() {
            found.push((40 + waited(Kind::Warming), door_candidate(Kind::Confide)));
        }
        if regard >= DEAR && door(Kind::Confide).is_some() && door(Kind::Invite).is_none() {
            found.push((42 + waited(Kind::Confide), door_candidate(Kind::Invite)));
        }
        if regard >= CLOSE && door(Kind::Invite).is_some() && door(Kind::Favour).is_none() {
            found.push((45 + waited(Kind::Invite), door_candidate(Kind::Favour)));
        }
        if regard >= CLOSE
            && room
            && door(Kind::Keepsake).is_none()
            && door(Kind::Favour).is_some_and(|at| now as i64 - at >= 3)
        {
            found.push((47 + waited(Kind::Favour), door_candidate(Kind::Keepsake)));
        }
        if regard <= GRUDGE {
            found.push((
                55,
                Candidate {
                    kind: Kind::Cold,
                    a,
                    b: None,
                    topic: topic(a, 0, 12),
                },
            ));
        }
        let worst = Need::ALL.iter().map(|n| lacks(*n)).max().unwrap_or(0);
        // Someone at the end of their tether with no goodwill left thinks
        // of leaving; so does someone who asked for help through hard
        // times, again and again, and was let down.
        let at_the_end = integer(state, a, REGARD).unwrap_or(0) <= -20 && worst >= 70;
        let let_down =
            unhelped(state, a) >= LET_DOWN && integer(state, a, REGARD).unwrap_or(0) < 15;
        if !(cast.stays)(a) && !settled(state, a) && (at_the_end || let_down) {
            found.push((
                90,
                Candidate {
                    kind: Kind::Leaving,
                    a,
                    b: None,
                    topic: topic(a, 0, 9),
                },
            ));
        }
    }
    // Strangers come by now and then, and more often to a place that is
    // nearly empty; only while there is a name nobody here or gone has had,
    // so nobody comes to stay as "A stranger".
    let few = people.len() * 2 < cast.most_people;
    if people.len() < cast.most_people
        && now % if few { 5 } else { 7 } == 4
        && !next_names(state, cast, 1).is_empty()
    {
        if let Some(visitor) = next_visitor(state, cast) {
            found.push((
                if few { 120 } else { 105 },
                Candidate {
                    kind: Kind::Visitor,
                    a: cast.host,
                    b: Some(visitor),
                    // Who turns up depends on how the place is doing:
                    // two branches of a World meet different strangers.
                    topic: mix(&[visitor.0, standing_of(state, &people), now]) % 1000,
                },
            ));
        }
    }
    // Most pressing first, though a kind of trouble heard lately waits its
    // turn; among equals, whichever the day's chance favours.
    for (score, candidate) in &mut found {
        let last = integer(
            state,
            cast.notes,
            &format!("lives.kind.{}", candidate.kind.id()),
        );
        if last.is_some_and(|last| now as i64 - last < 6) {
            *score -= 45;
        }
    }
    found.sort_by_key(|(score, candidate)| {
        (-score, mix(&[now, candidate.a.0, candidate.kind as u64]))
    });
    found
        .into_iter()
        .map(|(_, candidate)| candidate)
        // Nobody with a grudge against the player asks them for help.
        .filter(|candidate| {
            !(candidate.kind.asks_for_help()
                && integer(state, candidate.a, REGARD).unwrap_or(0) <= GRUDGE)
        })
        .filter(|candidate| {
            let rested = |family: &str| {
                integer(state, cast.notes, &rest_key(family))
                    .is_none_or(|last| now as i64 - last >= candidate.kind.rests() as i64)
            };
            rested(&candidate.family())
                && rested(&own_family(candidate.kind, candidate.a))
                && open_map(state, cast, &candidate.key()).is_none()
        })
        .collect()
}

/// One way to answer a situation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub id: &'static str,
    pub title: String,
    /// Why it cannot be given now, if it cannot.
    pub unavailable: Option<String>,
}

/// A situation as it is put to the player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Situation {
    pub key: String,
    pub kind: Kind,
    pub asker: EntityId,
    pub about: Option<EntityId>,
    pub prompt: String,
    /// How it is told when it comes up.
    pub told: String,
    pub answers: Vec<Answer>,
}

pub(crate) fn words_for(
    state: &WorldState,
    cast: &Cast,
    candidate: &Candidate,
) -> Vec<(&'static str, String)> {
    let mut words = vec![
        ("a", name(state, candidate.a)),
        ("unit", cast.unit.to_string()),
        ("settlement", cast.settlement.to_string()),
        ("short_of", cast.short_of.to_string()),
        ("gathering", name(state, cast.gathering)),
        (
            "topic",
            pick(cast.topics, candidate.topic)
                .copied()
                .unwrap_or("nothing much")
                .into(),
        ),
        (
            "outing",
            pick(cast.outings, candidate.topic)
                .copied()
                .unwrap_or("take a walk")
                .into(),
        ),
    ];
    if let Some(b) = candidate.b {
        if candidate.kind == Kind::Visitor {
            if let Some(visitors) = cast.visitors {
                let seed = mix(&[b.0, candidate.topic, 17]);
                words.push((
                    "b",
                    stranger_at_door(state, cast, candidate).unwrap_or_else(|| "A stranger".into()),
                ));
                let (trade, _) = pick(visitors.trades, seed / 7)
                    .copied()
                    .unwrap_or(("traveller", "traveller"));
                words.push(("trade", trade.into()));
                words.push((
                    "origin",
                    pick(visitors.origins, seed / 13)
                        .copied()
                        .unwrap_or("far away")
                        .into(),
                ));
                words.push(("way_out", visitors.way_out.into()));
            }
        } else {
            words.push(("b", name(state, b)));
            words.extend(pronouns(state, cast, b).slots(&B_SLOTS));
            if let Some(work) = (cast.work)(state, b) {
                words.push(("work", name(state, work)));
            }
        }
    }
    words.extend(pronouns(state, cast, candidate.a).slots(&A_SLOTS));
    // How people leave, for anyone thinking of it, not only a stranger.
    if let Some(visitors) = &cast.visitors {
        if !words.iter().any(|(slot, _)| *slot == "way_out") {
            words.push(("way_out", visitors.way_out.into()));
        }
    }
    if let Some(friend) = best_friend(state, candidate.a) {
        words.push(("friend", name(state, friend)));
    }
    if candidate.kind == Kind::Keepsake {
        let what = fill_owned(
            keepsake_of((cast.voice)(candidate.a), (cast.traits)(candidate.a)),
            &words,
        );
        words.push(("keepsake", what));
    }
    words
}

/// Which of the five doors a kind is, if it is one.
pub(crate) fn door_index(kind: Kind) -> Option<usize> {
    [
        Kind::Warming,
        Kind::Confide,
        Kind::Invite,
        Kind::Favour,
        Kind::Keepsake,
    ]
    .iter()
    .position(|door| *door == kind)
}

/// The scene someone opens at a door: their own words.
pub(crate) fn door_scene(cast: &Cast, candidate: &Candidate) -> Option<Scene> {
    let index = door_index(candidate.kind)?;
    Some(scene_of(
        (cast.voice)(candidate.a),
        (cast.traits)(candidate.a),
        index,
    ))
}

/// What someone leaves for the player while they are away.
pub(crate) const LEFT_FOR_YOU: [&str; 6] = [
    "a note under the door",
    "a sketch of the {gathering}",
    "a jar of something homemade",
    "a photograph from while you were gone",
    "a pressed flower in an envelope",
    "a postcard with a few lines on it",
];

/// What someone says when a moment they offered passes by.
pub(crate) const MISSED: [&str; 8] = [
    "Oh well. Another time.",
    "Never mind. It'll come round.",
    "I looked for you, that's all.",
    "Maybe next time.",
    "It was only a small thing.",
    "Went on my own. It was fine.",
    "You were busy. I understand.",
    "Some other day, then.",
];

/// What someone gives a newcomer after their first deed.
/// How many things have been left for the player, not counting welcomes.
pub(crate) const LEFT: &str = "lives.left";

/// The last season something was left for the player to mark.
pub(crate) const SEASON_GIFT: &str = "lives.season_gift";

/// What someone says when what they meant to tell the player goes untold.
pub(crate) const UNSAID: [&str; 6] = [
    "Never mind. It was nothing.",
    "Forget I said anything.",
    "Another day, maybe. It'll keep.",
    "It's fine. I'll tell you sometime.",
    "Oh, it doesn't matter now.",
    "I'll save it for a quieter day.",
];

/// What someone says leaving the player something as a season turns, the
/// next of these each season.
pub(crate) const SEASON_NOTES: [&str; 14] = [
    "Something for the turn of the season.",
    "A new season, and this made me think of you.",
    "The season's turning. I wanted you to have this.",
    "For the season ahead. May it be a kind one.",
    "Saw this and thought of you. Happy new season.",
    "Every season deserves a small gift. Here's yours.",
    "The days are changing. This one's for you.",
    "Kept this back for the season's first day.",
    "To mark the turn of the season, from me to you.",
    "Here's to the season coming in.",
    "A little something, now the season's changed.",
    "New season, same you. Thought you'd like this.",
    "For the start of a fresh season.",
    "It felt like a day for giving. Happy new season.",
];

pub(crate) const WELCOME: [&str; 4] = [
    "a hand-drawn map of the {settlement}",
    "a smooth stone from by the {gathering}",
    "a little welcome card, signed by everyone",
    "a paper star to hang up",
];

/// How a newcomer is greeted: who greets them, and a nudge to make
/// something.
pub(crate) const GREETINGS: [&str; 3] = [
    "Hello, you're new! I'm {name}. Build us something, if you like.",
    "Oh, a new face! I'm {name}. Welcome to the {settlement}.",
    "Welcome to the {settlement}! I'm {name}. Make yourself at home.",
];

/// How someone introduces themselves once the player has been about a
/// while: met on a quiet day, not on the first.
pub(crate) const MEETINGS: [&str; 4] = [
    "We haven't met properly. I'm {name}.",
    "I'm {name}, by the way. I've seen you about.",
    "Hello! I'm {name}. I hear you're new round here.",
    "At last! I'm {name}. I've been meaning to say hello.",
];

/// How someone introduces themselves who has crossed the player's path
/// already (written to them, asked them a favour, looked after something
/// of theirs) without saying hello: never as to a stranger.
pub(crate) const CROSSED_PATHS: [&str; 3] = [
    "We keep crossing paths, and I never said. I'm {name}.",
    "Hello again! I'm {name}, in case nobody said.",
    "Good to put a face to it all at last. I'm {name}.",
];

/// How a letter to the player begins and ends.
pub(crate) const LETTER_OPENINGS: [&str; 10] = [
    "A few lines, since I didn't see you today.",
    "Thought you'd like to know how things are.",
    "Writing because it's quiet here.",
    "I wanted to tell you something.",
    "Just a note, nothing much.",
    "Hello from me.",
    "You weren't about, so I'm writing.",
    "News, of a sort.",
    "Couldn't find you, so here's a letter.",
    "A quiet day. I thought of you.",
];
pub(crate) const LETTER_CLOSINGS: [&str; 8] = [
    "Come by soon.",
    "Yours, {name}.",
    "See you around the {settlement}.",
    "That's all. Take care.",
    "Write back, if you like.",
    "More when I see you.",
    "Best, {name}.",
    "Mind how you go.",
];

/// What a letter is, as something to keep.
pub(crate) const LETTER: &str = "a letter from {name}";

/// How many letters someone has written the player.
pub(crate) const WROTE: &str = "lives.wrote";

pub(crate) fn best_friend(state: &WorldState, person: EntityId) -> Option<EntityId> {
    cast_ids(state)
        .into_iter()
        .filter(|o| {
            *o != person && opinion(state, person, *o) >= 20 && opinion(state, *o, person) >= 0
        })
        .max_by_key(|o| opinion(state, person, *o))
}

pub(crate) struct Script {
    pub(crate) prompts: &'static [&'static str],
    pub(crate) told: &'static str,
    pub(crate) answers: &'static [(&'static str, &'static str)],
}

pub(crate) fn script(kind: Kind) -> Script {
    match kind {
        Kind::Feud => Script {
            prompts: &[
                "{b} and I aren't speaking. It started over {topic}.",
                "I can't be in a room with {b}. Not after {topic}.",
                "About {topic}: {b} has it all wrong.",
            ],
            told: "{a} and {b} fell out over {topic}",
            answers: &[
                ("mend", "Make peace"),
                ("side", "Back {a}"),
                ("leave", "Stay out"),
            ],
        },
        Kind::Sweet => Script {
            prompts: &[
                "Don't laugh. I think I'm sweet on {b}.",
                "Would it be mad to ask {b} to {outing}?",
                "I keep finding reasons to be wherever {b} is.",
            ],
            told: "{a} confided {a_they} {a_were} sweet on {b}",
            answers: &[("ask", "Go for it"), ("wait", "Wait a bit")],
        },
        Kind::Learn => Script {
            prompts: &[
                "I'd love to learn what {b} does. Would you ask {b_them}?",
                "{b} makes it look easy. Could I learn from {b_them}?",
                "I feel stuck. {b} could teach me a thing or two.",
            ],
            told: "{a} wanted to learn from {b}",
            answers: &[("teach", "Arrange lessons"), ("not_now", "Not now")],
        },
        Kind::Short => Script {
            prompts: &[
                "I'm short of {short_of} this {unit}. I hate asking.",
                "Could I be tided over? Just this once.",
                "My {short_of} ran out before the {unit} did.",
            ],
            told: "{a} was short of {short_of}",
            answers: &[
                ("fund", "Use the fund"),
                ("friend", "Ask {friend}"),
                ("jobs", "Find them odd jobs"),
                ("manage", "They'll manage"),
            ],
        },
        Kind::Lonely => Script {
            prompts: &[
                "Haven't really talked to anyone in days.",
                "Evenings are long on my own.",
                "Would anyone notice if I wasn't here?",
            ],
            told: "{a} was lonely",
            answers: &[("invite", "Take them out"), ("space", "Give them space")],
        },
        Kind::Worn => Script {
            prompts: &[
                "I'm running on nothing.",
                "I can't remember my last day off.",
                "If I sit down I'll fall asleep.",
            ],
            told: "{a} was worn out",
            answers: &[("rest", "A day off"), ("push", "Push on")],
        },
        Kind::Party => Script {
            prompts: &[
                "{b} and I want to celebrate. Everyone's invited?",
                "We'd like a do. {b} insists on dancing.",
            ],
            told: "{a} and {b} wanted to celebrate",
            answers: &[
                ("party", "Throw a party"),
                ("potluck", "Everyone brings a dish"),
                ("quiet", "Keep it small"),
            ],
        },
        Kind::Visitor => Script {
            prompts: &[
                "{b}, a {trade} from {origin}, wants a room.",
                "A {trade} called {b} came in on {way_out}.",
            ],
            told: "{b}, a {trade} from {origin}, asked to stay",
            answers: &[("decline", "Not this time"), ("welcome", "Offer a room")],
        },
        Kind::Leaving => Script {
            prompts: &[
                "I've been thinking of leaving.",
                "There's nothing keeping me here, is there?",
                "I've been looking at the timetable for {way_out}.",
            ],
            told: "{a} was thinking of leaving",
            answers: &[("stay", "Please stay"), ("go", "Wish them well")],
        },
        Kind::RoughPatch => Script {
            prompts: &[
                "{b} and I keep quarrelling.",
                "Things aren't right with {b}.",
            ],
            told: "{a} and {b} hit a rough patch",
            answers: &[("talk", "Help them talk"), ("apart", "Time apart")],
        },
        Kind::Warming => Script {
            prompts: &["It's good to see you."],
            told: "{a} shared a quiet moment with you",
            answers: &[("stay", "Stay a while"), ("wave", "Wave and go on")],
        },
        Kind::Invite => Script {
            prompts: &["Come with me somewhere?"],
            told: "{a} asked you along",
            answers: &[("go", "Go along"), ("another", "Another time")],
        },
        Kind::Confide => Script {
            prompts: &[
                "Can I tell you something? I nearly didn't come to the {settlement} at all.",
                "I've never said this to anyone here. I write poems at night. Bad ones.",
                "Promise you won't laugh. I'm scared I'm no good at what I do.",
                "I still keep a letter from someone I left behind. I read it most {unit}s.",
            ],
            told: "{a} told you something {a_they}'d never told anyone",
            answers: &[
                ("keep", "Keep it between you"),
                ("share", "Tell them to share it"),
            ],
        },
        Kind::Favour => Script {
            prompts: &[
                "You've been good to me. Let me do something for you, for once.",
                "I owe you. Name it.",
                "Anything you need, you only have to ask. I mean it.",
            ],
            told: "{a} offered you a favour",
            answers: &[
                ("word", "Put in a word with {friend}"),
                ("help", "Help someone who's struggling"),
                ("nothing", "Nothing, really"),
            ],
        },
        Kind::Keepsake => Script {
            prompts: &[
                "I want you to have this: {keepsake}. You'll know why.",
                "Here. {keepsake}. Don't argue, just keep it.",
            ],
            told: "{a} gave you {keepsake}",
            answers: &[
                ("keep", "Keep it safe"),
                ("show", "Put it where all can see"),
            ],
        },
        Kind::Cold => Script {
            prompts: &[
                "Oh. It's you.",
                "I've nothing to say to you.",
                "Don't think I've forgotten.",
            ],
            told: "{a} gave you the cold shoulder",
            answers: &[("sorry", "Say you're sorry"), ("leave", "Leave them be")],
        },
    }
}

pub(crate) fn fill_owned(template: &str, words: &[(&'static str, String)]) -> String {
    let borrowed = words
        .iter()
        .map(|(slot, word)| (*slot, word.as_str()))
        .collect::<Vec<_>>();
    articled(&fill(template, &borrowed))
}

/// "a engineer" as it is said: "an engineer". A capital "A" is an article
/// only where a sentence starts ("side A is" is a side).
pub(crate) fn articled(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 4);
    let mut rest = text;
    while let Some(at) = rest.find(['a', 'A']) {
        let starts = |before: &str| {
            let before = before.trim_end();
            before.is_empty() || before.ends_with(['.', '!', '?', '"'])
        };
        let before_ok = if rest[at..].starts_with('A') {
            starts(&format!("{out}{}", &rest[..at])) && (at == 0 || rest[..at].ends_with(' '))
        } else {
            at == 0 || !rest[..at].ends_with(|c: char| c.is_alphanumeric())
        };
        let after = &rest[at + 1..];
        let next = after.strip_prefix(' ').and_then(|word| word.chars().next());
        out.push_str(&rest[..=at]);
        if before_ok && next.is_some_and(|c| "aeioAEIO".contains(c)) {
            out.push('n');
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// A situation as it would be put, from its candidate.
pub(crate) fn compose(state: &WorldState, cast: &Cast, candidate: &Candidate) -> Situation {
    let words = words_for(state, cast, candidate);
    let script = script(candidate.kind);
    let prompt = match door_scene(cast, candidate) {
        Some(scene) => fill_owned(scene.prompt, &words),
        None => pick(script.prompts, candidate.topic / 3)
            .map(|prompt| fill_owned(prompt, &words))
            .unwrap_or_default(),
    };
    let answers = script
        .answers
        .iter()
        .filter(|(id, _)| match (candidate.kind, *id) {
            // Paying from a common fund, or throwing a party out of it,
            // needs a place that has one.
            (Kind::Short, "fund") | (Kind::Party, "party") => cast.fund.is_some(),
            (Kind::Short, "friend") | (Kind::Favour, "word") => {
                best_friend(state, candidate.a).is_some()
            }
            _ => true,
        })
        .map(|(id, title)| Answer {
            id,
            title: fill_owned(title, &words),
            unavailable: match (candidate.kind, *id) {
                (Kind::Short, "fund") | (Kind::Party, "party") => cast.fund.and_then(|fund| {
                    (integer(state, fund.entity, fund.key).unwrap_or(0) < fund.amount)
                        .then(|| format!("There isn't {} in {}", fund.amount, fund.name))
                }),
                _ => None,
            },
        })
        .collect();
    Situation {
        key: candidate.key(),
        kind: candidate.kind,
        asker: candidate.a,
        about: candidate.b,
        prompt,
        told: fill_owned(script.told, &words),
        answers,
    }
}

/// The situations open now, as they are put to the player.
pub fn situations(world: &World, cast: &Cast) -> Vec<Situation> {
    let state = world.state();
    let mut open = open(state, cast);
    open.sort_by_key(|(key, at)| (*at, key.clone()));
    open.into_iter()
        .filter_map(|(key, _)| {
            let candidate = Candidate::parse(&key)?;
            let mut situation = compose(state, cast, &candidate);
            // Told as it was first put, whatever has changed since.
            if let Some(Value::Text(prompt)) = open_map(state, cast, &key)?.get("prompt") {
                situation.prompt = prompt.clone();
            }
            Some(situation)
        })
        .collect()
}
