from pathlib import Path

cargo = Path("crates/world-cli/Cargo.toml")
text = cargo.read_text()
old = '''world-integrity = { path = "../world-integrity" }
world-persistence = { path = "../world-persistence" }
'''
new = '''world-integrity = { path = "../world-integrity" }
world-persistence = { path = "../world-persistence" }
world-projection = { path = "../world-projection" }
'''
if text.count(old) != 1:
    raise SystemExit(f"expected CLI dependencies once, found {text.count(old)}")
cargo.write_text(text.replace(old, new, 1))

path = Path("crates/world-cli/src/main.rs")
text = path.read_text()
text = text.replace(
    "use world_persistence::{ArchivedEvent, WorldArchive};\n",
    "use world_persistence::{ArchivedEvent, WorldArchive};\nuse world_projection::{ProjectionSnapshot, RelationEndpointRole, SelectionId, StateEvidenceEdge};\n",
    1,
)
text = text.replace(
    "    Why(PathBuf, u64),\n",
    "    Why(PathBuf, u64),\n    Evidence(PathBuf, SelectionId, usize),\n",
    1,
)
text = text.replace(
    "        Command::Why(path, event_id) => println!(\"{}\", why_report(&path, event_id)?),\n",
    "        Command::Why(path, event_id) => println!(\"{}\", why_report(&path, event_id)?),\n        Command::Evidence(path, selection, depth) => {\n            println!(\"{}\", evidence_report(&path, selection, depth)?)\n        }\n",
    1,
)

parse_marker = '''        [command, path, event_id] if command == "why" => {
            let event_id = event_id
                .parse::<u64>()
                .map_err(|_| CliError(format!("invalid event id: {event_id}")))?;
            Ok(Command::Why(PathBuf::from(path), event_id))
        }
'''
parse_insert = parse_marker + '''        [command, path, selection] if command == "evidence" => Ok(Command::Evidence(
            PathBuf::from(path),
            parse_selection_key(selection)?,
            2,
        )),
        [command, path, selection, depth] if command == "evidence" => Ok(Command::Evidence(
            PathBuf::from(path),
            parse_selection_key(selection)?,
            depth
                .parse::<usize>()
                .map_err(|_| CliError(format!("invalid evidence depth: {depth}")))?,
        )),
'''
if text.count(parse_marker) != 1:
    raise SystemExit(f"expected why parse marker once, found {text.count(parse_marker)}")
text = text.replace(parse_marker, parse_insert, 1)

usage_old = '''  world-cli why <file.world> <event-id>\n\\
  world-cli list-packs\n\n\\
'''
usage_new = '''  world-cli why <file.world> <event-id>\n\\
  world-cli evidence <file.world> <selection-key> [depth]\n\\
  world-cli list-packs\n\n\\
'''
if text.count(usage_old) != 1:
    raise SystemExit(f"expected usage command marker once, found {text.count(usage_old)}")
text = text.replace(usage_old, usage_new, 1)
text = text.replace(
    '''why         Trace an event recursively through its archived caused_by graph.\n\\
list-packs  List World Packs this build can create and restore."''',
    '''why         Trace an event recursively through its archived caused_by graph.\n\\
evidence    Print a typed evidence neighborhood around entity-N, relation-N, or event-N.\n\\
list-packs  List World Packs this build can create and restore."''',
    1,
)

load_marker = '''fn load_archive(path: &Path) -> Result<WorldArchive, Box<dyn Error>> {
'''
parse_helper = '''fn parse_selection_key(key: &str) -> Result<SelectionId, CliError> {
    SelectionId::from_stable_key(key)
        .ok_or_else(|| CliError(format!("invalid selection key: {key}")))
}

'''
if text.count(load_marker) != 1:
    raise SystemExit("load_archive marker missing")
text = text.replace(load_marker, parse_helper + load_marker, 1)

pack_marker = '''fn pack_report() -> Result<String, Box<dyn Error>> {
'''
evidence = r'''fn evidence_report(
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
    let neighborhood = snapshot
        .state_evidence_neighborhood(selection, max_depth)
        .ok_or_else(|| CliError(format!("selection is not visible: {}", selection.stable_key())))?;
    let mut lines = vec![
        format!("file: {}", path.display()),
        format!("evidence: {}", selection.stable_key()),
        format!("depth: {max_depth}"),
        format!("nodes: {}", neighborhood.nodes.len()),
    ];
    for node in neighborhood.nodes {
        lines.push(format!(
            "node {} {}",
            node.depth,
            node.selection.stable_key()
        ));
    }
    lines.push(format!("edges: {}", neighborhood.edges.len()));
    for edge in neighborhood.edges {
        lines.push(format_evidence_edge(edge));
    }
    Ok(lines.join("\n"))
}

fn format_evidence_edge(edge: StateEvidenceEdge) -> String {
    match edge {
        StateEvidenceEdge::EntityEvent(evidence) => format!(
            "edge entity-event entity-{} event-{}",
            evidence.entity, evidence.event
        ),
        StateEvidenceEdge::RelationEvent(evidence) => format!(
            "edge relation-event relation-{} event-{}",
            evidence.relation, evidence.event
        ),
        StateEvidenceEdge::EntityRelation(evidence) => {
            let role = match evidence.role {
                RelationEndpointRole::From => "from",
                RelationEndpointRole::To => "to",
            };
            format!(
                "edge entity-relation {role} entity-{} relation-{}",
                evidence.entity, evidence.relation
            )
        }
    }
}

'''
if text.count(pack_marker) != 1:
    raise SystemExit("pack_report marker missing")
text = text.replace(pack_marker, evidence + pack_marker, 1)

old_test = '''        assert_eq!(
            parse_command(["why", "sample.world", "7"]).unwrap(),
            Command::Why(PathBuf::from("sample.world"), 7)
        );
        assert_eq!(parse_command(["list-packs"]).unwrap(), Command::ListPacks);
'''
new_test = '''        assert_eq!(
            parse_command(["why", "sample.world", "7"]).unwrap(),
            Command::Why(PathBuf::from("sample.world"), 7)
        );
        assert_eq!(
            parse_command(["evidence", "sample.world", "relation-5"]).unwrap(),
            Command::Evidence(
                PathBuf::from("sample.world"),
                SelectionId::from_stable_key("relation-5").unwrap(),
                2,
            )
        );
        assert_eq!(
            parse_command(["evidence", "sample.world", "event-9", "0"]).unwrap(),
            Command::Evidence(
                PathBuf::from("sample.world"),
                SelectionId::from_stable_key("event-9").unwrap(),
                0,
            )
        );
        assert_eq!(parse_command(["list-packs"]).unwrap(), Command::ListPacks);
'''
if text.count(old_test) != 1:
    raise SystemExit("command test marker missing")
text = text.replace(old_test, new_test, 1)
text = text.replace(
    '''        assert!(parse_command(["why", "sample.world", "not-a-number"]).is_err());
''',
    '''        assert!(parse_command(["why", "sample.world", "not-a-number"]).is_err());
        assert!(parse_command(["evidence", "sample.world", "entity-07"]).is_err());
        assert!(parse_command(["evidence", "sample.world", "entity-7", "deep"]).is_err());
''',
    1,
)

validate_test_marker = '''    #[test]
    fn pack_report_lists_registered_worlds() {
'''
evidence_tests = r'''    #[test]
    fn evidence_report_exposes_a_machine_stable_depth_zero_neighborhood() {
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
                .or_else(|| {
                    snapshot
                        .inspectors
                        .keys()
                        .copied()
                        .find(|selection| snapshot.state_evidence_neighborhood(*selection, 0).is_some())
                });
            if let Some(root) = root {
                found = Some((snapshot, root));
                break;
            }
        }
        let (snapshot, root) = found.expect("a built-in Pack should expose a visible selection");
        let report = evidence_report_from_snapshot(
            Path::new("builtin.world"),
            &snapshot,
            root,
            0,
        )
        .unwrap();

        assert!(report.contains(&format!("evidence: {}", root.stable_key())));
        assert!(report.contains("depth: 0"));
        assert!(report.contains("nodes: 1"));
        assert!(report.contains(&format!("node 0 {}", root.stable_key())));
        assert!(report.contains("edges: 0"));
    }

    #[test]
    fn evidence_report_rejects_a_well_formed_but_invisible_selection() {
        let registry = world_builtins::registry().unwrap();
        let pack_id = registry.descriptors()[0].pack.id.clone();
        let session = registry.create(&pack_id).unwrap();
        let snapshot = session.snapshot();
        let hidden = SelectionId::from_stable_key("entity-18446744073709551615").unwrap();

        let error = evidence_report_from_snapshot(
            Path::new("builtin.world"),
            &snapshot,
            hidden,
            2,
        )
        .unwrap_err();
        assert!(error.to_string().contains("selection is not visible"));
    }

'''
if text.count(validate_test_marker) != 1:
    raise SystemExit("pack report test marker missing")
text = text.replace(validate_test_marker, evidence_tests + validate_test_marker, 1)
path.write_text(text)
