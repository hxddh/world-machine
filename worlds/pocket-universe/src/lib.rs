mod legacy;
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
pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.13.0";

pub const SEED_MARS_COLONY_COMMAND: &str = "pocket-universe.seed-mars-colony";
pub const SEED_1980S_TOWN_COMMAND: &str = "pocket-universe.seed-1980s-town";
pub const SEED_PENGUIN_CIVILIZATION_COMMAND: &str = "pocket-universe.seed-penguin-civilization";
pub const NUDGE_COMMAND: &str = "pocket-universe.nudge";
pub const BOLD_PATH_COMMAND: &str = "pocket-universe.choose-bold-path";
pub const CAREFUL_PATH_COMMAND: &str = "pocket-universe.choose-careful-path";
pub const SHARED_PROJECT_COMMAND: &str = "pocket-universe.relationship-shared-project";
pub const RIVALRY_COMMAND: &str = "pocket-universe.relationship-rivalry";
pub const OUTWARD_POSTURE_COMMAND: &str = "pocket-universe.posture-outward";
pub const ROOTED_POSTURE_COMMAND: &str = "pocket-universe.posture-rooted";

pub(crate) const UNIVERSE: EntityId = EntityId::new(1);
pub(crate) const SLOT_A: EntityId = EntityId::new(10);
pub(crate) const SLOT_B: EntityId = EntityId::new(11);
pub(crate) const SLOT_C: EntityId = EntityId::new(12);
pub(crate) const SLOT_D: EntityId = EntityId::new(13);
pub(crate) const SLOT_E: EntityId = EntityId::new(14);
pub(crate) const RELATIONSHIP: EntityId = EntityId::new(15);

pub(crate) const SEED: &str = "seed";
pub(crate) const GENERATION: &str = "generation";
pub(crate) const LAST_CHANGE: &str = "last_change";
pub(crate) const DECISION: &str = "decision";
pub(crate) const POSTURE: &str = "posture";
pub(crate) const POSTURE_GENERATION: &str = "posture_generation";
pub(crate) const LEGACY: &str = "legacy";
pub(crate) const LEGACY_SUMMARY: &str = "legacy_summary";
pub(crate) const RELATIONSHIP_DIRECTION: &str = "direction";
const RELATIONSHIP_TRUST: &str = "trust";
const RELATIONSHIP_TENSION: &str = "tension";
const RELATIONSHIP_LAST_DYNAMIC: &str = "last_dynamic";
const RELATIONSHIP_SOCIAL_ARC: &str = "social_arc";
const ANCHOR_PULSE: &str = "pulse";
const UNSEEDED: &str = "unseeded";
const BACKGROUND_PERIOD: u64 = 10;
const AGENT_CARE_ACTION: &str = "pocket_agent.care";
const AGENT_EXPLORE_ACTION: &str = "pocket_agent.explore";
const AGENT_CARE_COUNT: &str = "care_count";
const AGENT_EXPLORE_COUNT: &str = "explore_count";
const MIND_PROFILE_ARG: &str = "mind_profile";
const LAST_MIND_PROFILE: &str = "last_mind_profile";
const DETERMINISTIC_MIND_PROFILE: &str = "deterministic";
const CUSTOM_MIND_PROFILE: &str = "custom";

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
        let relationship = observation
            .entities
            .iter()
            .find(|entity| entity.id == RELATIONSHIP)
            .ok_or_else(|| {
                AgentRuntimeError::new("Pocket Mind observation is missing relationship state")
            })?;
        let direction = match relationship.component(RELATIONSHIP_DIRECTION) {
            Some(Value::Text(direction)) => direction.as_str(),
            _ => {
                return Err(AgentRuntimeError::new(
                    "Pocket Mind relationship is missing its direction",
                ))
            }
        };
        let universe = observation
            .entities
            .iter()
            .find(|entity| entity.id == UNIVERSE)
            .ok_or_else(|| {
                AgentRuntimeError::new("Pocket Mind observation is missing its World")
            })?;
        let posture = match universe.component(POSTURE) {
            Some(Value::Text(posture)) => posture.as_str(),
            _ => {
                return Err(AgentRuntimeError::new(
                    "Pocket Mind World is missing its posture",
                ))
            }
        };
        let primary_outcome = observation.events.iter().rev().find(|event| {
            event.actor == Some(SLOT_B)
                && matches!(
                    event.kind.as_str(),
                    "agent_cared_for_world" | "agent_explored_world"
                )
        });
        let fallback = match posture {
            "outward" => AGENT_EXPLORE_ACTION,
            "rooted" => AGENT_CARE_ACTION,
            "none" if care_count <= explore_count => AGENT_CARE_ACTION,
            "none" => AGENT_EXPLORE_ACTION,
            other => {
                return Err(AgentRuntimeError::new(format!(
                    "Pocket Mind World has unknown posture {other}"
                )))
            }
        };
        let desired = if observation.actor == SLOT_E {
            match (direction, primary_outcome.map(|event| event.kind.as_str())) {
                ("rivalry", Some("agent_cared_for_world")) => AGENT_CARE_ACTION,
                ("rivalry", Some("agent_explored_world")) => AGENT_EXPLORE_ACTION,
                (_, Some("agent_cared_for_world")) => AGENT_EXPLORE_ACTION,
                (_, Some("agent_explored_world")) => AGENT_CARE_ACTION,
                (_, _) => fallback,
            }
        } else {
            fallback
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
    mind_profile: String,
}

impl PocketUniverse<PocketMind> {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Self::with_agent_runtime_profile(PocketMind, DETERMINISTIC_MIND_PROFILE)
    }

    pub fn resume_archive(archive: &WorldArchive) -> Result<Self, Box<dyn Error>> {
        Self::resume_archive_with_agent_runtime_profile(
            archive,
            PocketMind,
            DETERMINISTIC_MIND_PROFILE,
        )
    }
}

impl<R> PocketUniverse<R>
where
    R: AgentRuntime,
{
    pub fn with_agent_runtime(mind: R) -> Result<Self, Box<dyn Error>> {
        Self::with_agent_runtime_profile(mind, CUSTOM_MIND_PROFILE)
    }

    pub fn with_agent_runtime_profile(
        mind: R,
        mind_profile: impl Into<String>,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            world: World::new(baseline()?),
            actions: build_action_registry()?,
            mind,
            mind_profile: validate_mind_profile(mind_profile.into())?,
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
            let primary_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                SLOT_B,
                &[growth],
            )?;
            let secondary_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                SLOT_E,
                &[primary_outcome],
            )?;
            let relationship = candidate
                .execute(
                    &self.actions,
                    &ActionRequest::new("update_relationship")
                        .caused_by(primary_outcome)
                        .caused_by(secondary_outcome),
                )?
                .id;
            let returned =
                legacy::resolve_period_consequences(&mut candidate, &self.actions, relationship)?;
            self.world = candidate;
            return Ok(returned);
        }

        let action = match command_id {
            SEED_MARS_COLONY_COMMAND => "seed_mars_colony",
            SEED_1980S_TOWN_COMMAND => "seed_1980s_town",
            SEED_PENGUIN_CIVILIZATION_COMMAND => "seed_penguin_civilization",
            BOLD_PATH_COMMAND => "choose_bold_path",
            CAREFUL_PATH_COMMAND => "choose_careful_path",
            SHARED_PROJECT_COMMAND => "steer_shared_project",
            RIVALRY_COMMAND => "steer_rivalry",
            OUTWARD_POSTURE_COMMAND => "choose_outward_posture",
            ROOTED_POSTURE_COMMAND => "choose_rooted_posture",
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
            let primary_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                SLOT_B,
                &[growth],
            )?;
            let secondary_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                SLOT_E,
                &[primary_outcome],
            )?;
            let relationship = candidate
                .execute(
                    &self.actions,
                    &ActionRequest::new("update_relationship")
                        .caused_by(primary_outcome)
                        .caused_by(secondary_outcome),
                )?
                .id;
            legacy::resolve_period_consequences(&mut candidate, &self.actions, relationship)?;
        }
        self.world = candidate;
        Ok(())
    }

    fn run_agent_turn_on(
        mind: &mut R,
        world: &mut World,
        registry: &ActionRegistry,
        mind_profile: &str,
        actor: EntityId,
        caused_by: &[EventId],
    ) -> Result<EventId, Box<dyn Error>> {
        let actions = vec![
            AvailableAction::new(
                "Care for the small world and reinforce what already exists.",
                ActionRequest::new(AGENT_CARE_ACTION).arg(MIND_PROFILE_ARG, mind_profile),
            ),
            AvailableAction::new(
                "Explore beyond the familiar routine and bring back a new thread.",
                ActionRequest::new(AGENT_EXPLORE_ACTION).arg(MIND_PROFILE_ARG, mind_profile),
            ),
        ];
        let execution = AgentExecutor::decide_and_execute(
            mind,
            &ScopedPerception::new([UNIVERSE, SLOT_A, SLOT_B, SLOT_E, RELATIONSHIP]),
            world,
            registry,
            actor,
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
        Self::resume_archive_with_agent_runtime_profile(archive, mind, CUSTOM_MIND_PROFILE)
    }

    pub fn resume_archive_with_agent_runtime_profile(
        archive: &WorldArchive,
        mind: R,
        mind_profile: impl Into<String>,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            world: archive.restore(&pocket_universe_pack_ref(), baseline()?)?,
            actions: build_action_registry()?,
            mind,
            mind_profile: validate_mind_profile(mind_profile.into())?,
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
    fn fresh(mind: R, mind_profile: &str) -> Result<Box<dyn WorldSession>, HostError> {
        Ok(Box::new(Self {
            world: PocketUniverse::with_agent_runtime_profile(mind, mind_profile)
                .map_err(HostError::session)?,
            return_since_event_count: None,
        }))
    }

    fn open_archive(
        archive: &WorldArchive,
        mind: R,
        mind_profile: &str,
    ) -> Result<Box<dyn WorldSession>, HostError> {
        Ok(Box::new(Self {
            world: PocketUniverse::resume_archive_with_agent_runtime_profile(
                archive,
                mind,
                mind_profile,
            )
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
    registration_with_validated_profile(|| PocketMind, DETERMINISTIC_MIND_PROFILE)
}

pub fn pocket_universe_registration_with_agent_runtime<R, F>(factory: F) -> WorldRegistration
where
    R: AgentRuntime + 'static,
    F: Fn() -> R + Send + Sync + 'static,
{
    registration_with_validated_profile(factory, CUSTOM_MIND_PROFILE)
}

pub fn pocket_universe_registration_with_agent_runtime_profile<R, F>(
    factory: F,
    mind_profile: impl Into<String>,
) -> Result<WorldRegistration, std::io::Error>
where
    R: AgentRuntime + 'static,
    F: Fn() -> R + Send + Sync + 'static,
{
    let mind_profile = validate_mind_profile(mind_profile.into())?;
    Ok(registration_with_validated_profile(factory, mind_profile))
}

fn registration_with_validated_profile<R, F>(
    factory: F,
    mind_profile: impl Into<String>,
) -> WorldRegistration
where
    R: AgentRuntime + 'static,
    F: Fn() -> R + Send + Sync + 'static,
{
    let factory = Arc::new(factory);
    let mind_profile = Arc::new(mind_profile.into());
    let create_factory = Arc::clone(&factory);
    let open_factory = Arc::clone(&factory);
    let create_profile = Arc::clone(&mind_profile);
    let open_profile = Arc::clone(&mind_profile);
    WorldRegistration::new(pocket_universe_descriptor(), move || {
        PocketUniverseSession::fresh(create_factory(), create_profile.as_str())
    })
    .with_archive_opener(move |archive| {
        PocketUniverseSession::open_archive(archive, open_factory(), open_profile.as_str())
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
            .with_component(POSTURE, "none")
            .with_component(POSTURE_GENERATION, 0_i64)
            .with_component(LEGACY, "forming")
            .with_component(LEGACY_SUMMARY, "")
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
    actions.register(ChooseOutwardPosture)?;
    actions.register(ChooseRootedPosture)?;
    actions.register(CareForWorld)?;
    actions.register(ExploreWorld)?;
    actions.register(UpdateRelationship)?;
    actions.register(ResolveSocialArc)?;
    legacy::register_actions(&mut actions)?;
    actions.register(SteerSharedProject)?;
    actions.register(SteerRivalry)?;
    Ok(actions)
}

struct SeedMarsColony;
struct Seed1980sTown;
struct SeedPenguinCivilization;
struct GrowUniverse;
struct ChooseBoldPath;
struct ChooseCarefulPath;
struct ChooseOutwardPosture;
struct ChooseRootedPosture;
struct CareForWorld;
struct ExploreWorld;
struct UpdateRelationship;
struct ResolveSocialArc;
struct SteerSharedProject;
struct SteerRivalry;

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
                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)
                    .with_component(LAST_MIND_PROFILE, "none"),
                Entity::new(SLOT_C, "place")
                    .with_component("name", "Hydroponics Bay")
                    .with_component("crop", "dwarf wheat"),
                Entity::new(SLOT_D, "rover")
                    .with_component("name", "Kestrel Rover")
                    .with_component("range", "18 km"),
                Entity::new(SLOT_E, "person")
                    .with_component("name", "Tomas Vale")
                    .with_component("role", "rover scout")
                    .with_component(AGENT_CARE_COUNT, 0_i64)
                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)
                    .with_component(LAST_MIND_PROFILE, "none"),
                relationship_entity("Nia ↔ Tomas"),
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
                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)
                    .with_component(LAST_MIND_PROFILE, "none"),
                Entity::new(SLOT_C, "radio_station")
                    .with_component("name", "K-88 Radio")
                    .with_component("format", "local mix"),
                Entity::new(SLOT_D, "bus")
                    .with_component("name", "Night Bus 6")
                    .with_component("route", "Maple Loop"),
                Entity::new(SLOT_E, "person")
                    .with_component("name", "Max Park")
                    .with_component("role", "radio volunteer")
                    .with_component(AGENT_CARE_COUNT, 0_i64)
                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)
                    .with_component(LAST_MIND_PROFILE, "none"),
                relationship_entity("Lena ↔ Max"),
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
                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)
                    .with_component(LAST_MIND_PROFILE, "none"),
                Entity::new(SLOT_C, "storehouse")
                    .with_component("name", "Fish Vault")
                    .with_component("reserve", "steady"),
                Entity::new(SLOT_D, "council")
                    .with_component("name", "Aurora Council")
                    .with_component("custom", "vote at moonrise"),
                Entity::new(SLOT_E, "penguin")
                    .with_component("name", "Miri")
                    .with_component("role", "fish-vault keeper")
                    .with_component(AGENT_CARE_COUNT, 0_i64)
                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)
                    .with_component(LAST_MIND_PROFILE, "none"),
                relationship_entity("Piko ↔ Miri"),
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
        let posture = posture_id_from_state(state)?;
        let social_arc = text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC)?;
        let legacy = legacy::legacy_id_from_state(state)?;
        let change = growth_message(&seed, next, &decision, &social_arc, &posture, &legacy);
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
    if actor != SLOT_B && actor != SLOT_E {
        return Err(ActionError::Invalid(format!(
            "Pocket Mind action requires a seeded actor ({SLOT_B} or {SLOT_E}), got {actor}"
        )));
    }
    let mind_profile = match request.args.get(MIND_PROFILE_ARG) {
        Some(Value::Text(profile)) if is_valid_mind_profile(profile) => profile.clone(),
        _ => {
            return Err(ActionError::Invalid(
                "Pocket Mind action requires a valid mind_profile label".into(),
            ))
        }
    };
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
    let (target, key, value, change) = mind_outcome(&seed, actor, care, next)?;
    let mut draft = EventDraft::new(if care {
        "agent_cared_for_world"
    } else {
        "agent_explored_world"
    });
    draft.targets = vec![actor, target];
    draft.payload.insert("seed".into(), seed.into());
    draft.payload.insert("change".into(), change.clone().into());
    draft.payload.insert("turn".into(), next.into());
    draft
        .payload
        .insert(MIND_PROFILE_ARG.into(), mind_profile.clone().into());
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
            entity: actor,
            key: LAST_MIND_PROFILE.into(),
            value: mind_profile.into(),
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
    actor: EntityId,
    care: bool,
    turn: i64,
) -> Result<(EntityId, &'static str, String, String), ActionError> {
    let outcome = match (seed, actor, care) {
        ("mars-colony", SLOT_B, true) => (
            SLOT_C,
            "crop",
            format!("Nia tending cycle {turn}"),
            format!("Nia tuned the hydroponics loop for care cycle {turn}."),
        ),
        ("mars-colony", SLOT_B, false) => (
            SLOT_D,
            "range",
            format!("Nia survey route {turn}"),
            format!("Nia sent Kestrel onto survey route {turn} beyond the familiar markers."),
        ),
        ("mars-colony", SLOT_E, true) => (
            SLOT_D,
            "status",
            format!("Tomas service cycle {turn}"),
            format!("Tomas serviced Kestrel after Nia's latest move, closing out maintenance cycle {turn}."),
        ),
        ("mars-colony", SLOT_E, false) => (
            SLOT_A,
            "survey_report",
            format!("ridge trace {turn}"),
            format!("Tomas followed Nia's lead and returned with ridge trace {turn} for Ares Habitat."),
        ),
        ("1980s-town", SLOT_B, true) => (
            SLOT_A,
            "status",
            format!("Lena's community night {turn}"),
            format!("Lena kept Maple Arcade open for community night {turn}."),
        ),
        ("1980s-town", SLOT_B, false) => (
            SLOT_D,
            "route",
            format!("Lena's late loop {turn}"),
            format!("Lena rode Night Bus 6 through late loop {turn} and came back with a new story."),
        ),
        ("1980s-town", SLOT_E, true) => (
            SLOT_C,
            "format",
            format!("Max community set {turn}"),
            format!("Max answered Lena's latest move with community set {turn} on K-88."),
        ),
        ("1980s-town", SLOT_E, false) => (
            SLOT_D,
            "route",
            format!("Max signal chase {turn}"),
            format!("Max followed the thread from Lena's night and mapped signal chase {turn} along Bus 6."),
        ),
        ("penguin-civilization", SLOT_B, true) => (
            SLOT_A,
            "status",
            format!("Piko reinforced span {turn}"),
            format!("Piko reinforced Icebridge span {turn} before the next cold tide."),
        ),
        ("penguin-civilization", SLOT_B, false) => (
            SLOT_D,
            "custom",
            format!("Piko's edge report {turn}"),
            format!("Piko returned from edge scout {turn} with a new route under the aurora."),
        ),
        ("penguin-civilization", SLOT_E, true) => (
            SLOT_C,
            "reserve",
            format!("Miri reserve cycle {turn}"),
            format!("Miri answered Piko's latest move by balancing Fish Vault reserve cycle {turn}."),
        ),
        ("penguin-civilization", SLOT_E, false) => (
            SLOT_D,
            "custom",
            format!("Miri tide map {turn}"),
            format!("Miri followed Piko's trail and brought the Aurora Council tide map {turn}."),
        ),
        _ => {
            return Err(ActionError::Invalid(format!(
                "unsupported Pocket Universe mind outcome: seed={seed}, actor={actor}, care={care}"
            )))
        }
    };
    Ok(outcome)
}

impl Action for UpdateRelationship {
    fn name(&self) -> &'static str {
        "update_relationship"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let primary = text_component_from_state(state, SLOT_B, "last_intent")?;
        let secondary = text_component_from_state(state, SLOT_E, "last_intent")?;
        let direction = text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_DIRECTION)?;
        let trust = integer_component(state, RELATIONSHIP, RELATIONSHIP_TRUST)?;
        let tension = integer_component(state, RELATIONSHIP, RELATIONSHIP_TENSION)?;

        let (mut trust_delta, mut tension_delta, dynamic) =
            match (primary.as_str(), secondary.as_str()) {
                ("care", "care") => (2, -1, "They reinforced the same fragile thing together."),
                ("explore", "explore") => (
                    -1,
                    2,
                    "They chased the same frontier and began to compete for it.",
                ),
                ("care", "explore") | ("explore", "care") => (
                    1,
                    -1,
                    "Their different instincts covered each other's blind spots.",
                ),
                _ => {
                    return Err(ActionError::Invalid(
                        "relationship update requires both actors to have acted".into(),
                    ))
                }
            };
        match direction.as_str() {
            "shared-project" => {
                trust_delta += 1;
                tension_delta -= 1;
            }
            "rivalry" => {
                tension_delta += 1;
            }
            "none" => {}
            other => {
                return Err(ActionError::Invalid(format!(
                    "unknown relationship direction: {other}"
                )))
            }
        }

        let next_trust = (trust + trust_delta).clamp(0, 10);
        let next_tension = (tension + tension_delta).clamp(0, 10);
        let summary = format!("{dynamic} Trust is {next_trust}; tension is {next_tension}.");
        let mut draft = EventDraft::new("relationship_shifted");
        draft.targets = vec![RELATIONSHIP, SLOT_B, SLOT_E];
        draft
            .payload
            .insert("summary".into(), summary.clone().into());
        draft.payload.insert("trust".into(), next_trust.into());
        draft.payload.insert("tension".into(), next_tension.into());
        draft.payload.insert("direction".into(), direction.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: RELATIONSHIP_TRUST.into(),
                value: next_trust.into(),
            },
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: RELATIONSHIP_TENSION.into(),
                value: next_tension.into(),
            },
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: RELATIONSHIP_LAST_DYNAMIC.into(),
                value: summary.clone().into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LAST_CHANGE.into(),
                value: summary.into(),
            },
        ];
        Ok(draft)
    }
}

fn social_arc_candidate(state: &WorldState) -> Result<Option<&'static str>, ActionError> {
    if text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC)? != "forming" {
        return Ok(None);
    }
    let direction = text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_DIRECTION)?;
    let trust = integer_component(state, RELATIONSHIP, RELATIONSHIP_TRUST)?;
    let tension = integer_component(state, RELATIONSHIP, RELATIONSHIP_TENSION)?;

    if direction == "shared-project" && trust >= 5 {
        return Ok(Some("partnership"));
    }
    if direction == "rivalry" && tension >= 5 {
        return Ok(Some("fracture"));
    }
    if trust >= 5 && trust >= tension + 2 {
        return Ok(Some("partnership"));
    }
    if tension >= 5 && tension >= trust + 2 {
        return Ok(Some("fracture"));
    }
    Ok(None)
}

impl Action for ResolveSocialArc {
    fn name(&self) -> &'static str {
        "resolve_social_arc"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let arc = social_arc_candidate(state)?.ok_or_else(|| {
            ActionError::Invalid("relationship has not reached a social-arc threshold".into())
        })?;
        let seed = seed_id_from_state(state)?;
        let trust = integer_component(state, RELATIONSHIP, RELATIONSHIP_TRUST)?;
        let tension = integer_component(state, RELATIONSHIP, RELATIONSHIP_TENSION)?;
        let (kind, summary, target, key, value) = match (seed.as_str(), arc) {
            ("mars-colony", "partnership") => (
                "partnership_formed",
                "Nia and Tomas stopped dividing the work into separate turns. Kestrel now launches with them as one expedition crew.",
                SLOT_D,
                "social_status",
                "joint expedition crew",
            ),
            ("mars-colony", "fracture") => (
                "relationship_fractured",
                "Nia and Tomas stopped trusting the same route. Kestrel now runs split survey plans with competing priorities.",
                SLOT_D,
                "social_status",
                "split survey routes",
            ),
            ("1980s-town", "partnership") => (
                "partnership_formed",
                "Lena and Max turned their late-night improvisation into a real partnership. K-88 now carries a shared neighborhood show.",
                SLOT_C,
                "social_format",
                "Lena + Max neighborhood show",
            ),
            ("1980s-town", "fracture") => (
                "relationship_fractured",
                "Lena and Max began pulling the same audience in different directions. K-88 now schedules competing late shows.",
                SLOT_C,
                "social_format",
                "competing late shows",
            ),
            ("penguin-civilization", "partnership") => (
                "partnership_formed",
                "Piko and Miri turned their different duties into one shared watch. The Aurora Council now plans around their joint reports.",
                SLOT_D,
                "social_order",
                "shared watch council",
            ),
            ("penguin-civilization", "fracture") => (
                "relationship_fractured",
                "Piko and Miri split the colony's priorities into rival camps. The Aurora Council now meets as two moonrise caucuses.",
                SLOT_D,
                "social_order",
                "split moonrise caucuses",
            ),
            _ => {
                return Err(ActionError::Invalid(format!(
                    "unsupported Pocket Universe social arc: seed={seed}, arc={arc}"
                )))
            }
        };
        let mut draft = EventDraft::new(kind);
        draft.targets = vec![RELATIONSHIP, SLOT_B, SLOT_E, target];
        draft.payload.insert("social_arc".into(), arc.into());
        draft.payload.insert("trust".into(), trust.into());
        draft.payload.insert("tension".into(), tension.into());
        draft.payload.insert("summary".into(), summary.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: RELATIONSHIP_SOCIAL_ARC.into(),
                value: arc.into(),
            },
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: RELATIONSHIP_LAST_DYNAMIC.into(),
                value: summary.into(),
            },
            StateChange::SetComponent {
                entity: target,
                key: key.into(),
                value: value.into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LAST_CHANGE.into(),
                value: summary.into(),
            },
        ];
        Ok(draft)
    }
}

impl Action for SteerSharedProject {
    fn name(&self) -> &'static str {
        "steer_shared_project"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        steer_relationship_draft(state, "shared-project")
    }
}

impl Action for SteerRivalry {
    fn name(&self) -> &'static str {
        "steer_rivalry"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        steer_relationship_draft(state, "rivalry")
    }
}

fn steer_relationship_draft(
    state: &WorldState,
    direction: &str,
) -> Result<EventDraft, ActionError> {
    if integer_component(state, UNIVERSE, GENERATION)? < 2 {
        return Err(ActionError::Invalid(
            "the relationship has not developed enough to steer yet".into(),
        ));
    }
    if text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC)? != "forming" {
        return Err(ActionError::Invalid(
            "this relationship has already resolved into a social arc".into(),
        ));
    }
    if text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_DIRECTION)? != "none" {
        return Err(ActionError::Invalid(
            "this relationship already has a chosen direction".into(),
        ));
    }
    let trust = integer_component(state, RELATIONSHIP, RELATIONSHIP_TRUST)?;
    let tension = integer_component(state, RELATIONSHIP, RELATIONSHIP_TENSION)?;
    let (next_trust, next_tension, summary) = match direction {
        "shared-project" => (
            (trust + 2).clamp(0, 10),
            (tension - 1).clamp(0, 10),
            "You gave them something neither could finish alone. Their relationship now leans toward a shared project.",
        ),
        "rivalry" => (
            trust,
            (tension + 2).clamp(0, 10),
            "You let competition sharpen the space between them. Their relationship now leans toward rivalry.",
        ),
        _ => return Err(ActionError::Invalid("unknown relationship direction".into())),
    };
    let mut draft = EventDraft::new("relationship_steered");
    draft.targets = vec![RELATIONSHIP, SLOT_B, SLOT_E];
    draft.payload.insert("direction".into(), direction.into());
    draft.payload.insert("summary".into(), summary.into());
    draft.changes = vec![
        StateChange::SetComponent {
            entity: RELATIONSHIP,
            key: RELATIONSHIP_DIRECTION.into(),
            value: direction.into(),
        },
        StateChange::SetComponent {
            entity: RELATIONSHIP,
            key: RELATIONSHIP_TRUST.into(),
            value: next_trust.into(),
        },
        StateChange::SetComponent {
            entity: RELATIONSHIP,
            key: RELATIONSHIP_TENSION.into(),
            value: next_tension.into(),
        },
        StateChange::SetComponent {
            entity: RELATIONSHIP,
            key: RELATIONSHIP_LAST_DYNAMIC.into(),
            value: summary.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),
            value: summary.into(),
        },
    ];
    Ok(draft)
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

impl Action for ChooseOutwardPosture {
    fn name(&self) -> &'static str {
        "choose_outward_posture"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        posture_draft(state, "outward")
    }
}

impl Action for ChooseRootedPosture {
    fn name(&self) -> &'static str {
        "choose_rooted_posture"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        posture_draft(state, "rooted")
    }
}

fn posture_draft(state: &WorldState, posture: &str) -> Result<EventDraft, ActionError> {
    let seed = seed_id_from_state(state)?;
    if seed == UNSEEDED {
        return Err(ActionError::Invalid(
            "choose a Pocket Universe seed before choosing its next direction".into(),
        ));
    }
    let generation = integer_component(state, UNIVERSE, GENERATION)?;
    if generation < 6 {
        return Err(ActionError::Invalid(
            "this Pocket Universe has not reached its second chapter yet".into(),
        ));
    }
    if decision_id_from_state(state)? == "none" {
        return Err(ActionError::Invalid(
            "the first intervention must settle before choosing a second direction".into(),
        ));
    }
    if text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC)? == "forming" {
        return Err(ActionError::Invalid(
            "the central relationship must resolve before choosing a second direction".into(),
        ));
    }
    if posture_id_from_state(state)? != "none" {
        return Err(ActionError::Invalid(
            "this Pocket Universe already has a second-chapter direction".into(),
        ));
    }

    let summary = match (seed.as_str(), posture) {
        ("mars-colony", "outward") => {
            "The colony opens Kestrel's ridge routes into a wider exploration network."
        }
        ("mars-colony", "rooted") => {
            "The colony turns its next chapter toward making Ares Habitat deeper, safer, and more self-sufficient."
        }
        ("1980s-town", "outward") => {
            "Maple Street lets the arcade, radio, and night bus pull new people into its orbit."
        }
        ("1980s-town", "rooted") => {
            "Maple Street turns its next chapter toward the local places and rituals that already feel like home."
        }
        ("penguin-civilization", "outward") => {
            "Icebridge invites the outer colonies into a wider network under the aurora."
        }
        ("penguin-civilization", "rooted") => {
            "Icebridge turns its next chapter toward winter systems meant to keep local life resilient."
        }
        (_, "outward") => "The World chooses to carry its next chapter outward.",
        (_, "rooted") => "The World chooses to deepen the home it has already made.",
        (_, other) => {
            return Err(ActionError::Invalid(format!(
                "unknown Pocket Universe posture: {other}"
            )))
        }
    };

    let mut draft = EventDraft::new("world_posture_chosen");
    draft.targets = vec![UNIVERSE, RELATIONSHIP, SLOT_B, SLOT_E];
    draft.payload.insert("posture".into(), posture.into());
    draft.payload.insert("summary".into(), summary.into());
    draft.changes = vec![
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: POSTURE.into(),
            value: posture.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: POSTURE_GENERATION.into(),
            value: generation.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),
            value: summary.into(),
        },
    ];
    Ok(draft)
}

fn relationship_entity(name: &str) -> Entity {
    Entity::new(RELATIONSHIP, "relationship")
        .with_component("name", name)
        .with_component("primary", Value::Entity(SLOT_B))
        .with_component("secondary", Value::Entity(SLOT_E))
        .with_component(RELATIONSHIP_TRUST, 0_i64)
        .with_component(RELATIONSHIP_TENSION, 0_i64)
        .with_component(RELATIONSHIP_DIRECTION, "none")
        .with_component(RELATIONSHIP_SOCIAL_ARC, "forming")
        .with_component(RELATIONSHIP_LAST_DYNAMIC, "forming")
}

fn seed_draft(
    state: &WorldState,
    seed: &str,
    universe_name: &str,
    entities: [Entity; 6],
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
            key: POSTURE.into(),
            value: "none".into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: POSTURE_GENERATION.into(),
            value: 0_i64.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LEGACY.into(),
            value: "forming".into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LEGACY_SUMMARY.into(),
            value: "".into(),
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

fn validate_mind_profile(profile: String) -> Result<String, std::io::Error> {
    if is_valid_mind_profile(&profile) {
        Ok(profile)
    } else {
        Err(std::io::Error::other(
            "mind profile must be one of: deterministic, pi, custom",
        ))
    }
}

fn is_valid_mind_profile(profile: &str) -> bool {
    matches!(
        profile,
        DETERMINISTIC_MIND_PROFILE | "pi" | CUSTOM_MIND_PROFILE
    )
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

fn posture_id_from_state(state: &WorldState) -> Result<String, ActionError> {
    match state
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(POSTURE))
    {
        Some(Value::Text(posture)) => Ok(posture.clone()),
        _ => Err(ActionError::Invalid(
            "Pocket Universe posture state is missing".into(),
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

fn text_component_from_state(
    state: &WorldState,
    entity: EntityId,
    key: &str,
) -> Result<String, ActionError> {
    match state
        .entity(entity)
        .and_then(|entity| entity.component(key))
    {
        Some(Value::Text(value)) => Ok(value.clone()),
        _ => Err(ActionError::Invalid(format!(
            "entity {entity} has no text component {key}"
        ))),
    }
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

fn growth_message(
    seed: &str,
    generation: i64,
    decision: &str,
    social_arc: &str,
    posture: &str,
    legacy: &str,
) -> String {
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
    let mut story = base.to_owned();
    if decision != "none" {
        let consequence = match decision {
            "follow-signal" => {
                "The signal expedition keeps pulling attention beyond the safe ridge."
            }
            "fortify-habitat" => {
                "The stronger habitat makes every later risk feel more deliberate."
            }
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
        story.push(' ');
        story.push_str(consequence);
    }
    let social_consequence = match (seed, social_arc) {
        (_, "forming") => None,
        ("mars-colony", "partnership") => {
            Some("Nia and Tomas now plan each rover cycle as one crew.")
        }
        ("mars-colony", "fracture") => {
            Some("Nia and Tomas now divide rover access into competing routes.")
        }
        ("1980s-town", "partnership") => {
            Some("Lena and Max now turn late-night discoveries into one shared broadcast.")
        }
        ("1980s-town", "fracture") => {
            Some("Lena and Max now compete to define the neighborhood's late-night rhythm.")
        }
        ("penguin-civilization", "partnership") => {
            Some("Piko and Miri now bring one shared watch report to the council.")
        }
        ("penguin-civilization", "fracture") => {
            Some("Piko and Miri now bring rival priorities to each moonrise council.")
        }
        (_, _) => Some("The relationship between the world's actors is now shaping later events."),
    };
    if let Some(social_consequence) = social_consequence {
        story.push(' ');
        story.push_str(social_consequence);
    }
    let posture_consequence = match (seed, posture) {
        (_, "none") => None,
        ("mars-colony", "outward") => Some(
            "The outward posture keeps pushing attention and infrastructure beyond the known ridge.",
        ),
        ("mars-colony", "rooted") => Some(
            "The rooted posture keeps pulling effort back toward a stronger home base.",
        ),
        ("1980s-town", "outward") => Some(
            "The outward posture keeps bringing unfamiliar faces into Maple Street's late-night life.",
        ),
        ("1980s-town", "rooted") => Some(
            "The rooted posture keeps turning familiar places into deeper neighborhood institutions.",
        ),
        ("penguin-civilization", "outward") => Some(
            "The outward posture keeps widening Icebridge's circle under the aurora.",
        ),
        ("penguin-civilization", "rooted") => Some(
            "The rooted posture keeps investing in winter systems that make home resilient.",
        ),
        (_, "outward") => Some("The outward posture keeps carrying the World toward new edges."),
        (_, "rooted") => Some("The rooted posture keeps deepening the World it already has."),
        (_, _) => Some("The World's chosen posture is shaping what happens next."),
    };
    if let Some(posture_consequence) = posture_consequence {
        story.push(' ');
        story.push_str(posture_consequence);
    }
    if let Some(legacy_consequence) = legacy::growth_consequence(seed, legacy) {
        story.push(' ');
        story.push_str(legacy_consequence);
    }
    story
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
            4
        );
        assert_eq!(
            new_events
                .iter()
                .filter(|event| {
                    event.kind == "agent_cared_for_world" || event.kind == "agent_explored_world"
                })
                .count(),
            4
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
        let mut universe = PocketUniverse::with_agent_runtime(MockAgentRuntime::scripted([
            AGENT_EXPLORE_ACTION,
            AGENT_CARE_ACTION,
        ]))
        .unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(1).unwrap();

        let decision = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "agent_decision_recorded" && event.actor == Some(SLOT_B))
            .unwrap();
        let outcome = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "agent_explored_world" && event.actor == Some(SLOT_B))
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

    #[test]
    fn one_period_runs_two_causally_chained_agent_turns() {
        let mut universe = PocketUniverse::with_agent_runtime(MockAgentRuntime::scripted([
            AGENT_EXPLORE_ACTION,
            AGENT_CARE_ACTION,
        ]))
        .unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(1).unwrap();

        let events = universe.world().events();
        let growth = events
            .iter()
            .find(|event| event.kind == "universe_grew")
            .unwrap();
        let primary_decision = events
            .iter()
            .find(|event| event.kind == "agent_decision_recorded" && event.actor == Some(SLOT_B))
            .unwrap();
        let primary_outcome = events
            .iter()
            .find(|event| event.kind == "agent_explored_world" && event.actor == Some(SLOT_B))
            .unwrap();
        let secondary_decision = events
            .iter()
            .find(|event| event.kind == "agent_decision_recorded" && event.actor == Some(SLOT_E))
            .unwrap();
        let secondary_outcome = events
            .iter()
            .find(|event| event.kind == "agent_cared_for_world" && event.actor == Some(SLOT_E))
            .unwrap();

        assert!(primary_decision.caused_by.contains(&growth.id));
        assert!(primary_outcome.caused_by.contains(&growth.id));
        assert!(primary_outcome.caused_by.contains(&primary_decision.id));
        assert!(secondary_decision.caused_by.contains(&primary_outcome.id));
        assert!(secondary_outcome.caused_by.contains(&primary_outcome.id));
        assert!(secondary_outcome.caused_by.contains(&secondary_decision.id));

        let why = universe.projection_snapshot().why;
        let chain = why.get(&secondary_outcome.id).unwrap();
        assert!(chain
            .nodes
            .iter()
            .any(|node| node.event == primary_outcome.id));
        assert!(chain.nodes.iter().any(|node| node.event == growth.id));
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
    fn second_agent_failure_rolls_back_growth_and_primary_agent_turn() {
        let mut universe = PocketUniverse::with_agent_runtime(MockAgentRuntime::scripted([
            AGENT_CARE_ACTION,
            "not-an-offered-action",
        ]))
        .unwrap();
        universe
            .invoke_projection_command(SEED_PENGUIN_CIVILIZATION_COMMAND)
            .unwrap();
        let before = universe.archive().unwrap();

        let error = universe.advance_periods(1).unwrap_err();

        assert!(error.to_string().contains("unavailable action"));
        assert_eq!(universe.archive().unwrap(), before);
        assert_eq!(universe.world().world_time(), 0);
    }

    #[test]
    fn mind_profile_is_durable_and_visible_to_snapshot_compare() {
        use world_compare::{compare_snapshots, DifferenceKind};

        let mut left = PocketUniverse::with_agent_runtime_profile(
            MockAgentRuntime::scripted([AGENT_CARE_ACTION, AGENT_CARE_ACTION]),
            DETERMINISTIC_MIND_PROFILE,
        )
        .unwrap();
        let mut right = PocketUniverse::with_agent_runtime_profile(
            MockAgentRuntime::scripted([AGENT_CARE_ACTION, AGENT_CARE_ACTION]),
            "pi",
        )
        .unwrap();
        left.invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        right
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        left.advance_periods(1).unwrap();
        right.advance_periods(1).unwrap();

        let left_outcome = left
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "agent_cared_for_world")
            .unwrap();
        assert_eq!(
            left_outcome.payload.get(MIND_PROFILE_ARG),
            Some(&Value::Text(DETERMINISTIC_MIND_PROFILE.into()))
        );

        let comparison =
            compare_snapshots(&left.projection_snapshot(), &right.projection_snapshot());
        let actor = comparison
            .entities
            .iter()
            .find(|difference| difference.id == world_projection::SelectionId::Entity(SLOT_B))
            .unwrap();
        assert_eq!(actor.kind, DifferenceKind::Changed);
        let profile = actor
            .inspector_rows
            .iter()
            .find(|row| row.key.label == "Last Mind Profile")
            .unwrap();
        assert_eq!(profile.left.as_deref(), Some(DETERMINISTIC_MIND_PROFILE));
        assert_eq!(profile.right.as_deref(), Some("pi"));
    }

    #[test]
    fn registration_profile_rejects_credentials_without_panicking() {
        for profile in [
            "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            "0123456789abcdef0123456789abcdef",
        ] {
            let error =
                pocket_universe_registration_with_agent_runtime_profile(|| PocketMind, profile)
                    .err()
                    .expect("credential-shaped registration profile must be rejected");
            assert!(error.to_string().contains("mind profile must be one of"));
        }
    }

    #[test]
    fn mind_profile_rejects_arbitrary_slug_and_credential_shaped_values() {
        for profile in [
            "mind-a",
            "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            "0123456789abcdef0123456789abcdef",
            "pi api-key=secret",
        ] {
            let error = PocketUniverse::with_agent_runtime_profile(PocketMind, profile)
                .err()
                .expect("non-closed-set mind profile must be rejected");
            assert!(error.to_string().contains("mind profile must be one of"));
        }
    }

    #[test]
    fn deterministic_mind_uses_durable_actor_memory_even_without_time_advancing() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();

        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();

        for actor_id in [SLOT_B, SLOT_E] {
            let actor = universe.world().state().entity(actor_id).unwrap();
            assert_eq!(actor.component(AGENT_CARE_COUNT), Some(&Value::Integer(1)));
            assert_eq!(
                actor.component(AGENT_EXPLORE_COUNT),
                Some(&Value::Integer(1))
            );
            let decisions = universe
                .world()
                .events()
                .iter()
                .filter(|event| {
                    event.kind == "agent_decision_recorded" && event.actor == Some(actor_id)
                })
                .filter_map(|event| match event.payload.get("selected_action") {
                    Some(Value::Text(action)) => Some(action.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let expected = if actor_id == SLOT_B {
                vec![AGENT_CARE_ACTION, AGENT_EXPLORE_ACTION]
            } else {
                vec![AGENT_EXPLORE_ACTION, AGENT_CARE_ACTION]
            };
            assert_eq!(decisions, expected);
        }
        assert_eq!(universe.world().world_time(), 0);
    }

    #[test]
    fn deterministic_secondary_actor_reacts_to_primary_outcome() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();

        universe.advance_periods(1).unwrap();

        let primary = universe.world().state().entity(SLOT_B).unwrap();
        let secondary = universe.world().state().entity(SLOT_E).unwrap();
        assert_eq!(
            primary.component(AGENT_CARE_COUNT),
            Some(&Value::Integer(1))
        );
        assert_eq!(
            primary.component(AGENT_EXPLORE_COUNT),
            Some(&Value::Integer(0))
        );
        assert_eq!(
            secondary.component(AGENT_CARE_COUNT),
            Some(&Value::Integer(0))
        );
        assert_eq!(
            secondary.component(AGENT_EXPLORE_COUNT),
            Some(&Value::Integer(1))
        );
        assert_eq!(
            secondary.component("last_intent"),
            Some(&Value::Text("explore".into()))
        );
    }

    #[test]
    fn relationship_direction_changes_future_secondary_behavior() {
        let mut shared = PocketUniverse::new().unwrap();
        let mut rivalry = PocketUniverse::new().unwrap();
        for universe in [&mut shared, &mut rivalry] {
            universe
                .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
                .unwrap();
            universe.advance_periods(2).unwrap();
        }

        shared
            .invoke_projection_command(SHARED_PROJECT_COMMAND)
            .unwrap();
        rivalry.invoke_projection_command(RIVALRY_COMMAND).unwrap();
        shared.advance_periods(1).unwrap();
        rivalry.advance_periods(1).unwrap();

        let shared_primary = shared.world().state().entity(SLOT_B).unwrap();
        let shared_secondary = shared.world().state().entity(SLOT_E).unwrap();
        let rivalry_primary = rivalry.world().state().entity(SLOT_B).unwrap();
        let rivalry_secondary = rivalry.world().state().entity(SLOT_E).unwrap();
        assert_eq!(
            shared_primary.component("last_intent"),
            Some(&Value::Text("care".into()))
        );
        assert_eq!(
            rivalry_primary.component("last_intent"),
            Some(&Value::Text("care".into()))
        );
        assert_eq!(
            shared_secondary.component("last_intent"),
            Some(&Value::Text("explore".into()))
        );
        assert_eq!(
            rivalry_secondary.component("last_intent"),
            Some(&Value::Text("care".into()))
        );

        let shared_relationship = shared.world().state().entity(RELATIONSHIP).unwrap();
        let rivalry_relationship = rivalry.world().state().entity(RELATIONSHIP).unwrap();
        assert_eq!(
            shared_relationship.component(RELATIONSHIP_DIRECTION),
            Some(&Value::Text("shared-project".into()))
        );
        assert_eq!(
            rivalry_relationship.component(RELATIONSHIP_DIRECTION),
            Some(&Value::Text("rivalry".into()))
        );
    }

    #[derive(Clone)]
    struct RecordingMind {
        observations: Arc<std::sync::Mutex<Vec<AgentObservation>>>,
    }

    impl AgentRuntime for RecordingMind {
        fn decide(
            &mut self,
            observation: &AgentObservation,
            _actions: &[AvailableAction],
        ) -> Result<AgentDecision, AgentRuntimeError> {
            self.observations.lock().unwrap().push(observation.clone());
            Ok(AgentDecision::choose(AGENT_CARE_ACTION))
        }
    }

    #[test]
    fn every_agent_provider_observes_durable_relationship_context() {
        let observations = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut universe = PocketUniverse::with_agent_runtime(RecordingMind {
            observations: Arc::clone(&observations),
        })
        .unwrap();
        universe
            .invoke_projection_command(SEED_1980S_TOWN_COMMAND)
            .unwrap();
        universe.advance_periods(2).unwrap();
        universe.invoke_projection_command(RIVALRY_COMMAND).unwrap();
        observations.lock().unwrap().clear();

        universe.advance_periods(1).unwrap();

        let captured = observations.lock().unwrap();
        assert_eq!(captured.len(), 2);
        for observation in captured.iter() {
            let relationship = observation
                .entities
                .iter()
                .find(|entity| entity.id == RELATIONSHIP)
                .expect("agent observation must contain the durable relationship entity");
            assert_eq!(
                relationship.component(RELATIONSHIP_DIRECTION),
                Some(&Value::Text("rivalry".into()))
            );
            assert!(matches!(
                relationship.component(RELATIONSHIP_TRUST),
                Some(Value::Integer(_))
            ));
            assert!(matches!(
                relationship.component(RELATIONSHIP_TENSION),
                Some(Value::Integer(_))
            ));
        }
        let secondary = captured
            .iter()
            .find(|observation| observation.actor == SLOT_E)
            .expect("secondary observation");
        assert!(secondary.events.iter().any(|event| {
            event.actor == Some(SLOT_B)
                && matches!(
                    event.kind.as_str(),
                    "agent_cared_for_world" | "agent_explored_world"
                )
        }));
    }

    #[test]
    fn shared_project_cascades_into_a_partnership_that_changes_the_world() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(2).unwrap();
        universe
            .invoke_projection_command(SHARED_PROJECT_COMMAND)
            .unwrap();
        universe.advance_periods(1).unwrap();

        let relationship = universe.world().state().entity(RELATIONSHIP).unwrap();
        assert_eq!(
            relationship.component(RELATIONSHIP_SOCIAL_ARC),
            Some(&Value::Text("partnership".into()))
        );
        assert_eq!(
            universe
                .world()
                .state()
                .entity(SLOT_D)
                .unwrap()
                .component("social_status"),
            Some(&Value::Text("joint expedition crew".into()))
        );
        let partnership = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "partnership_formed")
            .expect("partnership event");
        assert_eq!(partnership.caused_by.len(), 1);
        let relationship_shift = partnership.caused_by[0];
        assert_eq!(
            universe
                .world()
                .events()
                .iter()
                .find(|event| event.id == relationship_shift)
                .map(|event| event.kind.as_str()),
            Some("relationship_shifted")
        );
        let snapshot = universe.projection_snapshot();
        let why = snapshot.why(partnership.id).unwrap();
        let growth = universe
            .world()
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "universe_grew")
            .unwrap()
            .id;
        assert!(why.nodes.iter().any(|node| node.event == growth));

        universe.advance_periods(1).unwrap();
        assert_eq!(
            universe
                .world()
                .state()
                .entity(SLOT_D)
                .unwrap()
                .component("social_status"),
            Some(&Value::Text("joint expedition crew".into())),
            "ordinary later agent turns must not erase a resolved social arc"
        );
        let later_growth = universe
            .world()
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "universe_grew")
            .unwrap();
        assert!(matches!(
            later_growth.payload.get("change"),
            Some(Value::Text(change)) if change.contains("one crew")
        ));
    }

    #[test]
    fn rivalry_cascades_into_a_fracture_that_changes_the_world_and_is_forkable() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(2).unwrap();
        universe.invoke_projection_command(RIVALRY_COMMAND).unwrap();
        universe.advance_periods(2).unwrap();

        let relationship = universe.world().state().entity(RELATIONSHIP).unwrap();
        assert_eq!(
            relationship.component(RELATIONSHIP_SOCIAL_ARC),
            Some(&Value::Text("fracture".into()))
        );
        assert_eq!(
            universe
                .world()
                .state()
                .entity(SLOT_D)
                .unwrap()
                .component("social_status"),
            Some(&Value::Text("split survey routes".into()))
        );
        let fractured = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "relationship_fractured")
            .expect("fracture event")
            .id;

        universe.fork_before_event(fractured).unwrap();
        assert_eq!(
            universe
                .world()
                .state()
                .entity(RELATIONSHIP)
                .unwrap()
                .component(RELATIONSHIP_SOCIAL_ARC),
            Some(&Value::Text("forming".into()))
        );
        assert_ne!(
            universe
                .world()
                .state()
                .entity(SLOT_D)
                .unwrap()
                .component("social_status"),
            Some(&Value::Text("split survey routes".into()))
        );
    }

    #[test]
    fn resolved_social_arc_closes_relationship_steering() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(5).unwrap();

        assert_eq!(
            universe
                .world()
                .state()
                .entity(RELATIONSHIP)
                .unwrap()
                .component(RELATIONSHIP_SOCIAL_ARC),
            Some(&Value::Text("partnership".into()))
        );
        assert_eq!(
            universe
                .world()
                .state()
                .entity(RELATIONSHIP)
                .unwrap()
                .component(RELATIONSHIP_DIRECTION),
            Some(&Value::Text("none".into()))
        );
        let snapshot = universe.projection_snapshot();
        assert!(snapshot.command(SHARED_PROJECT_COMMAND).is_none());
        assert!(snapshot.command(RIVALRY_COMMAND).is_none());

        let before = universe.archive().unwrap();
        let error = universe
            .invoke_projection_command(RIVALRY_COMMAND)
            .expect_err("resolved relationship must reject later steering");
        assert!(error
            .to_string()
            .contains("already resolved into a social arc"));
        assert_eq!(universe.archive().unwrap(), before);
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
                .filter(|item| item.detail.starts_with("Lena"))
                .count(),
            1
        );
        assert_eq!(
            briefing
                .items
                .iter()
                .filter(|item| item.detail.starts_with("Max"))
                .count(),
            1
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
    fn complementary_deterministic_agents_build_trust() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(2).unwrap();

        let relationship = universe.world().state().entity(RELATIONSHIP).unwrap();
        assert_eq!(
            relationship.component(RELATIONSHIP_TRUST),
            Some(&Value::Integer(2))
        );
        assert_eq!(
            relationship.component(RELATIONSHIP_TENSION),
            Some(&Value::Integer(0))
        );
        assert_eq!(
            relationship.component(RELATIONSHIP_DIRECTION),
            Some(&Value::Text("none".into()))
        );
        assert_eq!(
            universe
                .world()
                .events()
                .iter()
                .filter(|event| event.kind == "relationship_shifted")
                .count(),
            2
        );
    }

    #[test]
    fn same_explore_choices_raise_tension_and_keep_full_causal_why() {
        let mut universe = PocketUniverse::with_agent_runtime(MockAgentRuntime::scripted([
            AGENT_EXPLORE_ACTION,
            AGENT_EXPLORE_ACTION,
        ]))
        .unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(1).unwrap();

        let relationship = universe.world().state().entity(RELATIONSHIP).unwrap();
        assert_eq!(
            relationship.component(RELATIONSHIP_TRUST),
            Some(&Value::Integer(0))
        );
        assert_eq!(
            relationship.component(RELATIONSHIP_TENSION),
            Some(&Value::Integer(2))
        );
        let shifted = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "relationship_shifted")
            .unwrap();
        assert_eq!(shifted.caused_by.len(), 2);
        let explored = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "agent_explored_world")
            .unwrap()
            .id;
        let growth = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "universe_grew")
            .unwrap()
            .id;
        let why = universe.projection_snapshot().why;
        let chain = why.get(&shifted.id).unwrap();
        assert!(chain.nodes.iter().any(|node| node.event == explored));
        assert!(chain.nodes.iter().any(|node| node.event == growth));
    }

    #[test]
    fn relationship_direction_is_durable_compareable_and_forkable() {
        use world_compare::{compare_snapshots, DifferenceKind};

        let mut shared = PocketUniverse::new().unwrap();
        let mut rivalry = PocketUniverse::new().unwrap();
        shared
            .invoke_projection_command(SEED_1980S_TOWN_COMMAND)
            .unwrap();
        rivalry
            .invoke_projection_command(SEED_1980S_TOWN_COMMAND)
            .unwrap();
        shared.advance_periods(2).unwrap();
        rivalry.advance_periods(2).unwrap();

        let before_choice = shared.archive().unwrap();
        let shared_snapshot = shared
            .invoke_projection_command(SHARED_PROJECT_COMMAND)
            .map(|_| shared.projection_snapshot())
            .unwrap();
        let rivalry_snapshot = rivalry
            .invoke_projection_command(RIVALRY_COMMAND)
            .map(|_| rivalry.projection_snapshot())
            .unwrap();

        let comparison = compare_snapshots(&shared_snapshot, &rivalry_snapshot);
        let relationship = comparison
            .entities
            .iter()
            .find(|difference| difference.id == world_projection::SelectionId::Entity(RELATIONSHIP))
            .unwrap();
        assert_eq!(relationship.kind, DifferenceKind::Changed);
        assert!(relationship.inspector_rows.iter().any(|row| {
            row.key.label == "Direction"
                && row.left.as_deref() == Some("shared-project")
                && row.right.as_deref() == Some("rivalry")
        }));

        let steer_event = shared
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "relationship_steered")
            .unwrap()
            .id;
        shared.fork_before_event(steer_event).unwrap();
        assert_eq!(shared.archive().unwrap(), before_choice);
        let commands = shared.projection_snapshot().commands;
        assert!(commands
            .iter()
            .any(|command| command.id == SHARED_PROJECT_COMMAND));
        assert!(commands.iter().any(|command| command.id == RIVALRY_COMMAND));
    }

    fn second_arc_world(direction_command: &str) -> PocketUniverse {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(3).unwrap();
        universe
            .invoke_projection_command(BOLD_PATH_COMMAND)
            .unwrap();
        universe
            .invoke_projection_command(direction_command)
            .unwrap();
        universe.advance_periods(3).unwrap();
        universe
    }

    fn last_intent(universe: &PocketUniverse, actor: EntityId) -> String {
        text_component_from_state(universe.world().state(), actor, "last_intent").unwrap()
    }

    #[test]
    fn second_arc_waits_for_the_first_arc_then_exposes_two_durable_directions() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(3).unwrap();
        universe
            .invoke_projection_command(BOLD_PATH_COMMAND)
            .unwrap();
        universe
            .invoke_projection_command(SHARED_PROJECT_COMMAND)
            .unwrap();

        let before_resolution = universe.projection_snapshot();
        assert!(!before_resolution
            .commands
            .iter()
            .any(|command| command.id == OUTWARD_POSTURE_COMMAND));

        universe.advance_periods(3).unwrap();
        let ready = universe.projection_snapshot();
        let command_ids = ready
            .commands
            .iter()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ready.briefing.as_ref().unwrap().title,
            "A second chapter is ready"
        );
        assert!(command_ids.contains(&OUTWARD_POSTURE_COMMAND));
        assert!(command_ids.contains(&ROOTED_POSTURE_COMMAND));
    }

    #[test]
    fn second_arc_posture_and_relationship_direction_compose_agent_behavior() {
        for (direction, outward_expected, rooted_expected) in [
            (
                SHARED_PROJECT_COMMAND,
                ("explore", "care"),
                ("care", "explore"),
            ),
            (RIVALRY_COMMAND, ("explore", "explore"), ("care", "care")),
        ] {
            let base = second_arc_world(direction);
            let archive = base.archive().unwrap();

            let mut outward = PocketUniverse::resume_archive(&archive).unwrap();
            outward
                .invoke_projection_command(OUTWARD_POSTURE_COMMAND)
                .unwrap();
            outward.invoke_projection_command(NUDGE_COMMAND).unwrap();
            assert_eq!(last_intent(&outward, SLOT_B), outward_expected.0);
            assert_eq!(last_intent(&outward, SLOT_E), outward_expected.1);

            let mut rooted = PocketUniverse::resume_archive(&archive).unwrap();
            rooted
                .invoke_projection_command(ROOTED_POSTURE_COMMAND)
                .unwrap();
            rooted.invoke_projection_command(NUDGE_COMMAND).unwrap();
            assert_eq!(last_intent(&rooted, SLOT_B), rooted_expected.0);
            assert_eq!(last_intent(&rooted, SLOT_E), rooted_expected.1);
        }
    }

    #[test]
    fn second_arc_posture_survives_archive_and_keeps_shaping_growth() {
        let mut universe = second_arc_world(SHARED_PROJECT_COMMAND);
        universe
            .invoke_projection_command(OUTWARD_POSTURE_COMMAND)
            .unwrap();
        let chosen = universe.projection_snapshot();
        assert!(chosen.briefing.as_ref().unwrap().items.iter().any(|item| {
            item.title == "World direction · Outward"
                && item.detail.contains("Nia keeps looking outward")
        }));
        assert_eq!(
            posture_id_from_state(universe.world().state()).unwrap(),
            "outward"
        );

        let archive = universe.archive().unwrap();
        let mut reopened = PocketUniverse::resume_archive(&archive).unwrap();
        assert_eq!(reopened.projection_snapshot(), chosen);
        assert_eq!(
            posture_id_from_state(reopened.world().state()).unwrap(),
            "outward"
        );

        let before = reopened.world().events().len();
        reopened.invoke_projection_command(NUDGE_COMMAND).unwrap();
        let growth = reopened.world().events()[before..]
            .iter()
            .find(|event| event.kind == "universe_grew")
            .unwrap();
        let change = match growth.payload.get("change") {
            Some(Value::Text(change)) => change,
            other => panic!("expected growth change text, got {other:?}"),
        };
        assert!(change.contains("outward posture"));
        assert!(reopened
            .projection_snapshot()
            .briefing
            .as_ref()
            .unwrap()
            .items
            .iter()
            .any(|item| item.title == "World direction · Outward"));
    }

    #[test]
    fn intervention_remains_visible_after_the_world_keeps_moving() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_1980S_TOWN_COMMAND)
            .unwrap();
        universe.advance_periods(3).unwrap();
        universe
            .invoke_projection_command(BOLD_PATH_COMMAND)
            .unwrap();

        let chosen = universe.projection_snapshot();
        assert!(chosen.briefing.as_ref().unwrap().items.iter().any(|item| {
            item.title == "Your influence · Community arcade"
                && item.detail.contains("organizes its evenings")
        }));

        universe.advance_periods(1).unwrap();
        let later = universe.projection_snapshot();
        assert!(later.briefing.as_ref().unwrap().items.iter().any(|item| {
            item.title == "Your influence · Community arcade"
                && item.detail.contains("organizes its evenings")
        }));
    }

    #[test]
    fn relationship_direction_stays_visible_and_upgrades_to_a_resolved_arc() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(2).unwrap();
        universe
            .invoke_projection_command(SHARED_PROJECT_COMMAND)
            .unwrap();

        let steered = universe.projection_snapshot();
        assert!(steered.briefing.as_ref().unwrap().items.iter().any(|item| {
            item.title == "Relationship · Shared project"
                && item.detail.contains("Trust 4 · tension 0")
        }));

        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        let resolved = universe.projection_snapshot();
        assert!(resolved
            .briefing
            .as_ref()
            .unwrap()
            .items
            .iter()
            .any(|item| {
                item.title == "Partnership formed" && item.detail.contains("durable partnership")
            }));
        assert!(!resolved
            .briefing
            .as_ref()
            .unwrap()
            .items
            .iter()
            .any(|item| { item.title == "Relationship · Shared project" }));
    }

    #[test]
    fn return_briefing_keeps_the_players_persistent_influence_in_context() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_PENGUIN_CIVILIZATION_COMMAND.into(),
            ))
            .unwrap();
        session.advance_background(3).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(CAREFUL_PATH_COMMAND.into()))
            .unwrap();

        let returned = session.advance_background(1).unwrap();
        let briefing = returned.briefing.as_ref().unwrap();
        assert_eq!(briefing.title, "While you were away");
        assert!(briefing.items.iter().any(|item| {
            item.title == "Your influence · Conserved reserves"
                && item.detail.contains("dark season")
        }));
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
        let briefing = chosen.briefing.as_ref().unwrap();
        assert_eq!(briefing.title, "Their relationship is taking shape");
        assert!(briefing.items.iter().any(|item| {
            item.title == "Your turn · Relationship" && item.detail.contains("leave them alone")
        }));
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
