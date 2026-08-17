from pathlib import Path

path = Path("crates/world-cli/src/main.rs")
text = path.read_text()

old = '''use world_projection::{ProjectionSnapshot, SelectionId};
use world_query::{
    query_neighborhood, query_neighborhood_comparison, query_shortest_path, Difference,
    EvidenceComparisonResult, EvidenceEdge,
};
'''
new = '''use world_projection::ProjectionSnapshot;
use world_query::{
    execute_comparison_query, execute_query, Difference, EvidenceComparisonRequest,
    EvidenceComparisonResult, EvidenceEdge, EvidenceQueryRequest, EvidenceQueryResponse,
};
'''
if text.count(old) != 1:
    raise SystemExit("import block marker missing")
text = text.replace(old, new, 1)

old = '''    Evidence(PathBuf, SelectionId, usize),
    EvidencePath(PathBuf, SelectionId, SelectionId),
    EvidenceCompare(PathBuf, PathBuf, SelectionId, usize),
'''
new = '''    Evidence(PathBuf, String, usize),
    EvidencePath(PathBuf, String, String),
    EvidenceCompare(PathBuf, PathBuf, String, usize),
'''
if text.count(old) != 1:
    raise SystemExit("Command evidence variants marker missing")
text = text.replace(old, new, 1)

text = text.replace('println!("{}", evidence_report(&path, selection, depth)?)', 'println!("{}", evidence_report(&path, &selection, depth)?)')
text = text.replace('println!("{}", evidence_path_report(&path, from, to)?)', 'println!("{}", evidence_path_report(&path, &from, &to)?)')
text = text.replace('evidence_compare_report(&left, &right, selection, depth)?', 'evidence_compare_report(&left, &right, &selection, depth)?')

for old, new, expected in [
    ('parse_selection_key(selection)?', 'selection.clone()', 4),
    ('parse_selection_key(from)?', 'from.clone()', 1),
    ('parse_selection_key(to)?', 'to.clone()', 1),
]:
    if text.count(old) != expected:
        raise SystemExit(f"unexpected count for {old}: {text.count(old)}")
    text = text.replace(old, new)

old = '''fn parse_selection_key(key: &str) -> Result<SelectionId, CliError> {
    SelectionId::from_stable_key(key)
        .ok_or_else(|| CliError(format!("invalid selection key: {key}")))
}

'''
if text.count(old) != 1:
    raise SystemExit("parse_selection_key helper marker missing")
text = text.replace(old, '', 1)

old = '''fn evidence_report(
    path: &Path,
    selection: SelectionId,
    max_depth: usize,
) -> Result<String, Box<dyn Error>> {
    let archive = load_archive(path)?;
    let registry = world_builtins::registry()?;
    let session = registry.open_archive(&archive)?;
    let snapshot = session.snapshot();
    evidence_report_from_snapshot(path, &snapshot, selection, max_depth)
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

fn evidence_report_from_snapshot(
    path: &Path,
    snapshot: &ProjectionSnapshot,
    selection: SelectionId,
    max_depth: usize,
) -> Result<String, CliError> {
    let neighborhood = query_neighborhood(snapshot, selection, max_depth)
        .map_err(|error| CliError(error.to_string()))?;
'''
new = '''fn evidence_report(
    path: &Path,
    selection: &str,
    max_depth: usize,
) -> Result<String, Box<dyn Error>> {
    let archive = load_archive(path)?;
    let registry = world_builtins::registry()?;
    let session = registry.open_archive(&archive)?;
    let snapshot = session.snapshot();
    evidence_report_from_snapshot(path, &snapshot, selection, max_depth)
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

fn evidence_report_from_snapshot(
    path: &Path,
    snapshot: &ProjectionSnapshot,
    selection: &str,
    max_depth: usize,
) -> Result<String, CliError> {
    let request = EvidenceQueryRequest::Neighborhood {
        root: selection.to_owned(),
        max_depth,
    };
    let response = execute_query(snapshot, &request).map_err(|error| CliError(error.to_string()))?;
    let EvidenceQueryResponse::Neighborhood {
        value: neighborhood,
    } = response
    else {
        unreachable!("neighborhood request returned a different response variant")
    };
'''
if text.count(old) != 1:
    raise SystemExit("evidence report block marker missing")
text = text.replace(old, new, 1)

old = '''fn evidence_path_report(
    path: &Path,
    from: SelectionId,
    to: SelectionId,
) -> Result<String, Box<dyn Error>> {
    let archive = load_archive(path)?;
    let registry = world_builtins::registry()?;
    let session = registry.open_archive(&archive)?;
    let snapshot = session.snapshot();
    evidence_path_report_from_snapshot(path, &snapshot, from, to)
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

fn evidence_path_report_from_snapshot(
    path: &Path,
    snapshot: &ProjectionSnapshot,
    from: SelectionId,
    to: SelectionId,
) -> Result<String, CliError> {
    let result =
        query_shortest_path(snapshot, from, to).map_err(|error| CliError(error.to_string()))?;
'''
new = '''fn evidence_path_report(
    path: &Path,
    from: &str,
    to: &str,
) -> Result<String, Box<dyn Error>> {
    let archive = load_archive(path)?;
    let registry = world_builtins::registry()?;
    let session = registry.open_archive(&archive)?;
    let snapshot = session.snapshot();
    evidence_path_report_from_snapshot(path, &snapshot, from, to)
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

fn evidence_path_report_from_snapshot(
    path: &Path,
    snapshot: &ProjectionSnapshot,
    from: &str,
    to: &str,
) -> Result<String, CliError> {
    let request = EvidenceQueryRequest::ShortestPath {
        from: from.to_owned(),
        to: to.to_owned(),
    };
    let response = execute_query(snapshot, &request).map_err(|error| CliError(error.to_string()))?;
    let EvidenceQueryResponse::ShortestPath { value: result } = response else {
        unreachable!("shortest-path request returned a different response variant")
    };
'''
if text.count(old) != 1:
    raise SystemExit("evidence path report block marker missing")
text = text.replace(old, new, 1)

old = '''fn evidence_compare_report(
    left_path: &Path,
    right_path: &Path,
    selection: SelectionId,
    max_depth: usize,
) -> Result<String, Box<dyn Error>> {
    let left_archive = load_archive(left_path)?;
    let right_archive = load_archive(right_path)?;
    let registry = world_builtins::registry()?;
    let left_session = registry.open_archive(&left_archive)?;
    let right_session = registry.open_archive(&right_archive)?;
    let left = left_session.snapshot();
    let right = right_session.snapshot();
    evidence_compare_report_from_snapshots(
        left_path, right_path, &left, &right, selection, max_depth,
    )
    .map_err(|error| Box::new(error) as Box<dyn Error>)
}

fn evidence_compare_report_from_snapshots(
    left_path: &Path,
    right_path: &Path,
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    selection: SelectionId,
    max_depth: usize,
) -> Result<String, CliError> {
    let comparison = query_neighborhood_comparison(left, right, selection, max_depth)
        .map_err(|error| CliError(error.to_string()))?;
'''
new = '''fn evidence_compare_report(
    left_path: &Path,
    right_path: &Path,
    selection: &str,
    max_depth: usize,
) -> Result<String, Box<dyn Error>> {
    let left_archive = load_archive(left_path)?;
    let right_archive = load_archive(right_path)?;
    let registry = world_builtins::registry()?;
    let left_session = registry.open_archive(&left_archive)?;
    let right_session = registry.open_archive(&right_archive)?;
    let left = left_session.snapshot();
    let right = right_session.snapshot();
    evidence_compare_report_from_snapshots(
        left_path, right_path, &left, &right, selection, max_depth,
    )
    .map_err(|error| Box::new(error) as Box<dyn Error>)
}

fn evidence_compare_report_from_snapshots(
    left_path: &Path,
    right_path: &Path,
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    selection: &str,
    max_depth: usize,
) -> Result<String, CliError> {
    let request = EvidenceComparisonRequest {
        root: selection.to_owned(),
        max_depth,
    };
    let comparison = execute_comparison_query(left, right, &request)
        .map_err(|error| CliError(error.to_string()))?;
'''
if text.count(old) != 1:
    raise SystemExit("evidence compare report block marker missing")
text = text.replace(old, new, 1)

# Command parser expectations now retain raw stable-key strings and delegate semantic validation.
replacements = {
    'SelectionId::from_stable_key("relation-5").unwrap()': '"relation-5".into()',
    'SelectionId::from_stable_key("event-9").unwrap()': '"event-9".into()',
    'SelectionId::from_stable_key("entity-1").unwrap()': '"entity-1".into()',
}
for old, new in replacements.items():
    text = text.replace(old, new)

old = '''        assert!(parse_command(["evidence", "sample.world", "entity-07"]).is_err());
        assert!(parse_command(["evidence", "sample.world", "entity-7", "deep"]).is_err());
        assert!(parse_command(["evidence-path", "sample.world", "entity-07", "event-9"]).is_err());
        assert!(parse_command(["evidence-path", "sample.world", "entity-7", "event-09"]).is_err());
        assert!(
            parse_command(["evidence-compare", "left.world", "right.world", "entity-07"]).is_err()
        );
'''
new = '''        assert_eq!(
            parse_command(["evidence", "sample.world", "entity-07"]).unwrap(),
            Command::Evidence(PathBuf::from("sample.world"), "entity-07".into(), 2)
        );
        assert!(parse_command(["evidence", "sample.world", "entity-7", "deep"]).is_err());
        assert_eq!(
            parse_command(["evidence-path", "sample.world", "entity-07", "event-09"]).unwrap(),
            Command::EvidencePath(
                PathBuf::from("sample.world"),
                "entity-07".into(),
                "event-09".into(),
            )
        );
        assert_eq!(
            parse_command(["evidence-compare", "left.world", "right.world", "entity-07"]).unwrap(),
            Command::EvidenceCompare(
                PathBuf::from("left.world"),
                PathBuf::from("right.world"),
                "entity-07".into(),
                2,
            )
        );
'''
if text.count(old) != 1:
    raise SystemExit("invalid selection parser assertions marker missing")
text = text.replace(old, new, 1)

# Route report tests through stable-key strings instead of typed SelectionId arguments.
text = text.replace(
    'evidence_report_from_snapshot(Path::new("builtin.world"), &snapshot, root, 0).unwrap();',
    'evidence_report_from_snapshot(Path::new("builtin.world"), &snapshot, &root.stable_key(), 0)\n                .unwrap();',
)
text = text.replace(
    'let hidden = SelectionId::from_stable_key("entity-18446744073709551615").unwrap();',
    'let hidden = "entity-18446744073709551615";',
)
text = text.replace(
    'evidence_report_from_snapshot(Path::new("builtin.world"), &snapshot, hidden, 2)',
    'evidence_report_from_snapshot(Path::new("builtin.world"), &snapshot, hidden, 2)',
)
text = text.replace(
    'evidence_path_report_from_snapshot(Path::new("builtin.world"), &snapshot, root, root)\n                .unwrap();',
    'evidence_path_report_from_snapshot(\n                Path::new("builtin.world"),\n                &snapshot,\n                &root.stable_key(),\n                &root.stable_key(),\n            )\n            .unwrap();',
)
text = text.replace(
    'evidence_path_report_from_snapshot(Path::new("builtin.world"), &snapshot, from, to)\n                .unwrap();',
    'evidence_path_report_from_snapshot(\n                Path::new("builtin.world"),\n                &snapshot,\n                &from.stable_key(),\n                &to.stable_key(),\n            )\n            .unwrap();',
)
text = text.replace(
    'evidence_path_report_from_snapshot(Path::new("builtin.world"), &snapshot, hidden, root)\n                .unwrap_err();',
    'evidence_path_report_from_snapshot(\n                Path::new("builtin.world"),\n                &snapshot,\n                hidden,\n                &root.stable_key(),\n            )\n            .unwrap_err();',
)
text = text.replace(
    '''            &snapshot,
            &snapshot,
            root,
            2,
''',
    '''            &snapshot,
            &snapshot,
            &root.stable_key(),
            2,
''',
    1,
)
text = text.replace(
    '''            &snapshot,
            &snapshot,
            hidden,
            2,
''',
    '''            &snapshot,
            &snapshot,
            hidden,
            2,
''',
    1,
)

# Add a boundary regression: malformed keys parse as CLI args but fail in world-query execution.
marker = '''    #[test]
    fn evidence_report_exposes_a_machine_stable_depth_zero_neighborhood() {
'''
test = '''    #[test]
    fn evidence_report_delegates_selection_key_validation_to_world_query() {
        let registry = world_builtins::registry().unwrap();
        let pack_id = registry.descriptors()[0].pack.id.clone();
        let session = registry.create(&pack_id).unwrap();
        let snapshot = session.snapshot();

        let error = evidence_report_from_snapshot(
            Path::new("builtin.world"),
            &snapshot,
            "entity-07",
            2,
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "invalid selection key: entity-07");
    }

'''
if text.count(marker) != 1:
    raise SystemExit("evidence test insertion marker missing")
text = text.replace(marker, test + marker, 1)

path.write_text(text)
