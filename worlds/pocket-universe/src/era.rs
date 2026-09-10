//! The era engine: what makes a World's story loop instead of end.
//!
//! The four chapters used to be a ladder chained at their call sites — the
//! legacy chapter called the pressure chapter, which called succession, and
//! when succession settled the World was finished. Measured, that was thirteen
//! periods of content followed by a World whose only remaining command did
//! nothing to its story.
//!
//! Here the four chapters are the stages of one **era**, and this module drives
//! them in order and then closes the loop: when a succession settles, the
//! successor becomes the anchor-keeper of a new era, and the World faces a
//! different threat. Two things keep the loop from being a replay of itself:
//! each seed has a pool of threats and the next one is never the one just
//! survived, and how the last era ended decides what the next one inherits.
//!
//! Selection is a deterministic function of the World's own state and history —
//! never a random draw — so two Worlds with the same history face the same
//! next era, and replay stays exact.

use super::*;

/// Which era the World is living. Absent on Worlds seeded before the engine
/// existed, which read as their first era.
pub(crate) const ERA: &str = "era";

/// Generations of calm after an era opens before its threat can rise. Without
/// it, an era inherited with its legacy intact would face a new pressure the
/// period it began.
const CALM_GENERATIONS: i64 = 2;

/// The argument carrying the threat the next era will face. The era engine
/// chooses it — the Action only checks that the choice is a legal one.
pub(crate) const PRESSURE_KIND_ARG: &str = "pressure_kind";

pub(crate) fn register_actions(actions: &mut ActionRegistry) -> Result<(), ActionError> {
    actions.register(BeginEra)?;
    Ok(())
}

pub(crate) fn era_from_state(state: &WorldState) -> i64 {
    match state
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(ERA))
    {
        Some(Value::Integer(era)) => *era,
        _ => 1,
    }
}

/// Generations of calm this era has had. Also the anchor a rising threat
/// measures itself against, so an era that has just begun is not immediately
/// under pressure.
pub(crate) fn calm_is_over(state: &WorldState) -> Result<bool, ActionError> {
    let generation = integer_component(state, UNIVERSE, GENERATION)?;
    let since = pressure::pressure_generation_from_state(state);
    Ok(generation >= since + CALM_GENERATIONS)
}

/// Drive one period through the stages of the current era, then close the loop
/// if the era has finished.
pub(crate) fn resolve_period(
    world: &mut World,
    actions: &ActionRegistry,
    relationship: EventId,
) -> Result<EventId, Box<dyn Error>> {
    let mut tail = legacy::resolve_stage(world, actions, relationship)?;
    tail = pressure::resolve_stage(world, actions, tail)?;
    tail = succession::resolve_stage(world, actions, tail)?;
    resolve_turnover(world, actions, tail)
}

/// How many past eras ended with the legacy handed over to be rewritten. Read
/// from the World's own event log rather than kept as a counter, so the history
/// has exactly one home.
pub(crate) fn renewed_eras(world: &World) -> i64 {
    world
        .events()
        .iter()
        .filter(|event| event.kind == "legacy_released")
        .count() as i64
}

fn resolve_turnover(
    world: &mut World,
    actions: &ActionRegistry,
    tail: EventId,
) -> Result<EventId, Box<dyn Error>> {
    if succession::succession_id_from_state(world.state()) != "settled" {
        return Ok(tail);
    }
    let state = world.state();
    let seed = seed_id_from_state(state)?;
    let next = next_pressure_kind(
        &seed,
        era_from_state(state) + 1,
        renewed_eras(world),
        &pressure::pressure_kind_from_state(state, &seed),
    );

    let mut request = ActionRequest::new("begin_era")
        .arg(PRESSURE_KIND_ARG, next)
        .caused_by(tail);
    for cause in turnover_causes(world) {
        if cause != tail {
            request = request.caused_by(cause);
        }
    }
    Ok(world.execute(actions, &request)?.id)
}

fn turnover_causes(world: &World) -> Vec<EventId> {
    let mut causes = Vec::new();
    for kind in [
        "legacy_entrusted",
        "legacy_released",
        "era_began",
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

/// The threats a seed can face. The first entry is the one the World has always
/// opened with, so a first era reads exactly as it did before eras existed.
pub(crate) fn pressure_pool(seed: &str) -> &'static [&'static str] {
    match seed {
        "mars-colony" => &["reclaimer", "dust-season", "relay-silence"],
        "1980s-town" => &["lease", "night-bus", "highway-mall"],
        "penguin-civilization" => &["span", "fish-vault", "early-thaw"],
        _ => &["strain"],
    }
}

/// The threat an era faces, chosen from the seed's pool.
///
/// Deterministic by construction: the era number and how many times this World
/// has let its legacy be rewritten pick the entry, and a collision with the
/// threat just survived steps to the next one. Two Worlds with the same history
/// face the same era; nothing here is random.
pub(crate) fn next_pressure_kind(
    seed: &str,
    era: i64,
    renewed: i64,
    just_survived: &str,
) -> &'static str {
    let pool = pressure_pool(seed);
    let span = pool.len() as i64;
    let mut index = (era - 1 + renewed).rem_euclid(span) as usize;
    if pool.len() > 1 && pool[index] == just_survived {
        index = (index + 1) % pool.len();
    }
    pool[index]
}

/// What a World's recent eras add up to. Derived from the event log rather
/// than stored: the endings are already durable Events, and a projection is
/// where reading them belongs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EraStanding {
    /// Not enough eras yet to be a pattern.
    Mixed,
    /// This many eras running have handed their legacy on unchanged.
    Settled(usize),
    /// This many eras running have let their legacy be rewritten.
    Restless(usize),
}

/// How each past era ended, oldest first: `true` where the legacy was let go.
pub(crate) fn era_endings(world: &World) -> Vec<bool> {
    world
        .events()
        .iter()
        .filter_map(|event| match event.kind.as_str() {
            "legacy_entrusted" => Some(false),
            "legacy_released" => Some(true),
            _ => None,
        })
        .collect()
}

pub(crate) fn standing(endings: &[bool]) -> EraStanding {
    let Some(last) = endings.last().copied() else {
        return EraStanding::Mixed;
    };
    let run = endings.iter().rev().take_while(|end| **end == last).count();
    if run < 2 {
        return EraStanding::Mixed;
    }
    if last {
        EraStanding::Restless(run)
    } else {
        EraStanding::Settled(run)
    }
}

struct BeginEra;

impl Action for BeginEra {
    fn name(&self) -> &'static str {
        "begin_era"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let succession = succession::succession_id_from_state(state);
        if succession != "settled" {
            return Err(ActionError::Invalid(format!(
                "this World's succession has not settled, so no era can follow it (succession is {succession})"
            )));
        }
        let seed = seed_id_from_state(state)?;
        let pool = pressure_pool(&seed);
        let kind = match request.args.get(PRESSURE_KIND_ARG) {
            Some(Value::Text(kind)) if pool.contains(&kind.as_str()) => kind.clone(),
            Some(Value::Text(kind)) => {
                return Err(ActionError::Invalid(format!(
                    "{kind} is not a threat this World can face"
                )))
            }
            _ => {
                return Err(ActionError::Invalid(
                    "a new era needs the threat it will face".into(),
                ))
            }
        };
        let surviving = pressure::pressure_kind_from_state(state, &seed);
        if pool.len() > 1 && kind == surviving {
            return Err(ActionError::Invalid(format!(
                "an era must not face the threat the last one just survived ({kind})"
            )));
        }

        let era = era_from_state(state) + 1;
        let generation = integer_component(state, UNIVERSE, GENERATION)?;
        let outcome = succession::succession_outcome_from_state(state);
        // A legacy handed on unchanged carries into the new era. One that was
        // let go has to form again from whatever the successor builds.
        let renewed = outcome == "renewed";
        let summary = opening_summary(&seed, renewed);

        let mut draft = EventDraft::new("era_began");
        draft.targets = vec![UNIVERSE];
        draft.payload.insert("seed".into(), seed.clone().into());
        draft.payload.insert("era".into(), era.into());
        draft
            .payload
            .insert("pressure_kind".into(), kind.clone().into());
        draft.payload.insert(
            "inherited".into(),
            if renewed { "renewed" } else { "continued" }.into(),
        );
        draft.payload.insert("summary".into(), summary.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: ERA.into(),
                value: era.into(),
            },
            // The next era starts before its own threat, with a successor who
            // is now simply the keeper rather than an open question.
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: pressure::PRESSURE.into(),
                value: "none".into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: pressure::PRESSURE_OUTCOME.into(),
                value: "none".into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: pressure::PRESSURE_GENERATION.into(),
                value: generation.into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: pressure::PRESSURE_KIND.into(),
                value: kind.into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: succession::SUCCESSION.into(),
                value: "none".into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: succession::SUCCESSION_PATIENCE.into(),
                value: 0_i64.into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: succession::SUCCESSION_OUTCOME.into(),
                value: "none".into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LAST_CHANGE.into(),
                value: summary.into(),
            },
        ];
        if renewed {
            draft.changes.push(StateChange::SetComponent {
                entity: UNIVERSE,
                key: LEGACY.into(),
                value: "forming".into(),
            });
        }
        Ok(draft)
    }
}

fn opening_summary(seed: &str, renewed: bool) -> &'static str {
    match (seed, renewed) {
        ("mars-colony", false) => {
            "Ares keeps the habits it was handed. A quieter stretch begins, and the ridge is the same ridge."
        }
        ("mars-colony", true) => {
            "Ares starts over on its successor's terms. Nothing about the ridge has changed; everything about how it is worked has."
        }
        ("1980s-town", false) => {
            "Maple Street keeps the nights it knows. A quieter stretch begins, and the arcade opens the same way."
        }
        ("1980s-town", true) => {
            "Maple Street runs on new rules now. The arcade is the same room and a different place."
        }
        ("penguin-civilization", false) => {
            "Icebridge keeps its rites. A quieter stretch begins, and the spans are walked as they always were."
        }
        ("penguin-civilization", true) => {
            "Icebridge walks its spans a new way. The bridge is the same bridge and a different crossing."
        }
        (_, false) => "The World keeps what it was handed, and a quieter stretch begins.",
        (_, true) => "The World starts over on its successor's terms.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_world_never_faces_the_threat_it_just_survived() {
        for seed in ["mars-colony", "1980s-town", "penguin-civilization"] {
            let pool = pressure_pool(seed);
            assert!(pool.len() > 1, "{seed} needs more than one threat");
            let mut surviving = pool[0];
            for era in 2..=12 {
                let next = next_pressure_kind(seed, era, 0, surviving);
                assert!(pool.contains(&next), "{seed} era {era} left the pool");
                assert_ne!(
                    next, surviving,
                    "{seed} era {era} repeats the threat just survived"
                );
                surviving = next;
            }
        }
    }

    #[test]
    fn the_same_history_always_produces_the_same_next_era() {
        for (era, renewed) in [(2, 0), (2, 1), (3, 0), (7, 4)] {
            let first = next_pressure_kind("mars-colony", era, renewed, "reclaimer");
            let again = next_pressure_kind("mars-colony", era, renewed, "reclaimer");
            assert_eq!(first, again);
        }
    }

    #[test]
    fn how_the_world_has_lived_changes_which_trouble_comes_next() {
        // This is the property a two-trouble pool could not have: with only two,
        // the no-repeat rule fully determined the next trouble from the one just
        // survived, and history had no room to act. With three it does.
        let a_settled_world = next_pressure_kind("mars-colony", 3, 0, "dust-season");
        let a_restless_world = next_pressure_kind("mars-colony", 3, 1, "dust-season");
        assert_ne!(
            a_settled_world, a_restless_world,
            "two Worlds at the same era with different histories met the same trouble"
        );

        for seed in ["mars-colony", "1980s-town", "penguin-civilization"] {
            let differing = (2..12)
                .flat_map(|era| {
                    pressure_pool(seed)
                        .iter()
                        .map(move |survived| (era, *survived))
                })
                .filter(|(era, survived)| {
                    next_pressure_kind(seed, *era, 0, survived)
                        != next_pressure_kind(seed, *era, 1, survived)
                })
                .count();
            assert!(
                differing > 0,
                "{seed}: letting the legacy go never changes what comes next"
            );
        }
    }

    #[test]
    fn a_run_of_the_same_ending_becomes_a_pattern_and_a_change_breaks_it() {
        assert_eq!(standing(&[]), EraStanding::Mixed);
        assert_eq!(standing(&[false]), EraStanding::Mixed);
        assert_eq!(standing(&[false, false]), EraStanding::Settled(2));
        assert_eq!(
            standing(&[true, false, false, false]),
            EraStanding::Settled(3)
        );
        assert_eq!(standing(&[true, true]), EraStanding::Restless(2));
        assert_eq!(standing(&[false, false, true]), EraStanding::Mixed);
        assert_eq!(
            standing(&[false, false, true, true]),
            EraStanding::Restless(2)
        );
    }

    #[test]
    fn an_unknown_seed_still_has_a_threat_to_face() {
        let pool = pressure_pool("something-else");
        assert_eq!(pool.len(), 1);
        assert_eq!(
            next_pressure_kind("something-else", 4, 2, "strain"),
            "strain"
        );
    }

    #[test]
    fn every_seed_opens_its_eras_differently_depending_on_what_it_inherited() {
        for seed in ["mars-colony", "1980s-town", "penguin-civilization"] {
            let continued = opening_summary(seed, false);
            let renewed = opening_summary(seed, true);
            assert!(!continued.is_empty());
            assert_ne!(continued, renewed, "{seed} reads the same either way");
        }
    }
}
