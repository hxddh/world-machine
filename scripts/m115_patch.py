from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match in {path}, found {count}")
    file.write_text(text.replace(old, new, 1))


# Document metadata: add a generic, optional one-line current-state summary.
replace_once(
    "crates/world-document/src/lib.rs",
    '''pub struct WorldDocumentMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<WorldLineage>,
}''',
    '''pub struct WorldDocumentMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<WorldLineage>,
}''',
    "document metadata field",
)
replace_once(
    "crates/world-document/src/lib.rs",
    '''    pub fn is_empty(&self) -> bool {
        self.display_title.is_none() && self.lineage.is_none()
    }''',
    '''    pub fn is_empty(&self) -> bool {
        self.display_title.is_none() && self.display_summary.is_none() && self.lineage.is_none()
    }''',
    "metadata empty check",
)
replace_once(
    "crates/world-document/src/lib.rs",
    '''    pub fn with_display_title(mut self, title: impl Into<String>) -> Self {
        self.metadata.display_title = Some(title.into());
        self
    }

    pub fn with_lineage(mut self, lineage: WorldLineage) -> Self {''',
    '''    pub fn with_display_title(mut self, title: impl Into<String>) -> Self {
        self.metadata.display_title = Some(title.into());
        self
    }

    pub fn with_display_summary(mut self, summary: impl Into<String>) -> Self {
        self.metadata.display_summary = Some(summary.into());
        self
    }

    pub fn with_lineage(mut self, lineage: WorldLineage) -> Self {''',
    "document summary builder",
)
replace_once(
    "crates/world-document/src/lib.rs",
    '''    #[test]
    fn lineage_round_trips_inside_the_same_world_file() {''',
    '''    #[test]
    fn display_summary_round_trips_as_document_only_metadata() {
        let document = WorldDocument::new(archive(8))
            .with_display_title("A Small Mars")
            .with_display_summary("Ridge Network · care-led");

        let json = document.to_json_pretty().unwrap();
        let decoded = WorldDocument::from_json(&json).unwrap();
        let pure_archive = WorldArchive::from_json(&json).unwrap();

        assert_eq!(
            decoded.metadata.display_summary.as_deref(),
            Some("Ridge Network · care-led")
        );
        assert_eq!(pure_archive.world_time, 8);
        assert!(json.contains("\\\"display_summary\\\""));
    }

    #[test]
    fn lineage_round_trips_inside_the_same_world_file() {''',
    "document summary round trip test",
)

# World Library: derive compact summaries from the existing generic Briefing contract.
replace_once(
    "crates/world-library/src/lib.rs",
    '''pub struct WorldDocumentSummary {
    pub id: WorldDocumentId,
    pub pack: WorldPackRef,
    pub display_title: Option<String>,
    pub world_time: u64,
    pub event_count: usize,
}''',
    '''pub struct WorldDocumentSummary {
    pub id: WorldDocumentId,
    pub pack: WorldPackRef,
    pub display_title: Option<String>,
    pub display_summary: Option<String>,
    pub world_time: u64,
    pub event_count: usize,
}''',
    "library summary field",
)
replace_once(
    "crates/world-library/src/lib.rs",
    '''        let mut document = WorldDocument::new(archive);
        document.metadata.display_title = snapshot_display_title(&snapshot);
        let revision = library.save_document_with_revision(&document_id, &document)?;''',
    '''        let mut document = WorldDocument::new(archive);
        document.metadata.display_title = snapshot_display_title(&snapshot);
        document.metadata.display_summary = snapshot_display_summary(&snapshot);
        let revision = library.save_document_with_revision(&document_id, &document)?;''',
    "create persists summary",
)
replace_once(
    "crates/world-library/src/lib.rs",
    '''        self.revision = revision;
        self.metadata = document.metadata;
        self.session = replacement;
        Ok(snapshot)''',
    '''        let mut metadata = document.metadata;
        if metadata.display_summary.is_none() {
            metadata.display_summary = snapshot_display_summary(&snapshot);
        }
        self.revision = revision;
        self.metadata = metadata;
        self.session = replacement;
        Ok(snapshot)''',
    "reload backfills live summary",
)
replace_once(
    "crates/world-library/src/lib.rs",
    '''        if let Some(title) = snapshot_display_title(&snapshot) {
            next_metadata.display_title = Some(title);
        }
        let next_document = WorldDocument {''',
    '''        if let Some(title) = snapshot_display_title(&snapshot) {
            next_metadata.display_title = Some(title);
        }
        next_metadata.display_summary = snapshot_display_summary(&snapshot);
        let next_document = WorldDocument {''',
    "interactive summary update",
)
replace_once(
    "crates/world-library/src/lib.rs",
    '''fn snapshot_display_title(snapshot: &ProjectionSnapshot) -> Option<String> {
    let title = snapshot.title.trim();
    (!title.is_empty()).then(|| title.to_owned())
}

fn summary(id: WorldDocumentId, document: &WorldDocument) -> WorldDocumentSummary {''',
    '''fn snapshot_display_title(snapshot: &ProjectionSnapshot) -> Option<String> {
    let title = snapshot.title.trim();
    (!title.is_empty()).then(|| title.to_owned())
}

const DISPLAY_SUMMARY_MAX_CHARS: usize = 220;

pub fn snapshot_display_summary(snapshot: &ProjectionSnapshot) -> Option<String> {
    let item = snapshot.briefing.as_ref()?.items.first()?;
    let title = normalize_summary_text(&item.title);
    let detail = normalize_summary_text(&item.detail);
    let summary = match (title.is_empty(), detail.is_empty()) {
        (true, true) => return None,
        (false, true) => title,
        (true, false) => detail,
        (false, false) if title == detail => title,
        (false, false) => format!("{title} · {detail}"),
    };
    Some(truncate_summary(summary))
}

fn normalize_summary_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_summary(value: String) -> String {
    if value.chars().count() <= DISPLAY_SUMMARY_MAX_CHARS {
        return value;
    }
    let mut compact = value
        .chars()
        .take(DISPLAY_SUMMARY_MAX_CHARS - 1)
        .collect::<String>();
    compact.push('…');
    compact
}

fn summary(id: WorldDocumentId, document: &WorldDocument) -> WorldDocumentSummary {''',
    "summary extraction helper",
)
replace_once(
    "crates/world-library/src/lib.rs",
    '''        pack: document.archive.pack.clone(),
        display_title: document.metadata.display_title.clone(),
        world_time: document.archive.world_time,''',
    '''        pack: document.archive.pack.clone(),
        display_title: document.metadata.display_title.clone(),
        display_summary: document.metadata.display_summary.clone(),
        world_time: document.archive.world_time,''',
    "library list exposes summary",
)
replace_once(
    "crates/world-library/src/lib.rs",
    '''    use world_projection::{ProjectionCapabilities, ProjectionCommand};''',
    '''    use world_projection::{
        BriefingItem, BriefingProjection, ProjectionCapabilities, ProjectionCommand,
    };''',
    "library test briefing imports",
)
replace_once(
    "crates/world-library/src/lib.rs",
    '''                capabilities: ProjectionCapabilities { fork: false },
                commands: vec![ProjectionCommand {''',
    '''                capabilities: ProjectionCapabilities { fork: false },
                briefing: Some(BriefingProjection {
                    eyebrow: "Mock".into(),
                    title: "Current mock state".into(),
                    items: vec![BriefingItem {
                        selection: None,
                        title: format!("Count {}", self.count),
                        detail: format!("Current durable count {}", self.count),
                    }],
                }),
                commands: vec![ProjectionCommand {''',
    "library mock summary",
)
replace_once(
    "crates/world-library/src/lib.rs",
    '''        assert_eq!(
            library.list().unwrap()[0].display_title.as_deref(),
            Some("Mock 0")
        );''',
    '''        assert_eq!(
            library.list().unwrap()[0].display_title.as_deref(),
            Some("Mock 0")
        );
        assert_eq!(
            library.list().unwrap()[0].display_summary.as_deref(),
            Some("Count 0 · Current durable count 0")
        );''',
    "created summary assertion",
)
replace_once(
    "crates/world-library/src/lib.rs",
    '''        assert_eq!(
            library.list().unwrap()[0].display_title.as_deref(),
            Some("Mock 1")
        );''',
    '''        assert_eq!(
            library.list().unwrap()[0].display_title.as_deref(),
            Some("Mock 1")
        );
        assert_eq!(
            library.list().unwrap()[0].display_summary.as_deref(),
            Some("Count 1 · Current durable count 1")
        );''',
    "updated summary assertion",
)
replace_once(
    "crates/world-library/src/lib.rs",
    '''    #[test]
    fn snapshot_display_title_ignores_blank_titles() {''',
    '''    #[test]
    fn snapshot_display_summary_uses_the_first_briefing_item_and_compacts_whitespace() {
        let snapshot = ProjectionSnapshot {
            briefing: Some(BriefingProjection {
                eyebrow: "Test".into(),
                title: "Today".into(),
                items: vec![BriefingItem {
                    selection: None,
                    title: "  Ridge   Network ".into(),
                    detail: "  Routes   now   persist.  ".into(),
                }],
            }),
            ..ProjectionSnapshot::default()
        };

        assert_eq!(
            snapshot_display_summary(&snapshot).as_deref(),
            Some("Ridge Network · Routes now persist.")
        );
    }

    #[test]
    fn snapshot_display_summary_is_bounded_for_library_cards() {
        let snapshot = ProjectionSnapshot {
            briefing: Some(BriefingProjection {
                eyebrow: "Test".into(),
                title: "Today".into(),
                items: vec![BriefingItem {
                    selection: None,
                    title: "State".into(),
                    detail: "x".repeat(400),
                }],
            }),
            ..ProjectionSnapshot::default()
        };
        let summary = snapshot_display_summary(&snapshot).unwrap();

        assert_eq!(summary.chars().count(), DISPLAY_SUMMARY_MAX_CHARS);
        assert!(summary.ends_with('…'));
    }

    #[test]
    fn snapshot_display_title_ignores_blank_titles() {''',
    "summary helper tests",
)

# Background progression updates summary through the same durable transaction.
replace_once(
    "crates/world-library/src/revision/background.rs",
    '''    required_archive, snapshot_display_title, DurableWorldSession, LibraryError, WorldLibrary,
};''',
    '''    required_archive, snapshot_display_summary, snapshot_display_title, DurableWorldSession,
    LibraryError, WorldLibrary,
};''',
    "background summary import",
)
replace_once(
    "crates/world-library/src/revision/background.rs",
    '''        if let Some(title) = snapshot_display_title(&snapshot) {
            next_metadata.display_title = Some(title);
        }
        let next_document = WorldDocument {''',
    '''        if let Some(title) = snapshot_display_title(&snapshot) {
            next_metadata.display_title = Some(title);
        }
        next_metadata.display_summary = snapshot_display_summary(&snapshot);
        let next_document = WorldDocument {''',
    "background summary update",
)

# Forks synthesize current summary from the live projection, with metadata fallback.
replace_once(
    "crates/world-library/src/revision/fork.rs",
    '''    required_archive, snapshot_display_title, DurableWorldSession, LibraryError, WorldDocumentId,
    WorldDocumentSummary, WorldLibrary,
};''',
    '''    required_archive, snapshot_display_summary, snapshot_display_title, DurableWorldSession,
    LibraryError, WorldDocumentId, WorldDocumentSummary, WorldLibrary,
};''',
    "fork summary import",
)
replace_once(
    "crates/world-library/src/revision/fork.rs",
    '''        let mut fork = WorldDocument::new(archive).with_lineage(lineage);
        fork.metadata.display_title = snapshot_display_title(&self.session.snapshot())
            .or_else(|| self.metadata.display_title.clone());''',
    '''        let snapshot = self.session.snapshot();
        let mut fork = WorldDocument::new(archive).with_lineage(lineage);
        fork.metadata.display_title = snapshot_display_title(&snapshot)
            .or_else(|| self.metadata.display_title.clone());
        fork.metadata.display_summary = snapshot_display_summary(&snapshot)
            .or_else(|| self.metadata.display_summary.clone());''',
    "fork summary propagation",
)
replace_once(
    "crates/world-library/src/revision/fork.rs",
    '''        let metadata = WorldDocumentMetadata {
            display_title: Some("Fork Source".into()),
            lineage: Some(inherited_lineage()),
        };''',
    '''        let metadata = WorldDocumentMetadata {
            display_title: Some("Fork Source".into()),
            display_summary: Some("Fork source summary".into()),
            lineage: Some(inherited_lineage()),
        };''',
    "fork metadata literal",
)

# Save As metadata fixture includes the new field; preservation test now covers it too.
replace_once(
    "crates/world-library/src/save_as.rs",
    '''        WorldDocumentMetadata {
            display_title: Some("Saved World".into()),
            lineage: Some(WorldLineage {''',
    '''        WorldDocumentMetadata {
            display_title: Some("Saved World".into()),
            display_summary: Some("Durable saved summary".into()),
            lineage: Some(WorldLineage {''',
    "save-as metadata fixture",
)

# Regression fixture verifies interactive/background metadata replacement keeps lineage and refreshes summary.
replace_once(
    "crates/world-library/src/revision/metadata_regression.rs",
    '''use world_projection::{
    ProjectionCapabilities, ProjectionCommand, ProjectionIntent, ProjectionSnapshot,
};''',
    '''use world_projection::{
    BriefingItem, BriefingProjection, ProjectionCapabilities, ProjectionCommand, ProjectionIntent,
    ProjectionSnapshot,
};''',
    "metadata regression briefing imports",
)
replace_once(
    "crates/world-library/src/revision/metadata_regression.rs",
    '''            capabilities: ProjectionCapabilities { fork: false },
            commands: vec![ProjectionCommand {''',
    '''            capabilities: ProjectionCapabilities { fork: false },
            briefing: Some(BriefingProjection {
                eyebrow: "Metadata".into(),
                title: "Current state".into(),
                items: vec![BriefingItem {
                    selection: None,
                    title: format!("State {}", self.count),
                    detail: format!("Durable summary {}", self.count),
                }],
            }),
            commands: vec![ProjectionCommand {''',
    "metadata regression mock summary",
)
replace_once(
    "crates/world-library/src/revision/metadata_regression.rs",
    '''    WorldDocument::new(archive(world_time))
        .with_display_title(format!("Metadata Mock {world_time}"))
        .with_lineage(lineage(label))''',
    '''    WorldDocument::new(archive(world_time))
        .with_display_title(format!("Metadata Mock {world_time}"))
        .with_display_summary(format!("Original summary {world_time}"))
        .with_lineage(lineage(label))''',
    "metadata regression source summary",
)
replace_once(
    "crates/world-library/src/revision/metadata_regression.rs",
    '''    assert_eq!(
        session.metadata().display_title.as_deref(),
        Some("Metadata Mock 6")
    );''',
    '''    assert_eq!(
        session.metadata().display_title.as_deref(),
        Some("Metadata Mock 6")
    );
    assert_eq!(
        session.metadata().display_summary.as_deref(),
        Some("State 6 · Durable summary 6")
    );''',
    "interactive summary regression",
)
replace_once(
    "crates/world-library/src/revision/metadata_regression.rs",
    '''    assert_eq!(
        stored.metadata.display_title.as_deref(),
        Some("Metadata Mock 6")
    );''',
    '''    assert_eq!(
        stored.metadata.display_title.as_deref(),
        Some("Metadata Mock 6")
    );
    assert_eq!(
        stored.metadata.display_summary.as_deref(),
        Some("State 6 · Durable summary 6")
    );''',
    "stored interactive summary regression",
)
replace_once(
    "crates/world-library/src/revision/metadata_regression.rs",
    '''    assert_eq!(
        session.metadata().display_title.as_deref(),
        Some("Metadata Mock 8")
    );''',
    '''    assert_eq!(
        session.metadata().display_title.as_deref(),
        Some("Metadata Mock 8")
    );
    assert_eq!(
        session.metadata().display_summary.as_deref(),
        Some("State 8 · Durable summary 8")
    );''',
    "background summary regression",
)
replace_once(
    "crates/world-library/src/revision/metadata_regression.rs",
    '''    assert_eq!(
        stored.metadata.display_title.as_deref(),
        Some("Metadata Mock 8")
    );''',
    '''    assert_eq!(
        stored.metadata.display_title.as_deref(),
        Some("Metadata Mock 8")
    );
    assert_eq!(
        stored.metadata.display_summary.as_deref(),
        Some("State 8 · Durable summary 8")
    );''',
    "stored background summary regression",
)
replace_once(
    "crates/world-library/src/revision/metadata_regression.rs",
    '''    second.metadata = WorldDocumentMetadata {
        display_title: first.metadata.display_title.clone(),
        lineage: Some(lineage("second")),
    };''',
    '''    second.metadata = WorldDocumentMetadata {
        display_title: first.metadata.display_title.clone(),
        display_summary: first.metadata.display_summary.clone(),
        lineage: Some(lineage("second")),
    };''',
    "metadata conflict literal",
)

# Saved strategy futures carry the evaluated future's generic summary into their new documents.
replace_once(
    "apps/world-machine-desktop/src/strategy_compare.rs",
    '''use world_library::{DurableWorldSession, WorldDocumentId, WorldLibrary};''',
    '''use world_library::{
    snapshot_display_summary, DurableWorldSession, WorldDocumentId, WorldLibrary,
};''',
    "strategy summary import",
)
replace_once(
    "apps/world-machine-desktop/src/strategy_compare.rs",
    '''        let right_display_title = evaluation
            .right
            .outcome()
            .and_then(|outcome| strategy_future_display_title(&outcome.snapshot.title));
        let left_archive = evaluation''',
    '''        let right_display_title = evaluation
            .right
            .outcome()
            .and_then(|outcome| strategy_future_display_title(&outcome.snapshot.title));
        let left_display_summary = evaluation
            .left
            .outcome()
            .and_then(|outcome| snapshot_display_summary(&outcome.snapshot));
        let right_display_summary = evaluation
            .right
            .outcome()
            .and_then(|outcome| snapshot_display_summary(&outcome.snapshot));
        let left_archive = evaluation''',
    "strategy derives summaries",
)
replace_once(
    "apps/world-machine-desktop/src/strategy_compare.rs",
    '''                    left_display_title,
                    right_display_title,
                    left_lineage,''',
    '''                    left_display_title,
                    right_display_title,
                    left_display_summary,
                    right_display_summary,
                    left_lineage,''',
    "strategy result summaries",
)
replace_once(
    "apps/world-machine-desktop/src/strategy_compare.rs",
    '''    left_display_title: Option<String>,
    right_display_title: Option<String>,
    left_lineage: WorldLineage,''',
    '''    left_display_title: Option<String>,
    right_display_title: Option<String>,
    left_display_summary: Option<String>,
    right_display_summary: Option<String>,
    left_lineage: WorldLineage,''',
    "strategy result fields",
)
replace_once(
    "apps/world-machine-desktop/src/strategy_compare.rs",
    '''        let (archive, lineage, display_title, label, side_label) = match side {''',
    '''        let (archive, lineage, display_title, display_summary, label, side_label) = match side {''',
    "strategy save tuple",
)
replace_once(
    "apps/world-machine-desktop/src/strategy_compare.rs",
    '''                self.left_display_title.clone(),
                self.left_label.clone(),
                "Future A",''',
    '''                self.left_display_title.clone(),
                self.left_display_summary.clone(),
                self.left_label.clone(),
                "Future A",''',
    "left strategy summary save",
)
replace_once(
    "apps/world-machine-desktop/src/strategy_compare.rs",
    '''                self.right_display_title.clone(),
                self.right_label.clone(),
                "Future B",''',
    '''                self.right_display_title.clone(),
                self.right_display_summary.clone(),
                self.right_label.clone(),
                "Future B",''',
    "right strategy summary save",
)
replace_once(
    "apps/world-machine-desktop/src/strategy_compare.rs",
    '''        let future = strategy_future_document(archive, lineage, display_title);''',
    '''        let future =
            strategy_future_document(archive, lineage, display_title, display_summary);''',
    "strategy future document call",
)
replace_once(
    "apps/world-machine-desktop/src/strategy_compare.rs",
    '''fn strategy_future_document(
    archive: WorldArchive,
    lineage: WorldLineage,
    display_title: Option<String>,
) -> WorldDocument {
    let document = WorldDocument::new(archive).with_lineage(lineage);
    match display_title {
        Some(title) => document.with_display_title(title),
        None => document,
    }
}''',
    '''fn strategy_future_document(
    archive: WorldArchive,
    lineage: WorldLineage,
    display_title: Option<String>,
    display_summary: Option<String>,
) -> WorldDocument {
    let mut document = WorldDocument::new(archive).with_lineage(lineage);
    if let Some(title) = display_title {
        document = document.with_display_title(title);
    }
    if let Some(summary) = display_summary {
        document = document.with_display_summary(summary);
    }
    document
}''',
    "strategy future document metadata",
)
replace_once(
    "apps/world-machine-desktop/src/strategy_compare.rs",
    '''        let title = strategy_future_display_title("  Ares Pocket Colony  ");
        let document = strategy_future_document(archive, lineage, title);

        assert_eq!(
            document.metadata.display_title.as_deref(),
            Some("Ares Pocket Colony")
        );
        assert!(document.metadata.lineage.is_some());''',
    '''        let title = strategy_future_display_title("  Ares Pocket Colony  ");
        let document = strategy_future_document(
            archive,
            lineage,
            title,
            Some("World legacy · Ridge Network".into()),
        );

        assert_eq!(
            document.metadata.display_title.as_deref(),
            Some("Ares Pocket Colony")
        );
        assert_eq!(
            document.metadata.display_summary.as_deref(),
            Some("World legacy · Ridge Network")
        );
        assert!(document.metadata.lineage.is_some());''',
    "strategy future summary test",
)

# My Worlds renders the durable summary directly from metadata, without opening every World.
replace_once(
    "apps/world-machine-desktop/src/main.rs",
    '''        let mut details = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_lg().child(title.clone()))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child(if title == pack_title {
                        format!(
                            "World time {} · {} events",
                            document.world_time, document.event_count
                        )
                    } else {
                        format!(
                            "{} · World time {} · {} events",
                            pack_title, document.world_time, document.event_count
                        )
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x8a8a82))
                    .child(document_label.clone()),
            );''',
    '''        let mut details = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_lg().child(title.clone()));
        if let Some(summary) = world_summary_description(&document) {
            details = details.child(
                div()
                    .text_sm()
                    .text_color(rgb(0x4f5968))
                    .child(summary),
            );
        }
        details = details
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child(if title == pack_title {
                        format!(
                            "World time {} · {} events",
                            document.world_time, document.event_count
                        )
                    } else {
                        format!(
                            "{} · World time {} · {} events",
                            pack_title, document.world_time, document.event_count
                        )
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x8a8a82))
                    .child(document_label.clone()),
            );''',
    "library card summary",
)
replace_once(
    "apps/world-machine-desktop/src/main.rs",
    '''fn world_summary_title(document: &WorldDocumentSummary, pack_title: &str) -> String {
    document
        .display_title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| pack_title.to_owned())
}
''',
    '''fn world_summary_title(document: &WorldDocumentSummary, pack_title: &str) -> String {
    document
        .display_title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| pack_title.to_owned())
}

#[cfg(target_os = "macos")]
fn world_summary_description(document: &WorldDocumentSummary) -> Option<String> {
    document
        .display_summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .map(str::to_owned)
}
''',
    "library summary helper",
)
replace_once(
    "apps/world-machine-desktop/src/main.rs",
    '''            display_title: Some("  Ares Pocket Colony  ".into()),
            world_time: 3,
            event_count: 7,''',
    '''            display_title: Some("  Ares Pocket Colony  ".into()),
            display_summary: Some("  Current thread · Ridge Network  ".into()),
            world_time: 3,
            event_count: 7,''',
    "main summary test fixture",
)
replace_once(
    "apps/world-machine-desktop/src/main.rs",
    '''        assert_eq!(
            world_summary_title(&summary, "Pocket Universe"),
            "Ares Pocket Colony"
        );
        summary.display_title = Some("   ".into());''',
    '''        assert_eq!(
            world_summary_title(&summary, "Pocket Universe"),
            "Ares Pocket Colony"
        );
        assert_eq!(
            world_summary_description(&summary).as_deref(),
            Some("Current thread · Ridge Network")
        );
        summary.display_title = Some("   ".into());''',
    "main summary helper assertion",
)

# Update remaining explicit metadata literals known to this crate.
# (CI will catch any other exhaustive struct literal in the workspace.)
