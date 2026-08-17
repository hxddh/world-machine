from pathlib import Path

path = Path("crates/world-cli/tests/machine_query_session.rs")
text = path.read_text()
text = text.replace(
    "use std::path::PathBuf;",
    "use std::path::{Path, PathBuf};",
)
text = text.replace(
    "fn run_session(path: &PathBuf, stdin: &str) -> Output {",
    "fn run_session(path: &Path, stdin: &str) -> Output {",
)
path.write_text(text)
