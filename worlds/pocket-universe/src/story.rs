//! The pair's storyteller: what each of them wants, what the place throws
//! at them, the days its calendar brings, and how each chapter ends.
//!
//! The mechanics are the shared `storylets` System. The words are written
//! once, with the place's own nouns filled in (a dust front on Mars, a
//! thunderstorm on Maple Street, a blizzard on Icebridge). What the
//! storyteller does is recorded as ordinary Events, so a World replays
//! without it.

use crate::{
    seed_id, RELATIONSHIP, RELATIONSHIP_TENSION, RELATIONSHIP_TRUST, SEED, SLOT_A, SLOT_B, SLOT_C,
    SLOT_D, SLOT_E, UNIVERSE,
};
use std::sync::OnceLock;
use storylets::script::Node;
use storylets::{build, mark};
use storylets::{Choice, Condition, Deck, Ease, Effect, Goal, Outcome, Pinned, Reading, Storylet};
use world_core::{ActionRegistry, EntityId, Event, EventId, Value, World, WorldError};

/// The entity the storyteller keeps its notes on.
pub(crate) const STORY: EntityId = EntityId::new(20);
/// Periods in a season; four make a year.
pub(crate) const SEASON_PERIODS: u64 = 30;
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
    /// What this adds to the chapter's ending.
    chapter: Option<&'static str>,
    /// What the chapter is called, when this is how its climax went.
    title: Option<&'static str>,
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
    /// For a want: how it goes when the other one takes it up after the
    /// player let it down.
    taken_up: Option<Said>,
}

/// The storyteller's mark for the player having lately been here: made
/// something or answered someone. While it is recent, the place takes up
/// the works the player let wait; a place left alone lets them wait.
pub(crate) const HANDS_SEEN: &str = "player_seen";
/// How many periods the player being here keeps the place building.
const HANDS_LATELY: u64 = 30;
/// How stale the mark may grow before a deed renews it.
pub(crate) const HANDS_SEEN_EVERY: u64 = 7;

/// Who takes up a want the player let down: the other of the pair.
fn helper_for(asker: EntityId) -> Option<(EntityId, &'static str)> {
    match asker {
        SLOT_B => Some((SLOT_E, "{Explorer}")),
        SLOT_E => Some((SLOT_B, "{Keeper}")),
        _ => None,
    }
}

/// A want let down, seen to by the other one: what its granting answer
/// does, told as their doing.
fn taken_up_said(answers: &[Answer], helper: &str) -> Option<Said> {
    let granting = answers
        .iter()
        .find(|answer| !answer.refuses && !answer.said.effects.is_empty())?;
    let said = &granting.said;
    let told = match said.told.split_once(' ') {
        Some(("The" | "A" | "An" | "They" | "Everyone" | "Nobody" | "Work", _)) => {
            let mut chars = said.told.chars();
            chars
                .next()
                .map(|first| first.to_lowercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        }
        _ => said.told.to_string(),
    };
    Some(Said {
        event: leak(format!("{}_taken_up", said.event)),
        told: leak(format!("{helper} took it on, and {told}")),
        line: said.line,
        by_other: said.by_other,
        effects: said.effects.clone(),
        remembered: said.remembered,
        chapter: None,
        title: None,
    })
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
        chapter: None,
        title: None,
    }
}

impl Said {
    fn remembered(mut self, line: &'static str) -> Self {
        self.remembered = Some(line);
        self
    }

    fn chapter(mut self, line: &'static str) -> Self {
        self.chapter = Some(line);
        self
    }

    fn titled(mut self, title: &'static str) -> Self {
        self.title = Some(title);
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
        rests: 16,
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
        rests: 12,
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
        effects: said
            .effects
            .iter()
            .cloned()
            .chain(leaves_behind(said.event))
            .collect(),
    };
    let mut requires = shape.requires;
    requires.extend(settled_by(id));
    let helper = helper_for(shape.asker);
    // The other one sees to a want the player let down, and to a part of
    // a work the player let go by.
    let builds = answers.iter().any(|answer| {
        answer
            .said
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::Advance(_)))
    });
    let taken_up = helper
        .filter(|_| shape.want || builds)
        .and_then(|(_, named)| taken_up_said(&answers, named));
    Spec {
        storylet: Storylet {
            id,
            asker: shape.asker,
            want: shape.want,
            requires,
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
            taken_up: taken_up
                .as_ref()
                .zip(helper)
                .map(|(said, (by, _))| storylets::TakenUp {
                    by,
                    outcome: outcome(said),
                    // A part of one of the place's works is taken up only
                    // while the player has lately been there.
                    requires: if said
                        .effects
                        .iter()
                        .any(|effect| matches!(effect, Effect::Advance(_)))
                    {
                        vec![Condition::MarkedWithin(HANDS_SEEN, HANDS_LATELY)]
                    } else {
                        Vec::new()
                    },
                }),
        },
        told,
        line,
        answers,
        lapse,
        taken_up,
    }
}

/// An incident (weather coming, something failing, a row) waits until a
/// new player's first days have passed.
fn after_first_days(mut spec: Spec) -> Spec {
    spec.storylet.requires.push(Condition::Since(
        crate::UNIVERSE,
        crate::arrival::ARRIVED,
        crate::arrival::FIRST_DAYS,
    ));
    spec
}

fn incidents() -> Vec<Spec> {
    incidents_at_any_time()
        .into_iter()
        .map(after_first_days)
        .collect()
}

fn calendar() -> Vec<Spec> {
    vec![
        spec(
            "supply",
            day(SLOT_B, 10, 2),
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
            day(SLOT_E, YEAR / 2, 36),
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
            day(SLOT_B, YEAR, 21),
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
                "Nobody remembered. Not one.",
                bond(-1, 1),
            ),
        ),
        spec(
            "birthday_explorer",
            day(SLOT_E, YEAR, 81),
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
                "Birthday came and went.",
                bond(-1, 1),
            ),
        ),
    ]
}

/// Text made up once and kept for the life of the program.
fn leak(text: String) -> &'static str {
    static KEPT: OnceLock<std::sync::Mutex<std::collections::BTreeSet<&'static str>>> =
        OnceLock::new();
    let mut kept = KEPT
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(known) = kept.get(text.as_str()) {
        return known;
    }
    let made: &'static str = Box::leak(text.into_boxed_str());
    kept.insert(made);
    made
}

/// One rung of a place's ladder: one of its works built, or, once every
/// one of them stands, one mended.
pub(crate) struct Rung {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) work: &'static crate::works::Work,
    told: &'static str,
    line: &'static str,
    progress: &'static str,
}

/// What is said as a work is mended, one after another.
const MENDED: [&str; 6] = [
    "Patched, sealed and good for years.",
    "Good as new. Better, maybe.",
    "A new hinge, a lick of paint, done.",
    "Fixed the bit everyone kept tripping on.",
    "Tightened every bolt. Twice.",
    "It'll see us out now.",
];

/// How many parts a work takes: a day the player gives it is two, a
/// stretch the people get on with it themselves is one.
const RUNG_PARTS: i64 = 4;

fn the(label: &str) -> String {
    let rest = label
        .strip_prefix("A ")
        .or_else(|| label.strip_prefix("An "))
        .unwrap_or(label);
    let mut chars = rest.chars();
    let lower = chars
        .next()
        .map(|first| first.to_lowercase().chain(chars).collect::<String>())
        .unwrap_or_default();
    format!("the {lower}")
}

/// A place's rungs, in order: each of its works once, then each mended,
/// so there is always one under way for years.
pub(crate) fn ladder(place: crate::places::Place) -> &'static [Rung] {
    static LADDERS: OnceLock<[Vec<Rung>; 3]> = OnceLock::new();
    let ladders = LADDERS.get_or_init(|| {
        use crate::places::Place;
        [Place::Ares, Place::Maple, Place::Ice].map(|place| {
            let mut rungs = Vec::new();
            for round in 0..2 {
                for work in crate::works::works(place) {
                    let the = the(work.label);
                    rungs.push(match round {
                        0 => Rung {
                            id: work.id,
                            label: work.label,
                            work,
                            told: work.told,
                            line: work.line,
                            progress: work.progress,
                        },
                        _ => Rung {
                            id: leak(format!("{}_mended", work.id)),
                            label: leak(format!("Mend {the}")),
                            work,
                            told: leak(format!("It's time to mend {the}")),
                            line: leak(format!("A few repairs and {the} is good as new.")),
                            progress: MENDED[rungs.len() % MENDED.len()],
                        },
                    });
                }
            }
            rungs
        })
    });
    &ladders[place.index()]
}

/// Every place's rungs.
fn all_rungs() -> impl Iterator<Item = (crate::places::Place, &'static Rung)> {
    use crate::places::Place;
    [Place::Ares, Place::Maple, Place::Ice]
        .into_iter()
        .flat_map(|place| ladder(place).iter().map(move |rung| (place, rung)))
}

/// The works of the ladder every place shared before each had its own,
/// which a World begun then may have finished: counted still, but never
/// asked for again.
const OLD_WORKS: [&str; 8] = [
    "hall",
    "store",
    "lookout",
    "workshop",
    "schoolroom",
    "long_table",
    "path_lights",
    "first_stone",
];

/// How many rungs of the place's ladder are finished, with any of the old
/// shared ladder a World finished before, and the small works its people
/// finished on their own.
pub(crate) fn works_finished(state: &world_core::WorldState) -> i64 {
    let deck = deck_ref();
    // A place's works are finished in order, so they are counted until the
    // first that is not.
    let own = crate::places::Place::of(state).map_or(0, |place| {
        ladder(place)
            .partition_point(|rung| storylets::progress(state, deck, rung.id) >= RUNG_PARTS)
    });
    let old = if storylets::progress(state, deck, OLD_WORKS[0]) > 0 {
        OLD_WORKS
            .iter()
            .flat_map(|work| {
                ["", "_mended", "_decorated", "_lit", "_extended"]
                    .map(|round| format!("{work}{round}"))
            })
            .filter(|id| storylets::progress(state, deck, id) >= 2)
            .count()
    } else {
        0
    };
    (own + old + crate::years::own_works_done(state)) as i64
}

/// The works of the place's ladder its people have finished, first to
/// last, each counted once however often it was mended or added on to.
pub(crate) fn works_done(state: &world_core::WorldState) -> Vec<&'static crate::works::Work> {
    let deck = deck_ref();
    let Some(place) = crate::places::Place::of(state) else {
        return Vec::new();
    };
    let works = crate::works::works(place);
    let ladder = &ladder(place)[..works.len()];
    let done =
        ladder.partition_point(|rung| storylets::progress(state, deck, rung.id) >= RUNG_PARTS);
    ladder[..done].iter().map(|rung| rung.work).collect()
}

#[cfg(test)]
/// A place's goals in the order it meets them: its first three, then its
/// ladder of works.
pub(crate) fn goal_ids(place: crate::places::Place) -> Vec<&'static str> {
    ["second_home", "beacon", "survey"]
        .into_iter()
        .chain(ladder(place).iter().map(|rung| rung.id))
        .collect()
}

/// A work as it is spoken of: "the dust wall round the landing pad".
pub(crate) fn the_work(work: &crate::works::Work) -> &'static str {
    leak(the(work.label))
}

fn seed_of(place: crate::places::Place) -> &'static str {
    use crate::places::Place;
    match place {
        Place::Ares => "mars-colony",
        Place::Maple => "1980s-town",
        Place::Ice => "penguin-civilization",
    }
}

/// The works as storylets: each asked for once the one before is done,
/// after the place's first three goals, and only in its own place. A day
/// the player gives it builds two parts; left to the people, or let go
/// by, it still goes on one part at a time, their own way.
fn works() -> Vec<Spec> {
    use Condition::{Finished, Is, Unfinished};
    all_rungs()
        .map(|(place, rung)| {
            let ladder = ladder(place);
            let index = ladder
                .iter()
                .position(|other| other.id == rung.id)
                .unwrap_or(0);
            let before = if index == 0 {
                vec![
                    Finished("second_home"),
                    Finished("beacon"),
                    Finished("survey"),
                ]
            } else {
                vec![Finished(ladder[index - 1].id)]
            };
            let shape = Shape {
                asker: rung.work.champion,
                want: true,
                requires: [
                    vec![Is(UNIVERSE, SEED, seed_of(place)), Unfinished(rung.id)],
                    before,
                ]
                .concat(),
                lasts: 3,
                // The place's first work comes round again quickly, while
                // it is new to building; after it, at the place's pace.
                rests: if index == 0 {
                    12
                } else {
                    crate::works::rests(place)
                },
                weight: 4,
                eases: vec![up("trust")],
                timely: false,
            };
            let own = crate::works::on_their_own(place);
            spec(
                leak(format!("work_{}", rung.id)),
                shape,
                (rung.told, rung.line),
                vec![
                    yes(
                        "build",
                        "Give it the day",
                        leak(format!(
                            "Everyone lends a hand. {} is a part nearer.",
                            rung.label
                        )),
                        said(
                            leak(format!("{}_part_built", rung.id)),
                            leak(format!("Work went on at {}", the(rung.label))),
                            rung.progress,
                            bond(1, 0),
                        )
                        .and([Effect::Advance(rung.id), Effect::Advance(rung.id)])
                        .remembered("It's coming along, what we're building."),
                    ),
                    no(
                        "leave",
                        "Leave it to them",
                        "They'll get on with it their own way, slower.",
                        said(
                            leak(format!("{}_left_to_them", rung.id)),
                            leak(format!("{} went on slowly, their own way", rung.label)),
                            own[index % 2],
                            bond(0, 1),
                        )
                        .and([Effect::Advance(rung.id)]),
                    ),
                ],
                said(
                    leak(format!("{}_went_on", rung.id)),
                    leak(format!("{} went on without you", rung.label)),
                    own[2 + index % 2],
                    Vec::new(),
                )
                .and([Effect::Advance(rung.id)]),
            )
        })
        .collect()
}

/// What a place's turns ask the player, one at each turn that brings
/// people and stores: who comes, who goes, and what is put by. The answer
/// is kept on the universe for the next turn to act on.
fn turn_questions() -> Vec<Spec> {
    use Condition::Is;
    let asked = |value: &'static str| {
        [
            Effect::Set {
                entity: UNIVERSE,
                key: crate::years::ASKED,
                text: value,
            },
            Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            },
        ]
    };
    let shape = |asker: EntityId, seed: &'static str, id: &'static str| Shape {
        asker,
        want: false,
        requires: vec![
            Is(UNIVERSE, SEED, seed),
            Is(UNIVERSE, crate::years::QUESTION, id),
        ],
        lasts: 5,
        rests: 1,
        weight: 5,
        eases: Vec::new(),
        timely: true,
    };
    vec![
        spec(
            "turn_ares_crew",
            shape(SLOT_B, "mars-colony", "turn_ares_crew"),
            (
                "The next shuttle has bunks free",
                "The next shuttle can bring more crew. Shall I ask for them?",
            ),
            vec![
                yes(
                    "ask",
                    "Ask for more crew",
                    "More hands, more mouths.",
                    said(
                        "more_crew_asked",
                        "{keeper} asked the next shuttle for more crew",
                        "More bunks to make up, then.",
                        bond(1, 0),
                    )
                    .and(asked("more")),
                ),
                yes(
                    "enough",
                    "We're enough",
                    "Keep Ares small and well fed.",
                    said(
                        "crew_declined",
                        "{keeper} told the shuttle Ares was full",
                        "Just us, then. More stew each.",
                        bond(0, 1),
                    )
                    .and(asked("fewer")),
                ),
            ],
            said(
                "crew_unasked",
                "Nobody answered the shuttle's offer",
                "I'll take that as a no.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_ares_cargo",
            shape(SLOT_E, "mars-colony", "turn_ares_cargo"),
            (
                "{explorer} wants to fill the next cargo hold",
                "Next window: seed stock, or room for a passenger?",
            ),
            vec![
                yes(
                    "seed",
                    "Seed stock",
                    "Wheat for a year.",
                    said(
                        "seed_ordered",
                        "{explorer} ordered seed stock for the next window",
                        "Wheat for a year. Good call.",
                        bond(1, 0),
                    )
                    .and(asked("stores")),
                ),
                yes(
                    "passenger",
                    "Room for a passenger",
                    "Someone new rather than something new.",
                    said(
                        "passenger_ordered",
                        "{explorer} gave the cargo space to a passenger",
                        "A new face beats a new drill.",
                        bond(1, 1),
                    )
                    .and(asked("more")),
                ),
            ],
            said(
                "cargo_unasked",
                "The cargo hold went out half empty",
                "Half a hold. Oh well.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_ares_homesick",
            shape(SLOT_B, "mars-colony", "turn_ares_homesick"),
            (
                "Some of the crew are thinking of going home",
                "A few want to go home at the next window. Should we ask them to stay?",
            ),
            vec![
                yes(
                    "stay",
                    "Ask them to stay",
                    "Ares needs them.",
                    said(
                        "crew_asked_to_stay",
                        "{keeper} asked the crew to stay another tour",
                        "They said they'd think about it. That's a yes.",
                        bond(1, 0),
                    )
                    .and(asked("stay")),
                ),
                yes(
                    "go",
                    "Let them go",
                    "Nobody's kept here.",
                    said(
                        "crew_let_go",
                        "{keeper} told the crew they were free to go",
                        "Fair enough. Ares isn't for everyone.",
                        bond(0, 1),
                    )
                    .and(asked("go")),
                ),
            ],
            said(
                "homesick_unasked",
                "Nobody spoke to the homesick crew",
                "They'll decide for themselves, then.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_ares_welcome",
            shape(SLOT_E, "mars-colony", "turn_ares_welcome"),
            (
                "{explorer} wants a welcome for the next shuttle",
                "Shall we throw a welcome when the shuttle comes?",
            ),
            vec![
                yes(
                    "party",
                    "Throw a welcome",
                    "Streamers made of wire.",
                    said(
                        "welcome_planned",
                        "{explorer} planned a welcome for the next shuttle",
                        "I'll make a banner!",
                        bond(1, -1),
                    )
                    .and(asked("more")),
                ),
                yes(
                    "save",
                    "Save the rations",
                    "A full larder is a welcome too.",
                    said(
                        "welcome_saved",
                        "The rations were saved for the next window",
                        "Sensible. Boring, but sensible.",
                        bond(0, 1),
                    )
                    .and(asked("stores")),
                ),
            ],
            said(
                "welcome_unasked",
                "The welcome was never planned",
                "They'll just have to welcome themselves.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_ares_report",
            shape(SLOT_B, "mars-colony", "turn_ares_report"),
            (
                "The mission board wants the habitat's report",
                "The board wants our report. Do we ask for people, or supplies?",
            ),
            vec![
                yes(
                    "people",
                    "Ask for people",
                    "A place grows by its people.",
                    said(
                        "report_people",
                        "{keeper} asked the board for more people",
                        "People it is. I'll warn the bunk room.",
                        bond(1, 0),
                    )
                    .and(asked("more")),
                ),
                yes(
                    "supplies",
                    "Ask for supplies",
                    "Full shelves first.",
                    said(
                        "report_supplies",
                        "{keeper} asked the board for supplies",
                        "Supplies it is. The shelves will thank us.",
                        bond(0, 0),
                    )
                    .and(asked("stores")),
                ),
            ],
            said(
                "report_late",
                "The habitat's report went in late",
                "The board will send whatever it likes, then.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_ares_rotation",
            shape(SLOT_B, "mars-colony", "turn_ares_rotation"),
            (
                "The mission board wants to rotate the crew",
                "The board wants to swap half the crew. Do we agree?",
            ),
            vec![
                yes(
                    "swap",
                    "Agree to the swap",
                    "New faces, fresh eyes.",
                    said(
                        "rotation_agreed",
                        "{keeper} agreed to rotate the crew",
                        "Some goodbyes, some hellos.",
                        bond(0, 1),
                    )
                    .and(asked("go")),
                ),
                yes(
                    "keep",
                    "Keep the crew we have",
                    "They know Ares now.",
                    said(
                        "rotation_refused",
                        "{keeper} kept the crew together",
                        "We know each other now. That counts.",
                        bond(1, 0),
                    )
                    .and(asked("stay")),
                ),
            ],
            said(
                "rotation_unasked",
                "The board rotated the crew as it liked",
                "Nobody asked us. Typical.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_maple_house_for_sale",
            shape(SLOT_B, "1980s-town", "turn_maple_house_for_sale"),
            (
                "The old Henderson place is for sale",
                "Someone wants to buy the old Henderson place. Put in a good word?",
            ),
            vec![
                yes(
                    "good_word",
                    "Put in a good word",
                    "New neighbours, new faces.",
                    said(
                        "good_word_put_in",
                        "{keeper} put in a good word for the new buyers",
                        "I told them the street's the best in town. It is.",
                        bond(1, 0),
                    )
                    .and(asked("more")),
                ),
                yes(
                    "stay_out",
                    "Stay out of it",
                    "Not our house, not our business.",
                    said(
                        "sale_left_alone",
                        "{keeper} stayed out of the house sale",
                        "Let the realtor worry about it.",
                        bond(0, 1),
                    )
                    .and(asked("fewer")),
                ),
            ],
            said(
                "sale_went_by",
                "The Henderson place sat empty a while longer",
                "Still empty. Kids think it's haunted.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_maple_street_fund",
            shape(SLOT_E, "1980s-town", "turn_maple_street_fund"),
            (
                "{explorer} has ideas for the street fund",
                "The street fund's got money in it. A welcome party, or save it?",
            ),
            vec![
                yes(
                    "party",
                    "A welcome party",
                    "Balloons on every porch.",
                    said(
                        "welcome_party_planned",
                        "{explorer} planned a welcome party for new neighbours",
                        "Balloons! Cake! A banner!",
                        bond(1, -1),
                    )
                    .and(asked("more")),
                ),
                yes(
                    "save",
                    "Save it",
                    "A rainy day always comes.",
                    said(
                        "fund_saved",
                        "The street fund was saved for a rainy day",
                        "Sensible. Boring, but sensible.",
                        bond(0, 1),
                    )
                    .and(asked("stores")),
                ),
            ],
            said(
                "fund_forgotten",
                "Nobody decided what to do with the street fund",
                "It'll sit in the tin, then.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_maple_moving_away",
            shape(SLOT_B, "1980s-town", "turn_maple_moving_away"),
            (
                "A family on the street is thinking of moving away",
                "The folks at the end of the street might move. Should we ask them to stay?",
            ),
            vec![
                yes(
                    "ask",
                    "Ask them to stay",
                    "The street wouldn't be the same.",
                    said(
                        "asked_to_stay",
                        "{keeper} asked the family to stay on Maple Street",
                        "They're thinking about it. I baked a pie. Pies work.",
                        bond(1, 0),
                    )
                    .and(asked("stay")),
                ),
                yes(
                    "let_go",
                    "Wish them luck",
                    "People move on. That's life.",
                    said(
                        "wished_luck",
                        "{keeper} wished the family luck wherever they went",
                        "We'll throw them a going-away party.",
                        bond(0, 1),
                    )
                    .and(asked("go")),
                ),
            ],
            said(
                "moving_unasked",
                "Nobody talked to the family about moving",
                "They'll make up their own minds.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_maple_empty_lot",
            shape(SLOT_E, "1980s-town", "turn_maple_empty_lot"),
            (
                "A band wants to use the empty lot",
                "A band wants to rehearse in the empty lot. Let them?",
            ),
            vec![
                yes(
                    "yes",
                    "Let them play",
                    "Music brings people.",
                    said(
                        "band_welcomed",
                        "{explorer} let the band rehearse in the empty lot",
                        "Loud. Terrible. Wonderful.",
                        bond(1, 1),
                    )
                    .and(asked("more")),
                ),
                yes(
                    "no",
                    "Keep it quiet",
                    "Some people sleep at night.",
                    said(
                        "band_turned_away",
                        "{explorer} asked the band to find somewhere else",
                        "Quiet nights, then.",
                        bond(0, 1),
                    )
                    .and(asked("fewer")),
                ),
            ],
            said(
                "band_unanswered",
                "The band waited, then went elsewhere",
                "There they go.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_maple_road_widening",
            shape(SLOT_B, "1980s-town", "turn_maple_road_widening"),
            (
                "The city wants to widen the road",
                "The city will pay the street to widen the road. Take the money?",
            ),
            vec![
                yes(
                    "take",
                    "Take the money",
                    "The street fund could use it.",
                    said(
                        "road_money_taken",
                        "Maple Street took the city's money for the road",
                        "New road, full fund. Fewer trees.",
                        bond(0, 1),
                    )
                    .and(asked("stores")),
                ),
                yes(
                    "fight",
                    "Fight it",
                    "The oaks stay.",
                    said(
                        "road_fought",
                        "{keeper} led the street against the road",
                        "We signed a petition. Forty names!",
                        bond(1, 0),
                    )
                    .and(asked("stay")),
                ),
            ],
            said(
                "road_undecided",
                "The street never made up its mind about the road",
                "The city will decide for us, then.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_maple_school_board",
            shape(SLOT_E, "1980s-town", "turn_maple_school_board"),
            (
                "The school wants more families on the street",
                "The school's short of kids. Should we put up a sign for new families?",
            ),
            vec![
                yes(
                    "sign",
                    "Put up a sign",
                    "Families wanted. Good school, great street.",
                    said(
                        "sign_put_up",
                        "{explorer} put up a sign for new families",
                        "Hand-painted. Only one spelling mistake.",
                        bond(1, 0),
                    )
                    .and(asked("more")),
                ),
                yes(
                    "no_sign",
                    "Leave it",
                    "The street's full enough.",
                    said(
                        "no_sign",
                        "Nobody put up a sign",
                        "Fair enough. It's cosy as it is.",
                        bond(0, 1),
                    )
                    .and(asked("fewer")),
                ),
            ],
            said(
                "school_unanswered",
                "The school's letter went unanswered",
                "I'll write back. Eventually.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_ice_migrants",
            shape(SLOT_B, "penguin-civilization", "turn_ice_migrants"),
            (
                "Migrants are asking to join the colony",
                "A few from the migrating flocks want to stay. Let them?",
            ),
            vec![
                yes(
                    "welcome",
                    "Let them stay",
                    "More beaks, more songs.",
                    said(
                        "migrants_welcomed",
                        "{keeper} welcomed the migrants to the colony",
                        "Make room on the floe!",
                        bond(1, 0),
                    )
                    .and(asked("more")),
                ),
                yes(
                    "turn_away",
                    "The floe is full",
                    "There's only so much ice.",
                    said(
                        "migrants_turned_away",
                        "{keeper} told the migrants the floe was full",
                        "Sorry. There's only so much ice.",
                        bond(0, 1),
                    )
                    .and(asked("fewer")),
                ),
            ],
            said(
                "migrants_waited",
                "The migrants waited, then flew on",
                "Well. They didn't wait long.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_ice_vault",
            shape(SLOT_E, "penguin-civilization", "turn_ice_vault"),
            (
                "{explorer} is counting the vault for the dark",
                "The vault could feed newcomers, or keep us fat through the dark. Which?",
            ),
            vec![
                yes(
                    "share",
                    "Feed newcomers",
                    "A full colony is a warm colony.",
                    said(
                        "vault_shared",
                        "{explorer} opened the vault to newcomers",
                        "Come one, come all. Bring a fish if you can.",
                        bond(1, 0),
                    )
                    .and(asked("more")),
                ),
                yes(
                    "keep",
                    "Keep it for us",
                    "The dark is long.",
                    said(
                        "vault_kept",
                        "{explorer} kept the vault for the long dark",
                        "Fat and happy through the dark. That's the plan.",
                        bond(0, 1),
                    )
                    .and(asked("stores")),
                ),
            ],
            said(
                "vault_uncounted",
                "Nobody decided about the vault",
                "It'll sort itself out. Probably.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_ice_young_ones",
            shape(SLOT_B, "penguin-civilization", "turn_ice_young_ones"),
            (
                "Some of the young ones want to follow the flocks",
                "The young ones want to see the world. Ask them to stay?",
            ),
            vec![
                yes(
                    "stay",
                    "Ask them to stay",
                    "The colony needs them.",
                    said(
                        "young_asked_to_stay",
                        "{keeper} asked the young ones to stay",
                        "They sulked, then stayed. Mostly.",
                        bond(1, 1),
                    )
                    .and(asked("stay")),
                ),
                yes(
                    "go",
                    "Let them go",
                    "The sea is theirs too.",
                    said(
                        "young_let_go",
                        "{keeper} told the young ones the sea was theirs",
                        "Go on, then. Come back fat.",
                        bond(0, 0),
                    )
                    .and(asked("go")),
                ),
            ],
            said(
                "young_unasked",
                "Nobody talked to the young ones",
                "They'll go or they won't.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_ice_naming_feast",
            shape(SLOT_E, "penguin-civilization", "turn_ice_naming_feast"),
            (
                "The council wants a naming feast for the new ones",
                "A naming feast for the new ones, or save the fish?",
            ),
            vec![
                yes(
                    "feast",
                    "Hold the feast",
                    "Every new one deserves a name and a fish.",
                    said(
                        "naming_feast_held",
                        "{explorer} held a naming feast for the new ones",
                        "Every name sung, every fish eaten.",
                        bond(1, -1),
                    )
                    .and(asked("more")),
                ),
                yes(
                    "save",
                    "Save the fish",
                    "Names are free. Fish aren't.",
                    said(
                        "naming_feast_saved",
                        "The fish for the naming feast were saved",
                        "A quiet naming, then. Still counts.",
                        bond(0, 1),
                    )
                    .and(asked("stores")),
                ),
            ],
            said(
                "feast_forgotten",
                "The naming feast was forgotten",
                "Nobody remembered. Poor little ones.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_ice_trade",
            shape(SLOT_B, "penguin-civilization", "turn_ice_trade"),
            (
                "A neighbouring colony wants to trade",
                "The colony across the water wants fish for kelp. Trade?",
            ),
            vec![
                yes(
                    "trade",
                    "Make the trade",
                    "Kelp keeps longer than fish.",
                    said(
                        "trade_made",
                        "{keeper} traded fish for kelp with the colony across the water",
                        "Good kelp, fair trade.",
                        bond(0, 0),
                    )
                    .and(asked("stores")),
                ),
                yes(
                    "invite",
                    "Invite them over instead",
                    "Why trade, when you can share?",
                    said(
                        "neighbours_invited",
                        "{keeper} invited the colony across the water to visit",
                        "They're coming! Some might stay!",
                        bond(1, 1),
                    )
                    .and(asked("more")),
                ),
            ],
            said(
                "trade_ignored",
                "The neighbours' offer went unanswered",
                "They'll ask someone else.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
        spec(
            "turn_ice_early_flocks",
            shape(SLOT_E, "penguin-civilization", "turn_ice_early_flocks"),
            (
                "The flocks are leaving early this year",
                "The flocks are going early. Some of ours want to go with them. Well?",
            ),
            vec![
                yes(
                    "stay",
                    "Ask them to stay",
                    "Home is here.",
                    said(
                        "flocks_resisted",
                        "{explorer} asked the colony to stay together",
                        "We stay. All of us.",
                        bond(1, 0),
                    )
                    .and(asked("stay")),
                ),
                yes(
                    "go",
                    "Let them choose",
                    "Everyone chooses their own ice.",
                    said(
                        "flocks_chosen",
                        "{explorer} let everyone choose their own way",
                        "Some will go. Some will stay. That's fair.",
                        bond(0, 1),
                    )
                    .and(asked("go")),
                ),
            ],
            said(
                "flocks_unasked",
                "The flocks left, and nobody said anything",
                "Quiet on the ice tonight.",
                Vec::new(),
            )
            .and([Effect::Set {
                entity: UNIVERSE,
                key: crate::years::QUESTION,
                text: "",
            }]),
        ),
    ]
}

/// The questions each place's turns ask, in the order they come round.
pub(crate) fn turn_question_ids(place: crate::places::Place) -> &'static [&'static str] {
    match place {
        crate::places::Place::Ares => &[
            "turn_ares_crew",
            "turn_ares_cargo",
            "turn_ares_homesick",
            "turn_ares_welcome",
            "turn_ares_report",
            "turn_ares_rotation",
        ],
        crate::places::Place::Maple => &[
            "turn_maple_house_for_sale",
            "turn_maple_street_fund",
            "turn_maple_moving_away",
            "turn_maple_empty_lot",
            "turn_maple_road_widening",
            "turn_maple_school_board",
        ],
        crate::places::Place::Ice => &[
            "turn_ice_migrants",
            "turn_ice_vault",
            "turn_ice_young_ones",
            "turn_ice_naming_feast",
            "turn_ice_trade",
            "turn_ice_early_flocks",
        ],
    }
}

fn specs() -> &'static [Spec] {
    static SPECS: OnceLock<Vec<Spec>> = OnceLock::new();
    SPECS.get_or_init(|| {
        let mut specs = wants();
        specs.extend(incidents());
        specs.extend(calendar());
        specs.extend(more());
        specs.extend(threads());
        specs.extend(climaxes());
        specs.extend(works());
        specs.extend(turn_questions());
        specs
    })
}

/// The storyteller's deck, as the `storylets` System asks for it.
pub(crate) fn deck() -> Deck {
    deck_ref().clone()
}

/// The storyteller's deck, made once.
pub(crate) fn deck_ref() -> &'static Deck {
    static DECK: OnceLock<Deck> = OnceLock::new();
    DECK.get_or_init(make_deck)
}

fn make_deck() -> Deck {
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
        ]
        .into_iter()
        .chain(all_rungs().map(|(_, rung)| Goal {
            id: rung.id,
            parts: RUNG_PARTS,
        }))
        .collect(),
        chapter_periods: CHAPTER_PERIODS,
        shortest_chapter: 10,
        pressures: vec!["the_long_dark", "breaking_point", "the_call"],
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
        rarer: 60,
    }
}

pub(crate) fn register_actions(
    actions: &mut ActionRegistry,
) -> Result<(), world_core::ActionError> {
    storylets::register_actions(actions, deck)?;
    lives::register_actions(actions, crate::life::cast)?;
    hands::register_actions(actions, crate::handwork::kit)?;
    conversation::register_actions(actions, crate::speech::kit)?;
    calendar::register_actions(actions, crate::almanac::almanac)?;
    actions.register(LendsToWork)?;
    actions.register(BondSettles)
}

/// When the player last lent a hand with the place's work under way, in
/// periods.
const LENT_WORK: &str = "story.lent_work";
/// The fewest periods between two hands lent to the place's works.
const LEND_WORK_EVERY: i64 = 30;

/// The work of the place's ladder under way, once its first three goals
/// are done.
fn rung_under_way(state: &world_core::WorldState) -> Option<&'static Rung> {
    let deck = deck_ref();
    let first_done = ["second_home", "beacon", "survey"]
        .iter()
        .all(|goal| storylets::finished(state, deck, goal));
    if !first_done {
        return None;
    }
    ladder(crate::places::Place::of(state)?)
        .iter()
        .find(|rung| storylets::progress(state, deck, rung.id) < RUNG_PARTS)
}

/// The player, making something, lends a hand with the place's work under
/// way, a month or more since they last did: what they make counts toward
/// the place's own ladder, so a maker's place goes further than one left
/// to itself.
struct LendsToWork;

impl world_core::Action for LendsToWork {
    fn name(&self) -> &'static str {
        "lend_to_work"
    }

    fn evaluate(
        &self,
        state: &world_core::WorldState,
        _request: &world_core::ActionRequest,
    ) -> Result<world_core::EventDraft, world_core::ActionError> {
        let deck = deck_ref();
        let rung = rung_under_way(state)
            .ok_or_else(|| world_core::ActionError::Invalid("no work under way".into()))?;
        let now = storylets::period_index(state, deck) as i64;
        let last = state
            .entity(STORY)
            .and_then(|story| match story.component(LENT_WORK) {
                Some(Value::Integer(at)) => Some(*at),
                _ => None,
            });
        if state.entity(STORY).is_none() || last.is_some_and(|last| now - last < LEND_WORK_EVERY) {
            return Err(world_core::ActionError::Invalid(
                "lent a hand lately".into(),
            ));
        }
        let done = storylets::progress(state, deck, rung.id);
        let mut draft = world_core::EventDraft::new("hand_lent");
        draft.actor = Some(rung.work.champion);
        draft.targets = vec![rung.work.champion];
        draft.payload.insert("work".into(), rung.id.into());
        draft.payload.insert(
            "told".into(),
            format!("You lent a hand with {}", the(rung.label)).into(),
        );
        draft.changes = vec![
            world_core::StateChange::SetComponent {
                entity: STORY,
                key: format!("story.goal.{}", rung.id),
                value: (done + 1).into(),
            },
            world_core::StateChange::SetComponent {
                entity: STORY,
                key: LENT_WORK.into(),
                value: now.into(),
            },
        ];
        Ok(draft)
    }
}

/// After the player makes something, they lend a hand with the place's
/// work under way, if there is one and they have not lately.
pub(crate) fn lend_to_work(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Option<EventId>, WorldError> {
    match world.execute(actions, &world_core::ActionRequest::new("lend_to_work")) {
        Ok(event) => Ok(Some(event.id)),
        Err(WorldError::Action(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Nothing between two people stays near either end for long: a period
/// with trust or tension this near an end eases it one step back.
struct BondSettles;
const SETTLES_BELOW: i64 = 1;
const SETTLES_ABOVE: i64 = 9;

/// Whether trust or tension is near enough an end to ease back.
fn unsettled(world: &World) -> bool {
    [RELATIONSHIP_TRUST, RELATIONSHIP_TENSION]
        .into_iter()
        .any(|key| !(SETTLES_BELOW + 1..SETTLES_ABOVE).contains(&integer(world, RELATIONSHIP, key)))
}

impl world_core::Action for BondSettles {
    fn name(&self) -> &'static str {
        "bond_settles"
    }

    fn evaluate(
        &self,
        state: &world_core::WorldState,
        _request: &world_core::ActionRequest,
    ) -> Result<world_core::EventDraft, world_core::ActionError> {
        let mut draft = world_core::EventDraft::new("bond_settled");
        for key in [RELATIONSHIP_TRUST, RELATIONSHIP_TENSION] {
            let value = match state
                .entity(RELATIONSHIP)
                .and_then(|bond| bond.component(key))
            {
                Some(Value::Integer(value)) => *value,
                _ => continue,
            };
            let eased = match value {
                ..=SETTLES_BELOW => value + 1,
                SETTLES_ABOVE.. => value - 1,
                _ => continue,
            };
            draft.changes.push(world_core::StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: key.into(),
                value: eased.into(),
            });
        }
        if draft.changes.is_empty() {
            return Err(world_core::ActionError::Invalid(
                "neither trust nor tension is at an end".into(),
            ));
        }
        draft.targets = vec![RELATIONSHIP];
        Ok(draft)
    }
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
pub(crate) fn fill(world: &World, text: &str) -> String {
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
        storylets::period_index(world.state(), deck_ref()),
        SEASON_PERIODS,
    ) as usize
}

/// What a moment adds to the chapter it happened in.
fn chapter_line(event: &Event) -> Option<&'static str> {
    Some(match event.kind.as_str() {
        "edge_claimed" => "{explorer} planted a flag past the edge of the map.",
        "second_ours" => "{keeper} and {explorer} moved into {second}.",
        "second_kept" => "{second} was finished and kept for whoever comes.",
        "newcomer_arrived" => "Someone new came to live with them.",
        "garden_planted" => "{keeper} planted a garden.",
        "photo_taken" => "They had their picture taken together.",
        "amends_made" => "{explorer} and {keeper} made up after a long quarrel.",
        "explorer_rescued" => "{keeper} went out into the cold for {explorer}.",
        "beacon_invited" => "Someone answered the beacon, and was invited.",
        _ => return None,
    })
}

/// How a chapter ends: how its climax went, and what happened in it that
/// they will remember.
fn chapter_ending(world: &World) -> (String, String) {
    let deck = deck_ref();
    let (_, started) = storylets::chapter(world.state(), deck);
    let mut title = None;
    let mut lines = Vec::<String>::new();
    for event in world
        .events()
        .iter()
        .filter(|event| event.world_time >= started)
    {
        let said = outcome_of(event);
        if let Some(named) = said.and_then(|said| said.title) {
            title = Some(fill(world, named));
        }
        if let Some(line) = said
            .and_then(|said| said.chapter)
            .or_else(|| chapter_line(event))
        {
            let line = fill(world, line);
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
    }
    let trust = integer(world, RELATIONSHIP, RELATIONSHIP_TRUST);
    let tension = integer(world, RELATIONSHIP, RELATIONSHIP_TENSION);
    // Without a climax to name it, a chapter is named first for the work
    // the place last finished in it, if it finished one.
    let season_name = SEASONS[season(world)];
    let named_for_work = title.is_none().then(|| {
        let place = crate::places::Place::of(world.state())?;
        let last = ladder(place)
            .iter()
            .take_while(|rung| storylets::progress(world.state(), deck, rung.id) >= RUNG_PARTS)
            .last()?;
        let finished_now = world
            .events()
            .iter()
            .rev()
            .take_while(|event| event.world_time >= started)
            .any(|event| event.kind.starts_with(last.id) && event.kind.len() > last.id.len());
        finished_now.then(|| format!("The {season_name} of {}", the(last.work.label)))
    });
    // Or for the work it went on with, if it went on with one.
    let named_for_work_in_hand = title.is_none().then(|| {
        let place = crate::places::Place::of(world.state())?;
        let next = ladder(place)
            .iter()
            .find(|rung| storylets::progress(world.state(), deck, rung.id) < RUNG_PARTS)?;
        let worked = world
            .events()
            .iter()
            .rev()
            .take_while(|event| event.world_time >= started)
            .any(|event| event.kind.starts_with(next.id) && event.kind.len() > next.id.len());
        worked.then(|| format!("The {season_name} they worked on {}", the(next.work.label)))
    });
    let title = title.unwrap_or_else(|| {
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
        format!("{feel} {}", SEASONS[season(world)])
    });
    // What changed between people this chapter, and a title no chapter
    // has had before.
    let news = lives::news_since(world, started);
    let mut candidates = named_for_work
        .flatten()
        .into_iter()
        .chain(named_for_work_in_hand.flatten())
        .collect::<Vec<_>>();
    candidates.push(title.clone());
    if let Some(first) = news.first() {
        candidates.push(format!("The {season_name} {}", lowered_start(first)));
    }
    let year = world.world_time() / crate::BACKGROUND_PERIOD / YEAR + 1;
    candidates.push(storylets::title_with_year(&title, season_name, year));
    candidates.push(format!(
        "The {season_name} of year {}",
        storylets::number_word(year)
    ));
    let lived = (world.world_time().saturating_sub(started)) / crate::BACKGROUND_PERIOD;
    let candidates = candidates
        .into_iter()
        .map(|title| storylets::fitted_title(title, lived, YEAR, season_name))
        .collect::<Vec<_>>();
    let title = storylets::unused_title(world, &candidates);
    let mut summary = if lines.len() > 3 {
        lines.split_off(lines.len() - 3)
    } else {
        lines
    };
    for line in news.iter().rev().take(2).rev() {
        summary.push(format!("{line}."));
    }
    if summary.is_empty() {
        summary.push(if trust >= 7 {
            fill(
                world,
                "{keeper} and {explorer} would trust each other with anything.",
            )
        } else if tension >= 7 {
            fill(world, "{keeper} and {explorer} could barely share a room.")
        } else {
            fill(world, "{keeper} and {explorer} kept each other going.")
        });
    }
    // What the player made this chapter is part of how it is told.
    if let Some(made) = hands::latest_made_since(world, started) {
        summary.push(format!("{made}."));
    }
    for who in [SLOT_B, SLOT_E] {
        let (granted, grudges) = storylets::kindness(world.state(), deck, who);
        if grudges > granted + 1 {
            let name = noun(world, if who == SLOT_B { "keeper" } else { "explorer" });
            summary.push(format!("{name} hasn't forgotten being let down."));
            break;
        }
    }
    (title, summary.join(" "))
}

/// A sentence as it reads after "The autumn": "The whole colony came"
/// turns to "the whole colony came".
fn lowered_start(sentence: &str) -> String {
    match sentence.split_once(' ') {
        Some((first, rest)) if matches!(first, "The" | "A" | "An") => {
            format!("{} {rest}", first.to_lowercase())
        }
        _ => sentence.to_string(),
    }
}

/// The turning point of each chapter.
fn climaxes() -> Vec<Spec> {
    use Condition::{ChapterEnding, Pressure};
    let climax = |asker: EntityId, pressure: &'static str| Shape {
        asker,
        want: false,
        requires: vec![Pressure(pressure), ChapterEnding(5)],
        lasts: 3,
        rests: 30,
        weight: 9,
        eases: Vec::new(),
        timely: true,
    };
    vec![
        spec(
            "long_dark",
            climax(SLOT_B, "the_long_dark"),
            (
                "The long dark has set in",
                "{Weather} for a week. Do we hole up together, or keep the work going?",
            ),
            vec![
                yes(
                    "together",
                    "Hole up together",
                    "A week of cards, soup and stories.",
                    said(
                        "long_dark_together",
                        "{keeper} and {explorer} waited out the long dark together",
                        "Best week I can remember.",
                        bond(2, -2),
                    )
                    .chapter("Through the long dark, {keeper} and {explorer} held on together.")
                    .titled("The long dark"),
                ),
                yes(
                    "work",
                    "Keep the work going",
                    "{explorer} goes out in it every day.",
                    said(
                        "long_dark_worked",
                        "{explorer} kept working through the long dark",
                        "Somebody has to.",
                        bond(-1, 2),
                    )
                    .and([Effect::Advance("survey")])
                    .chapter("Through the long dark, {explorer} kept the work going alone.")
                    .titled("The long dark, worked through"),
                ),
            ],
            said(
                "long_dark_passed",
                "The long dark passed",
                "Is it over? It's over.",
                bond(0, 1),
            )
            .chapter("The long dark came and went.")
            .titled("The long dark"),
        ),
        spec(
            "breaking_point",
            climax(SLOT_E, "breaking_point"),
            (
                "{explorer} has had enough",
                "I can't go on like this. Something has to change.",
            ),
            vec![
                yes(
                    "talk",
                    "Talk it all through",
                    "A whole night, everything said.",
                    said(
                        "breaking_point_talked",
                        "{keeper} and {explorer} talked it all through",
                        "I didn't know you felt that way.",
                        bond(2, -3),
                    )
                    .chapter("At the breaking point, they talked it all through.")
                    .titled("The night we talked"),
                ),
                yes(
                    "space",
                    "Give each other space",
                    "{explorer} sleeps at {place} for a while.",
                    said(
                        "breaking_point_apart",
                        "{keeper} and {explorer} took some time apart",
                        "Just for a while.",
                        bond(-1, -1),
                    )
                    .chapter("At the breaking point, they took time apart.")
                    .titled("Time apart"),
                ),
            ],
            said(
                "breaking_point_passed",
                "Nobody said what needed saying",
                "Forget I said anything.",
                bond(-1, 2),
            )
            .chapter("At the breaking point, nothing was said.")
            .titled("What went unsaid"),
        ),
        spec(
            "the_call",
            climax(SLOT_E, "the_call"),
            (
                "The signal is getting stronger",
                "The signal's stronger every night. Go and find it?",
            ),
            vec![
                yes(
                    "go",
                    "Go and find it",
                    "{explorer} sets off; {keeper} keeps {home}.",
                    said(
                        "the_call_answered",
                        "{explorer} went looking for the signal",
                        "I'll be back. I promise.",
                        bond(0, 1),
                    )
                    .and([Effect::Advance("survey"), Effect::Mark("invited")])
                    .chapter("{explorer} followed the call, and found someone at the end of it.")
                    .titled("The call"),
                ),
                yes(
                    "stay",
                    "Stay home",
                    "Some calls go unanswered.",
                    said(
                        "the_call_ignored",
                        "{explorer} stayed home and let the signal go",
                        "Some things are best left.",
                        bond(1, 0),
                    )
                    .chapter("{explorer} let the call go unanswered, and stayed.")
                    .titled("The call we let go"),
                ),
            ],
            said(
                "the_call_faded",
                "The signal faded",
                "Gone. Whatever it was.",
                bond(0, 0),
            )
            .chapter("The call faded before anyone answered it.")
            .titled("The call that faded"),
        ),
    ]
}

/// One period of the storyteller, once a World has begun.
pub(crate) fn tick(
    world: &mut World,
    actions: &ActionRegistry,
    away: bool,
) -> Result<Vec<EventId>, WorldError> {
    if seed_id(world) == "unseeded" {
        return Ok(Vec::new());
    }
    let mut events = Vec::new();
    let begins = world.state().entity(STORY).is_none();
    // Reaching an end is a turning point, even as it starts to ease.
    let ended = at_end(world);
    if unsettled(world) {
        events.push(
            world
                .execute(actions, &world_core::ActionRequest::new("bond_settles"))?
                .id,
        );
    }
    let reading = Reading {
        pinned: pinned(world),
        away,
        at_end: ended,
        chapter_ending: Box::new(chapter_ending),
        hold: waiting_for_the_player(world),
    };
    // What the year has turned to (someone takes something on, or a new
    // festival joins the year) comes first, so the day's round of lives
    // knows whether the day has already brought something new.
    events.extend(crate::years::tick(world, actions)?);
    let cast = crate::life::cast(world.state());
    events.extend(lives::tick_with(
        world,
        actions,
        &cast,
        away,
        reading.hold,
        &crate::firsts::quiet_days(world.state()),
    )?);
    let kit = crate::handwork::kit(world.state());
    events.extend(hands::tick(world, actions, &kit)?);
    events.extend(crate::plots::draw(world, actions)?);
    let almanac = crate::almanac::almanac(world.state());
    events.extend(calendar::tick(world, actions, &almanac)?);
    events.extend(storylets::tick(world, actions, deck_ref(), &reading)?);
    // A place whose story begins now opens its plots over the years.
    if begins {
        let kit = crate::handwork::kit(world.state());
        events.extend(hands::stage_plots(world, actions, &kit)?);
    }
    // Each season turning brings a gift, and a year on, someone remembers.
    events.extend(lives::season_turns(world, actions, &cast, SEASON_PERIODS)?);
    events.extend(lives::remember_a_year(world, actions, &cast, YEAR)?);
    // Last of all, someone may ask the player a favour talk can do.
    events.extend(crate::speech::favour_asked(world, actions, away)?);
    Ok(events)
}

/// Whether a new World is still waiting for the player's first deed
/// before anyone asks them anything: nothing has come up yet, the player
/// has neither made, given nor said anything, and its first period has not
/// passed.
fn waiting_for_the_player(world: &World) -> bool {
    let deck = deck_ref();
    let state = world.state();
    if storylets::anything_raised(state, deck) {
        return false;
    }
    let acted = state.entity(crate::handwork::kit(state).notes).is_some()
        || world.events().iter().any(conversation::is_talk);
    let (_, started) = storylets::chapter(state, deck);
    let first_period = state.entity(deck.story).is_none()
        || storylets::period_index(state, deck) <= started / deck.period.max(1);
    !acted && first_period
}

/// Once the player has done something of their own in a new World, the
/// first question comes straight after, if it has not come already.
pub(crate) fn after_first_deed(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    if waiting_for_the_player(world) {
        return Ok(Vec::new());
    }
    first_question(world, actions)
}

/// The first question, if nothing has been asked yet: a new World opens on
/// it, just after the hello, so the player has something to answer from
/// the start.
pub(crate) fn first_question(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    if seed_id(world) == "unseeded" || storylets::anything_raised(world.state(), deck_ref()) {
        return Ok(Vec::new());
    }
    let reading = Reading {
        pinned: pinned(world),
        away: false,
        at_end: at_end(world),
        chapter_ending: Box::new(chapter_ending),
        hold: false,
    };
    storylets::tick(world, actions, deck_ref(), &reading)
}

/// How a question is opened when it has come round before: never in last
/// time's words, and with what happened then.
const AGAIN: [&str; 6] = [
    "Here we are again.",
    "It's come round again.",
    "You'll remember this one.",
    "Same as before, I'm afraid.",
    "This again.",
    "Back to this, then.",
];

/// What the asker says as they ask, the `times`th time it has come up,
/// having ended `last` the time before.
fn asked(spec: &Spec, times: i64, last: Option<&str>) -> String {
    if times <= 1 {
        return spec.line.to_string();
    }
    let opener = AGAIN[((times - 2) as usize) % AGAIN.len()];
    let then = last.and_then(|last| {
        if last == "lapse" {
            Some(&spec.lapse)
        } else {
            spec.answers
                .iter()
                .find(|answer| answer.id == last)
                .map(|answer| &answer.said)
        }
    });
    match then {
        // Run on as one sentence: "Last time round, everyone pitched in",
        // never "Last time: Everyone".
        Some(said) => format!(
            "{opener} Last time round, {}. {}",
            world_projection::lowered(said.told, crate::legends::names()),
            spec.line
        ),
        None => format!("{opener} {}", spec.line),
    }
}

/// What the asker of an open question says as they ask it now.
fn asking(world: &World, spec: &Spec) -> String {
    let deck = deck_ref();
    let id = spec.storylet.id;
    fill(
        world,
        &asked(
            spec,
            storylets::times_raised(world.state(), deck, id),
            storylets::last_outcome(world.state(), deck, id),
        ),
    )
}

/// What someone says about how the player answered them, with `{ago}`
/// where when it was goes.
pub(crate) fn recalled(world: &World, event: &Event, who: EntityId) -> Option<String> {
    if event.actor != Some(who) || event.payload.contains_key("lapsed") {
        return None;
    }
    let spec = storylet_of(event)?;
    let choice = match event.payload.get("choice") {
        Some(Value::Text(choice)) => choice.as_str(),
        _ => return None,
    };
    let answer = spec.answers.iter().find(|answer| answer.id == choice)?;
    let after = match (answer.refuses, answer.said.remembered) {
        (true, _) => "I haven't forgotten.".to_string(),
        (false, Some(remembered)) => fill(world, remembered),
        (false, None) => "Thank you for that.".to_string(),
    };
    Some(format!(
        "When I asked you {{ago}}, you said “{}”. {after}",
        fill(world, answer.title).trim_end_matches('.')
    ))
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

/// A card for every answer that can be given now: the storyteller's, and
/// what people's lives have brought up.
pub(crate) fn commands(world: &World) -> Vec<world_projection::ProjectionCommand> {
    let mut commands = storylet_commands(world);
    commands.extend(crate::life::commands(world));
    commands
}

fn storylet_commands(world: &World) -> Vec<world_projection::ProjectionCommand> {
    let deck = deck_ref();
    storylets::answers(world.state(), deck)
        .into_iter()
        .filter_map(|(storylet, choice, unmet)| {
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
                    prompt: asking(world, spec),
                }),
                unavailable: (!unmet.is_empty()).then(|| "Not possible right now".to_string()),
                hand: None,
                preview: None,
            })
        })
        .collect()
}

/// What someone would ask for now, if they have a want open, and the answer
/// that grants it.
pub(crate) fn wanting(world: &World, who: EntityId) -> Option<(String, Option<String>)> {
    let deck = deck_ref();
    let storylet = storylets::open(world.state(), deck)
        .into_iter()
        .find(|storylet| storylet.want && storylet.asker == who)?;
    let spec = find(storylet.id)?;
    let grant = storylets::choices(world.state(), deck)
        .into_iter()
        .find(|(open, choice)| open.id == storylet.id && !choice.refuses)
        .map(|(open, choice)| command_id(open.id, choice.id));
    Some((asking(world, spec), grant))
}

pub(crate) fn kindness(world: &World, who: EntityId) -> (i64, i64) {
    storylets::kindness(world.state(), deck_ref(), who)
}

fn storylet_of(event: &Event) -> Option<&'static Spec> {
    match event.payload.get("storylet")? {
        Value::Text(id) => find(id),
        _ => None,
    }
}

/// How one of the storyteller's questions was settled, when `event`
/// settles one: the words of the answer the player chose, as they were
/// offered, or `None` inside when nobody answered in time. For legends:
/// "because you said …".
pub(crate) fn answer_words(world: &World, event: &Event) -> Option<Option<String>> {
    let spec = storylet_of(event)?;
    let said = outcome_of(event)?;
    if std::ptr::eq(said, &spec.lapse) {
        return Some(None);
    }
    let answer = spec
        .answers
        .iter()
        .find(|answer| std::ptr::eq(&answer.said, said))?;
    Some(Some(
        fill(world, answer.title).trim_end_matches('.').to_string(),
    ))
}

/// Whether `event` is an answer that turned the asker down.
pub(crate) fn answer_refuses(event: &Event) -> bool {
    let Some(spec) = storylet_of(event) else {
        return false;
    };
    spec.answers
        .iter()
        .any(|answer| answer.refuses && answer.said.event == event.kind)
}

/// The Events these storylets' answers and lapses are recorded as.
pub(crate) fn outcome_kinds(storylets: &[&str]) -> Vec<&'static str> {
    specs()
        .iter()
        .filter(|spec| storylets.contains(&spec.storylet.id))
        .flat_map(|spec| {
            spec.answers
                .iter()
                .map(|answer| answer.said.event)
                .chain([spec.lapse.event])
                .chain(spec.taken_up.iter().map(|taken| taken.event))
        })
        .collect()
}

fn outcome_of(event: &Event) -> Option<&'static Said> {
    let spec = storylet_of(event)?;
    if event.kind == "situation_arose" {
        return None;
    }
    if spec.lapse.event == event.kind {
        return Some(&spec.lapse);
    }
    if let Some(taken) = spec
        .taken_up
        .as_ref()
        .filter(|taken| taken.event == event.kind)
    {
        return Some(taken);
    }
    spec.answers
        .iter()
        .map(|answer| &answer.said)
        .find(|said| said.event == event.kind)
}

/// How one of the storyteller's moments is told.
pub(crate) fn told(world: &World, event: &Event) -> Option<String> {
    if event.kind == "hand_lent" {
        return match event.payload.get("told") {
            Some(Value::Text(told)) => Some(told.clone()),
            _ => None,
        };
    }
    if event.kind == "chapter_ended" {
        return match event.payload.get("title") {
            Some(Value::Text(title)) => Some(format!("{title} came to an end")),
            _ => None,
        };
    }
    // Being greeted is the first thing that happens to a newcomer.
    if lives::is_news(event) || event.kind == "greeted" {
        return lives::told(event).map(|told| crate::plots::renamed(world.state(), event, told));
    }
    if hands::is_hands(event) {
        return hands::told(event);
    }
    // Getting ready is part of everyday life; the day itself is a story.
    if calendar::is_calendar(event) {
        return (event.kind == "festival_held")
            .then(|| calendar::told(event))
            .flatten();
    }
    let spec = storylet_of(event)?;
    if event.kind == "situation_arose" {
        // Told against the times it came before.
        let told = fill(world, spec.told);
        return Some(match event.payload.get("times") {
            // "…going again" is not told "again again".
            Some(Value::Integer(2)) if told.ends_with(" again") => told,
            Some(Value::Integer(2)) => format!("{told} again"),
            Some(Value::Integer(times)) if *times > 2 => format!("{told} once more"),
            _ => told,
        });
    }
    Some(fill(world, outcome_of(event)?.told))
}

/// What someone says getting on with a work on their own: the next of
/// all the place's words for it, counted over every time, so the same is
/// not said again until each of the others has been.
fn own_way(world: &World, event: &Event, line: &'static str) -> &'static str {
    let Some(place) = crate::places::Place::of(world.state()) else {
        return line;
    };
    let own = crate::works::on_their_own(place);
    if !own.contains(&line) {
        return line;
    }
    let all = own
        .iter()
        .chain(crate::works::more_on_their_own(place).iter())
        .copied()
        .collect::<Vec<_>>();
    let on_their_own = |kind: &str| kind.ends_with("_went_on") || kind.ends_with("_left_to_them");
    let before = world
        .events()
        .iter()
        .take_while(|other| other.id != event.id)
        .filter(|other| on_their_own(&other.kind))
        .count();
    all[before % all.len()]
}

/// Who speaks at one of the storyteller's moments, and what they say.
pub(crate) fn line(world: &World, event: &Event) -> Option<(EntityId, String)> {
    if lives::is_life(event) {
        return lives::said(event);
    }
    if calendar::is_calendar(event) {
        return calendar::said(event);
    }
    if hands::is_hands(event) {
        return crate::handwork::enjoyed_line(world, event).or_else(|| hands::said(event));
    }
    let spec = storylet_of(event)?;
    let asker = spec.storylet.asker;
    let other = if asker == SLOT_B { SLOT_E } else { SLOT_B };
    if event.kind == "situation_arose" {
        let times = match event.payload.get("times") {
            Some(Value::Integer(times)) => *times,
            _ => 1,
        };
        let last = match event.payload.get("last") {
            Some(Value::Text(last)) => Some(last.as_str()),
            _ => None,
        };
        return Some((asker, fill(world, &asked(spec, times, last))));
    }
    let said = outcome_of(event)?;
    Some((
        if said.by_other { other } else { asker },
        fill(world, own_way(world, event, said.line)),
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
    let deck = deck_ref();
    let (home, beacon) = match seed_id(world) {
        "mars-colony" => (MarkShape::Dome, MarkShape::Tower),
        "1980s-town" => (MarkShape::Shop, MarkShape::Tower),
        _ => (MarkShape::Bridge, MarkShape::Lamp),
    };
    let goal = |id: &str, label: String, shape: MarkShape| {
        let parts = deck.goals.iter().find(|goal| goal.id == id)?.parts;
        Some(world_projection::Goal {
            id: id.into(),
            label,
            shape,
            done: storylets::progress(world.state(), deck, id).clamp(0, parts) as u32,
            parts: parts as u32,
        })
    };
    let mut goals = [
        ("second_home", "{second}", home),
        ("beacon", "{beacon}", beacon),
        ("survey", "The map past the edge", MarkShape::Rover),
    ]
    .into_iter()
    .filter_map(|(id, label, shape)| goal(id, fill(world, label), shape))
    .collect::<Vec<_>>();
    // Then the works done so far and the one in hand; what comes after is
    // not known yet.
    // What the people made on their own, when the player lent a hand.
    for (at, (label, shape)) in crate::years::own_works_finished(world.state())
        .into_iter()
        .enumerate()
    {
        goals.push(world_projection::Goal {
            id: format!("own_{at}"),
            label,
            shape,
            done: 1,
            parts: 1,
        });
    }
    let place = crate::places::Place::of(world.state());
    if let Some(place) = place.filter(|_| goals.iter().all(|goal| goal.done >= goal.parts)) {
        for rung in ladder(place) {
            let Some(next) = goal(rung.id, rung.label.to_string(), rung.work.shape) else {
                continue;
            };
            let finished = next.done >= next.parts;
            goals.push(next);
            if !finished {
                break;
            }
        }
    }
    goals
}

/// The chapters of the pair's story that have ended.
pub(crate) use storylets::chapters_shown as chapters;

// What answers leave behind.

pub(crate) const SECOND: EntityId = EntityId::new(21);
pub(crate) const BEACON: EntityId = EntityId::new(22);
pub(crate) const EDGE_FLAG: EntityId = EntityId::new(23);
pub(crate) const GARDEN: EntityId = EntityId::new(24);
pub(crate) const LANTERNS: EntityId = EntityId::new(25);
pub(crate) const BUNTING: EntityId = EntityId::new(26);
pub(crate) const TENT: EntityId = EntityId::new(27);
pub(crate) const PENNANT: EntityId = EntityId::new(28);
pub(crate) const FLOWERS: EntityId = EntityId::new(31);
pub(crate) const PARCEL: EntityId = EntityId::new(32);
pub(crate) const REST: EntityId = EntityId::new(33);
pub(crate) const MARKERS: EntityId = EntityId::new(34);
pub(crate) const LAMPS: EntityId = EntityId::new(35);
pub(crate) const WASHING: EntityId = EntityId::new(36);
/// Someone who comes to live here.
pub(crate) const NEWCOMER: EntityId = EntityId::new(30);

fn leaves_behind(event: &str) -> Vec<Effect> {
    match event {
        "second_begun" | "supply_built" => {
            vec![build(SECOND, "{second}, going up", "second", SLOT_A, None)]
        }
        "signal_followed" => vec![
            mark("signal"),
            build(BEACON, "{beacon}, going up", "beacon", SLOT_A, None),
        ],
        "beacon_raised" => {
            vec![build(BEACON, "{beacon}, going up", "beacon", SLOT_A, None)]
        }
        "supper_shared" => vec![
            mark("supper"),
            build(LANTERNS, "Lanterns from supper", "lantern", SLOT_A, Some(3)),
        ],
        "keeper_birthday_kept" | "explorer_birthday_kept" => vec![build(
            BUNTING,
            "Birthday bunting",
            "bunting",
            SLOT_A,
            Some(2),
        )],
        "window_gone" => vec![build(
            PENNANT,
            "A pennant from {window}",
            "flag",
            SLOT_A,
            Some(5),
        )],
        "amends_made" => vec![
            mark("made_up"),
            build(
                FLOWERS,
                "Flowers, by way of sorry",
                "garden",
                SLOT_A,
                Some(3),
            ),
        ],
        "spare_fetched" => vec![
            mark("spare"),
            build(PARCEL, "{spare}, just arrived", "parcel", SLOT_A, Some(3)),
        ],
        "keeper_rested" => vec![
            mark("rested"),
            build(REST, "{keeper}'s day off", "tent", SLOT_C, Some(2)),
        ],
        "keeper_taught" => vec![
            mark("taught"),
            build(
                MARKERS,
                "{explorer}'s route markers",
                "flag",
                SLOT_D,
                Some(4),
            ),
        ],
        "edge_explored" => vec![
            mark("edge"),
            build(MARKERS, "A marker past the edge", "flag", SLOT_D, Some(3)),
        ],
        "weather_weathered" | "long_dark_together" => vec![
            mark("weather_hit"),
            build(
                LAMPS,
                "Storm lamps in every window",
                "lantern",
                SLOT_A,
                Some(3),
            ),
        ],
        "weather_braved" | "long_dark_worked" | "the_call_answered" => vec![
            mark("weather_hit"),
            build(
                MARKERS,
                "{explorer}'s markers out in the weather",
                "flag",
                SLOT_D,
                Some(3),
            ),
        ],
        "failing_patched" => vec![
            mark("failing_alone"),
            build(PARCEL, "Tools and spare parts", "parcel", SLOT_A, Some(2)),
        ],
        "failing_fixed" | "tool_found" | "tool_made" => vec![build(
            PARCEL,
            "Tools and spare parts",
            "parcel",
            SLOT_A,
            Some(2),
        )],
        "signal_logged" | "stars_watched" => vec![
            mark("signal"),
            build(LAMPS, "A lamp burning late", "lantern", SLOT_A, Some(2)),
        ],
        "explorer_rescued" | "explorer_guided" => vec![
            mark("rescued"),
            build(
                LAMPS,
                "A light left on for {explorer}",
                "lantern",
                SLOT_A,
                Some(3),
            ),
        ],
        "supply_shared" => vec![build(
            PARCEL,
            "{supply}, shared out",
            "parcel",
            SLOT_A,
            Some(2),
        )],
        "keeper_birthday_quiet" | "explorer_birthday_quiet" => vec![build(
            PARCEL,
            "A birthday present",
            "parcel",
            SLOT_A,
            Some(2),
        )],
        "message_sent" => vec![
            mark("message"),
            build(
                PARCEL,
                "A letter home, waiting to go",
                "parcel",
                SLOT_D,
                Some(2),
            ),
        ],
        "cleaned_together" => vec![build(
            WASHING,
            "Washing on the line",
            "bunting",
            SLOT_A,
            Some(2),
        )],
        "photo_taken" => vec![
            mark("photo"),
            build(
                WASHING,
                "Bunting for the photograph",
                "bunting",
                SLOT_A,
                Some(2),
            ),
        ],
        "breaking_point_talked" | "old_times_laughed" | "frost_walk" => vec![build(
            LAMPS,
            "A lamp left burning all night",
            "lantern",
            SLOT_A,
            Some(2),
        )],
        "breaking_point_apart" => vec![build(
            REST,
            "{explorer}'s camp at {place}",
            "tent",
            SLOT_C,
            Some(4),
        )],
        "garden_feast" | "garden_stored" => vec![build(
            PARCEL,
            "Baskets from the garden",
            "parcel",
            SLOT_C,
            Some(2),
        )],
        "spare_refused" => vec![
            mark("spare_refused"),
            build(
                PARCEL,
                "The old seal, patched again",
                "parcel",
                SLOT_A,
                Some(2),
            ),
        ],
        "clean_up_skipped" => vec![build(
            PARCEL,
            "Dust piling up by the door",
            "parcel",
            SLOT_A,
            Some(2),
        )],
        "frost_stayed_in" | "window_watched" => vec![build(
            LAMPS,
            "A lamp in the window",
            "lantern",
            SLOT_A,
            Some(1),
        )],
        "supper_skipped" => vec![
            mark("supper_missed"),
            build(
                LAMPS,
                "Lamps burning late over the work",
                "lantern",
                SLOT_A,
                Some(1),
            ),
        ],
        "edge_refused" => vec![mark("edge_refused")],
        "keeper_kept_going" => vec![mark("rest_refused")],
        "weather_caught_them" => vec![mark("weather_hit")],
        "failing_spread" => vec![mark("failing_alone")],
        "quarrel_keeper_won" | "quarrel_explorer_won" | "quarrel_simmered" => {
            vec![mark("quarrel")]
        }
        "message_put_off" => vec![
            mark("message_waiting"),
            build(PARCEL, "An unsent letter", "parcel", SLOT_A, Some(2)),
        ],
        "lesson_refused" => vec![mark("lesson_refused")],
        "photo_put_off" => vec![mark("photo_later")],
        "amends_too_soon" => vec![mark("amends_refused")],
        "garden_refused" => vec![mark("garden_refused")],
        "spare_went_without" => vec![mark("spare_refused")],
        "edge_forgotten" => vec![mark("edge_refused")],
        "lesson_forgotten" => vec![mark("lesson_refused")],
        "keeper_worn_out" => vec![mark("rest_refused")],
        "garden_forgotten" => vec![mark("garden_refused")],
        "message_never_sent" => vec![mark("message_waiting")],
        "photo_forgotten" => vec![mark("photo_later")],
        _ => Vec::new(),
    }
}

fn settled_by(storylet: &str) -> Vec<Condition> {
    use Condition::Unmarked;
    match storylet {
        "keeper_spare" => vec![Unmarked("spare"), Unmarked("spare_refused")],
        "explorer_trip" => vec![Unmarked("edge"), Unmarked("edge_refused")],
        "explorer_teach" => vec![Unmarked("taught"), Unmarked("lesson_refused")],
        "keeper_garden" => vec![Unmarked("garden_refused")],
        "keeper_rest" => vec![Unmarked("rested"), Unmarked("rest_refused")],
        "letter_home" => vec![Unmarked("message"), Unmarked("message_waiting")],
        "photograph" => vec![Unmarked("photo_later")],
        _ => Vec::new(),
    }
}

/// More of what can happen to the pair.
fn more() -> Vec<Spec> {
    use Condition::Unmarked;
    vec![
        spec(
            "keeper_garden",
            want(SLOT_B, vec![up("trust"), down("tension")]).requires(vec![Unmarked("garden")]),
            (
                "{keeper} wants a garden",
                "Something green, just for looking at?",
            ),
            vec![
                yes(
                    "plant",
                    "Plant one",
                    "{explorer} helps dig it in beside {place}.",
                    said(
                        "garden_planted",
                        "{keeper} and {explorer} planted a little garden",
                        "It's small, but it's ours.",
                        bond(1, -1),
                    )
                    .and([
                        mark("garden"),
                        build(GARDEN, "{keeper}'s garden", "garden", SLOT_C, None),
                    ])
                    .remembered("The first shoots are up!"),
                ),
                no(
                    "no_room",
                    "No room for it",
                    "Everything here has to earn its keep.",
                    said(
                        "garden_refused",
                        "{keeper} was told there was no room for a garden",
                        "Fine. Everything's grey, then.",
                        bond(-1, 1),
                    ),
                ),
            ],
            said(
                "garden_forgotten",
                "{keeper} gave up on the garden",
                "Oh well. Never mind.",
                bond(0, 1),
            ),
        ),
        spec(
            "photograph",
            want(SLOT_B, vec![up("trust")]).requires(vec![Unmarked("photo")]),
            (
                "{keeper} wants a photograph of the two of them",
                "Could we take a picture of us, here?",
            ),
            vec![
                yes(
                    "take",
                    "Take it",
                    "A minute, and a picture to keep.",
                    said(
                        "photo_taken",
                        "{keeper} and {explorer} had their picture taken",
                        "Say cheese!",
                        bond(2, 0),
                    )
                    .and([mark("photo")])
                    .remembered("I keep looking at that picture."),
                ),
                no(
                    "later",
                    "Later",
                    "There's work to do.",
                    said(
                        "photo_put_off",
                        "The picture was put off",
                        "Later, then.",
                        bond(-1, 0),
                    ),
                ),
            ],
            said(
                "photo_forgotten",
                "Nobody took the picture",
                "Oh well.",
                bond(0, 0),
            ),
        ),
        spec(
            "letter_home",
            want(SLOT_E, vec![down("tension")]),
            (
                "{explorer} wants to send a message home",
                "I'd like to send a message home.",
            ),
            vec![
                yes(
                    "send",
                    "Send it",
                    "{explorer} takes an evening to write it.",
                    said(
                        "message_sent",
                        "{explorer} sent a message home",
                        "They'll get it in a few days.",
                        bond(0, -2),
                    )
                    .remembered("I wonder if they've read it yet."),
                ),
                no(
                    "not_now",
                    "Not now",
                    "There isn't the time.",
                    said(
                        "message_put_off",
                        "{explorer}'s message home waited",
                        "Another day.",
                        bond(0, 1),
                    ),
                ),
            ],
            said(
                "message_never_sent",
                "{explorer}'s message never went",
                "Doesn't matter.",
                bond(0, 1),
            ),
        ),
        spec(
            "stargazing",
            incident(SLOT_E, vec![up("trust"), down("tension")]),
            ("A clear night", "Clear sky tonight. Come and look?"),
            vec![
                yes(
                    "look",
                    "Go and look",
                    "An hour on the roof, looking up.",
                    said(
                        "stars_watched",
                        "{keeper} and {explorer} watched the stars",
                        "That one's ours, I've decided.",
                        bond(1, -1),
                    )
                    .by_other(),
                ),
                yes(
                    "tired",
                    "Too tired",
                    "{keeper} stays in.",
                    said(
                        "stars_missed",
                        "{explorer} watched the stars alone",
                        "Your loss.",
                        bond(0, 1),
                    ),
                ),
            ],
            said(
                "clouds_came",
                "The clouds came in",
                "Clouded over. Typical.",
                bond(0, 0),
            ),
        ),
        spec(
            "trader",
            incident(SLOT_E, vec![up("trust")]),
            ("A trader came by", "A trader's come by with odds and ends!"),
            vec![
                yes(
                    "trade",
                    "Trade with them",
                    "The trader pitches a tent for a while.",
                    said(
                        "traded",
                        "{keeper} and {explorer} traded with a passing trader",
                        "Look what I got for an old spanner!",
                        bond(1, 0),
                    )
                    .and([
                        build(TENT, "The trader's tent", "tent", SLOT_A, Some(3)),
                        mark("traded"),
                    ]),
                ),
                yes(
                    "send_on",
                    "Send them on",
                    "Strangers are trouble.",
                    said(
                        "trader_sent_on",
                        "The trader was sent on",
                        "We don't need anything.",
                        bond(0, 1),
                    ),
                ),
            ],
            said(
                "trader_left",
                "The trader moved on",
                "Gone already.",
                bond(0, 0),
            ),
        ),
        spec(
            "lost_tool",
            incident(SLOT_B, vec![down("tension")]),
            (
                "{keeper} lost a tool",
                "I've lost my good wrench. Help me look?",
            ),
            vec![
                yes(
                    "look",
                    "Help look",
                    "{explorer} turns {home} upside down.",
                    said(
                        "tool_found",
                        "{explorer} found {keeper}'s wrench",
                        "In the pocket of my own coat. Of course.",
                        bond(1, -1),
                    ),
                ),
                yes(
                    "new_one",
                    "Make a new one",
                    "A day's work at the bench.",
                    said(
                        "tool_made",
                        "{keeper} made a new wrench",
                        "Better than the old one, if I say so.",
                        bond(0, 1),
                    ),
                ),
            ],
            said(
                "tool_still_lost",
                "{keeper}'s wrench stayed lost",
                "I'll manage without.",
                bond(0, 1),
            ),
        ),
        spec(
            "clean_up",
            day(SLOT_B, 20, 5),
            ("It's clean-up day", "Clean-up day! Who's helping?"),
            vec![
                yes(
                    "together",
                    "Both of us",
                    "An afternoon's scrubbing, side by side.",
                    said(
                        "cleaned_together",
                        "{keeper} and {explorer} cleaned up {home} together",
                        "Spotless!",
                        bond(1, 0),
                    ),
                ),
                yes(
                    "skip",
                    "Skip it",
                    "It can wait.",
                    said(
                        "clean_up_skipped",
                        "Clean-up day was skipped",
                        "It'll keep.",
                        bond(0, 1),
                    ),
                ),
            ],
            said(
                "clean_up_forgotten",
                "Clean-up day came and went",
                "Forgot it myself, nearly.",
                bond(0, 0),
            ),
        ),
        spec(
            "first_frost",
            day(SLOT_E, YEAR, SEASON_PERIODS * 3 + 1),
            ("The first frost", "First frost! Everything's sparkling."),
            vec![
                yes(
                    "walk",
                    "Walk out in it",
                    "A cold morning, the two of them.",
                    said(
                        "frost_walk",
                        "{keeper} and {explorer} walked out in the first frost",
                        "Our breath is smoking!",
                        bond(1, -1),
                    ),
                ),
                yes(
                    "stay_in",
                    "Stay in the warm",
                    "Tea and blankets.",
                    said(
                        "frost_stayed_in",
                        "{keeper} and {explorer} stayed in from the frost",
                        "Pass the blanket.",
                        bond(0, -1),
                    ),
                ),
            ],
            said(
                "frost_melted",
                "The first frost melted",
                "Gone by noon.",
                bond(0, 0),
            ),
        ),
    ]
}

// ---- The story tables, read from data -------------------------------------
//
// What each place's pair want, what befalls a place at any time, and the
// threads that follow from what earlier answers marked are data
// (`data/*.json`): the builder calls of this file as a call tree
// (`storylets::script`), each table parsed once.

/// Every spec of one table of the story.
fn table(tree: &'static OnceLock<Node>, json: &'static str) -> Vec<Spec> {
    tree.get_or_init(|| storylets::script::parse(json).expect("the story tables"))
        .items()
        .iter()
        .map(story_spec)
        .collect()
}

/// What each place's pair want.
fn wants() -> Vec<Spec> {
    static TREE: OnceLock<Node> = OnceLock::new();
    table(&TREE, include_str!("../data/wants.json"))
}

/// What can befall a place at any time.
fn incidents_at_any_time() -> Vec<Spec> {
    static TREE: OnceLock<Node> = OnceLock::new();
    table(&TREE, include_str!("../data/incidents.json"))
}

/// The threads that follow from what earlier answers marked, in every
/// place; each place's newcomer, one storylet made three ways, is built
/// here.
fn threads() -> Vec<Spec> {
    use Condition::{Absent, Is, Marked, Unmarked};
    static TREE: OnceLock<Node> = OnceLock::new();
    let mut threads = table(&TREE, include_str!("../data/threads.json"));
    let follow = |asker: EntityId, requires: Vec<Condition>| Shape {
        requires,
        ..following(asker)
    };
    // Each place's newcomer: one storylet, made three ways.
    for (seed, id, name, kind, role) in [
        (
            "mars-colony",
            "newcomer_mars",
            "Ines Duarte",
            "person",
            "engineer",
        ),
        (
            "1980s-town",
            "newcomer_town",
            "Ray Kowalski",
            "person",
            "late-night DJ",
        ),
        (
            "penguin-civilization",
            "newcomer_ice",
            "Tuk",
            "penguin",
            "young fisher",
        ),
    ] {
        let arrive = Effect::Arrive {
            entity: NEWCOMER,
            kind,
            components: vec![
                ("name", Value::from(name)),
                ("role", Value::from(role)),
                ("location", Value::Entity(SLOT_A)),
                ("newcomer", Value::from(true)),
            ],
        };
        threads.push(spec(
            id,
            follow(
                SLOT_E,
                vec![
                    Is(UNIVERSE, SEED, seed),
                    Marked("invited", 2),
                    Absent(NEWCOMER),
                    Unmarked("newcomer_decided"),
                ],
            ),
            (
                "Someone new wants to stay",
                "Someone's here, asking if they can stay.",
            ),
            vec![
                yes(
                    "welcome",
                    "Welcome them",
                    "Three of them now.",
                    said(
                        "newcomer_arrived",
                        "Someone new came to live with {keeper} and {explorer}",
                        "Welcome! Mind the step.",
                        bond(1, 0),
                    )
                    .and([arrive, mark("newcomer_decided")]),
                ),
                yes(
                    "not_yet",
                    "Not yet",
                    "Two is enough for now.",
                    said(
                        "newcomer_turned_away",
                        "The newcomer was turned away",
                        "Sorry. Not yet.",
                        bond(0, 1),
                    )
                    .and([mark("newcomer_decided")]),
                ),
            ],
            said(
                "newcomer_left",
                "The newcomer didn't wait",
                "They've gone on.",
                bond(0, 0),
            )
            .and([mark("newcomer_decided")]),
        ));
    }
    // Nobody snaps at anybody in a new player's first days, whatever they
    // answered: it waits until the place has been lived in a while.
    threads
        .into_iter()
        .map(|spec| {
            if spec.storylet.id == "snapped" {
                after_first_days(spec)
            } else {
                spec
            }
        })
        .collect()
}

/// A person, place or thing the tables name.
fn entity(node: &Node) -> EntityId {
    match node.name() {
        "BEACON" => BEACON,
        "EDGE_FLAG" => EDGE_FLAG,
        "FLOWERS" => FLOWERS,
        "GARDEN" => GARDEN,
        "LAMPS" => LAMPS,
        "MARKERS" => MARKERS,
        "PARCEL" => PARCEL,
        "RELATIONSHIP" => RELATIONSHIP,
        "REST" => REST,
        "SECOND" => SECOND,
        "SLOT_A" => SLOT_A,
        "SLOT_B" => SLOT_B,
        "SLOT_C" => SLOT_C,
        "SLOT_D" => SLOT_D,
        "SLOT_E" => SLOT_E,
        "TENT" => TENT,
        "WASHING" => WASHING,
        other => panic!("the story tables name no entity {other}"),
    }
}

/// A component the tables name.
fn key(node: &Node) -> &'static str {
    match node.name() {
        "RELATIONSHIP_TENSION" => RELATIONSHIP_TENSION,
        "RELATIONSHIP_TRUST" => RELATIONSHIP_TRUST,
        other => panic!("the story tables name no key {other}"),
    }
}

/// A thread that follows from something marked: asked by `asker` once
/// `requires` holds.
fn following(asker: EntityId) -> Shape {
    Shape {
        asker,
        want: false,
        requires: Vec::new(),
        lasts: 3,
        rests: 30,
        weight: 6,
        eases: Vec::new(),
        timely: true,
    }
}

fn args<'a>(node: &'a Node, name: &str) -> &'a [Node] {
    match node {
        Node::Call(call, args) if *call == name => args,
        other => panic!("{name}(..) wanted, not {other:?}"),
    }
}

fn story_spec(node: &Node) -> Spec {
    let [id, shape, words, answers, lapse] = args(node, "spec") else {
        panic!("spec(id, shape, words, answers, lapse): {node:?}");
    };
    let [told, line] = words.items() else {
        panic!("(told, line): {words:?}");
    };
    spec(
        id.text(),
        story_shape(shape),
        (told.text(), line.text()),
        answers.items().iter().map(story_answer).collect(),
        story_said(lapse),
    )
}

fn story_shape(node: &Node) -> Shape {
    match node {
        Node::Call("follow", args) => Shape {
            requires: conditions(&args[1]),
            ..following(entity(&args[0]))
        },
        Node::Call("want", args) => want(entity(&args[0]), eases(&args[1])),
        Node::Call("incident", args) => incident(entity(&args[0]), eases(&args[1])),
        Node::Call(".requires", args) => story_shape(&args[0]).requires(conditions(&args[1])),
        other => panic!("a shape wanted, not {other:?}"),
    }
}

fn eases(node: &Node) -> Vec<Ease> {
    node.items()
        .iter()
        .map(|node| match node {
            Node::Call("up", args) => up(args[0].text()),
            Node::Call("down", args) => down(args[0].text()),
            other => panic!("up(..) or down(..) wanted, not {other:?}"),
        })
        .collect()
}

fn story_answer(node: &Node) -> Answer {
    let Node::Call(kind @ ("yes" | "no"), args) = node else {
        panic!("yes(..) or no(..) wanted, not {node:?}");
    };
    let answer = if *kind == "yes" { yes } else { no };
    answer(
        args[0].text(),
        args[1].text(),
        args[2].text(),
        story_said(&args[3]),
    )
}

fn conditions(node: &Node) -> Vec<Condition> {
    node.items()
        .iter()
        .map(|node| match node {
            Node::Call("Marked", args) => Condition::Marked(args[0].text(), args[1].int() as u64),
            Node::Call("Unmarked", args) => Condition::Unmarked(args[0].text()),
            Node::Call("Finished", args) => Condition::Finished(args[0].text()),
            Node::Call("Condition::Unfinished", args) => Condition::Unfinished(args[0].text()),
            Node::Call("Condition::AtLeast", args) => {
                Condition::AtLeast(entity(&args[0]), key(&args[1]), args[2].int())
            }
            other => panic!("a condition wanted, not {other:?}"),
        })
        .collect()
}

fn story_said(node: &Node) -> Said {
    match node {
        Node::Call("said", args) => said(
            args[0].text(),
            args[1].text(),
            args[2].text(),
            effects(&args[3]),
        ),
        Node::Call(".and", args) => story_said(&args[0]).and(effects(&args[1])),
        Node::Call(".remembered", args) => story_said(&args[0]).remembered(args[1].text()),
        Node::Call(".by_other", args) => story_said(&args[0]).by_other(),
        other => panic!("said(..) wanted, not {other:?}"),
    }
}

fn effects(node: &Node) -> Vec<Effect> {
    match node {
        Node::List(items) => items.iter().flat_map(effects).collect(),
        Node::Call("bond", args) => bond(args[0].int(), args[1].int()),
        Node::Call("mark", args) => vec![mark(args[0].text())],
        Node::Call("Effect::Advance", args) => vec![Effect::Advance(args[0].text())],
        Node::Call("build", args) => vec![build(
            entity(&args[0]),
            args[1].text(),
            args[2].text(),
            entity(&args[3]),
            args[4].optional_int().map(|lasts| lasts as u64),
        )],
        other => panic!("effects wanted, not {other:?}"),
    }
}

#[cfg(test)]
mod tables_as_data {
    /// Every story table's data parses, every call in it is one the Pack
    /// knows, and each storylet is there once.
    #[test]
    fn the_story_tables_are_whole() {
        for (table, json, specs) in [
            ("wants", include_str!("../data/wants.json"), super::wants()),
            (
                "incidents",
                include_str!("../data/incidents.json"),
                super::incidents_at_any_time(),
            ),
            (
                "threads",
                include_str!("../data/threads.json"),
                super::threads(),
            ),
        ] {
            let tree = storylets::script::parse(json).unwrap();
            assert!(tree.items().len() <= specs.len(), "{table}");
            let ids = specs
                .iter()
                .map(|spec| spec.storylet.id)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(ids.len(), specs.len(), "{table}: no storylet twice");
            for spec in &specs {
                assert!(spec.storylet.choices.len() >= 2, "{}", spec.storylet.id);
            }
        }
    }
}

/// How a fixture is drawn here: the second home and the beacon take the
/// place's own shapes.
pub(crate) fn fixture_shape(world: &World, shape: &str) -> world_projection::MarkShape {
    use world_projection::MarkShape;
    match (shape, seed_id(world)) {
        ("second", "mars-colony") => MarkShape::Dome,
        ("second", "1980s-town") => MarkShape::Shop,
        ("second", _) => MarkShape::Bridge,
        ("beacon", "penguin-civilization") => MarkShape::Lantern,
        ("beacon", _) => MarkShape::Tower,
        ("stall", _) => MarkShape::Stall,
        ("bunting", _) => MarkShape::Bunting,
        ("pier", _) => MarkShape::Pier,
        ("garden", _) => MarkShape::Garden,
        ("flag", _) => MarkShape::Flag,
        ("lantern", _) => MarkShape::Lantern,
        ("tent", _) => MarkShape::Tent,
        ("bench", _) => MarkShape::Bench,
        ("sprouts", _) => MarkShape::Sprouts,
        ("tree", _) => MarkShape::Tree,
        ("boat", _) => MarkShape::Boat,
        ("well", _) => MarkShape::Well,
        ("swing", _) => MarkShape::Swing,
        ("fountain", _) => MarkShape::Fountain,
        ("signpost", _) => MarkShape::Signpost,
        ("birdhouse", _) => MarkShape::Birdhouse,
        ("planter", _) => MarkShape::Planter,
        ("statue", _) => MarkShape::Statue,
        ("postbox", _) => MarkShape::Postbox,
        ("stone", _) => MarkShape::Statue,
        ("house", _) => MarkShape::House,
        ("shop", _) => MarkShape::Shop,
        ("tower", _) => MarkShape::Tower,
        ("dome", _) => MarkShape::Dome,
        _ => MarkShape::Parcel,
    }
}

/// What answers have put on the scene.
pub(crate) fn fixtures(world: &World) -> Vec<world_projection::CanvasItem> {
    storylets::fixtures(world.state())
        .into_iter()
        .map(|fixture| {
            let text = |key: &str| match fixture.component(key) {
                Some(Value::Text(value)) => value.clone(),
                _ => String::new(),
            };
            let at = match fixture.component("at") {
                Some(Value::Entity(at)) => Some(world_projection::SelectionId::Entity(*at)),
                _ => None,
            };
            world_projection::CanvasItem {
                id: world_projection::SelectionId::Entity(fixture.id),
                kind: world_projection::CanvasItemKind::Object,
                label: fill(world, &text("name")),
                detail: String::new(),
                x: 0.5,
                y: 0.5,
                changes: Vec::new(),
                shape: Some(fixture_shape(world, &text("shape"))),
                at,
                look: None,
                drawing: if fixture.component(lives::generations::MEMORIAL_OF).is_some() {
                    crate::drawings::memorial_drawing(world, &text("shape"))
                } else {
                    crate::drawings::fixture_drawing(world, &text("shape"))
                },
                stance: None,
                standing: None,
                mood: None,
                spot: match fixture.component(hands::SPOT) {
                    Some(Value::Integer(spot)) => Some((*spot).clamp(0, 100) as f32 / 100.0),
                    _ => None,
                },
                px: None,
                home: None,
                day: Vec::new(),
                built: match fixture.component("built") {
                    Some(Value::Integer(day)) => Some((*day).max(0) as u32),
                    _ => None,
                },
                ..Default::default()
            }
        })
        .collect()
}

/// Whether a storylet follows from an earlier answer: a second or third
/// act, or what a finished goal opens.
#[cfg(test)]
pub(crate) fn follows_from_an_answer(id: &str) -> bool {
    static IDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    IDS.get_or_init(|| threads().iter().map(|spec| spec.storylet.id).collect())
        .contains(&id)
}

/// Whether a storylet is one the calendar brings round again by design.
#[cfg(test)]
pub(crate) fn from_the_calendar(id: &str) -> bool {
    find(id).is_some_and(|spec| {
        spec.storylet
            .requires
            .iter()
            .any(|condition| matches!(condition, Condition::Every { .. }))
    })
}

/// The weather over each place, from how the World stands: dust storms on
/// Mars and blizzards on the ice while the weather or the long dark is
/// upon them, rain and snow on Maple Street by the season, as the period's
/// own number falls.
pub(crate) fn weather(world: &World) -> world_projection::Weather {
    use world_projection::Weather;
    let deck = deck_ref();
    let open = storylets::open(world.state(), deck);
    let rough = open
        .iter()
        .any(|storylet| matches!(storylet.id, "weather" | "long_dark"));
    let period = world.world_time() / crate::BACKGROUND_PERIOD;
    let roll = storylets::mix(&[period, 23]) % 10;
    let season = season(world);
    // A new player's first day is a fair one.
    if !rough && crate::arrival::arrived(world.state()).is_some_and(|first| period <= first) {
        return Weather::Clear;
    }
    match seed_id(world) {
        "mars-colony" => match (rough, roll) {
            (true, _) | (false, 0) => Weather::Dust,
            (false, 1) if season == 3 => Weather::Cloudy,
            _ => Weather::Clear,
        },
        "penguin-civilization" => match (rough, season, roll) {
            (true, _, _) => Weather::Storm,
            (false, 3, 0..=5) | (false, 2, 0..=2) | (false, _, 0) => Weather::Snow,
            (false, _, 1..=2) => Weather::Fog,
            (false, _, 3) => Weather::Cloudy,
            _ => Weather::Clear,
        },
        "1980s-town" => match (rough, season, roll) {
            (true, _, _) => Weather::Storm,
            (false, 3, 0..=3) => Weather::Snow,
            (false, 2 | 0, 0..=2) | (false, 1, 0) => Weather::Rain,
            (false, _, 4) | (false, 0..=2, 3) => Weather::Cloudy,
            (false, 2, 5) => Weather::Fog,
            _ => Weather::Clear,
        },
        _ => Weather::Clear,
    }
}

#[cfg(test)]
mod tests {
    /// Every question the storyteller asks, in every place, has at least
    /// two answers that change something: none is a question with only one
    /// real answer.
    #[test]
    fn every_question_has_two_answers_that_change_something() {
        let deck = super::deck();
        let thin = deck
            .storylets
            .iter()
            .filter(|storylet| {
                storylet
                    .choices
                    .iter()
                    .filter(|choice| !choice.outcome.effects.is_empty())
                    .count()
                    < 2
            })
            .map(|storylet| storylet.id)
            .collect::<Vec<_>>();
        assert!(thin.is_empty(), "questions with one real answer: {thin:?}");
    }

    /// Each place asks for its own works, in its own order, at its own
    /// pace, and a player who leaves them to the people still sees them
    /// built.
    #[test]
    fn every_place_has_a_ladder_of_its_own() {
        use crate::places::Place;
        let ladders = [Place::Ares, Place::Maple, Place::Ice].map(|place| {
            (
                place,
                crate::works::works(place)
                    .iter()
                    .map(|work| work.label)
                    .collect::<Vec<_>>(),
                crate::works::rests(place),
            )
        });
        for (at, (place, works, rests)) in ladders.iter().enumerate() {
            assert!(works.len() >= 30, "{place:?}: {} works", works.len());
            for (other, other_works, other_rests) in &ladders[at + 1..] {
                assert_ne!(works.len(), other_works.len(), "{place:?} and {other:?}");
                assert_ne!(rests, other_rests, "{place:?} and {other:?}");
                assert!(
                    works.iter().all(|work| !other_works.contains(work)),
                    "{place:?} and {other:?} share a work"
                );
            }
        }
        let deck = super::deck();
        for (place, _, _) in &ladders {
            for rung in super::ladder(*place) {
                let spec = super::find(&format!("work_{}", rung.id)).unwrap();
                let building = |choice: &storylets::Choice| {
                    choice
                        .outcome
                        .effects
                        .iter()
                        .any(|effect| matches!(effect, storylets::Effect::Advance(goal) if *goal == rung.id))
                };
                assert!(spec.storylet.choices.iter().all(building), "{}", rung.id);
                assert!(
                    spec.storylet
                        .lapse
                        .effects
                        .iter()
                        .any(|effect| matches!(effect, storylets::Effect::Advance(_))),
                    "{} goes nowhere when nobody answers",
                    rung.id
                );
                assert!(deck.goals.iter().any(|goal| goal.id == rung.id));
            }
        }
    }
}
