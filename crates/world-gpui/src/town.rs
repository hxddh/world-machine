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

/// The scene is laid out in proportion to whatever height it is given, so this
/// is a budget rather than a drawing constraint. It was 300, which on a 768px
/// screen left the workspace 206 pixels and cut the news off mid-sentence. The
/// picture is the thing you look at first and the news is the thing you came
/// for; the picture does not get to take the news off the screen.
pub(crate) const SCENE_HEIGHT: f32 = 150.0;

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
    // A resident the news is not about is still standing there. At 0.45 they
    // washed out to nearly nothing against the quay; the point is to lift the
    // few the return concerns, not to erase everyone else.
    hsla(hue / 360.0, 0.32, 0.52, if dimmed { 0.7 } else { 1.0 })
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
            // as shut from across the room. Spaced by a fraction of the pane
            // — at a fixed 6px apart they marched straight out of a short
            // window and the shutters stopped being shutters.
            let step = pane.height / 5.0;
            for slat in 0..4 {
                let mut bar = *pane;
                bar.y += step * (slat as f32 + 0.6);
                bar.height = (step * 0.34).max(1.0);
                window.paint_quad(quad(bar, origin, ink()));
            }
        }
    }
    window.paint_quad(quad(building.door, origin, timber()));
}

/// The picture, and the names that go on it.
///
/// Everything is painted inside one canvas, including the words. The first
/// version laid the scene out against a hardcoded 1100px and positioned the
/// labels as absolutely-placed divs over the top; in a window narrower than
/// that, the boat and its name were simply off the right-hand edge. A canvas
/// is handed its real width at paint time, which is the only place the true
/// figure is known.
pub(crate) fn scene(items: &[CanvasItem], lit: &[SelectionId]) -> Div {
    let items = items.to_vec();
    let lit = lit.to_vec();
    // Not shrinkable. The scene degrades badly below its budget — the parts
    // of a building are sized in pixels, not in fractions of the wall — so it
    // takes a fixed share and the workspace takes the rest.
    div().w_full().h(px(SCENE_HEIGHT)).flex_shrink_0().child(
        canvas(
            move |_, _, _| {},
            move |bounds, _, window, cx| {
                let plan = town_scene::plan(
                    &items,
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height),
                );
                paint(window, bounds, &plan, &lit);
                paint_names(window, cx, bounds, &plan);
            },
        )
        .size_full(),
    )
}

/// The words: a sign over each door, a name under each figure.
fn paint_names(
    window: &mut gpui::Window,
    cx: &mut gpui::App,
    bounds: Bounds<Pixels>,
    plan: &ScenePlan,
) {
    for building in &plan.buildings {
        write(
            window,
            cx,
            bounds,
            building.label.clone().into(),
            building.sign.x,
            building.sign.y + 3.0,
            building.sign.width,
            11.0,
            0x2f2822,
        );
    }
    for spot in &plan.folk {
        write(
            window,
            cx,
            bounds,
            spot.label.clone().into(),
            spot.feet.0 - 34.0,
            spot.feet.1 + 5.0,
            68.0,
            11.0,
            0x2f2822,
        );
    }
    for object in &plan.objects {
        // "Holed" is a thing that happens to boats. A damaged thing on the
        // quay is not holed, and saying so would be the drawing inventing
        // detail the World never claimed.
        let words = match (object.state, object.afloat) {
            (CanvasItemState::Gone, _) => format!("{} · gone", object.label),
            (CanvasItemState::Hurt, true) => format!("{} · holed", object.label),
            (CanvasItemState::Hurt, false) => format!("{} · damaged", object.label),
            _ => object.label.clone(),
        };
        // Under the thing, unless there is no "under" left. A boat sits at
        // 0.86 of the scene's height, so on a short scene the name went 16px
        // below it and out of the frame entirely — it came out painted across
        // the news underneath the picture.
        const LABEL_H: f32 = 10.0;
        // A line lower than a person's name. Both used to be written just
        // under the thing they name, so on the quay "Wedding bread order ·
        // gone" and "Mara" came out on the same line across each other.
        let below = object.at.1 + 16.0 + LABEL_H + 2.0;
        // Above the thing when there is no room under it. Clamping it to the
        // bottom edge instead dragged the name up onto the object's own mark:
        // a gone boat is a short dark dash, and "Sea Finch · gone" came out
        // struck through by it.
        let floor = if below + LABEL_H + 2.0 > f32::from(bounds.size.height) {
            object.at.1 - LABEL_H - 6.0
        } else {
            below
        };
        write(
            window,
            cx,
            bounds,
            words.into(),
            object.at.0 - 60.0,
            floor.max(2.0),
            120.0,
            LABEL_H,
            if object.afloat { 0x2f2822 } else { 0x4a4038 },
        );
    }
}

/// One centred line, clipped to the frame rather than allowed to run off it.
#[allow(clippy::too_many_arguments)]
fn write(
    window: &mut gpui::Window,
    cx: &mut gpui::App,
    bounds: Bounds<Pixels>,
    words: SharedString,
    x: f32,
    y: f32,
    width: f32,
    size: f32,
    colour: u32,
) {
    let x = x
        .max(2.0)
        .min((f32::from(bounds.size.width) - width).max(2.0));
    let font = window.text_style().font();
    let run = gpui::TextRun {
        len: words.len(),
        font,
        color: crate::theme_rgb(colour).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let line = window
        .text_system()
        .shape_line(words, px(size), &[run], None);
    let _ = line.paint(
        point(bounds.origin.x + px(x), bounds.origin.y + px(y)),
        px(size * 1.3),
        gpui::TextAlign::Center,
        Some(px(width)),
        window,
        cx,
    );
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
        if !object.afloat {
            // A thing on the quay is a crate, not a boat. The plan says which
            // is which from where the World put it; a thing indoors drawn with
            // a mast and a sail was the first thing wrong with this picture.
            let body = town_scene::Rect {
                x: x - 13.0,
                y: y - 13.0,
                width: 26.0,
                height: 18.0,
            };
            if object.state == CanvasItemState::Gone {
                // Absence, drawn as the outline of what is not there. The
                // first version filled it pale grey, which on a pale quay
                // meant nothing appeared at all.
                let edge = hsla(0.09, 0.12, 0.34, 0.5);
                for side in [
                    town_scene::Rect {
                        height: 1.5,
                        ..body
                    },
                    town_scene::Rect {
                        y: body.bottom() - 1.5,
                        height: 1.5,
                        ..body
                    },
                    town_scene::Rect { width: 1.5, ..body },
                    town_scene::Rect {
                        x: body.right() - 1.5,
                        width: 1.5,
                        ..body
                    },
                ] {
                    window.paint_quad(quad(side, bounds, edge));
                }
                continue;
            }
            window.paint_quad(quad(body, bounds, timber()));
            window.paint_quad(quad(
                town_scene::Rect {
                    y: y - 6.0,
                    height: 2.0,
                    ..body
                },
                bounds,
                ink(),
            ));
            continue;
        }
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
            hsla(0.08, 0.45, 0.72, if dimmed { 0.7 } else { 1.0 }),
        );
    }
}
