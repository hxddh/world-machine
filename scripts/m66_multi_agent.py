from pathlib import Path

lib = Path('worlds/pocket-universe/src/lib.rs')
s = lib.read_text()

s = s.replace('pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.5.0";', 'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.6.0";', 1)
s = s.replace('pub(crate) const SLOT_D: EntityId = EntityId::new(13);\n', 'pub(crate) const SLOT_D: EntityId = EntityId::new(13);\npub(crate) const SLOT_E: EntityId = EntityId::new(14);\n', 1)

# Make a Nudge one world-atomic growth + primary turn + secondary response.
old_nudge = '''            let outcome = Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &self.mind_profile,\n                &[growth],\n            )?;\n            self.world = candidate;\n            return Ok(outcome);\n'''
new_nudge = '''            let primary_outcome = Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &self.mind_profile,\n                SLOT_B,\n                &[growth],\n            )?;\n            let secondary_outcome = Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &self.mind_profile,\n                SLOT_E,\n                &[primary_outcome],\n            )?;\n            self.world = candidate;\n            return Ok(secondary_outcome);\n'''
if old_nudge not in s:
    raise SystemExit('nudge agent turn block not found')
s = s.replace(old_nudge, new_nudge, 1)

old_advance = '''            Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &self.mind_profile,\n                &[growth],\n            )?;\n'''
new_advance = '''            let primary_outcome = Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &self.mind_profile,\n                SLOT_B,\n                &[growth],\n            )?;\n            Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &self.mind_profile,\n                SLOT_E,\n                &[primary_outcome],\n            )?;\n'''
if old_advance not in s:
    raise SystemExit('advance agent turn block not found')
s = s.replace(old_advance, new_advance, 1)

s = s.replace(
    '''        mind_profile: &str,\n        caused_by: &[EventId],\n''',
    '''        mind_profile: &str,\n        actor: EntityId,\n        caused_by: &[EventId],\n''',
    1,
)
s = s.replace(
    '''            &ScopedPerception::new([UNIVERSE, SLOT_A]),\n            world,\n            registry,\n            SLOT_B,\n''',
    '''            &ScopedPerception::new([UNIVERSE, SLOT_A, SLOT_B, SLOT_E]),\n            world,\n            registry,\n            actor,\n''',
    1,
)

# Seed a second agent actor in each world.
s = s.replace(
    '''                Entity::new(SLOT_D, "rover")\n                    .with_component("name", "Kestrel Rover")\n                    .with_component("range", "18 km"),\n''',
    '''                Entity::new(SLOT_D, "rover")\n                    .with_component("name", "Kestrel Rover")\n                    .with_component("range", "18 km"),\n                Entity::new(SLOT_E, "person")\n                    .with_component("name", "Tomas Vale")\n                    .with_component("role", "rover scout")\n                    .with_component(AGENT_CARE_COUNT, 0_i64)\n                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)\n                    .with_component(LAST_MIND_PROFILE, "none"),\n''',
    1,
)
s = s.replace(
    '''                Entity::new(SLOT_D, "bus")\n                    .with_component("name", "Night Bus 6")\n                    .with_component("route", "Maple Loop"),\n''',
    '''                Entity::new(SLOT_D, "bus")\n                    .with_component("name", "Night Bus 6")\n                    .with_component("route", "Maple Loop"),\n                Entity::new(SLOT_E, "person")\n                    .with_component("name", "Max Park")\n                    .with_component("role", "radio volunteer")\n                    .with_component(AGENT_CARE_COUNT, 0_i64)\n                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)\n                    .with_component(LAST_MIND_PROFILE, "none"),\n''',
    1,
)
s = s.replace(
    '''                Entity::new(SLOT_D, "council")\n                    .with_component("name", "Aurora Council")\n                    .with_component("custom", "vote at moonrise"),\n''',
    '''                Entity::new(SLOT_D, "council")\n                    .with_component("name", "Aurora Council")\n                    .with_component("custom", "vote at moonrise"),\n                Entity::new(SLOT_E, "penguin")\n                    .with_component("name", "Miri")\n                    .with_component("role", "fish-vault keeper")\n                    .with_component(AGENT_CARE_COUNT, 0_i64)\n                    .with_component(AGENT_EXPLORE_COUNT, 0_i64)\n                    .with_component(LAST_MIND_PROFILE, "none"),\n''',
    1,
)

# seed_draft now accepts five entities.
s = s.replace('    entities: [Entity; 4],\n', '    entities: [Entity; 5],\n', 1)

# Allow both agent actors and make outcomes actor-specific.
s = s.replace(
    '''    if actor != SLOT_B {\n        return Err(ActionError::Invalid(format!(\n            "Pocket Mind action requires seed actor {SLOT_B}, got {actor}"\n        )));\n    }\n''',
    '''    if actor != SLOT_B && actor != SLOT_E {\n        return Err(ActionError::Invalid(format!(\n            "Pocket Mind action requires a seeded actor ({SLOT_B} or {SLOT_E}), got {actor}"\n        )));\n    }\n''',
    1,
)
s = s.replace('    let (target, key, value, change) = mind_outcome(&seed, care, next)?;\n', '    let (target, key, value, change) = mind_outcome(&seed, actor, care, next)?;\n', 1)

start = s.index('fn mind_outcome(\n')
end = s.index('\nimpl Action for ChooseBoldPath', start)
new_outcome = r'''fn mind_outcome(
    seed: &str,
    actor: EntityId,
    care: bool,
    turn: i64,
) -> Result<(EntityId, &'static str, String, String), ActionError> {
    let outcome = match (seed, actor, care) {
        ("mars-colony", SLOT_B, true) => (
            SLOT_C,
            "crop",
            format!("Nia tending cycle {turn}"),
            format!("Nia tuned the hydroponics loop for care cycle {turn}."),
        ),
        ("mars-colony", SLOT_B, false) => (
            SLOT_D,
            "range",
            format!("Nia survey route {turn}"),
            format!("Nia sent Kestrel onto survey route {turn} beyond the familiar markers."),
        ),
        ("mars-colony", SLOT_E, true) => (
            SLOT_D,
            "status",
            format!("Tomas service cycle {turn}"),
            format!("Tomas serviced Kestrel after Nia's latest move, closing out maintenance cycle {turn}."),
        ),
        ("mars-colony", SLOT_E, false) => (
            SLOT_A,
            "survey_report",
            format!("ridge trace {turn}"),
            format!("Tomas followed Nia's lead and returned with ridge trace {turn} for Ares Habitat."),
        ),
        ("1980s-town", SLOT_B, true) => (
            SLOT_A,
            "status",
            format!("Lena's community night {turn}"),
            format!("Lena kept Maple Arcade open for community night {turn}."),
        ),
        ("1980s-town", SLOT_B, false) => (
            SLOT_D,
            "route",
            format!("Lena's late loop {turn}"),
            format!("Lena rode Night Bus 6 through late loop {turn} and came back with a new story."),
        ),
        ("1980s-town", SLOT_E, true) => (
            SLOT_C,
            "format",
            format!("Max community set {turn}"),
            format!("Max answered Lena's latest move with community set {turn} on K-88."),
        ),
        ("1980s-town", SLOT_E, false) => (
            SLOT_D,
            "route",
            format!("Max signal chase {turn}"),
            format!("Max followed the thread from Lena's night and mapped signal chase {turn} along Bus 6."),
        ),
        ("penguin-civilization", SLOT_B, true) => (
            SLOT_A,
            "status",
            format!("Piko reinforced span {turn}"),
            format!("Piko reinforced Icebridge span {turn} before the next cold tide."),
        ),
        ("penguin-civilization", SLOT_B, false) => (
            SLOT_D,
            "custom",
            format!("Piko's edge report {turn}"),
            format!("Piko returned from edge scout {turn} with a new route under the aurora."),
        ),
        ("penguin-civilization", SLOT_E, true) => (
            SLOT_C,
            "reserve",
            format!("Miri reserve cycle {turn}"),
            format!("Miri answered Piko's latest move by balancing Fish Vault reserve cycle {turn}."),
        ),
        ("penguin-civilization", SLOT_E, false) => (
            SLOT_D,
            "custom",
            format!("Miri tide map {turn}"),
            format!("Miri followed Piko's trail and brought the Aurora Council tide map {turn}."),
        ),
        _ => {
            return Err(ActionError::Invalid(format!(
                "unsupported Pocket Universe mind outcome: seed={seed}, actor={actor}, care={care}"
            )))
        }
    };
    Ok(outcome)
}
'''
s = s[:start] + new_outcome + s[end:]

# Update existing tests for two turns per period.
s = s.replace(
    '''                .filter(|event| event.kind == "agent_decision_recorded")\n                .count(),\n            2\n''',
    '''                .filter(|event| event.kind == "agent_decision_recorded")\n                .count(),\n            4\n''',
    1,
)
s = s.replace(
    '''                    event.kind == "agent_cared_for_world" || event.kind == "agent_explored_world"\n                })\n                .count(),\n            2\n''',
    '''                    event.kind == "agent_cared_for_world" || event.kind == "agent_explored_world"\n                })\n                .count(),\n            4\n''',
    1,
)

s = s.replace(
    '''            PocketUniverse::with_agent_runtime(MockAgentRuntime::scripted([AGENT_EXPLORE_ACTION]))\n                .unwrap();\n''',
    '''            PocketUniverse::with_agent_runtime(MockAgentRuntime::scripted([\n                AGENT_EXPLORE_ACTION,\n                AGENT_CARE_ACTION,\n            ]))\n            .unwrap();\n''',
    1,
)
# Find primary actor's decision/outcome, not the secondary one.
s = s.replace(
    '''            .find(|event| event.kind == "agent_decision_recorded")\n''',
    '''            .find(|event| event.kind == "agent_decision_recorded" && event.actor == Some(SLOT_B))\n''',
    1,
)
s = s.replace(
    '''            .find(|event| event.kind == "agent_explored_world")\n''',
    '''            .find(|event| event.kind == "agent_explored_world" && event.actor == Some(SLOT_B))\n''',
    1,
)

# Turn the old multi-period failure test into the explicit second-agent atomicity regression.
s = s.replace('fn multi_period_failure_rolls_back_all_candidate_growth_and_agent_events()', 'fn second_agent_failure_rolls_back_growth_and_primary_agent_turn()', 1)
s = s.replace('        let error = universe.advance_periods(2).unwrap_err();\n', '        let error = universe.advance_periods(1).unwrap_err();\n', 1)

# Compare regression needs two same Care decisions per universe.
s = s.replace(
    'MockAgentRuntime::scripted([AGENT_CARE_ACTION]),\n            DETERMINISTIC_MIND_PROFILE,',
    'MockAgentRuntime::scripted([AGENT_CARE_ACTION, AGENT_CARE_ACTION]),\n            DETERMINISTIC_MIND_PROFILE,',
    1,
)
s = s.replace(
    'MockAgentRuntime::scripted([AGENT_CARE_ACTION]),\n            "pi",',
    'MockAgentRuntime::scripted([AGENT_CARE_ACTION, AGENT_CARE_ACTION]),\n            "pi",',
    1,
)

# Nudge memory now advances both actors independently. Replace the old global decision-vector assertion.
old_memory = '''        let actor = universe.world().state().entity(SLOT_B).unwrap();\n        assert_eq!(actor.component(AGENT_CARE_COUNT), Some(&Value::Integer(1)));\n        assert_eq!(\n            actor.component(AGENT_EXPLORE_COUNT),\n            Some(&Value::Integer(1))\n        );\n        assert_eq!(universe.world().world_time(), 0);\n        let decisions = universe\n            .world()\n            .events()\n            .iter()\n            .filter(|event| event.kind == "agent_decision_recorded")\n            .filter_map(|event| event.payload.get("selected_action"))\n            .collect::<Vec<_>>();\n        assert_eq!(\n            decisions,\n            vec![\n                &Value::Text(AGENT_CARE_ACTION.into()),\n                &Value::Text(AGENT_EXPLORE_ACTION.into())\n            ]\n        );\n'''
new_memory = '''        for actor_id in [SLOT_B, SLOT_E] {\n            let actor = universe.world().state().entity(actor_id).unwrap();\n            assert_eq!(actor.component(AGENT_CARE_COUNT), Some(&Value::Integer(1)));\n            assert_eq!(\n                actor.component(AGENT_EXPLORE_COUNT),\n                Some(&Value::Integer(1))\n            );\n            let decisions = universe\n                .world()\n                .events()\n                .iter()\n                .filter(|event| {\n                    event.kind == "agent_decision_recorded" && event.actor == Some(actor_id)\n                })\n                .filter_map(|event| event.payload.get("selected_action"))\n                .collect::<Vec<_>>();\n            assert_eq!(\n                decisions,\n                vec![\n                    &Value::Text(AGENT_CARE_ACTION.into()),\n                    &Value::Text(AGENT_EXPLORE_ACTION.into())\n                ]\n            );\n        }\n        assert_eq!(universe.world().world_time(), 0);\n'''
if old_memory not in s:
    raise SystemExit('deterministic memory test block not found')
s = s.replace(old_memory, new_memory, 1)

# Return briefing for latest period should expose both actor outcomes plus growth.
s = s.replace(
    '''        assert_eq!(\n            briefing\n                .items\n                .iter()\n                .filter(|item| item.detail.contains("Lena"))\n                .count(),\n            2\n        );\n''',
    '''        assert_eq!(\n            briefing\n                .items\n                .iter()\n                .filter(|item| item.detail.contains("Lena"))\n                .count(),\n            1\n        );\n        assert_eq!(\n            briefing\n                .items\n                .iter()\n                .filter(|item| item.detail.contains("Max"))\n                .count(),\n            1\n        );\n''',
    1,
)

# Add a direct causal-chain test before failure tests.
marker = '''    struct FailingMind;\n'''
extra = r'''    #[test]
    fn one_period_runs_two_causally_chained_agent_turns() {
        let mut universe = PocketUniverse::with_agent_runtime(MockAgentRuntime::scripted([
            AGENT_EXPLORE_ACTION,
            AGENT_CARE_ACTION,
        ]))
        .unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(1).unwrap();

        let events = universe.world().events();
        let growth = events.iter().find(|event| event.kind == "universe_grew").unwrap();
        let primary_decision = events
            .iter()
            .find(|event| event.kind == "agent_decision_recorded" && event.actor == Some(SLOT_B))
            .unwrap();
        let primary_outcome = events
            .iter()
            .find(|event| event.kind == "agent_explored_world" && event.actor == Some(SLOT_B))
            .unwrap();
        let secondary_decision = events
            .iter()
            .find(|event| event.kind == "agent_decision_recorded" && event.actor == Some(SLOT_E))
            .unwrap();
        let secondary_outcome = events
            .iter()
            .find(|event| event.kind == "agent_cared_for_world" && event.actor == Some(SLOT_E))
            .unwrap();

        assert!(primary_decision.caused_by.contains(&growth.id));
        assert!(primary_outcome.caused_by.contains(&growth.id));
        assert!(primary_outcome.caused_by.contains(&primary_decision.id));
        assert!(secondary_decision.caused_by.contains(&primary_outcome.id));
        assert!(secondary_outcome.caused_by.contains(&primary_outcome.id));
        assert!(secondary_outcome.caused_by.contains(&secondary_decision.id));

        let why = universe.projection_snapshot().why;
        let chain = why.get(&world_projection::SelectionId::Event(secondary_outcome.id)).unwrap();
        assert!(chain.nodes.iter().any(|node| node.id == world_projection::SelectionId::Event(primary_outcome.id)));
        assert!(chain.nodes.iter().any(|node| node.id == world_projection::SelectionId::Event(growth.id)));
    }

'''
if marker not in s:
    raise SystemExit('FailingMind marker not found')
s = s.replace(marker, extra + marker, 1)

lib.write_text(s)

# Projection: give five entities distinct canvas locations.
projection = Path('worlds/pocket-universe/src/projection.rs')
p = projection.read_text()
p = p.replace(
    'const POSITIONS: [(f32, f32); 4] = [(0.18, 0.30), (0.72, 0.26), (0.25, 0.74), (0.70, 0.70)];',
    'const POSITIONS: [(f32, f32); 5] = [\n        (0.16, 0.28),\n        (0.72, 0.24),\n        (0.18, 0.76),\n        (0.76, 0.72),\n        (0.48, 0.52),\n    ];',
    1,
)
projection.write_text(p)

# Versions.
for cargo_path in ['worlds/pocket-universe/Cargo.toml', 'apps/pocket-universe-pack/Cargo.toml']:
    cargo = Path(cargo_path)
    cargo.write_text(cargo.read_text().replace('version = "0.5.0"', 'version = "0.6.0"', 1))

# External Pack: assert both Pi actors exist and carry Pi provenance after a single period.
ext = Path('apps/pocket-universe-pack/tests/external_pack.rs')
e = ext.read_text()
needle = '''        let actor = reopened_snapshot\n            .inspectors\n            .values()\n            .find(|inspector| inspector.title == "Nia Chen")\n            .expect("Pi actor inspector");\n        assert!(actor\n            .sections\n            .iter()\n            .flat_map(|section| &section.rows)\n            .any(|row| { row.label == "Last Mind Profile" && row.value == "pi" }));\n'''
replacement = '''        for actor_title in ["Nia Chen", "Tomas Vale"] {\n            let actor = reopened_snapshot\n                .inspectors\n                .values()\n                .find(|inspector| inspector.title == actor_title)\n                .unwrap_or_else(|| panic!("missing Pi actor inspector: {actor_title}"));\n            assert!(actor\n                .sections\n                .iter()\n                .flat_map(|section| &section.rows)\n                .any(|row| { row.label == "Last Mind Profile" && row.value == "pi" }));\n        }\n'''
if needle not in e:
    raise SystemExit('Pi actor inspector assertion not found')
e = e.replace(needle, replacement, 1)
# One fake-Pi period should generate exactly two explore outcomes and no care outcomes.
needle2 = '''        assert!(pi_archive\n            .events\n            .iter()\n            .any(|event| event.kind == "agent_explored_world"));\n'''
replacement2 = '''        assert_eq!(\n            pi_archive\n                .events\n                .iter()\n                .filter(|event| event.kind == "agent_explored_world")\n                .count(),\n            2\n        );\n'''
if needle2 not in e:
    raise SystemExit('Pi explore assertion not found')
e = e.replace(needle2, replacement2, 1)
ext.write_text(e)
