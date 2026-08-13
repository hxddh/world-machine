use crate::{
    CASH, COMPANY, CUSTOMERS, CYCLE, GROWTH_LEAD, LAST_CHANGE, MARKET, PRODUCT, PRODUCT_LEAD,
    QUALITY, RELATIONSHIP, RUN_CYCLE_COMMAND, STATUS, TENSION, TRUST,
};
use world_core::{Entity, Event, Value, World};
use world_projection::{
    entity_title, inspectors_from_world, timeline_from_world, why_map_from_world, BriefingItem,
    BriefingProjection, CanvasItem, CanvasItemKind, CanvasProjection, CollectionItem,
    CollectionProjection, ProjectionCapabilities, ProjectionCommand, ProjectionSnapshot,
    SelectionId,
};

pub(crate) fn snapshot(world: &World, since_event_count: Option<usize>) -> ProjectionSnapshot {
    ProjectionSnapshot {
        title: title(world),
        world_time: world.world_time(),
        capabilities: ProjectionCapabilities {
            fork: !world.events().is_empty(),
        },
        briefing: Some(briefing(world, since_event_count)),
        commands: commands(world),
        collection: collection(world),
        timeline: timeline_from_world(world),
        canvas: canvas(world),
        inspectors: inspectors_from_world(world),
        why: why_map_from_world(world),
    }
}

fn title(world: &World) -> String {
    match text_component(world.state().entity(COMPANY), STATUS, "searching").as_str() {
        "traction" => "Northstar Micro Company · Traction".into(),
        "out-of-cash" => "Northstar Micro Company · Out of cash".into(),
        _ => "Northstar Micro Company".into(),
    }
}

fn commands(world: &World) -> Vec<ProjectionCommand> {
    if text_component(world.state().entity(COMPANY), STATUS, "searching") != "searching" {
        return Vec::new();
    }
    vec![ProjectionCommand {
        id: RUN_CYCLE_COMMAND.into(),
        title: "Run one company cycle".into(),
        detail: "Pay the burn, let Maya and Jon act, then see whether product, customers, runway, and working trust move together.".into(),
    }]
}

fn briefing(world: &World, since_event_count: Option<usize>) -> BriefingProjection {
    if let Some(since) = since_event_count.filter(|since| *since < world.events().len()) {
        return BriefingProjection {
            eyebrow: "Micro Company".into(),
            title: "While the company was running".into(),
            items: world.events()[since..]
                .iter()
                .rev()
                .filter(|event| event.kind != "agent_decision_recorded")
                .take(4)
                .map(return_item)
                .collect(),
        };
    }

    let company = world.state().entity(COMPANY);
    let cycle = integer_component(company, CYCLE, 0);
    let cash = integer_component(company, CASH, 0);
    let quality = integer_component(world.state().entity(PRODUCT), QUALITY, 0);
    let customers = integer_component(world.state().entity(MARKET), CUSTOMERS, 0);
    let trust = integer_component(world.state().entity(RELATIONSHIP), TRUST, 0);
    let tension = integer_component(world.state().entity(RELATIONSHIP), TENSION, 0);
    let status = text_component(company, STATUS, "searching");
    let last_change = text_component(company, LAST_CHANGE, "The company is quiet.");
    BriefingProjection {
        eyebrow: "Micro Company".into(),
        title: match status.as_str() {
            "traction" => "Traction found".into(),
            "out-of-cash" => "Runway exhausted".into(),
            _ => format!("Cycle {cycle} · Searching for traction"),
        },
        items: vec![
            BriefingItem {
                selection: Some(SelectionId::Entity(COMPANY)),
                title: format!("Cash {cash} · Quality {quality} · Customers {customers}"),
                detail: last_change,
            },
            BriefingItem {
                selection: Some(SelectionId::Entity(RELATIONSHIP)),
                title: format!("Working trust {trust} · Tension {tension}"),
                detail: "The leads' choices are ordinary World events, so their working pattern is inspectable, causal, and forkable.".into(),
            },
        ],
    }
}

fn return_item(event: &Event) -> BriefingItem {
    let detail = ["summary", "change"]
        .into_iter()
        .find_map(|key| match event.payload.get(key) {
            Some(Value::Text(value)) => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_else(|| event.kind.replace('_', " "));
    BriefingItem {
        selection: Some(SelectionId::Event(event.id)),
        title: match event.kind.as_str() {
            "market_cycle_started" => "The clock kept running".into(),
            "agent_built_product" => "Someone built the product".into(),
            "agent_sold_product" => "Someone sold the product".into(),
            "working_relationship_shifted" => "Their working relationship changed".into(),
            "company_found_traction" => "The company found traction".into(),
            "company_ran_out_of_cash" => "The company ran out of cash".into(),
            _ => event.kind.replace('_', " "),
        },
        detail,
    }
}

fn collection(world: &World) -> CollectionProjection {
    CollectionProjection {
        title: "Company World".into(),
        items: world
            .state()
            .entities()
            .filter(|entity| entity.id != COMPANY)
            .map(|entity| CollectionItem {
                id: SelectionId::Entity(entity.id),
                title: entity_title(entity),
                subtitle: entity.kind.replace('_', " "),
            })
            .collect(),
    }
}

fn canvas(world: &World) -> CanvasProjection {
    let positions = [
        (PRODUCT_LEAD, 0.18, 0.28),
        (GROWTH_LEAD, 0.78, 0.28),
        (PRODUCT, 0.20, 0.74),
        (MARKET, 0.78, 0.74),
        (RELATIONSHIP, 0.49, 0.50),
    ];
    CanvasProjection {
        items: positions
            .into_iter()
            .filter_map(|(id, x, y)| {
                world.state().entity(id).map(|entity| CanvasItem {
                    id: SelectionId::Entity(id),
                    kind: canvas_kind(entity),
                    label: entity_title(entity),
                    detail: entity.kind.replace('_', " "),
                    x,
                    y,
                })
            })
            .collect(),
    }
}

fn canvas_kind(entity: &Entity) -> CanvasItemKind {
    match entity.kind.as_str() {
        "person" => CanvasItemKind::Actor,
        "market" => CanvasItemKind::Place,
        _ => CanvasItemKind::Object,
    }
}

fn text_component(entity: Option<&Entity>, key: &str, fallback: &str) -> String {
    match entity.and_then(|entity| entity.component(key)) {
        Some(Value::Text(value)) => value.clone(),
        _ => fallback.into(),
    }
}

fn integer_component(entity: Option<&Entity>, key: &str, fallback: i64) -> i64 {
    match entity.and_then(|entity| entity.component(key)) {
        Some(Value::Integer(value)) => *value,
        _ => fallback,
    }
}
