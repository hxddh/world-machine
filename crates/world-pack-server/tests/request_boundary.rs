use std::io::Cursor;
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};
use world_pack_protocol::{
    decode_response, encode_request, PackRequest, PackRequestEnvelope, PackResponse,
};
use world_pack_server::{serve_jsonl, PackServerError, DEFAULT_MAX_REQUEST_BYTES};
use world_persistence::{WorldArchive, WorldPackRef};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

const PACK_ID: &str = "fixture.pack.request-boundary";
const PACK_VERSION: &str = "one";

struct FixtureSession;

impl WorldSession for FixtureSession {
    fn pack(&self) -> WorldPackRef {
        WorldPackRef::new(PACK_ID, PACK_VERSION)
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        ProjectionSnapshot::default()
    }

    fn handle(&mut self, _intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
        Ok(self.snapshot())
    }

    fn advance_background(&mut self, _periods: u64) -> Result<ProjectionSnapshot, HostError> {
        Ok(self.snapshot())
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        Ok(None)
    }
}

fn registration() -> WorldRegistration {
    WorldRegistration::new(
        WorldDescriptor {
            pack: WorldPackRef::new(PACK_ID, PACK_VERSION),
            title: "Pack Request Boundary Fixture".into(),
            description: "Locks the v1 physical JSONL request ceiling".into(),
        },
        || Ok(Box::new(FixtureSession)),
    )
}

fn describe_frame(total_wire_bytes: usize) -> Vec<u8> {
    let encoded = encode_request(&PackRequestEnvelope::new(1, PackRequest::Describe)).unwrap();
    assert!(
        encoded.len() + 1 <= total_wire_bytes,
        "requested boundary must fit the Describe envelope and framing LF"
    );

    let mut frame = Vec::with_capacity(total_wire_bytes);
    frame.extend_from_slice(encoded.as_bytes());
    frame.resize(total_wire_bytes - 1, b' ');
    frame.push(b'\n');
    assert_eq!(frame.len(), total_wire_bytes);
    frame
}

#[test]
fn exact_physical_request_ceiling_including_lf_is_accepted() {
    let input = describe_frame(DEFAULT_MAX_REQUEST_BYTES);
    let mut output = Vec::new();

    serve_jsonl(registration(), Cursor::new(input), &mut output).unwrap();

    let responses = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| decode_response(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].request_id, 1);
    assert!(matches!(responses[0].response, PackResponse::Descriptor { .. }));
}

#[test]
fn physical_request_ceiling_plus_one_is_fatal_before_dispatch() {
    let input = describe_frame(DEFAULT_MAX_REQUEST_BYTES + 1);
    let mut output = Vec::new();

    let error = serve_jsonl(registration(), Cursor::new(input), &mut output).unwrap_err();

    assert!(matches!(error, PackServerError::Protocol(message) if message.contains("Pack request exceeds")));
    assert!(
        output.is_empty(),
        "an oversized request must fail before a correlated Pack response is fabricated"
    );
}
