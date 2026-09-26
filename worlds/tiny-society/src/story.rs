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
    /// What this adds to the chapter's ending, when it closes.
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
        specs.extend(climaxes());
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
        pressures: vec!["storm_season", "hard_times", "inspector"],
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
        "bakery_closed" => "Harbor Bakery closed its doors.",
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
    let (_, started) = storylets::chapter(world.state(), &deck);
    let lived = world
        .events()
        .iter()
        .filter(|event| event.world_time >= started)
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
    let title = title.unwrap_or_else(|| {
        let feel = match spirits(world) {
            3.. => "A bright",
            1..=2 => "A good",
            -1..=0 => "A quiet",
            -3..=-2 => "A hard",
            _ => "A bitter",
        };
        let title = format!("{feel} {}", SEASONS[season(world)]);
        if storylets::last_chapter_title(world).is_some_and(|last| last.ends_with(&title[2..])) {
            format!("Another {}", &title[2..])
        } else {
            title
        }
    });
    // The last three things worth telling, with the climax among them.
    let mut summary = if lines.len() > 3 {
        lines.split_off(lines.len() - 3)
    } else {
        lines
    };
    if summary.is_empty() {
        summary.push(
            if text(world, BAKERY, OPERATING_STATUS).as_deref() == Some("closed") {
                "The bakery stayed shut.".to_string()
            } else {
                "The harbour kept its bakery.".to_string()
            },
        );
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
    let reading = Reading {
        pinned: pinned(world),
        away,
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

/// Questions that follow from earlier answers: second and third acts.
fn threads() -> Vec<Spec> {
    use crate::{HARBOR, PUB, SCHOOL};
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
            follow(SOFIA, vec![Marked("stall", 2), Unmarked("stall_help")]),
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
                            build(STALL, "Sofia and Mia's stall", "stall", PUB, None),
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
                vec![Marked("stall_grew", 2), Unmarked("pub_quiet_done")],
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
                vec![Marked("stall_refused", 2), Unmarked("sofia_decided")],
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
            follow(NOAH, vec![Marked("sofia_left", 3), Unmarked("letter_read")]),
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
            follow(LEO, vec![Marked("music", 2), Unmarked("music_decided")]),
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
                    Marked("oven", 2),
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
                    Marked("pier_opened", 2),
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
                vec![Marked("traveller", 2), Absent(ADA), Unmarked("ada_decided")],
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
            follow(EMMA, vec![Marked("roof_refused", 2), Unmarked("roof_done")]),
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
            follow(MIA, vec![Marked("garden", 3), Unmarked("garden_shared")]),
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
        spec(
            "record_catch",
            follow(JONAS, vec![Marked("nets", 2), Unmarked("record_catch")]),
            (
                "Jonas brought in a record catch",
                "The new nets! Biggest catch I've ever had. Sell it or share it?",
            ),
            vec![
                other(
                    "sell",
                    "Sell it",
                    "Jonas makes 45 on the mainland.",
                    said(
                        "record_catch_sold",
                        "Jonas sold a record catch",
                        "Best week's money I've had.",
                        earn(JONAS, 45).into_iter().chain([
                            mark("record_catch"),
                            build(
                                PARCELS,
                                "Crates of fish for the ferry",
                                "parcel",
                                HARBOR,
                                Some(2),
                            ),
                        ]),
                    ),
                ),
                other(
                    "share",
                    "Share it round",
                    "Every door gets a fish.",
                    said(
                        "record_catch_shared",
                        "Jonas shared a record catch round the harbour",
                        "Fish for everyone!",
                        [
                            mood(2),
                            mark("record_catch"),
                            build(NETS, "Fish smoking on lines", "bunting", HARBOR, Some(2)),
                        ],
                    ),
                ),
            ],
            said(
                "record_catch_spoiled",
                "Some of Jonas's catch spoiled",
                "Should have decided quicker.",
                [mark("record_catch")],
            ),
        ),
        spec(
            "nets_at_midnight",
            follow(JONAS, vec![Marked("nets_torn", 1), Unmarked("nets_help")]),
            (
                "Jonas is mending nets at midnight",
                "Still at these nets. Could anyone lend a hand?",
            ),
            vec![
                other(
                    "help",
                    "Evan lends a hand",
                    "An evening's work for two.",
                    said(
                        "nets_mended_together",
                        "Evan helped Jonas mend his nets",
                        "Done by midnight, with Evan's help.",
                        [mood(1), mark("nets_help")],
                    ),
                ),
                other(
                    "alone",
                    "He'll manage",
                    "Jonas works till dawn.",
                    said(
                        "nets_mended_alone",
                        "Jonas mended his nets alone, till dawn",
                        "Nobody came. Fine.",
                        [mood(-1), mark("nets_help")],
                    ),
                ),
            ],
            said(
                "nets_still_torn",
                "Jonas's nets stayed torn",
                "I'll fish with what I've got.",
                [mark("nets_help")],
            ),
        ),
        spec(
            "reading_aloud",
            follow(MIA, vec![Marked("books", 2), Unmarked("read_aloud")]),
            (
                "Mia finished her new books",
                "I read all my new books! Can I read one to the class?",
            ),
            vec![
                other(
                    "yes",
                    "Read it to everyone",
                    "An afternoon of stories.",
                    said(
                        "reading_aloud",
                        "Mia read her new book to the whole class",
                        "And then the whale said...",
                        [mood(1), mark("read_aloud")],
                    ),
                ),
                other(
                    "later",
                    "Maybe later",
                    "Lessons first.",
                    said(
                        "reading_put_off",
                        "Mia's reading was put off",
                        "Maybe next week.",
                        [mark("read_aloud")],
                    ),
                ),
            ],
            said(
                "reading_forgotten",
                "Nobody asked Mia to read",
                "Never mind.",
                [mark("read_aloud")],
            ),
        ),
        spec(
            "books_borrowed",
            follow(EMMA, vec![Marked("books_shared", 2), Unmarked("library")]),
            (
                "Emma wants a lending shelf",
                "Mia's sharing books. What if the school lent them to everyone?",
            ),
            vec![
                yes(
                    "shelf",
                    "Put up a lending shelf",
                    "Evan builds it; Emma pays him 15.",
                    vec![has(EMMA, 15)],
                    said(
                        "lending_shelf",
                        "The school opened a lending shelf",
                        "Borrow one, bring one back!",
                        pay(EMMA, EVAN, 15).into_iter().chain([
                            mood(1),
                            mark("library"),
                            build(SHELF, "The lending shelf", "stall", SCHOOL, None),
                        ]),
                    ),
                ),
                other(
                    "no",
                    "Not worth it",
                    "Nothing spent.",
                    said(
                        "no_lending_shelf",
                        "The school kept its books to itself",
                        "Fair enough.",
                        [mark("library")],
                    ),
                ),
            ],
            said(
                "lending_idea_forgotten",
                "The lending shelf never happened",
                "Oh well.",
                [mark("library")],
            ),
        ),
        spec(
            "hotel_again",
            follow(MARA, vec![Marked("hotel", 3), Unmarked("hotel_decided")]),
            (
                "The hotel wants a standing order",
                "The hotel liked our bread. They want it every week.",
            ),
            vec![
                other(
                    "yes",
                    "Every week, then",
                    "Mara makes 50 and bakes through the night.",
                    said(
                        "hotel_standing_order",
                        "Mara took a standing order from the mainland hotel",
                        "Every Tuesday, forty loaves.",
                        earn(MARA, 50).into_iter().chain([
                            mark("hotel_decided"),
                            build(CRATES, "Bread crates for the hotel", "parcel", HARBOR, None),
                        ]),
                    ),
                ),
                other(
                    "no",
                    "Once was enough",
                    "The harbour's bread comes first.",
                    said(
                        "hotel_turned_down_again",
                        "Mara said no to the hotel's standing order",
                        "Our own first. Always.",
                        [mood(1), mark("hotel_decided")],
                    ),
                ),
            ],
            said(
                "hotel_went_quiet",
                "The hotel stopped asking",
                "They've found someone else.",
                [mark("hotel_decided")],
            ),
        ),
        spec(
            "quarrel_after",
            follow(EVAN, vec![Marked("quarrel", 2), Unmarked("quarrel_after")]),
            (
                "Evan wants to make peace with Leo",
                "Leo and I are talking again. Drinks on me?",
            ),
            vec![
                yes(
                    "drinks",
                    "A round on Evan",
                    "Evan spends 15 at the Anchor.",
                    vec![has(EVAN, 15)],
                    said(
                        "peace_drink",
                        "Evan and Leo shared a drink",
                        "To old friends!",
                        pay(EVAN, LEO, 15)
                            .into_iter()
                            .chain([mood(2), mark("quarrel_after")]),
                    ),
                ),
                other(
                    "leave_it",
                    "Leave it be",
                    "Some things mend by themselves.",
                    said(
                        "peace_left",
                        "Evan and Leo left it there",
                        "We're fine. Mostly.",
                        [mark("quarrel_after")],
                    ),
                ),
            ],
            said(
                "peace_never_made",
                "Evan and Leo never quite made up",
                "We don't talk about it.",
                [mark("quarrel_after")],
            ),
        ),
        spec(
            "storm_repairs",
            follow(EVAN, vec![Marked("storm_hit", 1), Unmarked("storm_mended")]),
            (
                "The storm cracked the harbour wall",
                "The storm cracked the harbour wall. Mend it now?",
            ),
            vec![
                yes(
                    "mend",
                    "Mend it now",
                    "Noah pays Evan 40.",
                    vec![has(NOAH, 40)],
                    said(
                        "harbour_wall_mended",
                        "Evan mended the harbour wall after the storm",
                        "Good as new. Better.",
                        pay(NOAH, EVAN, 40).into_iter().chain([
                            mood(1),
                            mark("storm_mended"),
                            build(
                                PARCELS,
                                "Stones for the harbour wall",
                                "parcel",
                                HARBOR,
                                Some(2),
                            ),
                        ]),
                    ),
                ),
                other(
                    "later",
                    "It'll hold",
                    "Nothing spent, for now.",
                    said(
                        "harbour_wall_left",
                        "The harbour wall was left cracked",
                        "It'll hold. Probably.",
                        [mood(-1), mark("storm_mended")],
                    ),
                ),
            ],
            said(
                "harbour_wall_crumbled",
                "Part of the harbour wall fell in",
                "Should have mended it.",
                spend(NOAH, 30)
                    .into_iter()
                    .chain([mood(-1), mark("storm_mended")]),
            ),
        ),
        spec(
            "evan_back",
            follow(EVAN, vec![Marked("evan_away", 3), Unmarked("evan_back")]),
            (
                "Evan is back from the mainland",
                "I'm back! And they've a job for me there for good, if I want it.",
            ),
            vec![
                other(
                    "stay",
                    "Stay here",
                    "The harbour keeps its carpenter.",
                    said(
                        "evan_home",
                        "Evan came home to stay",
                        "Home's home.",
                        [
                            mood(2),
                            mark("evan_back"),
                            Effect::Unset {
                                entity: EVAN,
                                key: AWAY,
                            },
                        ],
                    ),
                ),
                other(
                    "go",
                    "Take the job",
                    "Evan leaves for good.",
                    said(
                        "evan_left",
                        "Evan left for the mainland for good",
                        "I'll visit. Often.",
                        [mood(-2), mark("evan_back")],
                    ),
                ),
            ],
            said(
                "evan_drifted_home",
                "Evan drifted home again",
                "Couldn't stay away.",
                [
                    mark("evan_back"),
                    Effect::Unset {
                        entity: EVAN,
                        key: AWAY,
                    },
                ],
            ),
        ),
        spec(
            "boats_relaunch",
            follow(
                JONAS,
                vec![Marked("boats_up", 1), Unmarked("boats_relaunched")],
            ),
            (
                "The storm has passed",
                "Storm's passed. Launch the boats together?",
            ),
            vec![
                other(
                    "together",
                    "All together",
                    "The whole harbour on the slip at dawn.",
                    said(
                        "boats_relaunched_together",
                        "The harbour launched its boats together after the storm",
                        "Heave! And away she goes!",
                        [
                            mood(2),
                            mark("boats_relaunched"),
                            build(
                                BUNTING,
                                "Bunting for the launch",
                                "bunting",
                                HARBOR,
                                Some(1),
                            ),
                        ],
                    ),
                ),
                other(
                    "one_by_one",
                    "One by one",
                    "Each boat when its owner's ready.",
                    said(
                        "boats_relaunched_slowly",
                        "The boats went back in one by one",
                        "No rush.",
                        [mark("boats_relaunched")],
                    ),
                ),
            ],
            said(
                "boats_back_in",
                "The boats drifted back into the water",
                "Back in, somehow.",
                [mark("boats_relaunched")],
            ),
        ),
        spec(
            "warm_snug",
            follow(LEO, vec![Marked("chimney", 2), Unmarked("snug")]),
            (
                "The pub is warm as toast",
                "The new chimney draws so well. Open up the old snug?",
            ),
            vec![
                yes(
                    "open",
                    "Open the snug",
                    "Leo spends 25 on a rug and two armchairs.",
                    vec![has(LEO, 25)],
                    said(
                        "snug_opened",
                        "Leo opened the old snug at the Anchor",
                        "Best seat in the house, by the fire.",
                        spend(LEO, 25).into_iter().chain([
                            mood(1),
                            mark("snug"),
                            build(LANTERNS, "A lamp in the snug window", "lantern", PUB, None),
                        ]),
                    ),
                ),
                other(
                    "no",
                    "Keep it shut",
                    "Nothing spent.",
                    said(
                        "snug_kept_shut",
                        "The snug stayed shut",
                        "Another winter, maybe.",
                        [mark("snug")],
                    ),
                ),
            ],
            said(
                "snug_forgotten",
                "Leo never got round to the snug",
                "One day.",
                [mark("snug")],
            ),
        ),
        spec(
            "quay_benches",
            follow(EVAN, vec![Marked("evan_stayed", 2), Unmarked("benches")]),
            (
                "Evan wants to build benches for the quay",
                "Since I stayed, how about benches for the quay?",
            ),
            vec![
                yes(
                    "build",
                    "Build them",
                    "Noah pays 25 for timber.",
                    vec![has(NOAH, 25)],
                    said(
                        "benches_built",
                        "Evan built benches along the quay",
                        "Sit down, everyone!",
                        spend(NOAH, 25).into_iter().chain([
                            mood(1),
                            mark("benches"),
                            build(BENCHES, "Benches on the quay", "stall", HARBOR, None),
                        ]),
                    ),
                ),
                other(
                    "no",
                    "Stand like everyone else",
                    "Nothing spent.",
                    said(
                        "benches_refused",
                        "The quay stayed bench-less",
                        "Standing's good for you, apparently.",
                        [mark("benches")],
                    ),
                ),
            ],
            said(
                "benches_forgotten",
                "Evan's benches never happened",
                "Maybe next year.",
                [mark("benches")],
            ),
        ),
        spec(
            "doctor_thanks",
            follow(EMMA, vec![Marked("doctor", 2), Unmarked("thanked_doctor")]),
            (
                "Mia is better",
                "Mia's better! Shall we send the doctor something?",
            ),
            vec![
                other(
                    "cake",
                    "Send a cake",
                    "Mara bakes one for the crossing.",
                    said(
                        "doctor_cake",
                        "The harbour sent the doctor a cake",
                        "With our thanks!",
                        [mood(1), mark("thanked_doctor")],
                    ),
                ),
                other(
                    "letter",
                    "A letter will do",
                    "Mia writes it herself.",
                    said(
                        "doctor_letter",
                        "Mia wrote the doctor a thank-you letter",
                        "Dear Doctor...",
                        [mark("thanked_doctor")],
                    ),
                ),
            ],
            said(
                "doctor_unthanked",
                "Nobody thanked the doctor",
                "Oh, we forgot.",
                [mark("thanked_doctor")],
            ),
        ),
        spec(
            "music_elsewhere",
            follow(
                LEO,
                vec![Marked("music_refused", 2), Unmarked("music_elsewhere")],
            ),
            (
                "The fiddler is playing the mainland pub",
                "That fiddler's packing out the mainland pub now. Get him back?",
            ),
            vec![
                yes(
                    "invite",
                    "Invite him back",
                    "Leo spends 45 to win him over.",
                    vec![has(LEO, 45)],
                    said(
                        "fiddler_won_back",
                        "The fiddler came back to the Anchor",
                        "He's back, and the place is full!",
                        spend(LEO, 45).into_iter().chain([
                            mood(2),
                            mark("music_elsewhere"),
                            build(BUNTING, "Bunting for the fiddler", "bunting", PUB, Some(3)),
                        ]),
                    ),
                ),
                other(
                    "let_go",
                    "Let him go",
                    "The Anchor stays quiet.",
                    said(
                        "fiddler_let_go",
                        "Leo let the fiddler go",
                        "Their loss. Or ours.",
                        [mood(-1), mark("music_elsewhere")],
                    ),
                ),
            ],
            said(
                "fiddler_gone",
                "The fiddler stayed on the mainland",
                "He's not coming back.",
                [mark("music_elsewhere")],
            ),
        ),
        spec(
            "oven_breaks",
            follow(
                MARA,
                vec![Marked("oven_patched", 2), Unmarked("oven_broke")],
            ),
            (
                "Mara's patched oven gave out",
                "The patched oven's died mid-batch. Now what?",
            ),
            vec![
                yes(
                    "buy",
                    "Buy a new one now",
                    "80 of Mara's savings, at last.",
                    vec![has(MARA, 80)],
                    said(
                        "oven_bought",
                        "Mara's new oven arrived",
                        "Should have done this weeks ago.",
                        spend(MARA, 80)
                            .into_iter()
                            .chain([mood(1), mark("oven_broke")]),
                    ),
                ),
                other(
                    "pub_range",
                    "Bake on the pub's range",
                    "Leo lends his kitchen for a week.",
                    said(
                        "baking_at_the_pub",
                        "Mara baked on the Anchor's range",
                        "Bread and ale, same roof.",
                        [
                            mood(-1),
                            mark("oven_broke"),
                            build(
                                PARCELS,
                                "Bread trays at the pub door",
                                "parcel",
                                PUB,
                                Some(3),
                            ),
                        ],
                    ),
                ),
            ],
            said(
                "no_bread",
                "The harbour went a day without bread",
                "No bread today. Sorry.",
                [mood(-2), mark("oven_broke")],
            ),
        ),
        spec(
            "fever_worse",
            follow(
                EMMA,
                vec![Marked("fever_rest", 2), Unmarked("fever_decided")],
            ),
            (
                "Mia is no better",
                "Mia's no better. Send for the doctor now?",
            ),
            vec![
                yes(
                    "doctor",
                    "Send for the doctor",
                    "Emma pays 45 for the crossing.",
                    vec![has(EMMA, 45)],
                    said(
                        "doctor_came_late",
                        "The doctor came for Mia at last",
                        "She'll be fine now.",
                        spend(EMMA, 45).into_iter().chain([
                            mood(1),
                            mark("fever_decided"),
                            build(PARCELS, "The doctor's trunk", "parcel", SCHOOL, Some(2)),
                        ]),
                    ),
                ),
                other(
                    "wait",
                    "Give it another day",
                    "A long night for Emma.",
                    said(
                        "fever_waited",
                        "Emma waited out Mia's fever",
                        "Her fever's broken. Thank goodness.",
                        [mood(-1), mark("fever_decided")],
                    ),
                ),
            ],
            said(
                "fever_broke",
                "Mia's fever broke on its own",
                "Over the worst.",
                [mark("fever_decided")],
            ),
        ),
        spec(
            "fete_grumbles",
            follow(
                NOAH,
                vec![Marked("fete_skipped", 2), Unmarked("fete_grumbled")],
            ),
            (
                "People miss the fête",
                "People are grumbling about no fête. Something small?",
            ),
            vec![
                yes(
                    "picnic",
                    "A picnic on the quay",
                    "Noah pays 20 for lemonade.",
                    vec![has(NOAH, 20)],
                    said(
                        "quay_picnic",
                        "The harbour had a picnic on the quay",
                        "Not a fête, but it'll do!",
                        spend(NOAH, 20).into_iter().chain([
                            mood(2),
                            mark("fete_grumbled"),
                            build(
                                BUNTING,
                                "Bunting for the picnic",
                                "bunting",
                                HARBOR,
                                Some(2),
                            ),
                        ]),
                    ),
                ),
                other(
                    "nothing",
                    "Let them grumble",
                    "Nothing spent.",
                    said(
                        "grumbles_ignored",
                        "Noah let the grumbling be",
                        "They'll get over it.",
                        [mood(-1), mark("fete_grumbled")],
                    ),
                ),
            ],
            said(
                "grumbles_faded",
                "The grumbling faded",
                "Nobody mentions it now.",
                [mark("fete_grumbled")],
            ),
        ),
        spec(
            "chimney_smokes",
            follow(
                LEO,
                vec![Marked("chimney_patched", 2), Unmarked("chimney_fixed")],
            ),
            (
                "The patched chimney smokes the pub out",
                "The patched chimney's smoking us out. Rebuild it properly?",
            ),
            vec![
                yes(
                    "rebuild",
                    "Rebuild it properly",
                    "Leo pays Evan 40.",
                    vec![has(LEO, 40)],
                    said(
                        "chimney_rebuilt_properly",
                        "Evan rebuilt the pub chimney properly",
                        "Draws like a dream now.",
                        pay(LEO, EVAN, 40).into_iter().chain([
                            mood(1),
                            mark("chimney_fixed"),
                            mark("chimney"),
                        ]),
                    ),
                ),
                other(
                    "windows",
                    "Open the windows",
                    "Smoke and draughts.",
                    said(
                        "pub_windows_open",
                        "The Anchor kept its windows open",
                        "Bit breezy. Bit smoky.",
                        [mood(-1), mark("chimney_fixed")],
                    ),
                ),
            ],
            said(
                "chimney_smoked_on",
                "The pub chimney smoked on",
                "Cough cough.",
                [mood(-1), mark("chimney_fixed")],
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
