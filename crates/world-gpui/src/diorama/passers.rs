//! Passers-by (1.0 P5, the v0.30 review: "no passers-by: a pan away from
//! the gathering crosses empty ground"): now and then someone who is not
//! one of the place's residents walks along its spine, from one side of a
//! stretch of it to the other, and is gone.
//!
//! Pure presentation, like the rest of the diorama: who passes, when and
//! where follows the seconds the window has been looked at, the place's
//! scenery and the World's day as a seed, never anything the World
//! records. A passer is no resident: nobody can click them, name them,
//! count them on a first screen or find them in the frame's `people`. They
//! wear the look of the place's own people (a penguin on the ice, a
//! colonist on a rust plain) in colours of their own. Nobody passes at
//! night, nor with Reduce Motion on (they would stand frozen mid-step).

use super::*;

/// How often a stretch of the place may see someone pass, in seconds.
pub const PERIOD: f32 = 34.0;

/// How long after the window opens the first may pass.
const LEAD: f32 = 6.0;

/// The share of periods in which someone passes.
const CHANCE: f32 = 0.7;

/// How fast a passer walks, in figure heights a second: a stroll.
const PACE: f32 = 1.1;

/// From and to which hour people pass.
const HOURS: (f32, f32) = (7.0, 21.0);

/// Someone passing: where along the stage they are (stage pixels), which
/// way they walk, how far into their walk they are (seconds), and the seed
/// their colours are drawn from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Passage {
    pub x: f32,
    pub facing: f32,
    pub since: f32,
    pub seed: u32,
}

/// A number from 0 to 1 for `seed`, `lane` and `turn`.
fn chance(seed: u32, lane: u32, turn: u32) -> f32 {
    (painter::hash2(lane as i32, turn as i32, seed) % 10_000) as f32 / 10_000.0
}

/// Everyone passing `seconds` into looking at a stage `width` long seen a
/// window `view_w` wide at a time, people `figure_h` tall, for the place
/// and day `seed`. Each window's width of the place is a lane of its own,
/// with someone crossing it (and a little past each end) in most periods,
/// each at a moment of the period of their own.
pub fn passages(width: f32, view_w: f32, figure_h: f32, seed: u32, seconds: f32) -> Vec<Passage> {
    if seconds < LEAD || view_w <= 0.0 || figure_h <= 0.0 {
        return Vec::new();
    }
    let lanes = (width / view_w).ceil().max(1.0) as u32;
    let speed = figure_h * PACE;
    let mut out = Vec::new();
    for lane in 0..lanes {
        let (from, to) = (
            lane as f32 * view_w,
            ((lane + 1) as f32 * view_w).min(width),
        );
        let reach = view_w * 0.25;
        let length = (to - from + reach * 2.0) / speed;
        // This period's passage and the last's, which may still be walking.
        let now = seconds - LEAD;
        let turn = (now / PERIOD) as u32;
        for turn in turn.saturating_sub(2)..=turn {
            if chance(seed, lane, turn) >= CHANCE {
                continue;
            }
            let start = turn as f32 * PERIOD + chance(seed ^ 0x5bd1, lane, turn) * PERIOD * 0.5;
            let since = now - start;
            if !(0.0..length).contains(&since) {
                continue;
            }
            let rightward = chance(seed ^ 0x9e37, lane, turn) < 0.5;
            let along = since * speed;
            let (x, facing) = if rightward {
                (from - reach + along, 1.0)
            } else {
                (to + reach - along, -1.0)
            };
            out.push(Passage {
                x,
                facing,
                since,
                seed: painter::hash2(lane as i32, turn as i32, seed ^ 0x2f6b),
            });
        }
    }
    out
}

/// Whether anyone passes at `hour`.
pub fn passing_hour(hour: f32) -> bool {
    (HOURS.0..HOURS.1).contains(&hour.rem_euclid(24.0))
}

/// The passers-by of a frame, as they are drawn: in the place's people's
/// look (the drawing and kind of one of its residents, picked by their
/// seed) in colours of their own, walking along the back of the spine a
/// little behind where residents stand, in screen pixels as `people` are.
pub(super) fn passers(
    snapshot: &ProjectionSnapshot,
    stage: &Stage,
    camera: Camera,
    seconds: f32,
) -> Vec<PersonPaint> {
    let residents = snapshot
        .canvas
        .items
        .iter()
        .filter(|item| item.kind == world_projection::CanvasItemKind::Actor)
        .filter(|item| {
            let figure = Figure::of(&item.id.stable_key(), item.look);
            figure.age.height() >= 0.9
        })
        .collect::<Vec<_>>();
    if residents.is_empty() {
        return Vec::new();
    }
    let seed = seed_of_scenery(&snapshot.scenery.unwrap_or(PLAIN)) ^ stage.day as u32;
    let z = camera.zoom;
    let y = stage.feet - stage.figure_h * 0.22;
    passages(stage.width, stage.view_w, stage.figure_h, seed, seconds)
        .into_iter()
        .enumerate()
        .map(|(n, passage)| {
            let model = residents[passage.seed as usize % residents.len()];
            let figure = Figure::of(&format!("passer-{}", passage.seed), model.look);
            let walk = walking(
                passage.x - passage.facing,
                passage.x,
                (passage.since, f32::MAX),
                1.0,
                false,
                stage.figure_h,
            );
            let (sx, sy) = camera.at(stage, passage.x, y);
            let height = stage.figure_h * 0.97 * z * figure.age.height();
            PersonPaint {
                index: usize::MAX - n,
                x: sx,
                y: sy,
                height,
                pose: Pose {
                    bob: walk.pose.bob * z,
                    facing: passage.facing,
                    ..walk.pose
                },
                glow: None,
                drawing: snapshot.drawing_of(model).cloned(),
                stance: Stance::Walking,
                mood: world_projection::Mood::default(),
                carrying: None,
                along: passage.x,
                seat: None,
                beat: None,
                behind: None,
                posture: None,
                turn: Turn::Front,
                detail: stage.figure_h * z >= turn::READS_FROM,
                figure,
            }
        })
        .collect()
}

/// The passers-by of `part` drawn live in the World window, in depth
/// with everyone else (the strip, covers and pictures have none): none at
/// night, with everything held still, or in a postcard's fold.
pub(super) fn deeps(
    part: &std::sync::Arc<Frame>,
    light: [f32; 3],
) -> Vec<super::living_stage::Deep> {
    use super::living_stage::{Deep, Item, Rank};
    if part.still || part.camera.fold > 0.0 || !passing_hour(part.hour) {
        return Vec::new();
    }
    part.passers
        .iter()
        .enumerate()
        .map(|(n, person)| {
            let h = person.height;
            let rect = (person.x - h, person.y - h * 1.5, h * 2.0, h * 1.6);
            let (part, person) = (part.clone(), person.clone());
            Deep {
                at: person.y,
                rank: Rank::Person,
                who: usize::MAX / 2 + n,
                items: vec![Item {
                    opacity: 1.0,
                    draw: super::stage_view::live(rect, None, move |window, (ox, oy, _, _)| {
                        paint_person_live(window, &part, &person, (ox, oy), light)
                    }),
                }],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same place, day and moment always have the same passers.
    #[test]
    fn passers_by_are_the_same_for_the_same_moment() {
        for seconds in [0.0, 10.0, 47.5, 300.0, 1234.5] {
            assert_eq!(
                passages(3300.0, 1100.0, 57.0, 42, seconds),
                passages(3300.0, 1100.0, 57.0, 42, seconds),
            );
        }
        // Another place or day passes at other moments.
        let busy = |seed: u32| {
            (0..600)
                .map(|s| passages(3300.0, 1100.0, 57.0, seed, s as f32).len())
                .collect::<Vec<_>>()
        };
        assert_ne!(busy(42), busy(43));
    }

    /// Nobody passes in the first moments, and then every stretch of the
    /// place sees someone pass within a few periods, walking steadily one
    /// way, never jumping, a passer at a time a lane or so.
    #[test]
    fn passers_by_walk_steadily_across_every_stretch() {
        let (width, view, figure) = (3300.0, 1100.0, 57.0);
        assert!(passages(width, view, figure, 7, 1.0).is_empty());
        let mut seen = [false; 3];
        let mut last: Option<Vec<Passage>> = None;
        let step = 0.25;
        let mut t = LEAD;
        while t < LEAD + PERIOD * 6.0 {
            let now = passages(width, view, figure, 7, t);
            assert!(now.len() <= 6, "{t}: {} at once", now.len());
            for passage in &now {
                let lane = ((passage.x / view).floor().clamp(0.0, 2.0)) as usize;
                seen[lane] = true;
                if let Some(before) = last
                    .as_ref()
                    .and_then(|last| last.iter().find(|b| b.seed == passage.seed))
                {
                    let moved = (passage.x - before.x) * passage.facing;
                    let expected = figure * PACE * step;
                    assert!(
                        (moved - expected).abs() < 1e-2,
                        "{t}: moved {moved} rather than {expected}"
                    );
                }
            }
            last = Some(now);
            t += step;
        }
        assert!(seen.iter().all(|seen| *seen), "{seen:?}");
    }

    /// Passers-by are presentation: making a frame with them changes
    /// nothing of the World, they are never among its `people` (nobody
    /// counts, names or clicks them), and they wear one of the place's own
    /// residents' drawings.
    #[test]
    fn passers_by_are_never_residents_or_world_state() {
        let wire: world_pack_protocol::ProjectionSnapshotWire =
            serde_json::from_str(include_str!("../../tests/fixtures/start-dust.json"))
                .expect("a wire snapshot");
        let snapshot = ProjectionSnapshot::try_from(wire).expect("a snapshot");
        let before = snapshot.clone();
        let stage = stage_at(&snapshot, 1100.0, 848.0, Clock::at(11));
        let living = living(
            &stage,
            &snapshot,
            0.0,
            Daylight::Day,
            &Default::default(),
            None,
        );
        let mut found = false;
        for n in 0..400 {
            let seconds = LEAD + n as f32 * 1.5;
            let frame = frame(
                &snapshot,
                &stage,
                &living,
                Camera::whole(&stage),
                seconds,
                Daylight::Day,
                &Glows::new(),
                1.0,
            );
            assert!(frame
                .people
                .iter()
                .all(|person| person.index < snapshot.canvas.items.len()));
            for passer in &frame.passers {
                found = true;
                assert!(passer.index >= snapshot.canvas.items.len());
                assert!(snapshot
                    .canvas
                    .items
                    .iter()
                    .any(|item| snapshot.drawing_of(item) == passer.drawing.as_ref()));
                assert_eq!(passer.stance, Stance::Walking);
            }
        }
        assert!(found, "someone passed in ten minutes");
        assert_eq!(snapshot, before, "the World is as it was");
    }

    /// Nobody passes at night.
    #[test]
    fn nobody_passes_at_night() {
        assert!(passing_hour(12.0) && passing_hour(19.5));
        assert!(!passing_hour(22.0) && !passing_hour(3.0) && !passing_hour(6.5));
    }
}
