#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
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

fn write_fixture_process(path: &Path, responses: &[String]) {
    let mut script = String::from("#!/bin/sh\n");
    for response in responses {
        script.push_str("IFS= read -r _line || exit 1\n");
        script.push_str("printf '%s\\n' ");
        script.push_str(&shell_quote(response));
        script.push('\n');
    }
    script.push_str("IFS= read -r _shutdown || true\n");
    fs::write(path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn write_v1_manifest(root: &Path) -> PathBuf {
    let mut manifest = PackManifest::process(descriptor(), "runtime.sh", Vec::new());
    manifest.protocol_version = PACK_PROTOCOL_VERSION_V1;
    let path = root.join("fixture.world-pack.json");
    fs::write(&path, manifest.to_json_pretty().unwrap()).unwrap();
    path
}

#[test]
fn host_runs_a_manifest_declared_v1_pack_using_v1_envelopes() {
    let root = temp_dir("v1-coexists");
    let runtime = root.join("runtime.sh");
    write_fixture_process(
        &runtime,
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

    let pack = ProcessPack::load(write_v1_manifest(&root)).unwrap();
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
    let runtime = root.join("runtime.sh");
    write_fixture_process(
        &runtime,
        &[response_line(
            PACK_PROTOCOL_VERSION_V2,
            1,
            PackResponse::Descriptor {
                descriptor: descriptor(),
            },
        )],
    );

    let pack = ProcessPack::load(write_v1_manifest(&root)).unwrap();
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
