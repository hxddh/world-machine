from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing marker for {label}")
    return text.replace(old, new, 1)

manifest = Path("crates/world-cli/Cargo.toml")
text = manifest.read_text()
text = replace_once(
    text,
    'world-investigation = { path = "../world-investigation" }\n',
    'world-investigation = { path = "../world-investigation" }\nworld-investigation-local = { path = "../world-investigation-local" }\n',
    "CLI local investigation dependency",
)
manifest.write_text(text)

main = Path("crates/world-cli/src/main.rs")
text = main.read_text()
text = replace_once(
    text,
    'use world_investigation::{\n    investigate_first_divergence, ComparisonQueryExecutor, FirstDivergenceInvestigationRequest,\n    InvestigationError,\n};\n',
    'use world_investigation::{\n    investigate_first_divergence, FirstDivergenceInvestigationRequest, InvestigationError,\n};\nuse world_investigation_local::LocalArchiveComparisonExecutor;\n',
    "CLI investigation imports",
)
text = replace_once(
    text,
    '''struct SnapshotComparisonQueryExecutor<'a> {\n    left: &'a ProjectionSnapshot,\n    right: &'a ProjectionSnapshot,\n}\n\nimpl ComparisonQueryExecutor for SnapshotComparisonQueryExecutor<'_> {\n    type Error = world_query::QueryError;\n\n    fn execute(\n        &mut self,\n        request: &EvidenceComparisonQueryRequest,\n    ) -> Result<world_query::EvidenceComparisonQueryResponse, Self::Error> {\n        execute_comparison_query_request(self.left, self.right, request)\n    }\n}\n\n''',
    '',
    "remove CLI-private comparison executor",
)
old = '''fn evidence_investigate_compare_json_report(\n    left_path: &Path,\n    right_path: &Path,\n    request_json: &str,\n) -> Result<String, Box<dyn Error>> {\n    let left_archive = load_archive(left_path)?;\n    let right_archive = load_archive(right_path)?;\n    let registry = world_builtins::registry()?;\n    let left_session = registry.open_archive(&left_archive)?;\n    let right_session = registry.open_archive(&right_archive)?;\n    let left = left_session.snapshot();\n    let right = right_session.snapshot();\n    evidence_investigate_compare_json_from_snapshots(&left, &right, request_json)\n        .map_err(|error| Box::new(error) as Box<dyn Error>)\n}\n\nfn evidence_investigate_compare_json_from_snapshots(\n    left: &ProjectionSnapshot,\n    right: &ProjectionSnapshot,\n    request_json: &str,\n) -> Result<String, CliError> {\n    let request = parse_investigation_request(request_json)?;\n    let mut executor = SnapshotComparisonQueryExecutor { left, right };\n    let output = match investigate_first_divergence(&mut executor, &request) {\n'''
new = '''fn evidence_investigate_compare_json_report(\n    left_path: &Path,\n    right_path: &Path,\n    request_json: &str,\n) -> Result<String, Box<dyn Error>> {\n    let mut executor = LocalArchiveComparisonExecutor::from_archive_paths(left_path, right_path)?;\n    evidence_investigate_compare_json_with_executor(&mut executor, request_json)\n        .map_err(|error| Box::new(error) as Box<dyn Error>)\n}\n\nfn evidence_investigate_compare_json_from_snapshots(\n    left: &ProjectionSnapshot,\n    right: &ProjectionSnapshot,\n    request_json: &str,\n) -> Result<String, CliError> {\n    let mut executor = LocalArchiveComparisonExecutor::new(left.clone(), right.clone());\n    evidence_investigate_compare_json_with_executor(&mut executor, request_json)\n}\n\nfn evidence_investigate_compare_json_with_executor(\n    executor: &mut LocalArchiveComparisonExecutor,\n    request_json: &str,\n) -> Result<String, CliError> {\n    let request = parse_investigation_request(request_json)?;\n    let output = match investigate_first_divergence(executor, &request) {\n'''
text = replace_once(text, old, new, "CLI investigation report refactor")
main.write_text(text)
