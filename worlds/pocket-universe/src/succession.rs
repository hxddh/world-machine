//! Chapter four: after the pressure has resolved one way or another, someone
//! who did not live through the World's beginning steps forward, and the
//! observer decides whether the World's legacy passes on intact or is theirs
//! to rewrite.
//!
//! Chapter three had a deadline; this one deliberately does not. The successor
//! waits as long as the observer leaves them waiting — but they are not idle
//! while they wait. Every generation without a decision deepens habits of
//! their own, so a World left alone for a long time is not the same World that
//! was left, and "release" gradually becomes a description of what has already
//! happened rather than a choice against it.
//!
//! Everything here is a validated Action producing an Event with explicit
//! state changes, so replay never re-runs these rules. The kernel knows
//! nothing about succession; it is Pocket Universe vocabulary only.

use super::*;

/// On `UNIVERSE`: `none` → `emerging` → `settled`.
pub(crate) const SUCCESSION: &str = "succession";
/// Generations the successor has waited for a decision.
pub(crate) const SUCCESSION_PATIENCE: &str = "succession_patience";
/// Durable record of how the World answered: `none`, `continued`, `renewed`.
pub(crate) const SUCCESSION_OUTCOME: &str = "succession_outcome";

/// Generations after the pressure resolved before a successor steps forward.
const EMERGE_AFTER_GENERATIONS: i64 = 2;
/// Patience at which the successor's own habits start to show.
const SETTLING_PATIENCE: i64 = 2;
/// Patience beyond which the successor has effectively made the work theirs.
const OWNED_PATIENCE: i64 = 4;

pub(crate) fn register_actions(actions: &mut ActionRegistry) -> Result<(), ActionError> {
    actions.register(RaiseSuccessor)?;
    actions.register(DeepenSuccessor)?;
    actions.register(EntrustLegacy)?;
    actions.register(ReleaseLegacy)?;
    Ok(())
}

/// Advance succession at most one step per period. Runs after the pressure
/// chapter so the emergence reads the outcome that period wrote.
pub(crate) fn resolve_stage(
    world: &mut World,
    actions: &ActionRegistry,
    tail: EventId,
) -> Result<EventId, Box<dyn Error>> {
    let state = world.state();
    let action = if emerge_candidate(state)? {
        "raise_successor"
    } else if deepen_candidate(state)? {
        "deepen_successor"
    } else {
        return Ok(tail);
    };
    let mut request = ActionRequest::new(action).caused_by(tail);
    for cause in succession_causes(world) {
        if cause != tail {
            request = request.caused_by(cause);
        }
    }
    Ok(world.execute(actions, &request)?.id)
}

pub(crate) fn succession_id_from_state(state: &WorldState) -> String {
    match state
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(SUCCESSION))
    {
        Some(Value::Text(succession)) => succession.clone(),
        _ => "none".into(),
    }
}

pub(crate) fn succession_patience_from_state(state: &WorldState) -> i64 {
    match state
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(SUCCESSION_PATIENCE))
    {
        Some(Value::Integer(patience)) => *patience,
        _ => 0,
    }
}

/// True while the observer can still entrust or release.
pub(crate) fn choice_open(succession: &str) -> bool {
    succession == "emerging"
}

/// How far the successor has gone toward making the work their own. The bands
/// are what the copy reads from, so they are named rather than numeric.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SuccessorStanding {
    /// Newly stepped forward; still working the way they were shown.
    NewHands,
    /// Long enough to have opinions about the work.
    OwnHabits,
    /// Long enough that the work is already theirs in all but name.
    AlreadyTheirs,
}

pub(crate) fn standing_for_patience(patience: i64) -> SuccessorStanding {
    if patience >= OWNED_PATIENCE {
        SuccessorStanding::AlreadyTheirs
    } else if patience >= SETTLING_PATIENCE {
        SuccessorStanding::OwnHabits
    } else {
        SuccessorStanding::NewHands
    }
}

fn succession_causes(world: &World) -> Vec<EventId> {
    let mut causes = Vec::new();
    for kind in [
        "successor_emerged",
        "successor_waited",
        "pressure_held",
        "pressure_reached",
        "anchor_lost",
        "anchor_recovered",
        "world_legacy_formed",
    ] {
        if let Some(event) = world.events().iter().rev().find(|event| event.kind == kind) {
            if !causes.contains(&event.id) {
                causes.push(event.id);
            }
        }
    }
    causes
}

/// The pressure chapter has to have finished before anyone can inherit.
fn pressure_settled(state: &WorldState) -> bool {
    matches!(
        pressure::pressure_id_from_state(state).as_str(),
        "held" | "reached" | "lost" | "recovered"
    )
}

fn emerge_candidate(state: &WorldState) -> Result<bool, ActionError> {
    if succession_id_from_state(state) != "none" || !pressure_settled(state) {
        return Ok(false);
    }
    let generation = integer_component(state, UNIVERSE, GENERATION)?;
    let since = pressure::pressure_generation_from_state(state);
    Ok(generation >= since + EMERGE_AFTER_GENERATIONS)
}

fn deepen_candidate(state: &WorldState) -> Result<bool, ActionError> {
    Ok(succession_id_from_state(state) == "emerging")
}

/// Seed-specific copy for the whole chapter.
#[derive(Clone, Copy)]
pub(crate) struct SuccessionCopy {
    pub successor: &'static str,
    pub emerged_summary: &'static str,
    pub new_hands_summary: &'static str,
    pub own_habits_summary: &'static str,
    pub already_theirs_summary: &'static str,
    pub entrust_title: &'static str,
    pub entrust_detail: &'static str,
    pub entrust_summary: &'static str,
    pub release_title: &'static str,
    pub release_detail: &'static str,
    pub release_summary: &'static str,
}

pub(crate) fn copy_for_seed(seed: &str) -> SuccessionCopy {
    match seed {
        "mars-colony" => SuccessionCopy {
            successor: "Ines",
            emerged_summary: "Ines, who was born on the ridge and has never seen Earth, takes the second shift at the reclaimer.",
            new_hands_summary: "Ines runs the reclaimer exactly the way Nia showed her, down to the order of the valves.",
            own_habits_summary: "Ines has started logging the reclaimer her own way. Nia has stopped correcting her.",
            already_theirs_summary: "The reclaimer runs on Ines's schedule now. Nia signs off on logs she no longer writes.",
            entrust_title: "Hand Ines the routines as they are",
            entrust_detail: "Nia teaches Ines the habits Ares built, unchanged, and steps back. The colony keeps doing what worked.",
            entrust_summary: "Nia handed Ares's routines to Ines unchanged. The colony's habits outlived the people who made them.",
            release_title: "Let Ines run Ares her own way",
            release_detail: "Nia lets Ines rebuild the colony's routines for people who were born here. What Ares learned is hers to keep or drop.",
            release_summary: "Ines rebuilt Ares's routines for people born on the ridge. What the colony learned is a story now, not a rule.",
        },
        "1980s-town" => SuccessionCopy {
            successor: "Dana",
            emerged_summary: "Dana, who grew up on the arcade's carpet, starts closing up on Fridays.",
            new_hands_summary: "Dana closes up exactly the way Lena taught her, including the sign in the window.",
            own_habits_summary: "Dana has changed the Friday rotation. Lena noticed and said nothing.",
            already_theirs_summary: "The Friday nights run on Dana's rules now. Lena comes in as a regular.",
            entrust_title: "Hand Dana the Friday rules as they are",
            entrust_detail: "Lena teaches Dana the arcade's rules unchanged and steps back. Maple Street keeps the nights it knows.",
            entrust_summary: "Lena handed the arcade's Friday rules to Dana unchanged. The nights outlived the people who set them.",
            release_title: "Let Dana run the nights her way",
            release_detail: "Lena lets Dana rebuild Friday for the kids who are here now. What the arcade was is hers to keep or drop.",
            release_summary: "Dana rebuilt Friday night for the kids who are here now. What the arcade was is a story now, not a rule.",
        },
        "penguin-civilization" => SuccessionCopy {
            successor: "Sura",
            emerged_summary: "Sura, hatched after the bridge was settled, starts walking the spans at each moonrise.",
            new_hands_summary: "Sura walks the spans in Piko's order, checking the same joints in the same sequence.",
            own_habits_summary: "Sura has changed which joints she checks first. Piko has stopped following behind her.",
            already_theirs_summary: "The moonrise walk is Sura's route now. Piko attends the council and says little.",
            entrust_title: "Hand Sura the crossing rites as they are",
            entrust_detail: "Piko teaches Sura the council's rites unchanged and steps back. Icebridge keeps the crossings it knows.",
            entrust_summary: "Piko handed the crossing rites to Sura unchanged. The rites outlived the keeper who kept them.",
            release_title: "Let Sura keep the bridge her way",
            release_detail: "Piko lets Sura rewrite the rites for a colony that has never lost a span. What Icebridge learned is hers to keep or drop.",
            release_summary: "Sura rewrote the crossing rites for a colony that never lost a span. What Icebridge learned is a story now, not a rule.",
        },
        _ => SuccessionCopy {
            successor: "the successor",
            emerged_summary: "Someone who did not live through this World's beginning steps forward to keep it running.",
            new_hands_summary: "The successor keeps the World running exactly the way they were shown.",
            own_habits_summary: "The successor has started doing the work their own way.",
            already_theirs_summary: "The work is the successor's now in all but name.",
            entrust_title: "Hand the legacy on as it is",
            entrust_detail: "Pass what this World built to the successor unchanged.",
            entrust_summary: "The World's legacy was handed on unchanged, and outlived the people who made it.",
            release_title: "Let the successor rewrite it",
            release_detail: "Let the successor rebuild what this World does for the people who are here now.",
            release_summary: "The successor rebuilt what this World does. What it learned is a story now, not a rule.",
        },
    }
}

/// How the same choice reads depending on how chapter three ended. A World
/// that held together hands on out of confidence; one that lost its anchor
/// hands on out of exhaustion, and the successor inherits that too.
pub(crate) fn inheritance_note(pressure_outcome: &str, continued: bool) -> &'static str {
    match (pressure_outcome, continued) {
        ("aligned", true) => {
            "The routines being handed on are the ones that answered the pressure."
        }
        ("aligned", false) => "The routines being let go are the ones that answered the pressure.",
        ("strained", true) => {
            "The routines being handed on are the ones that answered the pressure badly."
        }
        ("strained", false) => "What is being let go had already been strained once.",
        ("lost", true) => "What is being handed on is a World that did not answer in time.",
        ("lost", false) => "There is less to let go of than there would have been.",
        ("recovered", true) => "What is being handed on was rebuilt once already.",
        ("recovered", false) => "What is being let go was rebuilt once already.",
        (_, true) => "The World hands on what it has.",
        (_, false) => "The World lets go of what it has.",
    }
}

pub(crate) fn succession_outcome_from_state(state: &WorldState) -> String {
    match state
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(SUCCESSION_OUTCOME))
    {
        Some(Value::Text(outcome)) => outcome.clone(),
        _ => "none".into(),
    }
}

fn pressure_outcome_from_state(state: &WorldState) -> String {
    match state
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(pressure::PRESSURE_OUTCOME))
    {
        Some(Value::Text(outcome)) => outcome.clone(),
        _ => "none".into(),
    }
}

/// Chapter four never writes the anchor's `status`: that component is chapter
/// three's durable record of how the pressure ended, and a World that lost its
/// anchor must keep saying so even after somebody inherits it.
fn succession_draft(
    state: &WorldState,
    kind: &str,
    stage: &str,
    summary: &str,
    patience: i64,
    outcome: Option<&str>,
) -> Result<EventDraft, ActionError> {
    let seed = seed_id_from_state(state)?;
    let mut draft = EventDraft::new(kind);
    draft.targets = vec![UNIVERSE, SLOT_A];
    draft.payload.insert("seed".into(), seed.into());
    draft.payload.insert("stage".into(), stage.into());
    draft.payload.insert("summary".into(), summary.into());
    draft.payload.insert("patience".into(), patience.into());
    draft.changes = vec![
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: SUCCESSION.into(),
            value: stage.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: SUCCESSION_PATIENCE.into(),
            value: patience.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),
            value: summary.into(),
        },
    ];
    if let Some(outcome) = outcome {
        draft.payload.insert("outcome".into(), outcome.into());
        draft.changes.push(StateChange::SetComponent {
            entity: UNIVERSE,
            key: SUCCESSION_OUTCOME.into(),
            value: outcome.into(),
        });
    }
    Ok(draft)
}

struct RaiseSuccessor;
struct DeepenSuccessor;
struct EntrustLegacy;
struct ReleaseLegacy;

impl Action for RaiseSuccessor {
    fn name(&self) -> &'static str {
        "raise_successor"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if !emerge_candidate(state)? {
            return Err(ActionError::Invalid(
                "this World has nobody ready to inherit it yet".into(),
            ));
        }
        let copy = copy_for_seed(&seed_id_from_state(state)?);
        let mut draft = succession_draft(
            state,
            "successor_emerged",
            "emerging",
            copy.emerged_summary,
            0,
            None,
        )?;
        draft
            .payload
            .insert("successor".into(), copy.successor.into());
        Ok(draft)
    }
}

impl Action for DeepenSuccessor {
    fn name(&self) -> &'static str {
        "deepen_successor"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if !deepen_candidate(state)? {
            return Err(ActionError::Invalid(
                "this World has no successor waiting on a decision".into(),
            ));
        }
        let copy = copy_for_seed(&seed_id_from_state(state)?);
        let patience = succession_patience_from_state(state).saturating_add(1);
        let summary = match standing_for_patience(patience) {
            SuccessorStanding::NewHands => copy.new_hands_summary,
            SuccessorStanding::OwnHabits => copy.own_habits_summary,
            SuccessorStanding::AlreadyTheirs => copy.already_theirs_summary,
        };
        succession_draft(
            state,
            "successor_waited",
            "emerging",
            summary,
            patience,
            None,
        )
    }
}

fn answer_draft(state: &WorldState, continued: bool) -> Result<EventDraft, ActionError> {
    let seed = seed_id_from_state(state)?;
    if seed == UNSEEDED {
        return Err(ActionError::Invalid(
            "choose a Pocket Universe seed before deciding its succession".into(),
        ));
    }
    let succession = succession_id_from_state(state);
    if !choice_open(&succession) {
        return Err(ActionError::Invalid(format!(
            "this World has no successor waiting on a decision (succession is {succession})"
        )));
    }
    let patience = succession_patience_from_state(state);
    let copy = copy_for_seed(&seed);
    let (kind, outcome, summary) = if continued {
        ("legacy_entrusted", "continued", copy.entrust_summary)
    } else {
        ("legacy_released", "renewed", copy.release_summary)
    };
    let pressure_outcome = pressure_outcome_from_state(state);
    let mut draft = succession_draft(state, kind, "settled", summary, patience, Some(outcome))?;
    draft
        .payload
        .insert("successor".into(), copy.successor.into());
    draft
        .payload
        .insert("pressure_outcome".into(), pressure_outcome.clone().into());
    draft.payload.insert(
        "inheritance".into(),
        inheritance_note(&pressure_outcome, continued).into(),
    );
    Ok(draft)
}

impl Action for EntrustLegacy {
    fn name(&self) -> &'static str {
        "entrust_legacy"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        answer_draft(state, true)
    }
}

impl Action for ReleaseLegacy {
    fn name(&self) -> &'static str {
        "release_legacy"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let mut draft = answer_draft(state, false)?;
        // Releasing is durable and costly in the same currency chapter three
        // used: the legacy the World had built stops accumulating and the
        // successor's version has to earn its own cycles.
        draft.changes.push(StateChange::SetComponent {
            entity: UNIVERSE,
            key: LEGACY_CYCLES.into(),
            value: 0_i64.into(),
        });
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_seed_has_distinct_copy_for_every_stage() {
        for seed in ["mars-colony", "1980s-town", "penguin-civilization"] {
            let copy = copy_for_seed(seed);
            let summaries = [
                copy.emerged_summary,
                copy.new_hands_summary,
                copy.own_habits_summary,
                copy.already_theirs_summary,
                copy.entrust_summary,
                copy.release_summary,
            ];
            for (index, summary) in summaries.iter().enumerate() {
                assert!(!summary.is_empty(), "{seed} has an empty summary");
                assert!(
                    !summaries[index + 1..].contains(summary),
                    "{seed} reuses summary {summary}"
                );
            }
            assert_ne!(copy.entrust_title, copy.release_title);
            assert!(!copy.successor.is_empty());
        }
    }

    #[test]
    fn the_choice_is_open_only_while_a_successor_is_waiting() {
        assert!(choice_open("emerging"));
        for closed in ["none", "settled"] {
            assert!(!choice_open(closed), "{closed} should be closed");
        }
    }

    #[test]
    fn waiting_moves_the_successor_from_new_hands_to_owning_the_work() {
        assert_eq!(standing_for_patience(0), SuccessorStanding::NewHands);
        assert_eq!(standing_for_patience(1), SuccessorStanding::NewHands);
        assert_eq!(standing_for_patience(2), SuccessorStanding::OwnHabits);
        assert_eq!(standing_for_patience(3), SuccessorStanding::OwnHabits);
        assert_eq!(standing_for_patience(4), SuccessorStanding::AlreadyTheirs);
        assert_eq!(standing_for_patience(40), SuccessorStanding::AlreadyTheirs);
    }

    #[test]
    fn the_same_choice_reads_differently_after_each_pressure_ending() {
        let mut seen = Vec::new();
        for outcome in ["aligned", "strained", "lost", "recovered"] {
            for continued in [true, false] {
                let note = inheritance_note(outcome, continued);
                assert!(!note.is_empty());
                assert!(
                    !seen.contains(&note),
                    "{outcome}/{continued} reuses a note: {note}"
                );
                seen.push(note);
            }
        }
        // An unfinished or unknown pressure chapter still reads sensibly.
        assert!(!inheritance_note("none", true).is_empty());
        assert!(!inheritance_note("none", false).is_empty());
    }
}
