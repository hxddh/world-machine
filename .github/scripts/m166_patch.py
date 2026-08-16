from pathlib import Path

path = Path('crates/world-gpui/src/macos.rs')
text = path.read_text()

old = '''const ENTITY_HISTORY_LIMIT: usize = 6;
const RELATION_HISTORY_LIMIT: usize = 6;
const EVENT_ENTITY_EFFECT_LIMIT: usize = 6;
const EVENT_RELATION_EFFECT_LIMIT: usize = 6;
'''
new = '''const ENTITY_HISTORY_LIMIT: usize = 6;
const RELATION_HISTORY_LIMIT: usize = 6;
const ENTITY_RELATION_LIMIT: usize = 6;
const RELATION_ENDPOINT_LIMIT: usize = 6;
const EVENT_ENTITY_EFFECT_LIMIT: usize = 6;
const EVENT_RELATION_EFFECT_LIMIT: usize = 6;
'''
assert text.count(old) == 1, text.count(old)
text = text.replace(old, new)

old_entity = '''        if let SelectionId::Entity(entity) = selection {
            let history = self.snapshot.entity_history(entity);
'''
new_entity = '''        if let SelectionId::Entity(entity) = selection {
            let relations = self.snapshot.relations_for_entity(entity);
            if !relations.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for relation in relations.iter().take(ENTITY_RELATION_LIMIT) {
                    items = items.child(
                        self.entity_relation_item(SelectionId::Relation(*relation), cx),
                    );
                }
                panel = panel
                    .child(div().text_sm().child("Current relations"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66705f))
                            .child("Active relations connected to this visible entity. Select one to inspect the relation and its recorded history."),
                    )
                    .child(items);
                let hidden = relations.len().saturating_sub(ENTITY_RELATION_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777777))
                            .child(format!("{hidden} more current relations not shown")),
                    );
                }
            }

            let history = self.snapshot.entity_history(entity);
'''
assert text.count(old_entity) == 1, text.count(old_entity)
text = text.replace(old_entity, new_entity)

old_relation = '''        if let SelectionId::Relation(relation) = selection {
            let history = self.snapshot.relation_history(relation);
'''
new_relation = '''        if let SelectionId::Relation(relation) = selection {
            let endpoints = self.snapshot.entities_for_relation(relation);
            if !endpoints.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for entity in endpoints.iter().take(RELATION_ENDPOINT_LIMIT) {
                    items = items.child(
                        self.relation_endpoint_item(SelectionId::Entity(*entity), cx),
                    );
                }
                panel = panel
                    .child(div().text_sm().child("Current endpoints"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66705f))
                            .child("Visible entities connected by this active relation. Removed relation tombstones intentionally have no current endpoints."),
                    )
                    .child(items);
                let hidden = endpoints.len().saturating_sub(RELATION_ENDPOINT_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777777))
                            .child(format!("{hidden} more current endpoints not shown")),
                    );
                }
            }

            let history = self.snapshot.relation_history(relation);
'''
assert text.count(old_relation) == 1, text.count(old_relation)
text = text.replace(old_relation, new_relation)

marker = '''    fn event_entity_effect_item(
'''
assert text.count(marker) == 1, text.count(marker)
helpers = '''    fn entity_relation_item(
        &self,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot
            .inspector(selection)
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Relation".into(), "Active relation".into()));
        div()
            .id(SharedString::from(format!(
                "entity-current-relation-{}",
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

    fn relation_endpoint_item(
        &self,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot
            .inspector(selection)
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Entity".into(), "Visible endpoint".into()));
        div()
            .id(SharedString::from(format!(
                "relation-current-endpoint-{}",
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
text = text.replace(marker, helpers + marker)
path.write_text(text)
