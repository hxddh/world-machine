from pathlib import Path

path = Path("crates/world-cli/src/main.rs")
text = path.read_text()

old = '''    Evidence(PathBuf, String, usize),
    EvidencePath(PathBuf, String, String),
    EvidenceCompare(PathBuf, PathBuf, String, usize),
    ListPacks,
'''
new = '''    Evidence(PathBuf, String, usize),
    EvidencePath(PathBuf, String, String),
    EvidenceCompare(PathBuf, PathBuf, String, usize),
    EvidenceQuery(PathBuf, String),
    EvidenceCompareQuery(PathBuf, PathBuf, String),
    ListPacks,
'''
if text.count(old) != 1:
    raise SystemExit("Command marker missing")
text = text.replace(old, new, 1)

old = '''        Command::EvidenceCompare(left, right, selection, depth) => {
            println!(
                "{}",
                evidence_compare_report(&left, &right, &selection, depth)?
            )
        }
        Command::ListPacks => println!("{}", pack_report()?),
'''
new = '''        Command::EvidenceCompare(left, right, selection, depth) => {
            println!(
                "{}",
                evidence_compare_report(&left, &right, &selection, depth)?
            )
        }
        Command::EvidenceQuery(path, request) => {
            println!("{}", evidence_query_json_report(&path, &request)?)
        }
        Command::EvidenceCompareQuery(left, right, request) => {
            println!(
                "{}",
                evidence_compare_query_json_report(&left, &right, &request)?
            )
        }
        Command::ListPacks => println!("{}", pack_report()?),
'''
if text.count(old) != 1:
    raise SystemExit("main match marker missing")
text = text.replace(old, new, 1)

marker = '''        [command] if command == "list-packs" => Ok(Command::ListPacks),
'''
insert = '''        [command, path, request] if command == "evidence-query" => Ok(Command::EvidenceQuery(
            PathBuf::from(path),
            request.clone(),
        )),
        [command, left, right, request] if command == "evidence-compare-query" => {
            Ok(Command::EvidenceCompareQuery(
                PathBuf::from(left),
                PathBuf::from(right),
                request.clone(),
            ))
        }
'''
if text.count(marker) != 1:
    raise SystemExit("parse command insertion marker missing")
text = text.replace(marker, insert + marker, 1)

old = '''  world-cli evidence-path <file.world> <from-key> <to-key>\\n\\n\\
  world-cli evidence-compare <left.world> <right.world> <selection-key> [depth]\\n\\n\\
  world-cli list-packs\\n\\n\\
'''
new = '''  world-cli evidence-path <file.world> <from-key> <to-key>\\n\\n\\
  world-cli evidence-compare <left.world> <right.world> <selection-key> [depth]\\n\\n\\
  world-cli evidence-query <file.world> '<request-json>'\\n\\n\\
  world-cli evidence-compare-query <left.world> <right.world> '<request-json>'\\n\\n\\
  world-cli list-packs\\n\\n\\
'''
if text.count(old) != 1:
    raise SystemExit("usage command marker missing")
text = text.replace(old, new, 1)

old = '''evidence-path  Print the typed shortest evidence path between two selections.\\n\\
evidence-compare  Compare a typed evidence neighborhood between two World archives.\\n\\
list-packs  List World Packs this build can create and restore."
'''
new = '''evidence-path  Print the typed shortest evidence path between two selections.\\n\\
evidence-compare  Compare a typed evidence neighborhood between two World archives.\\n\\
evidence-query  Execute an EvidenceQueryRequest JSON document and emit a JSON status envelope.\\n\\
evidence-compare-query  Execute an EvidenceComparisonRequest JSON document and emit a JSON status envelope.\\n\\
list-packs  List World Packs this build can create and restore."
'''
if text.count(old) != 1:
    raise SystemExit("usage description marker missing")
text = text.replace(old, new, 1)

marker = '''fn format_evidence_comparison(
'''
functions = '''fn evidence_query_json_report(
    path: &Path,
    request_json: &str,
) -> Result<String, Box<dyn Error>> {
    let archive = load_archive(path)?;
    let registry = world_builtins::registry()?;
    let session = registry.open_archive(&archive)?;
    let snapshot = session.snapshot();
    evidence_query_json_from_snapshot(&snapshot, request_json)
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

fn evidence_query_json_from_snapshot(
    snapshot: &ProjectionSnapshot,
    request_json: &str,
) -> Result<String, CliError> {
    let request: EvidenceQueryRequest = serde_json::from_str(request_json)
        .map_err(|error| CliError(format!("invalid evidence query JSON: {error}")))?;
    let output = match execute_query(snapshot, &request) {
        Ok(response) => serde_json::json!({
            "status": "ok",
            "response": response,
        }),
        Err(error) => serde_json::json!({
            "status": "error",
            "error": error,
        }),
    };
    serde_json::to_string(&output)
        .map_err(|error| CliError(format!("failed to serialize evidence query JSON: {error}")))
}

fn evidence_compare_query_json_report(
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
    evidence_compare_query_json_from_snapshots(&left, &right, request_json)
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

fn evidence_compare_query_json_from_snapshots(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    request_json: &str,
) -> Result<String, CliError> {
    let request: EvidenceComparisonRequest = serde_json::from_str(request_json)
        .map_err(|error| CliError(format!("invalid evidence comparison query JSON: {error}")))?;
    let output = match execute_comparison_query(left, right, &request) {
        Ok(response) => serde_json::json!({
            "status": "ok",
            "response": response,
        }),
        Err(error) => serde_json::json!({
            "status": "error",
            "error": error,
        }),
    };
    serde_json::to_string(&output).map_err(|error| {
        CliError(format!(
            "failed to serialize evidence comparison query JSON: {error}"
        ))
    })
}

'''
if text.count(marker) != 1:
    raise SystemExit("JSON function insertion marker missing")
text = text.replace(marker, functions + marker, 1)

# Extend command parsing tests with the machine-facing JSON commands.
marker = '''        assert_eq!(parse_command(["list-packs"]).unwrap(), Command::ListPacks);
'''
test_parse = '''        let query_json = r#"{\"query\":\"neighborhood\",\"root\":\"entity-1\",\"max_depth\":2}"#;
        assert_eq!(
            parse_command(["evidence-query", "sample.world", query_json]).unwrap(),
            Command::EvidenceQuery(PathBuf::from("sample.world"), query_json.into())
        );
        let comparison_json = r#"{\"root\":\"entity-1\",\"max_depth\":2}"#;
        assert_eq!(
            parse_command([
                "evidence-compare-query",
                "left.world",
                "right.world",
                comparison_json,
            ])
            .unwrap(),
            Command::EvidenceCompareQuery(
                PathBuf::from("left.world"),
                PathBuf::from("right.world"),
                comparison_json.into(),
            )
        );
'''
if text.count(marker) != 1:
    raise SystemExit("parser test insertion marker missing")
text = text.replace(marker, test_parse + marker, 1)

marker = '''    #[test]
    fn evidence_report_delegates_selection_key_validation_to_world_query() {
'''
json_tests = '''    #[test]
    fn evidence_query_json_executes_neighborhood_and_shortest_path_requests() {
        let (snapshot, root) = first_visible_snapshot_and_key();

        let neighborhood_request = serde_json::to_string(&EvidenceQueryRequest::Neighborhood {
            root: root.clone(),
            max_depth: 0,
        })
        .unwrap();
        let neighborhood_json =
            evidence_query_json_from_snapshot(&snapshot, &neighborhood_request).unwrap();
        let neighborhood: serde_json::Value = serde_json::from_str(&neighborhood_json).unwrap();
        assert_eq!(neighborhood["status"], "ok");
        let response: EvidenceQueryResponse =
            serde_json::from_value(neighborhood["response"].clone()).unwrap();
        let EvidenceQueryResponse::Neighborhood { value } = response else {
            panic!("expected neighborhood response")
        };
        assert_eq!(value.root, root);
        assert_eq!(value.max_depth, 0);

        let path_request = serde_json::to_string(&EvidenceQueryRequest::ShortestPath {
            from: root.clone(),
            to: root.clone(),
        })
        .unwrap();
        let path_json = evidence_query_json_from_snapshot(&snapshot, &path_request).unwrap();
        let path: serde_json::Value = serde_json::from_str(&path_json).unwrap();
        assert_eq!(path["status"], "ok");
        let response: EvidenceQueryResponse =
            serde_json::from_value(path["response"].clone()).unwrap();
        let EvidenceQueryResponse::ShortestPath { value } = response else {
            panic!("expected shortest-path response")
        };
        assert_eq!(value.from, root);
        assert_eq!(value.to, root);
        assert!(value.steps.is_empty());
    }

    #[test]
    fn evidence_query_json_distinguishes_semantic_errors_from_malformed_json() {
        let (snapshot, _) = first_visible_snapshot_and_key();
        let semantic_json = evidence_query_json_from_snapshot(
            &snapshot,
            r#"{\"query\":\"neighborhood\",\"root\":\"entity-07\",\"max_depth\":2}"#,
        )
        .unwrap();
        let semantic: serde_json::Value = serde_json::from_str(&semantic_json).unwrap();
        assert_eq!(semantic["status"], "error");
        let error: world_query::QueryError =
            serde_json::from_value(semantic["error"].clone()).unwrap();
        assert_eq!(
            error,
            world_query::QueryError::InvalidSelectionKey("entity-07".into())
        );

        let malformed = evidence_query_json_from_snapshot(&snapshot, "{not-json").unwrap_err();
        assert!(malformed
            .to_string()
            .contains("invalid evidence query JSON"));
    }

    #[test]
    fn evidence_compare_query_json_returns_typed_comparison_result() {
        let (snapshot, root) = first_visible_snapshot_and_key();
        let request = serde_json::to_string(&EvidenceComparisonRequest {
            root: root.clone(),
            max_depth: 1,
        })
        .unwrap();
        let output =
            evidence_compare_query_json_from_snapshots(&snapshot, &snapshot, &request).unwrap();
        let output: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(output["status"], "ok");
        let comparison: EvidenceComparisonResult =
            serde_json::from_value(output["response"].clone()).unwrap();
        assert_eq!(comparison.root, root);
        assert!(comparison.identical);
    }

'''
if text.count(marker) != 1:
    raise SystemExit("JSON tests insertion marker missing")
text = text.replace(marker, json_tests + marker, 1)

marker = '''    fn empty_archive(pack_id: &str, world_time: u64) -> WorldArchive {
'''
helper = '''    fn first_visible_snapshot_and_key() -> (ProjectionSnapshot, String) {
        let registry = world_builtins::registry().unwrap();
        for descriptor in registry.descriptors() {
            let session = registry.create(&descriptor.pack.id).unwrap();
            let snapshot = session.snapshot();
            let root = snapshot
                .timeline
                .items
                .first()
                .map(|item| item.id)
                .or_else(|| snapshot.inspectors.keys().copied().next());
            if let Some(root) = root {
                return (snapshot, root.stable_key());
            }
        }
        panic!("a built-in Pack should expose a visible selection")
    }

'''
if text.count(marker) != 1:
    raise SystemExit("test helper insertion marker missing")
text = text.replace(marker, helper + marker, 1)

path.write_text(text)
