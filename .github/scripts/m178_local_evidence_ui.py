from pathlib import Path

path = Path("crates/world-strategy-gpui/src/lib.rs")
text = path.read_text()
text = text.replace(
    "    compare_divergence, ChangedCommand, ChangedTimelineItem, DifferenceKind, DivergenceImpactStage,\n    DivergenceSide, EntityDifference, RelationDifference, SnapshotComparison, SnapshotDivergence,\n",
    "    compare_divergence, compare_evidence_neighborhoods, ChangedCommand, ChangedTimelineItem,\n    DifferenceKind, DivergenceImpactStage, DivergenceSide, EntityDifference,\n    EvidenceNeighborhoodComparison, EvidenceNeighborhoodNodeDifference, RelationDifference,\n    SnapshotComparison, SnapshotDivergence,\n",
    1,
)
text = text.replace(
    "    InspectorProjection, ProjectionCommand, ProjectionSnapshot, SelectionId, TimelineItem, WhyNode,\n",
    "    InspectorProjection, ProjectionCommand, ProjectionSnapshot, RelationEndpointRole, SelectionId,\n    StateEvidenceEdge, TimelineItem, WhyNode,\n",
    1,
)
text = text.replace(
    "const EVENT_RELATION_EFFECT_LIMIT: usize = 6;\n",
    "const EVENT_RELATION_EFFECT_LIMIT: usize = 6;\nconst LOCAL_EVIDENCE_DEPTH: usize = 2;\nconst LOCAL_EVIDENCE_NODE_LIMIT: usize = 8;\nconst LOCAL_EVIDENCE_EDGE_LIMIT_PER_SIDE: usize = 6;\n",
    1,
)

old = """            .child(self.render_evidence_inspector(inspector));

        if let SelectionId::Entity(entity) = selected.selection {
"""
new = """            .child(self.render_evidence_inspector(inspector));

        if let Some(local) = self
            .local_evidence_comparison(selected.selection)
            .filter(|comparison| !comparison.is_identical())
        {
            panel = panel.child(self.render_local_evidence_divergence(&local, cx));
        }

        if let SelectionId::Entity(entity) = selected.selection {
"""
if text.count(old) != 1:
    raise SystemExit(f"expected selected evidence insertion once, found {text.count(old)}")
text = text.replace(old, new, 1)

marker = """    fn render_evidence_inspector(&self, inspector: &InspectorProjection) -> Div {
"""
helpers = r'''    fn local_evidence_comparison(
        &self,
        selection: SelectionId,
    ) -> Option<EvidenceNeighborhoodComparison> {
        let left = self.snapshot(ComparisonSide::Left)?;
        let right = self.snapshot(ComparisonSide::Right)?;
        compare_evidence_neighborhoods(left, right, selection, LOCAL_EVIDENCE_DEPTH)
    }

    fn render_local_evidence_divergence(
        &self,
        comparison: &EvidenceNeighborhoodComparison,
        cx: &mut Context<Self>,
    ) -> Div {
        let node_count = comparison.nodes.len();
        let edge_count = comparison.edges.left_only.len() + comparison.edges.right_only.len();
        let mut section = div()
            .mt_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd9dfd5))
            .bg(rgb(0xffffff))
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_sm().child("Local evidence divergence"))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x66705f))
                    .child(format!(
                        "Within {} hops of this selection: {} node-distance changes and {} typed edge changes between futures.",
                        comparison.max_depth, node_count, edge_count
                    )),
            );

        if !comparison.nodes.is_empty() {
            let mut nodes = div().flex().flex_col().gap_2().child(
                div()
                    .text_xs()
                    .text_color(rgb(0x657565))
                    .child("NODE DISTANCE CHANGES"),
            );
            for node in comparison.nodes.iter().take(LOCAL_EVIDENCE_NODE_LIMIT) {
                nodes = nodes.child(self.render_local_evidence_node(node, cx));
            }
            if let Some(notice) = hidden_notice(
                comparison.nodes.len(),
                LOCAL_EVIDENCE_NODE_LIMIT,
                "local evidence nodes",
            ) {
                nodes = nodes.child(truncation_notice(notice));
            }
            section = section.child(nodes);
        }

        if !comparison.edges.is_empty() {
            section = section.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x657565))
                    .child("TYPED EDGE CHANGES"),
            );
            section = section.child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.render_local_evidence_edge_side(
                        ComparisonSide::Left,
                        &comparison.edges.left_only,
                    ))
                    .child(self.render_local_evidence_edge_side(
                        ComparisonSide::Right,
                        &comparison.edges.right_only,
                    )),
            );
        }

        section
    }

    fn render_local_evidence_node(
        &self,
        node: &EvidenceNeighborhoodNodeDifference,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut row = div().flex().gap_2();
        if let Some(depth) = node.left_depth {
            row = row.child(self.render_local_evidence_node_side(
                ComparisonSide::Left,
                node.selection,
                depth,
                cx,
            ));
        } else {
            row = row.child(self.render_local_evidence_absent_side(ComparisonSide::Left));
        }
        if let Some(depth) = node.right_depth {
            row = row.child(self.render_local_evidence_node_side(
                ComparisonSide::Right,
                node.selection,
                depth,
                cx,
            ));
        } else {
            row = row.child(self.render_local_evidence_absent_side(ComparisonSide::Right));
        }
        row
    }

    fn render_local_evidence_node_side(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> Div {
        let title = self.selection_title(side, selection);
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "local-evidence-node-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .w(px(320.0))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                rgb(0x4e6fb3)
            } else {
                rgb(0xe2e4e8)
            })
            .bg(if selected {
                rgb(0xeef3ff)
            } else {
                rgb(0xf8f9fc)
            })
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x4e6fb3))
                    .child(format!("{} · {} hop{}", self.side_label(side), depth, if depth == 1 { "" } else { "s" })),
            )
            .child(div().text_sm().child(title))
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

    fn render_local_evidence_absent_side(&self, side: ComparisonSide) -> Div {
        div()
            .w(px(320.0))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xe2e4e8))
            .bg(rgb(0xf8f9fc))
            .text_xs()
            .text_color(rgb(0x888888))
            .child(format!("{} · outside this neighborhood", self.side_label(side)))
    }

    fn render_local_evidence_edge_side(
        &self,
        side: ComparisonSide,
        edges: &[StateEvidenceEdge],
    ) -> Div {
        let mut column = div()
            .w(px(320.0))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xe2e4e8))
            .bg(rgb(0xf8f9fc))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x4e6fb3))
                    .child(self.side_label(side).to_string()),
            );
        if edges.is_empty() {
            return column.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .child("No side-only typed edges"),
            );
        }
        for edge in edges.iter().take(LOCAL_EVIDENCE_EDGE_LIMIT_PER_SIDE) {
            column = column.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x555555))
                    .child(self.local_evidence_edge_label(side, *edge)),
            );
        }
        if let Some(notice) = hidden_notice(
            edges.len(),
            LOCAL_EVIDENCE_EDGE_LIMIT_PER_SIDE,
            "typed edges",
        ) {
            column = column.child(truncation_notice(notice));
        }
        column
    }

    fn local_evidence_edge_label(&self, side: ComparisonSide, edge: StateEvidenceEdge) -> String {
        match edge {
            StateEvidenceEdge::EntityEvent(evidence) => format!(
                "Recorded entity change: {} ↔ {}",
                self.selection_title(side, SelectionId::Entity(evidence.entity)),
                self.selection_title(side, SelectionId::Event(evidence.event)),
            ),
            StateEvidenceEdge::RelationEvent(evidence) => format!(
                "Recorded relation change: {} ↔ {}",
                self.selection_title(side, SelectionId::Relation(evidence.relation)),
                self.selection_title(side, SelectionId::Event(evidence.event)),
            ),
            StateEvidenceEdge::EntityRelation(evidence) => {
                let role = match evidence.role {
                    RelationEndpointRole::From => "From endpoint",
                    RelationEndpointRole::To => "To endpoint",
                };
                format!(
                    "{role}: {} ↔ {}",
                    self.selection_title(side, SelectionId::Entity(evidence.entity)),
                    self.selection_title(side, SelectionId::Relation(evidence.relation)),
                )
            }
        }
    }

    fn selection_title(&self, side: ComparisonSide, selection: SelectionId) -> String {
        self.snapshot(side)
            .and_then(|snapshot| snapshot.inspector(selection))
            .map(|inspector| inspector.title.clone())
            .unwrap_or_else(|| selection.stable_key())
    }

'''
if text.count(marker) != 1:
    raise SystemExit(f"expected evidence inspector marker once, found {text.count(marker)}")
text = text.replace(marker, helpers + marker, 1)
path.write_text(text)
