from pathlib import Path

FILES = [
    (Path("apps/tiny-society-pack/tests/external_pack.rs"), "tiny-society"),
    (Path("apps/pocket-universe-pack/tests/external_pack.rs"), "pocket-universe"),
    (Path("apps/micro-company-pack/tests/external_pack.rs"), "micro-company"),
]

for path, prefix in FILES:
    text = path.read_text()

    import_anchor = "use std::process::{self, Command};\nuse std::time::{SystemTime, UNIX_EPOCH};\n"
    import_replacement = (
        "use std::process::{self, Command};\n"
        "use std::sync::atomic::{AtomicU64, Ordering};\n"
        "use std::time::{SystemTime, UNIX_EPOCH};\n"
    )
    if text.count(import_anchor) != 1:
        raise SystemExit(f"unexpected import anchor in {path}")
    text = text.replace(import_anchor, import_replacement, 1)

    old = f'''fn temp_dir() -> PathBuf {{
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "world-machine-{prefix}-external-{{}}-{{nonce}}",
        process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}}
'''
    new = f'''static TEMP_DIR_NONCE: AtomicU64 = AtomicU64::new(1);

fn temp_dir() -> PathBuf {{
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let nonce = TEMP_DIR_NONCE.fetch_add(1, Ordering::Relaxed);
    let root = env::temp_dir().join(format!(
        "world-machine-{prefix}-external-{{}}-{{timestamp}}-{{nonce}}",
        process::id()
    ));
    fs::create_dir(&root).unwrap();
    root
}}
'''
    if text.count(old) != 1:
        raise SystemExit(f"unexpected temp_dir implementation in {path}")
    path.write_text(text.replace(old, new, 1))
