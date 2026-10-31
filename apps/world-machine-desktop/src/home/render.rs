//! Home drawn: the controls over My Worlds, the note about files that
//! cannot be read, each World's card, a new World's card, and the page.

#[allow(unused_imports)]
use super::*;

impl WorldMachineHome {
    /// What Home says about the files in the Worlds folder that cannot be
    /// listed: the damaged ones by name, each that has a good backup with
    /// the offer to restore it, and each the save contract refused with its
    /// own reason. None when every file is listed.
    pub(crate) fn unreadable_documents_notice(&self, cx: &mut Context<Self>) -> Option<gpui::Div> {
        let damaged = unreadable_documents_note(&self.unreadable_documents);
        let (refused, more_refused) = refused_documents(&self.unreadable_documents);
        if damaged.is_none() && refused.is_empty() {
            return None;
        }
        let mut notice = div()
            .w_full()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::DANGER))
            .bg(ui::color(tokens::DANGER_SOFT))
            .text_xs()
            .text_color(ui::color(tokens::DANGER))
            .flex()
            .flex_col()
            .gap_2();
        if let Some(note) = damaged {
            notice = notice.child(note);
            for target in restorable_documents(&self.library, &self.unreadable_documents) {
                let id = SharedString::from(format!("restore-unreadable-{}", target.title));
                notice = notice.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(target.title.clone())
                        .child(
                            ui::button(
                                id,
                                world_machine_desktop::restore::RESTORE_LABEL,
                                ui::ButtonKind::Primary,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.restore_last_good(target.clone(), cx);
                                },
                            )),
                        ),
                );
            }
        }
        for file in refused {
            notice = notice.child(format!(
                "{}: {}",
                file.file_name,
                ui::t(file.reason.clone())
            ));
        }
        if more_refused > 0 {
            notice = notice.child(format!("…and {more_refused} more that cannot be opened."));
        }
        Some(notice)
    }

    /// Find a World, and choose whether the list reads most-recent-first or
    /// A to Z. Shown only once there are enough Worlds for the list to be hard
    /// to scan.
    pub(crate) fn world_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut order = div().flex_shrink_0().flex().items_center().gap_2().child(
            div()
                .text_xs()
                .text_color(ui::color(tokens::TEXT_SECONDARY))
                .child(ui::t("Order")),
        );
        for sort in [WorldSort::Recent, WorldSort::Name] {
            let selected = self.world_sort == sort;
            let (border, background, text) = if selected {
                (0x6f86b0, 0xe9eef7, 0x314b72)
            } else {
                (0xd9d9d3, 0xffffff, 0x666666)
            };
            order = order.child(
                div()
                    .id(SharedString::from(format!(
                        "world-sort-{}",
                        sort.label().to_lowercase()
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(border))
                    .bg(crate::theme_rgb(background))
                    .text_color(crate::theme_rgb(text))
                    .text_xs()
                    .child(sort.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.world_sort = sort;
                        cx.notify();
                    })),
            );
        }

        div()
            .w_full()
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(self.world_search.clone()),
            )
            .child(order)
    }

    pub(crate) fn document_card(
        &self,
        document: WorldDocumentSummary,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open_id = document.id.clone();
        let export_id = document.id.clone();
        let rename_id = document.id.clone();
        let remove_id = document.id.clone();
        let pack_title = self
            .registry
            .descriptor_for(&document.pack)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| document.pack.id.clone());
        let title = self.distinct_title(&document, &pack_title);
        let document_label = document.id.to_string();

        let mut details = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap_1()
            .child(ui::heading(title.clone()).truncate());
        // What a person needs to tell their Worlds apart: which kind of World
        // it is and how far it has lived. Event counts and file ids are the
        // archive's business; the World's own window still shows the file.
        // A World asleep is told by the day the player said goodnight
        // to (v0.31 play: "Day 4" after Goodnight on day 3).
        let lived = world_gpui::i18n::day_lived(
            document.world_time,
            document.display_calendar.as_ref(),
            observer::asleep(&document),
        );
        details = details.child(ui::caption(world_card_meta(
            &title,
            &pack_title,
            lived,
            document.display_calendar.as_ref(),
        )));

        let renaming_this_world = self
            .renaming
            .as_ref()
            .is_some_and(|draft| draft.document == document.id);
        let removing_this_world = self.pending_removal.as_ref() == Some(&document.id);
        if renaming_this_world {
            let input = self
                .renaming
                .as_ref()
                .expect("renaming_this_world implies a draft")
                .input
                .clone();
            details = details.child(
                div().w_full().flex().flex_col().gap_2().child(input).child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            ui::button(
                                SharedString::from(format!("save-name-{document_label}")),
                                "Save name",
                                ui::ButtonKind::Secondary,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.commit_rename(cx))),
                        )
                        .child(
                            ui::button(
                                SharedString::from(format!("cancel-name-{document_label}")),
                                "Cancel",
                                ui::ButtonKind::Secondary,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.cancel_rename(cx))),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .text_xs()
                                .text_color(ui::color(tokens::TEXT_SECONDARY))
                                .child(ui::t(
                                    "An empty name lists this World under its own title again.",
                                )),
                        ),
                ),
            );
        } else if removing_this_world {
            details = details.child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(ui::color(tokens::DANGER))
                            .child(format!(
                                "Remove {title}? Its file moves to the {} folder inside your Worlds folder, so you can put it back.",
                                world_library::REMOVED_DIRECTORY
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                div()
                                    .id(SharedString::from(format!(
                                        "confirm-remove-{document_label}"
                                    )))
                                    .cursor_pointer()
                                    .p_2()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(ui::color(tokens::DANGER))
                                    .bg(ui::color(tokens::DANGER_SOFT))
                                    .text_color(ui::color(tokens::DANGER))
                                    .text_sm()
                                    .child(ui::t("Remove"))
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.confirm_removal(cx)),
                                    ),
                            )
                            .child(
                                div()
                                    .id(SharedString::from(format!(
                                        "keep-{document_label}"
                                    )))
                                    .cursor_pointer()
                                    .p_2()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(ui::color(tokens::BORDER_STRONG))
                                    .text_sm()
                                    .child(ui::t("Keep"))
                                    .on_click(cx.listener(|this, _, _, cx| this.cancel_removal(cx))),
                            ),
                    ),
            );
        }
        // What else can be done with a World waits behind its ⋯ and a
        // right-click, so the shelf is covers, not rows of links.
        let menu_open = self.card_menu.as_ref() == Some(&document.id)
            && !renaming_this_world
            && !removing_this_world;
        let menu =
            menu_open.then(|| {
                let item = |id: String, label: &'static str| {
                    div()
                        .id(SharedString::from(id))
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .text_sm()
                        .cursor_pointer()
                        .hover(|style| style.bg(ui::color(tokens::ROW_HOVER)))
                        .child(label)
                };
                div()
                    .id(SharedString::from(format!("menu-{document_label}")))
                    .absolute()
                    .top(px(6.0))
                    .right(px(44.0))
                    .w(px(200.0))
                    .p_1()
                    .rounded_lg()
                    .bg(ui::color(tokens::SURFACE))
                    .border_1()
                    .border_color(ui::color(tokens::BORDER))
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(item(format!("rename-{document_label}"), "Rename").on_click(
                        cx.listener(move |this, _, _, cx| {
                            this.card_menu = None;
                            this.begin_rename(rename_id.clone(), cx)
                        }),
                    ))
                    .child(
                        item(format!("export-{export_id}"), "Export…").on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.card_menu = None;
                                this.export_document(export_id.clone(), cx)
                            },
                        )),
                    )
                    .child(
                        item(format!("remove-{document_label}"), "Remove")
                            .text_color(ui::color(tokens::DANGER))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.card_menu = None;
                                this.request_removal(remove_id.clone(), cx)
                            })),
                    )
            });

        // A new day waiting for the player (v0.30 S4): opening the World
        // begins a new sitting, with its morning. One day, however long
        // they were away. Asleep, how long until the next, as its title
        // bar says it (1.0 P2).
        let waiting = document
            .display_moves_alone
            .then(|| observer::new_day_waiting(&document))
            .flatten()
            .filter(|waiting| *waiting);
        let unit = document
            .display_calendar
            .as_ref()
            .map(|calendar| calendar.unit.to_lowercase())
            .unwrap_or_else(|| "day".into());
        let day_line = world_card_day_line(
            waiting.is_some(),
            document.display_moves_alone && observer::asleep(&document),
            observer::next_sitting_in().unwrap_or(0),
            &unit,
            world_gpui::ui::play_freely(),
        );
        let mut cover = div()
            .id(SharedString::from(format!("cover-{document_label}")))
            .role(gpui::Role::Button)
            .aria_label(ui::t(format!("Open {title}")))
            .relative()
            .w_full()
            .h(px(WORLD_COVER_HEIGHT))
            .cursor_pointer()
            .child(if document.display_cast.is_empty() {
                ui::living_cover(
                    document.display_scenery.as_ref(),
                    &title,
                    document.id.as_str(),
                    &document.display_marks,
                )
                .size_full()
                .into_any_element()
            } else {
                // Its own people and buildings, as it last stood.
                world_gpui::diorama::cover(
                    document.display_scenery,
                    &document.display_marks,
                    document.display_cast.clone(),
                    document.display_drawings.clone(),
                )
                .size_full()
                .into_any_element()
            })
            .on_click(cx.listener({
                let open_id = open_id.clone();
                move |this, _, _, cx| this.open_document(open_id.clone(), cx)
            }));
        // The way in sits on the picture, the way a game shelf has it. The
        // click stops at the button, or the picture under it would open
        // the World a second time.
        cover = cover.child(
            div().absolute().bottom_3().right_3().child(
                ui::button(
                    SharedString::from(format!("open-{open_id}")),
                    "Open",
                    ui::ButtonKind::Primary,
                )
                .on_click(cx.listener({
                    let open_id = open_id.clone();
                    move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.open_document(open_id.clone(), cx)
                    }
                })),
            ),
        );
        if let Some(day_line) = day_line {
            cover =
                cover.child(
                    div()
                        .absolute()
                        .top_2()
                        .left_2()
                        .px_2()
                        .py_1()
                        .rounded_full()
                        .bg(ui::color(tokens::SURFACE))
                        .shadow_sm()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().size(px(6.0)).rounded_full().bg(ui::color(
                            if waiting.is_some() {
                                tokens::SUCCESS
                            } else {
                                tokens::TEXT_TERTIARY
                            },
                        )))
                        .child(
                            div()
                                .text_xs()
                                .text_color(ui::color(tokens::TEXT))
                                .child(day_line),
                        ),
                );
        }

        let menu_id = document.id.clone();
        cover = cover.child(
            div()
                .id(SharedString::from(format!("more-{document_label}")))
                .role(gpui::Role::Button)
                .aria_label(ui::t(format!("More for {title}")))
                .tooltip(ui::tip(format!("More for {title}")))
                .tooltip_show_delay(ui::TIP_DELAY)
                .aria_expanded(menu_open)
                .absolute()
                .top_2()
                .right_2()
                .size(px(30.0))
                .rounded_full()
                .bg(ui::color(tokens::SURFACE))
                .shadow_sm()
                .flex()
                .items_center()
                .justify_center()
                .text_color(ui::color(tokens::TEXT_SECONDARY))
                .cursor_pointer()
                .when(!menu_open, |more| {
                    more.opacity(0.0).group_hover(
                        SharedString::from(format!("card-{document_label}")),
                        |style| style.opacity(1.0),
                    )
                })
                .child("⋯")
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.card_menu = if this.card_menu.as_ref() == Some(&menu_id) {
                        None
                    } else {
                        Some(menu_id.clone())
                    };
                    cx.notify();
                })),
        );
        if let Some(menu) = menu {
            cover = cover.child(menu);
        }
        let right_click_id = document.id.clone();
        div()
            .id(SharedString::from(format!("document-{document_label}")))
            .role(gpui::Role::Group)
            .aria_label(title.clone())
            .group(SharedString::from(format!("card-{document_label}")))
            .on_mouse_down(
                gpui::MouseButton::Right,
                cx.listener(move |this, _, _, cx| {
                    this.card_menu = Some(right_click_id.clone());
                    cx.notify();
                }),
            )
            .w_full()
            .min_w(px(0.0))
            .rounded_xl()
            .overflow_hidden()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::SURFACE))
            .hover(|style| style.border_color(ui::color(tokens::BORDER_STRONG)))
            .flex()
            .flex_col()
            .child(cover)
            .child(div().p_4().child(details))
    }

    pub(crate) fn new_world_card(
        &self,
        descriptor: world_host::WorldDescriptor,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pack_id = descriptor.pack.id.clone();
        div()
            .id(SharedString::from(format!("new-world-{pack_id}")))
            .min_w(px(260.0))
            .flex_1()
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::SURFACE))
            .cursor_pointer()
            .hover(|style| {
                style
                    .border_color(ui::color(tokens::ACCENT))
                    .bg(ui::color(tokens::SURFACE_HOVER))
            })
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .mb_2()
                    .h(px(84.0))
                    .w_full()
                    .rounded_md()
                    .overflow_hidden()
                    .child(ui::cover(&descriptor.title, &pack_id).size_full()),
            )
            .child(ui::row_title(descriptor.title))
            .child(ui::detail(descriptor.description).line_clamp(3))
            .child(
                div()
                    .pt_2()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(ui::color(tokens::ACCENT_TEXT))
                    .child(ui::t("Start a World →")),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.create_world(pack_id.clone(), cx)))
    }
}

impl WorldMachineHome {
    /// My Worlds and the rest: the page one step away from the title
    /// screen, with every World, the Packs, and starting a World of any
    /// kind.
    pub(crate) fn library_page(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        // A World where nothing has happened yet is not one of My Worlds;
        // starting that kind of World again picks it back up.
        let documents = self
            .documents
            .iter()
            .filter(|document| has_begun(document))
            .cloned()
            .collect::<Vec<_>>();
        let has_documents = !documents.is_empty();
        let pack_filters = world_pack_filter_counts(&documents);
        let selected_world_pack = self.selected_world_pack.clone();
        let search_query = normalize_world_search(self.world_search.read(cx).text());
        let show_world_controls = documents.len() >= WORLD_SEARCH_THRESHOLD;
        let mut visible_cards = documents
            .iter()
            .filter(|document| world_matches_pack_filter(document, selected_world_pack.as_deref()))
            .map(|document| {
                let pack_title = self
                    .registry
                    .descriptor_for(&document.pack)
                    .map(|descriptor| descriptor.title.clone())
                    .unwrap_or_else(|| document.pack.id.clone());
                let title = world_summary_title(document, &pack_title);
                (title, pack_title, document.clone())
            })
            .filter(|(title, pack_title, document)| {
                world_matches_search(title, pack_title, document.id.as_str(), &search_query)
            })
            .map(|(title, _pack_title, document)| (title, document))
            .collect::<Vec<_>>();
        sort_world_cards(&mut visible_cards, self.world_sort);
        let visible_documents = visible_cards
            .into_iter()
            .map(|(_title, document)| document)
            .collect::<Vec<_>>();
        let visible_document_count = visible_documents.len();
        let descriptors = self
            .registry
            .descriptors()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let first_run = !has_documents;
        let featured_included = self
            .included_packs
            .iter()
            .find(|pack| pack.featured && !self.included_pack_is_installed(&pack.pack))
            .filter(|pack| !demo::ENABLED || demo::offers_pack(&pack.pack.id))
            .cloned();
        let featured_review_pending = self
            .pending_pack_install
            .as_ref()
            .zip(featured_included.as_ref())
            .is_some_and(|(preview, featured)| preview.pack() == &featured.pack);
        let show_featured = first_run
            && self.pending_pack_install.is_none()
            && self.ready_pack_to_create.is_none()
            && featured_included.is_some();

        let mut saved = div().w_full().flex().flex_col().gap_3();
        if !has_documents {
            saved = saved.child(
                div()
                    .p_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(ui::color(tokens::BORDER))
                    .text_sm()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(ui::t(
                        "No Worlds yet. Start one below; it grows a day at a time.",
                    )),
            );
        } else if visible_documents.is_empty() {
            let empty = if search_query.is_empty() {
                "No Worlds match this Pack filter."
            } else {
                "No Worlds match what you typed."
            };
            saved = saved.child(
                div()
                    .p_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(ui::color(tokens::BORDER))
                    .text_sm()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(empty),
            );
        } else {
            // Two columns that each stack their own cards, so a tall card
            // never leaves a hole beside a short one.
            let mut columns = [
                div().flex_1().min_w(px(0.0)).flex().flex_col().gap_4(),
                div().flex_1().min_w(px(0.0)).flex().flex_col().gap_4(),
            ];
            for (index, document) in visible_documents.into_iter().enumerate() {
                let column = std::mem::replace(&mut columns[index % 2], div());
                columns[index % 2] = column.child(self.document_card(document, cx));
            }
            let [left, right] = columns;
            saved = saved.child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .gap_4()
                    .child(left)
                    .child(right),
            );
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
                    .border_color(crate::theme_rgb(all_border))
                    .bg(crate::theme_rgb(all_background))
                    .text_color(crate::theme_rgb(all_text))
                    .text_xs()
                    .child(format!("All · {}", documents.len()))
                    .on_click(cx.listener(|this, _, _, cx| this.set_world_pack_filter(None, cx))),
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
                        .border_color(crate::theme_rgb(border))
                        .bg(crate::theme_rgb(background))
                        .text_color(crate::theme_rgb(text))
                        .text_xs()
                        .child(format!("{filter_title} · {count}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_world_pack_filter(Some(filter_pack.clone()), cx)
                        })),
                );
            }
        }

        let visible_included_packs = self
            .included_packs
            .iter()
            .filter(|included| !self.included_pack_is_installed(&included.pack))
            .filter(|included| !demo::ENABLED || demo::offers_pack(&included.pack.id))
            .filter(|included| {
                featured_included.as_ref().is_none_or(|featured| {
                    featured.pack != included.pack || (!show_featured && !featured_review_pending)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut included = div().w_full().flex().flex_col().gap_3();
        for pack in visible_included_packs.iter().cloned() {
            included = included.child(self.included_pack_card(pack, cx));
        }

        // The demo has no Packs to show beyond its own.
        let installed_packs = self
            .pack_catalog
            .as_ref()
            .filter(|_| !demo::ENABLED)
            .map(|catalog| catalog.entries().to_vec())
            .unwrap_or_default();
        let mut installed = div().w_full().flex().flex_col().gap_3();
        for pack in installed_packs.iter().cloned() {
            installed = installed.child(self.installed_pack_card(pack, cx));
        }

        let mut available = div().w_full().flex().flex_wrap().gap_3();
        // The World a newcomer should try first leads the list.
        let mut descriptors = descriptors;
        descriptors.sort_by_key(|descriptor| {
            !self
                .included_packs
                .iter()
                .any(|included| included.featured && included.pack.id == descriptor.pack.id)
        });
        // The demo offers its one Pack, and no other.
        for descriptor in descriptors
            .into_iter()
            .filter(|descriptor| !demo::ENABLED || demo::offers_pack(&descriptor.pack.id))
        {
            available = available.child(self.new_world_card(descriptor, cx));
        }

        let header = div()
            .id("world-machine-home-chrome")
            .w_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div().pb_2().flex().child(
                    ui::button("home-back", "Back", ui::ButtonKind::Secondary)
                        .on_click(cx.listener(|this, _, window, cx| this.show_title(window, cx))),
                ),
            )
            .child(ui::page_title(demo::HOME_TITLE))
            .child(ui::body("Small worlds that grow a day at a time."));

        let mut body = div()
            .w_full()
            .max_w(px(760.0))
            .mx_auto()
            .flex()
            .flex_col()
            .gap_4()
            .px_6()
            .pt_8()
            .pb_10()
            .child(header);

        if let Some(update) = self.available_update.clone() {
            body = body.child(self.update_banner(update, cx));
        }

        if let Some(preview) = self.pending_pack_install.clone() {
            body = body.child(self.pack_install_review_card(preview, cx));
        }

        if let Some(descriptor) = self.ready_pack_descriptor() {
            body = body.child(self.ready_pack_card(descriptor, cx));
        }

        if show_featured {
            let featured = featured_included.expect("show_featured requires a featured Pack");
            body = body
                .child(home_section_title("Start here"))
                .child(self.featured_included_pack_card(featured, cx));
        }

        // Shown whether or not any World can be listed: a folder of only
        // refused or damaged files must still say what happened to them.
        let mut unreadable_notice = self.unreadable_documents_notice(cx);
        if has_documents || self.included_packs.is_empty() {
            body = body.child(home_section_title(my_worlds_title(
                visible_document_count,
                documents.len(),
            )));
            if show_world_controls {
                body = body.child(self.world_controls(cx));
            }
            if let Some(notice) = unreadable_notice.take() {
                body = body.child(notice);
            }
            if show_world_controls && pack_filters.len() > 1 {
                body = body.child(world_filters);
            }
            body = body.child(saved);
        }
        if let Some(notice) = unreadable_notice {
            body = body
                .child(home_section_title(my_worlds_title(0, 0)))
                .child(notice);
        }

        if !visible_included_packs.is_empty() {
            body = body
                .child(home_section_title(if first_run {
                    "More worlds"
                } else {
                    "Included Worlds"
                }))
                .child(included);
        }

        body = body
            .child(home_section_title("Start a new World"))
            .child(available);

        if !installed_packs.is_empty() {
            let packs_title = if self.show_packs {
                format!("Packs · {} · hide", installed_packs.len())
            } else {
                format!("Packs · {} · show", installed_packs.len())
            };
            body = body.child(
                div()
                    .id("toggle-installed-packs")
                    .cursor_pointer()
                    .text_sm()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(packs_title)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_packs = !this.show_packs;
                        cx.notify();
                    })),
            );
            if self.show_packs {
                body = body.child(installed);
            }
        }

        div()
            .relative()
            .size_full()
            .bg(ui::color(tokens::WINDOW))
            .text_color(ui::color(tokens::TEXT))
            .flex()
            .flex_col()
            .child(
                div()
                    .id("world-machine-home-scroll")
                    .w_full()
                    .flex_1()
                    .overflow_y_scroll()
                    .child(body),
            )
    }

    /// What Home said it did, or could not do, floating over the bottom of
    /// either page instead of being inserted above it, so a Pack finishing
    /// its start-up never moves the card under the pointer.
    pub(crate) fn status_overlay(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Stateful<gpui::Div>> {
        let status = self.status.as_ref()?;
        let tone = match status.tone {
            HomeStatusTone::Info => tokens::ACCENT_TEXT,
            HomeStatusTone::Success => tokens::SUCCESS,
            HomeStatusTone::Error => tokens::DANGER,
        };
        Some(
            div()
                .id("world-machine-home-status")
                .absolute()
                .bottom_4()
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .px_6()
                .child(
                    div()
                        .max_w(px(640.0))
                        .w_full()
                        .pl_4()
                        .pr_2()
                        .py_2()
                        .rounded_lg()
                        .border_1()
                        .border_color(ui::color(tokens::BORDER_STRONG))
                        .bg(ui::color(tokens::SURFACE))
                        .shadow_md()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .text_sm()
                                .text_color(ui::color(tone))
                                .child(ui::t(status.message.clone())),
                        )
                        .when_some(status.restore.clone(), |row, target| {
                            row.child(
                                ui::button(
                                    "restore-world-machine-home-status",
                                    world_machine_desktop::restore::RESTORE_LABEL,
                                    ui::ButtonKind::Primary,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.status = None;
                                        this.restore_last_good(target.clone(), cx);
                                    },
                                )),
                            )
                        })
                        .child(
                            ui::button(
                                "dismiss-world-machine-home-status",
                                "Dismiss",
                                ui::ButtonKind::Secondary,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.status = None;
                                cx.notify();
                            })),
                        ),
                ),
        )
    }
}
