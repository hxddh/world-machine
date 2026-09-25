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
use gpui::{Animation, AnimationExt};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::Duration;
use world_projection::{
    CanvasChange, CanvasItem, CanvasItemKind, CanvasLinkTone, MarkShape, ProjectionSnapshot,
    SelectionId, Tone,
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
/// or saying something different about themselves. A connection that
/// differs (new, gone, relabelled, retoned or much stronger or weaker)
/// marks both of its ends, since two people can change toward each other
/// while each stays the same alone.
pub fn differences(this: &ProjectionSnapshot, other: &ProjectionSnapshot) -> BTreeSet<SelectionId> {
    let mut different = this
        .canvas
        .items
        .iter()
        .filter(|item| {
            !other.canvas.items.iter().any(|twin| {
                twin.id == item.id && twin.label == item.label && twin.detail == item.detail
            })
        })
        .map(|item| item.id)
        .collect::<BTreeSet<_>>();
    let same = |link: &world_projection::CanvasLink, twin: &world_projection::CanvasLink| {
        twin.from == link.from
            && twin.to == link.to
            && twin.label == link.label
            && twin.tone == link.tone
            && (twin.strength - link.strength).abs() < 0.1
    };
    let changed_links = this
        .canvas
        .links
        .iter()
        .filter(|link| !other.canvas.links.iter().any(|twin| same(link, twin)))
        .chain(
            other
                .canvas
                .links
                .iter()
                .filter(|link| !this.canvas.links.iter().any(|twin| same(link, twin))),
        );
    let here = |id: SelectionId| this.canvas.items.iter().any(|item| item.id == id);
    for link in changed_links {
        different.extend([link.from, link.to].into_iter().filter(|id| here(*id)));
    }
    different
}

/// What happens when something in a scene is clicked.
pub type SelectHandler = Rc<dyn Fn(SelectionId, &mut Window, &mut App)>;

/// The near ridge: where it starts on the left, ends on the right, and how
/// far its curve swells between.
const NEAR_RISE: f32 = 0.93;
const NEAR_FALL: f32 = 0.90;
const NEAR_LIFT: f32 = 0.04;

/// How high the near ridge stands at `x` (both as fractions of the stage),
/// following the same two curves `horizon` draws it with, so built things
/// stand on it rather than sinking into it where it rises.
fn near_ridge_top(x: f32) -> f32 {
    let quad = |a: (f32, f32), c: (f32, f32), b: (f32, f32), t: f32| {
        let u = 1.0 - t;
        (
            u * u * a.0 + 2.0 * u * t * c.0 + t * t * b.0,
            u * u * a.1 + 2.0 * u * t * c.1 + t * t * b.1,
        )
    };
    let middle = (0.45, (NEAR_RISE + NEAR_FALL) / 2.0);
    let (a, c, b) = if x <= middle.0 {
        ((0.0, NEAR_RISE), (0.2, NEAR_RISE - NEAR_LIFT), middle)
    } else {
        (middle, (0.75, NEAR_FALL + NEAR_LIFT), (1.0, NEAR_FALL))
    };
    // Find the point on the curve above x; a few halvings are plenty.
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..24 {
        let mid = (low + high) / 2.0;
        if quad(a, c, b, mid).0 < x {
            low = mid;
        } else {
            high = mid;
        }
    }
    quad(a, c, b, (low + high) / 2.0).1
}

/// How big built things are, and how far into the ridge they are set.
const MARK_SINK: f32 = 0.012;
const MARK_WIDTH: f32 = 26.0;
const MARK_HEIGHT: f32 = 36.0;
/// Past this many, the oldest fall off the left edge.
const MARK_LIMIT: usize = 16;

/// Where each of the newest `count` built things stands along the horizon,
/// as fractions of the stage's width, evenly spaced from left to right.
fn mark_positions(count: usize) -> Vec<f32> {
    let shown = count.min(MARK_LIMIT);
    (0..shown)
        .map(|index| 0.06 + 0.88 * (index as f32 + 0.5) / MARK_LIMIT as f32)
        .collect()
}

/// A built thing as a small silhouette standing on the ground line at the
/// bottom of its box: a house, a dome, a mast, a tree, a lamp, a shopfront
/// or a bridge.
fn mark_silhouette(shape: MarkShape, colour: Hsla, light: Hsla) -> gpui::Canvas<()> {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<gpui::Pixels>, _, window, _| {
            ui::paint_mark(window, bounds, shape, colour, light)
        },
    )
}

/// Where a World is, behind everything on its stage: its sky washed faintly
/// over the ground, and its two ridges along the bottom edge, so a Mars
/// colony stands on red dust and Icebridge on ice without anything on
/// stage becoming harder to read.
fn horizon(scenery: world_projection::Scenery) -> impl IntoElement {
    let colour = |hex: u32| -> Hsla { rgb(hex).into() };
    let wash = if world_theme::is_dark() { 0.30 } else { 0.22 };
    let sky_top = colour(scenery.sky_top).opacity(wash);
    let sky_bottom = colour(scenery.sky_bottom).opacity(wash);
    let far = colour(scenery.far).opacity(0.55);
    let near = colour(scenery.near).opacity(0.8);
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<gpui::Pixels>, _, window, _| {
            let origin = bounds.origin;
            let width = bounds.size.width;
            let height = bounds.size.height;
            let at = |x: f32, y: f32| point(origin.x + width * x, origin.y + height * y);
            window.paint_quad(gpui::fill(
                bounds,
                linear_gradient(
                    180.0,
                    linear_color_stop(sky_top, 0.0),
                    linear_color_stop(sky_bottom, 1.0),
                ),
            ));
            for (rise, fall, lift, colour) in [
                (0.86, 0.82, 0.05, far),
                (NEAR_RISE, NEAR_FALL, NEAR_LIFT, near),
            ] {
                let mut ridge = PathBuilder::fill();
                ridge.move_to(at(0.0, rise));
                ridge.curve_to(at(0.45, (rise + fall) / 2.0), at(0.2, rise - lift));
                ridge.curve_to(at(1.0, fall), at(0.75, fall + lift));
                ridge.line_to(at(1.0, 1.0));
                ridge.line_to(at(0.0, 1.0));
                ridge.close();
                if let Ok(path) = ridge.build() {
                    window.paint_path(path, colour);
                }
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// What Packs built before the rows were named in plain words still send.
const LEGACY_WHO_ROW: &str = "Actor";
const LEGACY_WITH_ROW: &str = "Targets";

/// Whose face an event wears: whoever did it, or, when nobody did (a
/// payroll that failed, a storm that damaged a boat), the first thing it
/// happened to.
pub fn event_actor(snapshot: &ProjectionSnapshot, selection: SelectionId) -> Option<String> {
    let inspector = snapshot.inspector(selection)?;
    let row = |label: &str| {
        inspector
            .sections
            .iter()
            .flat_map(|section| section.rows.iter())
            .find(|row| row.label == label)
            .map(|row| row.value.as_str())
    };
    row(world_projection::EVENT_WHO_ROW)
        .or_else(|| row(LEGACY_WHO_ROW))
        .or_else(|| {
            row(world_projection::EVENT_WITH_ROW)
                .or_else(|| row(LEGACY_WITH_ROW))
                .and_then(|targets| targets.split(", ").next())
        })
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string)
}

/// How loudly a tone should be drawn: trouble outranks good news, which
/// outranks neither.
fn severity(tone: Tone) -> u8 {
    match tone {
        Tone::Neutral => 0,
        Tone::Good => 1,
        Tone::Warning => 2,
        Tone::Bad => 3,
    }
}

/// The colour a tone is drawn in; neutral news uses the accent.
pub fn tone_token(tone: Tone) -> tokens::Token {
    match tone {
        Tone::Neutral => tokens::ACCENT,
        Tone::Good => tokens::SUCCESS,
        Tone::Warning => tokens::WARNING,
        Tone::Bad => tokens::DANGER,
    }
}

/// Everyone and everything the current news is about (the actors and
/// targets of the events the briefing reports) with the most serious tone
/// of the news about each.
pub fn in_the_news(snapshot: &ProjectionSnapshot, latest: usize) -> BTreeMap<String, Tone> {
    let mut names = BTreeMap::new();
    let mut note = |name: String, tone: Tone| {
        let entry = names.entry(name).or_insert(tone);
        if severity(tone) > severity(*entry) {
            *entry = tone;
        }
    };
    let Some(briefing) = snapshot.briefing.as_ref() else {
        return names;
    };
    let beats = briefing.beats();
    for beat in beats.iter().skip(beats.len().saturating_sub(latest)) {
        let beat: &world_projection::BriefingItem = beat;
        let Some(selection) = beat.selection else {
            continue;
        };
        match selection {
            SelectionId::Event(_) => {
                if let Some(inspector) = snapshot.inspector(selection) {
                    for row in inspector.sections.iter().flat_map(|s| s.rows.iter()) {
                        if [
                            world_projection::EVENT_WHO_ROW,
                            world_projection::EVENT_WITH_ROW,
                            LEGACY_WHO_ROW,
                            LEGACY_WITH_ROW,
                        ]
                        .contains(&row.label.as_str())
                        {
                            for name in row.value.split(", ") {
                                note(name.to_string(), beat.tone);
                            }
                        }
                    }
                }
            }
            SelectionId::Entity(_) | SelectionId::Relation(_) => {
                if let Some(inspector) = snapshot.inspector(selection) {
                    note(inspector.title.clone(), beat.tone);
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
        .map(|item| {
            let (w, h, top) = footprint(item.kind, compact);
            // A change chip under a name stands taller than the caption it
            // replaces, and in a compact scene there was no caption at all.
            let chip = match (item.kind, item.changes.is_empty(), compact) {
                (CanvasItemKind::Actor, false, false) => 6.0,
                (CanvasItemKind::Actor, false, true) | (CanvasItemKind::Object, false, _) => 20.0,
                _ => 0.0,
            };
            (w, h + chip, top)
        })
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
    // A preview can name a relationship that is drawn as a line rather than
    // a node; lighting it up means lighting up the two ends.
    let through_links = |targets: &BTreeSet<SelectionId>, id: SelectionId| {
        snapshot.canvas.links.iter().any(|link| {
            link.selection
                .is_some_and(|selection| targets.contains(&selection))
                && (link.from == id || link.to == id)
        })
    };
    // What glows, and in what colour.
    let glow_of = |item: &CanvasItem| -> Option<Tone> {
        match emphasis {
            Emphasis::News => news.get(&item.label).copied(),
            Emphasis::Only(targets) => (targets.contains(&item.id)
                || through_links(targets, item.id))
            .then_some(Tone::Neutral),
        }
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
    // A preview is something the player is pointing at right now, so it is
    // drawn stronger than news, with a ring as well as a glow.
    let previewing = matches!(emphasis, Emphasis::Only(_));
    let halos = items
        .iter()
        .zip(&placed)
        .filter_map(|(item, position)| {
            glow_of(item).map(|tone| (*position, hsla(tone_token(tone))))
        })
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
            for ((x, y), glow) in &halos {
                let centre = at(*x, *y);
                let layers: &[(f32, f32)] = if previewing {
                    &[(44.0, 0.14), (32.0, 0.22)]
                } else {
                    &[(40.0, 0.10), (30.0, 0.16)]
                };
                if previewing {
                    let radius = 46.0;
                    window.paint_quad(quad(
                        Bounds::new(
                            point(centre.x - px(radius), centre.y - px(radius)),
                            size(px(radius * 2.0), px(radius * 2.0)),
                        ),
                        px(radius),
                        glow.opacity(0.0),
                        px(2.0),
                        glow.opacity(0.7),
                        BorderStyle::default(),
                    ));
                }
                for &(radius, alpha) in layers {
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
        .when_some(snapshot.scenery, |stage, scenery| {
            stage.child(horizon(scenery))
        })
        .child(backdrop);

    // What the World has built stands on its horizon, oldest on the left,
    // so a colony a week old looks lived in. The newest rises into place.
    let shown = mark_positions(snapshot.canvas.marks.len());
    let marks_start = snapshot.canvas.marks.len() - shown.len();
    let (silhouette, light) = match snapshot.scenery {
        Some(scenery) => (rgb(scenery.near).into(), rgb(scenery.sun).into()),
        None => (hsla(tokens::BORDER_STRONG), hsla(tokens::WARNING)),
    };
    for (offset, x) in shown.into_iter().enumerate() {
        let index = marks_start + offset;
        let mark = &snapshot.canvas.marks[index];
        let newest = index + 1 == snapshot.canvas.marks.len();
        // The entrance animation moves its element by margin, so it moves
        // the picture inside the positioned box, never the box itself.
        let picture = div()
            .size_full()
            .child(mark_silhouette(mark.shape, silhouette, light).size_full());
        let mut node = div()
            .id(SharedString::from(format!("mark-{index}")))
            .absolute()
            .left(relative(x))
            .top(relative(near_ridge_top(x) + MARK_SINK))
            .ml(px(-MARK_WIDTH / 2.0))
            .mt(px(-MARK_HEIGHT))
            .w(px(MARK_WIDTH))
            .h(px(MARK_HEIGHT))
            .child(if newest {
                ui::arrive(picture, format!("mark-{index}"), 0).into_any_element()
            } else {
                picture.into_any_element()
            });
        if let Some(selection) = mark.selection {
            let on_select = on_select.clone();
            node = node
                .cursor_pointer()
                .on_click(move |_, window, cx| on_select(selection, window, cx));
        }
        scene = scene.child(node);
    }

    // Trouble breathes: a ring that slowly swells and fades around anything
    // whose news is a warning or worse, so it is seen before it is read.
    for (item, (x, y)) in items.iter().zip(placed.iter().copied()) {
        let Some(tone @ (Tone::Warning | Tone::Bad)) = glow_of(item) else {
            continue;
        };
        let ring = hsla(tone_token(tone));
        let size_px = 76.0;
        scene = scene.child(
            div()
                .absolute()
                .left(relative(x))
                .top(relative(y))
                .ml(px(-size_px / 2.0))
                .mt(px(-size_px / 2.0))
                .size(px(size_px))
                .rounded_full()
                .border_2()
                .border_color(ring)
                .with_animation(
                    SharedString::from(format!("trouble-{}", item.id.stable_key())),
                    Animation::new(Duration::from_millis(2400)).repeat(),
                    move |ring_div, t| {
                        // Swell outward while fading, then start again.
                        let grow = 0.72 + 0.28 * t;
                        ring_div
                            .opacity(0.75 * (1.0 - t))
                            .size(px(size_px * grow))
                            .ml(px(-size_px * grow / 2.0))
                            .mt(px(-size_px * grow / 2.0))
                    },
                ),
        );
    }

    for (item, (x, y)) in items.iter().zip(placed.iter().copied()) {
        let selection = item.id;
        let selected = selected == Some(selection);
        let glow = glow_of(item);
        let id = SharedString::from(format!("canvas-{}", selection.stable_key()));
        let node = match item.kind {
            CanvasItemKind::Actor => actor_node(
                &item.label,
                &item.detail,
                &item.changes,
                selected,
                glow,
                compact,
            ),
            CanvasItemKind::Place => place_node(
                &item.label,
                item.shape,
                &item.detail,
                &item.changes,
                selected,
                glow,
                compact,
            ),
            CanvasItemKind::Object => object_node(&item.label, &item.changes, selected),
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

/// What the World keeps score of, as a row of meters that is always on
/// screen: the stakes, the way Frostpunk keeps Hope and Discontent in view.
/// While a choice is under the pointer, each meter it would move shows
/// where it would end up and which way, as Reigns does before a swipe.
pub fn gauges(
    snapshot: &ProjectionSnapshot,
    previewing: Option<&world_projection::ProjectionCommand>,
) -> Option<Div> {
    if snapshot.gauges.is_empty() {
        return None;
    }
    let mut row = div().w_full().flex().gap_3();
    for gauge in &snapshot.gauges {
        let movement = previewing.and_then(|command| {
            command
                .moves
                .iter()
                .find(|step| step.gauge == gauge.id)
                .map(|step| step.by)
        });
        row = row.child(gauge_meter(gauge, movement));
    }
    Some(row)
}

/// How a move reads next to a meter: one arrow for a nudge, two for a
/// shove.
pub fn movement_arrows(by: i32) -> &'static str {
    match by {
        i32::MIN..=-200 => "▼▼",
        -199..=-1 => "▼",
        0 => "",
        1..=199 => "▲",
        _ => "▲▲",
    }
}

fn gauge_meter(gauge: &world_projection::Gauge, movement: Option<i32>) -> Div {
    let fill = tone_token(gauge.tone);
    let value = gauge.value.clamp(0.0, 1.0);
    let target = movement.map(|by| (value + by as f32 / 1000.0).clamp(0.0, 1.0));
    let mut track = div()
        .relative()
        .w_full()
        .h(px(6.0))
        .rounded_full()
        .overflow_hidden()
        .bg(ui::color(tokens::BORDER))
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .h_full()
                .w(relative(value))
                .rounded_full()
                .bg(ui::color(fill)),
        );
    // Where the choice would take it: the stretch it would gain drawn
    // faintly past the fill, the stretch it would lose cut back out.
    if let Some(target) = target {
        let (from, to) = (value.min(target), value.max(target));
        track = track.child(
            div()
                .absolute()
                .top_0()
                .h_full()
                .left(relative(from))
                .w(relative((to - from).max(0.02)))
                .bg(ui::color(tokens::ACCENT).opacity(if target >= value { 0.45 } else { 0.8 })),
        );
    }
    let arrows = movement.map(movement_arrows).unwrap_or("");
    div()
        .flex_1()
        .min_w(px(0.0))
        .px_3()
        .py_2()
        .rounded_lg()
        .border_1()
        .border_color(ui::color(if movement.is_some() {
            tokens::ACCENT
        } else {
            tokens::BORDER
        }))
        .bg(ui::color(tokens::SURFACE))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(ui::caption(gauge.label.clone()).truncate())
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap_1()
                        .when(!arrows.is_empty(), |line| {
                            line.child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(ui::color(tokens::ACCENT_TEXT))
                                    .child(arrows),
                            )
                        })
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(ui::color(tokens::TEXT))
                                .child(gauge.reading.clone()),
                        ),
                ),
        )
        .child(track)
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
        // The bars grow in from the left when the strip first appears, so
        // the World's life reads as something that unfolded.
        bars = bars.child(
            div()
                .w(px(4.0))
                .h(px(height))
                .rounded_sm()
                .bg(ui::color(colour))
                .with_animation(
                    SharedString::from(format!("activity-{}-{index}", times.len())),
                    Animation::new(ui::ENTRANCE).with_easing(ui::staggered(index / 4)),
                    move |bar, t| bar.h(px(2.0 + (height - 2.0) * t)),
                ),
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
                "{} moments · {}",
                times.len(),
                snapshot.span_label(first, last)
            ))),
    )
}

fn actor_node(
    name: &str,
    detail: &str,
    changes: &[CanvasChange],
    selected: bool,
    glow: Option<Tone>,
    compact: bool,
) -> Div {
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
    } else if let Some(tone) = glow {
        face = face.border_color(ui::color(tone_token(tone)));
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
        // On a return, what changed stands where what they are would.
        .when(!changes.is_empty(), |node| {
            node.child(change_chip(&changes[0]))
        })
        .when(changes.is_empty() && !compact, |node| {
            // Backed like the name above it, so it reads over any ground,
            // night-time streets and red dust included.
            node.child(
                ui::caption(crate::macos::capitalize(detail))
                    .px_1()
                    .rounded_sm()
                    .bg(ui::color(tokens::SURFACE).opacity(0.72))
                    .truncate(),
            )
        })
}

/// "cash ↓48", tinted by whether the change is good or bad news.
pub fn change_chip(change: &CanvasChange) -> Div {
    let (text, ground) = match change.tone {
        Tone::Neutral => (tokens::TEXT_SECONDARY, tokens::ROW_HOVER),
        Tone::Good => (tokens::SUCCESS, tokens::SUCCESS_SOFT),
        Tone::Warning => (tokens::WARNING, tokens::WARNING_SOFT),
        Tone::Bad => (tokens::DANGER, tokens::DANGER_SOFT),
    };
    div()
        .px_2()
        .rounded_full()
        .bg(ui::color(ground))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(ui::color(text))
        .truncate()
        .child(change_text(change))
}

/// The shortest honest reading of a change, sized for a chip under a name:
/// a number becomes its difference ("cash ↓48"), anything else its new
/// value ("→ repaired"), since the old one is what the player remembers.
pub fn change_text(change: &CanvasChange) -> String {
    let difference = match (change.before.parse::<i64>(), change.after.parse::<i64>()) {
        (Ok(before), Ok(after)) if after > before => format!("↑{}", after - before),
        (Ok(before), Ok(after)) => format!("↓{}", before - after),
        _ => format!("→ {}", change.after),
    };
    if change.label.is_empty() {
        difference
    } else {
        format!("{} {difference}", change.label)
    }
}

fn place_node(
    name: &str,
    shape: Option<MarkShape>,
    detail: &str,
    changes: &[CanvasChange],
    selected: bool,
    glow: Option<Tone>,
    compact: bool,
) -> Div {
    let (_, height, _) = footprint(CanvasItemKind::Place, compact);
    // A place in the news wears the news's colour on its edge; a halo
    // behind an opaque tile would not be seen.
    // The news's colour wins the edge; being selected shows as a tint, so
    // selecting a place in trouble never hides the trouble.
    let edge = match (glow, selected) {
        (Some(tone), _) => tone_token(tone),
        (None, true) => tokens::ACCENT,
        (None, false) => tokens::BORDER_STRONG,
    };
    div()
        .h(px(height))
        .px_3()
        .rounded_lg()
        .when(glow.is_some() || selected, |tile| tile.border_2())
        .when(glow.is_none() && !selected, |tile| tile.border_1())
        .border_color(ui::color(edge))
        .bg(ui::color(if selected {
            tokens::ACCENT_SOFT
        } else {
            tokens::SURFACE
        }))
        .shadow_sm()
        .hover(|style| style.border_color(ui::color(tokens::ACCENT)))
        .flex()
        .items_center()
        .gap_2()
        .child(place_icon(shape))
        .child(
            div()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(ui::row_title(name.to_string()).truncate())
                .child(match changes.first() {
                    Some(change) => change_chip(change),
                    None => ui::caption(crate::macos::capitalize(detail)).truncate(),
                }),
        )
}

fn object_node(name: &str, changes: &[CanvasChange], selected: bool) -> Div {
    // On a return, what changed hangs under the pill, as it does under a
    // person's name.
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .child(object_pill(name, selected))
        .when_some(changes.first(), |node, change| {
            node.child(change_chip(change))
        })
}

fn object_pill(name: &str, selected: bool) -> Div {
    div()
        .w_full()
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
/// A place's own silhouette on a small tile of ground: a dome for a
/// habitat, a shopfront for an arcade, a span for a bridge.
fn place_icon(shape: Option<MarkShape>) -> Div {
    div()
        .flex_shrink_0()
        .size(px(28.0))
        .p(px(5.0))
        .rounded_md()
        .bg(ui::color(tokens::SCENE_BOTTOM))
        .child(
            mark_silhouette(
                shape.unwrap_or_default(),
                hsla(tokens::SUCCESS),
                hsla(tokens::WARNING),
            )
            .size_full(),
        )
}

#[cfg(test)]
mod tests {
    use super::{
        change_text, differences, event_actor, footprint, layout, mark_positions, near_ridge_top,
        CROWDED, MARK_LIMIT, NEAR_FALL, NEAR_RISE,
    };
    use world_projection::ProjectionSnapshot;
    use world_projection::{CanvasChange, CanvasItem, CanvasItemKind, SelectionId, Tone};

    #[test]
    fn the_ridge_is_highest_where_it_swells_and_meets_its_ends() {
        assert!((near_ridge_top(0.0) - NEAR_RISE).abs() < 0.001);
        assert!((near_ridge_top(1.0) - NEAR_FALL).abs() < 0.001);
        assert!(near_ridge_top(0.2) < NEAR_RISE);
    }

    #[test]
    fn built_things_stand_in_order_and_the_oldest_give_way() {
        assert!(mark_positions(0).is_empty());
        let three = mark_positions(3);
        assert_eq!(three.len(), 3);
        assert!(three.windows(2).all(|pair| pair[0] < pair[1]));
        let many = mark_positions(40);
        assert_eq!(many.len(), MARK_LIMIT);
        assert!(many.iter().all(|x| (0.0..=1.0).contains(x)));
    }

    #[test]
    fn a_change_reads_as_its_difference_or_its_new_value() {
        let change = |label: &str, before: &str, after: &str| CanvasChange {
            label: label.into(),
            before: before.into(),
            after: after.into(),
            tone: Tone::Neutral,
        };
        assert_eq!(change_text(&change("cash", "85", "37")), "cash ↓48");
        assert_eq!(change_text(&change("cash", "37", "85")), "cash ↑48");
        assert_eq!(change_text(&change("", "broken", "repaired")), "→ repaired");
    }

    fn item(id: u64, kind: CanvasItemKind, x: f32, y: f32) -> CanvasItem {
        CanvasItem {
            id: SelectionId::from_stable_key(&format!("entity-{id}")).expect("an entity key"),
            kind,
            label: format!("Item {id}"),
            detail: String::new(),
            x,
            y,
            changes: Vec::new(),
            shape: None,
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

    #[test]
    fn a_relationship_that_differs_marks_both_people_even_when_they_do_not() {
        use world_projection::{CanvasLink, CanvasLinkTone};
        let mut partners = ProjectionSnapshot::default();
        partners.canvas.items = vec![
            item(1, CanvasItemKind::Actor, 0.2, 0.2),
            item(2, CanvasItemKind::Actor, 0.8, 0.8),
            item(3, CanvasItemKind::Place, 0.5, 0.1),
        ];
        let (nia, tomas) = (partners.canvas.items[0].id, partners.canvas.items[1].id);
        partners.canvas.links = vec![CanvasLink {
            from: nia,
            to: tomas,
            label: "Shared project".into(),
            tone: CanvasLinkTone::Warm,
            strength: 0.6,
            selection: None,
        }];
        let mut rivals = partners.clone();
        rivals.canvas.links[0].label = "Rivals".into();
        rivals.canvas.links[0].tone = CanvasLinkTone::Strained;

        let expected = vec![nia, tomas];
        assert_eq!(
            differences(&partners, &rivals)
                .into_iter()
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            differences(&rivals, &partners)
                .into_iter()
                .collect::<Vec<_>>(),
            expected
        );
        assert!(differences(&partners, &partners.clone()).is_empty());
    }

    #[test]
    fn an_event_nobody_did_wears_the_face_of_whoever_it_happened_to() {
        use world_projection::{InspectorProjection, InspectorRow, InspectorSection};
        let event = SelectionId::from_stable_key("event-7").expect("an event key");
        let inspector = |rows: Vec<(&str, &str)>| InspectorProjection {
            selection: event,
            title: String::new(),
            subtitle: String::new(),
            sections: vec![InspectorSection {
                title: String::new(),
                rows: rows
                    .into_iter()
                    .map(|(label, value)| InspectorRow {
                        label: label.into(),
                        value: value.into(),
                    })
                    .collect(),
            }],
        };
        let mut snapshot = ProjectionSnapshot::default();
        snapshot
            .inspectors
            .insert(event, inspector(vec![("Targets", "Jonas, Harbor Bakery")]));
        assert_eq!(event_actor(&snapshot, event).as_deref(), Some("Jonas"));
        snapshot.inspectors.insert(
            event,
            inspector(vec![("Actor", "Mara"), ("Targets", "Jonas")]),
        );
        assert_eq!(event_actor(&snapshot, event).as_deref(), Some("Mara"));
    }
}
