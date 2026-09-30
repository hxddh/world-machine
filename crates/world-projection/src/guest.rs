//! Guests from other Worlds: who they are, how they look and what they
//! say, read from the World they come from and never written back to it;
//! and how a guest stands on the canvas of the World they visit for as
//! long as they stay.
//!
//! A guest crosses from one World to another only as words. How they look
//! and the drawing they are drawn with travel as short texts (a "look
//! code" and a "drawing code") that the World they visit records with the
//! visit, so the visit replays exactly without the World they came from.

use crate::{
    AgeStage, CanvasItem, CanvasItemKind, Carry, DrawPart, DrawShape, Drawing, Ink, Look, Mood,
    ProjectionSnapshot, SelectionId, Stance, Voice,
};
use world_core::EventId;

/// A guest from another World: who they are, where from, the letter they
/// bring, and something they leave to keep; and, when their World shows
/// them, how they look, the drawing they are drawn with and something they
/// say there.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Guest {
    pub name: String,
    pub from: String,
    pub letter: String,
    pub gift: String,
    /// How they look at home.
    pub look: Option<Look>,
    /// The drawing their own World draws them with.
    pub drawing: Option<Drawing>,
    /// Something they say, in their own World's words.
    pub line: Option<String>,
}

/// What a guest says when their World gave them nothing to say.
pub const GUEST_GREETING: &str = "Thought I'd come and see how you're all getting on.";

/// The most parts a guest's drawing brings; the most points a shape does,
/// and all its shapes together. A resident as either Pack draws them has
/// about seventy parts.
pub const MOST_GUEST_PARTS: usize = 128;
pub const MOST_GUEST_POINTS: usize = 32;
pub const MOST_GUEST_ALL_POINTS: usize = 1024;
/// The most fixed colours a guest's drawing uses; the rest are the roles
/// (clothes, hair, skin…) the World paints in its own colours.
pub const MOST_GUEST_COLOURS: usize = 32;
/// How far from its middle any point of a guest's drawing may reach, in
/// its own height: a figure stays about its own size (residents reach
/// just under one).
pub const GUEST_DRAWING_REACH: f32 = 2.0;

/// The longest drawing code a visit records. A resident's is about 4 KB;
/// it is kept with the visit and travels on in the World's own code, so it
/// is kept small.
pub const MOST_DRAWING_CODE: usize = 8 * 1024;

impl Guest {
    /// Someone from the World a snapshot shows, read and never written:
    /// whoever last said something there, bringing that line as their
    /// letter and a postcard of the place. `None` when nobody lives there.
    pub fn from_snapshot(snapshot: &ProjectionSnapshot) -> Option<Self> {
        let residents = Self::residents(snapshot);
        let spoke = snapshot.voices.iter().rev().find_map(|voice| {
            let who = snapshot
                .canvas
                .items
                .iter()
                .find(|item| item.id == voice.speaker && item.kind == CanvasItemKind::Actor)?;
            residents.iter().find(|guest| guest.name == who.label)
        });
        let mut guest = spoke.or(residents.first())?.clone();
        guest.letter = guest.line.clone().unwrap_or_else(|| GUEST_GREETING.into());
        Some(guest)
    }

    /// Everyone living in the World a snapshot shows, each as a guest
    /// they could be: drawn as their World draws them, and saying the last
    /// thing they said there. Read and never written.
    pub fn residents(snapshot: &ProjectionSnapshot) -> Vec<Self> {
        let from = snapshot.title.clone();
        snapshot
            .canvas
            .items
            .iter()
            .filter(|item| item.kind == CanvasItemKind::Actor && !item.label.trim().is_empty())
            .filter(|item| !matches!(item.id, SelectionId::Event(_)))
            .map(|item| {
                let line = snapshot
                    .voices
                    .iter()
                    .rev()
                    .find(|voice| voice.speaker == item.id)
                    .map(|voice| voice.line.clone())
                    .or_else(|| {
                        snapshot
                            .talks
                            .iter()
                            .find(|talk| talk.who == item.id)
                            .map(|talk| talk.answer.clone())
                    })
                    .filter(|line| !line.trim().is_empty());
                Guest {
                    name: item.label.clone(),
                    from: from.clone(),
                    letter: GUEST_GREETING.into(),
                    gift: format!("a postcard of {from}"),
                    look: item.look,
                    drawing: snapshot.drawing_of(item).cloned(),
                    line,
                }
            })
            .collect()
    }

    /// The guest's look as a short text a visit can record.
    pub fn look_code(&self) -> Option<String> {
        self.look.as_ref().map(look_code)
    }

    /// The drawing the guest is drawn with where they stay: their own, as
    /// it travels (to a thousandth of its size).
    pub fn travelling_drawing(&self) -> Option<Drawing> {
        let code = self.drawing_code()?;
        drawing_from_code(&self.drawing.as_ref()?.id, &code)
    }

    /// The guest's drawing as a short text a visit can record: only a
    /// drawing within a guest's bounds (see [`guest_drawing_is_sound`]).
    pub fn drawing_code(&self) -> Option<String> {
        self.drawing
            .as_ref()
            .filter(|drawing| guest_drawing_is_sound(drawing))
            .map(drawing_code)
            .filter(|code| code.len() <= MOST_DRAWING_CODE)
    }
}

// ---- Looks, as text.

fn carry_id(carry: Carry) -> &'static str {
    match carry {
        Carry::Tool => "tool",
        Carry::Book => "book",
        Carry::Bread => "bread",
        Carry::Fish => "fish",
        Carry::Basket => "basket",
        Carry::Satchel => "satchel",
        Carry::Plant => "plant",
        Carry::Mug => "mug",
    }
}

fn carry_from(id: &str) -> Option<Carry> {
    Some(match id {
        "tool" => Carry::Tool,
        "book" => Carry::Book,
        "bread" => Carry::Bread,
        "fish" => Carry::Fish,
        "basket" => Carry::Basket,
        "satchel" => Carry::Satchel,
        "plant" => Carry::Plant,
        "mug" => Carry::Mug,
        _ => return None,
    })
}

fn age_id(age: AgeStage) -> &'static str {
    match age {
        AgeStage::Baby => "baby",
        AgeStage::Child => "child",
        AgeStage::Teen => "teen",
        AgeStage::Adult => "adult",
        AgeStage::Elder => "elder",
    }
}

fn age_from(id: &str) -> Option<AgeStage> {
    Some(match id {
        "baby" => AgeStage::Baby,
        "child" => AgeStage::Child,
        "teen" => AgeStage::Teen,
        "adult" => AgeStage::Adult,
        "elder" => AgeStage::Elder,
        _ => return None,
    })
}

/// A look as `key=value` pairs and flags, `;` between them:
/// `c=3a6ea5;h=2b1b0e;k=tool;a=elder;g;s`.
pub fn look_code(look: &Look) -> String {
    let mut parts = Vec::new();
    for (key, colour) in [("c", look.clothes), ("h", look.hair), ("k", look.skin)] {
        if let Some(colour) = colour {
            parts.push(format!("{key}={:06x}", colour & 0xFF_FFFF));
        }
    }
    if let Some(carry) = look.carries {
        parts.push(format!("t={}", carry_id(carry)));
    }
    if let Some(age) = look.age {
        parts.push(format!("a={}", age_id(age)));
    }
    for (flag, on) in [("b", look.bird), ("g", look.grey), ("s", look.stoop)] {
        if on {
            parts.push(flag.into());
        }
    }
    parts.join(";")
}

/// A look read back from its code; anything it does not know is left out.
pub fn look_from_code(code: &str) -> Look {
    let mut look = Look::default();
    for part in code.split(';').map(str::trim) {
        let (key, value) = part.split_once('=').unwrap_or((part, ""));
        let colour = || {
            u32::from_str_radix(value, 16)
                .ok()
                .filter(|c| *c <= 0xFF_FFFF)
        };
        match key {
            "c" => look.clothes = colour(),
            "h" => look.hair = colour(),
            "k" => look.skin = colour(),
            "t" => look.carries = carry_from(value),
            "a" => look.age = age_from(value),
            "b" => look.bird = true,
            "g" => look.grey = true,
            "s" => look.stoop = true,
            _ => {}
        }
    }
    look
}

// ---- Drawings, as text.

fn number(value: f32) -> String {
    let rounded = (value.clamp(-100.0, 100.0) * 1000.0).round() / 1000.0;
    let text = format!("{rounded:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() || text == "-" || text == "-0" {
        "0".into()
    } else {
        text.to_string()
    }
}

fn point(at: (f32, f32)) -> String {
    format!("{},{}", number(at.0), number(at.1))
}

/// A drawing as text: its aspect, then a part a clause, `;` between them.
/// A part is a shape letter and its numbers, then `@ink`, and after that
/// `~tone`, `!stances`, `?moods` and `^swing` when they are not plain.
/// `0.45;R -0.1,0.2 0.2,0.4 0.05@clothes~0.2!walking,talking;E 0,0.9 0.1,0.1@skin`
pub fn drawing_code(drawing: &Drawing) -> String {
    let mut out = number(drawing.aspect);
    for part in drawing.parts.iter().take(MOST_GUEST_PARTS) {
        out.push(';');
        match &part.shape {
            DrawShape::Rect { x, y, w, h, round } => out.push_str(&format!(
                "R {} {} {}",
                point((*x, *y)),
                point((*w, *h)),
                number(*round)
            )),
            DrawShape::Ellipse { x, y, rx, ry } => {
                out.push_str(&format!("E {} {}", point((*x, *y)), point((*rx, *ry))))
            }
            DrawShape::Polygon { points } => {
                out.push('P');
                for at in points.iter().take(MOST_GUEST_POINTS) {
                    out.push(' ');
                    out.push_str(&point(*at));
                }
            }
            DrawShape::Line { from, to, width } => out.push_str(&format!(
                "L {} {} {}",
                point(*from),
                point(*to),
                number(*width)
            )),
        }
        out.push('@');
        out.push_str(&part.ink.id());
        if part.tone != 0.0 {
            out.push_str(&format!("~{}", number(part.tone)));
        }
        if !part.stances.is_empty() {
            let stances = part.stances.iter().map(|s| s.id()).collect::<Vec<_>>();
            out.push_str(&format!("!{}", stances.join(",")));
        }
        if !part.moods.is_empty() {
            let moods = part.moods.iter().map(|m| m.id()).collect::<Vec<_>>();
            out.push_str(&format!("?{}", moods.join(",")));
        }
        if part.swing != 0.0 {
            out.push_str(&format!("^{}", number(part.swing)));
        }
    }
    out
}

fn read_number(text: &str) -> Option<f32> {
    let value = text.parse::<f32>().ok()?;
    value.is_finite().then(|| value.clamp(-100.0, 100.0))
}

fn read_point(text: &str) -> Option<(f32, f32)> {
    let (x, y) = text.split_once(',')?;
    Some((read_number(x)?, read_number(y)?))
}

fn read_part(clause: &str) -> Option<DrawPart> {
    // The marks after the ink, each introduced by its own character.
    let (shape, rest) = clause.split_once('@')?;
    let mut marks = Vec::new();
    let mut current = (' ', String::new());
    for c in rest.chars() {
        if matches!(c, '~' | '!' | '?' | '^') {
            marks.push(std::mem::replace(&mut current, (c, String::new())));
        } else {
            current.1.push(c);
        }
    }
    marks.push(current);
    let mut words = shape.split_whitespace();
    let letter = words.next()?;
    let numbers = words.collect::<Vec<_>>();
    let shape = match (letter, numbers.as_slice()) {
        ("R", [at, size, round]) => {
            let ((x, y), (w, h)) = (read_point(at)?, read_point(size)?);
            DrawShape::Rect {
                x,
                y,
                w,
                h,
                round: read_number(round)?,
            }
        }
        ("E", [at, radii]) => {
            let ((x, y), (rx, ry)) = (read_point(at)?, read_point(radii)?);
            DrawShape::Ellipse { x, y, rx, ry }
        }
        ("P", points) if points.len() <= MOST_GUEST_POINTS => DrawShape::Polygon {
            points: points
                .iter()
                .map(|at| read_point(at))
                .collect::<Option<Vec<_>>>()?,
        },
        ("L", [from, to, width]) => DrawShape::Line {
            from: read_point(from)?,
            to: read_point(to)?,
            width: read_number(width)?,
        },
        _ => return None,
    };
    let mut part = DrawPart {
        shape,
        ink: Ink::Clothes,
        tone: 0.0,
        stances: Vec::new(),
        moods: Vec::new(),
        swing: 0.0,
    };
    for (mark, value) in marks {
        match mark {
            ' ' => {
                // A fixed colour is exactly `#rrggbb`.
                let ink = value.trim();
                if ink.starts_with('#') && ink.len() != 7 {
                    return None;
                }
                part.ink = Ink::from_id(ink)?
            }
            '~' => part.tone = read_number(&value)?.clamp(-1.0, 1.0),
            '!' => part.stances = value.split(',').filter_map(Stance::from_id).collect(),
            '?' => part.moods = value.split(',').filter_map(Mood::from_id).collect(),
            '^' => part.swing = read_number(&value)?,
            _ => {}
        }
    }
    Some(part)
}

/// A drawing read back from its code, named `id`; `None` if the code is
/// not one, or not a drawing a guest may bring (see
/// [`guest_drawing_is_sound`]). A part it cannot read is left out.
pub fn drawing_from_code(id: &str, code: &str) -> Option<Drawing> {
    if code.len() > MOST_DRAWING_CODE {
        return None;
    }
    let mut clauses = code.split(';');
    let aspect = read_number(clauses.next()?)?.clamp(0.05, 8.0);
    let clauses = clauses.collect::<Vec<_>>();
    if clauses.len() > MOST_GUEST_PARTS {
        return None;
    }
    let parts = clauses
        .into_iter()
        .filter_map(read_part)
        .collect::<Vec<_>>();
    let drawing = Drawing::new(id, aspect, parts);
    guest_drawing_is_sound(&drawing).then_some(drawing)
}

/// Whether a drawing is one a guest may bring: some parts and no more than
/// [`MOST_GUEST_PARTS`], no shape of more than [`MOST_GUEST_POINTS`]
/// points nor more than [`MOST_GUEST_ALL_POINTS`] in all, every number
/// finite and every point within [`GUEST_DRAWING_REACH`], and no more than
/// [`MOST_GUEST_COLOURS`] fixed colours, each a colour (`0xRRGGBB`).
pub fn guest_drawing_is_sound(drawing: &Drawing) -> bool {
    let within = |value: f32| value.is_finite() && value.abs() <= GUEST_DRAWING_REACH;
    let mut points = 0;
    let mut colours = std::collections::BTreeSet::new();
    if drawing.parts.is_empty()
        || drawing.parts.len() > MOST_GUEST_PARTS
        || !drawing.aspect.is_finite()
    {
        return false;
    }
    for part in &drawing.parts {
        let numbers: Vec<f32> = match &part.shape {
            DrawShape::Rect { x, y, w, h, round } => vec![*x, *y, *w, *h, *round],
            DrawShape::Ellipse { x, y, rx, ry } => vec![*x, *y, *rx, *ry],
            DrawShape::Polygon { points: shape } => {
                if shape.len() > MOST_GUEST_POINTS {
                    return false;
                }
                points += shape.len();
                shape.iter().flat_map(|(x, y)| [*x, *y]).collect()
            }
            DrawShape::Line { from, to, width } => vec![from.0, from.1, to.0, to.1, *width],
        };
        if !numbers.into_iter().all(within) || !part.tone.is_finite() || !part.swing.is_finite() {
            return false;
        }
        if let Ink::Colour(colour) = part.ink {
            if colour > 0xff_ffff {
                return false;
            }
            colours.insert(colour);
        }
    }
    points <= MOST_GUEST_ALL_POINTS && colours.len() <= MOST_GUEST_COLOURS
}

// ---- A guest on the canvas.

/// A guest staying in this World, as the visit recorded them.
#[derive(Clone, Debug, PartialEq)]
pub struct Staying {
    /// The visit: their place on the timeline, and who they are on the
    /// canvas.
    pub visit: EventId,
    pub name: String,
    pub from: String,
    pub line: Option<String>,
    pub look: Option<String>,
    pub drawing: Option<String>,
}

/// The id a staying guest's drawing goes by.
pub fn guest_drawing_id(visit: EventId) -> String {
    format!("guest-{}", visit.0)
}

/// Stands each guest staying in this World on its canvas: drawn with their
/// own drawing (or as the app draws anyone, in their own look), beside the
/// place's people, and saying the line they brought. Presentation only:
/// who they are was recorded with the visit.
pub fn with_guests(snapshot: &mut ProjectionSnapshot, staying: &[Staying]) {
    for (index, guest) in staying.iter().enumerate() {
        let id = SelectionId::Event(guest.visit);
        if snapshot.canvas.items.iter().any(|item| item.id == id) {
            continue;
        }
        let drawing = guest
            .drawing
            .as_deref()
            .and_then(|code| drawing_from_code(&guest_drawing_id(guest.visit), code));
        let drawing_id = drawing.as_ref().map(|drawing| drawing.id.clone());
        if let Some(drawing) = drawing {
            snapshot.drawings.retain(|kept| kept.id != drawing.id);
            snapshot.drawings.push(drawing);
        }
        // Beside the middle of the place, a little apart from each other.
        let x = (0.46 + 0.07 * index as f32).min(0.95);
        let px = snapshot.canvas.width.map(|width| width * x);
        snapshot.canvas.items.push(CanvasItem {
            id,
            kind: CanvasItemKind::Actor,
            label: guest.name.clone(),
            detail: format!("Visiting from {}", guest.from),
            x,
            y: 0.5,
            look: guest.look.as_deref().map(look_from_code),
            drawing: drawing_id,
            stance: Some(Stance::Talking),
            mood: Some(Mood::Happy),
            px,
            ..CanvasItem::default()
        });
        if let Some(line) = guest.line.as_ref().filter(|line| !line.trim().is_empty()) {
            snapshot.voices.push(Voice {
                moment: id,
                speaker: id,
                line: line.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawing() -> Drawing {
        Drawing::new(
            "resident",
            0.45,
            vec![
                DrawPart::rect(-0.18, 0.1, 0.36, 0.45, Ink::Clothes)
                    .round(0.08)
                    .tone(0.2),
                DrawPart::ellipse(0.0, 0.78, 0.13, 0.15, Ink::Skin),
                DrawPart::polygon(
                    &[(-0.2, 0.9), (0.2, 0.9), (0.0, 1.0)],
                    Ink::Colour(0xc0463a),
                )
                .only(&[Stance::Walking, Stance::Waving])
                .feeling(&[Mood::Happy]),
                DrawPart::line((0.1, 0.4), (0.3, 0.6), 0.04, Ink::Hair).swing(0.125),
            ],
        )
    }

    #[test]
    fn a_look_and_a_drawing_survive_their_codes() {
        let look = Look {
            clothes: Some(0x3a6ea5),
            hair: Some(0x2b1b0e),
            skin: None,
            carries: Some(Carry::Fish),
            bird: false,
            age: Some(AgeStage::Elder),
            grey: true,
            stoop: true,
        };
        assert_eq!(look_from_code(&look_code(&look)), look);
        assert_eq!(
            look_from_code("nonsense;c=zz;b"),
            Look {
                bird: true,
                ..Look::default()
            }
        );

        let original = drawing();
        let code = drawing_code(&original);
        let back = drawing_from_code("resident", &code).expect("a drawing");
        assert_eq!(back, original, "{code}");
        assert_eq!(drawing_code(&back), code);
        assert_eq!(drawing_from_code("x", "not a drawing"), None);
        assert_eq!(drawing_from_code("x", "0.5;Q 1 2@wall"), None);
    }

    #[test]
    fn a_staying_guest_stands_on_the_canvas_with_their_drawing_and_line() {
        let mut snapshot = ProjectionSnapshot::default();
        let visit = EventId::new(42);
        let staying = Staying {
            visit,
            name: "Nia".into(),
            from: "Ares Station".into(),
            line: Some("Dust storm cleared at last.".into()),
            look: Some("c=aa3300;a=teen".into()),
            drawing: Some(drawing_code(&drawing())),
        };
        with_guests(&mut snapshot, std::slice::from_ref(&staying));
        with_guests(&mut snapshot, &[staying]);
        assert_eq!(snapshot.canvas.items.len(), 1, "stood once");
        let item = &snapshot.canvas.items[0];
        assert_eq!(item.label, "Nia");
        assert_eq!(item.look.and_then(|look| look.age), Some(AgeStage::Teen));
        let drawn = snapshot.drawing_of(item).expect("their own drawing");
        assert_eq!(drawn.parts, drawing().parts);
        assert_eq!(snapshot.voices[0].line, "Dust storm cleared at last.");
        // A guest from a snapshot is read as their World shows them, and
        // a guest standing here is nobody's resident to send on.
        assert!(Guest::residents(&snapshot).is_empty());
    }

    #[test]
    fn a_hostile_drawing_code_is_refused_whole() {
        let sound = drawing_code(&drawing());
        assert!(drawing_from_code("guest-1", &sound).is_some());
        let part = "E 0,0.5 0.1,0.1@skin";
        let far = "E 0,90 0.1,0.1@skin";
        let polygon = |points: usize| {
            let points = vec!["0.1,0.1"; points].join(" ");
            format!("P {points}@clothes")
        };
        let hostile = [
            // Too long a code, too many parts, a point far away, a shape
            // of too many points, too many points in all.
            format!("0.5;{}", vec![part; 2000].join(";")),
            format!("0.5;{}", vec![part; MOST_GUEST_PARTS + 1].join(";")),
            format!("0.5;{far}"),
            format!("0.5;{}", polygon(MOST_GUEST_POINTS + 1)),
            format!("0.5;{}", vec![polygon(MOST_GUEST_POINTS); 40].join(";")),
            // A colour that is not one, too many colours.
            "0.5;E 0,0.5 0.1,0.1@#ffffffff".to_string(),
            "0.5;E 0,0.5 0.1,0.1@#fff".to_string(),
            format!(
                "0.5;{}",
                (0..MOST_GUEST_COLOURS + 1)
                    .map(|colour| format!("E 0,0.5 0.1,0.1@#{colour:06x}"))
                    .collect::<Vec<_>>()
                    .join(";")
            ),
            // Numbers that are not.
            "0.5;E NaN,0.5 0.1,0.1@skin".to_string(),
            "0.5;E inf,0.5 0.1,0.1@skin".to_string(),
            "nothing".to_string(),
            "0.5".to_string(),
        ];
        for code in hostile {
            assert!(
                drawing_from_code("guest-1", &code).is_none(),
                "{}",
                &code[..code.len().min(80)]
            );
        }
    }

    #[test]
    fn a_guest_brings_only_a_sound_drawing() {
        let mut guest = Guest {
            name: "Ann".into(),
            drawing: Some(drawing()),
            ..Guest::default()
        };
        assert!(guest.drawing_code().is_some());
        guest.drawing = Some(Drawing::new(
            "giant",
            0.5,
            vec![DrawPart::rect(-50.0, 0.0, 100.0, 100.0, Ink::Clothes)],
        ));
        assert_eq!(guest.drawing_code(), None);
    }
}
