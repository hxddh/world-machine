mod projection;

use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, EventDraft, EventId,
    StateChange, Value, World, WorldState, WorldStateError,
};
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};
use world_persistence::{PersistenceError, WorldArchive, WorldPackRef};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub const POCKET_UNIVERSE_PACK_ID: &str = "world-machine.pocket-universe";
pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.1.0";

pub const SEED_MARS_COLONY_COMMAND: &str = "pocket-universe.seed-mars-colony";
pub const SEED_1980S_TOWN_COMMAND: &str = "pocket-universe.seed-1980s-town";
pub const SEED_PENGUIN_CIVILIZATION_COMMAND: &str =
    "pocket-universe.seed-penguin-civilization";
pub const NUDGE_COMMAND: &str = "pocket-universe.nudge";

pub(crate) const UNIVERSE: EntityId = EntityId::new(1);
pub(crate) const SLOT_A: EntityId = EntityId::new(10);
pub(crate) const SLOT_B: EntityId = EntityId::new(11);
pub(crate) const SLOT_C: EntityId = EntityId::new(12);
pub(crate) const SLOT_D: EntityId = EntityId::new(13);

pub(crate) const SEED: &str = "seed";
pub(crate) const GENERATION: &str = "generation";
pub(crate) const LAST_CHANGE: &str = "last_change";
const ANCHOR_PULSE: &str = "pulse";
const UNSEEDED: &str = "unseeded";
const BACKGROUND_PERIOD: u64 = 10;

pub fn pocket_universe_pack_ref() -> WorldPackRef {
    WorldPackRef::new(POCKET_UNIVERSE_PACK_ID, POCKET_UNIVERSE_PACK_VERSION)
}

pub struct PocketUniverse {
    world: World,
    actions: ActionRegistry,
}

impl PocketUniverse {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            world: World::new(baseline()?),
            actions: build_action_registry()?,
        })
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn projection_snapshot(&self) -> ProjectionSnapshot {
        projection::snapshot(&self.world)
    }

    pub fn invoke_projection_command(
        &mut self,
        command_id: &str,
    ) -> Result<EventId, Box<dyn Error>> {
        let action = match command_id {
            SEED_MARS_COLONY_COMMAND => "seed_mars_colony",
            SEED_1980S_TOWN_COMMAND => "seed_1980s_town",
            SEED_PENGUIN_CIVILIZATION_COMMAND => "seed_penguin_civilization",
            NUDGE_COMMAND => "grow_universe",
            _ => {
                return Err(
                    std::io::Error::other(format!("unknown projection command: {command_id}"))
                        .into(),
                )
            }
        };
        Ok(self
            .world
            .execute(&self.actions, &ActionRequest::new(action).actor(UNIVERSE))?
            .id)
    }

    pub fn advance_periods(&mut self, periods: u64) -> Result<(), Box<dyn Error>> {
        if periods == 0 {
            return Ok(());
        }
        let delta = periods
            .checked_mul(BACKGROUND_PERIOD)
            .ok_or_else(|| std::io::Error::other("Pocket Universe time overflow"))?;
        let target = self
            .world
            .world_time()
            .checked_add(delta)
            .ok_or_else(|| std::io::Error::other("Pocket Universe time overflow"))?;

        if seed_id(&self.world) != UNSEEDED {
            for period in 1..=periods {
                let at = self
                    .world
                    .world_time()
                    .checked_add(period * BACKGROUND_PERIOD)
                    .ok_or_else(|| std::io::Error::other("Pocket Universe time overflow"))?;
                self.world.schedule_at(
                    at,
                    ActionRequest::new("grow_universe").actor(UNIVERSE),
                )?;
            }
        }
        self.world.advance_to(&self.actions, target)?;
        Ok(())
    }

    pub fn fork_before_event(&mut self, event_id: EventId) -> Result<(), Box<dyn Error>> {
        let position = self
            .world
            .events()
            .iter()
            .position(|event| event.id == event_id)
            .ok_or_else(|| std::io::Error::other(format!("unknown event {event_id}")))?;
        self.world = self.world.fork_after(position)?;
        Ok(())
    }

    pub fn archive(&self) -> Result<WorldArchive, PersistenceError> {
        WorldArchive::capture(pocket_universe_pack_ref(), &self.world)
    }

    pub fn resume_archive(archive: &WorldArchive) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            world: archive.restore(&pocket_universe_pack_ref(), baseline()?)?,
            actions: build_action_registry()?,
        })
    }
}

struct PocketUniverseSession {
    world: PocketUniverse,
}

impl PocketUniverseSession {
    fn fresh() -> Result<Box<dyn WorldSession>, HostError> {
        Ok(Box::new(Self {
            world: PocketUniverse::new().map_err(HostError::session)?,
        }))
    }

    fn open_archive(archive: &WorldArchive) -> Result<Box<dyn WorldSession>, HostError> {
        Ok(Box::new(Self {
            world: PocketUniverse::resume_archive(archive).map_err(HostError::session)?,
        }))
    }
}

impl WorldSession for PocketUniverseSession {
    fn pack(&self) -> WorldPackRef {
        pocket_universe_pack_ref()
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        self.world.projection_snapshot()
    }

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
        match intent {
            ProjectionIntent::ForkBeforeEvent(event) => self
                .world
                .fork_before_event(event)
                .map_err(HostError::session)?,
            ProjectionIntent::InvokeCommand(command) => {
                self.world
                    .invoke_projection_command(&command)
                    .map_err(HostError::session)?;
            }
        }
        Ok(self.snapshot())
    }

    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
        self.world
            .advance_periods(periods)
            .map_err(HostError::session)?;
        Ok(self.snapshot())
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        self.world.archive().map(Some).map_err(HostError::session)
    }
}

pub fn pocket_universe_registration() -> WorldRegistration {
    WorldRegistration::new(
        WorldDescriptor {
            pack: pocket_universe_pack_ref(),
            title: "Pocket Universe".into(),
            description:
                "Create a tiny persistent world, let it grow, then return to see what changed."
                    .into(),
        },
        PocketUniverseSession::fresh,
    )
    .with_archive_opener(PocketUniverseSession::open_archive)
}

fn baseline() -> Result<WorldState, WorldStateError> {
    let mut state = WorldState::default();
    state.seed_entity(
        Entity::new(UNIVERSE, "universe")
            .with_component("name", "Untitled Pocket Universe")
            .with_component(SEED, UNSEEDED)
            .with_component(GENERATION, 0_i64)
            .with_component(LAST_CHANGE, "Nothing exists here yet."),
    )?;
    Ok(state)
}

fn build_action_registry() -> Result<ActionRegistry, ActionError> {
    let mut actions = ActionRegistry::new();
    actions.register(SeedMarsColony)?;
    actions.register(Seed1980sTown)?;
    actions.register(SeedPenguinCivilization)?;
    actions.register(GrowUniverse)?;
    Ok(actions)
}

struct SeedMarsColony;
struct Seed1980sTown;
struct SeedPenguinCivilization;
struct GrowUniverse;

impl Action for SeedMarsColony {
    fn name(&self) -> &'static str {
        "seed_mars_colony"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        seed_draft(
            state,
            "mars-colony",
            "Ares Pocket Colony",
            [
                Entity::new(SLOT_A, "habitat")
                    .with_component("name", "Ares Habitat")
                    .with_component("status", "pressurized")
                    .with_component(ANCHOR_PULSE, "first lights"),
                Entity::new(SLOT_B, "person")
                    .with_component("name", "Nia Chen")
                    .with_component("role", "systems keeper"),
                Entity::new(SLOT_C, "place")
                    .with_component("name", "Hydroponics Bay")
                    .with_component("crop", "dwarf wheat"),
                Entity::new(SLOT_D, "rover")
                    .with_component("name", "Kestrel Rover")
                    .with_component("range", "18 km"),
            ],
        )
    }
}

impl Action for Seed1980sTown {
    fn name(&self) -> &'static str {
        "seed_1980s_town"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        seed_draft(
            state,
            "1980s-town",
            "Maple Street · 1987",
            [
                Entity::new(SLOT_A, "place")
                    .with_component("name", "Maple Arcade")
                    .with_component("status", "open late")
                    .with_component(ANCHOR_PULSE, "new high score"),
                Entity::new(SLOT_B, "person")
                    .with_component("name", "Lena Ortiz")
                    .with_component("role", "night-shift student"),
                Entity::new(SLOT_C, "radio_station")
                    .with_component("name", "K-88 Radio")
                    .with_component("format", "local mix"),
                Entity::new(SLOT_D, "bus")
                    .with_component("name", "Night Bus 6")
                    .with_component("route", "Maple Loop"),
            ],
        )
    }
}

impl Action for SeedPenguinCivilization {
    fn name(&self) -> &'static str {
        "seed_penguin_civilization"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        seed_draft(
            state,
            "penguin-civilization",
            "Icebridge Colony",
            [
                Entity::new(SLOT_A, "colony")
                    .with_component("name", "Icebridge")
                    .with_component("status", "lanterns lit")
                    .with_component(ANCHOR_PULSE, "first fish bell"),
                Entity::new(SLOT_B, "penguin")
                    .with_component("name", "Piko")
                    .with_component("role", "bridge keeper"),
                Entity::new(SLOT_C, "storehouse")
                    .with_component("name", "Fish Vault")
                    .with_component("reserve", "steady"),
                Entity::new(SLOT_D, "council")
                    .with_component("name", "Aurora Council")
                    .with_component("custom", "vote at moonrise"),
            ],
        )
    }
}

impl Action for GrowUniverse {
    fn name(&self) -> &'static str {
        "grow_universe"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let seed = seed_id_from_state(state)?;
        if seed == UNSEEDED {
            return Err(ActionError::Invalid(
                "choose a Pocket Universe seed before growing it".into(),
            ));
        }
        let generation = integer_component(state, UNIVERSE, GENERATION)?;
        let next = generation + 1;
        let change = growth_message(&seed, next);
        let pulse = anchor_pulse(&seed, next);
        let mut draft = EventDraft::new("universe_grew");
        draft.targets = vec![UNIVERSE, SLOT_A];
        draft.payload.insert("seed".into(), seed.into());
        draft.payload.insert("generation".into(), next.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: GENERATION.into(),
                value: next.into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LAST_CHANGE.into(),
                value: change.into(),
            },
            StateChange::SetComponent {
                entity: SLOT_A,
                key: ANCHOR_PULSE.into(),
                value: pulse.into(),
            },
        ];
        Ok(draft)
    }
}

fn seed_draft(
    state: &WorldState,
    seed: &str,
    universe_name: &str,
    entities: [Entity; 4],
) -> Result<EventDraft, ActionError> {
    if seed_id_from_state(state)? != UNSEEDED {
        return Err(ActionError::Invalid(
            "this Pocket Universe has already chosen a seed".into(),
        ));
    }
    let mut draft = EventDraft::new("universe_seeded");
    draft.targets = vec![UNIVERSE];
    draft.payload.insert("seed".into(), seed.into());
    draft.changes = vec![
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: "name".into(),
            value: universe_name.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: SEED.into(),
            value: seed.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: GENERATION.into(),
            value: 0_i64.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),
            value: "A new world has taken shape.".into(),
        },
    ];
    draft
        .changes
        .extend(entities.into_iter().map(StateChange::CreateEntity));
    Ok(draft)
}

pub(crate) fn seed_id(world: &World) -> &str {
    world
        .state()
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(SEED))
        .and_then(|value| match value {
            Value::Text(value) => Some(value.as_str()),
            _ => None,
        })
        .unwrap_or(UNSEEDED)
}

fn seed_id_from_state(state: &WorldState) -> Result<String, ActionError> {
    match state
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(SEED))
    {
        Some(Value::Text(seed)) => Ok(seed.clone()),
        _ => Err(ActionError::Invalid(
            "Pocket Universe seed state is missing".into(),
        )),
    }
}

fn integer_component(
    state: &WorldState,
    entity: EntityId,
    key: &str,
) -> Result<i64, ActionError> {
    match state.entity(entity).and_then(|entity| entity.component(key)) {
        Some(Value::Integer(value)) => Ok(*value),
        _ => Err(ActionError::Invalid(format!(
            "entity {entity} has no integer component {key}"
        ))),
    }
}

fn growth_message(seed: &str, generation: i64) -> String {
    let cycle = ((generation - 1).rem_euclid(3)) as usize;
    let messages: [&[&str]; 3] = [
        &[
            "The colony opened a new water-recovery loop.",
            "A dust front changed the rover routes overnight.",
            "The hydroponics crew harvested its first shared meal.",
        ],
        &[
            "A handwritten tournament bracket appeared at the arcade.",
            "K-88 dedicated an hour to calls from the neighborhood.",
            "Night Bus 6 added an unscheduled stop after the rain.",
        ],
        &[
            "A new ice bridge shortened the walk to the Fish Vault.",
            "Piko rang the fish bell early after spotting a silver shoal.",
            "The Aurora Council adopted a new moonrise signal.",
        ],
    ];
    match seed {
        "mars-colony" => messages[0][cycle],
        "1980s-town" => messages[1][cycle],
        "penguin-civilization" => messages[2][cycle],
        _ => "The world changed in a small but persistent way.",
    }
    .into()
}

fn anchor_pulse(seed: &str, generation: i64) -> String {
    match seed {
        "mars-colony" => format!("sol-cycle {generation}"),
        "1980s-town" => format!("after-school night {generation}"),
        "penguin-civilization" => format!("aurora cycle {generation}"),
        _ => format!("cycle {generation}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> world_host::WorldRegistry {
        let mut registry = world_host::WorldRegistry::new();
        registry.register(pocket_universe_registration()).unwrap();
        registry
    }

    #[test]
    fn empty_universe_offers_multiple_world_seeds() {
        let registry = registry();
        let session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        let snapshot = session.snapshot();
        let commands = snapshot
            .commands
            .iter()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(snapshot.title, "Pocket Universe · Empty World");
        assert!(commands.contains(&SEED_MARS_COLONY_COMMAND));
        assert!(commands.contains(&SEED_1980S_TOWN_COMMAND));
        assert!(commands.contains(&SEED_PENGUIN_CIVILIZATION_COMMAND));
        assert!(snapshot.collection.items.is_empty());
    }

    #[test]
    fn one_pack_can_seed_distinct_world_shapes() {
        let registry = registry();
        let mut mars = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        let mut town = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();

        let mars_snapshot = mars
            .handle(ProjectionIntent::InvokeCommand(
                SEED_MARS_COLONY_COMMAND.into(),
            ))
            .unwrap();
        let town_snapshot = town
            .handle(ProjectionIntent::InvokeCommand(
                SEED_1980S_TOWN_COMMAND.into(),
            ))
            .unwrap();

        assert_eq!(mars.pack(), town.pack());
        assert_ne!(mars_snapshot.title, town_snapshot.title);
        assert_ne!(mars_snapshot.collection.items, town_snapshot.collection.items);
        assert!(mars_snapshot
            .collection
            .items
            .iter()
            .any(|item| item.title == "Ares Habitat"));
        assert!(town_snapshot
            .collection
            .items
            .iter()
            .any(|item| item.title == "Maple Arcade"));
    }

    #[test]
    fn seed_is_a_durable_one_time_choice() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_PENGUIN_CIVILIZATION_COMMAND.into(),
            ))
            .unwrap();

        let error = session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_MARS_COLONY_COMMAND.into(),
            ))
            .unwrap_err();
        assert!(error.to_string().contains("already chosen a seed"));
    }

    #[test]
    fn background_time_grows_a_seeded_world() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_MARS_COLONY_COMMAND.into(),
            ))
            .unwrap();
        let before = session.snapshot();

        let after = session.advance_background(2).unwrap();

        assert_eq!(after.world_time, before.world_time + 20);
        assert_eq!(after.timeline.items.len(), before.timeline.items.len() + 2);
        assert!(after
            .briefing
            .as_ref()
            .unwrap()
            .title
            .contains("Generation 2"));
    }

    #[test]
    fn archive_round_trip_preserves_seed_and_growth() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_1980S_TOWN_COMMAND.into(),
            ))
            .unwrap();
        session.advance_background(3).unwrap();
        let before = session.snapshot();
        let archive = session.archive().unwrap().unwrap();
        drop(session);

        let reopened = registry.open_archive(&archive).unwrap();

        assert_eq!(reopened.snapshot(), before);
        assert_eq!(reopened.archive().unwrap().unwrap(), archive);
    }

    #[test]
    fn forking_before_seed_returns_to_an_empty_universe() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        let seeded = session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_PENGUIN_CIVILIZATION_COMMAND.into(),
            ))
            .unwrap();
        let seed_event = seeded
            .timeline
            .items
            .iter()
            .find_map(|item| match item.id {
                world_projection::SelectionId::Event(id) => Some(id),
                _ => None,
            })
            .unwrap();

        let forked = session
            .handle(ProjectionIntent::ForkBeforeEvent(seed_event))
            .unwrap();

        assert_eq!(forked.title, "Pocket Universe · Empty World");
        assert!(forked.collection.items.is_empty());
        assert_eq!(forked.commands.len(), 3);
    }
}
