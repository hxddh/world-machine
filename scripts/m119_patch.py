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
    '''    documents: Vec<WorldDocumentSummary>,
    lineage: Option<LineageIndex>,''',
    '''    documents: Vec<WorldDocumentSummary>,
    selected_world_pack: Option<String>,
    lineage: Option<LineageIndex>,''',
    "home filter state",
)

text = replace_once(
    text,
    '''        let count = documents.len();
        self.documents = documents;
        self.refresh_lineage()?;
        Ok(count)''',
    '''        let count = documents.len();
        self.documents = documents;
        if !world_pack_filter_is_available(
            &self.documents,
            self.selected_world_pack.as_deref(),
        ) {
            self.selected_world_pack = None;
        }
        self.refresh_lineage()?;
        Ok(count)''',
    "refresh stale Pack filter",
)

text = replace_once(
    text,
    '''    fn document_title_for_id(&self, document_id: &WorldDocumentId) -> Option<String> {''',
    '''    fn set_world_pack_filter(&mut self, pack_id: Option<String>, cx: &mut Context<Self>) {
        self.selected_world_pack = pack_id;
        cx.notify();
    }

    fn pack_filter_title(&self, pack_id: &str) -> String {
        self.registry
            .descriptor(pack_id)
            .map(|descriptor| descriptor.title.clone())
            .or_else(|| {
                self.documents
                    .iter()
                    .find(|document| document.pack.id == pack_id)
                    .and_then(|document| self.registry.descriptor_for(&document.pack))
                    .map(|descriptor| descriptor.title.clone())
            })
            .unwrap_or_else(|| pack_id.to_owned())
    }

    fn document_title_for_id(&self, document_id: &WorldDocumentId) -> Option<String> {''',
    "Pack filter actions",
)

text = replace_once(
    text,
    '''        let documents = self.documents.clone();
        let has_documents = !documents.is_empty();
        let descriptors = self''',
    '''        let documents = self.documents.clone();
        let has_documents = !documents.is_empty();
        let pack_filters = world_pack_filter_counts(&documents);
        let selected_world_pack = self.selected_world_pack.clone();
        let visible_documents = documents
            .iter()
            .filter(|document| {
                world_matches_pack_filter(document, selected_world_pack.as_deref())
            })
            .cloned()
            .collect::<Vec<_>>();
        let visible_document_count = visible_documents.len();
        let descriptors = self''',
    "visible filtered documents",
)

text = replace_once(
    text,
    '''        } else {
            for document in documents {
                saved = saved.child(self.document_card(document, cx));
            }
        }

        let visible_included_packs = self''',
    '''        } else if visible_documents.is_empty() {
            saved = saved.child(
                div()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xe1e1dc))
                    .text_sm()
                    .text_color(rgb(0x777770))
                    .child("No Worlds match this Pack filter."),
            );
        } else {
            for document in visible_documents {
                saved = saved.child(self.document_card(document, cx));
            }
        }

        let mut world_filters = div().w_full().flex().gap_2();
        if pack_filters.len() > 1 {
            let all_selected = selected_world_pack.is_none();
            let (all_border, all_background, all_text) = if all_selected {
                (0x6f86b0, 0xe9eef7, 0x314b72)
            } else {
                (0xd9d9d3, 0xffffff, 0x666666)
            };
            world_filters = world_filters.child(
                div()
                    .id("world-pack-filter-all")
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(all_border))
                    .bg(rgb(all_background))
                    .text_color(rgb(all_text))
                    .text_xs()
                    .child(format!("All · {}", documents.len()))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.set_world_pack_filter(None, cx)
                    })),
            );
            for (pack_id, count) in pack_filters.iter() {
                let selected = selected_world_pack.as_deref() == Some(pack_id.as_str());
                let (border, background, text) = if selected {
                    (0x6f86b0, 0xe9eef7, 0x314b72)
                } else {
                    (0xd9d9d3, 0xffffff, 0x666666)
                };
                let filter_pack = pack_id.clone();
                let filter_title = self.pack_filter_title(pack_id);
                world_filters = world_filters.child(
                    div()
                        .id(SharedString::from(format!("world-pack-filter-{pack_id}")))
                        .cursor_pointer()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(border))
                        .bg(rgb(background))
                        .text_color(rgb(text))
                        .text_xs()
                        .child(format!("{filter_title} · {count}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_world_pack_filter(Some(filter_pack.clone()), cx)
                        })),
                );
            }
        }

        let visible_included_packs = self''',
    "Pack filter chips",
)

text = replace_once(
    text,
    '''        if has_documents || self.included_packs.is_empty() {
            body = body.child(div().text_sm().child("My Worlds")).child(saved);
        }''',
    '''        if has_documents || self.included_packs.is_empty() {
            let worlds_title = if selected_world_pack.is_some() {
                format!(
                    "My Worlds · {visible_document_count}/{}",
                    documents.len()
                )
            } else {
                format!("My Worlds · {}", documents.len())
            };
            body = body.child(div().text_sm().child(worlds_title));
            if pack_filters.len() > 1 {
                body = body.child(world_filters);
            }
            body = body.child(saved);
        }''',
    "My Worlds filtered header",
)

text = replace_once(
    text,
    '''#[cfg(target_os = "macos")]
fn lineage_child_preview(children: &[WorldDocumentId]) -> (&[WorldDocumentId], usize) {
    let visible = children.len().min(LINEAGE_CHILD_PREVIEW_LIMIT);
    (&children[..visible], children.len() - visible)
}
''',
    '''#[cfg(target_os = "macos")]
fn lineage_child_preview(children: &[WorldDocumentId]) -> (&[WorldDocumentId], usize) {
    let visible = children.len().min(LINEAGE_CHILD_PREVIEW_LIMIT);
    (&children[..visible], children.len() - visible)
}

#[cfg(target_os = "macos")]
fn world_matches_pack_filter(document: &WorldDocumentSummary, pack_id: Option<&str>) -> bool {
    pack_id.is_none_or(|pack_id| document.pack.id == pack_id)
}

#[cfg(target_os = "macos")]
fn world_pack_filter_is_available(
    documents: &[WorldDocumentSummary],
    pack_id: Option<&str>,
) -> bool {
    pack_id.is_none_or(|pack_id| {
        documents
            .iter()
            .any(|document| document.pack.id == pack_id)
    })
}

#[cfg(target_os = "macos")]
fn world_pack_filter_counts(documents: &[WorldDocumentSummary]) -> Vec<(String, usize)> {
    let mut filters = Vec::<(String, usize)>::new();
    for document in documents {
        if let Some((_pack_id, count)) = filters
            .iter_mut()
            .find(|(pack_id, _count)| pack_id == &document.pack.id)
        {
            *count += 1;
        } else {
            filters.push((document.pack.id.clone(), 1));
        }
    }
    filters
}
''',
    "Pack filter helpers",
)

text = replace_once(
    text,
    '''    #[test]
    fn lineage_child_preview_is_bounded_without_losing_total_count() {''',
    '''    #[test]
    fn world_pack_filters_preserve_recent_pack_order_and_counts() {
        let summary = |id: &str, pack_id: &str| WorldDocumentSummary {
            id: WorldDocumentId::new(id).unwrap(),
            pack: WorldPackRef::new(pack_id, "1.0.0"),
            display_title: None,
            display_summary: None,
            world_time: 0,
            event_count: 0,
        };
        let documents = vec![
            summary("pocket-new", "pocket-universe"),
            summary("tiny", "tiny-society"),
            summary("pocket-old", "pocket-universe"),
            summary("company", "micro-company"),
        ];

        assert_eq!(
            world_pack_filter_counts(&documents),
            vec![
                ("pocket-universe".into(), 2),
                ("tiny-society".into(), 1),
                ("micro-company".into(), 1),
            ]
        );
        assert!(world_matches_pack_filter(&documents[0], None));
        assert!(world_matches_pack_filter(
            &documents[0],
            Some("pocket-universe")
        ));
        assert!(!world_matches_pack_filter(
            &documents[1],
            Some("pocket-universe")
        ));
        assert!(world_pack_filter_is_available(
            &documents,
            Some("tiny-society")
        ));
        assert!(!world_pack_filter_is_available(
            &documents,
            Some("missing-pack")
        ));
    }

    #[test]
    fn lineage_child_preview_is_bounded_without_losing_total_count() {''',
    "Pack filter regression",
)

text = replace_once(
    text,
    '''                documents,
                lineage,
                included_packs,''',
    '''                documents,
                selected_world_pack: None,
                lineage,
                included_packs,''',
    "home filter initialization",
)

path.write_text(text)
