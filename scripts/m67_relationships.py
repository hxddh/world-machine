from pathlib import Path

lib = Path('worlds/pocket-universe/src/lib.rs')
s = lib.read_text()

s = s.replace('pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.6.0";', 'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.7.0";', 1)
s = s.replace(
    'pub const CAREFUL_PATH_COMMAND: &str = "pocket-universe.choose-careful-path";\n',
    'pub const CAREFUL_PATH_COMMAND: &str = "pocket-universe.choose-careful-path";\n'
    'pub const SHARED_PROJECT_COMMAND: &str = "pocket-universe.relationship-shared-project";\n'
    'pub const RIVALRY_COMMAND: &str = "pocket-universe.relationship-rivalry";\n',
    1,
)
s = s.replace('pub(crate) const SLOT_E: EntityId = EntityId::new(14);\n', 'pub(crate) const SLOT_E: EntityId = EntityId::new(14);\npub(crate) const RELATIONSHIP: EntityId = EntityId::new(15);\n', 1)
s = s.replace(
    'pub(crate) const DECISION: &str = "decision";\n',
    'pub(crate) const DECISION: &str = "decision";\n'
    'pub(crate) const RELATIONSHIP_DIRECTION: &str = "direction";\n'
    'const RELATIONSHIP_TRUST: &str = "trust";\n'
    'const RELATIONSHIP_TENSION: &str = "tension";\n'
    'const RELATIONSHIP_LAST_DYNAMIC: &str = "last_dynamic";\n',
    1,
)

# Nudge: commit the relationship update after both agent outcomes.
old_nudge = '''            let secondary_outcome = Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &self.mind_profile,\n                SLOT_E,\n                &[primary_outcome],\n            )?;\n            self.world = candidate;\n            return Ok(secondary_outcome);\n'''
new_nudge = '''            let secondary_outcome = Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &self.mind_profile,\n                SLOT_E,\n                &[primary_outcome],\n            )?;\n            let relationship = candidate\n                .execute(\n                    &self.actions,\n                    &ActionRequest::new("update_relationship")\n                        .caused_by(primary_outcome)\n                        .caused_by(secondary_outcome),\n                )?\n                .id;\n            self.world = candidate;\n            return Ok(relationship);\n'''
if old_nudge not in s:
    raise SystemExit('M66 nudge block not found')
s = s.replace(old_nudge, new_nudge, 1)

# Projection command mapping.
s = s.replace(
    '''            BOLD_PATH_COMMAND => "choose_bold_path",\n            CAREFUL_PATH_COMMAND => "choose_careful_path",\n''',
    '''            BOLD_PATH_COMMAND => "choose_bold_path",\n            CAREFUL_PATH_COMMAND => "choose_careful_path",\n            SHARED_PROJECT_COMMAND => "steer_shared_project",\n            RIVALRY_COMMAND => "steer_rivalry",\n''',
    1,
)

# Advance: after secondary response, update relationship in the same candidate transaction.
old_advance = '''            Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &self.mind_profile,\n                SLOT_E,\n                &[primary_outcome],\n            )?;\n'''
new_advance = '''            let secondary_outcome = Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &self.mind_profile,\n                SLOT_E,\n                &[primary_outcome],\n            )?;\n            candidate.execute(\n                &self.actions,\n                &ActionRequest::new("update_relationship")\n                    .caused_by(primary_outcome)\n                    .caused_by(secondary_outcome),\n            )?;\n'''
if old_advance not in s:
    raise SystemExit('M66 advance secondary block not found')
s = s.replace(old_advance, new_advance, 1)

# Register relationship actions.
s = s.replace(
    '''    actions.register(CareForWorld)?;\n    actions.register(ExploreWorld)?;\n    Ok(actions)\n}\n''',
    '''    actions.register(CareForWorld)?;\n    actions.register(ExploreWorld)?;\n    actions.register(UpdateRelationship)?;\n    actions.register(SteerSharedProject)?;\n    actions.register(SteerRivalry)?;\n    Ok(actions)\n}\n''',
    1,
)
s = s.replace(
    '''struct CareForWorld;\nstruct ExploreWorld;\n''',
    '''struct CareForWorld;\nstruct ExploreWorld;\nstruct UpdateRelationship;\nstruct SteerSharedProject;\nstruct SteerRivalry;\n''',
    1,
)

# Add relationship entity to every seed.
seed_replacements = [
    (
        '''                Entity::new(SLOT_E, "person")\n                    .with_component("name", "Tomas Vale")\n                    .with_component("role", "rover scout")\n                    .with_component(AGENT_CARE_COUNT, 0_i64)\n                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)\n                    .with_component(LAST_MIND_PROFILE, "none"),\n''',
        '''                Entity::new(SLOT_E, "person")\n                    .with_component("name", "Tomas Vale")\n                    .with_component("role", "rover scout")\n                    .with_component(AGENT_CARE_COUNT, 0_i64)\n                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)\n                    .with_component(LAST_MIND_PROFILE, "none"),\n                relationship_entity("Nia ↔ Tomas"),\n''',
    ),
    (
        '''                Entity::new(SLOT_E, "person")\n                    .with_component("name", "Max Park")\n                    .with_component("role", "radio volunteer")\n                    .with_component(AGENT_CARE_COUNT, 0_i64)\n                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)\n                    .with_component(LAST_MIND_PROFILE, "none"),\n''',
        '''                Entity::new(SLOT_E, "person")\n                    .with_component("name", "Max Park")\n                    .with_component("role", "radio volunteer")\n                    .with_component(AGENT_CARE_COUNT, 0_i64)\n                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)\n                    .with_component(LAST_MIND_PROFILE, "none"),\n                relationship_entity("Lena ↔ Max"),\n''',
    ),
    (
        '''                Entity::new(SLOT_E, "penguin")\n                    .with_component("name", "Miri")\n                    .with_component("role", "fish-vault keeper")\n                    .with_component(AGENT_CARE_COUNT, 0_i64)\n                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)\n                    .with_component(LAST_MIND_PROFILE, "none"),\n''',
        '''                Entity::new(SLOT_E, "penguin")\n                    .with_component("name", "Miri")\n                    .with_component("role", "fish-vault keeper")\n                    .with_component(AGENT_CARE_COUNT, 0_i64)\n                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)\n                    .with_component(LAST_MIND_PROFILE, "none"),\n                relationship_entity("Piko ↔ Miri"),\n''',
    ),
]
for old, new in seed_replacements:
    if old not in s:
        raise SystemExit('seed actor block not found')
    s = s.replace(old, new, 1)
s = s.replace('    entities: [Entity; 5],\n', '    entities: [Entity; 6],\n', 1)

# Insert relationship action implementations before ChooseBoldPath.
marker = 'impl Action for ChooseBoldPath {\n'
relationship_actions = r'''impl Action for UpdateRelationship {
    fn name(&self) -> &'static str {
        "update_relationship"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let primary = text_component_from_state(state, SLOT_B, "last_intent")?;
        let secondary = text_component_from_state(state, SLOT_E, "last_intent")?;
        let direction = text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_DIRECTION)?;
        let trust = integer_component(state, RELATIONSHIP, RELATIONSHIP_TRUST)?;
        let tension = integer_component(state, RELATIONSHIP, RELATIONSHIP_TENSION)?;

        let (mut trust_delta, mut tension_delta, dynamic) = match (primary.as_str(), secondary.as_str()) {
            ("care", "care") => (2, -1, "They reinforced the same fragile thing together."),
            ("explore", "explore") => (-1, 2, "They chased the same frontier and began to compete for it."),
            ("care", "explore") | ("explore", "care") => {
                (1, -1, "Their different instincts covered each other's blind spots.")
            }
            _ => return Err(ActionError::Invalid("relationship update requires both actors to have acted".into())),
        };
        match direction.as_str() {
            "shared-project" => {
                trust_delta += 1;
                tension_delta -= 1;
            }
            "rivalry" => {
                tension_delta += 1;
            }
            "none" => {}
            other => return Err(ActionError::Invalid(format!("unknown relationship direction: {other}"))),
        }

        let next_trust = (trust + trust_delta).clamp(0, 10);
        let next_tension = (tension + tension_delta).clamp(0, 10);
        let summary = format!(
            "{dynamic} Trust is {next_trust}; tension is {next_tension}."
        );
        let mut draft = EventDraft::new("relationship_shifted");
        draft.targets = vec![RELATIONSHIP, SLOT_B, SLOT_E];
        draft.payload.insert("summary".into(), summary.clone().into());
        draft.payload.insert("trust".into(), next_trust.into());
        draft.payload.insert("tension".into(), next_tension.into());
        draft.payload.insert("direction".into(), direction.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: RELATIONSHIP_TRUST.into(),
                value: next_trust.into(),
            },
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: RELATIONSHIP_TENSION.into(),
                value: next_tension.into(),
            },
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: RELATIONSHIP_LAST_DYNAMIC.into(),
                value: summary.clone().into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LAST_CHANGE.into(),
                value: summary.into(),
            },
        ];
        Ok(draft)
    }
}

impl Action for SteerSharedProject {
    fn name(&self) -> &'static str {
        "steer_shared_project"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        steer_relationship_draft(state, "shared-project")
    }
}

impl Action for SteerRivalry {
    fn name(&self) -> &'static str {
        "steer_rivalry"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        steer_relationship_draft(state, "rivalry")
    }
}

fn steer_relationship_draft(
    state: &WorldState,
    direction: &str,
) -> Result<EventDraft, ActionError> {
    if integer_component(state, UNIVERSE, GENERATION)? < 2 {
        return Err(ActionError::Invalid(
            "the relationship has not developed enough to steer yet".into(),
        ));
    }
    if text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_DIRECTION)? != "none" {
        return Err(ActionError::Invalid(
            "this relationship already has a chosen direction".into(),
        ));
    }
    let trust = integer_component(state, RELATIONSHIP, RELATIONSHIP_TRUST)?;
    let tension = integer_component(state, RELATIONSHIP, RELATIONSHIP_TENSION)?;
    let (next_trust, next_tension, summary) = match direction {
        "shared-project" => (
            (trust + 2).clamp(0, 10),
            (tension - 1).clamp(0, 10),
            "You gave them something neither could finish alone. Their relationship now leans toward a shared project.",
        ),
        "rivalry" => (
            trust,
            (tension + 2).clamp(0, 10),
            "You let competition sharpen the space between them. Their relationship now leans toward rivalry.",
        ),
        _ => return Err(ActionError::Invalid("unknown relationship direction".into())),
    };
    let mut draft = EventDraft::new("relationship_steered");
    draft.targets = vec![RELATIONSHIP, SLOT_B, SLOT_E];
    draft.payload.insert("direction".into(), direction.into());
    draft.payload.insert("summary".into(), summary.into());
    draft.changes = vec![
        StateChange::SetComponent {
            entity: RELATIONSHIP,
            key: RELATIONSHIP_DIRECTION.into(),
            value: direction.into(),
        },
        StateChange::SetComponent {
            entity: RELATIONSHIP,
            key: RELATIONSHIP_TRUST.into(),
            value: next_trust.into(),
        },
        StateChange::SetComponent {
            entity: RELATIONSHIP,
            key: RELATIONSHIP_TENSION.into(),
            value: next_tension.into(),
        },
        StateChange::SetComponent {
            entity: RELATIONSHIP,
            key: RELATIONSHIP_LAST_DYNAMIC.into(),
            value: summary.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),
            value: summary.into(),
        },
    ];
    Ok(draft)
}

'''
if marker not in s:
    raise SystemExit('ChooseBoldPath marker missing')
s = s.replace(marker, relationship_actions + marker, 1)

# Add relationship entity builder and state text helper.
marker2 = 'fn seed_draft(\n'
helper = '''fn relationship_entity(name: &str) -> Entity {\n    Entity::new(RELATIONSHIP, "relationship")\n        .with_component("name", name)\n        .with_component("primary", Value::Entity(SLOT_B))\n        .with_component("secondary", Value::Entity(SLOT_E))\n        .with_component(RELATIONSHIP_TRUST, 0_i64)\n        .with_component(RELATIONSHIP_TENSION, 0_i64)\n        .with_component(RELATIONSHIP_DIRECTION, "none")\n        .with_component(RELATIONSHIP_LAST_DYNAMIC, "forming")\n}\n\n'''
if marker2 not in s:
    raise SystemExit('seed_draft marker missing')
s = s.replace(marker2, helper + marker2, 1)

marker3 = 'fn integer_component(state: &WorldState, entity: EntityId, key: &str) -> Result<i64, ActionError> {\n'
text_helper = '''fn text_component_from_state(\n    state: &WorldState,\n    entity: EntityId,\n    key: &str,\n) -> Result<String, ActionError> {\n    match state.entity(entity).and_then(|entity| entity.component(key)) {\n        Some(Value::Text(value)) => Ok(value.clone()),\n        _ => Err(ActionError::Invalid(format!(\n            "entity {entity} has no text component {key}"\n        ))),\n    }\n}\n\n'''
if marker3 not in s:
    raise SystemExit('integer component marker missing')
s = s.replace(marker3, text_helper + marker3, 1)

# Tests: add relationship dynamics and steering/fork/compare regressions before generation-three test.
test_marker = '''    #[test]\n    fn generation_three_exposes_a_durable_intervention() {\n'''
new_tests = r'''    #[test]
    fn complementary_deterministic_agents_build_trust() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(2).unwrap();

        let relationship = universe.world().state().entity(RELATIONSHIP).unwrap();
        assert_eq!(relationship.component(RELATIONSHIP_TRUST), Some(&Value::Integer(2)));
        assert_eq!(relationship.component(RELATIONSHIP_TENSION), Some(&Value::Integer(0)));
        assert_eq!(relationship.component(RELATIONSHIP_DIRECTION), Some(&Value::Text("none".into())));
        assert_eq!(
            universe
                .world()
                .events()
                .iter()
                .filter(|event| event.kind == "relationship_shifted")
                .count(),
            2
        );
    }

    #[test]
    fn same_explore_choices_raise_tension_and_keep_full_causal_why() {
        let mut universe = PocketUniverse::with_agent_runtime(MockAgentRuntime::scripted([
            AGENT_EXPLORE_ACTION,
            AGENT_EXPLORE_ACTION,
        ]))
        .unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(1).unwrap();

        let relationship = universe.world().state().entity(RELATIONSHIP).unwrap();
        assert_eq!(relationship.component(RELATIONSHIP_TRUST), Some(&Value::Integer(0)));
        assert_eq!(relationship.component(RELATIONSHIP_TENSION), Some(&Value::Integer(2)));
        let shifted = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "relationship_shifted")
            .unwrap();
        assert_eq!(shifted.caused_by.len(), 2);
        let why = universe.projection_snapshot().why;
        let chain = why.get(&shifted.id).unwrap();
        assert!(chain.nodes.iter().any(|node| node.kind == "agent_explored_world"));
        assert!(chain.nodes.iter().any(|node| node.kind == "universe_grew"));
    }

    #[test]
    fn relationship_direction_is_durable_compareable_and_forkable() {
        use world_compare::{compare_snapshots, DifferenceKind};

        let mut shared = PocketUniverse::new().unwrap();
        let mut rivalry = PocketUniverse::new().unwrap();
        shared
            .invoke_projection_command(SEED_1980S_TOWN_COMMAND)
            .unwrap();
        rivalry
            .invoke_projection_command(SEED_1980S_TOWN_COMMAND)
            .unwrap();
        shared.advance_periods(2).unwrap();
        rivalry.advance_periods(2).unwrap();

        let before_choice = shared.archive().unwrap();
        let shared_snapshot = shared
            .invoke_projection_command(SHARED_PROJECT_COMMAND)
            .and_then(|_| Ok(shared.projection_snapshot()))
            .unwrap();
        let rivalry_snapshot = rivalry
            .invoke_projection_command(RIVALRY_COMMAND)
            .and_then(|_| Ok(rivalry.projection_snapshot()))
            .unwrap();

        let comparison = compare_snapshots(&shared_snapshot, &rivalry_snapshot);
        let relationship = comparison
            .entities
            .iter()
            .find(|difference| difference.id == world_projection::SelectionId::Entity(RELATIONSHIP))
            .unwrap();
        assert_eq!(relationship.kind, DifferenceKind::Changed);
        assert!(relationship.inspector_rows.iter().any(|row| {
            row.key.label == "Direction"
                && row.left.as_deref() == Some("shared-project")
                && row.right.as_deref() == Some("rivalry")
        }));

        let steer_event = shared
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "relationship_steered")
            .unwrap()
            .id;
        shared.fork_before_event(steer_event).unwrap();
        assert_eq!(shared.archive().unwrap(), before_choice);
        let commands = shared.projection_snapshot().commands;
        assert!(commands.iter().any(|command| command.id == SHARED_PROJECT_COMMAND));
        assert!(commands.iter().any(|command| command.id == RIVALRY_COMMAND));
    }

'''
if test_marker not in s:
    raise SystemExit('generation-three test marker missing')
s = s.replace(test_marker, new_tests + test_marker, 1)

lib.write_text(s)

# Projection: relationship commands + event titles + sixth canvas position.
projection = Path('worlds/pocket-universe/src/projection.rs')
p = projection.read_text()
p = p.replace(
    '''    seed_id, BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND, DECISION, GENERATION, LAST_CHANGE,\n    NUDGE_COMMAND, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,\n    SEED_PENGUIN_CIVILIZATION_COMMAND, UNIVERSE,\n''',
    '''    seed_id, BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND, DECISION, GENERATION, LAST_CHANGE,\n    NUDGE_COMMAND, RELATIONSHIP, RELATIONSHIP_DIRECTION, RIVALRY_COMMAND,\n    SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND, SEED_PENGUIN_CIVILIZATION_COMMAND,\n    SHARED_PROJECT_COMMAND, UNIVERSE,\n''',
    1,
)
needle = '''    let decision = text_component(world.state().entity(UNIVERSE), DECISION, "none");\n'''
insert = '''    let relationship_direction =\n        text_component(world.state().entity(RELATIONSHIP), RELATIONSHIP_DIRECTION, "none");\n    if generation >= 2 && relationship_direction == "none" {\n        commands.push(ProjectionCommand {\n            id: SHARED_PROJECT_COMMAND.into(),\n            title: "Give them a shared project".into(),\n            detail: "Create a goal that neither actor can complete alone; future interactions will lean toward trust.".into(),\n        });\n        commands.push(ProjectionCommand {\n            id: RIVALRY_COMMAND.into(),\n            title: "Let rivalry sharpen them".into(),\n            detail: "Keep both actors independent and let competition add pressure to future interactions.".into(),\n        });\n    }\n    let decision = text_component(world.state().entity(UNIVERSE), DECISION, "none");\n'''
if needle not in p:
    raise SystemExit('projection decision marker missing')
p = p.replace(needle, insert, 1)
p = p.replace(
    '''            "agent_explored_world" => "Someone explored beyond routine".into(),\n            _ => event.kind.replace('_', " "),\n''',
    '''            "agent_explored_world" => "Someone explored beyond routine".into(),\n            "relationship_shifted" => "Their relationship changed".into(),\n            "relationship_steered" => "You steered their relationship".into(),\n            _ => event.kind.replace('_', " "),\n''',
    1,
)
p = p.replace(
    '''    const POSITIONS: [(f32, f32); 5] = [\n        (0.16, 0.28),\n        (0.72, 0.24),\n        (0.18, 0.76),\n        (0.76, 0.72),\n        (0.48, 0.52),\n    ];\n''',
    '''    const POSITIONS: [(f32, f32); 6] = [\n        (0.14, 0.24),\n        (0.72, 0.22),\n        (0.16, 0.78),\n        (0.78, 0.74),\n        (0.50, 0.48),\n        (0.50, 0.82),\n    ];\n''',
    1,
)
projection.write_text(p)

# Version bumps.
for cargo_path in ['worlds/pocket-universe/Cargo.toml', 'apps/pocket-universe-pack/Cargo.toml']:
    cargo = Path(cargo_path)
    cargo.write_text(cargo.read_text().replace('version = "0.6.0"', 'version = "0.7.0"', 1))

# External E2E: fake Pi (Explore+Explore) produces durable tension on relationship entity.
ext = Path('apps/pocket-universe-pack/tests/external_pack.rs')
e = ext.read_text()
needle = '''        for actor_title in ["Nia Chen", "Tomas Vale"] {\n'''
insert = '''        let relationship = reopened_snapshot\n            .inspectors\n            .values()\n            .find(|inspector| inspector.title == "Nia ↔ Tomas")\n            .expect("Pi relationship inspector");\n        assert!(relationship\n            .sections\n            .iter()\n            .flat_map(|section| &section.rows)\n            .any(|row| row.label == "Trust" && row.value == "0"));\n        assert!(relationship\n            .sections\n            .iter()\n            .flat_map(|section| &section.rows)\n            .any(|row| row.label == "Tension" && row.value == "2"));\n\n        for actor_title in ["Nia Chen", "Tomas Vale"] {\n'''
if needle not in e:
    raise SystemExit('external actor loop marker missing')
e = e.replace(needle, insert, 1)
ext.write_text(e)
