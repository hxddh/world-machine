//! The light: the hour's grade and colour, the sky and its clouds, the
//! weather, the fog, the haze and the vignette.

use super::*;

/// The light an hour lays over the whole scene, like a diorama under a
/// lamp: a warm key light from the upper left and a cool shade low down,
/// each a colour and how strong it is.
pub fn grade(daylight: Daylight) -> ((u32, f32), (u32, f32)) {
    match daylight {
        Daylight::Dawn => ((0xffc79a, 0.16), (0x5a6aa8, 0.10)),
        Daylight::Day => ((0xfff0c8, 0.10), (0x4a6a9a, 0.08)),
        Daylight::Dusk => ((0xff9a5c, 0.20), (0x4a3070, 0.16)),
        Daylight::Night => ((0x9ab0ff, 0.06), (0x0a1030, 0.22)),
    }
}

/// Where each part of the day's light is at its fullest.
pub(super) const ANCHORS: [(f32, Daylight); 7] = [
    (0.0, Daylight::Night),
    (4.5, Daylight::Night),
    (6.5, Daylight::Dawn),
    (12.5, Daylight::Day),
    (19.0, Daylight::Dusk),
    (21.5, Daylight::Night),
    (24.0, Daylight::Night),
];

/// Which two parts of the day an hour lies between, and how far from the
/// first to the second (eased).
pub(crate) fn between(hour: f32) -> (Daylight, Daylight, f32) {
    let hour = hour.rem_euclid(24.0);
    let next = ANCHORS
        .iter()
        .position(|(at, _)| *at > hour)
        .unwrap_or(ANCHORS.len() - 1)
        .max(1);
    let (from_at, from) = ANCHORS[next - 1];
    let (to_at, to) = ANCHORS[next];
    let t = ((hour - from_at) / (to_at - from_at).max(0.01)).clamp(0.0, 1.0);
    (from, to, ease(t))
}

/// The light an hour lays over the scene, eased from one part of the day
/// into the next so no two hours look alike: the warm key light and the
/// cool shade of [`grade`], each a colour and how strong it is.
pub fn grade_at(hour: f32) -> ((u32, f32), (u32, f32)) {
    let hour = hour.rem_euclid(24.0);
    // `between` has already eased `t`: ease(t).
    let (from, to, t) = between(hour);
    let mix = |a: (u32, f32), b: (u32, f32), deep: f32| {
        let channel = |shift: u32| {
            let x = ((a.0 >> shift) & 0xff) as f32;
            let y = ((b.0 >> shift) & 0xff) as f32;
            ((x + (y - x) * t).round() as u32).min(255) << shift
        };
        (
            channel(16) | channel(8) | channel(0),
            a.1 + (b.1 - a.1) * t + deep,
        )
    };
    let (warm_a, cool_a) = grade(from);
    let (warm_b, cool_b) = grade(to);
    // Through the night the dark deepens towards the small hours and the
    // shade slowly turns from navy to the indigo before dawn.
    let (night_deep, night_turn) = if from == Daylight::Night && to == Daylight::Night {
        let into = if hour >= 21.5 {
            hour - 21.5
        } else {
            hour + 2.5
        } / 7.0;
        (0.05 * (1.0 - (into * 2.0 - 1.0).abs()), into)
    } else {
        (0.0, 0.0)
    };
    let (cool, cool_alpha) = mix(cool_a, cool_b, night_deep);
    let cool = if night_turn > 0.0 {
        let blue = (cool & 0xff) as f32 + 40.0 * night_turn;
        let red = ((cool >> 16) & 0xff) as f32 + 24.0 * night_turn;
        (cool & 0x00ff00) | ((red.min(255.0) as u32) << 16) | blue.min(255.0) as u32
    } else {
        cool
    };
    (mix(warm_a, warm_b, 0.0), (cool, cool_alpha))
}

/// How the hour and the weather colour everything lit by them, as a
/// multiplier per channel (red, green, blue): white at noon, gold at dusk,
/// moonlit blue at night, greyer under rain.
pub fn light_at(hour: f32, weather: Weather) -> [f32; 3] {
    let of = |daylight: Daylight| match daylight {
        Daylight::Dawn => [1.0, 0.9, 0.86],
        Daylight::Day => [1.0, 0.995, 0.97],
        Daylight::Dusk => [1.0, 0.85, 0.74],
        Daylight::Night => [0.34, 0.4, 0.6],
    };
    // `between` has already eased `t`: ease(t).
    let (from, to, t) = between(hour);
    let (a, b) = (of(from), of(to));
    let sky = [0, 1, 2].map(|channel| a[channel] + (b[channel] - a[channel]) * t);
    let air = match weather {
        Weather::Clear => [1.0, 1.0, 1.0],
        Weather::Cloudy => [0.93, 0.94, 0.96],
        Weather::Rain => [0.8, 0.83, 0.88],
        Weather::Storm => [0.6, 0.64, 0.72],
        Weather::Snow => [0.95, 0.97, 1.0],
        Weather::Fog => [0.94, 0.95, 0.96],
        Weather::Dust => [1.0, 0.84, 0.7],
    };
    [0, 1, 2].map(|channel| sky[channel] * air[channel])
}

/// Where the light comes from at `hour`: across (-1 from the left, the
/// morning's east, to 1 from the right) and how high the sun is (0 on the
/// horizon to 1 overhead; below 0 it has set).
pub fn sun_at(hour: f32) -> (f32, f32) {
    let day = ((hour - 5.5) / 14.5).clamp(-0.2, 1.2);
    let across = (day * 2.0 - 1.0).clamp(-1.0, 1.0);
    let high = (day * std::f32::consts::PI).sin();
    (across, high)
}

/// The colour of a window lit from inside.
pub(super) const LAMPLIGHT: (u8, u8, u8) = (0xff, 0xd2, 0x7a);

/// A colour between `a` and `b`, `t` of the way, mixed as light is.
pub(crate) fn mix(a: Hsla, b: Hsla, share: f32) -> Hsla {
    let (a, b): (gpui::Rgba, gpui::Rgba) = (a.into(), b.into());
    let share = share.clamp(0.0, 1.0);
    gpui::Rgba {
        r: a.r + (b.r - a.r) * share,
        g: a.g + (b.g - a.g) * share,
        b: a.b + (b.b - a.b) * share,
        a: a.a + (b.a - a.a) * share,
    }
    .into()
}

/// Whether the sun (or the moon) can be seen for the weather.
pub(super) fn sun_out(weather: Weather) -> bool {
    matches!(weather, Weather::Clear | Weather::Cloudy)
}

/// The vignette, cheap enough to paint right away.
pub(super) fn paint_vignette_image(window: &mut Window, bounds: Bounds<Pixels>) {
    let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let dpr = window.scale_factor().max(0.5);
    let mut key = Key::new("vignette");
    key.float(width).float(height).float(dpr);
    if let Some(vignette) = painter::cached(window, key.finish(), || {
        paint_vignette(width, height, dpr * 0.25)
    }) {
        let _ = window.paint_image(bounds, bounds, Corners::default(), vignette, 0, false);
    }
}

/// The air between one row of a folded postcard and the next nearer one:
/// a veil of the sky's own haze over the row behind, thickening from its
/// roofs down to its street, as far away things fade.
pub(super) fn paint_haze(
    window: &mut dyn Brush,
    frame: &Frame,
    ox: f32,
    oy: f32,
    span: (f32, f32),
    height: f32,
) {
    let light = light_at(frame.hour, frame.weather);
    let (_, bottom) = sky_colours(frame);
    let rgba: gpui::Rgba = bottom.into();
    let haze = Hsla::from(gpui::Rgba {
        r: rgba.r * light[0],
        g: rgba.g * light[1],
        b: rgba.b * light[2],
        a: 1.0,
    });
    let roofs = frame
        .at(0.0, frame.base - frame.building_h * row_at(0.30).1)
        .1;
    let street = frame.at(0.0, frame.front).1;
    let veil = 0.22 * frame.camera.fold.clamp(0.0, 1.0);
    let (x, w) = (ox + span.0, (span.1 - span.0).max(0.0));
    window.gradient(
        x,
        oy + roofs,
        w,
        (street - roofs).max(1.0),
        180.0,
        (haze.opacity(0.0), 0.0),
        (haze.opacity(veil), 1.0),
    );
    if street < height {
        window.rect(x, oy + street, w, height - street, 0.0, haze.opacity(veil));
    }
}

/// The sky, graded for the hour and the weather, with the sun where the
/// hour has it, or the moon and stars.
#[cfg(test)]
pub(super) fn paint_sky(frame: &Frame, width: f32, height: f32, scale: f32) -> Option<sk::Pixmap> {
    painter::in_strips(
        (width * scale).ceil() as u32,
        (height * scale).ceil() as u32,
        scale,
        (0.0, 0.0),
        &|canvas, at| painter::timed("sky", || paint_sky_on(canvas, at, frame, width, height)),
    )
}

/// The sky onto one strip of it.
pub(super) fn paint_sky_on(
    canvas: &mut Canvas,
    at: (i32, i32),
    frame: &Frame,
    width: f32,
    height: f32,
) {
    let (top, bottom) = sky_colours(frame);
    let horizon = frame.height * HORIZON;
    painter::fill_shaded(
        canvas,
        &Shape::polygon(&[
            (-16.0, -16.0),
            (width + 16.0, -16.0),
            (width + 16.0, height + 16.0),
            (-16.0, height + 16.0),
        ]),
        (0.0, 0.0),
        (0.0, horizon * 1.2),
        &[(0.0, top), (1.0, bottom)],
    );
    let (across, high) = sun_at(frame.hour);
    let night = frame.daylight == Daylight::Night;
    let k = (height / 848.0).clamp(0.3, 1.3);
    if sun_out(frame.weather) {
        if night {
            // Stars, fewer toward the horizon, and the moon.
            for index in 0..90_i32 {
                let seed = painter::hash2(index, 7, 0x5eed);
                let x = (seed % 1000) as f32 / 1000.0 * width;
                let y = ((seed / 1000) % 1000) as f32 / 1000.0;
                let y = y * y * horizon * 0.95;
                let bright = 0.35 + ((seed >> 20) % 100) as f32 / 160.0;
                let r = (1.1 * k).max(0.55 / canvas.scale);
                art::circle(canvas, x, y, r, gpui::white().opacity(bright));
            }
            let (mx, my) = (width * 0.8, horizon * 0.34);
            painter::glow(
                canvas,
                mx,
                my,
                90.0 * k,
                90.0 * k,
                art::hex(0x8a9ad0).opacity(0.35),
            );
            canvas.soft(mx, my, 22.0 * k, 22.0 * k, 2.0, art::hex(0xf4f1e6));
            canvas.soft(
                mx + 8.0 * k,
                my - 5.0 * k,
                20.0 * k,
                20.0 * k,
                2.0,
                top.opacity(0.92),
            );
        } else if high > -0.08 {
            let sun = art::hex(frame.scenery.sun);
            let (sx, sy) = (
                width * (0.5 + 0.38 * across),
                horizon * (0.9 - 0.62 * high.max(0.0)),
            );
            // The lower the sun, the warmer and wider its light on the sky.
            let low = (1.0 - high.max(0.0)).powi(2);
            let dim = if frame.weather == Weather::Cloudy {
                0.55
            } else {
                1.0
            };
            let warm = mix(sun, art::hex(0xff9a5c), low * 0.7);
            painter::glow(
                canvas,
                sx,
                sy,
                width * (0.25 + 0.3 * low),
                horizon * (0.5 + 0.4 * low),
                warm.opacity((0.18 + 0.32 * low) * dim),
            );
            canvas.soft(sx, sy, 58.0 * k, 58.0 * k, 40.0 * k, sun.opacity(0.2 * dim));
            canvas.soft(sx, sy, 34.0 * k, 34.0 * k, 3.0, sun.opacity(dim));
        }
    }
    // Under weather the sky greys, or reddens in dust.
    if let Some((tint, alpha)) = overcast(frame.weather) {
        canvas.rect(
            -16.0,
            -16.0,
            width + 32.0,
            height + 32.0,
            0.0,
            art::hex(tint).opacity(alpha),
        );
    }
    painter::grain(canvas, at, 0.03, 0.022);
}

/// The sky's colours at its top and at the horizon, for the hour.
pub(super) fn sky_colours(frame: &Frame) -> (Hsla, Hsla) {
    let top = art::hex(frame.scenery.sky_top);
    let bottom = art::hex(frame.scenery.sky_bottom);
    let tinted = |daylight: Daylight| {
        let (tint_top, tint_bottom) = match daylight {
            Daylight::Day => ((0xffffff, 0.0), (0xffffff, 0.0)),
            Daylight::Dawn => ((0xffbaa0, 0.20), (0xffe4c8, 0.14)),
            Daylight::Dusk => ((0x7a5a9a, 0.34), (0xffa060, 0.5)),
            Daylight::Night => ((0x0e1436, 0.78), (0x1c2450, 0.64)),
        };
        (
            mix(top, art::hex(tint_top.0), tint_top.1),
            mix(bottom, art::hex(tint_bottom.0), tint_bottom.1),
        )
    };
    let (from, to, t) = between(frame.hour);
    let (a, b) = (tinted(from), tinted(to));
    (mix(a.0, b.0, t), mix(a.1, b.1, t))
}

/// What the weather lays over the sky: grey under rain, slate in a storm,
/// pale before snow, rust in a dust storm.
pub(crate) fn overcast(weather: Weather) -> Option<(u32, f32)> {
    match weather {
        Weather::Clear => None,
        Weather::Cloudy => Some((0x9aa4ad, 0.22)),
        Weather::Rain => Some((0x6f7a86, 0.42)),
        Weather::Storm => Some((0x2f3844, 0.62)),
        Weather::Snow => Some((0xe8edf2, 0.35)),
        Weather::Fog => Some((0xd8dde0, 0.4)),
        Weather::Dust => Some((0xb8643a, 0.45)),
    }
}

/// The vignette: nothing in the middle, a very slight dark at the corners.
pub(super) fn paint_vignette(width: f32, height: f32, scale: f32) -> Option<sk::Pixmap> {
    let mut canvas = Canvas::new(
        (width * scale).ceil().max(2.0) as u32,
        (height * scale).ceil().max(2.0) as u32,
        scale,
        (0.0, 0.0),
    )?;
    painter::vignette(&mut canvas, width, height, art::hex(0x24180f).opacity(0.2));
    Some(canvas.pixmap)
}

/// A brush that colours everything it draws by the hour's light, so what
/// moves is lit like the still layers under it.
pub(super) struct Tint<'a> {
    pub(super) inner: &'a mut dyn Brush,
    pub(super) light: [f32; 3],
}

impl<'a> Tint<'a> {
    pub(super) fn new(inner: &'a mut dyn Brush, light: [f32; 3]) -> Self {
        Self { inner, light }
    }

    pub(super) fn lit(&self, colour: Hsla) -> Hsla {
        let rgba: gpui::Rgba = colour.into();
        gpui::Rgba {
            r: rgba.r * self.light[0],
            g: rgba.g * self.light[1],
            b: rgba.b * self.light[2],
            a: rgba.a,
        }
        .into()
    }
}

impl Brush for Tint<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Hsla) {
        let colour = self.lit(colour);
        self.inner.rect(x, y, w, h, radius, colour);
    }
    fn fill(&mut self, shape: &Shape, colour: Hsla) {
        let colour = self.lit(colour);
        self.inner.fill(shape, colour);
    }
    fn stroke(&mut self, shape: &Shape, width: f32, colour: Hsla) {
        let colour = self.lit(colour);
        self.inner.stroke(shape, width, colour);
    }
    fn soft(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, blur: f32, colour: Hsla) {
        // Shadows are the absence of light, and glows their own light.
        self.inner.soft(cx, cy, rx, ry, blur, colour);
    }
    fn gradient(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        angle: f32,
        from: (Hsla, f32),
        to: (Hsla, f32),
    ) {
        let (from, to) = ((self.lit(from.0), from.1), (self.lit(to.0), to.1));
        self.inner.gradient(x, y, w, h, angle, from, to);
    }
    fn picture(
        &mut self,
        image: &std::sync::Arc<gpui::RenderImage>,
        rect: crate::brush::Rect,
        clip: crate::brush::Rect,
    ) {
        // A picture is painted in the hour's light already.
        self.inner.picture(image, rect, clip);
    }
}

/// Clouds drifting across, slowly, each at its own pace, soft-edged; more
/// of them, and greyer, the worse the weather; gulls wheeling on a fair
/// day.
pub(super) fn paint_clouds(
    window: &mut dyn Brush,
    frame: &Frame,
    ox: f32,
    oy: f32,
    width: f32,
    height: f32,
) {
    let t = frame.seconds;
    let night = frame.daylight == Daylight::Night;
    let k = (height / 848.0).clamp(0.3, 1.3);
    let horizon = frame.at(0.0, frame.horizon).1;
    let sky = height * HORIZON;
    let (clouds, cloud) = match (frame.daylight, frame.weather) {
        (Daylight::Night, _) => (4, art::hex(0x9aa4c8).opacity(0.14)),
        (_, Weather::Clear) => (4, gpui::white().opacity(0.78)),
        (_, Weather::Cloudy) => (7, art::hex(0xe4e8ec).opacity(0.88)),
        (_, Weather::Rain | Weather::Snow) => (8, art::hex(0xc4cad0).opacity(0.9)),
        (_, Weather::Storm) => (9, art::hex(0x5a6470).opacity(0.95)),
        (_, Weather::Fog) => (6, gpui::white().opacity(0.6)),
        (_, Weather::Dust) => (6, art::hex(0xd99a6c).opacity(0.7)),
    };
    // Lit from below by a low sun.
    let cloud = match frame.daylight {
        Daylight::Dusk if sun_out(frame.weather) => mix(cloud, art::hex(0xffc4a8), 0.45),
        Daylight::Dawn if sun_out(frame.weather) => mix(cloud, art::hex(0xffe0cc), 0.35),
        _ => cloud,
    };
    let cloud = crate::setting::cloud_ink(frame.setting, cloud, art::hex(frame.scenery.far));
    let _ = horizon;
    for index in 0..clouds {
        let speed = 4.0 + index as f32 * 1.7;
        let span = width + 320.0;
        let x = ox
            + ((index as f32 * 331.0 + t * speed - frame.pan() * PARALLAX[0]).rem_euclid(span))
            - 160.0;
        let y = oy + sky * (0.12 + 0.11 * (index % 5) as f32);
        let s = (1.0 - (index % 5) as f32 * 0.12) * k;
        window.soft(x, y, 50.0 * s, 17.0 * s, 16.0 * s, cloud);
        window.soft(
            x + 30.0 * s,
            y - 9.0 * s,
            34.0 * s,
            17.0 * s,
            14.0 * s,
            cloud,
        );
        window.soft(
            x - 30.0 * s,
            y + 2.0 * s,
            28.0 * s,
            12.0 * s,
            12.0 * s,
            cloud,
        );
    }
    if !night && sun_out(frame.weather) && frame.setting.has_gulls() {
        for gull in 0..3 {
            let span = width + 200.0;
            let x = ox + ((gull as f32 * 417.0 + t * (18.0 + gull as f32 * 5.0)) % span) - 100.0;
            let y = oy + sky * (0.3 + 0.08 * gull as f32) + (t * 1.7 + gull as f32).sin() * 6.0;
            let flap = 3.0 + 2.0 * (t * 6.0 + gull as f32 * 2.0).sin();
            let mut wings = Shape::new();
            wings
                .move_to(x - 7.0 * k, y - flap * k)
                .line_to(x, y)
                .line_to(x + 7.0 * k, y - flap * k);
            window.stroke(&wings, 1.6, art::hex(0x3c4048).opacity(0.7));
        }
    }
}

/// Which way the wind blows smoke, and how hard: from the place's own seed
/// and the weather, turning slowly.
pub(super) fn wind(frame: &Frame) -> f32 {
    let seed = seed_of_scenery(&frame.scenery);
    let from = if seed.is_multiple_of(2) { 1.0 } else { -1.0 };
    let strength = match frame.weather {
        Weather::Storm => 3.0,
        Weather::Rain | Weather::Snow | Weather::Dust => 1.8,
        Weather::Cloudy => 1.2,
        _ => 0.8,
    };
    from * strength * (0.75 + 0.25 * (frame.seconds * 0.05).sin())
}

/// The light a quilt is seen by through a window at night: the lamp's.
pub(super) const LAMP: [f32; 3] = [1.0, 0.84, 0.6];

/// Rain, snow, dust and fog over the whole scene, and lightning in a storm.
pub(super) fn paint_weather(
    window: &mut dyn Brush,
    frame: &Frame,
    ox: f32,
    oy: f32,
    width: f32,
    height: f32,
    k: f32,
) {
    let t = frame.seconds;
    // Each drop or flake has its own place in a repeating fall.
    let fall = |index: u32, speed: f32, drift: f32| {
        let mut seed = index.wrapping_mul(0x9e37_79b9) ^ 0x85eb_ca6b;
        seed ^= seed >> 15;
        let x0 = (seed % 1000) as f32 / 1000.0;
        let phase = ((seed / 1000) % 1000) as f32 / 1000.0;
        let y = ((phase + t * speed / height.max(1.0)) % 1.0) * (height + 40.0) - 20.0;
        let x = (x0 * (width + 120.0) + y * drift) % (width + 120.0) - 60.0;
        (ox + x, oy + y)
    };
    match frame.weather {
        Weather::Rain | Weather::Storm => {
            let storm = frame.weather == Weather::Storm;
            let (count, speed, slant) = if storm {
                (220, 900.0, 0.35)
            } else {
                (120, 620.0, 0.12)
            };
            let ink = art::hex(0xdfe7ef).opacity(if storm { 0.55 } else { 0.45 });
            for index in 0..count {
                let (x, y) = fall(index, speed, slant);
                let len = (12.0 + (index % 5) as f32 * 2.0) * k;
                let mut streak = Shape::new();
                streak.move_to(x, y).line_to(x + len * slant, y + len);
                window.stroke(&streak, 1.0, ink);
            }
            // Now and then the sky lights up.
            if storm && !frame.still {
                let beat = t % 7.3;
                if beat < 0.12 || (0.2..0.26).contains(&beat) {
                    window.rect(ox, oy, width, height, 0.0, gpui::white().opacity(0.35));
                }
            }
        }
        Weather::Snow => {
            for index in 0..110 {
                let (x, y) = fall(index, 45.0 + (index % 7) as f32 * 6.0, 0.05);
                let sway = (t * 0.9 + index as f32).sin() * 6.0;
                art::circle(
                    window,
                    x + sway,
                    y,
                    (1.4 + (index % 3) as f32 * 0.7) * k,
                    gpui::white().opacity(0.85),
                );
            }
        }
        Weather::Dust => {
            for index in 0..140_u32 {
                let mut seed = index.wrapping_mul(0x2545_f491) ^ 0x68e3_1da4;
                seed ^= seed >> 13;
                let y = oy + ((seed % 1000) as f32 / 1000.0) * height;
                let x = ox
                    + ((((seed / 1000) % 1000) as f32 / 1000.0) * (width + 80.0)
                        + t * (60.0 + (index % 9) as f32 * 12.0))
                        % (width + 80.0)
                    - 40.0;
                art::circle(window, x, y, 1.2 * k, art::hex(0xe0a070).opacity(0.55));
            }
            let top = frame.at(0.0, frame.horizon).1.max(0.0);
            window.gradient(
                ox,
                oy + top,
                width,
                height - top,
                180.0,
                (art::hex(0xc07040).opacity(0.0), 0.0),
                (art::hex(0xc07040).opacity(0.18), 0.2),
            );
        }
        Weather::Fog => {
            for layer in fog_layers(frame.at(0.0, frame.horizon).1, width, height, t) {
                window.gradient(
                    ox + layer.x,
                    oy + layer.y,
                    layer.w,
                    layer.h,
                    180.0,
                    (gpui::white().opacity(layer.top), 0.0),
                    (gpui::white().opacity(layer.bottom), 1.0),
                );
            }
        }
        Weather::Clear | Weather::Cloudy => {}
    }
}

/// A layer of fog: where it lies on the scene, and how thick it is at its
/// top and bottom edges.
#[derive(Clone, Copy, Debug)]
pub(super) struct FogLayer {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) w: f32,
    pub(super) h: f32,
    pub(super) top: f32,
    pub(super) bottom: f32,
}

/// Fog for a scene: a veil thickening towards the ground and soft wisps
/// drifting across it. No layer has a hard edge inside the scene, so
/// nothing reads as a band however close the camera is.
pub(super) fn fog_layers(horizon: f32, width: f32, height: f32, t: f32) -> Vec<FogLayer> {
    let top = horizon.clamp(0.0, height);
    let deep = (height - top).max(1.0);
    let mut layers = vec![FogLayer {
        x: 0.0,
        y: top,
        w: width,
        h: deep,
        top: if top > 0.0 { 0.0 } else { 0.1 },
        bottom: 0.26,
    }];
    for wisp in 0..3 {
        let centre = top + deep * (0.25 + 0.3 * wisp as f32);
        let half = deep * 0.09;
        let drift = (t * (4.0 + wisp as f32)) % 60.0;
        layers.push(FogLayer {
            x: -60.0 + drift,
            y: centre - half,
            w: width + 120.0,
            h: half,
            top: 0.0,
            bottom: 0.14,
        });
        layers.push(FogLayer {
            x: -60.0 + drift,
            y: centre,
            w: width + 120.0,
            h: half,
            top: 0.14,
            bottom: 0.0,
        });
    }
    layers
}
