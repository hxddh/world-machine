//! Where the card floats at the bottom of the World, clear of whoever
//! asks and of the crowd, and how much of the scene it leaves.

use super::*;

/// Where the card floats at the bottom of the World: centred, or docked
/// to one side with the width it takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dock {
    pub(crate) side: DockSide,
    pub(crate) w: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DockSide {
    Centre,
    Left,
    Right,
}

/// How far the card keeps from the stage's edges.
pub(super) const CARD_MARGIN: f32 = 16.0;
/// How far a docked card keeps from the middle of the stage.
pub(super) const CARD_CLEAR: f32 = 64.0;

/// Where the card goes on a stage `width` wide, with whoever asks standing
/// at `asker_x` on screen and the rest of the people whose figures reach
/// down into the card's band at `crowd` (each where they stand and half
/// their width): wherever it covers the fewest of them, never the asker.
/// With an asker it prefers the far side from them, no wider than half the
/// stage less its margins, so the half they stand in stays clear; with
/// nobody asking, centred, unless that covers someone a side would not
/// (v0.29's talk card over the group by the water).
pub(crate) fn card_dock(width: f32, asker_x: Option<f32>, crowd: &[(f32, f32)]) -> Dock {
    let full = (CARD_WIDTH * crate::text_scale().sqrt()).min(width - CARD_MARGIN * 2.0);
    let centre = Dock {
        side: DockSide::Centre,
        w: full,
    };
    if width < 720.0 {
        return centre;
    }
    // Clear of the middle by more than anyone's shoulders, so someone
    // standing right at it is still in the open half.
    let side = |side| Dock {
        side,
        w: full.min(width / 2.0 - CARD_CLEAR),
    };
    let order = match asker_x {
        Some(x) if x > width / 2.0 => [side(DockSide::Left), centre, side(DockSide::Right)],
        Some(_) => [side(DockSide::Right), centre, side(DockSide::Left)],
        None => [centre, side(DockSide::Left), side(DockSide::Right)],
    };
    let covers = |dock: &Dock, x: f32, half: f32| {
        let (left, right) = dock.span(width);
        x + half > left && x - half < right
    };
    let cost = |dock: &Dock| {
        let asker = asker_x.is_some_and(|x| covers(dock, x, ASKER_HALF));
        let crowd = crowd
            .iter()
            .filter(|(x, half)| covers(dock, *x, *half))
            .count();
        usize::from(asker) * 1000 + crowd
    };
    // And each side narrowed to the open ground beside everyone, where that
    // still leaves a card wide enough to read.
    let least = CARD_LEAST * crate::text_scale().sqrt();
    let everyone = crowd
        .iter()
        .copied()
        .chain(asker_x.map(|x| (x, ASKER_HALF)))
        .collect::<Vec<_>>();
    let gap = 12.0;
    let left_free = everyone
        .iter()
        .map(|(x, half)| x - half - gap)
        .fold(width - CARD_MARGIN, f32::min)
        - CARD_MARGIN;
    let right_free = width
        - CARD_MARGIN
        - everyone
            .iter()
            .map(|(x, half)| x + half + gap)
            .fold(CARD_MARGIN, f32::max);
    let fitted = [(DockSide::Left, left_free), (DockSide::Right, right_free)]
        .into_iter()
        .filter(|(_, free)| *free >= least)
        .map(|(side, free)| Dock {
            side,
            w: free.min(full),
        });
    let mut best = order[0];
    for dock in order[1..].iter().copied().chain(fitted) {
        if cost(&dock) < cost(&best) {
            best = dock;
        }
    }
    best
}

/// How tall the return film's card stands at the foot of the stage,
/// generously (it is a caption and a button or two).
pub(crate) const FILM_CARD_ROOM: f32 = 200.0;

/// The card's three places on a stage `width` wide: centred, and docked
/// to either side clear of the middle.
pub(super) fn docks(width: f32) -> [Dock; 3] {
    let full = (CARD_WIDTH * crate::text_scale().sqrt()).min(width - CARD_MARGIN * 2.0);
    let side = |side| Dock {
        side,
        w: full.min(width / 2.0 - CARD_CLEAR),
    };
    [
        Dock {
            side: DockSide::Centre,
            w: full,
        },
        side(DockSide::Left),
        side(DockSide::Right),
    ]
}

/// The screen rect (left, top, right, bottom) of the film's card docked
/// as `dock` on a stage `width` by `height`.
pub(crate) fn film_card_rect(dock: Dock, (width, height): (f32, f32)) -> (f32, f32, f32, f32) {
    let (left, right) = dock.span(width);
    (
        left,
        height - CARD_MARGIN - FILM_CARD_ROOM,
        right,
        height - CARD_MARGIN,
    )
}

/// What of a scene, staged as `staged`, stands on screen under a card at
/// `card` (left, top, right, bottom) on a stage `width` wide: everyone it
/// stages and every prop it has out (but bunting, which is overhead), each
/// by the box it is drawn in, wherever the scene's camera has them, named
/// `person <who>` or `prop <what>`. Those off the screen are not counted.
pub(crate) fn scene_under_card(
    staged: &diorama::staging::Staged,
    stage: &Stage,
    card: (f32, f32, f32, f32),
    width: f32,
) -> Vec<String> {
    let Some(camera) = staged.camera else {
        return Vec::new();
    };
    let z = camera.zoom;
    let fig = stage.figure_h * z;
    let boxes = staged
        .people
        .iter()
        .map(|person| {
            let (x, y) = camera.at(stage, person.x, person.y);
            (
                format!("person {:?}", person.who),
                (x - fig * 0.22, y - fig, x + fig * 0.22, y),
            )
        })
        .chain(
            staged
                .props
                .iter()
                .filter(|prop| {
                    prop.held_by.is_none()
                        && !matches!(
                            prop.prop,
                            world_projection::Prop::Bunting | world_projection::Prop::Rain
                        )
                })
                .map(|prop| {
                    let (x, y) = camera.at(stage, prop.x, prop.y);
                    let (w, h) = (prop.w * z, prop.h * z);
                    (
                        format!("prop {:?}", prop.prop),
                        (x - w / 2.0, y - h, x + w / 2.0, y),
                    )
                }),
        );
    let (left, top, right, bottom) = card;
    boxes
        .filter(|(_, (x0, _, x1, _))| *x1 > 0.0 && *x0 < width)
        .filter(|(_, (x0, y0, x1, y1))| *x1 > left && *x0 < right && *y1 > top && *y0 < bottom)
        .map(|(what, _)| what)
        .collect()
}

/// Where a return beat's card docks for the whole of its scene, on a
/// stage `width` by `height`: of the centre and either side, wherever it
/// covers least of the scene (its people and props wherever its camera
/// has them, half a second apart all through it), the centre before a
/// side. A function of the scene, the stage and the window alone, so the
/// same whenever in the beat it is first asked, and steady through it
/// (v0.30 S3c round 3: the card jumped about across a storm, and sat on a
/// farewell's ferry).
pub(crate) fn scene_dock(
    script: &world_projection::SceneScript,
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    (width, height): (f32, f32),
) -> Dock {
    let [centre, left, right] = docks(width);
    if width < 720.0 {
        return centre;
    }
    let mut costs = [0_usize; 3];
    let moments = (script.seconds / 0.5).ceil().max(0.0) as usize;
    for n in 0..=moments {
        let t = (n as f32 * 0.5).min(script.seconds);
        let staged = diorama::staging::stage_scene(script, stage, snapshot, t);
        for (cost, dock) in costs.iter_mut().zip([centre, left, right]) {
            *cost += scene_under_card(&staged, stage, film_card_rect(dock, (width, height)), width)
                .len();
        }
    }
    let best = (0..3).min_by_key(|n| (costs[*n], *n)).unwrap_or(0);
    [centre, left, right][best]
}

/// The narrowest a card at the foot of the stage is made to keep clear of
/// people, at text size 1.
pub(super) const CARD_LEAST: f32 = 400.0;

/// Half of anyone's width, generously, for keeping a card off an asker.
pub(super) const ASKER_HALF: f32 = 24.0;

impl Dock {
    /// The span of the stage the card covers, left to right.
    pub(crate) fn span(&self, width: f32) -> (f32, f32) {
        match self.side {
            DockSide::Left => (CARD_MARGIN, CARD_MARGIN + self.w),
            DockSide::Right => (width - CARD_MARGIN - self.w, width - CARD_MARGIN),
            DockSide::Centre => (width / 2.0 - self.w / 2.0, width / 2.0 + self.w / 2.0),
        }
    }
}

/// A card floating at the bottom of the World, where `dock` puts it.
pub(super) fn bottom_card(card: impl IntoElement, dock: Dock) -> Div {
    let row = div()
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .pb_4()
        .px(px(CARD_MARGIN))
        .flex();
    match dock.side {
        DockSide::Centre => row.justify_center(),
        DockSide::Left => row.justify_start(),
        DockSide::Right => row.justify_end(),
    }
    .child(
        div()
            .debug_selector(|| "bottom-card".into())
            .w(px(dock.w))
            // A click on the card is the card's, never the ground's.
            .occlude()
            .child(card),
    )
}
