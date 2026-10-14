//! The people: where they walk and idle between turns, how they wave, and
//! how they are drawn live every frame with whatever else moves.

use super::*;

/// How long someone takes to walk to where a turn put them.
pub const WALK_SECONDS: f32 = 1.4;

pub(crate) fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A spring let go from 1 toward 0, `t` seconds on: how much of the
/// displacement is left, overshooting a little before it settles. Coats,
/// hair and a landing's squash all settle on it.
pub(crate) fn settle(t: f32) -> f32 {
    const SWING: gpui::SpringConfig = gpui::SpringConfig::new(170.0, 9.0, 1.0);
    if t <= 0.0 {
        return 1.0;
    }
    SWING
        .step(
            gpui::SpringState {
                position: 1.0,
                velocity: 0.0,
            },
            0.0,
            t.min(4.0),
        )
        .position
}

/// Where someone is this frame, and how they stand: presentation only.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Living {
    pub x: f32,
    pub pose: Pose,
    /// What they do with an idle moment, or how they answer a click, over
    /// whatever their Pack says they are doing.
    pub stance: Option<Stance>,
    /// What they do where an errand has taken them, over what their stop
    /// says they do.
    pub using: Option<Stance>,
    /// Carrying a load as they walk.
    pub loaded: bool,
}

/// How long a building or thing springs after a click, in seconds.
pub const BOUNCE_SECONDS: f32 = 0.5;

/// How long a wave lasts after someone is clicked on, in seconds.
pub const WAVE_SECONDS: f32 = 1.2;

/// What someone does with an idle moment `seconds` in: most of the time
/// nothing much, and now and then, on a cycle of their own, one of the
/// idle things people do. At night they only sit or look about.
pub(super) fn idle(seed: u32, seconds: f32, daylight: Daylight) -> Option<Stance> {
    let period = 16.0 + (seed % 9) as f32;
    let shifted = seconds + (seed % 997) as f32 * 0.53;
    let phase = (shifted % period) / period;
    if !(0.55..0.82).contains(&phase) {
        return None;
    }
    let turn = (shifted / period) as u32 + seed;
    let choices: &[Stance] = if daylight == Daylight::Night {
        &[Stance::Sitting, Stance::LookingAround]
    } else {
        &Stance::IDLE
    };
    Some(choices[(turn as usize) % choices.len()])
}

/// Someone in a group, `seconds` in: taking a turn to talk, on a cycle
/// of their own, and otherwise listening.
pub(super) fn talk(seed: u32, seconds: f32) -> Option<Stance> {
    let period = 7.0 + (seed % 5) as f32;
    let phase = ((seconds + (seed % 613) as f32 * 0.41) % period) / period;
    (phase < 0.4).then_some(Stance::Talking)
}

/// Everyone clicked on in the last [`WAVE_SECONDS`] waves and hops, from
/// how long ago each was clicked: a crouch, a stretched hop, a squashed
/// landing that springs back. `still` (Reduce Motion) keeps the wave and
/// leaves out the hop.
pub fn wave(
    living: &mut [Living],
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    poked: &BTreeMap<SelectionId, f32>,
    still: bool,
) {
    for (life, spot) in living.iter_mut().zip(&stage.people) {
        let Some(item) = snapshot.canvas.items.get(spot.index) else {
            continue;
        };
        let Some(ago) = poked.get(&item.id) else {
            continue;
        };
        if !(0.0..WAVE_SECONDS).contains(ago) || life.pose.stride.is_some() {
            continue;
        }
        life.stance = Some(Stance::Waving);
        if still {
            continue;
        }
        let (crouch, air) = (0.09, 0.42);
        if *ago < crouch {
            life.pose.squash = 1.0 - 0.1 * ease(ago / crouch);
        } else if *ago < crouch + air {
            let u = (ago - crouch) / air;
            let hop = (u * std::f32::consts::PI).sin();
            life.pose.bob += hop * stage.figure_h * 0.18;
            // Stretched going up and coming down, round at the top.
            life.pose.squash = 1.0 + 0.08 * (u * std::f32::consts::PI).cos().abs();
        } else {
            life.pose.squash = 1.0 - 0.12 * settle(ago - crouch - air);
        }
    }
}

/// How close two people stand, in figure heights, to be together: a
/// pair or a group talking.
pub(super) const TOGETHER: f32 = 0.9;

/// How far apart, in figure heights, the nearest members of two groups
/// stand, centre to centre: a body's width and 0.8 P of open quay more.
pub(super) const GROUP_GAP: f32 = 1.6;

/// The most people a group has: more and they are a queue.
#[cfg(test)]
pub(super) const GROUP_MOST: usize = 3;

/// Gathers the people standing along the quay into groups of two and
/// three facing each other, rather than rows (the art bible's §6): each
/// run of people standing shoulder to shoulder is split into twos and
/// threes, a pair close and side by side, a three as a little ring with
/// the middle one a step further back, and a stride between groups. Each
/// run stays centred where it stood, in its own lane, nudged clear of what
/// stands on the quay (`blocked`, for the lane among it) and of the doors
/// of what stands in front of it (`fronted`, for both lanes). Nobody is
/// moved who stands alone.
pub(super) fn gather(
    people: &mut [Spot],
    figure_h: f32,
    quay: f32,
    blocked: &[(f32, f32)],
    fronted: &[(f32, f32)],
) {
    let lane_of = |spot: &Spot| (spot.y - quay > figure_h * 0.3) as u8;
    let mut order = (0..people.len()).collect::<Vec<_>>();
    order.sort_by(|a, b| {
        lane_of(&people[*a])
            .cmp(&lane_of(&people[*b]))
            .then(people[*a].x.total_cmp(&people[*b].x))
            .then(people[*a].index.cmp(&people[*b].index))
    });
    // Runs: neighbours in a lane closer than a stride.
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for position in order {
        let spot = people[position];
        match runs.last_mut() {
            Some(run)
                if {
                    let last = &people[*run.last().expect("a run is never empty")];
                    lane_of(last) == lane_of(&spot) && spot.x - last.x < figure_h * GROUP_GAP
                } =>
            {
                run.push(position)
            }
            _ => runs.push(vec![position]),
        }
    }
    // Shoulder to shoulder in a group, a clear gap between groups (at
    // least 0.8 P of open quay between the nearest shoulders), and each
    // group a step nearer or further than the last, so two groups never
    // read as one line.
    let (near, between) = (figure_h * 0.64, figure_h * GROUP_GAP);
    let room = figure_h * 0.5;
    let width_of = |size: usize| near * (size - 1) as f32;
    // How wide a run stands once it is spread into its groups.
    let spread = |count: usize| {
        let sizes = group_sizes(count);
        sizes.iter().map(|size| width_of(*size)).sum::<f32>()
            + between * sizes.len().saturating_sub(1) as f32
    };
    // Two runs that would stand closer than a stride once each is spread
    // out (a crowd along the front, a year on) are one run: otherwise each
    // spreads into the next and the groups at their seam become one line.
    loop {
        let extent = |run: &Vec<usize>| {
            let centre = run.iter().map(|at| people[*at].x).sum::<f32>() / run.len() as f32;
            let half = spread(run.len()) / 2.0;
            (centre - half, centre + half)
        };
        let merge = runs.windows(2).position(|pair| {
            let (a, b) = (&pair[0], &pair[1]);
            lane_of(&people[a[0]]) == lane_of(&people[b[0]]) && extent(b).0 - extent(a).1 < between
        });
        let Some(at) = merge else {
            break;
        };
        let next = runs.remove(at + 1);
        runs[at].extend(next);
    }
    for run in runs.into_iter().filter(|run| run.len() > 1) {
        let sizes = group_sizes(run.len());
        let total = spread(run.len());
        let centre = run.iter().map(|at| people[*at].x).sum::<f32>() / run.len() as f32;
        let line = run.iter().map(|at| people[*at].y).sum::<f32>() / run.len() as f32;
        // Where each member would stand, from the run's left edge.
        // Never evenly: each stands a little nearer or further from the
        // last, by who they are (seeded, so the same every frame), a three
        // is a ring with its middle well back, a pair one half a step
        // behind the other (the v0.29 art director: rows and blobs).
        let mut places = Vec::with_capacity(run.len());
        let mut left = 0.0;
        let mut member = 0;
        for (group, size) in sizes.iter().enumerate() {
            let step = [0.0, -0.24, 0.12][group % 3] * figure_h;
            let mut x = 0.0;
            for nth in 0..*size {
                let seed = people[run[member]].index as u32;
                member += 1;
                let jitter = (painter::hash2(seed as i32, 53, 0x9e37) % 1000) as f32 / 1000.0;
                if nth > 0 {
                    x += near;
                }
                let back = step
                    + match (*size, nth) {
                        (3, 1) => figure_h * 0.26,
                        (2, 1) => figure_h * (0.06 + 0.08 * jitter),
                        _ => figure_h * 0.04 * jitter,
                    };
                places.push((left + x, back));
            }
            left += x.max(width_of(*size)) + between;
        }
        // Centred where the run stood, or nudged a little either way to
        // stand clear of what stands there; with no room, group by group.
        let front_lane = line - quay > figure_h * 0.3;
        let open_at = |x: f32| {
            let off = |(l, r): &(f32, f32)| x + room / 2.0 <= *l || x - room / 2.0 >= *r;
            (front_lane || blocked.iter().all(off)) && fronted.iter().all(off)
        };
        let clear = |shift: f32| places.iter().all(|(x, _)| open_at(shift + x));
        let home = centre - total / 2.0;
        let nudge = figure_h * 0.2;
        let Some(start) = (0..=40)
            .flat_map(|step| [step, -step])
            .map(|step| home + step as f32 * nudge)
            .find(|shift| clear(*shift))
        else {
            // No room for the run in one piece (a building's base, a
            // stall): each group on its own, as near where its members
            // stood as it can, clear of what stands there and a full
            // gap from every group placed before it, so groups never close
            // up into one line. A group with no room near at all stands
            // where it was, a clear step further back than its neighbour.
            let mut at = 0;
            let mut stood: Vec<(f32, f32)> = Vec::new();
            for (group, size) in sizes.iter().enumerate() {
                let members = run[at..at + size].to_vec();
                let own = places[at..at + size].to_vec();
                at += size;
                let first = own[0].0;
                let width = width_of(*size);
                let centre = members.iter().map(|m| people[*m].x).sum::<f32>() / *size as f32;
                let fits = |start: f32| {
                    own.iter().all(|(x, _)| open_at(start + x - first))
                        && stood
                            .iter()
                            .all(|(l, r)| start + width + between <= *l || start >= *r + between)
                };
                let home = centre - width / 2.0;
                match (0..=60)
                    .flat_map(|step| [step, -step])
                    .map(|step| home + step as f32 * nudge)
                    .find(|start| fits(*start))
                {
                    Some(start) => {
                        for (member, (x, back)) in members.iter().zip(&own) {
                            let spot = &mut people[*member];
                            let seed = spot.index as f32 * 0.37;
                            spot.x = start + x - first;
                            spot.y = line - back + seed.sin() * figure_h * 0.02;
                            spot.scale = 1.0 + (spot.y - quay) / figure_h * 0.35;
                        }
                        stood.push((start, start + width));
                    }
                    None => {
                        let (mut l, mut r) = (f32::MAX, f32::MIN);
                        for member in &members {
                            let spot = &mut people[*member];
                            if group % 2 == 1 {
                                spot.y = line - figure_h * 0.4;
                                spot.scale = 1.0 + (spot.y - quay) / figure_h * 0.35;
                            }
                            l = l.min(spot.x);
                            r = r.max(spot.x);
                        }
                        stood.push((l, r));
                    }
                }
            }
            continue;
        };
        for (at, (x, back)) in run.into_iter().zip(places) {
            let spot = &mut people[at];
            let seed = spot.index as f32 * 0.37;
            spot.x = start + x;
            spot.y = line - back + seed.sin() * figure_h * 0.02;
            spot.scale = 1.0 + (spot.y - quay) / figure_h * 0.35;
        }
    }
    // Someone a step nearer, in the front lane, never stands in front of
    // a group behind them, closing it up into one line of four: they step
    // clear of it, a full gap either side, if there is room near.
    let mut behind: Vec<(f32, f32)> = Vec::new();
    let mut xs = people
        .iter()
        .filter(|spot| lane_of(spot) == 0)
        .map(|spot| spot.x)
        .collect::<Vec<_>>();
    xs.sort_by(f32::total_cmp);
    for x in xs {
        match behind.last_mut() {
            Some((_, right)) if x - *right < between * 0.9 => *right = x,
            _ => behind.push((x, x)),
        }
    }
    let mut front = (0..people.len())
        .filter(|at| lane_of(&people[*at]) == 1)
        .collect::<Vec<_>>();
    front.sort_by(|a, b| people[*a].x.total_cmp(&people[*b].x));
    let nudge = figure_h * 0.2;
    for at in front {
        let x = people[at].x;
        let apart = |x: f32| {
            behind
                .iter()
                .all(|(l, r)| x <= l - between || x >= r + between)
        };
        if apart(x) {
            continue;
        }
        let others = people
            .iter()
            .enumerate()
            .filter(|(other, spot)| *other != at && lane_of(spot) == 1)
            .map(|(_, spot)| spot.x)
            .collect::<Vec<_>>();
        let open = |x: f32| {
            apart(x)
                && fronted
                    .iter()
                    .all(|(l, r)| x + room / 2.0 <= *l || x - room / 2.0 >= *r)
                && others.iter().all(|other| (other - x).abs() >= near)
        };
        if let Some(to) = (1..=40)
            .flat_map(|step| [step, -step])
            .map(|step| x + step as f32 * nudge)
            .find(|x| open(*x))
        {
            people[at].x = to;
        }
    }
}

/// How wide a standing person's silhouette is, in heights: for telling
/// how much two overlap.
pub(super) const BODY_W: f32 = 0.42;

/// How far apart two people may stand up or down the spine, in heights,
/// and still read as one line: their feet on one line, within a hand.
pub(super) const IN_LINE: f32 = 0.06;

/// How far apart along the spine two people in one line may stand, in
/// heights, and still read as one row.
pub(super) const ROW_GAP: f32 = 2.2;

/// Of people standing (feet at `x`, `y`, `height` tall, in screen
/// pixels): the most standing in one row (feet on one line, each within a
/// couple of strides of the next), and the most any two overlap, as a
/// share of the smaller silhouette. The art bible's §6 holds the first to
/// four and v0.29 the second to a fifth.
pub fn rows_and_overlap(people: &[(f32, f32, f32)]) -> (usize, f32) {
    let mut order = people.to_vec();
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut longest = usize::from(!order.is_empty());
    for (start, first) in order.iter().enumerate() {
        let mut count = 1;
        let mut last = *first;
        for next in &order[start + 1..] {
            let p = last.2.max(next.2).max(1.0);
            if next.0 - last.0 > ROW_GAP * p {
                break;
            }
            if (next.1 - first.1).abs() <= IN_LINE * p {
                count += 1;
                last = *next;
            }
        }
        longest = longest.max(count);
    }
    let body = |(x, y, h): (f32, f32, f32)| (x - h * BODY_W / 2.0, y - h, h * BODY_W, h);
    let mut most = 0.0_f32;
    for (at, a) in order.iter().enumerate() {
        for b in &order[at + 1..] {
            let (ra, rb) = (body(*a), body(*b));
            let w = (ra.0 + ra.2).min(rb.0 + rb.2) - ra.0.max(rb.0);
            let h = (ra.1 + ra.3).min(rb.1 + rb.3) - ra.1.max(rb.1);
            if w > 0.0 && h > 0.0 {
                let smaller = (ra.2 * ra.3).min(rb.2 * rb.3).max(1.0);
                most = most.max(w * h / smaller);
            }
        }
    }
    (longest, most)
}

/// How a run of `n` people splits into groups of two and three.
pub(super) fn group_sizes(n: usize) -> Vec<usize> {
    match n {
        0 => Vec::new(),
        1 => vec![1],
        2 => vec![2],
        3 => vec![3],
        4 => vec![2, 2],
        n if n % 3 == 1 => {
            let mut sizes = vec![3; (n - 4) / 3];
            sizes.extend([2, 2]);
            sizes
        }
        n if n % 3 == 2 => {
            let mut sizes = vec![3; (n - 2) / 3];
            sizes.push(2);
            sizes
        }
        n => vec![3; n / 3],
    }
}

/// The groups among the people standing on the stage: for each person
/// (by position in `people`), where the middle of their group is, if they
/// stand with anyone.
pub(crate) fn group_centres(people: &[Spot], figure_h: f32) -> Vec<Option<f32>> {
    let mut order = (0..people.len()).collect::<Vec<_>>();
    order.sort_by(|a, b| people[*a].x.total_cmp(&people[*b].x));
    let mut centres = vec![None; people.len()];
    let mut start = 0;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len()
            && people[order[end]].x - people[order[end - 1]].x < figure_h * TOGETHER
            && (people[order[end]].y - people[order[end - 1]].y).abs() < figure_h * 0.3
        {
            end += 1;
        }
        if end - start > 1 {
            let group = &order[start..end];
            let centre = group.iter().map(|at| people[*at].x).sum::<f32>() / group.len() as f32;
            for at in group {
                centres[*at] = Some(centre);
            }
        }
        start = end;
    }
    centres
}

/// Someone walking from `from` to `to` along their line, `since` seconds
/// into a walk `length` long: planted steps, a lean into the walk, a coat
/// that swings as they start and stop. Shorter, slower steps for someone
/// who takes their time (`pace`); a load held as they go.
pub(super) fn walking(
    from: f32,
    to: f32,
    (since, length): (f32, f32),
    pace: f32,
    loaded: bool,
    figure_h: f32,
) -> Living {
    let t = since / length.max(0.01);
    let facing = (to - from).signum();
    let phase = (since * 1.7 * pace.sqrt()).rem_euclid(1.0);
    let start = 1.0 - settle(since);
    let lean = facing * (0.05 * start + 0.018 * (phase * std::f32::consts::TAU * 2.0).sin());
    Living {
        x: from + (to - from) * ease(t),
        pose: Pose {
            stride: Some(phase),
            bob: (phase * std::f32::consts::TAU).sin().abs() * figure_h * 0.035,
            facing,
            squash: 1.0 - 0.03 * (phase * std::f32::consts::TAU * 2.0).cos().max(0.0),
            lean,
        },
        stance: None,
        using: None,
        loaded,
    }
}

/// Where each person is this frame, `seconds` into looking at the World.
///
/// Between turns everyone goes about their day. Where they stand, what
/// they use there and the errands they run from there (out to the nearest
/// thing to use, a while there, and back) follow the stage's clock and the
/// World's day, never `seconds`: the same World at the same moment is the
/// same town ([`errand_at`]). `seconds` only breathes and turns heads, and
/// times an idle moment. At night nobody runs an errand. Anyone `pinned`
/// (speaking, being asked, in the news) stays where they are. `walking` is
/// how far through the walk a turn started is, from where `before` had
/// them. Someone whose day has just taken them somewhere else strolls
/// there, with a load if they carry one there.
pub fn living(
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    seconds: f32,
    daylight: Daylight,
    pinned: &BTreeSet<SelectionId>,
    before: Option<(&Stage, &ProjectionSnapshot, f32)>,
) -> Vec<Living> {
    living_staged(stage, snapshot, seconds, daylight, pinned, before, None)
}

/// [`living`], with a scene playing: whoever the scene stages
/// (`staged`, see [`super::staging`]) stands, walks and faces as it
/// has them, and everyone else keeps their day. The stage is the one the
/// scene was cast onto ([`super::staging::cast_on`]), so each of them
/// stands on its line where the scene puts them.
pub fn living_staged(
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    seconds: f32,
    daylight: Daylight,
    pinned: &BTreeSet<SelectionId>,
    before: Option<(&Stage, &ProjectionSnapshot, f32)>,
    staged: Option<&super::staging::Staged>,
) -> Vec<Living> {
    let items = &snapshot.canvas.items;
    let figure_h = stage.figure_h;
    // The ground every building stands on: nobody's walk ever goes into
    // one (the art director's S1 blocker, a walker on a tower's
    // door), so a walk toward or past one stops beside it.
    let footprints = footprints(stage, snapshot);
    let clear = |from: f32, to: f32, y: f32| kept_out(&footprints, from, to, y, figure_h * 0.2);
    // Tall, thin things standing in the water in front of the shore (a tide
    // board, a lamp on the point): nobody stands right behind one, sliced
    // by it, but a step to its side.
    let posts = stage
        .things
        .iter()
        .filter(|spot| spot.y >= stage.front - 1.0)
        .filter_map(|spot| {
            let shape = items[spot.index].shape.unwrap_or(MarkShape::Parcel);
            let tall = matches!(
                shape,
                MarkShape::Signpost | MarkShape::Lamp | MarkShape::Lantern | MarkShape::Flag
            );
            tall.then_some((spot.x, spot.w * 0.3 + figure_h * 0.25))
        })
        .collect::<Vec<_>>();
    let beside_posts = |x: f32| {
        posts.iter().fold(x, |x, (post, reach)| {
            if (x - post).abs() < *reach {
                post + reach * if x < *post { -1.0 } else { 1.0 }
            } else {
                x
            }
        })
    };
    // Walking from `from` to `to`, `since` seconds of `length` in: planted
    // steps, a lean into the walk, a coat that swings as they start and
    // stop.
    let walk = |from: f32, to: f32, since: f32, length: f32, pace: f32, loaded: bool| {
        walking(from, to, (since, length), pace, loaded, figure_h)
    };
    // Just arrived: the coat swings on past the stop and settles back.
    let arrived = |life: Living, since: f32, facing: f32| Living {
        pose: Pose {
            lean: facing * 0.05 * settle(since),
            ..life.pose
        },
        ..life
    };
    let centres = group_centres(&stage.people, figure_h);
    let x_of = |index: usize| {
        stage
            .people
            .iter()
            .chain(&stage.things)
            .find(|spot| spot.index == index)
            .map(|spot| spot.x)
    };
    stage
        .people
        .iter()
        .zip(centres)
        .map(|(spot, centre)| {
            let item = &items[spot.index];
            let key = item.id.stable_key();
            let seed = art::seed_of(&key);
            let what = stage.doing.get(&spot.index).copied().unwrap_or_default();
            let at_fixture = what.fixture.is_some();
            let home = if at_fixture {
                spot.x
            } else {
                beside_posts(spot.x)
            };
            let breathe = (seconds * 1.7 + (seed % 100) as f32 * 0.07).sin();
            let pace = item.look.map_or(1.0, |look| Age::of(&look).pace());
            // In a scene: where it has them, as it has them.
            if let Some(cast) = staged.and_then(|staged| staged.person(spot.index)) {
                return cast.living(breathe, pace, figure_h);
            }
            // Walking over to where a turn put them.
            if let Some((old_stage, old_snapshot, progress)) = before {
                if let Some(old) = old_stage.person(item.id, old_snapshot) {
                    let old = Spot {
                        x: clear(home, old.x, spot.y),
                        ..old
                    };
                    if (old.x - home).abs() > 2.0 {
                        let since = progress * WALK_SECONDS;
                        if progress < 1.0 {
                            return walk(old.x, home, since, WALK_SECONDS, pace, false);
                        }
                        if since < WALK_SECONDS + SETTLE {
                            let still = Living {
                                x: home,
                                ..Living::default()
                            };
                            return arrived(still, since - WALK_SECONDS, (home - old.x).signum());
                        }
                    }
                }
            }
            // On the way to where their day has them this hour.
            if let Some((from, since)) = stage.routes.get(&spot.index) {
                let from = &clear(home, *from, spot.y);
                let length = (home - from).abs() / (figure_h * STROLL * pace);
                if *since < length {
                    return walk(*from, home, *since, length, pace, what.loaded);
                }
            }
            // Someone standing with others faces into their group, and
            // now and then glances away; alone, they look about. Whoever
            // is with someone faces them; whoever uses something faces it.
            let looking = ((seconds * 0.11 + (seed % 7) as f32).sin() * 1.4).clamp(-1.0, 1.0);
            let toward = |x: Option<f32>| {
                x.filter(|x| (x - home).abs() > 1.0)
                    .map(|x| (x - home).signum())
            };
            let partner = toward(what.with.and_then(x_of));
            let fixture = what.fixture.map(|(thing, how)| (toward(x_of(thing)), how));
            let facing = match (partner, fixture, centre) {
                (Some(partner), _, _) => partner,
                // Behind a stall, facing out over it, a turn either way.
                (None, Some((_, Use::Keep)), _) => looking.signum() * 0.3,
                (None, Some((Some(at), _)), _) => at,
                (None, Some((None, _)), _) => {
                    if seed.is_multiple_of(2) {
                        1.0
                    } else {
                        -1.0
                    }
                }
                (None, None, Some(centre)) if (centre - home).abs() > 1.0 => {
                    let glance = ((seconds * 0.07 + (seed % 11) as f32).sin() > 0.93) as u8;
                    (centre - home).signum() * if glance == 1 { -0.4 } else { 1.0 }
                }
                // The middle of a ring faces out, toward the viewer.
                (None, None, Some(_)) => looking * 0.3,
                (None, None, None) => looking,
            };
            // Seated, or standing to what they use: still on their spot.
            let sway = if at_fixture {
                0.0
            } else if centre.is_some() || partner.is_some() {
                1.0
            } else {
                3.0
            };
            let still = Living {
                x: home + (seconds * 0.23 + seed as f32).sin() * sway,
                pose: Pose {
                    stride: None,
                    bob: if what.fixture.is_some_and(|(_, how)| how == Use::Seat) {
                        0.0
                    } else {
                        breathe * 0.6
                    },
                    facing,
                    // Breathing: a touch taller on the breath in.
                    squash: 1.0 + 0.008 * breathe,
                    lean: 0.0,
                },
                stance: if pinned.contains(&item.id) {
                    None
                } else if centre.is_some() {
                    talk(seed, seconds).or_else(|| idle(seed, seconds, daylight))
                } else {
                    idle(seed, seconds, daylight)
                },
                using: None,
                loaded: false,
            };
            // Nobody wanders off from whoever they came with, or from what
            // they use, or at night; one of a group now and then goes on an
            // errand and comes back to it.
            if pinned.contains(&item.id)
                || at_fixture
                || what.with.is_some()
                || daylight == Daylight::Night
            {
                return still;
            }
            // An errand: out to where it takes them, a while there, and home.
            let Some(errand) = what.errand else {
                return still;
            };
            // All the way there (someone slower takes longer); beside a
            // building, never into it.
            let away = clear(home, errand.to, spot.y);
            if (away - home).abs() < 2.0 {
                return still;
            }
            let leg = (away - home).abs() / (figure_h * STROLL * pace);
            let clock = stage.clock;
            match errand_at(seed, stage.day, clock.hour, clock.into_hour, leg) {
                Some((Leg::Out, since)) => walk(home, away, since, leg, pace, false),
                Some((Leg::There, since)) => arrived(
                    Living {
                        x: away,
                        pose: Pose {
                            facing: (errand.at - away).signum(),
                            ..still.pose
                        },
                        stance: None,
                        using: Some(errand.stance),
                        loaded: false,
                    },
                    since,
                    (away - home).signum(),
                ),
                Some((Leg::Back, since)) => walk(away, home, since, leg, pace, errand.fetch),
                Some((Leg::Home, since)) => arrived(still, since, (home - away).signum()),
                None => still,
            }
        })
        .collect()
}

/// Every building's footprint on the stage (left, back, right, front):
/// the ground nobody's feet are ever on.
pub(super) fn footprints(
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
) -> Vec<(f32, f32, f32, f32)> {
    let setting = art::Setting::from_key(snapshot.canvas.setting.as_deref());
    stage
        .buildings
        .iter()
        .map(|spot| {
            let (w, h) = super::building_size(stage, snapshot, spot, setting);
            super::living_stage::footprint(spot.x, spot.y, w, h)
        })
        .collect()
}

/// The ground of a stage someone can stand on: what every building and
/// thing on it covers.
pub(super) struct Ground {
    width: f32,
    footprints: Vec<(f32, f32, f32, f32)>,
    /// Each building's foot and size: (x, base, width, height).
    boxes: Vec<(f32, f32, f32, f32)>,
    /// Each thing, props too: (item, x, base, width).
    things: Vec<(usize, f32, f32, f32)>,
}

impl Ground {
    pub(super) fn of(stage: &Stage, snapshot: &ProjectionSnapshot) -> Self {
        let setting = art::Setting::from_key(snapshot.canvas.setting.as_deref());
        Self {
            width: stage.width,
            footprints: footprints(stage, snapshot),
            boxes: stage
                .buildings
                .iter()
                .map(|spot| {
                    let (w, h) = super::building_size(stage, snapshot, spot, setting);
                    (spot.x, spot.y, w, h)
                })
                .collect(),
            // Every thing, props too: a pool's rail, a pier, a cart. Nobody
            // is stood on one.
            things: stage
                .things
                .iter()
                .map(|spot| {
                    let w = super::thing_width(stage, snapshot, spot, setting).max(spot.w);
                    (spot.index, spot.x, spot.y, w)
                })
                .collect(),
        }
    }

    /// Every building's footprint (left, back, right, front).
    pub(super) fn footprints(&self) -> &[(f32, f32, f32, f32)] {
        &self.footprints
    }

    /// Open ground for someone `p` tall with their feet at (`x`, `y`): in
    /// no building's footprint, nor standing at or behind a building's foot
    /// where its picture is (they would read as on its wall), nor in a
    /// thing standing at their depth (but `own`, the thing they use).
    pub(super) fn open(&self, x: f32, y: f32, p: f32, own: usize) -> bool {
        let body = p * 0.25;
        x > body
            && x < self.width - body
            && self
                .footprints
                .iter()
                .all(|footprint| !super::living_stage::inside(*footprint, x, y))
            && self.boxes.iter().all(|(bx, by, w, h)| {
                (x - bx).abs() >= w / 2.0 + body || y >= by + p * 0.3 || y <= by - h - p * 0.2
            })
            && self.things.iter().all(|(index, tx, ty, tw)| {
                *index == own
                    || (x - tx).abs() >= tw / 2.0 + body * 0.6
                    || (y - ty).abs() >= p * 0.45
            })
    }
}

/// How far apart, in figure heights, two people sit side by side on a
/// bench: about 1.3 body widths, shoulders clear.
pub(super) const SEAT_ROOM: f32 = 0.6;

/// How far from a lamp's post the nearest of a ring stands, in figure
/// heights (besides half the lamp's own width).
pub(super) const RING_REACH: f32 = 0.55;

/// How far from a lamp, in figure heights, anyone of its ring stands.
pub(super) const RING_MOST: f32 = 1.5;

/// How much of a bench's width, end sitter to end sitter, its sitters
/// spread over: the whole plank, a body's half width in from each end.
pub(super) const SEAT_SPAN: f32 = 0.62;

/// The highest seat, for someone's height, their feet still on the ground
/// (a lifeguard's chair is not sat on).
pub(super) const SEAT_MOST: f32 = 0.6;

/// How someone stands to `shape` at their stop, by what the stop says they
/// do there: on a bench's seat, in a ring round a lamp, behind a stall
/// (beside it with a load), or at its side. None for things nobody stands
/// to, or a stop that does not say (they stand in front, on the spine).
fn use_of(shape: MarkShape, stance: Option<Stance>) -> Option<Use> {
    // Only what the record says they do there: a stop that says nothing
    // stands them on the spine in front, as ever.
    let stance = stance?;
    match (shape, stance) {
        (MarkShape::Bench, Stance::Sitting) => Some(Use::Seat),
        (MarkShape::Lamp | MarkShape::Lantern, _) => Some(Use::Ring),
        (MarkShape::Stall, Stance::Carrying) => Some(Use::Beside),
        (MarkShape::Stall, _) => Some(Use::Keep),
        (
            MarkShape::Bench
            | MarkShape::Well
            | MarkShape::Fountain
            | MarkShape::Garden
            | MarkShape::Sprouts
            | MarkShape::Planter,
            _,
        ) => Some(Use::Beside),
        _ => None,
    }
}

/// What someone on an errand does where it takes them, at a thing of
/// `shape`: buys at a stall, draws water or tends the beds, sits by a
/// bench. None for things nobody runs an errand to.
fn errand_to(shape: MarkShape) -> Option<Stance> {
    match shape {
        MarkShape::Stall => Some(Stance::Talking),
        MarkShape::Well
        | MarkShape::Fountain
        | MarkShape::Garden
        | MarkShape::Sprouts
        | MarkShape::Planter
        | MarkShape::Postbox => Some(Stance::Working),
        // A sit down a while by a bench, on a seat of their own.
        MarkShape::Bench => Some(Stance::Sitting),
        _ => None,
    }
}

/// Stands those who use something at their stop to it (v0.30 S2d): a
/// bench's sitters along its seat, a lamp's gatherers in a ring round it
/// (never in its post), a stall's keeper behind it and whoever brings it a
/// load beside it; companions side by side; and gives everyone else out of
/// doors an errand from where they stand: out to the nearest thing to use
/// and back, or for a carrier back to where the load comes from. What is
/// in the water is used from the shore, as before.
pub(super) fn at_their_stops(
    stage: &mut Stage,
    snapshot: &ProjectionSnapshot,
    came_from: &BTreeMap<usize, usize>,
    slot_x: &BTreeMap<usize, f32>,
) {
    let items = &snapshot.canvas.items;
    let setting = art::Setting::from_key(snapshot.canvas.setting.as_deref());
    let figure_h = stage.figure_h;
    let index_of = items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.id, index))
        .collect::<BTreeMap<_, _>>();
    let out = stage
        .people
        .iter()
        .map(|spot| spot.index)
        .filter(|index| !stage.routes.contains_key(index))
        .collect::<BTreeSet<_>>();
    // Company only while both are out there.
    let doing = stage.doing.clone();
    for (person, what) in stage.doing.iter_mut() {
        what.with = what.with.filter(|with| {
            out.contains(person)
                && out.contains(with)
                && doing
                    .get(with)
                    .is_some_and(|other| other.with == Some(*person))
        });
    }
    // Who uses what, thing by thing.
    let mut by_thing = BTreeMap::<usize, Vec<usize>>::new();
    for (person, what) in &stage.doing {
        if !out.contains(person) {
            continue;
        }
        let Some(stop) = world_projection::stop_at(&items[*person].day, stage.clock.hour) else {
            continue;
        };
        let Some(thing) = index_of.get(&stop.at).copied() else {
            continue;
        };
        let Some(spot) = stage.things.iter().find(|spot| spot.index == thing) else {
            continue;
        };
        let shown = stage
            .shown_from
            .get(&thing)
            .is_none_or(|least| least.is_finite());
        if !shown || stage.depth_of(spot) == Depth::Water {
            continue;
        }
        let shape = items[thing].shape.unwrap_or(MarkShape::Parcel);
        if use_of(shape, what.stance).is_some() {
            by_thing.entry(thing).or_default().push(*person);
        }
    }
    let footprints = footprints(stage, snapshot);
    let room = figure_h * 0.25;
    let clear_of_buildings = |x: f32, y: f32| {
        footprints.iter().fold(x, |x, (left, back, right, front)| {
            if y > *back && y < *front && x > left - room && x < right + room {
                if x - left < right - x {
                    left - room
                } else {
                    right + room
                }
            } else {
                x
            }
        })
    };
    let ground = Ground::of(stage, snapshot);
    let open = |x: f32, y: f32, p: f32, own: usize| ground.open(x, y, p, own);
    for (thing, mut members) in by_thing {
        let spot = *stage
            .things
            .iter()
            .find(|spot| spot.index == thing)
            .expect("a thing on the stage");
        let shape = items[thing].shape.unwrap_or(MarkShape::Parcel);
        let p = figure_h * spot.scale;
        let w = super::thing_width(stage, snapshot, &spot, setting);
        // Companions next to each other.
        members.sort();
        let mut ordered: Vec<usize> = Vec::new();
        for member in members {
            if ordered.contains(&member) {
                continue;
            }
            ordered.push(member);
            if let Some(with) = stage.doing[&member].with {
                if !ordered.contains(&with) && out.contains(&with) {
                    let theirs = world_projection::stop_at(&items[with].day, stage.clock.hour)
                        .and_then(|stop| index_of.get(&stop.at).copied());
                    if theirs == Some(thing) {
                        ordered.push(with);
                    }
                }
            }
        }
        let seat = super::seat_height(stage, snapshot, &spot, setting);
        let seats = if seat / p <= SEAT_MOST && seat / p >= 0.2 {
            ((w * SEAT_SPAN / (SEAT_ROOM * p)).floor() as usize + 1).clamp(1, 3)
        } else {
            0
        };
        let sitting = ordered
            .iter()
            .filter(|person| use_of(shape, stage.doing[*person].stance) == Some(Use::Seat))
            .count()
            .min(seats);
        let (mut sat, mut kept, mut beside) = (0, false, 0);
        let (mut ringed, mut placed) = (Vec::<(f32, f32)>::new(), None::<(usize, f32)>);
        let mut besides = Vec::<(f32, f32)>::new();
        for person in ordered {
            let wants = use_of(shape, stage.doing[&person].stance).unwrap_or(Use::Beside);
            let how = match wants {
                Use::Seat if sat < sitting => Use::Seat,
                Use::Keep if !kept => Use::Keep,
                Use::Ring => Use::Ring,
                _ => Use::Beside,
            };
            let mut ring_scale = spot.scale;
            let (x, y) = match how {
                Use::Seat => {
                    let along = sat as f32 - (sitting as f32 - 1.0) / 2.0;
                    sat += 1;
                    // Spread along the whole plank, end to end, at least a
                    // body's room apart.
                    let apart = if sitting > 1 {
                        w * SEAT_SPAN / (sitting - 1) as f32
                    } else {
                        0.0
                    };
                    let x = spot.x + along * apart;
                    // Just in front of the bench, so drawn over it.
                    (x, spot.y + 0.5)
                }
                Use::Ring => {
                    // Either side of the post, then round toward the viewer
                    // and further out: a horseshoe open to the front,
                    // balanced left and right, never in the post nor hidden
                    // behind it, nor on a building's ground. Companions on
                    // the same side, next to each other.
                    const SLOTS: [(f32, f32); 6] = [
                        (1.0, 0.0),
                        (0.75, 0.45),
                        (1.6, 0.12),
                        (1.35, 0.6),
                        (2.2, 0.25),
                        (1.95, 0.75),
                    ];
                    let reach = RING_REACH * p + w * 0.2;
                    let with = stage.doing[&person].with;
                    let first = match placed {
                        Some((last, side)) if with == Some(last) => side,
                        _ if ringed.iter().filter(|(x, _)| *x > spot.x).count()
                            <= ringed.iter().filter(|(x, _)| *x < spot.x).count() =>
                        {
                            1.0
                        }
                        _ => -1.0,
                    };
                    // Beside whoever they came with: the slots level with
                    // them first.
                    let mut slots = SLOTS.to_vec();
                    if matches!(placed, Some((last, _)) if with == Some(last)) {
                        slots.sort_by(|a, b| a.1.total_cmp(&b.1));
                    }
                    // Round it, on open ground within a stride and a half
                    // of it. A lamp with none (a beacon on the hill
                    // between the houses) forms no ring: its gatherers
                    // stay where their stop stood them.
                    let free = |x: f32, y: f32| {
                        (x - spot.x).abs() <= RING_MOST * p
                            && (y - spot.y).abs() <= p
                            && ringed.iter().all(|(ox, oy): &(f32, f32)| {
                                let (dx, dy) = ((ox - x).abs(), (oy - y).abs());
                                dx >= p * 0.45 || (dy >= p * 0.4 && dx >= p * 0.2)
                            })
                            && open(x, y, p, thing)
                    };
                    let found = [first, -first].into_iter().find_map(|side| {
                        slots
                            .iter()
                            .map(|(dx, dy)| (spot.x + side * dx * reach, spot.y + dy * p))
                            .find(|(x, y)| free(*x, *y))
                            .map(|at| (side, at, spot.scale))
                    });
                    let Some((side, at, scale)) = found else {
                        continue;
                    };
                    ring_scale = scale;
                    placed = Some((person, side));
                    ringed.push(at);
                    at
                }
                Use::Keep => {
                    kept = true;
                    // Behind the counter: drawn before the stall, in front
                    // of whatever stands behind it.
                    (spot.x, spot.y - 0.16 * p)
                }
                Use::Beside => {
                    // At its side, on open ground: one side, then the
                    // other, then a step further out.
                    let first = if beside % 2 == 0 { 1.0 } else { -1.0 };
                    beside += 1;
                    let at = |side: f32, out: usize| {
                        (
                            spot.x + side * (w * 0.5 + 0.3 * p + out as f32 * 0.5 * p),
                            spot.y + 0.12 * p,
                        )
                    };
                    let found = (0..4)
                        .flat_map(|out| [(first, out), (-first, out)])
                        .map(|(side, out)| at(side, out))
                        .find(|(x, y)| {
                            open(*x, *y, p, thing)
                                && besides
                                    .iter()
                                    .all(|(ox, _): &(f32, f32)| (ox - x).abs() >= p * 0.45)
                        })
                        .unwrap_or_else(|| at(first, 0));
                    besides.push(found);
                    found
                }
            };
            let x = if how == Use::Ring {
                x
            } else {
                clear_of_buildings(x, y).clamp(figure_h * 0.3, stage.width - figure_h * 0.3)
            };
            if let Some(standing) = stage.people.iter_mut().find(|other| other.index == person) {
                standing.x = x;
                standing.y = y;
                standing.scale = ring_scale;
            }
            // Seen whenever what they use is.
            match stage.shown_from.get(&thing).copied() {
                Some(least) => stage.shown_from.insert(person, least),
                None => stage.shown_from.remove(&person),
            };
            if let Some(what) = stage.doing.get_mut(&person) {
                what.fixture = Some((thing, how));
            }
        }
    }
    // Companions anywhere else stand side by side: one steps in beside the
    // other, changing places with whoever stood there.
    let at_fixture = |doing: &BTreeMap<usize, Doing>, index: usize| {
        doing.get(&index).is_some_and(|what| what.fixture.is_some())
    };
    let pairs = stage
        .doing
        .iter()
        .filter_map(|(person, what)| Some((*person, what.with?)))
        .filter(|(a, b)| a < b)
        .collect::<Vec<_>>();
    let together = figure_h * TOGETHER;
    for (a, b) in pairs {
        if at_fixture(&stage.doing, a) || at_fixture(&stage.doing, b) {
            continue;
        }
        let find =
            |people: &[Spot], index: usize| people.iter().position(|spot| spot.index == index);
        let (Some(pa), Some(pb)) = (find(&stage.people, a), find(&stage.people, b)) else {
            continue;
        };
        let (sa, sb) = (stage.people[pa], stage.people[pb]);
        if (sa.x - sb.x).abs() <= together && (sa.y - sb.y).abs() < figure_h * 0.5 {
            continue;
        }
        let partnered = |index: usize| {
            stage
                .doing
                .get(&index)
                .is_some_and(|what| what.with.is_some())
        };
        let neighbour = stage
            .people
            .iter()
            .enumerate()
            .filter(|(_, spot)| {
                spot.index != a
                    && spot.index != b
                    && !partnered(spot.index)
                    && !at_fixture(&stage.doing, spot.index)
                    && (spot.x - sa.x).abs() <= together
                    && (spot.y - sa.y).abs() < figure_h * 0.5
            })
            .min_by(|x, y| (x.1.x - sa.x).abs().total_cmp(&(y.1.x - sa.x).abs()))
            .map(|(at, _)| at);
        match neighbour {
            Some(pc) => {
                let sc = stage.people[pc];
                stage.people[pb] = Spot { index: b, ..sc };
                stage.people[pc] = Spot {
                    index: sc.index,
                    ..sb
                };
            }
            None => {
                let side = if sb.x >= sa.x { 1.0 } else { -1.0 };
                let x = sa.x + side * figure_h * 0.62;
                let crowded = stage.people.iter().any(|spot| {
                    spot.index != a
                        && spot.index != b
                        && (spot.x - x).abs() < figure_h * 0.5
                        && (spot.y - sa.y).abs() < figure_h * 0.3
                });
                if !crowded {
                    stage.people[pb] = Spot {
                        x: clear_of_buildings(x, sa.y),
                        y: sa.y,
                        scale: sa.scale,
                        ..sb
                    };
                }
            }
        }
    }
    // Those dancing have a patch of ground of their own: a pair (or one)
    // with anyone else close by moves along their lane, together, to the
    // nearest open ground with nobody else within a stride.
    let dancing = stage
        .doing
        .iter()
        .filter(|(index, what)| {
            what.stance == Some(Stance::Dancing) && what.fixture.is_none() && out.contains(index)
        })
        .map(|(index, _)| *index)
        .collect::<BTreeSet<_>>();
    let mut moved = BTreeSet::new();
    for dancer in &dancing {
        if moved.contains(dancer) {
            continue;
        }
        let group = std::iter::once(*dancer)
            .chain(
                stage.doing[dancer]
                    .with
                    .filter(|with| dancing.contains(with)),
            )
            .collect::<Vec<_>>();
        let spots = group
            .iter()
            .filter_map(|index| stage.people.iter().position(|spot| spot.index == *index))
            .collect::<Vec<_>>();
        let clear_at = |shift: f32| {
            spots.iter().all(|at| {
                let spot = stage.people[*at];
                let (x, y) = (spot.x + shift, spot.y);
                open(x, y, figure_h * spot.scale, usize::MAX)
                    && stage.people.iter().all(|other| {
                        group.contains(&other.index)
                            || (other.x - x).abs() >= figure_h * 1.1
                            || (other.y - y).abs() >= figure_h * 0.5
                    })
            })
        };
        let shift = (0..=60)
            .flat_map(|step| [step, -step])
            .map(|step| step as f32 * figure_h * 0.25)
            .find(|shift| clear_at(*shift));
        if let Some(shift) = shift.filter(|shift| *shift != 0.0) {
            for at in &spots {
                stage.people[*at].x += shift;
            }
        }
        moved.extend(group);
    }
    // Everyone else out of doors runs an errand now and then: out to the
    // nearest thing to use, or for a carrier back where the load comes
    // from, and home again.
    let usable = stage
        .things
        .iter()
        .filter(|spot| {
            stage.depth_of(spot) != Depth::Water
                && stage
                    .shown_from
                    .get(&spot.index)
                    .is_none_or(|least| *least <= 1.0)
        })
        .filter_map(|spot| Some((*spot, errand_to(items[spot.index].shape?)?)))
        .collect::<Vec<_>>();
    // With nothing to use near, an errand calls at another door.
    let stops = stage
        .buildings
        .iter()
        .map(|spot| (spot.x, spot.w))
        .collect::<Vec<_>>();
    let reach = stage.view_w * 0.4;
    for spot in stage.people.clone() {
        if stage.routes.contains_key(&spot.index) {
            continue;
        }
        // A baby goes nowhere on their own (a pram stays by its people).
        let baby = items[spot.index]
            .look
            .is_some_and(|look| Age::of(&look) == Age::Baby);
        let what = stage.doing.entry(spot.index).or_default();
        if what.fixture.is_some() || what.with.is_some() || baby {
            continue;
        }
        let home = spot.x;
        let beside = |x: f32, w: f32| {
            let side = (x - home).signum();
            x - side * (w * 0.5 + figure_h * 0.35)
        };
        let carrier = what.stance == Some(Stance::Carrying);
        // Somewhere they can walk to along their line, no building between.
        let reachable = |to: f32| {
            (kept_out(&footprints, home, to, spot.y, figure_h * 0.2) - to).abs() < figure_h * 0.3
        };
        let fetch = came_from
            .get(&spot.index)
            .filter(|_| carrier)
            .and_then(|from| Some((*slot_x.get(from)?, stage.building_w * 0.5)))
            .filter(|(x, w)| (x - home).abs() < reach * 1.5 && reachable(beside(*x, *w)));
        what.errand = if let Some((x, w)) = fetch {
            Some(Errand {
                to: beside(x, w),
                at: x,
                stance: Stance::Working,
                fetch: true,
            })
        } else if let Some((thing, stance)) = usable
            .iter()
            .filter(|(thing, _)| {
                // On their own ground, so they are seen to get to it, and
                // far enough to be seen to go.
                let away = (thing.x - home).abs();
                (thing.y - spot.y).abs() < figure_h * 0.6
                    && away > figure_h * 2.0
                    && away < reach
                    && reachable(beside(thing.x, thing.w))
            })
            .min_by(|a, b| (a.0.x - home).abs().total_cmp(&(b.0.x - home).abs()))
        {
            Some(Errand {
                to: beside(thing.x, thing.w),
                at: thing.x,
                stance: *stance,
                fetch: false,
            })
        } else if stops.len() >= 2 {
            // Over to another door, as far as the next few, to call there.
            let seed = art::seed_of(&items[spot.index].id.stable_key());
            let nearest = stops
                .iter()
                .enumerate()
                .min_by(|a, b| (a.1 .0 - home).abs().total_cmp(&(b.1 .0 - home).abs()))
                .map(|(position, _)| position)
                .unwrap_or(0);
            let pick = (nearest + 1 + (seed as usize / 7) % (stops.len() - 1)) % stops.len();
            let (x, _) = stops[pick];
            // At the door, a step short of it, if they can walk there.
            let to = x - (x - home).signum() * figure_h * 0.3;
            ((x - home).abs() < stage.view_w * 0.45 && reachable(to)).then_some(Errand {
                to,
                at: x,
                stance: Stance::Talking,
                fetch: false,
            })
        } else {
            None
        };
    }
}

/// How long one stretch of an hour an errand may fall in is, in seconds.
pub const ERRAND_SLOT: f32 = 180.0;

/// How long someone stays where an errand takes them, in seconds.
pub const ERRAND_STAY: f32 = 40.0;

/// Where someone is in an errand at `into_hour` seconds into `hour` of the
/// World's `day`: a pure function of who they are (`seed`, of their stable
/// key), the day, the hour and how far into it, and how long each way
/// takes them (`leg`, in seconds). Half the stretches of an hour have one,
/// starting at a moment of their own and over within the stretch: so it
/// never runs over into the next hour.
pub fn errand_at(seed: u32, day: u64, hour: u8, into_hour: f32, leg: f32) -> Option<(Leg, f32)> {
    let slot = (into_hour / ERRAND_SLOT).floor().max(0.0);
    let mut mix = u64::from(seed)
        ^ day.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ u64::from(hour).wrapping_mul(0xbf58_476d_1ce4_e5b9)
        ^ (slot as u64).wrapping_mul(0x94d0_49bb_1331_11eb);
    mix ^= mix >> 31;
    mix = mix.wrapping_mul(0xd6e8_feb8_6659_fd93);
    mix ^= mix >> 32;
    if mix % 2 == 1 {
        return None;
    }
    let length = leg * 2.0 + ERRAND_STAY + SETTLE;
    let spare = ERRAND_SLOT - 4.0 - length;
    if spare <= 0.0 {
        return None;
    }
    let start = ((mix >> 8) % 1000) as f32 / 1000.0 * spare;
    let t = into_hour - slot * ERRAND_SLOT - start;
    match t {
        t if t < 0.0 => None,
        t if t < leg => Some((Leg::Out, t)),
        t if t < leg + ERRAND_STAY => Some((Leg::There, t - leg)),
        t if t < leg * 2.0 + ERRAND_STAY => Some((Leg::Back, t - leg - ERRAND_STAY)),
        t if t < length => Some((Leg::Home, t - leg * 2.0 - ERRAND_STAY)),
        _ => None,
    }
}

/// How long the coat swings on after someone stops, in seconds.
const SETTLE: f32 = 1.5;

/// The parts of an errand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Leg {
    Out,
    There,
    Back,
    Home,
}

/// Where a walk along the line `y` from `from` toward `to` ends so that
/// it goes into none of `footprints` (left, back, right, front): stopped
/// `room` short of the first it would enter. A walk that starts inside
/// one is let be.
pub fn kept_out(footprints: &[(f32, f32, f32, f32)], from: f32, to: f32, y: f32, room: f32) -> f32 {
    let mut to = to;
    for (left, back, right, front) in footprints {
        if y <= *back || y >= *front || (from > *left && from < *right) {
            continue;
        }
        if from <= *left && to > *left - room {
            to = to.min(left - room).max(from.min(to));
        } else if from >= *right && to < *right + room {
            to = to.max(right + room).min(from.max(to));
        }
    }
    to
}

/// A person as drawn this frame, in screen pixels.
#[derive(Clone, Debug)]
pub struct PersonPaint {
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub height: f32,
    pub figure: Figure,
    pub pose: Pose,
    /// A soft light on the ground under them: news, a preview, the one
    /// being asked.
    pub glow: Option<Hsla>,
    /// Their Pack's drawing of them, if it ships one.
    pub drawing: Option<Drawing>,
    /// What they are doing, for their drawing.
    pub stance: Stance,
    /// How they feel, for their face.
    pub mood: world_projection::Mood,
    /// A baby they carry in their arms.
    pub carrying: Option<Figure>,
    /// Where along the stage they are, in stage pixels: which row of a
    /// folded postcard they stand in.
    pub along: f32,
    /// Sitting on a bench: how high its seat is, for their height. Their
    /// feet stay on the ground (`y`), their legs reaching down to it.
    pub seat: Option<f32>,
    /// Which beat of a dance they are on (0 to 3).
    pub beat: Option<u8>,
    /// The thing (an item index) they stand behind, keeping it: drawn over
    /// them wherever people are painted over the still layers.
    pub behind: Option<usize>,
}

/// A baby out and about is carried: in the arms of a grown-up from their
/// home standing near them, who holds them at the chest, or, with nobody
/// of theirs close by, asleep in a pram where they are.
pub(super) fn carry_babies(people: &mut Vec<PersonPaint>, items: &[CanvasItem], figure_h: f32) {
    let babies = people
        .iter()
        .enumerate()
        .filter(|(_, person)| person.figure.age == Age::Baby && !person.figure.bird)
        .map(|(position, _)| position)
        .collect::<Vec<_>>();
    let mut carried = BTreeSet::new();
    for baby in babies {
        let home = items[people[baby].index].home;
        let x = people[baby].x;
        let carrier = people
            .iter()
            .enumerate()
            .filter(|(_, person)| {
                matches!(person.figure.age, Age::Adult | Age::Elder)
                    && !person.figure.bird
                    && person.carrying.is_none()
                    && !matches!(person.stance, Stance::Carrying | Stance::Dancing)
                    && person.seat.is_none()
                    && home.is_some()
                    && items[person.index].home == home
                    && (person.x - x).abs() < figure_h * 2.2
            })
            .min_by(|a, b| (a.1.x - x).abs().total_cmp(&(b.1.x - x).abs()))
            .map(|(position, _)| position);
        if let Some(carrier) = carrier {
            let figure = people[baby].figure;
            let holder = &mut people[carrier];
            holder.carrying = Some(figure);
            // Both arms round the baby, unless they are walking or sit.
            if !matches!(holder.stance, Stance::Walking | Stance::Sitting) {
                holder.stance = Stance::Working;
            }
            carried.insert(baby);
        }
    }
    let mut position = 0;
    people.retain(|_| {
        let keep = !carried.contains(&position);
        position += 1;
        keep
    });
}

/// At dusk the water holds the sky's gold: a warm sheen near the shore
/// fading out to sea, deeper further out, over the water from `front` to
/// `bottom` across `width` from `ox`. How much of dusk there is (0 by day
/// and at night).
pub(super) fn paint_dusk_water(
    window: &mut dyn Brush,
    frame: &Frame,
    ox: f32,
    width: f32,
    front: f32,
    bottom: f32,
) -> f32 {
    let dusk = dusk_share(frame.hour) * if sun_out(frame.weather) { 1.0 } else { 0.5 };
    if dusk > 0.0 {
        let (_, sky_low) = sky_colours(frame);
        let gold = mix(sky_low, art::hex(0xffb24a), 0.5);
        // Deeper out to sea, so the gold reads against it.
        window.gradient(
            ox,
            front,
            width,
            bottom - front,
            180.0,
            (art::hex(0x1c2a44).opacity(0.0), 0.0),
            (
                art::hex(0x1c2a44).opacity(0.55 * dusk_share(frame.hour)),
                1.0,
            ),
        );
        window.gradient(
            ox,
            front,
            width,
            (bottom - front) * 0.45,
            180.0,
            (gold.opacity(0.26 * dusk), 0.0),
            (gold.opacity(0.0), 1.0),
        );
    }
    dusk
}

/// At dusk and at night the lit windows shine on the water, trembling:
/// each building's light broken on the swell, down to `bottom` on screen,
/// `t` seconds in.
pub(super) fn paint_window_reflections(
    window: &mut dyn Brush,
    frame: &Frame,
    screen: &dyn Fn(f32, f32) -> (f32, f32),
    seen: &dyn Fn(f32, f32) -> bool,
    bottom: f32,
    t: f32,
) {
    let lit = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    if !frame.water || !lit || frame.buildings.is_empty() {
        return;
    }
    let z = frame.camera.zoom;
    let front = screen(0.0, frame.front).1;
    let deep = (bottom - front).max(0.0);
    let warm = art::hex(0xffcf7a);
    for building in &frame.buildings {
        if !seen(building.x, building.w * z) {
            continue;
        }
        let (x, _) = screen(building.x, building.base);
        let seed = building.index as f32 * 1.7;
        // Each lit window's light, broken on the swell.
        for streak in 0..2 {
            let along = (streak as f32 - 0.5) * building.w * z * 0.3;
            paint_reflection(
                window,
                x + along,
                front + 2.0 * z,
                deep * 0.42,
                building.w * z * 0.08,
                warm.opacity(0.34),
                t,
                seed + streak as f32 * 3.1,
            );
        }
    }
}

/// Everything that moves, over the still layers, lit by the hour.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_live(
    window: &mut dyn Brush,
    frame: &Frame,
    ox: f32,
    oy: f32,
    width: f32,
    height: f32,
    light: [f32; 3],
) {
    let t = frame.seconds;
    let z = frame.camera.zoom;
    let night = frame.daylight == Daylight::Night;
    let lit = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    let screen = |x: f32, y: f32| {
        let (sx, sy) = frame.at(x, y);
        (ox + sx, oy + sy)
    };
    let seen = |x: f32, reach: f32| {
        let sx = frame.at(x, 0.0).0;
        sx + reach >= 0.0 && sx - reach <= width
    };

    // The sea moves: short bright lines drifting and fading.
    if frame.water {
        let front = screen(0.0, frame.front).1;
        let bottom = oy + height;
        if front < bottom {
            // At dusk the water holds the sky's gold: a warm sheen near the
            // shore fading out to sea, and the swell's lights gold.
            let dusk = paint_dusk_water(window, frame, ox, width, front, bottom);
            let shimmer = mix(gpui::white(), art::hex(0xffc860), dusk).opacity(if night {
                0.12
            } else {
                0.26 + 0.2 * dusk
            });
            for row in 0..5 {
                let y = front + 16.0 * z + row as f32 * (bottom - front) / 5.5;
                for column in 0..9 {
                    let drift = (t * (6.0 + row as f32) + column as f32 * 137.0 - frame.pan() * z)
                        .rem_euclid(width + 80.0);
                    let x = ox + drift - 40.0;
                    let pulse = 0.5 + 0.5 * (t * 1.1 + column as f32 + row as f32 * 0.7).sin();
                    window.rect(
                        x,
                        y,
                        (18.0 + 10.0 * pulse) * z.min(1.5),
                        2.0,
                        1.0,
                        shimmer.opacity(shimmer.a * pulse),
                    );
                }
            }
        }
    }

    // In the rain the stones nearest the water are wet: darker toward the
    // edge, with a sheen of the sky on them.
    if frame.water && matches!(frame.weather, Weather::Rain | Weather::Storm) {
        let front = screen(0.0, frame.front).1;
        let reach = frame.figure_h * z * 0.9;
        let [r, g, b] = shadow_ink(frame.hour);
        let wet = Hsla::from(gpui::Rgba { r, g, b, a: 1.0 });
        window.gradient(
            ox,
            front - reach,
            width,
            reach,
            180.0,
            (wet.opacity(0.0), 0.0),
            (wet.opacity(0.2), 1.0),
        );
        let sheen = gpui::white().opacity(if night { 0.05 } else { 0.1 });
        for streak in 0..14 {
            let x =
                ox + ((streak as f32 * 197.0 - frame.pan() * z).rem_euclid(width + 60.0)) - 30.0;
            let y = front - reach * (0.15 + 0.6 * ((streak * 7) % 5) as f32 / 5.0);
            window.rect(x, y, 14.0 * z.min(1.5), 1.5, 0.75, sheen);
        }
    }

    // At dusk and at night the lit windows shine on the water, trembling.
    // Zoomed out to the postcard, smoke and reflections are too small to
    // see, and not drawn.
    let fine = z >= 0.6;
    if fine {
        paint_window_reflections(window, frame, &screen, &seen, oy + height, t);
    }

    // Buildings springing after a click, or rising just built.
    for building in frame.buildings.iter().filter(|b| b.moving()) {
        if !seen(building.x, building.w * z) {
            continue;
        }
        paint_building_live(window, frame, building, &screen, light);
    }

    // A chimney smokes: puffs rising, spreading and thinning, carried by
    // the wind.
    if fine && frame.weather != Weather::Storm {
        let blow = wind(frame);
        let smoke = if night {
            art::hex(0x8a90a0)
        } else {
            art::hex(0xcfcbc6)
        };
        for building in &frame.buildings {
            if building.shape != MarkShape::House
                || building.drawing.is_some()
                || building.palette.art.is_some()
                || !frame.setting.smokes()
            {
                continue;
            }
            if !seen(building.x, building.w * z * 2.0) {
                continue;
            }
            let (pot_x, pot_y) = art::chimney_top(
                building.x,
                building.base,
                building.w,
                building.h,
                &building.palette,
            );
            let (cx, cy) = screen(pot_x, pot_y);
            let (w, h) = (building.w * z, building.h * z);
            for puff in 0..5 {
                let age = (t * 0.3 + puff as f32 * 0.2 + building.index as f32 * 0.37) % 1.0;
                let rise = ease(age.min(1.0)) * h * 0.55;
                let drift = age * age * blow * w * 0.35;
                let r = w * (0.045 + age * 0.08);
                window.soft(
                    cx + drift,
                    cy - rise,
                    r,
                    r * 0.85,
                    r * 0.5,
                    smoke.opacity(0.6 * (1.0 - age) * (0.3 + 0.7 * (age * 6.0).min(1.0))),
                );
            }
        }
    }

    // Plots staked out on the ground, fading as the place becomes a
    // postcard.
    let opacity = mark::plot_opacity(z);
    if opacity > 0.01 {
        for plot in &frame.plots {
            if !seen(plot.x, plot.w * z) {
                continue;
            }
            let (x, y) = screen(plot.x, plot.y);
            let mut tinted = Tint::new(window, light);
            mark::paint_plot(
                &mut tinted,
                x,
                y,
                plot.w * z,
                opacity,
                frame.hot_plot == Some(plot.plot),
                t + plot.plot as f32,
            );
        }
    }
    paint_worn_places(window, frame, &screen, light, None);

    // A harbour lived in whatever is built: washing, a cart, rowboats.
    super::lived::paint_lived_in(window, frame, &screen, &seen, light);
    // Lamps along the spine, lit from dusk.
    paint_spine_lamps(window, frame, &screen, &seen, light);
    // What stands in the water stands on a jetty.
    paint_jetties(window, frame, &screen, &seen, light);

    // Things: carts, parcels, boats riding the swell.
    let mut things = frame.things.iter().collect::<Vec<_>>();
    things.sort_by(|a, b| a.base.total_cmp(&b.base).then(a.index.cmp(&b.index)));
    for thing in things {
        if !seen(thing.x, thing.w * z) {
            continue;
        }
        paint_thing_live(window, frame, thing, &screen, light);
    }

    // A street's poles, wires and parked cars, between its buildings and
    // its people (art's, B).
    crate::setting::paint_street_life(
        &mut Tint::new(window, light),
        &screen,
        frame.setting,
        frame.width,
        frame.view_w,
        frame.base,
        frame.front,
        frame.building_h,
        z,
        lit,
        seed_of_scenery(&frame.scenery),
        (ox, ox + width),
        (&street_lamps(frame), street_glow(frame)),
    );
    // People: a soft shadow where they stand, a longer one away from a
    // sun that is out, and themselves.
    for person in &frame.people {
        let x = ox + person.x;
        if x + person.height < ox || x - person.height > ox + width {
            continue;
        }
        paint_person_live(window, frame, person, (ox, oy), light);
        // A stall's keeper stands behind its counter: the stall again,
        // over them.
        if let Some(thing) = person
            .behind
            .and_then(|index| frame.things.iter().find(|thing| thing.index == index))
        {
            paint_thing_live(window, frame, thing, &screen, light);
        }
    }
    // The lighthouse's lamp, the brightest point on the land.
    paint_beacons(window, frame, &screen, &seen);
    // Bunting strung between two buildings.
    paint_garlands(window, frame, &screen, light);
    // The framing strip along the foot of the land, in front of all.
    paint_framing_strip(window, frame, ox, width, &screen, light);
    for (x, y, r, tone) in &frame.bonds {
        art::paint_bond(window, ox + x, oy + y, *r, *tone);
    }
}

/// The ink of the soft shadow at someone's feet, for the hour.
pub(super) fn contact_ink(frame: &Frame) -> Hsla {
    let [r, g, b] = shadow_ink(frame.hour);
    let night = frame.daylight == Daylight::Night;
    Hsla::from(gpui::Rgba { r, g, b, a: 1.0 }).opacity(if night { 0.3 } else { 0.25 })
}

/// A building springing after a click, or rising just built, drawn live
/// where `screen` puts it, lit by `light`.
pub(super) fn paint_building_live(
    window: &mut dyn Brush,
    frame: &Frame,
    building: &BuildingPaint,
    screen: &dyn Fn(f32, f32) -> (f32, f32),
    light: [f32; 3],
) {
    let z = frame.camera.zoom;
    let lit = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    let contact = contact_ink(frame);
    let (x, base) = screen(building.x, building.base);
    let (w, h) = (building.w * z, building.h * z);
    let grow = ease(building.grow);
    window.soft(x, base, w * 0.56 * grow, h * 0.05, h * 0.06, contact);
    let mut tinted = Tint::new(window, light);
    let mut hand = crate::hand::Hand::new(&mut tinted, frame.boil(), frame.setting.inks());
    let facing = if building.flip { -1.0 } else { 1.0 };
    let mut posed = Xform::about(
        &mut hand,
        (x, base),
        facing * building.squash.0 * (0.7 + 0.3 * grow),
        building.squash.1 * grow,
        0.0,
    );
    match &building.drawing {
        Some(drawing) => art::paint_drawing(
            &mut posed,
            x,
            base,
            w,
            h,
            drawing,
            &Inks::of_place(&building.palette).lit(lit),
            Stance::Standing,
            world_projection::Mood::Content,
            0.0,
            0.0,
            1.0,
        ),
        None => art::paint_building(&mut posed, x, base, w, h, building.shape, &building.palette),
    }
}

/// A thing drawn live where `screen` puts it, lit by `light`: a cart, a
/// parcel, a boat riding the swell, and the design it wears.
pub(super) fn paint_thing_live(
    window: &mut dyn Brush,
    frame: &Frame,
    thing: &ThingPaint,
    screen: &dyn Fn(f32, f32) -> (f32, f32),
    light: [f32; 3],
) {
    let t = frame.seconds;
    let z = frame.camera.zoom;
    let lit = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    let contact = contact_ink(frame);
    let (x, base) = screen(thing.x, thing.base);
    let w = thing.w * z * (0.6 + 0.4 * ease(thing.grow));
    if let Some(glow) = thing.glow {
        window.soft(
            x,
            base,
            w * 0.8,
            w * 0.16,
            w * 0.12,
            ground_glow(glow, 0.35),
        );
    }
    if thing.shape == MarkShape::Boat {
        // A darker patch of water under the hull.
        window.soft(
            x,
            base + w * 0.04,
            w * 0.5,
            w * 0.06,
            w * 0.06,
            contact.opacity(0.12),
        );
    } else {
        contact_shadow(window, frame.hour, x, base, w * 0.9);
    }
    let worn = frame
        .wearing
        .iter()
        .find(|worn| worn.index == thing.index)
        .and_then(|worn| Some((worn.wear, frame.pictures.get(&thing.index)?)));
    match worn {
        Some((Wear::Flag, picture)) => {
            paint_flag_pole(window, picture, x, base, w, t, wind(frame), light);
            return;
        }
        Some((Wear::Quilt, picture)) => {
            let (qw, qh) = mark::cloth_size(Wear::Quilt, w);
            mark::paint_quilt_line(
                window,
                picture,
                x,
                base - qh * 1.5,
                base,
                qw,
                qh,
                t,
                wind(frame),
            );
            return;
        }
        _ => {}
    }
    let mut tinted = Tint::new(window, light);
    let mut under = Under {
        inner: &mut tinted,
        most: if lit {
            1.0
        } else {
            sky_luma(&frame.scenery) * WALLS_UNDER_SKY
        },
    };
    // Over the screen's accent budget: quieted toward the place's own
    // colours.
    let mut muted = Mute {
        inner: &mut under,
        toward: art::hex(frame.setting.place_paints().neutral()),
        share: if thing.muted { 0.7 } else { 0.0 },
    };
    let mut facing = Xform::about(
        &mut muted,
        (x, base),
        if thing.flip { -1.0 } else { 1.0 },
        1.0,
        0.0,
    );
    let mut rolled = Xform::turned(&mut facing, (x, base), thing.roll);
    match &thing.drawing {
        Some(drawing) => art::paint_drawing(
            &mut rolled,
            x,
            base,
            w,
            w / drawing.aspect,
            drawing,
            &Inks::of_place(&thing.palette),
            Stance::Standing,
            world_projection::Mood::Content,
            thing.sway * 0.3,
            0.0,
            1.0,
        ),
        None => art::paint_thing(
            &mut rolled,
            x,
            base,
            w,
            thing.shape,
            &thing.palette,
            thing.sway,
        ),
    }
    match worn {
        Some((Wear::Sail, picture)) => {
            // A taller mast, and the sail on it, filling.
            let h = w * 0.55;
            let mast = x - w * 0.2;
            let (sw, sh) = mark::cloth_size(Wear::Sail, w);
            let top = base - h * 0.78 - sh;
            {
                let mut tinted = Tint::new(window, light);
                let mut rolled = Xform::turned(&mut tinted, (x, base), thing.roll);
                art::line(
                    &mut rolled,
                    (mast, base - h * 0.4),
                    (mast, top - h * 0.12),
                    2.0,
                    art::hex(0x6b4a33),
                );
                art::line(
                    &mut rolled,
                    (mast, base - h * 0.78),
                    (mast + sw * 1.02, base - h * 0.78),
                    1.6,
                    art::hex(0x6b4a33),
                );
            }
            mark::paint_sail(
                window,
                picture,
                mast + 1.0,
                top,
                sw,
                sh,
                t,
                wind(frame),
                thing.roll,
                base,
            );
        }
        Some((Wear::Sign, picture)) => {
            // A board hung from a post beside the stall.
            let post = x + w * 0.46;
            let tall = w * 0.8;
            {
                let mut tinted = Tint::new(window, light);
                tinted.rect(post - 1.2, base - tall, 2.4, tall, 1.0, art::hex(0x5b4636));
            }
            let (bw, bh) = mark::cloth_size(Wear::Sign, w * 1.4);
            mark::paint_sign(
                window,
                picture,
                post,
                base - tall + 2.0,
                bw,
                bh,
                1.0,
                (t * 1.1 + thing.index as f32).sin() * 0.5,
            );
        }
        _ => {}
    }
}

/// Someone drawn live, in the scene whose top-left is (`ox`, `oy`): a
/// soft shadow where they stand, a longer one away from a sun that is out,
/// and themselves, lit by `light`.
pub(super) fn paint_person_live(
    window: &mut dyn Brush,
    frame: &Frame,
    person: &PersonPaint,
    (ox, oy): (f32, f32),
    light: [f32; 3],
) {
    paint_person_ground(window, frame, person, (ox, oy));
    let contact = contact_ink(frame);
    let (x, y) = (ox + person.x, oy + person.y);
    // Lifted off the ground in a hop, the shadow shrinks and fades.
    let lift = (person.pose.bob / person.height.max(1.0) * 4.0).clamp(0.0, 0.6);
    window.soft(
        x,
        y,
        person.height * 0.2 * (1.0 - lift * 0.4),
        person.height * 0.045,
        person.height * 0.05,
        contact.opacity(1.0 - lift),
    );
    paint_person_body(window, frame, person, (x, y), light);
}

/// What someone throws on the ground, in the scene whose top-left is
/// (`ox`, `oy`): the glow under them if they are lit up, and by day a
/// longer shadow away from a sun that is out (not the soft shadow at
/// their feet).
pub(super) fn paint_person_ground(
    window: &mut dyn Brush,
    frame: &Frame,
    person: &PersonPaint,
    (ox, oy): (f32, f32),
) {
    let t = frame.seconds;
    let night = frame.daylight == Daylight::Night;
    let (across, high) = sun_at(frame.hour);
    let day = sun_out(frame.weather) && high > 0.02 && !night;
    let long = 0.22 + 0.9 * (1.0 - high.max(0.0)).powi(2);
    let away = if across < 0.0 { 1.0 } else { -1.0 };
    let (x, y) = (ox + person.x, oy + person.y);
    if let Some(glow) = person.glow {
        let breathe = 0.9 + 0.1 * (t * 2.4).sin();
        window.soft(
            x,
            y,
            person.height * 0.8 * breathe,
            person.height * 0.2 * breathe,
            person.height * 0.14,
            ground_glow(glow, 0.45),
        );
    }
    // Lifted off the ground in a hop, the shadow shrinks and fades.
    let lift = (person.pose.bob / person.height.max(1.0) * 4.0).clamp(0.0, 0.6);
    if day {
        let reach = person.height * long * 0.8;
        window.soft(
            x + away * reach * 0.5,
            y + person.height * 0.015,
            person.height * 0.1 + reach * 0.5,
            person.height * 0.035,
            person.height * 0.05,
            gpui::black().opacity(0.1 * (1.0 - lift)),
        );
    }
}

/// Someone themselves, their feet at (`x`, `y`) and `person.height` tall,
/// as they stand, lit by `light`: their seat if they sit, and a baby in
/// their arms.
pub(super) fn paint_person_body(
    window: &mut dyn Brush,
    frame: &Frame,
    person: &PersonPaint,
    (x, y): (f32, f32),
    light: [f32; 3],
) {
    let lit = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    let mut lit_by = Tint::new(window, light);
    // By day nobody's whites are lighter than the sky (the value bands).
    let mut tinted = Under {
        inner: &mut lit_by,
        most: if lit {
            1.0
        } else {
            sky_luma(&frame.scenery) * WALLS_UNDER_SKY
        },
    };
    // Someone sitting sits on something (art's, B): the bench they are
    // on, or a seat of their own.
    if person.stance == Stance::Sitting && !person.figure.bird && person.seat.is_none() {
        crate::setting::paint_seat(
            &mut tinted,
            frame.setting,
            x,
            y,
            person.height,
            person.pose.facing,
        );
    }
    let (stance, pose) = match (person.stance, person.beat) {
        (Stance::Dancing, beat) => dance(beat.unwrap_or(0), person.pose, person.height),
        (stance, _) => (stance, person.pose),
    };
    let on_bench = person
        .seat
        .zip(person.drawing.as_ref())
        .filter(|_| stance == Stance::Sitting)
        .map(|(seat, drawing)| seated(drawing, seat, person.figure.bird));
    age::paint_person(
        &mut tinted,
        x,
        y,
        person.height,
        &person.figure,
        on_bench.as_ref().or(person.drawing.as_ref()),
        stance,
        person.mood,
        pose,
    );
    if stance == Stance::Carrying && !person.figure.bird {
        paint_load(&mut tinted, frame.setting, person, (x, y));
    }
    if let Some(baby) = &person.carrying {
        let grown = person.height / person.figure.age.height().max(0.1);
        age::paint_bundle(
            &mut tinted,
            x,
            y,
            grown,
            person.pose.facing,
            person.pose.bob,
            baby,
            &person.figure,
        );
    }
}

/// Where a sitter's hips are in a drawing's sitting stance, as a share of
/// their height above their feet: the seat of their own a sitter brings
/// (`setting::paint_seat`) is this high.
const OWN_SEAT: f32 = 0.2;

/// How high a drawing's knees are when it sits, before it is drawn lower
/// (`world_projection::drop_of`), and the top of its shoes.
const KNEE: f32 = 0.36;

/// How far behind the near leg a sitter's far leg shows, in widths.
const FAR_LEG: f32 = 0.11;
const SHOE_TOP: f32 = 0.16;

/// `drawing` sitting on a seat `seat` of their height up: everything above
/// the knee lifted onto it, the shins reaching down to the ground and the
/// shoes left on it, so nothing floats (the art bible's §5). A bird is
/// perched, all of it lifted.
pub(super) fn seated(drawing: &Drawing, seat: f32, bird: bool) -> Drawing {
    use world_projection::DrawShape;
    let lift = (seat - OWN_SEAT).max(0.0);
    let reach = (KNEE - SHOE_TOP + lift) / (KNEE - SHOE_TOP);
    let mut out = drawing.clone();
    // The far leg, a little behind the near one and a shade darker, so
    // both show: the drawing sits in profile with one.
    if !bird {
        let legs = out
            .parts
            .iter()
            .position(|part| part.stances == [Stance::Sitting])
            .unwrap_or(out.parts.len());
        let far = out
            .parts
            .iter()
            .filter(|part| part.stances == [Stance::Sitting])
            .map(|part| {
                let mut part = part.clone();
                part.tone -= 0.18;
                match &mut part.shape {
                    DrawShape::Rect { x, .. } | DrawShape::Ellipse { x, .. } => *x -= FAR_LEG,
                    DrawShape::Polygon { points } => {
                        for point in points.iter_mut() {
                            point.0 -= FAR_LEG;
                        }
                    }
                    DrawShape::Line { from, to, .. } => {
                        from.0 -= FAR_LEG;
                        to.0 -= FAR_LEG;
                    }
                }
                part
            })
            .collect::<Vec<_>>();
        out.parts.splice(legs..legs, far);
    }
    for part in &mut out.parts {
        let ys = match &part.shape {
            DrawShape::Rect { y, h, .. } => vec![*y, y + h],
            DrawShape::Ellipse { y, ry, .. } => vec![y - ry, y + ry],
            DrawShape::Polygon { points } => points.iter().map(|(_, y)| *y).collect(),
            DrawShape::Line { from, to, .. } => vec![from.1, to.1],
        };
        let top = ys.iter().copied().fold(f32::MIN, f32::max);
        let legs = !bird && part.stances == [Stance::Sitting] && top <= KNEE;
        let map = |y: f32| -> f32 {
            match (legs, top <= SHOE_TOP) {
                (true, true) => y,
                (true, false) => {
                    SHOE_TOP + (y - SHOE_TOP).max(0.0) * reach + (y - SHOE_TOP).min(0.0)
                }
                (false, _) => y + lift,
            }
        };
        match &mut part.shape {
            DrawShape::Rect { y, h, .. } => {
                let (bottom, top) = (map(*y), map(*y + *h));
                *y = bottom;
                *h = top - bottom;
            }
            DrawShape::Ellipse { y, ry, .. } => {
                let (bottom, top) = (map(*y - *ry), map(*y + *ry));
                *y = (bottom + top) / 2.0;
                *ry = (top - bottom) / 2.0;
            }
            DrawShape::Polygon { points } => {
                for point in points.iter_mut() {
                    point.1 = map(point.1);
                }
            }
            DrawShape::Line { from, to, .. } => {
                from.1 = map(from.1);
                to.1 = map(to.1);
            }
        }
    }
    out
}

/// One beat (0 to 3) of a little dance: arms up and swaying one way, a
/// hand high on a skip, arms up and swaying the other way, a hand high
/// again; the feet apart and together. Drawn in the stances a drawing
/// already has.
pub(super) fn dance(beat: u8, pose: Pose, height: f32) -> (Stance, Pose) {
    let stance = if beat.is_multiple_of(2) {
        Stance::Celebrating
    } else {
        Stance::Waving
    };
    let (lean, step, skip) = match beat % 4 {
        0 => (-0.05, Some(0.125), 0.0),
        1 => (0.0, None, 0.05),
        2 => (0.05, Some(0.625), 0.0),
        _ => (0.0, None, 0.035),
    };
    (
        stance,
        Pose {
            stride: step,
            bob: pose.bob + skip * height,
            lean: pose.lean + lean,
            ..pose
        },
    )
}

/// The load someone carries in both arms, held against them at the waist,
/// their hands at its sides: a crate of the catch by water, of greens and
/// fruit where there is none.
pub(super) fn paint_load(
    window: &mut dyn Brush,
    setting: art::Setting,
    person: &PersonPaint,
    (x, y): (f32, f32),
) {
    let h = person.height;
    let pose = person.pose;
    let facing = if pose.facing < 0.0 { -1.0 } else { 1.0 };
    let mut posed = Xform::about(
        window,
        (x, y),
        1.0 / pose.squash.max(0.2).sqrt(),
        pose.squash,
        pose.lean,
    );
    let window: &mut dyn Brush = &mut posed;
    // A box half as wide as they are tall, held from the hip to the chest,
    // so it reads as a load and not a button on the coat.
    let (w, tall) = (h * 0.46, h * 0.25);
    let cx = x + facing * h * 0.07;
    let bottom = y - h * 0.22 - pose.bob;
    let top = bottom - tall;
    // In the place's own wood, or a wicker or a dark crate where the wood
    // is too near their clothes to tell apart: the one furthest from them
    // in lightness.
    let kit = crate::works::Kit::of(setting);
    let clothes = person.figure.clothes.l;
    let body = [kit.wood, art::hex(0xe2c48a), art::hex(0x5b3b22)]
        .into_iter()
        .max_by(|a, b| (a.l - clothes).abs().total_cmp(&(b.l - clothes).abs()))
        .unwrap_or(kit.wood);
    let edge = art::shade(body, if body.l > 0.5 { -0.45 } else { -0.3 });
    // What it holds peeps over the rim: a catch where there is water,
    // greens and fruit where there is not.
    if setting.water() == Some(false) {
        art::ellipse(window, cx - w * 0.18, top, w * 0.17, tall * 0.4, kit.leaf);
        art::ellipse(
            window,
            cx + w * 0.14,
            top + tall * 0.05,
            w * 0.15,
            tall * 0.34,
            kit.bloom,
        );
    } else {
        for dx in [-0.28_f32, 0.0, 0.28] {
            art::ellipse(
                window,
                cx + w * dx,
                top + tall * 0.04,
                w * 0.15,
                tall * 0.22,
                art::hex(0xc9d3dc),
            );
        }
    }
    let rim = (h * 0.018).max(0.8);
    art::rect(
        window,
        cx - w / 2.0 - rim,
        top - rim,
        w + rim * 2.0,
        tall + rim * 2.0,
        h * 0.02,
        edge,
    );
    art::rect(window, cx - w / 2.0, top, w, tall, h * 0.012, body);
    // Its slats.
    for v in [0.36_f32, 0.68] {
        art::rect(
            window,
            cx - w / 2.0,
            top + tall * v,
            w,
            (tall * 0.09).max(0.8),
            0.0,
            edge,
        );
    }
    // Hands at its sides.
    let skin = person.figure.skin;
    for side in [-1.0_f32, 1.0] {
        art::ellipse(
            window,
            cx + side * (w * 0.5 + rim),
            top + tall * 0.4,
            h * 0.04,
            h * 0.04,
            skin,
        );
    }
}

/// How far either side of someone a click or the pointer finds them, on
/// screen, for people standing at (`x`, feet `y`), `h` tall: about their
/// width and a little more, but never past half way to anyone standing
/// close enough to overlap, so the person nearest the pointer is the one
/// found, and two people are never pointed at at once (the art bible's
/// §6, and v0.26's wrong clicks).
pub fn reach_of(people: &[(f32, f32, f32)]) -> Vec<f32> {
    people
        .iter()
        .enumerate()
        .map(|(n, (x, y, h))| {
            let natural = h * 0.35 + 20.0;
            people
                .iter()
                .enumerate()
                .filter(|(m, (_, oy, oh))| {
                    *m != n && (oy - oh).max(y - h) < (oy + 24.0).min(y + 24.0)
                })
                .map(|(_, (ox, ..))| (ox - x).abs() / 2.0)
                .fold(natural, f32::min)
                .max(2.0)
        })
        .collect()
}

/// Which of the people at (`x`, feet `y`), `h` tall, the point (`px`,
/// `py`) is on, if any: the nearest whose reach it is in.
pub fn person_at(people: &[(f32, f32, f32)], px: f32, py: f32) -> Option<usize> {
    let reach = reach_of(people);
    let mut best: Option<(usize, f32)> = None;
    for (n, ((x, y, h), reach)) in people.iter().zip(reach).enumerate() {
        if (px - x).abs() > reach || py < y - h || py > y + 24.0 {
            continue;
        }
        let (dx, dy) = (px - x, (py - (y - h * 0.5)) * 0.5);
        let d = dx * dx + dy * dy;
        if best.is_none_or(|(_, least)| d < least) {
            best = Some((n, d));
        }
    }
    best.map(|(n, _)| n)
}

/// Bunting strung between two buildings' eaves (`Frame::string_bunting`):
/// a sagging line of little flags in the place's own colours.
pub(super) fn paint_garlands(
    window: &mut dyn Brush,
    frame: &Frame,
    screen: &dyn Fn(f32, f32) -> (f32, f32),
    light: [f32; 3],
) {
    let paints = frame.setting.place_paints();
    let inks = [
        paints.accents[0],
        paints.base[4],
        paints.base[2],
        paints.accents[1],
    ];
    let mut tinted = Tint::new(window, light);
    for (a, b) in &frame.garlands {
        let find = |index: usize| {
            frame
                .buildings
                .iter()
                .find(|building| building.index == index)
        };
        let (Some(a), Some(b)) = (find(*a), find(*b)) else {
            continue;
        };
        let (left, right) = if a.x < b.x { (a, b) } else { (b, a) };
        let from = screen(left.x + left.w * 0.36, left.base - left.h * 0.52);
        let to = screen(right.x - right.w * 0.36, right.base - right.h * 0.52);
        let span = (to.0 - from.0).abs();
        let sag = span * 0.12;
        let at = |u: f32| {
            (
                from.0 + (to.0 - from.0) * u,
                from.1 + (to.1 - from.1) * u + sag * 4.0 * u * (1.0 - u),
            )
        };
        let mut line = Shape::new();
        line.move_to(from.0, from.1).curve_to(
            to.0,
            to.1,
            (from.0 + to.0) / 2.0,
            (from.1 + to.1) / 2.0 + sag * 2.0,
        );
        let p = frame.figure_h * frame.camera.zoom;
        tinted.stroke(&line, (p * 0.025).max(0.8), art::hex(0x4a3a2e));
        let flags = ((span / (p * 0.32)) as usize).max(3);
        for n in 0..flags {
            let u = (n as f32 + 0.5) / flags as f32;
            let (x, y) = at(u);
            let size = p * 0.11;
            art::polygon(
                &mut tinted,
                &[(x - size, y), (x + size, y), (x, y + size * 1.7)],
                art::hex(inks[n % inks.len()]),
            );
        }
    }
}

/// Where a street's lamps stand, in stage pixels, when its Pack says.
pub(super) fn street_lamps(frame: &Frame) -> Vec<f32> {
    frame
        .look
        .as_ref()
        .map(|look| look.lamps.iter().map(|at| at * frame.view_w).collect())
        .unwrap_or_default()
}

/// What a street's lamps glow: the Pack's own colour, or sodium orange.
pub(super) fn street_glow(frame: &Frame) -> Hsla {
    match frame.look.as_ref().and_then(|look| look.glow) {
        Some(glow) => art::hex(glow),
        None => art::hex(0xffc96b),
    }
}
