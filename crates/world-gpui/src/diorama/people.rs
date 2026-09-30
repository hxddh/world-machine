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
    stage
        .people
        .iter()
        .map(|spot| {
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
            let still = Living {
                x: home + (seconds * 0.23 + seed as f32).sin() * 3.0,
                pose: Pose {
                    stride: None,
                    bob: breathe * 0.6,
                    facing: ((seconds * 0.11 + (seed % 7) as f32).sin() * 1.4).clamp(-1.0, 1.0),
                    // Breathing: a touch taller on the breath in.
                    squash: 1.0 + 0.008 * breathe,
                    lean: 0.0,
                },
                stance: if pinned.contains(&item.id) {
                    None
                } else {
                    idle(seed, seconds, daylight)
                },
            };
            if pinned.contains(&item.id) || daylight == Daylight::Night || stops.len() < 2 {
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
    let contact = gpui::black().opacity(if night { 0.26 } else { 0.2 });

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
            for streak in 0..3 {
                let along = (streak as f32 - 1.0) * building.w * z * 0.18;
                let pulse = 0.6 + 0.4 * (t * 1.3 + seed + streak as f32 * 2.1).sin();
                let reach = deep * (0.28 + 0.1 * streak as f32);
                window.soft(
                    x + along + (t * 0.9 + seed + streak as f32).sin() * 1.5,
                    front + 6.0 * z + reach / 2.0,
                    building.w * z * 0.025,
                    reach / 2.0,
                    building.w * z * 0.03,
                    warm.opacity(0.13 * pulse),
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
            window.soft(x, base, w * 0.8, w * 0.16, w * 0.12, glow.opacity(0.35));
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
            window.soft(x, base, w * 0.48, w * 0.07, w * 0.06, contact);
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
        let mut facing = Xform::about(
            &mut tinted,
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
    );
    // People: a soft shadow where they stand, a longer one away from a
    // sun that is out, and themselves.
    let long = 0.22 + 0.6 * (1.0 - high.max(0.0));
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
                glow.opacity(0.45),
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
            contact.opacity(contact.a * (1.0 - lift)),
        );
        let mut tinted = Tint::new(window, light);
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
    for (x, y, r, tone) in &frame.bonds {
        art::paint_bond(window, ox + x, oy + y, *r, *tone);
    }
}
