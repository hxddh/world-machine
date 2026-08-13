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
    '''    included_packs: Vec<included_packs::IncludedPack>,
    pending_pack_install: Option<PackInstallPreview>,
    probing_packs: Vec<WorldPackRef>,
''',
    '''    included_packs: Vec<included_packs::IncludedPack>,
    pending_pack_install: Option<PackInstallPreview>,
    ready_pack_to_create: Option<WorldPackRef>,
    probing_packs: Vec<WorldPackRef>,
''',
)

replace_once(
    '''        let result = catalog.install_reviewed_pending_probe(&preview);
        self.pending_pack_install = None;
''',
    '''        let result = catalog.install_reviewed_pending_probe(&preview);
        self.pending_pack_install = None;
        self.ready_pack_to_create = None;
''',
)

replace_once(
    '''                                Ok(()) => {
                                    this.status = Some(format!(
                                        "Trusted and tested {} @ {} · durable Create/Archive/Open succeeded · World time {} → {}",
                                        pack.id,
                                        pack.version,
                                        probe.created_world_time,
                                        probe.reopened_world_time
                                    ));
                                }
''',
    '''                                Ok(()) => {
                                    if activate_on_success {
                                        this.ready_pack_to_create = Some(pack.clone());
                                    }
                                    this.status = Some(format!(
                                        "Trusted and tested {} @ {} · durable Create/Archive/Open succeeded · World time {} → {}",
                                        pack.id,
                                        pack.version,
                                        probe.created_world_time,
                                        probe.reopened_world_time
                                    ));
                                }
''',
)

replace_once(
    '''                    Err(error) => {
                        let _ = this.rebuild_registry();
                        this.status = Some(format!(
                            "Installed and trusted {} @ {}, but its durable activation probe failed. The Pack remains disabled: {error}",
                            pack.id, pack.version
                        ));
                    }
''',
    '''                    Err(error) => {
                        if this.ready_pack_to_create.as_ref() == Some(&pack) {
                            this.ready_pack_to_create = None;
                        }
                        let _ = this.rebuild_registry();
                        this.status = Some(format!(
                            "Installed and trusted {} @ {}, but its durable activation probe failed. The Pack remains disabled: {error}",
                            pack.id, pack.version
                        ));
                    }
''',
)

replace_once(
    '''    fn cancel_pack_install(&mut self, cx: &mut Context<Self>) {
''',
    '''    fn ready_pack_descriptor(&self) -> Option<world_host::WorldDescriptor> {
        let pack = self.ready_pack_to_create.as_ref()?;
        let catalog = self.pack_catalog.as_ref()?;
        let installed = catalog.entry(pack)?;
        if !installed.enabled || !installed.active {
            return None;
        }
        if !matches!(catalog.availability(pack), PackAvailability::Ready) {
            return None;
        }
        self.registry.descriptor_for(pack).cloned()
    }

    fn dismiss_ready_pack(&mut self, cx: &mut Context<Self>) {
        self.ready_pack_to_create = None;
        cx.notify();
    }

    fn cancel_pack_install(&mut self, cx: &mut Context<Self>) {
''',
)

replace_once(
    '''    fn activate_pack(&mut self, pack: WorldPackRef, cx: &mut Context<Self>) {
''',
    '''    fn activate_pack(&mut self, pack: WorldPackRef, cx: &mut Context<Self>) {
        if self
            .ready_pack_to_create
            .as_ref()
            .is_some_and(|ready| ready.id == pack.id && ready != &pack)
        {
            self.ready_pack_to_create = None;
        }
''',
)

replace_once(
    '''    fn set_pack_enabled(&mut self, pack: WorldPackRef, enabled: bool, cx: &mut Context<Self>) {
''',
    '''    fn set_pack_enabled(&mut self, pack: WorldPackRef, enabled: bool, cx: &mut Context<Self>) {
        if !enabled && self.ready_pack_to_create.as_ref() == Some(&pack) {
            self.ready_pack_to_create = None;
        }
''',
)

replace_once(
    '''    fn create_world(&mut self, pack_id: String, cx: &mut Context<Self>) {
        let title = self
            .registry
            .descriptor(&pack_id)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| pack_id.clone());
        let document_id = match new_document_id(&pack_id, &self.library) {
            Ok(id) => id,
            Err(error) => {
                self.status = Some(format!("Could not create {title}: {error}"));
                cx.notify();
                return;
            }
        };
        let session =
            match DurableWorldSession::create(document_id, &pack_id, &self.registry, &self.library)
            {
                Ok(session) => session,
                Err(error) => {
                    self.status = Some(format!("Could not create {title}: {error}"));
                    cx.notify();
                    return;
                }
            };
        self.refresh_documents();
        self.open_session(session, title, cx);
    }
''',
    '''    fn create_world(&mut self, pack_id: String, cx: &mut Context<Self>) {
        let title = self
            .registry
            .descriptor(&pack_id)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| pack_id.clone());
        let document_id = match new_document_id(&pack_id, &self.library) {
            Ok(id) => id,
            Err(error) => {
                self.status = Some(format!("Could not create {title}: {error}"));
                cx.notify();
                return;
            }
        };
        let session =
            match DurableWorldSession::create(document_id, &pack_id, &self.registry, &self.library)
            {
                Ok(session) => session,
                Err(error) => {
                    self.status = Some(format!("Could not create {title}: {error}"));
                    cx.notify();
                    return;
                }
            };
        self.refresh_documents();
        if self
            .ready_pack_to_create
            .as_ref()
            .is_some_and(|ready| ready.id == pack_id)
        {
            self.ready_pack_to_create = None;
        }
        self.open_session(session, title, cx);
    }
''',
)

review_anchor = '''    fn pack_install_review_card(
'''
ready_card = '''    fn ready_pack_card(
        &self,
        descriptor: world_host::WorldDescriptor,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pack_id = descriptor.pack.id.clone();
        let title = descriptor.title.clone();
        let button_title = format!("Create {}", descriptor.title);
        div()
            .id("pack-ready-to-create")
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x8eb58a))
            .bg(rgb(0xf1f8ee))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child(format!("{title} is ready")))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x52604d))
                            .child("The Pack passed its durable probe and is active for new Worlds."),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x75806f))
                            .child(format!(
                                "{} @ {} · no World has been created yet",
                                descriptor.pack.id, descriptor.pack.version
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("create-ready-pack-world")
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x6f966b))
                            .text_sm()
                            .child(button_title)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.create_world(pack_id.clone(), cx)
                            })),
                    )
                    .child(
                        div()
                            .id("dismiss-ready-pack-world")
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0xcbd8c7))
                            .text_sm()
                            .child("Not now")
                            .on_click(cx.listener(|this, _, _, cx| this.dismiss_ready_pack(cx))),
                    ),
            )
    }

'''
replace_once(review_anchor, ready_card + review_anchor)

replace_once(
    '''        if let Some(preview) = self.pending_pack_install.clone() {
            body = body.child(self.pack_install_review_card(preview, cx));
        }

        body = body.child(div().text_sm().child("My Worlds")).child(saved);
''',
    '''        if let Some(preview) = self.pending_pack_install.clone() {
            body = body.child(self.pack_install_review_card(preview, cx));
        }

        if let Some(descriptor) = self.ready_pack_descriptor() {
            body = body.child(self.ready_pack_card(descriptor, cx));
        }

        body = body.child(div().text_sm().child("My Worlds")).child(saved);
''',
)

replace_once(
    '''                included_packs,
                pending_pack_install: None,
                probing_packs: Vec::new(),
''',
    '''                included_packs,
                pending_pack_install: None,
                ready_pack_to_create: None,
                probing_packs: Vec::new(),
''',
)

path.write_text(text)
