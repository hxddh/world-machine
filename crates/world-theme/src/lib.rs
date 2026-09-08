//! Light and dark appearance for the desktop renderers.
//!
//! Every renderer was written against one light palette of literal colours.
//! Rather than maintain a second palette by hand, `adapt` derives the dark
//! counterpart of any light colour: hue and saturation stay, lightness is
//! remapped so backgrounds become dark, cards sit slightly above the window
//! background, borders stay visible, and text and accents become light. The
//! window root sets the mode from the macOS appearance before it renders.

use std::sync::atomic::{AtomicBool, Ordering};

static DARK: AtomicBool = AtomicBool::new(false);

/// Records whether windows currently render in the dark appearance.
pub fn set_dark(dark: bool) {
    DARK.store(dark, Ordering::Relaxed);
}

pub fn is_dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

/// The colour to draw for a light-palette `0xRRGGBB` in the current mode.
pub fn adapt(hex: u32) -> u32 {
    if is_dark() {
        darken(hex)
    } else {
        hex
    }
}

/// The dark counterpart of a light-palette colour, independent of the mode.
pub fn darken(hex: u32) -> u32 {
    let (h, s, l) = to_hsl(hex);
    let lightness = if l > 0.85 {
        // Window and card backgrounds: white ends up slightly above the
        // off-white window background so cards still read as raised.
        0.10 + (l - 0.85) / 0.15 * 0.08
    } else if l > 0.6 {
        // Borders and light tints: dark greys that stay visible on a dark card.
        0.24 + (0.85 - l) / 0.25 * 0.14
    } else {
        // Text and accents: invert, never darker than mid-grey on a dark ground.
        (1.0 - l).max(0.55)
    };
    from_hsl(h, s, lightness)
}

fn to_hsl(hex: u32) -> (f32, f32, f32) {
    let r = ((hex >> 16) & 0xff) as f32 / 255.0;
    let g = ((hex >> 8) & 0xff) as f32 / 255.0;
    let b = (hex & 0xff) as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } / 6.0;
    (h, s, l)
}

fn from_hsl(h: f32, s: f32, l: f32) -> u32 {
    let channel = |t: f32| -> f32 {
        let t = if t < 0.0 {
            t + 1.0
        } else if t > 1.0 {
            t - 1.0
        } else {
            t
        };
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    let (r, g, b) = if s.abs() < f32::EPSILON {
        (l, l, l)
    } else {
        (channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0))
    };
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    (byte(r) << 16) | (byte(g) << 8) | byte(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lightness(hex: u32) -> f32 {
        to_hsl(hex).2
    }

    #[test]
    fn light_mode_returns_colours_unchanged() {
        set_dark(false);
        assert_eq!(adapt(0xf7f7f3), 0xf7f7f3);
        assert_eq!(adapt(0x202020), 0x202020);
    }

    #[test]
    fn backgrounds_become_dark_and_cards_stay_above_the_window() {
        let window = darken(0xf7f7f3);
        let card = darken(0xffffff);
        assert!(lightness(window) < 0.2, "{window:06x}");
        assert!(
            lightness(card) > lightness(window),
            "{card:06x} vs {window:06x}"
        );
    }

    #[test]
    fn text_becomes_light_and_borders_stay_visible() {
        assert!(lightness(darken(0x202020)) > 0.8);
        assert!(lightness(darken(0x666666)) >= 0.55);
        let border = darken(0xd9d9d3);
        assert!(
            lightness(border) > lightness(darken(0xffffff)),
            "{border:06x}"
        );
        assert!(lightness(border) < 0.4);
    }

    #[test]
    fn tinted_colours_keep_their_hue() {
        let (light_h, _, _) = to_hsl(0xf1f5fb);
        let (dark_h, _, dark_l) = to_hsl(darken(0xf1f5fb));
        assert!((light_h - dark_h).abs() < 0.02);
        assert!(dark_l < 0.2);
        let (accent_h, _, _) = to_hsl(0x9b4a42);
        let (dark_accent_h, _, dark_accent_l) = to_hsl(darken(0x9b4a42));
        assert!((accent_h - dark_accent_h).abs() < 0.02);
        assert!(dark_accent_l >= 0.55);
    }

    #[test]
    fn hsl_round_trips() {
        for hex in [0x000000, 0xffffff, 0x4e6fb3, 0xf1f5fb, 0x9b4a42, 0x123456] {
            let (h, s, l) = to_hsl(hex);
            let back = from_hsl(h, s, l);
            let diff =
                |a: u32, b: u32, shift: u32| ((a >> shift) & 0xff).abs_diff((b >> shift) & 0xff);
            assert!(
                diff(hex, back, 16) <= 1 && diff(hex, back, 8) <= 1 && diff(hex, back, 0) <= 1,
                "{hex:06x} -> {back:06x}"
            );
        }
    }
}
