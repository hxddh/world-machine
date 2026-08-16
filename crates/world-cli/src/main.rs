use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use world_integrity::{check_archive, ArchiveIntegrityError};
use world_persistence::{ArchivedEvent, WorldArchive};
use world_projection::{ProjectionSnapshot, RelationEndpointRole, SelectionId, StateEvidenceEdge};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Inspect(PathBuf),
    Check(PathBuf),
    Validate(PathBuf),
    Events(PathBuf),
    Why(PathBuf, u64),
    Evidence(PathBuf, SelectionId, usize),
    ListPacks,
    Help,
}

#[derive(Debug)]
struct CliError(String);

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for CliError {}

fn main() -> Result<(), Box<dyn Error>> {
    let command = parse_command(env::args().skip(1))?;
    match command {
        Command::Inspect(path) => println!("{}", inspect_report(&path)?),
        Command::Check(path) => println!("{}", check_report(&path)?),
        Command::Validate(path) => println!("{}", validate_report(&path)?),
        Command::Events(path) => println!("{}", events_report(&path)?),
        Command::Why(path, event_id) => println!("{}", why_report(&path, event_id)?),
        Command::Evidence(path, selection, depth) => {
            println!("{}", evidence_report(&path, selection, depth)?)
        }
        Command::ListPacks => println!("{}", pack_report()?),
        Command::Help => println!("{}", usage()),
    }
    Ok(())
}

fn parse_command<I, S>(args: I) -> Result<Command, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<String>>();
    match args.as_slice() {
        [command, path] if command == "inspect" => Ok(Command::Inspect(PathBuf::from(path))),
        [command, path] if command == "check" => Ok(Command::Check(PathBuf::from(path))),
        [command, path] if command == "validate" => Ok(Command::Validate(PathBuf::from(path))),
        [command, path] if command == "events" => Ok(Command::Events(PathBuf::from(path))),
        [command, path, event_id] if command == "why" => {
            let event_id = event_id
                .parse::<u64>()
                .map_err(|_| CliError(format!("invalid event id: {event_id}")))?;
            Ok(Command::Why(PathBuf::from(path), event_id))
        }
        [command, path, selection] if command == "evidence" => Ok(Command::Evidence(
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
        [command] if command == "list-packs" => Ok(Command::ListPacks),
        [command] if matches!(command.as_str(), "help" | "--help" | "-h") => Ok(Command::Help),
        [] => Ok(Command::Help),
        _ => Err(CliError(format!("invalid arguments\n\n{}", usage()))),
    }
}

fn usage() -> &'static str {
    "World Machine document tools\n\n\
Usage:\n\
  world-cli inspect <file.world>\n\
  world-cli check <file.world>\n\
  world-cli validate <file.world>\n\
  world-cli events <file.world>\n\
  world-cli why <file.world> <event-id>\n\
  world-cli evidence <file.world> <selection-key> [depth]\n\n\
  world-cli list-packs\n\n\
inspect     Parse and summarize a World archive without requiring its Pack.\n\
check       Verify Pack-independent archive structure and causal integrity.\n\
validate    Parse the archive and open it through the currently installed Pack registry.\n\
events      Print the archived event timeline, including actors, targets, and causal links.\n\
why         Trace an event recursively through its archived caused_by graph.\n\
list-packs  List World Packs this build can create and restore."
}

fn parse_selection_key(key: &str) -> Result<SelectionId, CliError> {
    SelectionId::from_stable_key(key)
        .ok_or_else(|| CliError(format!("invalid selection key: {key}")))
}

fn load_archive(path: &Path) -> Result<WorldArchive, Box<dyn Error>> {
    let json = fs::read_to_string(path)?;
    Ok(WorldArchive::from_json(&json)?)
}

fn inspect_report(path: &Path) -> Result<String, Box<dyn Error>> {
    let archive = load_archive(path)?;
    Ok(format_archive_report(path, &archive))
}

fn format_archive_report(path: &Path, archive: &WorldArchive) -> String {
    let mut lines = vec![
        format!("file: {}", path.display()),
        format!("format: {}@{}", archive.format, archive.format_version),
        format!("pack: {}@{}", archive.pack.id, archive.pack.version),
        format!("world_time: {}", archive.world_time),
        format!("events: {}", archive.events.len()),
        format!("pending: {}", archive.pending.len()),
    ];

    if let Some(event) = archive.events.last() {
        lines.push(format!(
            "last_event: #{} {} @ t={}",
            event.id, event.kind, event.world_time
        ));
    }
    lines.join("\n")
}

fn check_report(path: &Path) -> Result<String, Box<dyn Error>> {
    let archive = load_archive(path)?;
    Ok(check_report_from_archive(path, &archive)?)
}

fn check_report_from_archive(
    path: &Path,
    archive: &WorldArchive,
) -> Result<String, ArchiveIntegrityError> {
    let summary = check_archive(archive)?;
    let latest_event_time = summary
        .latest_event_time
        .map(|time| time.to_string())
        .unwrap_or_else(|| "-".into());
    Ok(format!(
        "{}\nintegrity: ok\nchecked_events: {}\nchecked_pending: {}\nlatest_event_time: {}",
        format_archive_report(path, archive),
        summary.event_count,
        summary.pending_count,
        latest_event_time
    ))
}

fn validate_report(path: &Path) -> Result<String, Box<dyn Error>> {
    let archive = load_archive(path)?;
    let registry = world_builtins::registry()?;
    let session = registry.open_archive(&archive)?;
    let snapshot = session.snapshot();

    Ok(format!(
        "{}\nvalidation: ok\nruntime_title: {}\nprojection_world_time: {}",
        format_archive_report(path, &archive),
        snapshot.title,
        snapshot.world_time
    ))
}

fn events_report(path: &Path) -> Result<String, Box<dyn Error>> {
    let archive = load_archive(path)?;
    Ok(events_report_from_archive(path, &archive))
}

fn events_report_from_archive(path: &Path, archive: &WorldArchive) -> String {
    let mut lines = vec![format!("file: {}", path.display())];
    if archive.events.is_empty() {
        lines.push("events: none".into());
        return lines.join("\n");
    }

    lines.push(format!("events: {}", archive.events.len()));
    for event in &archive.events {
        lines.push(format_event(event));
    }
    lines.join("\n")
}

fn format_event(event: &ArchivedEvent) -> String {
    let actor = event
        .actor
        .map(|actor| format!("#{actor}"))
        .unwrap_or_else(|| "-".into());
    format!(
        "#{} {} @ t={} actor={} targets={} caused_by={}",
        event.id,
        event.kind,
        event.world_time,
        actor,
        format_ids(&event.targets),
        format_ids(&event.caused_by)
    )
}

fn format_ids(ids: &[u64]) -> String {
    if ids.is_empty() {
        return "[]".into();
    }
    format!(
        "[{}]",
        ids.iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn why_report(path: &Path, event_id: u64) -> Result<String, Box<dyn Error>> {
    let archive = load_archive(path)?;
    why_report_from_archive(path, &archive, event_id)
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

fn why_report_from_archive(
    path: &Path,
    archive: &WorldArchive,
    event_id: u64,
) -> Result<String, CliError> {
    let events = event_index(archive)?;
    if !events.contains_key(&event_id) {
        return Err(CliError(format!("unknown event id: #{event_id}")));
    }

    let mut lines = vec![
        format!("file: {}", path.display()),
        format!("why: #{event_id}"),
    ];
    let mut visiting = BTreeSet::new();
    render_cause(event_id, &events, 0, &mut visiting, &mut lines);
    Ok(lines.join("\n"))
}

fn event_index(archive: &WorldArchive) -> Result<BTreeMap<u64, &ArchivedEvent>, CliError> {
    let mut events = BTreeMap::new();
    for event in &archive.events {
        if events.insert(event.id, event).is_some() {
            return Err(CliError(format!("duplicate event id: #{}", event.id)));
        }
    }
    Ok(events)
}

fn render_cause(
    event_id: u64,
    events: &BTreeMap<u64, &ArchivedEvent>,
    depth: usize,
    visiting: &mut BTreeSet<u64>,
    lines: &mut Vec<String>,
) {
    let indent = "  ".repeat(depth);
    if !visiting.insert(event_id) {
        lines.push(format!("{indent}[cycle] #{event_id}"));
        return;
    }

    match events.get(&event_id) {
        Some(event) => {
            lines.push(format!(
                "{indent}#{} {} @ t={}",
                event.id, event.kind, event.world_time
            ));
            for cause in &event.caused_by {
                render_cause(*cause, events, depth + 1, visiting, lines);
            }
        }
        None => lines.push(format!("{indent}[missing] #{event_id}")),
    }

    visiting.remove(&event_id);
}

fn evidence_report(
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
        .ok_or_else(|| {
            CliError(format!(
                "selection is not visible: {}",
                selection.stable_key()
            ))
        })?;
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

fn pack_report() -> Result<String, Box<dyn Error>> {
    let registry = world_builtins::registry()?;
    let descriptors = registry.descriptors();
    let mut lines = vec![format!("packs: {}", descriptors.len())];
    for descriptor in descriptors {
        lines.push(format!(
            "{}@{}\t{}\t{}",
            descriptor.pack.id, descriptor.pack.version, descriptor.title, descriptor.description
        ));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_document_commands() {
        assert_eq!(
            parse_command(["inspect", "sample.world"]).unwrap(),
            Command::Inspect(PathBuf::from("sample.world"))
        );
        assert_eq!(
            parse_command(["check", "sample.world"]).unwrap(),
            Command::Check(PathBuf::from("sample.world"))
        );
        assert_eq!(
            parse_command(["validate", "sample.world"]).unwrap(),
            Command::Validate(PathBuf::from("sample.world"))
        );
        assert_eq!(
            parse_command(["events", "sample.world"]).unwrap(),
            Command::Events(PathBuf::from("sample.world"))
        );
        assert_eq!(
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
        assert_eq!(parse_command(Vec::<String>::new()).unwrap(), Command::Help);
        assert!(parse_command(["inspect"]).is_err());
        assert!(parse_command(["why", "sample.world", "not-a-number"]).is_err());
        assert!(parse_command(["evidence", "sample.world", "entity-07"]).is_err());
        assert!(parse_command(["evidence", "sample.world", "entity-7", "deep"]).is_err());
    }

    #[test]
    fn inspect_report_is_pack_independent() {
        let archive = empty_archive("example.uninstalled", 42);
        let path = Path::new("sample.world");
        let report = format_archive_report(path, &archive);

        assert!(report.contains("file: sample.world"));
        assert!(report.contains("pack: example.uninstalled@7"));
        assert!(report.contains("world_time: 42"));
    }

    #[test]
    fn check_report_accepts_every_builtin_world_archive() {
        let registry = world_builtins::registry().unwrap();
        for descriptor in registry.descriptors() {
            let session = registry.create(&descriptor.pack.id).unwrap();
            let archive = session.archive().unwrap().unwrap();
            let report = check_report_from_archive(Path::new("builtin.world"), &archive).unwrap();
            assert!(report.contains("integrity: ok"));
            assert!(report.contains(&format!(
                "pack: {}@{}",
                descriptor.pack.id, descriptor.pack.version
            )));
        }
    }

    #[test]
    fn events_report_prints_causal_links() {
        let mut archive = empty_archive("example.uninstalled", 3);
        archive.events = vec![
            event(1, "storm", 1, vec![]),
            event(2, "damage", 2, vec![1]),
            event(3, "income_lost", 3, vec![2]),
        ];

        let report = events_report_from_archive(Path::new("case.world"), &archive);

        assert!(report.contains("#1 storm @ t=1"));
        assert!(report.contains("#2 damage @ t=2"));
        assert!(report.contains("caused_by=[#1]"));
    }

    #[test]
    fn why_report_traces_recursive_causes() {
        let mut archive = empty_archive("example.uninstalled", 3);
        archive.events = vec![
            event(1, "storm", 1, vec![]),
            event(2, "damage", 2, vec![1]),
            event(3, "income_lost", 3, vec![2]),
        ];

        let report = why_report_from_archive(Path::new("case.world"), &archive, 3).unwrap();

        assert!(report.contains("why: #3"));
        assert!(report.contains("#3 income_lost @ t=3\n  #2 damage @ t=2\n    #1 storm @ t=1"));
    }

    #[test]
    fn why_report_marks_missing_and_cyclic_causes() {
        let mut missing = empty_archive("example.uninstalled", 2);
        missing.events = vec![event(2, "damage", 2, vec![99])];
        let missing_report =
            why_report_from_archive(Path::new("missing.world"), &missing, 2).unwrap();
        assert!(missing_report.contains("  [missing] #99"));

        let mut cyclic = empty_archive("example.uninstalled", 2);
        cyclic.events = vec![event(1, "one", 1, vec![2]), event(2, "two", 2, vec![1])];
        let cyclic_report = why_report_from_archive(Path::new("cycle.world"), &cyclic, 1).unwrap();
        assert!(cyclic_report.contains("    [cycle] #1"));
    }

    #[test]
    fn why_report_rejects_duplicate_and_unknown_event_ids() {
        let mut duplicate = empty_archive("example.uninstalled", 1);
        duplicate.events = vec![event(1, "one", 1, vec![]), event(1, "again", 1, vec![])];
        assert!(why_report_from_archive(Path::new("duplicate.world"), &duplicate, 1).is_err());

        let archive = empty_archive("example.uninstalled", 0);
        assert!(why_report_from_archive(Path::new("empty.world"), &archive, 42).is_err());
    }

    #[test]
    fn validate_opens_a_builtin_world_archive() {
        let registry = world_builtins::registry().unwrap();
        let pack_id = registry.descriptors()[0].pack.id.clone();
        let session = registry.create(&pack_id).unwrap();
        let archive = session.archive().unwrap().unwrap();
        let path = temp_world_path("validate");
        fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();

        let report = validate_report(&path).unwrap();

        assert!(report.contains("validation: ok"));
        assert!(report.contains(&format!("pack: {pack_id}@")));
        let _ = fs::remove_file(path);
    }

    #[test]
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
                    snapshot.inspectors.keys().copied().find(|selection| {
                        snapshot
                            .state_evidence_neighborhood(*selection, 0)
                            .is_some()
                    })
                });
            if let Some(root) = root {
                found = Some((snapshot, root));
                break;
            }
        }
        let (snapshot, root) = found.expect("a built-in Pack should expose a visible selection");
        let report =
            evidence_report_from_snapshot(Path::new("builtin.world"), &snapshot, root, 0).unwrap();

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

        let error = evidence_report_from_snapshot(Path::new("builtin.world"), &snapshot, hidden, 2)
            .unwrap_err();
        assert!(error.to_string().contains("selection is not visible"));
    }

    #[test]
    fn pack_report_lists_registered_worlds() {
        let report = pack_report().unwrap();
        assert!(report.starts_with("packs: "));
        assert!(report.lines().count() >= 2);
    }

    fn empty_archive(pack_id: &str, world_time: u64) -> WorldArchive {
        WorldArchive {
            format: world_persistence::WORLD_ARCHIVE_FORMAT.into(),
            format_version: world_persistence::WORLD_ARCHIVE_VERSION,
            pack: world_persistence::WorldPackRef::new(pack_id, "7"),
            world_time,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn event(id: u64, kind: &str, world_time: u64, caused_by: Vec<u64>) -> ArchivedEvent {
        ArchivedEvent {
            id,
            kind: kind.into(),
            world_time,
            actor: None,
            targets: Vec::new(),
            caused_by,
            payload: BTreeMap::new(),
            changes: Vec::new(),
        }
    }

    fn temp_world_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        env::temp_dir().join(format!(
            "world-machine-cli-{label}-{}-{nonce}.world",
            process::id()
        ))
    }
}
