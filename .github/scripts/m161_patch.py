from pathlib import Path

path = Path('crates/world-strategy-gpui/src/lib.rs')
text = path.read_text()

old = '''const ENTITY_HISTORY_LIMIT: usize = 6;
const EVENT_ENTITY_EFFECT_LIMIT: usize = 6;
'''
new = '''const ENTITY_HISTORY_LIMIT: usize = 6;
const RELATION_HISTORY_LIMIT: usize = 6;
const EVENT_ENTITY_EFFECT_LIMIT: usize = 6;
const EVENT_RELATION_EFFECT_LIMIT: usize = 6;
'''
assert text.count(old) == 1, text.count(old)
text = text.replace(old, new)

marker = '''        if let SelectionId::Event(event) = selected.selection {
'''
assert text.count(marker) == 1, text.count(marker)
relation_history = '''        if let SelectionId::Relation(relation) = selected.selection {
            let history = snapshot.relation_history(relation);
            if !history.is_empty() {
                let mut history_list = div().flex().flex_col().gap_2();
                for item in history.iter().take(RELATION_HISTORY_LIMIT) {
                    history_list = history_list.child(self.render_relation_history_event(
                        selected.side,
                        item,
                        cx,
                    ));
                }
                if let Some(notice) = hidden_notice(
                    history.len(),
                    RELATION_HISTORY_LIMIT,
                    "recorded relation changes",
                ) {
                    history_list = history_list.child(truncation_notice(notice));
                }
                panel = panel
                    .child(div().text_sm().child("Recorded changes to this relation"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66705f))
                            .child("Recorded events that created, changed, or removed this relation incarnation. Select one to continue tracing on this same future."),
                    )
                    .child(history_list);
            }
        }

'''
text = text.replace(marker, relation_history + marker)

# Add changed relation cards inside the Event block, immediately before causal history.
needle = '''            let mut causes = div().flex().flex_col().gap_2();
'''
assert text.count(needle) == 1, text.count(needle)
relation_effects = '''            let changed_relations = snapshot.directly_changed_relations(event);
            if !changed_relations.is_empty() {
                let mut relations = div().flex().flex_col().gap_2();
                for relation in changed_relations.iter().take(EVENT_RELATION_EFFECT_LIMIT) {
                    relations = relations.child(self.render_event_relation_effect(
                        selected.side,
                        SelectionId::Relation(*relation),
                        cx,
                    ));
                }
                if let Some(notice) = hidden_notice(
                    changed_relations.len(),
                    EVENT_RELATION_EFFECT_LIMIT,
                    "directly changed relations",
                ) {
                    relations = relations.child(truncation_notice(notice));
                }
                panel = panel
                    .child(div().text_sm().child("Relations changed by this event"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66705f))
                            .child("Relation incarnations whose recorded lifetime or properties changed in this visible event. Removed relations remain inspectable as tombstones on this same future."),
                    )
                    .child(relations);
            }

'''
text = text.replace(needle, relation_effects + needle)

marker = '''    fn render_entity_history_event(
'''
assert text.count(marker) == 1, text.count(marker)
effect_fn = '''    fn render_event_relation_effect(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot(side)
            .and_then(|snapshot| snapshot.inspector(selection))
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Relation".into(), "Recorded relation".into()));
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "event-relation-effect-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                rgb(0x4e6fb3)
            } else {
                rgb(0xe2e4e8)
            })
            .bg(rgb(0xffffff))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(title))
            .child(div().text_xs().text_color(rgb(0x666666)).child(subtitle))
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

'''
text = text.replace(marker, effect_fn + marker)

marker = '''    fn render_evidence_inspector(&self, inspector: &InspectorProjection) -> Div {
'''
assert text.count(marker) == 1, text.count(marker)
history_fn = '''    fn render_relation_history_event(
        &self,
        side: ComparisonSide,
        item: &TimelineItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "relation-history-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                rgb(0x4e6fb3)
            } else {
                rgb(0xe2e4e8)
            })
            .bg(rgb(0xffffff))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x555555))
                    .child(item.subtitle.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(format!("World time {}", item.world_time)),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

'''
text = text.replace(marker, history_fn + marker)
path.write_text(text)
