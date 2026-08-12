from pathlib import Path

p = Path('worlds/pocket-universe/src/lib.rs')
s = p.read_text()
old = '''        assert_eq!(after.world_time, before.world_time + 20);\n        assert_eq!(after.timeline.items.len(), before.timeline.items.len() + 2);\n        let briefing = after.briefing.as_ref().unwrap();\n'''
new = '''        assert_eq!(after.world_time, before.world_time + 20);\n        let new_events = &session\n            .archive()\n            .unwrap()\n            .unwrap()\n            .events[before.timeline.items.len()..];\n        assert_eq!(\n            new_events\n                .iter()\n                .filter(|event| event.kind == "universe_grew")\n                .count(),\n            2\n        );\n        assert_eq!(\n            new_events\n                .iter()\n                .filter(|event| event.kind == "agent_decision_recorded")\n                .count(),\n            2\n        );\n        assert_eq!(\n            new_events\n                .iter()\n                .filter(|event| {\n                    event.kind == "agent_cared_for_world"\n                        || event.kind == "agent_explored_world"\n                })\n                .count(),\n            2\n        );\n        let briefing = after.briefing.as_ref().unwrap();\n'''
if old not in s:
    raise SystemExit('background event-count assertion block not found')
s = s.replace(old, new, 1)
p.write_text(s)
