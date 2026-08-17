use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_projection::SelectionId;
use world_query::{
    EvidenceCausalDirection, EvidenceQueryRequest, EvidenceQueryResponse, QueryError,
};

#[test]
fn session_processes_multiple_ndjson_requests_and_continues_after_semantic_errors() {
    let path = world_fixture();
    let selections = serde_json::to_string(&EvidenceQueryRequest::Selections).unwrap();
    let semantic_error = serde_json::to_string(&EvidenceQueryRequest::Neighborhood {
        root: "entity-07".into(),
        max_depth: 0,
    })
    .unwrap();
    let input = format!("\n{selections}\n{semantic_error}\n\n{selections}\n");

    let output = run_session(&path, &input);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(output.stderr.is_empty());
    let envelopes = stdout_lines(&output);
    assert_eq!(envelopes.len(), 3);

    assert_protocol(&envelopes[0]);
    assert_eq!(envelopes[0]["status"], "ok");
    let first: EvidenceQueryResponse =
        serde_json::from_value(envelopes[0]["response"].clone()).unwrap();
    let EvidenceQueryResponse::Selections { value: first } = first else {
        panic!("expected selections response")
    };
    assert!(!first.selections.is_empty());

    assert_protocol(&envelopes[1]);
    assert_eq!(envelopes[1]["status"], "error");
    let error: QueryError = serde_json::from_value(envelopes[1]["error"].clone()).unwrap();
    assert_eq!(error, QueryError::InvalidSelectionKey("entity-07".into()));

    assert_protocol(&envelopes[2]);
    assert_eq!(envelopes[2]["status"], "ok");
    let third: EvidenceQueryResponse =
        serde_json::from_value(envelopes[2]["response"].clone()).unwrap();
    let EvidenceQueryResponse::Selections { value: third } = third else {
        panic!("expected selections response")
    };
    assert_eq!(third, first);

    let _ = fs::remove_file(path);
}

#[test]
fn malformed_record_is_a_fatal_transport_error_after_prior_flushed_responses() {
    let path = world_fixture();
    let selections = serde_json::to_string(&EvidenceQueryRequest::Selections).unwrap();
    let input = format!("{selections}\n{{not-json\n{selections}\n");

    let output = run_session(&path, &input);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("invalid evidence query JSON"));
    let envelopes = stdout_lines(&output);
    assert_eq!(envelopes.len(), 1);
    assert_protocol(&envelopes[0]);
    assert_eq!(envelopes[0]["status"], "ok");

    let _ = fs::remove_file(path);
}

#[test]
fn session_flushes_before_eof_and_replays_a_causal_continuation_in_process() {
    let (path, root, expected_parent) = world_fixture_with_visible_causal_edge();
    let mut child = Command::new(env!("CARGO_BIN_EXE_world-cli"))
        .args(["evidence-query-session", path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().expect("stdin should be piped");
    let stdout = child.stdout.take().expect("stdout should be piped");
    let mut stdout = BufReader::new(stdout);

    let first_request = EvidenceQueryRequest::CausalNeighborhood {
        root: root.clone(),
        upstream_depth: 0,
        downstream_depth: 0,
    };
    writeln!(stdin, "{}", serde_json::to_string(&first_request).unwrap()).unwrap();
    stdin.flush().unwrap();

    let mut first_line = String::new();
    assert!(stdout.read_line(&mut first_line).unwrap() > 0);
    let first = typed_ok_response(&first_line);
    let EvidenceQueryResponse::CausalNeighborhood { value: first } = first else {
        panic!("expected causal-neighborhood response")
    };
    let continuation = first
        .upstream_continuations
        .first()
        .expect("depth-zero root should expose an upstream continuation");
    assert_eq!(continuation.event, root);
    assert_eq!(continuation.direction, EvidenceCausalDirection::Upstream);

    writeln!(
        stdin,
        "{}",
        serde_json::to_string(&continuation.request).unwrap()
    )
    .unwrap();
    stdin.flush().unwrap();

    let mut second_line = String::new();
    assert!(stdout.read_line(&mut second_line).unwrap() > 0);
    let second = typed_ok_response(&second_line);
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
    assert!(
        second
            .edges
            .iter()
            .any(|edge| edge.cause == expected_parent && edge.effect == continuation.event),
        "continued session window should retain induced-edge semantics"
    );

    drop(stdin);
    let status = child.wait().unwrap();
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("stderr should be piped")
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(status.success(), "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");

    let _ = fs::remove_file(path);
}

fn typed_ok_response(line: &str) -> EvidenceQueryResponse {
    let envelope: serde_json::Value = serde_json::from_str(line).unwrap();
    assert_protocol(&envelope);
    assert_eq!(envelope["status"], "ok");
    serde_json::from_value(envelope["response"].clone()).unwrap()
}

fn run_session(path: &Path, stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_world-cli"))
        .args(["evidence-query-session", path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stdout_lines(output: &Output) -> Vec<serde_json::Value> {
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn assert_protocol(envelope: &serde_json::Value) {
    assert_eq!(envelope["protocol"], "world-machine-evidence-query");
    assert_eq!(envelope["version"], 1);
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn world_fixture() -> PathBuf {
    let registry = world_builtins::registry().unwrap();
    for descriptor in registry.descriptors() {
        let session = registry.create(&descriptor.pack.id).unwrap();
        let snapshot = session.snapshot();
        if snapshot.timeline.items.is_empty() && snapshot.inspectors.is_empty() {
            continue;
        }
        let archive = session.archive().unwrap().unwrap();
        let path = temp_world_path();
        fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();
        return path;
    }
    panic!("a built-in Pack should expose a visible selection")
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
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-m200-{}-{nonce}.world",
        std::process::id()
    ))
}
