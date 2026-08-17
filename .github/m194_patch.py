from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


lib = Path("crates/world-query/src/lib.rs")
text = lib.read_text()
text = replace_once(
    text,
    "    InspectorProjection, ProjectionSnapshot, RelationEndpointRole, SelectionId, StateEvidenceEdge,\n};",
    "    InspectorProjection, ProjectionSnapshot, RelationEndpointRole, SelectionId, StateEvidenceEdge,\n    TimelineItem,\n};",
    "TimelineItem import",
)
text = replace_once(
    text,
    "    Why { event: String },\n    Influence { event: String },\n    Neighborhood { root: String, max_depth: usize },",
    "    Why { event: String },\n    Influence { event: String },\n    CausalPath { from: String, to: String },\n    Neighborhood { root: String, max_depth: usize },",
    "request variant",
)
text = replace_once(
    text,
    "    Why { value: EvidenceWhyResult },\n    Influence { value: EvidenceInfluenceResult },\n    Neighborhood { value: EvidenceNeighborhoodResult },",
    "    Why { value: EvidenceWhyResult },\n    Influence { value: EvidenceInfluenceResult },\n    CausalPath { value: EvidenceCausalPathResult },\n    Neighborhood { value: EvidenceNeighborhoodResult },",
    "response variant",
)
text = replace_once(
    text,
    "pub struct EvidenceInfluenceResult {\n    pub event: String,\n    pub nodes: Vec<EvidenceCausalNode>,\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\npub struct EvidenceCausalNode",
    "pub struct EvidenceInfluenceResult {\n    pub event: String,\n    pub nodes: Vec<EvidenceCausalNode>,\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\npub struct EvidenceCausalPathResult {\n    pub from: String,\n    pub to: String,\n    pub nodes: Vec<EvidenceCausalNode>,\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\npub struct EvidenceCausalNode",
    "causal path DTO",
)
text = replace_once(
    text,
    "    NoEvidencePath {\n        from: String,\n        to: String,\n    },\n    SelectionNotVisibleInEitherWorld(String),",
    "    NoEvidencePath {\n        from: String,\n        to: String,\n    },\n    NoCausalPath {\n        from: String,\n        to: String,\n    },\n    SelectionNotVisibleInEitherWorld(String),",
    "causal path error",
)
text = replace_once(
    text,
    "            Self::NoEvidencePath { from, to } => write!(f, \"no evidence path: {from} -> {to}\"),\n            Self::SelectionNotVisibleInEitherWorld(selection) => {",
    "            Self::NoEvidencePath { from, to } => write!(f, \"no evidence path: {from} -> {to}\"),\n            Self::NoCausalPath { from, to } => write!(f, \"no causal path: {from} -> {to}\"),\n            Self::SelectionNotVisibleInEitherWorld(selection) => {",
    "causal path display",
)
text = replace_once(
    text,
    "        EvidenceQueryRequest::Influence { event } => {\n            let event = parse_selection_key(event)?;\n            query_influence(snapshot, event).map(|value| EvidenceQueryResponse::Influence { value })\n        }\n        EvidenceQueryRequest::Neighborhood { root, max_depth } => {",
    "        EvidenceQueryRequest::Influence { event } => {\n            let event = parse_selection_key(event)?;\n            query_influence(snapshot, event).map(|value| EvidenceQueryResponse::Influence { value })\n        }\n        EvidenceQueryRequest::CausalPath { from, to } => {\n            let from = parse_selection_key(from)?;\n            let to = parse_selection_key(to)?;\n            query_causal_path(snapshot, from, to)\n                .map(|value| EvidenceQueryResponse::CausalPath { value })\n        }\n        EvidenceQueryRequest::Neighborhood { root, max_depth } => {",
    "execute causal path",
)

start = text.index("pub fn query_why(")
end = text.index("pub fn query_neighborhood(")
causal_block = r'''struct VisibleCausalGraph<'a> {
    events: std::collections::BTreeMap<SelectionId, &'a TimelineItem>,
    children: std::collections::BTreeMap<SelectionId, Vec<SelectionId>>,
}

impl<'a> VisibleCausalGraph<'a> {
    fn new(snapshot: &'a ProjectionSnapshot) -> Self {
        let events = snapshot
            .timeline
            .items
            .iter()
            .filter(|item| matches!(item.id, SelectionId::Event(_)))
            .map(|item| (item.id, item))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut children = std::collections::BTreeMap::<SelectionId, Vec<SelectionId>>::new();

        for item in events.values().copied() {
            for cause in &item.caused_by {
                let cause = SelectionId::Event(*cause);
                if events.contains_key(&cause) {
                    children.entry(cause).or_default().push(item.id);
                }
            }
        }
        for direct_children in children.values_mut() {
            direct_children.sort_by_key(|child| {
                let item = events
                    .get(child)
                    .copied()
                    .expect("causal child must remain visible");
                (item.world_time, *child)
            });
            direct_children.dedup();
        }

        Self { events, children }
    }

    fn require_event(&self, event: SelectionId) -> Result<(), QueryError> {
        if !matches!(event, SelectionId::Event(_)) {
            return Err(QueryError::SelectionKindMismatch {
                selection: event.stable_key(),
                expected: EvidenceSelectionKind::Event,
            });
        }
        if !self.events.contains_key(&event) {
            return Err(QueryError::SelectionNotVisible(event.stable_key()));
        }
        Ok(())
    }

    fn parents(&self, event: SelectionId) -> Vec<SelectionId> {
        let item = self
            .events
            .get(&event)
            .copied()
            .expect("causal event must remain visible");
        item.caused_by
            .iter()
            .map(|cause| SelectionId::Event(*cause))
            .filter(|cause| self.events.contains_key(cause))
            .collect()
    }

    fn children(&self, event: SelectionId) -> &[SelectionId] {
        self.children.get(&event).map(Vec::as_slice).unwrap_or(&[])
    }

    fn node(&self, event: SelectionId, depth: usize) -> EvidenceCausalNode {
        let item = self
            .events
            .get(&event)
            .copied()
            .expect("causal event must remain visible");
        EvidenceCausalNode {
            event: event.stable_key(),
            depth,
            world_time: item.world_time,
            title: item.title.clone(),
            subtitle: item.subtitle.clone(),
            caused_by: self
                .parents(event)
                .into_iter()
                .map(|cause| cause.stable_key())
                .collect(),
        }
    }
}

pub fn query_why(
    snapshot: &ProjectionSnapshot,
    event: SelectionId,
) -> Result<EvidenceWhyResult, QueryError> {
    let graph = VisibleCausalGraph::new(snapshot);
    graph.require_event(event)?;

    let mut discovered = std::collections::BTreeSet::from([event]);
    let mut queue = std::collections::VecDeque::from([(event, 0usize)]);
    let mut nodes = Vec::new();

    while let Some((current, depth)) = queue.pop_front() {
        nodes.push(graph.node(current, depth));
        for cause in graph.parents(current) {
            if discovered.insert(cause) {
                queue.push_back((cause, depth + 1));
            }
        }
    }

    Ok(EvidenceWhyResult {
        event: event.stable_key(),
        nodes,
    })
}

pub fn query_influence(
    snapshot: &ProjectionSnapshot,
    event: SelectionId,
) -> Result<EvidenceInfluenceResult, QueryError> {
    let graph = VisibleCausalGraph::new(snapshot);
    graph.require_event(event)?;

    let mut discovered = std::collections::BTreeSet::from([event]);
    let mut queue = std::collections::VecDeque::from([(event, 0usize)]);
    let mut nodes = Vec::new();

    while let Some((current, depth)) = queue.pop_front() {
        nodes.push(graph.node(current, depth));
        for child in graph.children(current) {
            if discovered.insert(*child) {
                queue.push_back((*child, depth + 1));
            }
        }
    }

    Ok(EvidenceInfluenceResult {
        event: event.stable_key(),
        nodes,
    })
}

pub fn query_causal_path(
    snapshot: &ProjectionSnapshot,
    from: SelectionId,
    to: SelectionId,
) -> Result<EvidenceCausalPathResult, QueryError> {
    let graph = VisibleCausalGraph::new(snapshot);
    graph.require_event(from)?;
    graph.require_event(to)?;

    let mut discovered = std::collections::BTreeSet::from([from]);
    let mut queue = std::collections::VecDeque::from([from]);
    let mut predecessor = std::collections::BTreeMap::<SelectionId, SelectionId>::new();

    while let Some(current) = queue.pop_front() {
        if current == to {
            break;
        }
        for child in graph.children(current) {
            if discovered.insert(*child) {
                predecessor.insert(*child, current);
                queue.push_back(*child);
            }
        }
    }

    if !discovered.contains(&to) {
        return Err(QueryError::NoCausalPath {
            from: from.stable_key(),
            to: to.stable_key(),
        });
    }

    let mut path = vec![to];
    let mut current = to;
    while current != from {
        current = *predecessor
            .get(&current)
            .expect("discovered causal target must have a predecessor");
        path.push(current);
    }
    path.reverse();

    Ok(EvidenceCausalPathResult {
        from: from.stable_key(),
        to: to.stable_key(),
        nodes: path
            .into_iter()
            .enumerate()
            .map(|(depth, event)| graph.node(event, depth))
            .collect(),
    })
}

'''
text = text[:start] + causal_block + text[end:]

text = replace_once(
    text,
    "            (\n                QueryError::NoEvidencePath {\n                    from: \"entity-1\".into(),\n                    to: \"event-9\".into(),\n                },\n                r#\"{\\\"error\\\":\\\"no-evidence-path\\\",\\\"details\\\":{\\\"from\\\":\\\"entity-1\\\",\\\"to\\\":\\\"event-9\\\"}}\"#,\n            ),\n            (\n                QueryError::SelectionNotVisibleInEitherWorld(\"relation-5\".into()),",
    "            (\n                QueryError::NoEvidencePath {\n                    from: \"entity-1\".into(),\n                    to: \"event-9\".into(),\n                },\n                r#\"{\\\"error\\\":\\\"no-evidence-path\\\",\\\"details\\\":{\\\"from\\\":\\\"entity-1\\\",\\\"to\\\":\\\"event-9\\\"}}\"#,\n            ),\n            (\n                QueryError::NoCausalPath {\n                    from: \"event-1\".into(),\n                    to: \"event-9\".into(),\n                },\n                r#\"{\\\"error\\\":\\\"no-causal-path\\\",\\\"details\\\":{\\\"from\\\":\\\"event-1\\\",\\\"to\\\":\\\"event-9\\\"}}\"#,\n            ),\n            (\n                QueryError::SelectionNotVisibleInEitherWorld(\"relation-5\".into()),",
    "query error serde case",
)
lib.write_text(text)

Path("crates/world-query/tests/causal_path.rs").write_text(r'''use world_core::{EntityId, EventId};
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_query, EvidenceQueryRequest, EvidenceQueryResponse, EvidenceSelectionKind, QueryError,
};

fn event(id: u64, world_time: u64, caused_by: &[u64]) -> TimelineItem {
    TimelineItem {
        id: SelectionId::Event(EventId::new(id)),
        world_time,
        title: format!("Event {id}"),
        subtitle: format!("world time {world_time}"),
        caused_by: caused_by.iter().copied().map(EventId::new).collect(),
    }
}

fn snapshot(items: Vec<TimelineItem>) -> ProjectionSnapshot {
    ProjectionSnapshot {
        timeline: TimelineProjection { items },
        ..ProjectionSnapshot::default()
    }
}

#[test]
fn causal_path_uses_deterministic_shortest_downstream_route_and_round_trips() {
    let snapshot = snapshot(vec![
        event(4, 4, &[3, 2]),
        event(3, 2, &[1]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let request: EvidenceQueryRequest = serde_json::from_str(
        r#"{"query":"causal-path","from":"event-1","to":"event-4"}"#,
    )
    .unwrap();
    let response = execute_query(&snapshot, &request).unwrap();
    let json = serde_json::to_string(&response).unwrap();
    let restored: EvidenceQueryResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, response);

    let EvidenceQueryResponse::CausalPath { value } = response else {
        panic!("expected causal-path response")
    };
    assert_eq!(value.from, "event-1");
    assert_eq!(value.to, "event-4");
    assert_eq!(
        value
            .nodes
            .iter()
            .map(|node| (node.event.as_str(), node.depth))
            .collect::<Vec<_>>(),
        vec![("event-1", 0), ("event-2", 1), ("event-4", 2)]
    );
}

#[test]
fn causal_path_identity_is_a_single_visible_node() {
    let snapshot = snapshot(vec![event(7, 11, &[])]);
    let response = execute_query(
        &snapshot,
        &EvidenceQueryRequest::CausalPath {
            from: "event-7".into(),
            to: "event-7".into(),
        },
    )
    .unwrap();
    let EvidenceQueryResponse::CausalPath { value } = response else {
        panic!("expected causal-path response")
    };
    assert_eq!(value.nodes.len(), 1);
    assert_eq!(value.nodes[0].event, "event-7");
    assert_eq!(value.nodes[0].depth, 0);
}

#[test]
fn causal_path_does_not_cross_hidden_or_reverse_edges() {
    let snapshot = snapshot(vec![event(1, 1, &[]), event(3, 3, &[2])]);
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::CausalPath {
                from: "event-1".into(),
                to: "event-3".into(),
            },
        ),
        Err(QueryError::NoCausalPath {
            from: "event-1".into(),
            to: "event-3".into(),
        })
    );

    let snapshot = snapshot(vec![event(1, 1, &[]), event(2, 2, &[1])]);
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::CausalPath {
                from: "event-2".into(),
                to: "event-1".into(),
            },
        ),
        Err(QueryError::NoCausalPath {
            from: "event-2".into(),
            to: "event-1".into(),
        })
    );
}

#[test]
fn causal_path_reuses_canonical_event_validation_for_both_endpoints() {
    let snapshot = snapshot(vec![event(1, 1, &[])]);
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::CausalPath {
                from: SelectionId::Entity(EntityId::new(1)).stable_key(),
                to: "event-1".into(),
            },
        ),
        Err(QueryError::SelectionKindMismatch {
            selection: "entity-1".into(),
            expected: EvidenceSelectionKind::Event,
        })
    );
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::CausalPath {
                from: "event-1".into(),
                to: "event-07".into(),
            },
        ),
        Err(QueryError::InvalidSelectionKey("event-07".into()))
    );
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::CausalPath {
                from: "event-1".into(),
                to: "event-99".into(),
            },
        ),
        Err(QueryError::SelectionNotVisible("event-99".into()))
    );
}
''')

Path("crates/world-cli/tests/machine_query_causal_path.rs").write_text(r'''use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_query::{EvidenceQueryRequest, EvidenceQueryResponse};

#[test]
fn stdin_causal_path_query_emits_a_versioned_typed_path() {
    let (path, event) = world_fixture_with_event();
    let request = serde_json::to_string(&EvidenceQueryRequest::CausalPath {
        from: event.clone(),
        to: event.clone(),
    })
    .unwrap();

    let output = run_query(
        &["evidence-query", path.to_str().unwrap(), "-"],
        Some(&request),
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["protocol"], "world-machine-evidence-query");
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["status"], "ok");
    let response: EvidenceQueryResponse =
        serde_json::from_value(envelope["response"].clone()).unwrap();
    let EvidenceQueryResponse::CausalPath { value } = response else {
        panic!("expected causal-path response")
    };
    assert_eq!(value.from, event);
    assert_eq!(value.to, value.from);
    assert_eq!(value.nodes.len(), 1);
    assert_eq!(value.nodes[0].event, value.from);
    assert_eq!(value.nodes[0].depth, 0);

    let _ = fs::remove_file(path);
}

fn run_query(args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_world-cli"))
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    if let Some(input) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin should be piped")
            .write_all(input.as_bytes())
            .unwrap();
    }

    child.wait_with_output().unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn world_fixture_with_event() -> (PathBuf, String) {
    let registry = world_builtins::registry().unwrap();
    for descriptor in registry.descriptors() {
        let session = registry.create(&descriptor.pack.id).unwrap();
        let snapshot = session.snapshot();
        let Some(event) = snapshot.timeline.items.first().map(|item| item.id) else {
            continue;
        };
        if !event.stable_key().starts_with("event-") {
            continue;
        }
        let archive = session.archive().unwrap().unwrap();
        let path = temp_world_path();
        fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();
        return (path, event.stable_key());
    }
    panic!("a built-in Pack should expose a visible timeline event")
}

fn temp_world_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("world-machine-causal-path-{unique}.world"))
}
''')

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M194 Machine Causal Path

Add a stable shortest causal path query between two visible Events and consolidate M192/M193 causal traversal onto one private visible-causal-graph primitive inside `world-query`.

## Current baseline

The machine investigation surface is complete through M193:

- state-evidence discovery, describe, neighborhood, shortest path, and comparison are already machine-readable;
- M192 adds upstream `why` traversal over visible persisted `caused_by` links;
- M193 adds downstream `influence` traversal with BFS minimum depth and deterministic `(world_time, SelectionId)` child ordering;
- CLI transport remains generic `evidence-query` JSON/stdin with protocol v1.

## Product goal

A caller should be able to ask:

```json
{"query":"causal-path","from":"event-1","to":"event-42"}
```

and receive one deterministic shortest visible causal route from cause to effect.

## Architecture boundary

1. `world-query` owns the private visible causal graph and path semantics.
2. Refactor `why` and `influence` to use the same graph helper so visibility/filtering/order cannot drift.
3. The graph is derived only from visible `ProjectionSnapshot.timeline.items` and persisted `caused_by` links.
4. Do not merge causal edges into the state-evidence graph.
5. Do not make inspector-only Events visible.
6. Reuse `EvidenceCausalNode` and protocol v1.
7. Keep `world-cli` transport-only; no new top-level command.

## M194 — `causal-path`

Add request:

```json
{"query":"causal-path","from":"event-1","to":"event-42"}
```

Return `EvidenceCausalPathResult { from, to, nodes }` with path nodes ordered source-to-target and path-relative depths 0..N.

## Path rules

- Both endpoints pass the existing canonical stable-key parser.
- Both endpoints must be timeline-visible Events; canonical wrong kinds use `SelectionKindMismatch`, invisible Events use `SelectionNotVisible`.
- Traverse only downstream `cause -> effect` edges where both endpoints are visible.
- Use BFS for shortest edge count.
- Equal-length paths are resolved by the same deterministic child ordering as M193: `(world_time, SelectionId)` ascending.
- `from == to` returns a one-node path at depth 0.
- If no visible downstream path exists, return stable `NoCausalPath { from, to }`.
- Hidden intermediate Events must never bridge a path.

## Internal causal graph

Introduce a private helper in `world-query` that owns:

- visible Event lookup;
- filtered persisted parent order;
- deterministic downstream children;
- visible `EvidenceCausalNode` materialization.

`why`, `influence`, and `causal-path` must all use it.

## Tests

At minimum prove:

1. request/response serde round-trip;
2. deterministic shortest-path tie-break through a diamond;
3. identity path returns one node;
4. reverse direction returns `NoCausalPath`;
5. hidden intermediate Events cannot create a path;
6. both endpoint validation paths remain stable;
7. `NoCausalPath` has pinned serialized shape;
8. existing M192/M193 tests remain green after refactor;
9. true stdin `world-cli` subprocess emits a v1 typed causal-path response.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M194

Do not add arbitrary causal subgraph export, causal comparison between worlds, HTTP/WebSocket/MCP, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
''')
