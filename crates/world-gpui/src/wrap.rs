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

/// Hiragana: in Japanese, the endings and particles that hold to the word
/// before them ("くださ / い" is a word broken in two).
fn hiragana(c: char) -> bool {
    matches!(c, '\u{3041}'..='\u{309F}')
}

/// A Chinese character (a kanji, in Japanese).
fn han(c: char) -> bool {
    matches!(
        c,
        '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}' | '々'
    )
}

/// Part of a word, in any script: what a phrase is made of, as against
/// the punctuation and spaces between phrases.
fn wordlike(c: char) -> bool {
    hiragana(c) || katakana(c) || han(c) || lettered(c)
}

/// Whether `text` is Japanese: it has kana in it.
fn japanese(text: &str) -> bool {
    text.chars()
        .any(|c| hiragana(c) || (katakana(c) && c != '・' && c != 'ー'))
}

/// A place a line may break: it ends at `at`, and the next starts at
/// `next` (past the space broken at, if any).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Break {
    at: usize,
    next: usize,
}

/// Every place `text` may break, by the rules above, in order. With
/// `phrases`, Japanese breaks only between phrases: never before kana
/// that ends a word or is a particle, never inside a run of kanji.
fn breaks_by(text: &str, phrases: bool) -> Vec<Break> {
    let chars = text.char_indices().collect::<Vec<_>>();
    let mut found = Vec::new();
    // Until a word has begun, marks that lead a line ("……", a lone "、")
    // stay with what follows them: never a line of marks alone.
    let mut begun = false;
    for i in 1..chars.len() {
        let (at, c) = chars[i];
        let prev = chars[i - 1].1;
        begun |= wordlike(prev);
        if !begun {
            continue;
        }
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
        if !between
            || closing(c)
            || opening(prev)
            || (katakana(prev) && katakana(c))
            || (lettered(prev) && lettered(c))
        {
            continue;
        }
        if phrases {
            // What ends a word, or a particle, holds to the word before it
            // ("ソフィア / について" is no better than "くださ / い").
            if hiragana(c) && wordlike(prev) {
                continue;
            }
            // A run of kanji is a word.
            if han(prev) && han(c) {
                continue;
            }
        }
        found.push(Break { at, next: at });
    }
    found
}

/// Every place `text` may break.
fn breaks(text: &str) -> Vec<Break> {
    breaks_by(text, false)
}

/// Where `text` may break if nothing else fits: before any character but
/// closing punctuation.
fn anywhere(text: &str) -> Vec<Break> {
    let lead = text
        .char_indices()
        .find(|(_, c)| wordlike(*c))
        .map_or(text.len(), |(at, _)| at);
    text.char_indices()
        .skip(1)
        .filter(|(at, _)| *at > lead)
        .filter(|(_, c)| !closing(*c) && *c != ' ')
        .map(|(at, _)| Break { at, next: at })
        .collect()
}

/// The lines `text` is broken into, as byte ranges, to fit `width`;
/// `x(i)` is how far along a single unbroken line byte `i` starts.
/// Japanese breaks between its phrases where a phrase fits, and only
/// inside one where none does.
pub fn lines(text: &str, width: f32, x: impl Fn(usize) -> f32) -> Vec<std::ops::Range<usize>> {
    let breaks = breaks(text);
    let (phrases, particles) = if japanese(text) {
        // Where no phrase fits, after a particle or a joining ending
        // ("ソフィアに / ついて"), rather than anywhere in a word.
        let particles = breaks
            .iter()
            .copied()
            .filter(|b| {
                text[..b.at].chars().next_back().is_some_and(|c| {
                    matches!(c, 'は' | 'が' | 'を' | 'に' | 'で' | 'て' | 'へ' | 'の')
                })
            })
            .collect();
        (breaks_by(text, true), particles)
    } else {
        (Vec::new(), Vec::new())
    };
    let fallback = anywhere(text);
    let mut out = Vec::new();
    let mut start = 0;
    while x(text.len()) - x(start) > width + 0.5 {
        let fits = |b: &&Break| b.at > start && x(b.at) - x(start) <= width + 0.5;
        let last_fitting = |list: &[Break]| {
            list.iter()
                .filter(|b| b.at > start)
                .take_while(|b| x(b.at) - x(start) <= width + 0.5)
                .last()
                .copied()
        };
        // A phrase break, unless it leaves the line less than a quarter full
        // where a plainer one fills it.
        let plain = last_fitting(&particles)
            .filter(|b| x(b.at) - x(start) >= width * 0.25)
            .or_else(|| last_fitting(&breaks));
        let chosen = match (last_fitting(&phrases), plain) {
            (Some(phrase), Some(plain))
                if x(phrase.at) - x(start) < width * 0.25 && plain.at > phrase.at =>
            {
                Some(plain)
            }
            (Some(phrase), _) => Some(phrase),
            (None, plain) => plain,
        }
        .or_else(|| fallback.iter().filter(fits).last().copied())
        .or_else(|| fallback.iter().find(|b| b.at > start).copied());
        let Some(chosen) = chosen else {
            break;
        };
        out.push(start..chosen.at);
        start = chosen.next;
    }
    out.push(start..text.len());
    out
}

/// How wide each of `ranges` is drawn, its trailing spaces aside.
fn widths(text: &str, ranges: &[std::ops::Range<usize>], x: &impl Fn(usize) -> f32) -> Vec<f32> {
    ranges
        .iter()
        .map(|range| {
            let end = range.start + text[range.clone()].trim_end().len();
            x(end) - x(range.start)
        })
        .collect()
}

/// Whether lines read evenly: the last at least three tenths as wide as
/// the widest, and, in a language written with spaces, no word alone on
/// a line of its own.
fn even(text: &str, ranges: &[std::ops::Range<usize>], x: &impl Fn(usize) -> f32) -> bool {
    if ranges.len() < 2 {
        return true;
    }
    let widths = widths(text, ranges, x);
    let widest = widths.iter().copied().fold(0.0, f32::max);
    let last = widths.last().copied().unwrap_or(0.0);
    if last < widest * 0.3 {
        return false;
    }
    let spaced = !text.chars().any(unspaced);
    let words = text.split_whitespace().count();
    !(spaced
        && words >= ranges.len() * 2
        && ranges
            .iter()
            .any(|range| text[range.clone()].split_whitespace().count() < 2))
}

/// The lines `text` is broken into for a speech bubble at most `width`
/// wide: as few as fit, and as even as they can be, so the bubble is no
/// wider than it needs and no line is left a stub ("with a / heavy /
/// crate"): the narrowest measure that keeps the fewest lines and reads
/// evenly, or one more line where none does.
pub fn balanced(text: &str, width: f32, x: impl Fn(usize) -> f32) -> Vec<std::ops::Range<usize>> {
    let fewest = lines(text, width, &x);
    if fewest.len() < 2 {
        return fewest;
    }
    let total = x(text.len());
    let steps = 48;
    for count in [fewest.len(), fewest.len() + 1] {
        let least = (total / count as f32).min(width);
        let mut first = None;
        for step in 0..=steps {
            let measure = least + (width - least) * step as f32 / steps as f32;
            let tried = lines(text, measure, &x);
            if tried.len() != count {
                continue;
            }
            if even(text, &tried, &x) {
                return tried;
            }
            first.get_or_insert(tried);
        }
        if count == fewest.len() {
            if let Some(first) = first.filter(|_| fewest.len() > 2) {
                // Three lines or more that cannot all be even: as even as
                // they come.
                return first;
            }
        }
    }
    fewest
}

/// The lines of `text` at `width`, at most `most` of them: if there are
/// more, the last shown ends at a word (or, in Chinese and Japanese, a
/// character) with an ellipsis `ellipsis` wide after it, never in the
/// middle of a word. Whether it was cut.
pub fn clamped(
    text: &str,
    width: f32,
    x: impl Fn(usize) -> f32,
    most: usize,
    ellipsis: f32,
) -> (Vec<std::ops::Range<usize>>, bool) {
    let mut all = lines(text, width, &x);
    let most = most.max(1);
    if all.len() <= most {
        return (all, false);
    }
    all.truncate(most);
    let start = all[most - 1].start;
    let room = |at: usize| x(at) - x(start) + ellipsis <= width + 0.5;
    let end = breaks(text)
        .into_iter()
        .filter(|b| b.at > start && room(b.at))
        .map(|b| b.at)
        .next_back()
        .or_else(|| {
            // A word longer than the line, or Chinese written without
            // breaks: any character but a closing mark.
            anywhere(text)
                .into_iter()
                .filter(|b| b.at > start && room(b.at))
                .map(|b| b.at)
                .next_back()
                .filter(|_| !text[start..].contains(' ') || text.chars().any(unspaced))
        })
        .unwrap_or(all[most - 1].end);
    // No comma or opening mark left hanging before the ellipsis.
    let kept = text[start..end].trim_end_matches(|c: char| {
        c == ' ' || matches!(c, ',' | ';' | ':' | '、' | '，' | '—' | '-') || opening(c)
    });
    all[most - 1] = start..start + kept.len();
    (all, true)
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
        // Kept to a few lines (clamped), or to one cut short (truncated):
        // cut at a word, with an ellipsis. Kept to one line uncut: GPUI's
        // own text, as it always was.
        let nowrap = style.white_space != WhiteSpace::Normal;
        let most = match (style.line_clamp, nowrap, style.text_overflow.is_some()) {
            (_, true, true) => Some(1),
            (_, true, false) => None,
            (most, false, _) => most,
        };
        if nowrap && most.is_none() {
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
                let mut used = 0;
                for (n, paragraph) in text.split('\n').enumerate() {
                    if most.is_some_and(|most| used >= most) {
                        break;
                    }
                    if n > 0 {
                        broken.push('\n');
                    }
                    let Some(width) = width else {
                        broken.push_str(paragraph);
                        used += 1;
                        continue;
                    };
                    let shaped = system.shape_line(
                        SharedString::from(paragraph.to_string()),
                        font_size,
                        &[style.to_run(paragraph.len())],
                        None,
                    );
                    let x = |i| f32::from(shaped.x_for_index(i));
                    let Some(most) = most else {
                        let parts = lines(paragraph, f32::from(width), x);
                        broken.push_str(
                            &parts
                                .iter()
                                .map(|part| paragraph[part.clone()].trim_end())
                                .collect::<Vec<_>>()
                                .join("\n"),
                        );
                        continue;
                    };
                    // Clamped: what is left of the lines to this paragraph.
                    let left = most - used;
                    let ellipsis = system.shape_line(
                        SharedString::from("…"),
                        font_size,
                        &[style.to_run("…".len())],
                        None,
                    );
                    let (parts, cut) = clamped(
                        paragraph,
                        f32::from(width),
                        x,
                        left,
                        f32::from(ellipsis.width),
                    );
                    let mut shown = parts
                        .iter()
                        .map(|part| paragraph[part.clone()].trim_end())
                        .collect::<Vec<_>>()
                        .join("\n");
                    if cut {
                        shown.push('…');
                    }
                    used += parts.len();
                    broken.push_str(&shown);
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

    /// Lines of `text` balanced for a bubble at most `width` wide.
    fn balanced_lines(text: &str, width: f32) -> Vec<String> {
        let x = |i: usize| {
            text[..i]
                .chars()
                .map(|c| if unspaced(c) { 2.0 } else { 1.0 })
                .sum::<f32>()
        };
        balanced(text, width, x)
            .into_iter()
            .map(|range| text[range].trim().to_string())
            .collect()
    }

    #[test]
    fn japanese_breaks_between_phrases_never_inside_a_word() {
        // "くださ / い" and "みせる / よ" were broken words.
        for (text, width) in [
            ("ここで手を開くか、Hキーを押してください。", 16.0),
            ("材木をくれたら、桟橋を造ってみせるよ。", 18.0),
            ("材木をくれたら、桟橋を造ってみせるよ。", 30.0),
            ("ソフィアについてもっと知りたい", 16.0),
        ] {
            let lines = broken(text, width);
            for pair in lines.windows(2) {
                let start = pair[1].chars().next().unwrap();
                let joint = format!("{}|{}", pair[0], pair[1]);
                for word in ["くださ|い", "みせる|よ", "ソフィ|ア", "桟|橋", "押し|て"]
                {
                    let (a, b) = word.split_once('|').unwrap();
                    assert!(
                        !(pair[0].ends_with(a) && pair[1].starts_with(b)),
                        "{text} at {width}: {joint}"
                    );
                }
                assert!(!closing(start), "{text} at {width}: {lines:?}");
            }
        }
        assert_eq!(
            broken("ここで手を開くか、Hキーを押してください。", 26.0),
            ["ここで手を開くか、Hキーを", "押してください。"]
        );
    }

    #[test]
    fn a_bubble_reads_evenly() {
        // Not "Watched the ferry come and go / from / the bench."
        let lines = balanced_lines("Watched the ferry come and go from the bench.", 36.0);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(
            lines.iter().all(|line| line.split(' ').count() >= 3),
            "{lines:?}"
        );
        // Not "with a / heavy / crate".
        let lines = balanced_lines("Evan came up the hill with a heavy crate.", 36.0);
        assert_eq!(lines.len(), 2, "{lines:?}");
        let widest = lines.iter().map(|line| line.len()).max().unwrap() as f32;
        let last = lines.last().unwrap().len() as f32;
        assert!(last >= widest * 0.3, "{lines:?}");
        // What fits on one line stays on one.
        assert_eq!(balanced_lines("Morning!", 36.0), ["Morning!"]);
    }

    #[test]
    fn a_clamped_caption_ends_at_a_word_with_an_ellipsis() {
        let text = "Leo walked round the town every year on Builders' Day, around the net sheds.";
        let x = |i: usize| text[..i].chars().count() as f32;
        let (lines, cut) = clamped(text, 24.0, x, 2, 1.0);
        assert!(cut);
        assert_eq!(lines.len(), 2);
        let last = &text[lines[1].clone()];
        // It ends with a whole word, never "the net s".
        let next = text[lines[1].end..].chars().next();
        assert!(next.is_none_or(|c| c == ' ' || c == ','), "{last:?}");
        assert!(last.chars().count() < 24, "{last:?}");
        // Chinese ends at a character, never before a closing mark.
        let text = "莱奥每年建设日都绕着小镇走一圈，一直走到渔网棚那边。";
        let x = |i: usize| text[..i].chars().count() as f32 * 2.0;
        let (lines, cut) = clamped(text, text_width(12), x, 1, 2.0);
        assert!(cut);
        let next = text[lines[0].end..].chars().next().unwrap();
        assert!(!closing(next), "{:?}", &text[lines[0].clone()]);
        // What fits is not cut.
        let (lines, cut) = clamped("A short one.", 24.0, |i| i as f32, 2, 1.0);
        assert!(!cut && lines.len() == 1);
    }

    fn text_width(characters: usize) -> f32 {
        characters as f32 * 2.0
    }

    /// Every Chinese and Japanese line in every catalog the app installs
    /// (its own, the Systems', every World's), broken at every measure
    /// from a few characters to a bubble's: no line starts with a mark
    /// that closes or ends something, and none is only punctuation.
    #[test]
    fn every_catalog_line_keeps_kinsoku_at_every_measure() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut files = Vec::new();
        for dir in ["crates", "worlds", "systems"] {
            for entry in std::fs::read_dir(root.join(dir))
                .into_iter()
                .flatten()
                .flatten()
            {
                for entry in std::fs::read_dir(entry.path().join("locales"))
                    .into_iter()
                    .flatten()
                    .flatten()
                {
                    let path = entry.path();
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    if name.ends_with("zh-Hans.tsv") || name.ends_with("ja.tsv") {
                        files.push(path);
                    }
                }
            }
        }
        assert!(files.len() >= 6, "{files:?}");
        let mut bad = Vec::new();
        let mut lines_read = 0;
        for file in &files {
            let text = std::fs::read_to_string(file).unwrap();
            for line in text.lines().filter(|line| !line.starts_with('#')) {
                let Some((_, shown)) = line.split_once('\t') else {
                    continue;
                };
                // Templates are broken once filled in.
                if !shown.chars().any(unspaced) || shown.contains('{') {
                    continue;
                }
                lines_read += 1;
                for width in [8.0, 11.0, 14.0, 17.0, 22.0, 27.0, 36.0] {
                    let lines = broken(shown, width);
                    for (n, part) in lines.iter().enumerate() {
                        let part = part.trim();
                        let first = part.chars().next();
                        let only_marks = !part.is_empty()
                            && part.chars().all(|c| closing(c) || opening(c) || c == ' ');
                        if (n > 0 && first.is_some_and(closing)) || only_marks {
                            bad.push(format!("{width}: {shown} => {lines:?}"));
                            break;
                        }
                    }
                }
            }
        }
        assert!(lines_read > 1000, "{lines_read} lines read");
        assert!(
            bad.is_empty(),
            "{} broken badly:\n{}",
            bad.len(),
            bad[..bad.len().min(30)].join("\n")
        );
    }
}
