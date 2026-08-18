use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_projection::SelectionId;

#[test]
fn stdio_process_lists_tools_and_invokes_first_divergence_in_one_session() {
    let (left, right, root) = divergent_world_fixture();
    let input = format!(
        "{}\n{}\n",
        serde_json::json!({"op": "list-tools"}),
        serde_json::json!({
            "op": "invoke",
            "call_id": "call-1",
            "tool": "world.first-divergence",
            "input": {
                "root": root,
                "direction": "upstream",
                "window_depth": 1,
                "max_depth": 3
            }
        })
    );
    let output = run_process(&left, &right, &input);
    assert!(output.status.success(), "{}", stderr(&output));
    let lines = stdout_values(&output);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["protocol"], "world-machine-readonly-tools");
    assert_eq!(lines[0]["type"], "catalog");
    assert_eq!(lines[0]["tools"][0]["name"], "world.first-divergence");
    assert_eq!(lines[1]["type"], "result");
    assert_eq!(lines[1]["call_id"], "call-1");
    assert_eq!(lines[1]["tool"], "world.first-divergence");
    assert_eq!(lines[1]["output"]["divergence_depth"], 1);
    assert_eq!(lines[1]["output"]["witnesses"][0]["trace"][0], root);
    cleanup(left, right);
}

#[test]
fn correlated_tool_error_does_not_terminate_stdio_session() {
    let (left, right, _) = divergent_world_fixture();
    let input = format!(
        "{}\n{}\n",
        serde_json::json!({
            "op": "invoke",
            "call_id": "missing",
            "tool": "world.missing",
            "input": {}
        }),
        serde_json::json!({"op": "list-tools"})
    );
    let output = run_process(&left, &right, &input);
    assert!(output.status.success(), "{}", stderr(&output));
    let lines = stdout_values(&output);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["type"], "error");
    assert_eq!(lines[0]["call_id"], "missing");
    assert_eq!(lines[0]["error"]["kind"], "unknown-tool");
    assert_eq!(lines[1]["type"], "catalog");
    cleanup(left, right);
}

#[test]
fn malformed_json_line_is_a_process_level_failure() {
    let (left, right, _) = divergent_world_fixture();
    let output = run_process(&left, &right, "{not-json\n");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("invalid JSON on stdin line 1"));
    assert!(output.stdout.is_empty());
    cleanup(left, right);
}

fn run_process(left: &Path, right: &Path, stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_world-agent-tool-stdio"))
        .args([left.to_str().unwrap(), right.to_str().unwrap()])
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

fn stdout_values(output: &Output) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn cleanup(left: PathBuf, right: PathBuf) {
    let _ = fs::remove_file(left);
    let _ = fs::remove_file(right);
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
        "world-machine-m215-stdio-{}-{nonce}-{label}.world",
        std::process::id()
    ))
}
