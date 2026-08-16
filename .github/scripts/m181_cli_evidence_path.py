from pathlib import Path

path = Path("crates/world-cli/src/main.rs")
text = path.read_text()

old = """    Evidence(PathBuf, SelectionId, usize),
    ListPacks,
"""
new = """    Evidence(PathBuf, SelectionId, usize),
    EvidencePath(PathBuf, SelectionId, SelectionId),
    ListPacks,
"""
if text.count(old) != 1:
    raise SystemExit(f"expected command variant marker once, found {text.count(old)}")
text = text.replace(old, new, 1)

old = """        Command::Evidence(path, selection, depth) => {
            println!("{}", evidence_report(&path, selection, depth)?)
        }
        Command::ListPacks => println!("{}", pack_report()?),
"""
new = """        Command::Evidence(path, selection, depth) => {
            println!("{}", evidence_report(&path, selection, depth)?)
        }
        Command::EvidencePath(path, from, to) => {
            println!("{}", evidence_path_report(&path, from, to)?)
        }
        Command::ListPacks => println!("{}", pack_report()?),
"""
if text.count(old) != 1:
    raise SystemExit(f"expected main match marker once, found {text.count(old)}")
text = text.replace(old, new, 1)

old = """        [command, path, selection, depth] if command == "evidence" => Ok(Command::Evidence(
            PathBuf::from(path),
            parse_selection_key(selection)?,
            depth
                .parse::<usize>()
                .map_err(|_| CliError(format!("invalid evidence depth: {depth}")))?,
        )),
        [command] if command == "list-packs" => Ok(Command::ListPacks),
"""
new = """        [command, path, selection, depth] if command == "evidence" => Ok(Command::Evidence(
            PathBuf::from(path),
            parse_selection_key(selection)?,
            depth
                .parse::<usize>()
                .map_err(|_| CliError(format!("invalid evidence depth: {depth}")))?,
        )),
        [command, path, from, to] if command == "evidence-path" => Ok(Command::EvidencePath(
            PathBuf::from(path),
            parse_selection_key(from)?,
            parse_selection_key(to)?,
        )),
        [command] if command == "list-packs" => Ok(Command::ListPacks),
"""
if text.count(old) != 1:
    raise SystemExit(f"expected parse marker once, found {text.count(old)}")
text = text.replace(old, new, 1)

usage_line = '  world-cli evidence <file.world> <selection-key> [depth]\\n\\\n'
if usage_line not in text:
    raise SystemExit("evidence usage line missing")
text = text.replace(
    usage_line,
    usage_line + '  world-cli evidence-path <file.world> <from-key> <to-key>\\n\\\n',
    1,
)
help_line = 'evidence    Print a typed evidence neighborhood around entity-N, relation-N, or event-N.\\n\\\n'
if help_line not in text:
    # M180 intentionally kept help minimal; insert before list-packs when absent.
    list_line = 'list-packs  List World Packs this build can create and restore."'
    if list_line not in text:
        raise SystemExit("list-packs help marker missing")
    text = text.replace(
        list_line,
        'evidence    Print a typed evidence neighborhood around entity-N, relation-N, or event-N.\\n\\\n'
        'evidence-path  Print the typed shortest evidence path between two selections.\\n\\\n'
        + list_line,
        1,
    )
else:
    text = text.replace(
        help_line,
        help_line + 'evidence-path  Print the typed shortest evidence path between two selections.\\n\\\n',
        1,
    )

marker = """fn pack_report() -> Result<String, Box<dyn Error>> {
"""
functions = r'''fn evidence_path_report(
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
    if snapshot.state_evidence_neighborhood(from, 0).is_none() {
        return Err(CliError(format!(
            "selection is not visible: {}",
            from.stable_key()
        )));
    }
    if snapshot.state_evidence_neighborhood(to, 0).is_none() {
        return Err(CliError(format!(
            "selection is not visible: {}",
            to.stable_key()
        )));
    }
    let path_steps = snapshot
        .state_evidence_shortest_path(from, to)
        .ok_or_else(|| {
            CliError(format!(
                "no evidence path: {} -> {}",
                from.stable_key(),
                to.stable_key()
            ))
        })?;
    let mut lines = vec![
        format!("file: {}", path.display()),
        format!("evidence-path: {} -> {}", from.stable_key(), to.stable_key()),
        format!("steps: {}", path_steps.len()),
    ];
    for (index, step) in path_steps.into_iter().enumerate() {
        lines.push(format!(
            "step {index} {} {} {}",
            step.from.stable_key(),
            evidence_edge_kind(step.edge),
            step.to.stable_key()
        ));
    }
    Ok(lines.join("\n"))
}

fn evidence_edge_kind(edge: StateEvidenceEdge) -> &'static str {
    match edge {
        StateEvidenceEdge::EntityEvent(_) => "entity-event",
        StateEvidenceEdge::RelationEvent(_) => "relation-event",
        StateEvidenceEdge::EntityRelation(evidence) => match evidence.role {
            RelationEndpointRole::From => "entity-relation:from",
            RelationEndpointRole::To => "entity-relation:to",
        },
    }
}

'''
if text.count(marker) != 1:
    raise SystemExit("pack_report marker missing")
text = text.replace(marker, functions + marker, 1)

old = """        assert_eq!(
            parse_command(["evidence", "sample.world", "event-9", "0"]).unwrap(),
            Command::Evidence(
                PathBuf::from("sample.world"),
                SelectionId::from_stable_key("event-9").unwrap(),
                0,
            )
        );
        assert_eq!(parse_command(["list-packs"]).unwrap(), Command::ListPacks);
"""
new = """        assert_eq!(
            parse_command(["evidence", "sample.world", "event-9", "0"]).unwrap(),
            Command::Evidence(
                PathBuf::from("sample.world"),
                SelectionId::from_stable_key("event-9").unwrap(),
                0,
            )
        );
        assert_eq!(
            parse_command(["evidence-path", "sample.world", "entity-1", "event-9"]).unwrap(),
            Command::EvidencePath(
                PathBuf::from("sample.world"),
                SelectionId::from_stable_key("entity-1").unwrap(),
                SelectionId::from_stable_key("event-9").unwrap(),
            )
        );
        assert_eq!(parse_command(["list-packs"]).unwrap(), Command::ListPacks);
"""
if text.count(old) != 1:
    raise SystemExit("command test marker missing")
text = text.replace(old, new, 1)
text = text.replace(
    '        assert!(parse_command(["evidence", "sample.world", "entity-7", "deep"]).is_err());\n',
    '        assert!(parse_command(["evidence", "sample.world", "entity-7", "deep"]).is_err());\n'
    '        assert!(parse_command(["evidence-path", "sample.world", "entity-07", "event-9"]).is_err());\n'
    '        assert!(parse_command(["evidence-path", "sample.world", "entity-7", "event-09"]).is_err());\n',
    1,
)

marker = """    #[test]
    fn pack_report_lists_registered_worlds() {
"""
tests = r'''    #[test]
    fn evidence_path_report_is_machine_stable_for_identity_path() {
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
        let report = evidence_path_report_from_snapshot(
            Path::new("builtin.world"),
            &snapshot,
            root,
            root,
        )
        .unwrap();

        assert!(report.contains(&format!(
            "evidence-path: {} -> {}",
            root.stable_key(),
            root.stable_key()
        )));
        assert!(report.contains("steps: 0"));
    }

    #[test]
    fn evidence_path_report_exposes_a_typed_one_step_edge_when_available() {
        let registry = world_builtins::registry().unwrap();
        let mut found = None;
        for descriptor in registry.descriptors() {
            let session = registry.create(&descriptor.pack.id).unwrap();
            let snapshot = session.snapshot();
            if let Some(edge) = snapshot.state_evidence_edges().first().copied() {
                let (from, to) = edge.selections();
                found = Some((snapshot, from, to, edge));
                break;
            }
        }
        let Some((snapshot, from, to, edge)) = found else {
            return;
        };
        let report = evidence_path_report_from_snapshot(
            Path::new("builtin.world"),
            &snapshot,
            from,
            to,
        )
        .unwrap();

        assert!(report.contains("steps: 1"));
        assert!(report.contains(&format!(
            "step 0 {} {} {}",
            from.stable_key(),
            evidence_edge_kind(edge),
            to.stable_key()
        )));
    }

    #[test]
    fn evidence_path_report_distinguishes_hidden_and_disconnected_selections() {
        let registry = world_builtins::registry().unwrap();
        let pack_id = registry.descriptors()[0].pack.id.clone();
        let session = registry.create(&pack_id).unwrap();
        let snapshot = session.snapshot();
        let root = snapshot
            .timeline
            .items
            .first()
            .map(|item| item.id)
            .or_else(|| snapshot.inspectors.keys().copied().next())
            .expect("built-in Pack should expose a visible selection");
        let hidden = SelectionId::from_stable_key("entity-18446744073709551615").unwrap();

        let error = evidence_path_report_from_snapshot(
            Path::new("builtin.world"),
            &snapshot,
            hidden,
            root,
        )
        .unwrap_err();
        assert!(error.to_string().contains("selection is not visible"));
    }

'''
if text.count(marker) != 1:
    raise SystemExit("pack report test marker missing")
text = text.replace(marker, tests + marker, 1)

path.write_text(text)
