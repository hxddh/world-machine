from pathlib import Path

p = Path("crates/world-projection/src/lib.rs")
text = p.read_text()
text = text.replace(
    ".map(|change| change_row(change, world))",
    ".map(change_row)",
    1,
)
start = text.index("fn change_row(change: &StateChange")
end = text.index("fn semantic_event_summary(event: &Event)", start)
replacement = '''fn change_row(change: &StateChange) -> InspectorRow {
    match change {
        StateChange::CreateEntity(entity) => InspectorRow {
            label: "Create entity".into(),
            value: entity_title(entity),
        },
        StateChange::RemoveEntity(entity) => InspectorRow {
            label: "Remove entity".into(),
            value: format!("Entity #{entity}"),
        },
        StateChange::SetComponent { entity, key, value } => InspectorRow {
            label: format!("Entity #{entity} · {}", humanize(key)),
            value: recorded_value_text(value),
        },
        StateChange::RemoveComponent { entity, key } => InspectorRow {
            label: format!("Entity #{entity} · {}", humanize(key)),
            value: "Removed".into(),
        },
        StateChange::CreateRelation(relation) => InspectorRow {
            label: "Create relation".into(),
            value: format!(
                "{} · Entity #{} → Entity #{}",
                humanize(&relation.kind),
                relation.from,
                relation.to
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
            value: recorded_value_text(value),
        },
        StateChange::RemoveRelationProperty { relation, key } => InspectorRow {
            label: format!("Relation #{relation} · {}", humanize(key)),
            value: "Removed".into(),
        },
    }
}

fn recorded_value_text(value: &Value) -> String {
    match value {
        Value::Null => "—".into(),
        Value::Bool(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Text(value) => value.clone(),
        Value::Entity(entity) => format!("Entity #{entity}"),
        Value::List(values) => values
            .iter()
            .map(recorded_value_text)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Map(values) => values
            .iter()
            .map(|(key, value)| format!("{key}: {}", recorded_value_text(value)))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

'''
text = text[:start] + replacement + text[end:]
text = text.replace(
    'assert_eq!(changes.rows[0].label, "Workspace · Status");',
    'assert_eq!(changes.rows[0].label, "Entity #1 · Status");',
    1,
)
p.write_text(text)
