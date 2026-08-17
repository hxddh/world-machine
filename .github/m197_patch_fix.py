from pathlib import Path

path = Path("crates/world-cli/tests/machine_query_causal_continuation.rs")
text = path.read_text()
old_import = "use std::path::PathBuf;"
new_import = "use std::path::{Path, PathBuf};"
old_sig = "fn run_typed_query(path: &PathBuf, request: &EvidenceQueryRequest) -> EvidenceQueryResponse {"
new_sig = "fn run_typed_query(path: &Path, request: &EvidenceQueryRequest) -> EvidenceQueryResponse {"
if text.count(old_import) != 1 or text.count(old_sig) != 1:
    raise SystemExit("M197 clippy fix anchors changed")
path.write_text(text.replace(old_import, new_import, 1).replace(old_sig, new_sig, 1))
