from pathlib import Path

p = Path("crates/world-projection/src/lib.rs")
text = p.read_text()

old_mod = "mod causal;\n"
new_mod = "mod causal;\nmod influence;\n"
if text.count(old_mod) != 1:
    raise SystemExit(f"module anchor count {text.count(old_mod)}")
text = text.replace(old_mod, new_mod, 1)

old = '''    if !payload.is_empty() {\n        sections.push(InspectorSection {\n            title: "Payload".into(),\n            rows: payload,\n        });\n    }\n\n    InspectorProjection {'''
new = '''    if !payload.is_empty() {\n        sections.push(InspectorSection {\n            title: "Payload".into(),\n            rows: payload,\n        });\n    }\n\n    let influence = influence::influence_rows(world, event.id);\n    if !influence.is_empty() {\n        sections.push(InspectorSection {\n            title: "Influence".into(),\n            rows: influence,\n        });\n    }\n\n    InspectorProjection {'''
if text.count(old) != 1:
    raise SystemExit(f"inspector anchor count {text.count(old)}")
text = text.replace(old, new, 1)
p.write_text(text)
