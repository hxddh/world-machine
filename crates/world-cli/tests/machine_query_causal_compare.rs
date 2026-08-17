use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_query::{
    EvidenceCausalComparisonRequest, EvidenceCausalComparisonResponse,
    EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse,
};

#[test]
fn stdin_causal_comparison_uses_existing_versioned_compare_transport() {
    let (path, root) = world_fixture_with_event();
    let request = EvidenceComparisonQueryRequest::Causal(
        EvidenceCausalComparisonRequest::CausalNeighborhood {
            root: root.clone(),
            upstream_depth: 1,
            downstream_depth: 1,
        },
    );
    let request = serde_json::to_string(&request).unwrap();

    let output = run_query(
        &[
            "evidence-compare-query",
            path.to_str().unwrap(),
            path.to_str().unwrap(),
            "-",
        ],
        Some(&request),
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["protocol"], "world-machine-evidence-query");
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["status"], "ok");

    let response: EvidenceComparisonQueryResponse =
        serde_json::from_value(envelope["response"].clone()).unwrap();
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::CausalNeighborhood { value },
    ) = response
    else {
        panic!("expected causal-neighborhood comparison response")
    };
    assert_eq!(value.root, root);
    assert!(value.identical);
    assert!(value.nodes.is_empty());
    assert!(value.left_only_edges.is_empty());
    assert!(value.right_only_edges.is_empty());

    let _ = fs::remove_file(path);
}

fn run_query(args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_world-cli"))
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    if let Some(input) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin should be piped")
            .write_all(input.as_bytes())
            .unwrap();
    }

    child.wait_with_output().unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn world_fixture_with_event() -> (PathBuf, String) {
    let registry = world_builtins::registry().unwrap();
    for descriptor in registry.descriptors() {
        let session = registry.create(&descriptor.pack.id).unwrap();
        let snapshot = session.snapshot();
        let Some(event) = snapshot
            .timeline
            .items
            .iter()
            .find(|item| item.id.stable_key().starts_with("event-"))
            .map(|item| item.id)
        else {
            continue;
        };
        let archive = session.archive().unwrap().unwrap();
        let path = temp_world_path();
        fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();
        return (path, event.stable_key());
    }
    panic!("a built-in Pack should expose a visible timeline event")
}

fn temp_world_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-m201-{}-{nonce}.world",
        std::process::id()
    ))
}
