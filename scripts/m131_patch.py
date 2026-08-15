from pathlib import Path


def replace_between(text: str, start: str, end: str, replacement: str, label: str) -> str:
    i = text.find(start)
    if i < 0:
        raise SystemExit(f"missing start anchor: {label}")
    j = text.find(end, i)
    if j < 0:
        raise SystemExit(f"missing end anchor: {label}")
    return text[:i] + replacement + text[j:]


def replace_exact(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one anchor for {label}, found {count}")
    return text.replace(old, new, 1)


influence_path = Path("crates/world-projection/src/influence.rs")
influence = influence_path.read_text()
start = "pub(crate) fn semantic_path_from_snapshot<'a>("
end = "fn children_from_timeline"
replacement = r'''pub(crate) fn semantic_path_from_snapshot<'a>(
    timeline: &'a TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    root: EventId,
) -> Vec<&'a TimelineItem> {
    semantic_path_details_from_snapshot(timeline, inspectors, root)
        .into_iter()
        .map(|(_, item, _)| item)
        .collect()
}

pub(crate) fn semantic_path_details_from_snapshot<'a>(
    timeline: &'a TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    root: EventId,
) -> Vec<(usize, &'a TimelineItem, String)> {
    let by_id = timeline
        .items
        .iter()
        .filter_map(|item| match item.id {
            SelectionId::Event(event) => Some((event, item)),
            SelectionId::Entity(_) => None,
        })
        .collect::<BTreeMap<_, _>>();
    let full_path = selected_path_event_ids(timeline, inspectors, root, &by_id);
    if full_path.is_empty() {
        return Vec::new();
    }

    let mut causal_steps = 0_usize;
    let mut details = Vec::new();
    for event in full_path {
        causal_steps += 1;
        let Some(item) = by_id.get(&event).copied() else {
            return Vec::new();
        };
        if !inspector_has_world_effect(inspectors.get(&item.id)) {
            continue;
        }
        let effect = semantic_effect_from_snapshot(timeline, inspectors, event)
            .unwrap_or_else(|| item.subtitle.clone());
        details.push((causal_steps, item, effect));
        causal_steps = 0;
    }
    details
}

fn selected_path_event_ids(
    timeline: &TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    root: EventId,
    by_id: &BTreeMap<EventId, &TimelineItem>,
) -> Vec<EventId> {
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
    let mut descendant_memo = BTreeMap::new();
    let mut descendant_visiting = BTreeSet::new();
    let terminal = semantic_ids
        .iter()
        .copied()
        .filter(|event| {
            !has_semantic_descendant(
                *event,
                &children,
                &semantic_ids,
                &mut descendant_memo,
                &mut descendant_visiting,
            )
        })
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

    let mut memo = BTreeMap::<EventId, Option<BestPathState>>::new();
    let mut visiting = BTreeSet::new();
    if best_semantic_path_state(terminal, root, by_id, inspectors, &mut memo, &mut visiting)
        .is_none()
    {
        return Vec::new();
    }

    let mut path = Vec::new();
    let mut current = terminal;
    while current != root {
        path.push(current);
        let Some(state) = memo.get(&current).and_then(|state| *state) else {
            return Vec::new();
        };
        let Some(predecessor) = state.predecessor else {
            return Vec::new();
        };
        current = predecessor;
    }
    path.reverse();
    path
}

fn semantic_effect_from_snapshot(
    timeline: &TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    event: EventId,
) -> Option<String> {
    let inspector = inspectors.get(&SelectionId::Event(event))?;
    let payload = inspector.sections.iter().find(|section| section.title == "Payload");
    let summary = payload.and_then(|section| {
        section.rows.iter().find_map(|row| {
            matches!(row.label.as_str(), "Summary" | "Change")
                .then(|| row.value.trim())
                .filter(|value| !value.is_empty())
        })
    });
    let payload_labels = payload
        .map(|section| {
            section
                .rows
                .iter()
                .filter(|row| !matches!(row.label.as_str(), "Summary" | "Change"))
                .map(|row| row.label.as_str())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let changes = inspector
        .sections
        .iter()
        .find(|section| section.title == "Changes")
        .map(|section| section.rows.as_slice())
        .unwrap_or_default();

    let matched = changes
        .iter()
        .filter(|row| payload_labels.contains(change_field_label(&row.label)))
        .collect::<Vec<_>>();
    let evidence_rows = if !matched.is_empty() {
        matched
    } else if summary.is_none() {
        changes.iter().collect()
    } else {
        Vec::new()
    };
    let evidence = evidence_rows
        .iter()
        .take(2)
        .map(|row| recorded_transition(timeline, inspectors, event, row))
        .collect::<Vec<_>>();
    let hidden = evidence_rows.len().saturating_sub(evidence.len());

    match (summary, evidence.is_empty()) {
        (Some(summary), true) => Some(summary.to_string()),
        (Some(summary), false) => {
            let mut text = format!("{summary} · Recorded state · {}", evidence.join(" · "));
            if hidden > 0 {
                text.push_str(&format!(" · +{hidden} more recorded changes"));
            }
            Some(text)
        }
        (None, false) => {
            let mut text = format!("Recorded state · {}", evidence.join(" · "));
            if hidden > 0 {
                text.push_str(&format!(" · +{hidden} more recorded changes"));
            }
            Some(text)
        }
        (None, true) => None,
    }
}

fn change_field_label(label: &str) -> &str {
    label.rsplit_once(" · ").map_or(label, |(_, field)| field)
}

fn recorded_transition(
    timeline: &TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    event: EventId,
    row: &crate::InspectorRow,
) -> String {
    match previous_recorded_value(timeline, inspectors, event, &row.label) {
        Some(previous) if previous != row.value => {
            format!("{} {previous} → {}", row.label, row.value)
        }
        Some(_) => format!("{} = {}", row.label, row.value),
        None => format!("{} → {}", row.label, row.value),
    }
}

fn previous_recorded_value(
    timeline: &TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    event: EventId,
    label: &str,
) -> Option<String> {
    let current = timeline
        .items
        .iter()
        .position(|item| item.id == SelectionId::Event(event))?;
    timeline.items.iter().skip(current + 1).find_map(|item| {
        inspectors.get(&item.id).and_then(|inspector| {
            inspector
                .sections
                .iter()
                .find(|section| section.title == "Changes")
                .and_then(|section| section.rows.iter().find(|row| row.label == label))
                .map(|row| row.value.clone())
        })
    })
}

'''
influence = replace_between(influence, start, end, replacement, "semantic path implementation")

test_anchor = r'''    #[test]
    fn semantic_influence_keeps_world_changes_and_explicit_summaries_without_kind_rules() {'''
new_test = r'''    #[test]
    fn semantic_path_details_explain_recorded_effects_and_fold_supporting_spans() {
        let timeline = TimelineProjection {
            items: vec![
                item(5, "Final Effect", &[1, 4]),
                item(4, "Milestone", &[3]),
                item(3, "Supporting Record", &[2]),
                item(2, "First World Effect", &[1]),
                item(1, "Choice", &[]),
            ],
        };
        let change = |id, value: &str| {
            inspector(
                id,
                vec![InspectorSection {
                    title: "Changes".into(),
                    rows: vec![InspectorRow {
                        label: "Entity #1 · Status".into(),
                        value: value.into(),
                    }],
                }],
            )
        };
        let inspectors = BTreeMap::from([
            (SelectionId::Event(EventId::new(1)), change(1, "before")),
            (SelectionId::Event(EventId::new(2)), change(2, "first")),
            (SelectionId::Event(EventId::new(4)), change(4, "milestone")),
            (SelectionId::Event(EventId::new(5)), change(5, "final")),
        ]);

        let details = semantic_path_details_from_snapshot(&timeline, &inspectors, EventId::new(1));
        assert_eq!(details.len(), 3);
        assert_eq!(details[0].0, 1);
        assert_eq!(details[1].0, 2, "one supporting record should be folded between visible stages");
        assert_eq!(details[2].0, 1);
        assert_eq!(details[0].1.id, SelectionId::Event(EventId::new(2)));
        assert_eq!(details[1].1.id, SelectionId::Event(EventId::new(4)));
        assert_eq!(details[2].1.id, SelectionId::Event(EventId::new(5)));
        assert!(details[0].2.contains("before → first"));
        assert!(details[1].2.contains("first → milestone"));
        assert!(details[2].2.contains("milestone → final"));
    }

''' + test_anchor
influence = replace_exact(influence, test_anchor, new_test, "semantic detail test")
influence_path.write_text(influence)

lib_path = Path("crates/world-projection/src/lib.rs")
lib = lib_path.read_text()
old = r'''    pub fn semantic_path(&self, event: EventId) -> Vec<&TimelineItem> {
        influence::semantic_path_from_snapshot(&self.timeline, &self.inspectors, event)
    }
'''
new = old + r'''
    pub fn semantic_path_details(&self, event: EventId) -> Vec<(usize, &TimelineItem, String)> {
        influence::semantic_path_details_from_snapshot(&self.timeline, &self.inspectors, event)
    }
'''
lib = replace_exact(lib, old, new, "ProjectionSnapshot semantic path details")
lib_path.write_text(lib)

mac_path = Path("crates/world-gpui/src/macos.rs")
mac = mac_path.read_text()
mac = replace_exact(
    mac,
    "        let semantic_path = self.snapshot.semantic_path(event);\n",
    "        let semantic_path = self.snapshot.semantic_path_details(event);\n",
    "semantic path fetch",
)
mac = replace_exact(
    mac,
    r'''            if semantic_path
                .iter()
                .any(|path_item| path_item.id == item.id)
            {''',
    r'''            if semantic_path
                .iter()
                .any(|(_, path_item, _)| path_item.id == item.id)
            {''',
    "semantic path membership",
)
mac = replace_exact(
    mac,
    r'''                for (index, item) in semantic_path.iter().enumerate() {
                    path_nodes = path_nodes.child(self.semantic_path_node(index + 1, item, cx));
                }''',
    r'''                for (index, (causal_steps, item, effect)) in semantic_path.iter().enumerate() {
                    path_nodes = path_nodes.child(self.semantic_path_node(
                        index + 1,
                        *causal_steps,
                        item,
                        effect,
                        cx,
                    ));
                }''',
    "short semantic path rendering",
)
mac = replace_exact(
    mac,
    r'''                for (index, item) in semantic_path.iter().take(2).enumerate() {
                    path_nodes = path_nodes.child(self.semantic_path_node(index + 1, item, cx));
                }''',
    r'''                for (index, (causal_steps, item, effect)) in
                    semantic_path.iter().take(2).enumerate()
                {
                    path_nodes = path_nodes.child(self.semantic_path_node(
                        index + 1,
                        *causal_steps,
                        item,
                        effect,
                        cx,
                    ));
                }''',
    "long semantic path head",
)
mac = replace_exact(
    mac,
    r'''                for (index, item) in semantic_path.iter().enumerate().skip(path_len - 3) {
                    path_nodes = path_nodes.child(self.semantic_path_node(index + 1, item, cx));
                }''',
    r'''                for (index, (causal_steps, item, effect)) in
                    semantic_path.iter().enumerate().skip(path_len - 3)
                {
                    path_nodes = path_nodes.child(self.semantic_path_node(
                        index + 1,
                        *causal_steps,
                        item,
                        effect,
                        cx,
                    ));
                }''',
    "long semantic path tail",
)
node_start = "    fn semantic_path_node(\n"
node_end = "    fn influence_node(\n"
node_replacement = r'''    fn semantic_path_node(
        &self,
        stage: usize,
        causal_steps: usize,
        item: &TimelineItem,
        effect: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        let source = if stage == 1 {
            "selected Event"
        } else {
            "previous visible stage"
        };
        let causal_context = if causal_steps == 1 {
            format!("Stage {stage} · direct recorded causal step from {source}")
        } else {
            format!(
                "Stage {stage} · {causal_steps} recorded causal steps from {source} · {} supporting records folded",
                causal_steps - 1
            )
        };
        let event_ref = match selection {
            SelectionId::Event(event) => format!("World time {} · Event #{event}", item.world_time),
            SelectionId::Entity(_) => unreachable!("semantic path items must be Events"),
        };
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
                    .child(causal_context),
            )
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x4f5f4f))
                    .child(effect.to_string()),
            )
            .child(div().text_xs().text_color(rgb(0x777777)).child(event_ref))
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

'''
mac = replace_between(mac, node_start, node_end, node_replacement, "semantic path node")
mac_path.write_text(mac)

choice_path = Path("worlds/pocket-universe/tests/choice_influence.rs")
choice = choice_path.read_text()
helper_anchor = r'''fn semantic_path_signature(
    snapshot: &world_projection::ProjectionSnapshot,
    event: world_core::EventId,
) -> Vec<(world_projection::SelectionId, String)> {
    snapshot
        .semantic_path(event)
        .into_iter()
        .map(|item| (item.id, item.title.clone()))
        .collect()
}
'''
helper_new = helper_anchor + r'''
fn semantic_path_detail_signature(
    snapshot: &world_projection::ProjectionSnapshot,
    event: world_core::EventId,
) -> Vec<(usize, world_projection::SelectionId, String, String)> {
    snapshot
        .semantic_path_details(event)
        .into_iter()
        .map(|(steps, item, effect)| (steps, item.id, item.title.clone(), effect))
        .collect()
}
'''
choice = replace_exact(choice, helper_anchor, helper_new, "Pocket detail helper")
assert_anchor = r'''    assert!(shifted < partnership);

    let intervention = universe.invoke_projection_command(BOLD_PATH_COMMAND)?;'''
assert_new = r'''    assert!(shifted < partnership);

    let relationship_details = semantic_path_detail_signature(&relationship_snapshot, relationship);
    assert_eq!(relationship_details.len(), relationship_path.len());
    assert!(relationship_details.iter().all(|(_, _, title, _)| title != "Agent Decision Recorded"));
    assert!(relationship_details.iter().any(|(steps, _, _, _)| *steps > 1),
        "the readable thread should report when supporting causal records were folded");
    let shifted_effect = relationship_details
        .iter()
        .find(|(_, _, title, _)| title == "Relationship Shifted")
        .map(|(_, _, _, effect)| effect)
        .expect("relationship shift should carry recorded semantic evidence");
    assert!(shifted_effect.contains("Trust is"));
    assert!(shifted_effect.contains("Recorded state"));
    assert!(shifted_effect.contains("Trust"));
    let partnership_effect = relationship_details
        .iter()
        .find(|(_, _, title, _)| title == "Partnership Formed")
        .map(|(_, _, _, effect)| effect)
        .expect("resolved social arc should carry its recorded summary");
    assert!(partnership_effect.contains("one expedition crew"));

    let intervention = universe.invoke_projection_command(BOLD_PATH_COMMAND)?;'''
choice = replace_exact(choice, assert_anchor, assert_new, "Pocket semantic evidence assertions")
reopen_anchor = r'''    assert_eq!(
        semantic_path_signature(&reopened_snapshot, relationship),
        semantic_path_signature(&intervention_snapshot, relationship),
        "archive/reopen must reconstruct the same compressed causal thread from persisted Events"
    );'''
reopen_new = reopen_anchor + r'''
    assert_eq!(
        semantic_path_detail_signature(&reopened_snapshot, relationship),
        semantic_path_detail_signature(&intervention_snapshot, relationship),
        "archive/reopen must reconstruct the same recorded causal explanation"
    );'''
choice = replace_exact(choice, reopen_anchor, reopen_new, "Pocket reopen detail assertion")
choice_path.write_text(choice)
