from pathlib import Path

path = Path("crates/world-projection/src/influence.rs")
text = path.read_text()
old = """        .filter_map(|item| match item.id {
            SelectionId::Event(event) => Some((event, item)),
            SelectionId::Entity(_) => None,
        })
"""
new = """        .filter_map(|item| match item.id {
            SelectionId::Event(event) => Some((event, item)),
            SelectionId::Entity(_) | SelectionId::Relation(_) => None,
        })
"""
assert text.count(old) == 1, f"timeline filter matches: {text.count(old)}"
text = text.replace(old, new, 1)
old = """    match item.id {
        SelectionId::Event(event) => event,
        SelectionId::Entity(_) => unreachable!(\"Timeline items must select Events\"),
    }
"""
new = """    match item.id {
        SelectionId::Event(event) => event,
        SelectionId::Entity(_) | SelectionId::Relation(_) => {
            unreachable!(\"Timeline items must select Events\")
        }
    }
"""
assert text.count(old) == 1, f"event id matches: {text.count(old)}"
path.write_text(text.replace(old, new, 1))
