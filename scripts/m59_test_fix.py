from pathlib import Path

p = Path('crates/world-pack-catalog/src/lib.rs')
s = p.read_text()
old = '''        assert!(installed.command_path.starts_with(root.join("Installed")));
'''
new = '''        assert!(installed
            .command_path
            .starts_with(managed_store_root(catalog.path())));
'''
if old not in s:
    raise SystemExit('managed-store assertion marker not found')
p.write_text(s.replace(old, new, 1))
