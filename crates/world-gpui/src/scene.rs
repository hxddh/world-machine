//! The World drawn as a place rather than listed as rows.
//!
//! Every Pack already says where its people, places and things stand
//! (`CanvasProjection`). This draws that as a small scene: places as tiles,
//! people as faces, the relations between them as lines, and a soft halo on
//! whoever the latest news is about, so the first thing a returning player
//! sees is the World and who moved in it, not a paragraph.

use crate::ui;
use gpui::{
    canvas, div, linear_color_stop, linear_gradient, point, prelude::*, px, quad, relative, rgb,
    size, App, BorderStyle, Bounds, Div, FontWeight, Hsla, PathBuilder, SharedString, Window,
};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasLinkTone, ProjectionSnapshot, SelectionId,
};
use world_theme::tokens;

const SCENE_HEIGHT: f32 = 340.0;
/// A World with more than this many things on stage draws them smaller and
/// gives the stage more room, rather than piling them on each other.
const CROWDED: usize = 8;
const CROWDED_SCENE_HEIGHT: f32 = 420.0;
/// How many of the latest news items get a halo; past a handful, a halo on
/// everyone is a halo on no one.
const HALOED_BEATS: usize = 3;
const PLACE_WIDTH: f32 = 184.0;
const PLACE_HEIGHT: f32 = 58.0;
const ACTOR_SIZE: f32 = 46.0;
const COMPACT_ACTOR_SIZE: f32 = 36.0;
const ACTOR_LABEL_WIDTH: f32 = 120.0;
const OBJECT_WIDTH: f32 = 132.0;
const OBJECT_HEIGHT: f32 = 30.0;
const LINK_LABEL_WIDTH: f32 = 180.0;

/// One line to draw: where it runs, how it reads, and how heavy it is.
struct Line {
    from: (f32, f32),
    to: (f32, f32),
    tone: CanvasLinkTone,
    width: f32,
}

/// Where a Pack's 0..1 position lands inside the scene, leaving a margin so
/// nothing placed at an edge is cut off.
fn scene_x(x: f32) -> f32 {
    0.09 + x.clamp(0.0, 1.0) * 0.82
}

fn scene_y(y: f32) -> f32 {
    0.15 + y.clamp(0.0, 1.0) * 0.70
}

fn hsla(token: tokens::Token) -> Hsla {
    rgb(token.hex()).into()
}

/// The items in `this` scene that are not the same in `other`: gone, new,
/// or saying something different about themselves.
pub fn differences(this: &ProjectionSnapshot, other: &ProjectionSnapshot) -> BTreeSet<SelectionId> {
    this.canvas
        .items
        .iter()
        .filter(|item| {
            !other.canvas.items.iter().any(|twin| {
                twin.id == item.id && twin.label == item.label && twin.detail == item.detail
            })
        })
        .map(|item| item.id)
        .collect()
}

/// What happens when something in a scene is clicked.
pub type SelectHandler = Rc<dyn Fn(SelectionId, &mut Window, &mut App)>;

/// The name of whoever an event was done by, as the projection records it.
pub fn event_actor(snapshot: &ProjectionSnapshot, selection: SelectionId) -> Option<String> {
    let inspector = snapshot.inspector(selection)?;
    inspector
        .sections
        .iter()
        .flat_map(|section| section.rows.iter())
        .find(|row| row.label == "Actor")
        .map(|row| row.value.clone())
}

/// Everyone and everything the current news is about: the actors and
/// targets of the events the briefing reports.
pub fn in_the_news(snapshot: &ProjectionSnapshot, latest: usize) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let Some(briefing) = snapshot.briefing.as_ref() else {
        return names;
    };
    let beats = briefing.beats();
    for beat in beats.iter().skip(beats.len().saturating_sub(latest)) {
        let Some(selection) = beat.selection else {
            continue;
        };
        match selection {
            SelectionId::Event(_) => {
                if let Some(inspector) = snapshot.inspector(selection) {
                    for row in inspector.sections.iter().flat_map(|s| s.rows.iter()) {
                        if row.label == "Actor" || row.label == "Targets" {
                            names.extend(row.value.split(", ").map(str::to_string));
                        }
                    }
                }
            }
            SelectionId::Entity(_) | SelectionId::Relation(_) => {
                if let Some(inspector) = snapshot.inspector(selection) {
                    names.insert(inspector.title.clone());
                }
            }
        }
    }
    names
}

/// Pairs of scene items joined by a relation the World records.
fn links(snapshot: &ProjectionSnapshot) -> Vec<(SelectionId, SelectionId)> {
    let on_scene = snapshot
        .canvas
        .items
        .iter()
        .map(|item| item.id)
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut pairs = Vec::new();
    for item in &snapshot.canvas.items {
        let SelectionId::Entity(entity) = item.id else {
            continue;
        };
        for relation in snapshot.relations_for_entity(entity) {
            if !seen.insert(relation) {
                continue;
            }
            let ends = snapshot
                .entities_for_relation(relation)
                .into_iter()
                .map(SelectionId::Entity)
                .filter(|end| on_scene.contains(end))
                .collect::<Vec<_>>();
            if let [from, to] = ends.as_slice() {
                if from != to {
                    pairs.push((*from, *to));
                }
            }
        }
    }
    pairs
}

/// How much room a node takes: its width, its height, and how far its top
/// sits above the point it is placed at.
fn footprint(kind: CanvasItemKind, compact: bool) -> (f32, f32, f32) {
    match (kind, compact) {
        (CanvasItemKind::Actor, false) => (ACTOR_LABEL_WIDTH, ACTOR_SIZE + 38.0, ACTOR_SIZE / 2.0),
        (CanvasItemKind::Actor, true) => {
            (92.0, COMPACT_ACTOR_SIZE + 22.0, COMPACT_ACTOR_SIZE / 2.0)
        }
        (CanvasItemKind::Place, false) => (PLACE_WIDTH, PLACE_HEIGHT, PLACE_HEIGHT / 2.0),
        (CanvasItemKind::Place, true) => (168.0, 48.0, 24.0),
        (CanvasItemKind::Object, _) => (OBJECT_WIDTH, OBJECT_HEIGHT, OBJECT_HEIGHT / 2.0),
    }
}

/// Where each item is drawn, as fractions of the stage. Starts from where
/// the Pack put it, then nudges overlapping nodes apart a little at a time
/// until nothing covers anything else, so a Pack's rough positions never
/// produce a pile. Deterministic: the same World always lays out the same.
fn layout(items: &[CanvasItem], width: f32, height: f32, compact: bool) -> Vec<(f32, f32)> {
    const GAP: f32 = 10.0;
    const EDGE: f32 = 6.0;
    let boxes = items
        .iter()
        .map(|item| footprint(item.kind, compact))
        .collect::<Vec<_>>();
    // Work in the centre of each node's box, in pixels.
    let mut centres = items
        .iter()
        .zip(&boxes)
        .map(|(item, (_, h, top))| {
            (
                scene_x(item.x) * width,
                scene_y(item.y) * height - top + h / 2.0,
            )
        })
        .collect::<Vec<_>>();
    for _ in 0..160 {
        let mut moved = false;
        for i in 0..centres.len() {
            for j in (i + 1)..centres.len() {
                let (wi, hi, _) = boxes[i];
                let (wj, hj, _) = boxes[j];
                let dx = centres[j].0 - centres[i].0;
                let dy = centres[j].1 - centres[i].1;
                let overlap_x = (wi + wj) / 2.0 + GAP - dx.abs();
                let overlap_y = (hi + hj) / 2.0 + GAP - dy.abs();
                if overlap_x <= 0.0 || overlap_y <= 0.0 {
                    continue;
                }
                moved = true;
                // Separate along whichever axis needs the smaller push; ties
                // break by list order so the result never depends on luck.
                if overlap_x / width < overlap_y / height {
                    let push = overlap_x / 2.0 * if dx < 0.0 { -1.0 } else { 1.0 };
                    centres[i].0 -= push;
                    centres[j].0 += push;
                } else {
                    let push = overlap_y / 2.0 * if dy < 0.0 { -1.0 } else { 1.0 };
                    centres[i].1 -= push;
                    centres[j].1 += push;
                }
            }
        }
        for (centre, (w, h, _)) in centres.iter_mut().zip(&boxes) {
            centre.0 = centre
                .0
                .clamp(EDGE + w / 2.0, (width - EDGE - w / 2.0).max(EDGE + w / 2.0));
            centre.1 = centre.1.clamp(
                EDGE + h / 2.0,
                (height - EDGE - h / 2.0).max(EDGE + h / 2.0),
            );
        }
        if !moved {
            break;
        }
    }
    // Back to the anchor point each node is drawn from, as fractions.
    centres
        .iter()
        .zip(&boxes)
        .map(|((x, y), (_, h, top))| (x / width, (y - h / 2.0 + top) / height))
        .collect()
}

/// What a scene draws attention to.
#[derive(Clone, Debug, Default)]
pub enum Emphasis {
    /// Whoever the latest news is about: the World window's default.
    #[default]
    News,
    /// These items, and nothing else: how a comparison points at what is
    /// different in this future.
    Only(BTreeSet<SelectionId>),
}

/// A World's scene at `width` pixels wide, or nothing when the Pack draws
/// none. `selected` is ringed; clicking anything calls `on_select`.
pub fn scene(
    snapshot: &ProjectionSnapshot,
    width: f32,
    selected: Option<SelectionId>,
    emphasis: &Emphasis,
    on_select: SelectHandler,
) -> Option<Div> {
    let items = &snapshot.canvas.items;
    if items.is_empty() {
        return None;
    }
    let compact = items.len() > CROWDED;
    let stage_height = if compact {
        CROWDED_SCENE_HEIGHT
    } else {
        SCENE_HEIGHT
    };
    let news = in_the_news(snapshot, HALOED_BEATS);
    let emphasised = |item: &CanvasItem| match emphasis {
        Emphasis::News => news.contains(&item.label),
        Emphasis::Only(items) => items.contains(&item.id),
    };
    let placed = layout(items, width.max(320.0), stage_height, compact);
    let positions = items
        .iter()
        .zip(&placed)
        .map(|(item, position)| (item.id, *position))
        .collect::<BTreeMap<_, _>>();
    // Relations the World records, drawn thin; connections the Pack
    // asks for, drawn with their tone and weight.
    let mut lines = links(snapshot)
        .into_iter()
        .filter_map(|(from, to)| {
            Some(Line {
                from: *positions.get(&from)?,
                to: *positions.get(&to)?,
                tone: CanvasLinkTone::Neutral,
                width: 1.5,
            })
        })
        .collect::<Vec<_>>();
    let pack_links = snapshot
        .canvas
        .links
        .iter()
        .filter_map(|link| Some((link, *positions.get(&link.from)?, *positions.get(&link.to)?)))
        .collect::<Vec<_>>();
    for (link, from, to) in &pack_links {
        lines.push(Line {
            from: *from,
            to: *to,
            tone: link.tone,
            width: 2.0 + 4.0 * link.strength,
        });
    }
    let halos = items
        .iter()
        .zip(&placed)
        .filter(|(item, _)| emphasised(item))
        .map(|(_, position)| *position)
        .collect::<Vec<_>>();

    let grid = hsla(tokens::SCENE_GRID);
    let tone_colour = |tone: CanvasLinkTone| match tone {
        CanvasLinkTone::Neutral => hsla(tokens::SCENE_LINK),
        CanvasLinkTone::Warm => hsla(tokens::SUCCESS).opacity(0.7),
        CanvasLinkTone::Strained => hsla(tokens::DANGER).opacity(0.7),
    };
    let lines = lines
        .into_iter()
        .map(|line| (line.from, line.to, tone_colour(line.tone), line.width))
        .collect::<Vec<_>>();
    let glow = hsla(tokens::ACCENT);
    let backdrop = canvas(
        |_, _, _| (),
        move |bounds: Bounds<gpui::Pixels>, _, window, _| {
            let origin = bounds.origin;
            let width = bounds.size.width;
            let height = bounds.size.height;
            let at = |x: f32, y: f32| point(origin.x + width * x, origin.y + height * y);

            // A faint dot grid: enough texture to read as ground.
            let step = 22.0;
            let mut y = step;
            while y < f32::from(height) {
                let mut x = step;
                while x < f32::from(width) {
                    window.paint_quad(quad(
                        Bounds::new(
                            point(origin.x + px(x), origin.y + px(y)),
                            size(px(2.0), px(2.0)),
                        ),
                        px(1.0),
                        grid,
                        px(0.0),
                        grid,
                        BorderStyle::default(),
                    ));
                    x += step;
                }
                y += step;
            }

            // Whoever the news is about glows, so a returning player sees
            // where things happened before reading what happened.
            for (x, y) in &halos {
                let centre = at(*x, *y);
                for (radius, alpha) in [(40.0, 0.08), (30.0, 0.12)] {
                    window.paint_quad(quad(
                        Bounds::new(
                            point(centre.x - px(radius), centre.y - px(radius)),
                            size(px(radius * 2.0), px(radius * 2.0)),
                        ),
                        px(radius),
                        glow.opacity(alpha),
                        px(0.0),
                        glow.opacity(0.0),
                        BorderStyle::default(),
                    ));
                }
            }

            // Relations as gently bowed lines, so crossing links stay
            // distinguishable.
            for ((x1, y1), (x2, y2), colour, width) in &lines {
                let from = at(*x1, *y1);
                let to = at(*x2, *y2);
                let control = point((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
                let mut path = PathBuilder::stroke(px(*width));
                path.move_to(from);
                path.curve_to(to, control);
                if let Ok(path) = path.build() {
                    window.paint_path(path, *colour);
                }
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();

    let mut scene = div()
        .relative()
        .flex_shrink_0()
        .w_full()
        .h(px(stage_height))
        .rounded_xl()
        .overflow_hidden()
        .border_1()
        .border_color(ui::color(tokens::BORDER))
        .bg(linear_gradient(
            180.0,
            linear_color_stop(hsla(tokens::SCENE_TOP), 0.0),
            linear_color_stop(hsla(tokens::SCENE_BOTTOM), 1.0),
        ))
        .child(backdrop);

    for (item, (x, y)) in items.iter().zip(placed.iter().copied()) {
        let selection = item.id;
        let selected = selected == Some(selection);
        let active = emphasised(item);
        let id = SharedString::from(format!("canvas-{}", selection.stable_key()));
        let node = match item.kind {
            CanvasItemKind::Actor => {
                actor_node(&item.label, &item.detail, selected, active, compact)
            }
            CanvasItemKind::Place => place_node(&item.label, &item.detail, selected, compact),
            CanvasItemKind::Object => object_node(&item.label, selected),
        };
        // Centre the node on its position, whatever its size.
        let (width, _, top_offset) = footprint(item.kind, compact);
        scene = scene.child(
            div()
                .id(id)
                .absolute()
                .left(relative(x))
                .top(relative(y))
                .ml(px(-width / 2.0))
                .mt(px(-top_offset))
                .w(px(width))
                .cursor_pointer()
                .child(node)
                .on_click({
                    let on_select = on_select.clone();
                    move |_, window, cx| on_select(selection, window, cx)
                }),
        );
    }

    // Each connection's name sits on its line and selects what it stands
    // for, so the relationship is reached by clicking between the people.
    for (index, (link, (x1, y1), (x2, y2))) in pack_links.iter().enumerate() {
        if link.label.is_empty() {
            continue;
        }
        let (text, border) = match link.tone {
            CanvasLinkTone::Neutral => (tokens::TEXT_SECONDARY, tokens::BORDER),
            CanvasLinkTone::Warm => (tokens::SUCCESS, tokens::SUCCESS),
            CanvasLinkTone::Strained => (tokens::DANGER, tokens::DANGER),
        };
        let mut pill = div()
            .id(SharedString::from(format!("scene-link-{index}")))
            .absolute()
            .left(relative((x1 + x2) / 2.0))
            .top(relative((y1 + y2) / 2.0))
            .ml(px(-LINK_LABEL_WIDTH / 2.0))
            .mt(px(-12.0))
            .w(px(LINK_LABEL_WIDTH))
            .flex()
            .justify_center()
            .child(
                div()
                    .px_2()
                    .py(px(2.0))
                    .rounded_full()
                    .border_1()
                    .border_color(ui::color(border).opacity(0.6))
                    .bg(ui::color(tokens::SURFACE))
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(ui::color(text))
                    .child(link.label.clone()),
            );
        if let Some(selection) = link.selection {
            let on_select = on_select.clone();
            pill = pill
                .cursor_pointer()
                .on_click(move |_, window, cx| on_select(selection, window, cx));
        }
        scene = scene.child(pill);
    }

    Some(scene)
}

/// When things happened across the World's whole life, as a strip of bars,
/// with the stretch the current news covers picked out.
pub fn activity(snapshot: &ProjectionSnapshot) -> Option<Div> {
    const BUCKETS: usize = 36;
    let times = snapshot
        .timeline
        .items
        .iter()
        .map(|item| item.world_time)
        .collect::<Vec<_>>();
    let (&first, &last) = (times.iter().min()?, times.iter().max()?);
    if last == first {
        return None;
    }
    let news_times = snapshot
        .briefing
        .as_ref()
        .map(|briefing| {
            briefing
                .beats()
                .iter()
                .filter_map(|beat| match beat.selection {
                    Some(SelectionId::Event(event)) => snapshot
                        .timeline
                        .items
                        .iter()
                        .find(|item| item.id == SelectionId::Event(event))
                        .map(|item| item.world_time),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let bucket_of = |time: u64| {
        (((time - first) as f64 / (last - first) as f64) * (BUCKETS - 1) as f64).round() as usize
    };
    let mut counts = [0_usize; BUCKETS];
    for time in &times {
        counts[bucket_of(*time)] += 1;
    }
    let highlighted = match (news_times.iter().min(), news_times.iter().max()) {
        (Some(&from), Some(&to)) => bucket_of(from)..=bucket_of(to),
        _ => {
            let end = bucket_of(last);
            end..=end
        }
    };
    let peak = counts.iter().copied().max().unwrap_or(1).max(1);

    let mut bars = div().h(px(34.0)).flex().items_end().gap(px(2.0));
    for (index, count) in counts.iter().enumerate() {
        let height = if *count == 0 {
            2.0
        } else {
            6.0 + 28.0 * (*count as f32 / peak as f32)
        };
        let colour = if highlighted.contains(&index) {
            tokens::ACCENT
        } else {
            tokens::BORDER_STRONG
        };
        bars = bars.child(
            div()
                .w(px(4.0))
                .h(px(height))
                .rounded_sm()
                .bg(ui::color(colour)),
        );
    }

    Some(
        div()
            .flex()
            .flex_col()
            .items_end()
            .gap_1()
            .child(bars)
            .child(ui::caption(format!(
                "{} moments · time {first}–{last}",
                times.len()
            ))),
    )
}

fn actor_node(name: &str, detail: &str, selected: bool, active: bool, compact: bool) -> Div {
    let size = if compact {
        COMPACT_ACTOR_SIZE
    } else {
        ACTOR_SIZE
    };
    let mut face = ui::avatar(name, size)
        .border_2()
        .border_color(ui::color(tokens::SURFACE));
    if selected {
        face = face.border_color(ui::color(tokens::ACCENT));
    } else if active {
        face = face.border_color(ui::color(tokens::ACCENT_SOFT));
    }
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(3.0))
        .child(face.shadow_sm())
        .child(
            div()
                .px_2()
                .rounded_md()
                .bg(ui::color(tokens::SURFACE).opacity(0.85))
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ui::color(tokens::TEXT))
                .truncate()
                .child(name.to_string()),
        )
        .when(!compact, |node| {
            node.child(ui::caption(crate::macos::capitalize(detail)).truncate())
        })
}

fn place_node(name: &str, detail: &str, selected: bool, compact: bool) -> Div {
    let (_, height, _) = footprint(CanvasItemKind::Place, compact);
    div()
        .h(px(height))
        .px_3()
        .rounded_lg()
        .border_1()
        .border_color(if selected {
            ui::color(tokens::ACCENT)
        } else {
            ui::color(tokens::BORDER_STRONG)
        })
        .bg(ui::color(tokens::SURFACE))
        .shadow_sm()
        .hover(|style| style.border_color(ui::color(tokens::ACCENT)))
        .flex()
        .items_center()
        .gap_2()
        .child(place_icon())
        .child(
            div()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(ui::row_title(name.to_string()).truncate())
                .child(ui::caption(crate::macos::capitalize(detail)).truncate()),
        )
}

fn object_node(name: &str, selected: bool) -> Div {
    div()
        .h(px(OBJECT_HEIGHT))
        .px_3()
        .rounded_full()
        .border_1()
        .border_color(if selected {
            ui::color(tokens::ACCENT)
        } else {
            ui::color(tokens::BORDER)
        })
        .bg(ui::color(tokens::SURFACE).opacity(0.9))
        .hover(|style| style.border_color(ui::color(tokens::ACCENT)))
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .child(
            div()
                .flex_shrink_0()
                .size(px(7.0))
                .rounded_sm()
                .bg(ui::color(tokens::TEXT_TERTIARY)),
        )
        .child(
            div()
                .min_w(px(0.0))
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(ui::color(tokens::TEXT_SECONDARY))
                .truncate()
                .child(name.to_string()),
        )
}

/// A small drawn house: a place, whatever the World calls it.
fn place_icon() -> Div {
    let roof = hsla(tokens::SUCCESS);
    div()
        .flex_shrink_0()
        .size(px(28.0))
        .rounded_md()
        .bg(ui::color(tokens::SCENE_BOTTOM))
        .child(
            canvas(
                |_, _, _| (),
                move |bounds: Bounds<gpui::Pixels>, _, window, _| {
                    let o = bounds.origin;
                    let at = |x: f32, y: f32| point(o.x + px(x), o.y + px(y));
                    let mut house = PathBuilder::fill();
                    house.move_to(at(14.0, 6.0));
                    house.line_to(at(23.0, 13.0));
                    house.line_to(at(21.0, 13.0));
                    house.line_to(at(21.0, 22.0));
                    house.line_to(at(7.0, 22.0));
                    house.line_to(at(7.0, 13.0));
                    house.line_to(at(5.0, 13.0));
                    house.close();
                    if let Ok(path) = house.build() {
                        window.paint_path(path, roof);
                    }
                },
            )
            .size_full(),
        )
}

#[cfg(test)]
mod tests {
    use super::{differences, footprint, layout, CROWDED};
    use world_projection::ProjectionSnapshot;
    use world_projection::{CanvasItem, CanvasItemKind, SelectionId};

    fn item(id: u64, kind: CanvasItemKind, x: f32, y: f32) -> CanvasItem {
        CanvasItem {
            id: SelectionId::from_stable_key(&format!("entity-{id}")).expect("an entity key"),
            kind,
            label: format!("Item {id}"),
            detail: String::new(),
            x,
            y,
        }
    }

    /// Tiny Society's own positions, which piled up before layout existed.
    fn harbour() -> Vec<CanvasItem> {
        use CanvasItemKind::{Actor, Object, Place};
        vec![
            item(1, Place, 0.08, 0.40),
            item(2, Place, 0.24, 0.16),
            item(3, Place, 0.52, 0.18),
            item(4, Place, 0.52, 0.52),
            item(5, Actor, 0.12, 0.62),
            item(6, Actor, 0.68, 0.32),
            item(7, Actor, 0.34, 0.28),
            item(8, Actor, 0.70, 0.74),
            item(9, Actor, 0.82, 0.67),
            item(10, Actor, 0.20, 0.52),
            item(11, Actor, 0.04, 0.72),
            item(12, Actor, 0.42, 0.12),
            item(13, Object, 0.02, 0.42),
            item(14, Object, 0.84, 0.22),
        ]
    }

    #[test]
    fn a_crowded_world_lays_out_without_overlaps() {
        let items = harbour();
        assert!(items.len() > CROWDED);
        let (width, height) = (700.0, 420.0);
        let placed = layout(&items, width, height, true);
        let boxes = items
            .iter()
            .zip(&placed)
            .map(|(item, (x, y))| {
                let (w, h, top) = footprint(item.kind, true);
                let left = x * width - w / 2.0;
                let top = y * height - top;
                (left, top, left + w, top + h)
            })
            .collect::<Vec<_>>();
        for (i, a) in boxes.iter().enumerate() {
            assert!(
                a.0 >= 0.0 && a.2 <= width && a.1 >= 0.0 && a.3 <= height,
                "{i} off stage: {a:?}"
            );
            for (j, b) in boxes.iter().enumerate().skip(i + 1) {
                let apart = a.2 <= b.0 || b.2 <= a.0 || a.3 <= b.1 || b.3 <= a.1;
                assert!(apart, "{i} and {j} overlap: {a:?} {b:?}");
            }
        }
    }

    #[test]
    fn the_same_world_always_lays_out_the_same() {
        let items = harbour();
        assert_eq!(
            layout(&items, 700.0, 420.0, true),
            layout(&items, 700.0, 420.0, true)
        );
    }

    #[test]
    fn a_comparison_points_at_what_is_different_in_each_future() {
        let mut held = ProjectionSnapshot::default();
        held.canvas.items = vec![
            item(1, CanvasItemKind::Place, 0.1, 0.1),
            item(2, CanvasItemKind::Actor, 0.5, 0.5),
        ];
        held.canvas.items[0].detail = "in crisis".into();
        let mut lost = held.clone();
        lost.canvas.items[0].detail = "lost".into();
        lost.canvas
            .items
            .push(item(3, CanvasItemKind::Actor, 0.8, 0.8));

        let first = held.canvas.items[0].id;
        let newcomer = lost.canvas.items[2].id;
        assert_eq!(
            differences(&held, &lost).into_iter().collect::<Vec<_>>(),
            vec![first]
        );
        assert_eq!(
            differences(&lost, &held).into_iter().collect::<Vec<_>>(),
            vec![first, newcomer]
        );
    }
}
