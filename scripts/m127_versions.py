from pathlib import Path
import re

replacements = {
    "worlds/pocket-universe/Cargo.toml": [('version = "0.14.6"', 'version = "0.15.0"')],
    "apps/pocket-universe-pack/Cargo.toml": [('version = "0.14.6"', 'version = "0.15.0"')],
    "worlds/pocket-universe/src/lib.rs": [('pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.14.6";', 'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.15.0";')],
}
for path, pairs in replacements.items():
    p = Path(path)
    text = p.read_text()
    for old, new in pairs:
        if text.count(old) != 1:
            raise SystemExit(f"unexpected match count for {old} in {path}: {text.count(old)}")
        text = text.replace(old, new, 1)
    p.write_text(text)

p = Path("apps/world-machine-desktop/src/included_packs.rs")
text = p.read_text()
if text.count("0.14.6") != 2:
    raise SystemExit("expected exactly two included-pack version matches")
p.write_text(text.replace("0.14.6", "0.15.0"))

p = Path("Cargo.lock")
text = p.read_text()
for name in ["pocket-universe", "pocket-universe-pack"]:
    pattern = re.compile(rf'(\[\[package\]\]\nname = "{name}"\nversion = ")0\.14\.6("\n)')
    text, count = pattern.subn(r'\g<1>0.15.0\g<2>', text, count=1)
    if count != 1:
        raise SystemExit(f"missing exact Cargo.lock block for {name}")
p.write_text(text)
