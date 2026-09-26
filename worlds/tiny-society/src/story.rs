//! The harbour's storyteller: what its people want, what the sea and the
//! calendar bring, and how each chapter of the town's life ends.
//!
//! The mechanics are the shared `storylets` System. Everything here is the
//! harbour's own: who asks what, what each answer costs and changes, and
//! the words it is told in. What the storyteller does is recorded as
//! ordinary Events, so replaying the town never runs it again.

use crate::model::{CONDITION, MAINLAND_MARKET, OPERATING_STATUS};
use crate::{BAKERY, EMMA, EVAN, JONAS, JONAS_BOAT, LEO, MARA, MIA, NOAH, SOFIA};
use society_basic::{CASH, JOB};
use std::sync::OnceLock;
use storylets::{Choice, Condition, Deck, Ease, Effect, Goal, Outcome, Pinned, Reading, Storylet};
use world_core::{ActionRegistry, EntityId, Event, EventId, Value, World, WorldError};

/// The entity the storyteller keeps its notes on.
pub(crate) const STORY: EntityId = EntityId::new(401);
/// How the harbour feels, from -5 to 5.
pub(crate) const MOOD: &str = "story.mood";
/// Days in a season; four make the harbour's year.
pub(crate) const SEASON_DAYS: u64 = 10;
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
    }
}

impl Said {
    fn remembered(mut self, line: &'static str) -> Self {
        self.remembered = Some(line);
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
            "school_roof",
            want(EMMA),
            (
                "Emma's school roof is leaking",
                "The roof's dripping on the desks again.",
            ),
            vec![
                yes(
                    "mend",
                    "Pay Evan to mend it",
                    "Noah puts in 60: 25 to Evan for the work, the rest for mainland slate.",
                    vec![has(NOAH, 60)],
                    said(
                        "school_roof_mended",
                        "Evan mended the school roof",
                        "Dry desks at last!",
                        [pay(NOAH, EVAN, 25), spend(NOAH, 35)]
                            .into_iter()
                            .flatten()
                            .chain([mood(1)]),
                    )
                    .remembered("The children can hear themselves think now."),
                ),
                no(
                    "bucket",
                    "A bucket will do",
                    "Nothing spent. Emma won't forget it.",
                    said(
                        "school_roof_left",
                        "Emma was told a bucket would do",
                        "A bucket. Wonderful.",
                        [mood(-1)],
                    ),
                ),
            ],
            said(
                "school_roof_fell_in",
                "Part of the school roof came in",
                "Half the ceiling's on the floor!",
                spend(NOAH, 80).into_iter().chain([mood(-1)]),
            ),
        ),
        spec(
            "music_night",
            want(LEO),
            (
                "Leo wants to put on a music night",
                "A fiddler and a full pub. What do you say?",
            ),
            vec![
                yes(
                    "hold",
                    "Put it on",
                    "Leo spends 40 on a fiddler from the mainland. The whole harbour comes.",
                    vec![has(LEO, 40)],
                    said(
                        "music_night_held",
                        "The Anchor Pub had a music night",
                        "Listen to them sing!",
                        spend(LEO, 40).into_iter().chain([mood(2)]),
                    )
                    .remembered("My feet still ache from dancing."),
                ),
                no(
                    "quiet",
                    "Keep it quiet",
                    "Nothing spent, and nothing to remember.",
                    said(
                        "music_night_called_off",
                        "Leo called off his music night",
                        "Another quiet night, then.",
                        [],
                    ),
                ),
            ],
            said(
                "music_night_forgotten",
                "Leo's music night never happened",
                "Nobody seemed to care.",
                [mood(-1)],
            ),
        ),
        spec(
            "new_oven",
            want(MARA).requires(vec![Condition::Is(BAKERY, OPERATING_STATUS, "open")]),
            (
                "Mara's oven is failing",
                "The old oven burns every other loaf.",
            ),
            vec![
                yes(
                    "buy",
                    "Buy a new oven",
                    "80 of Mara's savings go to the mainland for a new oven.",
                    vec![has(MARA, 80)],
                    said(
                        "oven_bought",
                        "Mara's new oven arrived",
                        "Listen to that oven roar!",
                        spend(MARA, 80).into_iter().chain([mood(1)]),
                    )
                    .remembered("Not a burnt loaf all week."),
                ),
                no(
                    "patch",
                    "Patch the old one",
                    "Evan patches it for 10. It won't last.",
                    said(
                        "oven_patched",
                        "Mara patched up the old oven",
                        "Held together with wire and hope.",
                        pay(MARA, EVAN, 10),
                    ),
                ),
            ],
            said(
                "oven_failed",
                "Mara's oven gave out",
                "A whole batch, ruined.",
                spend(MARA, 30).into_iter().chain([mood(-1)]),
            ),
        ),
        spec(
            "new_nets",
            want(JONAS).requires(vec![
                Condition::Is(JONAS_BOAT, CONDITION, "sound"),
                Condition::Is(JONAS, JOB, "fisher"),
            ]),
            ("Jonas's nets are torn", "My nets are more hole than net."),
            vec![
                yes(
                    "buy",
                    "Leo buys new nets",
                    "Leo spends 30 on mainland nets for Jonas.",
                    vec![has(LEO, 30)],
                    said(
                        "nets_bought",
                        "Jonas got new nets",
                        "These'll hold a whale!",
                        spend(LEO, 30).into_iter().chain([mood(1)]),
                    )
                    .remembered("The new nets are pulling their weight."),
                ),
                no(
                    "mend",
                    "Mend them yourself",
                    "Nothing spent. A long night for Jonas.",
                    said(
                        "nets_left_torn",
                        "Jonas was left to mend his own nets",
                        "I'll be up all night with these.",
                        [mood(-1)],
                    ),
                ),
            ],
            said(
                "nets_gave_way",
                "Jonas lost a catch through his torn nets",
                "Half the catch swam off.",
                [mood(-1)],
            ),
        ),
        spec(
            "pier_timber",
            want(EVAN).requires(vec![Condition::Unfinished("pier")]),
            (
                "Evan wants timber for the new pier",
                "Give me timber and I'll give you a pier.",
            ),
            vec![
                yes(
                    "buy",
                    "Buy the timber",
                    "Noah pays 70 for mainland timber. The pier grows a section.",
                    vec![has(NOAH, 70)],
                    said(
                        "pier_section_built",
                        "Evan built a section of the new pier",
                        "Another length of pier, straight and true.",
                        spend(NOAH, 70)
                            .into_iter()
                            .chain([Effect::Advance("pier"), mood(1)]),
                    )
                    .remembered("The pier's coming along."),
                ),
                no(
                    "wait",
                    "The pier can wait",
                    "Nothing spent, and no pier.",
                    said(
                        "pier_put_off",
                        "The new pier was put off again",
                        "Always next month.",
                        [mood(-1)],
                    ),
                ),
            ],
            said(
                "pier_timber_rotted",
                "The pier timber rotted on the quay",
                "Good wood, gone to rot.",
                [mood(-1)],
            ),
        ),
        spec(
            "market_stall",
            want(SOFIA),
            (
                "Sofia wants a stall of her own",
                "Just a little stall on market day. Please?",
            ),
            vec![
                yes(
                    "build",
                    "Set her up",
                    "50 of Sofia's savings go on a stall and her first stock.",
                    vec![has(SOFIA, 50)],
                    said(
                        "stall_opened",
                        "Sofia opened a stall of her own",
                        "My own stall! Come and see!",
                        spend(SOFIA, 50).into_iter().chain([mood(1)]),
                    )
                    .remembered("Sold out of jam again."),
                ),
                no(
                    "not_yet",
                    "Not this year",
                    "Nothing spent. Sofia keeps her savings, and her grievance.",
                    said(
                        "stall_refused",
                        "Sofia stayed behind the pub counter",
                        "Maybe next year, then.",
                        [mood(-1)],
                    ),
                ),
            ],
            said(
                "stall_dream_faded",
                "Sofia gave up on her stall",
                "Forget I asked.",
                [mood(-1)],
            ),
        ),
        spec(
            "school_books",
            want(MIA),
            ("Mia needs new schoolbooks", "My books are older than Emma!"),
            vec![
                yes(
                    "buy",
                    "Buy new books",
                    "Emma spends 25 on books from the mainland.",
                    vec![has(EMMA, 25)],
                    said(
                        "books_bought",
                        "Mia got new schoolbooks",
                        "They still smell new!",
                        spend(EMMA, 25).into_iter().chain([mood(1)]),
                    )
                    .remembered("I've read the new one twice already."),
                ),
                no(
                    "share",
                    "Share a friend's",
                    "Nothing spent. Mia makes do.",
                    said(
                        "books_shared",
                        "Mia was told to share a friend's books",
                        "We'll take turns, I suppose.",
                        [],
                    ),
                ),
            ],
            said(
                "books_went_without",
                "Mia went without new books",
                "Never mind.",
                [mood(-1)],
            ),
        ),
        spec(
            "harbour_lamp",
            want(NOAH).requires(vec![Condition::Unfinished("lamp")]),
            (
                "Noah wants a lamp on the point",
                "Boats need a light to come home by.",
            ),
            vec![
                yes(
                    "fund",
                    "Fund the lamp",
                    "Noah pays 90 for brass and oil. The lamp is a step nearer lit.",
                    vec![has(NOAH, 90)],
                    said(
                        "lamp_work_done",
                        "Work went on at the lamp on the point",
                        "One step closer to a light on the point.",
                        spend(NOAH, 90)
                            .into_iter()
                            .chain([Effect::Advance("lamp"), mood(1)]),
                    )
                    .remembered("Can't wait to see it lit."),
                ),
                no(
                    "later",
                    "The lamp can wait",
                    "Nothing spent. The point stays dark.",
                    said(
                        "lamp_put_off",
                        "The lamp on the point was put off",
                        "In the dark a while longer, then.",
                        [mood(-1)],
                    ),
                ),
            ],
            said(
                "lamp_forgotten",
                "Noah's lamp was forgotten",
                "Nobody remembers the lamp.",
                [mood(-1)],
            ),
        ),
    ]
}

fn incidents() -> Vec<Spec> {
    vec![
        spec(
            "storm_warning",
            incident(JONAS, vec![down("spirits"), down("money")]),
            ("A storm is coming", "Sky's turning black out west."),
            vec![
                yes(
                    "haul_up",
                    "Haul the boats up",
                    "Everyone lends a hand. Noah pays 20 for rope.",
                    vec![has(NOAH, 20)],
                    said(
                        "boats_hauled_up",
                        "The harbour hauled its boats up before the storm",
                        "All hands, heave!",
                        spend(NOAH, 20).into_iter().chain([mood(1)]),
                    )
                    .remembered("Not a boat lost in that storm."),
                ),
                other(
                    "ride_out",
                    "Let it come",
                    "Nothing spent now. Evan will have work after.",
                    said(
                        "storm_ridden_out",
                        "The storm battered the harbour",
                        "Hold on to something!",
                        pay(NOAH, EVAN, 30).into_iter().chain([mood(-2)]),
                    ),
                ),
            ],
            said(
                "storm_caught_the_harbour",
                "The storm caught the harbour unready",
                "Nobody was ready!",
                spend(NOAH, 40).into_iter().chain([mood(-2)]),
            ),
        ),
        spec(
            "traveller",
            incident(SOFIA, vec![up("money"), up("spirits")]),
            (
                "A traveller needs a room",
                "There's a traveller asking for a bed.",
            ),
            vec![
                other(
                    "room",
                    "Give her a room",
                    "The traveller pays Leo 40 and tells stories all night.",
                    said(
                        "traveller_stayed",
                        "A traveller stayed at the Anchor Pub",
                        "Came from the far side of the world, she says.",
                        earn(LEO, 40).into_iter().chain([mood(1)]),
                    )
                    .remembered("Still thinking about that traveller's stories."),
                ),
                other(
                    "send_on",
                    "Send her on",
                    "The pub stays quiet.",
                    said(
                        "traveller_sent_on",
                        "The traveller was sent on",
                        "Sorry, we're full.",
                        [],
                    ),
                ),
            ],
            said(
                "traveller_left",
                "The traveller moved on",
                "Gone on the morning ferry.",
                [],
            ),
        ),
        spec(
            "fever",
            incident(EMMA, vec![down("money")]),
            (
                "Mia has a fever",
                "Mia's burning up. Should we send for the doctor?",
            ),
            vec![
                yes(
                    "doctor",
                    "Send for the doctor",
                    "Emma pays 35 for the doctor's crossing.",
                    vec![has(EMMA, 35)],
                    said(
                        "doctor_came",
                        "The mainland doctor came for Mia",
                        "Rest and broth, the doctor says.",
                        spend(EMMA, 35).into_iter().chain([mood(1)]),
                    )
                    .remembered("Mia's back on her feet."),
                ),
                other(
                    "rest",
                    "Let her sleep",
                    "Nothing spent. A worried few days.",
                    said(
                        "fever_slept_off",
                        "Mia slept off her fever",
                        "She'll be right in a few days.",
                        [mood(-1)],
                    ),
                ),
            ],
            said(
                "fever_lingered",
                "Mia's fever lingered",
                "Still no better.",
                spend(EMMA, 20).into_iter().chain([mood(-1)]),
            ),
        ),
        spec(
            "hotel_order",
            incident(MARA, vec![up("money")]).requires(vec![Condition::Is(
                BAKERY,
                OPERATING_STATUS,
                "open",
            )]),
            (
                "A mainland hotel wants bread",
                "A hotel wants bread for a week. Can we manage?",
            ),
            vec![
                other(
                    "take",
                    "Take the order",
                    "The hotel pays Mara 90. The ovens run all night.",
                    said(
                        "hotel_order_baked",
                        "Mara baked for a mainland hotel",
                        "Flour to my elbows, but paid!",
                        earn(MARA, 90),
                    )
                    .remembered("That hotel wants more already."),
                ),
                other(
                    "decline",
                    "Turn it down",
                    "The harbour's own bread comes first.",
                    said(
                        "hotel_order_declined",
                        "Mara turned the hotel down",
                        "Our own come first.",
                        [mood(1)],
                    ),
                ),
            ],
            said(
                "hotel_went_elsewhere",
                "The hotel took its order elsewhere",
                "They've gone to the mainland baker.",
                [],
            ),
        ),
        spec(
            "quarrel",
            incident(LEO, vec![down("spirits")]).requires(vec![has(EVAN, 20)]),
            (
                "Leo and Evan are quarrelling",
                "Evan's owed me for months, and he knows it.",
            ),
            vec![
                other(
                    "leo",
                    "Side with Leo",
                    "Evan pays Leo the 20 he owes.",
                    said(
                        "quarrel_settled_for_leo",
                        "Evan paid Leo what he owed",
                        "About time.",
                        pay(EVAN, LEO, 20).into_iter().chain([mood(-1)]),
                    ),
                ),
                other(
                    "evan",
                    "Side with Evan",
                    "Leo lets the debt go.",
                    said(
                        "quarrel_settled_for_evan",
                        "Leo let Evan's debt go",
                        "Fine. Keep it.",
                        [mood(-1)],
                    ),
                ),
            ],
            said(
                "quarrel_festered",
                "Leo and Evan stopped speaking",
                "I've nothing to say to him.",
                [mood(-2)],
            ),
        ),
        spec(
            "harbour_fete",
            incident(NOAH, vec![up("spirits"), down("money")]),
            (
                "Noah wants to hold a harbour fête",
                "Bunting, a band, a tug-of-war. Shall we?",
            ),
            vec![
                yes(
                    "hold",
                    "Hold the fête",
                    "Noah pays 60 for bunting and a band.",
                    vec![has(NOAH, 60)],
                    said(
                        "fete_held",
                        "The harbour held a fête",
                        "What a day!",
                        spend(NOAH, 60).into_iter().chain([mood(2)]),
                    )
                    .remembered("Best fête in years."),
                ),
                other(
                    "skip",
                    "Skip it",
                    "Nothing spent. A dull week.",
                    said(
                        "fete_skipped",
                        "The fête was skipped",
                        "Maybe next year.",
                        [mood(-1)],
                    ),
                ),
            ],
            said(
                "fete_forgotten",
                "The fête never happened",
                "Nobody got round to it.",
                [mood(-1)],
            ),
        ),
        spec(
            "pier_piles",
            incident(EVAN, vec![down("money")]).requires(vec![Condition::Unfinished("pier")]),
            (
                "The new pier needs its piles driven",
                "The piles need driving before the tide turns.",
            ),
            vec![
                yes(
                    "drive",
                    "Drive the piles",
                    "Noah pays 50, and 20 of it is Evan's wage.",
                    vec![has(NOAH, 50)],
                    said(
                        "pier_piles_driven",
                        "The new pier's piles were driven",
                        "She'll stand a hundred years.",
                        [pay(NOAH, EVAN, 20), spend(NOAH, 30)]
                            .into_iter()
                            .flatten()
                            .chain([Effect::Advance("pier")]),
                    ),
                ),
                other(
                    "leave",
                    "Leave them",
                    "Nothing spent. The pier waits.",
                    said(
                        "pier_piles_left",
                        "The pier work stalled",
                        "Tide's turned. Missed it.",
                        [],
                    ),
                ),
            ],
            said(
                "pier_piles_washed_out",
                "The tide washed out the pier work",
                "All that work, gone with the tide.",
                [mood(-1)],
            ),
        ),
        spec(
            "lamp_glass",
            incident(NOAH, vec![down("money")]).requires(vec![Condition::Unfinished("lamp")]),
            (
                "The lamp's glass has come in",
                "The glass is at the mainland dock. Fetch it?",
            ),
            vec![
                yes(
                    "fetch",
                    "Ship it over",
                    "Noah pays 40 for the crossing.",
                    vec![has(NOAH, 40)],
                    said(
                        "lamp_glass_fetched",
                        "The lamp's glass came over from the mainland",
                        "Careful with that!",
                        spend(NOAH, 40).into_iter().chain([Effect::Advance("lamp")]),
                    ),
                ),
                other(
                    "leave",
                    "Leave it there",
                    "Nothing spent. It will keep, probably.",
                    said(
                        "lamp_glass_left",
                        "The lamp's glass waited at the dock",
                        "It'll keep.",
                        [],
                    ),
                ),
            ],
            said(
                "lamp_glass_lost",
                "The lamp's glass went astray",
                "Lost! How do you lose a lamp?",
                [mood(-1)],
            ),
        ),
        spec(
            "mackerel",
            incident(JONAS, vec![up("money"), up("spirits")])
                .requires(vec![Condition::Is(JONAS, JOB, "fisher")]),
            (
                "The bay is full of mackerel",
                "The bay's full of mackerel! Take the lot?",
            ),
            vec![
                other(
                    "sell",
                    "Sell the lot",
                    "Jonas makes 50.",
                    said(
                        "mackerel_sold",
                        "Jonas sold a glut of mackerel",
                        "Silver all the way to the mainland!",
                        earn(JONAS, 50),
                    ),
                ),
                other(
                    "share",
                    "Share them round",
                    "Nothing earned. Everyone eats well.",
                    said(
                        "mackerel_shared",
                        "Jonas shared his mackerel round the harbour",
                        "Mackerel for everyone!",
                        [mood(2)],
                    )
                    .remembered("Still smells of grilled mackerel round here."),
                ),
            ],
            said(
                "mackerel_moved_on",
                "The mackerel moved on",
                "Gone as fast as they came.",
                [],
            ),
        ),
        spec(
            "chimney_fire",
            incident(LEO, vec![down("spirits"), down("money")]),
            ("The pub chimney caught fire", "Chimney's caught! Get Evan!"),
            vec![
                yes(
                    "rebuild",
                    "Pay Evan to rebuild",
                    "Leo pays Evan 30.",
                    vec![has(LEO, 30)],
                    said(
                        "chimney_rebuilt",
                        "Evan rebuilt the pub chimney",
                        "Draws like a dream now.",
                        pay(LEO, EVAN, 30),
                    ),
                ),
                other(
                    "patch",
                    "Patch it and hope",
                    "Nothing spent. Smoky evenings.",
                    said(
                        "chimney_patched",
                        "Leo patched his chimney",
                        "It'll do. Probably.",
                        [mood(-1)],
                    ),
                ),
            ],
            said(
                "chimney_burned_out",
                "The pub chimney burned out",
                "Smoke everywhere!",
                spend(LEO, 50).into_iter().chain([mood(-1)]),
            ),
        ),
        spec(
            "mainland_work",
            incident(EVAN, vec![up("money"), down("spirits")]),
            (
                "There's mainland work for Evan",
                "A week's carpentry on the mainland. Should I go?",
            ),
            vec![
                other(
                    "go",
                    "Go",
                    "Evan earns 60. The harbour misses its carpenter.",
                    said(
                        "evan_worked_away",
                        "Evan took a week's work on the mainland",
                        "Back soon, with money in my pocket.",
                        earn(EVAN, 60).into_iter().chain([mood(-1)]),
                    ),
                ),
                other(
                    "stay",
                    "Stay",
                    "Nothing earned. The harbour keeps its carpenter.",
                    said(
                        "evan_stayed_home",
                        "Evan stayed home",
                        "Plenty to mend here anyway.",
                        [mood(1)],
                    ),
                ),
            ],
            said(
                "mainland_work_went",
                "Evan's mainland work went to someone else",
                "Too slow. Someone else took it.",
                [],
            ),
        ),
    ]
}

fn calendar() -> Vec<Spec> {
    let mut days = vec![
        spec(
            "market_day",
            day(SOFIA, 7, 3),
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
    for (id, who, at) in [
        ("birthday_jonas", JONAS, 6),
        ("birthday_mara", MARA, 11),
        ("birthday_leo", LEO, 16),
        ("birthday_emma", EMMA, 20),
        ("birthday_mia", MIA, 24),
        ("birthday_noah", NOAH, 29),
        ("birthday_evan", EVAN, 34),
        ("birthday_sofia", SOFIA, 38),
    ] {
        days.push(spec(
            id,
            day(who, YEAR, at),
            ("{name}'s birthday", "It's my birthday today!"),
            vec![
                yes(
                    "party",
                    "A party at the pub",
                    "Leo spends 20 on cake and a round.",
                    vec![has(LEO, 20)],
                    said(
                        "birthday_party",
                        "{name} had a birthday party",
                        "Best birthday ever!",
                        spend(LEO, 20).into_iter().chain([mood(1)]),
                    )
                    .remembered("Thanks again for the party."),
                ),
                other(
                    "card",
                    "A card from everyone",
                    "Nothing spent. A kind thought.",
                    said(
                        "birthday_card",
                        "{name} got a card from everyone",
                        "You all signed it!",
                        [],
                    ),
                ),
            ],
            said(
                "birthday_forgotten",
                "{name}'s birthday was forgotten",
                "Nobody remembered.",
                [mood(-1)],
            ),
        ));
    }
    days
}

fn specs() -> &'static [Spec] {
    static SPECS: OnceLock<Vec<Spec>> = OnceLock::new();
    SPECS.get_or_init(|| {
        let mut specs = wants();
        specs.extend(incidents());
        specs.extend(calendar());
        specs.extend(threads());
        specs
    })
}

pub(crate) fn deck() -> Deck {
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
        ],
        chapter_periods: CHAPTER_DAYS,
        shortest_chapter: 8,
        fresh_start: vec![Effect::Put {
            entity: STORY,
            key: MOOD,
            to: 0,
        }],
        most_open: 3,
    }
}

pub(crate) fn register_actions(
    actions: &mut ActionRegistry,
) -> Result<(), world_core::ActionError> {
    storylets::register_actions(actions, deck)
}

fn name_of(world: &World, id: EntityId) -> String {
    world
        .state()
        .entity(id)
        .map(world_projection::entity_title)
        .unwrap_or_else(|| "Someone".into())
}

fn named(world: &World, text: &str, who: EntityId) -> String {
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
    storylets::season(storylets::period_index(world.state(), &deck()), SEASON_DAYS) as usize
}

/// How a chapter of the harbour's life ends, from how the town stands.
fn chapter_ending(world: &World) -> (String, String) {
    let deck = deck();
    let feel = match spirits(world) {
        3.. => "A bright",
        1..=2 => "A good",
        -1..=0 => "A quiet",
        -3..=-2 => "A hard",
        _ => "A bitter",
    };
    let mut title = format!("{feel} {}", SEASONS[season(world)]);
    if storylets::last_chapter_title(world).is_some_and(|last| last.ends_with(&title[2..])) {
        title = format!("Another {}", &title[2..]);
    }
    let mut summary = Vec::new();
    summary.push(
        if text(world, BAKERY, OPERATING_STATUS).as_deref() == Some("closed") {
            "The bakery stayed shut."
        } else {
            "The harbour kept its bakery."
        },
    );
    let boat = text(world, JONAS_BOAT, CONDITION);
    summary.push(
        match (text(world, JONAS, JOB).as_deref(), boat.as_deref()) {
            (Some("fisher"), Some("sound")) => "Jonas fished every day he could.",
            (_, None) => "Jonas never went back to sea.",
            (_, Some("damaged")) => "Sea Finch waited on the slip.",
            _ => "Jonas found his feet ashore.",
        },
    );
    if storylets::progress(world.state(), &deck, "pier") >= 3 {
        summary.push("The new pier stands.");
    }
    if storylets::progress(world.state(), &deck, "lamp") >= 2 {
        summary.push("A lamp burns on the point.");
    }
    let sore = crate::talk::RESIDENTS
        .into_iter()
        .map(|who| (storylets::kindness(world.state(), &deck, who), who))
        .filter(|((granted, grudges), _)| grudges > granted)
        .max_by_key(|((granted, grudges), who)| (grudges - granted, std::cmp::Reverse(*who)))
        .map(|(_, who)| who);
    let mut summary = summary.join(" ");
    if let Some(who) = sore {
        summary.push_str(&format!(
            " {} hasn't forgotten being let down.",
            name_of(world, who)
        ));
    }
    (title, summary)
}

/// One day of the storyteller, at the end of the harbour's day.
pub(crate) fn tick(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    let money = crate::projection::gauges(world)
        .into_iter()
        .find(|gauge| gauge.id == "money")
        .map_or(0.5, |gauge| gauge.value);
    let reading = Reading {
        pinned: pinned(world),
        at_end: spirits(world).abs() >= 5 || !(0.02..=0.98).contains(&money),
        chapter_ending: Box::new(chapter_ending),
    };
    storylets::tick(world, actions, &deck(), &reading)
}

fn command_id(storylet: &str, choice: &str) -> String {
    format!("{STORY_COMMAND}{storylet}.{choice}")
}

/// The choice a command makes, if it is one of the storyteller's.
pub(crate) fn parse_command(command_id: &str) -> Option<(&str, &str)> {
    command_id.strip_prefix(STORY_COMMAND)?.split_once('.')
}

fn find(storylet: &str) -> Option<&'static Spec> {
    specs().iter().find(|spec| spec.storylet.id == storylet)
}

/// A card for every answer that can be given now, each asked by whoever
/// the storylet belongs to.
pub(crate) fn commands(world: &World) -> Vec<world_projection::ProjectionCommand> {
    let deck = deck();
    storylets::choices(world.state(), &deck)
        .into_iter()
        .filter_map(|(storylet, choice)| {
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
                    prompt: named(world, spec.line, storylet.asker),
                }),
            })
        })
        .collect()
}

/// What someone would ask for now, if they have a want open: what they
/// say, and the answer that grants it, if it can be given.
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
    Some((spec.line.to_string(), grant))
}

/// How many wants someone has had granted, and turned down or let lapse.
pub(crate) fn kindness(world: &World, who: EntityId) -> (i64, i64) {
    storylets::kindness(world.state(), &deck(), who)
}

fn storylet_of(event: &Event) -> Option<&'static Spec> {
    match event.payload.get("storylet")? {
        Value::Text(id) => find(id),
        _ => None,
    }
}

fn outcome_of(event: &Event) -> Option<(&'static Spec, &'static Said)> {
    let spec = storylet_of(event)?;
    if event.kind == "situation_arose" {
        return None;
    }
    if spec.lapse.event == event.kind {
        return Some((spec, &spec.lapse));
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
    storylet_of(event).is_some()
}

/// How the harbour tells one of the storyteller's moments.
pub(crate) fn told(world: &World, event: &Event) -> Option<String> {
    if event.kind == "chapter_ended" {
        let title = match event.payload.get("title") {
            Some(Value::Text(title)) => title.clone(),
            _ => return None,
        };
        return Some(format!("The chapter closed: {title}"));
    }
    let spec = storylet_of(event)?;
    let who = spec.storylet.asker;
    if event.kind == "situation_arose" {
        return Some(named(world, spec.told, who));
    }
    let (_, said) = outcome_of(event)?;
    Some(named(world, said.told, who))
}

/// What the asker says at one of the storyteller's moments.
pub(crate) fn line(event: &Event) -> Option<(EntityId, String)> {
    let spec = storylet_of(event)?;
    let who = spec.storylet.asker;
    if event.kind == "situation_arose" {
        return Some((who, spec.line.to_string()));
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
    [
        ("pier", "The new pier", world_projection::MarkShape::Bridge),
        (
            "lamp",
            "A lamp on the point",
            world_projection::MarkShape::Lamp,
        ),
    ]
    .into_iter()
    .filter_map(|(id, label, shape)| {
        let parts = deck.goals.iter().find(|goal| goal.id == id)?.parts;
        Some(world_projection::Goal {
            id: id.into(),
            label: label.into(),
            shape,
            done: storylets::progress(world.state(), &deck, id).clamp(0, parts) as u32,
            parts: parts as u32,
        })
    })
    .collect()
}

/// The chapters of the harbour's story that have ended.
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
/// People who can come to live in the harbour.
pub(crate) const ADA: EntityId = EntityId::new(9);
pub(crate) const IVO: EntityId = EntityId::new(10);
/// Where someone is when they have left the harbour.
pub(crate) const AWAY: &str = "away";

fn build(
    entity: EntityId,
    name: &'static str,
    shape: &'static str,
    at: EntityId,
    lasts: Option<u64>,
) -> Effect {
    Effect::Build {
        entity,
        name,
        shape,
        at,
        lasts,
    }
}

fn mark(name: &'static str) -> Effect {
    Effect::Mark(name)
}

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
        "school_roof_mended" | "school_roof_mended_at_last" => vec![mark("roof_done")],
        "school_roof_left" => vec![mark("roof_refused")],
        "music_night_held" => vec![
            build(BUNTING, "Bunting over the pub", "bunting", PUB, Some(3)),
            mark("music"),
        ],
        "oven_bought" => vec![mark("oven")],
        "nets_bought" => vec![mark("nets")],
        "pier_section_built" | "pier_piles_driven" => {
            vec![build(PIER, "The new pier, going up", "pier", HARBOR, None)]
        }
        "stall_opened" => vec![
            build(STALL, "Sofia's stall", "stall", PUB, None),
            mark("stall"),
        ],
        "stall_refused" => vec![mark("stall_refused")],
        "books_bought" => vec![mark("books")],
        "lamp_work_done" | "lamp_glass_fetched" => vec![build(
            LAMP,
            "The lamp post on the point, unlit",
            "lantern",
            HARBOR,
            None,
        )],
        "traveller_stayed" => vec![mark("traveller")],
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

/// Questions that follow from earlier answers: second and third acts.
fn threads() -> Vec<Spec> {
    use crate::{HARBOR, PUB};
    use Condition::{Absent, Finished, Marked, Unmarked};
    let follow = |asker: EntityId, requires: Vec<Condition>| Shape {
        asker,
        want: false,
        requires,
        lasts: 3,
        rests: 30,
        weight: 6,
        eases: Vec::new(),
        timely: true,
    };
    vec![
        spec(
            "stall_thriving",
            follow(SOFIA, vec![Marked("stall", 4), Unmarked("stall_help")]),
            (
                "Sofia's stall is doing well",
                "My stall's doing well! Could Mia help on Saturdays?",
            ),
            vec![
                other(
                    "take_mia",
                    "Take Mia on",
                    "Sofia pays Mia 15 a week from the stall.",
                    said(
                        "mia_at_the_stall",
                        "Mia started helping at Sofia's stall",
                        "Mia's a natural with customers!",
                        pay(SOFIA, MIA, 15).into_iter().chain([
                            mood(1),
                            mark("stall_help"),
                            mark("stall_grew"),
                        ]),
                    )
                    .remembered("Saturdays at the stall are the best."),
                ),
                other(
                    "keep_small",
                    "Keep it small",
                    "Sofia runs it alone.",
                    said(
                        "stall_kept_small",
                        "Sofia kept her stall small",
                        "Small suits me fine.",
                        [mark("stall_help")],
                    ),
                ),
            ],
            said(
                "stall_help_forgotten",
                "Sofia managed the stall alone",
                "I'll manage.",
                [mark("stall_help")],
            ),
        ),
        spec(
            "pub_quiet",
            follow(
                LEO,
                vec![Marked("stall_grew", 4), Unmarked("pub_quiet_done")],
            ),
            (
                "The pub has gone quiet",
                "The pub's quiet since Sofia's stall took off.",
            ),
            vec![
                yes(
                    "quiz",
                    "A quiz night",
                    "Leo spends 20 on prizes and a board.",
                    vec![has(LEO, 20)],
                    said(
                        "quiz_night",
                        "The Anchor Pub started a quiz night",
                        "Question one: how deep is the harbour?",
                        spend(LEO, 20).into_iter().chain([
                            mood(1),
                            mark("pub_quiet_done"),
                            build(BUNTING, "Bunting for quiz night", "bunting", PUB, Some(2)),
                        ]),
                    ),
                ),
                other(
                    "wait_it_out",
                    "It'll pick up",
                    "Nothing spent.",
                    said(
                        "pub_stayed_quiet",
                        "The pub stayed quiet",
                        "It'll pick up. It always does.",
                        [mood(-1), mark("pub_quiet_done")],
                    ),
                ),
            ],
            said(
                "pub_quiet_passed",
                "The pub found its feet again",
                "Busy again, thank goodness.",
                [mark("pub_quiet_done")],
            ),
        ),
        spec(
            "sofia_offer",
            follow(
                SOFIA,
                vec![Marked("stall_refused", 3), Unmarked("sofia_decided")],
            ),
            (
                "Sofia has an offer from the mainland",
                "There's a job on the mainland. Should I take it?",
            ),
            vec![
                yes(
                    "stay",
                    "Stay, and have your stall",
                    "Sofia spends 40 of her savings on the stall after all.",
                    vec![has(SOFIA, 40)],
                    said(
                        "sofia_stayed",
                        "Sofia stayed, and opened her stall",
                        "Then I'm staying. And I'm opening that stall!",
                        spend(SOFIA, 40).into_iter().chain([
                            mood(2),
                            mark("sofia_decided"),
                            mark("stall"),
                            build(STALL, "Sofia's stall", "stall", PUB, None),
                        ]),
                    ),
                ),
                other(
                    "go",
                    "Go, with our blessing",
                    "Sofia leaves for the mainland.",
                    said(
                        "sofia_left",
                        "Sofia left for the mainland",
                        "I'll write. I promise.",
                        [
                            mood(-1),
                            mark("sofia_decided"),
                            mark("sofia_left"),
                            Effect::Set {
                                entity: SOFIA,
                                key: AWAY,
                                text: "mainland",
                            },
                        ],
                    ),
                ),
            ],
            said(
                "sofia_went_anyway",
                "Sofia took the mainland job",
                "Nobody asked me to stay.",
                [
                    mood(-2),
                    mark("sofia_decided"),
                    mark("sofia_left"),
                    Effect::Set {
                        entity: SOFIA,
                        key: AWAY,
                        text: "mainland",
                    },
                ],
            ),
        ),
        spec(
            "sofia_letter",
            follow(NOAH, vec![Marked("sofia_left", 5), Unmarked("letter_read")]),
            (
                "A letter came from Sofia",
                "A letter from Sofia, on the mainland!",
            ),
            vec![
                other(
                    "read_out",
                    "Read it out at the pub",
                    "Everyone gathers.",
                    said(
                        "sofia_letter_read",
                        "Sofia's letter was read out at the pub",
                        "She's doing well. She misses us.",
                        [mood(2), mark("letter_read")],
                    ),
                ),
                other(
                    "pin_up",
                    "Pin it up at the harbour",
                    "Anyone can read it.",
                    said(
                        "sofia_letter_pinned",
                        "Sofia's letter was pinned up at the harbour",
                        "There, for everyone.",
                        [mood(1), mark("letter_read")],
                    ),
                ),
            ],
            said(
                "sofia_letter_kept",
                "Noah kept Sofia's letter",
                "I'll read it later.",
                [mark("letter_read")],
            ),
        ),
        spec(
            "music_monthly",
            follow(LEO, vec![Marked("music", 5), Unmarked("music_decided")]),
            (
                "The fiddler wants to come back",
                "The fiddler wants to come back every month.",
            ),
            vec![
                yes(
                    "monthly",
                    "Make it monthly",
                    "Leo spends 30 on lanterns for the pub front.",
                    vec![has(LEO, 30)],
                    said(
                        "music_monthly_began",
                        "The Anchor Pub made music night monthly",
                        "First Friday of every month. Tell everyone!",
                        spend(LEO, 30).into_iter().chain([
                            mood(1),
                            mark("music_decided"),
                            mark("music_monthly"),
                            build(LANTERNS, "Lanterns over the pub", "lantern", PUB, None),
                        ]),
                    ),
                ),
                other(
                    "once",
                    "Once was enough",
                    "Nothing spent.",
                    said(
                        "music_once",
                        "The music night stayed a one-off",
                        "Once was plenty.",
                        [mark("music_decided")],
                    ),
                ),
            ],
            said(
                "fiddler_moved_on",
                "The fiddler moved on",
                "He's playing the mainland now.",
                [mark("music_decided")],
            ),
        ),
        spec(
            "ferry_run",
            follow(
                MARA,
                vec![
                    Marked("oven", 3),
                    Unmarked("ferry_decided"),
                    Condition::Is(BAKERY, OPERATING_STATUS, "open"),
                ],
            ),
            (
                "Mara could bake for the ferry",
                "With the new oven I could bake for the ferry.",
            ),
            vec![
                other(
                    "start",
                    "Start the ferry run",
                    "Crates of bread go out on the morning ferry. Mara makes 40.",
                    said(
                        "ferry_run_began",
                        "Mara's bread started going out on the ferry",
                        "Crates on the quay by six!",
                        earn(MARA, 40).into_iter().chain([
                            mark("ferry_decided"),
                            build(CRATES, "Bread crates for the ferry", "parcel", HARBOR, None),
                        ]),
                    ),
                ),
                other(
                    "local",
                    "Keep it local",
                    "The harbour's bread comes first.",
                    said(
                        "bread_kept_local",
                        "Mara kept her bread for the harbour",
                        "Our own come first.",
                        [mood(1), mark("ferry_decided")],
                    ),
                ),
            ],
            said(
                "ferry_chance_passed",
                "The ferry found another baker",
                "Too slow, Mara.",
                [mark("ferry_decided")],
            ),
        ),
        spec(
            "pier_opening",
            follow(NOAH, vec![Finished("pier"), Unmarked("pier_opened")]),
            (
                "The new pier is finished",
                "The pier's finished! Shall we open it properly?",
            ),
            vec![
                other(
                    "party",
                    "Open it with a party",
                    "Bunting, a ribbon and the whole harbour.",
                    said(
                        "pier_opened",
                        "The harbour opened its new pier",
                        "I declare this pier open!",
                        [
                            mood(2),
                            mark("pier_opened"),
                            build(PIER, "The new pier", "pier", HARBOR, None),
                            build(
                                BUNTING,
                                "Bunting on the new pier",
                                "bunting",
                                HARBOR,
                                Some(4),
                            ),
                        ],
                    ),
                ),
                other(
                    "use_it",
                    "Just start using it",
                    "No fuss.",
                    said(
                        "pier_in_use",
                        "The new pier went into use",
                        "Tie up wherever you like.",
                        [
                            mark("pier_opened"),
                            build(PIER, "The new pier", "pier", HARBOR, None),
                        ],
                    ),
                ),
            ],
            said(
                "pier_opened_quietly",
                "The new pier opened without a fuss",
                "Well, it's open.",
                [
                    mark("pier_opened"),
                    build(PIER, "The new pier", "pier", HARBOR, None),
                ],
            ),
        ),
        spec(
            "fishing_family",
            follow(
                JONAS,
                vec![
                    Marked("pier_opened", 3),
                    Absent(IVO),
                    Unmarked("family_turned"),
                ],
            ),
            (
                "A fishing family wants to moor here",
                "A family wants to moor at our new pier. Room for them?",
            ),
            vec![
                other(
                    "welcome",
                    "Welcome them",
                    "Ivo and his boat join the harbour.",
                    said(
                        "ivo_arrived",
                        "Ivo's family came to live in the harbour",
                        "Welcome to the harbour, Ivo!",
                        [mood(2), newcomer(IVO, "Ivo", "fisher", HARBOR)],
                    ),
                ),
                other(
                    "no_room",
                    "No room for more boats",
                    "They sail on.",
                    said(
                        "family_turned_away",
                        "The fishing family sailed on",
                        "Sorry. Not this year.",
                        [mood(-1), mark("family_turned")],
                    ),
                ),
            ],
            said(
                "family_sailed_on",
                "The fishing family didn't wait",
                "They didn't wait for an answer.",
                [mark("family_turned")],
            ),
        ),
        spec(
            "lamp_lit",
            follow(NOAH, vec![Finished("lamp"), Unmarked("lamp_lit")]),
            (
                "The lamp on the point is ready",
                "The lamp's ready. Shall we light it tonight?",
            ),
            vec![
                other(
                    "everyone",
                    "Light it with everyone watching",
                    "The whole harbour walks out to the point.",
                    said(
                        "lamp_lit_together",
                        "The whole harbour watched the lamp lit",
                        "There she shines!",
                        [
                            mood(2),
                            mark("lamp_lit"),
                            build(LAMP, "The lamp on the point", "lantern", HARBOR, None),
                        ],
                    ),
                ),
                other(
                    "quietly",
                    "Just light it",
                    "Noah walks out alone.",
                    said(
                        "lamp_lit_quietly",
                        "Noah lit the lamp on the point",
                        "Boats will see that for miles.",
                        [
                            mood(1),
                            mark("lamp_lit"),
                            build(LAMP, "The lamp on the point", "lantern", HARBOR, None),
                        ],
                    ),
                ),
            ],
            said(
                "lamp_lit_anyway",
                "Somebody lit the lamp on the point",
                "Someone's lit it!",
                [
                    mark("lamp_lit"),
                    build(LAMP, "The lamp on the point", "lantern", HARBOR, None),
                ],
            ),
        ),
        spec(
            "traveller_stays",
            follow(
                LEO,
                vec![Marked("traveller", 3), Absent(ADA), Unmarked("ada_decided")],
            ),
            (
                "The traveller wants to stay",
                "That traveller wants to stay on. She could help at the pub.",
            ),
            vec![
                other(
                    "welcome",
                    "Welcome her",
                    "Ada takes the room above the pub.",
                    said(
                        "ada_arrived",
                        "Ada the traveller made the harbour her home",
                        "Ada's staying! Pour her a pint.",
                        [
                            mood(2),
                            mark("ada_decided"),
                            newcomer(ADA, "Ada", "pub_help", PUB),
                        ],
                    ),
                ),
                other(
                    "no_room",
                    "There's no room",
                    "She moves on.",
                    said(
                        "ada_moved_on",
                        "The traveller moved on",
                        "Maybe she'll come back.",
                        [mark("ada_decided")],
                    ),
                ),
            ],
            said(
                "ada_left",
                "The traveller left without an answer",
                "Gone before I could ask.",
                [mark("ada_decided")],
            ),
        ),
        spec(
            "roof_worse",
            follow(EMMA, vec![Marked("roof_refused", 3), Unmarked("roof_done")]),
            (
                "Parents are keeping children home",
                "The leaks are keeping children home now.",
            ),
            vec![
                yes(
                    "mend",
                    "Mend it now",
                    "Noah finds 70: 25 to Evan, the rest for slate.",
                    vec![has(NOAH, 70)],
                    said(
                        "school_roof_mended_at_last",
                        "The school roof was mended at last",
                        "Better late than never.",
                        [pay(NOAH, EVAN, 25), spend(NOAH, 45)]
                            .into_iter()
                            .flatten()
                            .chain([mood(1)]),
                    ),
                ),
                other(
                    "pub_classes",
                    "Teach at the pub",
                    "Lessons move to the Anchor's back room.",
                    said(
                        "school_moved_to_pub",
                        "Lessons moved into the Anchor Pub",
                        "Long division over the dartboard.",
                        [mood(-1), mark("roof_done")],
                    ),
                ),
            ],
            said(
                "roof_still_leaking",
                "The school roof still leaks",
                "Another bucket, then.",
                [mood(-1), mark("roof_done")],
            ),
        ),
        spec(
            "school_garden",
            want(MIA).requires(vec![Unmarked("garden")]),
            (
                "Mia wants a school garden",
                "Could we plant a garden by the school?",
            ),
            vec![
                yes(
                    "plant",
                    "Plant it",
                    "Emma spends 20 on seeds and a spade.",
                    vec![has(EMMA, 20)],
                    said(
                        "garden_planted",
                        "The school planted a garden",
                        "I planted the beans myself!",
                        spend(EMMA, 20).into_iter().chain([mood(1)]),
                    )
                    .remembered("Our beans are coming up!"),
                ),
                no(
                    "not_now",
                    "Not this year",
                    "Nothing spent.",
                    said(
                        "garden_put_off",
                        "The school garden was put off",
                        "Next year, then.",
                        [],
                    ),
                ),
            ],
            said(
                "garden_forgotten",
                "Nobody planted the school garden",
                "Never mind.",
                [mood(-1)],
            ),
        ),
        spec(
            "garden_harvest",
            follow(MIA, vec![Marked("garden", 6), Unmarked("garden_shared")]),
            (
                "The school garden has cropped",
                "Our garden's full of beans!",
            ),
            vec![
                other(
                    "share",
                    "Share them round",
                    "Every house gets a bag.",
                    said(
                        "beans_shared",
                        "The school's beans went round the harbour",
                        "Beans for everyone!",
                        [mood(2), mark("garden_shared")],
                    ),
                ),
                other(
                    "sell",
                    "Sell them at market",
                    "Mia makes 20.",
                    said(
                        "beans_sold",
                        "Mia sold the school's beans at market",
                        "Twenty for the class trip!",
                        earn(MIA, 20).into_iter().chain([mark("garden_shared")]),
                    ),
                ),
            ],
            said(
                "beans_went_over",
                "The beans went over",
                "Too late, they're tough now.",
                [mark("garden_shared")],
            ),
        ),
    ]
}

/// Everyone living in the harbour now: its first eight, less anyone who
/// has left, and anyone who has come to stay.
pub(crate) fn people(world: &World) -> Vec<EntityId> {
    crate::talk::RESIDENTS
        .into_iter()
        .chain([ADA, IVO])
        .filter(|id| world.state().entity(*id).is_some() && text(world, *id, AWAY).is_none())
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
        _ => MarkShape::Parcel,
    }
}

/// What answers have put on the scene, standing beside the places they
/// belong to.
pub(crate) fn fixtures(world: &World) -> Vec<world_projection::CanvasItem> {
    storylets::fixtures(world.state())
        .into_iter()
        .map(|fixture| {
            let shape = match fixture.component("shape") {
                Some(Value::Text(shape)) => fixture_shape(shape),
                _ => world_projection::MarkShape::Parcel,
            };
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
            }
        })
        .collect()
}
