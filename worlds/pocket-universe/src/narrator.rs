//! Who says what happened, when a World is asked to say it in its own words.
//!
//! Measured before this existed: eighty periods of play put 586 lines of prose
//! on screen, of which 114 were distinct, and the first repeated line arrived
//! at period 3. Two Worlds seeded the same way and answered the opposite way at
//! every choice still shared 46% of their prose word for word. The era engine
//! made a World's story unbounded; its vocabulary stayed a lookup table.
//!
//! The deterministic path is untouched and always writes its line first: every
//! consequence Event carries a `summary` and sets `last_change`. A narrator, if
//! this World has one, is then asked to say that same already-decided fact in
//! this World's own words, and what it says is recorded as a `world_narrated`
//! Event that overwrites `last_change`.
//!
//! Three rules keep this a decoration rather than a dependency:
//!
//! * **The table is the floor.** A narrator that is absent, silent, or returns
//!   something unusable leaves the World reading exactly as it did before. A
//!   World is never worse for having asked.
//! * **Structure is not on the table.** The narrator is handed facts that are
//!   already decided and already recorded. It cannot change which trouble comes
//!   next, when an era turns, or what a choice cost — only how it reads.
//! * **Recorded, never recomputed.** `World::replay` applies the StateChanges of
//!   recorded Events and never re-runs `evaluate`, so a narrated line is as
//!   durable and as replay-exact as any other fact, and replaying a World never
//!   asks a narrator anything.

use super::*;
use crate::projection;
use world_core::Event;

pub(crate) const NARRATED: &str = "world_narrated";
/// The Event this line re-words, so a briefing can prefer it over the table.
pub(crate) const ABOUT: &str = "about";
pub(crate) const TEXT: &str = "text";
pub(crate) const ABOUT_ARG: &str = "about";
pub(crate) const TEXT_ARG: &str = "text";

/// A narrated line is one paragraph of plain prose. Long enough for the two or
/// three sentences the table lines run to, short enough that a runaway
/// generation cannot flood a briefing or a World file.
pub(crate) const MAX_NARRATION_CHARS: usize = 600;

/// The already-decided facts a narrator is asked to put into words.
///
/// Everything here is read back out of the World's own recorded state and
/// Events. A narrator receives facts, never authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NarrationFacts {
    /// Which seed this World grew from, e.g. `mars-colony`.
    pub seed: String,
    /// Which era it is now.
    pub era: i64,
    /// The kind of the Event being re-worded, e.g. `pressure_peaked`.
    pub event_kind: String,
    /// The line the World would show if nobody narrated. A narrator is being
    /// asked to say this, not to replace what it says.
    pub table_summary: String,
}

/// Says what happened, in this World's own words.
///
/// Returning `None` — or anything [`usable_narration`] rejects — is always
/// allowed and always safe: the World keeps its table line.
pub trait Narrator {
    fn narrate(&mut self, facts: &NarrationFacts) -> Option<String>;

    /// Say everything a returning observer is about to read, in one go.
    ///
    /// A return budgets one request, however many lines it shows, so a narrator
    /// backed by a model overrides this to ask once for the whole briefing
    /// rather than once per line. The default asks line by line, which is what
    /// a local or scripted narrator wants.
    ///
    /// The returned lines correspond to `facts` by position; a shorter answer
    /// leaves the rest on their table lines, which is the usual fallback.
    fn narrate_all(&mut self, facts: &[NarrationFacts]) -> Vec<Option<String>> {
        facts.iter().map(|fact| self.narrate(fact)).collect()
    }
}

/// The default. A World with no narrator reads from the table, which is every
/// World shipped before this existed and every World whose observer has not
/// given the app a way to reach a model.
pub struct NoNarrator;

impl Narrator for NoNarrator {
    fn narrate(&mut self, _facts: &NarrationFacts) -> Option<String> {
        None
    }
}

/// Whether a narrated line can be shown to somebody.
///
/// Rejects what a bad or hostile generation produces — nothing, whitespace, a
/// flood, or control characters and line breaks that would break a briefing's
/// layout — and normalises the rest. Rejection is not an error: it means the
/// table line stands.
pub(crate) fn usable_narration(candidate: &str) -> Option<String> {
    let trimmed = candidate.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_NARRATION_CHARS {
        return None;
    }
    if trimmed
        .chars()
        .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'))
    {
        return None;
    }
    Some(trimmed.to_owned())
}

/// The facts behind each line a returning observer is about to read.
///
/// These are exactly the Events the return digest will show — the projection
/// decides that, and hands the same list here, so what gets put into words can
/// never drift from what gets read. `since` is where the observer last looked.
///
/// A World is narrated at the lines about to be read, not once per period: a
/// long absence resolves as fast as it does today and costs one narration.
pub(crate) fn return_facts(world: &World, since: usize) -> Vec<(EventId, NarrationFacts)> {
    let events = world.events();
    let Some(window) = events.get(since..) else {
        return Vec::new();
    };
    let state = world.state();
    let Ok(seed) = seed_id_from_state(state) else {
        return Vec::new();
    };
    let era = era::era_from_state(state);
    projection::digest_events(window)
        .into_iter()
        .filter_map(|(event, _)| {
            let Some(Value::Text(summary)) = event.payload.get("summary") else {
                return None;
            };
            Some((
                event.id,
                NarrationFacts {
                    seed: seed.clone(),
                    era,
                    event_kind: event.kind.clone(),
                    table_summary: summary.clone(),
                },
            ))
        })
        .collect()
}

/// Ask this World's narrator to say what is about to be read in its own words,
/// and record what it says.
///
/// Every unusable answer — no narrator, nothing to narrate, a silent line, a
/// line the Action would refuse — leaves that line on the table, which is the
/// point. A narrator that answers for none of them costs the World nothing.
pub(crate) fn narrate_return(
    world: &mut World,
    actions: &ActionRegistry,
    narrator: &mut dyn Narrator,
    since: usize,
) -> usize {
    let pending = return_facts(world, since)
        .into_iter()
        .filter(|(about, _)| !already_narrated(world, *about))
        .collect::<Vec<_>>();
    if pending.is_empty() {
        return 0;
    }
    let facts = pending
        .iter()
        .map(|(_, facts)| facts.clone())
        .collect::<Vec<_>>();
    let lines = narrator.narrate_all(&facts);

    let mut recorded = 0;
    for ((about, _), line) in pending.iter().zip(lines) {
        let Some(text) = line.as_deref().and_then(usable_narration) else {
            continue;
        };
        let request = ActionRequest::new("narrate_world")
            .actor(UNIVERSE)
            .arg(ABOUT_ARG, about.0 as i64)
            .arg(TEXT_ARG, text)
            .caused_by(*about);
        // Deliberately infallible. `usable_narration` has already accepted this
        // line, so the Action cannot reject it; and if that ever stopped being
        // true, a World that moved must not be reported as a World that failed.
        // The table line stands and everything that happened is kept.
        if world.execute(actions, &request).is_ok() {
            recorded += 1;
        }
    }
    recorded
}

/// Whether this Event has already been put into the World's own words. A
/// returning observer narrates what is new, never what they have already read.
fn already_narrated(world: &World, about: EventId) -> bool {
    world
        .events()
        .iter()
        .rev()
        .filter(|event| event.kind == NARRATED)
        .any(|event| matches!(event.payload.get(ABOUT), Some(Value::Integer(id)) if *id == about.0 as i64))
}

/// The line to show for an Event: this World's own words if it has them, and
/// the table line otherwise.
pub(crate) fn narrated_text(events: &[Event], about: EventId) -> Option<&str> {
    events
        .iter()
        .rev()
        .filter(|event| event.kind == NARRATED)
        .find(|event| {
            matches!(event.payload.get(ABOUT), Some(Value::Integer(id)) if *id == about.0 as i64)
        })
        .and_then(|event| match event.payload.get(TEXT) {
            Some(Value::Text(text)) => Some(text.as_str()),
            _ => None,
        })
}

pub(crate) fn register_actions(actions: &mut ActionRegistry) -> Result<(), ActionError> {
    actions.register(NarrateWorld)?;
    Ok(())
}

struct NarrateWorld;

impl Action for NarrateWorld {
    fn name(&self) -> &'static str {
        "narrate_world"
    }

    fn evaluate(
        &self,
        _state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let about = match request.args.get(ABOUT_ARG) {
            Some(Value::Integer(id)) if *id >= 0 => *id,
            _ => {
                return Err(ActionError::Invalid(
                    "a narrated line has to say which Event it re-words".into(),
                ))
            }
        };
        // Validated here as well as before the request, because an Action is
        // the only thing that decides what may become a durable Event.
        let text = match request.args.get(TEXT_ARG) {
            Some(Value::Text(text)) => usable_narration(text).ok_or_else(|| {
                ActionError::Invalid(
                    "a narrated line has to be one readable paragraph of plain prose".into(),
                )
            })?,
            _ => {
                return Err(ActionError::Invalid(
                    "a narrated line has to carry the words to say".into(),
                ))
            }
        };

        let mut draft = EventDraft::new(NARRATED);
        draft.targets = vec![UNIVERSE];
        draft.payload.insert(ABOUT.into(), about.into());
        draft.payload.insert(TEXT.into(), text.clone().into());
        draft.changes = vec![StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),
            value: text.into(),
        }];
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_readable_paragraph_is_accepted_and_tidied() {
        assert_eq!(
            usable_narration("  Ares closed its own loop.  "),
            Some("Ares closed its own loop.".to_owned())
        );
        let longest = "a".repeat(MAX_NARRATION_CHARS);
        assert_eq!(usable_narration(&longest), Some(longest.clone()));
    }

    #[test]
    fn nothing_a_bad_generation_produces_can_reach_a_briefing() {
        // Every rejection means the same thing: the table line stands.
        for unusable in [
            "",
            "   ",
            "\n\t ",
            "Two lines\nis not one paragraph.",
            "A carriage\rreturn is not either.",
            "Line\u{2028}separator",
            "Paragraph\u{2029}separator",
            "A bell\u{7} is not prose.",
        ] {
            assert_eq!(usable_narration(unusable), None, "{unusable:?}");
        }
        let flood = "a".repeat(MAX_NARRATION_CHARS + 1);
        assert_eq!(usable_narration(&flood), None);
    }

    #[test]
    fn length_is_counted_in_characters_rather_than_bytes() {
        // A World narrated in a language whose characters are several bytes
        // each must not be cut off earlier than one narrated in English.
        let multibyte = "世".repeat(MAX_NARRATION_CHARS);
        assert!(multibyte.len() > MAX_NARRATION_CHARS);
        assert_eq!(usable_narration(&multibyte), Some(multibyte));
    }
}
