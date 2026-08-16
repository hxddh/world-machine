from pathlib import Path

path = Path("crates/world-strategy-gpui/src/lib.rs")
text = path.read_text()
old = '''    fn render_local_evidence_node_side(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> Div {
'''
new = '''    fn render_local_evidence_node_side(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
'''
if text.count(old) != 1:
    raise SystemExit(f"expected one local evidence node helper, found {text.count(old)}")
path.write_text(text.replace(old, new, 1))
