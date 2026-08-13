from pathlib import Path

lib = Path('worlds/pocket-universe/src/lib.rs')
s = lib.read_text()

old = '''    pub fn invoke_projection_command(\n        &mut self,\n        command_id: &str,\n    ) -> Result<EventId, Box<dyn Error>> {\n        if command_id == NUDGE_COMMAND {\n            let growth = self\n                .world\n                .execute(\n                    &self.actions,\n                    &ActionRequest::new("grow_universe").actor(UNIVERSE),\n                )?\n                .id;\n            return self.run_agent_turn(&[growth]);\n        }\n\n        let action = match command_id {\n            SEED_MARS_COLONY_COMMAND => "seed_mars_colony",\n            SEED_1980S_TOWN_COMMAND => "seed_1980s_town",\n            SEED_PENGUIN_CIVILIZATION_COMMAND => "seed_penguin_civilization",\n            BOLD_PATH_COMMAND => "choose_bold_path",\n            CAREFUL_PATH_COMMAND => "choose_careful_path",\n            _ => {\n                return Err(std::io::Error::other(format!(\n                    "unknown projection command: {command_id}"\n                ))\n                .into())\n            }\n        };\n        Ok(self\n            .world\n            .execute(&self.actions, &ActionRequest::new(action).actor(UNIVERSE))?\n            .id)\n    }\n\n    pub fn advance_periods(&mut self, periods: u64) -> Result<(), Box<dyn Error>> {\n        for _ in 0..periods {\n            let target = self\n                .world\n                .world_time()\n                .checked_add(BACKGROUND_PERIOD)\n                .ok_or_else(|| std::io::Error::other("Pocket Universe time overflow"))?;\n            if seed_id(&self.world) == UNSEEDED {\n                self.world.advance_to(&self.actions, target)?;\n                continue;\n            }\n\n            self.world\n                .schedule_at(target, ActionRequest::new("grow_universe").actor(UNIVERSE))?;\n            let executed = self.world.advance_to(&self.actions, target)?;\n            let growth = executed.last().copied().ok_or_else(|| {\n                std::io::Error::other("scheduled Pocket Universe growth did not run")\n            })?;\n            self.run_agent_turn(&[growth])?;\n        }\n        Ok(())\n    }\n\n    fn run_agent_turn(&mut self, caused_by: &[EventId]) -> Result<EventId, Box<dyn Error>> {\n        let actions = vec![\n            AvailableAction::new(\n                "Care for the small world and reinforce what already exists.",\n                ActionRequest::new(AGENT_CARE_ACTION),\n            ),\n            AvailableAction::new(\n                "Explore beyond the familiar routine and bring back a new thread.",\n                ActionRequest::new(AGENT_EXPLORE_ACTION),\n            ),\n        ];\n        let execution = AgentExecutor::decide_and_execute(\n            &mut self.mind,\n            &ScopedPerception::new([UNIVERSE, SLOT_A]),\n            &mut self.world,\n            &self.actions,\n            SLOT_B,\n            &actions,\n            caused_by,\n        )?;\n        Ok(execution.outcome_event)\n    }\n'''
new = '''    pub fn invoke_projection_command(\n        &mut self,\n        command_id: &str,\n    ) -> Result<EventId, Box<dyn Error>> {\n        if command_id == NUDGE_COMMAND {\n            let mut candidate = self.world.clone();\n            let growth = candidate\n                .execute(\n                    &self.actions,\n                    &ActionRequest::new("grow_universe").actor(UNIVERSE),\n                )?\n                .id;\n            let outcome = Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &[growth],\n            )?;\n            self.world = candidate;\n            return Ok(outcome);\n        }\n\n        let action = match command_id {\n            SEED_MARS_COLONY_COMMAND => "seed_mars_colony",\n            SEED_1980S_TOWN_COMMAND => "seed_1980s_town",\n            SEED_PENGUIN_CIVILIZATION_COMMAND => "seed_penguin_civilization",\n            BOLD_PATH_COMMAND => "choose_bold_path",\n            CAREFUL_PATH_COMMAND => "choose_careful_path",\n            _ => {\n                return Err(std::io::Error::other(format!(\n                    "unknown projection command: {command_id}"\n                ))\n                .into())\n            }\n        };\n        Ok(self\n            .world\n            .execute(&self.actions, &ActionRequest::new(action).actor(UNIVERSE))?\n            .id)\n    }\n\n    pub fn advance_periods(&mut self, periods: u64) -> Result<(), Box<dyn Error>> {\n        let mut candidate = self.world.clone();\n        for _ in 0..periods {\n            let target = candidate\n                .world_time()\n                .checked_add(BACKGROUND_PERIOD)\n                .ok_or_else(|| std::io::Error::other("Pocket Universe time overflow"))?;\n            if seed_id(&candidate) == UNSEEDED {\n                candidate.advance_to(&self.actions, target)?;\n                continue;\n            }\n\n            candidate\n                .schedule_at(target, ActionRequest::new("grow_universe").actor(UNIVERSE))?;\n            let executed = candidate.advance_to(&self.actions, target)?;\n            let growth = executed.last().copied().ok_or_else(|| {\n                std::io::Error::other("scheduled Pocket Universe growth did not run")\n            })?;\n            Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &[growth],\n            )?;\n        }\n        self.world = candidate;\n        Ok(())\n    }\n\n    fn run_agent_turn_on(\n        mind: &mut R,\n        world: &mut World,\n        registry: &ActionRegistry,\n        caused_by: &[EventId],\n    ) -> Result<EventId, Box<dyn Error>> {\n        let actions = vec![\n            AvailableAction::new(\n                "Care for the small world and reinforce what already exists.",\n                ActionRequest::new(AGENT_CARE_ACTION),\n            ),\n            AvailableAction::new(\n                "Explore beyond the familiar routine and bring back a new thread.",\n                ActionRequest::new(AGENT_EXPLORE_ACTION),\n            ),\n        ];\n        let execution = AgentExecutor::decide_and_execute(\n            mind,\n            &ScopedPerception::new([UNIVERSE, SLOT_A]),\n            world,\n            registry,\n            SLOT_B,\n            &actions,\n            caused_by,\n        )?;\n        Ok(execution.outcome_event)\n    }\n'''
if old not in s:
    raise SystemExit('transaction target block not found')
s = s.replace(old, new, 1)

marker = '''    #[test]\n    fn deterministic_mind_uses_durable_actor_memory_even_without_time_advancing() {\n'''
extra = r'''    struct FailingMind;

    impl AgentRuntime for FailingMind {
        fn decide(
            &mut self,
            _observation: &AgentObservation,
            _actions: &[AvailableAction],
        ) -> Result<AgentDecision, AgentRuntimeError> {
            Err(AgentRuntimeError::new("Pocket Mind is unavailable"))
        }
    }

    #[test]
    fn nudge_runtime_failure_leaves_durable_world_unchanged() {
        let mut universe = PocketUniverse::with_agent_runtime(FailingMind).unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        let before = universe.archive().unwrap();

        let error = universe.invoke_projection_command(NUDGE_COMMAND).unwrap_err();

        assert!(error.to_string().contains("Pocket Mind is unavailable"));
        assert_eq!(universe.archive().unwrap(), before);
        assert_eq!(universe.world().world_time(), 0);
    }

    #[test]
    fn multi_period_failure_rolls_back_all_candidate_growth_and_agent_events() {
        let mut universe = PocketUniverse::with_agent_runtime(MockAgentRuntime::scripted([
            AGENT_CARE_ACTION,
            "not-an-offered-action",
        ]))
        .unwrap();
        universe
            .invoke_projection_command(SEED_PENGUIN_CIVILIZATION_COMMAND)
            .unwrap();
        let before = universe.archive().unwrap();

        let error = universe.advance_periods(2).unwrap_err();

        assert!(error.to_string().contains("unavailable action"));
        assert_eq!(universe.archive().unwrap(), before);
        assert_eq!(universe.world().world_time(), 0);
    }

'''
if marker not in s:
    raise SystemExit('test insertion marker not found')
s = s.replace(marker, extra + marker, 1)

old_test = '''        assert_eq!(briefing.title, "While you were away");\n        assert!(briefing\n            .items\n            .iter()\n            .all(|item| item.title != "Agent Decision Recorded"));\n        assert!(briefing\n            .items\n            .iter()\n            .any(|item| { item.detail.contains("Lena") }));\n'''
new_test = '''        assert_eq!(briefing.title, "While you were away");\n        assert_eq!(briefing.items.len(), 3);\n        assert!(briefing\n            .items\n            .iter()\n            .all(|item| item.title != "agent decision recorded"));\n        assert_eq!(\n            briefing\n                .items\n                .iter()\n                .filter(|item| item.detail.contains("Lena"))\n                .count(),\n            2\n        );\n'''
if old_test not in s:
    raise SystemExit('return briefing test block not found')
s = s.replace(old_test, new_test, 1)
lib.write_text(s)

projection = Path('worlds/pocket-universe/src/projection.rs')
p = projection.read_text()
old_projection = '''            items: events.iter().rev().take(3).map(return_item).collect(),\n'''
new_projection = '''            items: events\n                .iter()\n                .rev()\n                .filter(|event| event.kind != "agent_decision_recorded")\n                .take(3)\n                .map(return_item)\n                .collect(),\n'''
if old_projection not in p:
    raise SystemExit('return briefing projection block not found')
p = p.replace(old_projection, new_projection, 1)
projection.write_text(p)
