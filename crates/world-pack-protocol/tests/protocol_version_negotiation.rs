use world_pack_protocol::{
    decode_request, decode_response, encode_request, encode_response, PackDescriptor, PackManifest,
    PackRequest, PackRequestEnvelope, PackResponse, PackResponseEnvelope, PACK_PROTOCOL_VERSION,
    PACK_PROTOCOL_VERSION_V1, PACK_PROTOCOL_VERSION_V2, PACK_PROTOCOL_VERSION_V3,
};
use world_persistence::WorldPackRef;

#[test]
fn latest_protocol_is_v3_while_v1_and_v2_remain_supported() {
    assert_eq!(PACK_PROTOCOL_VERSION, PACK_PROTOCOL_VERSION_V3);
    assert_eq!(PACK_PROTOCOL_VERSION_V3, 3);
    assert_eq!(PACK_PROTOCOL_VERSION_V1, 1);
    assert_eq!(PACK_PROTOCOL_VERSION_V2, 2);

    let descriptor = PackDescriptor::new(
        WorldPackRef::new("fixture.negotiation", "1"),
        "Negotiation Fixture",
        "fixture",
    );
    let latest = PackManifest::process(descriptor, "runtime", Vec::new());
    assert_eq!(latest.protocol_version, PACK_PROTOCOL_VERSION_V3);
    assert!(latest.validate().is_ok());

    let mut v1 = latest.clone();
    v1.protocol_version = PACK_PROTOCOL_VERSION_V1;
    assert!(v1.validate().is_ok());

    let mut v2 = latest.clone();
    v2.protocol_version = PACK_PROTOCOL_VERSION_V2;
    assert!(v2.validate().is_ok());

    let mut unsupported = latest;
    unsupported.protocol_version = PACK_PROTOCOL_VERSION_V3 + 1;
    assert!(unsupported.validate().is_err());
}

#[test]
fn request_and_response_envelopes_accept_known_versions_but_reject_unknown_ones() {
    for version in [
        PACK_PROTOCOL_VERSION_V1,
        PACK_PROTOCOL_VERSION_V2,
        PACK_PROTOCOL_VERSION_V3,
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

    let unsupported = PACK_PROTOCOL_VERSION_V3 + 1;
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
