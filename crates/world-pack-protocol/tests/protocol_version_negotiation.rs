use world_pack_protocol::{
    decode_request, decode_response, encode_request, encode_response, PackDescriptor, PackManifest,
    PackRequest, PackRequestEnvelope, PackResponse, PackResponseEnvelope, PACK_PROTOCOL_VERSION,
    PACK_PROTOCOL_VERSION_V1, PACK_PROTOCOL_VERSION_V2, PACK_PROTOCOL_VERSION_V3,
    PACK_PROTOCOL_VERSION_V4, PACK_PROTOCOL_VERSION_V5, PACK_PROTOCOL_VERSION_V6,
    PACK_PROTOCOL_VERSION_V7, PACK_PROTOCOL_VERSION_V8,
};
use world_persistence::WorldPackRef;

#[test]
fn latest_protocol_is_v8_while_v1_to_v7_remain_supported() {
    assert_eq!(PACK_PROTOCOL_VERSION, PACK_PROTOCOL_VERSION_V8);
    assert_eq!(PACK_PROTOCOL_VERSION_V8, 8);
    assert_eq!(PACK_PROTOCOL_VERSION_V7, 7);
    assert_eq!(PACK_PROTOCOL_VERSION_V6, 6);
    assert_eq!(PACK_PROTOCOL_VERSION_V5, 5);
    assert_eq!(PACK_PROTOCOL_VERSION_V4, 4);
    assert_eq!(PACK_PROTOCOL_VERSION_V3, 3);
    assert_eq!(PACK_PROTOCOL_VERSION_V1, 1);
    assert_eq!(PACK_PROTOCOL_VERSION_V2, 2);

    let descriptor = PackDescriptor::new(
        WorldPackRef::new("fixture.negotiation", "1"),
        "Negotiation Fixture",
        "fixture",
    );
    let latest = PackManifest::process(descriptor, "runtime", Vec::new());
    assert_eq!(latest.protocol_version, PACK_PROTOCOL_VERSION_V8);
    assert!(latest.validate().is_ok());

    let mut v1 = latest.clone();
    v1.protocol_version = PACK_PROTOCOL_VERSION_V1;
    assert!(v1.validate().is_ok());

    let mut v2 = latest.clone();
    v2.protocol_version = PACK_PROTOCOL_VERSION_V2;
    assert!(v2.validate().is_ok());

    let mut v3 = latest.clone();
    v3.protocol_version = PACK_PROTOCOL_VERSION_V3;
    assert!(v3.validate().is_ok());

    let mut v4 = latest.clone();
    v4.protocol_version = PACK_PROTOCOL_VERSION_V4;
    assert!(v4.validate().is_ok());

    let mut v5 = latest.clone();
    v5.protocol_version = PACK_PROTOCOL_VERSION_V5;
    assert!(v5.validate().is_ok());

    let mut v6 = latest.clone();
    v6.protocol_version = PACK_PROTOCOL_VERSION_V6;
    assert!(v6.validate().is_ok());

    let mut v7 = latest.clone();
    v7.protocol_version = PACK_PROTOCOL_VERSION_V7;
    assert!(v7.validate().is_ok());

    let mut unsupported = latest;
    unsupported.protocol_version = PACK_PROTOCOL_VERSION_V8 + 1;
    assert!(unsupported.validate().is_err());
}

#[test]
fn request_and_response_envelopes_accept_known_versions_but_reject_unknown_ones() {
    for version in [
        PACK_PROTOCOL_VERSION_V1,
        PACK_PROTOCOL_VERSION_V2,
        PACK_PROTOCOL_VERSION_V3,
        PACK_PROTOCOL_VERSION_V4,
        PACK_PROTOCOL_VERSION_V5,
        PACK_PROTOCOL_VERSION_V6,
        PACK_PROTOCOL_VERSION_V7,
        PACK_PROTOCOL_VERSION_V8,
    ] {
        let request = PackRequestEnvelope::for_version(version, 7, PackRequest::Describe)
            .expect("supported request version");
        let decoded_request = decode_request(&encode_request(&request).unwrap()).unwrap();
        assert_eq!(decoded_request.protocol_version, version);
        assert_eq!(decoded_request.request_id, 7);
        assert_eq!(decoded_request.request, PackRequest::Describe);

        let response = PackResponseEnvelope::for_version(version, 7, PackResponse::Ok)
            .expect("supported response version");
        let decoded_response = decode_response(&encode_response(&response).unwrap()).unwrap();
        assert_eq!(decoded_response.protocol_version, version);
        assert_eq!(decoded_response.request_id, 7);
        assert_eq!(decoded_response.response, PackResponse::Ok);
    }

    let unsupported = PACK_PROTOCOL_VERSION_V8 + 1;
    assert!(PackRequestEnvelope::for_version(unsupported, 1, PackRequest::Describe).is_err());
    assert!(PackResponseEnvelope::for_version(unsupported, 1, PackResponse::Ok).is_err());
}

/// A Pack on an older protocol is never asked what to ask a model, nor
/// handed a model's response: it could not read either.
#[test]
fn hearing_through_the_app_needs_v3() {
    use world_pack_protocol::{EarsWire, ProjectionIntentWire, SelectionIdWire};
    let hear = PackRequest::Hear {
        to: SelectionIdWire::Entity { id: 1 },
        words: "hi".into(),
    };
    let said = |ears| PackRequest::Handle {
        intent: ProjectionIntentWire::Say {
            to: SelectionIdWire::Entity { id: 1 },
            words: "hi".into(),
            ears,
        },
    };
    for version in [PACK_PROTOCOL_VERSION_V1, PACK_PROTOCOL_VERSION_V2] {
        assert!(PackRequestEnvelope::for_version(version, 1, hear.clone()).is_err());
        assert!(PackRequestEnvelope::for_version(version, 1, said(EarsWire::Own)).is_err());
        assert!(PackRequestEnvelope::for_version(version, 1, said(EarsWire::World)).is_ok());
    }
    assert!(PackRequestEnvelope::for_version(PACK_PROTOCOL_VERSION_V3, 1, hear).is_ok());
    assert!(
        PackRequestEnvelope::for_version(PACK_PROTOCOL_VERSION_V3, 1, said(EarsWire::Own)).is_ok()
    );
}

/// A Pack before v4 would pass over an archive's checkpoint and replay the
/// wrong history, so it is never handed one.
#[test]
fn opening_from_a_checkpoint_needs_v4() {
    use world_persistence::{
        ArchivedCheckpoint, WorldArchive, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION,
    };
    let mut archive = WorldArchive {
        format: WORLD_ARCHIVE_FORMAT.into(),
        format_version: WORLD_ARCHIVE_VERSION,
        pack: WorldPackRef::new("fixture.negotiation", "1"),
        world_time: 9,
        events: Vec::new(),
        pending: Vec::new(),
        checkpoint: None,
    };
    for version in [
        PACK_PROTOCOL_VERSION_V1,
        PACK_PROTOCOL_VERSION_V2,
        PACK_PROTOCOL_VERSION_V3,
        PACK_PROTOCOL_VERSION_V4,
    ] {
        let open = PackRequest::Open {
            archive: archive.clone(),
        };
        assert!(PackRequestEnvelope::for_version(version, 1, open).is_ok());
    }
    archive.checkpoint = Some(ArchivedCheckpoint {
        events: 3,
        last_event: 3,
        world_time: 8,
        changes: Vec::new(),
    });
    let open = PackRequest::Open { archive };
    for version in [
        PACK_PROTOCOL_VERSION_V1,
        PACK_PROTOCOL_VERSION_V2,
        PACK_PROTOCOL_VERSION_V3,
    ] {
        assert!(PackRequestEnvelope::for_version(version, 1, open.clone()).is_err());
    }
    let envelope = PackRequestEnvelope::for_version(PACK_PROTOCOL_VERSION_V4, 1, open).unwrap();
    let decoded = decode_request(&encode_request(&envelope).unwrap()).unwrap();
    assert_eq!(decoded, envelope, "the checkpoint travels with the archive");
}

/// Marking a World and going back, and asking for only the events a host
/// has not saved, are v6's: a Pack before it is never asked.
#[test]
fn checkpoints_need_v6() {
    let requests = [
        PackRequest::Checkpoint,
        PackRequest::Rollback,
        PackRequest::ArchiveSince { events: 12 },
    ];
    for request in requests {
        for version in 1..PACK_PROTOCOL_VERSION_V6 {
            assert!(PackRequestEnvelope::for_version(version, 1, request.clone()).is_err());
        }
        let envelope =
            PackRequestEnvelope::for_version(PACK_PROTOCOL_VERSION_V6, 3, request.clone()).unwrap();
        assert_eq!(
            decode_request(&encode_request(&envelope).unwrap())
                .unwrap()
                .request,
            request
        );
    }
    let kept = PackResponseEnvelope::for_version(
        PACK_PROTOCOL_VERSION_V6,
        3,
        PackResponse::Checkpointed { kept: true },
    )
    .unwrap();
    assert_eq!(
        decode_response(&encode_response(&kept).unwrap())
            .unwrap()
            .response,
        PackResponse::Checkpointed { kept: true }
    );
}
