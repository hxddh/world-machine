//! Which way someone is turned, and a person's drawing turned that way
//! from its own parts (v0.30 S1c): to the camera, three-quarter, in
//! profile, or from behind. And the few strokes more a figure gets once it
//! is drawn big enough to read them: a shaded side, a coat's opening, a
//! nose, the light on the hair. Presentation only.

use world_projection::{DrawPart, DrawShape, Drawing, Ink};

/// How someone is turned to the camera. Which side they face is the pose's
/// `facing`; this is how far round.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Turn {
    /// To the camera: the drawing as its Pack drew it.
    #[default]
    Front,
    /// Three-quarter: the face toward their side, the far half of it
    /// foreshortened, the body a little narrower.
    ThreeQuarter,
    /// In profile: one eye, the face at the head's edge, the body narrow.
    Profile,
    /// From behind: no face, the back of the head.
    Back,
}

/// How tall, in window pixels, someone must be drawn for the turns and the
/// detail to read; smaller (the overview, the strip, a cover) they are
/// drawn as their Pack drew them.
pub const READS_FROM: f32 = 90.0;

/// The head of a person's drawing: its largest ellipse of skin in the top
/// third, as (x, y, rx, ry) in the drawing's units.
fn head_of(drawing: &Drawing) -> Option<(f32, f32, f32, f32)> {
    drawing
        .parts
        .iter()
        .filter(|part| part.ink == Ink::Skin)
        .filter_map(|part| match part.shape {
            DrawShape::Ellipse { x, y, rx, ry } if y > 0.6 => Some((x, y, rx, ry)),
            _ => None,
        })
        .max_by(|a, b| a.2.total_cmp(&b.2))
}

/// A shape's middle and how far it reaches either side and up and down.
fn extent(shape: &DrawShape) -> (f32, f32, f32, f32) {
    match shape {
        DrawShape::Rect { x, y, w, h, .. } => (x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0),
        DrawShape::Ellipse { x, y, rx, ry } => (*x, *y, *rx, *ry),
        DrawShape::Polygon { points } => {
            let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for (x, y) in points {
                x0 = x0.min(*x);
                y0 = y0.min(*y);
                x1 = x1.max(*x);
                y1 = y1.max(*y);
            }
            (
                (x0 + x1) / 2.0,
                (y0 + y1) / 2.0,
                (x1 - x0) / 2.0,
                (y1 - y0) / 2.0,
            )
        }
        DrawShape::Line { from, to, .. } => (
            (from.0 + to.0) / 2.0,
            (from.1 + to.1) / 2.0,
            (from.0 - to.0).abs() / 2.0,
            (from.1 - to.1).abs() / 2.0,
        ),
    }
}

/// Whether a part is a feature of the face on `head`: eyes, brows, a
/// mouth, cheeks, glasses, a nose. Small, well inside the head, and neither
/// skin (a hand raised to the eyes) nor clothes nor a lock of hair.
fn on_the_face(part: &DrawPart, (hx, hy, rx, ry): (f32, f32, f32, f32)) -> bool {
    let hair_line = part.ink == Ink::Hair && matches!(part.shape, DrawShape::Line { .. });
    if matches!(part.ink, Ink::Skin | Ink::Clothes) || (part.ink == Ink::Hair && !hair_line) {
        return false;
    }
    let (x, y, w, h) = extent(&part.shape);
    let inside = ((x - hx) / rx).powi(2) + ((y - hy) / ry).powi(2) <= 0.8;
    inside && w <= rx * 0.85 && h <= ry * 0.8
}

/// Every point of a shape moved by `f`, and a line's width or a rect's
/// corners scaled across by `across`.
fn mapped(shape: &DrawShape, across: f32, f: impl Fn(f32, f32) -> (f32, f32)) -> DrawShape {
    match shape {
        DrawShape::Rect { x, y, w, h, round } => {
            let (left, _) = f(*x, *y);
            let (right, _) = f(x + w, *y);
            DrawShape::Rect {
                x: left.min(right),
                y: *y,
                w: (right - left).abs(),
                h: *h,
                round: round * across,
            }
        }
        DrawShape::Ellipse { x, y, rx, ry } => {
            let (cx, cy) = f(*x, *y);
            DrawShape::Ellipse {
                x: cx,
                y: cy,
                rx: rx * across,
                ry: *ry,
            }
        }
        DrawShape::Polygon { points } => DrawShape::Polygon {
            points: points.iter().map(|(x, y)| f(*x, *y)).collect(),
        },
        DrawShape::Line { from, to, width } => DrawShape::Line {
            from: f(from.0, from.1),
            to: f(to.0, to.1),
            width: width * across.max(0.8),
        },
    }
}

/// `drawing` turned `turn` round from the camera, from its own parts: the
/// body narrowed, the face's features moved toward the side they face (the
/// far ones foreshortened, or out of sight in profile), or from behind no
/// face and the back of the head in their hair. A drawing with no head of
/// skin (a bird, a robot drawn otherwise) stays as it is.
pub fn turned(drawing: &Drawing, turn: Turn) -> Drawing {
    if turn == Turn::Front {
        return drawing.clone();
    }
    let Some(head) = head_of(drawing) else {
        return drawing.clone();
    };
    let (hx, hy, rx, ry) = head;
    // How narrow the body is turned, how far the face moves to the side,
    // and how far the far half of the face is foreshortened.
    let (body, shift, far) = match turn {
        Turn::Front => (1.0, 0.0, 1.0),
        Turn::ThreeQuarter => (0.88, rx * 0.3, 0.6),
        Turn::Profile => (0.68, rx * 0.55, 0.0),
        Turn::Back => (0.96, 0.0, 1.0),
    };
    let mut parts = Vec::with_capacity(drawing.parts.len() + 1);
    for part in &drawing.parts {
        let face = on_the_face(part, head);
        let (x, _, _, _) = extent(&part.shape);
        if face {
            match turn {
                Turn::Back => continue,
                Turn::Profile if x < hx - rx * 0.06 => continue,
                _ => {}
            }
            let shape = mapped(&part.shape, 1.0, |px, py| {
                let dx = px - hx;
                let dx = if dx < 0.0 { dx * far } else { dx };
                (hx + dx + shift, py)
            });
            parts.push(DrawPart {
                shape,
                ..part.clone()
            });
            continue;
        }
        let is_head = matches!(part.shape, DrawShape::Ellipse { x, y, rx: r, .. }
            if part.ink == Ink::Skin && (x - hx).abs() < 1e-4 && (y - hy).abs() < 1e-4 && (r - rx).abs() < 1e-4);
        let across = if is_head && turn == Turn::Profile {
            0.92
        } else {
            body
        };
        // Hair and hats sit a little back on a head turned away: its bulk
        // behind the face.
        let back = if part.ink == Ink::Hair && turn != Turn::Back {
            -shift * 0.3
        } else {
            0.0
        };
        let mut shape = mapped(&part.shape, across, |px, py| {
            (hx + (px - hx) * across + back, py)
        });
        // In profile the far arm and leg are behind the body: a shade
        // darker.
        let mut part = DrawPart {
            shape: shape.clone(),
            ..part.clone()
        };
        if turn == Turn::Profile && x < hx - 0.05 && part.ink != Ink::Hair && !is_head {
            part.tone -= 0.15;
        }
        if is_head && turn == Turn::Profile {
            // The head turned: its middle a touch toward the face.
            if let DrawShape::Ellipse { x, .. } = &mut shape {
                *x += rx * 0.06;
            }
            part.shape = shape;
        }
        parts.push(part);
        // From behind, the back of the head is their hair.
        if is_head && turn == Turn::Back {
            parts.push(DrawPart {
                shape: DrawShape::Ellipse {
                    x: hx,
                    y: hy + ry * 0.12,
                    rx: rx * 0.97,
                    ry: ry * 0.92,
                },
                ink: Ink::Hair,
                tone: -0.05,
                stances: part_stances(drawing, head),
                moods: Vec::new(),
                swing: 0.0,
            });
        }
    }
    Drawing {
        parts,
        ..drawing.clone()
    }
}

/// The stances the head of `drawing` shows in (every stance: empty).
fn part_stances(
    drawing: &Drawing,
    (hx, hy, rx, _): (f32, f32, f32, f32),
) -> Vec<world_projection::Stance> {
    drawing
        .parts
        .iter()
        .find(|part| {
            part.ink == Ink::Skin
                && matches!(part.shape, DrawShape::Ellipse { x, y, rx: r, .. }
                    if (x - hx).abs() < 1e-4 && (y - hy).abs() < 1e-4 && (r - rx).abs() < 1e-4)
        })
        .map(|part| part.stances.clone())
        .unwrap_or_default()
}

/// `drawing` with the few strokes more a figure drawn big enough to read
/// them gets, from its own parts: the side of the coat away from the light
/// shaded, the coat's opening and collar, a nose, and the light catching
/// the top of the hair. A drawing with no coat or head of its own (a bird)
/// stays as it is.
pub fn detailed(drawing: &Drawing) -> Drawing {
    let Some((hx, hy, rx, ry)) = head_of(drawing) else {
        return drawing.clone();
    };
    // The coat: the largest polygon of clothes below the head.
    let coat = drawing
        .parts
        .iter()
        .enumerate()
        .filter(|(_, part)| part.ink == Ink::Clothes && part.stances.is_empty())
        .filter_map(|(at, part)| match &part.shape {
            DrawShape::Polygon { points } if points.len() >= 3 => {
                let (x, y, w, h) = extent(&part.shape);
                (y < hy).then_some((at, (x, y, w, h), points.clone()))
            }
            _ => None,
        })
        .max_by(|a, b| (a.1 .2 * a.1 .3).total_cmp(&(b.1 .2 * b.1 .3)));
    let hair = drawing
        .parts
        .iter()
        .enumerate()
        .filter(|(_, part)| part.ink == Ink::Hair && part.stances.is_empty())
        .filter_map(|(at, part)| match part.shape {
            DrawShape::Ellipse { x, y, rx, ry } if y > hy => Some((at, (x, y, rx, ry))),
            _ => None,
        })
        .max_by(|a, b| a.1 .2.total_cmp(&b.1 .2));
    let mut inserts: Vec<(usize, DrawPart)> = Vec::new();
    if let Some((at, (cx, cy, w, h), points)) = coat {
        // The far side in shade: the coat's own outline, cut to its right
        // third (the light comes from the left of the picture).
        let cut = cx + w * 0.33;
        let shade = points
            .iter()
            .map(|(x, y)| (x.max(cut), *y))
            .collect::<Vec<_>>();
        inserts.push((
            at + 1,
            DrawPart {
                shape: DrawShape::Polygon { points: shade },
                ink: Ink::Shade,
                tone: 0.0,
                stances: Vec::new(),
                moods: Vec::new(),
                swing: 0.0,
            },
        ));
        // Its opening, down the middle, and the collar.
        inserts.push((
            at + 1,
            DrawPart {
                shape: DrawShape::Line {
                    from: (cx, cy + h * 0.92),
                    to: (cx, cy - h * 0.95),
                    width: w * 0.035,
                },
                ink: Ink::Clothes,
                tone: -0.32,
                stances: Vec::new(),
                moods: Vec::new(),
                swing: 0.0,
            },
        ));
        inserts.push((
            at + 1,
            DrawPart {
                shape: DrawShape::Polygon {
                    points: vec![
                        (cx - w * 0.36, cy + h * 1.02),
                        (cx + w * 0.36, cy + h * 1.02),
                        (cx, cy + h * 0.72),
                    ],
                },
                ink: Ink::Clothes,
                tone: 0.22,
                stances: Vec::new(),
                moods: Vec::new(),
                swing: 0.0,
            },
        ));
    }
    if let Some((at, (x, y, hrx, hry))) = hair {
        inserts.push((
            at + 1,
            DrawPart {
                shape: DrawShape::Ellipse {
                    x: x - hrx * 0.3,
                    y: y + hry * 0.35,
                    rx: hrx * 0.32,
                    ry: hry * 0.28,
                },
                ink: Ink::Hair,
                tone: 0.32,
                stances: Vec::new(),
                moods: Vec::new(),
                swing: 0.0,
            },
        ));
    }
    let mut parts = Vec::with_capacity(drawing.parts.len() + inserts.len() + 1);
    for (at, part) in drawing.parts.iter().enumerate() {
        parts.push(part.clone());
        for (_, insert) in inserts.iter().filter(|(after, _)| *after == at + 1) {
            parts.push(insert.clone());
        }
    }
    // A nose, a shade of the face, on it like the eyes (so it turns with
    // them and is gone from behind).
    parts.push(DrawPart {
        shape: DrawShape::Ellipse {
            x: hx,
            y: hy - ry * 0.22,
            rx: rx * 0.1,
            ry: ry * 0.12,
        },
        ink: Ink::Colour(0xb07a5a),
        tone: 0.0,
        stances: Vec::new(),
        moods: Vec::new(),
        swing: 0.0,
    });
    Drawing {
        parts,
        ..drawing.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person() -> Drawing {
        world_projection::person_base("someone").with("someone", world_projection::short_hair())
    }

    fn eyes(drawing: &Drawing) -> Vec<f32> {
        drawing
            .parts
            .iter()
            .filter(|part| part.ink == Ink::Colour(0x2a2522))
            .filter_map(|part| match part.shape {
                DrawShape::Ellipse { x, y, rx, .. } if (y - 0.81).abs() < 0.01 && rx < 0.05 => {
                    Some(x)
                }
                _ => None,
            })
            .collect()
    }

    /// Each turn is drawn from the person's own parts: two eyes to the
    /// camera, both moved toward their side three-quarter, one in profile
    /// at the head's edge, none from behind with their hair over the head.
    #[test]
    fn a_person_turns_from_their_own_parts() {
        let front = person();
        assert_eq!(eyes(&front).len(), 2);
        let three = turned(&front, Turn::ThreeQuarter);
        let (f, t) = (eyes(&front), eyes(&three));
        assert_eq!(t.len(), 2);
        assert!(t.iter().sum::<f32>() > f.iter().sum::<f32>() + 0.1);
        let profile = turned(&front, Turn::Profile);
        assert_eq!(eyes(&profile).len(), 1);
        assert!(eyes(&profile)[0] > 0.2);
        let back = turned(&front, Turn::Back);
        assert!(eyes(&back).is_empty());
        assert!(back.parts.iter().any(|part| part.ink == Ink::Hair
            && matches!(part.shape, DrawShape::Ellipse { y, .. } if (y - 0.8).abs() < 0.03)));
        // Narrower in profile than to the camera.
        let width = |drawing: &Drawing| {
            drawing
                .parts
                .iter()
                .filter(|part| part.ink == Ink::Clothes && part.stances.is_empty())
                .map(|part| extent(&part.shape).2)
                .fold(0.0, f32::max)
        };
        assert!(width(&profile) < width(&front) * 0.75);
        // Detail: more parts, the same head.
        let detailed = detailed(&front);
        assert!(detailed.parts.len() >= front.parts.len() + 4);
        assert_eq!(head_of(&detailed), head_of(&front));
    }
}
