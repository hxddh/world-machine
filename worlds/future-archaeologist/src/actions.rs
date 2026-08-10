use crate::model::*;
use world_core::{Action, ActionError, ActionRegistry, ActionRequest, EventDraft, StateChange, Value, WorldState};

pub(crate) fn register(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(RecoverDeletedMessage)?;
    Ok(())
}

struct RecoverDeletedMessage;

impl Action for RecoverDeletedMessage {
    fn name(&self) -> &'static str {
        "recover_deleted_message"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let visible = match state
            .entity(DELETED_MESSAGE)
            .and_then(|entity| entity.component(VISIBLE))
        {
            Some(Value::Bool(value)) => *value,
            _ => {
                return Err(ActionError::Invalid(
                    "deleted message has no visibility state".into(),
                ))
            }
        };
        if visible {
            return Err(ActionError::Invalid(
                "deleted message has already been recovered".into(),
            ));
        }

        let mut draft = EventDraft::new("artifact_recovered");
        draft.targets = vec![DELETED_MESSAGE];
        draft
            .payload
            .insert("method".into(), Value::Text("unallocated_scan".into()));
        draft.changes.push(StateChange::SetComponent {
            entity: DELETED_MESSAGE,
            key: VISIBLE.into(),
            value: true.into(),
        });
        Ok(draft)
    }
}
