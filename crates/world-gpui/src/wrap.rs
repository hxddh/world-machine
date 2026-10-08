//! Where a line of the World's words may break, and a text element that
//! breaks it there.
//!
//! GPUI's own wrapping breaks before any mark it does not count as part of
//! a word, so a question mark can wrap onto a line of its own ("Mend it
//! now / ?"), Chinese and Japanese lines can start with 。 or 」, and a
//! katakana name breaks mid-name ("ソフィ / ア"). The rules here:
//! - a line breaks at a space, or between two characters where one is
//!   Chinese or Japanese;
//! - never before closing punctuation (`?`, `!`, `)`, `。`, `、`, `」`,
//!   `ー`, small kana…) nor after opening punctuation (`(`, `「`…): kinsoku;
//! - never inside a run of katakana (a name, a loanword) or of Latin
//!   letters and digits;
//! - only if nothing else fits, anywhere but before closing punctuation.
//!
//! [`lines`] works out the lines for any measure of width, and [`text`] is
//! the element that draws them, as a plain text child would be drawn.

use gpui::{
    App, AvailableSpace, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, SharedString, Size, StyledText, WhiteSpace, Window, WrappedLine,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Characters a line may not start with.
fn closing(c: char) -> bool {
    matches!(
        c,
        '?' | '!'
            | ')'
            | ']'
            | '}'
            | ','
            | '.'
            | ':'
            | ';'
            | '…'
            | '‥'
            | '%'
            | '’'
            | '”'
            | '»'
            | '、'
            | '。'
            | '，'
            | '．'
            | '・'
            | '：'
            | '；'
            | '？'
            | '！'
            | 'ー'
            | '〜'
            | '～'
            | '」'
            | '』'
            | '）'
            | '】'
            | '〕'
            | '〉'
            | '》'
            | '｝'
            | '］'
            | '々'
            | 'ゝ'
            | 'ゞ'
            | 'ヽ'
            | 'ヾ'
            | 'ぁ'
            | 'ぃ'
            | 'ぅ'
            | 'ぇ'
            | 'ぉ'
            | 'っ'
            | 'ゃ'
            | 'ゅ'
            | 'ょ'
            | 'ゎ'
            | 'ゕ'
            | 'ゖ'
            | 'ァ'
            | 'ィ'
            | 'ゥ'
            | 'ェ'
            | 'ォ'
            | 'ッ'
            | 'ャ'
            | 'ュ'
            | 'ョ'
            | 'ヮ'
            | 'ヵ'
            | 'ヶ'
            | '\u{31F0}'..='\u{31FF}'
    )
}

/// Characters a line may not end with.
fn opening(c: char) -> bool {
    matches!(
        c,
        '(' | '['
            | '{'
            | '‘'
            | '“'
            | '«'
            | '「'
            | '『'
            | '（'
            | '【'
            | '〔'
            | '〈'
            | '《'
            | '｛'
            | '［'
    )
}

/// Written without spaces between words: Chinese, Japanese.
fn unspaced(c: char) -> bool {
    matches!(
        c,
        '\u{3000}'..='\u{30FF}'
            | '\u{31F0}'..='\u{31FF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{FF00}'..='\u{FFEF}'
    )
}

/// Katakana, with its long-vowel mark and the dot between names.
fn katakana(c: char) -> bool {
    matches!(c, '\u{30A0}'..='\u{30FF}' | '\u{31F0}'..='\u{31FF}' | '\u{FF66}'..='\u{FF9F}')
}

/// Part of a word written with spaces: letters, digits, an apostrophe.
fn lettered(c: char) -> bool {
    !unspaced(c) && (c.is_alphanumeric() || c == '\'' || c == '’' || c == '-')
}

/// A place a line may break: it ends at `at`, and the next starts at
/// `next` (past the space broken at, if any).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Break {
    at: usize,
    next: usize,
}

/// Every place `text` may break, by the rules above, in order.
fn breaks(text: &str) -> Vec<Break> {
    let chars = text.char_indices().collect::<Vec<_>>();
    let mut found = Vec::new();
    for i in 1..chars.len() {
        let (at, c) = chars[i];
        let prev = chars[i - 1].1;
        if c == ' ' {
            // Break at the last space of a run, if what follows may start
            // a line.
            let after = chars.get(i + 1).map(|(_, c)| *c);
            if prev != ' '
                && after.is_some_and(|after| after != ' ' && !closing(after))
                && !opening(prev)
            {
                found.push(Break { at, next: at + 1 });
            }
            continue;
        }
        if prev == ' ' || prev == '\u{00A0}' || c == '\u{00A0}' {
            continue;
        }
        let between = unspaced(prev) || unspaced(c);
        if between
            && !closing(c)
            && !opening(prev)
            && !(katakana(prev) && katakana(c))
            && !(lettered(prev) && lettered(c))
        {
            found.push(Break { at, next: at });
        }
    }
    found
}

/// Where `text` may break if nothing else fits: before any character but
/// closing punctuation.
fn anywhere(text: &str) -> Vec<Break> {
    text.char_indices()
        .skip(1)
        .filter(|(_, c)| !closing(*c) && *c != ' ')
        .map(|(at, _)| Break { at, next: at })
        .collect()
}

/// The lines `text` is broken into, as byte ranges, to fit `width`;
/// `x(i)` is how far along a single unbroken line byte `i` starts.
pub fn lines(text: &str, width: f32, x: impl Fn(usize) -> f32) -> Vec<std::ops::Range<usize>> {
    let breaks = breaks(text);
    let fallback = anywhere(text);
    let mut out = Vec::new();
    let mut start = 0;
    while x(text.len()) - x(start) > width + 0.5 {
        let fits = |b: &&Break| b.at > start && x(b.at) - x(start) <= width + 0.5;
        let chosen = breaks
            .iter()
            .filter(|b| b.at > start)
            .take_while(|b| x(b.at) - x(start) <= width + 0.5)
            .last()
            .or_else(|| fallback.iter().filter(fits).last())
            .or_else(|| fallback.iter().find(|b| b.at > start));
        let Some(chosen) = chosen.copied() else {
            break;
        };
        out.push(start..chosen.at);
        start = chosen.next;
    }
    out.push(start..text.len());
    out
}

/// A text child that breaks its lines by the rules above, drawn in the
/// style it inherits, as a plain text child would be. Text that is cut
/// short or kept to one line or a few (truncated, clamped, not wrapped)
/// is drawn by GPUI's own text, as it always was.
pub fn text(text: impl Into<SharedString>) -> Wrapped {
    Wrapped {
        text: text.into(),
        laid: Rc::default(),
        plain: None,
    }
}

pub struct Wrapped {
    text: SharedString,
    laid: Rc<RefCell<Option<Laid>>>,
    /// GPUI's own text, for text cut short, clamped or not wrapped.
    plain: Option<StyledText>,
}

struct Laid {
    lines: Vec<WrappedLine>,
    line_height: Pixels,
    width: Option<Pixels>,
    size: Size<Pixels>,
}

impl IntoElement for Wrapped {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Wrapped {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = window.text_style();
        if style.text_overflow.is_some()
            || style.line_clamp.is_some()
            || style.white_space != WhiteSpace::Normal
        {
            let plain = self.plain.insert(StyledText::new(self.text.clone()));
            return plain.request_layout(id, inspector, window, cx);
        }
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = window.pixel_snap(
            style
                .line_height
                .to_pixels(font_size.into(), window.rem_size()),
        );
        let text = self.text.clone();
        let laid = self.laid.clone();
        let id = window.request_measured_layout(
            Default::default(),
            move |known, available, window, _| {
                let width = known.width.or(match available.width {
                    AvailableSpace::Definite(width) => Some(width),
                    _ => None,
                });
                if let Some(laid) = laid.borrow().as_ref() {
                    if laid.width == width {
                        return laid.size;
                    }
                }
                let system = window.text_system();
                // Each paragraph on its own: a line already broken stays so.
                let mut broken = String::new();
                for (n, paragraph) in text.split('\n').enumerate() {
                    if n > 0 {
                        broken.push('\n');
                    }
                    let Some(width) = width else {
                        broken.push_str(paragraph);
                        continue;
                    };
                    let shaped = system.shape_line(
                        SharedString::from(paragraph.to_string()),
                        font_size,
                        &[style.to_run(paragraph.len())],
                        None,
                    );
                    let parts = lines(paragraph, f32::from(width), |i| {
                        f32::from(shaped.x_for_index(i))
                    });
                    broken.push_str(
                        &parts
                            .iter()
                            .map(|part| paragraph[part.clone()].trim_end())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    );
                }
                let shaped = system
                    .shape_text(
                        SharedString::from(broken.clone()),
                        font_size,
                        &[style.to_run(broken.len())],
                        width,
                        None,
                    )
                    .map(|lines| lines.into_iter().collect::<Vec<_>>())
                    .unwrap_or_default();
                let mut size = Size::<Pixels>::default();
                for line in &shaped {
                    let line = line.size(line_height);
                    size.height += line.height;
                    size.width = size.width.max(line.width).ceil();
                }
                laid.borrow_mut().replace(Laid {
                    lines: shaped,
                    line_height,
                    width,
                    size,
                });
                size
            },
        );
        (id, ())
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(plain) = self.plain.as_mut() {
            plain.prepaint(id, inspector, bounds, state, window, cx);
        }
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        prepainted: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(plain) = self.plain.as_mut() {
            plain.paint(id, inspector, bounds, state, prepainted, window, cx);
            return;
        }
        let laid = self.laid.borrow();
        let Some(laid) = laid.as_ref() else {
            return;
        };
        let align = window.text_style().text_align;
        let mut origin = bounds.origin;
        for line in &laid.lines {
            let _ = line.paint(origin, laid.line_height, align, Some(bounds), window, cx);
            origin.y += line.size(laid.line_height).height;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines of `text` at `width`, a Latin letter or space 1 wide and a
    /// Chinese or Japanese character 2.
    fn broken(text: &str, width: f32) -> Vec<String> {
        let x = |i: usize| {
            text[..i]
                .chars()
                .map(|c| if unspaced(c) { 2.0 } else { 1.0 })
                .sum::<f32>()
        };
        lines(text, width, x)
            .into_iter()
            .map(|range| text[range].to_string())
            .collect()
    }

    #[test]
    fn a_question_mark_never_starts_a_line() {
        // "Mend it now" is 11 wide: the mark goes with "now".
        assert_eq!(broken("Mend it now?", 11.0), ["Mend it", "now?"]);
        assert_eq!(
            broken("Could I learn from them?", 23.0),
            ["Could I learn from", "them?"]
        );
        assert_eq!(
            broken("Everyone's invited?", 18.0),
            ["Everyone's", "invited?"]
        );
        // Even with a space before it.
        assert_eq!(broken("Mend it now ?", 11.0), ["Mend it", "now ?"]);
    }

    #[test]
    fn chinese_and_japanese_keep_kinsoku() {
        for (text, width) in [
            ("今すぐ直す？", 10.0),
            ("现在就修吗？我们一起。", 10.0),
            ("「そうだね」と言った。", 10.0),
            ("お茶、飲みませんか。", 6.0),
        ] {
            for line in &broken(text, width).into_iter().skip(1).collect::<Vec<_>>() {
                let first = line.chars().next().unwrap();
                assert!(!closing(first), "{text}: a line starts with {first}");
            }
            for line in broken(text, width) {
                let last = line.chars().last().unwrap();
                assert!(!opening(last), "{text}: a line ends with {last}");
            }
        }
    }

    #[test]
    fn katakana_names_never_break_mid_name() {
        let lines = broken("ソフィアについてもっと", 10.0);
        assert!(
            lines.iter().any(|line| line.contains("ソフィア")),
            "{lines:?}"
        );
        let lines = broken("リーセとソフィアが来た", 12.0);
        assert!(
            lines.iter().any(|line| line.contains("リーセ"))
                && lines.iter().any(|line| line.contains("ソフィア")),
            "{lines:?}"
        );
    }

    #[test]
    fn latin_words_inside_chinese_stay_whole() {
        let lines = broken("你见到Jonas了吗？他好不好？", 12.0);
        assert!(lines.iter().any(|line| line.contains("Jonas")), "{lines:?}");
    }

    #[test]
    fn what_fits_is_not_broken_and_what_cannot_fit_still_breaks() {
        assert_eq!(broken("Mend it now?", 40.0), ["Mend it now?"]);
        // One word longer than the line breaks where it must.
        let lines = broken("Pneumonoultramicroscopic", 10.0);
        assert!(lines.len() > 1 && lines.concat() == "Pneumonoultramicroscopic");
    }
}
