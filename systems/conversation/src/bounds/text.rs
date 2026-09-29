//! Reading an answer the way the bounds need it: folded to one form
//! (full-width letters to plain ones, curly quotes to straight, lower
//! case), cut into sentences, and cut into words where words are spaced
//! (English) or kept as a run of characters where they are not (Chinese).

/// Whether a character is written in Chinese characters.
pub(super) fn is_han(c: char) -> bool {
    matches!(c as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x2A6DF)
}

/// Whether a character is a letter of a script neither English nor
/// Chinese: kana, hangul, Cyrillic, Greek, Arabic, Hebrew, Thai,
/// Devanagari.
pub(super) fn is_other_script(c: char) -> bool {
    // The middle dot and long mark Chinese also writes foreign names with.
    if matches!(c, '\u{30FB}' | '\u{30FC}') {
        return false;
    }
    matches!(c as u32,
        0x3040..=0x30FF | 0xAC00..=0xD7AF | 0x1100..=0x11FF | 0x0400..=0x04FF
        | 0x0370..=0x03FF | 0x0600..=0x06FF | 0x0590..=0x05FF | 0x0E00..=0x0E7F
        | 0x0900..=0x097F)
}

/// Whether a character is kana: Japanese, never Chinese or English.
pub(super) fn is_kana(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30FA)
}

/// Whether a letter carries a mark English does not write: ñ, ç, ü, é, ß.
pub(super) fn is_marked_latin(c: char) -> bool {
    matches!(c as u32, 0x00C0..=0x017F) && c != '\u{00D7}' && c != '\u{00F7}'
        || matches!(c, '¿' | '¡')
}

/// Whether a character is a picture rather than a letter: emoji and the
/// symbols chat puts in speech.
pub(super) fn is_pictograph(c: char) -> bool {
    matches!(c as u32,
        0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0x2B00..=0x2BFF | 0xFE0F | 0x200D
        | 0x2190..=0x21FF | 0x2460..=0x24FF | 0x25A0..=0x25FF)
}

/// One character in the form every check reads: full-width letters,
/// digits and punctuation as their plain forms, curly quotes straight,
/// the ideographic space a space.
pub(super) fn fold(c: char) -> char {
    match c {
        '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
        '\u{3000}' => ' ',
        '\u{2018}' | '\u{2019}' | '\u{201B}' | '\u{02BC}' | '\u{2032}' => '\'',
        '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{2033}' => '"',
        '\u{2010}' | '\u{2011}' => '-',
        _ => c,
    }
}

/// Words written short with a full stop that does not end a sentence.
const ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "st", "prof", "capt", "sgt", "lt", "gen", "col", "rev", "jr", "sr",
    "mt", "no", "vs", "etc",
];

/// Where one sentence ends and the next begins.
fn ends_sentence(c: char) -> bool {
    matches!(
        c,
        '.' | '!' | '?' | ';' | '\n' | '。' | '！' | '？' | '；' | '…'
    )
}

fn joins_word(c: char) -> bool {
    c == '\'' || c == '-'
}

fn word_char(c: char) -> bool {
    c.is_alphanumeric() && !is_han(c) && !is_other_script(c)
}

/// A word in spaced writing.
#[derive(Clone, Debug)]
pub(super) struct Word {
    /// In lower case, without a possessive `'s`.
    pub text: String,
    /// Written with a capital first letter.
    pub capital: bool,
    /// The first word of its sentence.
    pub first: bool,
    /// Written with a possessive `'s`: someone's, not someone.
    pub possessive: bool,
    /// Written with a letter English does not use (ñ, é, ü).
    pub marked: bool,
    /// Followed by a comma, a colon or a dash: a phrase ends here.
    pub pause: bool,
    pub sentence: usize,
}

/// An answer, read.
#[derive(Clone, Debug, Default)]
pub(super) struct Text {
    /// Folded and in lower case: what phrases and characters are found in.
    pub norm: String,
    /// Each sentence's byte range in `norm`.
    pub sentences: Vec<(usize, usize)>,
    pub words: Vec<Word>,
    /// How many Chinese characters it has.
    pub han: usize,
    /// How many letters of some third script it has.
    pub other_script: usize,
    /// How many of those are kana.
    pub kana: usize,
    pub pictographs: usize,
}

/// A word without the possessive it may carry.
pub(super) fn base(word: &str) -> &str {
    word.strip_suffix("'s")
        .or_else(|| word.strip_suffix("s'"))
        .unwrap_or(word)
}

impl Text {
    pub fn read(raw: &str) -> Self {
        let mut text = Text::default();
        let mut sentence_start = 0;
        let mut sentence = 0;
        let mut at_start = true;
        let mut current = String::new();
        let flush = |current: &mut String, text: &mut Text, at_start: &mut bool, sentence| {
            let trimmed = current.trim_matches(joins_word);
            if !trimmed.is_empty() {
                let capital = trimmed.chars().next().is_some_and(char::is_uppercase);
                let lower = trimmed.to_lowercase();
                let stripped = base(&lower);
                text.words.push(Word {
                    text: stripped.to_string(),
                    capital,
                    first: *at_start,
                    possessive: stripped.len() < lower.len(),
                    marked: trimmed.chars().any(is_marked_latin),
                    pause: false,
                    sentence,
                });
                *at_start = false;
            }
            current.clear();
        };
        let chars = raw.chars().map(fold).collect::<Vec<_>>();
        for (index, &c) in chars.iter().enumerate() {
            if is_han(c) {
                text.han += 1;
            } else if is_other_script(c) {
                text.other_script += 1;
                if is_kana(c) {
                    text.kana += 1;
                }
            } else if is_pictograph(c) {
                text.pictographs += 1;
            }
            let next = chars.get(index + 1).copied();
            let word_before = current.to_lowercase();
            let inner_join = joins_word(c) && !current.is_empty() && next.is_some_and(word_char);
            if word_char(c) || inner_join {
                current.push(c);
            } else {
                flush(&mut current, &mut text, &mut at_start, sentence);
                if is_han(c) {
                    at_start = false;
                }
                if matches!(c, ',' | ':' | '\u{2014}' | '\u{2013}' | '\u{3001}') {
                    if let Some(last) = text.words.last_mut().filter(|w| w.sentence == sentence) {
                        last.pause = true;
                    }
                }
            }
            for lower in c.to_lowercase() {
                text.norm.push(lower);
            }
            // A full stop inside a number or an abbreviation of single
            // letters ("a.m.") does not end a sentence.
            let inside = c == '.'
                && (next.is_some_and(|n| n.is_alphanumeric() && !is_han(n))
                    && index > 0
                    && chars[index - 1].is_alphanumeric()
                    || ABBREVIATIONS.contains(&word_before.as_str())
                    || (word_before.chars().count() == 1
                        && index > 0
                        && chars[index - 1].is_uppercase()));
            if ends_sentence(c) && !inside {
                flush(&mut current, &mut text, &mut at_start, sentence);
                text.sentences.push((sentence_start, text.norm.len()));
                sentence_start = text.norm.len();
                sentence += 1;
                at_start = true;
            }
        }
        flush(&mut current, &mut text, &mut at_start, sentence);
        if sentence_start < text.norm.len() {
            text.sentences.push((sentence_start, text.norm.len()));
        }
        text
    }

    /// The sentence a byte of `norm` is in.
    pub fn sentence_at(&self, byte: usize) -> usize {
        self.sentences
            .iter()
            .position(|(start, end)| byte >= *start && byte < *end)
            .unwrap_or(self.sentences.len().saturating_sub(1))
    }

    /// Where a phrase of spaced words starts, by word index. A word of
    /// the phrase ending in `*` matches any word it begins.
    pub fn find(&self, phrase: &str) -> Vec<usize> {
        let parts = phrase.split(' ').collect::<Vec<_>>();
        if parts.is_empty() || parts.len() > self.words.len() {
            return Vec::new();
        }
        (0..=self.words.len() - parts.len())
            .filter(|start| {
                parts.iter().enumerate().all(|(offset, part)| {
                    let word = &self.words[start + offset];
                    if offset > 0 && word.sentence != self.words[*start].sentence {
                        return false;
                    }
                    match part.strip_suffix('*') {
                        Some(stem) => word.text.starts_with(stem),
                        None => word.text == *part,
                    }
                })
            })
            .collect()
    }

    pub fn has(&self, phrase: &str) -> bool {
        !self.find(phrase).is_empty()
    }

    /// Where a run of Chinese characters is found, by byte in `norm`. A
    /// term written `^xyz|term` does not count right after x, y or z; one
    /// written `term|xyz^` does not count right before them.
    pub fn find_han(&self, term: &str) -> Vec<usize> {
        let (before, rest) = match term.strip_prefix('^') {
            Some(rest) => rest.split_once('|').unwrap_or(("", rest)),
            None => ("", term),
        };
        let (term, after) = match rest.strip_suffix('^') {
            Some(rest) => {
                let (term, after) = rest.split_once('|').unwrap_or((rest, ""));
                (term, after)
            }
            None => (rest, ""),
        };
        self.norm
            .match_indices(term)
            .filter(|(at, _)| {
                let prior = self.norm[..*at].chars().next_back();
                let next = self.norm[at + term.len()..].chars().next();
                !prior.is_some_and(|p| before.contains(p))
                    && !next.is_some_and(|n| after.contains(n))
            })
            .map(|(at, _)| at)
            .collect()
    }

    pub fn has_han(&self, term: &str) -> bool {
        !self.find_han(term).is_empty()
    }

    /// Whether any of `phrases` is there, each read as spaced words or as
    /// Chinese characters by how it is written.
    pub fn any(&self, phrases: &[&str]) -> bool {
        phrases.iter().any(|phrase| self.has_phrase(phrase))
    }

    pub fn has_phrase(&self, phrase: &str) -> bool {
        if is_han_phrase(phrase) {
            self.has_han(phrase)
        } else {
            self.has(phrase)
        }
    }

    /// Where a phrase is, as (sentence, position): a word index for spaced
    /// words, a byte for Chinese characters. Positions of the two kinds are
    /// never compared with each other.
    pub fn places(&self, phrase: &str) -> Vec<(usize, usize)> {
        if is_han_phrase(phrase) {
            self.find_han(phrase)
                .into_iter()
                .map(|at| (self.sentence_at(at), at))
                .collect()
        } else {
            self.find(phrase)
                .into_iter()
                .map(|at| (self.words[at].sentence, at))
                .collect()
        }
    }
}

/// Whether a phrase is written in Chinese characters rather than words.
pub(super) fn is_han_phrase(phrase: &str) -> bool {
    phrase.chars().any(is_han)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_width_and_curly_forms_read_as_plain_ones() {
        let text = Text::read("ＡＩ助手？I\u{2019}m Ｍａｒａ.");
        assert!(text.norm.starts_with("ai助手?i'm mara."));
        assert_eq!(text.han, 2);
        assert_eq!(text.words[0].text, "ai");
        assert!(text.words[0].capital);
        assert_eq!(text.words[1].text, "i'm");
        assert!(text.words[2].capital && !text.words[2].first);
    }

    #[test]
    fn sentences_and_phrases_are_found_in_their_sentence() {
        let text = Text::read("Leo's boat is out. Language model? No.");
        assert_eq!(text.sentences.len(), 3);
        assert_eq!(text.words[0].text, "leo");
        assert!(text.has("language model"));
        assert!(!text.has("out language"), "a phrase stays in its sentence");
        assert!(text.has("lang*"));
        assert_eq!(text.find("no"), vec![6]);
    }

    #[test]
    fn a_chinese_term_can_refuse_what_stands_beside_it() {
        let text = Text::read("我在渔网上补了个洞。你上网看看。");
        assert_eq!(text.find_han("网上").len(), 1);
        assert!(text.find_han("^渔鱼|网上").is_empty());
        assert_eq!(text.find_han("上网").len(), 1);
        assert!(text.has_han("上网|吧^"));
    }
}
