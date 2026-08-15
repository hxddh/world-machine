from pathlib import Path

source_path = Path('scripts/m122_patch.py')
source = source_path.read_text()
old = "replace_all_checked('Cargo.lock', 'version = \"0.14.1\"', 'version = \"0.14.2\"', 2)"
new = '''lock_path = ROOT / 'Cargo.lock'\nlock_text = lock_path.read_text()\nfor package in ('pocket-universe', 'pocket-universe-pack'):\n    old_block = f'[[package]]\\nname = "{package}"\\nversion = "0.14.1"'\n    new_block = f'[[package]]\\nname = "{package}"\\nversion = "0.14.2"'\n    if lock_text.count(old_block) != 1:\n        raise SystemExit(f'Cargo.lock: expected one {package} 0.14.1 package block')\n    lock_text = lock_text.replace(old_block, new_block, 1)\nlock_path.write_text(lock_text)'''
if source.count(old) != 1:
    raise SystemExit(f'expected one lockfile patch expression, found {source.count(old)}')
source = source.replace(old, new, 1)
exec(compile(source, str(source_path), 'exec'), {'__name__': '__main__'})
