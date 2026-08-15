from pathlib import Path

path = Path("crates/world-projection/src/influence.rs")
text = path.read_text()
old = '''fn recorded_transition(
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
'''
new = '''fn recorded_transition(
    timeline: &TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    event: EventId,
    row: &crate::InspectorRow,
) -> String {
    if !row.label.contains(" · ") {
        return format!("{} · {}", row.label, row.value);
    }
    match previous_recorded_value(timeline, inspectors, event, &row.label) {
        Some(previous) if previous != row.value => {
            format!("{} {previous} → {}", row.label, row.value)
        }
        Some(_) => format!("{} = {}", row.label, row.value),
        None => format!("{} → {}", row.label, row.value),
    }
}
'''
if text.count(old) != 1:
    raise SystemExit(f"recorded_transition anchor count={text.count(old)}")
text = text.replace(old, new, 1)
anchor = '''    #[test]
    fn semantic_influence_keeps_world_changes_and_explicit_summaries_without_kind_rules() {'''
test = '''    #[test]
    fn structural_change_labels_never_infer_a_previous_value_from_an_unrelated_event() {
        let timeline = TimelineProjection {
            items: vec![item(2, "Created", &[1]), item(1, "Earlier", &[])],
        };
        let create = |id, value: &str| {
            inspector(
                id,
                vec![InspectorSection {
                    title: "Changes".into(),
                    rows: vec![InspectorRow {
                        label: "Create entity".into(),
                        value: value.into(),
                    }],
                }],
            )
        };
        let inspectors = BTreeMap::from([
            (SelectionId::Event(EventId::new(1)), create(1, "First entity")),
            (SelectionId::Event(EventId::new(2)), create(2, "Second entity")),
        ]);

        let effect = semantic_effect_from_snapshot(&timeline, &inspectors, EventId::new(2))
            .expect("structural change should remain explainable");
        assert_eq!(effect, "Recorded state · Create entity · Second entity");
        assert!(!effect.contains("First entity"));
        assert!(!effect.contains("→"));
    }

''' + anchor
if text.count(anchor) != 1:
    raise SystemExit(f"test anchor count={text.count(anchor)}")
text = text.replace(anchor, test, 1)
path.write_text(text)
