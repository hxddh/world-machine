use crate::*;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, EventDraft, StateChange,
    Value, World, WorldState,
};

const ACTOR: EntityId = EntityId::new(1);
const VISIBLE: EntityId = EntityId::new(2);
const SECRET: EntityId = EntityId::new(3);

struct SetFlag;

impl Action for SetFlag {
    fn name(&self) -> &'static str {
        "set_flag"
    }

    fn evaluate(
        &self,
        _state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let target = match request.args.get("target") {
            Some(Value::Entity(id)) => *id,
            _ => return Err(ActionError::Invalid("missing target".into())),
        };
        let mut draft = EventDraft::new("flag_set");
        draft.targets = vec![target];
        draft.changes.push(StateChange::SetComponent {
            entity: target,
            key: "flag".into(),
            value: true.into(),
        });
        Ok(draft)
    }
}

fn fixture() -> (World, ActionRegistry) {
    let mut state = WorldState::default();
    state.seed_entity(Entity::new(ACTOR, "agent")).unwrap();
    state.seed_entity(Entity::new(VISIBLE, "visible")).unwrap();
    state
        .seed_entity(Entity::new(SECRET, "secret").with_component("secret", "hidden"))
        .unwrap();

    let mut registry = ActionRegistry::new();
    register_actions(&mut registry).unwrap();
    registry.register(SetFlag).unwrap();
    (World::new(state), registry)
}

#[test]
fn self_only_perception_does_not_expose_global_state() {
    let (world, _) = fixture();
    let observation = ScopedPerception::self_only()
        .observe(&world, ACTOR)
        .unwrap();

    assert_eq!(observation.entities.len(), 1);
    assert_eq!(observation.entities[0].id, ACTOR);
    assert!(observation
        .entities
        .iter()
        .all(|entity| entity.id != SECRET));
}

#[test]
fn scoped_perception_only_exposes_selected_entities() {
    let (world, _) = fixture();
    let observation = ScopedPerception::new([VISIBLE])
        .observe(&world, ACTOR)
        .unwrap();
    let ids: Vec<_> = observation
        .entities
        .iter()
        .map(|entity| entity.id)
        .collect();

    assert_eq!(ids, vec![ACTOR, VISIBLE]);
    assert!(!ids.contains(&SECRET));
}

#[test]
fn agent_decision_is_recorded_and_replay_does_not_call_runtime() {
    let (mut world, registry) = fixture();
    let mut runtime = MockAgentRuntime::scripted(["set_flag"]);
    let actions = [AvailableAction::new(
        "Set a visible flag",
        ActionRequest::new("set_flag").arg("target", VISIBLE),
    )];

    let execution = AgentExecutor::decide_and_execute(
        &mut runtime,
        &ScopedPerception::new([VISIBLE]),
        &mut world,
        &registry,
        ACTOR,
        &actions,
        &[],
    )
    .unwrap();

    assert_eq!(runtime.call_count(), 1);
    assert_eq!(
        world.event(execution.decision_event).unwrap().kind,
        "agent_decision_recorded"
    );
    assert!(world
        .event(execution.outcome_event)
        .unwrap()
        .caused_by
        .contains(&execution.decision_event));
    assert_eq!(
        world.state().entity(VISIBLE).unwrap().component("flag"),
        Some(&Value::Bool(true))
    );

    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
    assert_eq!(replayed.events(), world.events());
    assert_eq!(runtime.call_count(), 1);
}

#[test]
fn unavailable_agent_action_is_rejected_without_world_mutation() {
    let (mut world, registry) = fixture();
    let mut runtime = MockAgentRuntime::scripted(["not_offered"]);
    let actions = [AvailableAction::new(
        "Set a visible flag",
        ActionRequest::new("set_flag").arg("target", VISIBLE),
    )];

    let result = AgentExecutor::decide_and_execute(
        &mut runtime,
        &ScopedPerception::self_only(),
        &mut world,
        &registry,
        ACTOR,
        &actions,
        &[],
    );

    assert!(matches!(
        result,
        Err(AgentExecutionError::UnavailableAction(_))
    ));
    assert!(world.events().is_empty());
    assert!(world
        .state()
        .entity(VISIBLE)
        .unwrap()
        .component("flag")
        .is_none());
}
