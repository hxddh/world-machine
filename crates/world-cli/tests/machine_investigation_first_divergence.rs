use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_projection::SelectionId;

#[test]
fn stdin_progressive_investigation_emits_stable_machine_envelope() {
    let (left, right, root) = divergent_world_fixture();
    let request = serde_json::json!({
        "query": "first-divergence",
        "root": root,
        "direction": "upstream",
        "window_depth": 1,
        "max_depth": 3,
    });
    let output = run_query(&left, &right, &request.to_string());
    assert!(output.status.success(), "{}", stderr(&output));

    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["protocol"], "world-machine-evidence-investigation");
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["status"], "ok");
    assert_eq!(envelope["response"]["result"], "first-divergence");
    let value = &envelope["response"]["value"];
    assert_eq!(value["root"], request["root"]);
    assert_eq!(value["direction"], "upstream");
    assert_eq!(value["max_depth"], 3);
    assert_eq!(value["identical_within_depth"], false);
    assert_eq!(value["divergence_depth"], 1);
    assert_eq!(value["truncated"], false);
    let witnesses = value["witnesses"].as_array().unwrap();
    assert!(!witnesses.is_empty());
    let trace = witnesses[0]["trace"].as_array().unwrap();
    assert_eq!(trace.first().unwrap(), &request["root"]);

    let _ = fs::remove_file(left);
    let _ = fs::remove_file(right);
}

#[test]
fn query_errors_remain_status_error_with_zero_exit() {
    let (left, right, _) = divergent_world_fixture();
    let request = serde_json::json!({
        "query": "first-divergence",
        "root": "not-a-selection",
        "direction": "upstream",
        "window_depth": 1,
        "max_depth": 2,
    });
    let output = run_query(&left, &right, &request.to_string());
    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["status"], "error");
    assert_eq!(envelope["error"]["error"], "invalid-selection-key");
    assert_eq!(envelope["error"]["details"], "not-a-selection");
    let _ = fs::remove_file(left);
    let _ = fs::remove_file(right);
}

#[test]
fn malformed_investigation_json_is_a_cli_failure() {
    let (left, right, _) = divergent_world_fixture();
    let output = run_query(&left, &right, "{not-json");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("invalid evidence investigation JSON"));
    assert!(output.stdout.is_empty());
    let _ = fs::remove_file(left);
    let _ = fs::remove_file(right);
}

fn run_query(left: &Path, right: &Path, stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_world-cli"))
        .args([
            "evidence-investigate-compare",
            left.to_str().unwrap(),
            right.to_str().unwrap(),
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn divergent_world_fixture() -> (PathBuf, PathBuf, String) {
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
            let has_visible_cause = item
                .caused_by
                .iter()
                .map(|cause| SelectionId::Event(*cause))
                .any(|cause| visible.contains(&cause));
            if !has_visible_cause {
                continue;
            }

            let root = item.id.stable_key();
            let event_id = root.strip_prefix("event-").unwrap().parse::<u64>().unwrap();
            let mut archive = session.archive().unwrap().unwrap();
            let left = temp_world_path("left");
            fs::write(&left, archive.to_json_pretty().unwrap()).unwrap();
            let archived = archive
                .events
                .iter_mut()
                .find(|event| event.id == event_id)
                .expect("timeline event should exist in archive");
            archived.caused_by.clear();
            let right = temp_world_path("right");
            fs::write(&right, archive.to_json_pretty().unwrap()).unwrap();
            return (left, right, root);
        }
    }
    panic!("a built-in Pack should expose at least one timeline-visible causal edge")
}

fn temp_world_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-m210-{}-{nonce}-{label}.world",
        std::process::id()
    ))
}
