mod projection;

use std::error::Error;
use std::sync::Arc;
use world_agent::{
    register_actions as register_agent_actions, AgentDecision, AgentExecutor, AgentObservation,
    AgentRuntime, AgentRuntimeError, AvailableAction, ScopedPerception,
};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, EventDraft, EventId,
    StateChange, Value, World, WorldState, WorldStateError,
};
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};
use world_persistence::{PersistenceError, WorldArchive, WorldPackRef};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub const POCKET_UNIVERSE_PACK_ID: &str = "world-machine.pocket-universe";
pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.4.0";

pub const SEED_MARS_COLONY_COMMAND: &str = "pocket-universe.seed-mars-colony";
pub const SEED_1980S_TOWN_COMMAND: &str = "pocket-universe.seed-1980s-town";
pub const SEED_PENGUIN_CIVILIZATION_COMMAND: &str = "pocket-universe.seed-penguin-civilization";
pub const NUDGE_COMMAND: &str = "pocket-universe.nudge";
pub const BOLD_PATH_COMMAND: &str = "pocket-universe.choose-bold-path";
pub const CAREFUL_PATH_COMMAND: &str = "pocket-universe.choose-careful-path";

pub(crate) const UNIVERSE: EntityId = EntityId::new(1);
pub(crate) const SLOT_A: EntityId = EntityId::new(10);
pub(crate) const SLOT_B: EntityId = EntityId::new(11);
pub(crate) const SLOT_C: EntityId = EntityId::new(12);
pub(crate) const SLOT_D: EntityId = EntityId::new(13);

pub(crate) const SEED: &str = "seed";
pub(crate) const GENERATION: &str = "generation";
pub(crate) const LAST_CHANGE: &str = "last_change";
pub(crate) const DECISION: &str = "decision";
const ANCHOR_PULSE: &str = "pulse";
const UNSEEDED: &str = "unseeded";
const BACKGROUND_PERIOD: u64 = 10;
const AGENT_CARE_ACTION: &str = "pocket_agent.care";
const AGENT_EXPLORE_ACTION: &str = "pocket_agent.explore";
const AGENT_CARE_COUNT: &str = "care_count";
const AGENT_EXPLORE_COUNT: &str = "explore_count";

pub fn pocket_universe_pack_ref() -> WorldPackRef {
    WorldPackRef::new(POCKET_UNIVERSE_PACK_ID, POCKET_UNIVERSE_PACK_VERSION)
}

#[derive(Clone, Debug, Default)]
pub struct PocketMind;

impl AgentRuntime for PocketMind {
    fn decide(
        &mut self,
        observation: &AgentObservation,
        actions: &[AvailableAction],
    ) -> Result<AgentDecision, AgentRuntimeError> {
        let actor = observation
            .entities
            .iter()
            .find(|entity| entity.id == observation.actor)
            .ok_or_else(|| {
                AgentRuntimeError::new("Pocket Mind observation is missing its actor")
            })?;
        let count = |key: &str| match actor.component(key) {
            Some(Value::Integer(value)) => Ok(*value),
            _ => Err(AgentRuntimeError::new(format!(
                "Pocket Mind actor is missing integer component {key}"
            ))),
        };
        let care_count = count(AGENT_CARE_COUNT)?;
        let explore_count = count(AGENT_EXPLORE_COUNT)?;
        let desired = if care_count <= explore_count {
            AGENT_CARE_ACTION
        } else {
            AGENT_EXPLORE_ACTION
        };
        if !actions.iter().any(|action| action.name() == desired) {
            return Err(AgentRuntimeError::new(format!(
                "Pocket Mind expected offered action {desired}"
            )));
        }
        Ok(AgentDecision::choose(desired))
    }
}

pub struct PocketUniverse<R = PocketMind>
where
    R: AgentRuntime,
{
    world: World,
    actions: ActionRegistry,
    mind: R,
}

impl PocketUniverse<PocketMind> {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Self::with_agent_runtime(PocketMind)
    }

    pub fn resume_archive(archive: &WorldArchive) -> Result<Self, Box<dyn Error>> {
        Self::resume_archive_with_agent_runtime(archive, PocketMind)
    }
}

impl<R> PocketUniverse<R>
where
    R: AgentRuntime,
{
    pub fn with_agent_runtime(mind: R) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            world: World::new(baseline()?),
            actions: build_action_registry()?,
            mind,
        })
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn projection_snapshot(&self) -> ProjectionSnapshot {
        projection::snapshot(&self.world)
    }

    pub fn projection_snapshot_since(
        &self,
        since_event_count: Option<usize>,
    ) -> ProjectionSnapshot {
        projection::snapshot_since(&self.world, since_event_count)
    }

    pub fn invoke_projection_command(
        &mut self,
        command_id: &str,
    ) -> Result<EventId, Box<dyn Error>> {
        if command_id == NUDGE_COMMAND {
            let mut candidate = self.world.clone();
            let growth = candidate
                .execute(
                    &self.actions,
                    &ActionRequest::new("grow_universe").actor(UNIVERSE),
                )?
                .id;
            let outcome =
                Self::run_agent_turn_on(&mut self.mind, &mut candidate, &self.actions, &[growth])?;
            self.world = candidate;
            return Ok(outcome);
        }

        let action = match command_id {
            SEED_MARS_COLONY_COMMAND => "seed_mars_colony",
            SEED_1980S_TOWN_COMMAND => "seed_1980s_town",
            SEED_PENGUIN_CIVILIZATION_COMMAND => "seed_penguin_civilization",
            BOLD_PATH_COMMAND => "choose_bold_path",
            CAREFUL_PATH_COMMAND => "choose_careful_path",
            _ => {
                return Err(std::io::Error::other(format!(
                    "unknown projection command: {command_id}"
                ))
                .into())
            }
        };
        Ok(self
            .world
            .execute(&self.actions, &ActionRequest::new(action).actor(UNIVERSE))?
            .id)
    }

    pub fn advance_periods(&mut self, periods: u64) -> Result<(), Box<dyn Error>> {
        let mut candidate = self.world.clone();
        for _ in 0..periods {
            let target = candidate
                .world_time()
                .checked_add(BACKGROUND_PERIOD)
                .ok_or_else(|| std::io::Error::other("Pocket Universe time overflow"))?;
            if seed_id(&candidate) == UNSEEDED {
                candidate.advance_to(&self.actions, target)?;
                continue;
            }

            candidate.schedule_at(target, ActionRequest::new("grow_universe").actor(UNIVERSE))?;
            let executed = candidate.advance_to(&self.actions, target)?;
            let growth = executed.last().copied().ok_or_else(|| {
                std::io::Error::other("scheduled Pocket Universe growth did not run")
            })?;
            Self::run_agent_turn_on(&mut self.mind, &mut candidate, &self.actions, &[growth])?;
        }
        self.world = candidate;
        Ok(())
    }

    fn run_agent_turn_on(
        mind: &mut R,
        world: &mut World,
        registry: &ActionRegistry,
        caused_by: &[EventId],
    ) -> Result<EventId, Box<dyn Error>> {
        let actions = vec![
            AvailableAction::new(
                "Care for the small world and reinforce what already exists.",
                ActionRequest::new(AGENT_CARE_ACTION),
            ),
            AvailableAction::new(
                "Explore beyond the familiar routine and bring back a new thread.",
                ActionRequest::new(AGENT_EXPLORE_ACTION),
            ),
        ];
        let execution = AgentExecutor::decide_and_execute(
            mind,
            &ScopedPerception::new([UNIVERSE, SLOT_A]),
            world,
            registry,
            SLOT_B,
            &actions,
            caused_by,
        )?;
        Ok(execution.outcome_event)
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

    pub fn resume_archive_with_agent_runtime(
        archive: &WorldArchive,
        mind: R,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            world: archive.restore(&pocket_universe_pack_ref(), baseline()?)?,
            actions: build_action_registry()?,
            mind,
        })
    }
}

struct PocketUniverseSession<R>
where
    R: AgentRuntime,
{
    world: PocketUniverse<R>,
    return_since_event_count: Option<usize>,
}

impl<R> PocketUniverseSession<R>
where
    R: AgentRuntime + 'static,
{
    fn fresh(mind: R) -> Result<Box<dyn WorldSession>, HostError> {
        Ok(Box::new(Self {
            world: PocketUniverse::with_agent_runtime(mind).map_err(HostError::session)?,
            return_since_event_count: None,
        }))
    }

    fn open_archive(archive: &WorldArchive, mind: R) -> Result<Box<dyn WorldSession>, HostError> {
        Ok(Box::new(Self {
            world: PocketUniverse::resume_archive_with_agent_runtime(archive, mind)
                .map_err(HostError::session)?,
            return_since_event_count: None,
        }))
    }
}

impl<R> WorldSession for PocketUniverseSession<R>
where
    R: AgentRuntime + 'static,
{
    fn pack(&self) -> WorldPackRef {
        pocket_universe_pack_ref()
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        self.world
            .projection_snapshot_since(self.return_since_event_count)
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
        self.return_since_event_count = None;
        Ok(self.snapshot())
    }

    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
        let before = self.world.world().events().len();
        self.world
            .advance_periods(periods)
            .map_err(HostError::session)?;
        self.return_since_event_count =
            (self.world.world().events().len() > before).then_some(before);
        Ok(self.snapshot())
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        self.world.archive().map(Some).map_err(HostError::session)
    }
}

pub fn pocket_universe_descriptor() -> WorldDescriptor {
    WorldDescriptor {
        pack: pocket_universe_pack_ref(),
        title: "Pocket Universe".into(),
        description:
            "Create a tiny persistent world, let it grow, then return to see what changed.".into(),
    }
}

pub fn pocket_universe_registration() -> WorldRegistration {
    pocket_universe_registration_with_agent_runtime(|| PocketMind)
}

pub fn pocket_universe_registration_with_agent_runtime<R, F>(factory: F) -> WorldRegistration
where
    R: AgentRuntime + 'static,
    F: Fn() -> R + Send + Sync + 'static,
{
    let factory = Arc::new(factory);
    let create_factory = Arc::clone(&factory);
    let open_factory = Arc::clone(&factory);
    WorldRegistration::new(pocket_universe_descriptor(), move || {
        PocketUniverseSession::fresh(create_factory())
    })
    .with_archive_opener(move |archive| {
        PocketUniverseSession::open_archive(archive, open_factory())
    })
}

fn baseline() -> Result<WorldState, WorldStateError> {
    let mut state = WorldState::default();
    state.seed_entity(
        Entity::new(UNIVERSE, "universe")
            .with_component("name", "Untitled Pocket Universe")
            .with_component(SEED, UNSEEDED)
            .with_component(GENERATION, 0_i64)
            .with_component(DECISION, "none")
            .with_component(LAST_CHANGE, "Nothing exists here yet."),
    )?;
    Ok(state)
}

fn build_action_registry() -> Result<ActionRegistry, ActionError> {
    let mut actions = ActionRegistry::new();
    register_agent_actions(&mut actions)?;
    actions.register(SeedMarsColony)?;
    actions.register(Seed1980sTown)?;
    actions.register(SeedPenguinCivilization)?;
    actions.register(GrowUniverse)?;
    actions.register(ChooseBoldPath)?;
    actions.register(ChooseCarefulPath)?;
    actions.register(CareForWorld)?;
    actions.register(ExploreWorld)?;
    Ok(actions)
}

struct SeedMarsColony;
struct Seed1980sTown;
struct SeedPenguinCivilization;
struct GrowUniverse;
struct ChooseBoldPath;
struct ChooseCarefulPath;
struct CareForWorld;
struct ExploreWorld;

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
                    .with_component(ANCHOR_PULSE, "first lights")
                    .with_component("water_cycles", 0_i64),
                Entity::new(SLOT_B, "person")
                    .with_component("name", "Nia Chen")
                    .with_component("role", "systems keeper")
                    .with_component(AGENT_CARE_COUNT, 0_i64)
                    .with_component(AGENT_EXPLORE_COUNT, 0_i64),
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
                    .with_component(ANCHOR_PULSE, "new high score")
                    .with_component("high_scores", 0_i64),
                Entity::new(SLOT_B, "person")
                    .with_component("name", "Lena Ortiz")
                    .with_component("role", "night-shift student")
                    .with_component(AGENT_CARE_COUNT, 0_i64)
                    .with_component(AGENT_EXPLORE_COUNT, 0_i64),
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
                    .with_component(ANCHOR_PULSE, "first fish bell")
                    .with_component("bridge_spans", 1_i64),
                Entity::new(SLOT_B, "penguin")
                    .with_component("name", "Piko")
                    .with_component("role", "bridge keeper")
                    .with_component(AGENT_CARE_COUNT, 0_i64)
                    .with_component(AGENT_EXPLORE_COUNT, 0_i64),
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
        let decision = decision_id_from_state(state)?;
        let change = growth_message(&seed, next, &decision);
        let pulse = anchor_pulse(&seed, next);
        let (metric_key, metric_value) = growth_metric(state, &seed)?;
        let mut draft = EventDraft::new("universe_grew");
        draft.targets = vec![UNIVERSE, SLOT_A];
        draft.payload.insert("seed".into(), seed.into());
        draft.payload.insert("generation".into(), next.into());
        draft.payload.insert("change".into(), change.clone().into());
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
            StateChange::SetComponent {
                entity: SLOT_A,
                key: metric_key.into(),
                value: metric_value.into(),
            },
        ];
        Ok(draft)
    }
}

impl Action for CareForWorld {
    fn name(&self) -> &'static str {
        AGENT_CARE_ACTION
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        mind_action_draft(state, request, true)
    }
}

impl Action for ExploreWorld {
    fn name(&self) -> &'static str {
        AGENT_EXPLORE_ACTION
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        mind_action_draft(state, request, false)
    }
}

fn mind_action_draft(
    state: &WorldState,
    request: &ActionRequest,
    care: bool,
) -> Result<EventDraft, ActionError> {
    let actor = request
        .actor
        .ok_or_else(|| ActionError::Invalid("Pocket Mind action requires an actor".into()))?;
    if actor != SLOT_B {
        return Err(ActionError::Invalid(format!(
            "Pocket Mind action requires seed actor {SLOT_B}, got {actor}"
        )));
    }
    let seed = seed_id_from_state(state)?;
    if seed == UNSEEDED {
        return Err(ActionError::Invalid(
            "Pocket Mind cannot act before its world is seeded".into(),
        ));
    }
    let count_key = if care {
        AGENT_CARE_COUNT
    } else {
        AGENT_EXPLORE_COUNT
    };
    let next = integer_component(state, actor, count_key)? + 1;
    let (target, key, value, change) = mind_outcome(&seed, care, next)?;
    let mut draft = EventDraft::new(if care {
        "agent_cared_for_world"
    } else {
        "agent_explored_world"
    });
    draft.targets = vec![actor, target];
    draft.payload.insert("seed".into(), seed.into());
    draft.payload.insert("change".into(), change.clone().into());
    draft.payload.insert("turn".into(), next.into());
    draft.changes = vec![
        StateChange::SetComponent {
            entity: actor,
            key: count_key.into(),
            value: next.into(),
        },
        StateChange::SetComponent {
            entity: actor,
            key: "last_intent".into(),
            value: if care { "care" } else { "explore" }.into(),
        },
        StateChange::SetComponent {
            entity: target,
            key: key.into(),
            value: value.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),
            value: change.into(),
        },
    ];
    Ok(draft)
}

fn mind_outcome(
    seed: &str,
    care: bool,
    turn: i64,
) -> Result<(EntityId, &'static str, String, String), ActionError> {
    let outcome = match (seed, care) {
        ("mars-colony", true) => (
            SLOT_C,
            "crop",
            format!("Nia tending cycle {turn}"),
            format!("Nia tuned the hydroponics loop for care cycle {turn}."),
        ),
        ("mars-colony", false) => (
            SLOT_D,
            "range",
            format!("survey route {turn}"),
            format!("Nia sent Kestrel onto survey route {turn} beyond the familiar markers."),
        ),
        ("1980s-town", true) => (
            SLOT_A,
            "status",
            format!("Lena's community night {turn}"),
            format!("Lena kept Maple Arcade open for community night {turn}."),
        ),
        ("1980s-town", false) => (
            SLOT_D,
            "route",
            format!("Lena's late loop {turn}"),
            format!(
                "Lena rode Night Bus 6 through late loop {turn} and came back with a new story."
            ),
        ),
        ("penguin-civilization", true) => (
            SLOT_A,
            "status",
            format!("Piko reinforced span {turn}"),
            format!("Piko reinforced Icebridge span {turn} before the next cold tide."),
        ),
        ("penguin-civilization", false) => (
            SLOT_D,
            "custom",
            format!("Piko's edge report {turn}"),
            format!("Piko returned from edge scout {turn} with a new route under the aurora."),
        ),
        _ => {
            return Err(ActionError::Invalid(format!(
                "unsupported Pocket Universe seed: {seed}"
            )))
        }
    };
    Ok(outcome)
}

impl Action for ChooseBoldPath {
    fn name(&self) -> &'static str {
        "choose_bold_path"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        choice_draft(state, true)
    }
}

impl Action for ChooseCarefulPath {
    fn name(&self) -> &'static str {
        "choose_careful_path"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        choice_draft(state, false)
    }
}

fn choice_draft(state: &WorldState, bold: bool) -> Result<EventDraft, ActionError> {
    let seed = seed_id_from_state(state)?;
    if seed == UNSEEDED {
        return Err(ActionError::Invalid(
            "choose a Pocket Universe seed before intervening".into(),
        ));
    }
    if integer_component(state, UNIVERSE, GENERATION)? < 3 {
        return Err(ActionError::Invalid(
            "this Pocket Universe has not grown enough for that choice yet".into(),
        ));
    }
    if decision_id_from_state(state)? != "none" {
        return Err(ActionError::Invalid(
            "this Pocket Universe has already crossed its first intervention point".into(),
        ));
    }

    let (choice, summary, target, key, value) = match (seed.as_str(), bold) {
        ("mars-colony", true) => (
            "follow-signal",
            "Kestrel leaves the safe route to follow a repeating signal beyond the ridge.",
            SLOT_D,
            "status",
            "signal expedition",
        ),
        ("mars-colony", false) => (
            "fortify-habitat",
            "The colony diverts its spare capacity into sealing Ares Habitat before the next dust front.",
            SLOT_A,
            "status",
            "storm sealed",
        ),
        ("1980s-town", true) => (
            "community-arcade",
            "Maple Arcade turns its late hours into a neighborhood club instead of closing the shutters.",
            SLOT_A,
            "status",
            "community nights",
        ),
        ("1980s-town", false) => (
            "steady-business",
            "Maple Arcade keeps a quieter commercial rhythm and protects its small cash buffer.",
            SLOT_A,
            "status",
            "steady business",
        ),
        ("penguin-civilization", true) => (
            "winter-feast",
            "Icebridge opens the Fish Vault for a winter feast that brings distant colonies onto the bridge.",
            SLOT_C,
            "reserve",
            "festival opened",
        ),
        ("penguin-civilization", false) => (
            "conserve-reserves",
            "The Aurora Council keeps the Fish Vault sealed and stores extra reserves for the dark season.",
            SLOT_C,
            "reserve",
            "winter conserved",
        ),
        _ => {
            return Err(ActionError::Invalid(format!(
                "unsupported Pocket Universe seed: {seed}"
            )))
        }
    };

    let mut draft = EventDraft::new("universe_intervened");
    draft.targets = vec![UNIVERSE, target];
    draft.payload.insert("choice".into(), choice.into());
    draft.payload.insert("summary".into(), summary.into());
    draft.changes = vec![
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: DECISION.into(),
            value: choice.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),
            value: summary.into(),
        },
        StateChange::SetComponent {
            entity: target,
            key: key.into(),
            value: value.into(),
        },
    ];
    Ok(draft)
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
            key: DECISION.into(),
            value: "none".into(),
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

fn decision_id_from_state(state: &WorldState) -> Result<String, ActionError> {
    match state
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(DECISION))
    {
        Some(Value::Text(decision)) => Ok(decision.clone()),
        _ => Err(ActionError::Invalid(
            "Pocket Universe decision state is missing".into(),
        )),
    }
}

fn growth_metric(state: &WorldState, seed: &str) -> Result<(&'static str, i64), ActionError> {
    let key = match seed {
        "mars-colony" => "water_cycles",
        "1980s-town" => "high_scores",
        "penguin-civilization" => "bridge_spans",
        _ => {
            return Err(ActionError::Invalid(format!(
                "unsupported Pocket Universe seed: {seed}"
            )))
        }
    };
    Ok((key, integer_component(state, SLOT_A, key)? + 1))
}

fn integer_component(state: &WorldState, entity: EntityId, key: &str) -> Result<i64, ActionError> {
    match state
        .entity(entity)
        .and_then(|entity| entity.component(key))
    {
        Some(Value::Integer(value)) => Ok(*value),
        _ => Err(ActionError::Invalid(format!(
            "entity {entity} has no integer component {key}"
        ))),
    }
}

fn growth_message(seed: &str, generation: i64, decision: &str) -> String {
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
    let base = match seed {
        "mars-colony" => messages[0][cycle],
        "1980s-town" => messages[1][cycle],
        "penguin-civilization" => messages[2][cycle],
        _ => "The world changed in a small but persistent way.",
    };
    if decision == "none" {
        return base.into();
    }
    let consequence = match decision {
        "follow-signal" => "The signal expedition keeps pulling attention beyond the safe ridge.",
        "fortify-habitat" => "The stronger habitat makes every later risk feel more deliberate.",
        "community-arcade" => {
            "The arcade is becoming a place people organize their evenings around."
        }
        "steady-business" => "The arcade survives by staying small, predictable, and open.",
        "winter-feast" => {
            "The feast has turned Icebridge into a meeting point for distant colonies."
        }
        "conserve-reserves" => {
            "The sealed reserve gives the council more room to plan for the dark season."
        }
        _ => "The earlier intervention is still shaping what happens next.",
    };
    format!("{base} {consequence}")
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
    use world_agent::MockAgentRuntime;

    fn registry() -> world_host::WorldRegistry {
        let mut registry = world_host::WorldRegistry::new();
        registry.register(pocket_universe_registration()).unwrap();
        registry
    }

    #[test]
    fn registration_factory_creates_a_fresh_runtime_for_create_and_open() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let created = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&created);
        let registration = pocket_universe_registration_with_agent_runtime(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            PanicMind
        });
        let mut registry = world_host::WorldRegistry::new();
        registry.register(registration).unwrap();

        let session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        assert_eq!(created.load(Ordering::SeqCst), 1);
        let archive = session.archive().unwrap().unwrap();
        drop(session);

        let reopened = registry.open_archive(&archive).unwrap();
        assert_eq!(created.load(Ordering::SeqCst), 2);
        assert_eq!(reopened.archive().unwrap().unwrap(), archive);
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
        assert_ne!(
            mars_snapshot.collection.items,
            town_snapshot.collection.items
        );
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
        let new_events = &session.archive().unwrap().unwrap().events[before.timeline.items.len()..];
        assert_eq!(
            new_events
                .iter()
                .filter(|event| event.kind == "universe_grew")
                .count(),
            2
        );
        assert_eq!(
            new_events
                .iter()
                .filter(|event| event.kind == "agent_decision_recorded")
                .count(),
            2
        );
        assert_eq!(
            new_events
                .iter()
                .filter(|event| {
                    event.kind == "agent_cared_for_world" || event.kind == "agent_explored_world"
                })
                .count(),
            2
        );
        let briefing = after.briefing.as_ref().unwrap();
        assert_eq!(briefing.title, "While you were away");
        assert_eq!(briefing.items.len(), 3);
        assert!(briefing
            .items
            .iter()
            .all(|item| !item.detail.trim().is_empty()));
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
        session
            .handle(ProjectionIntent::InvokeCommand(NUDGE_COMMAND.into()))
            .unwrap();
        let before = session.snapshot();
        let archive = session.archive().unwrap().unwrap();
        drop(session);

        let reopened = registry.open_archive(&archive).unwrap();

        assert_eq!(reopened.snapshot(), before);
        assert_eq!(reopened.archive().unwrap().unwrap(), archive);
    }

    #[test]
    fn scripted_mind_selects_only_offered_actions_and_records_causal_outcome() {
        let mut universe =
            PocketUniverse::with_agent_runtime(MockAgentRuntime::scripted([AGENT_EXPLORE_ACTION]))
                .unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(1).unwrap();

        let decision = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "agent_decision_recorded")
            .unwrap();
        let outcome = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "agent_explored_world")
            .unwrap();
        assert_eq!(decision.actor, Some(SLOT_B));
        assert!(outcome.caused_by.contains(&decision.id));
        assert!(outcome.caused_by.iter().any(|cause| universe
            .world()
            .event(*cause)
            .is_some_and(|event| event.kind == "universe_grew")));
        assert_eq!(
            universe
                .world()
                .state()
                .entity(SLOT_B)
                .unwrap()
                .component(AGENT_EXPLORE_COUNT),
            Some(&Value::Integer(1))
        );
    }

    struct FailingMind;

    impl AgentRuntime for FailingMind {
        fn decide(
            &mut self,
            _observation: &AgentObservation,
            _actions: &[AvailableAction],
        ) -> Result<AgentDecision, AgentRuntimeError> {
            Err(AgentRuntimeError::new("Pocket Mind is unavailable"))
        }
    }

    #[test]
    fn nudge_runtime_failure_leaves_durable_world_unchanged() {
        let mut universe = PocketUniverse::with_agent_runtime(FailingMind).unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        let before = universe.archive().unwrap();

        let error = universe
            .invoke_projection_command(NUDGE_COMMAND)
            .unwrap_err();

        assert!(error.to_string().contains("Pocket Mind is unavailable"));
        assert_eq!(universe.archive().unwrap(), before);
        assert_eq!(universe.world().world_time(), 0);
    }

    #[test]
    fn multi_period_failure_rolls_back_all_candidate_growth_and_agent_events() {
        let mut universe = PocketUniverse::with_agent_runtime(MockAgentRuntime::scripted([
            AGENT_CARE_ACTION,
            "not-an-offered-action",
        ]))
        .unwrap();
        universe
            .invoke_projection_command(SEED_PENGUIN_CIVILIZATION_COMMAND)
            .unwrap();
        let before = universe.archive().unwrap();

        let error = universe.advance_periods(2).unwrap_err();

        assert!(error.to_string().contains("unavailable action"));
        assert_eq!(universe.archive().unwrap(), before);
        assert_eq!(universe.world().world_time(), 0);
    }

    #[test]
    fn deterministic_mind_uses_durable_actor_memory_even_without_time_advancing() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();

        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();

        let actor = universe.world().state().entity(SLOT_B).unwrap();
        assert_eq!(actor.component(AGENT_CARE_COUNT), Some(&Value::Integer(1)));
        assert_eq!(
            actor.component(AGENT_EXPLORE_COUNT),
            Some(&Value::Integer(1))
        );
        assert_eq!(universe.world().world_time(), 0);
        let decisions = universe
            .world()
            .events()
            .iter()
            .filter(|event| event.kind == "agent_decision_recorded")
            .filter_map(|event| event.payload.get("selected_action"))
            .collect::<Vec<_>>();
        assert_eq!(
            decisions,
            vec![
                &Value::Text(AGENT_CARE_ACTION.into()),
                &Value::Text(AGENT_EXPLORE_ACTION.into())
            ]
        );
    }

    #[test]
    fn deterministic_default_mind_keeps_identical_worlds_reproducible() {
        let mut left = PocketUniverse::new().unwrap();
        let mut right = PocketUniverse::new().unwrap();
        left.invoke_projection_command(SEED_PENGUIN_CIVILIZATION_COMMAND)
            .unwrap();
        right
            .invoke_projection_command(SEED_PENGUIN_CIVILIZATION_COMMAND)
            .unwrap();

        left.advance_periods(4).unwrap();
        right.advance_periods(4).unwrap();

        assert_eq!(left.archive().unwrap(), right.archive().unwrap());
        assert_eq!(left.projection_snapshot(), right.projection_snapshot());
    }

    #[test]
    fn return_briefing_hides_agent_plumbing_but_keeps_agent_outcomes() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_1980S_TOWN_COMMAND.into(),
            ))
            .unwrap();
        let returned = session.advance_background(2).unwrap();
        let briefing = returned.briefing.as_ref().unwrap();

        assert_eq!(briefing.title, "While you were away");
        assert_eq!(briefing.items.len(), 3);
        assert!(briefing
            .items
            .iter()
            .all(|item| item.title != "agent decision recorded"));
        assert_eq!(
            briefing
                .items
                .iter()
                .filter(|item| item.detail.contains("Lena"))
                .count(),
            2
        );
    }

    struct PanicMind;

    impl AgentRuntime for PanicMind {
        fn decide(
            &mut self,
            _observation: &AgentObservation,
            _actions: &[AvailableAction],
        ) -> Result<AgentDecision, AgentRuntimeError> {
            panic!("archive restore must never call the agent runtime")
        }
    }

    #[test]
    fn archive_restore_does_not_call_the_mind() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(2).unwrap();
        let archive = universe.archive().unwrap();

        let restored =
            PocketUniverse::resume_archive_with_agent_runtime(&archive, PanicMind).unwrap();

        assert_eq!(restored.archive().unwrap(), archive);
        assert_eq!(restored.world().events(), universe.world().events());
    }

    #[test]
    fn generation_three_exposes_a_durable_intervention() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_1980S_TOWN_COMMAND.into(),
            ))
            .unwrap();
        let grown = session.advance_background(3).unwrap();
        let command_ids = grown
            .commands
            .iter()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>();
        assert!(command_ids.contains(&BOLD_PATH_COMMAND));
        assert!(command_ids.contains(&CAREFUL_PATH_COMMAND));

        let chosen = session
            .handle(ProjectionIntent::InvokeCommand(BOLD_PATH_COMMAND.into()))
            .unwrap();
        assert_eq!(chosen.briefing.as_ref().unwrap().title, "Generation 3");
        assert!(!chosen
            .commands
            .iter()
            .any(|command| command.id == BOLD_PATH_COMMAND || command.id == CAREFUL_PATH_COMMAND));
        let universe = chosen
            .inspectors
            .get(&world_projection::SelectionId::Entity(UNIVERSE))
            .unwrap();
        assert!(universe
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .any(|row| { row.label == "Decision" && row.value == "community-arcade" }));

        let archive = session.archive().unwrap().unwrap();
        drop(session);
        let reopened = registry.open_archive(&archive).unwrap();
        assert_eq!(reopened.archive().unwrap().unwrap(), archive);
        assert!(!reopened
            .snapshot()
            .commands
            .iter()
            .any(|command| command.id == BOLD_PATH_COMMAND || command.id == CAREFUL_PATH_COMMAND));
    }

    #[test]
    fn forking_before_intervention_reopens_the_choice() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_PENGUIN_CIVILIZATION_COMMAND.into(),
            ))
            .unwrap();
        session.advance_background(3).unwrap();
        let chosen = session
            .handle(ProjectionIntent::InvokeCommand(CAREFUL_PATH_COMMAND.into()))
            .unwrap();
        let intervention = chosen
            .timeline
            .items
            .iter()
            .find(|item| item.title == "Universe Intervened")
            .and_then(|item| match item.id {
                world_projection::SelectionId::Event(id) => Some(id),
                _ => None,
            })
            .unwrap();

        let forked = session
            .handle(ProjectionIntent::ForkBeforeEvent(intervention))
            .unwrap();
        assert!(forked
            .commands
            .iter()
            .any(|command| command.id == BOLD_PATH_COMMAND));
        assert!(forked
            .commands
            .iter()
            .any(|command| command.id == CAREFUL_PATH_COMMAND));
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
