//! Every kind of scene staged from one Event made by hand: who walks
//! where, what props stand about, and nothing said but what was recorded.

use super::*;
use crate::scene;
use world_projection::{Act, Mark, SceneScript, Stance, MOST_SCENE_SECONDS};

const DEE: EntityId = EntityId::new(4);
const BENCH: EntityId = EntityId::new(200);

fn set(entity: EntityId, key: &str, value: Value) -> StateChange {
    StateChange::SetComponent {
        entity,
        key: key.into(),
        value,
    }
}

/// The one an Event is about, as `lives` records it.
fn about(mut draft: Draft, who: EntityId) -> Draft {
    draft.payload.push(("who", Value::Entity(who)));
    draft
}

/// One Event of each kind of scene, by hand, and the kind each plays as.
fn every_kind() -> (Toy, Vec<MomentKind>) {
    let drafts = vec![
        (
            draft("drawn_here", 1)
                .by(CY)
                .to(CY)
                .with("told", "Cy came because of the pier you built")
                .with("said", "What a lovely spot!")
                .changing(StateChange::CreateEntity(person(CY, "Cy"))),
            MomentKind::Arrival,
        ),
        (
            draft("bond_changed", 2)
                .by(ANA)
                .to(BO)
                .with("bond", "fell_out")
                .with("told", "Ana and Bo fell out")
                .with("said", "I'm done with Bo."),
            MomentKind::Quarrel,
        ),
        (
            draft("lived", 3)
                .by(ANA)
                .to(BO)
                .with("told", "Ana went fishing with Bo, and they had words")
                .with("said", "Bo and I had words over the nets.")
                .changing(set(ANA, lives::ROW, Value::Text("2:nets".into()))),
            MomentKind::Quarrel,
        ),
        (
            draft("bond_changed", 4)
                .by(BO)
                .to(ANA)
                .with("bond", "made_up")
                .with("told", "Bo and Ana made it up")
                .with("said", "Ana said sorry. So did I."),
            MomentKind::MakingUp,
        ),
        (
            draft("invited_out", 5)
                .to(ANA)
                .with("told", "You invited Ana to the square")
                .changing(set(ANA, lives::AT, Value::Entity(SQUARE))),
            MomentKind::Outing,
        ),
        (
            draft("built_by_hand", 6)
                .to(BENCH)
                .to(SQUARE)
                .with("told", "You built a bench by the square")
                .changing(StateChange::CreateEntity(
                    Entity::new(BENCH, "fixture").with_component("name", "Bench"),
                )),
            MomentKind::Build,
        ),
        (
            draft("festival_held", 7)
                .by(BO)
                .to(SQUARE)
                .with("name", "the Lantern Walk")
                .with("told", "The square had its Lantern Walk")
                .with("said", "Lanterns up!")
                .changing(set(ANA, "fun", Value::Integer(1)))
                .changing(set(BO, "fun", Value::Integer(1))),
            MomentKind::Festival,
        ),
        (draft("storm_weathered", 8).by(BO).to(BO), MomentKind::Storm),
        (
            draft("born", 9)
                .by(ANA)
                .to(DEE)
                .with("told", "Dee was born to Ana")
                .with("said", "Look at her hands!")
                .changing(StateChange::CreateEntity(person(DEE, "Dee"))),
            MomentKind::Birth,
        ),
        (
            draft("year_turned", 10)
                .by(ANA)
                .to(BO)
                .with("told", "Ana and Bo were married")
                .changing(set(ANA, "years.married", Value::Entity(BO))),
            MomentKind::Wedding,
        ),
        (
            draft("came_of_age", 11)
                .by(DEE)
                .to(DEE)
                .with("told", "Dee came of age"),
            MomentKind::ComingOfAge,
        ),
        (
            draft("retired", 12)
                .by(BO)
                .to(BO)
                .with("told", "Bo handed on the nets")
                .with("said", "Your turn now."),
            MomentKind::Farewell,
        ),
        (
            about(draft("died", 13), ANA)
                .by(CY)
                .to(ANA)
                .with("told", "Ana died, an old woman")
                .with("said", "She knew every boat by name."),
            MomentKind::Death,
        ),
        (
            draft("greeted", 14)
                .by(BO)
                .to(CY)
                .with("told", "Bo came over to say hello"),
            MomentKind::Other,
        ),
    ];
    let kinds = drafts.iter().map(|(_, kind)| *kind).collect();
    (
        world(drafts.into_iter().map(|(draft, _)| draft).collect()),
        kinds,
    )
}

fn staged(script: &SceneScript) -> Vec<SelectionId> {
    script.cues.iter().filter_map(|cue| cue.who).collect()
}

fn holds(script: &SceneScript, event: &Event) {
    assert_eq!(script.event, event.id);
    assert_eq!(script.id, format!("scene-{}", event.id));
    assert!(script.seconds > 0.0 && script.seconds <= MOST_SCENE_SECONDS);
    assert!(!script.cues.is_empty(), "{script:?}");
    let mut last = 0.0;
    for cue in &script.cues {
        assert!(cue.at >= last && cue.at < script.seconds, "{script:?}");
        last = cue.at;
    }
    // Everyone a cue moves is in the cast.
    for who in staged(script) {
        assert!(
            script.cast.iter().any(|(cast, _)| *cast == who),
            "{who:?} in {script:?}"
        );
    }
    // Nothing is said but what the Event recorded, by whoever said it.
    let speech = script.speech();
    match text(event, "said") {
        Some(said) => {
            assert_eq!(speech.len(), 1, "{script:?}");
            assert_eq!(speech[0].1, said);
            if let Some(who) = speech[0].0 {
                assert_eq!(Some(who), event.actor.map(SelectionId::Entity));
            }
        }
        None => assert!(speech.is_empty(), "{script:?}"),
    }
    assert_eq!(script.caption, text(event, "told").unwrap_or_default());
}

#[test]
fn every_kind_of_scene_is_staged_from_its_event() {
    let (toy, kinds) = every_kind();
    let world = toy.world();
    for (event, kind) in world.events().iter().zip(kinds) {
        let script = scene::scene(&toy, event.id).unwrap();
        assert_eq!(script.kind, kind, "{}", event.kind);
        holds(&script, event);
        // The same Event stages the same scene every time.
        assert_eq!(scene::scene(&toy, event.id), Some(script.clone()));
        let walks_to = |who: EntityId, to: Mark| {
            script.cues.iter().any(|cue| {
                cue.who == Some(SelectionId::Entity(who))
                    && matches!(cue.act, Act::WalkTo { to: mark, .. } if mark == to)
            })
        };
        let shows = |prop: Prop| {
            script
                .cues
                .iter()
                .any(|cue| matches!(cue.act, Act::Prop { prop: p, shown: true, .. } if p == prop))
        };
        let strikes = |stance: Stance| script.cues.iter().any(|cue| cue.act == Act::Strike(stance));
        let square = Some(SelectionId::Entity(SQUARE));
        match kind {
            MomentKind::Arrival => {
                assert_eq!(script.cast[0].0, SelectionId::Entity(CY));
                assert!(walks_to(CY, Mark::Offstage(false)));
                assert!(walks_to(CY, Mark::Place(SelectionId::Entity(SQUARE))));
                assert!(shows(Prop::Suitcase));
            }
            MomentKind::Quarrel => {
                assert!(walks_to(BO, Mark::Home(SelectionId::Entity(BO))));
            }
            MomentKind::MakingUp => assert!(strikes(Stance::Celebrating)),
            MomentKind::Outing => {
                assert_eq!(script.place, square);
                assert!(walks_to(ANA, Mark::Home(SelectionId::Entity(ANA))));
                assert!(walks_to(ANA, Mark::Place(SelectionId::Entity(SQUARE))));
            }
            MomentKind::Build => {
                assert_eq!(script.place, square);
                assert!(shows(Prop::Scaffold));
                // A bench is made at once: its scaffolding comes down.
                assert!(script.cues.iter().any(|cue| matches!(
                    cue.act,
                    Act::Prop {
                        prop: Prop::Scaffold,
                        shown: false,
                        ..
                    }
                )));
            }
            MomentKind::Festival => {
                assert_eq!(script.place, square);
                assert!(shows(Prop::Bunting));
                assert!(strikes(Stance::Dancing));
                assert_eq!(script.cast.len(), 2);
            }
            MomentKind::Storm => assert!(script
                .cues
                .iter()
                .any(|cue| cue.act == Act::Weather(Prop::Rain))),
            MomentKind::Birth => {
                assert_eq!(script.cast[0].0, SelectionId::Entity(DEE));
                assert!(shows(Prop::Cradle));
            }
            MomentKind::Wedding => {
                assert!(shows(Prop::Bunting) && shows(Prop::Bouquet));
            }
            MomentKind::ComingOfAge => assert!(strikes(Stance::Working)),
            MomentKind::Farewell => {
                assert!(walks_to(BO, Mark::Offstage(true)));
            }
            MomentKind::Death => {
                // The one who died is named, not walked on.
                assert_eq!(script.cast[0].0, SelectionId::Entity(ANA));
                assert!(!staged(&script).contains(&SelectionId::Entity(ANA)));
                assert!(shows(Prop::Wreath));
            }
            _ => {}
        }
    }
    assert_eq!(scene::scene(&toy, EventId::new(999)), None);
}

#[test]
fn a_work_opened_is_cut_open_and_a_moments_props_are_shown() {
    let (toy, _) = every_kind();
    let event = &toy.world().events()[0];
    let mut beat = scene::beat(&toy, event);
    beat.kind = MomentKind::WorkOpened;
    beat.props[1] = vec![Prop::Ferry, Prop::Lamp];
    let script = scene::script(&beat);
    assert_eq!(script.kind, MomentKind::WorkOpened);
    holds(&script, event);
    assert!(script.cues.iter().any(|cue| matches!(
        cue.act,
        Act::Prop {
            prop: Prop::Ribbon,
            shown: false,
            ..
        }
    )));
    // A lamp the moment shows stands at the place; a ferry is only ever
    // how people come and go.
    assert!(script.cues.iter().any(|cue| matches!(
        cue.act,
        Act::Prop {
            prop: Prop::Lamp,
            shown: true,
            ..
        }
    )));
    assert!(!script.cues.iter().any(|cue| matches!(
        cue.act,
        Act::Prop {
            prop: Prop::Ferry,
            ..
        }
    )));
}
