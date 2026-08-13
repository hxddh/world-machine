from pathlib import Path

p = Path('apps/pocket-universe-pack/Cargo.toml')
s = p.read_text()
needle = '[dev-dependencies]\n'
replacement = '[dev-dependencies]\nworld-core = { path = "../../crates/world-core" }\n'
if needle not in s:
    raise SystemExit('dev-dependencies section not found')
p.write_text(s.replace(needle, replacement, 1))
