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
    fn cards_are_distinguishable_from_the_window_in_both_appearances() {
        assert_ne!(SURFACE.light, WINDOW.light);
        assert!(luminance(SURFACE.dark) > luminance(WINDOW.dark));
        assert!(luminance(SIDEBAR.light) < luminance(WINDOW.light));
    }
}
