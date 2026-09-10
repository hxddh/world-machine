mod causal;
mod influence;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use world_core::{
    Entity, EntityId, Event, EventId, Relation, RelationId, StateChange, Value, World,
};

pub use causal::{why_from_world, why_map_from_world, WhyNode, WhyProjection};

pub const ENTITY_HISTORY_SECTION: &str = "Recorded entity changes";
pub const RELATION_HISTORY_SECTION: &str = "Recorded relation changes";
pub const RELATION_ENDPOINTS_SECTION: &str = "Active relation endpoints";
pub const RELATION_IDENTITY_SECTION: &str = "Relation identity endpoints";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SelectionId {
    Entity(EntityId),
    Relation(RelationId),
    Event(EventId),
}

impl SelectionId {
    pub fn stable_key(self) -> String {
        match self {
            Self::Entity(id) => format!("entity-{id}"),
            Self::Relation(id) => format!("relation-{id}"),
            Self::Event(id) => format!("event-{id}"),
        }
    }

    pub fn from_stable_key(key: &str) -> Option<Self> {
        if let Some(id) = canonical_stable_id(key, "entity-") {
            return Some(Self::Entity(EntityId::new(id)));
        }
        if let Some(id) = canonical_stable_id(key, "relation-") {
            return Some(Self::Relation(RelationId::new(id)));
        }
        canonical_stable_id(key, "event-").map(|id| Self::Event(EventId::new(id)))
    }
}

fn canonical_stable_id(key: &str, prefix: &str) -> Option<u64> {
    let raw = key.strip_prefix(prefix)?;
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let id = raw.parse::<u64>().ok()?;
    (id.to_string() == raw).then_some(id)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EntityEventEvidence {
    pub entity: EntityId,
    pub event: EventId,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RelationEventEvidence {
    pub relation: RelationId,
    pub event: EventId,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RelationEndpointRole {
    From,
    To,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RelationIdentity {
    pub from: EntityId,
    pub to: EntityId,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EntityRelationEvidence {
    pub entity: EntityId,
    pub relation: RelationId,
    pub role: RelationEndpointRole,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StateEvidenceEdge {
    EntityEvent(EntityEventEvidence),
    RelationEvent(RelationEventEvidence),
    EntityRelation(EntityRelationEvidence),
}

impl StateEvidenceEdge {
    pub fn selections(self) -> (SelectionId, SelectionId) {
        match self {
            Self::EntityEvent(evidence) => (
                SelectionId::Entity(evidence.entity),
                SelectionId::Event(evidence.event),
            ),
            Self::RelationEvent(evidence) => (
                SelectionId::Relation(evidence.relation),
                SelectionId::Event(evidence.event),
            ),
            Self::EntityRelation(evidence) => (
                SelectionId::Entity(evidence.entity),
                SelectionId::Relation(evidence.relation),
            ),
        }
    }

    pub fn other(self, selection: SelectionId) -> Option<SelectionId> {
        let (left, right) = self.selections();
        if selection == left {
            Some(right)
        } else if selection == right {
            Some(left)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateEvidencePathStep {
    pub from: SelectionId,
    pub edge: StateEvidenceEdge,
    pub to: SelectionId,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StateEvidenceNeighborhoodNode {
    pub selection: SelectionId,
    pub depth: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateEvidenceNeighborhood {
    pub root: SelectionId,
    pub max_depth: usize,
    pub nodes: Vec<StateEvidenceNeighborhoodNode>,
    pub edges: Vec<StateEvidenceEdge>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionIntent {
    ForkBeforeEvent(EventId),
    InvokeCommand(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionCommand {
    pub id: String,
    pub title: String,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProjectionCapabilities {
    pub fork: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectionSnapshot {
    pub title: String,
    pub world_time: u64,
    pub capabilities: ProjectionCapabilities,
    pub briefing: Option<BriefingProjection>,
    pub commands: Vec<ProjectionCommand>,
    pub collection: CollectionProjection,
    pub timeline: TimelineProjection,
    pub canvas: CanvasProjection,
    pub inspectors: BTreeMap<SelectionId, InspectorProjection>,
    pub why: BTreeMap<EventId, WhyProjection>,
}

impl ProjectionSnapshot {
    pub fn inspector(&self, selection: SelectionId) -> Option<&InspectorProjection> {
        self.inspectors.get(&selection)
    }

    pub fn why(&self, event: EventId) -> Option<&WhyProjection> {
        self.why.get(&event)
    }

    pub fn entity_event_evidence(&self) -> Vec<EntityEventEvidence> {
        let visible_events = visible_event_ids_by_key(&self.timeline);
        let mut evidence = BTreeSet::new();
        for (selection, inspector) in &self.inspectors {
            let SelectionId::Entity(entity) = *selection else {
                continue;
            };
            evidence.extend(entity_event_evidence_from_inspector(
                entity,
                inspector,
                &visible_events,
            ));
        }
        evidence.into_iter().collect()
    }

    pub fn entity_history(&self, entity: EntityId) -> Vec<&TimelineItem> {
        let Some(inspector) = self.inspector(SelectionId::Entity(entity)) else {
            return Vec::new();
        };
        let visible_events = visible_event_ids_by_key(&self.timeline);
        let event_ids = entity_event_evidence_from_inspector(entity, inspector, &visible_events)
            .into_iter()
            .map(|evidence| evidence.event)
            .collect::<BTreeSet<_>>();

        self.timeline
            .items
            .iter()
            .filter(
                |item| matches!(item.id, SelectionId::Event(event) if event_ids.contains(&event)),
            )
            .collect()
    }

    pub fn directly_changed_entities(&self, event: EventId) -> Vec<EntityId> {
        self.entity_event_evidence()
            .into_iter()
            .filter(|evidence| evidence.event == event)
            .map(|evidence| evidence.entity)
            .collect()
    }

    pub fn relation_event_evidence(&self) -> Vec<RelationEventEvidence> {
        let visible_events = visible_event_ids_by_key(&self.timeline);
        let mut evidence = BTreeSet::new();
        for (selection, inspector) in &self.inspectors {
            let SelectionId::Relation(relation) = *selection else {
                continue;
            };
            evidence.extend(relation_event_evidence_from_inspector(
                relation,
                inspector,
                &visible_events,
            ));
        }
        evidence.into_iter().collect()
    }

    pub fn relation_history(&self, relation: RelationId) -> Vec<&TimelineItem> {
        let Some(inspector) = self.inspector(SelectionId::Relation(relation)) else {
            return Vec::new();
        };
        let visible_events = visible_event_ids_by_key(&self.timeline);
        let event_ids =
            relation_event_evidence_from_inspector(relation, inspector, &visible_events)
                .into_iter()
                .map(|evidence| evidence.event)
                .collect::<BTreeSet<_>>();

        self.timeline
            .items
            .iter()
            .filter(
                |item| matches!(item.id, SelectionId::Event(event) if event_ids.contains(&event)),
            )
            .collect()
    }

    pub fn directly_changed_relations(&self, event: EventId) -> Vec<RelationId> {
        self.relation_event_evidence()
            .into_iter()
            .filter(|evidence| evidence.event == event)
            .map(|evidence| evidence.relation)
            .collect()
    }

    pub fn entity_relation_evidence(&self) -> Vec<EntityRelationEvidence> {
        let visible_entities = visible_entity_ids_by_key(&self.inspectors);
        let mut evidence = BTreeSet::new();
        for (selection, inspector) in &self.inspectors {
            let SelectionId::Relation(relation) = *selection else {
                continue;
            };
            evidence.extend(entity_relation_evidence_from_inspector(
                relation,
                inspector,
                &visible_entities,
            ));
        }
        evidence.into_iter().collect()
    }

    pub fn relations_for_entity(&self, entity: EntityId) -> Vec<RelationId> {
        self.entity_relation_evidence()
            .into_iter()
            .filter(|evidence| evidence.entity == entity)
            .map(|evidence| evidence.relation)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn entities_for_relation(&self, relation: RelationId) -> Vec<EntityId> {
        self.entity_relation_evidence()
            .into_iter()
            .filter(|evidence| evidence.relation == relation)
            .map(|evidence| evidence.entity)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Where each canvas item is drawn, after any layout the Pack asked for
    /// that could not be read has been replaced by a grid.
    ///
    /// Edges and boxes both read this, so a line cannot arrive where its own
    /// endpoint is not.
    pub fn canvas_placements(&self) -> Vec<(SelectionId, f32, f32)> {
        let asked = self
            .canvas
            .items
            .iter()
            .map(|item| (item.x, item.y))
            .collect::<Vec<_>>();
        let drawn = canvas_layout::placements(&asked);
        self.canvas
            .items
            .iter()
            .zip(drawn)
            .map(|(item, (x, y))| (item.id, x, y))
            .collect()
    }

    /// Every relation whose two ends are both on the canvas, with the
    /// positions of those ends.
    ///
    /// A relation with an endpoint the canvas does not place cannot be drawn
    /// truthfully, so it is left out rather than anchored somewhere
    /// convenient. Ordering follows relation id so a redraw never reshuffles
    /// the picture.
    pub fn canvas_edges(&self) -> Vec<CanvasEdge> {
        let mut positions = BTreeMap::new();
        for (selection, x, y) in self.canvas_placements() {
            if let SelectionId::Entity(entity) = selection {
                positions.insert(entity, (x, y));
            }
        }

        let mut edges = Vec::new();
        for (selection, inspector) in &self.inspectors {
            let SelectionId::Relation(relation) = *selection else {
                continue;
            };
            let Some(identity) = relation_identity_from_inspector(inspector) else {
                continue;
            };
            let (Some(from), Some(to)) =
                (positions.get(&identity.from), positions.get(&identity.to))
            else {
                continue;
            };
            edges.push(CanvasEdge {
                relation,
                from: identity.from,
                to: identity.to,
                label: inspector.title.clone(),
                from_x: from.0,
                from_y: from.1,
                to_x: to.0,
                to_y: to.1,
            });
        }
        edges
    }

    pub fn relation_identity(&self, relation: RelationId) -> Option<RelationIdentity> {
        self.inspector(SelectionId::Relation(relation))
            .and_then(relation_identity_from_inspector)
    }

    pub fn state_evidence_edges(&self) -> Vec<StateEvidenceEdge> {
        self.entity_event_evidence()
            .into_iter()
            .map(StateEvidenceEdge::EntityEvent)
            .chain(
                self.relation_event_evidence()
                    .into_iter()
                    .map(StateEvidenceEdge::RelationEvent),
            )
            .chain(
                self.entity_relation_evidence()
                    .into_iter()
                    .map(StateEvidenceEdge::EntityRelation),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn state_evidence_edges_for(&self, selection: SelectionId) -> Vec<StateEvidenceEdge> {
        self.state_evidence_edges()
            .into_iter()
            .filter(|edge| edge.other(selection).is_some())
            .collect()
    }

    pub fn state_evidence_neighbors(&self, selection: SelectionId) -> Vec<SelectionId> {
        self.state_evidence_edges_for(selection)
            .into_iter()
            .filter_map(|edge| edge.other(selection))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn state_evidence_shortest_path(
        &self,
        start: SelectionId,
        goal: SelectionId,
    ) -> Option<Vec<StateEvidencePathStep>> {
        if !self.state_evidence_selection_is_visible(start)
            || !self.state_evidence_selection_is_visible(goal)
        {
            return None;
        }
        if start == goal {
            return Some(Vec::new());
        }

        let edges = self.state_evidence_edges();
        let mut queue = VecDeque::from([start]);
        let mut visited = BTreeSet::from([start]);
        let mut previous = BTreeMap::<SelectionId, (SelectionId, StateEvidenceEdge)>::new();

        while let Some(current) = queue.pop_front() {
            let mut adjacent = edges
                .iter()
                .filter_map(|edge| edge.other(current).map(|next| (next, *edge)))
                .collect::<Vec<_>>();
            adjacent.sort();
            adjacent.dedup();

            for (next, edge) in adjacent {
                if !visited.insert(next) {
                    continue;
                }
                previous.insert(next, (current, edge));
                if next == goal {
                    let mut cursor = goal;
                    let mut path = Vec::new();
                    while cursor != start {
                        let (from, edge) = previous
                            .get(&cursor)
                            .copied()
                            .expect("visited evidence node must have a predecessor");
                        path.push(StateEvidencePathStep {
                            from,
                            edge,
                            to: cursor,
                        });
                        cursor = from;
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(next);
            }
        }

        None
    }

    pub fn state_evidence_neighborhood(
        &self,
        root: SelectionId,
        max_depth: usize,
    ) -> Option<StateEvidenceNeighborhood> {
        if !self.state_evidence_selection_is_visible(root) {
            return None;
        }

        let edges = self.state_evidence_edges();
        let mut depths = BTreeMap::from([(root, 0usize)]);
        let mut queue = VecDeque::from([root]);

        while let Some(current) = queue.pop_front() {
            let current_depth = depths[&current];
            if current_depth >= max_depth {
                continue;
            }

            let mut adjacent = edges
                .iter()
                .filter_map(|edge| edge.other(current))
                .collect::<Vec<_>>();
            adjacent.sort();
            adjacent.dedup();

            for next in adjacent {
                if depths.contains_key(&next) {
                    continue;
                }
                depths.insert(next, current_depth + 1);
                queue.push_back(next);
            }
        }

        let mut nodes = depths
            .iter()
            .map(|(selection, depth)| StateEvidenceNeighborhoodNode {
                selection: *selection,
                depth: *depth,
            })
            .collect::<Vec<_>>();
        nodes.sort_by_key(|node| (node.depth, node.selection));

        let visible = depths.keys().copied().collect::<BTreeSet<_>>();
        let edges = edges
            .into_iter()
            .filter(|edge| {
                let (left, right) = edge.selections();
                visible.contains(&left) && visible.contains(&right)
            })
            .collect();

        Some(StateEvidenceNeighborhood {
            root,
            max_depth,
            nodes,
            edges,
        })
    }

    fn state_evidence_selection_is_visible(&self, selection: SelectionId) -> bool {
        match selection {
            SelectionId::Entity(_) | SelectionId::Relation(_) => {
                self.inspectors.contains_key(&selection)
            }
            SelectionId::Event(_) => self.timeline.items.iter().any(|item| item.id == selection),
        }
    }

    pub fn influence(&self, event: EventId) -> Vec<(usize, &TimelineItem)> {
        influence::influence_from_timeline(&self.timeline, event)
    }

    pub fn semantic_influence(&self, event: EventId) -> Vec<(usize, &TimelineItem)> {
        influence::semantic_influence_from_snapshot(&self.timeline, &self.inspectors, event)
    }

    pub fn semantic_path(&self, event: EventId) -> Vec<&TimelineItem> {
        influence::semantic_path_from_snapshot(&self.timeline, &self.inspectors, event)
    }

    pub fn semantic_path_details(&self, event: EventId) -> Vec<(usize, &TimelineItem, String)> {
        influence::semantic_path_details_from_snapshot(&self.timeline, &self.inspectors, event)
    }

    pub fn command(&self, id: &str) -> Option<&ProjectionCommand> {
        self.commands.iter().find(|command| command.id == id)
    }
}

fn visible_entity_ids_by_key(
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
) -> BTreeMap<String, EntityId> {
    inspectors
        .keys()
        .filter_map(|selection| {
            let SelectionId::Entity(entity) = *selection else {
                return None;
            };
            Some((selection.stable_key(), entity))
        })
        .collect()
}

fn visible_event_ids_by_key(timeline: &TimelineProjection) -> BTreeMap<String, EventId> {
    timeline
        .items
        .iter()
        .filter_map(|item| {
            let SelectionId::Event(event) = item.id else {
                return None;
            };
            Some((item.id.stable_key(), event))
        })
        .collect()
}

fn entity_event_evidence_from_inspector(
    entity: EntityId,
    inspector: &InspectorProjection,
    visible_events: &BTreeMap<String, EventId>,
) -> BTreeSet<EntityEventEvidence> {
    history_event_ids_from_inspector(inspector, ENTITY_HISTORY_SECTION, visible_events)
        .into_iter()
        .map(|event| EntityEventEvidence { entity, event })
        .collect()
}

fn relation_event_evidence_from_inspector(
    relation: RelationId,
    inspector: &InspectorProjection,
    visible_events: &BTreeMap<String, EventId>,
) -> BTreeSet<RelationEventEvidence> {
    history_event_ids_from_inspector(inspector, RELATION_HISTORY_SECTION, visible_events)
        .into_iter()
        .map(|event| RelationEventEvidence { relation, event })
        .collect()
}

fn entity_relation_evidence_from_inspector(
    relation: RelationId,
    inspector: &InspectorProjection,
    visible_entities: &BTreeMap<String, EntityId>,
) -> BTreeSet<EntityRelationEvidence> {
    let Some(section) = inspector
        .sections
        .iter()
        .find(|section| section.title == RELATION_ENDPOINTS_SECTION)
    else {
        return BTreeSet::new();
    };

    section
        .rows
        .iter()
        .filter_map(|row| {
            let role = match row.label.as_str() {
                "From" | "from" => RelationEndpointRole::From,
                "To" | "to" => RelationEndpointRole::To,
                _ => return None,
            };
            visible_entities
                .get(row.value.as_str())
                .copied()
                .map(|entity| EntityRelationEvidence {
                    entity,
                    relation,
                    role,
                })
        })
        .collect()
}

fn relation_identity_from_inspector(inspector: &InspectorProjection) -> Option<RelationIdentity> {
    let section = inspector
        .sections
        .iter()
        .find(|section| section.title == RELATION_IDENTITY_SECTION)?;
    let endpoint = |label: &str| {
        section
            .rows
            .iter()
            .find(|row| row.label == label)
            .and_then(|row| entity_id_from_stable_key(&row.value))
    };
    Some(RelationIdentity {
        from: endpoint("From")?,
        to: endpoint("To")?,
    })
}

fn entity_id_from_stable_key(key: &str) -> Option<EntityId> {
    match SelectionId::from_stable_key(key) {
        Some(SelectionId::Entity(entity)) => Some(entity),
        Some(SelectionId::Relation(_) | SelectionId::Event(_)) | None => None,
    }
}

fn history_event_ids_from_inspector(
    inspector: &InspectorProjection,
    section_title: &str,
    visible_events: &BTreeMap<String, EventId>,
) -> BTreeSet<EventId> {
    let Some(section) = inspector
        .sections
        .iter()
        .find(|section| section.title == section_title)
    else {
        return BTreeSet::new();
    };

    section
        .rows
        .iter()
        .filter_map(|row| visible_events.get(row.value.as_str()).copied())
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct BriefingProjection {
    pub eyebrow: String,
    pub title: String,
    pub items: Vec<BriefingItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BriefingItem {
    pub selection: Option<SelectionId>,
    pub title: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollectionProjection {
    pub title: String,
    pub items: Vec<CollectionItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollectionItem {
    pub id: SelectionId,
    pub title: String,
    pub subtitle: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelineProjection {
    pub items: Vec<TimelineItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineItem {
    pub id: SelectionId,
    pub world_time: u64,
    pub title: String,
    pub subtitle: String,
    pub caused_by: Vec<EventId>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanvasProjection {
    pub items: Vec<CanvasItem>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasItemKind {
    Place,
    Actor,
    Object,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasItem {
    pub id: SelectionId,
    pub kind: CanvasItemKind,
    pub label: String,
    pub detail: String,
    pub x: f32,
    pub y: f32,
}

/// One relation drawn between two things on the canvas.
///
/// The canvas has always known where things are and has never shown what
/// joins them, which left a World looking like a scattering of labels. The
/// endpoints are not inferred from prose: a relation's inspector carries its
/// identity section, so `from` and `to` are as structural as the positions.
///
/// Positions are carried in the same normalized space the canvas items use,
/// so a renderer applies one transform to both and cannot put an edge
/// somewhere its own endpoints are not.
///
/// Order follows relation id, which the inspector map already guarantees: it
/// is a `BTreeMap` and `SelectionId::Relation` sorts by id. A redraw
/// therefore never reshuffles the picture.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasEdge {
    pub relation: RelationId,
    pub from: EntityId,
    pub to: EntityId,
    pub label: String,
    pub from_x: f32,
    pub from_y: f32,
    pub to_x: f32,
    pub to_y: f32,
}

/// Where the canvas puts things, in the pixel space a renderer draws into.
///
/// The item boxes and the edges between them are placed from these same
/// numbers. Keeping them here rather than in the renderer is what lets the
/// geometry be tested at all: the renderer crate only compiles on macOS.
pub mod canvas_layout {
    /// One axis-aligned rectangle to fill.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Rect {
        pub left: f32,
        pub top: f32,
        pub width: f32,
        pub height: f32,
    }

    pub const ORIGIN_X: f32 = 14.0;
    pub const ORIGIN_Y: f32 = 12.0;
    pub const SPAN_X: f32 = 288.0;
    pub const SPAN_Y: f32 = 240.0;
    /// Narrower than the column pitch by enough for an edge to be seen
    /// running between two columns. At 130 the gutter was 14pt and a line in
    /// it read as a smudge.
    pub const ITEM_WIDTH: f32 = 112.0;
    /// Tall enough for the two lines a canvas item carries plus its padding.
    /// At 46 the second line was clipped.
    pub const ITEM_HEIGHT: f32 = 58.0;
    pub const EDGE_THICKNESS: f32 = 4.0;

    /// The clear space between two columns of boxes, which is all the room an
    /// edge has to be visible in once the boxes are drawn over it.
    pub fn gutter() -> f32 {
        SPAN_X / (COLUMNS - 1) as f32 - ITEM_WIDTH
    }

    /// The narrowest centre column the World window offers, at the smallest
    /// window size that has ever been screenshotted: a 1024pt window less the
    /// two side panels and the column's own padding. The canvas is placed in
    /// absolute pixels, so it does not grow or shrink with the window; what it
    /// must do is fit.
    pub const NARROWEST_COLUMN: f32 = 1024.0 - 220.0 - 300.0 - 24.0;

    /// The box the canvas needs, which is what the renderer reserves.
    pub fn extent() -> (f32, f32) {
        (
            ORIGIN_X * 2.0 + SPAN_X + ITEM_WIDTH,
            ORIGIN_Y * 2.0 + SPAN_Y + ITEM_HEIGHT,
        )
    }

    /// The top-left corner of the box drawn for an item at this position.
    pub fn item_corner(x: f32, y: f32) -> (f32, f32) {
        (ORIGIN_X + x * SPAN_X, ORIGIN_Y + y * SPAN_Y)
    }

    /// The centre of that box, which is where an edge meets it.
    pub fn anchor(x: f32, y: f32) -> (f32, f32) {
        let (left, top) = item_corner(x, y);
        (left + ITEM_WIDTH / 2.0, top + ITEM_HEIGHT / 2.0)
    }

    /// How many boxes fit across the canvas without touching.
    pub const COLUMNS: usize = 3;

    /// Whether any two boxes at these positions would overlap.
    pub fn any_overlap(positions: &[(f32, f32)]) -> bool {
        let corners = positions
            .iter()
            .map(|(x, y)| item_corner(*x, *y))
            .collect::<Vec<_>>();
        for (index, a) in corners.iter().enumerate() {
            for b in corners.iter().skip(index + 1) {
                if (a.0 - b.0).abs() < ITEM_WIDTH && (a.1 - b.1).abs() < ITEM_HEIGHT {
                    return true;
                }
            }
        }
        false
    }

    /// A deterministic grid for this many things, filled left to right.
    pub fn grid(count: usize) -> Vec<(f32, f32)> {
        let rows = count.div_ceil(COLUMNS).max(1);
        (0..count)
            .map(|index| {
                let x = if COLUMNS > 1 {
                    (index % COLUMNS) as f32 / (COLUMNS - 1) as f32
                } else {
                    0.5
                };
                let y = if rows > 1 {
                    (index / COLUMNS) as f32 / (rows - 1) as f32
                } else {
                    0.5
                };
                (x, y)
            })
            .collect()
    }

    /// Where to actually draw things.
    ///
    /// A Pack's own positions are a picture of its World and are used
    /// whenever they can be read. When any two of them would overlap at the
    /// size the boxes are actually drawn, the whole set falls back to a grid:
    /// every Pack writes its own table of coordinates by hand, none of them
    /// knew the box size, and a legible grid beats an artful pile.
    ///
    /// All or nothing, deliberately. Nudging only the offending boxes would
    /// leave a layout that is neither the Pack's arrangement nor a grid.
    pub fn placements(positions: &[(f32, f32)]) -> Vec<(f32, f32)> {
        if any_overlap(positions) {
            grid(positions.len())
        } else {
            positions.to_vec()
        }
    }

    /// An edge routed as three right-angle segments: out, across, in.
    ///
    /// Right angles rather than a diagonal because an axis-aligned rectangle
    /// is the only shape a plain element tree can fill, and because a tidy
    /// elbow suits this app better than a hairline. A segment is never
    /// thinner than the stroke, so two things at the same height still show a
    /// visible join rather than nothing.
    pub fn edge_segments(from: (f32, f32), to: (f32, f32)) -> [Rect; 3] {
        let (x1, y1) = anchor(from.0, from.1);
        let (x2, y2) = anchor(to.0, to.1);
        let mid_x = (x1 + x2) / 2.0;
        let half = EDGE_THICKNESS / 2.0;
        [
            Rect {
                left: x1.min(mid_x),
                top: y1 - half,
                width: (mid_x - x1).abs().max(EDGE_THICKNESS),
                height: EDGE_THICKNESS,
            },
            Rect {
                left: mid_x - half,
                top: y1.min(y2),
                width: EDGE_THICKNESS,
                height: (y2 - y1).abs().max(EDGE_THICKNESS),
            },
            Rect {
                left: mid_x.min(x2),
                top: y2 - half,
                width: (x2 - mid_x).abs().max(EDGE_THICKNESS),
                height: EDGE_THICKNESS,
            },
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorProjection {
    pub selection: SelectionId,
    pub title: String,
    pub subtitle: String,
    pub sections: Vec<InspectorSection>,
}

impl InspectorProjection {
    pub fn display_sections(&self) -> impl Iterator<Item = &InspectorSection> {
        self.sections.iter().filter(|section| {
            !matches!(
                section.title.as_str(),
                ENTITY_HISTORY_SECTION
                    | RELATION_HISTORY_SECTION
                    | RELATION_ENDPOINTS_SECTION
                    | RELATION_IDENTITY_SECTION
            )
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorSection {
    pub title: String,
    pub rows: Vec<InspectorRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorRow {
    pub label: String,
    pub value: String,
}

pub fn timeline_from_world(world: &World) -> TimelineProjection {
    TimelineProjection {
        items: world
            .events()
            .iter()
            .rev()
            .map(|event| TimelineItem {
                id: SelectionId::Event(event.id),
                world_time: event.world_time,
                title: humanize(&event.kind),
                subtitle: event_summary(event, world),
                caused_by: event.caused_by.clone(),
            })
            .collect(),
    }
}

pub fn inspectors_from_world(world: &World) -> BTreeMap<SelectionId, InspectorProjection> {
    let recorded_change_events = recorded_entity_change_events(world);
    let recorded_relations = recorded_relation_incarnations(world);
    let mut inspectors = BTreeMap::new();
    for entity in world.state().entities() {
        inspectors.insert(
            SelectionId::Entity(entity.id),
            inspector_for_entity(entity, world, &recorded_change_events),
        );
    }
    for recorded in recorded_relations.values() {
        inspectors.insert(
            SelectionId::Relation(recorded.relation.id),
            inspector_for_relation(recorded, world),
        );
    }
    for event in world.events() {
        inspectors.insert(
            SelectionId::Event(event.id),
            inspector_for_event(event, world),
        );
    }
    inspectors
}

pub fn entity_title(entity: &Entity) -> String {
    match entity.component("name") {
        Some(Value::Text(name)) => name.clone(),
        _ => format!("{} #{}", humanize(&entity.kind), entity.id),
    }
}

pub fn value_text(value: &Value, world: &World) -> String {
    match value {
        Value::Null => "—".into(),
        Value::Bool(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Text(value) => value.clone(),
        Value::Entity(id) => world
            .state()
            .entity(*id)
            .map(entity_title)
            .unwrap_or_else(|| format!("Entity #{id}")),
        Value::List(values) => values
            .iter()
            .map(|value| value_text(value, world))
            .collect::<Vec<_>>()
            .join(", "),
        Value::Map(values) => values
            .iter()
            .map(|(key, value)| format!("{key}: {}", value_text(value, world)))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn inspector_for_entity(
    entity: &Entity,
    world: &World,
    recorded_change_events: &BTreeMap<EntityId, Vec<EventId>>,
) -> InspectorProjection {
    let components = entity
        .components
        .iter()
        .filter(|(key, _)| key.as_str() != "name")
        .map(|(key, value)| InspectorRow {
            label: humanize(key),
            value: value_text(value, world),
        })
        .collect::<Vec<_>>();

    let relations = world
        .state()
        .relations()
        .filter(|relation| relation.from == entity.id || relation.to == entity.id)
        .map(|relation| {
            let other = if relation.from == entity.id {
                relation.to
            } else {
                relation.from
            };
            let other = world
                .state()
                .entity(other)
                .map(entity_title)
                .unwrap_or_else(|| format!("Entity #{other}"));
            InspectorRow {
                label: humanize(&relation.kind),
                value: other,
            }
        })
        .collect::<Vec<_>>();
    let recorded_changes = recorded_entity_change_rows(entity.id, world, recorded_change_events);

    let mut sections = vec![InspectorSection {
        title: "State".into(),
        rows: components,
    }];
    if !relations.is_empty() {
        sections.push(InspectorSection {
            title: "Relations".into(),
            rows: relations,
        });
    }
    if !recorded_changes.is_empty() {
        sections.push(InspectorSection {
            title: ENTITY_HISTORY_SECTION.into(),
            rows: recorded_changes,
        });
    }

    InspectorProjection {
        selection: SelectionId::Entity(entity.id),
        title: entity_title(entity),
        subtitle: humanize(&entity.kind),
        sections,
    }
}

fn recorded_entity_change_rows(
    entity: EntityId,
    world: &World,
    recorded_change_events: &BTreeMap<EntityId, Vec<EventId>>,
) -> Vec<InspectorRow> {
    let Some(event_ids) = recorded_change_events.get(&entity) else {
        return Vec::new();
    };

    event_ids
        .iter()
        .rev()
        .filter_map(|event_id| world.event(*event_id))
        .map(|event| InspectorRow {
            label: format!(
                "World time {} · {}",
                event.world_time,
                humanize(&event.kind)
            ),
            value: SelectionId::Event(event.id).stable_key(),
        })
        .collect()
}

fn recorded_entity_change_events(world: &World) -> BTreeMap<EntityId, Vec<EventId>> {
    let mut relation_endpoints = world
        .baseline_state()
        .relations()
        .map(|relation| (relation.id, (relation.from, relation.to)))
        .collect::<BTreeMap<RelationId, (EntityId, EntityId)>>();
    let mut events_by_entity = BTreeMap::<EntityId, Vec<EventId>>::new();

    for event in world.events() {
        let mut affected = BTreeSet::new();
        for change in &event.changes {
            match change {
                StateChange::CreateEntity(entity) => {
                    events_by_entity.remove(&entity.id);
                    affected.insert(entity.id);
                }
                StateChange::RemoveEntity(entity) => {
                    affected.insert(*entity);
                    for (from, to) in relation_endpoints.values().copied() {
                        if from == *entity {
                            affected.insert(to);
                        }
                        if to == *entity {
                            affected.insert(from);
                        }
                    }
                    relation_endpoints.retain(|_, (from, to)| *from != *entity && *to != *entity);
                }
                StateChange::SetComponent { entity, .. }
                | StateChange::RemoveComponent { entity, .. } => {
                    affected.insert(*entity);
                }
                StateChange::CreateRelation(relation) => {
                    affected.insert(relation.from);
                    affected.insert(relation.to);
                    relation_endpoints.insert(relation.id, (relation.from, relation.to));
                }
                StateChange::RemoveRelation(relation) => {
                    if let Some((from, to)) = relation_endpoints.remove(relation) {
                        affected.insert(from);
                        affected.insert(to);
                    }
                }
                StateChange::SetRelationProperty { relation, .. }
                | StateChange::RemoveRelationProperty { relation, .. } => {
                    if let Some((from, to)) = relation_endpoints.get(relation).copied() {
                        affected.insert(from);
                        affected.insert(to);
                    }
                }
            }
        }

        for entity in affected {
            events_by_entity.entry(entity).or_default().push(event.id);
        }
    }

    events_by_entity
}
#[derive(Clone, Debug)]
struct RecordedRelationIncarnation {
    relation: Relation,
    active: bool,
    event_ids: Vec<EventId>,
}

fn recorded_relation_incarnations(
    world: &World,
) -> BTreeMap<RelationId, RecordedRelationIncarnation> {
    let mut relations = world
        .baseline_state()
        .relations()
        .map(|relation| {
            (
                relation.id,
                RecordedRelationIncarnation {
                    relation: relation.clone(),
                    active: true,
                    event_ids: Vec::new(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    for event in world.events() {
        let mut affected = BTreeSet::new();
        for change in &event.changes {
            match change {
                StateChange::CreateRelation(relation) => {
                    relations.insert(
                        relation.id,
                        RecordedRelationIncarnation {
                            relation: relation.clone(),
                            active: true,
                            event_ids: Vec::new(),
                        },
                    );
                    affected.insert(relation.id);
                }
                StateChange::RemoveRelation(relation) => {
                    if let Some(recorded) = relations.get_mut(relation) {
                        if recorded.active {
                            recorded.active = false;
                            affected.insert(*relation);
                        }
                    }
                }
                StateChange::SetRelationProperty {
                    relation,
                    key,
                    value,
                } => {
                    if let Some(recorded) = relations.get_mut(relation) {
                        if recorded.active {
                            recorded
                                .relation
                                .properties
                                .insert(key.clone(), value.clone());
                            affected.insert(*relation);
                        }
                    }
                }
                StateChange::RemoveRelationProperty { relation, key } => {
                    if let Some(recorded) = relations.get_mut(relation) {
                        if recorded.active {
                            recorded.relation.properties.remove(key);
                            affected.insert(*relation);
                        }
                    }
                }
                StateChange::RemoveEntity(entity) => {
                    let removed = relations
                        .iter()
                        .filter_map(|(relation, recorded)| {
                            (recorded.active
                                && (recorded.relation.from == *entity
                                    || recorded.relation.to == *entity))
                                .then_some(*relation)
                        })
                        .collect::<Vec<_>>();
                    for relation in removed {
                        if let Some(recorded) = relations.get_mut(&relation) {
                            recorded.active = false;
                            affected.insert(relation);
                        }
                    }
                }
                StateChange::CreateEntity(_)
                | StateChange::SetComponent { .. }
                | StateChange::RemoveComponent { .. } => {}
            }
        }

        for relation in affected {
            if let Some(recorded) = relations.get_mut(&relation) {
                recorded.event_ids.push(event.id);
            }
        }
    }

    relations
}

fn inspector_for_relation(
    recorded: &RecordedRelationIncarnation,
    world: &World,
) -> InspectorProjection {
    let relation = &recorded.relation;
    let relation_rows = vec![
        InspectorRow {
            label: "From".into(),
            value: relation_endpoint_text(relation.from, world),
        },
        InspectorRow {
            label: "To".into(),
            value: relation_endpoint_text(relation.to, world),
        },
        InspectorRow {
            label: "Status".into(),
            value: if recorded.active { "Active" } else { "Removed" }.into(),
        },
    ];
    let properties = relation
        .properties
        .iter()
        .map(|(key, value)| InspectorRow {
            label: humanize(key),
            value: recorded_value_text(value),
        })
        .collect::<Vec<_>>();
    let recorded_changes = recorded_relation_change_rows(recorded, world);

    let mut sections = vec![InspectorSection {
        title: "Relation".into(),
        rows: relation_rows,
    }];
    if !properties.is_empty() {
        sections.push(InspectorSection {
            title: "Properties".into(),
            rows: properties,
        });
    }
    sections.push(InspectorSection {
        title: RELATION_IDENTITY_SECTION.into(),
        rows: vec![
            InspectorRow {
                label: "From".into(),
                value: SelectionId::Entity(relation.from).stable_key(),
            },
            InspectorRow {
                label: "To".into(),
                value: SelectionId::Entity(relation.to).stable_key(),
            },
        ],
    });
    if recorded.active {
        sections.push(InspectorSection {
            title: RELATION_ENDPOINTS_SECTION.into(),
            rows: vec![
                InspectorRow {
                    label: "From".into(),
                    value: SelectionId::Entity(relation.from).stable_key(),
                },
                InspectorRow {
                    label: "To".into(),
                    value: SelectionId::Entity(relation.to).stable_key(),
                },
            ],
        });
    }
    if !recorded_changes.is_empty() {
        sections.push(InspectorSection {
            title: RELATION_HISTORY_SECTION.into(),
            rows: recorded_changes,
        });
    }

    InspectorProjection {
        selection: SelectionId::Relation(relation.id),
        title: humanize(&relation.kind),
        subtitle: format!(
            "Relation #{} · {}",
            relation.id,
            if recorded.active { "Active" } else { "Removed" }
        ),
        sections,
    }
}

fn relation_endpoint_text(entity: EntityId, world: &World) -> String {
    world
        .state()
        .entity(entity)
        .map(|entity| format!("{} · Entity #{}", entity_title(entity), entity.id))
        .unwrap_or_else(|| format!("Entity #{entity}"))
}

fn recorded_relation_change_rows(
    recorded: &RecordedRelationIncarnation,
    world: &World,
) -> Vec<InspectorRow> {
    recorded
        .event_ids
        .iter()
        .rev()
        .filter_map(|event_id| world.event(*event_id))
        .map(|event| InspectorRow {
            label: format!(
                "World time {} · {}",
                event.world_time,
                humanize(&event.kind)
            ),
            value: SelectionId::Event(event.id).stable_key(),
        })
        .collect()
}

fn inspector_for_event(event: &Event, world: &World) -> InspectorProjection {
    let mut context = Vec::new();
    if let Some(actor) = event.actor {
        context.push(InspectorRow {
            label: "Actor".into(),
            value: world
                .state()
                .entity(actor)
                .map(entity_title)
                .unwrap_or_else(|| format!("Entity #{actor}")),
        });
    }
    if !event.targets.is_empty() {
        context.push(InspectorRow {
            label: "Targets".into(),
            value: event
                .targets
                .iter()
                .map(|id| {
                    world
                        .state()
                        .entity(*id)
                        .map(entity_title)
                        .unwrap_or_else(|| format!("Entity #{id}"))
                })
                .collect::<Vec<_>>()
                .join(", "),
        });
    }
    if !event.caused_by.is_empty() {
        context.push(InspectorRow {
            label: "Caused by".into(),
            value: event
                .caused_by
                .iter()
                .map(|id| format!("Event #{id}"))
                .collect::<Vec<_>>()
                .join(", "),
        });
    }

    let payload = event
        .payload
        .iter()
        .map(|(key, value)| InspectorRow {
            label: humanize(key),
            value: recorded_value_text(value),
        })
        .collect::<Vec<_>>();
    let changes = event.changes.iter().map(change_row).collect::<Vec<_>>();

    let mut sections = vec![InspectorSection {
        title: "Context".into(),
        rows: context,
    }];
    if !payload.is_empty() {
        sections.push(InspectorSection {
            title: "Payload".into(),
            rows: payload,
        });
    }
    if !changes.is_empty() {
        sections.push(InspectorSection {
            title: "Changes".into(),
            rows: changes,
        });
    }

    InspectorProjection {
        selection: SelectionId::Event(event.id),
        title: humanize(&event.kind),
        subtitle: format!("World time {} · Event #{}", event.world_time, event.id),
        sections,
    }
}

fn change_row(change: &StateChange) -> InspectorRow {
    match change {
        StateChange::CreateEntity(entity) => InspectorRow {
            label: "Create entity".into(),
            value: entity_title(entity),
        },
        StateChange::RemoveEntity(entity) => InspectorRow {
            label: "Remove entity".into(),
            value: format!("Entity #{entity}"),
        },
        StateChange::SetComponent { entity, key, value } => InspectorRow {
            label: format!("Entity #{entity} · {}", humanize(key)),
            value: recorded_value_text(value),
        },
        StateChange::RemoveComponent { entity, key } => InspectorRow {
            label: format!("Entity #{entity} · {}", humanize(key)),
            value: "Removed".into(),
        },
        StateChange::CreateRelation(relation) => InspectorRow {
            label: "Create relation".into(),
            value: format!(
                "{} · Entity #{} → Entity #{}",
                humanize(&relation.kind),
                relation.from,
                relation.to
            ),
        },
        StateChange::RemoveRelation(relation) => InspectorRow {
            label: "Remove relation".into(),
            value: format!("Relation #{relation}"),
        },
        StateChange::SetRelationProperty {
            relation,
            key,
            value,
        } => InspectorRow {
            label: format!("Relation #{relation} · {}", humanize(key)),
            value: recorded_value_text(value),
        },
        StateChange::RemoveRelationProperty { relation, key } => InspectorRow {
            label: format!("Relation #{relation} · {}", humanize(key)),
            value: "Removed".into(),
        },
    }
}

fn recorded_value_text(value: &Value) -> String {
    match value {
        Value::Null => "—".into(),
        Value::Bool(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Text(value) => value.clone(),
        Value::Entity(entity) => format!("Entity #{entity}"),
        Value::List(values) => values
            .iter()
            .map(recorded_value_text)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Map(values) => values
            .iter()
            .map(|(key, value)| format!("{key}: {}", recorded_value_text(value)))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn semantic_event_summary(event: &Event) -> Option<&str> {
    ["summary", "change"]
        .into_iter()
        .find_map(|key| match event.payload.get(key) {
            Some(Value::Text(value)) if !value.trim().is_empty() => Some(value.as_str()),
            _ => None,
        })
}

pub(crate) fn event_summary(event: &Event, world: &World) -> String {
    let mut parts = Vec::new();
    if let Some(actor) = event
        .actor
        .and_then(|id| world.state().entity(id))
        .map(entity_title)
    {
        parts.push(actor);
    }
    if let Some(summary) = semantic_event_summary(event) {
        parts.push(summary.to_string());
    }
    parts.push(format!("Event #{}", event.id));
    parts.join(" · ")
}

pub(crate) fn humanize(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_core::{Entity, Event, EventId, StateChange, WorldState};

    fn sample_world() -> World {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId::new(1), "workspace")
                    .with_component("name", "Workspace")
                    .with_component("status", "active"),
            )
            .unwrap();
        World::from_history(
            state,
            &[Event {
                id: EventId::new(1),
                kind: "work_started".into(),
                world_time: 0,
                actor: Some(EntityId::new(1)),
                targets: vec![],
                caused_by: vec![],
                payload: BTreeMap::new(),
                changes: vec![],
            }],
        )
        .unwrap()
    }

    /// A World with two entities joined by one relation, projected by the
    /// real producer so the identity section under test is the real one.
    fn related_world() -> World {
        use world_core::Relation;
        let mut state = WorldState::default();
        state
            .seed_entity(Entity::new(EntityId::new(1), "person").with_component("name", "Nia"))
            .unwrap();
        state
            .seed_entity(Entity::new(EntityId::new(2), "person").with_component("name", "Tomas"))
            .unwrap();
        // Seeded high id first, so a test on ordering is not satisfied by the
        // order they happen to be written in.
        state
            .seed_relation(Relation::new(
                RelationId::new(7),
                "partnership",
                EntityId::new(1),
                EntityId::new(2),
            ))
            .unwrap();
        state
            .seed_relation(Relation::new(
                RelationId::new(3),
                "rivalry",
                EntityId::new(2),
                EntityId::new(1),
            ))
            .unwrap();
        World::from_history(state, &[]).unwrap()
    }

    fn placed(id: u64, x: f32, y: f32) -> CanvasItem {
        CanvasItem {
            id: SelectionId::Entity(EntityId::new(id)),
            kind: CanvasItemKind::Actor,
            label: format!("entity {id}"),
            detail: String::new(),
            x,
            y,
        }
    }

    fn snapshot_with(items: Vec<CanvasItem>) -> ProjectionSnapshot {
        ProjectionSnapshot {
            inspectors: inspectors_from_world(&related_world()),
            canvas: CanvasProjection { items },
            ..ProjectionSnapshot::default()
        }
    }

    #[test]
    fn a_layout_that_can_be_read_is_left_alone() {
        use canvas_layout::*;
        // Three things across the top: no two boxes touch, so the Pack's own
        // arrangement survives.
        let asked = vec![(0.0, 0.0), (0.5, 0.0), (1.0, 0.0)];
        assert!(!any_overlap(&asked));
        assert_eq!(placements(&asked), asked);
    }

    #[test]
    fn a_layout_that_cannot_be_read_becomes_a_grid() {
        use canvas_layout::*;
        // Two things in almost the same place: unreadable however the Pack
        // meant it.
        let asked = vec![(0.5, 0.5), (0.52, 0.5), (0.0, 0.0)];
        assert!(any_overlap(&asked));
        let drawn = placements(&asked);
        assert_ne!(drawn, asked);
        assert!(!any_overlap(&drawn), "{drawn:?}");
        assert_eq!(drawn.len(), asked.len());
    }

    #[test]
    fn the_grid_keeps_things_apart_and_on_the_canvas() {
        use canvas_layout::*;
        // The counts a World realistically has. Beyond this the canvas is
        // simply too small, which is a different problem from a broken rule.
        for count in 1..=15 {
            let drawn = grid(count);
            assert_eq!(drawn.len(), count);
            assert!(!any_overlap(&drawn), "count {count}: {drawn:?}");
            for (x, y) in &drawn {
                assert!((0.0..=1.0).contains(x) && (0.0..=1.0).contains(y));
            }
        }
    }

    #[test]
    fn an_edge_reads_the_positions_the_boxes_are_drawn_at() {
        // When the Pack's layout is replaced, the edge must move with the
        // boxes rather than pointing at where they used to be.
        let piled = snapshot_with(vec![placed(1, 0.5, 0.5), placed(2, 0.51, 0.5)]);
        let drawn = piled.canvas_placements();
        let edge = piled
            .canvas_edges()
            .into_iter()
            .find(|edge| edge.relation == RelationId::new(7))
            .expect("the partnership is drawable");

        let position = |entity: EntityId| {
            drawn
                .iter()
                .find(|(selection, _, _)| *selection == SelectionId::Entity(entity))
                .map(|(_, x, y)| (*x, *y))
                .unwrap()
        };
        assert_eq!((edge.from_x, edge.from_y), position(edge.from));
        assert_eq!((edge.to_x, edge.to_y), position(edge.to));
        assert_ne!((edge.from_x, edge.from_y), (0.5, 0.5));
    }

    #[test]
    fn an_edge_has_room_to_be_seen_between_two_columns() {
        use canvas_layout::*;
        // Boxes are drawn over the edges, so the only part of an edge that
        // shows is the run between two columns. If that gap is not
        // comfortably wider than the stroke, the edge reads as a smudge
        // rather than a connection.
        assert!(
            gutter() >= EDGE_THICKNESS * 4.0,
            "a {}pt gutter cannot show a {EDGE_THICKNESS}pt edge",
            gutter()
        );
    }

    #[test]
    fn the_canvas_fits_the_narrowest_column_it_is_drawn_in() {
        use canvas_layout::*;
        let (width, _) = extent();
        assert!(
            width <= NARROWEST_COLUMN,
            "the canvas needs {width}pt and the column offers {NARROWEST_COLUMN}pt, \
             so items at the right edge would be cut off"
        );
        // And every placeable position stays inside that box.
        for (x, y) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
            let (left, top) = item_corner(x, y);
            let (w, h) = extent();
            assert!(left >= 0.0 && left + ITEM_WIDTH <= w, "x={x}");
            assert!(top >= 0.0 && top + ITEM_HEIGHT <= h, "y={y}");
        }
    }

    #[test]
    fn an_edge_starts_and_ends_on_the_boxes_it_joins() {
        use canvas_layout::*;
        let from = (0.1_f32, 0.2_f32);
        let to = (0.7_f32, 0.9_f32);
        let segments = edge_segments(from, to);
        let (ax, ay) = anchor(from.0, from.1);
        let (bx, by) = anchor(to.0, to.1);

        // The first segment touches the source anchor and the last touches
        // the target anchor: an edge that floats free of its own endpoints is
        // the one failure a picture cannot survive.
        let touches = |rect: Rect, x: f32, y: f32| {
            x >= rect.left - 0.01
                && x <= rect.left + rect.width + 0.01
                && y >= rect.top - 0.01
                && y <= rect.top + rect.height + 0.01
        };
        assert!(touches(segments[0], ax, ay), "{:?}", segments[0]);
        assert!(touches(segments[2], bx, by), "{:?}", segments[2]);

        // And the three segments form one connected run.
        assert!(touches(
            segments[1],
            segments[0].left + segments[0].width,
            ay
        ));
        assert!(touches(segments[1], segments[2].left, by));
    }

    #[test]
    fn an_edge_between_two_things_at_the_same_height_is_still_visible() {
        use canvas_layout::*;
        let segments = edge_segments((0.1, 0.5), (0.8, 0.5));
        for segment in segments {
            assert!(
                segment.width >= EDGE_THICKNESS && segment.height >= EDGE_THICKNESS,
                "{segment:?}"
            );
        }
    }

    #[test]
    fn an_item_box_and_its_anchor_agree() {
        use canvas_layout::*;
        let (left, top) = item_corner(0.4, 0.6);
        let (x, y) = anchor(0.4, 0.6);
        assert!(x > left && x < left + ITEM_WIDTH);
        assert!(y > top && y < top + ITEM_HEIGHT);
    }

    #[test]
    fn an_edge_carries_the_positions_of_the_two_things_it_joins() {
        let snapshot = snapshot_with(vec![placed(1, 0.2, 0.8), placed(2, 0.6, 0.1)]);
        let edges = snapshot.canvas_edges();

        assert_eq!(edges.len(), 2, "{edges:?}");
        let edge = edges
            .iter()
            .find(|edge| edge.relation == RelationId::new(7))
            .expect("the partnership is drawable");
        assert_eq!(edge.from, EntityId::new(1));
        assert_eq!(edge.to, EntityId::new(2));
        assert_eq!((edge.from_x, edge.from_y), (0.2, 0.8));
        assert_eq!((edge.to_x, edge.to_y), (0.6, 0.1));
        assert!(!edge.label.is_empty());
    }

    #[test]
    fn a_relation_the_canvas_does_not_place_is_left_out_rather_than_anchored() {
        // Only one end is on the canvas: drawing it would put a line at a
        // position the World never claimed.
        let only_one_end = snapshot_with(vec![placed(1, 0.2, 0.8)]);
        assert!(only_one_end.canvas_edges().is_empty());

        let neither_end = snapshot_with(Vec::new());
        assert!(neither_end.canvas_edges().is_empty());
    }

    #[test]
    fn edges_are_ordered_by_relation_so_a_redraw_does_not_reshuffle_them() {
        let snapshot = snapshot_with(vec![placed(1, 0.2, 0.8), placed(2, 0.6, 0.1)]);
        let once = snapshot.canvas_edges();
        let again = snapshot.canvas_edges();
        assert_eq!(once, again);
        assert_eq!(
            once.len(),
            2,
            "the fixture must have enough edges to reorder"
        );
        assert!(once
            .windows(2)
            .all(|pair| pair[0].relation < pair[1].relation));
    }

    #[test]
    fn every_drawable_relation_is_drawn() {
        // The count the renderer must not silently reduce: every relation with
        // both ends placed appears exactly once.
        let snapshot = snapshot_with(vec![placed(1, 0.0, 0.0), placed(2, 1.0, 1.0)]);
        let drawable = snapshot
            .inspectors
            .keys()
            .filter(|selection| matches!(selection, SelectionId::Relation(_)))
            .count();
        assert_eq!(snapshot.canvas_edges().len(), drawable);
    }

    #[test]
    fn generic_projection_builds_timeline_and_inspectors() {
        let world = sample_world();
        let timeline = timeline_from_world(&world);
        let inspectors = inspectors_from_world(&world);

        assert_eq!(timeline.items.len(), 1);
        assert_eq!(timeline.items[0].title, "Work Started");
        assert_eq!(timeline.items[0].subtitle, "Workspace · Event #1");
        assert_eq!(
            inspectors
                .get(&SelectionId::Entity(EntityId::new(1)))
                .unwrap()
                .title,
            "Workspace"
        );
        assert!(inspectors.contains_key(&SelectionId::Event(EventId::new(1))));
    }

    #[test]
    fn timeline_surfaces_semantic_event_payload_without_domain_knowledge() {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId::new(1), "workspace").with_component("name", "Workspace"),
            )
            .unwrap();
        let world = World::from_history(
            state,
            &[Event {
                id: EventId::new(1),
                kind: "direction_chosen".into(),
                world_time: 0,
                actor: None,
                targets: vec![EntityId::new(1)],
                caused_by: vec![],
                payload: BTreeMap::from([
                    ("change".into(), Value::Text("fallback detail".into())),
                    (
                        "summary".into(),
                        Value::Text("A durable direction was chosen.".into()),
                    ),
                ]),
                changes: vec![],
            }],
        )
        .unwrap();

        let timeline = timeline_from_world(&world);
        assert_eq!(
            timeline.items[0].subtitle,
            "A durable direction was chosen. · Event #1"
        );
    }

    #[test]
    fn event_inspector_surfaces_recorded_state_changes() {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId::new(1), "workspace")
                    .with_component("name", "Workspace")
                    .with_component("status", "active"),
            )
            .unwrap();
        let world = World::from_history(
            state,
            &[Event {
                id: EventId::new(1),
                kind: "work_finished".into(),
                world_time: 1,
                actor: None,
                targets: vec![EntityId::new(1)],
                caused_by: vec![],
                payload: BTreeMap::new(),
                changes: vec![StateChange::SetComponent {
                    entity: EntityId::new(1),
                    key: "status".into(),
                    value: Value::Text("done".into()),
                }],
            }],
        )
        .unwrap();

        let inspectors = inspectors_from_world(&world);
        let changes = inspectors
            .get(&SelectionId::Event(EventId::new(1)))
            .unwrap()
            .sections
            .iter()
            .find(|section| section.title == "Changes")
            .expect("recorded StateChanges should be inspectable");
        assert_eq!(changes.rows.len(), 1);
        assert_eq!(changes.rows[0].label, "Entity #1 · Status");
        assert_eq!(changes.rows[0].value, "done");
    }

    #[test]
    fn entity_history_uses_stable_event_keys_and_typed_timeline_lookup() {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId::new(1), "workspace")
                    .with_component("name", "Workspace")
                    .with_component("status", "active"),
            )
            .unwrap();
        state
            .seed_entity(
                Entity::new(EntityId::new(2), "worker")
                    .with_component("name", "Worker")
                    .with_component("status", "idle"),
            )
            .unwrap();
        let world = World::from_history(
            state,
            &[
                Event {
                    id: EventId::new(1),
                    kind: "work_finished".into(),
                    world_time: 1,
                    actor: None,
                    targets: vec![EntityId::new(1)],
                    caused_by: vec![],
                    payload: BTreeMap::new(),
                    changes: vec![StateChange::SetComponent {
                        entity: EntityId::new(1),
                        key: "status".into(),
                        value: Value::Text("done".into()),
                    }],
                },
                Event {
                    id: EventId::new(2),
                    kind: "worker_changed".into(),
                    world_time: 2,
                    actor: None,
                    targets: vec![EntityId::new(2)],
                    caused_by: vec![],
                    payload: BTreeMap::new(),
                    changes: vec![StateChange::SetComponent {
                        entity: EntityId::new(2),
                        key: "status".into(),
                        value: Value::Text("busy".into()),
                    }],
                },
                Event {
                    id: EventId::new(3),
                    kind: "workspace_renamed".into(),
                    world_time: 3,
                    actor: None,
                    targets: vec![EntityId::new(1)],
                    caused_by: vec![EventId::new(1)],
                    payload: BTreeMap::new(),
                    changes: vec![StateChange::SetComponent {
                        entity: EntityId::new(1),
                        key: "name".into(),
                        value: Value::Text("Renamed Workspace".into()),
                    }],
                },
            ],
        )
        .unwrap();

        let inspectors = inspectors_from_world(&world);
        let recorded = inspectors
            .get(&SelectionId::Entity(EntityId::new(1)))
            .unwrap()
            .sections
            .iter()
            .find(|section| section.title == ENTITY_HISTORY_SECTION)
            .expect("history section should exist");
        assert_eq!(recorded.rows[0].label, "World time 3 · Workspace Renamed");
        assert_eq!(recorded.rows[0].value, "event-3");
        assert_eq!(recorded.rows[1].value, "event-1");

        let snapshot = ProjectionSnapshot {
            timeline: timeline_from_world(&world),
            inspectors,
            ..ProjectionSnapshot::default()
        };
        let history = snapshot.entity_history(EntityId::new(1));
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].id, SelectionId::Event(EventId::new(3)));
        assert_eq!(history[1].id, SelectionId::Event(EventId::new(1)));
    }

    #[test]
    fn event_payload_entity_references_are_recorded_not_current_state_derived() {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId::new(1), "workspace").with_component("name", "Workspace"),
            )
            .unwrap();
        let world = World::from_history(
            state,
            &[Event {
                id: EventId::new(1),
                kind: "workspace_renamed".into(),
                world_time: 1,
                actor: None,
                targets: vec![],
                caused_by: vec![],
                payload: BTreeMap::from([("subject".into(), Value::Entity(EntityId::new(1)))]),
                changes: vec![StateChange::SetComponent {
                    entity: EntityId::new(1),
                    key: "name".into(),
                    value: Value::Text("Renamed Workspace".into()),
                }],
            }],
        )
        .unwrap();

        let inspectors = inspectors_from_world(&world);
        let payload = inspectors
            .get(&SelectionId::Event(EventId::new(1)))
            .unwrap()
            .sections
            .iter()
            .find(|section| section.title == "Payload")
            .expect("recorded payload should be inspectable");

        assert_eq!(payload.rows.len(), 1);
        assert_eq!(payload.rows[0].label, "Subject");
        assert_eq!(payload.rows[0].value, "Entity #1");
        assert_eq!(
            inspectors
                .get(&SelectionId::Entity(EntityId::new(1)))
                .unwrap()
                .title,
            "Renamed Workspace"
        );
    }

    #[test]
    fn snapshot_command_lookup_is_generic() {
        let snapshot = ProjectionSnapshot {
            commands: vec![ProjectionCommand {
                id: "world.continue".into(),
                title: "Continue".into(),
                detail: "Let the world keep running".into(),
            }],
            ..ProjectionSnapshot::default()
        };

        assert_eq!(
            snapshot
                .command("world.continue")
                .map(|command| command.title.as_str()),
            Some("Continue")
        );
        assert!(snapshot.command("missing").is_none());
        assert!(!snapshot.capabilities.fork);
    }
}
