from pathlib import Path

path = Path(__file__).with_name("m69_social_arcs.py")
text = path.read_text()
old = '        let why = universe.projection_snapshot().why(partnership.id).unwrap();'
new = '''        let snapshot = universe.projection_snapshot();
        let why = snapshot.why(partnership.id).unwrap();'''
if text.count(old) != 1:
    raise SystemExit(f"expected one Why lifetime fragment, found {text.count(old)}")
path.write_text(text.replace(old, new, 1))
