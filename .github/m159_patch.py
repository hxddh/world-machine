from pathlib import Path

path = Path("crates/world-projection/src/lib.rs")
text = path.read_text()
old = "use world_core::{Entity, EntityId, Event, EventId, RelationId, StateChange, Value, World};\n"
new = "use world_core::{Entity, EntityId, Event, EventId, Relation, RelationId, StateChange, Value, World};\n"
assert text.count(old) == 1, f"import count: {text.count(old)}"
text = text.replace(old, new, 1)

old = '''    for entity in world.state().entities() {
        inspectors.insert(
            SelectionId::Entity(entity.id),
            inspector_for_entity(entity, world, &recorded_change_events),
        );
    }
    for event in world.events() {
'''
new = '''    for entity in world.state().entities() {
        inspectors.insert(
            SelectionId::Entity(entity.id),
            inspector_for_entity(entity, world, &recorded_change_events),
        );
    }
    for relation in world.state().relations() {
        inspectors.insert(
            SelectionId::Relation(relation.id),
            inspector_for_relation(relation, world),
        );
    }
    for event in world.events() {
'''
assert text.count(old) == 1, f"inspector loop count: {text.count(old)}"
text = text.replace(old, new, 1)

marker = '''fn inspector_for_event(event: &Event, world: &World) -> InspectorProjection {
'''
relation_fn = '''fn inspector_for_relation(relation: &Relation, world: &World) -> InspectorProjection {
    let endpoint_title = |entity| {
        world
            .state()
            .entity(entity)
            .map(entity_title)
            .unwrap_or_else(|| format!("Entity #{entity}"))
    };
    let context = vec![
        InspectorRow {
            label: "From".into(),
            value: endpoint_title(relation.from),
        },
        InspectorRow {
            label: "To".into(),
            value: endpoint_title(relation.to),
        },
    ];
    let properties = relation
        .properties
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
    if !properties.is_empty() {
        sections.push(InspectorSection {
            title: "Properties".into(),
            rows: properties,
        });
    }

    InspectorProjection {
        selection: SelectionId::Relation(relation.id),
        title: humanize(&relation.kind),
        subtitle: format!("Relation #{}", relation.id),
        sections,
    }
}

''' + marker
assert text.count(marker) == 1, f"event inspector marker count: {text.count(marker)}"
path.write_text(text.replace(marker, relation_fn, 1))
