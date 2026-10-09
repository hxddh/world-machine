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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Living {
    pub x: f32,
    pub pose: Pose,
    /// What they do with an idle moment, or how they answer a click, over
    /// whatever their Pack says they are doing.
    pub stance: Option<Stance>,
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
        let mut places = Vec::with_capacity(run.len());
        let mut left = 0.0;
        for (group, size) in sizes.iter().enumerate() {
            let step = [0.0, -0.24, 0.12][group % 3] * figure_h;
            for nth in 0..*size {
                let back = step
                    + if *size == 3 && nth == 1 {
                        figure_h * 0.22
                    } else {
                        0.0
                    };
                places.push((left + near * nth as f32, back));
            }
            left += width_of(*size) + between;
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
pub(super) fn group_centres(people: &[Spot], figure_h: f32) -> Vec<Option<f32>> {
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

/// Where each person is this frame, `seconds` into looking at the World.
///
/// Between turns everyone goes about their day: each on a cycle of their
/// own, most of it at home, part of it walking over to another place and
/// back. At night nobody wanders. Anyone `pinned` (speaking, being asked,
/// in the news) stays where they are. `walking` is how far through the walk
/// a turn started is, from where `before` had them. Someone whose day has
/// just taken them somewhere else strolls there.
pub fn living(
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    seconds: f32,
    daylight: Daylight,
    pinned: &BTreeSet<SelectionId>,
    before: Option<(&Stage, &ProjectionSnapshot, f32)>,
) -> Vec<Living> {
    let items = &snapshot.canvas.items;
    let stops = stage
        .buildings
        .iter()
        .chain(stage.things.iter())
        .map(|spot| spot.x)
        .collect::<Vec<_>>();
    let figure_h = stage.figure_h;
    // Walking from `from` to `to`, `since` seconds of `length` in: planted
    // steps, a lean into the walk, a coat that swings as they start and
    // stop.
    let walk = |from: f32, to: f32, since: f32, length: f32, pace: f32| {
        let t = since / length.max(0.01);
        let facing = (to - from).signum();
        // Shorter, slower steps for someone who takes their time.
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
        }
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
    stage
        .people
        .iter()
        .zip(centres)
        .map(|(spot, centre)| {
            let item = &items[spot.index];
            let key = item.id.stable_key();
            let seed = art::seed_of(&key);
            let home = spot.x;
            let breathe = (seconds * 1.7 + (seed % 100) as f32 * 0.07).sin();
            let pace = item.look.map_or(1.0, |look| Age::of(&look).pace());
            // Walking over to where a turn put them.
            if let Some((old_stage, old_snapshot, progress)) = before {
                if let Some(old) = old_stage.person(item.id, old_snapshot) {
                    if (old.x - home).abs() > 2.0 {
                        let since = progress * WALK_SECONDS;
                        if progress < 1.0 {
                            return walk(old.x, home, since, WALK_SECONDS, pace);
                        }
                        if since < WALK_SECONDS + 1.5 {
                            let still = Living {
                                x: home,
                                pose: Pose::default(),
                                stance: None,
                            };
                            return arrived(still, since - WALK_SECONDS, (home - old.x).signum());
                        }
                    }
                }
            }
            // On the way to where their day has them this hour.
            if let Some((from, since)) = stage.routes.get(&spot.index) {
                let length = (home - from).abs() / (figure_h * STROLL * pace);
                if *since < length {
                    return walk(*from, home, *since, length, pace);
                }
            }
            // Someone standing with others faces into their group, and
            // now and then glances away; alone, they look about.
            let looking = ((seconds * 0.11 + (seed % 7) as f32).sin() * 1.4).clamp(-1.0, 1.0);
            let facing = match centre {
                Some(centre) if (centre - home).abs() > 1.0 => {
                    let glance = ((seconds * 0.07 + (seed % 11) as f32).sin() > 0.93) as u8;
                    (centre - home).signum() * if glance == 1 { -0.4 } else { 1.0 }
                }
                // The middle of a ring faces out, toward the viewer.
                Some(_) => looking * 0.3,
                None => looking,
            };
            let still = Living {
                x: home
                    + (seconds * 0.23 + seed as f32).sin()
                        * if centre.is_some() { 1.0 } else { 3.0 },
                pose: Pose {
                    stride: None,
                    bob: breathe * 0.6,
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
            };
            // Nobody wanders off from the people they are talking with.
            if pinned.contains(&item.id)
                || centre.is_some()
                || daylight == Daylight::Night
                || stops.len() < 2
            {
                return still;
            }
            // A visit: out to another place, a while there, and home.
            let period = 34.0 + (seed % 17) as f32;
            let into = (seconds + (seed % 1000) as f32 * 0.37) % period;
            let phase = into / period;
            let nearest = stops
                .iter()
                .enumerate()
                .min_by(|a, b| (a.1 - home).abs().total_cmp(&(b.1 - home).abs()))
                .map(|(position, _)| position)
                .unwrap_or(0);
            let others = stops.len() - 1;
            let pick = (nearest + 1 + (seed as usize / 7) % others) % stops.len();
            // Along a panorama, only as far as the next few places.
            let away = stops[pick] + ((seed % 5) as f32 - 2.0) * stage.figure_h * 0.25;
            let away = away.clamp(home - stage.view_w * 0.45, home + stage.view_w * 0.45);
            // In the same while, someone slower gets less far.
            let away = home + (away - home) * pace.min(1.0);
            let leg = period * 0.08;
            match phase {
                p if (0.60..0.68).contains(&p) => walk(home, away, into - period * 0.60, leg, pace),
                p if (0.68..0.80).contains(&p) => arrived(
                    Living {
                        x: away,
                        stance: Some(Stance::LookingAround),
                        ..still
                    },
                    into - period * 0.68,
                    (away - home).signum(),
                ),
                p if (0.80..0.88).contains(&p) => walk(away, home, into - period * 0.80, leg, pace),
                p if (0.88..0.93).contains(&p) => {
                    arrived(still, into - period * 0.88, (home - away).signum())
                }
                _ => still,
            }
        })
        .collect()
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
            // Both arms round the baby, unless they are walking.
            if holder.stance != Stance::Walking {
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
    let (across, high) = sun_at(frame.hour);
    let day = sun_out(frame.weather) && high > 0.02 && !night;
    let screen = |x: f32, y: f32| {
        let (sx, sy) = frame.at(x, y);
        (ox + sx, oy + sy)
    };
    let seen = |x: f32, reach: f32| {
        let sx = frame.at(x, 0.0).0;
        sx + reach >= 0.0 && sx - reach <= width
    };
    let contact = {
        let [r, g, b] = shadow_ink(frame.hour);
        Hsla::from(gpui::Rgba { r, g, b, a: 1.0 }).opacity(if night { 0.3 } else { 0.25 })
    };

    // The sea moves: short bright lines drifting and fading.
    if frame.water {
        let front = screen(0.0, frame.front).1;
        let bottom = oy + height;
        if front < bottom {
            let shimmer = gpui::white().opacity(if night { 0.12 } else { 0.26 });
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
    if fine && frame.water && lit && !frame.buildings.is_empty() {
        let front = screen(0.0, frame.front).1;
        let bottom = oy + height;
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

    // Buildings springing after a click, or rising just built.
    for building in frame.buildings.iter().filter(|b| b.moving()) {
        if !seen(building.x, building.w * z) {
            continue;
        }
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
            None => {
                art::paint_building(&mut posed, x, base, w, h, building.shape, &building.palette)
            }
        }
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
    paint_worn_places(window, frame, &screen, light);

    // A harbour lived in whatever is built: washing, a cart, rowboats.
    super::lived::paint_harbour_life(window, frame, &screen, &seen, light);
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
                continue;
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
                continue;
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
    let long = 0.22 + 0.9 * (1.0 - high.max(0.0)).powi(2);
    let away = if across < 0.0 { 1.0 } else { -1.0 };
    for person in &frame.people {
        let (x, y) = (ox + person.x, oy + person.y);
        if x + person.height < ox || x - person.height > ox + width {
            continue;
        }
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
        window.soft(
            x,
            y,
            person.height * 0.2 * (1.0 - lift * 0.4),
            person.height * 0.045,
            person.height * 0.05,
            contact.opacity(1.0 - lift),
        );
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
        // Someone sitting sits on something (art's, B).
        if person.stance == Stance::Sitting && !person.figure.bird {
            crate::setting::paint_seat(
                &mut tinted,
                frame.setting,
                x,
                y,
                person.height,
                person.pose.facing,
            );
        }
        age::paint_person(
            &mut tinted,
            x,
            y,
            person.height,
            &person.figure,
            person.drawing.as_ref(),
            person.stance,
            person.mood,
            person.pose,
        );
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
fn street_lamps(frame: &Frame) -> Vec<f32> {
    frame
        .look
        .as_ref()
        .map(|look| look.lamps.iter().map(|at| at * frame.view_w).collect())
        .unwrap_or_default()
}

/// What a street's lamps glow: the Pack's own colour, or sodium orange.
fn street_glow(frame: &Frame) -> Hsla {
    match frame.look.as_ref().and_then(|look| look.glow) {
        Some(glow) => art::hex(glow),
        None => art::hex(0xffc96b),
    }
}
