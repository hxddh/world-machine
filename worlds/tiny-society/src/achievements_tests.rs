use super::*;
use crate::keeper::tests::year_lived;
use crate::TinySocietyBranch;
use std::collections::BTreeSet;

fn ids(world: &World) -> BTreeSet<&'static str> {
    unlocked(world)
        .into_iter()
        .map(|achievement| achievement.id.as_str())
        .collect()
}

#[test]
fn the_file_lists_every_achievement_whole_in_three_languages() {
    let all = all();
    assert!(all.len() >= 20, "{}", all.len());
    let unique = all
        .iter()
        .map(|achievement| achievement.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), all.len(), "no id twice");
    for achievement in all {
        // Steam's API names: capitals, digits and underscores.
        assert!(achievement
            .id
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'));
        for name in [&achievement.zh, &achievement.ja] {
            assert!(!name.is_ascii(), "{name}");
        }
        assert!(achievement.events.iter().all(|kind| !kind.is_empty()));
    }
    // Every page, a year, both projects, the books and the hearts.
    for unlock in [
        Unlock::Page("spring".into()),
        Unlock::Page("winter".into()),
        Unlock::Years(1),
        Unlock::Project("lighthouse".into(), None),
        Unlock::Project("school_roof".into(), None),
        Unlock::Finds(None),
        Unlock::Almanac,
        Unlock::Hearts(8),
    ] {
        assert!(all.iter().any(|a| a.unlock == unlock), "{unlock:?}");
    }
    for who in crate::keeper::HEARTS {
        let name = match who {
            crate::MARA => "Mara",
            crate::JONAS => "Jonas",
            crate::LEO => "Leo",
            crate::EMMA => "Emma",
            crate::SOFIA => "Sofia",
            crate::MIA => "Mia",
            crate::NOAH => "Noah",
            _ => "Evan",
        };
        assert!(all.iter().any(|a| a.unlock == Unlock::Heart(name.into())));
    }
    // A condition the file cannot mean is refused.
    assert!(parse("[[achievement]]\nid = \"X\"\nunlock = \"page\"\n").is_err());
    assert!(parse("id = \"X\"\n").is_err());
}

#[test]
fn a_new_harbour_has_unlocked_nothing() {
    let branch = TinySocietyBranch::new_world().unwrap();
    assert!(ids(branch.world()).is_empty(), "{:?}", ids(branch.world()));
}

#[test]
fn a_year_lived_unlocks_its_pages_its_year_and_the_first_steps() {
    let world = year_lived().world();
    let unlocked = ids(world);
    for id in [
        "BOOK_FIRST_PAGE",
        "BOOK_SUMMER_FAIR",
        "BOOK_AUTUMN_GALE",
        "BOOK_WINTER_LIGHTS",
        "BOOK_YEAR_ONE",
        "PROJECT_LIGHTHOUSE_BEGUN",
        "PROJECT_SCHOOL_ROOF_BEGUN",
    ] {
        assert!(unlocked.contains(id), "{id} not in {unlocked:?}");
    }
    for id in ["BOOK_YEAR_TWO", "BOOK_YEAR_FIVE", "FINDS_ALL"] {
        assert!(!unlocked.contains(id), "{id} in {unlocked:?}");
    }
    // Read from the record alone: the World replayed unlocks the same.
    let replayed = world.replay().unwrap();
    assert_eq!(ids(&replayed), unlocked);
    // A heart is unlocked exactly when all its beats are reached.
    for (who, beats) in crate::keeper::hearts(world) {
        let name = lives::first_name(world.state(), who);
        let id = format!("HEART_{}", name.to_uppercase());
        assert_eq!(
            unlocked.contains(id.as_str()),
            beats.iter().all(|beat| *beat),
            "{name}: {beats:?}"
        );
    }
}
