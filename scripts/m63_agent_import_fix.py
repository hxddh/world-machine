from pathlib import Path

p = Path('apps/pocket-universe-pack/tests/external_pack.rs')
s = p.read_text()
old = 'use pocket_universe::{BOLD_PATH_COMMAND, POCKET_UNIVERSE_PACK_ID, SEED_MARS_COLONY_COMMAND};\n'
new = 'use pocket_universe::{\n    BOLD_PATH_COMMAND, POCKET_UNIVERSE_PACK_ID, POCKET_UNIVERSE_PACK_VERSION,\n    SEED_MARS_COLONY_COMMAND,\n};\n'
if old not in s:
    raise SystemExit('Pocket Universe external test import not found')
s = s.replace(old, new, 1)
p.write_text(s)
