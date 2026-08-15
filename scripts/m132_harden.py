from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if text.count(old) != 1:
        raise SystemExit(f"anchor count in {path}: {text.count(old)} for {old[:100]!r}")
    p.write_text(text.replace(old, new, 1))


path = "crates/world-compare/src/lib.rs"
replace_once(
    path,
    '''    let shared_len = left_chronological
        .iter()
        .zip(&right_chronological)
        .take_while(|(left, right)| *left == *right)
        .count();
''',
    '''    let shared_len = left_chronological
        .iter()
        .zip(&right_chronological)
        .take_while(|(left_item, right_item)| {
            same_recorded_timeline_event(left, right, left_item, right_item)
        })
        .count();
''',
)

replace_once(
    path,
    '''fn divergence_side(
''',
    '''fn same_recorded_timeline_event(
    left_snapshot: &ProjectionSnapshot,
    right_snapshot: &ProjectionSnapshot,
    left: &TimelineItem,
    right: &TimelineItem,
) -> bool {
    left.id == right.id
        && left.world_time == right.world_time
        && left.title == right.title
        && left.caused_by == right.caused_by
        && recorded_event_evidence(left_snapshot, left.id)
            == recorded_event_evidence(right_snapshot, right.id)
}

fn recorded_event_evidence(
    snapshot: &ProjectionSnapshot,
    selection: SelectionId,
) -> Vec<(String, String, String)> {
    let Some(inspector) = snapshot.inspectors.get(&selection) else {
        return Vec::new();
    };

    inspector
        .sections
        .iter()
        .flat_map(|section| {
            section.rows.iter().filter_map(move |row| {
                let recorded = section.title == "Changes"
                    || (section.title == "Payload"
                        && matches!(row.label.as_str(), "Summary" | "Change"));
                recorded.then(|| {
                    (
                        section.title.clone(),
                        row.label.clone(),
                        row.value.clone(),
                    )
                })
            })
        })
        .collect()
}

fn divergence_side(
''',
)

# Add hardening regressions immediately before the existing reconvergence test.
replace_once(
    path,
    '''    #[test]
    fn divergence_uses_the_longest_common_prefix_not_a_later_reconverged_event() {
''',
    '''    #[test]
    fn divergence_ignores_current_state_derived_timeline_display_drift_in_shared_history() {
        let left_common = TimelineItem {
            id: SelectionId::Event(EventId::new(1)),
            world_time: 1,
            title: "Common".into(),
            subtitle: "Alice · Event #1".into(),
            caused_by: vec![],
        };
        let right_common = TimelineItem {
            subtitle: "Renamed Alice · Event #1".into(),
            ..left_common.clone()
        };
        let left_first = event(2, "Left choice", 2);
        let right_first = event(2, "Right choice", 2);
        let left = snapshot(
            2,
            [],
            vec![left_first.clone(), left_common.clone()],
            vec![],
        );
        let right = snapshot(
            2,
            [],
            vec![right_first.clone(), right_common],
            vec![],
        );

        let divergence = compare_divergence(&left, &right).expect("histories diverged");
        assert_eq!(divergence.shared_frontier, Some(left_common));
        assert_eq!(divergence.left.first_difference, Some(left_first));
        assert_eq!(divergence.right.first_difference, Some(right_first));
    }

    #[test]
    fn divergence_detects_same_id_semantic_difference_from_recorded_event_evidence() {
        let item = TimelineItem {
            id: SelectionId::Event(EventId::new(1)),
            world_time: 1,
            title: "Choice Made".into(),
            subtitle: "Event #1".into(),
            caused_by: vec![],
        };
        let inspector = |summary: &str| {
            let selection = SelectionId::Event(EventId::new(1));
            (
                selection,
                InspectorProjection {
                    selection,
                    title: "Choice Made".into(),
                    subtitle: String::new(),
                    sections: vec![InspectorSection {
                        title: "Payload".into(),
                        rows: vec![InspectorRow {
                            label: "Summary".into(),
                            value: summary.into(),
                        }],
                    }],
                },
            )
        };
        let left = snapshot(1, [inspector("Outward")], vec![item.clone()], vec![]);
        let right = snapshot(1, [inspector("Rooted")], vec![item.clone()], vec![]);

        let divergence = compare_divergence(&left, &right).expect("recorded semantics differ");
        assert_eq!(divergence.shared_frontier, None);
        assert_eq!(divergence.left.first_difference, Some(item.clone()));
        assert_eq!(divergence.right.first_difference, Some(item));
    }

    #[test]
    fn divergence_uses_the_longest_common_prefix_not_a_later_reconverged_event() {
''',
)

path = "crates/world-strategy-gpui/src/lib.rs"
replace_once(
    path,
    '''        for stage in side.impact.iter().take(DIVERGENCE_IMPACT_LIMIT) {
            column = column.child(self.render_divergence_stage(stage));
        }
''',
    '''        for (index, stage) in side
            .impact
            .iter()
            .take(DIVERGENCE_IMPACT_LIMIT)
            .enumerate()
        {
            column = column.child(self.render_divergence_stage(stage, index == 0));
        }
''',
)
replace_once(
    path,
    '''    fn render_divergence_stage(&self, stage: &DivergenceImpactStage) -> Div {
        let supporting = stage.causal_steps.saturating_sub(1);
        let causal_context = if supporting == 0 {
            format!(
                "{} recorded causal {} from previous visible stage",
                stage.causal_steps,
                if stage.causal_steps == 1 {
                    "step"
                } else {
                    "steps"
                }
            )
        } else {
            format!(
                "{} recorded causal steps · {} supporting {} folded",
                stage.causal_steps,
                supporting,
                if supporting == 1 { "record" } else { "records" }
            )
        };
''',
    '''    fn render_divergence_stage(&self, stage: &DivergenceImpactStage, first: bool) -> Div {
        let supporting = stage.causal_steps.saturating_sub(1);
        let origin = if first {
            "first recorded difference"
        } else {
            "previous visible stage"
        };
        let causal_context = if supporting == 0 {
            format!(
                "{} recorded causal {} from {origin}",
                stage.causal_steps,
                if stage.causal_steps == 1 {
                    "step"
                } else {
                    "steps"
                }
            )
        } else {
            format!(
                "{} recorded causal steps from {origin} · {} supporting {} folded",
                stage.causal_steps,
                supporting,
                if supporting == 1 { "record" } else { "records" }
            )
        };
''',
)
