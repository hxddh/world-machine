mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use world_host::WorldRegistry;
use world_pack_process::{ProcessPack, ProcessPackSource};
use world_pack_protocol::{
    encode_response, PackDescriptor, PackManifest, PackResponse, PackResponseEnvelope,
    ProjectionSnapshotWire, PACK_PROTOCOL_VERSION_V1, PACK_PROTOCOL_VERSION_V2,
};
use world_persistence::WorldPackRef;

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "world-pack-process-protocol-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn descriptor() -> PackDescriptor {
    PackDescriptor::new(
        WorldPackRef::new("fixture.protocol-v1", "1"),
        "Protocol v1 Fixture",
        "A v1 process Pack fixture",
    )
}

fn response_line(version: u32, request_id: u64, response: PackResponse) -> String {
    let envelope = PackResponseEnvelope::for_version(version, request_id, response).unwrap();
    encode_response(&envelope).unwrap()
}

fn v1_fixture(root: &Path, responses: &[String]) -> ProcessPack {
    let mut manifest = PackManifest::process(descriptor(), "runtime", Vec::new());
    manifest.protocol_version = PACK_PROTOCOL_VERSION_V1;
    support::fixture_pack(root, manifest, &support::respond(responses))
}

#[test]
fn host_runs_a_manifest_declared_v1_pack_using_v1_envelopes() {
    let root = temp_dir("v1-coexists");
    let pack = v1_fixture(
        &root,
        &[
            response_line(
                PACK_PROTOCOL_VERSION_V1,
                1,
                PackResponse::Descriptor {
                    descriptor: descriptor(),
                },
            ),
            response_line(
                PACK_PROTOCOL_VERSION_V1,
                2,
                PackResponse::Snapshot {
                    snapshot: ProjectionSnapshotWire {
                        title: "Created over v1".into(),
                        ..ProjectionSnapshotWire::default()
                    },
                },
            ),
        ],
    );

    assert_eq!(pack.protocol_version, PACK_PROTOCOL_VERSION_V1);
    let source = ProcessPackSource::from_packs(vec![pack]);
    let mut registry = WorldRegistry::new();
    registry.install_source(&source).unwrap();

    let session = registry.create("fixture.protocol-v1").unwrap();
    assert_eq!(session.snapshot().title, "Created over v1");
}

#[test]
fn host_rejects_response_protocol_drift_from_manifest_version() {
    let root = temp_dir("version-drift");
    let pack = v1_fixture(
        &root,
        &[response_line(
            PACK_PROTOCOL_VERSION_V2,
            1,
            PackResponse::Descriptor {
                descriptor: descriptor(),
            },
        )],
    );

    let source = ProcessPackSource::from_packs(vec![pack]);
    let mut registry = WorldRegistry::new();
    registry.install_source(&source).unwrap();

    let error = registry
        .create("fixture.protocol-v1")
        .err()
        .expect("v2 response must not be accepted for a v1 manifest");
    let message = error.to_string();
    assert!(message.contains("protocol version mismatch"));
    assert!(message.contains("expected 1, got 2"));
}
