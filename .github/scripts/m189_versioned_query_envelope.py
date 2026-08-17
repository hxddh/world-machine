from pathlib import Path

main = Path("crates/world-cli/src/main.rs")
text = main.read_text()

marker = '''use world_query::{
    execute_comparison_query, execute_query, Difference, EvidenceComparisonRequest,
    EvidenceComparisonResult, EvidenceEdge, EvidenceQueryRequest, EvidenceQueryResponse,
};

'''
insert = marker + '''const QUERY_PROTOCOL: &str = "world-machine-evidence-query";
const QUERY_PROTOCOL_VERSION: u64 = 1;

'''
if text.count(marker) != 1:
    raise SystemExit("query import marker missing")
text = text.replace(marker, insert, 1)

for old, new, expected in [
    (
        '''        Ok(response) => serde_json::json!({
            "status": "ok",
            "response": response,
        }),
        Err(error) => serde_json::json!({
            "status": "error",
            "error": error,
        }),
''',
        '''        Ok(response) => serde_json::json!({
            "protocol": QUERY_PROTOCOL,
            "version": QUERY_PROTOCOL_VERSION,
            "status": "ok",
            "response": response,
        }),
        Err(error) => serde_json::json!({
            "protocol": QUERY_PROTOCOL,
            "version": QUERY_PROTOCOL_VERSION,
            "status": "error",
            "error": error,
        }),
''',
        2,
    ),
]:
    if text.count(old) != expected:
        raise SystemExit(f"unexpected envelope block count: {text.count(old)}")
    text = text.replace(old, new)

# Pin protocol metadata in helper-level JSON tests.
text = text.replace(
    'assert_eq!(neighborhood["status"], "ok");',
    'assert_query_protocol(&neighborhood);\n        assert_eq!(neighborhood["status"], "ok");',
)
text = text.replace(
    'assert_eq!(path["status"], "ok");',
    'assert_query_protocol(&path);\n        assert_eq!(path["status"], "ok");',
)
text = text.replace(
    'assert_eq!(semantic["status"], "error");',
    'assert_query_protocol(&semantic);\n        assert_eq!(semantic["status"], "error");',
)
text = text.replace(
    'assert_eq!(output["status"], "ok");',
    'assert_query_protocol(&output);\n        assert_eq!(output["status"], "ok");',
    1,
)

marker = '''    fn first_visible_snapshot_and_key() -> (ProjectionSnapshot, String) {
'''
helper = '''    fn assert_query_protocol(envelope: &serde_json::Value) {
        assert_eq!(envelope["protocol"], QUERY_PROTOCOL);
        assert_eq!(envelope["version"], QUERY_PROTOCOL_VERSION);
    }

'''
if text.count(marker) != 1:
    raise SystemExit("test helper insertion marker missing")
text = text.replace(marker, helper + marker, 1)
main.write_text(text)

integration = Path("crates/world-cli/tests/machine_query_transport.rs")
test_text = integration.read_text()

# Every protocol response parsed by the subprocess tests must carry the same identity.
needle = 'let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();\n'
count = test_text.count(needle)
if count != 6:
    raise SystemExit(f"unexpected subprocess envelope count: {count}")
test_text = test_text.replace(
    needle,
    needle + '    assert_protocol(&envelope);\n',
)

marker = '''fn run_query(args: &[&str], stdin: Option<&str>) -> Output {
'''
helper = '''fn assert_protocol(envelope: &serde_json::Value) {
    assert_eq!(envelope["protocol"], "world-machine-evidence-query");
    assert_eq!(envelope["version"], 1);
}

'''
if test_text.count(marker) != 1:
    raise SystemExit("integration helper insertion marker missing")
test_text = test_text.replace(marker, helper + marker, 1)
integration.write_text(test_text)
