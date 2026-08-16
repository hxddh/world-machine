from pathlib import Path

path = Path('crates/world-gpui/src/macos.rs')
text = path.read_text()

old_constants = '''const ENTITY_HISTORY_LIMIT: usize = 6;
const EVENT_ENTITY_EFFECT_LIMIT: usize = 6;
'''
new_constants = '''const ENTITY_HISTORY_LIMIT: usize = 6;
const RELATION_HISTORY_LIMIT: usize = 6;
const EVENT_ENTITY_EFFECT_LIMIT: usize = 6;
const EVENT_RELATION_EFFECT_LIMIT: usize = 6;
'''
assert text.count(old_constants) == 1, text.count(old_constants)
text = text.replace(old_constants, new_constants)

marker = '''        if let SelectionId::Event(event) = selection {
'''
assert text.count(marker) == 1, text.count(marker)
relation_history = '''        if let SelectionId::Relation(relation) = selection {
            let history = self.snapshot.relation_history(relation);
            if !history.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for item in history.iter().take(RELATION_HISTORY_LIMIT) {
                    items = items.child(self.relation_history_item(item, cx));
                }
                panel = panel
                    .child(div().text_sm().child("Recorded changes to this relation"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66705f))
                            .child("Recorded events that created, changed, or removed this relation incarnation. Select one to inspect the event, trace its causes and effects, or fork before it."),
                    )
                    .child(items);
                let hidden = history.len().saturating_sub(RELATION_HISTORY_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777777))
                            .child(format!("{hidden} more recorded relation changes not shown")),
                    );
                }
            }
        }

'''
text = text.replace(marker, relation_history + marker)

start = text.index('    fn render_inspector(&self, cx: &mut Context<Self>) -> Option<Div> {')
end = text.index('\n    fn event_entity_effect_item(', start)
block = text[start:end]
needle = '\n        }\n\n        Some(panel)'
pos = block.rfind(needle)
assert pos != -1
relation_effects = '''

            let changed_relations = self.snapshot.directly_changed_relations(event);
            if !changed_relations.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for relation in changed_relations.iter().take(EVENT_RELATION_EFFECT_LIMIT) {
                    items = items.child(
                        self.event_relation_effect_item(SelectionId::Relation(*relation), cx),
                    );
                }
                panel = panel
                    .child(div().text_sm().child("Relations changed by this event"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66705f))
                            .child("Relation incarnations whose recorded lifetime or properties directly changed in this visible event. Removed relations remain inspectable as recorded tombstones."),
                    )
                    .child(items);
                let hidden = changed_relations
                    .len()
                    .saturating_sub(EVENT_RELATION_EFFECT_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777777))
                            .child(format!("{hidden} more directly changed relations not shown")),
                    );
                }
            }
'''
block = block[:pos] + relation_effects + block[pos:]
text = text[:start] + block + text[end:]

marker = '''    fn entity_history_item(&self, item: &TimelineItem, cx: &mut Context<Self>) -> impl IntoElement {
'''
assert text.count(marker) == 1, text.count(marker)
effect_fn = '''    fn event_relation_effect_item(
        &self,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot
            .inspector(selection)
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Relation".into(), "Recorded relation".into()));
        div()
            .id(SharedString::from(format!(
                "event-relation-effect-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xe2e4e8))
            .bg(rgb(0xf8f9fc))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(title))
            .child(div().text_xs().text_color(rgb(0x666666)).child(subtitle))
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

'''
text = text.replace(marker, effect_fn + marker)

marker = '''    fn render_influence(&self, cx: &mut Context<Self>) -> Option<Div> {
'''
assert text.count(marker) == 1, text.count(marker)
history_fn = '''    fn relation_history_item(&self, item: &TimelineItem, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = item.id;
        div()
            .id(SharedString::from(format!(
                "relation-history-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xe2e4e8))
            .bg(rgb(0xf8f9fc))
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
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

'''
text = text.replace(marker, history_fn + marker)
path.write_text(text)
