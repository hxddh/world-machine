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
//! lines from sentences; anything no catalog knows is shown as it is.

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
}

/// Lines in one language and their translation into another.
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    exact: HashMap<String, String>,
    templates: Vec<Template>,
    /// Lines with slots, as written, for filling them in every way.
    written: HashMap<String, String>,
}

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
        self.templates.retain(|template| template.pieces != pieces);
        self.templates.push(Template {
            pieces,
            translation,
            anchor,
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
                        .map(|word| {
                            let shown = self
                                .exact(word)
                                .map(str::to_string)
                                .or_else(|| self.whole(word, 1))
                                .unwrap_or_else(|| (*word).to_string());
                            (
                                english.replace(&marker, word),
                                chinese.replace(&marker, &shown),
                            )
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
        let sentences = split_sentences(text);
        if sentences.len() < 2 {
            return self.whole(text, 0);
        }
        let mut any = false;
        let mut out = String::new();
        for sentence in sentences {
            match self.whole(sentence, 0) {
                Some(found) => {
                    any = true;
                    out.push_str(&found);
                }
                None => {
                    if !out.is_empty() && !ends_wide(&out) {
                        out.push(' ');
                    }
                    out.push_str(sentence);
                }
            }
        }
        any.then_some(out)
    }

    /// A whole piece of text: exactly, with its first letter's case
    /// turned, without its closing stop, or by a template.
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
        if depth > 2 {
            return None;
        }
        for template in &self.templates {
            if !text.contains(template.anchor.as_str())
                && !turned.contains(template.anchor.as_str())
            {
                continue;
            }
            // "!" where the line has "." (and the other way) still fits,
            // and keeps its own stop.
            let swapped = swap_stop(text);
            for candidate in [text, turned.as_str(), swapped.as_str()] {
                if let Some(slots) = fit(&template.pieces, candidate) {
                    let mut out = template.translation.clone();
                    if candidate == swapped && swapped != text {
                        out = match text.chars().last() {
                            Some('!') => out.replacen('。', "！", 1),
                            _ => out.replacen('！', "。", 1),
                        };
                    }
                    for (name, value) in slots {
                        let shown = self
                            .whole(value, depth + 1)
                            .unwrap_or_else(|| value.to_string());
                        out = out.replace(&format!("{{{name}}}"), &shown);
                    }
                    return Some(out);
                }
            }
        }
        // A sentence with a word tacked on the end (", mind.", ", love.")
        // is the sentence, then that word.
        if let Some(comma) = text.rfind(", ") {
            let (head, tail) = (&text[..comma], &text[comma..]);
            if tail.len() <= 24 {
                let stop = if text.ends_with('!') { "!" } else { "." };
                if let (Some(head), Some(tail)) = (
                    self.whole(&format!("{head}{stop}"), depth + 1),
                    self.exact.get(tail),
                ) {
                    let head = head.trim_end_matches(['。', '！', '.', '!']);
                    return Some(format!("{head}{tail}"));
                }
            }
        }
        None
    }
}

/// What fills each slot, if `text` fits the template's pieces: the words
/// in order, the first at the start and the last at the end.
fn fit<'a>(pieces: &'a [Piece], text: &'a str) -> Option<Vec<(&'a str, &'a str)>> {
    let mut slots = Vec::new();
    let mut at = 0;
    let mut open: Option<&str> = None;
    for (index, piece) in pieces.iter().enumerate() {
        match piece {
            Piece::Words(words) => {
                let last = index + 1 == pieces.len();
                let found = if open.is_none() {
                    text[at..].starts_with(words.as_str()).then_some(at)
                } else if last {
                    text[at..]
                        .ends_with(words.as_str())
                        .then(|| text.len() - words.len())
                        .filter(|found| *found >= at)
                } else {
                    text[at..].find(words.as_str()).map(|found| at + found)
                }?;
                if let Some(slot) = open.take() {
                    let value = &text[at..found];
                    if value.trim().is_empty() {
                        return None;
                    }
                    slots.push((slot, value.trim()));
                }
                at = found + words.len();
            }
            Piece::Slot(name) => {
                if open.is_some() {
                    // Two slots side by side cannot be told apart.
                    return None;
                }
                open = Some(name.as_str());
            }
        }
    }
    match open {
        Some(slot) => {
            let value = &text[at..];
            if value.trim().is_empty() {
                return None;
            }
            slots.push((slot, value.trim()));
        }
        None if at != text.len() => return None,
        None => {}
    }
    Some(slots)
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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Language {
    #[default]
    English,
    SimplifiedChinese,
}

impl Language {
    pub fn id(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::SimplifiedChinese => "zh-Hans",
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
        }
    }
}

struct State {
    language: Language,
    catalog: Catalog,
}

fn state() -> &'static RwLock<State> {
    static STATE: OnceLock<RwLock<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        RwLock::new(State {
            language: Language::English,
            catalog: Catalog::default(),
        })
    })
}

fn memo() -> &'static Mutex<HashMap<String, Option<String>>> {
    static MEMO: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    MEMO.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Adds lines to the Simplified Chinese catalog.
pub fn install(text: &str) {
    if let Ok(mut state) = state().write() {
        state.catalog.extend(text);
    }
    if let Ok(mut memo) = memo().lock() {
        memo.clear();
    }
}

/// Shows the app in `language` from now on.
pub fn set_language(language: Language) {
    if let Ok(mut state) = state().write() {
        state.language = language;
    }
}

pub fn language() -> Language {
    state()
        .read()
        .map(|state| state.language)
        .unwrap_or_default()
}

/// `text` in the language the app is shown in, as far as it is known.
pub fn tr(text: &str) -> Cow<'_, str> {
    if language() == Language::English || text.trim().is_empty() {
        return Cow::Borrowed(text);
    }
    if let Some(found) = memo().lock().ok().and_then(|memo| memo.get(text).cloned()) {
        return match found {
            Some(found) => Cow::Owned(found),
            None => Cow::Borrowed(text),
        };
    }
    let found = state()
        .read()
        .ok()
        .and_then(|state| state.catalog.translate(text));
    if let Ok(mut memo) = memo().lock() {
        if memo.len() > 20_000 {
            memo.clear();
        }
        memo.insert(text.to_string(), found.clone());
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
    }
}
