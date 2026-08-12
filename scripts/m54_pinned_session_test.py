from pathlib import Path

p = Path('crates/world-pack-process/src/lib.rs')
s = p.read_text()
old = '''        let source = ProcessPackSource::from_manifest_paths([manifest_path]).unwrap();
        let mut registry = WorldRegistry::new();'''
new = '''        let pack = ProcessPack::load(manifest_path).unwrap();
        let pin = pack.current_pin().unwrap();
        let source = ProcessPackSource::from_packs(vec![pack.with_pin(pin)]);
        let mut registry = WorldRegistry::new();'''
if old not in s:
    raise SystemExit('session source marker not found')
s = s.replace(old, new, 1)
p.write_text(s)
