from pathlib import Path

path = Path("crates/world-compare/src/lib.rs")
text = path.read_text()
old = """        .and_then(|item| match item.id {
            SelectionId::Event(event) => Some(event),
            SelectionId::Entity(_) => None,
        })
"""
new = """        .and_then(|item| match item.id {
            SelectionId::Event(event) => Some(event),
            SelectionId::Entity(_) | SelectionId::Relation(_) => None,
        })
"""
assert text.count(old) == 1, f"divergence match count: {text.count(old)}"
text = text.replace(old, new, 1)
old = """        .filter_map(|(id, inspector)| match id {
            SelectionId::Entity(_) => Some((*id, inspector)),
            SelectionId::Event(_) => None,
        })
"""
new = """        .filter_map(|(id, inspector)| match id {
            SelectionId::Entity(_) => Some((*id, inspector)),
            SelectionId::Relation(_) | SelectionId::Event(_) => None,
        })
"""
assert text.count(old) == 1, f"entity inspector match count: {text.count(old)}"
path.write_text(text.replace(old, new, 1))
