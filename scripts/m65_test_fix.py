from pathlib import Path

p = Path('worlds/pocket-universe/src/lib.rs')
s = p.read_text()
old = '''        let error = PocketUniverse::with_agent_runtime_profile(PocketMind, "pi api-key=secret")\n            .unwrap_err();\n'''
new = '''        let error = PocketUniverse::with_agent_runtime_profile(PocketMind, "pi api-key=secret")\n            .err()\n            .expect("freeform mind profile must be rejected");\n'''
if old not in s:
    raise SystemExit('mind profile rejection assertion not found')
p.write_text(s.replace(old, new, 1))
