//! Speech bubbles: how a line is broken into rows and pages, how long
//! it shows, and where each bubble, a pointer's hint and the card of
//! someone being asked go on the stage.

use super::*;

/// How tall the row of gauges and handles over the sky is, generously.
pub(super) const HUD_ROOM: f32 = 64.0;
/// How tall the card at the foot of the stage is, generously.
pub(super) const CARD_ROOM: f32 = 250.0;
/// How tall a chapter's ending card stands, generously, at text size 1.
pub(super) const CHAPTER_ROOM: f32 = 400.0;
/// The zoom control's size: two buttons, one over the other.
pub(super) const ZOOM_SIZE: (f32, f32) = (36.0, 68.0);
/// How far a bubble's tail reaches down to the head it speaks for.
pub(super) const TAIL: f32 = 8.0;
/// How much room is kept between a bubble and whatever it must not
/// cover, and the stage's edge.
pub(super) const BUBBLE_CLEAR: f32 = 6.0;

/// How big a speech bubble showing `page` is drawn, generously: its width
/// and its height, the tail below it included.
#[cfg(test)]
pub(crate) fn bubble_size(page: &str) -> (f32, f32) {
    let scale = crate::text_scale();
    let widest = page.lines().map(text_width).max().unwrap_or(0) as f32;
    let rows = page.lines().count().max(1) as f32;
    let w = (widest * 7.4 * scale + 30.0).clamp(56.0, BUBBLE_ROOM);
    (w, rows * 20.0 * scale + 18.0 + TAIL)
}

/// A speech bubble as placed on the stage: the box its words are in, and
/// its tail, from the box's foot at `tail` down to the head at (`head`,
/// `foot`): straight down, or slanting where the head is too near the
/// stage's edge for the box to be right over it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placed {
    pub(crate) area: Area,
    pub(crate) tail: f32,
    pub(crate) head: f32,
    pub(crate) foot: f32,
}

impl Placed {
    /// The box around the tail.
    pub(crate) fn stalk(&self) -> Area {
        let bottom = self.area.y + self.area.h;
        Area {
            x: self.tail.min(self.head) - 7.0,
            y: bottom,
            w: (self.tail - self.head).abs() + 14.0,
            h: (self.foot - bottom).max(0.0),
        }
    }

    /// What the bubble covers: its box, and its tail.
    pub(crate) fn parts(&self) -> [Area; 2] {
        [self.area, self.stalk()]
    }

    /// The whole of it: box and tail.
    pub(crate) fn whole(&self) -> Area {
        Area {
            h: (self.foot - self.area.y).max(self.area.h),
            ..self.area
        }
    }
}

impl Area {
    /// How much of this lies over `other`, in square pixels.
    pub(super) fn over(&self, other: &Area) -> f32 {
        let w = (self.x + self.w).min(other.x + other.w) - self.x.max(other.x);
        let h = (self.y + self.h).min(other.y + other.h) - self.y.max(other.y);
        w.max(0.0) * h.max(0.0)
    }

    /// The same, with `by` more room all round.
    pub(super) fn grown(&self, by: f32) -> Area {
        Area {
            x: self.x - by,
            y: self.y - by,
            w: self.w + by * 2.0,
            h: self.h + by * 2.0,
        }
    }
}

/// Places speech bubbles, each `(x, y, w, h)` over a head at `x`, `y`
/// (the bubble `w` by `h`, its tail included), one after another on a
/// stage `width` wide: each right over its head if it can be, else moved
/// along or lifted higher (its tail still pointing down at the head), so
/// that it never covers the interface (`interface`: the gauges and
/// handles, the zoom control, the card) or a bubble placed before it, and
/// stays on the stage. Where there is no room anywhere, it goes where it
/// covers least.
pub(crate) fn place_bubbles(
    bubbles: &[(f32, f32, f32, f32)],
    interface: &[Area],
    (width, _height): (f32, f32),
) -> Vec<Placed> {
    let mut placed: Vec<Placed> = Vec::new();
    for (index, &(x, y, w, h)) in bubbles.iter().enumerate() {
        let w = w.min(width - BUBBLE_CLEAR * 2.0).max(1.0);
        // Among others speaking close by, a bubble leans away from them
        // from the start, so theirs have room over their own heads.
        let near = bubbles
            .iter()
            .enumerate()
            .filter(|(other, (ox, oy, ..))| {
                *other != index && (ox - x).abs() < w && (oy - y).abs() < h * 2.0
            })
            .map(|(_, (ox, ..))| *ox)
            .collect::<Vec<_>>();
        let away = if near.is_empty() {
            0.0
        } else {
            let middle = near.iter().sum::<f32>() / near.len() as f32;
            if x < middle {
                -1.0
            } else if x > middle {
                1.0
            } else {
                0.0
            }
        };
        // As far along as a bubble goes: its head under its corner.
        let edge = (0.5 - 16.0 / w).max(0.0);
        let shifts = if away == 0.0 {
            [0.0, -0.25, 0.25, -edge, edge]
        } else {
            [edge, 0.25, 0.0, -0.25, -edge].map(|shift| shift * away)
        };
        let boxed = h - TAIL;
        // What is taken: the interface, the bubbles placed, and over the
        // heads of those still to place, where their tails will go.
        let taken = interface
            .iter()
            .copied()
            .chain(placed.iter().flat_map(|other| other.parts()))
            .chain(bubbles[index + 1..].iter().map(|(ox, oy, ..)| Area {
                x: ox - 10.0,
                y: 0.0,
                w: 20.0,
                h: *oy,
            }))
            .collect::<Vec<_>>();
        let mut best: Option<(f32, Placed)> = None;
        'search: for lift in [0.0, 16.0, 32.0, 56.0, 88.0, 128.0, 176.0, 232.0] {
            for shift in shifts {
                let left = (x - w / 2.0 + shift * w)
                    .min(width - BUBBLE_CLEAR - w)
                    .max(BUBBLE_CLEAR);
                // The tail leaves the box clear of its rounded corners,
                // slanting to a head too near the edge; a box moved along
                // keeps its tail straight down.
                let tail = x.clamp(left + 14.0, left + w - 14.0);
                if (tail - x).abs() > 0.5 && shift != 0.0 {
                    continue;
                }
                let top = y - h - lift;
                if top < BUBBLE_CLEAR {
                    continue;
                }
                let candidate = Placed {
                    area: Area {
                        x: left,
                        y: top,
                        w,
                        h: boxed,
                    },
                    tail,
                    head: x,
                    foot: y,
                };
                let covered = candidate
                    .parts()
                    .iter()
                    .flat_map(|part| {
                        taken
                            .iter()
                            .map(|other| part.grown(BUBBLE_CLEAR).over(other))
                    })
                    .sum::<f32>();
                if best.is_none_or(|(least, _)| covered < least) {
                    best = Some((covered, candidate));
                }
                if covered <= 0.0 {
                    break 'search;
                }
            }
        }
        // Too near the top, or too near an edge, for any of that: right
        // over the head, as low as it can go.
        let chosen = best.map(|(_, placed)| placed).unwrap_or_else(|| {
            let left =
                (x - w / 2.0).clamp(BUBBLE_CLEAR, (width - BUBBLE_CLEAR - w).max(BUBBLE_CLEAR));
            Placed {
                area: Area {
                    x: left,
                    y: (y - h).max(0.0),
                    w,
                    h: boxed.min(y.max(1.0)),
                },
                tail: x.clamp(left + 14.0, left + w - 14.0),
                head: x,
                foot: y,
            }
        });
        placed.push(chosen);
    }
    placed
}

/// Where each pointer is drawn on a stage `width` by `height`, generously:
/// under its handle in the top right, beside the zoom control, or just
/// above the card.
pub(crate) fn hint_area(pointer: Pointer, width: f32, height: f32) -> Area {
    match pointer {
        Pointer::Hands | Pointer::Drawer | Pointer::Letters | Pointer::Strip => Area {
            x: width - 16.0 - HINT_WIDTH - 60.0,
            y: 40.0,
            w: HINT_WIDTH + 60.0,
            h: 140.0,
        },
        Pointer::Zoom => Area {
            x: 16.0,
            y: zoom_top(height) - 20.0,
            w: 48.0 + HINT_WIDTH,
            h: 110.0,
        },
        Pointer::Cards => Area {
            x: width / 2.0 - CARD_WIDTH / 2.0,
            y: height - 460.0,
            w: CARD_WIDTH,
            h: 160.0,
        },
    }
}

/// How wide a speech bubble's line is, in the width of a Latin letter;
/// a Chinese or Japanese character takes two.
pub(crate) const BUBBLE_LINE: usize = 36;

/// How wide a bubble line is at the text size the player chose: larger
/// text, fewer letters to a line.
pub(super) fn bubble_line() -> usize {
    ((BUBBLE_LINE as f32 / crate::text_scale()) as usize).max(14)
}

pub(super) fn text_width(text: &str) -> usize {
    text.chars()
        .map(|character| {
            if !character.is_ascii() && character.is_alphanumeric() && !character.is_alphabetic()
                || ('\u{2e80}'..='\u{9fff}').contains(&character)
                || ('\u{ff00}'..='\u{ffef}').contains(&character)
                || ('\u{3000}'..='\u{303f}').contains(&character)
            {
                2
            } else {
                1
            }
        })
        .sum()
}

/// Punctuation that closes a phrase, and so never begins a line (the
/// Chinese and Japanese line-breaking rule, kinsoku shori).
#[cfg(test)]
pub(crate) fn never_starts_a_line(character: char) -> bool {
    matches!(
        character,
        '，' | '。'
            | '、'
            | '！'
            | '？'
            | '：'
            | '；'
            | '」'
            | '』'
            | '）'
            | '】'
            | '》'
            | '〉'
            | '〕'
            | '”'
            | '’'
            | '…'
            | '・'
            | '～'
            | 'ー'
            | ','
            | '.'
            | '!'
            | '?'
            | ':'
            | ';'
            | ')'
            | ']'
    )
}

/// A line cut into bubble lines: broken between words, or anywhere in a
/// language written without spaces, but never before a closing mark or
/// after an opening one: the character before a comma goes down to the
/// next line with it.
pub(super) fn bubble_rows(line: &str) -> Vec<String> {
    // Broken by the World's one set of rules (`crate::wrap`): never before
    // a closing mark, never after an opening one, never inside a katakana
    // name or a word, and a question mark never alone.
    let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
    crate::wrap::lines(&line, bubble_line() as f32, |at| {
        text_width(&line[..at]) as f32
    })
    .into_iter()
    .map(|range| line[range].trim().to_string())
    .filter(|row| !row.is_empty())
    .collect()
}

/// Where a sentence ends: after its closing mark (and any closing quote
/// or bracket after it).
pub(super) fn ends_a_sentence(character: char) -> bool {
    matches!(character, '.' | '!' | '?' | '…' | '。' | '！' | '？')
}

/// A mark that closes a quotation or an aside: after a sentence's end,
/// it belongs to that sentence (`?"` and `。」`).
pub(super) fn closes(character: char) -> bool {
    matches!(
        character,
        '"' | '\'' | ')' | ']' | '”' | '’' | '」' | '』' | '）' | '】' | '》' | '〉'
    )
}

/// A line cut into its sentences, each with its closing marks; a
/// language written without spaces is cut after its full stops too.
pub(crate) fn sentences(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut sentence = String::new();
    let characters = line.chars().collect::<Vec<_>>();
    // The mark that ended the sentence, while closing marks follow it.
    let mut ended: Option<char> = None;
    for (index, character) in characters.iter().copied().enumerate() {
        sentence.push(character);
        if ends_a_sentence(character) {
            ended = Some(character);
        } else if !(ended.is_some() && closes(character)) {
            ended = None;
        }
        let Some(mark) = ended else {
            continue;
        };
        let next = characters.get(index + 1).copied();
        // More marks to come: `?!`, `。」`.
        if next.is_some_and(|next| ends_a_sentence(next) || closes(next)) {
            continue;
        }
        // "Mr. Lark" and "3.5" are not the end of anything: a full stop
        // ends a sentence written with spaces only before a space.
        let spaced = !matches!(mark, '。' | '！' | '？');
        if spaced && next.is_some_and(|next| !next.is_whitespace()) {
            continue;
        }
        ended = None;
        let done = std::mem::take(&mut sentence);
        if !done.trim().is_empty() {
            out.push(done.trim().to_string());
        }
    }
    if !sentence.trim().is_empty() {
        out.push(sentence.trim().to_string());
    }
    out
}

/// How many bubble lines a page shows at most: two, or for one sentence
/// too long for two, as many as it takes, up to this, so no sentence is
/// ever cut off and finished on the next page.
pub(super) const PAGE_MOST: usize = 4;

/// A line cut into pages for a speech bubble. A page holds whole
/// sentences, as many as fit in two bubble lines; a sentence too long for
/// two lines has a page of its own, as tall as it needs (up to
/// [`PAGE_MOST`] lines; only a sentence longer still goes on over pages).
/// So a page never stops in the middle of a sentence and leaves the
/// reader waiting for its end.
pub fn speech_pages(line: &str) -> Vec<String> {
    let joined = |a: &str, b: &str| {
        if a.is_empty() {
            b.to_string()
        } else if a
            .chars()
            .last()
            .is_some_and(|c| c.is_ascii() || c.is_whitespace())
            || b.chars().next().is_some_and(|c| c.is_ascii())
        {
            format!("{a} {b}")
        } else {
            format!("{a}{b}")
        }
    };
    let mut pages: Vec<Vec<String>> = Vec::new();
    let mut page = String::new();
    for sentence in sentences(line) {
        let together = joined(&page, &sentence);
        if bubble_rows(&together).len() <= 2 {
            page = together;
            continue;
        }
        if !page.is_empty() {
            pages.push(bubble_rows(&std::mem::take(&mut page)));
        }
        let rows = bubble_rows(&sentence);
        if rows.len() <= 2 {
            page = sentence;
        } else {
            // A long sentence on its own page, as tall as it needs; one
            // longer than a bubble can hold goes on over pages.
            for chunk in rows.chunks(PAGE_MOST) {
                pages.push(chunk.to_vec());
            }
        }
    }
    if !page.is_empty() {
        pages.push(bubble_rows(&page));
    }
    if pages.is_empty() {
        return vec![String::new()];
    }
    pages.into_iter().map(|rows| rows.join("\n")).collect()
}

/// How long an answer to the player stays over its speaker: long enough
/// to read every page of it.
pub(super) fn answer_seconds(answer: &str) -> f32 {
    ANSWER_SECONDS.max(LINE_SECONDS * speech_pages(answer).len() as f32 + 2.0)
}

/// How wide a speech bubble can be.
pub(super) const BUBBLE_ROOM: f32 = 300.0;

/// How far a bubble's words keep from its sides.
pub(super) const BUBBLE_PAD: f32 = 12.0;

/// A page of speech laid out for its bubble, measured by the text system
/// itself: its lines, broken by the World's rules (`crate::wrap`) and as
/// even as they can be, and how big the bubble is drawn (its width and
/// its height, the tail below it included). What is measured is what is
/// drawn: no line is broken again where the measure was out.
pub(crate) fn lay_bubble(window: &Window, page: &str, strong: bool) -> (Vec<String>, f32, f32) {
    let paragraph = page
        .lines()
        .map(str::trim)
        .filter(|row| !row.is_empty())
        .fold(String::new(), |all, row| {
            let spaced = all
                .chars()
                .last()
                .is_some_and(|c| c.is_ascii() && !c.is_whitespace())
                || row.chars().next().is_some_and(|c| c.is_ascii());
            if all.is_empty() {
                row.to_string()
            } else if spaced {
                format!("{all} {row}")
            } else {
                format!("{all}{row}")
            }
        });
    let mut style = window.text_style();
    style.font_size = gpui::rems(0.875).into();
    if strong {
        style.font_weight = FontWeight::MEDIUM;
    }
    let rem = window.rem_size();
    let font_size = style.font_size.to_pixels(rem);
    let line_height = f32::from(style.line_height.to_pixels(font_size.into(), rem));
    let shaped = window.text_system().shape_line(
        SharedString::from(paragraph.clone()),
        font_size,
        &[style.to_run(paragraph.len())],
        None,
    );
    let x = |i: usize| f32::from(shaped.x_for_index(i));
    let room = BUBBLE_ROOM * crate::text_scale().clamp(1.0, 1.4) - BUBBLE_PAD * 2.0;
    let ranges = crate::wrap::balanced(&paragraph, room, x);
    let widest = ranges
        .iter()
        .map(|range| {
            let end = range.start + paragraph[range.clone()].trim_end().len();
            x(end) - x(range.start)
        })
        .fold(0.0, f32::max);
    let rows = ranges
        .iter()
        .map(|range| paragraph[range.clone()].trim().to_string())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let w = (widest.ceil() + BUBBLE_PAD * 2.0 + 2.0).max(56.0);
    let h = rows.len().max(1) as f32 * line_height + 16.0 + TAIL;
    (rows, w, h)
}

/// Whether a head at `x`, `y` is on a stage `width` by `height`.
pub(super) fn on_screen(x: f32, y: f32, (width, height): (f32, f32)) -> bool {
    (4.0..=width - 4.0).contains(&x) && y > 0.0 && y <= height
}

/// How much wider an edge tab is than a bubble with the same words: the
/// speaker's face and the arrow toward them.
pub(super) const TAB_FACE: f32 = 52.0;

/// Where the words of someone off the stage go: a tab at the edge they
/// are beyond, level with them as far as it can be, clear of the
/// interface. Its box, and whether it is on the left.
pub(super) fn place_tab(
    x: f32,
    y: f32,
    (w, h): (f32, f32),
    interface: &[Area],
    (width, height): (f32, f32),
) -> (Area, bool) {
    let left = x < width / 2.0;
    let tw = (w + TAB_FACE).min(width - BUBBLE_CLEAR * 2.0);
    let th = (h - TAIL).max(44.0);
    let ax = if left {
        BUBBLE_CLEAR
    } else {
        width - BUBBLE_CLEAR - tw
    };
    let lowest = (height - th - BUBBLE_CLEAR).max(HUD_ROOM);
    let want = (y - th / 2.0).clamp(HUD_ROOM + BUBBLE_CLEAR, lowest);
    let mut best: Option<(f32, Area)> = None;
    for step in 0..24 {
        let by = if step % 2 == 0 {
            step / 2
        } else {
            -(step / 2 + 1)
        } as f32
            * 20.0;
        let area = Area {
            x: ax,
            y: (want + by).clamp(HUD_ROOM + BUBBLE_CLEAR, lowest),
            w: tw,
            h: th,
        };
        let covered = interface
            .iter()
            .map(|other| area.grown(BUBBLE_CLEAR).over(other))
            .sum::<f32>();
        if best.is_none_or(|(least, _)| covered < least) {
            best = Some((covered, area));
        }
        if covered <= 0.0 {
            break;
        }
    }
    (
        best.map(|(_, area)| area).unwrap_or(Area {
            x: ax,
            y: want,
            w: tw,
            h: th,
        }),
        left,
    )
}

/// A speech bubble: what someone says, in the lines [`lay_bubble`] broke
/// it into, where [`place_bubbles`] put it, with a tail pointing down at
/// their head.
pub(super) fn bubble(rows: Vec<String>, placed: Placed, opacity: f32, strong: bool) -> Div {
    let area = placed.whole();
    let stalk = placed.stalk();
    let ink: Hsla = color(tokens::TEXT).into();
    // White by day; after dark the night's own card, never a glare.
    let ground: Hsla = color(tokens::SURFACE).into();
    // Lifted clear of something, the tail reaches down the further, and
    // the narrower; to a head near the edge it slants.
    let (tail, head) = (placed.tail - stalk.x, placed.head - stalk.x);
    let half = if stalk.h > TAIL * 2.0 { 5.0 } else { 7.0 };
    let tail_shape = canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            let h = f32::from(bounds.size.height);
            art::polygon(
                window,
                &[
                    (ox + tail - half, oy - 1.0),
                    (ox + tail + half, oy - 1.0),
                    (ox + head, oy + h),
                ],
                ground,
            );
        },
    )
    .absolute()
    .left(px(stalk.x - area.x))
    .top(px(placed.area.h))
    .w(px(stalk.w))
    .h(px(stalk.h));
    div()
        .absolute()
        .left(px(area.x))
        .top(px(area.y))
        .w(px(area.w))
        .h(px(area.h))
        .opacity(opacity)
        .child(
            div()
                .absolute()
                .left_0()
                .bottom(px(stalk.h))
                .w_full()
                .px(px(BUBBLE_PAD))
                .py_2()
                .rounded_xl()
                .bg(ground)
                .shadow_md()
                .text_sm()
                .text_center()
                .when(strong, |text| text.font_weight(FontWeight::MEDIUM))
                .text_color(ink)
                .flex()
                .flex_col()
                .items_center()
                .children(
                    rows.into_iter()
                        .map(|row| div().whitespace_nowrap().child(row)),
                ),
        )
        .child(tail_shape)
}

/// The words of someone off the stage, at its edge: their face, the words,
/// and an arrow toward where they are.
pub(super) fn edge_tab(
    rows: Vec<String>,
    area: Area,
    left: bool,
    likeness: Likeness,
    opacity: f32,
    strong: bool,
) -> Div {
    let arrow = div()
        .flex_shrink_0()
        .text_sm()
        .text_color(color(tokens::TEXT_SECONDARY))
        .child(if left { "‹" } else { "›" });
    let words = div()
        .flex_1()
        .min_w(px(0.0))
        .text_sm()
        .text_center()
        .when(strong, |text| text.font_weight(FontWeight::MEDIUM))
        .text_color(color(tokens::TEXT))
        .flex()
        .flex_col()
        .items_center()
        .children(
            rows.into_iter()
                .map(|row| div().whitespace_nowrap().child(row)),
        );
    let face = div().flex_shrink_0().child(portrait(likeness, 28.0, false));
    let row = div()
        .absolute()
        .left(px(area.x))
        .top(px(area.y))
        .w(px(area.w))
        .min_h(px(area.h))
        .opacity(opacity)
        .px_2()
        .py_2()
        .rounded_xl()
        .bg(color(tokens::SURFACE))
        .shadow_md()
        .flex()
        .items_center()
        .gap_1();
    if left {
        row.child(arrow).child(face).child(words)
    } else {
        row.child(words).child(face).child(arrow)
    }
}

/// Where the card of someone being asked goes, generously: beside them
/// (their head at `x`, `head`) on whichever side has room, never over
/// them, level with their head as far as the stage `width` by `height`
/// allows, clear of an open drawer and always inside the window; `talked`
/// exchanges make it taller, though never taller than the window has room
/// for (what does not fit scrolls inside it).
pub(crate) fn asking_place(
    x: f32,
    head: f32,
    (width, height): (f32, f32),
    drawer: bool,
    talked: usize,
    scale: f32,
) -> Area {
    let room = width - if drawer { DRAWER_WIDTH } else { 0.0 };
    let left = if x + 40.0 + ASKING_WIDTH < room - ASKING_CLEAR {
        x + 40.0
    } else if x - 40.0 - ASKING_WIDTH >= ASKING_CLEAR {
        x - 40.0 - ASKING_WIDTH
    } else if x > room / 2.0 {
        // No room either side: the side with more, as far from them as
        // the window allows.
        ASKING_CLEAR
    } else {
        (room - ASKING_CLEAR - ASKING_WIDTH).max(ASKING_CLEAR)
    }
    // Always inside the window, beside the drawer if it is open.
    .clamp(
        ASKING_CLEAR,
        (room - ASKING_CLEAR - ASKING_WIDTH).max(ASKING_CLEAR),
    );
    let room_tall = (height - HUD_ROOM - ASKING_CLEAR).max(120.0);
    let tall = ((360.0 + talked as f32 * 96.0) * scale).min(room_tall);
    Area {
        x: left,
        y: (head - 24.0).clamp(HUD_ROOM, (height - ASKING_CLEAR - tall).max(HUD_ROOM)),
        w: ASKING_WIDTH,
        h: tall,
    }
}
