from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


main = Path("crates/world-cli/src/main.rs")
text = main.read_text()
text = replace_once(
    text,
    "use std::io::{self, Read};",
    "use std::io::{self, BufRead, Read, Write};",
    "session io imports",
)
text = replace_once(
    text,
    "    EvidenceQuery(PathBuf, String),\n    EvidenceCompareQuery(PathBuf, PathBuf, String),",
    "    EvidenceQuery(PathBuf, String),\n    EvidenceQuerySession(PathBuf),\n    EvidenceCompareQuery(PathBuf, PathBuf, String),",
    "session command variant",
)
text = replace_once(
    text,
    "        Command::EvidenceQuery(path, request) => {\n            let request = read_query_request(&request)?;\n            println!(\"{}\", evidence_query_json_report(&path, &request)?)\n        }\n        Command::EvidenceCompareQuery(left, right, request) => {",
    "        Command::EvidenceQuery(path, request) => {\n            let request = read_query_request(&request)?;\n            println!(\"{}\", evidence_query_json_report(&path, &request)?)\n        }\n        Command::EvidenceQuerySession(path) => evidence_query_session(&path)?,\n        Command::EvidenceCompareQuery(left, right, request) => {",
    "session main dispatch",
)
text = replace_once(
    text,
    "        [command, path, request] if command == \"evidence-query\" => {\n            Ok(Command::EvidenceQuery(PathBuf::from(path), request.clone()))\n        }\n        [command, left, right, request] if command == \"evidence-compare-query\" => {",
    "        [command, path, request] if command == \"evidence-query\" => {\n            Ok(Command::EvidenceQuery(PathBuf::from(path), request.clone()))\n        }\n        [command, path] if command == \"evidence-query-session\" => {\n            Ok(Command::EvidenceQuerySession(PathBuf::from(path)))\n        }\n        [command, left, right, request] if command == \"evidence-compare-query\" => {",
    "session parser",
)
text = replace_once(
    text,
    "  world-cli evidence-query <file.world> <request-json|->\\n\\n\\\n  world-cli evidence-compare-query <left.world> <right.world> <request-json|->\\n\\n\\\n",
    "  world-cli evidence-query <file.world> <request-json|->\\n\\n\\\n  world-cli evidence-query-session <file.world>\\n\\n\\\n  world-cli evidence-compare-query <left.world> <right.world> <request-json|->\\n\\n\\\n",
    "session usage command",
)
text = replace_once(
    text,
    "evidence-query  Execute an EvidenceQueryRequest JSON document and emit a JSON status envelope. Use - to read JSON from stdin.\\n\\\nevidence-compare-query  Execute an EvidenceComparisonRequest JSON document and emit a JSON status envelope. Use - to read JSON from stdin.\\n\\\n",
    "evidence-query  Execute an EvidenceQueryRequest JSON document and emit a JSON status envelope. Use - to read JSON from stdin.\\n\\\nevidence-query-session  Load one World snapshot, then execute newline-delimited EvidenceQueryRequest JSON documents from stdin and emit one v1 status envelope per non-empty line.\\n\\\nevidence-compare-query  Execute an EvidenceComparisonRequest JSON document and emit a JSON status envelope. Use - to read JSON from stdin.\\n\\\n",
    "session usage description",
)

session_code = r'''fn evidence_query_session(path: &Path) -> Result<(), Box<dyn Error>> {
    let archive = load_archive(path)?;
    let registry = world_builtins::registry()?;
    let session = registry.open_archive(&archive)?;
    let snapshot = session.snapshot();
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_evidence_query_session(&snapshot, stdin.lock(), stdout.lock())
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

fn run_evidence_query_session<R, W>(
    snapshot: &ProjectionSnapshot,
    reader: R,
    mut writer: W,
) -> Result<(), CliError>
where
    R: BufRead,
    W: Write,
{
    for line in reader.lines() {
        let line = line
            .map_err(|error| CliError(format!("failed to read evidence query session: {error}")))?;
        if line.trim().is_empty() {
            continue;
        }

        let output = evidence_query_json_from_snapshot(snapshot, &line)?;
        writeln!(writer, "{output}").map_err(|error| {
            CliError(format!("failed to write evidence query session response: {error}"))
        })?;
        writer.flush().map_err(|error| {
            CliError(format!("failed to flush evidence query session response: {error}"))
        })?;
    }
    Ok(())
}

'''
text = replace_once(
    text,
    "fn evidence_query_json_from_snapshot(\n    snapshot: &ProjectionSnapshot,",
    session_code + "fn evidence_query_json_from_snapshot(\n    snapshot: &ProjectionSnapshot,",
    "session implementation",
)
main.write_text(text)

Path("crates/world-cli/tests/machine_query_session.rs").write_text(r'''use std::fs;
use std::io::Write;
use std::path::PathBuf;
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

fn run_session(path: &PathBuf, stdin: &str) -> Output {
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
''')

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M200 Persistent Machine Query Session

Turn the stable machine evidence-query contract into an efficient long-lived CLI transport by loading one World snapshot once and processing a stream of newline-delimited query requests over stdin.

## Current baseline

The machine investigation surface is semantically stable through M199:

- M185–M198 established typed query DTOs, stable errors, protocol-v1 envelopes, visible selection/detail, state-evidence queries, and a complete causal investigation family;
- M199 proves cross-query causal consistency without requiring product-code changes;
- `world-cli evidence-query <file.world> <request-json|->` already exposes the generic query contract, but each invocation parses/restores the World and then exits.

For an external agent adapter or interactive investigator, process-per-query restore cost is unnecessary and creates avoidable latency.

## Product goal

Add:

```text
world-cli evidence-query-session <file.world>
```

The command restores the World once, takes one immutable `ProjectionSnapshot`, then reads `EvidenceQueryRequest` documents as NDJSON from stdin until EOF.

For every non-empty input line it emits exactly one existing protocol-v1 status envelope followed by `\n`, then flushes stdout before reading/processing further work.

## Transport contract

1. Input is **one complete JSON request per line**. Multi-line pretty-printed JSON is intentionally not part of the session framing.
2. Empty or whitespace-only lines are ignored and produce no response.
3. Responses are strictly ordered and positional: the Nth non-empty valid request produces the Nth envelope.
4. Success and semantic `QueryError` responses use the exact existing one-shot envelope:
   - `{protocol, version:1, status:"ok", response:...}`
   - `{protocol, version:1, status:"error", error:...}`
5. A semantic query error does **not** terminate the session; later records continue.
6. Malformed request JSON remains a transport failure, matching one-shot semantics. It writes no synthetic QueryError envelope, terminates the process nonzero, and reports the existing `invalid evidence query JSON` error on stderr.
7. Any already completed response line must have been flushed before a later malformed record terminates the process.
8. EOF after valid records exits zero.
9. No request IDs are added in M200; ordered NDJSON is sufficient for this sequential transport and avoids changing the v1 envelope.

## Architecture boundary

- Implement session framing only in `world-cli`.
- Reuse `evidence_query_json_from_snapshot` so one-shot and session envelopes cannot drift.
- Load archive, registry session, and snapshot exactly once before the input loop.
- Keep the snapshot immutable/read-only for the lifetime of the session.
- Do not move streaming, stdin/stdout, or process concerns into `world-query`, `world-projection`, or `world-core`.
- Do not expose a full ProjectionSnapshot to in-world AgentRuntime.

## Tests

Prove at minimum with real subprocess tests:

1. multiple NDJSON requests produce multiple ordered protocol-v1 envelopes in one process;
2. blank lines are ignored;
3. a semantic QueryError produces an error envelope and the following valid request still succeeds;
4. malformed JSON after one valid record exits nonzero, reports the transport error on stderr, and preserves exactly the already-completed response line;
5. existing one-shot machine-query subprocess tests remain unchanged and green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M200

Do not add request IDs, concurrency, out-of-order responses, mutation commands, world reload/watch semantics, comparison sessions, TCP/HTTP/WebSocket/MCP, AgentRuntime access, protocol v2, or automatic recovery after malformed JSON.
''')
