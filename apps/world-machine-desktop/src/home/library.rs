//! The player's Worlds on Home: reading the Library, creating, opening,
//! importing, exporting and restoring Worlds, renaming and removing
//! them, and how they are found, ordered and named.

#[allow(unused_imports)]
use super::*;

impl WorldMachineHome {
    pub(crate) fn refresh_documents(&mut self) -> Result<usize, HomeStatus> {
        let listing = self
            .library
            .listing()
            .map_err(|error| HomeStatus::error(format!("Could not read World Library: {error}")))?;
        let count = listing.documents.len();
        self.documents = listing.documents;
        self.unreadable_documents = listing.unreadable;
        report_unreadable_documents(&self.unreadable_documents);
        // A World that is gone from the Library cannot still be mid-rename or
        // mid-removal on a card.
        let documents = &self.documents;
        let renaming_is_gone = self.renaming.as_ref().is_some_and(|draft| {
            !documents
                .iter()
                .any(|document| document.id == draft.document)
        });
        let removal_is_gone = self
            .pending_removal
            .as_ref()
            .is_some_and(|pending| !documents.iter().any(|document| &document.id == pending));
        if renaming_is_gone {
            self.renaming = None;
        }
        if removal_is_gone {
            self.pending_removal = None;
        }
        if !world_pack_filter_is_available(&self.documents, self.selected_world_pack.as_deref()) {
            self.selected_world_pack = None;
        }
        Ok(count)
    }

    pub(crate) fn refresh_from_menu(&mut self, cx: &mut Context<Self>) {
        self.status = Some(match self.refresh_documents() {
            Ok(count) => HomeStatus::success(format!("My Worlds · {count} World(s)")),
            Err(status) => status,
        });
        cx.notify();
    }

    pub(crate) fn sync_documents_after_mutation(&mut self) -> Option<HomeStatus> {
        self.refresh_documents().err()
    }

    /// Opens a World window.
    pub(crate) fn open_session(
        &mut self,
        mut session: DurableWorldSession,
        title: String,
        cx: &mut Context<Self>,
    ) {
        let is_library_world = session.document_id().is_some();
        // One day a day (v0.30 S4): the sitting this opening is, from the
        // device's ledger; a new one brings the World's morning. Time away
        // no longer moves a World on its own. The demo keeps sittings too:
        // its first session is the first sitting, and its second morning
        // the next, begun at once with Sleep now (it plays freely).
        let sat = observer::sit(&mut session, &self.registry, &self.library);
        let sync_error = if is_library_world && matches!(&sat, Ok(sat) if sat.morning) {
            self.refresh_documents().err()
        } else {
            None
        };
        let (ledger, sitting_error) = match sat {
            Ok(sat) => (Some(SittingKept::Ledger(sat.ledger())), None),
            Err(error) => (Some(SittingKept::Unreadable), Some(error)),
        };
        let registry = Arc::clone(&self.registry);
        let library = Arc::clone(&self.library);
        let bounds = remembered_window_bounds(RememberedWindow::World, cx)
            .unwrap_or_else(|| Bounds::centered(None, size(px(1100.0), px(900.0)), cx));
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                watch_appearance(window);
                let view =
                    cx.new(|cx| WorldDocumentView::new(session, registry, library, ledger, cx));
                ask_before_closing(view, window, cx)
            },
        );

        // A window opening is its own confirmation; Home only speaks up when
        // there is something the window does not already say.
        self.status = match opened {
            Ok(_) => sitting_error.map(|error| {
                HomeStatus::info(format!(
                    "Opened {title}, but its sittings could not be read: {error}"
                ))
            }),
            Err(error) => Some(HomeStatus::error(format!(
                "Could not open {title}: {error}"
            ))),
        };
        if let Some(status) = sync_error {
            self.status = Some(status);
        }
        cx.notify();
    }

    pub(crate) fn create_world(&mut self, pack_id: String, cx: &mut Context<Self>) {
        // A World of this kind that was opened but never begun is picked up
        // where it was left rather than joined by another empty one.
        if let Some(unbegun) = self
            .documents
            .iter()
            .find(|document| document.pack.id == pack_id && !has_begun(document))
            .map(|document| document.id.clone())
        {
            self.open_document(unbegun, cx);
            self.step_aside_if_first_world(cx);
            return;
        }
        let title = self
            .registry
            .descriptor(&pack_id)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| pack_id.clone());
        let document_id = match new_document_id(&pack_id, &self.library) {
            Ok(id) => id,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not create {title}: {error}"
                )));
                cx.notify();
                return;
            }
        };
        let session =
            match DurableWorldSession::create(document_id, &pack_id, &self.registry, &self.library)
            {
                Ok(session) => session,
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Could not create {title}: {error}"
                    )));
                    cx.notify();
                    return;
                }
            };
        let sync_error = self.sync_documents_after_mutation();
        if self
            .ready_pack_to_create
            .as_ref()
            .is_some_and(|ready| ready.id == pack_id)
        {
            self.ready_pack_to_create = None;
        }
        self.open_session(session, title, cx);
        self.step_aside_if_first_world(cx);
        if let Some(status) = sync_error {
            self.status = Some(status);
            cx.notify();
        }
    }

    /// Close Home once the first World of a first launch is open; it comes
    /// back when that World's window closes.
    pub(crate) fn step_aside_if_first_world(&mut self, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.step_aside_for_first_world) {
            return;
        }
        cx.defer(|cx| {
            let world_open = cx
                .windows()
                .iter()
                .any(|window| window.downcast::<WorldDocumentView>().is_some());
            if !world_open {
                return;
            }
            for window in cx.windows() {
                if let Some(home) = window.downcast::<WorldMachineHome>() {
                    let _ = home.update(cx, |_, window, _| window.remove_window());
                }
            }
        });
    }

    pub(crate) fn open_document(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        // One window, and one writer, per World: an open one comes forward.
        if focus_open_world(cx, &document_id) {
            return;
        }
        let summary = self
            .documents
            .iter()
            .find(|document| document.id == document_id);
        if let Some(document) = summary {
            if self.registry.descriptor_for(&document.pack).is_none() {
                // The 1.0 save contract: a World a newer Pack saved is
                // refused as a newer World Machine's, not as a Pack to find.
                let refused = self
                    .registry
                    .descriptor(&document.pack.id)
                    .and_then(|active| {
                        match world_library::pack_version_age(
                            &active.pack.version,
                            &document.pack.version,
                        ) {
                            world_library::FormatAge::Newer => Some(LibraryError::NewerSave),
                            world_library::FormatAge::Preview => Some(LibraryError::PreviewSave),
                            world_library::FormatAge::Current => None,
                        }
                    });
                self.status = Some(HomeStatus::error(match refused {
                    Some(refused) => refused.to_string(),
                    None => self.missing_pack_message(&document.pack),
                }));
                cx.notify();
                return;
            }
        }
        let title = summary
            .and_then(|document| self.registry.descriptor_for(&document.pack))
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| document_id.to_string());
        let session =
            match DurableWorldSession::open(document_id.clone(), &self.registry, &self.library) {
                Ok(session) => session,
                Err(error) => {
                    let target = RestoreTarget {
                        path: self.library.path(&document_id),
                        title: title.clone(),
                        document: Some(document_id),
                    };
                    self.status = Some(HomeStatus::open_failed(
                        format!("Could not open {title}: {error}"),
                        &error,
                        target,
                    ));
                    cx.notify();
                    return;
                }
            };
        self.open_session(session, title, cx);
    }

    pub(crate) fn open_external_path(&mut self, source: PathBuf, cx: &mut Context<Self>) {
        if sharing::is_world_code_file(&source) {
            self.visit_path(&source, cx);
            return;
        }
        if is_world_pack_file(&source) {
            self.review_pack_path(source, None, false, cx);
            return;
        }
        if let Some(document_id) = library_document_id_for_path(&source, &self.library) {
            self.open_document(document_id, cx);
            return;
        }
        if !is_world_file(&source) {
            self.status = Some(HomeStatus::error(format!(
                "Could not open {}: choose a {} document or {} Pack",
                source.display(),
                WORLD_DOCUMENT_SUFFIX,
                PACK_BUNDLE_SUFFIX
            )));
            cx.notify();
            return;
        }
        // A World file already open here comes forward rather than opening
        // twice (it has one writer).
        let open = open_world_where(cx, |session| session.file_path() == Some(source.as_path()));
        if focus_world(cx, open) {
            return;
        }
        let session = match DurableWorldSession::open_file(source.clone(), &self.registry) {
            Ok(session) => session,
            Err(error) => {
                let target = RestoreTarget {
                    path: source.clone(),
                    title: source.display().to_string(),
                    document: None,
                };
                self.status = Some(HomeStatus::open_failed(
                    format!("Could not open {}: {error}", source.display()),
                    &error,
                    target,
                ));
                cx.notify();
                return;
            }
        };
        let pack = session.pack();
        let title = self
            .registry
            .descriptor_for(&pack)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or(pack.id);
        self.open_session(session, title, cx);
    }

    /// Puts a World that failed to load back to its last good save, and
    /// opens it.
    pub(crate) fn restore_last_good(&mut self, target: RestoreTarget, cx: &mut Context<Self>) {
        match world_machine_desktop::restore::restore_last_good(&target.path) {
            Ok(_) => {
                if let Some(status) = self.sync_documents_after_mutation() {
                    self.status = Some(status);
                    cx.notify();
                    return;
                }
                match target.document {
                    Some(id) => self.open_document(id, cx),
                    None => self.open_external_path(target.path.clone(), cx),
                }
                // Opened, it says so; a World that still fails says why.
                if self.status.is_none() {
                    self.status = Some(HomeStatus::success(
                        world_machine_desktop::restore::restored_message(&target.title),
                    ));
                }
            }
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not restore {}: {error}",
                    target.title
                )));
            }
        }
        cx.notify();
    }

    pub(crate) fn import_path(&mut self, source: PathBuf, cx: &mut Context<Self>) {
        if !is_world_file(&source) {
            self.status = Some(HomeStatus::error(format!(
                "Could not import {}: choose a {} file",
                source.display(),
                WORLD_DOCUMENT_SUFFIX
            )));
            cx.notify();
            return;
        }
        let document_id = match imported_document_id(&source, &self.library) {
            Ok(id) => id,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not import {}: {error}",
                    source.display()
                )));
                cx.notify();
                return;
            }
        };
        let session = match DurableWorldSession::import_file(
            document_id,
            &source,
            &self.registry,
            &self.library,
        ) {
            Ok(session) => session,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not import {}: {error}",
                    source.display()
                )));
                cx.notify();
                return;
            }
        };
        // A file sat with before keeps its sittings: they are in the file.
        let pack = session.pack();
        let title = self
            .registry
            .descriptor_for(&pack)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or(pack.id);
        let sync_error = self.sync_documents_after_mutation();
        self.open_session(session, title, cx);
        if let Some(status) = sync_error {
            self.status = Some(status);
            cx.notify();
        }
    }

    pub(crate) fn import_world(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import World".into()),
        });
        cx.spawn(async move |this, cx| {
            let source = match picker.await {
                Ok(Ok(Some(mut paths))) => paths.pop(),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    diagnostics::error(format!("import dialog: {error}"));
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(
                            world_machine_desktop::plain_errors::no_file_window(),
                        ));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    diagnostics::error(format!("import dialog: {error}"));
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(
                            world_machine_desktop::plain_errors::file_window_closed(),
                        ));
                        cx.notify();
                    });
                    return;
                }
            };
            let Some(source) = source else {
                return;
            };
            let _ = this.update(cx, |this, cx| this.import_path(source, cx));
        })
        .detach();
    }

    pub(crate) fn export_document(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        let semantic_title = self
            .documents
            .iter()
            .find(|document| document.id == document_id)
            .and_then(|document| document.display_title.as_deref())
            .unwrap_or_default();
        let suggested_name = suggested_world_file_name(semantic_title, document_id.as_str());
        let save_dialog = cx.prompt_for_new_path(&PathBuf::default(), Some(&suggested_name));
        cx.spawn(async move |this, cx| {
            let mut without_dialog = false;
            let destination = match save_dialog.await {
                Ok(Ok(Some(path))) => canonical_world_path(path),
                Ok(Ok(None)) => return,
                // No Save window on this computer: exported to Documents
                // anyway, and Home says where.
                Ok(Err(error)) => {
                    diagnostics::error(format!("export dialog: {error}"));
                    without_dialog = true;
                    match world_machine_desktop::plain_errors::fallback_folder() {
                        Ok(folder) => {
                            canonical_world_path(world_machine_desktop::plain_errors::free_path(
                                &folder,
                                &suggested_name,
                            ))
                        }
                        Err(error) => {
                            diagnostics::error(format!("export: {error}"));
                            let _ = this.update(cx, |this, cx| {
                                this.status = Some(HomeStatus::error(
                                    world_machine_desktop::plain_errors::not_saved_without_dialog(),
                                ));
                                cx.notify();
                            });
                            return;
                        }
                    }
                }
                Err(error) => {
                    diagnostics::error(format!("export dialog: {error}"));
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(
                            world_machine_desktop::plain_errors::file_window_closed(),
                        ));
                        cx.notify();
                    });
                    return;
                }
            };

            let _ = this.update(cx, |this, cx| {
                // An open World is exported through its session, its latest
                // turns written first.
                let exported = match open_world_document(cx, &document_id) {
                    Some(document) => match document.try_borrow() {
                        Ok(document) => document
                            .session
                            .export_file(&destination, &document.library),
                        Err(_) => Err(LibraryError::InUse(this.library.path(&document_id))),
                    },
                    None => this.library.export_file(&document_id, &destination),
                };
                match exported {
                    Ok(()) if without_dialog => {
                        this.status = Some(HomeStatus::success(
                            world_machine_desktop::plain_errors::saved_without_dialog(&destination),
                        ));
                    }
                    Ok(()) => {
                        this.status = Some(HomeStatus::success(format!(
                            "Exported {} to {}",
                            document_id,
                            destination.display()
                        )));
                    }
                    Err(error) => {
                        diagnostics::error(format!("export {document_id}: {error}"));
                        this.status = Some(HomeStatus::error(
                            world_machine_desktop::plain_errors::could_not(
                                "Couldn't export this World.",
                                &error.to_string(),
                            ),
                        ));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn set_world_pack_filter(
        &mut self,
        pack_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.selected_world_pack = pack_id;
        cx.notify();
    }

    pub(crate) fn pack_filter_title(&self, pack_id: &str) -> String {
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

    pub(crate) fn document_pack_title(&self, document_id: &WorldDocumentId) -> String {
        self.documents
            .iter()
            .find(|document| &document.id == document_id)
            .map(|document| {
                self.registry
                    .descriptor_for(&document.pack)
                    .map(|descriptor| descriptor.title.clone())
                    .unwrap_or_else(|| document.pack.id.clone())
            })
            .unwrap_or_else(|| document_id.to_string())
    }

    /// Open the name field on a World card. A World that already has a name
    /// opens on that name; an unnamed one opens empty and shows its World
    /// Pack's title as the placeholder.
    pub(crate) fn begin_rename(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        text_field::bind_keys(cx);
        let current = self
            .documents
            .iter()
            .find(|document| document.id == document_id)
            .and_then(|document| document.display_title.clone())
            .unwrap_or_default();
        let placeholder = rename_placeholder(&self.document_pack_title(&document_id));
        let input = cx.new(|cx| TextInput::new(placeholder, cx).with_text(current));
        self.pending_removal = None;
        self.renaming = Some(RenameDraft {
            document: document_id,
            input,
        });
        cx.notify();
    }

    pub(crate) fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming = None;
        cx.notify();
    }

    pub(crate) fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.renaming.take() else {
            return;
        };
        let typed = draft.input.read(cx).text().trim().to_owned();
        let title = (!typed.is_empty()).then_some(typed);
        let pack_title = self.document_pack_title(&draft.document);
        // An open World is renamed through its session, so the name is
        // written by its own writer and never races a turn being saved; a
        // closed one in its file (refused if another app has it open).
        let renamed = match open_world_document(cx, &draft.document) {
            Some(document) => {
                let renamed = match document.try_borrow_mut() {
                    Ok(mut document) => {
                        let library = Arc::clone(&document.library);
                        document.session.rename(title.as_deref(), &library)
                    }
                    Err(_) => Err(LibraryError::InUse(self.library.path(&draft.document))),
                };
                refresh_world_windows(&document, cx);
                renamed
            }
            None => self
                .library
                .set_display_title(&draft.document, title.as_deref()),
        };
        match renamed {
            Ok(summary) => {
                mark_library_changed();
                let message = rename_result_message(summary.display_title.as_deref(), &pack_title);
                self.status = Some(match self.sync_documents_after_mutation() {
                    Some(error) => error,
                    None => HomeStatus::success(message),
                });
            }
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not rename this World: {error}"
                )));
            }
        }
        cx.notify();
    }

    /// Removing a World is one click to ask and one to confirm; the card
    /// itself carries the question, so nothing is removed by a stray click.
    pub(crate) fn request_removal(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        self.renaming = None;
        self.pending_removal = Some(document_id);
        cx.notify();
    }

    pub(crate) fn cancel_removal(&mut self, cx: &mut Context<Self>) {
        self.pending_removal = None;
        cx.notify();
    }

    pub(crate) fn confirm_removal(&mut self, cx: &mut Context<Self>) {
        let Some(document_id) = self.pending_removal.take() else {
            return;
        };
        let title = self
            .document_title_for_id(&document_id)
            .unwrap_or_else(|| document_id.to_string());
        // An open World is not removed from under its window.
        if focus_open_world(cx, &document_id) {
            self.status = Some(HomeStatus::error(format!(
                "{title} is open: close its window first, then remove it."
            )));
            cx.notify();
            return;
        }
        let removed = self.library.remove(&document_id);
        match removed {
            Ok(path) => {
                mark_library_changed();
                diagnostics::info(format!("removed World {document_id} to {}", path.display()));
                self.status = Some(match self.sync_documents_after_mutation() {
                    Some(error) => error,
                    None => HomeStatus::success(removal_result_message(&title)),
                });
            }
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not remove this World: {error}"
                )));
            }
        }
        cx.notify();
    }

    pub(crate) fn document_title_for_id(&self, document_id: &WorldDocumentId) -> Option<String> {
        self.documents
            .iter()
            .find(|document| &document.id == document_id)
            .map(|document| {
                let pack_title = self
                    .registry
                    .descriptor_for(&document.pack)
                    .map(|descriptor| descriptor.title.clone())
                    .unwrap_or_else(|| document.pack.id.clone());
                self.distinct_title(document, &pack_title)
            })
    }

    /// A World's name on Home, told apart from any other of the same name:
    /// the second "Tiny Society" is "Tiny Society 2".
    pub(crate) fn distinct_title(
        &self,
        document: &WorldDocumentSummary,
        pack_title: &str,
    ) -> String {
        let title = world_summary_title(document, pack_title);
        let titled = |other: &WorldDocumentSummary| {
            let other_pack = self
                .registry
                .descriptor_for(&other.pack)
                .map(|descriptor| descriptor.title.clone())
                .unwrap_or_else(|| other.pack.id.clone());
            world_summary_title(other, &other_pack)
        };
        let mut same = self
            .documents
            .iter()
            .filter(|other| titled(other) == title)
            .map(|other| other.id.to_string())
            .collect::<Vec<_>>();
        same.sort();
        distinct(&title, &same, &document.id.to_string())
    }
}

/// How My Worlds is ordered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorldSort {
    /// The Library's own order: most recently played first.
    Recent,
    /// What the cards say, A to Z.
    Name,
}

impl WorldSort {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Recent => "Recent",
            Self::Name => "Name",
        }
    }
}

/// Beyond this many Worlds the list needs finding and ordering; below it the
/// controls would be clutter on a screen that shows every World at once.
pub(crate) const WORLD_SEARCH_THRESHOLD: usize = 10;

pub(crate) fn normalize_world_search(query: &str) -> String {
    query.trim().to_lowercase()
}

/// A World matches what was typed when the text appears in something the card
/// itself shows: its name, its World Pack's title, or its file identity.
pub(crate) fn world_matches_search(
    title: &str,
    pack_title: &str,
    document_id: &str,
    query: &str,
) -> bool {
    if query.is_empty() {
        return true;
    }
    [title, pack_title, document_id]
        .iter()
        .any(|field| field.to_lowercase().contains(query))
}

/// Order the cards. `Recent` leaves the Library's own most-recently-played
/// order alone; `Name` sorts by what the card shows, ignoring case, with the
/// file identity breaking ties so the order never wobbles between renders.
pub(crate) fn sort_world_cards(cards: &mut [(String, WorldDocumentSummary)], sort: WorldSort) {
    if sort == WorldSort::Name {
        cards.sort_by(|(left_title, left), (right_title, right)| {
            left_title
                .to_lowercase()
                .cmp(&right_title.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });
    }
}

/// What the My Worlds heading says once some Worlds are filtered out.
pub(crate) fn my_worlds_title(visible: usize, total: usize) -> String {
    if visible == total {
        format!("My Worlds · {total}")
    } else {
        format!("My Worlds · {visible}/{total}")
    }
}

/// The placeholder in the name field: an unnamed World shows the World Pack's
/// own title, which is exactly what the card falls back to.
pub(crate) fn rename_placeholder(pack_title: &str) -> String {
    format!("{pack_title} — name this World")
}

pub(crate) fn rename_result_message(display_title: Option<&str>, pack_title: &str) -> String {
    // A World open in a window is renamed through its own session (its
    // writer writes the name in turn with its turns), so nothing there needs
    // reloading.
    match display_title {
        Some(title) => format!("Renamed to {title}."),
        None => format!("Name cleared; this World is listed as {pack_title} again."),
    }
}

pub(crate) fn removal_result_message(title: &str) -> String {
    format!(
        "Removed {title}. Its file moved to the {} folder inside your Worlds folder.",
        world_library::REMOVED_DIRECTORY
    )
}

pub(crate) const UNREADABLE_DOCUMENT_NAME_LIMIT: usize = 3;

/// The files the 1.0 save contract refused (a preview's, or a newer World
/// Machine's), each said with its own reason, up to the limit; and how many
/// more there are.
pub(crate) fn refused_documents(
    unreadable: &[UnreadableWorldFile],
) -> (Vec<&UnreadableWorldFile>, usize) {
    let refused = unreadable
        .iter()
        .filter(|file| file.age != world_library::FormatAge::Current)
        .collect::<Vec<_>>();
    let hidden = refused.len().saturating_sub(UNREADABLE_DOCUMENT_NAME_LIMIT);
    (
        refused
            .into_iter()
            .take(UNREADABLE_DOCUMENT_NAME_LIMIT)
            .collect(),
        hidden,
    )
}

/// The damaged files that can be put back to their last good save, up to
/// the limit, each as a World to restore.
pub(crate) fn restorable_documents(
    library: &WorldLibrary,
    unreadable: &[UnreadableWorldFile],
) -> Vec<RestoreTarget> {
    unreadable
        .iter()
        .filter(|file| file.restorable)
        .take(UNREADABLE_DOCUMENT_NAME_LIMIT)
        .map(|file| {
            let stem = file
                .file_name
                .strip_suffix(WORLD_DOCUMENT_SUFFIX)
                .unwrap_or(&file.file_name);
            RestoreTarget {
                path: library.root().join(&file.file_name),
                title: stem.to_owned(),
                document: WorldDocumentId::new(stem).ok(),
            }
        })
        .collect()
}

/// Name the files that could not be read, without letting a folder full of
/// them push every World off the screen. Files the save contract refused are
/// not among them: they are said with their reasons (`refused_documents`).
pub(crate) fn unreadable_documents_note(unreadable: &[UnreadableWorldFile]) -> Option<String> {
    let unreadable = unreadable
        .iter()
        .filter(|file| file.age == world_library::FormatAge::Current)
        .collect::<Vec<_>>();
    if unreadable.is_empty() {
        return None;
    }
    let named = unreadable
        .iter()
        .take(UNREADABLE_DOCUMENT_NAME_LIMIT)
        .map(|file| file.file_name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let hidden = unreadable
        .len()
        .saturating_sub(UNREADABLE_DOCUMENT_NAME_LIMIT);
    let names = if hidden > 0 {
        format!("{named}, and {hidden} more")
    } else {
        named
    };
    Some(if unreadable.len() == 1 {
        format!("One file in your Worlds folder could not be read and is not listed: {names}.")
    } else {
        format!(
            "{} files in your Worlds folder could not be read and are not listed: {names}.",
            unreadable.len()
        )
    })
}

pub(crate) fn report_unreadable_documents(unreadable: &[UnreadableWorldFile]) {
    for file in unreadable {
        diagnostics::error(format!(
            "could not read {} in the Worlds folder: {}",
            file.file_name, file.reason
        ));
    }
}

/// A heading over one group of cards on Home.
pub(crate) fn home_section_title(text: impl Into<SharedString>) -> gpui::Div {
    div().pt_4().child(ui::heading(text))
}

/// Whether anything has happened in a World yet.
pub(crate) fn has_begun(document: &WorldDocumentSummary) -> bool {
    document.event_count > 0 || document.world_time > 0
}

/// How tall a World's cover stands on Home.
pub(crate) const WORLD_COVER_HEIGHT: f32 = 196.0;

/// The line under a World's name on Home.
pub(crate) fn world_card_meta(
    title: &str,
    pack_title: &str,
    world_time: u64,
    calendar: Option<&world_projection::Calendar>,
) -> String {
    // The World's own count ("Sol 5"), the way its window says it.
    let age = if world_time == 0 {
        ui::t("just begun").to_string()
    } else {
        world_gpui::i18n::moment_label(
            &world_projection::ProjectionSnapshot {
                calendar: calendar.cloned(),
                ..world_projection::ProjectionSnapshot::default()
            },
            world_time,
        )
    };
    if title == pack_title {
        age
    } else {
        format!("{} · {age}", ui::t(pack_title.to_string()))
    }
}

pub(crate) fn world_summary_title(document: &WorldDocumentSummary, pack_title: &str) -> String {
    document
        .display_title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(pack_title)
        .to_owned()
}

/// `title` for the World `id`, of the Worlds `same` (by id, oldest
/// first) that share it: the first keeps it, the next are numbered.
pub(crate) fn distinct(title: &str, same: &[String], id: &str) -> String {
    match same.iter().position(|other| other == id) {
        Some(at) if at > 0 => format!("{title} {}", at + 1),
        _ => title.to_owned(),
    }
}

pub(crate) fn world_matches_pack_filter(
    document: &WorldDocumentSummary,
    pack_id: Option<&str>,
) -> bool {
    pack_id.is_none_or(|pack_id| document.pack.id == pack_id)
}

pub(crate) fn world_pack_filter_is_available(
    documents: &[WorldDocumentSummary],
    pack_id: Option<&str>,
) -> bool {
    pack_id.is_none_or(|pack_id| documents.iter().any(|document| document.pack.id == pack_id))
}

pub(crate) fn world_pack_filter_counts(documents: &[WorldDocumentSummary]) -> Vec<(String, usize)> {
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

/// What a World is called: the name its owner gave it, or the durable identity
/// of its file when it has no name.
pub(crate) fn document_display_name(display_title: Option<&str>, durable_label: &str) -> String {
    display_title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(durable_label)
        .to_owned()
}

pub(crate) fn session_display_name(session: &DurableWorldSession) -> String {
    let durable_label = session.display_name();
    document_display_name(session.metadata().display_title.as_deref(), &durable_label)
}

pub(crate) fn document_window_title(document_label: &str) -> String {
    format!("{document_label} — World Machine")
}

pub(crate) fn start_after_install_matches(
    pending: Option<&WorldPackRef>,
    pack: &WorldPackRef,
) -> bool {
    pending == Some(pack)
}

pub(crate) fn format_program_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    if bytes >= MIB {
        format!("{:.1} MiB · {bytes} bytes", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB · {bytes} bytes", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} bytes")
    }
}
