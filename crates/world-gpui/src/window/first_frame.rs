//! A new World's first frame (1.0 P5): the place's landmarks whole, round
//! whoever welcomes the player.
//!
//! The camera at rest (v0.30 S1c) is close: people 140 px tall, a window
//! showing under half of what the overview shows. On most first days that
//! is the right frame, the welcomer in a ring before a home. But a place
//! with a landmark wider than half that window (a bridge over a lead of
//! open water) cannot show it at rest beside the welcomer and the homes
//! round it: the first frame showed the welcomer by two empty plots, and
//! the place itself only on a zoom out (the v0.30 review). So on a World's
//! first day, where the overview's opening holds such a landmark whole and
//! the view at rest does not hold all it holds, the window opens pulled
//! back, as close as still holds every
//! building the overview's opening holds whole, with the welcomer well in
//! view: never further out than the overview, never closer than rest.
//! Only the opening: the first zoom or pan is the player's own.
//!
//! Presentation only: a function of the layout and nothing the World
//! records.

use super::arrival::{best_view, busiest_view, opening_view};
use crate::diorama::{keep_on, Camera, Stage};

/// A landmark: a building wider than this share of the window at rest.
pub(crate) const LANDMARK: f32 = 0.5;

/// How many zooms between rest and the overview are tried.
const STEPS: usize = 24;

/// The buildings (by index) the window centred on `centre` at `zoom`
/// shows whole.
pub(crate) fn whole_at(stage: &Stage, zoom: f32, centre: f32) -> Vec<usize> {
    let half = stage.view_w / (2.0 * zoom);
    let centre = keep_on(centre, half, stage.width);
    stage
        .buildings
        .iter()
        .filter(|spot| stage.shows(spot.index, zoom))
        .filter(|spot| {
            spot.x - spot.w / 2.0 >= centre - half && spot.x + spot.w / 2.0 <= centre + half
        })
        .map(|spot| spot.index)
        .collect()
}

/// Where and how close a new World's first frame opens, round whoever
/// welcomes the player at stage `speaker`: `None` to open at rest, as
/// every later sitting does; else the zoom and the centre.
pub(crate) fn first_frame(stage: &Stage, speaker: f32) -> Option<(f32, f32)> {
    let rest = Camera::rest_zoom(stage);
    if rest <= 1.0 {
        return None;
    }
    // What the overview's opening holds whole round the welcomer: the
    // place as a player first met it (v0.29's opening framing).
    let overview = opening_view(stage, speaker, || best_view(stage, speaker));
    let wanted = whole_at(stage, 1.0, overview);
    if wanted.len() < 2 {
        return None;
    }
    let at_rest = whole_at(stage, rest, busiest_view(stage, Some(speaker)));
    let landmark = stage.view_w / rest * LANDMARK;
    let landmark = stage
        .buildings
        .iter()
        .any(|spot| wanted.contains(&spot.index) && spot.w >= landmark);
    if !landmark || wanted.iter().all(|index| at_rest.contains(index)) {
        return None;
    }
    // From rest out: the closest zoom with a centre that holds them all,
    // the welcomer a tenth of the way in or more.
    for step in 0..=STEPS {
        let zoom = rest + (1.0 - rest) * step as f32 / STEPS as f32;
        let half = stage.view_w / (2.0 * zoom);
        let margin = half * 0.1;
        let tries = 48;
        let centre = (0..=tries)
            .map(|n| speaker - (half - margin) + 2.0 * (half - margin) * n as f32 / tries as f32)
            .map(|c| keep_on(c, half, stage.width))
            .filter(|c| (speaker - c).abs() <= half - margin)
            .filter(|c| {
                let whole = whole_at(stage, zoom, *c);
                wanted.iter().all(|index| whole.contains(index))
            })
            .min_by(|a, b| {
                // The most evenly framed: the wanted buildings' middle
                // nearest the window's.
                let middle = |c: f32| {
                    let spots = stage.buildings.iter().filter(|s| wanted.contains(&s.index));
                    let (lo, hi) = spots.fold((f32::MAX, f32::MIN), |(lo, hi), s| {
                        (lo.min(s.x - s.w / 2.0), hi.max(s.x + s.w / 2.0))
                    });
                    ((lo + hi) / 2.0 - c).abs()
                };
                middle(*a).total_cmp(&middle(*b))
            });
        if let Some(centre) = centre {
            return Some((zoom, centre));
        }
    }
    Some((1.0, overview))
}
