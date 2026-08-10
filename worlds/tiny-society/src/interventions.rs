use crate::{actions::text_component, model::*};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EventDraft, Value, WorldState,
};

pub(crate) fn register(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(RetainWorker)?;
    Ok(())
}

struct RetainWorker;

impl Action for RetainWorker {
    fn name(&self) -> &'static str {
        "retain_worker"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, WEDDING_ORDER, ORDER_STATUS)? != "lost" {
            return Err(ActionError::Invalid(
                "the bakery has not lost the wedding order".into(),
            ));
        }
        if state.relation(TEMP_BAKERY_JOB).is_none() {
            return Err(ActionError::Invalid(
                "Jonas is no longer a temporary bakery worker".into(),
            ));
        }

        let mut draft = EventDraft::new("worker_retained");
        draft.actor = Some(MARA);
        draft.targets = vec![JONAS, BAKERY];
        draft
            .payload
            .insert("decision".into(), Value::Text("second_chance".into()));
        Ok(draft)
    }
}
