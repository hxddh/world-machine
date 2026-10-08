//! Talk you can trust, in the harbour (v0.28): words on a topic no
//! resident talks about are answered with care and never sent to a model;
//! a line can be reported, and the report is an Event caused by it; a
//! model's answer is shown as one; and two or three openers stand beside
//! the words the player can type.

use crate::{TinySocietyBranch, MARA};
use world_pack_testkit::red_team::Proposes;
use world_projection::SelectionId;

fn branch() -> TinySocietyBranch {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    branch.begin_story().unwrap();
    branch
}

#[test]
fn a_crisis_is_answered_with_care_and_never_reaches_a_model() {
    let mut branch = branch();
    let world = branch.world();
    let kit = crate::speech::kit(world.state());
    for (words, help) in [
        ("I want to kill myself", "findahelpline.com"),
        ("我真的不想活了", "热线"),
        ("もう死にたい", "相談窓口"),
    ] {
        assert!(conversation::hearing_for(branch.world(), &kit, MARA, words).is_none());
        branch
            .say_with(
                MARA,
                words,
                &mut Proposes("Oh, don't be silly.".into(), None),
            )
            .unwrap();
        let exchange = conversation::exchanges_today(branch.world()).pop().unwrap();
        assert!(exchange.reply.contains(help), "{words}: {}", exchange.reply);
        assert!(!exchange.voiced);
    }
}

#[test]
fn a_line_is_reported_as_an_event_caused_by_it_and_a_models_answer_is_shown_as_one() {
    let mut branch = branch();
    branch
        .say_with(
            MARA,
            "How's the bread today?",
            &mut Proposes("Golden, and still warm.".into(), None),
        )
        .unwrap();
    let snapshot = branch.projection_snapshot();
    let exchange = snapshot.exchanges.last().unwrap().clone();
    assert!(exchange.voiced && !exchange.reported);
    let SelectionId::Event(spoken) = exchange.moment else {
        panic!("an exchange is its Event");
    };
    let ids = branch
        .invoke_projection_command(&conversation::report::command(spoken))
        .unwrap();
    let report = branch.world().event(ids[0]).unwrap();
    assert_eq!(report.kind, conversation::report::REPORTED);
    assert_eq!(report.caused_by, vec![spoken]);
    assert!(
        branch
            .projection_snapshot()
            .exchanges
            .last()
            .unwrap()
            .reported
    );
    // A line is reported once, and only a line can be.
    assert!(branch
        .invoke_projection_command(&conversation::report::command(spoken))
        .is_err());
    assert!(branch
        .invoke_projection_command(&conversation::report::command(ids[0]))
        .is_err());
    // Replay re-applies the report from its record.
    let replayed = branch.world().replay().unwrap();
    assert_eq!(replayed.state(), branch.world().state());
}

#[test]
fn openers_stand_beside_the_words_for_everyone_the_player_can_talk_to() {
    let branch = branch();
    let snapshot = branch.projection_snapshot();
    let mara = snapshot
        .openers
        .iter()
        .find(|openers| openers.who == SelectionId::Entity(MARA))
        .expect("openers for Mara");
    assert!((2..=3).contains(&mara.lines.len()), "{:?}", mara.lines);
    // Every opener, in every language the harbour speaks, is in its
    // catalogs.
    for template in conversation::openers::ALL {
        for (lang, catalog) in [("zh", crate::ZH_HANS), ("ja", crate::JA)] {
            assert!(
                catalog
                    .lines()
                    .any(|line| line.split('\t').next() == Some(template)),
                "{template} has no {lang} line"
            );
        }
    }
}
