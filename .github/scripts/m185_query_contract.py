from pathlib import Path

p = Path("crates/world-query/src/lib.rs")
t = p.read_text()

marker = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceNeighborhoodResult {
'''
insert = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "query", rename_all = "kebab-case")]
pub enum EvidenceQueryRequest {
    Neighborhood { root: String, max_depth: usize },
    ShortestPath { from: String, to: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum EvidenceQueryResponse {
    Neighborhood { value: EvidenceNeighborhoodResult },
    ShortestPath { value: EvidencePathResult },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceComparisonRequest {
    pub root: String,
    pub max_depth: usize,
}

'''
if t.count(marker) != 1:
    raise SystemExit("result marker missing")
t = t.replace(marker, insert + marker, 1)

old = '''pub enum QueryError {
    SelectionNotVisible(String),
'''
new = '''pub enum QueryError {
    InvalidSelectionKey(String),
    SelectionNotVisible(String),
'''
if t.count(old) != 1:
    raise SystemExit("QueryError marker missing")
t = t.replace(old, new, 1)

old = '''        match self {
            Self::SelectionNotVisible(selection) => {
'''
new = '''        match self {
            Self::InvalidSelectionKey(selection) => {
                write!(f, "invalid selection key: {selection}")
            }
            Self::SelectionNotVisible(selection) => {
'''
if t.count(old) != 1:
    raise SystemExit("QueryError display marker missing")
t = t.replace(old, new, 1)

marker = '''pub fn query_neighborhood(
'''
functions = '''pub fn execute_query(
    snapshot: &ProjectionSnapshot,
    request: &EvidenceQueryRequest,
) -> Result<EvidenceQueryResponse, QueryError> {
    match request {
        EvidenceQueryRequest::Neighborhood { root, max_depth } => {
            let root = parse_selection_key(root)?;
            query_neighborhood(snapshot, root, *max_depth)
                .map(|value| EvidenceQueryResponse::Neighborhood { value })
        }
        EvidenceQueryRequest::ShortestPath { from, to } => {
            let from = parse_selection_key(from)?;
            let to = parse_selection_key(to)?;
            query_shortest_path(snapshot, from, to)
                .map(|value| EvidenceQueryResponse::ShortestPath { value })
        }
    }
}

pub fn execute_comparison_query(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    request: &EvidenceComparisonRequest,
) -> Result<EvidenceComparisonResult, QueryError> {
    let root = parse_selection_key(&request.root)?;
    query_neighborhood_comparison(left, right, root, request.max_depth)
}

fn parse_selection_key(key: &str) -> Result<SelectionId, QueryError> {
    SelectionId::from_stable_key(key)
        .ok_or_else(|| QueryError::InvalidSelectionKey(key.to_owned()))
}

'''
if t.count(marker) != 1:
    raise SystemExit("query_neighborhood marker missing")
t = t.replace(marker, functions + marker, 1)

# Add contract tests before the existing first test.
marker = '''    #[test]
    fn neighborhood_and_path_are_stable_serializable_dtos() {
'''
tests = r'''    #[test]
    fn serialized_query_requests_execute_without_callers_parsing_selection_ids() {
        let snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        let request: EvidenceQueryRequest = serde_json::from_str(
            r#"{"query":"neighborhood","root":"relation-5","max_depth":2}"#,
        )
        .unwrap();
        let response = execute_query(&snapshot, &request).unwrap();
        let EvidenceQueryResponse::Neighborhood { value } = response else {
            panic!("expected neighborhood response");
        };
        assert_eq!(value.root, "relation-5");
        assert!(value.nodes.iter().any(|node| node.selection == "entity-2" && node.depth == 2));

        let request: EvidenceQueryRequest = serde_json::from_str(
            r#"{"query":"shortest-path","from":"relation-5","to":"entity-2"}"#,
        )
        .unwrap();
        let response = execute_query(&snapshot, &request).unwrap();
        let EvidenceQueryResponse::ShortestPath { value } = response else {
            panic!("expected shortest path response");
        };
        assert_eq!(value.from, "relation-5");
        assert_eq!(value.to, "entity-2");
        assert_eq!(value.steps.len(), 2);
    }

    #[test]
    fn query_contract_rejects_noncanonical_selection_keys() {
        let snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        let request = EvidenceQueryRequest::Neighborhood {
            root: "entity-01".into(),
            max_depth: 2,
        };
        assert_eq!(
            execute_query(&snapshot, &request),
            Err(QueryError::InvalidSelectionKey("entity-01".into()))
        );
    }

    #[test]
    fn comparison_request_executes_typed_future_comparison() {
        let left = snapshot(EntityId::new(1), EntityId::new(3));
        let right = snapshot(EntityId::new(3), EntityId::new(1));
        let request: EvidenceComparisonRequest = serde_json::from_str(
            r#"{"root":"relation-5","max_depth":1}"#,
        )
        .unwrap();
        let result = execute_comparison_query(&left, &right, &request).unwrap();
        assert!(!result.identical);
        assert_eq!(result.left_only_edges.len(), 2);
        assert_eq!(result.right_only_edges.len(), 2);

        let encoded = serde_json::to_string(&result).unwrap();
        let restored: EvidenceComparisonResult = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored, result);
    }

'''
if t.count(marker) != 1:
    raise SystemExit("first test marker missing")
t = t.replace(marker, tests + marker, 1)
p.write_text(t)
