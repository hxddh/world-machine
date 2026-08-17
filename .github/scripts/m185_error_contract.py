from pathlib import Path

path = Path("crates/world-query/src/lib.rs")
text = path.read_text()

old = '''#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryError {
'''
new = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "error", content = "details", rename_all = "kebab-case")]
pub enum QueryError {
'''
if text.count(old) != 1:
    raise SystemExit("QueryError derive marker missing or ambiguous")
text = text.replace(old, new, 1)

marker = '''    #[test]
    fn serialized_query_requests_execute_without_callers_parsing_selection_ids() {
'''
tests = r'''    #[test]
    fn query_errors_have_stable_serializable_shapes() {
        let cases = [
            (
                QueryError::InvalidSelectionKey("entity-01".into()),
                r#"{"error":"invalid-selection-key","details":"entity-01"}"#,
            ),
            (
                QueryError::SelectionNotVisible("entity-99".into()),
                r#"{"error":"selection-not-visible","details":"entity-99"}"#,
            ),
            (
                QueryError::NoEvidencePath {
                    from: "entity-1".into(),
                    to: "event-9".into(),
                },
                r#"{"error":"no-evidence-path","details":{"from":"entity-1","to":"event-9"}}"#,
            ),
            (
                QueryError::SelectionNotVisibleInEitherWorld("relation-5".into()),
                r#"{"error":"selection-not-visible-in-either-world","details":"relation-5"}"#,
            ),
        ];

        for (error, expected_json) in cases {
            let json = serde_json::to_string(&error).unwrap();
            assert_eq!(json, expected_json);
            let restored: QueryError = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, error);
        }
    }

'''
if text.count(marker) != 1:
    raise SystemExit("query contract test marker missing or ambiguous")
text = text.replace(marker, tests + marker, 1)
path.write_text(text)
