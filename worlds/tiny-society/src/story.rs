//! The harbour's storyteller: what its people want, what the sea and the
//! calendar bring, and how each chapter of the town's life ends.
//!
//! The mechanics are the shared `storylets` System. Everything here is the
//! harbour's own: who asks what, what each answer costs and changes, and
//! the words it is told in. What the storyteller does is recorded as
//! ordinary Events, so replaying the town never runs it again.

use crate::model::{CONDITION, MAINLAND_MARKET, OPERATING_STATUS};
use crate::{BAKERY, EMMA, EVAN, HARBOR, JONAS, JONAS_BOAT, LEO, MARA, MIA, NOAH, SOFIA};
use society_basic::{CASH, JOB};
use std::sync::OnceLock;
use storylets::script::Node;
use storylets::{build, mark};
use storylets::{
    text_hash, Choice, Condition, Deck, Ease, Effect, Goal, Outcome, Pinned, Reading, Storylet,
};
use world_core::{ActionRegistry, EntityId, Event, EventId, Value, World, WorldError};

/// The entity the storyteller keeps its notes on.
pub(crate) const STORY: EntityId = EntityId::new(401);
/// How the harbour feels, from -5 to 5.
pub(crate) const MOOD: &str = "story.mood";
/// Days in a season; four make the harbour's year.
pub(crate) const SEASON_DAYS: u64 = 30;
/// Days in a chapter of the harbour's life.
pub(crate) const CHAPTER_DAYS: u64 = 24;
/// The one way to wait.
pub(crate) const WAIT_COMMAND: &str = "tiny-society.let-day-pass";
const STORY_COMMAND: &str = "tiny-society.story.";
const YEAR: u64 = SEASON_DAYS * 4;

/// What happens when a storylet is answered one way, or runs out: the
/// Event, how the harbour tells it, what the asker says, what it changes,
/// and what the asker still says about it in the days after.
struct Said {
    event: &'static str,
    told: &'static str,
    line: &'static str,
    effects: Vec<Effect>,
    remembered: Option<&'static str>,
    /// What this adds to the chapter's ending, when it closes.
    chapter: Option<&'static str>,
    /// What the chapter is called, when this is how its climax went.
    title: Option<&'static str>,
    /// The Event whose leavings this leaves, when it is another answer's
    /// outcome reached another way.
    behind: Option<&'static str>,
}

struct Answer {
    id: &'static str,
    title: &'static str,
    detail: &'static str,
    requires: Vec<Condition>,
    refuses: bool,
    said: Said,
}

/// A storylet with its words.
struct Spec {
    storylet: Storylet,
    told: &'static str,
    line: &'static str,
    answers: Vec<Answer>,
    lapse: Said,
    /// For a want: how it goes when someone else takes it up after the
    /// player let it down.
    taken_up: Option<Said>,
}

/// Where the name of whoever took a want up goes in how it is told.
const HELPER: &str = "{helper}";

/// Who takes up a want the player let down: one of the harbour's own,
/// never the asker, the same one each time for the same want.
fn helper_for(id: &str, asker: EntityId) -> EntityId {
    let others = crate::talk::RESIDENTS
        .into_iter()
        .filter(|who| *who != asker)
        .collect::<Vec<_>>();
    others[(text_hash(id) as usize) % others.len()]
}

/// A want let down, seen to by someone else: what its granting answer does
/// without the money, told as their doing.
fn taken_up_said(answers: &[Answer]) -> Option<Said> {
    let without_money = |said: &Said| {
        said.effects
            .iter()
            .filter(|effect| !matches!(effect, Effect::Add { key, .. } if *key == CASH))
            .cloned()
            .collect::<Vec<_>>()
    };
    let granting = answers
        .iter()
        .filter(|answer| !answer.refuses)
        .find(|answer| !without_money(&answer.said).is_empty())?;
    let said = &granting.said;
    let told = match said.told.split_once(' ') {
        Some(("The" | "A" | "An" | "Work" | "They" | "Everyone" | "Nobody", _)) => {
            lowered(said.told)
        }
        _ => said.told.to_string(),
    };
    Some(Said {
        event: leak(format!("{}_taken_up", said.event)),
        told: leak(format!("{HELPER} took it on, and {told}")),
        line: said.line,
        // Seeing to it themselves, the harbour grows readier to get on
        // with things of its own.
        effects: without_money(said)
            .into_iter()
            .chain([initiative(TAKEN_UP_READINESS)])
            .collect(),
        remembered: said.remembered,
        chapter: None,
        title: None,
        behind: Some(said.behind.unwrap_or(said.event)),
    })
}

fn pay(from: EntityId, to: EntityId, amount: i64) -> [Effect; 2] {
    [
        Effect::Add {
            entity: from,
            key: CASH,
            by: -amount,
            min: 0,
            max: i64::MAX,
        },
        Effect::Add {
            entity: to,
            key: CASH,
            by: amount,
            min: 0,
            max: i64::MAX,
        },
    ]
}

fn spend(from: EntityId, amount: i64) -> [Effect; 2] {
    pay(from, MAINLAND_MARKET, amount)
}

fn earn(to: EntityId, amount: i64) -> [Effect; 2] {
    pay(MAINLAND_MARKET, to, amount)
}

fn mood(by: i64) -> Effect {
    Effect::Add {
        entity: STORY,
        key: MOOD,
        by,
        min: -5,
        max: 5,
    }
}

fn has(who: EntityId, amount: i64) -> Condition {
    Condition::AtLeast(who, CASH, amount)
}

fn said(
    event: &'static str,
    told: &'static str,
    line: &'static str,
    effects: impl IntoIterator<Item = Effect>,
) -> Said {
    Said {
        event,
        told,
        line,
        effects: effects.into_iter().collect(),
        remembered: None,
        chapter: None,
        title: None,
        behind: None,
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
}

fn yes(
    id: &'static str,
    title: &'static str,
    detail: &'static str,
    requires: Vec<Condition>,
    said: Said,
) -> Answer {
    Answer {
        id,
        title,
        detail,
        requires,
        refuses: false,
        said,
    }
}

fn no(id: &'static str, title: &'static str, detail: &'static str, said: Said) -> Answer {
    Answer {
        id,
        title,
        detail,
        requires: Vec::new(),
        refuses: true,
        said,
    }
}

fn other(id: &'static str, title: &'static str, detail: &'static str, said: Said) -> Answer {
    Answer {
        refuses: false,
        ..no(id, title, detail, said)
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

fn want(asker: EntityId) -> Shape {
    Shape {
        asker,
        want: true,
        requires: Vec::new(),
        lasts: 3,
        rests: 16,
        weight: 3,
        eases: vec![down("money"), up("spirits")],
        timely: false,
    }
}

fn incident(asker: EntityId, eases: Vec<Ease>) -> Shape {
    Shape {
        asker,
        want: false,
        requires: Vec::new(),
        lasts: 2,
        rests: 16,
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

/// How everyone doing it together is offered, in turn.
const TOGETHER: [(&str, &str); 4] = [
    (
        "Everyone helps",
        "No money spent. It costs everyone an evening.",
    ),
    (
        "Ask the neighbours",
        "Nothing spent, but it's a long day for everyone.",
    ),
    (
        "All pitch in",
        "No money changes hands. Everyone's tired after.",
    ),
    (
        "Make do together",
        "Nothing spent. Everyone gives up a day to it.",
    ),
];

/// A way to get what a paid answer gets without the money: the whole
/// harbour does it together, and is worn out after. Money it would have
/// moved stays where it is; what it needs besides money it still needs.
fn together(paid: &Answer, turn: usize) -> Answer {
    let (title, detail) = TOGETHER[turn % TOGETHER.len()];
    let effects = paid
        .said
        .effects
        .iter()
        .filter(|effect| !matches!(effect, Effect::Add { key, .. } if *key == CASH))
        .cloned()
        .chain(crate::talk::RESIDENTS.map(|who| Effect::Add {
            entity: who,
            key: lives::Need::Rest.key(),
            by: 8,
            min: 0,
            max: 100,
        }))
        .collect();
    Answer {
        id: "together",
        title,
        detail,
        requires: paid
            .requires
            .iter()
            .filter(|condition| !matches!(condition, Condition::AtLeast(_, key, _) if *key == CASH))
            .cloned()
            .collect(),
        refuses: false,
        said: Said {
            event: leak(format!("{}_together", paid.said.event)),
            told: leak(format!(
                "Everyone pitched in, and {}",
                match paid.said.told.split_once(' ') {
                    Some(("The" | "A" | "An" | "Work", _)) => lowered(paid.said.told),
                    _ => paid.said.told.to_string(),
                }
            )),
            line: paid.said.line,
            effects,
            remembered: paid.said.remembered,
            chapter: paid.said.chapter,
            title: paid.said.title,
            behind: Some(paid.said.behind.unwrap_or(paid.said.event)),
        },
    }
}

fn spec(
    id: &'static str,
    shape: Shape,
    (told, line): (&'static str, &'static str),
    mut answers: Vec<Answer>,
    lapse: Said,
) -> Spec {
    // Turning down a want, or letting it lapse, is remembered: a harbour
    // let down often enough starts things of its own.
    let want = shape.want;
    let outcome = |said: &Said, let_down: i64| Outcome {
        event: said.event,
        effects: said
            .effects
            .iter()
            .cloned()
            .chain(leaves_behind(said.behind.unwrap_or(said.event)))
            .chain((let_down > 0).then(|| initiative(let_down)))
            .collect(),
    };
    let refused = |refuses: bool| i64::from(want && refuses);
    let unanswered = i64::from(want);
    // Every question has at least two answers that change something and
    // cost nothing, so being short of money never leaves only one.
    let free_and_changing =
        answers
            .iter()
            .filter(|answer| {
                !answer.requires.iter().any(
                    |condition| matches!(condition, Condition::AtLeast(_, key, _) if *key == CASH),
                ) && !outcome(&answer.said, refused(answer.refuses))
                    .effects
                    .is_empty()
            })
            .count();
    if free_and_changing < 2 {
        let paid = answers.iter().position(|answer| {
            answer
                .requires
                .iter()
                .any(|condition| matches!(condition, Condition::AtLeast(_, key, _) if *key == CASH))
        });
        if let Some(paid) = paid {
            let turn = text_hash(id) as usize;
            let together = together(&answers[paid], turn);
            // Offered straight after what costs money, the same thing got
            // another way.
            answers.insert(paid + 1, together);
        }
    }
    let mut requires = shape.requires;
    requires.extend(settled_by(id));
    // Someone else sees to a want the player let down, and to a part of a
    // work the player let go by.
    let builds = answers.iter().any(|answer| {
        answer
            .said
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::Advance(_)))
    });
    let taken_up = (want || builds).then(|| taken_up_said(&answers)).flatten();
    let helper = helper_for(id, shape.asker);
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
                    outcome: outcome(&answer.said, refused(answer.refuses)),
                    refuses: answer.refuses,
                })
                .collect(),
            lapse: outcome(&lapse, unanswered),
            lasts: shape.lasts,
            rests: shape.rests,
            weight: shape.weight,
            eases: shape.eases,
            timely: shape.timely,
            taken_up: taken_up.as_ref().map(|said| storylets::TakenUp {
                by: helper,
                outcome: outcome(said, 0),
                // A part of one of the harbour's works is taken up only
                // while the player has lately been here: making things or
                // answering people sets the harbour building.
                requires: if said
                    .effects
                    .iter()
                    .any(|effect| matches!(effect, Effect::Advance(_)))
                {
                    vec![Condition::MarkedWithin(LENT_MARK, LENT_LATELY)]
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

/// An incident (a storm coming, a quarrel, a loss) waits until a new
/// player's first days have passed.
fn after_first_days(mut spec: Spec) -> Spec {
    spec.storylet.requires.push(Condition::Since(
        HARBOR,
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
    let mut days = vec![
        spec(
            "market_day",
            day(SOFIA, 10, 3),
            ("It's market day", "Market day! What shall we do?"),
            vec![
                other(
                    "sell",
                    "Sell jam",
                    "Sofia makes 25.",
                    said(
                        "market_jam_sold",
                        "Sofia sold jam on market day",
                        "Sold out by noon!",
                        earn(SOFIA, 25),
                    ),
                ),
                yes(
                    "treat",
                    "Pies all round",
                    "Noah pays 20 for pies all round.",
                    vec![has(NOAH, 20)],
                    said(
                        "market_lunch",
                        "The harbour treated itself on market day",
                        "Hot pies all round!",
                        spend(NOAH, 20).into_iter().chain([mood(1)]),
                    ),
                ),
            ],
            said(
                "market_came_and_went",
                "Market day came and went",
                "Quiet market today.",
                [],
            ),
        ),
        spec(
            "regatta",
            day(NOAH, YEAR, SEASON_DAYS + 5),
            ("It's regatta day", "Regatta day! Who's racing?"),
            vec![
                yes(
                    "race",
                    "Race Sea Finch",
                    "Jonas takes the harbour's 40 prize if he wins, and he usually does.",
                    vec![Condition::Is(JONAS_BOAT, CONDITION, "sound"), has(NOAH, 40)],
                    said(
                        "regatta_won",
                        "Sea Finch won the regatta",
                        "First past the buoy!",
                        pay(NOAH, JONAS, 40).into_iter().chain([mood(2)]),
                    )
                    .remembered("Did you see Sea Finch fly?"),
                ),
                other(
                    "dinghies",
                    "Race the dinghies instead",
                    "Nothing spent. Evan's dinghy against the school's raft.",
                    said(
                        "regatta_dinghies",
                        "The harbour raced its dinghies at the regatta",
                        "Evan's dinghy won by a nose!",
                        [
                            mood(1),
                            Effect::Add {
                                entity: MIA,
                                key: lives::Need::Company.key(),
                                by: -15,
                                min: 0,
                                max: 100,
                            },
                        ],
                    ),
                ),
                other(
                    "watch",
                    "Just watch",
                    "Nothing ventured.",
                    said(
                        "regatta_watched",
                        "The harbour watched the regatta",
                        "What a race!",
                        [mood(1)],
                    ),
                ),
            ],
            said(
                "regatta_sailed",
                "The regatta sailed without the harbour",
                "Maybe next year.",
                [],
            ),
        ),
        spec(
            "harvest_supper",
            day(LEO, YEAR, SEASON_DAYS * 2 + 5),
            (
                "It's harvest supper night",
                "Harvest supper at the pub. Who's paying?",
            ),
            vec![
                yes(
                    "feast",
                    "A proper feast",
                    "Leo spends 45 on a proper spread.",
                    vec![has(LEO, 45)],
                    said(
                        "harvest_feast",
                        "The harbour sat down to a harvest feast",
                        "Eat up, there's plenty!",
                        spend(LEO, 45).into_iter().chain([mood(2)]),
                    )
                    .remembered("I'm still full from the harvest supper."),
                ),
                other(
                    "potluck",
                    "Everyone brings a dish",
                    "Nothing spent. Mara's pie, Jonas's fish, Leo's ale.",
                    said(
                        "harvest_potluck",
                        "The harbour shared a potluck supper",
                        "Mara's pie, Jonas's fish, my ale!",
                        [mood(1)],
                    ),
                ),
            ],
            said(
                "harvest_skipped",
                "Harvest supper was skipped",
                "No supper this year.",
                [mood(-1)],
            ),
        ),
        spec(
            "winter_coal",
            day(EMMA, YEAR, SEASON_DAYS * 3 + 2),
            ("The school needs coal", "It's freezing in the classroom."),
            vec![
                yes(
                    "buy",
                    "Buy coal",
                    "Noah pays 50 for a winter's coal.",
                    vec![has(NOAH, 50)],
                    said(
                        "school_stove_lit",
                        "The school stove was lit",
                        "Warm hands, sharp minds.",
                        spend(NOAH, 50).into_iter().chain([mood(1)]),
                    ),
                ),
                other(
                    "coats",
                    "Teach in coats",
                    "Nothing spent. Cold fingers.",
                    said(
                        "school_in_coats",
                        "The school taught in coats",
                        "Keep your mittens on.",
                        [mood(-1)],
                    ),
                ),
            ],
            said(
                "school_went_cold",
                "The school went cold",
                "Too cold to hold a pen.",
                [mood(-1)],
            ),
        ),
    ];
    // Everyone's birthday is their own, in their own words, and how the
    // harbour marks it is a real choice: a party out of the fund, a present
    // from it, a cake from a friend, or a card from everyone.
    for (id, who, at, [asks, party, card, forgotten], [gift, cake]) in [
        (
            "birthday_jonas",
            JONAS,
            6,
            [
                "Birthday today. Don't make a fuss.",
                "Cake and a pint. Can't argue with that.",
                "A card! Even Noah signed.",
                "Birthday. Nobody noticed.",
            ],
            [
                "A new knife for the gutting? You shouldn't have.",
                "Mara's cake, shaped like a fish. I'm touched.",
            ],
        ),
        (
            "birthday_mara",
            MARA,
            11,
            [
                "Baked my own birthday cake. Again.",
                "A party, and I didn't have to bake!",
                "Everyone signed. There's flour on it already.",
                "Nobody remembered. Typical.",
            ],
            [
                "A proper apron, with pockets. From everyone?",
                "Somebody else baked for me. Sofia, you angel.",
            ],
        ),
        (
            "birthday_leo",
            LEO,
            16,
            [
                "It's my birthday. Drinks are on me. Just the one.",
                "Best birthday the Anchor's seen!",
                "I'll pin your card over the bar.",
                "Birthday, and I served the drinks myself.",
            ],
            [
                "A pipe, from the whole harbour. I'm speechless.",
                "Mara's walnut cake. She remembered.",
            ],
        ),
        (
            "birthday_emma",
            EMMA,
            20,
            [
                "Birthday today. The children made me a crown.",
                "Such a lovely party!",
                "The children drew on the card too.",
                "Not even the children remembered.",
            ],
            [
                "A fountain pen! My old one's all blots.",
                "A cake with the alphabet in icing. Mara, honestly.",
            ],
        ),
        (
            "birthday_mia",
            MIA,
            24,
            [
                "It's my birthday! Guess how old!",
                "Best birthday ever!",
                "A card from everyone! Even Emma!",
                "Nobody remembered. Nobody.",
            ],
            [
                "A real pocketknife! Evan's going to show me how.",
                "A cake with a candle for every year. I blew them all out!",
            ],
        ),
        (
            "birthday_noah",
            NOAH,
            29,
            [
                "My birthday, if anyone's counting.",
                "A splendid do. Thank you all.",
                "A card. Very kind. Very kind.",
                "My birthday came and went.",
            ],
            [
                "A new logbook. Leather. From the fund? Goodness.",
                "Seed cake, my favourite. How did Mara know?",
            ],
        ),
        (
            "birthday_evan",
            EVAN,
            34,
            [
                "Another year older. Still got all my fingers.",
                "Now that was a party.",
                "A card! I'll make a frame for it.",
                "Forgot my own birthday, and so did everyone.",
            ],
            [
                "A new plane, sharp as anything. Thank you.",
                "Mara made me a cake shaped like a boat. It floated, nearly.",
            ],
        ),
        (
            "birthday_sofia",
            SOFIA,
            38,
            [
                "It's my birthday. I want nothing. Well, maybe cake.",
                "A party! I didn't expect that.",
                "You all signed it. Thank you.",
                "Nobody remembered. I'm fine. Really.",
            ],
            [
                "Silk ribbon for the stall! Everyone chipped in?",
                "Mara's lemon cake. I said I wanted nothing. I lied.",
            ],
        ),
    ] {
        let baker = if who == MARA { SOFIA } else { MARA };
        let fond = |of: EntityId, to: EntityId, by: i64| Effect::Add {
            entity: of,
            key: leak(format!("lives.opinion.{to}")),
            by,
            min: -100,
            max: 100,
        };
        let need = |need: lives::Need, by: i64| Effect::Add {
            entity: who,
            key: need.key(),
            by,
            min: 0,
            max: 100,
        };
        let regard = |by: i64| Effect::Add {
            entity: who,
            key: lives::REGARD,
            by,
            min: -100,
            max: 100,
        };
        days.push(spec(
            id,
            day(who, YEAR, at * 3),
            ("{name}'s birthday", asks),
            vec![
                yes(
                    "party",
                    "A party at the pub",
                    "The harbour fund pays Leo 20 for cake and a round. Everyone comes.",
                    vec![has(HARBOR, 20)],
                    said(
                        "birthday_party",
                        "{name} had a birthday party",
                        party,
                        pay(HARBOR, LEO, 20)
                            .into_iter()
                            .chain([mood(1), need(lives::Need::Company, -30)])
                            .chain(
                                crate::talk::RESIDENTS
                                    .into_iter()
                                    .filter(|other| *other != who)
                                    .map(|other| fond(other, who, 3)),
                            ),
                    )
                    .remembered("Thanks again for the party."),
                ),
                yes(
                    "gift",
                    "A present from the fund",
                    "The harbour fund pays 15 for something they've wanted.",
                    vec![has(HARBOR, 15)],
                    said(
                        "birthday_gift",
                        "{name} unwrapped a present from the harbour",
                        gift,
                        spend(HARBOR, 15).into_iter().chain([regard(8)]),
                    )
                    .remembered("I use your present every day."),
                ),
                other(
                    "cake",
                    if who == MARA {
                        "A cake from Sofia"
                    } else {
                        "A cake from Mara"
                    },
                    "Nothing spent. A long evening at the oven, and a friend made.",
                    said(
                        "birthday_cake",
                        if who == MARA {
                            "Sofia baked {name} a birthday cake"
                        } else {
                            "Mara baked {name} a birthday cake"
                        },
                        cake,
                        [
                            fond(who, baker, 8),
                            fond(baker, who, 4),
                            Effect::Add {
                                entity: baker,
                                key: lives::Need::Rest.key(),
                                by: 10,
                                min: 0,
                                max: 100,
                            },
                        ],
                    ),
                ),
                other(
                    "card",
                    "A card from everyone",
                    "Nothing spent. A kind thought, and it's noticed.",
                    said(
                        "birthday_card",
                        "{name} got a card from everyone",
                        card,
                        [regard(4), need(lives::Need::Company, -10)],
                    ),
                ),
            ],
            said(
                "birthday_forgotten",
                "{name}'s birthday was forgotten",
                forgotten,
                [mood(-1), regard(-4)],
            ),
        ));
    }
    days
}

/// What the harbour builds towards after the pier and the lamp, one at a
/// time, in order: each asked for by the person who cares most, paid for
/// by the harbour fund a part at a time. There is always one in hand.
pub(crate) struct Work {
    pub id: &'static str,
    pub label: &'static str,
    pub shape: world_projection::MarkShape,
    champion: EntityId,
    parts: i64,
    cost: i64,
    told: &'static str,
    line: &'static str,
    /// What comes of it the second time round, once every work is built:
    /// what it is called, what is asked, and what its champion says.
    again: (&'static str, &'static str, &'static str),
}

pub(crate) const WORKS: &[Work] = {
    use world_projection::MarkShape as M;
    &[
        Work {
            id: "bandstand",
            label: "A bandstand on the square",
            shape: M::Tent,
            champion: SOFIA,
            parts: 2,
            cost: 50,
            told: "Sofia wants a bandstand for summer nights",
            line: "Music on the square every Saturday. Picture it.",
            again: (
                "The bandstand painted red and gold",
                "Sofia wants the bandstand painted red and gold",
                "Red and gold, like a proper seaside bandstand.",
            ),
        },
        Work {
            id: "sea_wall",
            label: "A sea wall at the point",
            shape: M::Pier,
            champion: JONAS,
            parts: 3,
            cost: 50,
            told: "Jonas wants a sea wall before the storms",
            line: "One more winter like the last and the point's gone.",
            again: (
                "Steps down the sea wall",
                "Jonas wants steps down the sea wall",
                "Steps down to the rocks, so the children stop climbing.",
            ),
        },
        Work {
            id: "school_garden",
            label: "A garden for the school",
            shape: M::Garden,
            champion: EMMA,
            parts: 2,
            cost: 30,
            told: "Emma wants a garden for the school",
            line: "The children should see things grow.",
            again: (
                "A shed for the school garden",
                "Emma wants a shed for the school garden",
                "The spades live in my classroom. It has to stop.",
            ),
        },
        Work {
            id: "harbour_clock",
            label: "The harbour clock going again",
            shape: M::Tower,
            champion: NOAH,
            parts: 2,
            cost: 60,
            told: "Noah wants the harbour clock going again",
            line: "Stopped at ten past four since before I was born.",
            again: (
                "A chime for the harbour clock",
                "Noah wants the harbour clock to chime the hours",
                "A clock that strikes the hour. Like a real town.",
            ),
        },
        Work {
            id: "boathouse",
            label: "A boathouse for the winter",
            shape: M::House,
            champion: EVAN,
            parts: 3,
            cost: 45,
            told: "Evan wants a boathouse for the winter",
            line: "Somewhere dry to mend the boats.",
            again: (
                "A slipway down from the boathouse",
                "Evan wants a slipway down from the boathouse",
                "No more dragging hulls over the shingle.",
            ),
        },
        Work {
            id: "fountain",
            label: "A fountain on the square",
            shape: M::Fountain,
            champion: MIA,
            parts: 2,
            cost: 50,
            told: "Mia wants a fountain on the square",
            line: "A fountain! With a fish that spits!",
            again: (
                "A stone rim round the fountain",
                "Mia wants a stone rim round the fountain to sit on",
                "Somewhere to sit and dangle your hands in.",
            ),
        },
        Work {
            id: "fishers_statue",
            label: "A statue of the first fishers",
            shape: M::Statue,
            champion: MARA,
            parts: 2,
            cost: 60,
            told: "Mara wants a statue of the first fishers",
            line: "They built this place. They deserve a stone.",
            again: (
                "A plaque with the first fishers' names",
                "Mara wants the first fishers' names on a plaque",
                "Their names, so nobody forgets them.",
            ),
        },
        Work {
            id: "new_well",
            label: "A new well by the cottages",
            shape: M::Well,
            champion: LEO,
            parts: 2,
            cost: 40,
            told: "Leo wants a new well by the cottages",
            line: "The old one tastes of iron.",
            again: (
                "A little roof over the well",
                "Leo wants a little roof over the new well",
                "Rain in the bucket's no good to anyone.",
            ),
        },
        Work {
            id: "postbox",
            label: "A postbox for the harbour",
            shape: M::Postbox,
            champion: EMMA,
            parts: 1,
            cost: 30,
            told: "Emma wants a postbox so the children can write",
            line: "Letters to the mainland, from us!",
            again: (
                "The postbox painted red",
                "Emma wants the postbox painted red",
                "A postbox should be red. Everyone knows that.",
            ),
        },
        Work {
            id: "birdhouses",
            label: "Birdhouses along the lane",
            shape: M::Birdhouse,
            champion: MIA,
            parts: 2,
            cost: 20,
            told: "Mia wants birdhouses along the lane",
            line: "The swallows need houses too.",
            again: (
                "A bird table by the birdhouses",
                "Mia wants a bird table to go with the birdhouses",
                "The robins need somewhere to eat.",
            ),
        },
        Work {
            id: "signposts",
            label: "Signposts for visitors",
            shape: M::Signpost,
            champion: SOFIA,
            parts: 1,
            cost: 25,
            told: "Sofia wants signposts for the visitors",
            line: "Half of them end up in the harbour looking for the pub.",
            again: (
                "A map board by the pier",
                "Sofia wants a map board for the visitors",
                "One big map, and nobody asks me the way again.",
            ),
        },
        Work {
            id: "quay_planters",
            label: "Flowers along the quay",
            shape: M::Planter,
            champion: MARA,
            parts: 2,
            cost: 25,
            told: "Mara wants flowers along the quay",
            line: "A bit of colour for market day.",
            again: (
                "Window boxes on the quay houses",
                "Mara wants window boxes on the quay houses",
                "Every sill on the quay in flower.",
            ),
        },
        Work {
            id: "school_swings",
            label: "Swings by the school",
            shape: M::Swing,
            champion: MIA,
            parts: 2,
            cost: 35,
            told: "Mia wants swings by the school",
            line: "Real swings. Not a rope on a tree.",
            again: (
                "A slide beside the school swings",
                "Mia wants a slide beside the school swings",
                "A slide! The tallest one on the island.",
            ),
        },
        Work {
            id: "lighthouse_paint",
            label: "The lighthouse painted",
            shape: M::Tower,
            champion: JONAS,
            parts: 2,
            cost: 45,
            told: "Jonas wants the lighthouse painted",
            line: "Ships can't see it for the peeling.",
            again: (
                "A brighter lamp in the lighthouse",
                "Jonas wants a brighter lamp in the lighthouse",
                "Fresh paint, old bulb. Let's finish the job.",
            ),
        },
    ]
};

/// The works of the harbour's later years, once every first work has been
/// built and come round again: new things, not another coat on old ones.
pub(crate) const LATER_WORKS: &[Work] = {
    use world_projection::MarkShape as M;
    const ONCE: (&str, &str, &str) = ("", "", "");
    &[
        Work {
            id: "lifeboat_station",
            label: "A lifeboat station on the point",
            shape: M::House,
            champion: JONAS,
            parts: 3,
            cost: 60,
            told: "Jonas wants a lifeboat station on the point",
            line: "Next time a boat's in trouble, we'll be ready.",
            again: ONCE,
        },
        Work {
            id: "village_hall",
            label: "A hall for dances and meetings",
            shape: M::House,
            champion: NOAH,
            parts: 3,
            cost: 60,
            told: "Noah wants a hall for the whole harbour",
            line: "Somewhere to meet that isn't the pub. No offence, Leo.",
            again: ONCE,
        },
        Work {
            id: "bathing_huts",
            label: "Striped bathing huts on the beach",
            shape: M::Tent,
            champion: SOFIA,
            parts: 2,
            cost: 40,
            told: "Sofia wants bathing huts on the beach",
            line: "Striped huts, all in a row. Picture it!",
            again: ONCE,
        },
        Work {
            id: "orchard",
            label: "An orchard on the hill",
            shape: M::Tree,
            champion: EMMA,
            parts: 3,
            cost: 35,
            told: "Emma wants an orchard on the hill",
            line: "Apples for every child in the school, one day.",
            again: ONCE,
        },
        Work {
            id: "smokehouse",
            label: "A smokehouse for the catch",
            shape: M::House,
            champion: MARA,
            parts: 2,
            cost: 45,
            told: "Mara wants a smokehouse for the catch",
            line: "Smoked mackerel on the bakery counter. Think of it.",
            again: ONCE,
        },
        Work {
            id: "footbridge",
            label: "A footbridge over the stream",
            shape: M::Bridge,
            champion: EVAN,
            parts: 2,
            cost: 50,
            told: "Evan wants a footbridge over the stream",
            line: "Wet boots every morning. Not any more.",
            again: ONCE,
        },
        Work {
            id: "telescope",
            label: "A telescope on the cliff",
            shape: M::Tower,
            champion: MIA,
            parts: 2,
            cost: 45,
            told: "Mia wants a telescope on the cliff",
            line: "I want to see Saturn's rings. Properly.",
            again: ONCE,
        },
        Work {
            id: "cliff_path",
            label: "A path along the cliffs",
            shape: M::Signpost,
            champion: LEO,
            parts: 2,
            cost: 35,
            told: "Leo wants a proper path along the cliffs",
            line: "The best view on the island, and you need goat's legs to reach it.",
            again: ONCE,
        },
        Work {
            id: "fish_market",
            label: "A fish market on the quay",
            shape: M::Stall,
            champion: JONAS,
            parts: 2,
            cost: 45,
            told: "Jonas wants a fish market on the quay",
            line: "Sell it fresh off the boat, where folk can see it.",
            again: ONCE,
        },
        Work {
            id: "bread_oven",
            label: "An oven everyone can bake in",
            shape: M::House,
            champion: MARA,
            parts: 2,
            cost: 40,
            told: "Mara wants an oven the whole harbour can use",
            line: "Sunday loaves for anyone who brings their own dough.",
            again: ONCE,
        },
        Work {
            id: "chapel_bell",
            label: "A bell for the chapel tower",
            shape: M::Tower,
            champion: NOAH,
            parts: 2,
            cost: 55,
            told: "Noah wants a bell for the chapel tower",
            line: "The tower's been silent fifty years. Long enough.",
            again: ONCE,
        },
        Work {
            id: "puppet_theatre",
            label: "A puppet theatre for the school",
            shape: M::Tent,
            champion: EMMA,
            parts: 2,
            cost: 30,
            told: "Emma wants a puppet theatre for the school",
            line: "The children have written a play. It needs a stage.",
            again: ONCE,
        },
        Work {
            id: "glasshouse",
            label: "A glasshouse for winter greens",
            shape: M::Garden,
            champion: MARA,
            parts: 2,
            cost: 50,
            told: "Mara wants a glasshouse for winter greens",
            line: "Lettuce in January. Imagine the faces.",
            again: ONCE,
        },
        Work {
            id: "rowing_club",
            label: "A rowing club by the slipway",
            shape: M::House,
            champion: EVAN,
            parts: 2,
            cost: 45,
            told: "Evan wants a rowing club by the slipway",
            line: "Four oars, one boat, and every Sunday morning.",
            again: ONCE,
        },
        Work {
            id: "duck_pond",
            label: "A duck pond on the green",
            shape: M::Fountain,
            champion: MIA,
            parts: 2,
            cost: 30,
            told: "Mia wants a duck pond on the green",
            line: "Ducks! We need ducks. Everyone needs ducks.",
            again: ONCE,
        },
        Work {
            id: "cottages",
            label: "New cottages for newcomers",
            shape: M::House,
            champion: EVAN,
            parts: 3,
            cost: 70,
            told: "Evan wants cottages for the newcomers",
            line: "Folk keep coming. They'll need roofs.",
            again: ONCE,
        },
        Work {
            id: "beacon",
            label: "A beacon on the hill",
            shape: M::Lamp,
            champion: JONAS,
            parts: 2,
            cost: 40,
            told: "Jonas wants a beacon on the hill",
            line: "Lit on feast nights, and on nights a boat is late.",
            again: ONCE,
        },
        Work {
            id: "ferry_shelter",
            label: "A shelter for the ferry queue",
            shape: M::Tent,
            champion: NOAH,
            parts: 2,
            cost: 35,
            told: "Noah wants a shelter where folk wait for the ferry",
            line: "Nobody should wait for the ferry in the rain.",
            again: ONCE,
        },
        Work {
            id: "picnic_tables",
            label: "Picnic tables on the green",
            shape: M::Bench,
            champion: SOFIA,
            parts: 1,
            cost: 25,
            told: "Sofia wants picnic tables on the green",
            line: "Summer lunches outside. Tables, please, not laps.",
            again: ONCE,
        },
        Work {
            id: "sundial",
            label: "A sundial in the school yard",
            shape: M::Statue,
            champion: EMMA,
            parts: 1,
            cost: 20,
            told: "Emma wants a sundial in the school yard",
            line: "A clock that runs on sunshine. What a lesson.",
            again: ONCE,
        },
        Work {
            id: "dovecote",
            label: "A dovecote behind the bakery",
            shape: M::Birdhouse,
            champion: MARA,
            parts: 1,
            cost: 20,
            told: "Mara wants a dovecote behind the bakery",
            line: "White doves on the bakery roof. Very grand.",
            again: ONCE,
        },
        Work {
            id: "seal_hide",
            label: "A hide for watching the seals",
            shape: M::House,
            champion: MIA,
            parts: 2,
            cost: 35,
            told: "Mia wants a hide for watching the seals",
            line: "They're shy. We need somewhere to sit very still.",
            again: ONCE,
        },
        Work {
            id: "herb_garden",
            label: "A herb garden by the pub",
            shape: M::Garden,
            champion: LEO,
            parts: 2,
            cost: 25,
            told: "Leo wants a herb garden by the pub",
            line: "Fresh mint for the summer punch. Don't tell the brewery.",
            again: ONCE,
        },
        Work {
            id: "maypole",
            label: "A maypole on the green",
            shape: M::Flag,
            champion: SOFIA,
            parts: 1,
            cost: 20,
            told: "Sofia wants a maypole on the green",
            line: "Ribbons, a fiddle and everyone going round.",
            again: ONCE,
        },
        Work {
            id: "tide_gauge",
            label: "A tide board at the harbour mouth",
            shape: M::Signpost,
            champion: NOAH,
            parts: 1,
            cost: 25,
            told: "Noah wants a tide board at the harbour mouth",
            line: "So every skipper knows the water before they leave.",
            again: ONCE,
        },
        Work {
            id: "reading_room",
            label: "A reading room above the pub",
            shape: M::Shop,
            champion: LEO,
            parts: 2,
            cost: 40,
            told: "Leo wants a reading room above the pub",
            line: "Books upstairs, beer downstairs. Civilised.",
            again: ONCE,
        },
        Work {
            id: "boat_yard",
            label: "A yard for building boats",
            shape: M::House,
            champion: EVAN,
            parts: 3,
            cost: 60,
            told: "Evan wants a proper yard for building boats",
            line: "The first boat built here in forty years. That's the plan.",
            again: ONCE,
        },
        Work {
            id: "mural",
            label: "A painted map on the harbour wall",
            shape: M::Bunting,
            champion: SOFIA,
            parts: 1,
            cost: 20,
            told: "Sofia wants the harbour painted on the harbour wall",
            line: "Every house, every boat, every one of us. Painted big.",
            again: ONCE,
        },
        Work {
            id: "lookout_tower",
            label: "A lookout tower on the headland",
            shape: M::Tower,
            champion: NOAH,
            parts: 2,
            cost: 50,
            told: "Noah wants a lookout tower on the headland",
            line: "See a storm an hour sooner. That's an hour to haul the boats.",
            again: ONCE,
        },
        Work {
            id: "sea_pool",
            label: "A sea pool in the rocks",
            shape: M::Fountain,
            champion: MIA,
            parts: 2,
            cost: 45,
            told: "Mia wants a sea pool built into the rocks",
            line: "The tide fills it twice a day. Free swimming for ever.",
            again: ONCE,
        },
        Work {
            id: "net_loft",
            label: "A new net loft over the quay",
            shape: M::House,
            champion: JONAS,
            parts: 2,
            cost: 50,
            told: "Jonas wants a new net loft over the quay",
            line: "The old one leans. One more gale and it lies down.",
            again: ONCE,
        },
        Work {
            id: "school_library",
            label: "A library corner for the school",
            shape: M::Shop,
            champion: EMMA,
            parts: 2,
            cost: 35,
            told: "Emma wants a proper library corner for the school",
            line: "Every child should have a shelf of their own to choose from.",
            again: ONCE,
        },
        Work {
            id: "tea_rooms",
            label: "Tea rooms on the pier",
            shape: M::Shop,
            champion: SOFIA,
            parts: 2,
            cost: 50,
            told: "Sofia wants tea rooms at the end of the pier",
            line: "Scones, a pot of tea and the whole sea to look at.",
            again: ONCE,
        },
        Work {
            id: "bread_cart",
            label: "A bread cart for the far cottages",
            shape: M::Stall,
            champion: MARA,
            parts: 1,
            cost: 30,
            told: "Mara wants a cart to take bread to the far cottages",
            line: "Old Mrs Pell can't walk to the bakery any more. The bread can walk to her.",
            again: ONCE,
        },
        Work {
            id: "pub_terrace",
            label: "A terrace on the front of the pub",
            shape: M::Bench,
            champion: LEO,
            parts: 2,
            cost: 45,
            told: "Leo wants a terrace on the front of the pub",
            line: "Summer evenings outside, with the sea right there.",
            again: ONCE,
        },
        Work {
            id: "workshop",
            label: "A workshop anyone can use",
            shape: M::House,
            champion: EVAN,
            parts: 2,
            cost: 50,
            told: "Evan wants a workshop anyone on the island can use",
            line: "Tools on the wall, a bench for everyone. Mend your own chair.",
            again: ONCE,
        },
        Work {
            id: "gull_gate",
            label: "A gate to keep the gulls off the bins",
            shape: M::Signpost,
            champion: NOAH,
            parts: 1,
            cost: 20,
            told: "Noah wants a gate to keep the gulls off the bins",
            line: "They've learned to lift the lids. I've seen it.",
            again: ONCE,
        },
        Work {
            id: "paddling_pool",
            label: "A paddling pool for the little ones",
            shape: M::Fountain,
            champion: EMMA,
            parts: 1,
            cost: 25,
            told: "Emma wants a paddling pool for the little ones",
            line: "The sea's too cold for the smallest. A warm pool, in the sun.",
            again: ONCE,
        },
        Work {
            id: "lantern_walk",
            label: "Lanterns along the harbour wall",
            shape: M::Lantern,
            champion: SOFIA,
            parts: 2,
            cost: 35,
            told: "Sofia wants lanterns all along the harbour wall",
            line: "Walk home at night by lantern light. Every night, not just feast nights.",
            again: ONCE,
        },
        Work {
            id: "bandstand_roof",
            label: "A roof for the bandstand",
            shape: M::Tent,
            champion: LEO,
            parts: 1,
            cost: 30,
            told: "Leo wants a roof on the bandstand",
            line: "The fiddles don't like the rain. Neither does the fiddler.",
            again: ONCE,
        },
        Work {
            id: "herring_shed",
            label: "A shed for salting herring",
            shape: M::House,
            champion: JONAS,
            parts: 2,
            cost: 40,
            told: "Jonas wants a shed for salting the herring",
            line: "My gran salted herring all winter. We can again.",
            again: ONCE,
        },
        Work {
            id: "wind_break",
            label: "A windbreak for the school yard",
            shape: M::Planter,
            champion: MIA,
            parts: 1,
            cost: 25,
            told: "Mia wants a windbreak for the school yard",
            line: "At break the wind blows the little ones over. Honestly.",
            again: ONCE,
        },
        Work {
            id: "chapel_windows",
            label: "New windows in the old chapel",
            shape: M::Tower,
            champion: MARA,
            parts: 2,
            cost: 50,
            told: "Mara wants new windows in the old chapel",
            line: "Coloured glass, like my gran remembered. The light on the floor.",
            again: ONCE,
        },
        Work {
            id: "jetty_ladder",
            label: "A ladder down the jetty for swimmers",
            shape: M::Pier,
            champion: EVAN,
            parts: 1,
            cost: 20,
            told: "Evan wants a ladder down the jetty for swimmers",
            line: "Getting in is easy. Getting out is the trick.",
            again: ONCE,
        },
    ]
};

/// What the harbour's people make for themselves, without being asked
/// and without the fund, when the player keeps saying not now: small
/// things, a part at a time, slower than what the fund pays for.
pub(crate) struct OwnWork {
    pub id: &'static str,
    pub label: &'static str,
    pub shape: world_projection::MarkShape,
    champion: EntityId,
    helper: EntityId,
    /// What comes up: who is making what.
    told: &'static str,
    line: &'static str,
    /// What the fund would put in, if asked.
    cost: i64,
}

/// How many parts each of the harbour's own works takes.
const OWN_PARTS: i64 = 2;

/// Where the harbour's readiness to get on with things by itself is kept:
/// each want turned down or let lapse adds to it, each of its own works
/// takes from it.
pub(crate) const INITIATIVE: &str = "story.initiative";
/// How much the harbour must have been let down before it starts
/// something of its own.
const INITIATIVE_TO_START: i64 = 2;
/// What someone else taking up a want the player let down adds to the
/// harbour's readiness to get on with things itself.
const TAKEN_UP_READINESS: i64 = 2;
/// What each part of one of its own works takes from the harbour's
/// readiness. Since v0.24 nobody asks the same thing again and again, so
/// the harbour is let down less often, and a part takes less.
const OWN_PART_COST: i64 = 1;

fn initiative(by: i64) -> Effect {
    Effect::Add {
        entity: STORY,
        key: INITIATIVE,
        by,
        min: 0,
        max: 12,
    }
}

pub(crate) const OWN_WORKS: &[OwnWork] = {
    use world_projection::MarkShape as M;
    &[
        OwnWork {
            id: "own_driftwood_bench",
            label: "A driftwood bench on the shingle",
            shape: M::Bench,
            champion: EVAN,
            helper: JONAS,
            told: "Evan and Jonas are making a bench out of driftwood",
            line: "Found two planks on the tide line. Watch this.",
            cost: 15,
        },
        OwnWork {
            id: "own_rope_swing",
            label: "A rope swing on the old oak",
            shape: M::Swing,
            champion: MIA,
            helper: EVAN,
            told: "Mia and Evan are hanging a rope swing on the old oak",
            line: "If nobody's building swings, we'll hang our own.",
            cost: 10,
        },
        OwnWork {
            id: "own_painted_stones",
            label: "Painted stones along the path",
            shape: M::Planter,
            champion: EMMA,
            helper: MIA,
            told: "Emma's class is painting stones for the path",
            line: "Thirty children, thirty stones, and a lot of paint.",
            cost: 10,
        },
        OwnWork {
            id: "own_notice_board",
            label: "A notice board outside the pub",
            shape: M::Signpost,
            champion: LEO,
            helper: SOFIA,
            told: "Leo and Sofia are putting up a notice board",
            line: "Somewhere to pin what's on. We'll do it ourselves.",
            cost: 15,
        },
        OwnWork {
            id: "own_herb_bed",
            label: "A herb bed behind the bakery",
            shape: M::Garden,
            champion: MARA,
            helper: EMMA,
            told: "Mara and Emma are digging a herb bed",
            line: "Thyme, sage and a bit of patience.",
            cost: 10,
        },
        OwnWork {
            id: "own_cairn",
            label: "A cairn on the point",
            shape: M::Statue,
            champion: JONAS,
            helper: NOAH,
            told: "Jonas and Noah are raising a cairn on the point",
            line: "A stone for every boat that came home. We'll carry them up.",
            cost: 10,
        },
        OwnWork {
            id: "own_window_boxes",
            label: "Window boxes on the cottages",
            shape: M::Planter,
            champion: SOFIA,
            helper: MARA,
            told: "Sofia and Mara are knocking up window boxes",
            line: "Old fish crates, a bit of soil. Who needs the fund?",
            cost: 15,
        },
        OwnWork {
            id: "own_bait_shed",
            label: "A bait shed by the slipway",
            shape: M::House,
            champion: JONAS,
            helper: EVAN,
            told: "Jonas and Evan are putting up a bait shed",
            line: "Nobody asked. We're building it anyway.",
            cost: 20,
        },
        OwnWork {
            id: "own_sandpit",
            label: "A sandpit by the school",
            shape: M::Garden,
            champion: EMMA,
            helper: EVAN,
            told: "Emma and Evan are making a sandpit for the little ones",
            line: "Sand's free. The beach is right there.",
            cost: 10,
        },
        OwnWork {
            id: "own_bird_table",
            label: "A bird table on the green",
            shape: M::Birdhouse,
            champion: MIA,
            helper: NOAH,
            told: "Mia and Noah are making a bird table",
            line: "Noah's got the wood. I've got the crumbs.",
            cost: 10,
        },
        OwnWork {
            id: "own_stepping_stones",
            label: "Stepping stones over the stream",
            shape: M::Bridge,
            champion: EVAN,
            helper: LEO,
            told: "Evan and Leo are laying stepping stones over the stream",
            line: "Big flat ones. Nobody falls in twice.",
            cost: 15,
        },
        OwnWork {
            id: "own_book_box",
            label: "A book swap box by the quay",
            shape: M::Postbox,
            champion: EMMA,
            helper: SOFIA,
            told: "Emma and Sofia are making a book swap box",
            line: "Take a book, leave a book. The honest way.",
            cost: 10,
        },
        OwnWork {
            id: "own_flag_line",
            label: "A line of flags across the lane",
            shape: M::Bunting,
            champion: LEO,
            helper: MIA,
            told: "Leo and Mia are sewing flags for the lane",
            line: "Old shirts make grand flags. Don't ask whose.",
            cost: 10,
        },
        OwnWork {
            id: "own_net_rack",
            label: "A drying rack for the nets",
            shape: M::Flag,
            champion: JONAS,
            helper: MARA,
            told: "Jonas and Mara are building a rack to dry the nets",
            line: "Nets on the rack, not on the bakery wall. Mara's orders.",
            cost: 15,
        },
        OwnWork {
            id: "own_wildflowers",
            label: "Wildflowers on the cliff path",
            shape: M::Garden,
            champion: MARA,
            helper: NOAH,
            told: "Mara and Noah are sowing wildflowers along the cliff path",
            line: "A pocket of seed and a windy day. That's all it takes.",
            cost: 10,
        },
        OwnWork {
            id: "own_fire_pit",
            label: "A fire pit on the beach",
            shape: M::Lantern,
            champion: LEO,
            helper: JONAS,
            told: "Leo and Jonas are digging a fire pit on the beach",
            line: "Stones in a ring, and the summer nights are ours.",
            cost: 10,
        },
        OwnWork {
            id: "own_hen_house",
            label: "A hen house behind the school",
            shape: M::House,
            champion: EMMA,
            helper: MARA,
            told: "Emma and Mara are building a hen house for the school",
            line: "Eggs for the bakery, lessons for the children.",
            cost: 20,
        },
        OwnWork {
            id: "own_lookout",
            label: "A lookout seat on the headland",
            shape: M::Bench,
            champion: NOAH,
            helper: EVAN,
            told: "Noah and Evan are making a lookout seat on the headland",
            line: "A seat to watch the boats come in. I'll carry the tools.",
            cost: 15,
        },
        OwnWork {
            id: "own_jar_lanterns",
            label: "Jam-jar lanterns up the lane",
            shape: M::Lantern,
            champion: SOFIA,
            helper: EMMA,
            told: "Sofia and Emma are hanging jam-jar lanterns up the lane",
            line: "A candle in every jar. The lane will twinkle.",
            cost: 10,
        },
        OwnWork {
            id: "own_shell_path",
            label: "A shell path to the chapel",
            shape: M::Planter,
            champion: MIA,
            helper: SOFIA,
            told: "Mia and Sofia are laying a path of shells to the chapel",
            line: "Every shell on the beach is ours for the picking.",
            cost: 10,
        },
        OwnWork {
            id: "own_boat_planter",
            label: "An old boat full of flowers",
            shape: M::Boat,
            champion: JONAS,
            helper: MIA,
            told: "Jonas and Mia are filling an old boat with flowers",
            line: "She won't float again, but she'll bloom.",
            cost: 10,
        },
        OwnWork {
            id: "own_skittle_alley",
            label: "A skittle alley behind the pub",
            shape: M::Stall,
            champion: LEO,
            helper: EVAN,
            told: "Leo and Evan are laying a skittle alley behind the pub",
            line: "Nine pins and a bowl. Old-fashioned fun.",
            cost: 15,
        },
        OwnWork {
            id: "own_weathervane",
            label: "A tin weathervane on the net loft",
            shape: M::Flag,
            champion: NOAH,
            helper: MIA,
            told: "Noah and Mia are cutting a weathervane out of tin",
            line: "A fish that points into the wind. Mia's drawing.",
            cost: 10,
        },
        OwnWork {
            id: "own_story_chair",
            label: "A storytelling chair in the school",
            shape: M::Bench,
            champion: EMMA,
            helper: LEO,
            told: "Emma and Leo are carving a storytelling chair",
            line: "Whoever sits in it has to tell a story. School rules.",
            cost: 15,
        },
        OwnWork {
            id: "own_bee_hives",
            label: "Two beehives on the hill",
            shape: M::Birdhouse,
            champion: MARA,
            helper: JONAS,
            told: "Mara and Jonas are setting up beehives on the hill",
            line: "Honey for the bakery, if the bees agree.",
            cost: 20,
        },
        OwnWork {
            id: "own_kite_hill",
            label: "Kites for the children on the hill",
            shape: M::Flag,
            champion: SOFIA,
            helper: NOAH,
            told: "Sofia and Noah are making kites for the children",
            line: "Brown paper, string and a good wind. Up they go.",
            cost: 10,
        },
        OwnWork {
            id: "own_tide_pools",
            label: "Name boards for the rock pools",
            shape: M::Signpost,
            champion: MIA,
            helper: JONAS,
            told: "Mia and Jonas are naming the rock pools",
            line: "Every pool gets a name and a board. Crab Castle's mine.",
            cost: 10,
        },
        OwnWork {
            id: "own_quay_mosaic",
            label: "A pebble mosaic on the quay",
            shape: M::Planter,
            champion: EVAN,
            helper: SOFIA,
            told: "Evan and Sofia are setting a pebble mosaic in the quay",
            line: "A great big fish in pebbles. You'll see it from the ferry.",
            cost: 15,
        },
        OwnWork {
            id: "own_music_shed",
            label: "A practice shed for the band",
            shape: M::House,
            champion: LEO,
            helper: EMMA,
            told: "Leo and Emma are fixing up a shed for the band to practise in",
            line: "Somewhere the fiddles can be as loud as they like.",
            cost: 20,
        },
        OwnWork {
            id: "own_apple_press",
            label: "An apple press on the green",
            shape: M::Well,
            champion: NOAH,
            helper: MARA,
            told: "Noah and Mara are building an apple press",
            line: "Every windfall on the island, into cider or juice.",
            cost: 15,
        },
    ]
};

/// A made-up text for the life of the program: each different one is
/// kept once, however often the deck is dealt.
pub(crate) fn leak(text: String) -> &'static str {
    static KEPT: std::sync::OnceLock<std::sync::Mutex<std::collections::BTreeSet<&'static str>>> =
        std::sync::OnceLock::new();
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

/// One rung of the ladder of works: a work built, then built on once
/// more, then the later years' works, each after the one before.
pub(crate) struct Rung {
    pub id: &'static str,
    pub label: &'static str,
    pub work: &'static Work,
    told: &'static str,
    line: &'static str,
}

/// Every rung of the ladder, in order: every first work once, then each
/// built on once more, then the works of the later years, so there is
/// always one under way.
pub(crate) fn ladder() -> &'static [Rung] {
    static LADDER: std::sync::OnceLock<Vec<Rung>> = std::sync::OnceLock::new();
    LADDER.get_or_init(|| {
        let first = WORKS.iter().map(|work| Rung {
            id: work.id,
            label: work.label,
            work,
            told: work.told,
            line: work.line,
        });
        let again = WORKS.iter().map(|work| Rung {
            id: leak(format!("{}_painted", work.id)),
            label: work.again.0,
            work,
            told: work.again.1,
            line: work.again.2,
        });
        let later = LATER_WORKS.iter().map(|work| Rung {
            id: work.id,
            label: work.label,
            work,
            told: work.told,
            line: work.line,
        });
        first.chain(again).chain(later).collect()
    })
}

/// The rungs the ladder had before the later years' works: flowers round
/// every work, then lamps and bunting on it. Nothing asks for them now,
/// but a World that began them keeps what it built, and its history is
/// still told.
fn retired_rungs() -> &'static [Rung] {
    static RETIRED: std::sync::OnceLock<Vec<Rung>> = std::sync::OnceLock::new();
    RETIRED.get_or_init(|| {
        let mut rungs = Vec::new();
        for (round, name) in [(2, "flowers"), (3, "lit")] {
            for work in WORKS {
                let the = the(work.label);
                let (label, told, line) = if round == 2 {
                    (
                        format!("Flowers round {the}"),
                        format!("Flowers would brighten {the}"),
                        format!("Something growing round {the}. That's all it needs."),
                    )
                } else {
                    (
                        format!("Lamps and bunting on {the}"),
                        format!("Let's light up {the}"),
                        format!(
                            "A few lamps and a string of bunting on {the}. It'll glow at night."
                        ),
                    )
                };
                rungs.push(Rung {
                    id: leak(format!("{}_{name}", work.id)),
                    label: leak(label),
                    work,
                    told: leak(told),
                    line: leak(line),
                });
            }
        }
        rungs
    })
}

/// "A bandstand on the square" as "the bandstand on the square".
pub(crate) fn the(label: &str) -> String {
    // "The bandstand painted red and gold" is the bandstand.
    let label = painted(label).map_or(label, |(thing, _)| thing);
    let rest = label
        .strip_prefix("A ")
        .or_else(|| label.strip_prefix("An "))
        .or_else(|| label.strip_prefix("The "));
    let rest = rest.unwrap_or(label);
    let mut chars = rest.chars();
    let lower = chars
        .next()
        .map(|first| first.to_lowercase().chain(chars).collect::<String>())
        .unwrap_or_default();
    format!("the {lower}")
}

/// A work's label that tells what was done to a thing ("The bandstand
/// painted red and gold"): the thing, and how it was painted.
fn painted(label: &str) -> Option<(&str, &str)> {
    let (thing, how) = label.split_once(" painted")?;
    // "A painted map" is a map.
    (thing.split_whitespace().count() > 1 || !matches!(thing, "A" | "An" | "The"))
        .then_some((thing, how.trim()))
}

/// What finishing a work was, for "We've finished …": "painting the
/// bandstand red and gold", or the work itself.
fn the_job(label: &str) -> String {
    match painted(label) {
        Some((_, "")) => format!("painting {}", the(label)),
        Some((_, how)) => format!("painting {} {how}", the(label)),
        None => the(label),
    }
}

fn lowered(label: &str) -> String {
    let mut chars = label.chars();
    chars
        .next()
        .map(|first| first.to_lowercase().chain(chars).collect::<String>())
        .unwrap_or_default()
}

/// What whoever asked for a work says as a part of it is built, one of
/// these in turn along the ladder, so it is not the same every time.
const PART_LINES: [&str; 12] = [
    "Coming along nicely.",
    "Another part done. Look at that.",
    "Getting there, bit by bit.",
    "You can see what it'll be now.",
    "Stood back and had a look. Not bad.",
    "Sawdust everywhere. Always a good sign.",
    "The best bit's still to come.",
    "One more step. I can feel it.",
    "Nearly looks like the drawing.",
    "My back aches. Worth it.",
    "It's taking shape, isn't it?",
    "Half the harbour came to watch today.",
];

/// What they still say about it in the days after.
const PART_REMEMBERED: [&str; 6] = [
    "It's coming along, what we're building.",
    "I walk past it twice a day just to look.",
    "The children ask every morning if it's done.",
    "Every part we build, the harbour stands taller.",
    "I dream about it, you know. The finished thing.",
    "People stop and stare. I love that.",
];

/// What someone says when they got on with one of the harbour's own
/// works without waiting to be asked.
const OWN_LAPSE_LINES: [&str; 6] = [
    "Didn't wait to be asked.",
    "We just got on with it.",
    "Nobody said no, so we said yes.",
    "An evening's work, and look.",
    "Borrowed a saw and got started.",
    "If you want a thing done, do it yourself.",
];

/// What whoever asked for a work says as it is opened with a party.
const OPENING_LINES: [&str; 6] = [
    "Speeches, cake and everyone in their best!",
    "Bunting up and the whole harbour here!",
    "A ribbon, a pair of scissors and a cheer!",
    "I said a few words. Well, a lot of words.",
    "The children cut the ribbon. Twice.",
    "Everyone came. Even the gulls.",
];

/// What they say as people just start using it.
const IN_USE_LINES: [&str; 6] = [
    "No fuss. We just started.",
    "Used it twice already today.",
    "Folk found it before I'd put the tools away.",
    "Best opening is using it, I say.",
    "It's as if it was always there.",
    "Somebody's sitting in it already.",
];

/// Once a work on the ladder is finished, whoever asked for it asks how it
/// should be opened.
fn opening(rung: &Rung, index: usize) -> Spec {
    use Condition::{Finished, Unmarked};
    let the = the(rung.label);
    let job = the_job(rung.label);
    let named = {
        let mut chars = the.chars();
        chars
            .next()
            .map(|first| first.to_uppercase().chain(chars).collect::<String>())
            .unwrap_or_default()
    };
    let opened: &'static str = leak(format!("opened_{}", rung.id));
    spec(
        leak(format!("open_{}", rung.id)),
        Shape {
            asker: rung.work.champion,
            want: false,
            // Only a work built since openings began is opened: one a World
            // finished long before is not asked about all over again.
            requires: vec![
                Finished(rung.id),
                Condition::Marked(worked_on(rung.id), 0),
                Unmarked(opened),
            ],
            lasts: 3,
            rests: 30,
            weight: 6,
            eases: Vec::new(),
            timely: true,
        },
        (
            leak(format!("The harbour finished {job}")),
            leak(format!("We've finished {job}! How shall we open it?")),
        ),
        vec![
            yes(
                "party",
                "Open it with a party",
                "The harbour fund pays 20 for bunting and cake.",
                vec![has(HARBOR, 20)],
                said(
                    leak(format!("{}_opened", rung.id)),
                    leak(format!("The harbour opened {the} with a party")),
                    OPENING_LINES[index % OPENING_LINES.len()],
                    spend(HARBOR, 20).into_iter().chain([mood(2), mark(opened)]),
                ),
            ),
            other(
                "use",
                "Just start using it",
                "Nothing spent. It's there to be used.",
                said(
                    leak(format!("{}_in_use", rung.id)),
                    leak(format!("Folk started using {the}")),
                    IN_USE_LINES[index % IN_USE_LINES.len()],
                    [mood(1), mark(opened)],
                ),
            ),
        ],
        said(
            leak(format!("{}_opened_quietly", rung.id)),
            leak(format!("{named} opened without any fuss")),
            "It just sort of opened.",
            [mark(opened)],
        ),
    )
}

/// The mark a part of a work built leaves, so that once it is finished it
/// can be opened.
fn worked_on(rung: &str) -> &'static str {
    leak(format!("worked_on_{rung}"))
}

fn works() -> Vec<Spec> {
    use Condition::{Finished, Marked, Unfinished};
    let mut specs = Vec::new();
    let ladder = ladder();
    let retired = retired_rungs();
    for (index, rung) in ladder.iter().chain(retired).enumerate() {
        let work = rung.work;
        let before = if index == 0 {
            vec![Finished("pier"), Finished("lamp")]
        } else if index < ladder.len() {
            vec![Finished(ladder[index - 1].id)]
        } else {
            // Retired: never asked for again.
            vec![Marked("retired_ladder", 0)]
        };
        let lower = lowered(rung.label);
        let shape = Shape {
            asker: work.champion,
            want: true,
            requires: [vec![Unfinished(rung.id)], before].concat(),
            lasts: 3,
            // A part every three weeks or so, so a first work takes a
            // month or more and the ladder lasts past a year; later, with
            // the harbour practised at building, every fortnight or so.
            rests: if index < WORKS.len() { 20 } else { 14 },
            weight: 4,
            eases: vec![up("spirits")],
            timely: false,
        };
        specs.push(spec(
            leak(format!("work_{}", rung.id)),
            shape,
            (rung.told, rung.line),
            vec![
                yes(
                    "fund",
                    "Pay for the next part",
                    leak(format!(
                        "The harbour fund pays {}. {} is a part nearer.",
                        work.cost, rung.label
                    )),
                    vec![has(HARBOR, work.cost)],
                    said(
                        leak(format!("{}_part_built", rung.id)),
                        leak(format!("Work went on at {lower}")),
                        PART_LINES[index % PART_LINES.len()],
                        spend(HARBOR, work.cost).into_iter().chain([
                            Effect::Advance(rung.id),
                            mood(1),
                            mark(worked_on(rung.id)),
                        ]),
                    )
                    .remembered(PART_REMEMBERED[index % PART_REMEMBERED.len()]),
                ),
                no(
                    "wait",
                    "It can wait",
                    "Nothing spent this time. Folk may start something of their own.",
                    said(
                        leak(format!("{}_put_off", rung.id)),
                        leak(format!("{} was put off", rung.label)),
                        "Another time, then.",
                        [],
                    ),
                ),
            ],
            said(
                leak(format!("{}_waited", rung.id)),
                leak(format!("{} waited another while", rung.label)),
                "It'll keep.",
                [],
            ),
        ));
        // Every work on the ladder is opened, once finished.
        if index < ladder.len() {
            specs.push(opening(rung, index));
        }
    }
    specs
}

/// The harbour's own works as storylets: each comes up once the harbour
/// has been let down often enough and has taken it up next. Every
/// way it goes builds a part; what the answer changes is who pays, and
/// who grows closer doing it.
fn own_works() -> Vec<Spec> {
    use Condition::Unfinished;
    let mut specs = Vec::new();
    for (index, work) in OWN_WORKS.iter().enumerate() {
        // Whichever the harbour has taken up next: what the player made
        // or built decides it (see [`next_own_work`]).
        let requires = vec![
            Unfinished(work.id),
            Condition::AtLeast(STORY, INITIATIVE, INITIATIVE_TO_START),
            Condition::Is(STORY, OWN_NEXT, work.id),
        ];
        let lower = lowered(work.label);
        let (champion, helper) = (work.champion, work.helper);
        let closer = |by: i64| {
            [
                Effect::Add {
                    entity: champion,
                    key: leak(format!("lives.opinion.{helper}")),
                    by,
                    min: -100,
                    max: 100,
                },
                Effect::Add {
                    entity: helper,
                    key: leak(format!("lives.opinion.{champion}")),
                    by,
                    min: -100,
                    max: 100,
                },
            ]
        };
        let shape = Shape {
            asker: champion,
            want: false,
            requires,
            lasts: 3,
            rests: 6,
            weight: 4,
            eases: vec![up("spirits")],
            timely: false,
        };
        specs.push(spec(
            work.id,
            shape,
            (work.told, work.line),
            vec![
                yes(
                    "chip_in",
                    "Chip in from the fund",
                    leak(format!(
                        "The harbour fund pays {} for materials. {} goes up a part.",
                        work.cost, work.label
                    )),
                    vec![has(HARBOR, work.cost)],
                    said(
                        leak(format!("{}_helped", work.id)),
                        leak(format!("The harbour chipped in for {lower}")),
                        PART_LINES[(index + 5) % PART_LINES.len()],
                        spend(HARBOR, work.cost).into_iter().chain([
                            Effect::Advance(work.id),
                            mood(1),
                            initiative(-OWN_PART_COST),
                        ]),
                    )
                    .remembered(PART_REMEMBERED[(index + 2) % PART_REMEMBERED.len()]),
                ),
                other(
                    "leave",
                    "Leave them to it",
                    "Nothing spent. They'll do it their own way, a bit at a time.",
                    said(
                        leak(format!("{}_by_hand", work.id)),
                        leak(format!(
                            "Work went on at {lower}, by the harbour's own hands"
                        )),
                        PART_LINES[(index + 9) % PART_LINES.len()],
                        [Effect::Advance(work.id), initiative(-OWN_PART_COST)]
                            .into_iter()
                            .chain(closer(6)),
                    ),
                ),
            ],
            said(
                leak(format!("{}_went_on", work.id)),
                leak(format!("{} went on without anyone asking", work.label)),
                OWN_LAPSE_LINES[index % OWN_LAPSE_LINES.len()],
                [Effect::Advance(work.id), initiative(-OWN_PART_COST)]
                    .into_iter()
                    .chain(closer(3)),
            ),
        ));
    }
    specs
}

fn specs() -> &'static [Spec] {
    static SPECS: OnceLock<Vec<Spec>> = OnceLock::new();
    SPECS.get_or_init(|| {
        let mut specs = wants();
        specs.extend(works());
        specs.extend(own_works());
        specs.extend(incidents());
        specs.extend(calendar());
        specs.extend(threads());
        specs.extend(climaxes());
        specs
    })
}

/// The storyteller's deck. It is the same every time, so it is made once:
/// the whole deck is cloned out of the specs otherwise, and people, talks,
/// goals and the weather each ask for it.
pub(crate) fn deck() -> &'static Deck {
    static DECK: OnceLock<Deck> = OnceLock::new();
    DECK.get_or_init(made_deck)
}

fn made_deck() -> Deck {
    Deck {
        story: STORY,
        story_name: "The harbour's year",
        period: crate::persistence::WORLD_DAY_TICKS,
        storylets: specs().iter().map(|spec| spec.storylet.clone()).collect(),
        goals: vec![
            Goal {
                id: "pier",
                parts: 3,
            },
            Goal {
                id: "lamp",
                parts: 2,
            },
        ]
        .into_iter()
        .chain(ladder().iter().chain(retired_rungs()).map(|rung| Goal {
            id: rung.id,
            parts: if rung.id == rung.work.id {
                rung.work.parts
            } else {
                2
            },
        }))
        .chain(OWN_WORKS.iter().map(|work| Goal {
            id: work.id,
            parts: OWN_PARTS,
        }))
        .collect(),
        chapter_periods: CHAPTER_DAYS,
        shortest_chapter: 8,
        pressures: vec!["storm_season", "hard_times", "inspector"],
        fresh_start: vec![Effect::Put {
            entity: STORY,
            key: MOOD,
            to: 0,
        }],
        most_open: 3,
        rarer: 60,
    }
}

pub(crate) fn register_actions(
    actions: &mut ActionRegistry,
) -> Result<(), world_core::ActionError> {
    storylets::register_kept_actions(actions, deck)?;
    lives::register_actions(actions, crate::life::cast_in)?;
    hands::register_actions(actions, crate::handwork::kit)?;
    conversation::register_actions(actions, crate::speech::kit)?;
    calendar::register_actions(actions, crate::almanac::almanac)?;
    actions.register(LendsAHand)?;
    actions.register(TakesUpOwnWork)?;
    actions.register(SpiritsSettle)
}

/// When the player last lent a hand with one of the harbour's own works,
/// in periods.
const LENT: &str = "story.lent";
/// The storyteller's mark for the player having lately been here: made
/// something, lent a hand, or answered someone. While it is recent, the
/// harbour takes up the works the player let wait; a harbour its player
/// has left alone lets them wait.
pub(crate) const LENT_MARK: &str = "player_seen";
/// How many days the player being here keeps the harbour building.
const LENT_LATELY: u64 = 30;
/// How stale the mark may grow before a deed renews it.
const SEEN_EVERY: u64 = 7;
/// The fewest days between two hands lent.
const LEND_EVERY: i64 = 12;

/// The harbour's own work the player can help with, if there is one: one
/// begun and not done, or, once the harbour has been let down at all, the
/// next one it would start.
fn own_work_under_way(state: &world_core::WorldState) -> Option<&'static OwnWork> {
    let deck = deck();
    let begun = OWN_WORKS.iter().find(|work| {
        let done = storylets::progress(state, deck, work.id);
        done > 0 && done < OWN_PARTS
    });
    let let_down = state.entity(STORY).is_some_and(
        |story| matches!(story.component(INITIATIVE), Some(Value::Integer(at)) if *at >= 1),
    );
    begun.or_else(|| {
        let_down
            .then(|| own_next(state).filter(|work| !storylets::finished(state, deck, work.id)))
            .flatten()
    })
}

/// Which of the harbour's own works it has taken up next.
pub(crate) const OWN_NEXT: &str = "story.own_next";
/// When the harbour last took up one of its own works, in days.
const OWN_SINCE: &str = "story.own_since";
/// The fewest days between two of the harbour's own works it takes up:
/// left to itself, and while the player is making things too.
const OWN_GAP_ALONE: i64 = 60;
const OWN_GAP_WITH_HANDS: i64 = 30;

/// The own work the harbour has taken up next, if it has.
fn own_next(state: &world_core::WorldState) -> Option<&'static OwnWork> {
    match state.entity(STORY)?.component(OWN_NEXT)? {
        Value::Text(id) => OWN_WORKS.iter().find(|work| work.id == id),
        _ => None,
    }
}

/// The harbour's own works each thing the player makes by hand, or builds
/// on a plot, sets it thinking of: a bench made, and someone starts on a
/// storytelling chair. Things made and plot works count toward the
/// harbour's own ladder this way, so a maker's harbour goes its own way.
fn inspired_by(thing: &str) -> &'static [&'static str] {
    match thing {
        "bench" | "picnic_tables" | "study_hut" => &["own_story_chair", "own_lookout"],
        "lamp" | "lanterns" => &["own_jar_lanterns", "own_fire_pit"],
        "stall" | "picnic" | "crab_shack" => &["own_skittle_alley", "own_fire_pit"],
        "flagpole" | "harbour_flag" => &["own_weathervane", "own_flag_line"],
        "bunting" | "swing" | "hilltop_swing" => &["own_kite_hill", "own_rope_swing"],
        "vegetables" | "sheepfold" => &["own_hen_house", "own_herb_bed"],
        "apple_tree" | "windmill" => &["own_apple_press", "own_bee_hives"],
        "well" | "signpost" | "frog_pond" => &["own_tide_pools", "own_stepping_stones"],
        "fountain" | "statue" => &["own_quay_mosaic", "own_cairn"],
        "birdhouse" | "sunflowers" | "wildflowers" => &["own_bee_hives", "own_bird_table"],
        "postbox" | "bookshop" => &["own_book_box", "own_notice_board"],
        "rowboat" | "boathouse" | "slipway" | "boat_rack" => &["own_boat_planter", "own_net_rack"],
        "flowerboxes" | "herbs" => &["own_shell_path", "own_window_boxes"],
        "bandstand" | "pottery" | "gallery" => &["own_music_shed", "own_skittle_alley"],
        "net_store" | "sail_loft" | "ice_house" => &["own_net_rack", "own_bait_shed"],
        _ => &[],
    }
}

/// What the player has made by hand or built on plots, the latest first.
fn made_lately(world: &World) -> Vec<String> {
    let state = world.state();
    let mut made = world
        .events_of_kind(&[
            "built_by_hand",
            "decorated_by_hand",
            "planted_by_hand",
            "plot_finished",
        ])
        .into_iter()
        .filter_map(|event| {
            let thing = event
                .targets
                .first()
                .and_then(|made| state.entity(*made))
                .and_then(|made| match made.component("hands.thing") {
                    Some(Value::Text(thing)) => Some(thing.clone()),
                    _ => None,
                })?;
            Some((event.id, thing))
        })
        .collect::<Vec<_>>();
    made.sort_by_key(|(id, _)| std::cmp::Reverse(*id));
    made.into_iter().map(|(_, thing)| thing).collect()
}

/// The own work the harbour takes up next: one already begun; else one
/// what the player made or built most lately set it thinking of; else the
/// next not done, in turn.
fn next_own_work(world: &World) -> Option<&'static OwnWork> {
    let state = world.state();
    let deck = deck();
    let unfinished = |work: &&OwnWork| !storylets::finished(state, deck, work.id);
    let begun = OWN_WORKS
        .iter()
        .filter(unfinished)
        .find(|work| storylets::progress(state, deck, work.id) > 0);
    begun
        .or_else(|| {
            made_lately(world).iter().find_map(|thing| {
                inspired_by(thing).iter().find_map(|id| {
                    OWN_WORKS
                        .iter()
                        .filter(unfinished)
                        .find(|work| work.id == *id)
                })
            })
        })
        .or_else(|| OWN_WORKS.iter().find(unfinished))
}

/// The harbour takes up its next own work, once the one before is done and
/// it has been let down at all.
struct TakesUpOwnWork;

impl world_core::Action for TakesUpOwnWork {
    fn name(&self) -> &'static str {
        "own_work_taken_up"
    }

    fn evaluate(
        &self,
        state: &world_core::WorldState,
        request: &world_core::ActionRequest,
    ) -> Result<world_core::EventDraft, world_core::ActionError> {
        let deck = deck();
        let id = match request.args.get("work") {
            Some(Value::Text(id)) => id.as_str(),
            _ => return Err(world_core::ActionError::Invalid("missing work".into())),
        };
        let work = OWN_WORKS
            .iter()
            .find(|work| work.id == id)
            .ok_or_else(|| world_core::ActionError::Invalid(format!("no own work {id}")))?;
        if own_next(state).is_some_and(|next| !storylets::finished(state, deck, next.id))
            || storylets::finished(state, deck, work.id)
        {
            return Err(world_core::ActionError::Invalid(
                "the harbour has its own work in hand".into(),
            ));
        }
        // One at a time, and not too soon after the last: sooner while the
        // player is making things too.
        let now = storylets::period_index(state, deck) as i64;
        let since = state
            .entity(STORY)
            .and_then(|story| match story.component(OWN_SINCE) {
                Some(Value::Integer(at)) => Some(*at),
                _ => None,
            });
        let hands = storylets::Condition::MarkedWithin(LENT_MARK, LENT_LATELY);
        let gap = if storylets::holds(state, deck, &hands) {
            OWN_GAP_WITH_HANDS
        } else {
            OWN_GAP_ALONE
        };
        if since.is_some_and(|since| now - since < gap) {
            return Err(world_core::ActionError::Invalid(
                "the harbour started something lately".into(),
            ));
        }
        let mut draft = world_core::EventDraft::new("own_work_taken_up");
        draft.payload.insert("work".into(), work.id.into());
        draft.changes = vec![
            world_core::StateChange::SetComponent {
                entity: STORY,
                key: OWN_NEXT.into(),
                value: work.id.into(),
            },
            world_core::StateChange::SetComponent {
                entity: STORY,
                key: OWN_SINCE.into(),
                value: now.into(),
            },
        ];
        Ok(draft)
    }
}

/// Once the harbour has been let down at all and has no own work in hand,
/// it takes up its next.
pub(crate) fn take_up_own_work(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Option<EventId>, WorldError> {
    let state = world.state();
    let deck = deck();
    let let_down = state.entity(STORY).is_some_and(
        |story| matches!(story.component(INITIATIVE), Some(Value::Integer(at)) if *at >= 1),
    );
    if !let_down || own_next(state).is_some_and(|next| !storylets::finished(state, deck, next.id)) {
        return Ok(None);
    }
    let Some(work) = next_own_work(world) else {
        return Ok(None);
    };
    let request = world_core::ActionRequest::new("own_work_taken_up").arg("work", work.id);
    match world.execute(actions, &request) {
        Ok(event) => Ok(Some(event.id)),
        Err(WorldError::Action(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

/// What someone says when the player turns up to help with what they
/// are making.
const LENT_LINES: [&str; 6] = [
    "An extra pair of hands! Hold this end.",
    "You came to help? Grab a hammer.",
    "With you here we'll finish by dark.",
    "Mind your thumbs. Thanks for coming.",
    "Knew you'd turn up sooner or later.",
    "Another part done, thanks to you.",
];

/// The player, making something of their own, lends a hand with what the
/// harbour is making of its own accord: it goes up a part.
struct LendsAHand;

impl world_core::Action for LendsAHand {
    fn name(&self) -> &'static str {
        "lend_a_hand"
    }

    fn evaluate(
        &self,
        state: &world_core::WorldState,
        _request: &world_core::ActionRequest,
    ) -> Result<world_core::EventDraft, world_core::ActionError> {
        let deck = deck();
        let work = own_work_under_way(state)
            .ok_or_else(|| world_core::ActionError::Invalid("nothing under way".into()))?;
        let now = storylets::period_index(state, deck) as i64;
        let story = state.entity(STORY);
        let last = story.and_then(|story| match story.component(LENT) {
            Some(Value::Integer(at)) => Some(*at),
            _ => None,
        });
        if last.is_some_and(|last| now - last < LEND_EVERY) {
            return Err(world_core::ActionError::Invalid(
                "lent a hand lately".into(),
            ));
        }
        let done = storylets::progress(state, deck, work.id);
        let mut draft = world_core::EventDraft::new("hand_lent");
        draft.actor = Some(work.champion);
        draft.targets = vec![work.champion];
        draft.payload.insert("work".into(), work.id.into());
        draft.payload.insert(
            "told".into(),
            format!("You lent a hand with {}", lowered(work.label)).into(),
        );
        let said = LENT_LINES[(now as usize + done as usize) % LENT_LINES.len()];
        draft.payload.insert("said".into(), said.into());
        draft.changes = vec![
            world_core::StateChange::SetComponent {
                entity: STORY,
                key: format!("story.goal.{}", work.id),
                value: (done + 1).into(),
            },
            world_core::StateChange::SetComponent {
                entity: STORY,
                key: LENT.into(),
                value: now.into(),
            },
            world_core::StateChange::SetComponent {
                entity: STORY,
                key: format!("story.mark.{LENT_MARK}"),
                value: now.into(),
            },
        ];
        Ok(draft)
    }
}

/// The harbour sees the player here (answering someone, or making
/// something), when it has not lately: see [`LENT_MARK`].
pub(crate) fn player_seen(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Option<EventId>, WorldError> {
    storylets::mark_now(world, actions, LENT_MARK, SEEN_EVERY)
}

/// After the player makes something, they lend a hand with the harbour's
/// own work under way, if there is one and they have not lately.
pub(crate) fn lend_a_hand(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    let mut taken = take_up_own_work(world, actions)?
        .into_iter()
        .collect::<Vec<_>>();
    match world.execute(actions, &world_core::ActionRequest::new("lend_a_hand")) {
        Ok(event) => {
            taken.push(event.id);
            Ok(taken)
        }
        Err(WorldError::Action(_)) => {
            // Nothing to lend a hand with: the harbour still saw what the
            // player made.
            taken.extend(player_seen(world, actions)?);
            Ok(taken)
        }
        Err(error) => Err(error),
    }
}

/// Nobody stays at the very top or bottom for long: a day at either end of
/// the harbour's spirits eases them one step back.
struct SpiritsSettle;

impl world_core::Action for SpiritsSettle {
    fn name(&self) -> &'static str {
        "spirits_settle"
    }

    fn evaluate(
        &self,
        state: &world_core::WorldState,
        _request: &world_core::ActionRequest,
    ) -> Result<world_core::EventDraft, world_core::ActionError> {
        let mood = match state.entity(STORY).and_then(|story| story.component(MOOD)) {
            Some(Value::Integer(mood)) => *mood,
            _ => 0,
        };
        if mood.abs() < 5 {
            return Err(world_core::ActionError::Invalid(
                "the harbour's spirits are not at an end".into(),
            ));
        }
        let mut draft = world_core::EventDraft::new("spirits_settled");
        draft.changes.push(world_core::StateChange::SetComponent {
            entity: STORY,
            key: MOOD.into(),
            value: (mood - mood.signum()).into(),
        });
        Ok(draft)
    }
}

fn name_of(world: &World, id: EntityId) -> String {
    world
        .state()
        .entity(id)
        .map(world_projection::entity_title)
        .unwrap_or_else(|| "Someone".into())
}

pub(crate) fn named(world: &World, text: &str, who: EntityId) -> String {
    if text.contains("{name}") {
        text.replace("{name}", &name_of(world, who))
    } else {
        text.to_string()
    }
}

fn integer(world: &World, id: EntityId, key: &str) -> Option<i64> {
    match world.state().entity(id)?.component(key)? {
        Value::Integer(value) => Some(*value),
        _ => None,
    }
}

fn text(world: &World, id: EntityId, key: &str) -> Option<String> {
    match world.state().entity(id)?.component(key)? {
        Value::Text(value) => Some(value.clone()),
        _ => None,
    }
}

/// How the harbour feels, from -5 to 5.
/// The harbour's spirits, read straight from its state.
pub(crate) fn spirits_of(state: &world_core::WorldState) -> i64 {
    match state.entity(STORY).and_then(|story| story.component(MOOD)) {
        Some(Value::Integer(mood)) => *mood * 2,
        _ => 0,
    }
}

pub(crate) fn spirits(world: &World) -> i64 {
    integer(world, STORY, MOOD).unwrap_or(0)
}

/// The harbour's gauges that are stuck, or nearly, at one end: money in
/// town, and spirits.
fn pinned(world: &World) -> Vec<Pinned> {
    let mut pinned = Vec::new();
    if let Some(money) = crate::projection::gauges(world)
        .into_iter()
        .find(|gauge| gauge.id == "money")
    {
        if money.value >= 0.8 {
            pinned.push(Pinned {
                gauge: "money",
                high: true,
            });
        } else if money.value <= 0.2 {
            pinned.push(Pinned {
                gauge: "money",
                high: false,
            });
        }
    }
    match spirits(world) {
        4.. => pinned.push(Pinned {
            gauge: "spirits",
            high: true,
        }),
        ..=-4 => pinned.push(Pinned {
            gauge: "spirits",
            high: false,
        }),
        _ => {}
    }
    pinned
}

const SEASONS: [&str; 4] = ["spring", "summer", "autumn", "winter"];

/// Which season it is in the harbour.
pub(crate) fn season(world: &World) -> usize {
    storylets::season(storylets::period_index(world.state(), deck()), SEASON_DAYS) as usize
}

/// What a moment adds to the chapter it happened in, when that chapter
/// closes: the things a person would tell you about that stretch.
fn chapter_line(event: &Event) -> Option<&'static str> {
    Some(match event.kind.as_str() {
        "stall_opened" => "Sofia opened a stall of her own.",
        "sofia_stayed" => "Sofia stayed, and opened her stall after all.",
        "sofia_left" | "sofia_went_anyway" => "Sofia left for a job on the mainland.",
        "mia_at_the_stall" => "Mia started helping at Sofia's stall.",
        "music_monthly_began" => "The Anchor made its music night a monthly thing.",
        "ferry_run_began" => "Mara's bread went out on the morning ferry.",
        "pier_opened" | "pier_in_use" | "pier_opened_quietly" => {
            "The new pier was finished and opened."
        }
        "ivo_arrived" => "Ivo's fishing family came to live in the harbour.",
        "ada_arrived" => "Ada the traveller made the harbour her home.",
        "lamp_lit_together" | "lamp_lit_quietly" | "lamp_lit_anyway" => {
            "A lamp was lit on the point."
        }
        "garden_planted" => "The school planted a garden.",
        "school_roof_mended" | "school_roof_mended_at_last" => "The school roof was mended.",
        "school_moved_to_pub" => "Lessons moved into the pub.",
        "oven_bought" => "Mara got her new oven.",
        "regatta_won" => "Sea Finch won the regatta.",
        "fete_held" => "The harbour held a fête.",
        "quiz_night" => "The pub started a quiz night.",
        "bakery_closed" => "Harbour Bakery closed its doors.",
        "bakery_reopened" | "bakery_reopened_lean" => "Mara reopened the bakery.",
        "boat_sold" => "Jonas sold Sea Finch.",
        "boat_repaired" => "Sea Finch went back to sea.",
        _ => return None,
    })
}

/// How a chapter of the harbour's life ends: how its climax went, and
/// what happened in it that people will remember, in the order it
/// happened. A chapter that ends before anything worth telling falls back
/// on how the town stands.
fn chapter_ending(world: &World) -> (String, String) {
    let deck = deck();
    let (_, started) = storylets::chapter(world.state(), deck);
    // Events are recorded in time order, so the chapter's are the tail.
    let events = world.events();
    let lived = events[events.partition_point(|event| event.world_time < started)..]
        .iter()
        .collect::<Vec<_>>();
    let mut title = None;
    let mut lines = Vec::<String>::new();
    for event in &lived {
        let said = outcome_of(event).map(|(_, said)| said);
        if let Some(said) = said {
            if let Some(named_title) = said.title {
                title = Some(named_title.to_string());
            }
        }
        let line = said
            .and_then(|said| said.chapter)
            .or_else(|| chapter_line(event));
        if let Some(line) = line {
            let line = line.to_string();
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
    }
    let season_name = SEASONS[season(world)];
    let year = period_of(world) / YEAR + 1;
    // A chapter in which something was finished can be named for it.
    let finished = lived.iter().rev().find_map(|event| {
        let goal = match outcome_of(event) {
            Some((_, said)) => said.effects.iter().find_map(|effect| match effect {
                Effect::Advance(goal) => Some(*goal),
                _ => None,
            }),
            None if event.kind == "hand_lent" => match event.payload.get("work") {
                Some(Value::Text(work)) => OWN_WORKS
                    .iter()
                    .find(|own| own.id == work)
                    .map(|own| own.id),
                _ => None,
            },
            None => None,
        }?;
        storylets::finished(world.state(), deck, goal)
            .then(|| goal_label(goal))
            .flatten()
    });
    let named_for_work = finished.map(|label| format!("The {season_name} of {}", the(label)));
    let title = title.or(named_for_work).unwrap_or_else(|| {
        let feel = match spirits(world) {
            3.. => "A bright",
            1..=2 => "A good",
            -1..=0 => "A quiet",
            -3..=-2 => "A hard",
            _ => "A bitter",
        };
        format!("{feel} {season_name}")
    });
    // What changed between people this chapter: who became friends or
    // fell out, who got together, who came and who went.
    let news = lives::news_since(world, started);
    let mut candidates = vec![title.clone()];
    if let Some(first) = news.first() {
        candidates.push(format!("The {season_name} {}", lowered_start(first)));
    }
    candidates.push(storylets::title_with_year(&title, season_name, year));
    candidates.push(format!(
        "The {season_name} of year {}",
        storylets::number_word(year)
    ));
    let lived_days = period_of(world).saturating_sub(started / crate::persistence::WORLD_DAY_TICKS);
    let candidates = candidates
        .into_iter()
        .map(|title| storylets::fitted_title(title, lived_days, YEAR, season_name))
        .collect::<Vec<_>>();
    let title = storylets::unused_title(world, &candidates);
    // The last three things worth telling, with the climax among them, and
    // two of what changed between people.
    let mut summary = if lines.len() > 3 {
        lines.split_off(lines.len() - 3)
    } else {
        lines
    };
    for line in news.iter().rev().take(2).rev() {
        summary.push(format!("{line}."));
    }
    if summary.is_empty() {
        summary.push(
            if text(world, BAKERY, OPERATING_STATUS).as_deref() == Some("closed") {
                "The bakery stayed shut.".to_string()
            } else {
                "The harbour kept its bakery.".to_string()
            },
        );
    }
    // Whoever was let down this chapter, and has not forgotten.
    let mut let_down = std::collections::BTreeMap::<EntityId, usize>::new();
    for event in &lived {
        let Some((spec, said)) = outcome_of(event) else {
            continue;
        };
        let refused = spec
            .answers
            .iter()
            .any(|answer| std::ptr::eq(&answer.said, said) && answer.refuses);
        let lapsed = std::ptr::eq(&spec.lapse, said);
        if spec.storylet.want && (refused || lapsed) {
            *let_down.entry(spec.storylet.asker).or_default() += 1;
        }
    }
    let sore = let_down
        .into_iter()
        .max_by_key(|(who, count)| (*count, std::cmp::Reverse(*who)))
        .map(|(who, _)| who);
    // What the player made this chapter is part of how it is told.
    if let Some(made) = hands::latest_made_since(world, started) {
        summary.push(format!("{made}."));
    }
    let mut summary = summary.join(" ");
    if let Some(who) = sore {
        summary.push_str(&format!(
            " {} hasn't forgotten being let down.",
            name_of(world, who)
        ));
    }
    (title, summary)
}

fn period_of(world: &World) -> u64 {
    world.world_time() / crate::persistence::WORLD_DAY_TICKS
}

/// A sentence as it reads after "The autumn": "Mara and Leo became
/// friends" stays as it is, "The whole harbour came" turns to "the whole
/// harbour came".
fn lowered_start(sentence: &str) -> String {
    match sentence.split_once(' ') {
        Some((first, rest)) if matches!(first, "The" | "A" | "An") => {
            format!("{} {rest}", first.to_lowercase())
        }
        _ => sentence.to_string(),
    }
}

/// The turning point of each chapter: one question near its end, on what
/// the chapter has been about.
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
            "great_storm",
            climax(NOAH, "storm_season"),
            (
                "The great storm is coming",
                "The biggest storm in years is coming. What do we save?",
            ),
            vec![
                yes(
                    "boats",
                    "Haul every boat up",
                    "Sea Finch and every boat come up the slip. Noah pays 30 for rope and rollers.",
                    vec![has(NOAH, 30)],
                    said(
                        "great_storm_boats_saved",
                        "The harbour hauled up every boat before the great storm",
                        "Not one boat lost!",
                        spend(NOAH, 30).into_iter().chain([mood(1)]),
                    )
                    .chapter("When the great storm came, the harbour saved its boats.")
                    .titled("The year of the great storm"),
                ),
                other(
                    "board_up",
                    "Board up the town",
                    "Evan boards every window; Noah pays him 30.",
                    said(
                        "great_storm_boarded_up",
                        "The harbour boarded up against the great storm",
                        "Hammers all night, and it held.",
                        pay(NOAH, EVAN, 30).into_iter().chain([mood(1)]),
                    )
                    .chapter("When the great storm came, the harbour boarded up and held.")
                    .titled("The storm we boarded up against"),
                ),
                // Only a harbour that weathered one before knows the drill.
                yes(
                    "ready",
                    "We know the drill",
                    "Everyone did this last time. Nothing spent.",
                    vec![Condition::RaisedBefore("great_storm")],
                    said(
                        "great_storm_weathered_again",
                        "The harbour knew what to do, having weathered a great storm before",
                        "Same as last time. Boats up, shutters down.",
                        [mood(2)],
                    )
                    .chapter("The harbour met the great storm like old hands.")
                    .titled("The storm we were ready for"),
                ),
            ],
            said(
                "great_storm_caught_us",
                "The great storm caught the harbour unready",
                "Nobody was ready for that.",
                spend(NOAH, 60).into_iter().chain([mood(-2)]),
            )
            .chapter("The great storm caught the harbour unready.")
            .titled("The storm that caught us"),
        ),
        spec(
            "town_meeting",
            climax(NOAH, "hard_times"),
            (
                "Noah called a town meeting",
                "Money's tight everywhere. How do we get through?",
            ),
            vec![
                yes(
                    "share",
                    "Share what we have",
                    "Leo and Noah put 20 each toward whoever is short.",
                    vec![has(LEO, 20), has(NOAH, 20)],
                    said(
                        "hard_times_shared",
                        "The harbour agreed to share what it had",
                        "Nobody goes without. Agreed?",
                        [pay(LEO, JONAS, 20), pay(NOAH, SOFIA, 20)]
                            .into_iter()
                            .flatten()
                            .chain([mood(2)]),
                    )
                    .chapter("When times were hard, the harbour shared what it had.")
                    .titled("The lean season we shared"),
                ),
                other(
                    "own",
                    "Everyone for themselves",
                    "Every house looks after its own.",
                    said(
                        "hard_times_alone",
                        "The harbour agreed every house would look after its own",
                        "Fair enough. We all have our own to feed.",
                        [mood(-2)],
                    )
                    .chapter("When times were hard, everyone looked after their own.")
                    .titled("Every house for itself"),
                ),
            ],
            said(
                "meeting_empty",
                "Nobody came to Noah's meeting",
                "An empty hall. Well.",
                [mood(-1)],
            )
            .chapter("When times were hard, nobody came to the meeting.")
            .titled("The lean season"),
        ),
        spec(
            "inspector",
            climax(EMMA, "inspector"),
            (
                "The mainland inspector is coming",
                "The inspector comes tomorrow. How do we show the school?",
            ),
            vec![
                yes(
                    "show",
                    "Put on a show",
                    "Noah pays 40 for paint and a new flag.",
                    vec![has(NOAH, 40)],
                    said(
                        "inspector_impressed",
                        "The inspector found the school at its best",
                        "Top marks! Well, nearly.",
                        spend(NOAH, 40).into_iter().chain([mood(1)]),
                    )
                    .chapter("The inspector found the school at its best.")
                    .titled("The inspector's visit"),
                ),
                other(
                    "as_is",
                    "As we are",
                    "No fuss, no paint.",
                    said(
                        "inspector_saw_it_as_is",
                        "The inspector saw the school just as it is",
                        "Honest, at least.",
                        [],
                    )
                    .chapter("The inspector saw the school just as it is.")
                    .titled("The school as it is"),
                ),
            ],
            said(
                "inspector_unannounced",
                "The inspector came before anyone was ready",
                "Today? I thought it was tomorrow!",
                [mood(-1)],
            )
            .chapter("The inspector caught the school unawares.")
            .titled("The unexpected inspector"),
        ),
    ]
}

/// One day of the storyteller, at the end of the harbour's day.
pub(crate) fn tick(
    world: &mut World,
    actions: &ActionRegistry,
    away: bool,
) -> Result<Vec<EventId>, WorldError> {
    let money = crate::projection::gauges(world)
        .into_iter()
        .find(|gauge| gauge.id == "money")
        .map_or(0.5, |gauge| gauge.value);
    // Spirits at an end are a turning point, even as they ease a step.
    let at_end = spirits(world).abs() >= 5 || !(0.02..=0.98).contains(&money);
    let mut settled = Vec::new();
    if spirits(world).abs() >= 5 {
        settled.push(
            world
                .execute(actions, &world_core::ActionRequest::new("spirits_settle"))?
                .id,
        );
    }
    let reading = Reading {
        pinned: pinned(world),
        away,
        at_end,
        chapter_ending: Box::new(chapter_ending),
        hold: waiting_for_the_player(world),
    };
    let mut events = settled;
    let begins = world.state().entity(STORY).is_none();
    // What the year brings comes first, so the day's round of lives knows
    // whether the day has already brought something new.
    events.extend(crate::years::tick(world, actions)?);
    // Whoever what the player built draws comes before the day's round, so
    // a room come free goes to them before a stranger at the door.
    events.extend(crate::plots::draw(world, actions)?);
    let cast = crate::life::cast_in(world.state());
    events.extend(lives::tick_with(
        world,
        actions,
        &cast,
        away,
        reading.hold,
        &crate::firsts::quiet_days(),
    )?);
    let kit = crate::handwork::kit(world.state());
    events.extend(hands::tick(world, actions, &kit)?);
    let almanac = crate::almanac::almanac(world.state());
    events.extend(calendar::tick(world, actions, &almanac)?);
    events.extend(take_up_own_work(world, actions)?);
    let mut told = storylets::tick(world, actions, deck(), &reading)?;
    told.extend(storylets::bring_forward(world, actions, deck(), &reading)?);
    events.extend(mementos(world, actions, &told)?);
    events.extend(told);
    // A harbour whose story begins now opens its plots over the years.
    if begins {
        let kit = crate::handwork::kit(world.state());
        events.extend(hands::stage_plots(world, actions, &kit)?);
    }
    // Last of all, someone may ask the player a favour talk can do.
    events.extend(crate::speech::favour_asked(world, actions, away)?);
    Ok(events)
}

/// Whether a new harbour is still waiting for the player's first deed
/// before anyone asks them anything: nothing has come up yet, the player
/// has neither made, given nor said anything, and its first day has not
/// passed.
fn waiting_for_the_player(world: &World) -> bool {
    let deck = deck();
    let state = world.state();
    if storylets::anything_raised(state, deck) {
        return false;
    }
    let acted = state.entity(crate::handwork::kit(state).notes).is_some()
        || world.events().iter().any(conversation::is_talk);
    let (_, started) = storylets::chapter(state, deck);
    let first_day = state.entity(deck.story).is_none()
        || storylets::period_index(state, deck) <= started / deck.period.max(1);
    !acted && first_day
}

/// Once the player has done something of their own in a new harbour, the
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

/// The first question, if nothing has been asked yet: a new harbour opens
/// on it, just after the hello, so the player has something to answer
/// from the start.
pub(crate) fn first_question(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    if storylets::anything_raised(world.state(), deck()) {
        return Ok(Vec::new());
    }
    let money = crate::projection::gauges(world)
        .into_iter()
        .find(|gauge| gauge.id == "money")
        .map_or(0.5, |gauge| gauge.value);
    let reading = Reading {
        pinned: pinned(world),
        away: false,
        at_end: spirits(world).abs() >= 5 || !(0.02..=0.98).contains(&money),
        chapter_ending: Box::new(chapter_ending),
        hold: false,
    };
    storylets::tick(world, actions, deck(), &reading)
}

fn command_id(storylet: &str, choice: &str) -> String {
    format!("{STORY_COMMAND}{storylet}.{choice}")
}

/// The choice a command makes, if it is one of the storyteller's.
pub(crate) fn parse_command(command_id: &str) -> Option<(&str, &str)> {
    command_id.strip_prefix(STORY_COMMAND)?.split_once('.')
}

/// The kinds of Event a storylet's moments are recorded as: coming up,
/// each answer's and letting it go.
pub(crate) fn storylet_kinds() -> impl Iterator<Item = &'static str> {
    specs().iter().flat_map(|spec| {
        std::iter::once("situation_arose")
            .chain(spec.answers.iter().map(|answer| answer.said.event))
            .chain(std::iter::once(spec.lapse.event))
            .chain(spec.taken_up.iter().map(|taken| taken.event))
    })
}

fn find(storylet: &str) -> Option<&'static Spec> {
    specs().iter().find(|spec| spec.storylet.id == storylet)
}

/// A card for every answer that can be given now, each asked by whoever
/// the storylet belongs to.
pub(crate) fn commands(world: &World) -> Vec<world_projection::ProjectionCommand> {
    let mut commands = storylet_commands(world);
    commands.extend(crate::life::commands(world));
    commands
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
    let deck = deck();
    let id = spec.storylet.id;
    named(
        world,
        &asked(
            spec,
            storylets::times_raised(world.state(), deck, id),
            storylets::last_outcome(world.state(), deck, id),
        ),
        spec.storylet.asker,
    )
}

fn storylet_commands(world: &World) -> Vec<world_projection::ProjectionCommand> {
    let deck = deck();
    storylets::answers(world.state(), deck)
        .into_iter()
        // An answer only a harbour that remembers the last time can give is
        // not shown the first time.
        .filter(|(_, _, unmet)| {
            !unmet
                .iter()
                .any(|condition| matches!(condition, Condition::RaisedBefore(_)))
        })
        .filter_map(|(storylet, choice, unmet)| {
            let spec = find(storylet.id)?;
            let answer = spec.answers.iter().find(|answer| answer.id == choice.id)?;
            Some(world_projection::ProjectionCommand {
                id: command_id(storylet.id, choice.id),
                title: named(world, answer.title, storylet.asker),
                detail: named(world, answer.detail, storylet.asker),
                effects: Vec::new(),
                scenery: None,
                asker: Some(world_projection::SelectionId::Entity(storylet.asker)),
                moves: Vec::new(),
                question: Some(world_projection::Question {
                    id: storylet.id.into(),
                    prompt: asking(world, spec),
                }),
                unavailable: unmet.first().map(|condition| why_not(world, condition)),
                hand: None,
                preview: None,
                role: None,
            })
        })
        .collect()
}

/// Why an answer cannot be given now, in the harbour's words.
fn why_not(world: &World, condition: &Condition) -> String {
    match condition {
        Condition::AtLeast(who, key, amount) if *key == CASH => {
            format!("{} hasn't {amount} to spare", name_of(world, *who))
        }
        _ => "Not possible right now".into(),
    }
}

/// What someone would ask for now, if they have a want open: what they
/// say, and the answer that grants it, if it can be given.
pub(crate) fn wanting(world: &World, who: EntityId) -> Option<(String, Option<String>)> {
    let deck = deck();
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

/// How many wants someone has had granted, and turned down or let lapse.
pub(crate) fn kindness(world: &World, who: EntityId) -> (i64, i64) {
    storylets::kindness(world.state(), deck(), who)
}

fn storylet_of(event: &Event) -> Option<&'static Spec> {
    match event.payload.get("storylet")? {
        Value::Text(id) => find(id),
        _ => None,
    }
}

/// How one of the storyteller's questions was settled, when `event`
/// settles one: the words of the answer the player chose, or `None` inside
/// when nobody answered in time. For legends: "because you said …".
pub(crate) fn answer_words(event: &Event) -> Option<Option<&'static str>> {
    let (spec, said) = outcome_of(event)?;
    if std::ptr::eq(said, &spec.lapse) {
        return Some(None);
    }
    let answer = spec
        .answers
        .iter()
        .find(|answer| std::ptr::eq(&answer.said, said))?;
    Some(Some(answer.title))
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

/// The ids of these storylets' outcomes: the Events their answers and
/// lapses are recorded as.
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

fn outcome_of(event: &Event) -> Option<(&'static Spec, &'static Said)> {
    let spec = storylet_of(event)?;
    if event.kind == "situation_arose" {
        return None;
    }
    if spec.lapse.event == event.kind {
        return Some((spec, &spec.lapse));
    }
    if let Some(taken) = spec
        .taken_up
        .as_ref()
        .filter(|taken| taken.event == event.kind)
    {
        return Some((spec, taken));
    }
    let said = spec
        .answers
        .iter()
        .map(|answer| &answer.said)
        .find(|said| said.event == event.kind)?;
    Some((spec, said))
}

/// Whether an Event is one of the storyteller's small moments: something
/// coming up, answered or let go. A chapter closing is not small.
pub(crate) fn is_storylet(event: &Event) -> bool {
    storylet_of(event).is_some() || lives::is_news(event)
}

/// How the harbour tells one of the storyteller's moments.
pub(crate) fn told(world: &World, event: &Event) -> Option<String> {
    if event.kind == "hand_lent" {
        return match event.payload.get("told") {
            Some(Value::Text(told)) => Some(told.clone()),
            _ => None,
        };
    }
    if event.kind == "chapter_ended" {
        let title = match event.payload.get("title") {
            Some(Value::Text(title)) => title.clone(),
            _ => return None,
        };
        return Some(format!("{title} came to an end"));
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
    let who = spec.storylet.asker;
    if event.kind == "situation_arose" {
        // Told against the times it came before: a storm again, and once
        // more after that.
        let told = named(world, spec.told, who);
        return Some(match event.payload.get("times") {
            // "…going again" is not told "again again".
            Some(Value::Integer(2)) if told.ends_with(" again") => told,
            Some(Value::Integer(2)) => format!("{told} again"),
            Some(Value::Integer(times)) if *times > 2 => format!("{told} once more"),
            _ => told,
        });
    }
    let (_, said) = outcome_of(event)?;
    let told = named(world, said.told, who);
    Some(match event.payload.get("by") {
        Some(Value::Entity(by)) if told.contains(HELPER) => {
            told.replace(HELPER, &name_of(world, *by))
        }
        _ => told,
    })
}

/// What the asker says at one of the storyteller's moments.
pub(crate) fn line(event: &Event) -> Option<(EntityId, String)> {
    if event.kind == "hand_lent" {
        return match (event.actor, event.payload.get("said")) {
            (Some(who), Some(Value::Text(said))) => Some((who, said.clone())),
            _ => None,
        };
    }
    if lives::is_life(event) {
        return lives::said(event);
    }
    if calendar::is_calendar(event) {
        return calendar::said(event);
    }
    if hands::is_hands(event) {
        return hands::said(event);
    }
    let spec = storylet_of(event)?;
    let who = spec.storylet.asker;
    if event.kind == "situation_arose" {
        let times = match event.payload.get("times") {
            Some(Value::Integer(times)) => *times,
            _ => 1,
        };
        let last = match event.payload.get("last") {
            Some(Value::Text(last)) => Some(last.as_str()),
            _ => None,
        };
        return Some((who, asked(spec, times, last)));
    }
    let (_, said) = outcome_of(event)?;
    Some((who, said.line.to_string()))
}

/// Whether one of the storyteller's moments went well for the harbour.
pub(crate) fn tone(event: &Event) -> Option<world_projection::Tone> {
    use world_projection::Tone;
    if event.kind == "chapter_ended" {
        return Some(Tone::Neutral);
    }
    if event.kind == "hand_lent" {
        return Some(Tone::Good);
    }
    let spec = storylet_of(event)?;
    if event.kind == "situation_arose" {
        return Some(if spec.storylet.want {
            Tone::Neutral
        } else {
            Tone::Warning
        });
    }
    let (_, said) = outcome_of(event)?;
    let moved = said
        .effects
        .iter()
        .map(|effect| match effect {
            Effect::Add { key, by, .. } if *key == MOOD => *by,
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

/// What someone says about how the player answered them, with `{ago}`
/// where when it was goes: the answer they were given, and what came of
/// it.
pub(crate) fn recalled(event: &Event, who: EntityId) -> Option<String> {
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
        (true, _) => "I haven't forgotten.",
        (false, Some(remembered)) => remembered,
        (false, None) => "Thank you for that.",
    };
    Some(format!(
        "When I asked you {{ago}}, you said “{}”. {after}",
        answer.title.trim_end_matches('.')
    ))
}

/// What someone still says, the day after something went their
/// way: the newest such moment of theirs.
pub(crate) fn remembered(world: &World, who: EntityId) -> Option<&'static str> {
    let now = world.world_time();
    let since = now.saturating_sub(crate::persistence::WORLD_DAY_TICKS);
    world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time >= since)
        .filter(|event| event.world_time < now && event.actor == Some(who))
        .find_map(|event| outcome_of(event).and_then(|(_, said)| said.remembered))
}

/// The harbour's standing goals, for the horizon: the new pier and the
/// lamp on the point.
pub(crate) fn goals(world: &World) -> Vec<world_projection::Goal> {
    let deck = deck();
    let goal = |id: &str, label: &str, shape| {
        let parts = deck.goals.iter().find(|goal| goal.id == id)?.parts;
        Some(world_projection::Goal {
            id: id.into(),
            label: label.into(),
            shape,
            done: storylets::progress(world.state(), deck, id).clamp(0, parts) as u32,
            parts: parts as u32,
        })
    };
    let mut goals = [
        ("pier", "The new pier", world_projection::MarkShape::Bridge),
        (
            "lamp",
            "A lamp on the point",
            world_projection::MarkShape::Lamp,
        ),
    ]
    .into_iter()
    .filter_map(|(id, label, shape)| goal(id, label, shape))
    .collect::<Vec<_>>();
    // The works done so far, and the one in hand; what comes after is
    // not known yet.
    if goals.iter().all(|goal| goal.done >= goal.parts) {
        for rung in ladder() {
            let Some(next) = goal(rung.id, rung.label, rung.work.shape) else {
                continue;
            };
            let finished = next.done >= next.parts;
            if !finished {
                // What a World finished on the ladder as it used to be.
                goals.extend(
                    retired_rungs()
                        .iter()
                        .filter_map(|rung| goal(rung.id, rung.label, rung.work.shape))
                        .filter(|goal| goal.done >= goal.parts),
                );
            }
            goals.push(next);
            if !finished {
                break;
            }
        }
    }
    // What the harbour made of its own accord: those done, and the one
    // under way.
    let own = OWN_WORKS
        .iter()
        .filter_map(|work| goal(work.id, work.label, work.shape))
        .filter(|own| own.done > 0)
        .collect::<Vec<_>>();
    let (done, under_way): (Vec<_>, Vec<_>) =
        own.into_iter().partition(|own| own.done >= own.parts);
    goals.extend(done);
    goals.extend(under_way);
    goals
}

/// What a goal is called, if it is one of the harbour's.
pub(crate) fn goal_label(goal: &str) -> Option<&'static str> {
    match goal {
        "pier" => Some("The new pier"),
        "lamp" => Some("A lamp on the point"),
        _ => ladder()
            .iter()
            .chain(retired_rungs())
            .find(|rung| rung.id == goal)
            .map(|rung| rung.label)
            .or_else(|| {
                OWN_WORKS
                    .iter()
                    .find(|work| work.id == goal)
                    .map(|work| work.label)
            }),
    }
}

/// What a gathering the player made happen is called, if an answer made
/// one: a music night, a feast, a party.
fn gathering(world: &World, event: &Event) -> Option<String> {
    let kind = event.kind.trim_end_matches("_together");
    Some(match kind {
        "music_night_held" => "the music night".into(),
        "harvest_feast" => "the harvest feast".into(),
        "fete_held" => "the fête".into(),
        "quiz_night" => "the quiz night".into(),
        "birthday_party" => {
            let spec = storylet_of(event)?;
            format!("{}'s birthday party", name_of(world, spec.storylet.asker))
        }
        _ => {
            let spec = storylet_of(event)?;
            let rung = spec.storylet.id.strip_prefix("open_")?;
            if kind != format!("{rung}_opened") {
                return None;
            }
            format!("the opening of {}", the(goal_label(rung)?))
        }
    })
}

/// At a gathering an answer made happen, two people fond of each other
/// may get together.
pub(crate) fn gathered(
    world: &mut World,
    actions: &ActionRegistry,
    answered: EventId,
) -> Result<Vec<EventId>, WorldError> {
    let Some(at) = world
        .event(answered)
        .and_then(|event| gathering(world, event))
    else {
        return Ok(Vec::new());
    };
    Ok(lives::match_at(world, actions, &at)?.into_iter().collect())
}

/// Who a goal belongs to: whoever asked for it.
fn goal_champion(goal: &str) -> Option<EntityId> {
    match goal {
        "pier" => Some(EVAN),
        "lamp" => Some(NOAH),
        _ => ladder()
            .iter()
            .chain(retired_rungs())
            .find(|rung| rung.id == goal)
            .map(|rung| rung.work.champion)
            .or_else(|| {
                OWN_WORKS
                    .iter()
                    .find(|work| work.id == goal)
                    .map(|work| work.champion)
            }),
    }
}

/// What someone gives the player to keep from a work just finished.
const MEMENTOS: [&str; 8] = [
    "a splinter from {x}",
    "a spare nail from {x}",
    "a sketch of {x}",
    "an offcut from {x}",
    "a chip of paint from {x}",
    "the first shaving from {x}",
    "a pebble from under {x}",
    "a scrap of the plans for {x}",
];

/// What they say as they give it.
const MEMENTO_SAID: [&str; 4] = [
    "It's done. Keep a bit of it.",
    "Something to remember the day it was finished.",
    "We made it together. This is yours.",
    "Every time you hold it, think of the day it was done.",
];

/// The goal an event finished, if it finished one.
fn finished_by(event: &Event) -> Option<&'static str> {
    let deck = deck();
    let goal = match outcome_of(event) {
        Some((_, said)) => said.effects.iter().find_map(|effect| match effect {
            Effect::Advance(goal) => Some(*goal),
            _ => None,
        }),
        None if event.kind == "hand_lent" => match event.payload.get("work") {
            Some(Value::Text(work)) => OWN_WORKS
                .iter()
                .find(|own| own.id == work)
                .map(|own| own.id),
            _ => None,
        },
        None => None,
    }?;
    // Finished by this event: it set the goal to its last part.
    let parts = deck.goals.iter().find(|spec| spec.id == goal)?.parts;
    let changed_to = event.changes.iter().find_map(|change| match change {
        world_core::StateChange::SetComponent {
            key,
            value: Value::Integer(done),
            ..
        } if *key == format!("story.goal.{goal}") => Some(*done),
        _ => None,
    })?;
    (changed_to == parts).then_some(goal)
}

/// When a work is finished, whoever asked for it gives the player
/// something of it to keep, if the week has room.
pub(crate) fn mementos(
    world: &mut World,
    actions: &ActionRegistry,
    events: &[EventId],
) -> Result<Vec<EventId>, WorldError> {
    let finished = events
        .iter()
        .filter_map(|id| world.event(*id))
        .filter_map(finished_by)
        .collect::<Vec<_>>();
    let mut given = Vec::new();
    for goal in finished {
        let (Some(label), Some(who)) = (goal_label(goal), goal_champion(goal)) else {
            continue;
        };
        if lives::gone(world.state(), who) || world.state().entity(who).is_none() {
            continue;
        }
        let at = text_hash(goal) as usize;
        let what = MEMENTOS[at % MEMENTOS.len()].replace("{x}", &the(label));
        let said = MEMENTO_SAID[(at / 7) % MEMENTO_SAID.len()];
        let cast = crate::life::cast_in(world.state());
        given.extend(lives::give_keepsake(
            world, actions, &cast, who, &what, said,
        )?);
    }
    Ok(given)
}

/// Whether a goal is one of the harbour's own works.
#[cfg(test)]
pub(crate) fn is_own_work(goal: &str) -> bool {
    OWN_WORKS.iter().any(|work| work.id == goal)
}

/// The chapters of the harbour's story that have ended.
pub(crate) use storylets::chapters_shown as chapters;

// What answers leave behind.
//
// Things the harbour builds or puts up stand on the scene as fixtures,
// people who arrive are residents like any other, and what happened is
// marked so that later questions can follow from it.

pub(crate) const STALL: EntityId = EntityId::new(410);
pub(crate) const BUNTING: EntityId = EntityId::new(411);
pub(crate) const PIER: EntityId = EntityId::new(412);
pub(crate) const LAMP: EntityId = EntityId::new(413);
pub(crate) const CRATES: EntityId = EntityId::new(414);
pub(crate) const GARDEN: EntityId = EntityId::new(415);
pub(crate) const LANTERNS: EntityId = EntityId::new(416);
pub(crate) const PENNANT: EntityId = EntityId::new(417);
pub(crate) const PARCELS: EntityId = EntityId::new(418);
pub(crate) const NETS: EntityId = EntityId::new(419);
pub(crate) const JAM: EntityId = EntityId::new(420);
pub(crate) const SHELF: EntityId = EntityId::new(421);
pub(crate) const BENCHES: EntityId = EntityId::new(422);
pub(crate) const BUCKET: EntityId = EntityId::new(423);
/// People who can come to live in the harbour.
pub(crate) const ADA: EntityId = EntityId::new(9);
pub(crate) const IVO: EntityId = EntityId::new(10);
/// Where someone is when they have left the harbour.
pub(crate) const AWAY: &str = "away";

fn newcomer(entity: EntityId, name: &'static str, job: &'static str, at: EntityId) -> Effect {
    Effect::Arrive {
        entity,
        kind: "resident",
        components: vec![
            ("name", Value::from(name)),
            (CASH, Value::from(30_i64)),
            (JOB, Value::from(job)),
            ("location", Value::Entity(at)),
            (crate::model::MISSED_SHIFTS, Value::from(0_i64)),
            ("newcomer", Value::from(true)),
        ],
    }
}

/// What each answer (or lapse), by the Event it is recorded as, leaves in
/// the harbour besides what it costs.
fn leaves_behind(event: &str) -> Vec<Effect> {
    use crate::{HARBOR, PUB, SCHOOL};
    match event {
        "school_roof_mended" | "school_roof_mended_at_last" => vec![
            mark("roof_done"),
            Effect::Demolish(BUCKET),
            build(
                PARCELS,
                "Slates for the school roof",
                "parcel",
                SCHOOL,
                Some(2),
            ),
        ],
        "school_roof_left" => vec![
            mark("roof_refused"),
            build(
                BUCKET,
                "A bucket under the school leak",
                "parcel",
                SCHOOL,
                None,
            ),
        ],
        "school_moved_to_pub" => vec![build(
            BUCKET,
            "School desks outside the pub",
            "parcel",
            PUB,
            Some(4),
        )],
        "lamp_glass_left" => vec![build(
            PARCELS,
            "The lamp glass, waiting at the dock",
            "parcel",
            HARBOR,
            Some(3),
        )],
        "pier_piles_left" | "pier_put_off" => vec![build(
            PARCELS,
            "Pier timber, waiting on the quay",
            "parcel",
            HARBOR,
            Some(3),
        )],
        "sofia_letter_pinned" => vec![build(
            PARCELS,
            "Sofia's letter on the noticeboard",
            "parcel",
            HARBOR,
            Some(4),
        )],
        "stall_kept_small" | "pub_stayed_quiet" => vec![build(
            JAM,
            "A quiet night at the pub",
            "lantern",
            PUB,
            Some(1),
        )],
        "books_shared" => vec![mark("books_shared")],
        "music_night_called_off" => vec![mark("music_refused")],
        "fever_slept_off" => vec![mark("fever_rest")],
        "fete_skipped" => vec![mark("fete_skipped")],
        "music_night_held" => vec![
            build(BUNTING, "Bunting over the pub", "bunting", PUB, Some(3)),
            mark("music"),
        ],
        "oven_bought" => vec![
            mark("oven"),
            build(
                PARCELS,
                "The new oven, just delivered",
                "parcel",
                BAKERY,
                Some(2),
            ),
        ],
        "nets_bought" => vec![
            mark("nets"),
            build(
                NETS,
                "New nets drying on the quay",
                "bunting",
                HARBOR,
                Some(3),
            ),
        ],
        "pier_section_built" => vec![build(
            PIER,
            "The new pier, a new section on",
            "pier",
            HARBOR,
            None,
        )],
        "pier_piles_driven" => vec![build(
            PIER,
            "The new pier, on its piles",
            "pier",
            HARBOR,
            None,
        )],
        "stall_opened" => vec![
            build(STALL, "Sofia's stall", "stall", PUB, None),
            mark("stall"),
        ],
        "stall_refused" => vec![mark("stall_refused")],
        "books_bought" => vec![
            mark("books"),
            build(PARCELS, "A box of new books", "parcel", SCHOOL, Some(2)),
        ],
        "lamp_work_done" | "lamp_glass_fetched" => vec![build(
            LAMP,
            "The lamp post on the point, unlit",
            "lantern",
            HARBOR,
            None,
        )],
        "traveller_stayed" => vec![
            mark("traveller"),
            build(PARCELS, "The traveller's bags", "parcel", PUB, Some(2)),
        ],
        "fete_held" => vec![build(
            BUNTING,
            "Bunting for the fête",
            "bunting",
            HARBOR,
            Some(4),
        )],
        "regatta_won" => vec![build(
            PENNANT,
            "The regatta pennant",
            "flag",
            HARBOR,
            Some(8),
        )],
        "birthday_party" => vec![build(BUNTING, "Birthday bunting", "bunting", PUB, Some(2))],
        "harvest_feast" => vec![build(
            LANTERNS,
            "Lanterns from the harvest supper",
            "lantern",
            PUB,
            Some(5),
        )],
        "garden_planted" => vec![
            build(GARDEN, "The school garden", "garden", SCHOOL, None),
            mark("garden"),
        ],
        "oven_patched" => vec![
            mark("oven_patched"),
            build(
                PARCELS,
                "Wire and patches for the old oven",
                "parcel",
                BAKERY,
                Some(2),
            ),
        ],
        "nets_left_torn" => vec![
            mark("nets_torn"),
            build(NETS, "Torn nets on the quay", "bunting", HARBOR, Some(3)),
        ],
        "boats_hauled_up" => vec![
            mark("boats_up"),
            build(PARCELS, "Boats up on the slip", "parcel", HARBOR, Some(2)),
        ],
        "storm_ridden_out" | "storm_caught_the_harbour" => vec![
            mark("storm_hit"),
            build(
                PARCELS,
                "Storm wreckage on the quay",
                "parcel",
                HARBOR,
                Some(3),
            ),
        ],
        "doctor_came" => vec![
            mark("doctor"),
            build(PARCELS, "The doctor's trunk", "parcel", SCHOOL, Some(2)),
        ],
        "hotel_order_baked" => vec![
            mark("hotel"),
            build(PARCELS, "Bread for the hotel", "parcel", BAKERY, Some(2)),
        ],
        "quarrel_settled_for_leo" | "quarrel_settled_for_evan" => vec![mark("quarrel")],
        "chimney_rebuilt" => vec![
            mark("chimney"),
            build(
                PARCELS,
                "Bricks for the pub chimney",
                "parcel",
                PUB,
                Some(2),
            ),
        ],
        "evan_stayed_home" => vec![mark("evan_stayed")],
        "chimney_patched" => vec![
            mark("chimney_patched"),
            build(
                PARCELS,
                "Bricks for the pub chimney",
                "parcel",
                PUB,
                Some(2),
            ),
        ],
        "mackerel_sold" => vec![build(
            PARCELS,
            "Crates of mackerel",
            "parcel",
            HARBOR,
            Some(2),
        )],
        "mackerel_shared" => vec![build(
            NETS,
            "Mackerel smoking on lines",
            "bunting",
            HARBOR,
            Some(2),
        )],
        "evan_worked_away" => vec![
            mark("evan_away"),
            Effect::Set {
                entity: EVAN,
                key: AWAY,
                text: "mainland",
            },
        ],
        "market_jam_sold" => vec![build(
            JAM,
            "A jam table on market day",
            "stall",
            PUB,
            Some(1),
        )],
        "market_lunch" | "harvest_potluck" => vec![build(
            BUNTING,
            "Bunting for the market",
            "bunting",
            HARBOR,
            Some(1),
        )],
        "regatta_watched" => vec![build(PENNANT, "A regatta flag", "flag", HARBOR, Some(2))],
        "school_stove_lit" => vec![build(
            PARCELS,
            "Coal for the school",
            "parcel",
            SCHOOL,
            Some(3),
        )],
        "birthday_card" => vec![build(PARCELS, "A birthday present", "parcel", PUB, Some(1))],
        "quiz_night" | "beans_shared" | "reading_aloud" => vec![build(
            BUNTING,
            "Bunting for the evening",
            "bunting",
            SCHOOL,
            Some(1),
        )],
        "school_roof_fell_in" => vec![mark("roof_refused")],
        "stall_dream_faded" => vec![mark("stall_refused")],
        "nets_gave_way" => vec![mark("nets_torn")],
        "music_night_forgotten" => vec![mark("music_refused")],
        "oven_failed" => vec![mark("oven_patched")],
        _ => Vec::new(),
    }
}

/// What has to be still unsettled for a storylet to come up: a want once
/// granted does not come back the same.
fn settled_by(storylet: &str) -> Vec<Condition> {
    use Condition::Unmarked;
    match storylet {
        "school_roof" => vec![Unmarked("roof_done"), Unmarked("roof_refused")],
        "new_oven" => vec![Unmarked("oven")],
        "new_nets" => vec![Unmarked("nets")],
        "market_stall" => vec![Unmarked("stall"), Unmarked("stall_refused")],
        "school_books" => vec![Unmarked("books")],
        "music_night" => vec![Unmarked("music_monthly")],
        _ => Vec::new(),
    }
}

// ---- The harbour's story tables, read from data ---------------------------
//
// What the harbour's people want, what befalls it at any time, and the
// threads that follow from what earlier answers marked are data
// (`data/*.json`): the builder calls of this file as a call tree
// (`storylets::script`), each table parsed once.

/// Every spec of one table of the story.
fn table(tree: &'static OnceLock<Node>, json: &'static str) -> Vec<Spec> {
    tree.get_or_init(|| storylets::script::parse(json).expect("the harbour's story tables"))
        .items()
        .iter()
        .map(story_spec)
        .collect()
}

/// What the harbour's people want.
fn wants() -> Vec<Spec> {
    static TREE: OnceLock<Node> = OnceLock::new();
    table(&TREE, include_str!("../data/wants.json"))
}

/// What can befall the harbour at any time.
fn incidents_at_any_time() -> Vec<Spec> {
    static TREE: OnceLock<Node> = OnceLock::new();
    table(&TREE, include_str!("../data/incidents.json"))
}

/// The threads that follow from what earlier answers marked.
fn threads() -> Vec<Spec> {
    static TREE: OnceLock<Node> = OnceLock::new();
    table(&TREE, include_str!("../data/threads.json"))
}

/// A person, place or thing the tables name.
fn entity(node: &Node) -> EntityId {
    use crate::{PUB, SCHOOL};
    match node.name() {
        "ADA" => ADA,
        "BAKERY" => BAKERY,
        "BENCHES" => BENCHES,
        "BUNTING" => BUNTING,
        "CRATES" => CRATES,
        "EMMA" => EMMA,
        "EVAN" => EVAN,
        "HARBOR" => HARBOR,
        "IVO" => IVO,
        "JONAS" => JONAS,
        "JONAS_BOAT" => JONAS_BOAT,
        "LAMP" => LAMP,
        "LANTERNS" => LANTERNS,
        "LEO" => LEO,
        "MARA" => MARA,
        "MIA" => MIA,
        "NETS" => NETS,
        "NOAH" => NOAH,
        "PARCELS" => PARCELS,
        "PIER" => PIER,
        "PUB" => PUB,
        "SCHOOL" => SCHOOL,
        "SHELF" => SHELF,
        "SOFIA" => SOFIA,
        "STALL" => STALL,
        other => panic!("the story tables name no entity {other}"),
    }
}

/// A component the tables name.
fn key(node: &Node) -> &'static str {
    match node.name() {
        "AWAY" => AWAY,
        "CONDITION" => CONDITION,
        "JOB" => JOB,
        "OPERATING_STATUS" => OPERATING_STATUS,
        other => panic!("the story tables name no key {other}"),
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
        // A thread: asked once what it follows from is marked.
        Node::Call("follow", args) => Shape {
            asker: entity(&args[0]),
            want: false,
            requires: conditions(&args[1]),
            lasts: 3,
            rests: 30,
            weight: 6,
            eases: Vec::new(),
            timely: true,
        },
        Node::Call("want", args) => want(entity(&args[0])),
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

fn conditions(node: &Node) -> Vec<Condition> {
    node.items()
        .iter()
        .map(|node| match node {
            Node::Call("Marked", args) => Condition::Marked(args[0].text(), args[1].int() as u64),
            Node::Call("Unmarked", args) => Condition::Unmarked(args[0].text()),
            Node::Call("Finished", args) => Condition::Finished(args[0].text()),
            Node::Call("Condition::Unfinished", args) => Condition::Unfinished(args[0].text()),
            Node::Call("Absent", args) => Condition::Absent(entity(&args[0])),
            Node::Call("Condition::Is", args) => {
                Condition::Is(entity(&args[0]), key(&args[1]), args[2].text())
            }
            Node::Call("has", args) => has(entity(&args[0]), args[1].int()),
            other => panic!("a condition wanted, not {other:?}"),
        })
        .collect()
}

fn story_answer(node: &Node) -> Answer {
    match node {
        Node::Call("yes", args) => yes(
            args[0].text(),
            args[1].text(),
            args[2].text(),
            conditions(&args[3]),
            story_said(&args[4]),
        ),
        Node::Call("other", args) => other(
            args[0].text(),
            args[1].text(),
            args[2].text(),
            story_said(&args[3]),
        ),
        Node::Call("no", args) => no(
            args[0].text(),
            args[1].text(),
            args[2].text(),
            story_said(&args[3]),
        ),
        other => panic!("an answer wanted, not {other:?}"),
    }
}

fn story_said(node: &Node) -> Said {
    match node {
        Node::Call("said", args) => said(
            args[0].text(),
            args[1].text(),
            args[2].text(),
            effects(&args[3]),
        ),
        Node::Call(".remembered", args) => story_said(&args[0]).remembered(args[1].text()),
        other => panic!("said(..) wanted, not {other:?}"),
    }
}

fn effects(node: &Node) -> Vec<Effect> {
    match node {
        Node::List(items) => items.iter().flat_map(effects).collect(),
        Node::Call(".chain", args) => effects(&args[0])
            .into_iter()
            .chain(effects(&args[1]))
            .collect(),
        Node::Call("pay", args) => pay(entity(&args[0]), entity(&args[1]), args[2].int()).to_vec(),
        Node::Call("spend", args) => spend(entity(&args[0]), args[1].int()).to_vec(),
        Node::Call("earn", args) => earn(entity(&args[0]), args[1].int()).to_vec(),
        Node::Call("mood", args) => vec![mood(args[0].int())],
        Node::Call("mark", args) => vec![mark(args[0].text())],
        Node::Call("Effect::Advance", args) => vec![Effect::Advance(args[0].text())],
        Node::Call("build", args) => vec![build(
            entity(&args[0]),
            args[1].text(),
            args[2].text(),
            entity(&args[3]),
            args[4].optional_int().map(|lasts| lasts as u64),
        )],
        Node::Call("newcomer", args) => vec![newcomer(
            entity(&args[0]),
            args[1].text(),
            args[2].text(),
            entity(&args[3]),
        )],
        Node::Struct("Effect::Set", _) => vec![Effect::Set {
            entity: entity(node.field("entity")),
            key: key(node.field("key")),
            text: node.field("text").text(),
        }],
        Node::Struct("Effect::Unset", _) => vec![Effect::Unset {
            entity: entity(node.field("entity")),
            key: key(node.field("key")),
        }],
        other => panic!("effects wanted, not {other:?}"),
    }
}

/// Everyone living in the harbour now: its first eight, less anyone who
/// has left, and anyone who has come to stay. Someone away who is back
/// asking a question of their own (Evan home from the mainland) is here
/// while it is open.
pub(crate) fn people(world: &World) -> Vec<EntityId> {
    people_in(world.state())
}

/// Everyone living in the harbour now, read straight from its state.
pub(crate) fn people_in(state: &world_core::WorldState) -> Vec<EntityId> {
    let deck = deck();
    let asking = storylets::open(state, deck)
        .into_iter()
        .map(|storylet| storylet.asker)
        .collect::<Vec<_>>();
    let arrivals = lives::arrivals(state, &crate::life::cast());
    let grown_here = lives::grown_here(state, &crate::life::cast());
    crate::talk::RESIDENTS
        .into_iter()
        .chain([ADA, IVO])
        .chain(arrivals)
        .chain(grown_here)
        .filter(|id| {
            state.entity(*id).is_some()
                && !lives::gone(state, *id)
                && (!matches!(
                    state.entity(*id).and_then(|entity| entity.component(AWAY)),
                    Some(Value::Text(_))
                ) || asking.contains(id))
        })
        .collect()
}

/// How a fixture is drawn.
pub(crate) fn fixture_shape(shape: &str) -> world_projection::MarkShape {
    use world_projection::MarkShape;
    match shape {
        "stall" => MarkShape::Stall,
        "bunting" => MarkShape::Bunting,
        "pier" => MarkShape::Pier,
        "garden" => MarkShape::Garden,
        "flag" => MarkShape::Flag,
        "lantern" => MarkShape::Lantern,
        "tent" => MarkShape::Tent,
        "bench" => MarkShape::Bench,
        "sprouts" => MarkShape::Sprouts,
        "tree" => MarkShape::Tree,
        "boat" => MarkShape::Boat,
        "well" => MarkShape::Well,
        "swing" => MarkShape::Swing,
        "fountain" => MarkShape::Fountain,
        "signpost" => MarkShape::Signpost,
        "birdhouse" => MarkShape::Birdhouse,
        "planter" => MarkShape::Planter,
        "statue" => MarkShape::Statue,
        "postbox" => MarkShape::Postbox,
        "stone" => MarkShape::Statue,
        "house" => MarkShape::House,
        "shop" => MarkShape::Shop,
        "tower" => MarkShape::Tower,
        _ => MarkShape::Parcel,
    }
}

/// What answers have put on the scene, standing beside the places they
/// belong to.
pub(crate) fn fixtures(world: &World) -> Vec<world_projection::CanvasItem> {
    storylets::fixtures(world.state())
        .into_iter()
        .map(|fixture| {
            let named = match fixture.component("shape") {
                Some(Value::Text(shape)) => shape.as_str(),
                _ => "",
            };
            let shape = fixture_shape(named);
            let at = match fixture.component("at") {
                Some(Value::Entity(at)) => Some(world_projection::SelectionId::Entity(*at)),
                _ => None,
            };
            world_projection::CanvasItem {
                id: world_projection::SelectionId::Entity(fixture.id),
                kind: world_projection::CanvasItemKind::Object,
                label: world_projection::entity_title(fixture),
                detail: String::new(),
                x: 0.5,
                y: 0.5,
                changes: Vec::new(),
                shape: Some(shape),
                at,
                look: None,
                drawing: if fixture.component(lives::generations::MEMORIAL_OF).is_some() {
                    Some(format!("harbour-memorial-{named}"))
                } else {
                    crate::drawings::fixture_drawing(named)
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

/// The weather over the harbour, from how the World stands: a gale while a
/// storm is on, snow and grey skies in winter, rain and fog in the
/// changeable seasons, as the day's own number falls.
pub(crate) fn weather(world: &World) -> world_projection::Weather {
    use world_projection::Weather;
    let deck = deck();
    let stormy = storylets::open(world.state(), deck)
        .iter()
        .any(|storylet| matches!(storylet.id, "storm_warning" | "great_storm"));
    let day = world.world_time() / crate::persistence::WORLD_DAY_TICKS;
    let recent_storm = world.events().iter().rev().take(80).any(|event| {
        event.kind == "storm_started"
            && event.world_time / crate::persistence::WORLD_DAY_TICKS + 1 >= day
    });
    if stormy || recent_storm {
        return Weather::Storm;
    }
    // A new player's first day is a fair one.
    if crate::arrival::arrived(world.state()).is_some_and(|first| day <= first) {
        return Weather::Clear;
    }
    let roll = storylets::mix(&[day, 17]) % 10;
    match (season(world), roll) {
        (3, 0..=3) => Weather::Snow,
        (3, 4..=6) => Weather::Cloudy,
        (2, 0..=2) => Weather::Rain,
        (2, 3..=4) => Weather::Fog,
        (2, 5..=6) => Weather::Cloudy,
        (0, 0..=1) => Weather::Rain,
        (0, 2) => Weather::Fog,
        (0, 3..=4) => Weather::Cloudy,
        (1, 0) => Weather::Rain,
        (1, 1..=2) => Weather::Cloudy,
        _ => Weather::Clear,
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
