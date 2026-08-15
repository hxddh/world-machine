from pathlib import Path


def replace_exact(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing patch anchor: {label}")
    return text.replace(old, new, 1)


lib = Path("crates/world-projection/src/lib.rs")
text = lib.read_text()
text = replace_exact(
    text,
    "use world_core::{Entity, EntityId, Event, EventId, Value, World};",
    "use world_core::{Entity, EntityId, Event, EventId, StateChange, Value, World};",
    "projection imports",
)
text = replace_exact(
    text,
    '''    pub fn influence(&self, event: EventId) -> Vec<(usize, &TimelineItem)> {
        influence::influence_from_timeline(&self.timeline, event)
    }

    pub fn command(&self, id: &str) -> Option<&ProjectionCommand> {''',
    '''    pub fn influence(&self, event: EventId) -> Vec<(usize, &TimelineItem)> {
        influence::influence_from_timeline(&self.timeline, event)
    }

    pub fn semantic_influence(&self, event: EventId) -> Vec<(usize, &TimelineItem)> {
        influence::semantic_influence_from_snapshot(&self.timeline, &self.inspectors, event)
    }

    pub fn command(&self, id: &str) -> Option<&ProjectionCommand> {''',
    "semantic influence method",
)
text = replace_exact(
    text,
    '''    let payload = event
        .payload
        .iter()
        .map(|(key, value)| InspectorRow {
            label: humanize(key),
            value: value_text(value, world),
        })
        .collect::<Vec<_>>();

    let mut sections = vec![InspectorSection {
        title: "Context".into(),
        rows: context,
    }];
    if !payload.is_empty() {
        sections.push(InspectorSection {
            title: "Payload".into(),
            rows: payload,
        });
    }

    InspectorProjection {''',
    '''    let payload = event
        .payload
        .iter()
        .map(|(key, value)| InspectorRow {
            label: humanize(key),
            value: value_text(value, world),
        })
        .collect::<Vec<_>>();
    let changes = event
        .changes
        .iter()
        .map(|change| change_row(change, world))
        .collect::<Vec<_>>();

    let mut sections = vec![InspectorSection {
        title: "Context".into(),
        rows: context,
    }];
    if !payload.is_empty() {
        sections.push(InspectorSection {
            title: "Payload".into(),
            rows: payload,
        });
    }
    if !changes.is_empty() {
        sections.push(InspectorSection {
            title: "Changes".into(),
            rows: changes,
        });
    }

    InspectorProjection {''',
    "event changes section",
)
text = replace_exact(
    text,
    '''fn semantic_event_summary(event: &Event) -> Option<&str> {''',
    '''fn change_row(change: &StateChange, world: &World) -> InspectorRow {
    match change {
        StateChange::CreateEntity(entity) => InspectorRow {
            label: "Create entity".into(),
            value: entity_title(entity),
        },
        StateChange::RemoveEntity(entity) => InspectorRow {
            label: "Remove entity".into(),
            value: entity_reference(*entity, world),
        },
        StateChange::SetComponent { entity, key, value } => InspectorRow {
            label: format!("{} · {}", entity_reference(*entity, world), humanize(key)),
            value: value_text(value, world),
        },
        StateChange::RemoveComponent { entity, key } => InspectorRow {
            label: format!("{} · {}", entity_reference(*entity, world), humanize(key)),
            value: "Removed".into(),
        },
        StateChange::CreateRelation(relation) => InspectorRow {
            label: "Create relation".into(),
            value: format!(
                "{} · {} → {}",
                humanize(&relation.kind),
                entity_reference(relation.from, world),
                entity_reference(relation.to, world)
            ),
        },
        StateChange::RemoveRelation(relation) => InspectorRow {
            label: "Remove relation".into(),
            value: format!("Relation #{relation}"),
        },
        StateChange::SetRelationProperty {
            relation,
            key,
            value,
        } => InspectorRow {
            label: format!("Relation #{relation} · {}", humanize(key)),
            value: value_text(value, world),
        },
        StateChange::RemoveRelationProperty { relation, key } => InspectorRow {
            label: format!("Relation #{relation} · {}", humanize(key)),
            value: "Removed".into(),
        },
    }
}

fn entity_reference(entity: EntityId, world: &World) -> String {
    world
        .state()
        .entity(entity)
        .map(entity_title)
        .unwrap_or_else(|| format!("Entity #{entity}"))
}

fn semantic_event_summary(event: &Event) -> Option<&str> {''',
    "change formatter",
)
text = replace_exact(
    text,
    "    use world_core::{Entity, Event, EventId, WorldState};",
    "    use world_core::{Entity, Event, EventId, StateChange, WorldState};",
    "test imports",
)
text = replace_exact(
    text,
    '''    #[test]
    fn snapshot_command_lookup_is_generic() {''',
    '''    #[test]
    fn event_inspector_surfaces_recorded_state_changes() {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId::new(1), "workspace")
                    .with_component("name", "Workspace")
                    .with_component("status", "active"),
            )
            .unwrap();
        let world = World::from_history(
            state,
            &[Event {
                id: EventId::new(1),
                kind: "work_finished".into(),
                world_time: 1,
                actor: None,
                targets: vec![EntityId::new(1)],
                caused_by: vec![],
                payload: BTreeMap::new(),
                changes: vec![StateChange::SetComponent {
                    entity: EntityId::new(1),
                    key: "status".into(),
                    value: Value::Text("done".into()),
                }],
            }],
        )
        .unwrap();

        let inspectors = inspectors_from_world(&world);
        let changes = inspectors
            .get(&SelectionId::Event(EventId::new(1)))
            .unwrap()
            .sections
            .iter()
            .find(|section| section.title == "Changes")
            .expect("recorded StateChanges should be inspectable");
        assert_eq!(changes.rows.len(), 1);
        assert_eq!(changes.rows[0].label, "Workspace · Status");
        assert_eq!(changes.rows[0].value, "done");
    }

    #[test]
    fn snapshot_command_lookup_is_generic() {''',
    "changes test",
)
lib.write_text(text)


macos = Path("crates/world-gpui/src/macos.rs")
text = macos.read_text()
start = text.index("    fn render_influence(&self, cx: &mut Context<Self>) -> Option<Div> {")
end = text.index("    fn render_why(&self, cx: &mut Context<Self>) -> Option<Div> {", start)
new_render = '''    fn render_influence(&self, cx: &mut Context<Self>) -> Option<Div> {
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
text = text[:start] + new_render + text[end:]
old_node = '''        let prefix = if depth == 1 {
            "Direct effect".to_string()
        } else {
            format!("Later · {depth} steps")
        };'''
new_node = '''        let prefix = if depth == 1 {
            "Direct world effect".to_string()
        } else {
            format!("Later world effect · {depth} causal steps")
        };'''
text = replace_exact(text, old_node, new_node, "semantic influence labels")
macos.write_text(text)
