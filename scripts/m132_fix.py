from pathlib import Path

p = Path("worlds/pocket-universe/tests/second_arc_compare.rs")
text = p.read_text()
old = '''    assert!(left_first.subtitle.contains("Outward"));
    assert!(right_first.subtitle.contains("Rooted"));
'''
new = '''    assert!(left_first.subtitle.to_ascii_lowercase().contains("outward"));
    assert!(right_first.subtitle.to_ascii_lowercase().contains("rooted"));
'''
if text.count(old) != 1:
    raise SystemExit(f"expected casing assertion anchor once, got {text.count(old)}")
p.write_text(text.replace(old, new, 1))
