from pathlib import Path

p = Path('worlds/pocket-universe/src/lib.rs')
s = p.read_text()
old = '''        let desired = if (observation.world_time / BACKGROUND_PERIOD).is_multiple_of(2) {\n            AGENT_CARE_ACTION\n        } else {\n            AGENT_EXPLORE_ACTION\n        };\n'''
new = '''        let actor = observation\n            .entities\n            .iter()\n            .find(|entity| entity.id == observation.actor)\n            .ok_or_else(|| AgentRuntimeError::new("Pocket Mind observation is missing its actor"))?;\n        let count = |key: &str| match actor.component(key) {\n            Some(Value::Integer(value)) => Ok(*value),\n            _ => Err(AgentRuntimeError::new(format!(\n                "Pocket Mind actor is missing integer component {key}"\n            ))),\n        };\n        let care_count = count(AGENT_CARE_COUNT)?;\n        let explore_count = count(AGENT_EXPLORE_COUNT)?;\n        let desired = if care_count <= explore_count {\n            AGENT_CARE_ACTION\n        } else {\n            AGENT_EXPLORE_ACTION\n        };\n'''
if old not in s:
    raise SystemExit('world-time PocketMind policy not found')
s = s.replace(old, new, 1)

marker = '''    #[test]\n    fn deterministic_default_mind_keeps_identical_worlds_reproducible() {\n'''
extra = r'''    #[test]
    fn deterministic_mind_uses_durable_actor_memory_even_without_time_advancing() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();

        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();

        let actor = universe.world().state().entity(SLOT_B).unwrap();
        assert_eq!(actor.component(AGENT_CARE_COUNT), Some(&Value::Integer(1)));
        assert_eq!(
            actor.component(AGENT_EXPLORE_COUNT),
            Some(&Value::Integer(1))
        );
        assert_eq!(universe.world().world_time(), 0);
        let decisions = universe
            .world()
            .events()
            .iter()
            .filter(|event| event.kind == "agent_decision_recorded")
            .filter_map(|event| event.payload.get("selected_action"))
            .collect::<Vec<_>>();
        assert_eq!(
            decisions,
            vec![
                &Value::Text(AGENT_CARE_ACTION.into()),
                &Value::Text(AGENT_EXPLORE_ACTION.into())
            ]
        );
    }

'''
if marker not in s:
    raise SystemExit('deterministic mind test marker missing')
s = s.replace(marker, extra + marker, 1)
p.write_text(s)
