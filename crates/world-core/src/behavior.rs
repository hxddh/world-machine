use crate::{ActionRegistry, ActionRequest, Event, EventId, World, WorldError, WorldState};
use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BehaviorKind {
    Rule,
    Native,
}

pub trait Behavior: Send + Sync {
    fn name(&self) -> &str;

    fn kind(&self) -> BehaviorKind;

    fn handles(&self, event: &Event) -> bool;

    fn react(&self, state: &WorldState, event: &Event) -> Vec<ActionRequest>;
}

pub struct RuleBehavior<F> {
    name: String,
    subscriptions: HashSet<String>,
    handler: F,
}

impl<F> RuleBehavior<F> {
    pub fn new<I, S>(name: impl Into<String>, subscriptions: I, handler: F) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            name: name.into(),
            subscriptions: subscriptions.into_iter().map(Into::into).collect(),
            handler,
        }
    }
}

impl<F> Behavior for RuleBehavior<F>
where
    F: Fn(&WorldState, &Event) -> Vec<ActionRequest> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> BehaviorKind {
        BehaviorKind::Rule
    }

    fn handles(&self, event: &Event) -> bool {
        self.subscriptions.contains(&event.kind)
    }

    fn react(&self, state: &WorldState, event: &Event) -> Vec<ActionRequest> {
        (self.handler)(state, event)
    }
}

pub struct NativeBehavior<F> {
    name: String,
    subscriptions: HashSet<String>,
    handler: F,
}

impl<F> NativeBehavior<F> {
    pub fn new<I, S>(name: impl Into<String>, subscriptions: I, handler: F) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            name: name.into(),
            subscriptions: subscriptions.into_iter().map(Into::into).collect(),
            handler,
        }
    }
}

impl<F> Behavior for NativeBehavior<F>
where
    F: Fn(&WorldState, &Event) -> Vec<ActionRequest> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> BehaviorKind {
        BehaviorKind::Native
    }

    fn handles(&self, event: &Event) -> bool {
        self.subscriptions.contains(&event.kind)
    }

    fn react(&self, state: &WorldState, event: &Event) -> Vec<ActionRequest> {
        (self.handler)(state, event)
    }
}

#[derive(Default)]
pub struct BehaviorRegistry {
    behaviors: Vec<Box<dyn Behavior>>,
    names: HashSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BehaviorRegistryError {
    DuplicateName(String),
}

impl fmt::Display for BehaviorRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateName(name) => write!(f, "behavior already registered: {name}"),
        }
    }
}

impl Error for BehaviorRegistryError {}

impl BehaviorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<B>(&mut self, behavior: B) -> Result<(), BehaviorRegistryError>
    where
        B: Behavior + 'static,
    {
        let name = behavior.name().to_owned();
        if !self.names.insert(name.clone()) {
            return Err(BehaviorRegistryError::DuplicateName(name));
        }
        self.behaviors.push(Box::new(behavior));
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn Behavior> {
        self.behaviors.iter().map(Box::as_ref)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BehaviorRunStatus {
    Complete,
    BudgetExhausted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BehaviorRun {
    pub root_event: EventId,
    pub generated_events: Vec<EventId>,
    pub executed_actions: usize,
    pub status: BehaviorRunStatus,
}

#[derive(Debug)]
pub enum BehaviorRuntimeError {
    UnknownEvent(EventId),
    ActionFailed {
        behavior: String,
        trigger_event: EventId,
        source: WorldError,
    },
}

impl fmt::Display for BehaviorRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEvent(event) => write!(f, "unknown trigger event: {event}"),
            Self::ActionFailed {
                behavior,
                trigger_event,
                source,
            } => write!(
                f,
                "behavior {behavior} failed while reacting to event {trigger_event}: {source}"
            ),
        }
    }
}

impl Error for BehaviorRuntimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ActionFailed { source, .. } => Some(source),
            Self::UnknownEvent(_) => None,
        }
    }
}

pub struct BehaviorRuntime;

impl BehaviorRuntime {
    pub fn run_from_event(
        world: &mut World,
        actions: &ActionRegistry,
        behaviors: &BehaviorRegistry,
        root_event: EventId,
        max_actions: usize,
    ) -> Result<BehaviorRun, BehaviorRuntimeError> {
        let root = world
            .event(root_event)
            .cloned()
            .ok_or(BehaviorRuntimeError::UnknownEvent(root_event))?;

        let mut queue = VecDeque::from([root]);
        let mut generated_events = Vec::new();
        let mut executed_actions = 0;

        while let Some(trigger) = queue.pop_front() {
            for behavior in behaviors
                .iter()
                .filter(|behavior| behavior.handles(&trigger))
            {
                for mut request in behavior.react(world.state(), &trigger) {
                    if executed_actions >= max_actions {
                        return Ok(BehaviorRun {
                            root_event,
                            generated_events,
                            executed_actions,
                            status: BehaviorRunStatus::BudgetExhausted,
                        });
                    }

                    if !request.caused_by.contains(&trigger.id) {
                        request.caused_by.push(trigger.id);
                    }

                    let generated = world
                        .execute(actions, &request)
                        .map_err(|source| BehaviorRuntimeError::ActionFailed {
                            behavior: behavior.name().to_owned(),
                            trigger_event: trigger.id,
                            source,
                        })?
                        .clone();

                    executed_actions += 1;
                    generated_events.push(generated.id);
                    queue.push_back(generated);
                }
            }
        }

        Ok(BehaviorRun {
            root_event,
            generated_events,
            executed_actions,
            status: BehaviorRunStatus::Complete,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, ActionError, Entity, EntityId, EventDraft, Value, WorldState};

    struct Record;

    impl Action for Record {
        fn name(&self) -> &'static str {
            "record"
        }

        fn evaluate(
            &self,
            _state: &WorldState,
            request: &ActionRequest,
        ) -> Result<EventDraft, ActionError> {
            let label = match request.args.get("label") {
                Some(Value::Text(label)) => label.clone(),
                _ => return Err(ActionError::Invalid("missing text arg: label".into())),
            };
            let mut draft = EventDraft::new("recorded");
            draft.payload.insert("label".into(), label.into());
            Ok(draft)
        }
    }

    struct Ping;

    impl Action for Ping {
        fn name(&self) -> &'static str {
            "ping"
        }

        fn evaluate(
            &self,
            _state: &WorldState,
            _request: &ActionRequest,
        ) -> Result<EventDraft, ActionError> {
            Ok(EventDraft::new("pinged"))
        }
    }

    fn world() -> World {
        let mut state = WorldState::default();
        state
            .seed_entity(Entity::new(EntityId::new(1), "fixture"))
            .unwrap();
        World::new(state)
    }

    fn actions() -> ActionRegistry {
        let mut registry = ActionRegistry::new();
        registry.register(Record).unwrap();
        registry.register(Ping).unwrap();
        registry
    }

    #[test]
    fn behaviors_run_in_registration_order_and_preserve_action_order() {
        let actions = actions();
        let mut behaviors = BehaviorRegistry::new();
        behaviors
            .register(RuleBehavior::new("rule", ["pinged"], |_state, _event| {
                vec![
                    ActionRequest::new("record").arg("label", "rule-a"),
                    ActionRequest::new("record").arg("label", "rule-b"),
                ]
            }))
            .unwrap();
        behaviors
            .register(NativeBehavior::new(
                "native",
                ["pinged"],
                |_state, _event| vec![ActionRequest::new("record").arg("label", "native")],
            ))
            .unwrap();

        let mut world = world();
        let root = world
            .execute(&actions, &ActionRequest::new("ping"))
            .unwrap()
            .id;
        let run =
            BehaviorRuntime::run_from_event(&mut world, &actions, &behaviors, root, 10).unwrap();

        assert_eq!(run.status, BehaviorRunStatus::Complete);
        assert_eq!(run.executed_actions, 3);
        let labels: Vec<_> = world.events()[1..]
            .iter()
            .map(|event| event.payload.get("label"))
            .collect();
        assert_eq!(
            labels,
            vec![
                Some(&Value::Text("rule-a".into())),
                Some(&Value::Text("rule-b".into())),
                Some(&Value::Text("native".into())),
            ]
        );
        assert!(world.events()[1..]
            .iter()
            .all(|event| event.caused_by == vec![root]));
    }

    #[test]
    fn behavior_chain_is_fifo_and_causally_linked() {
        let actions = actions();
        let mut behaviors = BehaviorRegistry::new();
        behaviors
            .register(RuleBehavior::new("first", ["pinged"], |_state, _event| {
                vec![ActionRequest::new("record").arg("label", "first")]
            }))
            .unwrap();
        behaviors
            .register(NativeBehavior::new(
                "second",
                ["recorded"],
                |_state, event| match event.payload.get("label") {
                    Some(Value::Text(label)) if label == "first" => {
                        vec![ActionRequest::new("record").arg("label", "second")]
                    }
                    _ => Vec::new(),
                },
            ))
            .unwrap();

        let mut world = world();
        let root = world
            .execute(&actions, &ActionRequest::new("ping"))
            .unwrap()
            .id;
        let run =
            BehaviorRuntime::run_from_event(&mut world, &actions, &behaviors, root, 10).unwrap();

        assert_eq!(run.generated_events.len(), 2);
        let first = run.generated_events[0];
        let second = run.generated_events[1];
        assert_eq!(world.event(first).unwrap().caused_by, vec![root]);
        assert_eq!(world.event(second).unwrap().caused_by, vec![first]);
    }

    #[test]
    fn action_budget_stops_an_unbounded_reaction_chain() {
        let actions = actions();
        let mut behaviors = BehaviorRegistry::new();
        behaviors
            .register(RuleBehavior::new("loop", ["pinged"], |_state, _event| {
                vec![ActionRequest::new("ping")]
            }))
            .unwrap();

        let mut world = world();
        let root = world
            .execute(&actions, &ActionRequest::new("ping"))
            .unwrap()
            .id;
        let run =
            BehaviorRuntime::run_from_event(&mut world, &actions, &behaviors, root, 3).unwrap();

        assert_eq!(run.status, BehaviorRunStatus::BudgetExhausted);
        assert_eq!(run.executed_actions, 3);
        assert_eq!(run.generated_events.len(), 3);
        assert_eq!(world.events().len(), 4);
    }

    #[test]
    fn replay_does_not_rerun_behaviors() {
        let actions = actions();
        let mut behaviors = BehaviorRegistry::new();
        behaviors
            .register(RuleBehavior::new("rule", ["pinged"], |_state, _event| {
                vec![ActionRequest::new("record").arg("label", "once")]
            }))
            .unwrap();

        let mut world = world();
        let root = world
            .execute(&actions, &ActionRequest::new("ping"))
            .unwrap()
            .id;
        BehaviorRuntime::run_from_event(&mut world, &actions, &behaviors, root, 10).unwrap();

        let replayed = world.replay().unwrap();
        assert_eq!(replayed.events(), world.events());
        assert_eq!(replayed.events().len(), 2);
    }

    #[test]
    fn duplicate_behavior_names_are_rejected() {
        let mut behaviors = BehaviorRegistry::new();
        behaviors
            .register(RuleBehavior::new("same", ["pinged"], |_state, _event| {
                Vec::new()
            }))
            .unwrap();
        let result =
            behaviors.register(RuleBehavior::new("same", ["recorded"], |_state, _event| {
                Vec::new()
            }));

        assert_eq!(
            result.unwrap_err(),
            BehaviorRegistryError::DuplicateName("same".into())
        );
    }
}
