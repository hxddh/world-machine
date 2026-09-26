//! The pair's storyteller: what each of them wants, what the place throws
//! at them, the days its calendar brings, and how each chapter ends.
//!
//! The mechanics are the shared `storylets` System. The words are written
//! once, with the place's own nouns filled in (a dust front on Mars, a
//! thunderstorm on Maple Street, a blizzard on Icebridge). What the
//! storyteller does is recorded as ordinary Events, so a World replays
//! without it.

use crate::{
    seed_id, RELATIONSHIP, RELATIONSHIP_DIRECTION, RELATIONSHIP_TENSION, RELATIONSHIP_TRUST,
    SLOT_A, SLOT_B, SLOT_C, SLOT_E,
};
use std::sync::OnceLock;
use storylets::{Choice, Condition, Deck, Ease, Effect, Goal, Outcome, Pinned, Reading, Storylet};
use world_core::{ActionRegistry, EntityId, Event, EventId, Value, World, WorldError};

/// The entity the storyteller keeps its notes on.
pub(crate) const STORY: EntityId = EntityId::new(20);
/// Periods in a season; four make a year.
pub(crate) const SEASON_PERIODS: u64 = 10;
const YEAR: u64 = SEASON_PERIODS * 4;
const CHAPTER_PERIODS: u64 = 24;
const STORY_COMMAND: &str = "pocket-universe.story.";

struct Said {
    event: &'static str,
    told: &'static str,
    line: &'static str,
    /// Who says it: the asker, unless this names the other one.
    by_other: bool,
    effects: Vec<Effect>,
    remembered: Option<&'static str>,
}

struct Answer {
    id: &'static str,
    title: &'static str,
    detail: &'static str,
    requires: Vec<Condition>,
    refuses: bool,
    said: Said,
}

struct Spec {
    storylet: Storylet,
    told: &'static str,
    line: &'static str,
    answers: Vec<Answer>,
    lapse: Said,
}

fn bond(trust: i64, tension: i64) -> Vec<Effect> {
    let mut effects = Vec::new();
    for (key, by) in [(RELATIONSHIP_TRUST, trust), (RELATIONSHIP_TENSION, tension)] {
        if by != 0 {
            effects.push(Effect::Add {
                entity: RELATIONSHIP,
                key,
                by,
                min: 0,
                max: 10,
            });
        }
    }
    effects
}

fn said(event: &'static str, told: &'static str, line: &'static str, effects: Vec<Effect>) -> Said {
    Said {
        event,
        told,
        line,
        by_other: false,
        effects,
        remembered: None,
    }
}

impl Said {
    fn remembered(mut self, line: &'static str) -> Self {
        self.remembered = Some(line);
        self
    }

    fn by_other(mut self) -> Self {
        self.by_other = true;
        self
    }

    fn and(mut self, effects: impl IntoIterator<Item = Effect>) -> Self {
        self.effects.extend(effects);
        self
    }
}

fn yes(id: &'static str, title: &'static str, detail: &'static str, said: Said) -> Answer {
    Answer {
        id,
        title,
        detail,
        requires: Vec::new(),
        refuses: false,
        said,
    }
}

fn no(id: &'static str, title: &'static str, detail: &'static str, said: Said) -> Answer {
    Answer {
        refuses: true,
        ..yes(id, title, detail, said)
    }
}

struct Shape {
    asker: EntityId,
    want: bool,
    requires: Vec<Condition>,
    lasts: u64,
    rests: u64,
    weight: u32,
    eases: Vec<Ease>,
    timely: bool,
}

fn want(asker: EntityId, eases: Vec<Ease>) -> Shape {
    Shape {
        asker,
        want: true,
        requires: Vec::new(),
        lasts: 3,
        rests: 8,
        weight: 3,
        eases,
        timely: false,
    }
}

fn incident(asker: EntityId, eases: Vec<Ease>) -> Shape {
    Shape {
        asker,
        want: false,
        requires: Vec::new(),
        lasts: 2,
        rests: 7,
        weight: 2,
        eases,
        timely: false,
    }
}

fn day(asker: EntityId, every: u64, at: u64) -> Shape {
    Shape {
        asker,
        want: false,
        requires: vec![Condition::Every { every, at }],
        lasts: 1,
        rests: 1,
        weight: 1,
        eases: Vec::new(),
        timely: true,
    }
}

impl Shape {
    fn requires(mut self, conditions: Vec<Condition>) -> Self {
        self.requires.extend(conditions);
        self
    }
}

fn up(gauge: &'static str) -> Ease {
    Ease { gauge, up: true }
}

fn down(gauge: &'static str) -> Ease {
    Ease { gauge, up: false }
}

fn spec(
    id: &'static str,
    shape: Shape,
    (told, line): (&'static str, &'static str),
    answers: Vec<Answer>,
    lapse: Said,
) -> Spec {
    let outcome = |said: &Said| Outcome {
        event: said.event,
        effects: said.effects.clone(),
    };
    Spec {
        storylet: Storylet {
            id,
            asker: shape.asker,
            want: shape.want,
            requires: shape.requires,
            choices: answers
                .iter()
                .map(|answer| Choice {
                    id: answer.id,
                    requires: answer.requires.clone(),
                    outcome: outcome(&answer.said),
                    refuses: answer.refuses,
                })
                .collect(),
            lapse: outcome(&lapse),
            lasts: shape.lasts,
            rests: shape.rests,
            weight: shape.weight,
            eases: shape.eases,
            timely: shape.timely,
        },
        told,
        line,
        answers,
        lapse,
    }
}

fn wants() -> Vec<Spec> {
    vec![
        spec(
            "keeper_spare",
            want(SLOT_B, vec![up("trust")]),
            (
                "{keeper} needs {spare}",
                "If I had {spare}, I'd sleep at night.",
            ),
            vec![
                yes(
                    "fetch",
                    "Send {explorer} for one",
                    "{explorer} makes the trip. {keeper} owes them one.",
                    said(
                        "spare_fetched",
                        "{explorer} brought {keeper} {spare}",
                        "You went all that way for this?",
                        bond(2, 0),
                    )
                    .remembered("Slept like a stone since you brought that."),
                ),
                no(
                    "make_do",
                    "Make do",
                    "Nobody goes anywhere. {keeper} lies awake.",
                    said(
                        "spare_refused",
                        "{keeper} was told to make do",
                        "Fine. I'll make do.",
                        bond(-1, 2),
                    ),
                ),
            ],
            said(
                "spare_went_without",
                "{keeper} went without {spare}",
                "Nobody listens.",
                bond(-1, 1),
            ),
        ),
        spec(
            "explorer_trip",
            want(SLOT_E, vec![up("tension"), down("trust")]),
            (
                "{explorer} wants to go past the edge of the map",
                "Just one trip past the edge of the map.",
            ),
            vec![
                yes(
                    "go",
                    "Go",
                    "{explorer} is gone for days. {keeper} keeps watch alone.",
                    said(
                        "edge_explored",
                        "{explorer} went past the edge of the map",
                        "You should see what's out there!",
                        bond(-1, 1),
                    )
                    .and([Effect::Advance("survey")])
                    .remembered("I keep drawing what I saw past the edge."),
                ),
                no(
                    "stay",
                    "Stay close",
                    "{keeper} is glad. {explorer} isn't.",
                    said(
                        "edge_refused",
                        "{explorer} was kept close to home",
                        "One day I'll just go.",
                        bond(0, 2),
                    ),
                ),
            ],
            said(
                "edge_forgotten",
                "{explorer} gave up on the edge of the map",
                "Forget it.",
                bond(0, 1),
            ),
        ),
        spec(
            "keeper_rest",
            want(SLOT_B, vec![down("tension"), up("trust")]),
            ("{keeper} needs a day off", "Could I have one day off?"),
            vec![
                yes(
                    "rest",
                    "Take the day",
                    "{explorer} covers {home} for a day.",
                    said(
                        "keeper_rested",
                        "{keeper} took a day off while {explorer} covered",
                        "Thank you. I needed that.",
                        bond(1, -2),
                    )
                    .remembered("Still feel rested."),
                ),
                no(
                    "keep_going",
                    "Keep going",
                    "{home} needs everyone.",
                    said(
                        "keeper_kept_going",
                        "{keeper} kept going without a break",
                        "Right. Back to it.",
                        bond(0, 2),
                    ),
                ),
            ],
            said(
                "keeper_worn_out",
                "{keeper} wore out",
                "I can't keep my eyes open.",
                bond(0, 1),
            ),
        ),
        spec(
            "explorer_teach",
            want(SLOT_E, vec![up("trust"), down("tension")]),
            (
                "{explorer} wants to teach {keeper} something",
                "Let me show you how I find my way out there.",
            ),
            vec![
                yes(
                    "learn",
                    "Show me",
                    "A day lost to work, a day won between them.",
                    said(
                        "keeper_taught",
                        "{explorer} taught {keeper} to find the way",
                        "See? You're a natural.",
                        bond(2, -1),
                    )
                    .remembered("You picked that up fast."),
                ),
                no(
                    "no_time",
                    "No time",
                    "The work comes first.",
                    said(
                        "lesson_refused",
                        "{keeper} had no time for {explorer}'s lesson",
                        "Another time, then.",
                        bond(-1, 0),
                    ),
                ),
            ],
            said(
                "lesson_forgotten",
                "{explorer}'s lesson never happened",
                "Never mind.",
                bond(-1, 0),
            ),
        ),
        spec(
            "keeper_grow",
            want(SLOT_B, vec![up("trust")]).requires(vec![Condition::Unfinished("second_home")]),
            (
                "{keeper} wants to start {second}",
                "We could make room for {second}.",
            ),
            vec![
                yes(
                    "start",
                    "Start building",
                    "Hard work for both of them. It will show on the horizon.",
                    said(
                        "second_begun",
                        "{keeper} and {explorer} worked on {second}",
                        "One more piece in place.",
                        bond(1, 1),
                    )
                    .and([Effect::Advance("second_home")])
                    .remembered("My hands still ache from building."),
                ),
                no(
                    "not_yet",
                    "Not yet",
                    "What they have is enough for now.",
                    said(
                        "second_put_off",
                        "{second} was put off",
                        "Some day.",
                        bond(0, 1),
                    ),
                ),
            ],
            said(
                "second_forgotten",
                "Nobody spoke of {second} again",
                "Maybe it was a silly idea.",
                bond(0, 1),
            ),
        ),
    ]
}

fn incidents() -> Vec<Spec> {
    vec![
        spec(
            "weather",
            incident(SLOT_E, vec![up("tension"), down("trust")]),
            ("{weather} is coming", "{Weather} on the horizon!"),
            vec![
                yes(
                    "batten",
                    "Batten down together",
                    "Both of them, side by side, all night.",
                    said(
                        "weather_weathered",
                        "They sat out {weather} together",
                        "We made it through that one.",
                        bond(1, -1),
                    )
                    .remembered("That was a long night."),
                ),
                yes(
                    "push_on",
                    "Push on through",
                    "{explorer} won't wait. {keeper} worries.",
                    said(
                        "weather_braved",
                        "{explorer} pushed on through {weather}",
                        "A little weather never stopped me.",
                        bond(-1, 2),
                    )
                    .and([Effect::Advance("survey")]),
                ),
            ],
            said(
                "weather_caught_them",
                "{weather} caught them apart",
                "Where were you?",
                bond(-1, 2),
            ),
        ),
        spec(
            "failing",
            incident(SLOT_B, vec![up("tension"), down("trust")]),
            (
                "Trouble at {home}: {failing}",
                "It's {failing}. I need a hand, now!",
            ),
            vec![
                yes(
                    "together",
                    "Mend it together",
                    "{explorer} drops everything.",
                    said(
                        "failing_fixed",
                        "They mended {failing} together",
                        "Good as new. Well. Good enough.",
                        bond(1, 0),
                    ),
                ),
                yes(
                    "alone",
                    "Manage alone",
                    "{explorer} stays out. {keeper} copes.",
                    said(
                        "failing_patched",
                        "{keeper} patched {failing} alone",
                        "Thanks for nothing.",
                        bond(-1, 2),
                    ),
                ),
            ],
            said(
                "failing_spread",
                "Nobody saw to {failing}, and it got worse",
                "It's getting worse!",
                bond(-2, 2),
            ),
        ),
        spec(
            "signal",
            incident(SLOT_E, vec![]),
            ("{signal} came in", "Did you hear that? {Signal}!"),
            vec![
                yes(
                    "follow",
                    "Follow it",
                    "{explorer} sets off. It may lead somewhere.",
                    said(
                        "signal_followed",
                        "{explorer} followed {signal}",
                        "I knew there was something out there.",
                        bond(0, 1),
                    )
                    .and([Effect::Advance("beacon")])
                    .remembered("Still thinking about that signal."),
                ),
                yes(
                    "log",
                    "Note it down",
                    "Nobody leaves. {keeper} writes it down.",
                    said(
                        "signal_logged",
                        "{keeper} wrote down {signal}",
                        "For another day.",
                        bond(1, 0),
                    )
                    .by_other(),
                ),
            ],
            said(
                "signal_faded",
                "{signal} faded away",
                "Gone. Did we imagine it?",
                bond(0, 0),
            ),
        ),
        spec(
            "quarrel",
            incident(SLOT_B, vec![down("trust"), up("tension")]),
            (
                "{keeper} and {explorer} are arguing",
                "You never listen when it matters!",
            ),
            vec![
                yes(
                    "keeper",
                    "{keeper}'s right",
                    "{explorer} won't like it.",
                    said(
                        "quarrel_keeper_won",
                        "{keeper} won the argument",
                        "Thank you.",
                        bond(-1, 1),
                    ),
                ),
                yes(
                    "explorer",
                    "{explorer}'s right",
                    "{keeper} won't like it.",
                    said(
                        "quarrel_explorer_won",
                        "{explorer} won the argument",
                        "Told you so.",
                        bond(-1, 1),
                    )
                    .by_other(),
                ),
            ],
            said(
                "quarrel_simmered",
                "The argument simmered on",
                "I'm not talking about it.",
                bond(-1, 2),
            ),
        ),
        spec(
            "amends",
            incident(SLOT_E, vec![down("tension"), up("trust")]).requires(vec![
                Condition::AtLeast(RELATIONSHIP, RELATIONSHIP_TENSION, 4),
            ]),
            (
                "{explorer} wants to make amends",
                "I said things I shouldn't have.",
            ),
            vec![
                yes(
                    "amends",
                    "Make up",
                    "The rivalry is set down, for now.",
                    said(
                        "amends_made",
                        "{explorer} and {keeper} made it up",
                        "Friends again?",
                        bond(2, -3),
                    )
                    .and([Effect::Set {
                        entity: RELATIONSHIP,
                        key: RELATIONSHIP_DIRECTION,
                        text: "none",
                    }])
                    .remembered("I'm glad we talked."),
                ),
                no(
                    "not_yet",
                    "Too soon",
                    "The hurt stays where it is.",
                    said(
                        "amends_too_soon",
                        "{keeper} wasn't ready to make it up",
                        "Not yet.",
                        bond(0, 1),
                    )
                    .by_other(),
                ),
            ],
            said(
                "time_healed",
                "Time took the edge off",
                "Let's not fight.",
                bond(1, -2),
            ),
        ),
        spec(
            "close_call",
            incident(SLOT_E, vec![down("tension"), up("trust")]),
            (
                "{explorer} is caught out in the cold",
                "I'm lost out here. Can you hear me?",
            ),
            vec![
                yes(
                    "rescue",
                    "Go out after them",
                    "{keeper} leaves {home} to find them.",
                    said(
                        "explorer_rescued",
                        "{keeper} brought {explorer} home",
                        "You came for me.",
                        bond(3, -3),
                    )
                    .remembered("I owe you my life, you know."),
                ),
                yes(
                    "guide",
                    "Talk them home",
                    "{keeper} stays, and talks all night.",
                    said(
                        "explorer_guided",
                        "{keeper} talked {explorer} home",
                        "Your voice got me back.",
                        bond(2, -2),
                    ),
                ),
            ],
            said(
                "explorer_found_the_way",
                "{explorer} found the way home alone",
                "Made it. Just.",
                bond(1, -2),
            ),
        ),
        spec(
            "supper",
            incident(SLOT_B, vec![down("tension"), up("trust")]),
            (
                "{keeper} wants to cook for them both",
                "Supper, the two of us?",
            ),
            vec![
                yes(
                    "eat",
                    "Supper together",
                    "An evening off, and a long talk.",
                    said(
                        "supper_shared",
                        "{keeper} and {explorer} shared a supper",
                        "This is nice.",
                        bond(1, -2),
                    )
                    .remembered("That was a good supper."),
                ),
                yes(
                    "work",
                    "Work through it",
                    "There's too much to do.",
                    said(
                        "supper_skipped",
                        "They worked through supper",
                        "Another time.",
                        bond(0, 1),
                    ),
                ),
            ],
            said(
                "supper_went_cold",
                "The supper went cold",
                "I'll eat it myself, then.",
                bond(-1, 0),
            ),
        ),
        spec(
            "beacon_work",
            incident(SLOT_E, vec![up("trust")]).requires(vec![Condition::Unfinished("beacon")]),
            (
                "{beacon} could go up",
                "If we put up {beacon}, anyone could find us.",
            ),
            vec![
                yes(
                    "build",
                    "Put it up",
                    "Days of work, and it will stand on the horizon.",
                    said(
                        "beacon_raised",
                        "Work went on at {beacon}",
                        "Higher every day!",
                        bond(1, 1),
                    )
                    .and([Effect::Advance("beacon")]),
                ),
                yes(
                    "later",
                    "Later",
                    "There's enough to do.",
                    said(
                        "beacon_left",
                        "{beacon} was left for later",
                        "Later, then.",
                        bond(0, 0),
                    ),
                ),
            ],
            said(
                "beacon_forgotten",
                "Nobody went back to {beacon}",
                "Nobody cares about the beacon.",
                bond(0, 0),
            ),
        ),
    ]
}

fn calendar() -> Vec<Spec> {
    vec![
        spec(
            "supply",
            day(SLOT_B, 6, 2),
            ("{supply} is in", "{Supply}! What do we do with it?"),
            vec![
                yes(
                    "share",
                    "Share it out",
                    "Half each, no arguments.",
                    said(
                        "supply_shared",
                        "They shared out {supply}",
                        "Half each.",
                        bond(1, 0),
                    ),
                ),
                yes(
                    "build",
                    "Build with it",
                    "It all goes into building.",
                    said(
                        "supply_built",
                        "{supply} went into {second}",
                        "Every bit counts.",
                        bond(0, 1),
                    )
                    .and([Effect::Advance("second_home")]),
                ),
            ],
            said(
                "supply_sat",
                "{supply} sat unopened",
                "Nobody's touched it.",
                bond(0, 0),
            ),
        ),
        spec(
            "window",
            day(SLOT_E, YEAR / 2, 12),
            ("It's {window}", "It's {window}! Are we going?"),
            vec![
                yes(
                    "go",
                    "Go together",
                    "{home} can mind itself for a day.",
                    said(
                        "window_gone",
                        "They went to {window} together",
                        "Best day in ages.",
                        bond(2, -1),
                    )
                    .remembered("I keep thinking about {window}."),
                ),
                yes(
                    "watch",
                    "Watch from home",
                    "Someone has to stay.",
                    said(
                        "window_watched",
                        "They watched {window} from {home}",
                        "Next time.",
                        bond(0, 1),
                    ),
                ),
            ],
            said(
                "window_missed",
                "{window} came and went",
                "Missed it again.",
                bond(0, 1),
            ),
        ),
        spec(
            "birthday_keeper",
            day(SLOT_B, YEAR, 7),
            ("It's {keeper}'s birthday", "It's my birthday today!"),
            vec![
                yes(
                    "party",
                    "A surprise party",
                    "{explorer} has something planned.",
                    said(
                        "keeper_birthday_kept",
                        "{explorer} surprised {keeper} on their birthday",
                        "You remembered!",
                        bond(2, -1),
                    )
                    .remembered("Best birthday I've had."),
                ),
                yes(
                    "quiet",
                    "A quiet one",
                    "Nothing much. A card.",
                    said(
                        "keeper_birthday_quiet",
                        "{keeper} had a quiet birthday",
                        "That's fine.",
                        bond(0, 0),
                    ),
                ),
            ],
            said(
                "keeper_birthday_forgotten",
                "{keeper}'s birthday was forgotten",
                "Nobody remembered.",
                bond(-1, 1),
            ),
        ),
        spec(
            "birthday_explorer",
            day(SLOT_E, YEAR, 27),
            ("It's {explorer}'s birthday", "Guess what day it is?"),
            vec![
                yes(
                    "party",
                    "A surprise party",
                    "{keeper} has something planned.",
                    said(
                        "explorer_birthday_kept",
                        "{keeper} surprised {explorer} on their birthday",
                        "For me?",
                        bond(2, -1),
                    )
                    .remembered("Still wearing the present."),
                ),
                yes(
                    "quiet",
                    "A quiet one",
                    "Nothing much. A card.",
                    said(
                        "explorer_birthday_quiet",
                        "{explorer} had a quiet birthday",
                        "Just another day.",
                        bond(0, 0),
                    ),
                ),
            ],
            said(
                "explorer_birthday_forgotten",
                "{explorer}'s birthday was forgotten",
                "Nobody remembered.",
                bond(-1, 1),
            ),
        ),
    ]
}

fn specs() -> &'static [Spec] {
    static SPECS: OnceLock<Vec<Spec>> = OnceLock::new();
    SPECS.get_or_init(|| {
        let mut specs = wants();
        specs.extend(incidents());
        specs.extend(calendar());
        specs
    })
}

pub(crate) fn deck() -> Deck {
    Deck {
        story: STORY,
        story_name: "The pair's story",
        period: crate::BACKGROUND_PERIOD,
        storylets: specs().iter().map(|spec| spec.storylet.clone()).collect(),
        goals: vec![
            Goal {
                id: "second_home",
                parts: 3,
            },
            Goal {
                id: "beacon",
                parts: 2,
            },
            Goal {
                id: "survey",
                parts: 3,
            },
        ],
        chapter_periods: CHAPTER_PERIODS,
        shortest_chapter: 10,
        // A new chapter starts nearer the middle than the last one ended:
        // whatever was at its end eases back.
        fresh_start: [RELATIONSHIP_TRUST, RELATIONSHIP_TENSION]
            .into_iter()
            .map(|key| Effect::Toward {
                entity: RELATIONSHIP,
                key,
                target: 5,
                by: 3,
            })
            .collect(),
        most_open: 3,
    }
}

pub(crate) fn register_actions(
    actions: &mut ActionRegistry,
) -> Result<(), world_core::ActionError> {
    storylets::register_actions(actions, deck)
}

/// The nouns each place fills into the storyteller's words.
fn noun(world: &World, key: &str) -> String {
    let seed = seed_id(world);
    let first = |id: EntityId| {
        world
            .state()
            .entity(id)
            .map(world_projection::entity_title)
            .and_then(|name| name.split_whitespace().next().map(str::to_string))
            .unwrap_or_else(|| "someone".into())
    };
    let title = |id: EntityId| {
        world
            .state()
            .entity(id)
            .map(world_projection::entity_title)
            .unwrap_or_else(|| "home".into())
    };
    let pick = |mars: &str, town: &str, ice: &str| -> String {
        match seed {
            "mars-colony" => mars,
            "1980s-town" => town,
            _ => ice,
        }
        .to_string()
    };
    match key {
        "keeper" => first(SLOT_B),
        "explorer" => first(SLOT_E),
        "home" => title(SLOT_A),
        "place" => title(SLOT_C),
        "weather" => pick("a dust front", "a thunderstorm", "a blizzard"),
        "spare" => pick("a spare seal", "a spare fuse", "a spare lantern"),
        "failing" => pick(
            "a failing seal",
            "a sparking fuse box",
            "a crack in the ice",
        ),
        "signal" => pick(
            "a signal from the old lander",
            "a late call-in from out of town",
            "a far-off song across the ice",
        ),
        "supply" => pick("the supply drop", "the delivery truck", "the herring run"),
        "window" => pick(
            "the launch window",
            "the county fair",
            "the great migration",
        ),
        "second" => pick(
            "a second dome",
            "a back room at the arcade",
            "a second bridge span",
        ),
        "beacon" => pick("a relay mast", "a new radio mast", "a lantern tower"),
        _ => String::new(),
    }
}

/// The storyteller's words with the place's nouns filled in. A noun at the
/// start of a sentence is written with a capital (`{Weather}`).
fn fill(world: &World, text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('}') else {
            break;
        };
        let key = &rest[start + 1..start + end];
        let word = noun(world, &key.to_lowercase());
        if key.starts_with(char::is_uppercase) {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                out.extend(first.to_uppercase());
                out.push_str(chars.as_str());
            }
        } else {
            out.push_str(&word);
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    // A sentence always starts with a capital, whatever was filled in.
    let mut chars = out.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => out,
    }
}

fn integer(world: &World, id: EntityId, key: &str) -> i64 {
    match world
        .state()
        .entity(id)
        .and_then(|entity| entity.component(key))
    {
        Some(Value::Integer(value)) => *value,
        _ => 0,
    }
}

/// Trust and tension, near or at either end.
fn pinned(world: &World) -> Vec<Pinned> {
    let mut pinned = Vec::new();
    for (gauge, key) in [
        ("trust", RELATIONSHIP_TRUST),
        ("tension", RELATIONSHIP_TENSION),
    ] {
        match integer(world, RELATIONSHIP, key) {
            8.. => pinned.push(Pinned { gauge, high: true }),
            ..=2 => pinned.push(Pinned { gauge, high: false }),
            _ => {}
        }
    }
    pinned
}

fn at_end(world: &World) -> bool {
    [RELATIONSHIP_TRUST, RELATIONSHIP_TENSION]
        .into_iter()
        .any(|key| !(1..=9).contains(&integer(world, RELATIONSHIP, key)))
}

const SEASONS: [&str; 4] = ["spring", "summer", "autumn", "winter"];

pub(crate) fn season(world: &World) -> usize {
    storylets::season(
        storylets::period_index(world.state(), &deck()),
        SEASON_PERIODS,
    ) as usize
}

fn chapter_ending(world: &World) -> (String, String) {
    let deck = deck();
    let trust = integer(world, RELATIONSHIP, RELATIONSHIP_TRUST);
    let tension = integer(world, RELATIONSHIP, RELATIONSHIP_TENSION);
    let feel = if trust >= 7 && tension <= 3 {
        "A close"
    } else if tension >= 7 && trust <= 3 {
        "A bitter"
    } else if tension >= 6 {
        "A stormy"
    } else if trust >= 6 {
        "A kind"
    } else {
        "A quiet"
    };
    let mut title = format!("{feel} {}", SEASONS[season(world)]);
    if storylets::last_chapter_title(world).is_some_and(|last| last.ends_with(&title[2..])) {
        title = format!("Another {}", &title[2..]);
    }
    let mut summary = vec![if trust >= 7 {
        fill(
            world,
            "{keeper} and {explorer} would trust each other with anything.",
        )
    } else if tension >= 7 {
        fill(world, "{keeper} and {explorer} could barely share a room.")
    } else {
        fill(world, "{keeper} and {explorer} kept each other going.")
    }];
    if storylets::progress(world.state(), &deck, "second_home") >= 3 {
        summary.push(fill(world, "{second} stands finished."));
    }
    if storylets::progress(world.state(), &deck, "beacon") >= 2 {
        summary.push(fill(world, "{beacon} is up."));
    }
    if storylets::progress(world.state(), &deck, "survey") >= 3 {
        summary.push(fill(world, "{explorer} mapped the land past the edge."));
    }
    for who in [SLOT_B, SLOT_E] {
        let (granted, grudges) = storylets::kindness(world.state(), &deck, who);
        if grudges > granted + 1 {
            let name = noun(world, if who == SLOT_B { "keeper" } else { "explorer" });
            summary.push(format!("{name} hasn't forgotten being let down."));
            break;
        }
    }
    (title, summary.join(" "))
}

/// One period of the storyteller, once a World has begun.
pub(crate) fn tick(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    if seed_id(world) == "unseeded" {
        return Ok(Vec::new());
    }
    let reading = Reading {
        pinned: pinned(world),
        at_end: at_end(world),
        chapter_ending: Box::new(chapter_ending),
    };
    storylets::tick(world, actions, &deck(), &reading)
}

fn command_id(storylet: &str, choice: &str) -> String {
    format!("{STORY_COMMAND}{storylet}.{choice}")
}

pub(crate) fn parse_command(command_id: &str) -> Option<(&str, &str)> {
    command_id.strip_prefix(STORY_COMMAND)?.split_once('.')
}

fn find(storylet: &str) -> Option<&'static Spec> {
    specs().iter().find(|spec| spec.storylet.id == storylet)
}

/// A card for every answer that can be given now.
pub(crate) fn commands(world: &World) -> Vec<world_projection::ProjectionCommand> {
    let deck = deck();
    storylets::choices(world.state(), &deck)
        .into_iter()
        .filter_map(|(storylet, choice)| {
            let spec = find(storylet.id)?;
            let answer = spec.answers.iter().find(|answer| answer.id == choice.id)?;
            Some(world_projection::ProjectionCommand {
                id: command_id(storylet.id, choice.id),
                title: fill(world, answer.title),
                detail: fill(world, answer.detail),
                effects: Vec::new(),
                scenery: None,
                asker: Some(world_projection::SelectionId::Entity(storylet.asker)),
                moves: Vec::new(),
                question: Some(world_projection::Question {
                    id: storylet.id.into(),
                    prompt: fill(world, spec.line),
                }),
            })
        })
        .collect()
}

/// What someone would ask for now, if they have a want open, and the answer
/// that grants it.
pub(crate) fn wanting(world: &World, who: EntityId) -> Option<(String, Option<String>)> {
    let deck = deck();
    let storylet = storylets::open(world.state(), &deck)
        .into_iter()
        .find(|storylet| storylet.want && storylet.asker == who)?;
    let spec = find(storylet.id)?;
    let grant = storylets::choices(world.state(), &deck)
        .into_iter()
        .find(|(open, choice)| open.id == storylet.id && !choice.refuses)
        .map(|(open, choice)| command_id(open.id, choice.id));
    Some((fill(world, spec.line), grant))
}

pub(crate) fn kindness(world: &World, who: EntityId) -> (i64, i64) {
    storylets::kindness(world.state(), &deck(), who)
}

fn storylet_of(event: &Event) -> Option<&'static Spec> {
    match event.payload.get("storylet")? {
        Value::Text(id) => find(id),
        _ => None,
    }
}

fn outcome_of(event: &Event) -> Option<&'static Said> {
    let spec = storylet_of(event)?;
    if event.kind == "situation_arose" {
        return None;
    }
    if spec.lapse.event == event.kind {
        return Some(&spec.lapse);
    }
    spec.answers
        .iter()
        .map(|answer| &answer.said)
        .find(|said| said.event == event.kind)
}

/// How one of the storyteller's moments is told.
pub(crate) fn told(world: &World, event: &Event) -> Option<String> {
    if event.kind == "chapter_ended" {
        return match event.payload.get("title") {
            Some(Value::Text(title)) => Some(format!("The chapter closed: {title}")),
            _ => None,
        };
    }
    let spec = storylet_of(event)?;
    if event.kind == "situation_arose" {
        return Some(fill(world, spec.told));
    }
    Some(fill(world, outcome_of(event)?.told))
}

/// Who speaks at one of the storyteller's moments, and what they say.
pub(crate) fn line(world: &World, event: &Event) -> Option<(EntityId, String)> {
    let spec = storylet_of(event)?;
    let asker = spec.storylet.asker;
    let other = if asker == SLOT_B { SLOT_E } else { SLOT_B };
    if event.kind == "situation_arose" {
        return Some((asker, fill(world, spec.line)));
    }
    let said = outcome_of(event)?;
    Some((
        if said.by_other { other } else { asker },
        fill(world, said.line),
    ))
}

pub(crate) fn tone(event: &Event) -> Option<world_projection::Tone> {
    use world_projection::Tone;
    if event.kind == "chapter_ended" {
        return Some(Tone::Neutral);
    }
    let spec = storylet_of(event)?;
    if event.kind == "situation_arose" {
        return Some(if spec.storylet.want {
            Tone::Neutral
        } else {
            Tone::Warning
        });
    }
    let moved = outcome_of(event)?
        .effects
        .iter()
        .map(|effect| match effect {
            Effect::Add { key, by, .. } if *key == RELATIONSHIP_TRUST => *by,
            Effect::Add { key, by, .. } if *key == RELATIONSHIP_TENSION => -*by,
            Effect::Advance(_) => 1,
            _ => 0,
        })
        .sum::<i64>();
    Some(match moved {
        1.. => Tone::Good,
        0 => Tone::Neutral,
        _ => Tone::Warning,
    })
}

/// What someone still says in the period after something went their
/// way.
pub(crate) fn remembered(world: &World, who: EntityId) -> Option<String> {
    let now = world.world_time();
    let since = now.saturating_sub(crate::BACKGROUND_PERIOD);
    world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time >= since)
        .filter(|event| event.world_time < now && event.actor == Some(who))
        .find_map(|event| outcome_of(event).and_then(|said| said.remembered))
        .map(|line| fill(world, line))
}

/// The newest thing the storyteller set going this period, told, for a
/// headline when nothing larger is happening.
pub(crate) fn headline(world: &World) -> Option<String> {
    let now = world.world_time();
    world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time == now)
        .find(|event| event.kind == "situation_arose" || event.kind == "chapter_ended")
        .and_then(|event| told(world, event))
}

/// The pair's standing goals, for the horizon.
pub(crate) fn goals(world: &World) -> Vec<world_projection::Goal> {
    use world_projection::MarkShape;
    if seed_id(world) == "unseeded" {
        return Vec::new();
    }
    let deck = deck();
    let (home, beacon) = match seed_id(world) {
        "mars-colony" => (MarkShape::Dome, MarkShape::Tower),
        "1980s-town" => (MarkShape::Shop, MarkShape::Tower),
        _ => (MarkShape::Bridge, MarkShape::Lamp),
    };
    [
        ("second_home", "{second}", home),
        ("beacon", "{beacon}", beacon),
        ("survey", "The map past the edge", MarkShape::Rover),
    ]
    .into_iter()
    .filter_map(|(id, label, shape)| {
        let parts = deck.goals.iter().find(|goal| goal.id == id)?.parts;
        Some(world_projection::Goal {
            id: id.into(),
            label: fill(world, label),
            shape,
            done: storylets::progress(world.state(), &deck, id).clamp(0, parts) as u32,
            parts: parts as u32,
        })
    })
    .collect()
}

/// The chapters of the pair's story that have ended.
pub(crate) fn chapters(world: &World) -> Vec<world_projection::Chapter> {
    storylets::chapters_ended(world)
        .into_iter()
        .map(|ended| world_projection::Chapter {
            number: ended.number.max(0) as u32,
            title: ended.title,
            summary: ended.summary,
            moment: Some(world_projection::SelectionId::Event(ended.event)),
        })
        .collect()
}
