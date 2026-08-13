from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


lib_path = ROOT / "worlds/pocket-universe/src/lib.rs"
text = lib_path.read_text()
for old, new, label in [
    ('                "status",\n                "joint expedition crew",', '                "social_status",\n                "joint expedition crew",', 'mars partnership field'),
    ('                "status",\n                "split survey routes",', '                "social_status",\n                "split survey routes",', 'mars fracture field'),
    ('                "format",\n                "Lena + Max neighborhood show",', '                "social_format",\n                "Lena + Max neighborhood show",', 'town partnership field'),
    ('                "format",\n                "competing late shows",', '                "social_format",\n                "competing late shows",', 'town fracture field'),
    ('                "custom",\n                "shared watch council",', '                "social_order",\n                "shared watch council",', 'penguin partnership field'),
    ('                "custom",\n                "split moonrise caucuses",', '                "social_order",\n                "split moonrise caucuses",', 'penguin fracture field'),
]:
    text = replace_once(text, old, new, label)

text = replace_once(
    text,
    '.component("status"),\n            Some(&Value::Text("joint expedition crew".into()))',
    '.component("social_status"),\n            Some(&Value::Text("joint expedition crew".into()))',
    'partnership assertion field',
)
fracture_old = '.component("status"),\n            Some(&Value::Text("split survey routes".into()))'
fracture_new = '.component("social_status"),\n            Some(&Value::Text("split survey routes".into()))'
if text.count(fracture_old) != 2:
    raise SystemExit(f"fracture assertion fields: expected two matches, found {text.count(fracture_old)}")
text = text.replace(fracture_old, fracture_new)
text = replace_once(
    text,
    '''        universe.advance_periods(1).unwrap();
        let later_growth = universe
''',
    '''        universe.advance_periods(1).unwrap();
        assert_eq!(
            universe
                .world()
                .state()
                .entity(SLOT_D)
                .unwrap()
                .component("social_status"),
            Some(&Value::Text("joint expedition crew".into())),
            "ordinary later agent turns must not erase a resolved social arc"
        );
        let later_growth = universe
''',
    'partnership persistence regression',
)
lib_path.write_text(text)

external_path = ROOT / "apps/pocket-universe-pack/tests/external_pack.rs"
external = external_path.read_text()
external = replace_once(
    external,
    'row.label == "Status" && row.value == "split survey routes"',
    'row.label == "Social Status" && row.value == "split survey routes"',
    'external social status label',
)
external_path.write_text(external)
