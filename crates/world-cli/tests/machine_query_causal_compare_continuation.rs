use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_projection::SelectionId;
use world_query::{
    EvidenceCausalComparisonRequest, EvidenceCausalComparisonResponse, EvidenceCausalDirection,
    EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse,
};

#[test]
fn stdin_causal_comparison_continuation_replays_through_existing_compare_transport() {
    let (path, root) = world_fixture_with_visible_causal_edge();
    let first_request = EvidenceComparisonQueryRequest::Causal(
        EvidenceCausalComparisonRequest::CausalNeighborhood {
            root: root.clone(),
            upstream_depth: 0,
            downstream_depth: 0,
        },
    );
    let first = run_typed_compare(&path, &path, &first_request);
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::CausalNeighborhood { value: first },
    ) = first
    else {
        panic!("expected causal-neighborhood comparison response")
    };
    let continuation = first
        .upstream_continuations
        .first()
        .expect("visible causal parent should create a depth-zero upstream continuation");
    assert_eq!(continuation.event, root);
    assert_eq!(continuation.direction, EvidenceCausalDirection::Upstream);
    assert!(continuation.left_frontier);
    assert!(continuation.right_frontier);

    let second = run_typed_compare(&path, &path, &continuation.request);
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::CausalNeighborhood { value: second },
    ) = second
    else {
        panic!("expected causal-neighborhood comparison response")
    };
    assert!(second.identical);
    assert_eq!(second.upstream_depth, 1);

    let _ = fs::remove_file(path);
}

fn run_typed_compare(
    left: &Path,
    right: &Path,
    request: &EvidenceComparisonQueryRequest,
) -> EvidenceComparisonQueryResponse {
    let request = serde_json::to_string(request).unwrap();
    let output = run_query(
        &[
            "evidence-compare-query",
            left.to_str().unwrap(),
            right.to_str().unwrap(),
            "-",
        ],
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

fn world_fixture_with_visible_causal_edge() -> (PathBuf, String) {
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
            if item
                .caused_by
                .iter()
                .map(|cause| SelectionId::Event(*cause))
                .any(|cause| visible.contains(&cause))
            {
                let archive = session.archive().unwrap().unwrap();
                let path = temp_world_path();
                fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();
                return (path, item.id.stable_key());
            }
        }
    }
    panic!("a built-in Pack should expose at least one timeline-visible causal edge")
}

fn temp_world_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-m202-{}-{nonce}.world",
        std::process::id()
    ))
}
