from pathlib import Path

path = Path("apps/world-machine-desktop/src/main.rs")
text = path.read_text()


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(
            f"expected one replacement anchor, found {count}: {old[:100]!r}"
        )
    text = text.replace(old, new, 1)


replace_once(
    '#[cfg(target_os = "macos")]\nmod observer;\n',
    '#[cfg(target_os = "macos")]\nmod included_packs;\n#[cfg(target_os = "macos")]\nmod observer;\n',
)

replace_once(
    "    lineage: Option<LineageIndex>,\n    pending_pack_install: Option<PackInstallPreview>,\n",
    "    lineage: Option<LineageIndex>,\n    included_packs: Vec<included_packs::IncludedPack>,\n    pending_pack_install: Option<PackInstallPreview>,\n",
)

rebuild_anchor = '''    fn rebuild_registry(&mut self) -> Result<(), String> {
        let registry = build_registry(self.pack_catalog.as_ref())?;
        self.registry = Arc::new(registry);
        Ok(())
    }

'''
helpers = rebuild_anchor + '''    fn review_pack_path(
        &mut self,
        source: PathBuf,
        expected_pack: Option<WorldPackRef>,
        cx: &mut Context<Self>,
    ) {
        if self.pack_catalog.is_none() {
            match PackCatalog::open(&self.pack_catalog_path) {
                Ok(catalog) => self.pack_catalog = Some(catalog),
                Err(error) => {
                    self.status = Some(format!(
                        "Could not open Installed Packs catalog {}: {error}",
                        self.pack_catalog_path.display()
                    ));
                    cx.notify();
                    return;
                }
            }
        }

        let catalog = self.pack_catalog.as_ref().unwrap();
        match catalog.inspect_install(&source) {
            Ok(preview) => {
                if expected_pack
                    .as_ref()
                    .is_some_and(|expected| preview.pack() != expected)
                {
                    let expected = expected_pack.unwrap();
                    self.pending_pack_install = None;
                    self.status = Some(format!(
                        "Included Pack identity mismatch: expected {} @ {}, found {} @ {}",
                        expected.id,
                        expected.version,
                        preview.pack().id,
                        preview.pack().version
                    ));
                    cx.notify();
                    return;
                }
                self.status = Some(format!(
                    "Review {} @ {} before trusting its executable bytes",
                    preview.pack().id,
                    preview.pack().version
                ));
                self.pending_pack_install = Some(preview);
            }
            Err(error) => {
                self.pending_pack_install = None;
                self.status = Some(format!("Could not inspect {}: {error}", source.display()));
            }
        }
        cx.notify();
    }

    fn review_included_pack(
        &mut self,
        pack: included_packs::IncludedPack,
        cx: &mut Context<Self>,
    ) {
        self.review_pack_path(pack.path, Some(pack.pack), cx);
    }

'''
replace_once(rebuild_anchor, helpers)

install_start = text.index("    fn install_pack(&mut self, cx: &mut Context<Self>) {")
install_end = text.index("\n    fn confirm_pack_install", install_start)
new_install = '''    fn install_pack(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Install World Pack".into()),
        });
        cx.spawn(async move |this, cx| {
            let source = match picker.await {
                Ok(Ok(Some(mut paths))) => paths.pop(),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(format!("Could not open Install Pack dialog: {error}"));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(format!("Install Pack dialog was interrupted: {error}"));
                        cx.notify();
                    });
                    return;
                }
            };
            let Some(source) = source else {
                return;
            };
            let _ = this.update(cx, |this, cx| this.review_pack_path(source, None, cx));
        })
        .detach();
    }
'''
text = text[:install_start] + new_install + text[install_end:]

review_anchor = "    fn pack_install_review_card(\n"
review_pos = text.index(review_anchor)
included_card = '''    fn included_pack_card(
        &self,
        pack: included_packs::IncludedPack,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let review_pack = pack.clone();
        let identity = format!("{} @ {}", pack.pack.id, pack.pack.version);
        div()
            .id(SharedString::from(format!(
                "included-pack-{}-{}",
                pack.pack.id, pack.pack.version
            )))
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xc8d5c0))
            .bg(rgb(0xf7fbf5))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child(pack.title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child(pack.description),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x75806f))
                            .child(format!(
                                "{identity} · Included external Pack · review required"
                            )),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from(format!(
                        "review-included-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x91a486))
                    .text_sm()
                    .child("Review & Install")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.review_included_pack(review_pack.clone(), cx)
                    })),
            )
    }

'''
text = text[:review_pos] + included_card + text[review_pos:]

installed_anchor = '''        let installed_packs = self
            .pack_catalog
            .as_ref()
            .map(|catalog| catalog.entries().to_vec())
            .unwrap_or_default();
'''
included_render = '''        let visible_included_packs = self
            .included_packs
            .iter()
            .filter(|included| {
                !self.pack_catalog.as_ref().is_some_and(|catalog| {
                    catalog
                        .entries()
                        .iter()
                        .any(|installed| installed.pack == included.pack)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut included = div().w_full().flex().flex_col().gap_3();
        if visible_included_packs.is_empty() {
            included = included.child(
                div()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xe1e1dc))
                    .text_sm()
                    .text_color(rgb(0x777770))
                    .child("All Worlds included with this app are already installed."),
            );
        } else {
            for pack in visible_included_packs {
                included = included.child(self.included_pack_card(pack, cx));
            }
        }

''' + installed_anchor
replace_once(installed_anchor, included_render)

old_sections = '''        body = body
            .child(div().text_sm().child("My Worlds"))
            .child(saved)
            .child(div().text_sm().child("Installed Packs"))
            .child(installed)
            .child(div().text_sm().child("New World"))
            .child(available);
'''
new_sections = '''        body = body
            .child(div().text_sm().child("My Worlds"))
            .child(saved);

        if !self.included_packs.is_empty() {
            body = body
                .child(div().text_sm().child("Included Worlds"))
                .child(included);
        }

        body = body
            .child(div().text_sm().child("Installed Packs"))
            .child(installed)
            .child(div().text_sm().child("New World"))
            .child(available);
'''
replace_once(old_sections, new_sections)

documents_anchor = "    let (documents, lineage, library_status) = match library.list() {\n"
included_discovery = '''    let (included_packs, included_status) = match included_packs::discover() {
        Ok(packs) => (packs, None),
        Err(error) => (
            Vec::new(),
            Some(format!("Could not locate included World Packs: {error}")),
        ),
    };
''' + documents_anchor
replace_once(documents_anchor, included_discovery)

replace_once(
    "    let status = pack_status.or(library_status);\n",
    "    let status = pack_status.or(library_status).or(included_status);\n",
)

replace_once(
    "                lineage,\n                pending_pack_install: None,\n",
    "                lineage,\n                included_packs,\n                pending_pack_install: None,\n",
)

path.write_text(text)
