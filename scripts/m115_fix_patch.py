from pathlib import Path

path = Path("scripts/m115_patch.py")
text = path.read_text()
old = '''    ''' + "'''fn world_summary_title(document: &WorldDocumentSummary, pack_title: &str) -> String {\n    document\n        .display_title\n        .as_deref()\n        .map(str::trim)\n        .filter(|title| !title.is_empty())\n        .map(str::to_owned)\n        .unwrap_or_else(|| pack_title.to_owned())\n}\n'''" + ''',
    ''' + "'''fn world_summary_title(document: &WorldDocumentSummary, pack_title: &str) -> String {\n    document\n        .display_title\n        .as_deref()\n        .map(str::trim)\n        .filter(|title| !title.is_empty())\n        .map(str::to_owned)\n        .unwrap_or_else(|| pack_title.to_owned())\n}\n\n#[cfg(target_os = \\\"macos\\\")]\nfn world_summary_description(document: &WorldDocumentSummary) -> Option<String> {\n    document\n        .display_summary\n        .as_deref()\n        .map(str::trim)\n        .filter(|summary| !summary.is_empty())\n        .map(str::to_owned)\n}\n'''" + ''','''
new = '''    ''' + "'''#[cfg(target_os = \\\"macos\\\")]\nfn world_summary_title(document: &WorldDocumentSummary, pack_title: &str) -> String {\n    document\n        .display_title\n        .as_deref()\n        .map(str::trim)\n        .filter(|title| !title.is_empty())\n        .unwrap_or(pack_title)\n        .to_owned()\n}\n'''" + ''',
    ''' + "'''#[cfg(target_os = \\\"macos\\\")]\nfn world_summary_title(document: &WorldDocumentSummary, pack_title: &str) -> String {\n    document\n        .display_title\n        .as_deref()\n        .map(str::trim)\n        .filter(|title| !title.is_empty())\n        .unwrap_or(pack_title)\n        .to_owned()\n}\n\n#[cfg(target_os = \\\"macos\\\")]\nfn world_summary_description(document: &WorldDocumentSummary) -> Option<String> {\n    document\n        .display_summary\n        .as_deref()\n        .map(str::trim)\n        .filter(|summary| !summary.is_empty())\n        .map(str::to_owned)\n}\n'''" + ''','''
if old not in text:
    raise SystemExit("expected old M115 helper patch block was not found")
path.write_text(text.replace(old, new, 1))
