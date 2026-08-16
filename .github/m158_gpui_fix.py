from pathlib import Path

path = Path("crates/world-gpui/src/macos.rs")
text = path.read_text()
old = '''        let event_ref = match selection {
            SelectionId::Event(event) => format!("World time {} · Event #{event}", item.world_time),
            SelectionId::Entity(_) => unreachable!("semantic path items must be Events"),
        };
'''
new = '''        let event_ref = match selection {
            SelectionId::Event(event) => format!("World time {} · Event #{event}", item.world_time),
            SelectionId::Entity(_) | SelectionId::Relation(_) => {
                unreachable!("semantic path items must be Events")
            }
        };
'''
assert text.count(old) == 1, f"semantic path match count: {text.count(old)}"
path.write_text(text.replace(old, new, 1))
