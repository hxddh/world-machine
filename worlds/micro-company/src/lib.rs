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

pub const MICRO_COMPANY_PACK_ID: &str = "world-machine.micro-company";
pub const MICRO_COMPANY_PACK_VERSION: &str = "0.1.0";
pub const RUN_CYCLE_COMMAND: &str = "micro-company.run-cycle";

pub(crate) const COMPANY: EntityId = EntityId::new(1);
pub(crate) const PRODUCT_LEAD: EntityId = EntityId::new(10);
pub(crate) const GROWTH_LEAD: EntityId = EntityId::new(11);
pub(crate) const PRODUCT: EntityId = EntityId::new(12);
pub(crate) const MARKET: EntityId = EntityId::new(13);
pub(crate) const RELATIONSHIP: EntityId = EntityId::new(14);

pub(crate) const CASH: &str = "cash";
pub(crate) const STATUS: &str = "status";
pub(crate) const CYCLE: &str = "cycle";
pub(crate) const LAST_CHANGE: &str = "last_change";
pub(crate) const QUALITY: &str = "quality";
pub(crate) const CUSTOMERS: &str = "customers";
pub(crate) const TRUST: &str = "trust";
pub(crate) const TENSION: &str = "tension";
const LAST_DYNAMIC: &str = "last_dynamic";
const BUILD_COUNT: &str = "build_count";
const SELL_COUNT: &str = "sell_count";
const LAST_INTENT: &str = "last_intent";
const LAST_MIND_PROFILE: &str = "last_mind_profile";
const MIND_PROFILE_ARG: &str = "mind_profile";
const DETERMINISTIC_MIND_PROFILE: &str = "deterministic";
const CUSTOM_MIND_PROFILE: &str = "custom";
const BUILD_ACTION: &str = "company_agent.build";
const SELL_ACTION: &str = "company_agent.sell";
const BACKGROUND_PERIOD: u64 = 10;

pub fn micro_company_pack_ref() -> WorldPackRef {
    WorldPackRef::new(MICRO_COMPANY_PACK_ID, MICRO_COMPANY_PACK_VERSION)
}

#[derive(Clone, Debug, Default)]
pub struct CompanyMind;

impl AgentRuntime for CompanyMind {
    fn decide(
        &mut self,
        observation: &AgentObservation,
        actions: &[AvailableAction],
    ) -> Result<AgentDecision, AgentRuntimeError> {
        let desired = if observation.actor == PRODUCT_LEAD {
            let product = observation
                .entities
                .iter()
                .find(|entity| entity.id == PRODUCT)
                .ok_or_else(|| AgentRuntimeError::new("Company Mind cannot see the product"))?;
            let market = observation
                .entities
                .iter()
                .find(|entity| entity.id == MARKET)
                .ok_or_else(|| AgentRuntimeError::new("Company Mind cannot see the market"))?;
            let quality = integer_value(product.component(QUALITY), QUALITY)?;
            let customers = integer_value(market.component(CUSTOMERS), CUSTOMERS)?;
            if quality <= customers {
                BUILD_ACTION
            } else {
                SELL_ACTION
            }
        } else if observation.actor == GROWTH_LEAD {
            let product_outcome = observation.events.iter().rev().find(|event| {
                event.actor == Some(PRODUCT_LEAD)
                    && matches!(event.kind.as_str(), "agent_built_product" | "agent_sold_product")
            });
            match product_outcome.map(|event| event.kind.as_str()) {
                Some("agent_built_product") => SELL_ACTION,
                Some("agent_sold_product") => BUILD_ACTION,
                _ => SELL_ACTION,
            }
        } else {
            return Err(AgentRuntimeError::new("Company Mind received an unknown actor"));
        };

        if !actions.iter().any(|action| action.name() == desired) {
            return Err(AgentRuntimeError::new(format!(
                "Company Mind expected offered action {desired}"
            )));
        }
        Ok(AgentDecision::choose(desired))
    }
}

fn integer_value(value: Option<&Value>, key: &str) -> Result<i64, AgentRuntimeError> {
    match value {
        Some(Value::Integer(value)) => Ok(*value),
        _ => Err(AgentRuntimeError::new(format!(
            "Company Mind is missing integer component {key}"
        ))),
    }
}

pub struct MicroCompany<R = CompanyMind>
where
    R: AgentRuntime,
{
    world: World,
    actions: ActionRegistry,
    mind: R,
    mind_profile: String,
}

impl MicroCompany<CompanyMind> {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Self::with_agent_runtime_profile(CompanyMind, DETERMINISTIC_MIND_PROFILE)
    }

    pub fn resume_archive(archive: &WorldArchive) -> Result<Self, Box<dyn Error>> {
        Self::resume_archive_with_agent_runtime_profile(
            archive,
            CompanyMind,
            DETERMINISTIC_MIND_PROFILE,
        )
    }
}

impl<R> MicroCompany<R>
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
        projection::snapshot(&self.world, None)
    }

    pub fn projection_snapshot_since(
        &self,
        since_event_count: Option<usize>,
    ) -> ProjectionSnapshot {
        projection::snapshot(&self.world, since_event_count)
    }

    pub fn invoke_projection_command(&mut self, command_id: &str) -> Result<(), Box<dyn Error>> {
        if command_id != RUN_CYCLE_COMMAND {
            return Err(std::io::Error::other(format!(
                "unknown projection command: {command_id}"
            ))
            .into());
        }
        self.advance_cycles(1)
    }

    pub fn advance_cycles(&mut self, periods: u64) -> Result<(), Box<dyn Error>> {
        let mut candidate = self.world.clone();
        for _ in 0..periods {
            if company_status(candidate.state())? != "searching" {
                break;
            }
            let target = candidate
                .world_time()
                .checked_add(BACKGROUND_PERIOD)
                .ok_or_else(|| std::io::Error::other("Micro Company time overflow"))?;
            candidate.schedule_at(
                target,
                ActionRequest::new("market_cycle").actor(COMPANY),
            )?;
            let executed = candidate.advance_to(&self.actions, target)?;
            let market_event = executed.last().copied().ok_or_else(|| {
                std::io::Error::other("scheduled Micro Company market cycle did not run")
            })?;

            let product_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                PRODUCT_LEAD,
                &[market_event],
            )?;
            let growth_outcome = Self::run_agent_turn_on(
                &mut self.mind,
                &mut candidate,
                &self.actions,
                &self.mind_profile,
                GROWTH_LEAD,
                &[product_outcome],
            )?;
            let relationship = candidate
                .execute(
                    &self.actions,
                    &ActionRequest::new("update_working_relationship")
                        .caused_by(product_outcome)
                        .caused_by(growth_outcome),
                )?
                .id;
            if resolution_candidate(candidate.state())?.is_some() {
                candidate.execute(
                    &self.actions,
                    &ActionRequest::new("resolve_company").caused_by(relationship),
                )?;
            }
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
                "Build the product: spend one unit of cash to improve product quality.",
                ActionRequest::new(BUILD_ACTION).arg(MIND_PROFILE_ARG, mind_profile),
            ),
            AvailableAction::new(
                "Sell the product: win one customer and bring two units of cash back into the company.",
                ActionRequest::new(SELL_ACTION).arg(MIND_PROFILE_ARG, mind_profile),
            ),
        ];
        let execution = AgentExecutor::decide_and_execute(
            mind,
            &ScopedPerception::new([
                COMPANY,
                PRODUCT_LEAD,
                GROWTH_LEAD,
                PRODUCT,
                MARKET,
                RELATIONSHIP,
            ]),
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
        WorldArchive::capture(micro_company_pack_ref(), &self.world)
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
            world: archive.restore(&micro_company_pack_ref(), baseline()?)?,
            actions: build_action_registry()?,
            mind,
            mind_profile: validate_mind_profile(mind_profile.into())?,
        })
    }
}

struct MicroCompanySession<R>
where
    R: AgentRuntime,
{
    company: MicroCompany<R>,
    return_since_event_count: Option<usize>,
}

impl<R> MicroCompanySession<R>
where
    R: AgentRuntime + 'static,
{
    fn fresh(mind: R, mind_profile: &str) -> Result<Box<dyn WorldSession>, HostError> {
        Ok(Box::new(Self {
            company: MicroCompany::with_agent_runtime_profile(mind, mind_profile)
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
            company: MicroCompany::resume_archive_with_agent_runtime_profile(
                archive,
                mind,
                mind_profile,
            )
            .map_err(HostError::session)?,
            return_since_event_count: None,
        }))
    }
}

impl<R> WorldSession for MicroCompanySession<R>
where
    R: AgentRuntime + 'static,
{
    fn pack(&self) -> WorldPackRef {
        micro_company_pack_ref()
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        self.company
            .projection_snapshot_since(self.return_since_event_count)
    }

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
        match intent {
            ProjectionIntent::ForkBeforeEvent(event) => self
                .company
                .fork_before_event(event)
                .map_err(HostError::session)?,
            ProjectionIntent::InvokeCommand(command) => self
                .company
                .invoke_projection_command(&command)
                .map_err(HostError::session)?,
        }
        self.return_since_event_count = None;
        Ok(self.snapshot())
    }

    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
        let before = self.company.world().events().len();
        self.company
            .advance_cycles(periods)
            .map_err(HostError::session)?;
        self.return_since_event_count =
            (self.company.world().events().len() > before).then_some(before);
        Ok(self.snapshot())
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        self.company.archive().map(Some).map_err(HostError::session)
    }
}

pub fn micro_company_descriptor() -> WorldDescriptor {
    WorldDescriptor {
        pack: micro_company_pack_ref(),
        title: "Micro Company".into(),
        description:
            "A tiny persistent company where product, growth, runway, and working relationships evolve together."
                .into(),
    }
}

pub fn micro_company_registration() -> WorldRegistration {
    registration_with_validated_profile(|| CompanyMind, DETERMINISTIC_MIND_PROFILE)
}

pub fn micro_company_registration_with_agent_runtime<R, F>(factory: F) -> WorldRegistration
where
    R: AgentRuntime + 'static,
    F: Fn() -> R + Send + Sync + 'static,
{
    registration_with_validated_profile(factory, CUSTOM_MIND_PROFILE)
}

pub fn micro_company_registration_with_agent_runtime_profile<R, F>(
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
    WorldRegistration::new(micro_company_descriptor(), move || {
        MicroCompanySession::fresh(create_factory(), create_profile.as_str())
    })
    .with_archive_opener(move |archive| {
        MicroCompanySession::open_archive(archive, open_factory(), open_profile.as_str())
    })
}

fn baseline() -> Result<WorldState, WorldStateError> {
    let mut state = WorldState::default();
    state.seed_entity(
        Entity::new(COMPANY, "company")
            .with_component("name", "Northstar Micro Company")
            .with_component(CASH, 6_i64)
            .with_component(STATUS, "searching")
            .with_component(CYCLE, 0_i64)
            .with_component(
                LAST_CHANGE,
                "The company has one product idea, one customer, and six units of runway.",
            ),
    )?;
    state.seed_entity(
        Entity::new(PRODUCT_LEAD, "person")
            .with_component("name", "Maya Chen")
            .with_component("role", "product lead")
            .with_component(BUILD_COUNT, 0_i64)
            .with_component(SELL_COUNT, 0_i64)
            .with_component(LAST_INTENT, "none")
            .with_component(LAST_MIND_PROFILE, "none"),
    )?;
    state.seed_entity(
        Entity::new(GROWTH_LEAD, "person")
            .with_component("name", "Jon Bell")
            .with_component("role", "growth lead")
            .with_component(BUILD_COUNT, 0_i64)
            .with_component(SELL_COUNT, 0_i64)
            .with_component(LAST_INTENT, "none")
            .with_component(LAST_MIND_PROFILE, "none"),
    )?;
    state.seed_entity(
        Entity::new(PRODUCT, "product")
            .with_component("name", "Northstar")
            .with_component(QUALITY, 1_i64),
    )?;
    state.seed_entity(
        Entity::new(MARKET, "market")
            .with_component("name", "First Customers")
            .with_component(CUSTOMERS, 1_i64),
    )?;
    state.seed_entity(
        Entity::new(RELATIONSHIP, "working_relationship")
            .with_component("name", "Maya ↔ Jon")
            .with_component(TRUST, 0_i64)
            .with_component(TENSION, 0_i64)
            .with_component(LAST_DYNAMIC, "forming"),
    )?;
    Ok(state)
}

fn build_action_registry() -> Result<ActionRegistry, ActionError> {
    let mut actions = ActionRegistry::new();
    register_agent_actions(&mut actions)?;
    actions.register(MarketCycle)?;
    actions.register(BuildProduct)?;
    actions.register(SellProduct)?;
    actions.register(UpdateWorkingRelationship)?;
    actions.register(ResolveCompany)?;
    Ok(actions)
}

struct MarketCycle;
struct BuildProduct;
struct SellProduct;
struct UpdateWorkingRelationship;
struct ResolveCompany;

impl Action for MarketCycle {
    fn name(&self) -> &'static str {
        "market_cycle"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if company_status(state)? != "searching" {
            return Err(ActionError::Invalid(
                "resolved company cannot start another market cycle".into(),
            ));
        }
        let next_cycle = integer_component(state, COMPANY, CYCLE)? + 1;
        let next_cash = integer_component(state, COMPANY, CASH)? - 1;
        let summary = format!(
            "Cycle {next_cycle} opened with one unit of runway spent before either lead could act."
        );
        let mut draft = EventDraft::new("market_cycle_started");
        draft.targets = vec![COMPANY, PRODUCT, MARKET];
        draft.payload.insert("cycle".into(), next_cycle.into());
        draft.payload.insert("summary".into(), summary.clone().into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: COMPANY,
                key: CYCLE.into(),
                value: next_cycle.into(),
            },
            StateChange::SetComponent {
                entity: COMPANY,
                key: CASH.into(),
                value: next_cash.into(),
            },
            StateChange::SetComponent {
                entity: COMPANY,
                key: LAST_CHANGE.into(),
                value: summary.into(),
            },
        ];
        Ok(draft)
    }
}

impl Action for BuildProduct {
    fn name(&self) -> &'static str {
        BUILD_ACTION
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        agent_action_draft(state, request, true)
    }
}

impl Action for SellProduct {
    fn name(&self) -> &'static str {
        SELL_ACTION
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        agent_action_draft(state, request, false)
    }
}

fn agent_action_draft(
    state: &WorldState,
    request: &ActionRequest,
    build: bool,
) -> Result<EventDraft, ActionError> {
    let actor = request
        .actor
        .ok_or_else(|| ActionError::Invalid("company agent action requires an actor".into()))?;
    if actor != PRODUCT_LEAD && actor != GROWTH_LEAD {
        return Err(ActionError::Invalid(format!(
            "company agent action requires a company lead, got {actor}"
        )));
    }
    if company_status(state)? != "searching" {
        return Err(ActionError::Invalid(
            "resolved company cannot accept another agent action".into(),
        ));
    }
    let mind_profile = match request.args.get(MIND_PROFILE_ARG) {
        Some(Value::Text(profile)) if is_valid_mind_profile(profile) => profile.clone(),
        _ => {
            return Err(ActionError::Invalid(
                "company agent action requires a valid mind_profile".into(),
            ))
        }
    };
    let count_key = if build { BUILD_COUNT } else { SELL_COUNT };
    let next_count = integer_component(state, actor, count_key)? + 1;
    let actor_name = text_component(state, actor, "name")?;
    let mut draft = EventDraft::new(if build {
        "agent_built_product"
    } else {
        "agent_sold_product"
    });
    draft.targets = if build {
        vec![actor, PRODUCT, COMPANY]
    } else {
        vec![actor, MARKET, COMPANY]
    };
    draft
        .payload
        .insert(MIND_PROFILE_ARG.into(), mind_profile.clone().into());
    draft.payload.insert("turn".into(), next_count.into());

    let mut changes = vec![
        StateChange::SetComponent {
            entity: actor,
            key: count_key.into(),
            value: next_count.into(),
        },
        StateChange::SetComponent {
            entity: actor,
            key: LAST_INTENT.into(),
            value: if build { "build" } else { "sell" }.into(),
        },
        StateChange::SetComponent {
            entity: actor,
            key: LAST_MIND_PROFILE.into(),
            value: mind_profile.into(),
        },
    ];

    if build {
        let quality = integer_component(state, PRODUCT, QUALITY)? + 1;
        let cash = integer_component(state, COMPANY, CASH)? - 1;
        let summary = format!(
            "{actor_name} spent one unit of runway and pushed Northstar to quality {quality}."
        );
        draft.payload.insert("summary".into(), summary.clone().into());
        changes.extend([
            StateChange::SetComponent {
                entity: PRODUCT,
                key: QUALITY.into(),
                value: quality.into(),
            },
            StateChange::SetComponent {
                entity: COMPANY,
                key: CASH.into(),
                value: cash.into(),
            },
            StateChange::SetComponent {
                entity: COMPANY,
                key: LAST_CHANGE.into(),
                value: summary.into(),
            },
        ]);
    } else {
        let customers = integer_component(state, MARKET, CUSTOMERS)? + 1;
        let cash = integer_component(state, COMPANY, CASH)? + 2;
        let summary = format!(
            "{actor_name} won customer {customers} and brought two units of cash back into Northstar."
        );
        draft.payload.insert("summary".into(), summary.clone().into());
        changes.extend([
            StateChange::SetComponent {
                entity: MARKET,
                key: CUSTOMERS.into(),
                value: customers.into(),
            },
            StateChange::SetComponent {
                entity: COMPANY,
                key: CASH.into(),
                value: cash.into(),
            },
            StateChange::SetComponent {
                entity: COMPANY,
                key: LAST_CHANGE.into(),
                value: summary.into(),
            },
        ]);
    }
    draft.changes = changes;
    Ok(draft)
}

impl Action for UpdateWorkingRelationship {
    fn name(&self) -> &'static str {
        "update_working_relationship"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let product_intent = text_component(state, PRODUCT_LEAD, LAST_INTENT)?;
        let growth_intent = text_component(state, GROWTH_LEAD, LAST_INTENT)?;
        let trust = integer_component(state, RELATIONSHIP, TRUST)?;
        let tension = integer_component(state, RELATIONSHIP, TENSION)?;
        let complementary = product_intent != growth_intent;
        let next_trust = if complementary {
            (trust + 1).clamp(0, 10)
        } else {
            trust
        };
        let next_tension = if complementary {
            (tension - 1).clamp(0, 10)
        } else {
            (tension + 2).clamp(0, 10)
        };
        let summary = if complementary {
            "Maya and Jon covered different company risks in the same cycle; their working trust increased."
        } else {
            "Maya and Jon pushed the company in the same direction at once; working tension increased."
        };
        let mut draft = EventDraft::new("working_relationship_shifted");
        draft.targets = vec![RELATIONSHIP, PRODUCT_LEAD, GROWTH_LEAD];
        draft.payload.insert("trust".into(), next_trust.into());
        draft.payload.insert("tension".into(), next_tension.into());
        draft.payload.insert("summary".into(), summary.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: TRUST.into(),
                value: next_trust.into(),
            },
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: TENSION.into(),
                value: next_tension.into(),
            },
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: LAST_DYNAMIC.into(),
                value: summary.into(),
            },
        ];
        Ok(draft)
    }
}

fn resolution_candidate(state: &WorldState) -> Result<Option<&'static str>, ActionError> {
    if company_status(state)? != "searching" {
        return Ok(None);
    }
    let cash = integer_component(state, COMPANY, CASH)?;
    if cash <= 0 {
        return Ok(Some("out-of-cash"));
    }
    let quality = integer_component(state, PRODUCT, QUALITY)?;
    let customers = integer_component(state, MARKET, CUSTOMERS)?;
    if quality >= 3 && customers >= 3 {
        return Ok(Some("traction"));
    }
    Ok(None)
}

impl Action for ResolveCompany {
    fn name(&self) -> &'static str {
        "resolve_company"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let status = resolution_candidate(state)?
            .ok_or_else(|| ActionError::Invalid("company has not reached a resolution".into()))?;
        let cash = integer_component(state, COMPANY, CASH)?;
        let quality = integer_component(state, PRODUCT, QUALITY)?;
        let customers = integer_component(state, MARKET, CUSTOMERS)?;
        let (kind, summary) = match status {
            "traction" => (
                "company_found_traction",
                "Northstar found traction: product quality and customer pull grew together without consuming its runway.",
            ),
            "out-of-cash" => (
                "company_ran_out_of_cash",
                "Northstar ran out of runway before customer pull could catch up with what the team kept building.",
            ),
            _ => return Err(ActionError::Invalid("unknown company resolution".into())),
        };
        let mut draft = EventDraft::new(kind);
        draft.targets = vec![COMPANY, PRODUCT, MARKET, RELATIONSHIP];
        draft.payload.insert("status".into(), status.into());
        draft.payload.insert("cash".into(), cash.into());
        draft.payload.insert("quality".into(), quality.into());
        draft.payload.insert("customers".into(), customers.into());
        draft.payload.insert("summary".into(), summary.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: COMPANY,
                key: STATUS.into(),
                value: status.into(),
            },
            StateChange::SetComponent {
                entity: COMPANY,
                key: LAST_CHANGE.into(),
                value: summary.into(),
            },
        ];
        Ok(draft)
    }
}

fn company_status(state: &WorldState) -> Result<String, ActionError> {
    text_component(state, COMPANY, STATUS)
}

fn text_component(
    state: &WorldState,
    entity: EntityId,
    key: &str,
) -> Result<String, ActionError> {
    match state.entity(entity).and_then(|entity| entity.component(key)) {
        Some(Value::Text(value)) => Ok(value.clone()),
        _ => Err(ActionError::Invalid(format!(
            "entity {entity} has no text component {key}"
        ))),
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

#[cfg(test)]
mod tests {
    use super::*;
    use world_agent::MockAgentRuntime;

    fn text(state: &WorldState, entity: EntityId, key: &str) -> String {
        match state.entity(entity).unwrap().component(key).unwrap() {
            Value::Text(value) => value.clone(),
            other => panic!("expected text {key}, got {other:?}"),
        }
    }

    fn integer(state: &WorldState, entity: EntityId, key: &str) -> i64 {
        match state.entity(entity).unwrap().component(key).unwrap() {
            Value::Integer(value) => *value,
            other => panic!("expected integer {key}, got {other:?}"),
        }
    }

    #[test]
    fn deterministic_company_reaches_traction_after_two_cycles() {
        let mut company = MicroCompany::new().unwrap();
        company.advance_cycles(2).unwrap();

        assert_eq!(company.world().world_time(), 20);
        assert_eq!(text(company.world().state(), COMPANY, STATUS), "traction");
        assert_eq!(integer(company.world().state(), COMPANY, CASH), 6);
        assert_eq!(integer(company.world().state(), PRODUCT, QUALITY), 3);
        assert_eq!(integer(company.world().state(), MARKET, CUSTOMERS), 3);
        assert_eq!(integer(company.world().state(), RELATIONSHIP, TRUST), 2);
        assert_eq!(integer(company.world().state(), RELATIONSHIP, TENSION), 0);

        let resolution = company
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "company_found_traction")
            .expect("traction event");
        let snapshot = company.projection_snapshot();
        let why = snapshot.why(resolution.id).expect("traction Why");
        let cycle_event = company
            .world()
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "market_cycle_started")
            .unwrap()
            .id;
        assert!(why.nodes.iter().any(|node| node.event == cycle_event));
        assert!(snapshot.command(RUN_CYCLE_COMMAND).is_none());
    }

    #[test]
    fn same_build_runtime_burns_runway_and_builds_tension() {
        let mind = MockAgentRuntime::scripted([
            BUILD_ACTION,
            BUILD_ACTION,
            BUILD_ACTION,
            BUILD_ACTION,
        ]);
        let mut company = MicroCompany::with_agent_runtime(mind).unwrap();
        company.advance_cycles(2).unwrap();

        assert_eq!(text(company.world().state(), COMPANY, STATUS), "out-of-cash");
        assert_eq!(integer(company.world().state(), COMPANY, CASH), 0);
        assert_eq!(integer(company.world().state(), PRODUCT, QUALITY), 5);
        assert_eq!(integer(company.world().state(), MARKET, CUSTOMERS), 1);
        assert_eq!(integer(company.world().state(), RELATIONSHIP, TRUST), 0);
        assert_eq!(integer(company.world().state(), RELATIONSHIP, TENSION), 4);
        assert_eq!(
            company
                .world()
                .events()
                .iter()
                .filter(|event| event.kind == "agent_built_product")
                .count(),
            4
        );
        assert!(company
            .world()
            .events()
            .iter()
            .any(|event| event.kind == "company_ran_out_of_cash"));
    }

    #[test]
    fn second_agent_failure_rolls_back_the_entire_company_cycle() {
        let mind = MockAgentRuntime::scripted([BUILD_ACTION]);
        let mut company = MicroCompany::with_agent_runtime(mind).unwrap();
        let before = company.archive().unwrap();
        let error = company
            .advance_cycles(1)
            .expect_err("second decision should fail");
        assert!(error.to_string().contains("no scripted decision"));
        assert_eq!(company.archive().unwrap(), before);
        assert_eq!(company.world().world_time(), 0);
    }

    #[test]
    fn archive_restore_replays_truth_without_calling_the_mind() {
        let mut company = MicroCompany::new().unwrap();
        company.advance_cycles(1).unwrap();
        let archive = company.archive().unwrap();
        let snapshot = company.projection_snapshot();

        let empty = MockAgentRuntime::scripted(Vec::<String>::new());
        let mut restored =
            MicroCompany::resume_archive_with_agent_runtime(&archive, empty).unwrap();
        assert_eq!(restored.projection_snapshot(), snapshot);
        assert_eq!(restored.archive().unwrap(), archive);
        assert!(restored.advance_cycles(1).is_err());
        assert_eq!(restored.archive().unwrap(), archive);
    }

    #[test]
    fn forking_before_company_resolution_reopens_the_company() {
        let mut company = MicroCompany::new().unwrap();
        company.advance_cycles(2).unwrap();
        let resolution = company
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "company_found_traction")
            .unwrap()
            .id;
        company.fork_before_event(resolution).unwrap();
        assert_eq!(text(company.world().state(), COMPANY, STATUS), "searching");
        assert!(company.projection_snapshot().command(RUN_CYCLE_COMMAND).is_some());
    }
}
