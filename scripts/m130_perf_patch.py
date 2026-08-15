from pathlib import Path


def replace_exact(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing patch anchor: {label}")
    if text.count(old) != 1:
        raise SystemExit(f"patch anchor not unique: {label} ({text.count(old)})")
    return text.replace(old, new, 1)


p = Path("crates/world-projection/src/influence.rs")
text = p.read_text()
text = replace_exact(
    text,
    '''    let children = children_from_timeline(timeline);
    let terminal = semantic_ids
        .iter()
        .copied()
        .filter(|event| !has_semantic_descendant(*event, &children, &semantic_ids))
''',
    '''    let children = children_from_timeline(timeline);
    let mut descendant_memo = BTreeMap::new();
    let mut descendant_visiting = BTreeSet::new();
    let terminal = semantic_ids
        .iter()
        .copied()
        .filter(|event| {
            !has_semantic_descendant(
                *event,
                &children,
                &semantic_ids,
                &mut descendant_memo,
                &mut descendant_visiting,
            )
        })
''',
    "memoized terminal selection",
)
text = replace_exact(
    text,
    '''    let mut memo = BTreeMap::<EventId, Option<Vec<EventId>>>::new();
    let mut visiting = BTreeSet::new();
    let Some(path) =
        best_semantic_path_to(terminal, root, &by_id, inspectors, &mut memo, &mut visiting)
    else {
        return Vec::new();
    };

    path.into_iter()
        .filter_map(|event| by_id.get(&event).copied())
        .collect()
''',
    '''    let mut memo = BTreeMap::<EventId, Option<BestPathState>>::new();
    let mut visiting = BTreeSet::new();
    if best_semantic_path_state(
        terminal,
        root,
        &by_id,
        inspectors,
        &mut memo,
        &mut visiting,
    )
    .is_none()
    {
        return Vec::new();
    }

    let mut path = Vec::new();
    let mut current = terminal;
    while current != root {
        if inspector_has_world_effect(
            inspectors.get(&SelectionId::Event(current)),
        ) {
            path.push(current);
        }
        let Some(state) = memo.get(&current).and_then(|state| *state) else {
            return Vec::new();
        };
        let Some(predecessor) = state.predecessor else {
            return Vec::new();
        };
        current = predecessor;
    }
    path.reverse();

    path.into_iter()
        .filter_map(|event| by_id.get(&event).copied())
        .collect()
''',
    "predecessor reconstruction",
)
text = replace_exact(
    text,
    '''fn has_semantic_descendant(
    root: EventId,
    children: &BTreeMap<EventId, Vec<&TimelineItem>>,
    semantic: &BTreeSet<EventId>,
) -> bool {
    let mut visited = BTreeSet::from([root]);
    let mut queue = VecDeque::from([root]);
    while let Some(parent) = queue.pop_front() {
        let Some(next) = children.get(&parent) else {
            continue;
        };
        for item in next {
            let child = event_id(item);
            if !visited.insert(child) {
                continue;
            }
            if semantic.contains(&child) {
                return true;
            }
            queue.push_back(child);
        }
    }
    false
}

fn best_semantic_path_to(
    current: EventId,
    root: EventId,
    by_id: &BTreeMap<EventId, &TimelineItem>,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    memo: &mut BTreeMap<EventId, Option<Vec<EventId>>>,
    visiting: &mut BTreeSet<EventId>,
) -> Option<Vec<EventId>> {
    if current == root {
        return Some(Vec::new());
    }
    if let Some(cached) = memo.get(&current) {
        return cached.clone();
    }
    if !visiting.insert(current) {
        return None;
    }

    let result = by_id.get(&current).and_then(|item| {
        let semantic = inspector_has_world_effect(inspectors.get(&item.id));
        let mut best = None::<Vec<EventId>>;
        for cause in &item.caused_by {
            let Some(mut candidate) =
                best_semantic_path_to(*cause, root, by_id, inspectors, memo, visiting)
            else {
                continue;
            };
            if semantic {
                candidate.push(current);
            }
            let should_replace = best.as_ref().map_or(true, |existing| {
                candidate.len() > existing.len()
                    || (candidate.len() == existing.len() && candidate > *existing)
            });
            if should_replace {
                best = Some(candidate);
            }
        }
        best
    });

    visiting.remove(&current);
    memo.insert(current, result.clone());
    result
}
''',
    '''fn has_semantic_descendant(
    root: EventId,
    children: &BTreeMap<EventId, Vec<&TimelineItem>>,
    semantic: &BTreeSet<EventId>,
    memo: &mut BTreeMap<EventId, bool>,
    visiting: &mut BTreeSet<EventId>,
) -> bool {
    if let Some(cached) = memo.get(&root) {
        return *cached;
    }
    if !visiting.insert(root) {
        return false;
    }

    let result = children.get(&root).is_some_and(|next| {
        next.iter().any(|item| {
            let child = event_id(item);
            semantic.contains(&child)
                || has_semantic_descendant(child, children, semantic, memo, visiting)
        })
    });

    visiting.remove(&root);
    memo.insert(root, result);
    result
}

#[derive(Clone, Copy, Debug)]
struct BestPathState {
    semantic_count: usize,
    predecessor: Option<EventId>,
}

fn best_semantic_path_state(
    current: EventId,
    root: EventId,
    by_id: &BTreeMap<EventId, &TimelineItem>,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    memo: &mut BTreeMap<EventId, Option<BestPathState>>,
    visiting: &mut BTreeSet<EventId>,
) -> Option<BestPathState> {
    if current == root {
        return Some(BestPathState {
            semantic_count: 0,
            predecessor: None,
        });
    }
    if let Some(cached) = memo.get(&current) {
        return *cached;
    }
    if !visiting.insert(current) {
        return None;
    }

    let result = by_id.get(&current).and_then(|item| {
        let current_is_semantic = inspector_has_world_effect(inspectors.get(&item.id));
        let mut best = None::<(EventId, usize)>;
        for cause in &item.caused_by {
            let Some(previous) = best_semantic_path_state(
                *cause,
                root,
                by_id,
                inspectors,
                memo,
                visiting,
            ) else {
                continue;
            };
            let semantic_count = previous.semantic_count + usize::from(current_is_semantic);
            let should_replace = best.is_none_or(|(best_cause, best_count)| {
                semantic_count > best_count
                    || (semantic_count == best_count && *cause > best_cause)
            });
            if should_replace {
                best = Some((*cause, semantic_count));
            }
        }
        best.map(|(predecessor, semantic_count)| BestPathState {
            semantic_count,
            predecessor: Some(predecessor),
        })
    });

    visiting.remove(&current);
    memo.insert(current, result);
    result
}
''',
    "linear memoized path derivation",
)
p.write_text(text)
