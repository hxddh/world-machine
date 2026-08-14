from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


lib_path = Path("worlds/pocket-universe/src/lib.rs")
lib = lib_path.read_text()

lib = replace_once(lib, "mod projection;", "mod legacy;\nmod projection;", "legacy module")
lib = replace_once(
    lib,
    'pub(crate) const POSTURE: &str = "posture";\npub(crate) const RELATIONSHIP_DIRECTION: &str = "direction";',
    'pub(crate) const POSTURE: &str = "posture";\npub(crate) const POSTURE_GENERATION: &str = "posture_generation";\npub(crate) const LEGACY: &str = "legacy";\npub(crate) const LEGACY_SUMMARY: &str = "legacy_summary";\npub(crate) const RELATIONSHIP_DIRECTION: &str = "direction";',
    "legacy state keys",
)
lib = replace_once(
    lib,
    '''            let returned = if social_arc_candidate(candidate.state())?.is_some() {
                candidate
                    .execute(
                        &self.actions,
                        &ActionRequest::new("resolve_social_arc").caused_by(relationship),
                    )?
                    .id
            } else {
                relationship
            };''',
    '''            let returned =
                legacy::resolve_period_consequences(&mut candidate, &self.actions, relationship)?;''',
    "nudge consequence hook",
)
lib = replace_once(
    lib,
    '''            if social_arc_candidate(candidate.state())?.is_some() {
                candidate.execute(
                    &self.actions,
                    &ActionRequest::new("resolve_social_arc").caused_by(relationship),
                )?;
            }''',
    '''            legacy::resolve_period_consequences(&mut candidate, &self.actions, relationship)?;''',
    "background consequence hook",
)
lib = replace_once(
    lib,
    '''            .with_component(DECISION, "none")
            .with_component(POSTURE, "none")
            .with_component(LAST_CHANGE, "Nothing exists here yet."),''',
    '''            .with_component(DECISION, "none")
            .with_component(POSTURE, "none")
            .with_component(POSTURE_GENERATION, 0_i64)
            .with_component(LEGACY, "forming")
            .with_component(LEGACY_SUMMARY, "")
            .with_component(LAST_CHANGE, "Nothing exists here yet."),''',
    "baseline legacy state",
)
lib = replace_once(
    lib,
    '''    actions.register(ResolveSocialArc)?;
    actions.register(SteerSharedProject)?;''',
    '''    actions.register(ResolveSocialArc)?;
    legacy::register_actions(&mut actions)?;
    actions.register(SteerSharedProject)?;''',
    "register legacy action",
)
lib = replace_once(
    lib,
    '''        let posture = posture_id_from_state(state)?;
        let social_arc = text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC)?;
        let change = growth_message(&seed, next, &decision, &social_arc, &posture);''',
    '''        let posture = posture_id_from_state(state)?;
        let social_arc = text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC)?;
        let legacy = legacy::legacy_id_from_state(state)?;
        let change = growth_message(&seed, next, &decision, &social_arc, &posture, &legacy);''',
    "growth reads legacy",
)
lib = replace_once(
    lib,
    '''    if integer_component(state, UNIVERSE, GENERATION)? < 6 {
        return Err(ActionError::Invalid(
            "this Pocket Universe has not reached its second chapter yet".into(),
        ));
    }''',
    '''    let generation = integer_component(state, UNIVERSE, GENERATION)?;
    if generation < 6 {
        return Err(ActionError::Invalid(
            "this Pocket Universe has not reached its second chapter yet".into(),
        ));
    }''',
    "capture posture generation",
)
lib = replace_once(
    lib,
    '''        StateChange::SetComponent {
            entity: UNIVERSE,
            key: POSTURE.into(),
            value: posture.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),''',
    '''        StateChange::SetComponent {
            entity: UNIVERSE,
            key: POSTURE.into(),
            value: posture.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: POSTURE_GENERATION.into(),
            value: generation.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),''',
    "persist posture generation",
)
lib = replace_once(
    lib,
    '''        StateChange::SetComponent {
            entity: UNIVERSE,
            key: POSTURE.into(),
            value: "none".into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),''',
    '''        StateChange::SetComponent {
            entity: UNIVERSE,
            key: POSTURE.into(),
            value: "none".into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: POSTURE_GENERATION.into(),
            value: 0_i64.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LEGACY.into(),
            value: "forming".into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LEGACY_SUMMARY.into(),
            value: "".into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),''',
    "seed resets legacy state",
)
lib = replace_once(
    lib,
    '''    social_arc: &str,
    posture: &str,
) -> String {''',
    '''    social_arc: &str,
    posture: &str,
    legacy: &str,
) -> String {''',
    "growth legacy argument",
)
lib = replace_once(
    lib,
    '''    if let Some(posture_consequence) = posture_consequence {
        story.push(' ');
        story.push_str(posture_consequence);
    }
    story
}''',
    '''    if let Some(posture_consequence) = posture_consequence {
        story.push(' ');
        story.push_str(posture_consequence);
    }
    if let Some(legacy_consequence) = legacy::growth_consequence(seed, legacy) {
        story.push(' ');
        story.push_str(legacy_consequence);
    }
    story
}''',
    "legacy growth consequence",
)
lib_path.write_text(lib)

projection_path = Path("worlds/pocket-universe/src/projection.rs")
projection = projection_path.read_text()
projection = replace_once(
    projection,
    '''    seed_id, BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND, DECISION, GENERATION, LAST_CHANGE,
    NUDGE_COMMAND, OUTWARD_POSTURE_COMMAND, POSTURE, RELATIONSHIP, RELATIONSHIP_DIRECTION,''',
    '''    seed_id, BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND, DECISION, GENERATION, LAST_CHANGE, LEGACY,
    LEGACY_SUMMARY, NUDGE_COMMAND, OUTWARD_POSTURE_COMMAND, POSTURE, RELATIONSHIP,
    RELATIONSHIP_DIRECTION,''',
    "projection legacy imports",
)
projection = replace_once(
    projection,
    '''    if let Some(item) = posture_consequence_item(world) {
        items.push(item);
    }
    items''',
    '''    if let Some(item) = posture_consequence_item(world) {
        items.push(item);
    }
    if let Some(item) = legacy_consequence_item(world) {
        items.push(item);
    }
    items''',
    "legacy persistent consequence",
)
projection = replace_once(
    projection,
    '''fn relationship_consequence_item(world: &World) -> Option<BriefingItem> {''',
    '''fn legacy_consequence_item(world: &World) -> Option<BriefingItem> {
    let legacy = text_component(world.state().entity(UNIVERSE), LEGACY, "forming");
    if legacy == "forming" {
        return None;
    }
    let summary = text_component(
        world.state().entity(UNIVERSE),
        LEGACY_SUMMARY,
        "This World now carries a durable legacy from its earlier choices.",
    );
    Some(BriefingItem {
        selection: Some(SelectionId::Entity(UNIVERSE)),
        title: format!("World legacy · {}", legacy_label(&legacy)),
        detail: summary,
    })
}

fn legacy_label(legacy: &str) -> String {
    legacy
        .split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn relationship_consequence_item(world: &World) -> Option<BriefingItem> {''',
    "legacy briefing item",
)
projection = replace_once(
    projection,
    '''            "relationship_fractured" => "Their relationship fractured".into(),
            _ => event.kind.replace('_', " "),''',
    '''            "relationship_fractured" => "Their relationship fractured".into(),
            "world_legacy_formed" => "A world legacy formed".into(),
            _ => event.kind.replace('_', " "),''',
    "legacy return label",
)
projection_path.write_text(projection)
