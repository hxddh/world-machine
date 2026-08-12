from pathlib import Path

lib = Path('worlds/pocket-universe/src/lib.rs')
s = lib.read_text()

s = s.replace(
    'use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};\n',
    'use world_agent::{\n    register_actions as register_agent_actions, AgentDecision, AgentExecutor, AgentObservation,\n    AgentRuntime, AgentRuntimeError, AvailableAction, ScopedPerception,\n};\nuse world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};\n',
    1,
)
s = s.replace('pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.2.0";', 'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.3.0";', 1)
s = s.replace(
    'const BACKGROUND_PERIOD: u64 = 10;\n',
    'const BACKGROUND_PERIOD: u64 = 10;\nconst AGENT_CARE_ACTION: &str = "pocket_agent.care";\nconst AGENT_EXPLORE_ACTION: &str = "pocket_agent.explore";\nconst AGENT_CARE_COUNT: &str = "care_count";\nconst AGENT_EXPLORE_COUNT: &str = "explore_count";\n',
    1,
)
old_struct = '''pub struct PocketUniverse {\n    world: World,\n    actions: ActionRegistry,\n}\n\nimpl PocketUniverse {\n    pub fn new() -> Result<Self, Box<dyn Error>> {\n        Ok(Self {\n            world: World::new(baseline()?),\n            actions: build_action_registry()?,\n        })\n    }\n'''
new_struct = '''#[derive(Clone, Debug, Default)]\npub struct PocketMind;\n\nimpl AgentRuntime for PocketMind {\n    fn decide(\n        &mut self,\n        observation: &AgentObservation,\n        actions: &[AvailableAction],\n    ) -> Result<AgentDecision, AgentRuntimeError> {\n        let desired = if (observation.world_time / BACKGROUND_PERIOD).is_multiple_of(2) {\n            AGENT_CARE_ACTION\n        } else {\n            AGENT_EXPLORE_ACTION\n        };\n        if !actions.iter().any(|action| action.name() == desired) {\n            return Err(AgentRuntimeError::new(format!(\n                "Pocket Mind expected offered action {desired}"\n            )));\n        }\n        Ok(AgentDecision::choose(desired))\n    }\n}\n\npub struct PocketUniverse<R = PocketMind>\nwhere\n    R: AgentRuntime,\n{\n    world: World,\n    actions: ActionRegistry,\n    mind: R,\n}\n\nimpl PocketUniverse<PocketMind> {\n    pub fn new() -> Result<Self, Box<dyn Error>> {\n        Self::with_agent_runtime(PocketMind)\n    }\n\n    pub fn resume_archive(archive: &WorldArchive) -> Result<Self, Box<dyn Error>> {\n        Self::resume_archive_with_agent_runtime(archive, PocketMind)\n    }\n}\n\nimpl<R> PocketUniverse<R>\nwhere\n    R: AgentRuntime,\n{\n    pub fn with_agent_runtime(mind: R) -> Result<Self, Box<dyn Error>> {\n        Ok(Self {\n            world: World::new(baseline()?),\n            actions: build_action_registry()?,\n            mind,\n        })\n    }\n'''
if old_struct not in s:
    raise SystemExit('PocketUniverse struct/new block not found')
s = s.replace(old_struct, new_struct, 1)

old_invoke = '''    pub fn invoke_projection_command(\n        &mut self,\n        command_id: &str,\n    ) -> Result<EventId, Box<dyn Error>> {\n        let action = match command_id {\n            SEED_MARS_COLONY_COMMAND => "seed_mars_colony",\n            SEED_1980S_TOWN_COMMAND => "seed_1980s_town",\n            SEED_PENGUIN_CIVILIZATION_COMMAND => "seed_penguin_civilization",\n            NUDGE_COMMAND => "grow_universe",\n            BOLD_PATH_COMMAND => "choose_bold_path",\n            CAREFUL_PATH_COMMAND => "choose_careful_path",\n            _ => {\n                return Err(std::io::Error::other(format!(\n                    "unknown projection command: {command_id}"\n                ))\n                .into())\n            }\n        };\n        Ok(self\n            .world\n            .execute(&self.actions, &ActionRequest::new(action).actor(UNIVERSE))?\n            .id)\n    }\n'''
new_invoke = '''    pub fn invoke_projection_command(\n        &mut self,\n        command_id: &str,\n    ) -> Result<EventId, Box<dyn Error>> {\n        if command_id == NUDGE_COMMAND {\n            let growth = self\n                .world\n                .execute(\n                    &self.actions,\n                    &ActionRequest::new("grow_universe").actor(UNIVERSE),\n                )?\n                .id;\n            return self.run_agent_turn(&[growth]);\n        }\n\n        let action = match command_id {\n            SEED_MARS_COLONY_COMMAND => "seed_mars_colony",\n            SEED_1980S_TOWN_COMMAND => "seed_1980s_town",\n            SEED_PENGUIN_CIVILIZATION_COMMAND => "seed_penguin_civilization",\n            BOLD_PATH_COMMAND => "choose_bold_path",\n            CAREFUL_PATH_COMMAND => "choose_careful_path",\n            _ => {\n                return Err(std::io::Error::other(format!(\n                    "unknown projection command: {command_id}"\n                ))\n                .into())\n            }\n        };\n        Ok(self\n            .world\n            .execute(&self.actions, &ActionRequest::new(action).actor(UNIVERSE))?\n            .id)\n    }\n'''
if old_invoke not in s:
    raise SystemExit('invoke block not found')
s = s.replace(old_invoke, new_invoke, 1)

old_advance = '''    pub fn advance_periods(&mut self, periods: u64) -> Result<(), Box<dyn Error>> {\n        if periods == 0 {\n            return Ok(());\n        }\n        let delta = periods\n            .checked_mul(BACKGROUND_PERIOD)\n            .ok_or_else(|| std::io::Error::other("Pocket Universe time overflow"))?;\n        let target = self\n            .world\n            .world_time()\n            .checked_add(delta)\n            .ok_or_else(|| std::io::Error::other("Pocket Universe time overflow"))?;\n\n        if seed_id(&self.world) != UNSEEDED {\n            for period in 1..=periods {\n                let at = self\n                    .world\n                    .world_time()\n                    .checked_add(period * BACKGROUND_PERIOD)\n                    .ok_or_else(|| std::io::Error::other("Pocket Universe time overflow"))?;\n                self.world\n                    .schedule_at(at, ActionRequest::new("grow_universe").actor(UNIVERSE))?;\n            }\n        }\n        self.world.advance_to(&self.actions, target)?;\n        Ok(())\n    }\n'''
new_advance = '''    pub fn advance_periods(&mut self, periods: u64) -> Result<(), Box<dyn Error>> {\n        for _ in 0..periods {\n            let target = self\n                .world\n                .world_time()\n                .checked_add(BACKGROUND_PERIOD)\n                .ok_or_else(|| std::io::Error::other("Pocket Universe time overflow"))?;\n            if seed_id(&self.world) == UNSEEDED {\n                self.world.advance_to(&self.actions, target)?;\n                continue;\n            }\n\n            self.world.schedule_at(\n                target,\n                ActionRequest::new("grow_universe").actor(UNIVERSE),\n            )?;\n            let executed = self.world.advance_to(&self.actions, target)?;\n            let growth = executed\n                .last()\n                .copied()\n                .ok_or_else(|| std::io::Error::other("scheduled Pocket Universe growth did not run"))?;\n            self.run_agent_turn(&[growth])?;\n        }\n        Ok(())\n    }\n\n    fn run_agent_turn(&mut self, caused_by: &[EventId]) -> Result<EventId, Box<dyn Error>> {\n        let actions = vec![\n            AvailableAction::new(\n                "Care for the small world and reinforce what already exists.",\n                ActionRequest::new(AGENT_CARE_ACTION),\n            ),\n            AvailableAction::new(\n                "Explore beyond the familiar routine and bring back a new thread.",\n                ActionRequest::new(AGENT_EXPLORE_ACTION),\n            ),\n        ];\n        let execution = AgentExecutor::decide_and_execute(\n            &mut self.mind,\n            &ScopedPerception::new([UNIVERSE, SLOT_A]),\n            &mut self.world,\n            &self.actions,\n            SLOT_B,\n            &actions,\n            caused_by,\n        )?;\n        Ok(execution.outcome_event)\n    }\n'''
if old_advance not in s:
    raise SystemExit('advance block not found')
s = s.replace(old_advance, new_advance, 1)

old_resume = '''    pub fn resume_archive(archive: &WorldArchive) -> Result<Self, Box<dyn Error>> {\n        Ok(Self {\n            world: archive.restore(&pocket_universe_pack_ref(), baseline()?)?,\n            actions: build_action_registry()?,\n        })\n    }\n'''
# M62 already moved resume to impl PocketUniverse<PocketMind>; old method should no longer exist after struct replacement if exact text follows below.
if old_resume in s:
    s = s.replace(old_resume, '', 1)

archive_marker = '''    pub fn archive(&self) -> Result<WorldArchive, PersistenceError> {\n        WorldArchive::capture(pocket_universe_pack_ref(), &self.world)\n    }\n\n'''
archive_new = archive_marker + '''    pub fn resume_archive_with_agent_runtime(\n        archive: &WorldArchive,\n        mind: R,\n    ) -> Result<Self, Box<dyn Error>> {\n        Ok(Self {\n            world: archive.restore(&pocket_universe_pack_ref(), baseline()?)?,\n            actions: build_action_registry()?,\n            mind,\n        })\n    }\n\n'''
if archive_marker not in s:
    raise SystemExit('archive marker missing')
s = s.replace(archive_marker, archive_new, 1)

s = s.replace(
    '''fn build_action_registry() -> Result<ActionRegistry, ActionError> {\n    let mut actions = ActionRegistry::new();\n''',
    '''fn build_action_registry() -> Result<ActionRegistry, ActionError> {\n    let mut actions = ActionRegistry::new();\n    register_agent_actions(&mut actions)?;\n''',
    1,
)
s = s.replace(
    '''    actions.register(ChooseBoldPath)?;\n    actions.register(ChooseCarefulPath)?;\n    Ok(actions)\n}\n''',
    '''    actions.register(ChooseBoldPath)?;\n    actions.register(ChooseCarefulPath)?;\n    actions.register(CareForWorld)?;\n    actions.register(ExploreWorld)?;\n    Ok(actions)\n}\n''',
    1,
)
s = s.replace(
    '''struct ChooseBoldPath;\nstruct ChooseCarefulPath;\n''',
    '''struct ChooseBoldPath;\nstruct ChooseCarefulPath;\nstruct CareForWorld;\nstruct ExploreWorld;\n''',
    1,
)
# Add mind counters to all three actor seeds.
s = s.replace('.with_component("role", "systems keeper"),', '.with_component("role", "systems keeper")\n                    .with_component(AGENT_CARE_COUNT, 0_i64)\n                    .with_component(AGENT_EXPLORE_COUNT, 0_i64),', 1)
s = s.replace('.with_component("role", "night-shift student"),', '.with_component("role", "night-shift student")\n                    .with_component(AGENT_CARE_COUNT, 0_i64)\n                    .with_component(AGENT_EXPLORE_COUNT, 0_i64),', 1)
s = s.replace('.with_component("role", "bridge keeper"),', '.with_component("role", "bridge keeper")\n                    .with_component(AGENT_CARE_COUNT, 0_i64)\n                    .with_component(AGENT_EXPLORE_COUNT, 0_i64),', 1)

choice_marker = '''impl Action for ChooseBoldPath {\n'''
agent_actions = r'''impl Action for CareForWorld {
    fn name(&self) -> &'static str {
        AGENT_CARE_ACTION
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        mind_action_draft(state, request, true)
    }
}

impl Action for ExploreWorld {
    fn name(&self) -> &'static str {
        AGENT_EXPLORE_ACTION
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        mind_action_draft(state, request, false)
    }
}

fn mind_action_draft(
    state: &WorldState,
    request: &ActionRequest,
    care: bool,
) -> Result<EventDraft, ActionError> {
    let actor = request
        .actor
        .ok_or_else(|| ActionError::Invalid("Pocket Mind action requires an actor".into()))?;
    if actor != SLOT_B {
        return Err(ActionError::Invalid(format!(
            "Pocket Mind action requires seed actor {SLOT_B}, got {actor}"
        )));
    }
    let seed = seed_id_from_state(state)?;
    if seed == UNSEEDED {
        return Err(ActionError::Invalid(
            "Pocket Mind cannot act before its world is seeded".into(),
        ));
    }
    let count_key = if care {
        AGENT_CARE_COUNT
    } else {
        AGENT_EXPLORE_COUNT
    };
    let next = integer_component(state, actor, count_key)? + 1;
    let (target, key, value, change) = mind_outcome(&seed, care, next)?;
    let mut draft = EventDraft::new(if care {
        "agent_cared_for_world"
    } else {
        "agent_explored_world"
    });
    draft.targets = vec![actor, target];
    draft.payload.insert("seed".into(), seed.into());
    draft.payload.insert("change".into(), change.clone().into());
    draft.payload.insert("turn".into(), next.into());
    draft.changes = vec![
        StateChange::SetComponent {
            entity: actor,
            key: count_key.into(),
            value: next.into(),
        },
        StateChange::SetComponent {
            entity: actor,
            key: "last_intent".into(),
            value: if care { "care" } else { "explore" }.into(),
        },
        StateChange::SetComponent {
            entity: target,
            key: key.into(),
            value: value.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),
            value: change.into(),
        },
    ];
    Ok(draft)
}

fn mind_outcome(
    seed: &str,
    care: bool,
    turn: i64,
) -> Result<(EntityId, &'static str, String, String), ActionError> {
    let outcome = match (seed, care) {
        ("mars-colony", true) => (
            SLOT_C,
            "crop",
            format!("Nia tending cycle {turn}"),
            format!("Nia tuned the hydroponics loop for care cycle {turn}."),
        ),
        ("mars-colony", false) => (
            SLOT_D,
            "range",
            format!("survey route {turn}"),
            format!("Nia sent Kestrel onto survey route {turn} beyond the familiar markers."),
        ),
        ("1980s-town", true) => (
            SLOT_A,
            "status",
            format!("Lena's community night {turn}"),
            format!("Lena kept Maple Arcade open for community night {turn}."),
        ),
        ("1980s-town", false) => (
            SLOT_D,
            "route",
            format!("Lena's late loop {turn}"),
            format!("Lena rode Night Bus 6 through late loop {turn} and came back with a new story."),
        ),
        ("penguin-civilization", true) => (
            SLOT_A,
            "status",
            format!("Piko reinforced span {turn}"),
            format!("Piko reinforced Icebridge span {turn} before the next cold tide."),
        ),
        ("penguin-civilization", false) => (
            SLOT_D,
            "custom",
            format!("Piko's edge report {turn}"),
            format!("Piko returned from edge scout {turn} with a new route under the aurora."),
        ),
        _ => {
            return Err(ActionError::Invalid(format!(
                "unsupported Pocket Universe seed: {seed}"
            )))
        }
    };
    Ok(outcome)
}

'''
if choice_marker not in s:
    raise SystemExit('choice impl marker missing')
s = s.replace(choice_marker, agent_actions + choice_marker, 1)

# Tests: import world-agent mock, adapt return briefing count, and add mind seam tests.
test_mod = '''mod tests {\n    use super::*;\n'''
replace_test_mod = '''mod tests {\n    use super::*;\n    use world_agent::MockAgentRuntime;\n'''
if test_mod not in s:
    raise SystemExit('test module marker missing')
s = s.replace(test_mod, replace_test_mod, 1)
# M62 background test expected 2 return items. Now each period has growth + agent outcome, decision filtered later => 4 meaningful, capped 3.
s = s.replace('        assert_eq!(briefing.items.len(), 2);', '        assert_eq!(briefing.items.len(), 3);', 1)

insert_test_marker = '''    #[test]\n    fn generation_three_exposes_a_durable_intervention() {\n'''
new_tests = r'''    #[test]
    fn scripted_mind_selects_only_offered_actions_and_records_causal_outcome() {
        let mut universe = PocketUniverse::with_agent_runtime(MockAgentRuntime::scripted([
            AGENT_EXPLORE_ACTION,
        ]))
        .unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(1).unwrap();

        let decision = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "agent_decision_recorded")
            .unwrap();
        let outcome = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "agent_explored_world")
            .unwrap();
        assert_eq!(decision.actor, Some(SLOT_B));
        assert!(outcome.caused_by.contains(&decision.id));
        assert!(outcome
            .caused_by
            .iter()
            .any(|cause| universe.world().event(*cause).is_some_and(|event| event.kind == "universe_grew")));
        assert_eq!(
            universe
                .world()
                .state()
                .entity(SLOT_B)
                .unwrap()
                .component(AGENT_EXPLORE_COUNT),
            Some(&Value::Integer(1))
        );
    }

    #[test]
    fn deterministic_default_mind_keeps_identical_worlds_reproducible() {
        let mut left = PocketUniverse::new().unwrap();
        let mut right = PocketUniverse::new().unwrap();
        left.invoke_projection_command(SEED_PENGUIN_CIVILIZATION_COMMAND)
            .unwrap();
        right
            .invoke_projection_command(SEED_PENGUIN_CIVILIZATION_COMMAND)
            .unwrap();

        left.advance_periods(4).unwrap();
        right.advance_periods(4).unwrap();

        assert_eq!(left.archive().unwrap(), right.archive().unwrap());
        assert_eq!(left.projection_snapshot(), right.projection_snapshot());
    }

    #[test]
    fn return_briefing_hides_agent_plumbing_but_keeps_agent_outcomes() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_1980S_TOWN_COMMAND.into(),
            ))
            .unwrap();
        let returned = session.advance_background(2).unwrap();
        let briefing = returned.briefing.as_ref().unwrap();

        assert_eq!(briefing.title, "While you were away");
        assert!(briefing.items.iter().all(|item| item.title != "Agent Decision Recorded"));
        assert!(briefing.items.iter().any(|item| {
            item.detail.contains("Lena")
        }));
    }

    struct PanicMind;

    impl AgentRuntime for PanicMind {
        fn decide(
            &mut self,
            _observation: &AgentObservation,
            _actions: &[AvailableAction],
        ) -> Result<AgentDecision, AgentRuntimeError> {
            panic!("archive restore must never call the agent runtime")
        }
    }

    #[test]
    fn archive_restore_does_not_call_the_mind() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(2).unwrap();
        let archive = universe.archive().unwrap();

        let restored =
            PocketUniverse::resume_archive_with_agent_runtime(&archive, PanicMind).unwrap();

        assert_eq!(restored.archive().unwrap(), archive);
        assert_eq!(restored.world().events(), universe.world().events());
    }

'''
if insert_test_marker not in s:
    raise SystemExit('agent test insertion marker missing')
s = s.replace(insert_test_marker, new_tests + insert_test_marker, 1)

lib.write_text(s)

# Projection: filter generic decision plumbing from return briefing and name agent outcomes.
projection = Path('worlds/pocket-universe/src/projection.rs')
p = projection.read_text()
p = p.replace(
    '''            items: events\n                .iter()\n                .rev()\n                .take(3)\n                .map(return_item)\n                .collect(),\n''',
    '''            items: events\n                .iter()\n                .rev()\n                .filter(|event| event.kind != "agent_decision_recorded")\n                .take(3)\n                .map(return_item)\n                .collect(),\n''',
    1,
)
p = p.replace(
    '''            "universe_seeded" => "A world began".into(),\n            _ => event.kind.replace('_', " "),\n''',
    '''            "universe_seeded" => "A world began".into(),\n            "agent_cared_for_world" => "Someone cared for the world".into(),\n            "agent_explored_world" => "Someone explored beyond routine".into(),\n            _ => event.kind.replace('_', " "),\n''',
    1,
)
projection.write_text(p)

# Cargo dependency/version updates.
cargo = Path('worlds/pocket-universe/Cargo.toml')
c = cargo.read_text().replace('version = "0.2.0"', 'version = "0.3.0"', 1)
c = c.replace('[dependencies]\n', '[dependencies]\nworld-agent = { path = "../../crates/world-agent" }\n', 1)
cargo.write_text(c)
app_cargo = Path('apps/pocket-universe-pack/Cargo.toml')
a = app_cargo.read_text().replace('version = "0.2.0"', 'version = "0.3.0"', 1)
app_cargo.write_text(a)

# External E2E: assert exact version and that agent decisions/outcomes survive the process/archive path.
external = Path('apps/pocket-universe-pack/tests/external_pack.rs')
e = external.read_text()
e = e.replace(
    '    BOLD_PATH_COMMAND, POCKET_UNIVERSE_PACK_ID, SEED_MARS_COLONY_COMMAND,\n',
    '    BOLD_PATH_COMMAND, POCKET_UNIVERSE_PACK_ID, POCKET_UNIVERSE_PACK_VERSION,\n    SEED_MARS_COLONY_COMMAND,\n',
    1,
)
e = e.replace(
    '    assert_eq!(preview.pack().id, POCKET_UNIVERSE_PACK_ID);\n',
    '    assert_eq!(preview.pack().id, POCKET_UNIVERSE_PACK_ID);\n    assert_eq!(preview.pack().version, POCKET_UNIVERSE_PACK_VERSION);\n',
    1,
)
e = e.replace(
    '''    let archive = session.archive().unwrap().unwrap();\n    let before = session.snapshot();\n''',
    '''    let archive = session.archive().unwrap().unwrap();\n    assert!(archive\n        .events\n        .iter()\n        .any(|event| event.kind == "agent_decision_recorded"));\n    assert!(archive.events.iter().any(|event| {\n        event.kind == "agent_cared_for_world" || event.kind == "agent_explored_world"\n    }));\n    let before = session.snapshot();\n''',
    1,
)
external.write_text(e)
