use crate::model::*;
use std::collections::BTreeMap;
use world_core::{Entity, Event, Relation, WorldState, WorldStateError};

pub(crate) fn baseline() -> Result<WorldState, WorldStateError> {
    let mut state = WorldState::default();

    for entity in [
        Entity::new(TERMINAL, "recovered_device")
            .with_component("name", "Terminal 17")
            .with_component("serial", "AST-T17-8841")
            .with_component("status", "offline"),
        Entity::new(MIRA, "person")
            .with_component("name", "Mira Voss")
            .with_component("role", "materials researcher"),
        Entity::new(ELIAS, "person")
            .with_component("name", "Elias Reed")
            .with_component("role", "systems auditor"),
        Entity::new(PLATFORM_12, "place")
            .with_component("name", "Platform 12")
            .with_component("district", "North Transit Ring"),
        Entity::new(ASTERION, "organization")
            .with_component("name", "Asterion Labs")
            .with_component("sector", "advanced materials"),
        artifact(
            CALENDAR_FRAGMENT,
            "Calendar fragment",
            "calendar",
            "local calendar",
            true,
            MEETING_SCHEDULED.0,
            "23:10 — Platform 12 / E. Reed",
            "2047-11-03 23:10",
        ),
        artifact(
            TAXI_RECEIPT,
            "Taxi receipt",
            "receipt",
            "mobility cache",
            true,
            TAXI_RIDE_RECORDED.0,
            "Drop-off: North Transit Ring, Gate C",
            "2047-11-03 23:36",
        ),
        artifact(
            PLATFORM_PHOTO,
            "Platform photo",
            "photo",
            "camera roll",
            true,
            PHOTO_CAPTURED.0,
            "Two figures beside Platform 12; one carries a silver case",
            "2047-11-03 23:42",
        ),
        artifact(
            WIFI_LOG,
            "Wi-Fi association log",
            "network_log",
            "system diagnostics",
            true,
            PLATFORM_ACCESSED.0,
            "Terminal 17 associated with platform-guest at Platform 12",
            "2047-11-03 23:44",
        ),
        artifact(
            PROJECT_COPY_LOG,
            "Project copy log",
            "file_log",
            "filesystem journal",
            true,
            PROTOTYPE_COPIED.0,
            "prototype_bundle.delta copied to removable volume",
            "2047-11-03 23:47",
        ),
        artifact(
            DELETED_MESSAGE,
            "Deleted message fragment",
            "message",
            "unallocated message store",
            false,
            MESSAGE_DELETED.0,
            "Mira: Do not bring the prototype back to Asterion.",
            "2047-11-03 23:53",
        ),
    ] {
        state.seed_entity(entity)?;
    }

    state.seed_relation(Relation::new(
        ASTERION_EMPLOYS_MIRA,
        "employs",
        ASTERION,
        MIRA,
    ))?;
    state.seed_relation(Relation::new(
        ASTERION_EMPLOYS_ELIAS,
        "employs",
        ASTERION,
        ELIAS,
    ))?;
    state.seed_relation(Relation::new(
        TERMINAL_OWNED_BY_ELIAS,
        "owned_by",
        TERMINAL,
        ELIAS,
    ))?;

    Ok(state)
}

pub(crate) fn truth_events() -> Vec<Event> {
    vec![
        event(
            MEETING_SCHEDULED,
            "meeting_scheduled",
            10,
            Some(MIRA),
            vec![ELIAS, PLATFORM_12],
            vec![],
        ),
        event(
            TAXI_RIDE_RECORDED,
            "taxi_ride_recorded",
            20,
            Some(MIRA),
            vec![PLATFORM_12],
            vec![MEETING_SCHEDULED],
        ),
        event(
            PHOTO_CAPTURED,
            "photo_captured",
            30,
            Some(MIRA),
            vec![ELIAS, PLATFORM_12],
            vec![TAXI_RIDE_RECORDED],
        ),
        event(
            PLATFORM_ACCESSED,
            "platform_accessed",
            35,
            Some(ELIAS),
            vec![TERMINAL, PLATFORM_12],
            vec![PHOTO_CAPTURED],
        ),
        event(
            PROTOTYPE_COPIED,
            "prototype_copied",
            40,
            Some(ELIAS),
            vec![TERMINAL],
            vec![PLATFORM_ACCESSED],
        ),
        event(
            WARNING_MESSAGE_SENT,
            "warning_message_sent",
            45,
            Some(MIRA),
            vec![ELIAS],
            vec![PROTOTYPE_COPIED],
        ),
        event(
            MESSAGE_DELETED,
            "message_deleted",
            50,
            Some(ELIAS),
            vec![DELETED_MESSAGE],
            vec![WARNING_MESSAGE_SENT],
        ),
    ]
}

fn artifact(
    id: world_core::EntityId,
    name: &str,
    kind: &str,
    source: &str,
    visible: bool,
    event_ref: u64,
    summary: &str,
    timestamp: &str,
) -> Entity {
    Entity::new(id, "artifact")
        .with_component("name", name)
        .with_component(ARTIFACT_KIND, kind)
        .with_component(SOURCE, source)
        .with_component(VISIBLE, visible)
        .with_component(EVENT_REF, event_ref as i64)
        .with_component(SUMMARY, summary)
        .with_component(TIMESTAMP, timestamp)
}

fn event(
    id: world_core::EventId,
    kind: &str,
    world_time: u64,
    actor: Option<world_core::EntityId>,
    targets: Vec<world_core::EntityId>,
    caused_by: Vec<world_core::EventId>,
) -> Event {
    Event {
        id,
        kind: kind.into(),
        world_time,
        actor,
        targets,
        caused_by,
        payload: BTreeMap::new(),
        changes: vec![],
    }
}
