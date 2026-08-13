from pathlib import Path

lib = Path('worlds/pocket-universe/src/lib.rs')
s = lib.read_text()

s = s.replace('pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.7.0";', 'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.8.0";', 1)

old_policy = '''        let care_count = count(AGENT_CARE_COUNT)?;\n        let explore_count = count(AGENT_EXPLORE_COUNT)?;\n        let primary_outcome = observation.events.iter().rev().find(|event| {\n            event.actor == Some(SLOT_B)\n                && matches!(\n                    event.kind.as_str(),\n                    "agent_cared_for_world" | "agent_explored_world"\n                )\n        });\n        let desired = if observation.actor == SLOT_E {\n            match primary_outcome.map(|event| event.kind.as_str()) {\n                Some("agent_cared_for_world") => AGENT_EXPLORE_ACTION,\n                Some("agent_explored_world") => AGENT_CARE_ACTION,\n                _ if care_count <= explore_count => AGENT_CARE_ACTION,\n                _ => AGENT_EXPLORE_ACTION,\n            }\n        } else if care_count <= explore_count {\n            AGENT_CARE_ACTION\n        } else {\n            AGENT_EXPLORE_ACTION\n        };\n'''
new_policy = '''        let care_count = count(AGENT_CARE_COUNT)?;\n        let explore_count = count(AGENT_EXPLORE_COUNT)?;\n        let relationship = observation\n            .entities\n            .iter()\n            .find(|entity| entity.id == RELATIONSHIP)\n            .ok_or_else(|| {\n                AgentRuntimeError::new("Pocket Mind observation is missing relationship state")\n            })?;\n        let direction = match relationship.component(RELATIONSHIP_DIRECTION) {\n            Some(Value::Text(direction)) => direction.as_str(),\n            _ => {\n                return Err(AgentRuntimeError::new(\n                    "Pocket Mind relationship is missing its direction",\n                ))\n            }\n        };\n        let primary_outcome = observation.events.iter().rev().find(|event| {\n            event.actor == Some(SLOT_B)\n                && matches!(\n                    event.kind.as_str(),\n                    "agent_cared_for_world" | "agent_explored_world"\n                )\n        });\n        let desired = if observation.actor == SLOT_E {\n            match (direction, primary_outcome.map(|event| event.kind.as_str())) {\n                ("rivalry", Some("agent_cared_for_world")) => AGENT_CARE_ACTION,\n                ("rivalry", Some("agent_explored_world")) => AGENT_EXPLORE_ACTION,\n                (_, Some("agent_cared_for_world")) => AGENT_EXPLORE_ACTION,\n                (_, Some("agent_explored_world")) => AGENT_CARE_ACTION,\n                (_, _) if care_count <= explore_count => AGENT_CARE_ACTION,\n                _ => AGENT_EXPLORE_ACTION,\n            }\n        } else if care_count <= explore_count {\n            AGENT_CARE_ACTION\n        } else {\n            AGENT_EXPLORE_ACTION\n        };\n'''
if old_policy not in s:
    raise SystemExit('PocketMind M67 policy block not found')
s = s.replace(old_policy, new_policy, 1)

s = s.replace(
    '&ScopedPerception::new([UNIVERSE, SLOT_A, SLOT_B, SLOT_E]),',
    '&ScopedPerception::new([UNIVERSE, SLOT_A, SLOT_B, SLOT_E, RELATIONSHIP]),',
    1,
)

# Add tests before the existing deterministic reproducibility test.
marker = '''    #[test]\n    fn deterministic_default_mind_keeps_identical_worlds_reproducible() {\n'''
extra = r'''    #[test]
    fn relationship_direction_changes_future_secondary_behavior() {
        let mut shared = PocketUniverse::new().unwrap();
        let mut rivalry = PocketUniverse::new().unwrap();
        for universe in [&mut shared, &mut rivalry] {
            universe
                .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
                .unwrap();
            universe.advance_periods(2).unwrap();
        }

        shared
            .invoke_projection_command(SHARED_PROJECT_COMMAND)
            .unwrap();
        rivalry.invoke_projection_command(RIVALRY_COMMAND).unwrap();
        shared.advance_periods(1).unwrap();
        rivalry.advance_periods(1).unwrap();

        let shared_primary = shared.world().state().entity(SLOT_B).unwrap();
        let shared_secondary = shared.world().state().entity(SLOT_E).unwrap();
        let rivalry_primary = rivalry.world().state().entity(SLOT_B).unwrap();
        let rivalry_secondary = rivalry.world().state().entity(SLOT_E).unwrap();
        assert_eq!(
            shared_primary.component("last_intent"),
            Some(&Value::Text("care".into()))
        );
        assert_eq!(
            rivalry_primary.component("last_intent"),
            Some(&Value::Text("care".into()))
        );
        assert_eq!(
            shared_secondary.component("last_intent"),
            Some(&Value::Text("explore".into()))
        );
        assert_eq!(
            rivalry_secondary.component("last_intent"),
            Some(&Value::Text("care".into()))
        );

        let shared_relationship = shared.world().state().entity(RELATIONSHIP).unwrap();
        let rivalry_relationship = rivalry.world().state().entity(RELATIONSHIP).unwrap();
        assert_eq!(
            shared_relationship.component(RELATIONSHIP_DIRECTION),
            Some(&Value::Text("shared-project".into()))
        );
        assert_eq!(
            rivalry_relationship.component(RELATIONSHIP_DIRECTION),
            Some(&Value::Text("rivalry".into()))
        );
    }

    #[derive(Clone)]
    struct RecordingMind {
        observations: Arc<std::sync::Mutex<Vec<AgentObservation>>>,
    }

    impl AgentRuntime for RecordingMind {
        fn decide(
            &mut self,
            observation: &AgentObservation,
            _actions: &[AvailableAction],
        ) -> Result<AgentDecision, AgentRuntimeError> {
            self.observations.lock().unwrap().push(observation.clone());
            Ok(AgentDecision::choose(AGENT_CARE_ACTION))
        }
    }

    #[test]
    fn every_agent_provider_observes_durable_relationship_context() {
        let observations = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut universe = PocketUniverse::with_agent_runtime(RecordingMind {
            observations: Arc::clone(&observations),
        })
        .unwrap();
        universe
            .invoke_projection_command(SEED_1980S_TOWN_COMMAND)
            .unwrap();
        universe.advance_periods(2).unwrap();
        universe
            .invoke_projection_command(RIVALRY_COMMAND)
            .unwrap();
        observations.lock().unwrap().clear();

        universe.advance_periods(1).unwrap();

        let captured = observations.lock().unwrap();
        assert_eq!(captured.len(), 2);
        for observation in captured.iter() {
            let relationship = observation
                .entities
                .iter()
                .find(|entity| entity.id == RELATIONSHIP)
                .expect("agent observation must contain the durable relationship entity");
            assert_eq!(
                relationship.component(RELATIONSHIP_DIRECTION),
                Some(&Value::Text("rivalry".into()))
            );
            assert!(matches!(
                relationship.component(RELATIONSHIP_TRUST),
                Some(Value::Integer(_))
            ));
            assert!(matches!(
                relationship.component(RELATIONSHIP_TENSION),
                Some(Value::Integer(_))
            ));
        }
        let secondary = captured
            .iter()
            .find(|observation| observation.actor == SLOT_E)
            .expect("secondary observation");
        assert!(secondary.events.iter().any(|event| {
            event.actor == Some(SLOT_B)
                && matches!(
                    event.kind.as_str(),
                    "agent_cared_for_world" | "agent_explored_world"
                )
        }));
    }

'''
if marker not in s:
    raise SystemExit('deterministic reproducibility marker not found')
s = s.replace(marker, extra + marker, 1)

lib.write_text(s)

for cargo_path in ['worlds/pocket-universe/Cargo.toml', 'apps/pocket-universe-pack/Cargo.toml']:
    cargo = Path(cargo_path)
    cargo.write_text(cargo.read_text().replace('version = "0.7.0"', 'version = "0.8.0"', 1))
