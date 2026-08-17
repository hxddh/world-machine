use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_query::{EvidenceQueryRequest, EvidenceQueryResponse, QueryError};

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
