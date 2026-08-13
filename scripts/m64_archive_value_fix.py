from pathlib import Path

cargo = Path('apps/pocket-universe-pack/Cargo.toml')
s = cargo.read_text()
s = s.replace('world-core = { path = "../../crates/world-core" }\n', 'world-persistence = { path = "../../crates/world-persistence" }\n', 1)
cargo.write_text(s)

test = Path('apps/pocket-universe-pack/tests/external_pack.rs')
t = test.read_text()
t = t.replace(
    '== Some(&world_core::Value::Text("pocket_agent.explore".into()))',
    '== Some(&world_persistence::ArchivedValue::Text("pocket_agent.explore".into()))',
    1,
)
test.write_text(t)
