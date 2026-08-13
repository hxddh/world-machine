from pathlib import Path

p = Path('worlds/pocket-universe/src/lib.rs')
s = p.read_text()
old = '''        let care_count = count(AGENT_CARE_COUNT)?;\n        let explore_count = count(AGENT_EXPLORE_COUNT)?;\n        let desired = if care_count <= explore_count {\n            AGENT_CARE_ACTION\n        } else {\n            AGENT_EXPLORE_ACTION\n        };\n'''
new = '''        let care_count = count(AGENT_CARE_COUNT)?;\n        let explore_count = count(AGENT_EXPLORE_COUNT)?;\n        let primary_outcome = observation.events.iter().rev().find(|event| {\n            event.actor == Some(SLOT_B)\n                && matches!(\n                    event.kind.as_str(),\n                    "agent_cared_for_world" | "agent_explored_world"\n                )\n        });\n        let desired = if observation.actor == SLOT_E {\n            match primary_outcome.map(|event| event.kind.as_str()) {\n                Some("agent_cared_for_world") => AGENT_EXPLORE_ACTION,\n                Some("agent_explored_world") => AGENT_CARE_ACTION,\n                _ if care_count <= explore_count => AGENT_CARE_ACTION,\n                _ => AGENT_EXPLORE_ACTION,\n            }\n        } else if care_count <= explore_count {\n            AGENT_CARE_ACTION\n        } else {\n            AGENT_EXPLORE_ACTION\n        };\n'''
if old not in s:
    raise SystemExit('PocketMind balancing policy not found')
s = s.replace(old, new, 1)

old_decisions = '''            assert_eq!(\n                decisions,\n                vec![\n                    &Value::Text(AGENT_CARE_ACTION.into()),\n                    &Value::Text(AGENT_EXPLORE_ACTION.into())\n                ]\n            );\n'''
new_primary = '''            let expected = if actor_id == SLOT_B {\n                vec![\n                    &Value::Text(AGENT_CARE_ACTION.into()),\n                    &Value::Text(AGENT_EXPLORE_ACTION.into()),\n                ]\n            } else {\n                vec![\n                    &Value::Text(AGENT_EXPLORE_ACTION.into()),\n                    &Value::Text(AGENT_CARE_ACTION.into()),\n                ]\n            };\n            assert_eq!(decisions, expected);\n'''
if old_decisions not in s:
    raise SystemExit('memory decision assertion not found')
s = s.replace(old_decisions, new_primary, 1)

marker = '''    #[test]\n    fn deterministic_default_mind_keeps_identical_worlds_reproducible() {\n'''
extra = r'''    #[test]
    fn deterministic_secondary_actor_reacts_to_primary_outcome() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();

        universe.advance_periods(1).unwrap();

        let primary = universe.world().state().entity(SLOT_B).unwrap();
        let secondary = universe.world().state().entity(SLOT_E).unwrap();
        assert_eq!(primary.component(AGENT_CARE_COUNT), Some(&Value::Integer(1)));
        assert_eq!(primary.component(AGENT_EXPLORE_COUNT), Some(&Value::Integer(0)));
        assert_eq!(secondary.component(AGENT_CARE_COUNT), Some(&Value::Integer(0)));
        assert_eq!(secondary.component(AGENT_EXPLORE_COUNT), Some(&Value::Integer(1)));
        assert_eq!(secondary.component("last_intent"), Some(&Value::Text("explore".into())));
    }

'''
if marker not in s:
    raise SystemExit('deterministic reproducibility test marker not found')
s = s.replace(marker, extra + marker, 1)
p.write_text(s)
