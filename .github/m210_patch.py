from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing patch anchor: {label}")
    return text.replace(old, new, 1)


cargo = Path("crates/world-cli/Cargo.toml")
text = cargo.read_text()
text = replace_once(
    text,
    'world-integrity = { path = "../world-integrity" }\n',
    'world-integrity = { path = "../world-integrity" }\nworld-investigation = { path = "../world-investigation" }\n',
    "world-cli dependency",
)
cargo.write_text(text)

main = Path("crates/world-cli/src/main.rs")
text = main.read_text()
text = replace_once(
    text,
    'use world_integrity::{check_archive, ArchiveIntegrityError};\n',
    'use world_integrity::{check_archive, ArchiveIntegrityError};\nuse world_investigation::{\n    investigate_first_divergence, ComparisonQueryExecutor, FirstDivergenceInvestigationRequest,\n    InvestigationError,\n};\n',
    "investigation imports",
)
text = replace_once(
    text,
    'const QUERY_PROTOCOL_VERSION: u64 = 1;\n',
    'const QUERY_PROTOCOL_VERSION: u64 = 1;\nconst INVESTIGATION_PROTOCOL: &str = "world-machine-evidence-investigation";\nconst INVESTIGATION_PROTOCOL_VERSION: u64 = 1;\n',
    "investigation protocol constants",
)
text = replace_once(
    text,
    '    EvidenceCompareQuery(PathBuf, PathBuf, String),\n    ListPacks,\n',
    '    EvidenceCompareQuery(PathBuf, PathBuf, String),\n    EvidenceInvestigateCompare(PathBuf, PathBuf, String),\n    ListPacks,\n',
    "command variant",
)
text = replace_once(
    text,
    '        Command::EvidenceCompareQuery(left, right, request) => {\n            let request = read_query_request(&request)?;\n            println!(\n                "{}",\n                evidence_compare_query_json_report(&left, &right, &request)?\n            )\n        }\n        Command::ListPacks => println!("{}", pack_report()?),\n',
    '        Command::EvidenceCompareQuery(left, right, request) => {\n            let request = read_query_request(&request)?;\n            println!(\n                "{}",\n                evidence_compare_query_json_report(&left, &right, &request)?\n            )\n        }\n        Command::EvidenceInvestigateCompare(left, right, request) => {\n            let request = read_query_request(&request)?;\n            println!(\n                "{}",\n                evidence_investigate_compare_json_report(&left, &right, &request)?\n            )\n        }\n        Command::ListPacks => println!("{}", pack_report()?),\n',
    "main command dispatch",
)
text = replace_once(
    text,
    '        [command, left, right, request] if command == "evidence-compare-query" => {\n            Ok(Command::EvidenceCompareQuery(\n                PathBuf::from(left),\n                PathBuf::from(right),\n                request.clone(),\n            ))\n        }\n        [command] if command == "list-packs" => Ok(Command::ListPacks),\n',
    '        [command, left, right, request] if command == "evidence-compare-query" => {\n            Ok(Command::EvidenceCompareQuery(\n                PathBuf::from(left),\n                PathBuf::from(right),\n                request.clone(),\n            ))\n        }\n        [command, left, right, request] if command == "evidence-investigate-compare" => {\n            Ok(Command::EvidenceInvestigateCompare(\n                PathBuf::from(left),\n                PathBuf::from(right),\n                request.clone(),\n            ))\n        }\n        [command] if command == "list-packs" => Ok(Command::ListPacks),\n',
    "command parser",
)
text = replace_once(
    text,
    '  world-cli evidence-compare-query <left.world> <right.world> <request-json|->\\n\\n\\\n  world-cli list-packs\\n\\n\\\n',
    '  world-cli evidence-compare-query <left.world> <right.world> <request-json|->\\n\\n\\\n  world-cli evidence-investigate-compare <left.world> <right.world> <request-json|->\\n\\n\\\n  world-cli list-packs\\n\\n\\\n',
    "usage command",
)
text = replace_once(
    text,
    'evidence-compare-query  Execute a legacy evidence comparison or tagged causal comparison JSON document and emit a JSON status envelope. Use - to read JSON from stdin.\\n\\\nlist-packs  List World Packs this build can create and restore."\n',
    'evidence-compare-query  Execute a legacy evidence comparison or tagged causal comparison JSON document and emit a JSON status envelope. Use - to read JSON from stdin.\\n\\\nevidence-investigate-compare  Progressively investigate first causal divergence across two World archives and emit an investigation JSON status envelope. Use - to read JSON from stdin.\\n\\\nlist-packs  List World Packs this build can create and restore."\n',
    "usage description",
)

anchor = '''fn format_evidence_comparison(\n    left_path: &Path,\n'''
insert = r'''struct SnapshotComparisonQueryExecutor<'a> {
    left: &'a ProjectionSnapshot,
    right: &'a ProjectionSnapshot,
}

impl ComparisonQueryExecutor for SnapshotComparisonQueryExecutor<'_> {
    type Error = world_query::QueryError;

    fn execute(
        &mut self,
        request: &EvidenceComparisonQueryRequest,
    ) -> Result<world_query::EvidenceComparisonQueryResponse, Self::Error> {
        execute_comparison_query_request(self.left, self.right, request)
    }
}

fn evidence_investigate_compare_json_report(
    left_path: &Path,
    right_path: &Path,
    request_json: &str,
) -> Result<String, Box<dyn Error>> {
    let left_archive = load_archive(left_path)?;
    let right_archive = load_archive(right_path)?;
    let registry = world_builtins::registry()?;
    let left_session = registry.open_archive(&left_archive)?;
    let right_session = registry.open_archive(&right_archive)?;
    let left = left_session.snapshot();
    let right = right_session.snapshot();
    evidence_investigate_compare_json_from_snapshots(&left, &right, request_json)
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

fn evidence_investigate_compare_json_from_snapshots(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    request_json: &str,
) -> Result<String, CliError> {
    let request = parse_investigation_request(request_json)?;
    let mut executor = SnapshotComparisonQueryExecutor { left, right };
    let output = match investigate_first_divergence(&mut executor, &request) {
        Ok(result) => serde_json::json!({
            "protocol": INVESTIGATION_PROTOCOL,
            "version": INVESTIGATION_PROTOCOL_VERSION,
            "status": "ok",
            "response": {
                "result": "first-divergence",
                "value": {
                    "root": result.root,
                    "direction": result.direction,
                    "max_depth": result.max_depth,
                    "identical_within_depth": result.identical_within_depth,
                    "divergence_depth": result.divergence_depth,
                    "witnesses": result.witnesses,
                    "truncated": result.truncated,
                }
            },
        }),
        Err(InvestigationError::Executor(error)) => serde_json::json!({
            "protocol": INVESTIGATION_PROTOCOL,
            "version": INVESTIGATION_PROTOCOL_VERSION,
            "status": "error",
            "error": error,
        }),
        Err(InvestigationError::InvalidWindowDepth) => investigation_error("invalid-window-depth"),
        Err(InvestigationError::UnexpectedResponse) => investigation_error("unexpected-response"),
        Err(InvestigationError::InvalidContinuation) => investigation_error("invalid-continuation"),
        Err(InvestigationError::InvalidTrace) => investigation_error("invalid-trace"),
        Err(InvestigationError::UnexpectedNestedRootPresence) => {
            investigation_error("unexpected-nested-root-presence")
        }
    };
    serde_json::to_string(&output).map_err(|error| {
        CliError(format!(
            "failed to serialize evidence investigation JSON: {error}"
        ))
    })
}

fn investigation_error(error: &str) -> serde_json::Value {
    serde_json::json!({
        "protocol": INVESTIGATION_PROTOCOL,
        "version": INVESTIGATION_PROTOCOL_VERSION,
        "status": "error",
        "error": { "error": error },
    })
}

fn parse_investigation_request(
    request_json: &str,
) -> Result<FirstDivergenceInvestigationRequest, CliError> {
    let value: serde_json::Value = serde_json::from_str(request_json)
        .map_err(|error| CliError(format!("invalid evidence investigation JSON: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| CliError("invalid evidence investigation JSON: expected object".into()))?;
    let query = investigation_string_field(object, "query")?;
    if query != "first-divergence" {
        return Err(CliError(format!(
            "unsupported evidence investigation query: {query}"
        )));
    }
    let root = investigation_string_field(object, "root")?.to_owned();
    let direction = match investigation_string_field(object, "direction")? {
        "upstream" => world_query::EvidenceCausalDirection::Upstream,
        "downstream" => world_query::EvidenceCausalDirection::Downstream,
        direction => {
            return Err(CliError(format!(
                "invalid evidence investigation direction: {direction}"
            )))
        }
    };
    Ok(FirstDivergenceInvestigationRequest {
        root,
        direction,
        window_depth: investigation_usize_field(object, "window_depth")?,
        max_depth: investigation_usize_field(object, "max_depth")?,
    })
}

fn investigation_string_field<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<&'a str, CliError> {
    object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            CliError(format!(
                "invalid evidence investigation JSON: {field} must be a string"
            ))
        })
}

fn investigation_usize_field(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<usize, CliError> {
    let value = object
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            CliError(format!(
                "invalid evidence investigation JSON: {field} must be a non-negative integer"
            ))
        })?;
    usize::try_from(value).map_err(|_| {
        CliError(format!(
            "invalid evidence investigation JSON: {field} is too large"
        ))
    })
}

'''
text = replace_once(text, anchor, insert + anchor, "investigation implementation")
main.write_text(text)

next_task = Path("NEXT_TASK.md")
next_task.write_text('''# Next Coding Task — M210 CLI Investigation Adapter

Expose the M209 read-only progressive investigation boundary through `world-cli` without duplicating continuation scheduling or weakening the Projection/AgentRuntime boundary.

## Current baseline

M203–M208 define and prove deterministic, replayable first-divergence semantics. M209 packages those semantics in `world-investigation`, whose production dependency is only `world-query` and whose executor trait prevents the scheduler from reaching `ProjectionSnapshot` directly. The remaining gap is a concrete local adapter that external automation can invoke today.

## M210 — local CLI adapter

Add `world-cli evidence-investigate-compare <left.world> <right.world> <request-json|->`.

The request is an orchestration-layer JSON document:

```json
{"query":"first-divergence","root":"event-7","direction":"upstream","window_depth":2,"max_depth":12}
```

`world-cli` opens the two archives, owns the snapshots locally, implements `ComparisonQueryExecutor`, and delegates all progressive scheduling to `world-investigation`.

## Machine contract

- Emit a separate `world-machine-evidence-investigation` version-1 JSON envelope so orchestration results are not confused with the existing `world-machine-evidence-query` version-1 response DTOs.
- Successful responses contain the M209 absolute result: root, direction, max depth, bounded identity, absolute divergence depth, original-root witnesses, and truncation.
- Underlying `QueryError` values retain their existing stable serialized shape inside the investigation error envelope.
- M209 orchestration contract errors use stable kebab-case error keys.
- Malformed request JSON, unsupported query names, missing/wrong field types, and invalid direction remain CLI transport/input failures: non-zero exit, stderr, no success envelope.
- `-` reads one full JSON document from stdin, matching the existing machine-query commands.

## Boundary rules

- `world-cli` may hold `ProjectionSnapshot`; `world-investigation` still may not.
- The CLI adapter must call `investigate_first_divergence` rather than reimplement replay, offset accumulation, frontier convergence, or trace composition.
- No mutation authority and no AgentRuntime access are introduced.

## Validation

- subprocess test for stdin investigation and a real two-archive first divergence;
- stable investigation envelope and absolute witness trace;
- underlying query error remains a status-error envelope with exit zero;
- malformed JSON remains a non-zero CLI failure;
- `cargo fmt --all -- --check`;
- boundary checks, `world-cli` / `world-investigation` tests, Clippy, full workspace CI, external Pack conformance.

## Non-goals

No Agent tool adapter yet, no MCP/HTTP/WebSocket, no server cursor/session, no protocol-v2 change to evidence queries, no arbitrary graph export, and no mutation APIs.
''')

# Add a real subprocess transport test.
test = Path("crates/world-cli/tests/machine_investigation_first_divergence.rs")
test.write_text(r'''use std::collections::BTreeSet;
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
            let event_id = root
                .strip_prefix("event-")
                .unwrap()
                .parse::<u64>()
                .unwrap();
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
''')
