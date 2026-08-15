from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count} for {old[:80]!r}")
    file.write_text(text.replace(old, new, 1))


# Pocket Universe release + durable state keys.
replace_once(
    "worlds/pocket-universe/src/lib.rs",
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.13.2";',
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.14.0";',
)
replace_once(
    "worlds/pocket-universe/src/lib.rs",
    'pub(crate) const LEGACY_SUMMARY: &str = "legacy_summary";\n',
    'pub(crate) const LEGACY_SUMMARY: &str = "legacy_summary";\n'
    'pub(crate) const LEGACY_BEHAVIOR: &str = "legacy_behavior";\n'
    'pub(crate) const LEGACY_CYCLES: &str = "legacy_cycles";\n',
)
replace_once(
    "worlds/pocket-universe/src/lib.rs",
    '            .with_component(LEGACY, "forming")\n'
    '            .with_component(LEGACY_SUMMARY, "")\n'
    '            .with_component(LAST_CHANGE, "Nothing exists here yet."),',
    '            .with_component(LEGACY, "forming")\n'
    '            .with_component(LEGACY_SUMMARY, "")\n'
    '            .with_component(LEGACY_BEHAVIOR, "forming")\n'
    '            .with_component(LEGACY_CYCLES, 0_i64)\n'
    '            .with_component(LAST_CHANGE, "Nothing exists here yet."),',
)
replace_once(
    "worlds/pocket-universe/src/lib.rs",
    '        StateChange::SetComponent {\n'
    '            entity: UNIVERSE,\n'
    '            key: LEGACY_SUMMARY.into(),\n'
    '            value: "".into(),\n'
    '        },\n'
    '        StateChange::SetComponent {\n'
    '            entity: UNIVERSE,\n'
    '            key: LAST_CHANGE.into(),',
    '        StateChange::SetComponent {\n'
    '            entity: UNIVERSE,\n'
    '            key: LEGACY_SUMMARY.into(),\n'
    '            value: "".into(),\n'
    '        },\n'
    '        StateChange::SetComponent {\n'
    '            entity: UNIVERSE,\n'
    '            key: LEGACY_BEHAVIOR.into(),\n'
    '            value: "forming".into(),\n'
    '        },\n'
    '        StateChange::SetComponent {\n'
    '            entity: UNIVERSE,\n'
    '            key: LEGACY_CYCLES.into(),\n'
    '            value: 0_i64.into(),\n'
    '        },\n'
    '        StateChange::SetComponent {\n'
    '            entity: UNIVERSE,\n'
    '            key: LAST_CHANGE.into(),',
)

# Legacy formation becomes a durable feedback loop from the following period onward.
replace_once(
    "worlds/pocket-universe/src/legacy.rs",
    'const LEGACY_STATUS: &str = "legacy_status";\n',
    'const LEGACY_STATUS: &str = "legacy_status";\nconst LEGACY_PATTERN: &str = "legacy_pattern";\n',
)
replace_once(
    "worlds/pocket-universe/src/legacy.rs",
    'pub(crate) fn register_actions(actions: &mut ActionRegistry) -> Result<(), ActionError> {\n'
    '    actions.register(ResolveLegacy)?;\n'
    '    Ok(())\n'
    '}\n',
    'pub(crate) fn register_actions(actions: &mut ActionRegistry) -> Result<(), ActionError> {\n'
    '    actions.register(ResolveLegacy)?;\n'
    '    actions.register(ReinforceLegacy)?;\n'
    '    Ok(())\n'
    '}\n',
)
replace_once(
    "worlds/pocket-universe/src/legacy.rs",
    '''pub(crate) fn resolve_period_consequences(
    world: &mut World,
    actions: &ActionRegistry,
    relationship: EventId,
) -> Result<EventId, Box<dyn Error>> {
    let mut tail = relationship;
    if social_arc_candidate(world.state())?.is_some() {
        tail = world
            .execute(
                actions,
                &ActionRequest::new("resolve_social_arc").caused_by(tail),
            )?
            .id;
    }
    if legacy_candidate(world.state())?.is_some() {
        let mut request = ActionRequest::new("resolve_legacy").caused_by(tail);
        for cause in historical_causes(world) {
            if cause != tail {
                request = request.caused_by(cause);
            }
        }
        tail = world.execute(actions, &request)?.id;
    }
    Ok(tail)
}
''',
    '''pub(crate) fn resolve_period_consequences(
    world: &mut World,
    actions: &ActionRegistry,
    relationship: EventId,
) -> Result<EventId, Box<dyn Error>> {
    let legacy_existed = legacy_id_from_state(world.state())? != "forming";
    let mut tail = relationship;
    if social_arc_candidate(world.state())?.is_some() {
        tail = world
            .execute(
                actions,
                &ActionRequest::new("resolve_social_arc").caused_by(tail),
            )?
            .id;
    }
    if legacy_candidate(world.state())?.is_some() {
        let mut request = ActionRequest::new("resolve_legacy").caused_by(tail);
        for cause in historical_causes(world) {
            if cause != tail {
                request = request.caused_by(cause);
            }
        }
        tail = world.execute(actions, &request)?.id;
    }
    if legacy_existed {
        let formed = world
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "world_legacy_formed")
            .map(|event| event.id)
            .ok_or_else(|| {
                ActionError::Invalid(
                    "Pocket Universe has a durable legacy without its formation event".into(),
                )
            })?;
        let mut request = ActionRequest::new("reinforce_legacy").caused_by(tail);
        if formed != tail {
            request = request.caused_by(formed);
        }
        tail = world.execute(actions, &request)?.id;
    }
    Ok(tail)
}
''',
)
replace_once(
    "worlds/pocket-universe/src/legacy.rs",
    '''fn historical_causes(world: &World) -> Vec<EventId> {
''',
    '''#[derive(Clone, Debug, Eq, PartialEq)]
struct ReinforcementCandidate {
    legacy: String,
    behavior: String,
    target: EntityId,
    cycle: i64,
    pattern: String,
    summary: String,
}

fn reinforcement_candidate(
    state: &WorldState,
) -> Result<Option<ReinforcementCandidate>, ActionError> {
    let legacy = legacy_id_from_state(state)?;
    if legacy == "forming" {
        return Ok(None);
    }
    let behavior = text_component_from_state(state, UNIVERSE, LEGACY_BEHAVIOR)?;
    if !matches!(behavior.as_str(), "care-led" | "explore-led" | "balanced") {
        return Err(ActionError::Invalid(format!(
            "unknown Pocket Universe legacy behavior: {behavior}"
        )));
    }
    let seed = seed_id_from_state(state)?;
    let posture = posture_id_from_state(state)?;
    let social_arc = text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC)?;
    let (expected_legacy, target, _, _) = archetype(&seed, &posture, &social_arc).ok_or_else(|| {
        ActionError::Invalid(format!(
            "unsupported Pocket Universe reinforcement: seed={seed}, posture={posture}, social_arc={social_arc}"
        ))
    })?;
    if expected_legacy != legacy {
        return Err(ActionError::Invalid(format!(
            "Pocket Universe legacy state disagrees with its durable causes: expected={expected_legacy}, actual={legacy}"
        )));
    }
    let cycle = integer_component(state, UNIVERSE, LEGACY_CYCLES)?
        .checked_add(1)
        .ok_or_else(|| ActionError::Invalid("Pocket Universe legacy cycle overflow".into()))?;
    let (pattern, summary) = reinforcement_semantics(&legacy, &behavior, cycle)?;
    Ok(Some(ReinforcementCandidate {
        legacy,
        behavior,
        target,
        cycle,
        pattern,
        summary,
    }))
}

fn reinforcement_semantics(
    legacy: &str,
    behavior: &str,
    cycle: i64,
) -> Result<(String, String), ActionError> {
    let label = match legacy {
        "ridge-network" => "ridge network",
        "competing-frontiers" => "competing frontiers",
        "habitat-commons" => "habitat commons",
        "sealed-districts" => "sealed districts",
        "night-network" => "night network",
        "rival-scenes" => "rival scenes",
        "neighborhood-commons" => "neighborhood commons",
        "split-blocks" => "split blocks",
        "aurora-league" => "aurora league",
        "rival-routes" => "rival routes",
        "winter-commons" => "winter commons",
        "divided-houses" => "divided houses",
        other => {
            return Err(ActionError::Invalid(format!(
                "unknown Pocket Universe legacy: {other}"
            )))
        }
    };
    let (pattern_kind, behavior_phrase) = match behavior {
        "care-led" => ("stewardship", "repeated care, upkeep, and stewardship"),
        "explore-led" => ("expansion", "repeated exploration, route expansion, and experimentation"),
        "balanced" => ("adaptive", "coordinated upkeep and expansion"),
        other => {
            return Err(ActionError::Invalid(format!(
                "unknown Pocket Universe legacy behavior: {other}"
            )))
        }
    };
    let pattern = format!("{pattern_kind} cycle {cycle}");
    let summary = format!(
        "The {label} reinforced itself through {behavior_phrase}. Legacy cycle {cycle} is now a durable {pattern_kind} pattern."
    );
    Ok((pattern, summary))
}

fn historical_causes(world: &World) -> Vec<EventId> {
''',
)
replace_once(
    "worlds/pocket-universe/src/legacy.rs",
    '''            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LEGACY_SUMMARY.into(),
                value: candidate.summary.clone().into(),
            },
            StateChange::SetComponent {
                entity: candidate.target,
                key: LEGACY_STATUS.into(),
                value: candidate.status_value.into(),
            },
''',
    '''            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LEGACY_SUMMARY.into(),
                value: candidate.summary.clone().into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LEGACY_BEHAVIOR.into(),
                value: candidate.behavior.into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LEGACY_CYCLES.into(),
                value: 0_i64.into(),
            },
            StateChange::SetComponent {
                entity: candidate.target,
                key: LEGACY_STATUS.into(),
                value: candidate.status_value.into(),
            },
''',
)
replace_once(
    "worlds/pocket-universe/src/legacy.rs",
    '''impl Action for ResolveLegacy {
''',
    '''struct ReinforceLegacy;

impl Action for ReinforceLegacy {
    fn name(&self) -> &'static str {
        "reinforce_legacy"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let candidate = reinforcement_candidate(state)?.ok_or_else(|| {
            ActionError::Invalid("this World does not have a durable legacy to reinforce".into())
        })?;
        let mut draft = EventDraft::new("legacy_reinforced");
        draft.targets = vec![UNIVERSE, candidate.target];
        draft
            .payload
            .insert("legacy".into(), candidate.legacy.clone().into());
        draft
            .payload
            .insert("behavior".into(), candidate.behavior.clone().into());
        draft.payload.insert("cycle".into(), candidate.cycle.into());
        draft
            .payload
            .insert("pattern".into(), candidate.pattern.clone().into());
        draft
            .payload
            .insert("summary".into(), candidate.summary.clone().into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LEGACY_CYCLES.into(),
                value: candidate.cycle.into(),
            },
            StateChange::SetComponent {
                entity: candidate.target,
                key: LEGACY_PATTERN.into(),
                value: candidate.pattern.into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LAST_CHANGE.into(),
                value: candidate.summary.into(),
            },
        ];
        Ok(draft)
    }
}

impl Action for ResolveLegacy {
''',
)
# Add focused unit proof that behavior is state-shaping, not merely descriptive payload.
legacy_path = Path("worlds/pocket-universe/src/legacy.rs")
legacy_text = legacy_path.read_text()
legacy_text += '''\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn repeated_behavior_produces_distinct_durable_reinforcement_patterns() {\n        let care = reinforcement_semantics("ridge-network", "care-led", 1).unwrap();\n        let explore = reinforcement_semantics("ridge-network", "explore-led", 1).unwrap();\n        let balanced = reinforcement_semantics("ridge-network", "balanced", 1).unwrap();\n\n        assert_eq!(care.0, "stewardship cycle 1");\n        assert_eq!(explore.0, "expansion cycle 1");\n        assert_eq!(balanced.0, "adaptive cycle 1");\n        assert_ne!(care, explore);\n        assert_ne!(care, balanced);\n        assert_ne!(explore, balanced);\n        assert!(care.1.contains("repeated care"));\n        assert!(explore.1.contains("repeated exploration"));\n        assert!(balanced.1.contains("coordinated upkeep and expansion"));\n    }\n}\n'''
legacy_path.write_text(legacy_text)

# Extend emergent Legacy regression through the first autonomous feedback cycle.
replace_once(
    "worlds/pocket-universe/tests/emergent_legacy.rs",
    'use world_compare::{compare_snapshots, DifferenceKind, EntityDifference};\nuse world_persistence::ArchivedValue;\n',
    'use world_compare::{compare_snapshots, DifferenceKind, EntityDifference};\nuse world_core::{EntityId, Value};\nuse world_persistence::ArchivedValue;\n',
)
replace_once(
    "worlds/pocket-universe/tests/emergent_legacy.rs",
    '''    assert!(summary.contains("signal expedition"));
    assert!(summary.contains("care /"));
    assert!(summary.contains("explore"));

    let cause_kinds = legacy
''',
    '''    assert!(summary.contains("signal expedition"));
    assert!(summary.contains("care /"));
    assert!(summary.contains("explore"));
    assert_eq!(
        legacy.payload.get("behavior"),
        Some(&ArchivedValue::Text("balanced".into()))
    );
    let universe_state = universe
        .world()
        .state()
        .entity(EntityId::new(1))
        .expect("Pocket Universe state should contain its World entity");
    assert_eq!(
        universe_state.component("legacy_behavior"),
        Some(&Value::Text("balanced".into()))
    );
    assert_eq!(
        universe_state.component("legacy_cycles"),
        Some(&Value::Integer(0))
    );
    assert!(!archive
        .events
        .iter()
        .any(|event| event.kind == "legacy_reinforced"));

    let cause_kinds = legacy
''',
)
replace_once(
    "worlds/pocket-universe/tests/emergent_legacy.rs",
    '''    reopened.invoke_projection_command(NUDGE_COMMAND)?;
    let after = reopened.archive()?;
    let growth = after
''',
    '''    reopened.invoke_projection_command(NUDGE_COMMAND)?;
    let after = reopened.archive()?;
    let reinforced = after
        .events
        .iter()
        .rev()
        .find(|event| event.kind == "legacy_reinforced")
        .expect("the first period after formation should reinforce the durable legacy");
    assert_eq!(
        reinforced.payload.get("cycle"),
        Some(&ArchivedValue::Integer(1))
    );
    assert_eq!(
        reinforced.payload.get("pattern"),
        Some(&ArchivedValue::Text("adaptive cycle 1".into()))
    );
    let reinforcement_causes = reinforced
        .caused_by
        .iter()
        .filter_map(|id| after.events.iter().find(|event| event.id == *id))
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();
    assert!(reinforcement_causes.contains(&"relationship_shifted"));
    assert!(reinforcement_causes.contains(&"world_legacy_formed"));
    let universe_state = reopened
        .world()
        .state()
        .entity(EntityId::new(1))
        .expect("reopened World should keep its durable state");
    assert_eq!(
        universe_state.component("legacy_cycles"),
        Some(&Value::Integer(1))
    );
    let ridge = reopened
        .world()
        .state()
        .entity(EntityId::new(13))
        .expect("ridge-network legacy target should exist");
    assert_eq!(
        ridge.component("legacy_pattern"),
        Some(&Value::Text("adaptive cycle 1".into()))
    );

    let growth = after
''',
)

# Why must connect current reinforcement both to this period and the original formation event.
replace_once(
    "worlds/pocket-universe/tests/legacy_why.rs",
    '''    let reopened = PocketUniverse::resume_archive(&archive)?;
    let reopened_snapshot = reopened.projection_snapshot();
''',
    '''    let mut reopened = PocketUniverse::resume_archive(&archive)?;
    let reopened_snapshot = reopened.projection_snapshot();
''',
)
replace_once(
    "worlds/pocket-universe/tests/legacy_why.rs",
    '''    assert_eq!(
        reopened_snapshot.why(legacy_event_id),
        Some(why),
        "archive/reopen should preserve the same causal explanation"
    );

    Ok(())
}
''',
    '''    assert_eq!(
        reopened_snapshot.why(legacy_event_id),
        Some(why),
        "archive/reopen should preserve the same causal explanation"
    );

    reopened.advance_periods(1)?;
    let reinforced_archive = reopened.archive()?;
    let reinforced = reinforced_archive
        .events
        .iter()
        .rev()
        .find(|event| event.kind == "legacy_reinforced")
        .expect("the following period should reinforce the legacy");
    let reinforced_event_id = EventId::new(reinforced.id);
    let reinforced_snapshot = reopened.projection_snapshot();
    let reinforced_why = reinforced_snapshot
        .why(reinforced_event_id)
        .expect("legacy reinforcement should have a generic Why projection");
    assert_eq!(reinforced_why.nodes[0].title, "Legacy Reinforced");
    let reinforced_titles = reinforced_why
        .nodes
        .iter()
        .map(|node| node.title.as_str())
        .collect::<Vec<_>>();
    assert!(reinforced_titles.contains(&"World Legacy Formed"));
    assert!(reinforced_titles.contains(&"Relationship Shifted"));

    let reopened_again = PocketUniverse::resume_archive(&reinforced_archive)?;
    assert_eq!(
        reopened_again
            .projection_snapshot()
            .why(reinforced_event_id),
        Some(reinforced_why),
        "archive/reopen should preserve the reinforcement explanation"
    );

    Ok(())
}
''',
)

# Release version synchronization.
replace_once(
    "worlds/pocket-universe/Cargo.toml",
    'version = "0.13.2"',
    'version = "0.14.0"',
)
replace_once(
    "apps/pocket-universe-pack/Cargo.toml",
    'version = "0.13.2"',
    'version = "0.14.0"',
)
replace_once(
    "apps/world-machine-desktop/src/included_packs.rs",
    'version: "0.13.2",',
    'version: "0.14.0",',
)
replace_once(
    "apps/world-machine-desktop/src/included_packs.rs",
    'assert_eq!(packs[0].pack.version, "0.13.2");',
    'assert_eq!(packs[0].pack.version, "0.14.0");',
)

print("M120 patch applied")
