//! Home: the window that lists the player's Worlds and the World Packs,
//! and starts, opens, imports and restores them. Moved out of `main.rs`
//! as it stood; what it does is split by subject below.

#[allow(unused_imports)]
use super::*;

mod library;
mod packs;
mod render;
mod status;
pub(crate) use library::*;
pub(crate) use status::*;

pub(crate) struct WorldMachineHome {
    pub(crate) registry: Arc<world_host::WorldRegistry>,
    pub(crate) library: Arc<WorldLibrary>,
    pub(crate) pack_catalog: Option<PackCatalog>,
    pub(crate) pack_catalog_path: PathBuf,
    pub(crate) documents: Vec<WorldDocumentSummary>,
    pub(crate) selected_world_pack: Option<String>,
    pub(crate) included_packs: Vec<included_packs::IncludedPack>,
    pub(crate) pending_pack_install: Option<PackInstallPreview>,
    pub(crate) pending_start_after_install: Option<WorldPackRef>,
    pub(crate) ready_pack_to_create: Option<WorldPackRef>,
    pub(crate) probing_packs: Vec<WorldPackRef>,
    pub(crate) status: Option<HomeStatus>,
    /// Installed-Pack management is hidden behind one line on Home until asked
    /// for; nothing in the ordinary path needs it.
    pub(crate) show_packs: bool,
    /// A newer stable release found at launch, shown as a banner until
    /// dismissed or downloaded.
    pub(crate) available_update: Option<updates::AvailableUpdate>,
    /// Files in the Worlds folder that name themselves Worlds but cannot be
    /// read. They are named on Home instead of hiding every other World.
    pub(crate) unreadable_documents: Vec<UnreadableWorldFile>,
    /// The World whose name is being typed, if any.
    pub(crate) renaming: Option<RenameDraft>,
    /// The World whose removal is waiting for a second click.
    pub(crate) pending_removal: Option<WorldDocumentId>,
    /// The World whose ⋯ menu is open on Home.
    pub(crate) card_menu: Option<WorldDocumentId>,
    /// How My Worlds is ordered, for this run of the app.
    pub(crate) world_sort: WorldSort,
    /// What was typed into Find a World.
    pub(crate) world_search: Entity<TextInput>,
    /// On a first launch Home steps aside once the first World opens, so a
    /// new player meets one window, not two.
    pub(crate) step_aside_for_first_world: bool,
}

/// A World name being typed on Home. Only one World is renamed at a time, so
/// the field lives here rather than one per card.
pub(crate) struct RenameDraft {
    pub(crate) document: WorldDocumentId,
    pub(crate) input: Entity<TextInput>,
}

impl WorldMachineHome {
    pub(crate) fn start_system_open_listener(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let mut observed_library_revision = library_change_revision();
            let mut ticks_since_geometry_flush = 0u32;
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;

                let Some(this) = this.upgrade() else {
                    return;
                };

                // Windows report where they are as they render; write any
                // change a few times a second rather than on every frame of a
                // drag.
                ticks_since_geometry_flush += 1;
                if ticks_since_geometry_flush >= WINDOW_GEOMETRY_FLUSH_TICKS {
                    ticks_since_geometry_flush = 0;
                    flush_window_geometry();
                }
                let paths = system_open::drain_paths();
                let revision = library_change_revision();
                let library_changed = revision != observed_library_revision;
                if library_changed {
                    observed_library_revision = revision;
                }
                if paths.is_empty() && !library_changed {
                    continue;
                }

                this.update(cx, |this, cx| {
                    for path in paths {
                        match path {
                            Ok(path) => this.open_external_path(path, cx),
                            Err(error) => {
                                this.status = Some(HomeStatus::error(format!(
                                    "Could not open World file: {error}"
                                )));
                            }
                        }
                    }
                    if library_changed {
                        if let Err(error) = this.refresh_documents() {
                            this.status = Some(error);
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// One background request to the Releases API; a newer stable version
    /// becomes a banner on Home. Silent on failure, off with
    /// WORLD_MACHINE_NO_UPDATE_CHECK=1.
    pub(crate) fn start_update_check(&mut self, cx: &mut Context<Self>) {
        if !updates::enabled() {
            return;
        }
        let task = cx
            .background_executor()
            .spawn(async move { updates::check() });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if let Some(update) = result {
                    diagnostics::info(format!("update available: {}", update.version));
                    this.available_update = Some(update);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(crate) fn update_banner(
        &self,
        update: updates::AvailableUpdate,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let url = update.url.clone();
        div()
            .id("update-available")
            .w_full()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::BORDER_STRONG))
            .bg(ui::color(tokens::ACCENT_SOFT))
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .text_sm()
                    .text_color(ui::color(tokens::ACCENT_TEXT))
                    .child(format!(
                        "World Machine {} is available. You have {}.",
                        update.version,
                        build_info::APP_VERSION
                    )),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .gap_2()
                    .child(
                        ui::button("download-update", "Download", ui::ButtonKind::Secondary)
                            .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))),
                    )
                    .child(
                        ui::button("dismiss-update", "Later", ui::ButtonKind::Secondary).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.available_update = None;
                                cx.notify();
                            }),
                        ),
                    ),
            )
    }
}
