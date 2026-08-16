from pathlib import Path

path = Path('crates/world-strategy-gpui/src/lib.rs')
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

old_entity = '''        if let SelectionId::Entity(entity) = selected.selection {
            let history = snapshot.entity_history(entity);
'''
new_entity = '''        if let SelectionId::Entity(entity) = selected.selection {
            let relations = snapshot.relations_for_entity(entity);
            if !relations.is_empty() {
                let mut relation_list = div().flex().flex_col().gap_2();
                for relation in relations.iter().take(ENTITY_RELATION_LIMIT) {
                    relation_list = relation_list.child(self.render_entity_current_relation(
                        selected.side,
                        SelectionId::Relation(*relation),
                        cx,
                    ));
                }
                if let Some(notice) = hidden_notice(
                    relations.len(),
                    ENTITY_RELATION_LIMIT,
                    "current relations",
                ) {
                    relation_list = relation_list.child(truncation_notice(notice));
                }
                panel = panel
                    .child(div().text_sm().child("Current relations"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66705f))
                            .child("Active relations connected to this visible entity on the selected future. Select one to inspect that relation on the same side."),
                    )
                    .child(relation_list);
            }

            let history = snapshot.entity_history(entity);
'''
assert text.count(old_entity) == 1, text.count(old_entity)
text = text.replace(old_entity, new_entity)

old_relation = '''        if let SelectionId::Relation(relation) = selected.selection {
            let history = snapshot.relation_history(relation);
'''
new_relation = '''        if let SelectionId::Relation(relation) = selected.selection {
            let endpoints = snapshot.entities_for_relation(relation);
            if !endpoints.is_empty() {
                let mut endpoint_list = div().flex().flex_col().gap_2();
                for entity in endpoints.iter().take(RELATION_ENDPOINT_LIMIT) {
                    endpoint_list = endpoint_list.child(self.render_relation_current_endpoint(
                        selected.side,
                        SelectionId::Entity(*entity),
                        cx,
                    ));
                }
                if let Some(notice) = hidden_notice(
                    endpoints.len(),
                    RELATION_ENDPOINT_LIMIT,
                    "current relation endpoints",
                ) {
                    endpoint_list = endpoint_list.child(truncation_notice(notice));
                }
                panel = panel
                    .child(div().text_sm().child("Current endpoints"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66705f))
                            .child("Visible entities connected by this active relation on the selected future. Removed relation tombstones intentionally have no current endpoints."),
                    )
                    .child(endpoint_list);
            }

            let history = snapshot.relation_history(relation);
'''
assert text.count(old_relation) == 1, text.count(old_relation)
text = text.replace(old_relation, new_relation)

marker = '''    fn render_event_relation_effect(
'''
assert text.count(marker) == 1, text.count(marker)
helpers = '''    fn render_entity_current_relation(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot(side)
            .and_then(|snapshot| snapshot.inspector(selection))
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Relation".into(), "Active relation".into()));
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "entity-current-relation-{}-{}",
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

    fn render_relation_current_endpoint(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot(side)
            .and_then(|snapshot| snapshot.inspector(selection))
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Entity".into(), "Visible endpoint".into()));
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "relation-current-endpoint-{}-{}",
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
text = text.replace(marker, helpers + marker)
path.write_text(text)
