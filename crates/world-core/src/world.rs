use crate::{
    ActionError, ActionRegistry, ActionRequest, Event, EventId, ScheduleId, Scheduler, WorldState,
    WorldStateError,
};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct World {
    baseline: WorldState,
    state: WorldState,
    events: Vec<Event>,
    scheduler: Scheduler,
    next_event_id: u64,
}

#[derive(Debug)]
pub enum WorldError {
    Action(ActionError),
    State(WorldStateError),
    TimeRegression {
        current: u64,
        requested: u64,
    },
    ScheduleInPast {
        current: u64,
        requested: u64,
    },
    ScheduledActionFailed {
        schedule_id: ScheduleId,
        source: Box<WorldError>,
    },
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Action(error) => error.fmt(f),
            Self::State(error) => error.fmt(f),
            Self::TimeRegression { current, requested } => {
                write!(
                    f,
                    "world time cannot move backwards: {current} -> {requested}"
                )
            }
            Self::ScheduleInPast { current, requested } => {
                write!(
                    f,
                    "cannot schedule action in the past: {requested} < {current}"
                )
            }
            Self::ScheduledActionFailed {
                schedule_id,
                source,
            } => write!(f, "scheduled action {schedule_id} failed: {source}"),
        }
    }
}

impl Error for WorldError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Action(error) => Some(error),
            Self::State(error) => Some(error),
            Self::ScheduledActionFailed { source, .. } => Some(source.as_ref()),
            Self::TimeRegression { .. } | Self::ScheduleInPast { .. } => None,
        }
    }
}

impl From<ActionError> for WorldError {
    fn from(value: ActionError) -> Self {
        Self::Action(value)
    }
}

impl From<WorldStateError> for WorldError {
    fn from(value: WorldStateError) -> Self {
        Self::State(value)
    }
}

impl World {
    pub fn new(initial_state: WorldState) -> Self {
        Self {
            baseline: initial_state.clone(),
            state: initial_state,
            events: Vec::new(),
            scheduler: Scheduler::new(),
            next_event_id: 1,
        }
    }

    pub fn state(&self) -> &WorldState {
        &self.state
    }

    pub fn world_time(&self) -> u64 {
        self.state.world_time()
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    pub fn schedule_at(
        &mut self,
        world_time: u64,
        request: ActionRequest,
    ) -> Result<ScheduleId, WorldError> {
        if world_time < self.world_time() {
            return Err(WorldError::ScheduleInPast {
                current: self.world_time(),
                requested: world_time,
            });
        }
        Ok(self.scheduler.schedule_at(world_time, request))
    }

    pub fn advance_to(
        &mut self,
        registry: &ActionRegistry,
        world_time: u64,
    ) -> Result<Vec<EventId>, WorldError> {
        if world_time < self.world_time() {
            return Err(WorldError::TimeRegression {
                current: self.world_time(),
                requested: world_time,
            });
        }

        let mut executed = Vec::new();
        while let Some(scheduled) = self.scheduler.next_due(world_time) {
            let previous_time = self.world_time();
            self.state.set_world_time(scheduled.world_time);

            match self.execute(registry, &scheduled.request) {
                Ok(event) => {
                    let event_id = event.id;
                    let completed = self.scheduler.complete(scheduled.id);
                    debug_assert!(completed, "due scheduled action must still be queued");
                    executed.push(event_id);
                }
                Err(error) => {
                    self.state.set_world_time(previous_time);
                    return Err(WorldError::ScheduledActionFailed {
                        schedule_id: scheduled.id,
                        source: Box::new(error),
                    });
                }
            }
        }

        self.state.set_world_time(world_time);
        Ok(executed)
    }

    pub fn execute(
        &mut self,
        registry: &ActionRegistry,
        request: &ActionRequest,
    ) -> Result<&Event, WorldError> {
        let mut draft = registry.evaluate(&self.state, request)?;
        if draft.actor.is_none() {
            draft.actor = request.actor;
        }
        if draft.caused_by.is_empty() {
            draft.caused_by = request.caused_by.clone();
        }

        let event = Event {
            id: EventId::new(self.next_event_id),
            kind: draft.kind,
            world_time: self.world_time(),
            actor: draft.actor,
            targets: draft.targets,
            caused_by: draft.caused_by,
            payload: draft.payload,
            changes: draft.changes,
        };

        self.apply_event(&event)?;
        self.next_event_id += 1;
        self.events.push(event);
        Ok(self.events.last().expect("event was just appended"))
    }

    pub fn replay(&self) -> Result<Self, WorldError> {
        let mut replayed = Self::from_history(self.baseline.clone(), &self.events)?;
        replayed.state.set_world_time(self.world_time());
        replayed.scheduler = self.scheduler.clone();
        Ok(replayed)
    }

    pub fn fork_after(&self, event_count: usize) -> Result<Self, WorldError> {
        let end = event_count.min(self.events.len());
        Self::from_history(self.baseline.clone(), &self.events[..end])
    }

    pub fn from_history(baseline: WorldState, events: &[Event]) -> Result<Self, WorldError> {
        let mut world = Self::new(baseline);
        for event in events {
            world.apply_event(event)?;
            world.events.push(event.clone());
            world.next_event_id = world.next_event_id.max(event.id.0 + 1);
        }
        Ok(world)
    }

    fn apply_event(&mut self, event: &Event) -> Result<(), WorldError> {
        if event.world_time < self.world_time() {
            return Err(WorldError::TimeRegression {
                current: self.world_time(),
                requested: event.world_time,
            });
        }
        let mut candidate = self.state.clone();
        candidate.set_world_time(event.world_time);
        for change in &event.changes {
            candidate.apply_change(change)?;
        }
        self.state = candidate;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, ActionError, Entity, EntityId, EventDraft, StateChange, Value};

    struct TransferUnits;

    impl Action for TransferUnits {
        fn name(&self) -> &'static str {
            "transfer_units"
        }

        fn evaluate(
            &self,
            state: &WorldState,
            request: &ActionRequest,
        ) -> Result<EventDraft, ActionError> {
            let from = match request.args.get("from") {
                Some(Value::Entity(id)) => *id,
                _ => return Err(ActionError::Invalid("missing entity arg: from".into())),
            };
            let to = match request.args.get("to") {
                Some(Value::Entity(id)) => *id,
                _ => return Err(ActionError::Invalid("missing entity arg: to".into())),
            };
            let amount = match request.args.get("amount") {
                Some(Value::Integer(amount)) if *amount > 0 => *amount,
                _ => {
                    return Err(ActionError::Invalid(
                        "amount must be a positive integer".into(),
                    ))
                }
            };

            let read_units = |id: EntityId| -> Result<i64, ActionError> {
                match state
                    .entity(id)
                    .and_then(|entity| entity.component("units"))
                {
                    Some(Value::Integer(value)) => Ok(*value),
                    _ => Err(ActionError::Invalid(format!(
                        "entity {id} has no integer units component"
                    ))),
                }
            };

            let from_units = read_units(from)?;
            let to_units = read_units(to)?;
            if from_units < amount {
                return Err(ActionError::Invalid("insufficient units".into()));
            }

            let mut draft = EventDraft::new("units_transferred");
            draft.targets = vec![from, to];
            draft.payload.insert("amount".into(), amount.into());
            draft.changes = vec![
                StateChange::SetComponent {
                    entity: from,
                    key: "units".into(),
                    value: (from_units - amount).into(),
                },
                StateChange::SetComponent {
                    entity: to,
                    key: "units".into(),
                    value: (to_units + amount).into(),
                },
            ];
            Ok(draft)
        }
    }

    fn baseline() -> WorldState {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId::new(1), "container").with_component("units", 100_i64),
            )
            .unwrap();
        state
            .seed_entity(Entity::new(EntityId::new(2), "container").with_component("units", 20_i64))
            .unwrap();
        state
    }

    fn registry() -> ActionRegistry {
        let mut registry = ActionRegistry::new();
        registry.register(TransferUnits).unwrap();
        registry
    }

    fn transfer(amount: i64) -> ActionRequest {
        ActionRequest::new("transfer_units")
            .arg("from", EntityId::new(1))
            .arg("to", EntityId::new(2))
            .arg("amount", amount)
    }

    #[test]
    fn action_event_state_replay_is_deterministic() {
        let registry = registry();
        let mut world = World::new(baseline());
        world.advance_to(&registry, 42).unwrap();
        let event = world.execute(&registry, &transfer(30)).unwrap().clone();

        assert_eq!(event.kind, "units_transferred");
        assert_eq!(event.world_time, 42);
        assert_eq!(world.events().len(), 1);
        assert_eq!(
            world
                .state()
                .entity(EntityId::new(1))
                .unwrap()
                .component("units"),
            Some(&Value::Integer(70))
        );
        assert_eq!(
            world
                .state()
                .entity(EntityId::new(2))
                .unwrap()
                .component("units"),
            Some(&Value::Integer(50))
        );

        let replayed = world.replay().unwrap();
        assert_eq!(replayed.state(), world.state());
        assert_eq!(replayed.events(), world.events());
        assert_eq!(replayed.scheduler(), world.scheduler());
    }

    #[test]
    fn invalid_action_does_not_mutate_or_append_event() {
        let registry = registry();
        let original = baseline();
        let mut world = World::new(original.clone());
        let result = world.execute(&registry, &transfer(1_000));

        assert!(matches!(
            result,
            Err(WorldError::Action(ActionError::Invalid(_)))
        ));
        assert_eq!(world.state(), &original);
        assert!(world.events().is_empty());
    }

    #[test]
    fn causal_reference_is_preserved() {
        let registry = registry();
        let mut world = World::new(baseline());

        let first = world.execute(&registry, &transfer(10)).unwrap().id;
        let second = world
            .execute(
                &registry,
                &ActionRequest::new("transfer_units")
                    .caused_by(first)
                    .arg("from", EntityId::new(2))
                    .arg("to", EntityId::new(1))
                    .arg("amount", 5_i64),
            )
            .unwrap();

        assert_eq!(second.caused_by, vec![first]);
    }

    struct BrokenMutation;

    impl Action for BrokenMutation {
        fn name(&self) -> &'static str {
            "broken_mutation"
        }

        fn evaluate(
            &self,
            _state: &WorldState,
            _request: &ActionRequest,
        ) -> Result<EventDraft, ActionError> {
            let mut draft = EventDraft::new("broken");
            draft.changes = vec![
                StateChange::SetComponent {
                    entity: EntityId::new(1),
                    key: "units".into(),
                    value: 0_i64.into(),
                },
                StateChange::SetComponent {
                    entity: EntityId::new(999),
                    key: "units".into(),
                    value: 1_i64.into(),
                },
            ];
            Ok(draft)
        }
    }

    #[test]
    fn event_application_is_atomic() {
        let mut registry = registry();
        registry.register(BrokenMutation).unwrap();
        let original = baseline();
        let mut world = World::new(original.clone());

        let result = world.execute(&registry, &ActionRequest::new("broken_mutation"));

        assert!(matches!(
            result,
            Err(WorldError::State(WorldStateError::EntityNotFound(_)))
        ));
        assert_eq!(world.state(), &original);
        assert!(world.events().is_empty());
    }

    #[test]
    fn fork_replays_only_the_selected_prefix() {
        let registry = registry();
        let mut world = World::new(baseline());

        world.execute(&registry, &transfer(10)).unwrap();
        world.execute(&registry, &transfer(15)).unwrap();

        let fork = world.fork_after(1).unwrap();
        assert_eq!(fork.events().len(), 1);
        assert_eq!(
            fork.state()
                .entity(EntityId::new(1))
                .unwrap()
                .component("units"),
            Some(&Value::Integer(90))
        );
        assert_eq!(
            fork.state()
                .entity(EntityId::new(2))
                .unwrap()
                .component("units"),
            Some(&Value::Integer(30))
        );
    }

    #[test]
    fn scheduled_actions_execute_in_time_then_insertion_order() {
        let registry = registry();
        let mut world = World::new(baseline());

        let first = world.schedule_at(10, transfer(10)).unwrap();
        let second = world.schedule_at(10, transfer(15)).unwrap();
        let third = world.schedule_at(20, transfer(5)).unwrap();

        assert_eq!(
            world
                .scheduler()
                .pending()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![first, second, third]
        );

        let executed = world.advance_to(&registry, 20).unwrap();
        assert_eq!(executed.len(), 3);
        assert_eq!(
            world
                .events()
                .iter()
                .map(|event| event.world_time)
                .collect::<Vec<_>>(),
            vec![10, 10, 20]
        );
        assert_eq!(
            world
                .events()
                .iter()
                .map(|event| event.payload.get("amount"))
                .collect::<Vec<_>>(),
            vec![
                Some(&Value::Integer(10)),
                Some(&Value::Integer(15)),
                Some(&Value::Integer(5)),
            ]
        );
        assert_eq!(world.world_time(), 20);
        assert_eq!(world.scheduler().pending().count(), 0);
    }

    #[test]
    fn failed_scheduled_action_stays_queued_and_does_not_corrupt_state() {
        let registry = registry();
        let mut world = World::new(baseline());
        let successful = world.schedule_at(10, transfer(10)).unwrap();
        let failing = world.schedule_at(20, transfer(1_000)).unwrap();
        let later = world.schedule_at(30, transfer(5)).unwrap();

        let result = world.advance_to(&registry, 30);

        assert!(matches!(
            result,
            Err(WorldError::ScheduledActionFailed { schedule_id, .. }) if schedule_id == failing
        ));
        assert_eq!(world.events().len(), 1);
        assert_eq!(world.events()[0].world_time, 10);
        assert_eq!(world.world_time(), 10);
        assert!(world.scheduler().get(successful).is_none());
        assert!(world.scheduler().get(failing).is_some());
        assert!(world.scheduler().get(later).is_some());
        assert_eq!(
            world
                .state()
                .entity(EntityId::new(1))
                .unwrap()
                .component("units"),
            Some(&Value::Integer(90))
        );
        assert_eq!(
            world
                .state()
                .entity(EntityId::new(2))
                .unwrap()
                .component("units"),
            Some(&Value::Integer(30))
        );
    }

    #[test]
    fn scheduling_in_the_past_is_rejected_without_queue_mutation() {
        let registry = registry();
        let mut world = World::new(baseline());
        world.advance_to(&registry, 10).unwrap();

        let result = world.schedule_at(9, transfer(1));

        assert!(matches!(result, Err(WorldError::ScheduleInPast { .. })));
        assert_eq!(world.scheduler().pending().count(), 0);
        assert_eq!(world.world_time(), 10);
    }

    #[test]
    fn replay_does_not_reexecute_scheduler_decisions() {
        let registry = registry();
        let mut world = World::new(baseline());
        world.schedule_at(10, transfer(10)).unwrap();
        world.advance_to(&registry, 10).unwrap();
        let pending = world.schedule_at(20, transfer(5)).unwrap();

        let replayed = world.replay().unwrap();

        assert_eq!(replayed.events(), world.events());
        assert_eq!(replayed.state(), world.state());
        assert!(replayed.scheduler().get(pending).is_some());
        assert_eq!(replayed.scheduler().pending().count(), 1);
    }
}
