from pathlib import Path

path = Path("crates/world-agent-tools/src/lib.rs")
text = path.read_text()


def remove_second_region(text: str, marker: str, end_marker: str, label: str) -> str:
    first = text.find(marker)
    if first < 0:
        raise SystemExit(f"missing first marker for {label}")
    second = text.find(marker, first + len(marker))
    if second < 0:
        return text
    end = text.find(end_marker, second)
    if end < 0:
        raise SystemExit(f"missing end marker for {label}")
    text = text[:second] + text[end:]
    if text.find(marker, text.find(marker) + len(marker)) >= 0:
        raise SystemExit(f"more than two copies found for {label}")
    return text


text = remove_second_region(
    text,
    "pub fn read_only_json_tool_catalog()",
    "#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\n#[serde(deny_unknown_fields)]",
    "catalog",
)
text = remove_second_region(
    text,
    "#[derive(Debug)]\npub enum JsonToolDispatchError",
    "#[cfg(test)]",
    "tool-set block",
)
text = remove_second_region(
    text,
    "    #[test]\n    fn read_only_catalog_is_deterministic_unique_and_read_only()",
    "    #[test]\n    fn output_is_serializable_without_exposing_executor_or_world_internals()",
    "M213 tests",
)

for marker, expected in [
    ("pub fn read_only_json_tool_catalog()", 1),
    ("pub enum JsonToolDispatchError", 1),
    ("pub trait ReadOnlyJsonToolSet", 1),
    ("pub struct WorldReadOnlyToolSet", 1),
    ("fn read_only_catalog_is_deterministic_unique_and_read_only()", 1),
    ("fn tool_set_lists_the_same_catalog_exposed_to_provider_adapters()", 1),
    ("fn tool_set_dispatches_known_name_through_existing_json_tool()", 1),
    ("fn unknown_tool_name_fails_before_executor_use()", 1),
]:
    actual = text.count(marker)
    if actual != expected:
        raise SystemExit(f"expected {expected} copy of {marker!r}, found {actual}")

path.write_text(text)
