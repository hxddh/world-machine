use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use world_analyst_client::{AnalystTurnProcess, AnalystTurnProcessConfig};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn rust_process_client_reuses_one_turn_host_and_pi_session() {
    if !cfg!(unix) {
        return;
    }

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("world-analyst-client should live under crates/")
        .to_path_buf();
    let temp = temp_dir("process");
    fs::create_dir_all(&temp).unwrap();
    let fake_pi = temp.join("fake-pi.mjs");
    write_fake_pi(&fake_pi);

    let mut config = AnalystTurnProcessConfig::new(
        repo_root.join("integrations/pi/world-machine-analyst-turn-host.mjs"),
        "left.world",
        "right.world",
    );
    config.provider = Some("fake".into());
    config.model = Some("fake".into());
    config.pi_program = Some(fake_pi.clone());
    config.analyst_program = Some(fake_pi.clone());

    let mut process = AnalystTurnProcess::spawn(&config).unwrap();
    let process_id = process.id();

    let first = process.ask("first", Some(2_000)).unwrap();
    assert_eq!(process.id(), process_id);
    assert_eq!(first.text.as_deref(), Some("answer-1"));
    assert_eq!(first.tool_calls.len(), 1);
    assert_eq!(first.tool_calls[0].call_id, "tool-1");
    assert_eq!(first.tool_calls[0].tool, "world.first-divergence");
    assert_eq!(first.tool_calls[0].output["turn"], 1);

    let second = process.ask("second", Some(2_000)).unwrap();
    assert_eq!(process.id(), process_id);
    assert_eq!(second.text.as_deref(), Some("answer-2"));
    assert_eq!(second.tool_calls[0].call_id, "tool-2");
    assert_eq!(second.tool_calls[0].output["turn"], 2);

    let status = process.shutdown().unwrap();
    assert!(status.success());
    let _ = fs::remove_dir_all(temp);
}

fn write_fake_pi(path: &Path) {
    let source = r#"#!/usr/bin/env node
let buffer = Buffer.alloc(0);
let turn = 0;
process.stdin.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const newline = buffer.indexOf(0x0a);
    if (newline < 0) break;
    const line = buffer.subarray(0, newline).toString("utf8");
    buffer = buffer.subarray(newline + 1);
    if (!line) continue;
    const request = JSON.parse(line);
    turn += 1;
    emit({ type: "response", id: request.id, command: "prompt", success: true });
    emit({
      type: "tool_execution_start",
      toolCallId: "tool-" + turn,
      toolName: "world_first_divergence",
      args: { root: "event-" + turn }
    });
    emit({
      type: "tool_execution_end",
      toolCallId: "tool-" + turn,
      toolName: "world_first_divergence",
      result: {
        content: [{ type: "text", text: "provider-only" }],
        details: {
          worldMachineTool: "world.first-divergence",
          output: { turn }
        }
      },
      isError: false
    });
    emit({
      type: "message_end",
      message: { role: "assistant", content: "answer-" + turn }
    });
    emit({ type: "agent_settled" });
  }
});
function emit(value) { process.stdout.write(JSON.stringify(value) + "\n"); }
"#;
    fs::write(path, source).unwrap();
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-m221-{label}-{}-{nonce}",
        std::process::id()
    ))
}
