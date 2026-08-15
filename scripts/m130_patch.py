from pathlib import Path


def replace_exact(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing patch anchor: {label}")
    if text.count(old) != 1:
        raise SystemExit(f"patch anchor not unique: {label} ({text.count(old)})")
    return text.replace(old, new, 1)


# world-projection: derive one representative semantic causal thread from recorded DAG edges.
p = Path("crates/world-projection/src/influence.rs")
text = p.read_text()
text = replace_exact(
    text,
    '''    let mut children = BTreeMap::<EventId, Vec<&TimelineItem>>::new();
    for item in &timeline.items {
        for cause in &item.caused_by {
            children.entry(*cause).or_default().push(item);
        }
    }
''',
    '''    let children = children_from_timeline(timeline);
''',
    "reuse children map",
)
text = replace_exact(
    text,
    '''pub(crate) fn semantic_influence_from_snapshot<'a>(
    timeline: &'a TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    root: EventId,
) -> Vec<(usize, &'a TimelineItem)> {
    influence_from_timeline(timeline, root)
        .into_iter()
        .filter(|(_, item)| inspector_has_world_effect(inspectors.get(&item.id)))
        .collect()
}

fn inspector_has_world_effect(inspector: Option<&InspectorProjection>) -> bool {
''',
    '''pub(crate) fn semantic_influence_from_snapshot<'a>(
    timeline: &'a TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    root: EventId,
) -> Vec<(usize, &'a TimelineItem)> {
    influence_from_timeline(timeline, root)
        .into_iter()
        .filter(|(_, item)| inspector_has_world_effect(inspectors.get(&item.id)))
        .collect()
}

pub(crate) fn semantic_path_from_snapshot<'a>(
    timeline: &'a TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    root: EventId,
) -> Vec<&'a TimelineItem> {
    let by_id = timeline
        .items
        .iter()
        .filter_map(|item| match item.id {
            SelectionId::Event(event) => Some((event, item)),
            SelectionId::Entity(_) => None,
        })
        .collect::<BTreeMap<_, _>>();
    if !by_id.contains_key(&root) {
        return Vec::new();
    }

    let semantic = semantic_influence_from_snapshot(timeline, inspectors, root);
    if semantic.is_empty() {
        return Vec::new();
    }
    let semantic_ids = semantic
        .iter()
        .map(|(_, item)| event_id(item))
        .collect::<BTreeSet<_>>();
    let children = children_from_timeline(timeline);
    let terminal = semantic_ids
        .iter()
        .copied()
        .filter(|event| !has_semantic_descendant(*event, &children, &semantic_ids))
        .max_by(|left, right| {
            let left_item = by_id
                .get(left)
                .expect("semantic influence event must exist in Timeline");
            let right_item = by_id
                .get(right)
                .expect("semantic influence event must exist in Timeline");
            left_item
                .world_time
                .cmp(&right_item.world_time)
                .then_with(|| left.cmp(right))
        });
    let Some(terminal) = terminal else {
        return Vec::new();
    };

    let mut memo = BTreeMap::<EventId, Option<Vec<EventId>>>::new();
    let mut visiting = BTreeSet::new();
    let Some(path) = best_semantic_path_to(
        terminal,
        root,
        &by_id,
        inspectors,
        &mut memo,
        &mut visiting,
    ) else {
        return Vec::new();
    };

    path.into_iter()
        .filter_map(|event| by_id.get(&event).copied())
        .collect()
}

fn children_from_timeline(timeline: &TimelineProjection) -> BTreeMap<EventId, Vec<&TimelineItem>> {
    let mut children = BTreeMap::<EventId, Vec<&TimelineItem>>::new();
    for item in &timeline.items {
        for cause in &item.caused_by {
            children.entry(*cause).or_default().push(item);
        }
    }
    children
}

fn has_semantic_descendant(
    root: EventId,
    children: &BTreeMap<EventId, Vec<&TimelineItem>>,
    semantic: &BTreeSet<EventId>,
) -> bool {
    let mut visited = BTreeSet::from([root]);
    let mut queue = VecDeque::from([root]);
    while let Some(parent) = queue.pop_front() {
        let Some(next) = children.get(&parent) else {
            continue;
        };
        for item in next {
            let child = event_id(item);
            if !visited.insert(child) {
                continue;
            }
            if semantic.contains(&child) {
                return true;
            }
            queue.push_back(child);
        }
    }
    false
}

fn best_semantic_path_to(
    current: EventId,
    root: EventId,
    by_id: &BTreeMap<EventId, &TimelineItem>,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    memo: &mut BTreeMap<EventId, Option<Vec<EventId>>>,
    visiting: &mut BTreeSet<EventId>,
) -> Option<Vec<EventId>> {
    if current == root {
        return Some(Vec::new());
    }
    if let Some(cached) = memo.get(&current) {
        return cached.clone();
    }
    if !visiting.insert(current) {
        return None;
    }

    let result = by_id.get(&current).and_then(|item| {
        let semantic = inspector_has_world_effect(inspectors.get(&item.id));
        let mut best = None::<Vec<EventId>>;
        for cause in &item.caused_by {
            let Some(mut candidate) = best_semantic_path_to(
                *cause,
                root,
                by_id,
                inspectors,
                memo,
                visiting,
            ) else {
                continue;
            };
            if semantic {
                candidate.push(current);
            }
            let should_replace = best.as_ref().map_or(true, |existing| {
                candidate.len() > existing.len()
                    || (candidate.len() == existing.len() && candidate > *existing)
            });
            if should_replace {
                best = Some(candidate);
            }
        }
        best
    });

    visiting.remove(&current);
    memo.insert(current, result.clone());
    result
}

fn inspector_has_world_effect(inspector: Option<&InspectorProjection>) -> bool {
''',
    "semantic path derivation",
)
text = replace_exact(
    text,
    '''    #[test]
    fn semantic_influence_keeps_world_changes_and_explicit_summaries_without_kind_rules() {
''',
    '''    #[test]
    fn semantic_path_prefers_real_intermediate_world_stages_over_a_direct_shortcut() {
        let timeline = TimelineProjection {
            items: vec![
                item(5, "Final Effect", &[1, 4]),
                item(4, "Milestone", &[3]),
                item(3, "Supporting Record", &[2]),
                item(2, "First World Effect", &[1]),
                item(1, "Choice", &[]),
            ],
        };
        let changes = |id| {
            inspector(
                id,
                vec![InspectorSection {
                    title: "Changes".into(),
                    rows: vec![InspectorRow {
                        label: "Entity #1 · Status".into(),
                        value: format!("stage {id}"),
                    }],
                }],
            )
        };
        let inspectors = BTreeMap::from([
            (SelectionId::Event(EventId::new(2)), changes(2)),
            (SelectionId::Event(EventId::new(4)), changes(4)),
            (SelectionId::Event(EventId::new(5)), changes(5)),
        ]);

        let path = semantic_path_from_snapshot(&timeline, &inspectors, EventId::new(1));
        assert_eq!(
            path.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![
                SelectionId::Event(EventId::new(2)),
                SelectionId::Event(EventId::new(4)),
                SelectionId::Event(EventId::new(5)),
            ]
        );
    }

    #[test]
    fn semantic_influence_keeps_world_changes_and_explicit_summaries_without_kind_rules() {
''',
    "semantic path test",
)
p.write_text(text)


# ProjectionSnapshot: expose the derived thread without adding persisted/snapshot fields.
p = Path("crates/world-projection/src/lib.rs")
text = p.read_text()
text = replace_exact(
    text,
    '''    pub fn semantic_influence(&self, event: EventId) -> Vec<(usize, &TimelineItem)> {
        influence::semantic_influence_from_snapshot(&self.timeline, &self.inspectors, event)
    }

    pub fn command(&self, id: &str) -> Option<&ProjectionCommand> {
''',
    '''    pub fn semantic_influence(&self, event: EventId) -> Vec<(usize, &TimelineItem)> {
        influence::semantic_influence_from_snapshot(&self.timeline, &self.inspectors, event)
    }

    pub fn semantic_path(&self, event: EventId) -> Vec<&TimelineItem> {
        influence::semantic_path_from_snapshot(&self.timeline, &self.inspectors, event)
    }

    pub fn command(&self, id: &str) -> Option<&ProjectionCommand> {
''',
    "ProjectionSnapshot semantic_path",
)
p.write_text(text)


# Generic GPUI: show one compressed representative causal thread, then the remaining semantic branches.
p = Path("crates/world-gpui/src/macos.rs")
text = p.read_text()
old_render = '''    fn render_influence(&self, cx: &mut Context<Self>) -> Option<Div> {
        let SelectionId::Event(event) = self.selected? else {
            return None;
        };
        let raw_influence = self.snapshot.influence(event);
        if raw_influence.is_empty() {
            return None;
        }
        let semantic_influence = self.snapshot.semantic_influence(event);

        let recorded = raw_influence.len();
        let visible = semantic_influence.len();
        let folded = recorded.saturating_sub(visible);
        let direct = semantic_influence
            .iter()
            .filter(|(depth, _)| *depth == 1)
            .count();
        let max_depth = semantic_influence
            .iter()
            .map(|(depth, _)| *depth)
            .max()
            .unwrap_or_default();
        let mut nodes = div().flex().flex_col().gap_1();
        for (depth, item) in semantic_influence.iter().take(10) {
            nodes = nodes.child(self.influence_node(*depth, item, cx));
        }

        let summary = if visible == 0 {
            format!(
                "No world-visible effects yet · {recorded} recorded downstream {} · {folded} supporting {} folded",
                if recorded == 1 { "event" } else { "events" },
                if folded == 1 { "record" } else { "records" },
            )
        } else {
            format!(
                "{visible} world-visible {} from {recorded} recorded downstream {} · {direct} direct · {folded} supporting {} folded · up to {max_depth} causal {}",
                if visible == 1 { "effect" } else { "effects" },
                if recorded == 1 { "event" } else { "events" },
                if folded == 1 { "record" } else { "records" },
                if max_depth == 1 { "step" } else { "steps" },
            )
        };

        let mut panel = div()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd7e2d7))
            .bg(rgb(0xf7fbf7))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x657565))
                    .child("SEMANTIC IMPACT"),
            )
            .child(div().text_lg().child("What this affected"))
            .child(div().text_xs().text_color(rgb(0x657565)).child(summary))
            .child(nodes);

        if visible > 10 {
            panel = panel.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x657565))
                    .child(format!("+{} more world-visible effects", visible - 10)),
            );
        }
        if folded > 0 {
            panel = panel.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x657565))
                    .child("Supporting records remain available in Timeline and Why."),
            );
        }
        Some(panel)
    }
'''
new_render = '''    fn render_influence(&self, cx: &mut Context<Self>) -> Option<Div> {
        let SelectionId::Event(event) = self.selected? else {
            return None;
        };
        let raw_influence = self.snapshot.influence(event);
        if raw_influence.is_empty() {
            return None;
        }
        let semantic_influence = self.snapshot.semantic_influence(event);
        let semantic_path = self.snapshot.semantic_path(event);

        let recorded = raw_influence.len();
        let visible = semantic_influence.len();
        let folded = recorded.saturating_sub(visible);
        let direct = semantic_influence
            .iter()
            .filter(|(depth, _)| *depth == 1)
            .count();
        let max_depth = semantic_influence
            .iter()
            .map(|(depth, _)| *depth)
            .max()
            .unwrap_or_default();
        let mut other_nodes = div().flex().flex_col().gap_1();
        let mut other_count = 0_usize;
        for (depth, item) in &semantic_influence {
            if semantic_path.iter().any(|path_item| path_item.id == item.id) {
                continue;
            }
            other_count += 1;
            if other_count <= 6 {
                other_nodes = other_nodes.child(self.influence_node(*depth, item, cx));
            }
        }

        let summary = if visible == 0 {
            format!(
                "No world-visible effects yet · {recorded} recorded downstream {} · {folded} supporting {} folded",
                if recorded == 1 { "event" } else { "events" },
                if folded == 1 { "record" } else { "records" },
            )
        } else {
            format!(
                "{visible} world-visible {} from {recorded} recorded downstream {} · {direct} direct · {folded} supporting {} folded · up to {max_depth} causal {}",
                if visible == 1 { "effect" } else { "effects" },
                if recorded == 1 { "event" } else { "events" },
                if folded == 1 { "record" } else { "records" },
                if max_depth == 1 { "step" } else { "steps" },
            )
        };

        let mut panel = div()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd7e2d7))
            .bg(rgb(0xf7fbf7))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x657565))
                    .child("SEMANTIC IMPACT"),
            )
            .child(div().text_lg().child("What this affected"))
            .child(div().text_xs().text_color(rgb(0x657565)).child(summary));

        if !semantic_path.is_empty() {
            let path_len = semantic_path.len();
            let mut path_nodes = div().flex().flex_col().gap_1();
            if path_len <= 6 {
                for (index, item) in semantic_path.iter().enumerate() {
                    path_nodes = path_nodes.child(self.semantic_path_node(index + 1, item, cx));
                }
            } else {
                for (index, item) in semantic_path.iter().take(2).enumerate() {
                    path_nodes = path_nodes.child(self.semantic_path_node(index + 1, item, cx));
                }
                path_nodes = path_nodes.child(
                    div()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(rgb(0x657565))
                        .child(format!(
                            "+{} intermediate world-visible stages",
                            path_len - 5
                        )),
                );
                for (index, item) in semantic_path.iter().enumerate().skip(path_len - 3) {
                    path_nodes = path_nodes.child(self.semantic_path_node(index + 1, item, cx));
                }
            }
            panel = panel
                .child(div().text_xs().text_color(rgb(0x657565)).child("HOW IT UNFOLDED"))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x657565))
                        .child(format!(
                            "Representative causal thread from the selected Event to the latest downstream effect · {path_len} world-visible {}",
                            if path_len == 1 { "stage" } else { "stages" }
                        )),
                )
                .child(path_nodes);
        }

        if other_count > 0 {
            panel = panel
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x657565))
                        .child("OTHER WORLD-VISIBLE EFFECTS"),
                )
                .child(other_nodes);
            if other_count > 6 {
                panel = panel.child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x657565))
                        .child(format!("+{} more world-visible effects", other_count - 6)),
                );
            }
        }
        if folded > 0 {
            panel = panel.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x657565))
                    .child("Supporting records remain available in Timeline and Why."),
            );
        }
        Some(panel)
    }
'''
text = replace_exact(text, old_render, new_render, "GPUI semantic impact renderer")
text = replace_exact(
    text,
    '''    fn influence_node(
        &self,
        depth: usize,
        item: &TimelineItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
''',
    '''    fn semantic_path_node(
        &self,
        stage: usize,
        item: &TimelineItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        div()
            .id(SharedString::from(format!(
                "semantic-path-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .bg(rgb(0xffffff))
            .cursor_pointer()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x657565))
                    .child(format!("Stage {stage}")),
            )
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(item.subtitle.clone()),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn influence_node(
        &self,
        depth: usize,
        item: &TimelineItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
''',
    "GPUI semantic path node",
)
p.write_text(text)


# Pocket Universe: prove the generic thread follows real M127 edges and survives archive/reopen.
p = Path("worlds/pocket-universe/tests/choice_influence.rs")
text = p.read_text()
text = replace_exact(
    text,
    '''fn semantic_influence_signature(
    snapshot: &world_projection::ProjectionSnapshot,
    event: world_core::EventId,
) -> Vec<(usize, world_projection::SelectionId, String)> {
    snapshot
        .semantic_influence(event)
        .into_iter()
        .map(|(depth, item)| (depth, item.id, item.title.clone()))
        .collect()
}
''',
    '''fn semantic_influence_signature(
    snapshot: &world_projection::ProjectionSnapshot,
    event: world_core::EventId,
) -> Vec<(usize, world_projection::SelectionId, String)> {
    snapshot
        .semantic_influence(event)
        .into_iter()
        .map(|(depth, item)| (depth, item.id, item.title.clone()))
        .collect()
}

fn semantic_path_signature(
    snapshot: &world_projection::ProjectionSnapshot,
    event: world_core::EventId,
) -> Vec<(world_projection::SelectionId, String)> {
    snapshot
        .semantic_path(event)
        .into_iter()
        .map(|item| (item.id, item.title.clone()))
        .collect()
}
''',
    "Pocket semantic path helper",
)
text = replace_exact(
    text,
    '''    assert!(semantic_relationship.len() < raw_relationship.len());

    let intervention = universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
''',
    '''    assert!(semantic_relationship.len() < raw_relationship.len());

    let relationship_path = semantic_path_signature(&relationship_snapshot, relationship);
    assert!(relationship_path.len() >= 3);
    assert!(relationship_path
        .iter()
        .all(|(_, title)| title != "Agent Decision Recorded"));
    let shifted = relationship_path
        .iter()
        .position(|(_, title)| title == "Relationship Shifted")
        .expect("the compressed thread should include the materialized relationship shift");
    let partnership = relationship_path
        .iter()
        .position(|(_, title)| title == "Partnership Formed")
        .expect("the latest relationship thread should reach the resolved social arc");
    assert!(shifted < partnership);

    let intervention = universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
''',
    "Pocket relationship thread assertions",
)
text = replace_exact(
    text,
    '''    assert_eq!(
        semantic_influence_signature(&reopened_snapshot, intervention),
        semantic_intervention,
        "archive/reopen must reconstruct the same semantic influence from persisted Events"
    );
''',
    '''    assert_eq!(
        semantic_influence_signature(&reopened_snapshot, intervention),
        semantic_intervention,
        "archive/reopen must reconstruct the same semantic influence from persisted Events"
    );
    assert_eq!(
        semantic_path_signature(&reopened_snapshot, relationship),
        semantic_path_signature(&intervention_snapshot, relationship),
        "archive/reopen must reconstruct the same compressed causal thread from persisted Events"
    );
''',
    "Pocket archive thread assertion",
)
p.write_text(text)
