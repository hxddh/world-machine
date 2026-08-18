from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing marker for {label}")
    return text.replace(old, new, 1)


root_manifest = Path("Cargo.toml")
text = root_manifest.read_text()
text = replace_once(
    text,
    '  "crates/world-investigation",\n  "crates/world-agent-tools",\n  "crates/world-agent-tool-host",',
    '  "crates/world-investigation",\n  "crates/world-investigation-local",\n  "crates/world-agent-tools",\n  "crates/world-agent-tool-host",\n  "crates/world-agent-tool-stdio",',
    "workspace members",
)
root_manifest.write_text(text)

local_dir = Path("crates/world-investigation-local")
(local_dir / "src").mkdir(parents=True, exist_ok=True)
(local_dir / "tests").mkdir(parents=True, exist_ok=True)
(local_dir / "Cargo.toml").write_text(r'''[package]
name = "world-investigation-local"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
publish = false

[dependencies]
world-builtins = { path = "../world-builtins" }
world-host = { path = "../world-host" }
world-investigation = { path = "../world-investigation" }
world-persistence = { path = "../world-persistence" }
world-projection = { path = "../world-projection" }
world-query = { path = "../world-query" }
''')
(local_dir / "src/lib.rs").write_text(r'''use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;
use world_host::HostError;
use world_investigation::ComparisonQueryExecutor;
use world_persistence::{PersistenceError, WorldArchive};
use world_projection::ProjectionSnapshot;
use world_query::{
    execute_comparison_query_request, EvidenceComparisonQueryRequest,
    EvidenceComparisonQueryResponse, QueryError,
};

pub struct LocalArchiveComparisonExecutor {
    left: ProjectionSnapshot,
    right: ProjectionSnapshot,
}

impl LocalArchiveComparisonExecutor {
    pub fn new(left: ProjectionSnapshot, right: ProjectionSnapshot) -> Self {
        Self { left, right }
    }

    pub fn from_archive_paths(
        left_path: &Path,
        right_path: &Path,
    ) -> Result<Self, LocalArchiveComparisonOpenError> {
        let left_json = fs::read_to_string(left_path)
            .map_err(LocalArchiveComparisonOpenError::ReadLeft)?;
        let right_json = fs::read_to_string(right_path)
            .map_err(LocalArchiveComparisonOpenError::ReadRight)?;
        let left_archive = WorldArchive::from_json(&left_json)
            .map_err(LocalArchiveComparisonOpenError::ParseLeft)?;
        let right_archive = WorldArchive::from_json(&right_json)
            .map_err(LocalArchiveComparisonOpenError::ParseRight)?;
        Self::from_archives(&left_archive, &right_archive)
    }

    pub fn from_archives(
        left_archive: &WorldArchive,
        right_archive: &WorldArchive,
    ) -> Result<Self, LocalArchiveComparisonOpenError> {
        let registry =
            world_builtins::registry().map_err(LocalArchiveComparisonOpenError::Registry)?;
        let left_session = registry
            .open_archive(left_archive)
            .map_err(LocalArchiveComparisonOpenError::OpenLeft)?;
        let right_session = registry
            .open_archive(right_archive)
            .map_err(LocalArchiveComparisonOpenError::OpenRight)?;
        Ok(Self::new(left_session.snapshot(), right_session.snapshot()))
    }

    pub fn left(&self) -> &ProjectionSnapshot {
        &self.left
    }

    pub fn right(&self) -> &ProjectionSnapshot {
        &self.right
    }
}

impl ComparisonQueryExecutor for LocalArchiveComparisonExecutor {
    type Error = QueryError;

    fn execute(
        &mut self,
        request: &EvidenceComparisonQueryRequest,
    ) -> Result<EvidenceComparisonQueryResponse, Self::Error> {
        execute_comparison_query_request(&self.left, &self.right, request)
    }
}

#[derive(Debug)]
pub enum LocalArchiveComparisonOpenError {
    ReadLeft(std::io::Error),
    ReadRight(std::io::Error),
    ParseLeft(PersistenceError),
    ParseRight(PersistenceError),
    Registry(HostError),
    OpenLeft(HostError),
    OpenRight(HostError),
}

impl fmt::Display for LocalArchiveComparisonOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadLeft(error) => write!(f, "failed to read left World archive: {error}"),
            Self::ReadRight(error) => write!(f, "failed to read right World archive: {error}"),
            Self::ParseLeft(error) => write!(f, "failed to parse left World archive: {error}"),
            Self::ParseRight(error) => write!(f, "failed to parse right World archive: {error}"),
            Self::Registry(error) => write!(f, "failed to build local World registry: {error}"),
            Self::OpenLeft(error) => write!(f, "failed to open left World archive: {error}"),
            Self::OpenRight(error) => write!(f, "failed to open right World archive: {error}"),
        }
    }
}

impl Error for LocalArchiveComparisonOpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadLeft(error) | Self::ReadRight(error) => Some(error),
            Self::ParseLeft(error) | Self::ParseRight(error) => Some(error),
            Self::Registry(error) | Self::OpenLeft(error) | Self::OpenRight(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn opens_builtin_archive_pair_and_owns_snapshots() {
        let registry = world_builtins::registry().unwrap();
        let descriptor = registry.descriptors().into_iter().next().unwrap();
        let session = registry.create(&descriptor.pack.id).unwrap();
        let archive = session.archive().unwrap().unwrap();
        let path = temp_world_path("shared");
        fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();

        let executor = LocalArchiveComparisonExecutor::from_archive_paths(&path, &path).unwrap();
        assert_eq!(executor.left().title, executor.right().title);
        assert_eq!(executor.left().world_time, executor.right().world_time);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_errors_preserve_left_right_attribution() {
        let missing_left = temp_world_path("missing-left");
        let missing_right = temp_world_path("missing-right");
        let error = LocalArchiveComparisonExecutor::from_archive_paths(&missing_left, &missing_right)
            .err()
            .unwrap();
        assert!(matches!(error, LocalArchiveComparisonOpenError::ReadLeft(_)));
    }

    fn temp_world_path(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "world-machine-m215-local-{}-{nonce}-{label}.world",
            std::process::id()
        ))
    }
}
''')
(local_dir / "tests/authority_boundary.rs").write_text(r'''#[test]
fn local_investigation_adapter_has_no_agent_or_provider_dependencies() {
    let manifest = include_str!("../Cargo.toml").to_ascii_lowercase();
    for forbidden in [
        "world-agent =",
        "world-agent-tools",
        "world-agent-tool-host",
        "world-pi-rpc",
        "gpui",
        "openai",
        "anthropic",
        "reqwest",
        "hyper",
        "axum",
        "tokio",
        "websocket",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "local investigation adapter contains forbidden dependency {forbidden}"
        );
    }
}
''')

cli_manifest = Path("crates/world-cli/Cargo.toml")
text = cli_manifest.read_text()
text = replace_once(
    text,
    'world-investigation = { path = "../world-investigation" }\n',
    'world-investigation = { path = "../world-investigation" }\nworld-investigation-local = { path = "../world-investigation-local" }\n',
    "world-cli local investigation dependency",
)
cli_manifest.write_text(text)

cli = Path("crates/world-cli/src/main.rs")
text = cli.read_text()
text = replace_once(
    text,
    'use world_investigation::{\n    investigate_first_divergence, ComparisonQueryExecutor, FirstDivergenceInvestigationRequest,\n    InvestigationError,\n};\n',
    'use world_investigation::{\n    investigate_first_divergence, FirstDivergenceInvestigationRequest, InvestigationError,\n};\nuse world_investigation_local::LocalArchiveComparisonExecutor;\n',
    "world-cli investigation imports",
)
old_executor = r'''struct SnapshotComparisonQueryExecutor<'a> {
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

'''
text = replace_once(text, old_executor, "", "remove CLI-local comparison executor")
old_report = r'''fn evidence_investigate_compare_json_report(
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
'''
new_report = r'''fn evidence_investigate_compare_json_report(
    left_path: &Path,
    right_path: &Path,
    request_json: &str,
) -> Result<String, Box<dyn Error>> {
    let mut executor = LocalArchiveComparisonExecutor::from_archive_paths(left_path, right_path)?;
    evidence_investigate_compare_json_with_executor(&mut executor, request_json)
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

fn evidence_investigate_compare_json_from_snapshots(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    request_json: &str,
) -> Result<String, CliError> {
    let mut executor = LocalArchiveComparisonExecutor::new(left.clone(), right.clone());
    evidence_investigate_compare_json_with_executor(&mut executor, request_json)
}

fn evidence_investigate_compare_json_with_executor(
    executor: &mut LocalArchiveComparisonExecutor,
    request_json: &str,
) -> Result<String, CliError> {
    let request = parse_investigation_request(request_json)?;
    let output = match investigate_first_divergence(executor, &request) {
'''
text = replace_once(text, old_report, new_report, "reuse local comparison executor")
cli.write_text(text)

stdio_dir = Path("crates/world-agent-tool-stdio")
(stdio_dir / "src").mkdir(parents=True, exist_ok=True)
(stdio_dir / "tests").mkdir(parents=True, exist_ok=True)
(stdio_dir / "Cargo.toml").write_text(r'''[package]
name = "world-agent-tool-stdio"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
publish = false

[dependencies]
serde_json = "1"
world-agent-tool-host = { path = "../world-agent-tool-host" }
world-agent-tools = { path = "../world-agent-tools" }
world-investigation-local = { path = "../world-investigation-local" }

[dev-dependencies]
world-builtins = { path = "../world-builtins" }
world-projection = { path = "../world-projection" }
''')
(stdio_dir / "src/main.rs").write_text(r'''use serde_json::Value;
use std::env;
use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use world_agent_tool_host::{ReadOnlyJsonToolHost, ReadOnlyJsonToolHostProtocolError};
use world_agent_tools::{FirstDivergenceTool, ReadOnlyJsonToolRegistry};
use world_investigation_local::LocalArchiveComparisonExecutor;

const USAGE: &str = "Usage: world-agent-tool-stdio <left.world> <right.world>\n\nReads one world-machine-readonly-tools JSON request per stdin line and writes one response per line. The World archive pair is fixed at process startup.";

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.as_slice() == ["--help"] || args.as_slice() == ["-h"] {
        println!("{USAGE}");
        return Ok(());
    }
    let [left, right] = args.as_slice() else {
        return Err(Box::new(StdioCliError(format!(
            "invalid arguments\n\n{USAGE}"
        ))));
    };

    let executor = LocalArchiveComparisonExecutor::from_archive_paths(
        &PathBuf::from(left),
        &PathBuf::from(right),
    )?;
    let mut registry = ReadOnlyJsonToolRegistry::new();
    registry.register(FirstDivergenceTool::new(executor))?;
    let mut host = ReadOnlyJsonToolHost::new(registry);
    let stdin = io::stdin();
    let stdout = io::stdout();
    serve_json_lines(
        &mut host,
        BufReader::new(stdin.lock()),
        BufWriter::new(stdout.lock()),
    )?;
    Ok(())
}

fn serve_json_lines<E, R, W>(
    host: &mut ReadOnlyJsonToolHost<E>,
    reader: R,
    mut writer: W,
) -> Result<(), StdioAdapterError>
where
    E: fmt::Display,
    R: BufRead,
    W: Write,
{
    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(StdioAdapterError::Read)?;
        if line.trim().is_empty() {
            continue;
        }
        let request = serde_json::from_str::<Value>(&line).map_err(|source| {
            StdioAdapterError::InvalidJson {
                line: line_number,
                source,
            }
        })?;
        let response = host
            .handle_json(request)
            .map_err(|source| StdioAdapterError::InvalidHostRequest {
                line: line_number,
                source,
            })?;
        serde_json::to_writer(&mut writer, &response).map_err(StdioAdapterError::Serialize)?;
        writer.write_all(b"\n").map_err(StdioAdapterError::Write)?;
        writer.flush().map_err(StdioAdapterError::Write)?;
    }
    Ok(())
}

#[derive(Debug)]
struct StdioCliError(String);

impl fmt::Display for StdioCliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for StdioCliError {}

#[derive(Debug)]
enum StdioAdapterError {
    Read(io::Error),
    InvalidJson {
        line: usize,
        source: serde_json::Error,
    },
    InvalidHostRequest {
        line: usize,
        source: ReadOnlyJsonToolHostProtocolError,
    },
    Serialize(serde_json::Error),
    Write(io::Error),
}

impl fmt::Display for StdioAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(f, "failed to read stdin: {error}"),
            Self::InvalidJson { line, source } => {
                write!(f, "invalid JSON on stdin line {line}: {source}")
            }
            Self::InvalidHostRequest { line, source } => {
                write!(f, "invalid host request on stdin line {line}: {source}")
            }
            Self::Serialize(error) => write!(f, "failed to serialize host response: {error}"),
            Self::Write(error) => write!(f, "failed to write stdout: {error}"),
        }
    }
}

impl Error for StdioAdapterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(error) | Self::Write(error) => Some(error),
            Self::InvalidJson { source, .. } => Some(source),
            Self::InvalidHostRequest { source, .. } => Some(source),
            Self::Serialize(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;
    use std::io::Cursor;
    use world_agent_tools::{JsonToolInvocationError, ReadOnlyJsonTool, ReadOnlyJsonToolDescriptor};

    struct EchoTool;

    impl ReadOnlyJsonTool for EchoTool {
        type ExecutorError = Infallible;

        fn json_descriptor(&self) -> ReadOnlyJsonToolDescriptor {
            ReadOnlyJsonToolDescriptor {
                name: "world.echo",
                description: "Echo input for stdio framing tests.",
                read_only: true,
                input_schema: serde_json::json!({"type": "object"}),
            }
        }

        fn invoke_json(
            &mut self,
            input: Value,
        ) -> Result<Value, JsonToolInvocationError<Self::ExecutorError>> {
            Ok(input)
        }
    }

    #[test]
    fn framing_skips_blank_lines_and_emits_one_json_line_per_request() {
        let mut registry = ReadOnlyJsonToolRegistry::<Infallible>::new();
        registry.register(EchoTool).unwrap();
        let mut host = ReadOnlyJsonToolHost::new(registry);
        let input = Cursor::new(
            b"\n{\"op\":\"invoke\",\"call_id\":\"c1\",\"tool\":\"world.echo\",\"input\":{\"v\":1}}\n{\"op\":\"list-tools\"}\n",
        );
        let mut output = Vec::new();
        serve_json_lines(&mut host, input, &mut output).unwrap();
        let lines = String::from_utf8(output).unwrap().lines().map(str::to_owned).collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        let first: Value = serde_json::from_str(&lines[0]).unwrap();
        let second: Value = serde_json::from_str(&lines[1]).unwrap();
        assert_eq!(first["call_id"], "c1");
        assert_eq!(first["output"]["v"], 1);
        assert_eq!(second["type"], "catalog");
    }
}
''')
(stdio_dir / "tests/stdio_process.rs").write_text(r'''use std::collections::BTreeSet;
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
''')
(stdio_dir / "tests/authority_boundary.rs").write_text(r'''#[test]
fn stdio_adapter_has_only_leaf_transport_authority() {
    let manifest = include_str!("../Cargo.toml");
    let dependencies = manifest
        .split_once("[dependencies]")
        .unwrap()
        .1
        .split_once("[dev-dependencies]")
        .unwrap()
        .0
        .to_ascii_lowercase();

    for forbidden in [
        "world-agent =",
        "world-projection",
        "world-core",
        "world-pi-rpc",
        "gpui",
        "openai",
        "anthropic",
        "reqwest",
        "hyper",
        "axum",
        "tokio",
        "websocket",
    ] {
        assert!(
            !dependencies.contains(forbidden),
            "stdio production dependency boundary contains forbidden token {forbidden}"
        );
    }

    let source = include_str!("../src/main.rs");
    let production = source.split_once("#[cfg(test)]").unwrap().0.to_ascii_lowercase();
    for forbidden in [
        "agentruntime",
        "agentobservation",
        "projectionsnapshot",
        "world_projection",
        "world_core",
        "pi_rpc",
        "openai",
        "anthropic",
        "reqwest",
        "hyper::",
        "axum",
        "tokio",
        "websocket",
    ] {
        assert!(
            !production.contains(forbidden),
            "stdio production source contains forbidden authority token {forbidden}"
        );
    }
}
''')

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M215 Local Stdio Analyst Process

Turn the M214 transport-neutral analyst host into a real long-lived local process without introducing provider SDKs or network authority, and remove the duplicate M210 archive/snapshot comparison executor from `world-cli`.

## Current baseline

M209 owns progressive investigation semantics. M210 exposes them through a CLI-local `ComparisonQueryExecutor`. M211–M213 define read-only tools, JSON dispatch, and deterministic registry semantics. M214 adds a strict transport-neutral external analyst host while keeping `world-pi-rpc` decision-only. The remaining gap is a concrete local process that an Agent adapter can spawn and talk to repeatedly.

## M215 — reusable local executor + JSON-lines stdio

Add `world-investigation-local` as the explicit authority-bearing companion to query-only `world-investigation`:

- own the left/right `ProjectionSnapshot` values;
- load two `.world` archives through built-in Pack restoration;
- implement M209 `ComparisonQueryExecutor` by delegating to existing `world-query` semantics;
- preserve typed side-specific read/parse/open errors;
- contain no Agent/provider/network/UI dependencies.

Refactor M210 `world-cli evidence-investigate-compare` to reuse this adapter rather than maintaining a private snapshot executor.

Add `world-agent-tool-stdio <left.world> <right.world>`:

- bind the archive pair once at process startup, so tool calls cannot choose arbitrary filesystem paths;
- register `world.first-divergence` in the M213 registry;
- read one M214 JSON request per non-empty stdin line;
- write and flush exactly one M214 JSON response line per valid host request;
- keep correlated tool-level errors in-band and continue serving later requests;
- treat malformed JSON or malformed host request envelopes as process-level failures.

## Authority boundary

The concrete local archive/Projection authority lives only in `world-investigation-local`. The stdio process depends on that adapter and the M214/M213 layers but does not directly depend on Projection/Core, in-world `world-agent`/`world-pi-rpc`, provider SDKs, or network/server stacks.

Invocation remains:

`stdio framing -> M214 host -> M213 registry -> M212 JSON tool -> M211 typed tool -> M209 investigation -> world-investigation-local -> world-query`

## Validation

- local adapter opens real built-in archives and preserves left/right failure attribution;
- existing M210 CLI subprocess investigation remains green after refactor;
- one stdio session can list tools then invoke a real first divergence;
- unknown tool returns correlated error and does not terminate the session;
- malformed JSON is a non-zero process-level failure;
- crate-level authority guards for local executor and stdio leaf;
- fmt, boundaries, focused tests/Clippy, full workspace CI, external Pack conformance, and macOS/GPUI/.app validation because workspace/lockfile changes.

## Non-goals

No OpenAI/Anthropic/Pi adapter yet, no MCP/HTTP/WebSocket server, no mutable tools, no in-world AgentRuntime tool injection, no arbitrary archive paths in tool input, and no evidence-query protocol v2.
''')
