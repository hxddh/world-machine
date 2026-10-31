//! Talk never says "I don't understand", in English, Chinese or Japanese
//! (1.0 P4a), in both built-in Packs as a player meets them: through the
//! same session a window drives, a broad run of questions (the shapes
//! residents answer, and nonsense, typos and the off-topic) is put to
//! several residents on several days. No answer, in any language the app
//! shows it in, is a way of saying "I don't follow"; every answer leaves
//! three things to ask beside the words; an answer that cannot say is
//! read whole in each language; and every one of those things to ask,
//! shown and said in each language, is answered from what the resident
//! knows.

use world_projection::{
    CanvasItemKind, CommandRole, Ears, ProjectionIntent, ProjectionSnapshot, SelectionId,
};

fn catalog(catalogs: &[&str], voices: &str) -> world_i18n::Catalog {
    let mut catalog = catalogs
        .iter()
        .fold(world_i18n::Catalog::default(), |mut all, text| {
            all.extend(text);
            all
        });
    catalog.extend(voices);
    catalog
}

/// Ways of saying "I don't follow", in each language.
const NON_ANSWERS: &[&str] = &[
    "understand",
    "not sure what you mean",
    "what do you mean",
    "didn't follow",
    "don't follow",
    "say that another way",
    "didn't catch",
    "come again",
    "没听懂",
    "听不懂",
    "不明白",
    "不懂你",
    "什么意思",
    "换个说法",
    "再说一遍",
    "わからない",
    "分からない",
    "わかんない",
    "どういう意味",
    "別の言い方",
    "もう一度言って",
];

/// What a player types, `{x}` someone else who lives there.
const ASKED: &[&str] = &[
    "What do you do?",
    "How's work?",
    "What happened?",
    "Who is {x}?",
    "Where is {x}?",
    "How are you?",
    "Are you going to the festival?",
    "Is the pier hard to build?",
    "Did you write me a letter?",
    "asdfghjkl",
    "?",
    "What's the capital of France?",
    "how r u",
    "你会写代码吗？",
    "宇宙の果てはどこ？",
];

/// What stays in Latin letters in a line shown in Chinese or Japanese:
/// people's names, and a few words both write so.
fn names() -> std::collections::BTreeSet<String> {
    tiny_society::people_names()
        .into_iter()
        .chain(pocket_universe::people_names())
        .flat_map(str::split_whitespace)
        .chain(["MTV", "DJ", "CD", "TV"])
        .map(str::to_string)
        .collect()
}

/// The words of `shown` left in English: Latin words that are no name.
fn english_in(shown: &str, names: &std::collections::BTreeSet<String>) -> Vec<String> {
    shown
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|word| word.len() > 1 && !names.contains(*word))
        .map(str::to_string)
        .collect()
}

fn non_answer(line: &str) -> Option<&'static str> {
    let lower = line.to_lowercase();
    NON_ANSWERS
        .iter()
        .copied()
        .find(|phrase| lower.contains(phrase))
}

/// Whether an answer is someone saying they cannot answer.
fn deflects(answer: &str) -> bool {
    conversation::grounded::LEADS
        .iter()
        .flat_map(|(_, leads)| leads.iter())
        .any(|lead| answer.starts_with(lead))
}

fn answer_to(snapshot: &ProjectionSnapshot, who: SelectionId) -> String {
    snapshot
        .exchanges
        .iter()
        .rev()
        .find(|exchange| exchange.who == who)
        .map(|exchange| exchange.answer.clone())
        .expect("an answer")
}

fn openers_of(snapshot: &ProjectionSnapshot, who: SelectionId) -> Vec<String> {
    snapshot
        .openers
        .iter()
        .find(|openers| openers.who == who)
        .map(|openers| openers.lines.clone())
        .unwrap_or_default()
}

fn say(
    session: &mut Box<dyn world_host::WorldSession>,
    who: SelectionId,
    words: &str,
) -> ProjectionSnapshot {
    session
        .handle(ProjectionIntent::Say {
            to: who,
            words: words.into(),
            ears: Ears::World,
        })
        .unwrap_or_else(|error| panic!("{words}: {error}"))
}

#[test]
fn talk_never_says_it_does_not_understand_in_any_language() {
    let languages = [
        (
            "zh",
            catalog(&world_builtins::ZH_HANS, &world_builtins::zh_hans_voices()),
        ),
        (
            "ja",
            catalog(&world_builtins::JA, &world_builtins::ja_voices()),
        ),
    ];
    let names = names();
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    let places: [(&str, Option<&str>); 4] = [
        (tiny_society::TINY_SOCIETY_PACK_ID, None),
        (
            pocket_universe::POCKET_UNIVERSE_PACK_ID,
            Some(pocket_universe::SEED_MARS_COLONY_COMMAND),
        ),
        (
            pocket_universe::POCKET_UNIVERSE_PACK_ID,
            Some(pocket_universe::SEED_1980S_TOWN_COMMAND),
        ),
        (
            pocket_universe::POCKET_UNIVERSE_PACK_ID,
            Some(pocket_universe::SEED_PENGUIN_CIVILIZATION_COMMAND),
        ),
    ];
    let mut failures = Vec::new();
    let (mut deflections, mut chips) = (0, 0);
    for (pack, seed) in places {
        let mut session = registry.create(pack).unwrap();
        if let Some(seed) = seed {
            session
                .handle(ProjectionIntent::InvokeCommand(seed.into()))
                .unwrap();
        }
        for day in 0..2 {
            let snapshot = session.snapshot();
            let people = snapshot
                .canvas
                .items
                .iter()
                .filter(|item| item.kind == CanvasItemKind::Actor)
                .filter(|item| snapshot.openers.iter().any(|o| o.who == item.id))
                .map(|item| (item.id, item.label.clone()))
                .collect::<Vec<_>>();
            assert!(people.len() >= 2, "{pack} {seed:?}: someone to talk to");
            for (at, (who, _)) in people.iter().enumerate().skip(day).step_by(2).take(3) {
                let other = &people[(at + 1) % people.len()].1;
                let other = other.split_whitespace().next().unwrap_or(other);
                for asked in ASKED {
                    let words = asked.replace("{x}", other);
                    let snapshot = say(&mut session, *who, &words);
                    let answer = answer_to(&snapshot, *who);
                    let mut shown = vec![("en", answer.clone())];
                    for (lang, catalog) in &languages {
                        shown.push((
                            lang,
                            catalog.translate(&answer).unwrap_or_else(|| answer.clone()),
                        ));
                    }
                    for (lang, line) in &shown {
                        if let Some(phrase) = non_answer(line) {
                            failures.push(format!("{lang} {words}: {line} ({phrase})"));
                        }
                    }
                    let offered = openers_of(&snapshot, *who);
                    if offered.len() != 3 {
                        failures.push(format!("{words}: {} to ask: {offered:?}", offered.len()));
                    }
                    if deflects(&answer) {
                        deflections += 1;
                        for (lang, line) in &shown[1..] {
                            let left = english_in(line, &names);
                            if !left.is_empty() {
                                failures.push(format!("{lang} {answer} => {line} {left:?}"));
                            }
                        }
                    }
                }
                // Each thing to ask, as the player's language shows it and
                // says it, answered from what they know.
                for (lang, catalog) in [("en", None)].into_iter().chain(
                    languages
                        .iter()
                        .map(|(lang, catalog)| (*lang, Some(catalog))),
                ) {
                    let snapshot = session.snapshot();
                    for line in openers_of(&snapshot, *who) {
                        let words = match catalog {
                            Some(catalog) => catalog.translate(&line).unwrap_or(line.clone()),
                            None => line.clone(),
                        };
                        let left = english_in(&words, &names);
                        if catalog.is_some() && !left.is_empty() {
                            failures.push(format!("{lang} chip {line} => {words} {left:?}"));
                        }
                        let answer = answer_to(&say(&mut session, *who, &words), *who);
                        chips += 1;
                        if deflects(&answer) || non_answer(&answer).is_some() {
                            failures.push(format!("{lang} chip {words}: {answer}"));
                        }
                        // The answer, read in the language asked in.
                        if let Some(catalog) = catalog {
                            let shown =
                                catalog.translate(&answer).unwrap_or_else(|| answer.clone());
                            let left = english_in(&shown, &names);
                            if non_answer(&shown).is_some() || !left.is_empty() {
                                failures.push(format!("{lang} {answer} => {shown} {left:?}"));
                            }
                        }
                    }
                }
            }
            // The day passes by its own card.
            let snapshot = session.snapshot();
            let pass = snapshot
                .commands
                .iter()
                .find(|command| command.role == Some(CommandRole::PassesTime))
                .map(|command| command.id.clone())
                .unwrap_or_else(|| pocket_universe::NUDGE_COMMAND.into());
            session
                .handle(ProjectionIntent::InvokeCommand(pass))
                .unwrap();
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(
        deflections >= 20,
        "{deflections} answers that could not say"
    );
    assert!(chips >= 100, "{chips} things to ask");
}
