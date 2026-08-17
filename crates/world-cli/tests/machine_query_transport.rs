use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_query::{
    EvidenceComparisonRequest, EvidenceComparisonResult, EvidenceQueryRequest,
    EvidenceQueryResponse, QueryError,
};

#[test]
fn stdin_selection_describe_emits_a_versioned_typed_description() {
    let (path, root) = world_fixture();
    let request = serde_json::to_string(&EvidenceQueryRequest::Describe {
        selection: root.clone(),
    })
    .unwrap();

    let output = run_query(
        &["evidence-query", path.to_str().unwrap(), "-"],
        Some(&request),
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_protocol(&envelope);
    assert_eq!(envelope["status"], "ok");
    let response: EvidenceQueryResponse =
        serde_json::from_value(envelope["response"].clone()).unwrap();
    let EvidenceQueryResponse::Description { value } = response else {
        panic!("expected description response")
    };
    assert_eq!(value.selection, root);
    assert!(!value.title.is_empty());

    let _ = fs::remove_file(path);
}

#[test]
fn stdin_selection_discovery_emits_a_versioned_typed_index() {
    let (path, _) = world_fixture();
    let request = serde_json::to_string(&EvidenceQueryRequest::Selections).unwrap();

    let output = run_query(
        &["evidence-query", path.to_str().unwrap(), "-"],
        Some(&request),
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_protocol(&envelope);
    assert_eq!(envelope["status"], "ok");
    let response: EvidenceQueryResponse =
        serde_json::from_value(envelope["response"].clone()).unwrap();
    let EvidenceQueryResponse::Selections { value } = response else {
        panic!("expected selections response")
    };
    assert!(!value.selections.is_empty());
    assert!(value.selections.iter().all(|selection| {
        selection.selection.starts_with("entity-")
            || selection.selection.starts_with("relation-")
            || selection.selection.starts_with("event-")
    }));

    let _ = fs::remove_file(path);
}

#[test]
fn stdin_neighborhood_and_shortest_path_queries_emit_typed_json() {
    let (path, root) = world_fixture();

    let neighborhood = serde_json::to_string(&EvidenceQueryRequest::Neighborhood {
        root: root.clone(),
        max_depth: 0,
    })
    .unwrap();
    let output = run_query(
        &["evidence-query", path.to_str().unwrap(), "-"],
        Some(&neighborhood),
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_protocol(&envelope);
    assert_eq!(envelope["status"], "ok");
    let response: EvidenceQueryResponse =
        serde_json::from_value(envelope["response"].clone()).unwrap();
    let EvidenceQueryResponse::Neighborhood { value } = response else {
        panic!("expected neighborhood response")
    };
    assert_eq!(value.root, root);
    assert_eq!(value.max_depth, 0);

    let shortest_path = serde_json::to_string(&EvidenceQueryRequest::ShortestPath {
        from: root.clone(),
        to: root.clone(),
    })
    .unwrap();
    let output = run_query(
        &["evidence-query", path.to_str().unwrap(), "-"],
        Some(&shortest_path),
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_protocol(&envelope);
    assert_eq!(envelope["status"], "ok");
    let response: EvidenceQueryResponse =
        serde_json::from_value(envelope["response"].clone()).unwrap();
    let EvidenceQueryResponse::ShortestPath { value } = response else {
        panic!("expected shortest-path response")
    };
    assert_eq!(value.from, root);
    assert_eq!(value.to, root);
    assert!(value.steps.is_empty());

    let _ = fs::remove_file(path);
}

#[test]
fn stdin_semantic_error_is_json_and_exits_zero() {
    let (path, _) = world_fixture();
    let request = serde_json::to_string(&EvidenceQueryRequest::Neighborhood {
        root: "entity-07".into(),
        max_depth: 2,
    })
    .unwrap();

    let output = run_query(
        &["evidence-query", path.to_str().unwrap(), "-"],
        Some(&request),
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_protocol(&envelope);
    assert_eq!(envelope["status"], "error");
    let error: QueryError = serde_json::from_value(envelope["error"].clone()).unwrap();
    assert_eq!(error, QueryError::InvalidSelectionKey("entity-07".into()));
    let _ = fs::remove_file(path);
}

#[test]
fn malformed_stdin_json_is_a_transport_failure() {
    let (path, _) = world_fixture();

    let output = run_query(
        &["evidence-query", path.to_str().unwrap(), "-"],
        Some("{not-json"),
    );

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(stderr(&output).contains("invalid evidence query JSON"));
    let _ = fs::remove_file(path);
}

#[test]
fn stdin_comparison_query_emits_typed_comparison_json() {
    let (path, root) = world_fixture();
    let request = serde_json::to_string(&EvidenceComparisonRequest {
        root: root.clone(),
        max_depth: 1,
    })
    .unwrap();

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
    assert_protocol(&envelope);
    assert_eq!(envelope["status"], "ok");
    let comparison: EvidenceComparisonResult =
        serde_json::from_value(envelope["response"].clone()).unwrap();
    assert_eq!(comparison.root, root);
    assert!(comparison.identical);
    let _ = fs::remove_file(path);
}

#[test]
fn inline_json_query_remains_compatible() {
    let (path, root) = world_fixture();
    let request = serde_json::to_string(&EvidenceQueryRequest::Neighborhood {
        root: root.clone(),
        max_depth: 0,
    })
    .unwrap();

    let output = run_query(&["evidence-query", path.to_str().unwrap(), &request], None);

    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_protocol(&envelope);
    assert_eq!(envelope["status"], "ok");
    let response: EvidenceQueryResponse =
        serde_json::from_value(envelope["response"].clone()).unwrap();
    let EvidenceQueryResponse::Neighborhood { value } = response else {
        panic!("expected neighborhood response")
    };
    assert_eq!(value.root, root);
    let _ = fs::remove_file(path);
}

fn assert_protocol(envelope: &serde_json::Value) {
    assert_eq!(envelope["protocol"], "world-machine-evidence-query");
    assert_eq!(envelope["version"], 1);
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

fn world_fixture() -> (PathBuf, String) {
    let registry = world_builtins::registry().unwrap();
    for descriptor in registry.descriptors() {
        let session = registry.create(&descriptor.pack.id).unwrap();
        let snapshot = session.snapshot();
        let root = snapshot
            .timeline
            .items
            .first()
            .map(|item| item.id)
            .or_else(|| snapshot.inspectors.keys().copied().next());
        if let Some(root) = root {
            let archive = session.archive().unwrap().unwrap();
            let path = temp_world_path();
            fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();
            return (path, root.stable_key());
        }
    }
    panic!("a built-in Pack should expose a visible selection")
}

fn temp_world_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-m188-{}-{nonce}.world",
        std::process::id()
    ))
}
