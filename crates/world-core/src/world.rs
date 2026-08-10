use crate::{
    ActionError, ActionRegistry, ActionRequest, Event, EventId, WorldState, WorldStateError,
};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct World {
    baseline: WorldState,
    state: WorldState,
    events: Vec<Event>,
    next_event_id: u64,
}

#[derive(Debug)]
pub enum WorldError {
    Action(ActionError),
    State(WorldStateError),
    TimeRegression { current: u64, requested: u64 },
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Action(error) => error.fmt(f),
            Self::State(error) => error.fmt(f),
            Self::TimeRegression { current, requested } => {
                write!(f, "world time cannot move backwards: {current} -> {requested}")
            }
        }
    }
}

impl Error for WorldError {}

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
            next_event_id: 1,
        }
    }

    pub fn state(&self) -> &WorldState {
        &self.state
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn advance_to(&mut self, world_time: u64) -> Result<(), WorldError> {
        if world_time < self.state.world_time {
            return Err(WorldError::TimeRegression {
                current: self.state.world_time,
                requested: world_time,
            });
        }
        self.state.world_time = world_time;
        Ok(())
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
            world_time: self.state.world_time,
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
        replayed.advance_to(self.state.world_time)?;
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
        if event.world_time < self.state.world_time {
            return Err(WorldError::TimeRegression {
                current: self.state.world_time,
                requested: event.world_time,
            });
        }
        let mut candidate = self.state.clone();
        candidate.world_time = event.world_time;
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
                _ => return Err(ActionError::Invalid("amount must be a positive integer".into())),
            };

            let read_units = |id: EntityId| -> Result<i64, ActionError> {
                match state.entity(id).and_then(|entity| entity.component("units")) {
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
            .seed_entity(
                Entity::new(EntityId::new(2), "container").with_component("units", 20_i64),
            )
            .unwrap();
        state
    }

    #[test]
    fn action_event_state_replay_is_deterministic() {
        let mut registry = ActionRegistry::new();
        registry.register(TransferUnits).unwrap();

        let mut world = World::new(baseline());
        world.advance_to(42).unwrap();
        let event = world
            .execute(
                &registry,
                &ActionRequest::new("transfer_units")
                    .arg("from", EntityId::new(1))
                    .arg("to", EntityId::new(2))
                    .arg("amount", 30_i64),
            )
            .unwrap()
            .clone();

        assert_eq!(event.kind, "units_transferred");
        assert_eq!(event.world_time, 42);
        assert_eq!(world.events().len(), 1);
        assert_eq!(
            world.state().entity(EntityId::new(1)).unwrap().component("units"),
            Some(&Value::Integer(70))
        );
        assert_eq!(
            world.state().entity(EntityId::new(2)).unwrap().component("units"),
            Some(&Value::Integer(50))
        );

        let replayed = world.replay().unwrap();
        assert_eq!(replayed.state(), world.state());
        assert_eq!(replayed.events(), world.events());
    }

    #[test]
    fn invalid_action_does_not_mutate_or_append_event() {
        let mut registry = ActionRegistry::new();
        registry.register(TransferUnits).unwrap();

        let original = baseline();
        let mut world = World::new(original.clone());
        let result = world.execute(
            &registry,
            &ActionRequest::new("transfer_units")
                .arg("from", EntityId::new(1))
                .arg("to", EntityId::new(2))
                .arg("amount", 1_000_i64),
        );

        assert!(matches!(result, Err(WorldError::Action(ActionError::Invalid(_)))));
        assert_eq!(world.state(), &original);
        assert!(world.events().is_empty());
    }

    #[test]
    fn causal_reference_is_preserved() {
        let mut registry = ActionRegistry::new();
        registry.register(TransferUnits).unwrap();
        let mut world = World::new(baseline());

        let first = world
            .execute(
                &registry,
                &ActionRequest::new("transfer_units")
                    .arg("from", EntityId::new(1))
                    .arg("to", EntityId::new(2))
                    .arg("amount", 10_i64),
            )
            .unwrap()
            .id;

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
        let mut registry = ActionRegistry::new();
        registry.register(BrokenMutation).unwrap();
        let original = baseline();
        let mut world = World::new(original.clone());

        let result = world.execute(&registry, &ActionRequest::new("broken_mutation"));

        assert!(matches!(result, Err(WorldError::State(WorldStateError::EntityNotFound(_)))));
        assert_eq!(world.state(), &original);
        assert!(world.events().is_empty());
    }

    #[test]
    fn fork_replays_only_the_selected_prefix() {
        let mut registry = ActionRegistry::new();
        registry.register(TransferUnits).unwrap();
        let mut world = World::new(baseline());

        world
            .execute(
                &registry,
                &ActionRequest::new("transfer_units")
                    .arg("from", EntityId::new(1))
                    .arg("to", EntityId::new(2))
                    .arg("amount", 10_i64),
            )
            .unwrap();
        world
            .execute(
                &registry,
                &ActionRequest::new("transfer_units")
                    .arg("from", EntityId::new(1))
                    .arg("to", EntityId::new(2))
                    .arg("amount", 15_i64),
            )
            .unwrap();

        let fork = world.fork_after(1).unwrap();
        assert_eq!(fork.events().len(), 1);
        assert_eq!(
            fork.state().entity(EntityId::new(1)).unwrap().component("units"),
            Some(&Value::Integer(90))
        );
        assert_eq!(
            fork.state().entity(EntityId::new(2)).unwrap().component("units"),
            Some(&Value::Integer(30))
        );
    }

}
