//! "Report this line": the player can say a resident's answer was wrong to
//! have said (harmful, not of the World, not in character). A report is an
//! Action like any other, recorded as an Event caused by the exchange it is
//! about, with the answer's words and who wrote them (the World, or a model
//! and the judge that passed it), so a report travels with the World and
//! replays with it. It changes nothing else.

use crate::SPOKEN;
use world_core::{
    Action, ActionError, ActionRequest, Event, EventDraft, EventId, Value, World, WorldState,
};

/// The kind of Event a report is recorded as.
pub const REPORTED: &str = "line_reported";

/// The command a screen sends to report the answer of the exchange
/// recorded as Event `id`: `conversation.report:<id>`.
pub const COMMAND: &str = "conversation.report:";

/// The command that reports the answer recorded as `spoken`.
pub fn command(spoken: EventId) -> String {
    format!("{COMMAND}{}", spoken.0)
}

/// The exchange a report command is about, if it is one.
pub fn parse_command(command: &str) -> Option<EventId> {
    command
        .strip_prefix(COMMAND)?
        .parse::<u64>()
        .ok()
        .map(EventId::new)
}

/// The Action that reports the answer recorded as `spoken`.
pub fn request(spoken: EventId) -> ActionRequest {
    ActionRequest::new(ACTION)
        .arg("line", Value::Integer(spoken.0 as i64))
        .caused_by(spoken)
}

/// Whether the answer recorded as `spoken` has been reported.
pub fn reported(world: &World, spoken: EventId) -> bool {
    world
        .events_of_kind(&[REPORTED])
        .iter()
        .any(|event| event.caused_by.contains(&spoken))
}

/// How a report is told in the World's history.
pub fn told(event: &Event) -> Option<String> {
    if event.kind != REPORTED {
        return None;
    }
    match event.payload.get("told") {
        Some(Value::Text(told)) => Some(told.clone()),
        _ => None,
    }
}

const ACTION: &str = "conversation_report";

pub(crate) struct Reports;

impl Action for Reports {
    fn name(&self) -> &'static str {
        ACTION
    }

    fn evaluate(
        &self,
        _state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let line = match request.args.get("line") {
            Some(Value::Integer(line)) if *line >= 0 => EventId::new(*line as u64),
            _ => return Err(ActionError::Invalid("report which line?".into())),
        };
        // The exchange itself is the cause the request names; [`report`]
        // checks it is one before asking.
        if !request.caused_by.contains(&line) {
            return Err(ActionError::Invalid(
                "a report is caused by the line it reports".into(),
            ));
        }
        let mut draft = EventDraft::new(REPORTED);
        draft
            .payload
            .insert("line".into(), Value::Integer(line.0 as i64));
        if let Some(Value::Entity(who)) = request.args.get("who") {
            draft.targets = vec![*who];
        }
        // What was reported and who wrote it, kept on the report itself.
        for key in ["reply", "by", "judge"] {
            if let Some(Value::Text(text)) = request.args.get(key) {
                if crate::clean(text, crate::MOST_REPLY) {
                    draft.payload.insert(key.into(), text.clone().into());
                }
            }
        }
        draft
            .payload
            .insert("told".into(), "You reported something said to you".into());
        Ok(draft)
    }
}

/// Reports the answer recorded as `spoken`: an Event caused by it, with
/// the answer's words and who wrote them; an error if `spoken` is no
/// exchange, or was reported already.
pub fn report(
    world: &mut World,
    actions: &world_core::ActionRegistry,
    spoken: EventId,
) -> Result<EventId, String> {
    let event = world
        .event(spoken)
        .filter(|event| event.kind == SPOKEN)
        .ok_or_else(|| format!("{spoken} is not something said"))?;
    if reported(world, spoken) {
        return Err("that line is reported already".into());
    }
    let text = |key: &str| match event.payload.get(key) {
        Some(Value::Text(text)) => Some(text.clone()),
        _ => None,
    };
    let mut request = request(spoken);
    if let Some(who) = event.targets.first() {
        request = request.arg("who", Value::Entity(*who));
    }
    if let Some(reply) = text("reply") {
        request = request.arg("reply", reply);
    }
    let by = if text("voiced").is_some() {
        "model"
    } else {
        "world"
    };
    request = request.arg("by", by);
    if let Some(judge) = text("judge") {
        request = request.arg("judge", judge);
    }
    Ok(world
        .execute(actions, &request)
        .map_err(|error| error.to_string())?
        .id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_command_names_the_line_it_reports() {
        let spoken = EventId::new(42);
        assert_eq!(parse_command(&command(spoken)), Some(spoken));
        assert_eq!(parse_command("conversation.report:x"), None);
        assert_eq!(parse_command("some-pack.wait"), None);
    }
}
