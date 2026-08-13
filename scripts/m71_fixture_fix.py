from pathlib import Path

path = Path("apps/world-machine-desktop/src/main.rs")
text = path.read_text()
old = '''    fn starter_pack_index_rejects_traversal_nested_absolute_non_bundle_and_duplicates() {
        let root = temp_dir("invalid");
        for invalid in [
'''
new = '''    fn starter_pack_index_rejects_traversal_nested_absolute_non_bundle_and_duplicates() {
        let root = temp_dir("invalid");
        fs::write(root.join("same.worldpack"), b"same").unwrap();
        for invalid in [
'''
if text.count(old) != 1:
    raise SystemExit(f"expected one invalid-index test anchor, found {text.count(old)}")
path.write_text(text.replace(old, new, 1))
