from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if old not in text:
        raise SystemExit(f"anchor not found in {path}: {old[:120]!r}")
    if text.count(old) != 1:
        raise SystemExit(f"anchor not unique in {path}: {text.count(old)}")
    p.write_text(text.replace(old, new, 1))


# world-compare: add a derived divergence view without changing SnapshotComparison schema.
path = "crates/world-compare/src/lib.rs"
replace_once(
    path,
    "#[derive(Clone, Copy, Debug, Eq, PartialEq)]\npub enum DifferenceKind {",
    "#[derive(Clone, Debug, PartialEq)]\npub struct SnapshotDivergence {\n"
    "    pub shared_frontier: Option<TimelineItem>,\n"
    "    pub left: DivergenceSide,\n"
    "    pub right: DivergenceSide,\n"
    "}\n\n"
    "#[derive(Clone, Debug, Default, PartialEq)]\npub struct DivergenceSide {\n"
    "    pub first_difference: Option<TimelineItem>,\n"
    "    pub impact: Vec<DivergenceImpactStage>,\n"
    "}\n\n"
    "#[derive(Clone, Debug, PartialEq)]\npub struct DivergenceImpactStage {\n"
    "    pub causal_steps: usize,\n"
    "    pub event: TimelineItem,\n"
    "    pub effect: String,\n"
    "}\n\n"
    "#[derive(Clone, Copy, Debug, Eq, PartialEq)]\npub enum DifferenceKind {",
)

replace_once(
    path,
    "fn compare_entities(\n",
    "pub fn compare_divergence(\n"
    "    left: &ProjectionSnapshot,\n"
    "    right: &ProjectionSnapshot,\n"
    ") -> Option<SnapshotDivergence> {\n"
    "    let left_chronological = left.timeline.items.iter().rev().collect::<Vec<_>>();\n"
    "    let right_chronological = right.timeline.items.iter().rev().collect::<Vec<_>>();\n"
    "    let shared_len = left_chronological\n"
    "        .iter()\n"
    "        .zip(&right_chronological)\n"
    "        .take_while(|(left, right)| *left == *right)\n"
    "        .count();\n\n"
    "    if shared_len == left_chronological.len() && shared_len == right_chronological.len() {\n"
    "        return None;\n"
    "    }\n\n"
    "    Some(SnapshotDivergence {\n"
    "        shared_frontier: shared_len\n"
    "            .checked_sub(1)\n"
    "            .and_then(|index| left_chronological.get(index))\n"
    "            .map(|item| (*item).clone()),\n"
    "        left: divergence_side(left, &left_chronological, shared_len),\n"
    "        right: divergence_side(right, &right_chronological, shared_len),\n"
    "    })\n"
    "}\n\n"
    "fn divergence_side(\n"
    "    snapshot: &ProjectionSnapshot,\n"
    "    chronological: &[&TimelineItem],\n"
    "    shared_len: usize,\n"
    ") -> DivergenceSide {\n"
    "    let first_difference = chronological.get(shared_len).map(|item| (*item).clone());\n"
    "    let impact = first_difference\n"
    "        .as_ref()\n"
    "        .and_then(|item| match item.id {\n"
    "            SelectionId::Event(event) => Some(event),\n"
    "            SelectionId::Entity(_) => None,\n"
    "        })\n"
    "        .map(|event| {\n"
    "            snapshot\n"
    "                .semantic_path_details(event)\n"
    "                .into_iter()\n"
    "                .map(|(causal_steps, event, effect)| DivergenceImpactStage {\n"
    "                    causal_steps,\n"
    "                    event: event.clone(),\n"
    "                    effect,\n"
    "                })\n"
    "                .collect()\n"
    "        })\n"
    "        .unwrap_or_default();\n\n"
    "    DivergenceSide {\n"
    "        first_difference,\n"
    "        impact,\n"
    "    }\n"
    "}\n\n"
    "fn compare_entities(\n",
)

# Add focused generic tests before the test module closes.
p = Path(path)
text = p.read_text()
insert = r'''

    #[test]
    fn divergence_uses_the_longest_common_prefix_not_a_later_reconverged_event() {
        let common = event(1, "Common", 1);
        let left_first = event(2, "Left choice", 2);
        let right_first = event(2, "Right choice", 2);
        let reconverged = event(3, "Same later event", 3);
        let left = snapshot(
            3,
            [],
            vec![reconverged.clone(), left_first.clone(), common.clone()],
            vec![],
        );
        let right = snapshot(
            3,
            [],
            vec![reconverged, right_first.clone(), common.clone()],
            vec![],
        );

        let divergence = compare_divergence(&left, &right).expect("histories diverged");
        assert_eq!(divergence.shared_frontier, Some(common));
        assert_eq!(divergence.left.first_difference, Some(left_first));
        assert_eq!(divergence.right.first_difference, Some(right_first));
    }

    #[test]
    fn divergence_reuses_recorded_semantic_impact_from_each_first_difference() {
        let common = TimelineItem {
            id: SelectionId::Event(EventId::new(1)),
            world_time: 1,
            title: "Common".into(),
            subtitle: "Event #1".into(),
            caused_by: vec![],
        };
        let left_first = TimelineItem {
            id: SelectionId::Event(EventId::new(2)),
            world_time: 2,
            title: "Left choice".into(),
            subtitle: "Left choice · Event #2".into(),
            caused_by: vec![EventId::new(1)],
        };
        let left_support = TimelineItem {
            id: SelectionId::Event(EventId::new(3)),
            world_time: 3,
            title: "Supporting record".into(),
            subtitle: "Event #3".into(),
            caused_by: vec![EventId::new(2)],
        };
        let left_effect = TimelineItem {
            id: SelectionId::Event(EventId::new(4)),
            world_time: 4,
            title: "Left effect".into(),
            subtitle: "Left effect · Event #4".into(),
            caused_by: vec![EventId::new(3)],
        };
        let right_first = TimelineItem {
            id: SelectionId::Event(EventId::new(2)),
            world_time: 2,
            title: "Right choice".into(),
            subtitle: "Right choice · Event #2".into(),
            caused_by: vec![EventId::new(1)],
        };
        let right_effect = TimelineItem {
            id: SelectionId::Event(EventId::new(3)),
            world_time: 3,
            title: "Right effect".into(),
            subtitle: "Right effect · Event #3".into(),
            caused_by: vec![EventId::new(2)],
        };

        let effect_inspector = |id: u64, value: &str| {
            let selection = SelectionId::Event(EventId::new(id));
            (
                selection,
                InspectorProjection {
                    selection,
                    title: format!("Event {id}"),
                    subtitle: String::new(),
                    sections: vec![world_projection::InspectorSection {
                        title: "Changes".into(),
                        rows: vec![world_projection::InspectorRow {
                            label: "Entity #1 · State".into(),
                            value: value.into(),
                        }],
                    }],
                },
            )
        };

        let left = snapshot(
            4,
            [effect_inspector(4, "left")],
            vec![left_effect.clone(), left_support, left_first.clone(), common.clone()],
            vec![],
        );
        let right = snapshot(
            3,
            [effect_inspector(3, "right")],
            vec![right_effect.clone(), right_first.clone(), common],
            vec![],
        );

        let divergence = compare_divergence(&left, &right).expect("histories diverged");
        assert_eq!(divergence.left.first_difference, Some(left_first));
        assert_eq!(divergence.right.first_difference, Some(right_first));
        assert_eq!(divergence.left.impact.len(), 1);
        assert_eq!(divergence.left.impact[0].causal_steps, 2);
        assert_eq!(divergence.left.impact[0].event, left_effect);
        assert!(divergence.left.impact[0].effect.contains("left"));
        assert_eq!(divergence.right.impact.len(), 1);
        assert_eq!(divergence.right.impact[0].causal_steps, 1);
        assert_eq!(divergence.right.impact[0].event, right_effect);
        assert!(divergence.right.impact[0].effect.contains("right"));
    }

    #[test]
    fn ancestor_comparison_keeps_the_shared_frontier_and_one_sided_continuation() {
        let first = event(1, "First", 1);
        let frontier = event(2, "Frontier", 2);
        let continuation = event(3, "Continuation", 3);
        let left = snapshot(2, [], vec![frontier.clone(), first.clone()], vec![]);
        let right = snapshot(
            3,
            [],
            vec![continuation.clone(), frontier.clone(), first],
            vec![],
        );

        let divergence = compare_divergence(&left, &right).expect("right side continued");
        assert_eq!(divergence.shared_frontier, Some(frontier));
        assert_eq!(divergence.left.first_difference, None);
        assert_eq!(divergence.right.first_difference, Some(continuation));
    }
'''
# Insert before the final brace of the cfg(test) module.
idx = text.rfind("\n}")
if idx == -1:
    raise SystemExit("world-compare final module brace not found")
p.write_text(text[:idx] + insert + text[idx:])

# world-strategy-gpui: render divergence before the existing difference inventory.
path = "crates/world-strategy-gpui/src/lib.rs"
replace_once(
    path,
    "use world_compare::{\n    ChangedCommand, ChangedTimelineItem, DifferenceKind, EntityDifference, SnapshotComparison,\n};",
    "use world_compare::{\n"
    "    compare_divergence, ChangedCommand, ChangedTimelineItem, DifferenceKind, DivergenceImpactStage,\n"
    "    DivergenceSide, EntityDifference, SnapshotComparison, SnapshotDivergence,\n"
    "};",
)
replace_once(
    path,
    "const INSPECTOR_ROW_LIMIT: usize = 6;",
    "const INSPECTOR_ROW_LIMIT: usize = 6;\nconst DIVERGENCE_IMPACT_LIMIT: usize = 4;",
)

anchor = "    fn render_comparison(&self, comparison: &SnapshotComparison) -> Div {\n"
render_code = r'''    fn divergence(&self) -> Option<SnapshotDivergence> {
        match &self.source {
            ComparisonSource::Strategies(evaluation) => {
                match (&evaluation.left, &evaluation.right) {
                    (StrategyRun::Success(left), StrategyRun::Success(right)) => {
                        compare_divergence(&left.snapshot, &right.snapshot)
                    }
                    _ => None,
                }
            }
            ComparisonSource::Saved { left, right, .. } => compare_divergence(left, right),
        }
    }

    fn render_divergence(&self, divergence: &SnapshotDivergence) -> Div {
        let shared = match &divergence.shared_frontier {
            Some(frontier) => div()
                .p_3()
                .rounded_md()
                .bg(rgb(0xffffff))
                .border_1()
                .border_color(rgb(0xd9dfd5))
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x657565))
                        .child("SHARED HISTORY ENDS HERE"),
                )
                .child(div().text_sm().child(frontier.title.clone()))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x666666))
                        .child(frontier.subtitle.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x777777))
                        .child(format!("World time {}", frontier.world_time)),
                ),
            None => div()
                .p_3()
                .rounded_md()
                .bg(rgb(0xffffff))
                .border_1()
                .border_color(rgb(0xd9dfd5))
                .text_sm()
                .child("No identical recorded Timeline prefix"),
        };

        div()
            .w(px(700.0))
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xcfd8c8))
            .bg(rgb(0xf7faf5))
            .flex()
            .flex_col()
            .gap_3()
            .child(div().text_lg().child("Where these futures split"))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x66705f))
                    .child("Longest identical Timeline prefix, followed by each side's first recorded difference and its representative world-visible causal impact."),
            )
            .child(shared)
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(self.render_divergence_side(&self.left_label, &divergence.left))
                    .child(self.render_divergence_side(&self.right_label, &divergence.right)),
            )
    }

    fn render_divergence_side(&self, label: &str, side: &DivergenceSide) -> Div {
        let mut column = div()
            .w(px(325.0))
            .p_3()
            .rounded_md()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe2e4e8))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x4e6fb3))
                    .child(label.to_string()),
            );

        if let Some(first) = &side.first_difference {
            column = column
                .child(div().text_xs().text_color(rgb(0x777777)).child("FIRST RECORDED DIFFERENCE"))
                .child(div().text_sm().child(first.title.clone()))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x555555))
                        .child(first.subtitle.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x777777))
                        .child(format!("World time {}", first.world_time)),
                );
        } else {
            column = column.child(
                div()
                    .text_sm()
                    .text_color(rgb(0x777777))
                    .child("This side stops at the shared frontier."),
            );
            return column;
        }

        column = column.child(
            div()
                .text_xs()
                .text_color(rgb(0x657565))
                .child("HOW THIS FUTURE UNFOLDED"),
        );
        if side.impact.is_empty() {
            return column.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child("No later world-visible effects recorded from this difference yet."),
            );
        }

        for stage in side.impact.iter().take(DIVERGENCE_IMPACT_LIMIT) {
            column = column.child(self.render_divergence_stage(stage));
        }
        if let Some(notice) = hidden_notice(
            side.impact.len(),
            DIVERGENCE_IMPACT_LIMIT,
            "later impact stages",
        ) {
            column = column.child(truncation_notice(notice));
        }
        column
    }

    fn render_divergence_stage(&self, stage: &DivergenceImpactStage) -> Div {
        let supporting = stage.causal_steps.saturating_sub(1);
        let causal_context = if supporting == 0 {
            format!(
                "{} recorded causal {} from previous visible stage",
                stage.causal_steps,
                if stage.causal_steps == 1 { "step" } else { "steps" }
            )
        } else {
            format!(
                "{} recorded causal steps · {} supporting {} folded",
                stage.causal_steps,
                supporting,
                if supporting == 1 { "record" } else { "records" }
            )
        };
        div()
            .p_2()
            .rounded_md()
            .bg(rgb(0xf8f9fc))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x657565))
                    .child(causal_context),
            )
            .child(div().text_sm().child(stage.event.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x555555))
                    .child(stage.effect.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(format!("World time {}", stage.event.world_time)),
            )
    }

'''
replace_once(path, anchor, render_code + anchor)

replace_once(
    path,
    "        let comparison = match &self.source {\n",
    "        if let Some(divergence) = self.divergence() {\n"
    "            body = body.child(self.render_divergence(&divergence));\n"
    "        }\n\n"
    "        let comparison = match &self.source {\n",
)

# Pocket Universe: make the durable second-arc compare prove real divergence + downstream impact.
path = "worlds/pocket-universe/tests/second_arc_compare.rs"
replace_once(
    path,
    "use world_compare::{compare_snapshots, DifferenceKind, EntityDifference};",
    "use world_compare::{compare_divergence, compare_snapshots, DifferenceKind, EntityDifference};",
)
replace_once(
    path,
    "    assert!(comparison.timeline.changed.iter().any(|event| {\n        event.left.title == \"World Posture Chosen\"\n            && event.right.title == \"World Posture Chosen\"\n            && event.left.subtitle != event.right.subtitle\n    }));\n",
    "    assert!(comparison.timeline.changed.iter().any(|event| {\n"
    "        event.left.title == \"World Posture Chosen\"\n"
    "            && event.right.title == \"World Posture Chosen\"\n"
    "            && event.left.subtitle != event.right.subtitle\n"
    "    }));\n\n"
    "    let divergence = compare_divergence(&left, &right)\n"
    "        .expect(\"the two durable second-arc futures must diverge\");\n"
    "    let shared_frontier = divergence\n"
    "        .shared_frontier\n"
    "        .as_ref()\n"
    "        .expect(\"both futures share the full history before the posture choice\");\n"
    "    assert_ne!(shared_frontier.title, \"World Posture Chosen\");\n"
    "    let left_first = divergence\n"
    "        .left\n"
    "        .first_difference\n"
    "        .as_ref()\n"
    "        .expect(\"outward future has a first difference\");\n"
    "    let right_first = divergence\n"
    "        .right\n"
    "        .first_difference\n"
    "        .as_ref()\n"
    "        .expect(\"rooted future has a first difference\");\n"
    "    assert_eq!(left_first.title, \"World Posture Chosen\");\n"
    "    assert_eq!(right_first.title, \"World Posture Chosen\");\n"
    "    assert!(left_first.subtitle.contains(\"Outward\"));\n"
    "    assert!(right_first.subtitle.contains(\"Rooted\"));\n"
    "    assert!(!divergence.left.impact.is_empty());\n"
    "    assert!(!divergence.right.impact.is_empty());\n"
    "    assert!(divergence\n"
    "        .left\n"
    "        .impact\n"
    "        .iter()\n"
    "        .all(|stage| stage.event.title != \"Agent Decision Recorded\"));\n"
    "    assert!(divergence\n"
    "        .right\n"
    "        .impact\n"
    "        .iter()\n"
    "        .all(|stage| stage.event.title != \"Agent Decision Recorded\"));\n"
    "    assert_ne!(\n"
    "        divergence\n"
    "            .left\n"
    "            .impact\n"
    "            .iter()\n"
    "            .map(|stage| stage.effect.as_str())\n"
    "            .collect::<Vec<_>>(),\n"
    "        divergence\n"
    "            .right\n"
    "            .impact\n"
    "            .iter()\n"
    "            .map(|stage| stage.effect.as_str())\n"
    "            .collect::<Vec<_>>()\n"
    "    );\n",
)
