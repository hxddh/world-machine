from pathlib import Path

p = Path('worlds/pocket-universe/src/lib.rs')
s = p.read_text()
old_shared = '''        let shared_snapshot = shared\n            .invoke_projection_command(SHARED_PROJECT_COMMAND)\n            .and_then(|_| Ok(shared.projection_snapshot()))\n            .unwrap();\n'''
new_shared = '''        let shared_snapshot = shared\n            .invoke_projection_command(SHARED_PROJECT_COMMAND)\n            .map(|_| shared.projection_snapshot())\n            .unwrap();\n'''
old_rivalry = '''        let rivalry_snapshot = rivalry\n            .invoke_projection_command(RIVALRY_COMMAND)\n            .and_then(|_| Ok(rivalry.projection_snapshot()))\n            .unwrap();\n'''
new_rivalry = '''        let rivalry_snapshot = rivalry\n            .invoke_projection_command(RIVALRY_COMMAND)\n            .map(|_| rivalry.projection_snapshot())\n            .unwrap();\n'''
if old_shared not in s or old_rivalry not in s:
    raise SystemExit('relationship compare clippy targets not found')
s = s.replace(old_shared, new_shared, 1).replace(old_rivalry, new_rivalry, 1)
p.write_text(s)
