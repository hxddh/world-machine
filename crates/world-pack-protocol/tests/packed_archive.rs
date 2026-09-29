//! From v5 the archive `open` hands over and `archive` hands back is
//! packed: its compact encoding, deflated, in base64. It reads back as the
//! exact archive, checkpoint and pending actions included, and a Pack on an
//! older protocol is still handed the tagged archive it reads.

use std::collections::BTreeMap;
use world_pack_protocol::{
    decode_request, decode_response, encode_request, encode_response, pack_archive,
    pack_frame_limit, unpack_archive, PackRequest, PackRequestEnvelope, PackResponse,
    PackResponseEnvelope, PACKED_ARCHIVE_PREFIX, PACK_FRAME_LIMIT, PACK_FRAME_LIMIT_BEFORE_V5,
    PACK_PROTOCOL_VERSION_V3, PACK_PROTOCOL_VERSION_V4, PACK_PROTOCOL_VERSION_V5,
};
use world_persistence::{
    ArchivedActionRequest, ArchivedCheckpoint, ArchivedEntity, ArchivedEvent,
    ArchivedScheduledAction, ArchivedStateChange, ArchivedValue, WorldArchive, WorldPackRef,
    WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION,
};

/// An archive with every kind of change and value, `days` days long.
fn archive(days: u64) -> WorldArchive {
    let mut events = Vec::new();
    let mut id = 0;
    for day in 1..=days {
        id += 1;
        let person = 1 + day % 7;
        let mut payload = BTreeMap::new();
        payload.insert("day".into(), ArchivedValue::Integer(day as i64));
        payload.insert("who".into(), ArchivedValue::Entity(person));
        payload.insert(
            "said".into(),
            ArchivedValue::Text(format!(
                "Morning, day {day} — the pier is {}% done",
                day % 100
            )),
        );
        let mut components = BTreeMap::new();
        components.insert("name".into(), ArchivedValue::Text(format!("Crate {day}")));
        components.insert(
            "#".into(),
            ArchivedValue::Map(BTreeMap::from([("{}".into(), ArchivedValue::Null)])),
        );
        events.push(ArchivedEvent {
            id,
            kind: "fixture.day".into(),
            world_time: day * 24,
            actor: Some(person),
            targets: vec![person, 100 + day],
            caused_by: if id > 1 { vec![id - 1] } else { Vec::new() },
            payload,
            changes: vec![
                ArchivedStateChange::CreateEntity {
                    entity: ArchivedEntity {
                        id: 100 + day,
                        kind: "crate".into(),
                        components,
                    },
                },
                ArchivedStateChange::SetComponent {
                    entity: person,
                    key: "mood".into(),
                    value: ArchivedValue::List(vec![
                        ArchivedValue::Bool(day % 2 == 0),
                        ArchivedValue::Integer(-(day as i64)),
                    ]),
                },
                ArchivedStateChange::RemoveComponent {
                    entity: person,
                    key: "errand".into(),
                },
            ],
        });
    }
    WorldArchive {
        format: WORLD_ARCHIVE_FORMAT.into(),
        format_version: WORLD_ARCHIVE_VERSION,
        pack: WorldPackRef::new("fixture.packed", "1.0.0"),
        world_time: days * 24 + 5,
        events,
        pending: vec![ArchivedScheduledAction {
            world_time: days * 24 + 24,
            request: ArchivedActionRequest {
                actor: Some(3),
                action: "fixture.visit".into(),
                args: BTreeMap::from([("to".into(), ArchivedValue::Entity(4))]),
                caused_by: vec![id],
            },
        }],
        checkpoint: Some(ArchivedCheckpoint {
            events: 2,
            last_event: 2,
            world_time: 48,
            changes: vec![ArchivedStateChange::SetComponent {
                entity: 2,
                key: "mood".into(),
                value: ArchivedValue::Text("glad".into()),
            }],
        }),
    }
}

#[test]
fn a_packed_archive_reads_back_exactly() {
    let archive = archive(400);
    let packed = pack_archive(&archive).unwrap();
    assert!(packed.starts_with(PACKED_ARCHIVE_PREFIX));
    assert_eq!(unpack_archive(&packed).unwrap(), archive);

    let tagged = serde_json::to_string(&archive).unwrap();
    assert!(
        packed.len() * 10 < tagged.len(),
        "packed {} bytes against tagged {}",
        packed.len(),
        tagged.len()
    );
}

#[test]
fn v5_open_and_archive_carry_the_archive_packed() {
    let archive = archive(60);
    let open = PackRequestEnvelope::for_version(
        PACK_PROTOCOL_VERSION_V5,
        3,
        PackRequest::Open {
            archive: archive.clone(),
        },
    )
    .unwrap();
    let line = encode_request(&open).unwrap();
    assert!(line.contains(&format!("\"archive\":\"{PACKED_ARCHIVE_PREFIX}")));
    assert!(!line.contains("fixture.day"), "the events travel deflated");
    assert_eq!(decode_request(&line).unwrap(), open);

    for kept in [Some(archive.clone()), None] {
        let response = PackResponseEnvelope::for_version(
            PACK_PROTOCOL_VERSION_V5,
            4,
            PackResponse::Archive { archive: kept },
        )
        .unwrap();
        let line = encode_response(&response).unwrap();
        assert_eq!(decode_response(&line).unwrap(), response);
    }
}

#[test]
fn a_pack_before_v5_is_handed_the_tagged_archive() {
    let archive = archive(30);
    for version in [PACK_PROTOCOL_VERSION_V3, PACK_PROTOCOL_VERSION_V4] {
        let mut archive = archive.clone();
        if version < PACK_PROTOCOL_VERSION_V4 {
            archive.checkpoint = None;
        }
        let open = PackRequestEnvelope::for_version(
            version,
            1,
            PackRequest::Open {
                archive: archive.clone(),
            },
        )
        .unwrap();
        let line = encode_request(&open).unwrap();
        assert!(line.contains("\"archive\":{\"format\":\"world-machine\""));
        assert!(!line.contains(PACKED_ARCHIVE_PREFIX));
        assert_eq!(decode_request(&line).unwrap(), open);

        let response = PackResponseEnvelope::for_version(
            version,
            2,
            PackResponse::Archive {
                archive: Some(archive),
            },
        )
        .unwrap();
        let line = encode_response(&response).unwrap();
        assert!(!line.contains(PACKED_ARCHIVE_PREFIX));
        assert_eq!(decode_response(&line).unwrap(), response);
    }
    assert_eq!(
        pack_frame_limit(PACK_PROTOCOL_VERSION_V4),
        PACK_FRAME_LIMIT_BEFORE_V5
    );
    assert_eq!(pack_frame_limit(PACK_PROTOCOL_VERSION_V5), PACK_FRAME_LIMIT);
}

#[test]
fn a_damaged_packed_archive_is_refused() {
    let packed = pack_archive(&archive(10)).unwrap();
    assert!(unpack_archive(&packed[PACKED_ARCHIVE_PREFIX.len()..]).is_err());
    assert!(unpack_archive(&format!("{PACKED_ARCHIVE_PREFIX}not base64!")).is_err());
    let truncated = &packed[..packed.len() / 2];
    assert!(unpack_archive(truncated).is_err());

    let line = format!(
        "{{\"protocol_version\":5,\"request_id\":1,\"request\":{{\"type\":\"open\",\"archive\":\"{}\"}}}}",
        truncated
    );
    assert!(decode_request(&line).is_err());
}
