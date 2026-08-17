from pathlib import Path
import sys


def stage() -> None:
    patch = Path('.github/m194_patch.py')
    source = patch.read_text()
    write_at = source.index('lib.write_text(text)')
    failing_at = source.rfind('text = replace_once(', 0, write_at)
    if failing_at < 0:
        raise SystemExit('could not locate final staged replace_once')
    patch.write_text(source[:failing_at] + source[write_at:])


def finish() -> None:
    lib = Path('crates/world-query/src/lib.rs')
    text = lib.read_text()
    anchor = '''            (
                QueryError::SelectionNotVisibleInEitherWorld("relation-5".into()),'''
    insertion = '''            (
                QueryError::NoCausalPath {
                    from: "event-1".into(),
                    to: "event-9".into(),
                },
                r#"{"error":"no-causal-path","details":{"from":"event-1","to":"event-9"}}"#,
            ),
''' + anchor
    count = text.count(anchor)
    if count != 1:
        raise SystemExit(f'NoCausalPath serde anchor count: {count}')
    lib.write_text(text.replace(anchor, insertion, 1))


if len(sys.argv) != 2 or sys.argv[1] not in {'stage', 'finish'}:
    raise SystemExit('usage: m194_patch_fix.py <stage|finish>')

if sys.argv[1] == 'stage':
    stage()
else:
    finish()
