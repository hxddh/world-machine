from pathlib import Path

cargo = Path("crates/world-cli/Cargo.toml")
t = cargo.read_text()
t = t.replace('world-compare = { path = "../world-compare" }\n', 'world-query = { path = "../world-query" }\n')
cargo.write_text(t)

p = Path("crates/world-cli/src/main.rs")
t = p.read_text()
old_import = '''use world_compare::{
    compare_evidence_neighborhoods, DifferenceKind, EvidenceNeighborhoodComparison,
};
use world_integrity::{check_archive, ArchiveIntegrityError};
use world_persistence::{ArchivedEvent, WorldArchive};
use world_projection::{ProjectionSnapshot, RelationEndpointRole, SelectionId, StateEvidenceEdge};
'''
new_import = '''use world_integrity::{check_archive, ArchiveIntegrityError};
use world_persistence::{ArchivedEvent, WorldArchive};
use world_projection::{ProjectionSnapshot, SelectionId};
use world_query::{
    query_neighborhood, query_neighborhood_comparison, query_shortest_path, Difference,
    EvidenceComparisonResult, EvidenceEdge,
};
'''
if t.count(old_import) != 1:
    raise SystemExit("expected old evidence imports once")
t = t.replace(old_import, new_import, 1)

start = t.index('fn evidence_report_from_snapshot(')
end = t.index('fn pack_report()', start)
new_block = r'''fn evidence_report_from_snapshot(
    path: &Path,
    snapshot: &ProjectionSnapshot,
    selection: SelectionId,
    max_depth: usize,
) -> Result<String, CliError> {
    let neighborhood = query_neighborhood(snapshot, selection, max_depth)
        .map_err(|error| CliError(error.to_string()))?;
    let mut lines = vec![
        format!("file: {}", path.display()),
        format!("evidence: {}", neighborhood.root),
        format!("depth: {}", neighborhood.max_depth),
        format!("nodes: {}", neighborhood.nodes.len()),
    ];
    for node in neighborhood.nodes {
        lines.push(format!("node {} {}", node.depth, node.selection));
    }
    lines.push(format!("edges: {}", neighborhood.edges.len()));
    for edge in &neighborhood.edges {
        lines.push(format_evidence_edge(edge));
    }
    Ok(lines.join("\n"))
}

fn format_evidence_edge(edge: &EvidenceEdge) -> String {
    match edge {
        EvidenceEdge::EntityEvent { entity, event } => {
            format!("edge entity-event {entity} {event}")
        }
        EvidenceEdge::RelationEvent { relation, event } => {
            format!("edge relation-event {relation} {event}")
        }
        EvidenceEdge::EntityRelation {
            entity,
            relation,
            role,
        } => format!(
            "edge entity-relation {} {entity} {relation}",
            match role {
                world_query::RelationRole::From => "from",
                world_query::RelationRole::To => "to",
            }
        ),
    }
}

fn evidence_path_report(
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
    let result = query_shortest_path(snapshot, from, to)
        .map_err(|error| CliError(error.to_string()))?;
    let mut lines = vec![
        format!("file: {}", path.display()),
        format!("evidence-path: {} -> {}", result.from, result.to),
        format!("steps: {}", result.steps.len()),
    ];
    for (index, step) in result.steps.into_iter().enumerate() {
        lines.push(format!(
            "step {index} {} {} {}",
            step.from,
            evidence_edge_kind(&step.edge),
            step.to
        ));
    }
    Ok(lines.join("\n"))
}

fn evidence_edge_kind(edge: &EvidenceEdge) -> &'static str {
    match edge {
        EvidenceEdge::EntityEvent { .. } => "entity-event",
        EvidenceEdge::RelationEvent { .. } => "relation-event",
        EvidenceEdge::EntityRelation { role, .. } => match role {
            world_query::RelationRole::From => "entity-relation:from",
            world_query::RelationRole::To => "entity-relation:to",
        },
    }
}

fn evidence_compare_report(
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
    Ok(format_evidence_comparison(
        left_path,
        right_path,
        &comparison,
    ))
}

fn format_evidence_comparison(
    left_path: &Path,
    right_path: &Path,
    comparison: &EvidenceComparisonResult,
) -> String {
    let mut lines = vec![
        format!("left: {}", left_path.display()),
        format!("right: {}", right_path.display()),
        format!("evidence-compare: {}", comparison.root),
        format!("depth: {}", comparison.max_depth),
        format!("identical: {}", comparison.identical),
        format!("node-changes: {}", comparison.nodes.len()),
    ];
    for node in &comparison.nodes {
        lines.push(format!(
            "node {} {} {} {}",
            difference_kind_key(node.kind),
            node.selection,
            optional_depth(node.left_depth),
            optional_depth(node.right_depth)
        ));
    }
    lines.push(format!(
        "left-only-edges: {}",
        comparison.left_only_edges.len()
    ));
    for edge in &comparison.left_only_edges {
        lines.push(format!("left-{}", format_evidence_edge(edge)));
    }
    lines.push(format!(
        "right-only-edges: {}",
        comparison.right_only_edges.len()
    ));
    for edge in &comparison.right_only_edges {
        lines.push(format!("right-{}", format_evidence_edge(edge)));
    }
    lines.join("\n")
}

fn difference_kind_key(kind: Difference) -> &'static str {
    match kind {
        Difference::LeftOnly => "left-only",
        Difference::RightOnly => "right-only",
        Difference::Changed => "changed",
    }
}

fn optional_depth(depth: Option<usize>) -> String {
    depth.map_or_else(|| "-".into(), |depth| depth.to_string())
}

'''
t = t[:start] + new_block + t[end:]

# Existing test compares a runtime edge label; keep that check test-only.
t = t.replace('evidence_edge_kind(edge),', 'runtime_evidence_edge_kind(edge),', 1)
insert_marker = '#[cfg(test)]\nmod tests {'
helper = '''#[cfg(test)]
fn runtime_evidence_edge_kind(edge: world_projection::StateEvidenceEdge) -> &'static str {
    match edge {
        world_projection::StateEvidenceEdge::EntityEvent(_) => "entity-event",
        world_projection::StateEvidenceEdge::RelationEvent(_) => "relation-event",
        world_projection::StateEvidenceEdge::EntityRelation(evidence) => match evidence.role {
            world_projection::RelationEndpointRole::From => "entity-relation:from",
            world_projection::RelationEndpointRole::To => "entity-relation:to",
        },
    }
}

'''
if t.count(insert_marker) != 1:
    raise SystemExit("test module marker missing")
t = t.replace(insert_marker, helper + insert_marker, 1)
p.write_text(t)
