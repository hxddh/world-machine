//! Selection ids a presentation derives for what is not an entity of its
//! own: the home a household lives in, a work in a Pack's catalog.
//!
//! They live above [`world_core::RESERVED_ENTITY_IDS`], which the kernel
//! never gives to an entity, so a derived id can never name a real one. A
//! renderer reads any id from there up as "drawn, not inspected" (see
//! [`is_derived`]). The ranges, all disjoint:
//!
//! | ids | what |
//! |---|---|
//! | 900,000,000 to 909,999,999 | the home of founder *n*, at 900,000,000 + *n*, for *n* below 10,000,000 |
//! | 910,000,000 to 919,999,999 | the work at place *i* in a Pack's catalog, at 910,000,000 + *i* |
//! | 1,010,000,000 to 1,899,999,999 | the home of founder *n* from 10,000,000 up, at 1,000,000,000 + *n* |
//!
//! Homes of founders below 10,000,000 keep the ids they always had, so a
//! World's screens and saved choices read as before; a founder from
//! 10,000,000 up, which the first range would have put among the works,
//! has a home in the third.

use crate::SelectionId;
use world_core::{EntityId, RESERVED_ENTITY_IDS};

/// Where the homes of founders below [`NEAR_FOUNDERS`] start.
pub const HOME_IDS: u64 = RESERVED_ENTITY_IDS;
/// How many founders have a home in the first range.
pub const NEAR_FOUNDERS: u64 = 10_000_000;
/// Where works start: right after the first range of homes.
pub const WORK_IDS: u64 = HOME_IDS + NEAR_FOUNDERS;
/// How many works a catalog may have.
pub const MOST_WORKS: u64 = 10_000_000;
/// Where the homes of founders from [`NEAR_FOUNDERS`] up are counted from.
pub const FAR_HOME_IDS: u64 = 1_000_000_000;

/// The home of a household, by its founder.
pub fn home_id(founder: EntityId) -> SelectionId {
    let id = if founder.0 < NEAR_FOUNDERS {
        HOME_IDS + founder.0
    } else {
        FAR_HOME_IDS + founder.0
    };
    SelectionId::Entity(EntityId::new(id))
}

/// Whether a selection is a home.
pub fn is_home(selection: SelectionId) -> bool {
    matches!(selection, SelectionId::Entity(id)
        if (HOME_IDS..WORK_IDS).contains(&id.0)
            || (FAR_HOME_IDS + NEAR_FOUNDERS..FAR_HOME_IDS + RESERVED_ENTITY_IDS).contains(&id.0))
}

/// The work at place `index` in a catalog.
///
/// # Panics
///
/// For a place past [`MOST_WORKS`], which would run into other ids: no
/// catalog is that long.
pub fn work_id(index: usize) -> SelectionId {
    let index = index as u64;
    assert!(
        index < MOST_WORKS,
        "a catalog holds at most {MOST_WORKS} works"
    );
    SelectionId::Entity(EntityId::new(WORK_IDS + index))
}

/// Whether a selection is a work in a catalog.
pub fn is_work(selection: SelectionId) -> bool {
    matches!(selection, SelectionId::Entity(id) if (WORK_IDS..WORK_IDS + MOST_WORKS).contains(&id.0))
}

/// Whether an entity id is derived rather than a real entity's: drawn, and
/// never inspected as an entity.
pub fn is_derived(id: u64) -> bool {
    id >= RESERVED_ENTITY_IDS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(selection: SelectionId) -> u64 {
        match selection {
            SelectionId::Entity(id) => id.0,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_ranges_are_disjoint_and_above_every_entity() {
        let ranges = [
            (HOME_IDS, HOME_IDS + NEAR_FOUNDERS),
            (WORK_IDS, WORK_IDS + MOST_WORKS),
            (
                FAR_HOME_IDS + NEAR_FOUNDERS,
                FAR_HOME_IDS + RESERVED_ENTITY_IDS,
            ),
        ];
        for (at, (start, end)) in ranges.iter().enumerate() {
            assert!(*start >= RESERVED_ENTITY_IDS && start < end);
            for (other_start, other_end) in &ranges[at + 1..] {
                assert!(end <= other_start || other_end <= start, "{at}");
            }
        }
    }

    #[test]
    fn a_founder_at_or_above_ten_million_does_not_collide_with_a_work() {
        // As before, for every founder a World has had.
        assert_eq!(number(home_id(EntityId::new(7))), 900_000_007);
        assert_eq!(number(home_id(EntityId::new(9_999_999))), 909_999_999);
        for founder in [10_000_000, 10_000_001, 123_456_789, RESERVED_ENTITY_IDS - 1] {
            let home = home_id(EntityId::new(founder));
            assert!(is_home(home), "{founder}");
            assert!(!is_work(home), "{founder}");
            assert!(is_derived(number(home)));
            for index in [0, 1, 99, MOST_WORKS as usize - 1] {
                assert_ne!(home, work_id(index));
            }
        }
        for index in [0, 5, MOST_WORKS as usize - 1] {
            let work = work_id(index);
            assert!(is_work(work) && !is_home(work), "{index}");
            assert!(is_derived(number(work)));
        }
        assert!(!is_derived(RESERVED_ENTITY_IDS - 1));
    }

    #[test]
    #[should_panic(expected = "a catalog holds at most")]
    fn a_catalog_past_its_range_is_refused() {
        let _ = work_id(MOST_WORKS as usize);
    }
}
