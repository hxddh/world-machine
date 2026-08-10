use std::collections::BTreeMap;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, EventDraft, StateChange,
    Value, World, WorldState,
};

struct MoveUnits;

impl Action for MoveUnits {
    fn name(&self) -> &'static str {
        "move_units"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let entity_arg = |name: &str| match request.args.get(name) {
            Some(Value::Entity(id)) => Ok(*id),
            _ => Err(ActionError::Invalid(format!("missing entity arg: {name}"))),
        };
        let from = entity_arg("from")?;
        let to = entity_arg("to")?;
        let amount = match request.args.get("amount") {
            Some(Value::Integer(value)) if *value > 0 => *value,
            _ => return Err(ActionError::Invalid("amount must be positive".into())),
        };

        let units = |id: EntityId| match state.entity(id).and_then(|e| e.component("units")) {
            Some(Value::Integer(value)) => Ok(*value),
            _ => Err(ActionError::Invalid(format!("entity {id} has no units"))),
        };

        let source = units(from)?;
        let destination = units(to)?;
        if source < amount {
            return Err(ActionError::Invalid("insufficient units".into()));
        }

        let mut payload = BTreeMap::new();
        payload.insert("amount".into(), amount.into());

        Ok(EventDraft {
            kind: "units_moved".into(),
            actor: request.actor,
            targets: vec![from, to],
            caused_by: request.caused_by.clone(),
            payload,
            changes: vec![
                StateChange::SetComponent {
                    entity: from,
                    key: "units".into(),
                    value: (source - amount).into(),
                },
                StateChange::SetComponent {
                    entity: to,
                    key: "units".into(),
                    value: (destination + amount).into(),
                },
            ],
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut initial = WorldState::default();
    initial.seed_entity(
        Entity::new(EntityId::new(1), "container")
            .with_component("name", "alpha")
            .with_component("units", 100_i64),
    )?;
    initial.seed_entity(
        Entity::new(EntityId::new(2), "container")
            .with_component("name", "beta")
            .with_component("units", 20_i64),
    )?;

    let mut registry = ActionRegistry::new();
    registry.register(MoveUnits)?;

    let mut world = World::new(initial);
    world.advance_to(&registry, 10)?;
    let event = world
        .execute(
            &registry,
            &ActionRequest::new("move_units")
                .arg("from", EntityId::new(1))
                .arg("to", EntityId::new(2))
                .arg("amount", 30_i64),
        )?
        .clone();

    println!("event #{} {} @ t={}", event.id, event.kind, event.world_time);
    for entity in world.state().entities() {
        println!("entity #{} {} {:?}", entity.id, entity.kind, entity.components);
    }

    let replayed = world.replay()?;
    assert_eq!(replayed.state(), world.state());
    println!("replay: deterministic ({} event)", replayed.events().len());

    Ok(())
}
