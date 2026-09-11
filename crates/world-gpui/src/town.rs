//! Draw a World as the place it is.
//!
//! The window used to show a World as rows of text and a scatter of boxes.
//! Every product this is meant to sit beside — a colony, a kingdom, a village
//! you come back to — puts the place on screen and stands the people in it, so
//! that whether the shop is shut is something you see rather than something you
//! read.
//!
//! All the arithmetic is in `world_projection::town_scene`, which has tests
//! that run anywhere. Nothing here decides where a thing goes; it decides what
//! colour it is and puts the paint down.

use gpui::{
    canvas, div, fill, hsla, point, prelude::*, px, Bounds, Div, Hsla, Path, Pixels, SharedString,
};
use world_projection::town_scene::{self, BuildingShape, ScenePlan};
use world_projection::{CanvasItem, CanvasItemKind, CanvasItemState, SelectionId};

pub(crate) const SCENE_HEIGHT: f32 = 300.0;

fn ink() -> Hsla {
    hsla(0.07, 0.14, 0.16, 1.0)
}
fn sky() -> Hsla {
    hsla(0.11, 0.28, 0.87, 1.0)
}
fn quay() -> Hsla {
    hsla(0.11, 0.32, 0.79, 1.0)
}
fn sea() -> Hsla {
    hsla(0.47, 0.16, 0.57, 1.0)
}
fn wall(state: CanvasItemState) -> Hsla {
    match state {
        CanvasItemState::Working => hsla(0.11, 0.46, 0.90, 1.0),
        _ => hsla(0.11, 0.20, 0.83, 1.0),
    }
}
fn glass(state: CanvasItemState) -> Hsla {
    match state {
        CanvasItemState::Working => hsla(0.11, 0.78, 0.68, 1.0),
        _ => hsla(0.09, 0.06, 0.42, 1.0),
    }
}
fn timber() -> Hsla {
    hsla(0.08, 0.30, 0.42, 1.0)
}
/// A roof colour per building, so the street is not three of the same house.
fn roof(index: usize) -> Hsla {
    const ROOFS: [f32; 4] = [0.045, 0.56, 0.14, 0.30];
    hsla(ROOFS[index % ROOFS.len()], 0.42, 0.42, 1.0)
}
fn coat(hue: f32, dimmed: bool) -> Hsla {
    hsla(hue / 360.0, 0.32, 0.52, if dimmed { 0.45 } else { 1.0 })
}

/// Whether this World has told the canvas enough to be drawn as a place.
///
/// A Pack that supplies a loose scatter of items with no places and nobody
/// standing at them still gets the plain boxes it always had.
pub(crate) fn is_a_place(items: &[CanvasItem]) -> bool {
    items.iter().any(|item| item.kind == CanvasItemKind::Place)
        && items.iter().any(|item| item.at.is_some())
}

fn tri(a: (f32, f32), b: (f32, f32), c: (f32, f32), origin: Bounds<Pixels>) -> Path<Pixels> {
    let at = |p: (f32, f32)| point(origin.origin.x + px(p.0), origin.origin.y + px(p.1));
    let mut path = Path::new(at(a));
    path.line_to(at(b));
    path.line_to(at(c));
    path.line_to(at(a));
    path
}

/// A circle, four curves round. Used for heads and for the glow behind
/// somebody the news is about.
fn disc(centre: (f32, f32), r: f32, origin: Bounds<Pixels>) -> Path<Pixels> {
    let at = |x: f32, y: f32| point(origin.origin.x + px(x), origin.origin.y + px(y));
    let (cx, cy) = centre;
    let k = r * 0.5523;
    let mut path = Path::new(at(cx, cy - r));
    path.curve_to(at(cx + r, cy), at(cx + k, cy - r));
    path.curve_to(at(cx, cy + r), at(cx + r, cy + k));
    path.curve_to(at(cx - r, cy), at(cx - k, cy + r));
    path.curve_to(at(cx, cy - r), at(cx - r, cy - k));
    path
}

fn quad(r: town_scene::Rect, origin: Bounds<Pixels>, colour: Hsla) -> gpui::PaintQuad {
    fill(
        Bounds::new(
            point(origin.origin.x + px(r.x), origin.origin.y + px(r.y)),
            gpui::size(px(r.width), px(r.height)),
        ),
        colour,
    )
}

fn band(origin: Bounds<Pixels>, top: f32, bottom: f32, colour: Hsla) -> gpui::PaintQuad {
    fill(
        Bounds::new(
            point(origin.origin.x, origin.origin.y + px(top)),
            gpui::size(origin.size.width, px(bottom - top)),
        ),
        colour,
    )
}

fn paint_building(
    window: &mut gpui::Window,
    origin: Bounds<Pixels>,
    index: usize,
    building: &BuildingShape,
) {
    window.paint_quad(quad(building.body, origin, wall(building.state)));
    window.paint_path(
        tri(
            (building.body.x - 10.0, building.body.y),
            building.roof_peak,
            (building.body.right() + 10.0, building.body.y),
            origin,
        ),
        roof(index),
    );
    window.paint_quad(quad(building.sign, origin, hsla(0.11, 0.50, 0.94, 1.0)));
    for pane in &building.windows {
        window.paint_quad(quad(*pane, origin, glass(building.state)));
        if building.state != CanvasItemState::Working {
            // Shutters: four slats across a dark window, so a shut shop reads
            // as shut from across the room.
            for slat in 0..4 {
                let mut bar = *pane;
                bar.y += 4.0 + slat as f32 * 6.0;
                bar.height = 2.0;
                window.paint_quad(quad(bar, origin, ink()));
            }
        }
    }
    window.paint_quad(quad(building.door, origin, timber()));
}

/// The picture, and the names that go on it.
pub(crate) fn scene(items: &[CanvasItem], lit: &[SelectionId]) -> Div {
    let plan = town_scene::plan(items, 1100.0, SCENE_HEIGHT);
    let lit = lit.to_vec();
    let paint_plan = plan.clone();

    let mut layer = div().relative().w_full().h(px(SCENE_HEIGHT));
    layer = layer.child(
        canvas(
            move |_, _, _| {},
            move |bounds, _, window, _| paint(window, bounds, &paint_plan, &lit),
        )
        .absolute()
        .inset_0(),
    );
    for building in &plan.buildings {
        layer = layer.child(caption(
            building.sign.x + 4.0,
            building.sign.y + 4.0,
            building.sign.width - 8.0,
            building.label.clone(),
            0x2f2822,
        ));
    }
    for spot in &plan.folk {
        layer = layer.child(caption(
            spot.feet.0 - 30.0,
            spot.feet.1 + 6.0,
            60.0,
            spot.label.clone(),
            0x2f2822,
        ));
    }
    for object in &plan.objects {
        let words = match object.state {
            CanvasItemState::Gone => format!("{} · gone", object.label),
            CanvasItemState::Hurt => format!("{} · holed", object.label),
            _ => object.label.clone(),
        };
        layer = layer.child(caption(
            object.at.0 - 60.0,
            object.at.1 + 18.0,
            120.0,
            words,
            0x2f2822,
        ));
    }
    layer
}

fn caption(x: f32, y: f32, width: f32, words: impl Into<SharedString>, colour: u32) -> Div {
    div()
        .absolute()
        .left(px(x))
        .top(px(y))
        .w(px(width))
        .text_xs()
        .text_center()
        .text_color(crate::theme_rgb(colour))
        .child(words.into())
}

fn paint(window: &mut gpui::Window, bounds: Bounds<Pixels>, plan: &ScenePlan, lit: &[SelectionId]) {
    window.paint_quad(band(bounds, 0.0, plan.ground_y, sky()));
    window.paint_quad(band(bounds, plan.ground_y, plan.water_y, quay()));
    window.paint_quad(band(bounds, plan.water_y, plan.height, sea()));

    for (index, building) in plan.buildings.iter().enumerate() {
        paint_building(window, bounds, index, building);
    }

    if let Some(deck) = plan.jetty {
        window.paint_quad(quad(deck, bounds, hsla(0.09, 0.38, 0.66, 1.0)));
        for pile in 0..3 {
            let x = deck.x + 30.0 + pile as f32 * (deck.width - 60.0) / 2.0;
            window.paint_quad(quad(
                town_scene::Rect {
                    x,
                    y: deck.bottom(),
                    width: 8.0,
                    height: plan.height - deck.bottom(),
                },
                bounds,
                timber(),
            ));
        }
    }

    for object in &plan.objects {
        let (x, y) = object.at;
        match object.state {
            // An empty mooring is a rope on the water and nothing else, which
            // is the point of drawing it at all.
            CanvasItemState::Gone => {
                window.paint_quad(quad(
                    town_scene::Rect {
                        x: x - 16.0,
                        y,
                        width: 32.0,
                        height: 2.5,
                    },
                    bounds,
                    ink(),
                ));
            }
            state => {
                let listing = state == CanvasItemState::Hurt;
                let hull = if listing {
                    hsla(0.045, 0.40, 0.48, 1.0)
                } else {
                    hsla(0.055, 0.55, 0.52, 1.0)
                };
                let dip = if listing { 6.0 } else { 0.0 };
                window.paint_path(
                    tri(
                        (x - 30.0, y - dip),
                        (x + 30.0, y + dip),
                        (x, y + 16.0),
                        bounds,
                    ),
                    hull,
                );
                window.paint_quad(quad(
                    town_scene::Rect {
                        x: x - 1.0,
                        y: y - 34.0,
                        width: 2.5,
                        height: 34.0,
                    },
                    bounds,
                    ink(),
                ));
                if !listing {
                    window.paint_path(
                        tri(
                            (x + 3.0, y - 32.0),
                            (x + 24.0, y - 20.0),
                            (x + 3.0, y - 10.0),
                            bounds,
                        ),
                        hsla(0.11, 0.50, 0.94, 1.0),
                    );
                }
            }
        }
    }

    let anybody_lit = !lit.is_empty();
    for spot in &plan.folk {
        let is_lit = lit.contains(&spot.id);
        let dimmed = anybody_lit && !is_lit;
        let (x, y) = spot.feet;
        if is_lit {
            window.paint_path(
                disc((x, y - 20.0), 22.0, bounds),
                hsla(spot.hue / 360.0, 0.60, 0.62, 0.22),
            );
        }
        window.paint_quad(quad(
            town_scene::Rect {
                x: x - 9.0,
                y: y - 20.0,
                width: 18.0,
                height: 20.0,
            },
            bounds,
            coat(spot.hue, dimmed),
        ));
        window.paint_path(
            disc((x, y - 26.0), 6.0, bounds),
            hsla(0.08, 0.55, 0.80, if dimmed { 0.45 } else { 1.0 }),
        );
    }
}
