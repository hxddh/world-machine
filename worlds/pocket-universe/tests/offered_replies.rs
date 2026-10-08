//! Pocket Universe's favours, as Tiny Society's: the favour's quick reply is a structured intent (`Ears::Offered`): a
//! click does the favour it offers, in whatever words the player's button
//! showed, and never goes through the hearing. The same words typed are
//! heard like any others.

use world_projection::{CommandRole, Ears, ProjectionIntent, SelectionId};

/// Which kind of favour is open, by the reply the place offers for it.
fn kind_of(reply: &str) -> &'static str {
    if reply.contains("looking in on you") {
        "ask_after"
    } else if reply.contains("come out") {
        "invite"
    } else if reply.contains("great job") {
        "cheer_up"
    } else if reply.contains("sorry") {
        "sorry"
    } else {
        panic!("a reply of no kind: {reply}")
    }
}

#[test]
fn a_clicked_quick_reply_does_the_favour_whatever_its_words() {
    let registry = {
        let mut registry = world_host::WorldRegistry::new();
        registry
            .register(pocket_universe::pocket_universe_registration())
            .unwrap();
        registry
    };
    let mut session = registry.create("world-machine.pocket-universe").unwrap();
    let mut snapshot = session
        .handle(ProjectionIntent::InvokeCommand(
            pocket_universe::SEED_1980S_TOWN_COMMAND.into(),
        ))
        .unwrap();
    let pass = snapshot
        .commands
        .iter()
        .find(|command| command.role == Some(CommandRole::PassesTime))
        .expect("the place says which choice lets time pass")
        .id
        .clone();
    let mut done = std::collections::BTreeSet::new();
    for day in 0..400 {
        if done.len() == 3 {
            break;
        }
        if let Some(favour) = snapshot.favour.clone().filter(|favour| !favour.done) {
            let kind = kind_of(&favour.reply);
            // An invitation can be turned down whatever is said; the other
            // kinds are done by the reply alone.
            if kind != "invite" && !done.contains(kind) {
                let SelectionId::Entity(_) = favour.whom else {
                    panic!("a favour for nobody")
                };
                // Typed, words the place's ears cannot make out do nothing.
                snapshot = session
                    .handle(ProjectionIntent::Say {
                        to: favour.whom,
                        words: "Zxq vlorp?".into(),
                        ears: Ears::Own,
                    })
                    .unwrap();
                assert!(
                    snapshot.favour.as_ref().is_some_and(|open| !open.done),
                    "{kind}: unclear words did a favour"
                );
                // Clicked, the button's words (here, as a translation might
                // put them) do it.
                snapshot = session
                    .handle(ProjectionIntent::Say {
                        to: favour.whom,
                        words: "按钮上的话。".into(),
                        ears: Ears::Offered,
                    })
                    .unwrap();
                assert!(
                    snapshot.favour.as_ref().is_some_and(|after| after.done),
                    "{kind}: the clicked quick reply did not do the favour"
                );
                done.insert(kind);
            }
        }
        // A word now and then keeps the favours coming.
        if day % 9 == 0 {
            if let Some(someone) = snapshot.favour.as_ref().map(|favour| favour.asker) {
                let _ = session.handle(ProjectionIntent::Say {
                    to: someone,
                    words: "Morning!".into(),
                    ears: Ears::Own,
                });
            }
        }
        snapshot = session
            .handle(ProjectionIntent::InvokeCommand(pass.clone()))
            .unwrap();
    }
    assert_eq!(done.len(), 3, "kinds done by a click: {done:?}");
}

/// With no favour open for whom the words are said to, an offered reply
/// is just words, heard like any others: nothing on the screen is trusted
/// but who was clicked.
#[test]
fn an_offered_reply_with_no_favour_open_is_heard_as_words() {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    let mut session = registry.create("world-machine.pocket-universe").unwrap();
    let snapshot = session
        .handle(ProjectionIntent::InvokeCommand(
            pocket_universe::SEED_1980S_TOWN_COMMAND.into(),
        ))
        .unwrap();
    assert!(
        snapshot.favour.is_none(),
        "a favour open on the first morning"
    );
    let someone = snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.kind == world_projection::CanvasItemKind::Actor)
        .map(|item| item.id)
        .expect("someone on the first morning");
    let after = session
        .handle(ProjectionIntent::Say {
            to: someone,
            words: "Good morning!".into(),
            ears: Ears::Offered,
        })
        .unwrap();
    assert!(after.favour.is_none());
    // Said and answered like typed words.
    assert!(after.exchanges_with(someone).next().is_some());
}
