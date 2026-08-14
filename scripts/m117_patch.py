from pathlib import Path

path = Path("apps/world-machine-desktop/src/main.rs")
text = path.read_text()
old = '''        let mut details = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_lg().child(title.clone()))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child(if title == pack_title {
                        format!(
                            "World time {} · {} events",
                            document.world_time, document.event_count
                        )
                    } else {
                        format!(
                            "{} · World time {} · {} events",
                            pack_title, document.world_time, document.event_count
                        )
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x8a8a82))
                    .child(document_label.clone()),
            );'''
new = '''        let mut details = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_lg().child(title.clone()));
        if let Some(summary) = world_summary_description(&document) {
            details = details.child(
                div()
                    .text_sm()
                    .text_color(rgb(0x4f5968))
                    .child(summary),
            );
        }
        details = details
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child(if title == pack_title {
                        format!(
                            "World time {} · {} events",
                            document.world_time, document.event_count
                        )
                    } else {
                        format!(
                            "{} · World time {} · {} events",
                            pack_title, document.world_time, document.event_count
                        )
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x8a8a82))
                    .child(document_label.clone()),
            );'''
count = text.count(old)
if count != 1:
    raise SystemExit(f"expected exactly one My Worlds details block, found {count}")
path.write_text(text.replace(old, new, 1))
