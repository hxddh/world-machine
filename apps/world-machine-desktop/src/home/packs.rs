//! World Packs on Home: reviewing and installing a Pack, the Packs this
//! app ships, probing and activating them, turning them on and off, and
//! their cards.

#[allow(unused_imports)]
use super::*;

impl WorldMachineHome {
    pub(crate) fn rebuild_registry(&mut self) -> Result<(), String> {
        let registry = build_registry(self.pack_catalog.as_ref())?;
        self.registry = Arc::new(registry);
        Ok(())
    }

    pub(crate) fn review_pack_path(
        &mut self,
        source: PathBuf,
        expected_pack: Option<WorldPackRef>,
        start_after_install: bool,
        cx: &mut Context<Self>,
    ) {
        self.pending_start_after_install = None;
        if self.pack_catalog.is_none() {
            match PackCatalog::open(&self.pack_catalog_path) {
                Ok(catalog) => self.pack_catalog = Some(catalog),
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Could not open Installed Packs catalog {}: {error}",
                        self.pack_catalog_path.display()
                    )));
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
                    self.status = Some(HomeStatus::error(format!(
                        "Included Pack identity mismatch: expected {} @ {}, found {} @ {}",
                        expected.id,
                        expected.version,
                        preview.pack().id,
                        preview.pack().version
                    )));
                    cx.notify();
                    return;
                }
                self.status = Some(HomeStatus::info(format!(
                    "Review {} @ {} before trusting its executable bytes",
                    preview.pack().id,
                    preview.pack().version
                )));
                self.pending_start_after_install =
                    start_after_install.then(|| preview.pack().clone());
                self.pending_pack_install = Some(preview);
            }
            Err(error) => {
                self.pending_pack_install = None;
                self.status = Some(HomeStatus::error(format!(
                    "Could not inspect {}: {error}",
                    source.display()
                )));
            }
        }
        cx.notify();
    }

    pub(crate) fn review_included_pack(
        &mut self,
        pack: included_packs::IncludedPack,
        start_after_install: bool,
        cx: &mut Context<Self>,
    ) {
        self.review_pack_path(pack.path, Some(pack.pack), start_after_install, cx);
    }

    /// Install, probe, and activate the World Packs shipped inside this app
    /// bundle so a fresh install reaches a living World without a review
    /// dialog. The bundle is one code-signed unit, so these Packs carry the
    /// same trust as the app binary; the catalog still pins their exact
    /// content on install and the durable probe still runs before activation.
    /// User-supplied `.worldpack` files keep the explicit review flow.
    /// Packs that are already in the catalog, enabled or not, keep that state;
    /// only their program is brought up to the one this app ships.
    pub(crate) fn activate_included_packs(&mut self, cx: &mut Context<Self>) {
        let packs = self.included_packs.clone();
        for pack in packs {
            if self.included_pack_is_installed(&pack.pack) {
                self.refresh_included_pack(&pack);
                continue;
            }
            let Some(catalog) = self.pack_catalog.as_mut() else {
                return;
            };
            let preview = match catalog.inspect_install(&pack.path) {
                Ok(preview) => preview,
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Could not prepare {}: {error}",
                        pack.title
                    )));
                    continue;
                }
            };
            if preview.pack() != &pack.pack {
                self.status = Some(HomeStatus::error(format!(
                    "Could not prepare {}: bundle contains {} @ {}, expected {} @ {}",
                    pack.title,
                    preview.pack().id,
                    preview.pack().version,
                    pack.pack.id,
                    pack.pack.version
                )));
                continue;
            }
            match catalog.install_reviewed_pending_probe(&preview) {
                Ok(installed) => {
                    // A fresh install (no saved Worlds yet) opens straight
                    // into its first World once the featured Pack is ready;
                    // otherwise Home offers the Create handoff.
                    let create_now = pack.featured && self.documents.is_empty();
                    self.step_aside_for_first_world |= create_now;
                    self.start_pack_probe(
                        installed.pack,
                        true,
                        create_now,
                        pack.featured && !create_now,
                        cx,
                    );
                    self.status = Some(HomeStatus::info(format!(
                        "Preparing {} for its first launch…",
                        pack.title
                    )));
                }
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Could not prepare {}: {error}",
                        pack.title
                    )));
                }
            }
        }
        cx.notify();
    }

    /// An included Pack keeps its version when its program is fixed (the
    /// Worlds made with it are stamped with that version), so a catalog that
    /// installed it from an earlier copy of the app would keep the earlier
    /// program. Where the program or manifest this app ships differs, it
    /// replaces the installed one, which stays enabled and active as it was.
    pub(crate) fn refresh_included_pack(&mut self, pack: &included_packs::IncludedPack) {
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        match catalog.refresh_bundle(&pack.path) {
            Ok(PackRefresh::Replaced(installed)) => {
                diagnostics::info(format!(
                    "included pack {} @ {} replaced with the program this app ships",
                    installed.pack.id, installed.pack.version
                ));
                if let Err(error) = self.rebuild_registry() {
                    self.status = Some(HomeStatus::error(format!(
                        "{} was updated, but Registry rebuild failed: {error}",
                        pack.title
                    )));
                }
            }
            Ok(PackRefresh::Unchanged | PackRefresh::NotInstalled) => {}
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not update {}: {error}",
                    pack.title
                )));
            }
        }
    }

    pub(crate) fn install_pack(&mut self, cx: &mut Context<Self>) {
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
                        this.status = Some(HomeStatus::error(format!(
                            "Could not open Install Pack dialog: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Install Pack dialog was interrupted: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
            };
            let Some(source) = source else {
                return;
            };
            let _ = this.update(cx, |this, cx| {
                this.review_pack_path(source, None, false, cx)
            });
        })
        .detach();
    }

    pub(crate) fn confirm_pack_install(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = self.pending_pack_install.clone() else {
            return;
        };
        let start_after_install =
            start_after_install_matches(self.pending_start_after_install.as_ref(), preview.pack());
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        let result = catalog.install_reviewed_pending_probe(&preview);
        self.pending_pack_install = None;
        self.pending_start_after_install = None;
        self.ready_pack_to_create = None;
        match result {
            Ok(installed) => {
                self.start_pack_probe(installed.pack, true, start_after_install, true, cx);
            }
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Pack was not installed. Re-open it to review current content: {error}"
                )));
                cx.notify();
            }
        }
    }

    pub(crate) fn is_pack_probing(&self, pack: &WorldPackRef) -> bool {
        self.probing_packs.iter().any(|candidate| candidate == pack)
    }

    pub(crate) fn start_pack_probe(
        &mut self,
        pack: WorldPackRef,
        activate_on_success: bool,
        create_on_success: bool,
        offer_create_on_success: bool,
        cx: &mut Context<Self>,
    ) {
        if self.is_pack_probing(&pack) {
            return;
        }
        let Some(catalog) = self.pack_catalog.clone() else {
            return;
        };
        self.probing_packs.push(pack.clone());
        self.status = Some(HomeStatus::info(if create_on_success {
            "Testing this World before first launch…".into()
        } else if let Some(title) = self.included_pack_title(&pack) {
            format!("Preparing {title} for its first launch…")
        } else {
            format!(
                "Testing trusted Pack {} @ {} · Create → Archive → fresh-process Open…",
                pack.id, pack.version
            )
        }));
        cx.notify();

        let probe_pack = pack.clone();
        let task = cx
            .background_executor()
            .spawn(async move { catalog.probe(&probe_pack) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.probing_packs.retain(|candidate| candidate != &pack);
                match result {
                    Ok(probe) => {
                        let transition = this
                            .pack_catalog
                            .as_mut()
                            .ok_or_else(|| "Installed Packs catalog is unavailable".to_string())
                            .and_then(|catalog| {
                                catalog
                                    .set_enabled(&pack, true)
                                    .map_err(|error| error.to_string())?;
                                if activate_on_success {
                                    catalog.activate(&pack).map_err(|error| error.to_string())?;
                                }
                                Ok(())
                            });
                        match transition {
                            Ok(()) => match this.rebuild_registry() {
                                Ok(()) => {
                                    if activate_on_success && create_on_success {
                                        this.ready_pack_to_create = None;
                                        this.create_world(pack.id.clone(), cx);
                                        return;
                                    }
                                    if activate_on_success && offer_create_on_success {
                                        this.ready_pack_to_create = Some(pack.clone());
                                    }
                                    diagnostics::info(format!(
                                        "pack {} @ {} passed its durable probe · World time {} → {}",
                                        pack.id,
                                        pack.version,
                                        probe.created_world_time,
                                        probe.reopened_world_time
                                    ));
                                    // An included Pack getting ready in the background
                                    // is not news; the log keeps the probe's numbers.
                                    // A Pack the person installed themselves is.
                                    this.status = if this.included_pack_title(&pack).is_some() {
                                        None
                                    } else {
                                        let title = this
                                            .registry
                                            .descriptor_for(&pack)
                                            .map(|descriptor| descriptor.title.clone())
                                            .unwrap_or_else(|| pack.id.clone());
                                        Some(HomeStatus::success(format!(
                                            "{title} is installed and ready."
                                        )))
                                    };
                                }
                                Err(error) => {
                                    this.status = Some(HomeStatus::error(format!(
                                        "Pack {} @ {} passed its durable probe, but Registry rebuild failed: {error}",
                                        pack.id, pack.version
                                    )));
                                }
                            },
                            Err(error) => {
                                this.status = Some(HomeStatus::error(format!(
                                    "Pack {} @ {} passed its durable probe, but could not be enabled: {error}",
                                    pack.id, pack.version
                                )));
                            }
                        }
                    }
                    Err(error) => {
                        if this.ready_pack_to_create.as_ref() == Some(&pack) {
                            this.ready_pack_to_create = None;
                        }
                        let _ = this.rebuild_registry();
                        this.status = Some(HomeStatus::error(format!(
                            "Installed and trusted {} @ {}, but its durable activation probe failed. The Pack remains disabled: {error}",
                            pack.id, pack.version
                        )));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn included_pack_title(&self, pack: &WorldPackRef) -> Option<&'static str> {
        self.included_packs
            .iter()
            .find(|included| &included.pack == pack)
            .map(|included| included.title)
    }

    pub(crate) fn ready_pack_descriptor(&self) -> Option<world_host::WorldDescriptor> {
        let pack = self.ready_pack_to_create.as_ref()?;
        // "Start your first World" is untrue for someone who already has one.
        if self
            .documents
            .iter()
            .any(|document| document.pack.id == pack.id)
        {
            return None;
        }
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

    pub(crate) fn dismiss_ready_pack(&mut self, cx: &mut Context<Self>) {
        self.ready_pack_to_create = None;
        cx.notify();
    }

    pub(crate) fn cancel_pack_install(&mut self, cx: &mut Context<Self>) {
        self.pending_pack_install = None;
        self.pending_start_after_install = None;
        self.status = Some(HomeStatus::info(
            "Pack installation cancelled; no external code was installed.",
        ));
        cx.notify();
    }

    pub(crate) fn activate_pack(&mut self, pack: WorldPackRef, cx: &mut Context<Self>) {
        if self
            .ready_pack_to_create
            .as_ref()
            .is_some_and(|ready| ready.id == pack.id && ready != &pack)
        {
            self.ready_pack_to_create = None;
        }
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        match catalog.activate(&pack) {
            Ok(()) => match self.rebuild_registry() {
                Ok(()) => {
                    self.status = Some(HomeStatus::success(format!(
                        "Activated {} @ {} for new Worlds",
                        pack.id, pack.version
                    )))
                }
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Changed active Pack to {} @ {}, but Registry rebuild failed: {error}",
                        pack.id, pack.version
                    )))
                }
            },
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not activate {} @ {}: {error}",
                    pack.id, pack.version
                )))
            }
        }
        cx.notify();
    }

    pub(crate) fn set_pack_enabled(
        &mut self,
        pack: WorldPackRef,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        if !enabled && self.ready_pack_to_create.as_ref() == Some(&pack) {
            self.ready_pack_to_create = None;
        }
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        match catalog.set_enabled(&pack, enabled) {
            Ok(()) => match self.rebuild_registry() {
                Ok(()) => {
                    self.status = Some(HomeStatus::success(format!(
                        "{} {} @ {}",
                        if enabled { "Enabled" } else { "Disabled" },
                        pack.id,
                        pack.version
                    )))
                }
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Updated {} @ {}, but Registry rebuild failed: {error}",
                        pack.id, pack.version
                    )))
                }
            },
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not {} {} @ {}: {error}",
                    if enabled { "enable" } else { "disable" },
                    pack.id,
                    pack.version
                )))
            }
        }
        cx.notify();
    }

    pub(crate) fn missing_pack_message(&self, pack: &WorldPackRef) -> String {
        let availability = self
            .pack_catalog
            .as_ref()
            .map(|catalog| catalog.availability(pack))
            .unwrap_or(PackAvailability::NotInstalled);
        match availability {
            PackAvailability::Ready => format!(
                "Could not open World: {} @ {} should be available, but is not registered",
                pack.id, pack.version
            ),
            PackAvailability::Disabled => format!(
                "Could not open World: {} @ {} is installed but disabled. Use Test & Enable under Installed Packs.",
                pack.id, pack.version
            ),
            PackAvailability::Invalid { reason } => format!(
                "Could not open World: {} @ {} is installed but no longer matches its approved content: {reason}",
                pack.id, pack.version
            ),
            PackAvailability::MissingVersion { installed_versions } => format!(
                "Could not open World: it requires {} @ {}, but installed versions are {}. Install that exact Pack version.",
                pack.id,
                pack.version,
                installed_versions.join(", ")
            ),
            PackAvailability::NotInstalled => format!(
                "Could not open World: it requires {} @ {}, which is not installed. Use Install Pack… to add that exact version.",
                pack.id, pack.version
            ),
        }
    }

    pub(crate) fn included_pack_is_installed(&self, pack: &WorldPackRef) -> bool {
        self.pack_catalog.as_ref().is_some_and(|catalog| {
            catalog
                .entries()
                .iter()
                .any(|installed| &installed.pack == pack)
        })
    }

    pub(crate) fn featured_included_pack_card(
        &self,
        pack: included_packs::IncludedPack,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let review_pack = pack.clone();
        let identity = format!("{} @ {}", pack.pack.id, pack.pack.version);
        div()
            .id("featured-included-world")
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::BORDER_STRONG))
            .bg(ui::color(tokens::ACCENT_SOFT))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::ACCENT_TEXT))
                            .child("START HERE · A WORLD THAT GROWS A DAY AT A TIME"),
                    )
                    .child(div().text_lg().child(pack.title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child(pack.description),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(ui::color(tokens::ACCENT_TEXT))
                            .child(pack.experience),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::ACCENT_TEXT))
                            .child(format!(
                                "{identity} · Included external World · reviewed before it runs"
                            )),
                    ),
            )
            .child(
                ui::button(
                    "review-featured-included-world",
                    "Review & Start",
                    ui::ButtonKind::Secondary,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.review_included_pack(review_pack.clone(), true, cx)
                })),
            )
    }

    pub(crate) fn included_pack_card(
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
            .border_color(ui::color(tokens::BORDER_STRONG))
            .bg(ui::color(tokens::SUCCESS_SOFT))
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
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child(pack.description),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child(pack.experience),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_TERTIARY))
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
                    .border_color(ui::color(tokens::BORDER_STRONG))
                    .text_sm()
                    .child(ui::t("Review & Install"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.review_included_pack(review_pack.clone(), false, cx)
                    })),
            )
    }

    pub(crate) fn ready_pack_card(
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
            .rounded_lg()
            .border_color(ui::color(tokens::ACCENT))
            .bg(ui::color(tokens::ACCENT_SOFT))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                // Without a basis of its own this column asks for the whole
                // sentence on one line and pushes the buttons past the window
                // edge at the size Home opens at, which put the app's own
                // first-run call to action half off screen. flex_1 alone is not
                // enough: a flex item's automatic minimum size is its content,
                // so the column still refuses to go narrower than the sentence
                // until min_w_0 says it may.
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(ui::heading(format!("{title} is ready")))
                    .child(ui::body(
                        "Start your first World. It grows a day at a time, and you can always create another.",
                    )),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .gap_2()
                    .child(
                        ui::button("create-ready-pack-world", button_title, ui::ButtonKind::Primary)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.create_world(pack_id.clone(), cx)
                            })),
                    )
                    .child(
                        ui::button("dismiss-ready-pack-world", "Not now", ui::ButtonKind::Secondary)
                            .on_click(cx.listener(|this, _, _, cx| this.dismiss_ready_pack(cx))),
                    ),
            )
    }

    pub(crate) fn pack_install_review_card(
        &self,
        preview: PackInstallPreview,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let start_after_install =
            start_after_install_matches(self.pending_start_after_install.as_ref(), preview.pack());
        let review_title = if start_after_install {
            "Before this World starts"
        } else {
            "A new Pack wants to install"
        };
        let confirm_title = if start_after_install {
            "Trust & Start"
        } else {
            "Install & Trust"
        };
        let format = preview.kind().label();
        let size = format_program_size(preview.program_bytes());
        let source = preview.source_path();
        let file = source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| source.display().to_string());
        let version = preview.pack().version.clone();
        let identity = preview.pack().id.clone();
        let runtime = preview.runtime_name().to_owned();
        let sha = preview.program_sha256().to_owned();
        // The first and last few characters are what anyone compares.
        let fingerprint = if sha.len() > 16 {
            format!("{}…{}", &sha[..8], &sha[sha.len() - 8..])
        } else {
            sha.clone()
        };
        let fact = |label: &'static str, value: String| {
            div()
                .flex()
                .gap_3()
                .child(div().w(px(96.0)).flex_shrink_0().child(ui::caption(label)))
                .child(
                    div()
                        .min_w(px(0.0))
                        .flex_1()
                        .child(ui::body(value).truncate()),
                )
        };

        div()
            .id("pack-install-review")
            .w_full()
            .p_5()
            .rounded_lg()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::SURFACE))
            .shadow_sm()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .size(px(64.0))
                            .flex_shrink_0()
                            .rounded_lg()
                            .overflow_hidden()
                            .child(ui::cover(preview.title(), &identity).size_full()),
                    )
                    .child(
                        div()
                            .min_w(px(0.0))
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(ui::section_label(review_title))
                            .child(ui::heading(preview.title().to_owned()))
                            .child(ui::caption(preview.description().to_owned())),
                    ),
            )
            .child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(ui::color(tokens::WINDOW))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(fact("Version", version))
                    .child(fact("Runs as", format!("{runtime} · {format} · {size}")))
                    .child(fact("Fingerprint", fingerprint))
                    .child(fact("From", file))
                    .child(fact("Identity", identity)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .size(px(8.0))
                            .flex_shrink_0()
                            .rounded_full()
                            .bg(ui::color(tokens::WARNING)),
                    )
                    .child(ui::body(if start_after_install {
                        "Nothing from this Pack has run yet. Trusting it approves exactly these bytes, then your World opens once its self-test passes."
                    } else {
                        "Nothing from this Pack has run yet. Trusting it approves exactly these bytes; any other version is refused."
                    })),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        ui::button("cancel-pack-install", "Cancel", ui::ButtonKind::Secondary)
                            .on_click(cx.listener(|this, _, _, cx| this.cancel_pack_install(cx))),
                    )
                    .child(
                        ui::button("confirm-pack-install", confirm_title, ui::ButtonKind::Primary)
                            .on_click(cx.listener(|this, _, _, cx| this.confirm_pack_install(cx))),
                    ),
            )
    }

    pub(crate) fn installed_pack_card(
        &self,
        pack: InstalledPack,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let availability = self
            .pack_catalog
            .as_ref()
            .map(|catalog| catalog.availability(&pack.pack))
            .unwrap_or(PackAvailability::NotInstalled);
        let probing = self.is_pack_probing(&pack.pack);
        let state = if probing {
            "Testing durable round-trip…".to_string()
        } else {
            match &availability {
                PackAvailability::Ready if pack.active => "Active".to_string(),
                PackAvailability::Ready => "Historical · available for saved Worlds".to_string(),
                PackAvailability::Disabled => {
                    "Disabled · trusted, not runnable until tested".to_string()
                }
                PackAvailability::Invalid { reason } => format!("Invalid · {reason}"),
                PackAvailability::MissingVersion { .. } => "Missing exact version".to_string(),
                PackAvailability::NotInstalled => "Not installed".to_string(),
            }
        };
        let activate_pack = pack.pack.clone();
        let toggle_pack = pack.pack.clone();
        let test_pack = pack.pack.clone();
        let enabled = pack.enabled;
        let active = pack.active;

        let mut actions = div().flex().gap_2();
        if !probing && enabled && !active {
            actions = actions.child(
                div()
                    .id(SharedString::from(format!(
                        "activate-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(ui::color(tokens::BORDER_STRONG))
                    .text_sm()
                    .child(ui::t("Activate"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.activate_pack(activate_pack.clone(), cx)
                    })),
            );
        }
        if !probing && enabled {
            actions = actions.child(
                div()
                    .id(SharedString::from(format!(
                        "toggle-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(ui::color(tokens::BORDER_STRONG))
                    .text_sm()
                    .child(ui::t("Disable"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_pack_enabled(toggle_pack.clone(), false, cx)
                    })),
            );
        } else if !probing && !enabled {
            actions = actions.child(
                div()
                    .id(SharedString::from(format!(
                        "test-enable-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(ui::color(tokens::BORDER_STRONG))
                    .text_sm()
                    .child(ui::t("Test & Enable"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.start_pack_probe(test_pack.clone(), false, false, false, cx)
                    })),
            );
        }

        div()
            .id(SharedString::from(format!(
                "installed-pack-{}-{}",
                pack.pack.id, pack.pack.version
            )))
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::BORDER_STRONG))
            .bg(ui::color(tokens::SURFACE))
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
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child(pack.description),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_TERTIARY))
                            .child(format!(
                                "{} @ {} · {state}",
                                pack.pack.id, pack.pack.version
                            )),
                    ),
            )
            .child(actions)
    }
}
