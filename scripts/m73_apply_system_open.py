from pathlib import Path

path = Path("apps/world-machine-desktop/src/main.rs")
text = path.read_text()


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(
            f"expected one replacement anchor, found {count}: {old[:120]!r}"
        )
    text = text.replace(old, new, 1)


replace_once(
    '''#[cfg(target_os = "macos")]
use world_pack_catalog::{InstalledPack, PackAvailability, PackCatalog, PackInstallPreview};
''',
    '''#[cfg(target_os = "macos")]
use world_pack_bundle::PACK_BUNDLE_SUFFIX;
#[cfg(target_os = "macos")]
use world_pack_catalog::{InstalledPack, PackAvailability, PackCatalog, PackInstallPreview};
''',
)

replace_once(
    '''    fn open_external_path(&mut self, source: PathBuf, cx: &mut Context<Self>) {
        if let Some(document_id) = library_document_id_for_path(&source, &self.library) {
''',
    '''    fn open_external_path(&mut self, source: PathBuf, cx: &mut Context<Self>) {
        if is_world_pack_file(&source) {
            self.review_pack_path(source, None, cx);
            return;
        }
        if let Some(document_id) = library_document_id_for_path(&source, &self.library) {
''',
)

replace_once(
    '''        if !is_world_file(&source) {
            self.status = Some(format!(
                "Could not open {}: choose a {} file",
                source.display(),
                WORLD_DOCUMENT_SUFFIX
            ));
''',
    '''        if !is_world_file(&source) {
            self.status = Some(format!(
                "Could not open {}: choose a {} document or {} Pack",
                source.display(),
                WORLD_DOCUMENT_SUFFIX,
                PACK_BUNDLE_SUFFIX
            ));
''',
)

is_world_anchor = '''#[cfg(target_os = "macos")]
fn is_world_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.ends_with(WORLD_DOCUMENT_SUFFIX) || name.ends_with(LEGACY_WORLD_DOCUMENT_SUFFIX)
        })
}

'''
replace_once(
    is_world_anchor,
    is_world_anchor
    + '''#[cfg(target_os = "macos")]
fn is_world_pack_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(PACK_BUNDLE_SUFFIX))
}

#[cfg(all(test, target_os = "macos"))]
mod file_type_tests {
    use super::*;

    #[test]
    fn system_open_distinguishes_world_documents_from_portable_packs() {
        assert!(is_world_file(Path::new("/tmp/example.world")));
        assert!(!is_world_pack_file(Path::new("/tmp/example.world")));

        assert!(is_world_pack_file(Path::new("/tmp/example.worldpack")));
        assert!(!is_world_file(Path::new("/tmp/example.worldpack")));

        assert!(!is_world_pack_file(Path::new("/tmp/example.worldpack.backup")));
        assert!(!is_world_pack_file(Path::new("/tmp/example.world-pack.json")));
    }
}

''',
)

path.write_text(text)
