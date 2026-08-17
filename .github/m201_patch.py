from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


test = Path("crates/world-cli/tests/machine_query_session.rs")
text = test.read_text()
text = replace_once(
    text,
    "use std::fs;\nuse std::io::Write;",
    "use std::collections::BTreeSet;\nuse std::fs;\nuse std::io::{BufRead, BufReader, Read, Write};",
    "session test std imports",
)
text = replace_once(
    text,
    "use world_query::{EvidenceQueryRequest, EvidenceQueryResponse, QueryError};",
    "use world_projection::SelectionId;\nuse world_query::{\n    EvidenceCausalDirection, EvidenceQueryRequest, EvidenceQueryResponse, QueryError,\n};",
    "session test query imports",
)

interactive_test = r'''
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
'''
text = replace_once(
    text,
    "fn run_session(path: &Path, stdin: &str) -> Output {",
    interactive_test + "\nfn run_session(path: &Path, stdin: &str) -> Output {",
    "interactive session test",
)

causal_fixture = r'''
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

'''
text = replace_once(
    text,
    "fn temp_world_path() -> PathBuf {",
    causal_fixture + "fn temp_world_path() -> PathBuf {",
    "causal fixture",
)
test.write_text(text)

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M201 Persistent Machine Query Session

Turn the stable machine evidence-query contract into an efficient long-lived CLI transport by restoring one World snapshot once and processing a stream of newline-delimited query requests over stdin.

## Current baseline

The machine investigation surface is complete through M200:

- M185–M198 established typed query DTOs, stable semantic errors, protocol-v1 envelopes, visible selection/detail, state-evidence queries, and the causal investigation family;
- M199 locked the causal surface with cross-query invariants rather than adding new product semantics;
- M200 added executable causal continuations so bounded causal frontiers carry typed replayable `EvidenceQueryRequest::CausalNeighborhood` requests;
- `world-cli evidence-query <file.world> <request-json|->` exposes the generic machine contract, but each invocation still restores the World and exits.

M201 makes repeated investigation efficient without weakening any semantic or visibility boundary.

## Product goal

Add:

```text
world-cli evidence-query-session <file.world>
```

The command restores the World once, takes one immutable `ProjectionSnapshot`, then reads `EvidenceQueryRequest` documents as NDJSON from stdin until EOF.

For every non-empty input line it emits exactly one existing protocol-v1 status envelope followed by `\n`, then flushes stdout immediately so interactive callers can submit the next request without closing stdin.

## Transport contract

1. Input framing is one complete JSON request per line. Multi-line pretty-printed JSON is intentionally outside the session contract.
2. Empty or whitespace-only lines are ignored and produce no response.
3. Responses are ordered and positional: the Nth non-empty valid request produces the Nth envelope.
4. Success and semantic `QueryError` responses reuse the exact existing one-shot v1 envelope.
5. A semantic query error does not terminate the session; later requests continue.
6. Malformed request JSON remains a transport failure, matching one-shot semantics: no synthetic QueryError envelope, nonzero exit, existing stderr diagnostic.
7. Every completed response is flushed before the session waits for another request. A later malformed record cannot erase already completed output.
8. EOF after valid records exits zero.
9. M200 continuation requests are ordinary `EvidenceQueryRequest` values and can be replayed directly inside the same session process.
10. No request IDs are added in M201; sequential NDJSON already has unambiguous positional correlation.

## Architecture boundary

- Session framing belongs only in `world-cli`.
- Reuse `evidence_query_json_from_snapshot` so one-shot and session envelopes cannot drift.
- Load archive, registry session, and snapshot exactly once before the input loop.
- Keep the snapshot immutable/read-only for the entire session.
- Do not move stdin/stdout, buffering, or process concerns into `world-query`, `world-projection`, or `world-core`.
- Do not expose `ProjectionSnapshot` to in-world AgentRuntime.

## Tests

Prove with real subprocess behavior:

1. multiple NDJSON requests produce ordered protocol-v1 envelopes in one process;
2. blank lines are ignored;
3. a semantic QueryError emits an error envelope and a following valid request still succeeds;
4. malformed JSON after one valid record exits nonzero while preserving the already completed response;
5. a truly interactive caller can write one request, receive the flushed response before EOF, extract an M200 causal continuation, replay it through the same process, and receive the continued causal window;
6. existing one-shot, M199 invariant, and M200 continuation tests remain green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M201

Do not add request IDs, concurrency, out-of-order responses, mutation commands, World reload/watch semantics, comparison sessions, TCP/HTTP/WebSocket/MCP, AgentRuntime access, automatic malformed-record recovery, or protocol v2.
''')
