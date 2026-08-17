from pathlib import Path

path = Path("crates/world-query/src/lib.rs")
text = path.read_text()

old = '''    let mut visited = std::collections::BTreeSet::new();
    let mut nodes = Vec::new();
    visit_visible_causes(event, 0, &visible, &mut visited, &mut nodes);

    Ok(EvidenceWhyResult {
        event: event.stable_key(),
        nodes,
    })
}

fn visit_visible_causes(
    event: SelectionId,
    depth: usize,
    visible: &std::collections::BTreeMap<SelectionId, &TimelineItem>,
    visited: &mut std::collections::BTreeSet<SelectionId>,
    nodes: &mut Vec<EvidenceWhyNode>,
) {
    if !visited.insert(event) {
        return;
    }
    let Some(item) = visible.get(&event).copied() else {
        return;
    };

    let caused_by = item
        .caused_by
        .iter()
        .map(|cause| SelectionId::Event(*cause))
        .filter(|cause| visible.contains_key(cause))
        .collect::<Vec<_>>();
    nodes.push(EvidenceWhyNode {
        event: event.stable_key(),
        depth,
        world_time: item.world_time,
        title: item.title.clone(),
        subtitle: item.subtitle.clone(),
        caused_by: caused_by.iter().map(|cause| cause.stable_key()).collect(),
    });

    for cause in caused_by {
        visit_visible_causes(cause, depth + 1, visible, visited, nodes);
    }
}
'''
new = '''    let mut discovered = std::collections::BTreeSet::from([event]);
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
            .collect::<Vec<_>>();

        nodes.push(EvidenceWhyNode {
            event: current.stable_key(),
            depth,
            world_time: item.world_time,
            title: item.title.clone(),
            subtitle: item.subtitle.clone(),
            caused_by: caused_by.iter().map(|cause| cause.stable_key()).collect(),
        });

        for cause in caused_by {
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
'''
if text.count(old) != 1:
    raise SystemExit("query_why traversal block missing or ambiguous")
text = text.replace(old, new, 1)

marker = '''    #[test]
    fn why_query_filters_hidden_causes_and_cycle_protects() {
'''
test = '''    #[test]
    fn why_query_preserves_direct_cause_order_and_minimum_depth() {
        let mut snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        snapshot.timeline.items = vec![
            TimelineItem {
                id: SelectionId::Event(EventId::new(3)),
                world_time: 3,
                title: "Final".into(),
                subtitle: "Root".into(),
                caused_by: vec![EventId::new(2), EventId::new(1)],
            },
            TimelineItem {
                id: SelectionId::Event(EventId::new(2)),
                world_time: 2,
                title: "First direct cause".into(),
                subtitle: "Also points at event 1".into(),
                caused_by: vec![EventId::new(1)],
            },
            TimelineItem {
                id: SelectionId::Event(EventId::new(1)),
                world_time: 1,
                title: "Second direct cause".into(),
                subtitle: "Direct and indirect".into(),
                caused_by: Vec::new(),
            },
        ];

        let value = query_why(&snapshot, SelectionId::Event(EventId::new(3))).unwrap();
        assert_eq!(value.nodes[0].caused_by, vec!["event-2", "event-1"]);
        assert_eq!(
            value
                .nodes
                .iter()
                .map(|node| (node.event.as_str(), node.depth))
                .collect::<Vec<_>>(),
            vec![("event-3", 0), ("event-2", 1), ("event-1", 1)]
        );
    }

'''
if text.count(marker) != 1:
    raise SystemExit("why hidden/cycle test marker missing")
text = text.replace(marker, test + marker, 1)
path.write_text(text)
