use crate::model::*;
use std::collections::BTreeMap;
use world_core::{Entity, Event, Relation, WorldState, WorldStateError};

struct ArtifactSeed<'a> {
    id: world_core::EntityId,
    name: &'a str,
    kind: &'a str,
    source: &'a str,
    visible: bool,
    event_ref: u64,
    summary: &'a str,
    timestamp: &'a str,
}

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
        artifact(ArtifactSeed {
            id: CALENDAR_FRAGMENT,
            name: "Calendar fragment",
            kind: "calendar",
            source: "local calendar",
            visible: true,
            event_ref: MEETING_SCHEDULED.0,
            summary: "23:10 — Platform 12 / E. Reed",
            timestamp: "2047-11-03 23:10",
        }),
        artifact(ArtifactSeed {
            id: TAXI_RECEIPT,
            name: "Taxi receipt",
            kind: "receipt",
            source: "mobility cache",
            visible: true,
            event_ref: TAXI_RIDE_RECORDED.0,
            summary: "Drop-off: North Transit Ring, Gate C",
            timestamp: "2047-11-03 23:36",
        }),
        artifact(ArtifactSeed {
            id: PLATFORM_PHOTO,
            name: "Platform photo",
            kind: "photo",
            source: "camera roll",
            visible: true,
            event_ref: PHOTO_CAPTURED.0,
            summary: "Two figures beside Platform 12; one carries a silver case",
            timestamp: "2047-11-03 23:42",
        }),
        artifact(ArtifactSeed {
            id: WIFI_LOG,
            name: "Wi-Fi association log",
            kind: "network_log",
            source: "system diagnostics",
            visible: true,
            event_ref: PLATFORM_ACCESSED.0,
            summary: "Terminal 17 associated with platform-guest at Platform 12",
            timestamp: "2047-11-03 23:44",
        }),
        artifact(ArtifactSeed {
            id: PROJECT_COPY_LOG,
            name: "Project copy log",
            kind: "file_log",
            source: "filesystem journal",
            visible: true,
            event_ref: PROTOTYPE_COPIED.0,
            summary: "prototype_bundle.delta copied to removable volume",
            timestamp: "2047-11-03 23:47",
        }),
        artifact(ArtifactSeed {
            id: DELETED_MESSAGE,
            name: "Deleted message fragment",
            kind: "message",
            source: "unallocated message store",
            visible: false,
            event_ref: MESSAGE_DELETED.0,
            summary: "Mira: Do not bring the prototype back to Asterion.",
            timestamp: "2047-11-03 23:53",
        }),
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

fn artifact(seed: ArtifactSeed<'_>) -> Entity {
    Entity::new(seed.id, "artifact")
        .with_component("name", seed.name)
        .with_component(ARTIFACT_KIND, seed.kind)
        .with_component(SOURCE, seed.source)
        .with_component(VISIBLE, seed.visible)
        .with_component(EVENT_REF, seed.event_ref as i64)
        .with_component(SUMMARY, seed.summary)
        .with_component(TIMESTAMP, seed.timestamp)
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
