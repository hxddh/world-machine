from pathlib import Path

cargo = Path("crates/world-cli/Cargo.toml")
text = cargo.read_text()
old = 'world-builtins = { path = "../world-builtins" }\n'
new = old + 'world-compare = { path = "../world-compare" }\n'
if text.count(old) != 1:
    raise SystemExit(f"expected world-builtins dependency once, found {text.count(old)}")
cargo.write_text(text.replace(old, new, 1))

path = Path("crates/world-cli/src/main.rs")
text = path.read_text()
text = text.replace(
    "use world_integrity::{check_archive, ArchiveIntegrityError};\n",
    "use world_compare::{compare_evidence_neighborhoods, DifferenceKind, EvidenceNeighborhoodComparison};\nuse world_integrity::{check_archive, ArchiveIntegrityError};\n",
    1,
)
text = text.replace(
    "    EvidencePath(PathBuf, SelectionId, SelectionId),\n    ListPacks,\n",
    "    EvidencePath(PathBuf, SelectionId, SelectionId),\n    EvidenceCompare(PathBuf, PathBuf, SelectionId, usize),\n    ListPacks,\n",
    1,
)
text = text.replace(
    """        Command::EvidencePath(path, from, to) => {
            println!("{}", evidence_path_report(&path, from, to)?)
        }
        Command::ListPacks => println!("{}", pack_report()?),
""",
    """        Command::EvidencePath(path, from, to) => {
            println!("{}", evidence_path_report(&path, from, to)?)
        }
        Command::EvidenceCompare(left, right, selection, depth) => {
            println!("{}", evidence_compare_report(&left, &right, selection, depth)?)
        }
        Command::ListPacks => println!("{}", pack_report()?),
""",
    1,
)
text = text.replace(
    """        [command, path, from, to] if command == "evidence-path" => Ok(Command::EvidencePath(
            PathBuf::from(path),
            parse_selection_key(from)?,
            parse_selection_key(to)?,
        )),
        [command] if command == "list-packs" => Ok(Command::ListPacks),
""",
    """        [command, path, from, to] if command == "evidence-path" => Ok(Command::EvidencePath(
            PathBuf::from(path),
            parse_selection_key(from)?,
            parse_selection_key(to)?,
        )),
        [command, left, right, selection] if command == "evidence-compare" => {
            Ok(Command::EvidenceCompare(
                PathBuf::from(left),
                PathBuf::from(right),
                parse_selection_key(selection)?,
                2,
            ))
        }
        [command, left, right, selection, depth] if command == "evidence-compare" => {
            Ok(Command::EvidenceCompare(
                PathBuf::from(left),
                PathBuf::from(right),
                parse_selection_key(selection)?,
                depth
                    .parse::<usize>()
                    .map_err(|_| CliError(format!("invalid evidence depth: {depth}")))?,
            ))
        }
        [command] if command == "list-packs" => Ok(Command::ListPacks),
""",
    1,
)

lines = text.splitlines(keepends=True)
usage = [i for i, line in enumerate(lines) if '  world-cli evidence-path <file.world> <from-key> <to-key>' in line]
if len(usage) != 1:
    raise SystemExit(f"expected one evidence-path usage line, found {len(usage)}")
i = usage[0] + 1
lines.insert(i, lines[i - 1].replace('evidence-path <file.world> <from-key> <to-key>', 'evidence-compare <left.world> <right.world> <selection-key> [depth]'))
text = ''.join(lines)
text = text.replace(
    'evidence-path  Print the typed shortest evidence path between two selections.\\n\\\n',
    'evidence-path  Print the typed shortest evidence path between two selections.\\n\\\n'
    'evidence-compare  Compare a typed evidence neighborhood between two World archives.\\n\\\n',
    1,
)

marker = """fn pack_report() -> Result<String, Box<dyn Error>> {
"""
functions = r'''fn evidence_compare_report(
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
        left_path,
        right_path,
        &left,
        &right,
        selection,
        max_depth,
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
    let comparison = compare_evidence_neighborhoods(left, right, selection, max_depth).ok_or_else(|| {
        CliError(format!(
            "selection is not visible in either world: {}",
            selection.stable_key()
        ))
    })?;
    Ok(format_evidence_comparison(
        left_path,
        right_path,
        &comparison,
    ))
}

fn format_evidence_comparison(
    left_path: &Path,
    right_path: &Path,
    comparison: &EvidenceNeighborhoodComparison,
) -> String {
    let mut lines = vec![
        format!("left: {}", left_path.display()),
        format!("right: {}", right_path.display()),
        format!("evidence-compare: {}", comparison.root.stable_key()),
        format!("depth: {}", comparison.max_depth),
        format!("identical: {}", comparison.is_identical()),
        format!("node-changes: {}", comparison.nodes.len()),
    ];
    for node in &comparison.nodes {
        lines.push(format!(
            "node {} {} {} {}",
            difference_kind_key(node.kind),
            node.selection.stable_key(),
            optional_depth(node.left_depth),
            optional_depth(node.right_depth)
        ));
    }
    lines.push(format!("left-only-edges: {}", comparison.edges.left_only.len()));
    for edge in &comparison.edges.left_only {
        lines.push(format!("left-{}", format_evidence_edge(*edge)));
    }
    lines.push(format!("right-only-edges: {}", comparison.edges.right_only.len()));
    for edge in &comparison.edges.right_only {
        lines.push(format!("right-{}", format_evidence_edge(*edge)));
    }
    lines.join("\n")
}

fn difference_kind_key(kind: DifferenceKind) -> &'static str {
    match kind {
        DifferenceKind::LeftOnly => "left-only",
        DifferenceKind::RightOnly => "right-only",
        DifferenceKind::Changed => "changed",
    }
}

fn optional_depth(depth: Option<usize>) -> String {
    depth.map_or_else(|| "-".into(), |depth| depth.to_string())
}

'''
if text.count(marker) != 1:
    raise SystemExit("pack_report marker missing")
text = text.replace(marker, functions + marker, 1)

parse_test_marker = '        assert_eq!(parse_command(["list-packs"]).unwrap(), Command::ListPacks);\n'
parse_tests = '''        assert_eq!(
            parse_command(["evidence-compare", "left.world", "right.world", "relation-5"]).unwrap(),
            Command::EvidenceCompare(
                PathBuf::from("left.world"),
                PathBuf::from("right.world"),
                SelectionId::from_stable_key("relation-5").unwrap(),
                2,
            )
        );
        assert_eq!(
            parse_command(["evidence-compare", "left.world", "right.world", "event-9", "3"]).unwrap(),
            Command::EvidenceCompare(
                PathBuf::from("left.world"),
                PathBuf::from("right.world"),
                SelectionId::from_stable_key("event-9").unwrap(),
                3,
            )
        );
'''
if text.count(parse_test_marker) != 1:
    raise SystemExit("list-packs parse test marker missing")
text = text.replace(parse_test_marker, parse_tests + parse_test_marker, 1)
text = text.replace(
    '        assert!(parse_command(["evidence-path", "sample.world", "entity-7", "event-09"]).is_err());\n',
    '        assert!(parse_command(["evidence-path", "sample.world", "entity-7", "event-09"]).is_err());\n'
    '        assert!(parse_command(["evidence-compare", "left.world", "right.world", "entity-07"]).is_err());\n'
    '        assert!(parse_command(["evidence-compare", "left.world", "right.world", "entity-7", "deep"]).is_err());\n',
    1,
)

marker = """    #[test]
    fn pack_report_lists_registered_worlds() {
"""
tests = r'''    #[test]
    fn evidence_compare_report_is_stable_for_identical_snapshot() {
        let registry = world_builtins::registry().unwrap();
        let mut found = None;
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
                found = Some((snapshot, root));
                break;
            }
        }
        let (snapshot, root) = found.expect("a built-in Pack should expose a visible selection");
        let report = evidence_compare_report_from_snapshots(
            Path::new("left.world"),
            Path::new("right.world"),
            &snapshot,
            &snapshot,
            root,
            2,
        )
        .unwrap();

        assert!(report.contains(&format!("evidence-compare: {}", root.stable_key())));
        assert!(report.contains("identical: true"));
        assert!(report.contains("node-changes: 0"));
        assert!(report.contains("left-only-edges: 0"));
        assert!(report.contains("right-only-edges: 0"));
    }

    #[test]
    fn evidence_compare_report_rejects_root_hidden_on_both_sides() {
        let registry = world_builtins::registry().unwrap();
        let pack_id = registry.descriptors()[0].pack.id.clone();
        let session = registry.create(&pack_id).unwrap();
        let snapshot = session.snapshot();
        let hidden = SelectionId::from_stable_key("entity-18446744073709551615").unwrap();

        let error = evidence_compare_report_from_snapshots(
            Path::new("left.world"),
            Path::new("right.world"),
            &snapshot,
            &snapshot,
            hidden,
            2,
        )
        .unwrap_err();
        assert!(error.to_string().contains("not visible in either world"));
    }

'''
if text.count(marker) != 1:
    raise SystemExit("pack report test marker missing")
text = text.replace(marker, tests + marker, 1)
path.write_text(text)
