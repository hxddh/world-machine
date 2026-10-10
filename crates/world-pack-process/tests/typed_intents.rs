//! A design or a name goes to a Pack typed only when the Pack speaks v8 and
//! says it takes them; a Pack on v7, or one that does not say so, hears
//! the command it offered with the argument after `=`, as it always did.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use world_host::WorldRegistry;
use world_pack_process::{ProcessPack, ProcessPackSource};
use world_pack_protocol::{
    encode_response, PackDescriptor, PackManifest, PackResponse, PackResponseEnvelope,
    ProjectionSnapshotWire, CAPABILITY_DESIGNS, CAPABILITY_NAMES, PACK_PROTOCOL_VERSION_V7,
    PACK_PROTOCOL_VERSION_V8,
};
use world_persistence::WorldPackRef;
use world_projection::{Design, ProjectionIntent};

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "world-pack-process-typed-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn descriptor() -> PackDescriptor {
    PackDescriptor::new(
        WorldPackRef::new("fixture.typed", "1"),
        "Typed Fixture",
        "A Pack that writes down what it is asked",
    )
}

fn response_line(version: u32, request_id: u64, response: PackResponse) -> String {
    let envelope = PackResponseEnvelope::for_version(version, request_id, response).unwrap();
    encode_response(&envelope).unwrap()
}

fn snapshot() -> PackResponse {
    PackResponse::Snapshot {
        snapshot: ProjectionSnapshotWire {
            title: "Typed".into(),
            ..ProjectionSnapshotWire::default()
        },
    }
}

/// A Pack that answers in turn and writes every request it reads to
/// `requests.log`.
fn write_pack(root: &Path, version: u32, described: PackDescriptor) -> ProcessPack {
    let log = root.join("requests.log");
    let responses = [
        response_line(
            version,
            1,
            PackResponse::Descriptor {
                descriptor: described,
            },
        ),
        response_line(version, 2, snapshot()),
        response_line(version, 3, snapshot()),
        response_line(version, 4, snapshot()),
    ];
    let mut steps = vec![format!("log\t{}", log.display())];
    steps.extend(support::respond(&responses));
    let mut manifest = PackManifest::process(descriptor(), "runtime", Vec::new());
    manifest.protocol_version = version;
    support::fixture_pack(root, manifest, &steps)
}

fn design() -> Design {
    Design::parse(&format!("{}:2f", "01".repeat(128))).unwrap()
}

/// The requests the Pack read for a design and a name.
fn sent(version: u32, described: PackDescriptor) -> Vec<String> {
    let root = temp_dir(&format!("v{version}"));
    let pack = write_pack(&root, version, described);
    let mut registry = WorldRegistry::new();
    registry
        .install_source(&ProcessPackSource::from_packs(vec![pack]))
        .unwrap();
    let mut session = registry.create("fixture.typed").unwrap();
    session
        .handle(ProjectionIntent::Design {
            target: "fixture.mark.design.7".into(),
            pattern: design(),
        })
        .unwrap();
    session
        .handle(ProjectionIntent::Name {
            target: "fixture.mark.name.7".into(),
            name: "Ann = Bea".into(),
        })
        .unwrap();
    drop(session);
    let log = fs::read_to_string(root.join("requests.log")).unwrap();
    let _ = fs::remove_dir_all(root);
    log.lines().skip(2).map(str::to_string).collect()
}

#[test]
fn a_pack_on_v7_hears_a_design_and_a_name_as_the_command_it_offered() {
    let requests = sent(PACK_PROTOCOL_VERSION_V7, descriptor());
    assert_eq!(requests.len(), 2, "{requests:?}");
    assert!(
        requests[0].contains(r#""type":"invoke_command""#)
            && requests[0].contains(&format!("fixture.mark.design.7={}", design().text())),
        "{}",
        requests[0]
    );
    assert!(
        requests[1].contains(r#""command":"fixture.mark.name.7=Ann = Bea""#),
        "{}",
        requests[1]
    );
}

#[test]
fn a_pack_on_v8_that_takes_designs_and_names_hears_them_typed() {
    let described = descriptor().with_capabilities([CAPABILITY_DESIGNS, CAPABILITY_NAMES]);
    let requests = sent(PACK_PROTOCOL_VERSION_V8, described);
    assert!(
        requests[0].contains(r#""type":"design""#)
            && requests[0].contains(r#""target":"fixture.mark.design.7""#)
            && requests[0].contains(&format!(r#""pattern":"{}""#, design().text())),
        "{}",
        requests[0]
    );
    assert!(
        requests[1].contains(r#""type":"name""#) && requests[1].contains(r#""name":"Ann = Bea""#),
        "{}",
        requests[1]
    );
}

#[test]
fn a_pack_on_v8_that_does_not_say_it_takes_them_hears_the_old_form() {
    let requests = sent(PACK_PROTOCOL_VERSION_V8, descriptor());
    assert!(
        requests[0].contains(r#""type":"invoke_command""#),
        "{}",
        requests[0]
    );
    assert!(
        requests[1].contains(r#""type":"invoke_command""#),
        "{}",
        requests[1]
    );
}
