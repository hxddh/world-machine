from pathlib import Path

p = Path('scripts/m56_managed_store.py')
s = p.read_text()
s = s.replace(r"manifest_json.push(b'\n');", r"manifest_json.push(b'\\n');", 1)
p.write_text(s)
