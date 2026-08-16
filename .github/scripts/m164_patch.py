from pathlib import Path

path = Path('crates/world-strategy-gpui/src/lib.rs')
text = path.read_text()

old_import = '''use world_compare::{
    compare_divergence, ChangedCommand, ChangedTimelineItem, DifferenceKind, DivergenceImpactStage,
    DivergenceSide, EntityDifference, SnapshotComparison, SnapshotDivergence,
};
'''
new_import = '''use world_compare::{
    compare_divergence, ChangedCommand, ChangedTimelineItem, DifferenceKind, DivergenceImpactStage,
    DivergenceSide, EntityDifference, RelationDifference, SnapshotComparison, SnapshotDivergence,
};
'''
assert text.count(old_import) == 1, text.count(old_import)
text = text.replace(old_import, new_import)

old_const = 'const ENTITY_DIFFERENCE_LIMIT: usize = 10;\n'
new_const = 'const ENTITY_DIFFERENCE_LIMIT: usize = 10;\nconst RELATION_DIFFERENCE_LIMIT: usize = 10;\n'
assert text.count(old_const) == 1
text = text.replace(old_const, new_const)

needle = '''        let mut timeline = div().flex().flex_col().gap_2();
'''
assert text.count(needle) == 1, text.count(needle)
relation_block = '''        let mut relations = div().flex().flex_col().gap_2();
        for difference in comparison
            .relations
            .iter()
            .take(RELATION_DIFFERENCE_LIMIT)
        {
            relations = relations.child(self.render_relation_difference(difference, cx));
        }
        if comparison.relations.is_empty() {
            relations = relations.child(
                div()
                    .text_sm()
                    .text_color(rgb(0x777777))
                    .child("No relation state differences"),
            );
        } else if let Some(notice) = hidden_notice(
            comparison.relations.len(),
            RELATION_DIFFERENCE_LIMIT,
            "relation differences",
        ) {
            relations = relations.child(truncation_notice(notice));
        }

'''
text = text.replace(needle, relation_block + needle)

old_text = '''                "Select an entity or timeline side to inspect evidence from that specific future.",
'''
new_text = '''                "Select an entity, relation, or timeline side to inspect evidence from that specific future.",
'''
assert text.count(old_text) == 1
text = text.replace(old_text, new_text)

old_chips = '''                    .child(summary_chip("Entities", comparison.entities.len()))
                    .child(summary_chip("Timeline", timeline_changes))
                    .child(summary_chip("Commands", command_changes)),
'''
new_chips = '''                    .child(summary_chip("Entities", comparison.entities.len()))
                    .child(summary_chip("Relations", comparison.relations.len()))
                    .child(summary_chip("Timeline", timeline_changes))
                    .child(summary_chip("Commands", command_changes)),
'''
assert text.count(old_chips) == 1
text = text.replace(old_chips, new_chips)

old_sections = '''            .child(div().text_sm().child("Entity state"))
            .child(entities)
            .child(div().text_sm().child("Timeline"))
            .child(timeline)
'''
new_sections = '''            .child(div().text_sm().child("Entity state"))
            .child(entities)
            .child(div().text_sm().child("Relation state"))
            .child(relations)
            .child(div().text_sm().child("Timeline"))
            .child(timeline)
'''
assert text.count(old_sections) == 1
text = text.replace(old_sections, new_sections)

marker = '''    fn render_entity_inspection_chip(
'''
assert text.count(marker) == 1
relation_render = '''    fn render_relation_difference(
        &self,
        difference: &RelationDifference,
        cx: &mut Context<Self>,
    ) -> Div {
        let title = difference
            .left
            .as_ref()
            .map(|relation| relation.title.clone())
            .or_else(|| difference.right.as_ref().map(|relation| relation.title.clone()))
            .unwrap_or_else(|| difference.id.stable_key());

        let mut inspection = div().flex().gap_2();
        if matches!(
            difference.kind,
            DifferenceKind::LeftOnly | DifferenceKind::Changed
        ) {
            inspection = inspection.child(self.render_relation_inspection_chip(
                ComparisonSide::Left,
                difference.id,
                cx,
            ));
        }
        if matches!(
            difference.kind,
            DifferenceKind::RightOnly | DifferenceKind::Changed
        ) {
            inspection = inspection.child(self.render_relation_inspection_chip(
                ComparisonSide::Right,
                difference.id,
                cx,
            ));
        }

        let mut rows = div().flex().flex_col().gap_1();
        if !difference.inspector_rows.is_empty() {
            rows = rows.child(
                div()
                    .flex()
                    .gap_2()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(div().w(px(150.0)).child("Field"))
                    .child(div().w(px(140.0)).child(self.left_label.clone()))
                    .child(div().w(px(140.0)).child(self.right_label.clone())),
            );
        }
        for row in difference.inspector_rows.iter().take(INSPECTOR_ROW_LIMIT) {
            rows = rows.child(
                div()
                    .flex()
                    .gap_2()
                    .text_xs()
                    .child(
                        div()
                            .w(px(150.0))
                            .text_color(rgb(0x666666))
                            .child(row.key.label.clone()),
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .child(row.left.clone().unwrap_or_else(|| "—".into())),
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .child(row.right.clone().unwrap_or_else(|| "—".into())),
                    ),
            );
        }
        if let Some(notice) = hidden_notice(
            difference.inspector_rows.len(),
            INSPECTOR_ROW_LIMIT,
            "changed fields",
        ) {
            rows = rows.child(truncation_notice(notice));
        }

        div()
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
                    .flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(div().text_sm().child(title))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x777777))
                                    .child(difference_kind_label(difference.kind)),
                            ),
                    )
                    .child(inspection),
            )
            .child(rows)
    }

    fn render_relation_inspection_chip(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "relation-difference-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                rgb(0x4e6fb3)
            } else {
                rgb(0xcfd6e5)
            })
            .bg(if selected {
                rgb(0xeef3ff)
            } else {
                rgb(0xf8f9fc)
            })
            .cursor_pointer()
            .text_xs()
            .text_color(rgb(0x4e6fb3))
            .child(format!(
                "Inspect {}",
                match side {
                    ComparisonSide::Left => "Left",
                    ComparisonSide::Right => "Right",
                }
            ))
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

'''
text = text.replace(marker, relation_render + marker)
path.write_text(text)
