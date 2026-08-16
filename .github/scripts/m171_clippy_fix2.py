from pathlib import Path
import re

path = Path("crates/world-compare/src/lib.rs")
text = path.read_text()
old_signature = '''        from_id: u64,
        from_text: &str,
        to_id: u64,
        to_text: &str,
    ) -> (SelectionId, InspectorProjection) {
'''
new_signature = '''        from: (u64, &str),
        to: (u64, &str),
    ) -> (SelectionId, InspectorProjection) {
        let (from_id, from_text) = from;
        let (to_id, to_text) = to;
'''
if text.count(old_signature) != 1:
    raise SystemExit(f"expected one helper signature, found {text.count(old_signature)}")
text = text.replace(old_signature, new_signature)

pattern = re.compile(
    r'(?P<prefix>(?<!fn )relation_inspector_with_endpoints\(\n'
    r'(?P<indent>[ \t]+)(?P<a1>[^,\n]+),\n'
    r'(?P=indent)(?P<a2>[^,\n]+),\n'
    r'(?P=indent)(?P<a3>[^,\n]+),\n'
    r'(?P=indent)(?P<a4>[^,\n]+),\n)'
    r'(?P=indent)(?P<from_id>\d+),\n'
    r'(?P=indent)(?P<from_text>"[^"\n]*"),\n'
    r'(?P=indent)(?P<to_id>\d+),\n'
    r'(?P=indent)(?P<to_text>"[^"\n]*"),\n'
)

def replace_call(match: re.Match[str]) -> str:
    indent = match.group("indent")
    return (
        match.group("prefix")
        + f'{indent}({match.group("from_id")}, {match.group("from_text")}),\n'
        + f'{indent}({match.group("to_id")}, {match.group("to_text")}),\n'
    )

text, count = pattern.subn(replace_call, text)
if count != 9:
    raise SystemExit(f"expected 9 endpoint helper calls, transformed {count}")
path.write_text(text)
