//! How the people a player can talk to are shown: how each stands with the
//! player, who is asking something now, and the face each wears.

use crate::Kit;
use std::collections::BTreeSet;
use world_core::{EntityId, World};
use world_projection::{Mood, ProjectionCommand, SelectionId};

/// How someone the player can talk to stands with them.
pub fn standing_of(world: &World, kit: &Kit, who: EntityId) -> Option<world_projection::Standing> {
    let state = world.state();
    crate::can_talk_to(state, kit, who).then(|| {
        let standing = crate::standing(state, kit, who);
        world_projection::Standing {
            level: standing.level,
            words: standing.words,
        }
    })
}

/// Everyone asking the player something in `commands`.
pub fn askers(commands: &[ProjectionCommand]) -> BTreeSet<EntityId> {
    commands
        .iter()
        .filter(|command| command.question.is_some())
        .filter_map(|command| match command.asker {
            Some(SelectionId::Entity(who)) => Some(who),
            _ => None,
        })
        .collect()
}

/// How someone feels, for their face: thinking while they are asking the
/// player something, cross while hurt by what the player said, else how
/// their life is going. Someone too young for an everyday life wears the
/// face `young` gives.
pub fn mood_of(
    world: &World,
    kit: &Kit,
    who: EntityId,
    askers: &BTreeSet<EntityId>,
    young: impl FnOnce() -> Option<Mood>,
) -> Option<Mood> {
    let state = world.state();
    if !lives::enrolled(state, who) {
        return young();
    }
    if askers.contains(&who) {
        return Some(Mood::Thinking);
    }
    if standing_of(world, kit, who).is_some_and(|standing| standing.level < 0) {
        return Some(Mood::Cross);
    }
    Some(match lives::mood(state, who) {
        "cross" => Mood::Cross,
        "sad" => Mood::Sad,
        "happy" => Mood::Happy,
        _ => Mood::Content,
    })
}

/// What a Pack's own Chinese (`catalog`, English and Chinese a tab apart
/// on each line) calls a name, if it translates it.
///
/// Each catalog is read once into an index, kept for the life of the
/// process: reading the whole catalog for every name made each check of a
/// model's words cost a read of every catalog for every name the World
/// knows (most of a Pack's red-team tests, H2 v0.28). The first
/// line that translates a name is the one taken, as before.
pub fn in_chinese(catalog: &'static str, name: &str) -> Option<&'static str> {
    type Index = std::collections::HashMap<&'static str, &'static str>;
    type Indexes = std::collections::HashMap<(usize, usize), std::sync::Arc<Index>>;
    static INDEXES: std::sync::OnceLock<std::sync::Mutex<Indexes>> = std::sync::OnceLock::new();
    let key = (catalog.as_ptr() as usize, catalog.len());
    let index = {
        let mut indexes = INDEXES
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::sync::Arc::clone(indexes.entry(key).or_insert_with(|| {
            let mut index = Index::new();
            for (english, chinese) in catalog.lines().filter_map(|line| line.split_once('\t')) {
                if english != chinese {
                    index.entry(english).or_insert(chinese);
                }
            }
            std::sync::Arc::new(index)
        }))
    };
    index.get(name).copied()
}

#[cfg(test)]
mod tests {
    use super::in_chinese;

    /// The index finds what reading the catalog line by line found: the
    /// first line that translates a name, skipping lines that leave it as
    /// it is and lines with no translation at all.
    #[test]
    fn a_name_is_translated_by_the_first_line_that_translates_it() {
        const CATALOG: &str =
            "Tuk\tTuk\nTuk\t图克\nTuk\t图克二\nno tab here\nPiko\t皮可\nMiri\tMiri\n";
        let by_reading = |name: &str| {
            CATALOG.lines().find_map(|line| {
                let (english, chinese) = line.split_once('\t')?;
                (english == name && chinese != name).then_some(chinese)
            })
        };
        for name in ["Tuk", "Piko", "Miri", "no tab here", "Nobody", ""] {
            assert_eq!(in_chinese(CATALOG, name), by_reading(name), "{name:?}");
        }
        assert_eq!(in_chinese(CATALOG, "Tuk"), Some("图克"));
        assert_eq!(in_chinese(CATALOG, "Miri"), None);
    }
}
