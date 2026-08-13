from pathlib import Path

p = Path('worlds/pocket-universe/src/lib.rs')
s = p.read_text()
old = '''        let why = universe.projection_snapshot().why;\n        let chain = why.get(&shifted.id).unwrap();\n        assert!(chain.nodes.iter().any(|node| node.kind == "agent_explored_world"));\n        assert!(chain.nodes.iter().any(|node| node.kind == "universe_grew"));\n'''
new = '''        let explored = universe\n            .world()\n            .events()\n            .iter()\n            .find(|event| event.kind == "agent_explored_world")\n            .unwrap()\n            .id;\n        let growth = universe\n            .world()\n            .events()\n            .iter()\n            .find(|event| event.kind == "universe_grew")\n            .unwrap()\n            .id;\n        let why = universe.projection_snapshot().why;\n        let chain = why.get(&shifted.id).unwrap();\n        assert!(chain.nodes.iter().any(|node| node.event == explored));\n        assert!(chain.nodes.iter().any(|node| node.event == growth));\n'''
if old not in s:
    raise SystemExit('relationship Why assertion block not found')
p.write_text(s.replace(old, new, 1))
