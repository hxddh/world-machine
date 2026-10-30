use super::*;

#[test]
fn finder_knows_every_file_the_app_opens_and_what_it_is() {
    let plist = include_str!("../macos/Info.plist.in");
    for suffix in [
        WORLD_DOCUMENT_SUFFIX,
        world_library::WORLD_CODE_SUFFIX,
        PACK_BUNDLE_SUFFIX,
    ] {
        let extension = suffix.trim_start_matches('.');
        assert!(
            plist.contains(&format!("<string>{extension}</string>")),
            "{extension} is declared"
        );
    }
    // A World file is gzip, not JSON; a World code is text.
    assert!(plist.contains("<string>org.gnu.gnu-zip-archive</string>"));
    assert!(!plist.contains("public.json"));
    assert!(plist.contains("<string>public.utf8-plain-text</string>"));
    // What Finder hands the app goes down the Open menu's own paths.
    assert!(sharing::is_world_code_file(Path::new("/tmp/a.worldcode")));
    assert!(is_world_file(Path::new("/tmp/a.world")));
    let (url, path) = if cfg!(windows) {
        (
            "file:///C:/tmp/Leo%27s%20town.worldcode",
            r"C:\tmp\Leo's town.worldcode",
        )
    } else {
        (
            "file:///tmp/Leo%27s%20town.worldcode",
            "/tmp/Leo's town.worldcode",
        )
    };
    assert_eq!(
        system_open::path_from_open_url(url).unwrap(),
        PathBuf::from(path)
    );
}

#[test]
fn document_status_tone_is_explicit_not_inferred_from_message_text() {
    let success = DocumentStatus::success("failed-looking note after a durable save");
    let error = DocumentStatus::error("failed");

    assert_eq!(success.tone, DocumentStatusTone::Success);
    assert_eq!(error.tone, DocumentStatusTone::Error);
    assert_eq!(success.message, "failed-looking note after a durable save");
}

#[test]
fn library_change_revision_advances_after_mark() {
    let before = library_change_revision();
    mark_library_changed();
    assert!(library_change_revision() > before);
}

#[test]
fn home_status_tone_is_explicit_not_inferred_from_message_text() {
    let info = HomeStatus::info("Could not-looking informational text");
    let success = HomeStatus::success("done");
    let error = HomeStatus::error("failed");

    assert_eq!(info.tone, HomeStatusTone::Info);
    assert_eq!(success.tone, HomeStatusTone::Success);
    assert_eq!(error.tone, HomeStatusTone::Error);
    assert_eq!(info.message, "Could not-looking informational text");
}

#[test]
fn world_pack_filters_preserve_recent_pack_order_and_counts() {
    let summary = |id: &str, pack_id: &str| WorldDocumentSummary {
        id: WorldDocumentId::new(id).unwrap(),
        pack: WorldPackRef::new(pack_id, "1.0.0"),
        display_title: None,
        display_summary: None,
        display_scenery: None,
        display_calendar: None,
        display_marks: Vec::new(),
        display_moves_alone: false,
        display_cast: Vec::new(),
        display_drawings: Vec::new(),
        host_sittings: None,
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
fn home_counts_a_world_in_its_own_time_and_says_what_waits() {
    let sols = world_projection::Calendar {
        unit: "Sol".into(),
        length: 10,
        season: None,
        coming: None,
        festival_today: false,
        year: None,
    };
    assert_eq!(
        world_card_meta("Ares Pocket Colony", "Pocket Universe", 50, Some(&sols)),
        "Pocket Universe · Sol 5"
    );
    assert_eq!(
        world_card_meta("Pocket Universe", "Pocket Universe", 0, Some(&sols)),
        "just begun"
    );
    assert_eq!(new_day_waiting_line("sol"), "A new sol is waiting");
    assert_eq!(new_day_waiting_line("day"), "A new day is waiting");
}

#[test]
fn a_world_nothing_has_happened_in_is_not_listed_yet() {
    let mut summary = WorldDocumentSummary {
        id: WorldDocumentId::new("fresh").unwrap(),
        pack: WorldPackRef::new("world-machine.pocket-universe", "0.25.0"),
        display_title: None,
        display_summary: None,
        display_scenery: None,
        display_calendar: None,
        display_marks: Vec::new(),
        display_moves_alone: false,
        display_cast: Vec::new(),
        display_drawings: Vec::new(),
        host_sittings: None,
        world_time: 0,
        event_count: 0,
    };
    assert!(!has_begun(&summary));
    summary.event_count = 3;
    assert!(has_begun(&summary));
}

#[test]
fn a_world_says_when_it_next_moves_on_its_own() {
    assert_eq!(
        keeps_going_line(5 * 3600 + 10, "sol", world_gpui::i18n::Rest::Waits),
        "Waits for you · a new sol in 6 h"
    );
    assert_eq!(
        keeps_going_line(20 * 60, "night", world_gpui::i18n::Rest::Asleep),
        "Asleep · a new night in 20 min"
    );
    assert_eq!(
        keeps_going_line(0, "day", world_gpui::i18n::Rest::Waits),
        "Waits for you · a new day comes with tomorrow's visit"
    );
}

#[test]
fn worlds_of_one_name_are_told_apart() {
    let same = ["tiny-society-1".to_string(), "tiny-society-2".to_string()];
    assert_eq!(
        distinct("Tiny Society", &same, "tiny-society-1"),
        "Tiny Society"
    );
    assert_eq!(
        distinct("Tiny Society", &same, "tiny-society-2"),
        "Tiny Society 2"
    );
    assert_eq!(distinct("Ares", &["ares".to_string()], "ares"), "Ares");
}

#[test]
fn world_summary_title_prefers_semantic_title_and_falls_back_cleanly() {
    let pack = WorldPackRef::new("pocket-universe", "0.10.0");
    let mut summary = WorldDocumentSummary {
        id: WorldDocumentId::new("mars").unwrap(),
        pack,
        display_title: Some("  Ares Pocket Colony  ".into()),
        display_summary: Some("  Current thread · Ridge Network  ".into()),
        display_scenery: None,
        display_calendar: None,
        display_marks: Vec::new(),
        display_moves_alone: false,
        display_cast: Vec::new(),
        display_drawings: Vec::new(),
        host_sittings: None,
        world_time: 3,
        event_count: 7,
    };

    assert_eq!(
        world_summary_title(&summary, "Pocket Universe"),
        "Ares Pocket Colony"
    );
    summary.display_title = Some("   ".into());
    assert_eq!(
        world_summary_title(&summary, "Pocket Universe"),
        "Pocket Universe"
    );
    summary.display_title = None;
    assert_eq!(
        world_summary_title(&summary, "Pocket Universe"),
        "Pocket Universe"
    );
}

#[test]
fn finding_a_world_matches_what_the_card_shows() {
    let query = normalize_world_search("  MAPLE  ");
    assert_eq!(query, "maple");

    assert!(world_matches_search(
        "Maple Street · 1987",
        "Pocket Universe",
        "pocket-universe-3",
        &query
    ));
    assert!(world_matches_search(
        "Ares Colony",
        "Pocket Universe",
        "maple-import",
        &query
    ));
    assert!(!world_matches_search(
        "Ares Colony",
        "Pocket Universe",
        "pocket-universe-3",
        &query
    ));

    // An empty box hides nothing.
    assert!(world_matches_search("Ares", "Tiny Society", "tiny-1", ""));
    // The World Pack's own title is searchable too.
    assert!(world_matches_search(
        "Ares",
        "Tiny Society",
        "tiny-1",
        &normalize_world_search("tiny")
    ));
}

#[test]
fn ordering_by_name_ignores_case_and_stays_stable() {
    let card = |id: &str, title: &str| {
        (
            title.to_owned(),
            WorldDocumentSummary {
                id: WorldDocumentId::new(id).unwrap(),
                pack: WorldPackRef::new("pocket-universe", "1.0.0"),
                display_title: Some(title.to_owned()),
                display_summary: None,
                display_scenery: None,
                display_calendar: None,
                display_marks: Vec::new(),
                display_moves_alone: false,
                display_cast: Vec::new(),
                display_drawings: Vec::new(),
                host_sittings: None,
                world_time: 0,
                event_count: 0,
            },
        )
    };
    let recent_order = vec![
        card("c", "zephyr"),
        card("a", "Maple Street"),
        card("b", "ares colony"),
        card("d", "Maple Street"),
    ];

    let mut untouched = recent_order.clone();
    sort_world_cards(&mut untouched, WorldSort::Recent);
    assert_eq!(
        untouched
            .iter()
            .map(|(_, document)| document.id.as_str())
            .collect::<Vec<_>>(),
        vec!["c", "a", "b", "d"],
        "Recent must leave the Library's own order alone"
    );

    let mut by_name = recent_order;
    sort_world_cards(&mut by_name, WorldSort::Name);
    assert_eq!(
        by_name
            .iter()
            .map(|(_, document)| document.id.as_str())
            .collect::<Vec<_>>(),
        vec!["b", "a", "d", "c"],
        "equal names fall back to the file identity so the order is stable"
    );
}

#[test]
fn the_my_worlds_heading_counts_what_is_hidden() {
    assert_eq!(my_worlds_title(7, 7), "My Worlds · 7");
    assert_eq!(my_worlds_title(2, 7), "My Worlds · 2/7");
    assert_eq!(my_worlds_title(0, 7), "My Worlds · 0/7");
}

#[test]
fn renaming_reports_the_name_or_the_pack_title_it_fell_back_to() {
    let named = rename_result_message(Some("Maple Street"), "Pocket Universe");
    assert_eq!(named, "Renamed to Maple Street.");
    assert!(
        !named.contains("Reload"),
        "an open World is renamed in place"
    );
    let cleared = rename_result_message(None, "Pocket Universe");
    assert_eq!(
        cleared,
        "Name cleared; this World is listed as Pocket Universe again."
    );
    assert_eq!(
        rename_placeholder("Pocket Universe"),
        "Pocket Universe — name this World"
    );
}

#[test]
fn removal_says_where_the_file_went() {
    let message = removal_result_message("Maple Street");
    assert!(message.starts_with("Removed Maple Street."));
    assert!(message.contains(world_library::REMOVED_DIRECTORY));
}

#[test]
fn unreadable_files_are_named_without_crowding_out_the_worlds() {
    let file = |name: &str| UnreadableWorldFile {
        file_name: name.into(),
        reason: "unexpected end of input".into(),
        age: world_library::FormatAge::Current,
        restorable: false,
    };

    assert_eq!(unreadable_documents_note(&[]), None);
    assert_eq!(
        unreadable_documents_note(&[file("truncated.world")]).as_deref(),
        Some(
            "One file in your Worlds folder could not be read and is not listed: truncated.world."
        )
    );
    assert_eq!(
        unreadable_documents_note(&[file("a.world"), file("b.world")]).as_deref(),
        Some(
            "2 files in your Worlds folder could not be read and are not listed: a.world, b.world."
        )
    );
    assert_eq!(
            unreadable_documents_note(&[
                file("a.world"),
                file("b.world"),
                file("c.world"),
                file("d.world"),
                file("e.world"),
            ])
            .as_deref(),
            Some(
                "5 files in your Worlds folder could not be read and are not listed: a.world, b.world, c.world, and 2 more."
            )
        );
}

#[test]
fn refused_files_are_said_with_their_reasons_not_as_damaged() {
    let file = |name: &str, age| UnreadableWorldFile {
        file_name: name.into(),
        reason: "This World is from a preview version and can't be opened.".into(),
        age,
        restorable: false,
    };
    let preview = world_library::FormatAge::Preview;
    let current = world_library::FormatAge::Current;

    let only_refused = [file("old.world", preview)];
    assert_eq!(unreadable_documents_note(&only_refused), None);
    let (refused, hidden) = refused_documents(&only_refused);
    assert_eq!(refused, vec![&only_refused[0]]);
    assert_eq!(hidden, 0);

    let mixed = [
        file("a.world", preview),
        file("b.world", current),
        file("c.world", preview),
        file("d.world", preview),
        file("e.world", preview),
    ];
    assert_eq!(
        unreadable_documents_note(&mixed).as_deref(),
        Some("One file in your Worlds folder could not be read and is not listed: b.world.")
    );
    let (refused, hidden) = refused_documents(&mixed);
    assert_eq!(
        refused
            .iter()
            .map(|file| file.file_name.as_str())
            .collect::<Vec<_>>(),
        vec!["a.world", "c.world", "d.world"]
    );
    assert_eq!(hidden, 1);
}

#[test]
fn suggested_world_file_name_prefers_semantic_unicode_title() {
    assert_eq!(
        suggested_world_file_name("  Maple Street · 1987  ", "pocket-universe-42"),
        "Maple Street · 1987.world"
    );
    assert_eq!(
        suggested_world_file_name("Ares / Ice: Colony?", "pocket-universe-42"),
        "Ares - Ice- Colony.world"
    );
}

#[test]
fn suggested_world_file_name_falls_back_to_durable_identity() {
    assert_eq!(
        suggested_world_file_name("   ", "pocket-universe-42"),
        "pocket-universe-42.world"
    );
    assert_eq!(suggested_world_file_name("", "kept.world"), "kept.world");
}

#[test]
fn a_world_is_called_by_its_name_and_falls_back_to_its_durable_identity() {
    assert_eq!(
        document_display_name(Some("  Maple Street · 1987  "), "pocket-universe-42"),
        "Maple Street · 1987"
    );
    assert_eq!(
        document_display_name(Some("   "), "pocket-universe-42"),
        "pocket-universe-42"
    );
    assert_eq!(
        document_display_name(None, "pocket-universe-42"),
        "pocket-universe-42"
    );
}

#[test]
fn document_window_title_carries_what_the_world_is_called() {
    assert_eq!(
        document_window_title("pocket-universe-42"),
        "pocket-universe-42 — World Machine"
    );
    assert_eq!(
        document_window_title("Maple Street · 1987"),
        "Maple Street · 1987 — World Machine"
    );
}

#[test]
fn start_intent_is_bound_to_exact_pack_identity() {
    let pocket_010 = WorldPackRef::new("pocket-universe", "0.10.0");
    let same = WorldPackRef::new("pocket-universe", "0.10.0");
    let newer = WorldPackRef::new("pocket-universe", "0.11.0");
    let other = WorldPackRef::new("micro-company", "0.10.0");

    assert!(start_after_install_matches(Some(&pocket_010), &same));
    assert!(!start_after_install_matches(Some(&pocket_010), &newer));
    assert!(!start_after_install_matches(Some(&pocket_010), &other));
    assert!(!start_after_install_matches(None, &pocket_010));
}

#[test]
fn system_open_distinguishes_world_documents_from_portable_packs() {
    assert!(is_world_file(Path::new("/tmp/example.world")));
    assert!(!is_world_pack_file(Path::new("/tmp/example.world")));

    assert!(is_world_pack_file(Path::new("/tmp/example.worldpack")));
    assert!(!is_world_file(Path::new("/tmp/example.worldpack")));

    assert!(!is_world_pack_file(Path::new(
        "/tmp/example.worldpack.backup"
    )));
    assert!(!is_world_pack_file(Path::new(
        "/tmp/example.world-pack.json"
    )));
}
