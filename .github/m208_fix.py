from pathlib import Path

path = Path("crates/world-query/tests/causal_first_divergence_trace_composition_consistency.rs")
text = path.read_text()
old = '    let mut seen = BTreeSet::<(usize, String, Vec<String>)>::new();\n'
new = '    let mut seen = BTreeSet::<(usize, String)>::new();\n'
if text.count(old) != 1:
    raise SystemExit(f"seen type: expected one match, found {text.count(old)}")
text = text.replace(old, new, 1)
old = '        if !seen.insert((offset, serialized, prefix.clone())) {\n'
new = '        if !seen.insert((offset, serialized)) {\n'
if text.count(old) != 1:
    raise SystemExit(f"seen key: expected one match, found {text.count(old)}")
text = text.replace(old, new, 1)
path.write_text(text)
