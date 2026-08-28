use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use world_query::{
    EvidenceComparisonRequest, EvidenceComparisonResult, EvidenceQueryRequest,
    EvidenceQueryResponse, QueryError,
};

#[test]
fn stdin_why_query_emits_a_versioned_typed_causal_history() {
    let (path, event) = world_fixture_with_event();
    let request = serde_json::to_string(&EvidenceQueryRequest::Why {
        event: event.clone(),
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
    let EvidenceQueryResponse::Why { value } = response else {
        panic!("expected why response")
    };
    assert_eq!(value.event, event);
    assert!(!value.nodes.is_empty());
    assert_eq!(value.nodes[0].event, value.event);
    assert_eq!(value.nodes[0].depth, 0);

    let _ = fs::remove_file(path);
}

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
fn oversized_stdin_fails_before_eof_and_before_query_execution() {
    const MAX_STDIN_BYTES: usize = 64 * 1024 * 1024;
    const CHUNK_BYTES: usize = 64 * 1024;

    let mut child = Command::new(env!("CARGO_BIN_EXE_world-cli"))
        .args(["evidence-query", "definitely-missing-m258.world", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let chunk = vec![b'x'; CHUNK_BYTES];
    let mut remaining = MAX_STDIN_BYTES + 1;
    while remaining > 0 {
        let count = remaining.min(chunk.len());
        match child
            .stdin
            .as_mut()
            .expect("stdin should remain open during overflow")
            .write_all(&chunk[..count])
        {
            Ok(()) => remaining -= count,
            Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => break,
            Err(error) => panic!("failed to stream oversized stdin: {error}"),
        }
    }

    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "world-cli waited for EOF after stdin was already oversized"
        );
        std::thread::sleep(Duration::from_millis(10));
    };

    assert!(!status.success());
    let mut stdout = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    assert!(
        stdout.is_empty(),
        "oversized stdin must not emit a query envelope"
    );
    let mut stderr_bytes = Vec::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_end(&mut stderr_bytes)
        .unwrap();
    let stderr = String::from_utf8_lossy(&stderr_bytes);
    assert!(stderr.contains("machine query stdin exceeded the 67108864-byte transport limit"));
    assert!(!stderr.contains("definitely-missing-m258.world"));
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

fn world_fixture_with_event() -> (PathBuf, String) {
    let registry = world_builtins::registry().unwrap();
    for descriptor in registry.descriptors() {
        let session = registry.create(&descriptor.pack.id).unwrap();
        let snapshot = session.snapshot();
        let Some(event) = snapshot.timeline.items.first().map(|item| item.id) else {
            continue;
        };
        if !event.stable_key().starts_with("event-") {
            continue;
        }
        let archive = session.archive().unwrap().unwrap();
        let path = temp_world_path();
        fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();
        return (path, event.stable_key());
    }
    panic!("a built-in Pack should expose a visible timeline event")
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

static TEMP_WORLD_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn temp_world_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp_world_path_with_nonce(nonce)
}

fn temp_world_path_with_nonce(nonce: u128) -> PathBuf {
    let sequence = TEMP_WORLD_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "world-machine-m188-{}-{nonce}-{sequence}.world",
        std::process::id()
    ))
}

#[test]
fn temp_world_paths_are_unique_for_equal_nonce_across_threads() {
    let nonce = 42;
    let handles = (0..64)
        .map(|_| std::thread::spawn(move || temp_world_path_with_nonce(nonce)))
        .collect::<Vec<_>>();
    let paths = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(paths.len(), 64);
}
