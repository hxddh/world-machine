//! The named colours every new surface is drawn with.
//!
//! `adapt` exists because every renderer used to be written against literal
//! light colours, and deriving their dark counterparts by formula is the best
//! that can be done for a palette nobody chose. A token is the alternative: a
//! role ("secondary text", "raised card") with a light and a dark value picked
//! together, so the two appearances are designed rather than computed, and a
//! colour changes in one place instead of in every file that happened to
//! spell it out.

use crate::is_dark;

/// One colour role with its light and dark value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Token {
    pub light: u32,
    pub dark: u32,
}

impl Token {
    pub const fn new(light: u32, dark: u32) -> Self {
        Self { light, dark }
    }

    /// The `0xRRGGBB` value for the current appearance.
    pub fn hex(self) -> u32 {
        if is_dark() {
            self.dark
        } else {
            self.light
        }
    }
}

/// The window behind everything.
pub const WINDOW: Token = Token::new(0xf6f5f1, 0x1b1b1d);
/// A card or panel raised above the window.
pub const SURFACE: Token = Token::new(0xffffff, 0x242427);
/// A card under the pointer.
pub const SURFACE_HOVER: Token = Token::new(0xfaf9f6, 0x2b2b2f);
/// Side panels, set a step back from the reading column.
pub const SIDEBAR: Token = Token::new(0xefede8, 0x202023);
/// A row under the pointer on the window or sidebar.
pub const ROW_HOVER: Token = Token::new(0xe7e5df, 0x2c2c30);
/// The row that is currently selected.
pub const ROW_SELECTED: Token = Token::new(0xe2e8f4, 0x2a3346);

/// Hairlines between regions and around cards.
pub const BORDER: Token = Token::new(0xe4e1da, 0x333337);
/// Borders that have to be seen, such as an unselected control.
pub const BORDER_STRONG: Token = Token::new(0xd0ccc3, 0x46464b);

/// Body text and headings.
pub const TEXT: Token = Token::new(0x1d1c1a, 0xedece8);
/// Supporting text under a heading.
pub const TEXT_SECONDARY: Token = Token::new(0x5c5952, 0xb2b0aa);
/// Labels, timestamps, and anything that should recede.
pub const TEXT_TERTIARY: Token = Token::new(0x8b877e, 0x85837d);

/// The one colour that means "you can act here".
pub const ACCENT: Token = Token::new(0x3a5796, 0x7f9ddc);
pub const ACCENT_HOVER: Token = Token::new(0x2f4a84, 0x93aee6);
/// Text drawn on a filled accent control.
pub const ON_ACCENT: Token = Token::new(0xffffff, 0x0f1420);
/// A tinted background for accent-bordered panels.
pub const ACCENT_SOFT: Token = Token::new(0xeef2fa, 0x232c40);
/// Accent-coloured text on the window or a card.
pub const ACCENT_TEXT: Token = Token::new(0x34508c, 0x9db4e8);

pub const SUCCESS: Token = Token::new(0x3d7550, 0x7fc195);
pub const WARNING: Token = Token::new(0xa46a12, 0xe0ad5c);
pub const DANGER: Token = Token::new(0xa93f3a, 0xe58a84);

/// The ground a World's scene is drawn on: a soft sky fading into land.
pub const SCENE_TOP: Token = Token::new(0xeef1f4, 0x1f2328);
pub const SCENE_BOTTOM: Token = Token::new(0xe9ebe2, 0x1c1f1b);
/// The faint dot grid over the scene.
pub const SCENE_GRID: Token = Token::new(0xd6d9d0, 0x2f332e);
/// Lines between things that are connected in the scene.
pub const SCENE_LINK: Token = Token::new(0xa9b3c7, 0x56627a);

/// Background and foreground for a person's avatar, picked from a fixed set
/// of hues by `seed` so the same person always wears the same colour.
pub fn avatar(seed: u64) -> (Token, Token) {
    const PAIRS: [(Token, Token); 8] = [
        (
            Token::new(0xdbe6fb, 0x2b3a5c),
            Token::new(0x2c4a86, 0xc3d4f7),
        ),
        (
            Token::new(0xf8e0d6, 0x5a3327),
            Token::new(0x8a3b22, 0xf3c7b6),
        ),
        (
            Token::new(0xdcf0e2, 0x24452f),
            Token::new(0x2f6b43, 0xbfe6cb),
        ),
        (
            Token::new(0xf3e4f7, 0x4a2f55),
            Token::new(0x6e3a82, 0xe5c7f0),
        ),
        (
            Token::new(0xfbefcf, 0x4f4220),
            Token::new(0x7a5a10, 0xf1dca0),
        ),
        (
            Token::new(0xd8f1f3, 0x21474b),
            Token::new(0x1f6970, 0xb6e5e9),
        ),
        (
            Token::new(0xf8dde6, 0x552735),
            Token::new(0x8c2f4e, 0xf2c1d1),
        ),
        (
            Token::new(0xe6e8ee, 0x353841),
            Token::new(0x454b5c, 0xd3d7e2),
        ),
    ];
    PAIRS[(seed % PAIRS.len() as u64) as usize]
}

/// A stable seed for a name, so colours do not change between launches.
pub fn seed(text: &str) -> u64 {
    // FNV-1a: tiny, dependency-free, and stable across platforms.
    text.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(0x100000001b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(hex: u32) -> f64 {
        let channel = |shift: u32| {
            let c = ((hex >> shift) & 0xff) as f64 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
    }

    fn contrast(a: u32, b: u32) -> f64 {
        let (hi, lo) = {
            let (x, y) = (luminance(a), luminance(b));
            if x > y {
                (x, y)
            } else {
                (y, x)
            }
        };
        (hi + 0.05) / (lo + 0.05)
    }

    fn both(token: Token, ground: Token) -> [f64; 2] {
        [
            contrast(token.light, ground.light),
            contrast(token.dark, ground.dark),
        ]
    }

    #[test]
    fn body_and_supporting_text_meet_wcag_aa_on_every_ground() {
        for ground in [WINDOW, SURFACE, SIDEBAR, ROW_SELECTED, ACCENT_SOFT] {
            for ratio in both(TEXT, ground) {
                assert!(ratio >= 7.0, "{ratio}");
            }
            for ratio in both(TEXT_SECONDARY, ground) {
                assert!(ratio >= 4.5, "{ratio}");
            }
        }
    }

    #[test]
    fn labels_and_accents_stay_legible() {
        for ground in [WINDOW, SURFACE, SIDEBAR] {
            for ratio in both(TEXT_TERTIARY, ground) {
                assert!(ratio >= 3.0, "{ratio}");
            }
            for ratio in both(ACCENT_TEXT, ground) {
                assert!(ratio >= 4.5, "{ratio}");
            }
        }
        for ratio in both(ON_ACCENT, ACCENT) {
            assert!(ratio >= 4.5, "{ratio}");
        }
    }

    #[test]
    fn avatar_initials_are_legible_in_every_hue() {
        for seed in 0..8 {
            let (bg, fg) = avatar(seed);
            for ratio in both(fg, bg) {
                assert!(ratio >= 4.5, "hue {seed}: {ratio}");
            }
        }
    }

    #[test]
    fn a_name_always_gets_the_same_colour() {
        assert_eq!(seed("Nia Chen"), seed("Nia Chen"));
        assert_ne!(seed("Nia Chen"), seed("Tomas Vale"));
    }

    #[test]
    fn cards_are_distinguishable_from_the_window_in_both_appearances() {
        assert_ne!(SURFACE.light, WINDOW.light);
        assert!(luminance(SURFACE.dark) > luminance(WINDOW.dark));
        assert!(luminance(SIDEBAR.light) < luminance(WINDOW.light));
    }
}
