from pathlib import Path
import re

path = Path("scripts/m115_patch.py")
text = path.read_text()
pattern = re.compile(
    r'''replace_once\(\n    "apps/world-machine-desktop/src/main\.rs",\n.*?\n    "library summary helper",\n\)\n''',
    re.DOTALL,
)
replacement = r'''replace_once(
    "apps/world-machine-desktop/src/main.rs",
    '''#[cfg(target_os = "macos")]
fn world_summary_title(document: &WorldDocumentSummary, pack_title: &str) -> String {
    document
        .display_title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(pack_title)
        .to_owned()
}
''',
    '''#[cfg(target_os = "macos")]
fn world_summary_title(document: &WorldDocumentSummary, pack_title: &str) -> String {
    document
        .display_title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(pack_title)
        .to_owned()
}

#[cfg(target_os = "macos")]
fn world_summary_description(document: &WorldDocumentSummary) -> Option<String> {
    document
        .display_summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .map(str::to_owned)
}
''',
    "library summary helper",
)
'''
updated, count = pattern.subn(replacement, text, count=1)
if count != 1:
    raise SystemExit(f"expected one library summary helper patch call, found {count}")
path.write_text(updated)
