//! Showing the app and its Worlds in another language, as presentation
//! only: what a World records is always its own words, and a translation
//! is worked out as the words are shown, never stored.
//!
//! A catalog holds lines in English and their translation, one a line,
//! separated by a tab. A line may have `{slots}`: it then matches any text
//! with the same words around them, and what fills each slot is translated
//! in turn, so "You built a bench by Harbor Bakery" is shown from "You
//! built a {} by {at}" with "bench" and "Harbor Bakery" translated too.
//! Text is translated a sentence at a time, since the Worlds build their
//! lines from sentences, and a run of sentences the catalog has as one
//! line is taken whole. A line is never part translated: a template fits
//! only when what fills each of its slots can be shown too, translated or
//! as a name or number, which reads the same in any language; anything no
//! catalog knows is shown as it is.
//!
//! People's names are translated only as a catalog says: the Chinese
//! catalogs keep them in Latin letters, and the Japanese ones write them in
//! katakana (`Mara<tab>マーラ`), which fills every slot a name is in. A
//! catalog may also say which words
//! a speaker uses in place of others (`=nest<tab>home`), so a line said in
//! their own words is read as the plain one.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, RwLock};

/// One piece of a catalog line: words to find as they are, or a slot.
#[derive(Clone, Debug, PartialEq)]
enum Piece {
    Words(String),
    Slot(String),
}

#[derive(Clone, Debug)]
struct Template {
    pieces: Vec<Piece>,
    translation: String,
    /// The longest run of words, to rule a template out quickly.
    anchor: String,
    /// How many sentences it is.
    sentences: usize,
}

/// Lines in one language and their translation into another.
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    exact: HashMap<String, String>,
    templates: Vec<Template>,
    /// Lines with slots, as written, for filling them in every way.
    written: HashMap<String, String>,
    /// Words a speaker says in place of others ("nest" for "home"), from
    /// lines written `=nest<tab>home`: a sentence nobody wrote with the
    /// one is tried again with the other.
    instead: Vec<(String, String)>,
    /// How the language says "I" and "me", from a line written
    /// `@I<tab>我`: a line someone writes of themselves ("I went fishing
    /// with Miri") is read as the same line told of anyone, with the
    /// speaker put in as a name.
    myself: Option<String>,
}

/// Stands for the speaker while a line told of themselves is read as one
/// told of anyone: a name nobody has.
const MYSELF: &str = "Myselfname";

fn pieces(line: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        if open > 0 {
            pieces.push(Piece::Words(rest[..open].to_string()));
        }
        pieces.push(Piece::Slot(rest[open + 1..open + close].to_string()));
        rest = &rest[open + close + 1..];
    }
    if !rest.is_empty() {
        pieces.push(Piece::Words(rest.to_string()));
    }
    pieces
}

impl Catalog {
    /// A catalog from tab-separated lines; lines without a tab, blank or
    /// starting with `#` are ignored, and a later line wins.
    pub fn parse(text: &str) -> Self {
        let mut catalog = Self::default();
        catalog.extend(text);
        catalog
    }

    fn add_template(&mut self, pieces: Vec<Piece>, translation: String) {
        let anchor = pieces
            .iter()
            .filter_map(|piece| match piece {
                Piece::Words(words) => Some(words.clone()),
                Piece::Slot(_) => None,
            })
            .max_by_key(|words| words.trim().len())
            .unwrap_or_default();
        let written = pieces
            .iter()
            .map(|piece| match piece {
                Piece::Words(words) => words.as_str(),
                Piece::Slot(_) => "Slot",
            })
            .collect::<String>();
        let sentences = split_sentences(&written).len();
        self.templates.retain(|template| template.pieces != pieces);
        self.templates.push(Template {
            pieces,
            translation,
            anchor,
            sentences,
        });
    }

    /// Adds another catalog's lines to this one.
    pub fn extend(&mut self, text: &str) {
        for line in text.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((from, to)) = line.split_once('\t') else {
                continue;
            };
            let (from, to) = (unescape(from), unescape(to.trim_end()));
            if from == "@I" {
                if !to.is_empty() {
                    self.myself = Some(to);
                }
                continue;
            }
            if let Some(said) = from.strip_prefix('=') {
                if !said.is_empty() && !to.is_empty() {
                    self.instead.retain(|(known, _)| known != said);
                    self.instead.push((said.to_string(), to));
                }
                continue;
            }
            if to.is_empty() || to == "SKIP" {
                continue;
            }
            if from.contains('{') && from.contains('}') {
                self.written.insert(from.clone(), to.clone());
                let pieces = pieces(&from);
                // A line that is all slots would match anything.
                let anchor = pieces
                    .iter()
                    .filter_map(|piece| match piece {
                        Piece::Words(words) => Some(words.clone()),
                        Piece::Slot(_) => None,
                    })
                    .max_by_key(|words| words.trim().len())
                    .unwrap_or_default();
                // Nor may a line with only a word or two around its slots
                // ("{} of {}"): it would take any sentence apart.
                let letters = pieces
                    .iter()
                    .filter_map(|piece| match piece {
                        Piece::Words(words) => {
                            Some(words.chars().filter(|c| c.is_alphabetic()).count())
                        }
                        Piece::Slot(_) => None,
                    })
                    .sum::<usize>();
                // A line that starts with its words is held at the start,
                // so a short one ("By {place}") is safe; one that starts
                // with a slot needs more words around it.
                let starts_with_words = matches!(pieces.first(), Some(Piece::Words(_)));
                if anchor.trim().len() < 2 || (letters < 4 && !starts_with_words) {
                    continue;
                }
                // "{opener} Last time: {}." is also "Last time: {}." when
                // the opener is a sentence of its own.
                if let (Some(Piece::Slot(slot)), Some(Piece::Words(next))) =
                    (pieces.first(), pieces.get(1))
                {
                    let marker = format!("{{{slot}}}");
                    if next.starts_with(' ')
                        && next[1..].starts_with(|c: char| c.is_uppercase())
                        && to.starts_with(&marker)
                    {
                        let mut rest = pieces[1..].to_vec();
                        if let Some(Piece::Words(first)) = rest.first_mut() {
                            *first = first[1..].to_string();
                        }
                        self.add_template(rest, to[marker.len()..].trim_start().to_string());
                    }
                }
                self.add_template(pieces, to);
            } else {
                self.exact.insert(from, to);
            }
        }
        // Longer, more specific templates are tried first.
        self.templates.sort_by_key(|template| {
            std::cmp::Reverse(
                template
                    .pieces
                    .iter()
                    .map(|piece| match piece {
                        Piece::Words(words) => words.len(),
                        Piece::Slot(_) => 0,
                    })
                    .sum::<usize>(),
            )
        });
    }

    /// The translation of exactly `text`, if the catalog has it.
    pub fn exact(&self, text: &str) -> Option<&str> {
        self.exact.get(text).map(String::as_str)
    }

    /// Lines for every way a template's slots can be filled from `slots`:
    /// the English filled in, and the template's translation filled with
    /// each word's translation (or the word, if it has none). For lines put
    /// together from lists, such as a person's own sayings, whose slots sit
    /// side by side and could not be told apart by matching.
    pub fn expand(&self, template: &str, slots: &[(&str, &[&str])]) -> Vec<(String, String)> {
        // A line of slots alone reads in the same order in translation.
        let only_slots = pieces(template).iter().all(|piece| {
            matches!(piece, Piece::Slot(_))
                || matches!(piece, Piece::Words(words) if words.trim().is_empty())
        });
        let translated = match self
            .written
            .get(template)
            .or_else(|| self.exact.get(template))
        {
            Some(translated) => translated.clone(),
            None if only_slots => template.replace("} {", "}{"),
            None => return Vec::new(),
        };
        let mut made = vec![(template.to_string(), translated)];
        for (slot, words) in slots {
            let marker = format!("{{{slot}}}");
            if !template.contains(&marker) {
                continue;
            }
            made = made
                .into_iter()
                .flat_map(|(english, chinese)| {
                    words
                        .iter()
                        .filter_map(|word| {
                            // A word nobody translated is left out rather
                            // than shown in English inside the line; a name
                            // or a number reads the same in any language.
                            let shown = self
                                .exact(word)
                                .map(str::to_string)
                                .or_else(|| {
                                    self.translate(word)
                                        .filter(|shown| keeps_only_names(word, shown))
                                })
                                .or_else(|| reads_as_a_name(word).then(|| (*word).to_string()))?;
                            Some((
                                english.replace(&marker, word),
                                chinese.replace(&marker, &shown),
                            ))
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
        }
        made
    }

    pub fn len(&self) -> usize {
        self.exact.len() + self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `text` translated as far as this catalog knows how, a sentence at a
    /// time; `None` if nothing in it was known.
    pub fn translate(&self, text: &str) -> Option<String> {
        if let Some(found) = self.exact.get(text.trim()) {
            return Some(found.clone());
        }
        // Names or labels set side by side ("Mara · Emma"): each as the
        // catalog has it, a name as the language writes names.
        if text.contains(" · ") && self.whole(text, 0).is_none() {
            let parts = text
                .split(" · ")
                .map(|part| {
                    self.exact(part.trim())
                        .map(str::to_string)
                        .or_else(|| self.whole(part, 1))
                })
                .collect::<Vec<_>>();
            if parts.iter().any(Option::is_some) {
                return Some(
                    parts
                        .into_iter()
                        .zip(text.split(" · "))
                        .map(|(shown, part)| shown.unwrap_or_else(|| part.to_string()))
                        .collect::<Vec<_>>()
                        .join(" · "),
                );
            }
        }
        let sentences = split_sentences(text);
        if sentences.len() < 2 {
            return self.whole(text, 0);
        }
        let mut any = false;
        let mut out = String::new();
        let mut at = 0;
        while at < sentences.len() {
            // A run of sentences the catalog has as one line ("It hasn't
            // rung in fifty years. I still listen for it.") is taken whole,
            // the longest first, so a line said after another keeps the
            // translation it has on its own.
            let run = (at + 2..=sentences.len().min(at + 4))
                .rev()
                .find_map(|end| {
                    let from = offset(text, sentences[at]);
                    let last = sentences[end - 1];
                    let to = offset(text, last) + last.len();
                    let run = &text[from..to];
                    self.whole(run, 0)
                        .filter(|found| found != run)
                        .map(|found| (end, found))
                });
            let (next, found) = match run {
                Some((end, found)) => (end, Some(found)),
                None => (at + 1, self.whole(sentences[at], 0)),
            };
            match found {
                Some(found) => {
                    any = true;
                    out.push_str(&found);
                }
                None => {
                    if !out.is_empty() && !ends_wide(&out) {
                        out.push(' ');
                    }
                    out.push_str(sentences[at]);
                }
            }
            at = next;
        }
        any.then_some(out)
    }

    /// A whole piece of text: exactly, with its first letter's case
    /// turned, without its closing stop, or by a template whose slots are
    /// all filled by what can be shown too. Never part translated: what
    /// cannot be shown whole is `None`, and stays as it was written.
    fn whole(&self, text: &str, depth: usize) -> Option<String> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        if let Some(found) = self.exact.get(text) {
            return Some(found.clone());
        }
        let turned = turn_first(text);
        if let Some(found) = self.exact.get(&turned) {
            return Some(found.clone());
        }
        let bare = text.trim_end_matches(['.', '!', '?']);
        if bare.len() < text.len() {
            if let Some(found) = self
                .exact
                .get(bare)
                .or_else(|| self.exact.get(&turn_first(bare)))
            {
                let stop = &text[bare.len()..];
                return Some(format!("{found}{}", wide_stop(stop)));
            }
        } else {
            for stop in [".", "!"] {
                if let Some(found) = self.exact.get(&format!("{text}{stop}")) {
                    return Some(found.trim_end_matches(['。', '！', '.', '!']).to_string());
                }
            }
        }
        // "Wise!" where the catalog has "Wise." (and the other way), with
        // its own stop; templates are tried with it too.
        let swapped = swap_stop(text);
        if swapped != text {
            if let Some(found) = self
                .exact
                .get(&swapped)
                .or_else(|| self.exact.get(&turn_first(&swapped)))
            {
                let bare = found.trim_end_matches(['。', '！', '.', '!']);
                return Some(format!("{bare}{}", wide_stop(&text[text.len() - 1..])));
            }
        }
        if depth > 2 {
            return None;
        }
        // A line the catalog knows only without its closing full stop
        // ("Leo retired to the quay."), as a caption closes it: that line,
        // closed the translation's way. Only a whole line, never what
        // fills a slot, so the stop stays at the end.
        if depth == 0 {
            if let Some(bare) = text.strip_suffix('.').filter(|bare| !bare.ends_with('.')) {
                if let Some(found) = self.whole(bare, depth + 1) {
                    let found = found.trim_end_matches(['。', '.']);
                    return Some(format!("{found}。"));
                }
            }
        }
        // A sentence with a word tacked on the end (", mind.", ", love.")
        // is the sentence, then that word. Tried before the templates when
        // the catalog knows the ending by itself, so a template's last slot
        // does not take the ending in with it.
        if let Some(found) = self.with_ending(text, depth) {
            return Some(found);
        }
        // A template fits when what fills its slots can be shown too:
        // translated, or a name or a number, which read the same in any
        // language. The first such fit wins.
        let sentences = split_sentences(text).len();
        for template in &self.templates {
            // A template is as many sentences as the text it fits.
            if template.sentences != sentences
                || !text.contains(template.anchor.as_str())
                    && !turned.contains(template.anchor.as_str())
                    && !swapped.contains(template.anchor.as_str())
            {
                continue;
            }
            // A first letter turned is for the template's own first word:
            // a slot's words are taken as they are written.
            let starts_with_slot = matches!(template.pieces.first(), Some(Piece::Slot(_)));
            for candidate in [text, turned.as_str(), swapped.as_str()] {
                if starts_with_slot && candidate == turned && turned != text {
                    continue;
                }
                for slots in fits(&template.pieces, candidate) {
                    // No slot holds more than one sentence.
                    if slots
                        .iter()
                        .any(|(_, value)| split_sentences(value).len() > 1)
                    {
                        continue;
                    }
                    let mut out = template.translation.clone();
                    if candidate == swapped && swapped != text {
                        out = match text.chars().last() {
                            Some('!') => out.replacen('。', "！", 1),
                            _ => out.replacen('！', "。", 1),
                        };
                    }
                    let mut clean = true;
                    for (name, value) in slots {
                        let shown = match self.whole(value, depth + 1) {
                            Some(shown) => shown,
                            None => {
                                // "I" is the speaker, not a name: read
                                // below as the language says "I".
                                clean &= reads_as_a_name(value) && value != "I";
                                // A list of names is listed the Chinese way,
                                // each name as the catalog writes it.
                                value
                                    .split(", ")
                                    .map(|name| {
                                        self.exact(name).map_or(name.to_string(), str::to_string)
                                    })
                                    .collect::<Vec<_>>()
                                    .join("、")
                            }
                        };
                        // Unnamed slots are filled in order, one each.
                        let marker = format!("{{{name}}}");
                        out = if name.is_empty() {
                            out.replacen(&marker, &shown, 1)
                        } else {
                            out.replace(&marker, &shown)
                        };
                    }
                    if clean {
                        return Some(out);
                    }
                }
            }
        }
        // Said of oneself ("I went fishing with Miri", "Leo gave me a
        // gift"): read as told of anyone, with the speaker put in as a
        // name, then said as the language says "I".
        if let Some(myself) = self.myself.as_ref().filter(|_| depth <= 1) {
            let told = text
                .split(' ')
                .map(|word| {
                    let bare = word.trim_end_matches(|c: char| !c.is_alphanumeric());
                    match bare {
                        "I" | "me" => format!("{MYSELF}{}", &word[bare.len()..]),
                        _ => word.to_string(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            if told != text {
                // Only where the speaker is still in it after.
                if let Some(found) = self
                    .whole(&told, depth + 1)
                    .filter(|found| found.contains(MYSELF))
                {
                    return Some(found.replace(MYSELF, myself));
                }
            }
        }
        // Said in someone's own words ("Nest to Nessa now."): read with the
        // words they say instead turned back, one at a time and then all.
        if depth <= 1 && sentences == 1 {
            let mut all = text.to_string();
            for (said, meant) in &self.instead {
                let back = replace_word(text, said, meant);
                if back == text {
                    continue;
                }
                if let Some(found) = self.whole(&back, depth + 1) {
                    return Some(found);
                }
                all = replace_word(&all, said, meant);
            }
            if all != text {
                if let Some(found) = self.whole(&all, depth + 1) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// `text` as a sentence with an ending of its own (", mind.",
    /// ", love!"), if the catalog knows both the sentence and the ending.
    fn with_ending(&self, text: &str, depth: usize) -> Option<String> {
        let comma = text.rfind(", ")?;
        let (head, tail) = (&text[..comma], &text[comma..]);
        if tail.len() > 24 || head.is_empty() {
            return None;
        }
        let stop = text
            .chars()
            .last()
            .filter(|stop| matches!(stop, '!' | '?' | '.'))
            .unwrap_or('.');
        let bare = tail.trim_end_matches(['.', '!', '?']);
        let ending = [
            tail.to_string(),
            format!("{bare}."),
            format!("{bare}!"),
            format!("{bare}?"),
        ]
        .into_iter()
        .find_map(|tail| self.exact.get(&tail))?;
        // A question's ending ("..., eh?") hangs on a plain sentence.
        let head_stop = if stop == '!' { '!' } else { '.' };
        let head = self.whole(&format!("{head}{head_stop}"), depth + 1)?;
        let head = head.trim_end_matches(['。', '！', '？', '.', '!', '?']);
        let ending = ending.trim_end_matches(['。', '！', '？', '.', '!', '?']);
        Some(format!("{head}{ending}{}", wide_stop(&stop.to_string())))
    }
}

/// Whether text left as it is reads as a name or a number, or a list of
/// them: a few words each, every one starting with a capital or a digit
/// ("Harbor Bakery", "Lin Mei", "12", "Tobias, Olek, Hana"), and no
/// sentence in it.
fn reads_as_a_name(text: &str) -> bool {
    text.split(", ").all(|name| {
        let words = name
            .split(|c: char| c.is_whitespace() || c == '-')
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>();
        !words.is_empty()
            && words.len() <= 4
            && !name.contains(['.', '!', '?', ',', ':', ';'])
            && words.iter().all(|word| {
                word.chars()
                    .next()
                    .is_some_and(|first| first.is_uppercase() || first.is_ascii_digit())
            })
    })
}

/// `text` with each whole word `word` (in any case) replaced by `with`,
/// keeping a capital at its start.
fn replace_word(text: &str, word: &str, with: &str) -> String {
    let lower = text.to_lowercase();
    if lower.len() != text.len() {
        return text.to_string();
    }
    let mut out = String::new();
    let (mut last, mut from) = (0, 0);
    while let Some(found) = lower[from..].find(word) {
        let start = from + found;
        let end = start + word.len();
        let before = lower[..start].chars().next_back();
        let after = lower[end..].chars().next();
        if before.is_none_or(|c| !c.is_alphanumeric()) && after.is_none_or(|c| !c.is_alphanumeric())
        {
            out.push_str(&text[last..start]);
            if text[start..].starts_with(|c: char| c.is_uppercase()) {
                out.push_str(&turn_first(with));
            } else {
                out.push_str(with);
            }
            last = end;
        }
        from = end;
    }
    out.push_str(&text[last..]);
    out
}

/// Whether `shown`, a translation of `text`, has no words left in Latin
/// letters but the names `text` has.
fn keeps_only_names(text: &str, shown: &str) -> bool {
    shown
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|word| word.len() > 1)
        .all(|word| {
            word.starts_with(|c: char| c.is_ascii_uppercase())
                && text
                    .split(|c: char| !c.is_ascii_alphabetic())
                    .any(|own| own == word)
        })
}

/// Every way `text` fits the template's pieces, the likeliest first: what
/// fills each slot, the words in order, the first at the start and the
/// last at the end. Words between two slots may be found more than once
/// ("{count} {fish}"), so each place they are found is tried in turn.
fn fits<'a>(pieces: &'a [Piece], text: &'a str) -> Vec<Vec<(&'a str, &'a str)>> {
    let mut out = Vec::new();
    fit_from(pieces, text, 0, 0, None, &mut Vec::new(), &mut out);
    out
}

fn fit_from<'a>(
    pieces: &'a [Piece],
    text: &'a str,
    index: usize,
    at: usize,
    open: Option<&'a str>,
    slots: &mut Vec<(&'a str, &'a str)>,
    out: &mut Vec<Vec<(&'a str, &'a str)>>,
) {
    if out.len() >= 8 {
        return;
    }
    let Some(piece) = pieces.get(index) else {
        match open {
            Some(slot) => {
                let value = &text[at..];
                if !value.trim().is_empty() {
                    slots.push((slot, value.trim()));
                    out.push(slots.clone());
                    slots.pop();
                }
            }
            None if at == text.len() => out.push(slots.clone()),
            None => {}
        }
        return;
    };
    match piece {
        Piece::Slot(name) => {
            // Two slots side by side cannot be told apart.
            if open.is_none() {
                fit_from(pieces, text, index + 1, at, Some(name), slots, out);
            }
        }
        Piece::Words(words) => {
            let last = index + 1 == pieces.len();
            let found: Vec<usize> = if open.is_none() {
                text[at..]
                    .starts_with(words.as_str())
                    .then_some(at)
                    .into_iter()
                    .collect()
            } else if last {
                text[at..]
                    .ends_with(words.as_str())
                    .then(|| text.len() - words.len())
                    .filter(|found| *found >= at)
                    .into_iter()
                    .collect()
            } else {
                text[at..]
                    .match_indices(words.as_str())
                    .map(|(found, _)| at + found)
                    .collect()
            };
            for found in found {
                let pushed = match open {
                    Some(slot) => {
                        let value = &text[at..found];
                        if value.trim().is_empty() {
                            continue;
                        }
                        slots.push((slot, value.trim()));
                        true
                    }
                    None => false,
                };
                fit_from(
                    pieces,
                    text,
                    index + 1,
                    found + words.len(),
                    None,
                    slots,
                    out,
                );
                if pushed {
                    slots.pop();
                }
            }
        }
    }
}

/// Where `part`, a slice of `text`, starts in it.
fn offset(text: &str, part: &str) -> usize {
    part.as_ptr() as usize - text.as_ptr() as usize
}

fn swap_stop(text: &str) -> String {
    match text.chars().last() {
        Some('!') => format!("{}.", &text[..text.len() - 1]),
        Some('.') => format!("{}!", &text[..text.len() - 1]),
        _ => text.to_string(),
    }
}

fn turn_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.is_uppercase() => first.to_lowercase().chain(chars).collect(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn wide_stop(stop: &str) -> &'static str {
    match stop.chars().next() {
        Some('!') => "！",
        Some('?') => "？",
        _ => "。",
    }
}

fn ends_wide(text: &str) -> bool {
    text.chars().last().is_some_and(|last| !last.is_ascii())
}

/// Sentences, each with its closing stop, split after `.`, `!` or `?`
/// and a space.
fn split_sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    for index in 0..bytes.len() {
        if matches!(bytes[index], b'.' | b'!' | b'?')
            && bytes.get(index + 1) == Some(&b' ')
            && bytes
                .get(index + 2)
                .is_some_and(|next| next.is_ascii_uppercase() || *next == b'"')
        {
            out.push(text[start..=index].trim());
            start = index + 2;
        }
    }
    if start < text.len() {
        out.push(text[start..].trim());
    }
    out.into_iter().filter(|part| !part.is_empty()).collect()
}

fn unescape(text: &str) -> String {
    text.replace("\\n", "\n").replace("\\t", "\t")
}

/// The languages the app can be shown in.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Language {
    #[default]
    English,
    SimplifiedChinese,
    Japanese,
}

impl Language {
    /// Every language, in the order a picker lists them.
    pub const ALL: [Language; 3] = [
        Language::English,
        Language::SimplifiedChinese,
        Language::Japanese,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::SimplifiedChinese => "zh-Hans",
            Language::Japanese => "ja",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        let id = id.trim().to_lowercase();
        if id.starts_with("zh-hans")
            || id == "zh"
            || id.starts_with("zh-cn")
            || id.starts_with("zh_cn")
        {
            Some(Language::SimplifiedChinese)
        } else if id == "ja" || id.starts_with("ja-") || id.starts_with("ja_") {
            Some(Language::Japanese)
        } else if id.starts_with("en") {
            Some(Language::English)
        } else {
            None
        }
    }

    /// What the language is called, in itself.
    pub fn name(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::SimplifiedChinese => "简体中文",
            Language::Japanese => "日本語",
        }
    }

    /// Whether it is written without spaces between words, as Chinese and
    /// Japanese are.
    pub fn without_spaces(self) -> bool {
        matches!(self, Language::SimplifiedChinese | Language::Japanese)
    }
}

struct State {
    language: Language,
    /// A catalog for each language the app can be shown in but English.
    catalogs: HashMap<Language, Catalog>,
}

fn state() -> &'static RwLock<State> {
    static STATE: OnceLock<RwLock<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        RwLock::new(State {
            language: Language::English,
            catalogs: HashMap::new(),
        })
    })
}

type Memo = HashMap<(Language, String), Option<String>>;

fn memo() -> &'static Mutex<Memo> {
    static MEMO: OnceLock<Mutex<Memo>> = OnceLock::new();
    MEMO.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Adds lines to the Simplified Chinese catalog.
pub fn install(text: &str) {
    install_for(Language::SimplifiedChinese, text);
}

/// Adds lines to the catalog of `language`.
pub fn install_for(language: Language, text: &str) {
    if language == Language::English {
        return;
    }
    if let Ok(mut state) = state().write() {
        state.catalogs.entry(language).or_default().extend(text);
    }
    if let Ok(mut memo) = memo().lock() {
        memo.retain(|(shown_in, _), _| *shown_in != language);
    }
}

/// Shows the app in `language` from now on.
pub fn set_language(language: Language) {
    if let Ok(mut state) = state().write() {
        state.language = language;
    }
}

thread_local! {
    static THIS_THREAD: std::cell::Cell<Option<Language>> = const { std::cell::Cell::new(None) };
}

/// Shows everything this thread draws in `language` (or, with `None`,
/// in the app's language again), whatever the rest of the app is shown
/// in. For a test that walks a window in one language while other tests
/// run beside it in another.
pub fn set_thread_language(language: Option<Language>) {
    THIS_THREAD.with(|this| this.set(language));
}

pub fn language() -> Language {
    if let Some(language) = THIS_THREAD.with(std::cell::Cell::get) {
        return language;
    }
    state()
        .read()
        .map(|state| state.language)
        .unwrap_or_default()
}

/// `text` in the language the app is shown in, as far as it is known.
pub fn tr(text: &str) -> Cow<'_, str> {
    let shown_in = language();
    if shown_in == Language::English || text.trim().is_empty() {
        return Cow::Borrowed(text);
    }
    let key = (shown_in, text.to_string());
    if let Some(found) = memo().lock().ok().and_then(|memo| memo.get(&key).cloned()) {
        return match found {
            Some(found) => Cow::Owned(found),
            None => Cow::Borrowed(text),
        };
    }
    let found = state().read().ok().and_then(|state| {
        state
            .catalogs
            .get(&shown_in)
            .and_then(|catalog| catalog.translate(text))
    });
    if let Ok(mut memo) = memo().lock() {
        if memo.len() > 20_000 {
            memo.clear();
        }
        memo.insert(key, found.clone());
    }
    match found {
        Some(found) => Cow::Owned(found),
        None => Cow::Borrowed(text),
    }
}

/// The same, owned.
pub fn tr_owned(text: &str) -> String {
    tr(text).into_owned()
}

/// The language this Mac is set to, if the app has a catalog for it:
/// `LANG` first, then the system's own list.
pub fn system_language() -> Language {
    if let Some(language) = std::env::var("WORLD_MACHINE_LANGUAGE")
        .ok()
        .and_then(|id| Language::from_id(&id))
    {
        return language;
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("/usr/bin/defaults")
            .args(["read", "-g", "AppleLanguages"])
            .output()
        {
            let listed = String::from_utf8_lossy(&output.stdout);
            if let Some(first) = listed
                .split(|c: char| c == '"' || c == ',' || c.is_whitespace() || c == '(' || c == ')')
                .find(|part| !part.is_empty())
            {
                return Language::from_id(first).unwrap_or_default();
            }
        }
    }
    std::env::var("LANG")
        .ok()
        .and_then(|id| Language::from_id(&id))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATALOG: &str = "\
Bench\t长椅
Harbor Bakery\t港口面包房
You built a {} by {at}\t你在{at}旁边造了一个{}
Morning!\t早上好！
Thank you.\t谢谢。
Keepsakes · {}\t纪念品 · {}
Mara gave you {what}\tMara送了你{what}
a hand-drawn map of the {settlement}\t一张手绘的{settlement}地图
harbour\t港湾
id_like_this\tSKIP
";

    /// Names in a list, or set side by side, are each written as the
    /// language writes names.
    #[test]
    fn names_in_lists_are_each_written_the_languages_way() {
        let catalog = Catalog::parse(
            "Hana\tハナ\nRosa\tローザ\nTobias\tトビアス\nMara\tマーラ\nEmma\tエマ\n\
             {} and {} settled in the harbour for good\t{}と{}は港に腰を落ち着けた\n",
        );
        assert_eq!(
            catalog
                .translate("Hana, Rosa and Tobias settled in the harbour for good")
                .as_deref(),
            Some("ハナ、ローザとトビアスは港に腰を落ち着けた")
        );
        assert_eq!(
            catalog.translate("Mara · Emma").as_deref(),
            Some("マーラ · エマ")
        );
        assert_eq!(
            catalog.translate("Mara · Lena").as_deref(),
            Some("マーラ · Lena")
        );
    }

    /// A line someone writes of themselves reads as the line told of
    /// anyone, with "I" as the language says it.
    #[test]
    fn a_line_told_of_oneself_is_read_as_told_of_anyone() {
        let catalog = Catalog::parse(
            "@I\t我\n{name} went fishing with {other}\t{name}和{other}去钓鱼了\n\
             {a} gave {b} a gift\t{a}送了{b}一份礼物\n",
        );
        assert_eq!(
            catalog.translate("I went fishing with Miri").as_deref(),
            Some("我和Miri去钓鱼了")
        );
        assert_eq!(
            catalog.translate("Leo gave me a gift").as_deref(),
            Some("Leo送了我一份礼物")
        );
        assert_eq!(catalog.translate("I went sailing").as_deref(), None);
    }

    #[test]
    fn whole_lines_and_templates_are_translated() {
        let catalog = Catalog::parse(CATALOG);
        assert_eq!(catalog.translate("Bench").as_deref(), Some("长椅"));
        assert_eq!(
            catalog
                .translate("You built a bench by Harbor Bakery")
                .as_deref(),
            Some("你在港口面包房旁边造了一个长椅")
        );
        assert_eq!(
            catalog.translate("Keepsakes · 4").as_deref(),
            Some("纪念品 · 4")
        );
        assert_eq!(
            catalog
                .translate("Mara gave you a hand-drawn map of the harbour")
                .as_deref(),
            Some("Mara送了你一张手绘的港湾地图")
        );
        assert_eq!(catalog.translate("id_like_this"), None);
        assert_eq!(catalog.translate("Something nobody wrote"), None);
    }

    #[test]
    fn a_line_closed_with_a_full_stop_is_the_line_it_closes() {
        let catalog = Catalog::parse(CATALOG);
        assert_eq!(
            catalog
                .translate("You built a bench by Harbor Bakery.")
                .as_deref(),
            Some("你在港口面包房旁边造了一个长椅。")
        );
        assert_eq!(catalog.translate("Thank you.").as_deref(), Some("谢谢。"));
        assert_eq!(catalog.translate("Something nobody wrote."), None);
    }

    #[test]
    fn a_line_is_translated_a_sentence_at_a_time() {
        let catalog = Catalog::parse(CATALOG);
        assert_eq!(
            catalog.translate("Morning! Thank you.").as_deref(),
            Some("早上好！谢谢。")
        );
        // What nobody translated stays as it is, beside what they did.
        assert_eq!(
            catalog.translate("Morning! Who knows.").as_deref(),
            Some("早上好！Who knows.")
        );
        // A first letter turned by a line said mid-sentence still matches.
        assert_eq!(catalog.translate("bench").as_deref(), Some("长椅"));
        assert_eq!(catalog.translate("Thank you").as_deref(), Some("谢谢"));
    }

    #[test]
    fn the_language_is_chosen_and_english_is_untouched() {
        install(CATALOG);
        set_language(Language::English);
        assert_eq!(tr("Bench"), "Bench");
        set_language(Language::SimplifiedChinese);
        assert_eq!(tr("Bench"), "长椅");
        assert_eq!(tr("Unknown words"), "Unknown words");
        set_language(Language::English);
        assert_eq!(
            Language::from_id("zh-Hans-CN"),
            Some(Language::SimplifiedChinese)
        );
        assert_eq!(Language::from_id("en-GB"), Some(Language::English));
        assert_eq!(Language::from_id("fr"), None);
        assert_eq!(Language::from_id("ja-JP"), Some(Language::Japanese));
        assert_eq!(Language::from_id("ja"), Some(Language::Japanese));
    }

    #[test]
    fn each_language_has_its_own_catalog() {
        install("Bench\t长椅\n");
        install_for(Language::Japanese, "Bench\tベンチ\nMorning!\tおはよう！\n");
        set_thread_language(Some(Language::Japanese));
        assert_eq!(tr("Bench"), "ベンチ");
        assert_eq!(tr("Morning! Bench."), "おはよう！ベンチ。");
        set_thread_language(Some(Language::SimplifiedChinese));
        assert_eq!(tr("Bench"), "长椅");
        set_thread_language(None);
        assert!(Language::Japanese.without_spaces());
        assert_eq!(Language::Japanese.name(), "日本語");
    }

    const WHOLE: &str = "\
{told}, bigger than last year, around the {made} you made\t{told}，比去年更热闹，就在你做的{made}旁边
The swallows came back\t燕子回来了
A year ago today: {told}.\t一年前的今天：{told}。
bench\t长椅
{a} sends their love.\t{a}向你问好。
, mind.\t，记着。
Home to {name} now.\t现在回{name}的家了。
It hasn't rung in fifty years. I still listen for it.\t五十年没响过了。我还是会留神听。
Funny, we've never talked about the bell.\t说来好笑，我们从没聊过那口钟。
{count} {fish} in the vault.\t鱼库里有{count}{fish}。
herring\t鲱鱼
{} and {last} settled in for good\t{}和{last}安了家
=nest\thome
";

    #[test]
    fn a_line_made_of_parts_is_one_sentence_with_named_slots() {
        let catalog = Catalog::parse(WHOLE);
        assert_eq!(
            catalog
                .translate(
                    "A year ago today: The swallows came back, bigger than last year, around the bench you made."
                )
                .as_deref(),
            Some("一年前的今天：燕子回来了，比去年更热闹，就在你做的长椅旁边。")
        );
        // An ending of someone's own is not taken into a slot.
        assert_eq!(
            catalog.translate("Mara sends their love, mind.").as_deref(),
            Some("Mara向你问好，记着。")
        );
        // "!" where the catalog has "." keeps its own stop.
        assert_eq!(
            catalog.translate("Mara sends their love!").as_deref(),
            Some("Mara向你问好！")
        );
    }

    #[test]
    fn a_run_of_sentences_written_together_is_taken_whole() {
        let catalog = Catalog::parse(WHOLE);
        assert_eq!(
            catalog
                .translate(
                    "Funny, we've never talked about the bell. It hasn't rung in fifty years. I still listen for it."
                )
                .as_deref(),
            Some("说来好笑，我们从没聊过那口钟。五十年没响过了。我还是会留神听。")
        );
    }

    #[test]
    fn slots_are_filled_only_by_what_can_be_shown() {
        let catalog = Catalog::parse(WHOLE);
        // Words between two slots are tried at each place they are found.
        assert_eq!(
            catalog.translate("12 herring in the vault.").as_deref(),
            Some("鱼库里有12鲱鱼。")
        );
        // Names are kept as they are written, and a list of them is listed
        // the Chinese way.
        assert_eq!(
            catalog
                .translate("Tobias, Olek and Rosa settled in for good")
                .as_deref(),
            Some("Tobias、Olek和Rosa安了家")
        );
        // A slot that would hold words nobody translated does not fit.
        assert_eq!(catalog.translate("Home to the old red house now."), None);
    }

    #[test]
    fn a_speakers_own_words_are_read_as_the_plain_ones() {
        let catalog = Catalog::parse(WHOLE);
        assert_eq!(
            catalog.translate("Nest to Pip now.").as_deref(),
            Some("现在回Pip的家了。")
        );
    }
}
