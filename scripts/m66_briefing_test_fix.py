from pathlib import Path

p = Path('worlds/pocket-universe/src/lib.rs')
s = p.read_text()
s = s.replace(
    '''                .filter(|item| item.detail.contains("Lena"))\n                .count(),\n            1\n''',
    '''                .filter(|item| item.detail.starts_with("Lena"))\n                .count(),\n            1\n''',
    1,
)
s = s.replace(
    '''                .filter(|item| item.detail.contains("Max"))\n                .count(),\n            1\n''',
    '''                .filter(|item| item.detail.starts_with("Max"))\n                .count(),\n            1\n''',
    1,
)
p.write_text(s)
