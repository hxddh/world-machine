use std::error::Error;
use std::fmt;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub u64);

        impl $name {
            pub const fn new(value: u64) -> Self {
                Self(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(EntityId);
id_type!(RelationId);
id_type!(EventId);

id_type!(ScheduleId);

/// Entity ids from here up are never given to an entity.
///
/// They are kept for ids *derived* from entities, which a presentation may
/// use to name something that is not an entity of its own (a building drawn
/// for someone, a place in a catalog). An Action that tries to create an
/// entity here is refused, so a derived id can never name a real entity.
/// Which derived ids mean what is for the layers above to document; the
/// kernel only keeps the space free.
pub const RESERVED_ENTITY_IDS: u64 = 900_000_000;

/// A block of entity ids a System gives out, `first` and the `room - 1`
/// after it, for entities of one kind that come into being as a World runs.
///
/// Blocks are chosen by hand, one per kind of newcomer, so that an id says
/// which kind it is. [`check_id_blocks`] proves a set of them is sound, and
/// [`crate::WorldState::free_id_in`] gives out the lowest free id in one,
/// from the state alone, so the same World always gives out the same ids
/// and replay never needs to ask.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdBlock {
    pub first: u64,
    pub room: u64,
}

impl IdBlock {
    pub const fn new(first: u64, room: u64) -> Self {
        Self { first, room }
    }

    /// The first id past the block.
    pub const fn end(self) -> u64 {
        self.first.saturating_add(self.room)
    }

    pub const fn contains(self, id: EntityId) -> bool {
        id.0 >= self.first && id.0 < self.end()
    }

    /// Every id in the block, lowest first.
    pub fn ids(self) -> impl Iterator<Item = EntityId> {
        (self.first..self.end()).map(EntityId::new)
    }

    pub const fn overlaps(self, other: IdBlock) -> bool {
        self.first < other.end() && other.first < self.end()
    }
}

impl fmt::Display for IdBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.first, self.end())
    }
}

/// Why ids could not be given out, or a set of blocks is not sound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdError {
    /// Every id in the block is taken.
    Full(IdBlock),
    /// A block with no room, or one that reaches [`RESERVED_ENTITY_IDS`].
    Unusable { name: String, block: IdBlock },
    /// Two blocks share ids.
    Overlap {
        first: String,
        second: String,
        at: u64,
    },
    /// An entity was to be created with a reserved id.
    Reserved(EntityId),
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(block) => write!(f, "every id in {block} is taken"),
            Self::Unusable { name, block } => write!(
                f,
                "the id block {name} ({block}) is empty or reaches the reserved ids from {RESERVED_ENTITY_IDS}"
            ),
            Self::Overlap { first, second, at } => {
                write!(f, "the id blocks {first} and {second} both hold {at}")
            }
            Self::Reserved(id) => write!(
                f,
                "entity id {id} is reserved: ids from {RESERVED_ENTITY_IDS} are never entities"
            ),
        }
    }
}

impl Error for IdError {}

/// Checks a set of named blocks: each has room, lies wholly below
/// [`RESERVED_ENTITY_IDS`], and shares no id with another. A World that
/// hands out ids from these blocks can then never give one id twice, or
/// give out one that a presentation derives.
pub fn check_id_blocks(blocks: &[(&str, IdBlock)]) -> Result<(), IdError> {
    for (name, block) in blocks {
        if block.room == 0 || block.end() > RESERVED_ENTITY_IDS {
            return Err(IdError::Unusable {
                name: (*name).to_string(),
                block: *block,
            });
        }
    }
    for (at, (first, a)) in blocks.iter().enumerate() {
        for (second, b) in &blocks[at + 1..] {
            if a.overlaps(*b) {
                return Err(IdError::Overlap {
                    first: (*first).to_string(),
                    second: (*second).to_string(),
                    at: a.first.max(b.first),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Action, ActionError, ActionRegistry, ActionRequest, Entity, EventDraft, StateChange, Value,
        World, WorldState,
    };

    /// An item arrives, taking the lowest free id in the block the request
    /// names, or the id it names outright.
    struct Arrive;

    impl Action for Arrive {
        fn name(&self) -> &'static str {
            "arrive"
        }

        fn evaluate(
            &self,
            state: &WorldState,
            request: &ActionRequest,
        ) -> Result<EventDraft, ActionError> {
            let id = match request.args.get("id") {
                Some(Value::Entity(id)) => *id,
                _ => state
                    .free_id_in(IdBlock::new(10, 3))
                    .map_err(|error| ActionError::Invalid(error.to_string()))?,
            };
            let mut draft = EventDraft::new("arrived");
            draft
                .changes
                .push(StateChange::CreateEntity(Entity::new(id, "item")));
            Ok(draft)
        }
    }

    fn world() -> (World, ActionRegistry) {
        let mut actions = ActionRegistry::new();
        actions.register(Arrive).unwrap();
        (World::new(WorldState::default()), actions)
    }

    #[test]
    fn a_block_gives_out_its_lowest_free_id_until_it_is_full_and_says_so() {
        let (mut world, actions) = world();
        let block = IdBlock::new(10, 3);
        assert_eq!(world.state().room_left_in(block), 3);
        for expected in [10, 11, 12] {
            let event = world
                .execute(&actions, &ActionRequest::new("arrive"))
                .unwrap();
            assert_eq!(
                event.changes,
                vec![StateChange::CreateEntity(Entity::new(
                    EntityId::new(expected),
                    "item"
                ))]
            );
        }
        assert_eq!(world.state().room_left_in(block), 0);
        assert_eq!(world.state().free_id_in(block), Err(IdError::Full(block)));
        // Full is loud: the Action is refused with the reason, and nothing
        // was recorded.
        let refused = world
            .execute(&actions, &ActionRequest::new("arrive"))
            .unwrap_err();
        assert_eq!(refused.to_string(), "every id in 10..13 is taken");
        assert_eq!(world.events().len(), 3);
        // Replay gives the same ids without asking the Action again.
        let replayed = world.replay().unwrap();
        assert_eq!(replayed.state(), world.state());
    }

    #[test]
    fn a_gap_is_filled_first() {
        let mut state = WorldState::default();
        for id in [10, 12] {
            state
                .seed_entity(Entity::new(EntityId::new(id), "item"))
                .unwrap();
        }
        let block = IdBlock::new(10, 3);
        assert_eq!(state.free_id_in(block), Ok(EntityId::new(11)));
        assert_eq!(state.room_left_in(block), 1);
    }

    #[test]
    fn a_reserved_id_is_never_given_to_an_entity() {
        let (mut world, actions) = world();
        let reserved = EntityId::new(RESERVED_ENTITY_IDS + 7);
        let error = world
            .execute(
                &actions,
                &ActionRequest::new("arrive").arg("id", Value::Entity(reserved)),
            )
            .unwrap_err();
        assert!(error.to_string().contains("reserved"), "{error}");
        assert!(world.state().entity(reserved).is_none());
        assert!(world.events().is_empty());
        // Just below the line is an ordinary id.
        let last = EntityId::new(RESERVED_ENTITY_IDS - 1);
        world
            .execute(
                &actions,
                &ActionRequest::new("arrive").arg("id", Value::Entity(last)),
            )
            .unwrap();
    }

    #[test]
    fn blocks_are_checked_for_room_the_reserved_line_and_overlaps() {
        assert_eq!(
            check_id_blocks(&[
                ("alpha", IdBlock::new(5000, 200)),
                ("gamma", IdBlock::new(6000, 300))
            ]),
            Ok(())
        );
        assert_eq!(
            check_id_blocks(&[
                ("alpha", IdBlock::new(5000, 200)),
                ("beta", IdBlock::new(5150, 100))
            ]),
            Err(IdError::Overlap {
                first: "alpha".into(),
                second: "beta".into(),
                at: 5150
            })
        );
        assert!(matches!(
            check_id_blocks(&[("empty", IdBlock::new(1, 0))]),
            Err(IdError::Unusable { .. })
        ));
        assert!(matches!(
            check_id_blocks(&[("high", IdBlock::new(RESERVED_ENTITY_IDS - 1, 2))]),
            Err(IdError::Unusable { .. })
        ));
    }
}
