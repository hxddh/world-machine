//! Host: someone from another of the player's Worlds comes to visit,
//! brings a letter and leaves something to keep. Nothing is shared
//! between the Worlds: the guest arrives here as words, checked by an
//! Action in this World, and nothing is written back to theirs.

use super::*;

/// The most a guest's name, home, letter or gift can run to.
pub const MOST_GUEST_TEXT: usize = 280;

fn guest_text<'a>(request: &'a ActionRequest, key: &str) -> Result<&'a str, ActionError> {
    let text = arg_text(request, key)?.trim();
    if text.is_empty() || text.chars().count() > MOST_GUEST_TEXT {
        return Err(ActionError::Invalid(format!(
            "a guest's {key} is missing or too long"
        )));
    }
    Ok(text)
}

/// A guest from another World is welcomed.
pub(crate) struct Hosts(pub(crate) fn(&WorldState) -> Cast);

impl Action for Hosts {
    fn name(&self) -> &'static str {
        "lives_hosts_guest"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let guest = guest_text(request, "name")?;
        let home = guest_text(request, "from")?;
        let letter = guest_text(request, "letter")?;
        let gift = arg_text(request, "gift")
            .ok()
            .map(str::trim)
            .filter(|gift| !gift.is_empty());
        if gift.is_some_and(|gift| gift.chars().count() > MOST_GUEST_TEXT) {
            return Err(ActionError::Invalid("a guest's gift is too long".into()));
        }
        let host = cast.host;
        if state.entity(host).is_none() || gone(state, host) {
            return Err(ActionError::Invalid("nobody to welcome them".into()));
        }
        let mut draft = EventDraft::new("guest_visited");
        draft.actor = Some(host);
        draft.targets = vec![host];
        draft.payload.insert("guest".into(), guest.into());
        draft.payload.insert("from".into(), home.into());
        draft.payload.insert(
            "told".into(),
            format!("{guest} came over from {home} to visit").into(),
        );
        draft.payload.insert(
            "said".into(),
            format!("{guest} brought you a letter. Here.").into(),
        );
        draft.payload.insert(
            "note".into(),
            format!("From {guest}, of {home}: {letter}").into(),
        );
        if let Some(gift) = gift {
            draft.payload.insert("keepsake".into(), gift.into());
            draft.payload.insert("kept".into(), true.into());
        }
        Ok(draft)
    }
}

/// Welcomes a guest from another World: a letter for the letter box, and,
/// if the week has room, something to keep.
pub fn host_guest(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    name: &str,
    from: &str,
    letter: &str,
    gift: &str,
) -> Result<EventId, WorldError> {
    let mut request = ActionRequest::new("lives_hosts_guest")
        .actor(cast.host)
        .arg("name", name)
        .arg("from", from)
        .arg("letter", letter);
    if room_for_keepsake(world, cast) {
        request = request.arg("gift", gift);
    }
    Ok(world.execute(actions, &request)?.id)
}
