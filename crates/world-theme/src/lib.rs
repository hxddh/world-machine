//! Light and dark appearance for the desktop renderers.
//!
//! Every renderer was written against one light palette of literal colours.
//! Rather than maintain a second palette by hand, `adapt` derives the dark
//! counterpart of any light colour: hue and saturation stay, lightness is
//! remapped so backgrounds become dark, cards sit slightly above the window
//! background, borders stay visible, and text and accents become light. The
//! window root sets the mode from the macOS appearance before it renders.

#![forbid(unsafe_code)]

pub mod tokens;

use std::cell::Cell;

thread_local! {
    /// The appearance windows render in, on the thread that renders them
    /// (GPUI's main thread): each window's root sets it before it renders.
    /// Kept per thread so that tests drawing windows side by side, one of
    /// them dark, never see each other's.
    static DARK: Cell<bool> = const { Cell::new(false) };
    /// The window's own appearance, whatever the hour: what its reading
    /// surfaces keep (see [`reading`]).
    static OWN_DARK: Cell<bool> = const { Cell::new(false) };
    /// Whether the window is over a night scene.
    static NIGHT: Cell<bool> = const { Cell::new(false) };
    /// Whether light colours are drawn as paper under a lamp.
    static LAMP: Cell<bool> = const { Cell::new(false) };
    /// Whether reading surfaces keep their own light at night (they do;
    /// tests turn it off to tell them from the rest).
    static READING_LIGHT: Cell<bool> = const { Cell::new(true) };
}

/// Records whether windows currently render in the dark appearance.
///
/// `WORLD_MACHINE_APPEARANCE=dark` or `=light` overrides what the system
/// reports, so either appearance can be checked on a machine that only
/// offers one (the Linux preview under Xvfb reports light, always).
pub fn set_dark(dark: bool) {
    set_dark_or_night(dark, false);
}

/// Records the appearance of a window that also goes dark at night (a
/// World's, over its night sky): dark if the system is (or is forced to
/// be), and at `night` whatever it is. What it lays over its scene is
/// drawn so; what the player reads in it keeps the window's own
/// appearance ([`reading`]).
pub fn set_dark_or_night(dark: bool, night: bool) {
    let own = forced_appearance().unwrap_or(dark);
    OWN_DARK.with(|cell| cell.set(own));
    NIGHT.with(|cell| cell.set(night));
    LAMP.with(|cell| cell.set(false));
    DARK.with(|dark_now| dark_now.set(night || own));
}

/// Builds a reading surface (a card, the drawer, a page) in the window's
/// own appearance rather than the night's: at night in the light
/// appearance it is paper under a lamp, warm and a little dimmer than by
/// day, with its words as dark as ever, so it reads as easily as by day
/// without glaring; in the dark appearance it is dark as always.
pub fn reading<T>(build: impl FnOnce() -> T) -> T {
    if !READING_LIGHT.with(Cell::get) {
        return build();
    }
    let (dark, lamp) = (is_dark(), is_lamp());
    let own = OWN_DARK.with(Cell::get);
    DARK.with(|cell| cell.set(own));
    LAMP.with(|cell| cell.set(!own && NIGHT.with(Cell::get)));
    let built = build();
    DARK.with(|cell| cell.set(dark));
    LAMP.with(|cell| cell.set(lamp));
    built
}

/// Whether reading surfaces keep their own light at night (on unless a
/// test turns it off, to draw them as the rest of the night's interface).
pub fn set_reading_light(on: bool) {
    READING_LIGHT.with(|cell| cell.set(on));
}

/// Whether light colours are drawn now as paper under a lamp: while a
/// reading surface is built at night in the light appearance.
pub fn is_lamp() -> bool {
    LAMP.with(Cell::get)
}

/// What lamplight does to a colour: each channel dimmed a little, blue the
/// most, so white paper turns the warm cream of a page under a lamp and
/// ink stays ink.
pub const LAMPLIGHT: [f32; 3] = [0.9, 0.86, 0.78];

/// A light-palette colour as it looks under a lamp.
pub fn lamplit(hex: u32) -> u32 {
    let channel = |shift: u32, by: f32| {
        let value = ((hex >> shift) & 0xff) as f32 * by;
        (value.round() as u32).min(255)
    };
    (channel(16, LAMPLIGHT[0]) << 16) | (channel(8, LAMPLIGHT[1]) << 8) | channel(0, LAMPLIGHT[2])
}

fn forced_appearance() -> Option<bool> {
    static FORCED: std::sync::OnceLock<Option<bool>> = std::sync::OnceLock::new();
    *FORCED.get_or_init(
        || match std::env::var("WORLD_MACHINE_APPEARANCE").ok()?.as_str() {
            "dark" => Some(true),
            "light" => Some(false),
            _ => None,
        },
    )
}

pub fn is_dark() -> bool {
    DARK.with(Cell::get)
}

/// The colour to draw for a light-palette `0xRRGGBB` in the current mode.
pub fn adapt(hex: u32) -> u32 {
    if is_dark() {
        darken(hex)
    } else if is_lamp() {
        lamplit(hex)
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

    /// Relative luminance, as WCAG has it.
    fn luminance(hex: u32) -> f32 {
        let linear = |shift: u32| {
            let c = ((hex >> shift) & 0xff) as f32 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(16) + 0.7152 * linear(8) + 0.0722 * linear(0)
    }

    fn contrast(a: u32, b: u32) -> f32 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// At night a World window's scene and what hangs over it go dark,
    /// while its cards and drawer keep the player's own appearance: in the
    /// light one, paper under a lamp with its words as easy to read as by
    /// day; in the dark one, dark as ever.
    #[test]
    fn reading_surfaces_keep_their_own_light_at_night() {
        set_dark_or_night(false, true);
        assert!(is_dark(), "the night's interface is dark");
        let paper = reading(|| tokens::SURFACE.hex());
        let ink = reading(|| tokens::TEXT.hex());
        let soft = reading(|| tokens::TEXT_SECONDARY.hex());
        assert!(is_dark() && !is_lamp(), "and dark again after");
        let (r, g, b) = ((paper >> 16) & 0xff, (paper >> 8) & 0xff, paper & 0xff);
        assert!(r > g && g > b, "warm paper: {paper:06x}");
        assert!(
            luminance(paper) < luminance(tokens::SURFACE.light),
            "dimmer than day"
        );
        assert!(contrast(paper, ink) >= 7.0, "{:.1}", contrast(paper, ink));
        assert!(contrast(paper, soft) >= 4.5, "{:.1}", contrast(paper, soft));

        // In the dark appearance it is dark, day or night.
        set_dark_or_night(true, true);
        assert_eq!(reading(|| tokens::SURFACE.hex()), tokens::SURFACE.dark);
        // By day, nothing changes.
        set_dark_or_night(false, false);
        assert_eq!(reading(|| tokens::SURFACE.hex()), tokens::SURFACE.light);
        set_dark(false);
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
