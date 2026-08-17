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
    "    Why { event: String },\n    Neighborhood { root: String, max_depth: usize },",
    "    Why { event: String },\n    Influence { event: String },\n    Neighborhood { root: String, max_depth: usize },",
    "request variant",
)
text = replace_once(
    text,
    "    Why { value: EvidenceWhyResult },\n    Neighborhood { value: EvidenceNeighborhoodResult },",
    "    Why { value: EvidenceWhyResult },\n    Influence { value: EvidenceInfluenceResult },\n    Neighborhood { value: EvidenceNeighborhoodResult },",
    "response variant",
)
text = replace_once(
    text,
    "pub struct EvidenceWhyResult {\n    pub event: String,\n    pub nodes: Vec<EvidenceCausalNode>,\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\npub struct EvidenceCausalNode",
    "pub struct EvidenceWhyResult {\n    pub event: String,\n    pub nodes: Vec<EvidenceCausalNode>,\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\npub struct EvidenceInfluenceResult {\n    pub event: String,\n    pub nodes: Vec<EvidenceCausalNode>,\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\npub struct EvidenceCausalNode",
    "influence DTO",
)
text = replace_once(
    text,
    "        EvidenceQueryRequest::Why { event } => {\n            let event = parse_selection_key(event)?;\n            query_why(snapshot, event).map(|value| EvidenceQueryResponse::Why { value })\n        }\n        EvidenceQueryRequest::Neighborhood { root, max_depth } => {",
    "        EvidenceQueryRequest::Why { event } => {\n            let event = parse_selection_key(event)?;\n            query_why(snapshot, event).map(|value| EvidenceQueryResponse::Why { value })\n        }\n        EvidenceQueryRequest::Influence { event } => {\n            let event = parse_selection_key(event)?;\n            query_influence(snapshot, event)\n                .map(|value| EvidenceQueryResponse::Influence { value })\n        }\n        EvidenceQueryRequest::Neighborhood { root, max_depth } => {",
    "execute arm",
)
influence_fn = r'''pub fn query_influence(
    snapshot: &ProjectionSnapshot,
    event: SelectionId,
) -> Result<EvidenceInfluenceResult, QueryError> {
    if !matches!(event, SelectionId::Event(_)) {
        return Err(QueryError::SelectionKindMismatch {
            selection: event.stable_key(),
            expected: EvidenceSelectionKind::Event,
        });
    }

    let visible = snapshot
        .timeline
        .items
        .iter()
        .filter(|item| matches!(item.id, SelectionId::Event(_)))
        .map(|item| (item.id, item))
        .collect::<std::collections::BTreeMap<_, _>>();
    if !visible.contains_key(&event) {
        return Err(QueryError::SelectionNotVisible(event.stable_key()));
    }

    let mut children = std::collections::BTreeMap::<SelectionId, Vec<SelectionId>>::new();
    for item in snapshot
        .timeline
        .items
        .iter()
        .filter(|item| matches!(item.id, SelectionId::Event(_)))
    {
        for cause in &item.caused_by {
            let cause = SelectionId::Event(*cause);
            if visible.contains_key(&cause) {
                children.entry(cause).or_default().push(item.id);
            }
        }
    }
    for direct_children in children.values_mut() {
        direct_children.sort_by_key(|child| {
            let item = visible
                .get(child)
                .copied()
                .expect("causal child must remain visible");
            (item.world_time, *child)
        });
        direct_children.dedup();
    }

    let mut discovered = std::collections::BTreeSet::from([event]);
    let mut queue = std::collections::VecDeque::from([(event, 0usize)]);
    let mut nodes = Vec::new();

    while let Some((current, depth)) = queue.pop_front() {
        let item = visible
            .get(&current)
            .copied()
            .expect("queued causal event must remain visible");
        let caused_by = item
            .caused_by
            .iter()
            .map(|cause| SelectionId::Event(*cause))
            .filter(|cause| visible.contains_key(cause))
            .map(|cause| cause.stable_key())
            .collect();

        nodes.push(EvidenceCausalNode {
            event: current.stable_key(),
            depth,
            world_time: item.world_time,
            title: item.title.clone(),
            subtitle: item.subtitle.clone(),
            caused_by,
        });

        if let Some(direct_children) = children.get(&current) {
            for child in direct_children {
                if discovered.insert(*child) {
                    queue.push_back((*child, depth + 1));
                }
            }
        }
    }

    Ok(EvidenceInfluenceResult {
        event: event.stable_key(),
        nodes,
    })
}

'''
text = replace_once(
    text,
    "    Ok(EvidenceWhyResult {\n        event: event.stable_key(),\n        nodes,\n    })\n}\n\npub fn query_neighborhood(",
    "    Ok(EvidenceWhyResult {\n        event: event.stable_key(),\n        nodes,\n    })\n}\n\n" + influence_fn + "pub fn query_neighborhood(",
    "query_influence",
)
lib.write_text(text)

Path("crates/world-query/tests").mkdir(parents=True, exist_ok=True)
Path("crates/world-query/tests/causal_influence.rs").write_text(r'''use world_core::{EntityId, EventId};
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_query, EvidenceInfluenceResult, EvidenceQueryRequest, EvidenceQueryResponse,
    EvidenceSelectionKind, QueryError,
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

fn influence(snapshot: &ProjectionSnapshot, event: &str) -> EvidenceInfluenceResult {
    let response = execute_query(
        snapshot,
        &EvidenceQueryRequest::Influence {
            event: event.into(),
        },
    )
    .unwrap();
    let EvidenceQueryResponse::Influence { value } = response else {
        panic!("expected influence response")
    };
    value
}

#[test]
fn serialized_influence_traverses_visible_descendants_at_minimum_depth() {
    let snapshot = snapshot(vec![
        event(5, 5, &[4, 1]),
        event(3, 3, &[1, 99]),
        event(4, 4, &[2]),
        event(2, 2, &[1]),
        event(1, 1, &[4]),
    ]);
    let request: EvidenceQueryRequest =
        serde_json::from_str(r#"{"query":"influence","event":"event-1"}"#).unwrap();
    let response = execute_query(&snapshot, &request).unwrap();
    let json = serde_json::to_string(&response).unwrap();
    let restored: EvidenceQueryResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, response);

    let EvidenceQueryResponse::Influence { value } = response else {
        panic!("expected influence response")
    };
    assert_eq!(value.event, "event-1");
    assert_eq!(
        value
            .nodes
            .iter()
            .map(|node| (node.event.as_str(), node.depth))
            .collect::<Vec<_>>(),
        vec![
            ("event-1", 0),
            ("event-2", 1),
            ("event-3", 1),
            ("event-5", 1),
            ("event-4", 2),
        ]
    );
    assert_eq!(value.nodes[2].caused_by, vec!["event-1"]);
    assert_eq!(value.nodes[3].caused_by, vec!["event-4", "event-1"]);
    assert_eq!(
        value.nodes.iter().filter(|node| node.event == "event-1").count(),
        1
    );
}

#[test]
fn direct_children_use_world_time_then_selection_id_not_timeline_order() {
    let snapshot = snapshot(vec![event(3, 2, &[1]), event(2, 2, &[1]), event(1, 1, &[])]);
    let value = influence(&snapshot, "event-1");
    assert_eq!(
        value
            .nodes
            .iter()
            .map(|node| node.event.as_str())
            .collect::<Vec<_>>(),
        vec!["event-1", "event-2", "event-3"]
    );
}

#[test]
fn influence_enforces_event_kind_canonical_keys_and_timeline_visibility() {
    let snapshot = snapshot(vec![event(1, 1, &[])]);
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::Influence {
                event: SelectionId::Entity(EntityId::new(1)).stable_key(),
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
            &EvidenceQueryRequest::Influence {
                event: "event-07".into(),
            },
        ),
        Err(QueryError::InvalidSelectionKey("event-07".into()))
    );
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::Influence {
                event: "event-99".into(),
            },
        ),
        Err(QueryError::SelectionNotVisible("event-99".into()))
    );
}
''')

Path("crates/world-cli/tests/machine_query_influence.rs").write_text(r'''use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_query::{EvidenceQueryRequest, EvidenceQueryResponse};

#[test]
fn stdin_influence_query_emits_a_versioned_typed_downstream_history() {
    let (path, event) = world_fixture_with_event();
    let request = serde_json::to_string(&EvidenceQueryRequest::Influence {
        event: event.clone(),
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
    let EvidenceQueryResponse::Influence { value } = response else {
        panic!("expected influence response")
    };
    assert_eq!(value.event, event);
    assert!(!value.nodes.is_empty());
    assert_eq!(value.nodes[0].event, value.event);
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
    std::env::temp_dir().join(format!("world-machine-influence-{unique}.world"))
}
''')

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M193 Machine Causal Influence

Expose downstream causal influence through the existing machine query contract so external investigators can ask what visible Events were influenced by a visible root Event.

## Current baseline

The machine investigation surface is complete through M192:

- M173–M189: canonical machine evidence queries, comparison, JSON subprocess/stdin transport, stable semantic errors, and protocol v1 envelopes.
- M190: deterministic visible selection discovery.
- M191: display-safe structured selection detail with timeline-owned Event visibility.
- M192: `why` causal ancestry over visible persisted `TimelineItem.caused_by`, with hidden-cause filtering, cycle protection, and breadth-first minimum-depth semantics.

The workflow now supports `selections -> describe -> neighborhood / shortest-path / why`. M193 adds the downstream half of persisted causal investigation without merging causal edges into the state-evidence graph.

## Product goal

A caller should be able to ask:

```json
{"query":"influence","event":"event-42"}
```

and receive a deterministic visible downstream causal traversal rooted at that Event.

## Architecture boundary

1. `world-query` owns the machine influence DTO and traversal semantics.
2. Reuse the generic M192 `EvidenceCausalNode`; do not create a second almost-identical causal node shape.
3. `world-cli` remains thin JSON/subprocess transport; no new top-level command.
4. Derive influence only from visible `ProjectionSnapshot.timeline.items` and their persisted `caused_by` links.
5. Never traverse or export Events absent from the visible timeline.
6. Keep causal traversal separate from state-evidence adjacency and inspector visibility.
7. Keep protocol identity/version at `world-machine-evidence-query` v1; this is additive.
8. Do not expose the full projection to AgentRuntime.

## M193 — `influence` query

Extend `EvidenceQueryRequest` with:

```json
{"query":"influence","event":"event-42"}
```

Return `EvidenceInfluenceResult { event, nodes }` using `Vec<EvidenceCausalNode>`.

## Traversal rules

- Parse through the existing canonical stable-key boundary.
- Canonical entity/relation roots return the existing `SelectionKindMismatch { expected: event }` error.
- Root must be a timeline-visible Event or return `SelectionNotVisible`.
- Build child adjacency by reversing visible persisted `caused_by` edges only when both endpoints are visible timeline Events.
- Root depth is 0. Direct effects are depth 1. Use BFS so multiply reachable Events get minimum causal depth.
- Deduplicate and cycle-protect traversal.
- There is no persisted child vector, so direct children use stable `(world_time, SelectionId)` ascending order. This intentionally avoids coupling the machine contract to timeline presentation order.
- Each exported node retains its persisted `caused_by` order, filtered to visible timeline Events.

## Tests

Prove at minimum:

1. serialized `influence` request/response serde round-trip;
2. chain/branch/diamond traversal returns minimum BFS depth;
3. same-depth child order is deterministic by world time then typed selection ID, independent of input timeline ordering;
4. hidden referenced causes do not leak through exported node metadata or become adjacency roots;
5. cycles do not duplicate or loop;
6. canonical wrong-kind, malformed stable key, and invisible Event errors remain stable;
7. a real stdin `world-cli` subprocess emits the v1 typed influence response;
8. all M190–M192 query behavior remains green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M193

Do not add causal path-between-events queries, free-text search, HTTP/WebSocket/MCP, AgentRuntime access, raw World/Event mutation data, Pack-specific causal semantics, or protocol v2.
''')
