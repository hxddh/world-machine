from pathlib import Path

p = Path('worlds/pocket-universe/src/lib.rs')
s = p.read_text()
s = s.replace(
    '        let chain = why.get(&world_projection::SelectionId::Event(secondary_outcome.id)).unwrap();\n        assert!(chain.nodes.iter().any(|node| node.id == world_projection::SelectionId::Event(primary_outcome.id)));\n        assert!(chain.nodes.iter().any(|node| node.id == world_projection::SelectionId::Event(growth.id)));\n',
    '        let chain = why.get(&secondary_outcome.id).unwrap();\n        assert!(chain.nodes.iter().any(|node| node.event == primary_outcome.id));\n        assert!(chain.nodes.iter().any(|node| node.event == growth.id));\n',
    1,
)
p.write_text(s)
