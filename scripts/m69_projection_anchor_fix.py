from pathlib import Path

path = Path(__file__).with_name("m69_social_arcs.py")
text = path.read_text()
old = '            "relationship_steered" => "You changed their direction".into(),\\n'
new = '            "relationship_steered" => "You steered their relationship".into(),\\n'
if text.count(old) != 2:
    raise SystemExit(f"expected two projection anchor fragments, found {text.count(old)}")
path.write_text(text.replace(old, new))
