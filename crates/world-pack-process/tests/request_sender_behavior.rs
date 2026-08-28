#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use world_host::{WorldPackSource, WorldRegistry};
use world_pack_process::{ProcessPack, ProcessPackSource, DEFAULT_MAX_REQUEST_BYTES};
use world_pack_protocol::{
    encode_response, PackDescriptor, PackManifest, PackResponse, PackResponseEnvelope,
    ProjectionCapabilitiesWire, ProjectionSnapshotWire,
};
use world_persistence::WorldPackRef;
use world_projection::ProjectionIntent;

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "world-pack-process-m261-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn descriptor() -> PackDescriptor {
    PackDescriptor::new(
        WorldPackRef::new("fixture.m261.request-bound", "1"),
        "M261 Request Boundary Fixture",
        "Proves local sender rejection does not perturb the process session",
    )
}

fn snapshot(world_time: u64, title: &str) -> ProjectionSnapshotWire {
    ProjectionSnapshotWire {
        title: title.into(),
        world_time,
        capabilities: ProjectionCapabilitiesWire { fork: false },
        ..ProjectionSnapshotWire::default()
    }
}

fn response_line(request_id: u64, response: PackResponse) -> String {
    encode_response(&PackResponseEnvelope::new(request_id, response)).unwrap()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
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
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[test]
fn oversized_multibyte_request_is_local_nonfatal_and_does_not_consume_request_id() {
    let root = temp_dir("local-reject-reuse");
    let runtime = root.join("runtime.sh");
    write_fixture_process(
        &runtime,
        &[
            response_line(
                1,
                PackResponse::Descriptor {
                    descriptor: descriptor(),
                },
            ),
            response_line(
                2,
                PackResponse::Snapshot {
                    snapshot: snapshot(0, "Created"),
                },
            ),
            response_line(
                3,
                PackResponse::Snapshot {
                    snapshot: snapshot(1, "Advanced after local rejection"),
                },
            ),
        ],
    );
    let manifest = PackManifest::process(descriptor(), "runtime.sh", Vec::new());
    let manifest_path = root.join("fixture.world-pack.json");
    fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();

    let pack = ProcessPack::load(&manifest_path).unwrap();
    let source = ProcessPackSource::from_packs(vec![pack]);
    let mut registry = WorldRegistry::new();
    registry.install_source(&source).unwrap();
    let mut session = registry.create("fixture.m261.request-bound").unwrap();
    assert_eq!(session.snapshot().title, "Created");

    // Half as many scalar values as the byte ceiling, but each `é` is two UTF-8
    // bytes. Character-counting would accept this request; byte-counting must
    // reject it locally once the JSON envelope pushes the physical frame over
    // the existing 16 MiB Pack request ceiling.
    let oversized = "é".repeat(DEFAULT_MAX_REQUEST_BYTES / 2);
    assert!(oversized.chars().count() < DEFAULT_MAX_REQUEST_BYTES);
    assert!(oversized.len() >= DEFAULT_MAX_REQUEST_BYTES);
    let error = session
        .handle(ProjectionIntent::InvokeCommand(oversized))
        .unwrap_err();
    assert!(error.to_string().contains("request frame exceeds"));
    assert_eq!(session.snapshot().title, "Created");

    // The fixture has no response slot for the rejected Handle. Therefore this
    // succeeds only if zero oversized bytes crossed stdin, the child stayed
    // alive, and request id 3 was not consumed by the local rejection.
    let advanced = session.advance_background(1).unwrap();
    assert_eq!(advanced.world_time, 1);
    assert_eq!(advanced.title, "Advanced after local rejection");

    drop(session);
    let _ = fs::remove_dir_all(root);
}
