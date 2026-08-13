from pathlib import Path

path = Path(__file__).with_name("m69_social_arcs.py")
text = path.read_text()
replacements = [
    ('                "status",\n                "joint expedition crew",', '                "social_status",\n                "joint expedition crew",'),
    ('                "status",\n                "split survey routes",', '                "social_status",\n                "split survey routes",'),
    ('                "format",\n                "Lena + Max neighborhood show",', '                "social_format",\n                "Lena + Max neighborhood show",'),
    ('                "format",\n                "competing late shows",', '                "social_format",\n                "competing late shows",'),
    ('                "custom",\n                "shared watch council",', '                "social_order",\n                "shared watch council",'),
    ('                "custom",\n                "split moonrise caucuses",', '                "social_order",\n                "split moonrise caucuses",'),
    ('.component("status"),\n            Some(&Value::Text("joint expedition crew".into()))', '.component("social_status"),\n            Some(&Value::Text("joint expedition crew".into()))'),
    ('.component("status"),\n            Some(&Value::Text("split survey routes".into()))', '.component("social_status"),\n            Some(&Value::Text("split survey routes".into()))'),
    ('.component("status"),\n            Some(&Value::Text("split survey routes".into()))\n        );', '.component("social_status"),\n            Some(&Value::Text("split survey routes".into()))\n        );'),
    ('row.label == "Status" && row.value == "split survey routes"', 'row.label == "Social Status" && row.value == "split survey routes"'),
]
for old, new in replacements:
    if old not in text:
        raise SystemExit(f"missing persistent arc fragment: {old!r}")
    text = text.replace(old, new, 1)
path.write_text(text)
