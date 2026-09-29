use super::*;
use std::collections::BTreeMap;
use world_core::{Entity, WorldState};

const ANA: EntityId = EntityId::new(1);
const BO: EntityId = EntityId::new(2);
const CY: EntityId = EntityId::new(3);
const SQUARE: EntityId = EntityId::new(100);
const DAY: u64 = 10;

struct Toy(World);

impl Teller for Toy {
    fn world(&self) -> &World {
        &self.0
    }
    fn day_length(&self) -> u64 {
        DAY
    }
    fn names(&self) -> &[&'static str] {
        &["Ana", "Bo", "Cy"]
    }
    fn title(&self, subject: EntityId) -> Option<String> {
        self.0
            .state()
            .entity(subject)
            .map(|_| lives::name(self.0.state(), subject))
    }
    fn line(&self, _: EntityId, event: &Event) -> Option<String> {
        text(event, "told").map(str::to_string)
    }
    fn naming_kinds(&self) -> &[&'static str] {
        &[
            "situation_came_up",
            "situation_answered",
            "bond_changed",
            "greeted",
            "roof_mended",
            "reacted",
        ]
    }
    fn storylet_answer(&self, event: &Event) -> Option<Answered> {
        (text(event, "choice") == Some("help")).then(|| Answered::Said("Let's help".into()))
    }
    fn storm_kinds(&self) -> &[&'static str] {
        &["storm_weathered"]
    }
    fn cause_words(&self, cause: &Event) -> Option<String> {
        (cause.kind == "storm_weathered").then(|| "after the great storm".into())
    }
    fn is_person(&self, id: EntityId) -> bool {
        id != SQUARE
    }
}

struct Draft {
    kind: &'static str,
    day: u64,
    actor: Option<EntityId>,
    targets: Vec<EntityId>,
    caused_by: Vec<u64>,
    payload: Vec<(&'static str, Value)>,
    changes: Vec<StateChange>,
}

fn draft(kind: &'static str, day: u64) -> Draft {
    Draft {
        kind,
        day,
        actor: None,
        targets: vec![],
        caused_by: vec![],
        payload: vec![],
        changes: vec![],
    }
}

impl Draft {
    fn by(mut self, who: EntityId) -> Self {
        self.actor = Some(who);
        self
    }
    fn to(mut self, who: EntityId) -> Self {
        self.targets.push(who);
        self
    }
    fn with(mut self, key: &'static str, value: &str) -> Self {
        self.payload.push((key, Value::Text(value.into())));
        self
    }
    fn after(mut self, id: u64) -> Self {
        self.caused_by.push(id);
        self
    }
    fn changing(mut self, change: StateChange) -> Self {
        self.changes.push(change);
        self
    }
}

fn person(id: EntityId, name: &str) -> Entity {
    Entity::new(id, "person").with_component("name", name)
}

fn world(drafts: Vec<Draft>) -> Toy {
    let mut state = WorldState::default();
    state.seed_entity(person(ANA, "Ana")).unwrap();
    state.seed_entity(person(BO, "Bo")).unwrap();
    state
        .seed_entity(Entity::new(SQUARE, "place").with_component("name", "the square"))
        .unwrap();
    let events = drafts
        .into_iter()
        .enumerate()
        .map(|(at, draft)| Event {
            id: EventId::new(at as u64 + 1),
            kind: draft.kind.into(),
            world_time: draft.day * DAY,
            actor: draft.actor,
            targets: draft.targets,
            caused_by: draft.caused_by.into_iter().map(EventId::new).collect(),
            payload: draft
                .payload
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect::<BTreeMap<_, _>>(),
            changes: draft.changes,
        })
        .collect::<Vec<_>>();
    Toy(World::from_history(state, &events).unwrap())
}

fn history() -> Toy {
    world(vec![
        // 1: a quarrel comes up between Ana and Bo...
        draft("situation_came_up", 2)
            .by(ANA)
            .to(BO)
            .with("kind", "feud")
            .with("situation", "feud.1.2.7")
            .with("told", "Ana and Bo fell out over the nets"),
        // 2: ...and the player makes peace.
        draft("situation_answered", 3)
            .by(ANA)
            .to(BO)
            .with("kind", "feud")
            .with("answer", "mend")
            .with("situation", "feud.1.2.7")
            .with("told", "Ana and Bo talked it through"),
        // 3: they become friends a few days on.
        draft("bond_changed", 5)
            .by(ANA)
            .to(BO)
            .with("told", "Ana and Bo became friends"),
        // 4: a festival...
        draft("festival_held", 9)
            .by(BO)
            .to(SQUARE)
            .with("name", "the Lantern Walk")
            .with("told", "The square had its Lantern Walk"),
        // 5: ...where Ana says hello to someone.
        draft("greeted", 9)
            .by(ANA)
            .to(ANA)
            .with("told", "Ana came over to say hello"),
        // 6: a question of the storyteller's...
        draft("situation_arose", 12)
            .by(BO)
            .to(BO)
            .with("storylet", "roof"),
        // 7: ...answered: the player said "Let's help".
        draft("roof_mended", 13)
            .by(BO)
            .to(BO)
            .with("storylet", "roof")
            .with("choice", "help")
            .with("told", "Bo mended the roof"),
        // 8: a storm, and 9: what it recorded it caused.
        draft("storm_weathered", 20).by(BO).to(BO),
        draft("reacted", 21)
            .by(BO)
            .to(BO)
            .after(8)
            .with("told", "Bo patched the boat"),
        // 10: Cy moves in; 11: Bo leaves.
        draft("year_turned", 130)
            .by(CY)
            .to(CY)
            .with("told", "Cy moved in")
            .changing(StateChange::CreateEntity(person(CY, "Cy"))),
        draft("year_turned", 140)
            .by(BO)
            .to(BO)
            .with("told", "Bo moved away")
            .changing(StateChange::SetComponent {
                entity: BO,
                key: lives::GONE.into(),
                value: Value::Bool(true),
            }),
    ])
}

#[test]
fn every_line_says_what_brought_it_about_from_what_was_recorded() {
    let toy = history();
    let legend = legend(&toy, SelectionId::Entity(ANA)).unwrap();
    assert_eq!(legend.title, "Ana");
    let because = legend
        .lines
        .iter()
        .map(|line| (line.text.as_str(), line.because.as_deref()))
        .collect::<Vec<_>>();
    assert_eq!(
        because,
        [
            (
                "Ana and Bo talked it through",
                Some("because you said “Make peace”")
            ),
            (
                "Ana and Bo became friends",
                Some("because you said “Make peace”")
            ),
            ("Ana came over to say hello", Some("at the Lantern Walk")),
        ]
    );
    // What was only asked, and never answered, is no part of a life.
    assert!(legend.lines.iter().all(|line| !line.text.contains("nets")));
    // Each cause is an Event this World recorded, never later than the line.
    for line in &legend.lines {
        match (&line.because, line.cause) {
            (Some(_), Some(cause)) => {
                assert!(cause <= line.event.unwrap());
                assert!(toy.0.event(cause).is_some());
            }
            (None, None) => {}
            other => panic!("a cause with no words, or words with none: {other:?}"),
        }
    }
}

#[test]
fn a_choice_names_the_words_the_player_chose_and_a_chain_its_first_cause() {
    let toy = history();
    let bo = legend(&toy, SelectionId::Entity(BO)).unwrap();
    let line = |text: &str| bo.lines.iter().find(|line| line.text == text).unwrap();
    assert_eq!(
        line("Bo mended the roof").because.as_deref(),
        Some("because you said “Let's help”")
    );
    assert_eq!(line("Bo mended the roof").cause, Some(EventId::new(6)));
    assert_eq!(
        line("Bo patched the boat").because.as_deref(),
        Some("after the great storm")
    );
    assert!(legend(&toy, SelectionId::Event(EventId::new(1))).is_none());
}

#[test]
fn a_year_names_who_came_and_who_left_and_its_best_moment() {
    let toy = history();
    let world = &toy.0;
    let beat = |id: u64, kind| Beat {
        event: world.event(EventId::new(id)).unwrap(),
        kind,
        title: "t".into(),
        cast: vec![BO],
        place: Some(SQUARE),
        captions: ["a".into(), "b".into(), "c".into()],
        moods: [None, Some(Mood::Happy), None],
    };
    let all = moments(
        vec![
            beat(10, MomentKind::Other),
            beat(11, MomentKind::Farewell),
            beat(8, MomentKind::Storm),
        ],
        DAY,
    );
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].id, "moment-8");
    assert_eq!(all[0].day, 20);
    assert_eq!(all[0].cast(), [SelectionId::Entity(BO)]);
    let year = Year::of(2, 120, DAY);
    assert!(year.holds(130 * DAY) && year.holds(120 * DAY) && !year.holds(119 * DAY));
    let page = almanac(&toy, year, "Year two".into(), &[ANA, BO, CY], vec![], &all);
    assert_eq!(page.arrived, ["Cy"]);
    assert_eq!(page.left, ["Bo"]);
    assert_eq!(page.best.as_deref(), Some("moment-11"));
    assert_eq!(page.cast.len(), 2);
    let first = almanac(
        &toy,
        Year::of(1, 120, DAY),
        String::new(),
        &[ANA, BO, CY],
        vec![],
        &all,
    );
    assert!(first.arrived.is_empty());
    assert_eq!(first.best.as_deref(), Some("moment-8"));
}

#[test]
fn a_variant_is_the_same_for_the_same_seed() {
    let options = ["a", "b", "c", "d"];
    assert_eq!(pick(7, &options), pick(7, &options));
    let seen = (0..40)
        .map(|seed| pick(seed, &options))
        .collect::<BTreeSet<_>>();
    assert_eq!(seen.len(), 4);
}
