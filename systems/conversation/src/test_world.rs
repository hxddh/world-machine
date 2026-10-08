//! A small World for this System's own tests: Mara and Leo by the
//! sea, who do not get on, and the bakery.

use crate::*;
use world_core::Entity;

pub(crate) const MARA: EntityId = EntityId::new(1);
pub(crate) const LEO: EntityId = EntityId::new(2);
pub(crate) const BAKERY: EntityId = EntityId::new(10);

pub(crate) fn kit(_: &WorldState) -> Kit {
    Kit {
        period: 10,
        unit: "day",
        settlement: "the cove",
        people: |state| {
            [MARA, LEO]
                .into_iter()
                .filter(|id| state.entity(*id).is_some())
                .collect()
        },
        places: |_| vec![BAKERY],
        place_line: |_, _, _| "The bakery's doing fine.".into(),
        need_line: |_, _| ("Flour, mostly.".into(), Some("tiny.flour".into())),
        work_line: |_, _| Some("Up at four for the bread.".into()),
        place_mood: |_| "The cove's quiet.".into(),
        coming_up: |_| None,
        aliases: |_| Vec::new(),
        weather: |_| "Grey and wet.".into(),
        occasions: |_| Vec::new(),
        recalled: |_, _, _| None,
        era: Era::Radio,
        elsewhere: &[],
        lexicon: crate::no_lexicon,
        era_has: &[],
        era_lacks: &[],
    }
}

fn person(id: EntityId, name: &str, other: EntityId) -> Entity {
    Entity::new(id, "person")
        .with_component("name", name)
        .with_component("lives.traits", "warm steady")
        .with_component(lives::REGARD, 10_i64)
        .with_component(Need::Company.key(), 70_i64)
        .with_component(format!("lives.opinion.{}", other.0), -40_i64)
}

pub(crate) fn world() -> (World, ActionRegistry) {
    let mut state = WorldState::default();
    state.seed_entity(person(MARA, "Mara", LEO)).unwrap();
    state.seed_entity(person(LEO, "Leo", MARA)).unwrap();
    state
        .seed_entity(Entity::new(BAKERY, "place").with_component("name", "Cove Bakery"))
        .unwrap();
    let mut actions = ActionRegistry::default();
    register_actions(&mut actions, kit).unwrap();
    (World::new(state), actions)
}

#[test]
fn every_opener_is_heard_as_what_it_says() {
    let (world, _) = world();
    let kit = kit(world.state());
    let heard = |words: &str| hear(world.state(), &kit, MARA, words).intent;
    // Asking after someone low is asking how they are, or what troubles
    // them: either is what it says.
    let after_them = [Intent::HowAreYou, Intent::Worry];
    assert!(after_them.contains(&heard(openers::HOLDING_UP)));
    assert_eq!(
        heard(&openers::THINK_OF.replace("{name}", "Leo")),
        Intent::ThinkOf
    );
    assert_eq!(heard(openers::NEED), Intent::Need);
    assert_eq!(heard(openers::WORK), Intent::Work);
    assert_eq!(heard(openers::NEWS), Intent::News);
    // As the catalogs say them in Chinese and Japanese.
    for (words, intent) in [
        ("你看起来很累。还撑得住吗？", Intent::Worry),
        ("你觉得Leo怎么样？", Intent::ThinkOf),
        ("你需要什么吗？", Intent::Need),
        ("工作怎么样？", Intent::Work),
        ("最近有什么新鲜事吗？", Intent::News),
        ("疲れてるみたいだね。大丈夫？", Intent::Worry),
        ("Leoのこと、どう思う？", Intent::ThinkOf),
        ("何か必要なものはある？", Intent::Need),
        ("仕事はどう？", Intent::Work),
        ("最近なにか変わったことある？", Intent::News),
    ] {
        let heard = heard(words);
        assert!(
            heard == intent || (after_them.contains(&intent) && after_them.contains(&heard)),
            "{words}: {heard:?}"
        );
    }
    // Mara is lonely, thinks little of Leo and needs flour: those first.
    assert_eq!(
        openers::openers(&world, &kit, MARA),
        vec![
            openers::HOLDING_UP.to_string(),
            "What do you think of Leo?".to_string(),
            openers::NEED.to_string(),
        ]
    );
}

#[test]
fn the_prompt_crate_holds_this_systems_meanings_and_text_rules() {
    let ids = Intent::ALL
        .iter()
        .map(|intent| intent.id())
        .collect::<Vec<_>>();
    assert_eq!(ids, world_voice_prompt::MEANINGS.to_vec());
    for text in [
        "Ann",
        "\u{202E}Ann\u{2066} \u{200B}Bea",
        "  a\n\tb ",
        "李小龙",
        "\u{0}x",
    ] {
        assert_eq!(
            world_voice_prompt::text::clean_text(text),
            world_core::text::clean_text(text),
            "{text:?}"
        );
    }
}
