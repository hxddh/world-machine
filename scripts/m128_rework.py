from pathlib import Path

lib = Path("crates/world-projection/src/lib.rs")
text = lib.read_text()
old = '''    pub fn why(&self, event: EventId) -> Option<&WhyProjection> {\n        self.why.get(&event)\n    }\n\n    pub fn command(&self, id: &str) -> Option<&ProjectionCommand> {'''
new = '''    pub fn why(&self, event: EventId) -> Option<&WhyProjection> {\n        self.why.get(&event)\n    }\n\n    pub fn influence(&self, event: EventId) -> Vec<(usize, &TimelineItem)> {\n        influence::influence_from_timeline(&self.timeline, event)\n    }\n\n    pub fn command(&self, id: &str) -> Option<&ProjectionCommand> {'''
if text.count(old) != 1:
    raise SystemExit(f"snapshot method anchor count {text.count(old)}")
text = text.replace(old, new, 1)
old = '''\n    let influence = influence::influence_rows(world, event.id);\n    if !influence.is_empty() {\n        sections.push(InspectorSection {\n            title: "Influence".into(),\n            rows: influence,\n        });\n    }\n'''
if text.count(old) != 1:
    raise SystemExit(f"old inspector influence count {text.count(old)}")
text = text.replace(old, "\n", 1)
lib.write_text(text)

macos = Path("crates/world-gpui/src/macos.rs")
text = macos.read_text()
anchor = '''    fn render_why(&self, cx: &mut Context<Self>) -> Option<Div> {'''
method = '''    fn render_influence(&self, cx: &mut Context<Self>) -> Option<Div> {\n        let SelectionId::Event(event) = self.selected? else {\n            return None;\n        };\n        let influence = self.snapshot.influence(event);\n        if influence.is_empty() {\n            return None;\n        }\n\n        let total = influence.len();\n        let direct = influence.iter().filter(|(depth, _)| *depth == 1).count();\n        let max_depth = influence\n            .iter()\n            .map(|(depth, _)| *depth)\n            .max()\n            .unwrap_or(1);\n        let mut nodes = div().flex().flex_col().gap_1();\n        for (depth, item) in influence.iter().take(10) {\n            nodes = nodes.child(self.influence_node(*depth, item, cx));\n        }\n\n        let mut panel = div()\n            .p_3()\n            .rounded_md()\n            .border_1()\n            .border_color(rgb(0xd7e2d7))\n            .bg(rgb(0xf7fbf7))\n            .flex()\n            .flex_col()\n            .gap_2()\n            .child(div().text_lg().child("What this affected"))\n            .child(\n                div()\n                    .text_xs()\n                    .text_color(rgb(0x657565))\n                    .child(format!(\n                        "{} later {} · {} direct · up to {} causal {}",\n                        total,\n                        if total == 1 { "event" } else { "events" },\n                        direct,\n                        max_depth,\n                        if max_depth == 1 { "step" } else { "steps" },\n                    )),\n            )\n            .child(nodes);\n\n        if total > 10 {\n            panel = panel.child(\n                div()\n                    .text_xs()\n                    .text_color(rgb(0x657565))\n                    .child(format!("+{} more affected events", total - 10)),\n            );\n        }\n        Some(panel)\n    }\n\n'''
if text.count(anchor) != 1:
    raise SystemExit(f"render why anchor count {text.count(anchor)}")
text = text.replace(anchor, method + anchor, 1)
anchor = '''    fn why_node(&self, node: &WhyNode, cx: &mut Context<Self>) -> impl IntoElement {'''
method = '''    fn influence_node(\n        &self,\n        depth: usize,\n        item: &TimelineItem,\n        cx: &mut Context<Self>,\n    ) -> impl IntoElement {\n        let selection = item.id;\n        let prefix = if depth == 1 {\n            "Direct effect".to_string()\n        } else {\n            format!("Later · {depth} steps")\n        };\n        div()\n            .id(SharedString::from(format!(\n                "influence-{}",\n                selection.stable_key()\n            )))\n            .p_2()\n            .rounded_md()\n            .bg(rgb(0xffffff))\n            .cursor_pointer()\n            .child(div().text_xs().text_color(rgb(0x657565)).child(prefix))\n            .child(div().text_sm().child(item.title.clone()))\n            .child(\n                div()\n                    .text_xs()\n                    .text_color(rgb(0x777777))\n                    .child(item.subtitle.clone()),\n            )\n            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))\n    }\n\n'''
if text.count(anchor) != 1:
    raise SystemExit(f"why node anchor count {text.count(anchor)}")
text = text.replace(anchor, method + anchor, 1)
old = '''            if let Some(why) = self.render_why(cx) {\n                center = center.child(why);\n            }\n'''
new = '''            if let Some(why) = self.render_why(cx) {\n                center = center.child(why);\n            }\n            if let Some(influence) = self.render_influence(cx) {\n                center = center.child(influence);\n            }\n'''
if text.count(old) != 1:
    raise SystemExit(f"render insertion count {text.count(old)}")
text = text.replace(old, new, 1)
macos.write_text(text)
