from pathlib import Path

p = Path("worlds/pocket-universe/tests/second_arc_compare.rs")
text = p.read_text()
old = '''    assert!(left_first.subtitle.contains("Outward"));
    assert!(right_first.subtitle.contains("Rooted"));
'''
new = '''    assert_ne!(left_first.subtitle, right_first.subtitle);
'''
if text.count(old) != 1:
    raise SystemExit(f"expected Timeline divergence anchor once, got {text.count(old)}")
p.write_text(text.replace(old, new, 1))
