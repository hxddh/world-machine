use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_projection::SelectionId;
use world_query::{EvidenceCausalDirection, EvidenceQueryRequest, EvidenceQueryResponse};

#[test]
fn stdin_frontier_continuation_can_be_replayed_as_the_next_machine_query() {
    let (path, root, expected_parent) = world_fixture_with_visible_causal_edge();
    let request = EvidenceQueryRequest::CausalNeighborhood {
        root: root.clone(),
        upstream_depth: 0,
        downstream_depth: 0,
    };
    let first = run_typed_query(&path, &request);
    let EvidenceQueryResponse::CausalNeighborhood { value: first } = first else {
        panic!("expected causal-neighborhood response")
    };
    assert!(first.upstream_truncated);
    assert_eq!(first.upstream_frontier, vec![root.clone()]);
    let continuation = first
        .upstream_continuations
        .first()
        .expect("depth-zero causal root should expose an upstream continuation");
    assert_eq!(continuation.event, root);
    assert_eq!(continuation.direction, EvidenceCausalDirection::Upstream);

    let second = run_typed_query(&path, &continuation.request);
    let EvidenceQueryResponse::CausalNeighborhood { value: second } = second else {
        panic!("expected causal-neighborhood response")
    };
    assert!(
        second
            .upstream
            .iter()
            .any(|node| node.event == expected_parent),
        "continuation should reveal the visible causal parent"
    );

    let _ = fs::remove_file(path);
}

fn run_typed_query(path: &Path, request: &EvidenceQueryRequest) -> EvidenceQueryResponse {
    let request = serde_json::to_string(request).unwrap();
    let output = run_query(
        &["evidence-query", path.to_str().unwrap(), "-"],
        Some(&request),
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["protocol"], "world-machine-evidence-query");
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["status"], "ok");
    serde_json::from_value(envelope["response"].clone()).unwrap()
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

fn world_fixture_with_visible_causal_edge() -> (PathBuf, String, String) {
    let registry = world_builtins::registry().unwrap();
    for descriptor in registry.descriptors() {
        let session = registry.create(&descriptor.pack.id).unwrap();
        let snapshot = session.snapshot();
        let visible = snapshot
            .timeline
            .items
            .iter()
            .map(|item| item.id)
            .collect::<BTreeSet<_>>();
        for item in &snapshot.timeline.items {
            for cause in &item.caused_by {
                let cause = SelectionId::Event(*cause);
                if !visible.contains(&cause) {
                    continue;
                }
                let archive = session.archive().unwrap().unwrap();
                let path = temp_world_path();
                fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();
                return (path, item.id.stable_key(), cause.stable_key());
            }
        }
    }
    panic!("a built-in Pack should expose at least one timeline-visible causal edge")
}

fn temp_world_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("world-machine-causal-continuation-{unique}.world"))
}
