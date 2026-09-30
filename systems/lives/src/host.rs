//! Host: someone from another of the player's Worlds comes to visit,
//! brings a letter and leaves something to keep. Nothing is shared
//! between the Worlds: the guest arrives here as words, checked by an
//! Action in this World, and nothing is written back to theirs.

use super::*;

/// The most a guest's name, home, letter or gift can run to.
pub const MOST_GUEST_TEXT: usize = 280;

/// The most a guest's look or drawing, as the text they travel as, can run
/// to. A drawing is recorded with the visit and travels on in this World's
/// code, so it is kept small: a resident's own drawing is under 2 KiB.
pub const MOST_GUEST_LOOK: usize = 200;
pub const MOST_GUEST_DRAWING: usize = 8 * 1024;

/// How many periods a guest stays.
pub const GUEST_STAY_PERIODS: u64 = 3;

/// A guest's optional words: absent or blank is nothing; too long is
/// refused. Characters that change how text reads without being seen are
/// taken out (see [`world_core::text::clean_text`]).
fn optional_words(
    request: &ActionRequest,
    key: &str,
    most: usize,
) -> Result<Option<String>, ActionError> {
    let Ok(text) = arg_text(request, key) else {
        return Ok(None);
    };
    let text = world_core::text::clean_text(text);
    if text.chars().count() > most {
        return Err(ActionError::Invalid(format!("a guest's {key} is too long")));
    }
    Ok((!text.is_empty()).then_some(text))
}

/// A guest's optional code (their look, their drawing): absent or blank is
/// nothing; too long, or with any hidden control character in it, is
/// refused, since a code is read as it is.
fn optional_code<'a>(
    request: &'a ActionRequest,
    key: &str,
    most: usize,
) -> Result<Option<&'a str>, ActionError> {
    let Some(text) = arg_text(request, key).ok().map(str::trim) else {
        return Ok(None);
    };
    if text.len() > most || text.chars().any(world_core::text::is_hidden_control) {
        return Err(ActionError::Invalid(format!("a guest's {key} is too long")));
    }
    Ok((!text.is_empty()).then_some(text))
}

/// A guest from another World as this World hears of them: their name,
/// home, letter and gift, and, when their World shows them, a line they
/// say, how they look and the drawing they are drawn with, the last two as
/// the short texts they travel as. Nothing here is read from their World.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GuestWords<'a> {
    pub name: &'a str,
    pub from: &'a str,
    pub letter: &'a str,
    pub gift: &'a str,
    pub line: Option<&'a str>,
    pub look: Option<&'a str>,
    pub drawing: Option<&'a str>,
}

/// A guest staying in this World now, as their visit recorded them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Staying {
    pub visit: EventId,
    pub name: String,
    pub from: String,
    pub line: Option<String>,
    pub look: Option<String>,
    pub drawing: Option<String>,
}

/// A guest's words, cleaned of characters that change how text reads
/// without being seen: their name, where they are from, their letter.
fn guest_text(request: &ActionRequest, key: &str) -> Result<String, ActionError> {
    let text = world_core::text::clean_text(arg_text(request, key)?);
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
        let gift = optional_words(request, "gift", MOST_GUEST_TEXT)?;
        let line = optional_words(request, "line", MOST_GUEST_TEXT)?;
        let look = optional_code(request, "look", MOST_GUEST_LOOK)?;
        let drawing = optional_code(request, "drawing", MOST_GUEST_DRAWING)?;
        let host = cast.host;
        if state.entity(host).is_none() || gone(state, host) {
            return Err(ActionError::Invalid("nobody to welcome them".into()));
        }
        let mut draft = EventDraft::new("guest_visited");
        draft.actor = Some(host);
        draft.targets = vec![host];
        draft.payload.insert("guest".into(), guest.as_str().into());
        draft.payload.insert("from".into(), home.as_str().into());
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
        // A guest who comes as they look at home stays a few days, and
        // stands among the people here while they do.
        if line.is_some() || look.is_some() || drawing.is_some() {
            let until = state
                .world_time()
                .saturating_add(cast.period.max(1).saturating_mul(GUEST_STAY_PERIODS));
            draft
                .payload
                .insert("until".into(), Value::Integer(until as i64));
            for (key, text) in [
                ("line", line.as_deref()),
                ("look", look),
                ("drawing", drawing),
            ] {
                if let Some(text) = text {
                    draft.payload.insert(key.into(), text.into());
                }
            }
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
    host_guest_with(
        world,
        actions,
        cast,
        &GuestWords {
            name,
            from,
            letter,
            gift,
            ..GuestWords::default()
        },
    )
}

/// Welcomes a guest who comes as they look at home: as [`host_guest`],
/// and they stay a few days, saying their line.
pub fn host_guest_with(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    guest: &GuestWords,
) -> Result<EventId, WorldError> {
    let mut request = ActionRequest::new("lives_hosts_guest")
        .actor(cast.host)
        .arg("name", guest.name)
        .arg("from", guest.from)
        .arg("letter", guest.letter);
    if room_for_keepsake(world, cast) {
        request = request.arg("gift", guest.gift);
    }
    for (key, text) in [
        ("line", guest.line),
        ("look", guest.look),
        ("drawing", guest.drawing),
    ] {
        if let Some(text) = text.filter(|text| !text.trim().is_empty()) {
            request = request.arg(key, text);
        }
    }
    Ok(world.execute(actions, &request)?.id)
}

/// The guests staying in this World now, earliest first, as their visits
/// recorded them.
pub fn guests_staying(world: &World, cast: &Cast) -> Vec<Staying> {
    let now = world.world_time();
    let stay = cast.period.max(1).saturating_mul(GUEST_STAY_PERIODS);
    let text = |event: &Event, key: &str| match event.payload.get(key) {
        Some(Value::Text(text)) => Some(text.clone()),
        _ => None,
    };
    let mut staying = world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time.saturating_add(stay) > now)
        .filter(|event| event.kind == "guest_visited")
        .filter_map(|event| {
            let Some(Value::Integer(until)) = event.payload.get("until") else {
                return None;
            };
            (*until > now as i64).then(|| Staying {
                visit: event.id,
                name: text(event, "guest").unwrap_or_default(),
                from: text(event, "from").unwrap_or_default(),
                line: text(event, "line"),
                look: text(event, "look"),
                drawing: text(event, "drawing"),
            })
        })
        .take(4)
        .collect::<Vec<_>>();
    staying.reverse();
    staying
}
