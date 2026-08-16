from pathlib import Path

path = Path('crates/world-compare/src/lib.rs')
text = path.read_text()

old = '''    left.title == right.title
        && left.subtitle == right.subtitle
        && indexed_relation_state_rows(left) == indexed_relation_state_rows(right)
        && left_snapshot.relation_identity(relation) == right_snapshot.relation_identity(relation)
}
'''
new = '''    match (
        left_snapshot.relation_identity(relation),
        right_snapshot.relation_identity(relation),
    ) {
        (Some(left_identity), Some(right_identity)) => {
            left.title == right.title
                && left.subtitle == right.subtitle
                && indexed_relation_state_rows(left) == indexed_relation_state_rows(right)
                && left_identity == right_identity
        }
        _ => same_inspector_state(left, right),
    }
}
'''
assert text.count(old) == 1, text.count(old)
text = text.replace(old, new)

marker = '''    #[test]
    fn relation_endpoint_identity_change_is_a_relation_state_change() {
'''
assert text.count(marker) == 1
legacy_test = '''    #[test]
    fn legacy_relation_without_stable_identity_falls_back_to_visible_endpoint_rows() {
        let (id, mut left) = relation_inspector_with_endpoints(
            5,
            "Works With",
            "Active",
            "2",
            1,
            "Alice · Entity #1",
            2,
            "Bob · Entity #2",
        );
        let (_, mut right) = relation_inspector_with_endpoints(
            5,
            "Works With",
            "Active",
            "2",
            1,
            "Alice · Entity #1",
            3,
            "Carol · Entity #3",
        );
        left.sections
            .retain(|section| section.title != RELATION_IDENTITY_SECTION);
        right
            .sections
            .retain(|section| section.title != RELATION_IDENTITY_SECTION);

        let comparison = compare_snapshots(
            &snapshot(20, [(id, left)], vec![], vec![]),
            &snapshot(20, [(id, right)], vec![], vec![]),
        );

        assert_eq!(comparison.relations.len(), 1);
        assert_eq!(comparison.relations[0].kind, DifferenceKind::Changed);
        assert!(comparison.relations[0]
            .inspector_rows
            .iter()
            .any(|row| row.key.label == "To"));
    }

'''
text = text.replace(marker, legacy_test + marker)
path.write_text(text)
