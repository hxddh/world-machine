from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


path = Path("apps/world-machine-desktop/src/main.rs")
text = path.read_text()
text = replace_once(
    text,
    '''#[cfg(target_os = "macos")]
const PACK_CATALOG_OVERRIDE_ENV: &str = "WORLD_MACHINE_PACK_CATALOG";
''',
    '''#[cfg(target_os = "macos")]
const PACK_CATALOG_OVERRIDE_ENV: &str = "WORLD_MACHINE_PACK_CATALOG";
#[cfg(target_os = "macos")]
const LINEAGE_CHILD_PREVIEW_LIMIT: usize = 4;
''',
    "lineage preview limit",
)
text = replace_once(
    text,
    '''                for child_id in &node.children {
                    let child_label = child_id.to_string();''',
    '''                let (visible_children, hidden_children) = lineage_child_preview(&node.children);
                for child_id in visible_children {
                    let child_label = child_id.to_string();''',
    "bounded child loop",
)
text = replace_once(
    text,
    '''                    );
                }
                details = details.child(branches);
            }
        }
''',
    '''                    );
                }
                if hidden_children > 0 {
                    branches = branches.child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777770))
                            .child(format!(
                                "+{hidden_children} more branches · listed as their own Worlds"
                            )),
                    );
                }
                details = details.child(branches);
            }
        }
''',
    "hidden branch notice",
)
text = replace_once(
    text,
    '''#[cfg(target_os = "macos")]
fn world_summary_description(document: &WorldDocumentSummary) -> Option<String> {
    document
        .display_summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .map(str::to_owned)
}
''',
    '''#[cfg(target_os = "macos")]
fn world_summary_description(document: &WorldDocumentSummary) -> Option<String> {
    document
        .display_summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .map(str::to_owned)
}

#[cfg(target_os = "macos")]
fn lineage_child_preview(children: &[WorldDocumentId]) -> (&[WorldDocumentId], usize) {
    let visible = children.len().min(LINEAGE_CHILD_PREVIEW_LIMIT);
    (&children[..visible], children.len() - visible)
}
''',
    "lineage preview helper",
)
text = replace_once(
    text,
    '''    #[test]
    fn semantic_world_titles_prefer_document_metadata() {''',
    '''    #[test]
    fn lineage_child_preview_is_bounded_without_losing_total_count() {
        let children = (0..6)
            .map(|index| WorldDocumentId::new(format!("child-{index}")).unwrap())
            .collect::<Vec<_>>();
        let (visible, hidden) = lineage_child_preview(&children);

        assert_eq!(visible.len(), LINEAGE_CHILD_PREVIEW_LIMIT);
        assert_eq!(visible[0].as_str(), "child-0");
        assert_eq!(visible[3].as_str(), "child-3");
        assert_eq!(hidden, 2);
    }

    #[test]
    fn semantic_world_titles_prefer_document_metadata() {''',
    "lineage preview regression",
)
path.write_text(text)
