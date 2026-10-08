//! A World's names, held: every name a Pack can produce is known in each
//! language it speaks, and none of the World's own lines reads as naming
//! someone invented (`conversation`'s `invented` check, or a name a judge
//! would list from the line, read by `Grounds::invented`).
//!
//! A Pack hands its catalogs (English, then the line in another language,
//! one line a row) and a hearing in each language; the names a judge would
//! list are stood in for by what can be found without one: katakana runs
//! and Latin-letter names inside Chinese and Japanese, and runs of
//! capitalised words inside an English sentence.

/// Each line of a catalog in each language, its slots (`{name}`) filled
/// with `name`: (language, line). `catalogs` are (language, TSV of English
/// and the line in that language); the English of each is read once. A
/// row whose English is in `fixtures` (a test's own out-of-World line, in
/// the catalog only so the test can be read in every language) is left
/// out, in every language.
pub fn catalog_lines(
    catalogs: &[(&str, &str)],
    name: &str,
    fixtures: &[&str],
) -> Vec<(String, String)> {
    // In Chinese and Japanese a name stands apart from what is next to it,
    // as the game writes two slots side by side.
    let spaced = format!(" {name} ");
    let fill = |line: &str, name: &str| {
        let mut out = String::new();
        let mut rest = line;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            match rest[open..].find('}') {
                Some(close) => {
                    out.push_str(name);
                    rest = &rest[open + close + 1..];
                }
                None => {
                    out.push_str(&rest[open..]);
                    rest = "";
                }
            }
        }
        out.push_str(rest);
        out
    };
    let mut lines = Vec::new();
    let mut english = std::collections::BTreeSet::new();
    for (lang, catalog) in catalogs {
        for row in catalog.lines() {
            let Some((en, other)) = row.split_once('\t') else {
                continue;
            };
            if fixtures.contains(&en) {
                continue;
            }
            if english.insert(en.to_string()) {
                lines.push(("en".to_string(), fill(en, name)));
            }
            lines.push((lang.to_string(), fill(other, &spaced).trim().to_string()));
        }
    }
    lines
}

fn is_katakana(c: char) -> bool {
    matches!(c as u32, 0x30A1..=0x30FA) || c == 'ー' || c == '・'
}

/// The names a judge would list from `line`, as far as they can be found
/// without one.
pub fn names_in(lang: &str, line: &str) -> Vec<String> {
    let mut names = Vec::new();
    if lang == "en" {
        // Runs of capitalised words that do not begin a sentence.
        let mut run: Vec<&str> = Vec::new();
        let mut first = true;
        for word in line.split_whitespace() {
            let bare = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-');
            let capital = bare.chars().next().is_some_and(char::is_uppercase);
            if capital
                && !first
                && bare.chars().count() > 1
                && bare != "I"
                && !bare.starts_with("I'")
            {
                run.push(bare);
            } else if !run.is_empty() {
                names.push(run.join(" "));
                run.clear();
            }
            first = word.ends_with(['.', '!', '?', ':', ';', '"']);
            if first && !run.is_empty() {
                names.push(run.join(" "));
                run.clear();
            }
        }
        if !run.is_empty() {
            names.push(run.join(" "));
        }
        return names;
    }
    let mut run = String::new();
    let mut latin = String::new();
    for c in line.chars().chain([' ']) {
        if is_katakana(c) {
            run.push(c);
        } else {
            if run.trim_matches(['ー', '・']).chars().count() >= 2 {
                names.push(run.clone());
            }
            run.clear();
        }
        if c.is_ascii_alphabetic() || (!latin.is_empty() && (c == '-' || c == '\'')) {
            latin.push(c);
        } else {
            if latin.chars().filter(char::is_ascii_alphabetic).count() >= 2
                && latin.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            {
                names.push(latin.clone());
            }
            latin.clear();
        }
    }
    names
}

/// Every line of `lines` that reads as naming someone invented, by the
/// checks or by a name a judge would list, each with why; `hearing` gives
/// the hearing a line in each language is checked with.
pub fn invented_in_lines(
    lines: &[(String, String)],
    hearing: impl Fn(&str) -> conversation::Hearing,
) -> Vec<String> {
    let mut hearings = std::collections::BTreeMap::new();
    let mut failed = Vec::new();
    for (lang, line) in lines {
        let hearing = hearings
            .entry(lang.clone())
            .or_insert_with(|| hearing(lang));
        let grounds = conversation::grounds_of(hearing);
        let checked = conversation::checked(line, &grounds);
        // A line another check declines (a machine, the world outside) is
        // a test's, not the World's.
        if checked
            .strict
            .is_some_and(|why| why != conversation::OutOfWorld::Stranger)
        {
            continue;
        }
        if checked.found.contains(&"invented") {
            failed.push(format!("[{lang}] checks: {line}"));
            continue;
        }
        for name in names_in(lang, line) {
            if let Some(shape) = grounds.invented(&name, line) {
                failed.push(format!("[{lang}] {name} ({shape:?}): {line}"));
            }
        }
    }
    failed
}

/// Every name of `names` the hearing does not know, in the forms given
/// for it in each language.
pub fn unknown_names(names: &[String], hearing: &conversation::Hearing) -> Vec<String> {
    let grounds = conversation::grounds_of(hearing);
    names
        .iter()
        .filter(|name| !grounds.knows_name(name))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_found_as_a_judge_would_list_them() {
        assert_eq!(
            names_in("en", "We sailed with Old Tam to the Anchor Pub. Then home."),
            vec!["Old Tam", "Anchor Pub"]
        );
        assert_eq!(names_in("zh", "老Tam去了锚酒馆。"), vec!["Tam"]);
        assert_eq!(
            names_in("ja", "タムじいさんがシーフィンチ号に乗った。"),
            vec!["タム", "シーフィンチ"]
        );
        let lines = catalog_lines(
            &[(
                "zh",
                "Hi {name}.\t你好{name}。\nMy cousin Pedro.\t我表哥佩德罗。\n",
            )],
            "Mara",
            &["My cousin Pedro."],
        );
        assert_eq!(
            lines,
            vec![
                ("en".to_string(), "Hi Mara.".to_string()),
                ("zh".to_string(), "你好 Mara 。".to_string())
            ]
        );
    }
}
