//! World files and their names: ids for new and imported documents, file
//! names from titles, and which files are Worlds or Packs. Moved out of
//! `main.rs` as it stood.

#[allow(unused_imports)]
use super::*;

/// Now, in Unix seconds.
pub(crate) fn unix_now() -> Option<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_secs())
}

/// "Keeps going without you · next sol in 5 h": what the app is about, in
/// one line, with the one number that makes it true.
pub(crate) fn keeps_going_line(remaining_seconds: u64, unit: &str) -> String {
    world_gpui::i18n::keeps_going(remaining_seconds, unit)
}

pub(crate) fn new_document_id(
    pack_id: &str,
    library: &WorldLibrary,
) -> Result<WorldDocumentId, LibraryError> {
    unique_document_id(sanitize_document_base(pack_id), Some(library))
}

pub(crate) fn imported_document_id(
    source: &Path,
    library: &WorldLibrary,
) -> Result<WorldDocumentId, LibraryError> {
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("imported-world");
    let base = file_name
        .strip_suffix(LEGACY_WORLD_DOCUMENT_SUFFIX)
        .or_else(|| file_name.strip_suffix(WORLD_DOCUMENT_SUFFIX))
        .unwrap_or(file_name);
    unique_document_id(sanitize_document_base(base), Some(library))
}

pub(crate) fn library_document_id_for_path(
    source: &Path,
    library: &WorldLibrary,
) -> Option<WorldDocumentId> {
    if source.parent()? != library.root() {
        return None;
    }
    let file_name = source.file_name()?.to_str()?;
    let raw_id = file_name
        .strip_suffix(LEGACY_WORLD_DOCUMENT_SUFFIX)
        .or_else(|| file_name.strip_suffix(WORLD_DOCUMENT_SUFFIX))?;
    let id = WorldDocumentId::new(raw_id).ok()?;
    library
        .contains(&id)
        .ok()
        .filter(|exists| *exists)
        .map(|_| id)
}

pub(crate) fn unique_document_id(
    mut base: String,
    library: Option<&WorldLibrary>,
) -> Result<WorldDocumentId, LibraryError> {
    if base.is_empty() {
        base = "imported-world".into();
    }
    let candidate = WorldDocumentId::new(base.clone())?;
    match library {
        Some(library) if library.contains(&candidate)? => {}
        _ => return Ok(candidate),
    }

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    WorldDocumentId::new(format!("{base}-{}-{nonce}", process::id()))
}

pub(crate) fn sanitize_document_base(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .take(80)
        .collect()
}

pub(crate) fn suggested_world_file_name(semantic_title: &str, fallback_label: &str) -> String {
    let semantic_title = semantic_title.trim();
    let source = if semantic_title.is_empty() {
        fallback_label
    } else {
        semantic_title
    };
    let source = source
        .strip_suffix(LEGACY_WORLD_DOCUMENT_SUFFIX)
        .or_else(|| source.strip_suffix(WORLD_DOCUMENT_SUFFIX))
        .unwrap_or(source);
    let stem = source
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '-'
            } else {
                ch
            }
        })
        .take(80)
        .collect::<String>();
    let stem = stem.trim_matches(|ch: char| ch.is_whitespace() || matches!(ch, '.' | '-'));
    let stem = if stem.is_empty() { "World" } else { stem };
    format!("{stem}{WORLD_DOCUMENT_SUFFIX}")
}

pub(crate) fn canonical_world_path(mut path: PathBuf) -> PathBuf {
    let Some(file_name) = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
    else {
        return path;
    };

    if file_name.ends_with(WORLD_DOCUMENT_SUFFIX) {
        return path;
    }
    if let Some(base) = file_name.strip_suffix(LEGACY_WORLD_DOCUMENT_SUFFIX) {
        path.set_file_name(format!("{base}{WORLD_DOCUMENT_SUFFIX}"));
    } else {
        path.set_extension("world");
    }
    path
}

pub(crate) fn is_world_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.ends_with(WORLD_DOCUMENT_SUFFIX) || name.ends_with(LEGACY_WORLD_DOCUMENT_SUFFIX)
        })
}

pub(crate) fn is_world_pack_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(PACK_BUNDLE_SUFFIX))
}
