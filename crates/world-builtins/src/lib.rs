use world_host::{HostError, WorldPackSource, WorldRegistration, WorldRegistry};

pub struct BuiltinWorlds;

/// The Simplified Chinese the built-in Worlds are shown in: the Systems'
/// words every World shares, Tiny Society's own and Pocket Universe's.
pub const ZH_HANS: [&str; 3] = [
    include_str!("../locales/systems.zh-Hans.tsv"),
    tiny_society::ZH_HANS,
    include_str!("../../../worlds/pocket-universe/locales/zh-Hans.tsv"),
];

/// The Japanese the built-in Worlds are shown in, the same way.
pub const JA: [&str; 3] = [
    include_str!("../locales/systems.ja.tsv"),
    tiny_society::JA,
    include_str!("../../../worlds/pocket-universe/locales/ja.tsv"),
];

/// Every line the core residents of Tiny Society and Pocket Universe can
/// say of their own, in Simplified Chinese: their templates filled in
/// every way, each word translated. Worked out from `ZH_HANS`, to install
/// beside it.
pub fn zh_hans_voices() -> String {
    voices(&ZH_HANS)
}

/// The same lines in Japanese, worked out from `JA`.
pub fn ja_voices() -> String {
    voices(&JA)
}

fn voices(catalogs: &[&str]) -> String {
    let catalog = catalogs
        .iter()
        .fold(world_i18n::Catalog::default(), |mut all, text| {
            all.extend(text);
            all
        });
    let mut out = String::new();
    let templates = tiny_society::voice_templates()
        .into_iter()
        .chain(pocket_universe::voice_templates());
    for (lines, slots) in templates {
        // A line is shown a sentence at a time, so each of its sentences
        // is filled in on its own.
        let sentences = lines.iter().flat_map(|line| {
            let mut parts = Vec::new();
            let mut start = 0;
            for (index, _) in line.match_indices(['.', '!', '?']) {
                if line[index + 1..].starts_with(' ') {
                    parts.push(&line[start..=index]);
                    start = index + 2;
                }
            }
            parts.push(&line[start..]);
            parts
        });
        for template in sentences {
            for (english, chinese) in catalog.expand(template, slots) {
                out.push_str(&english);
                out.push('\t');
                out.push_str(&chinese);
                out.push('\n');
            }
        }
    }
    out
}

impl WorldPackSource for BuiltinWorlds {
    fn registrations(&self) -> Result<Vec<WorldRegistration>, HostError> {
        Ok(vec![tiny_society::tiny_society_registration()])
    }
}

pub fn registry() -> Result<WorldRegistry, HostError> {
    let mut registry = WorldRegistry::new();
    registry.install_source(&BuiltinWorlds)?;
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn builtin_source_exposes_tiny_society() {
        let registrations = BuiltinWorlds.registrations().unwrap();
        let ids = registrations
            .iter()
            .map(|registration| registration.descriptor.pack.id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(ids.len(), 1);
        assert!(ids.contains(tiny_society::TINY_SOCIETY_PACK_ID));
    }

    #[test]
    fn catalog_lists_tiny_society() {
        let registry = registry().unwrap();
        let ids = registry
            .descriptors()
            .into_iter()
            .map(|descriptor| descriptor.pack.id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(ids.len(), 1);
        assert!(ids.contains(tiny_society::TINY_SOCIETY_PACK_ID));
    }

    #[test]
    fn every_builtin_can_create_archive_and_reopen() {
        let registry = registry().unwrap();
        let ids = registry
            .descriptors()
            .into_iter()
            .map(|descriptor| descriptor.pack.id.clone())
            .collect::<Vec<_>>();

        for id in ids {
            let session = registry.create(&id).unwrap();
            let pack = session.pack();
            let archive = session.archive().unwrap().unwrap();
            let reopened = registry.open_archive(&archive).unwrap();

            assert_eq!(reopened.pack(), pack);
            assert_eq!(reopened.snapshot().title, session.snapshot().title);
        }
    }

    /// Sixteen months of Tiny Society in Chinese, into its second year:
    /// nearly every sentence it shows is in the catalogs, and what is left
    /// is only names.
    #[test]
    fn a_year_of_tiny_society_is_shown_in_chinese() {
        a_year_of_tiny_society_is_shown_in(&ZH_HANS, zh_hans_voices(), "Chinese");
    }

    /// The same sixteen months in Japanese.
    #[test]
    fn a_year_of_tiny_society_is_shown_in_japanese() {
        a_year_of_tiny_society_is_shown_in(&JA, ja_voices(), "Japanese");
    }

    fn a_year_of_tiny_society_is_shown_in(catalogs: &[&str], voices: String, language: &str) {
        let mut catalog = catalogs
            .iter()
            .fold(world_i18n::Catalog::default(), |mut all, text| {
                all.extend(text);
                all
            });
        assert!(voices.lines().count() >= 900, "{}", voices.lines().count());
        catalog.extend(&voices);
        let registry = registry().unwrap();
        let mut session = registry.create(tiny_society::TINY_SOCIETY_PACK_ID).unwrap();
        let mut shown = std::collections::BTreeSet::new();
        for day in 0..480 {
            let snapshot = session.snapshot();
            let mut texts = Vec::new();
            texts.extend(snapshot.voices.iter().map(|voice| voice.line.clone()));
            for command in &snapshot.commands {
                texts.push(command.title.clone());
                texts.push(command.detail.clone());
            }
            texts.extend(snapshot.keepsakes.iter().map(|kept| kept.what.clone()));
            texts.extend(snapshot.keepsakes.iter().map(|kept| kept.note.clone()));
            texts.extend(
                snapshot
                    .letters
                    .iter()
                    .rev()
                    .take(2)
                    .map(|letter| letter.note.clone()),
            );
            texts.extend(snapshot.book.iter().map(|entry| entry.name.clone()));
            texts.extend(snapshot.book.iter().map(|entry| entry.hint.clone()));
            texts.extend(
                snapshot
                    .chapters
                    .iter()
                    .map(|chapter| chapter.title.clone()),
            );
            texts.extend(snapshot.goals.iter().map(|goal| goal.label.clone()));
            if let Some(briefing) = &snapshot.briefing {
                texts.extend(briefing.items.iter().map(|item| item.title.clone()));
            }
            shown.extend(texts.into_iter().filter(|text| !text.trim().is_empty()));
            let pick = snapshot
                .commands
                .iter()
                .find(|c| c.question.is_some() && c.unavailable.is_none())
                .map(|c| c.id.clone());
            if let Some(pick) = pick {
                let _ = session.handle(world_projection::ProjectionIntent::InvokeCommand(pick));
            }
            let wait = if day % 7 == 3 {
                snapshot
                    .commands
                    .iter()
                    .find(|c| {
                        c.hand.as_ref().is_some_and(|hand| hand.verb == "Build")
                            && c.unavailable.is_none()
                    })
                    .map(|c| c.id.clone())
            } else {
                None
            };
            if let Some(build) = wait {
                let _ = session.handle(world_projection::ProjectionIntent::InvokeCommand(build));
            }
            session
                .handle(world_projection::ProjectionIntent::InvokeCommand(
                    "tiny-society.let-day-pass".into(),
                ))
                .unwrap();
        }
        let names = registry_names();
        let english = |text: &str| {
            text.split(|c: char| !c.is_ascii_alphabetic())
                .filter(|word| word.len() > 1 && !names.contains(*word))
                .count()
        };
        let mut left = Vec::new();
        for text in &shown {
            let translated = catalog.translate(text).unwrap_or_else(|| text.clone());
            if english(&translated) > 0 {
                left.push((text.clone(), translated));
            }
        }
        let share = 1.0 - left.len() as f64 / shown.len() as f64;
        eprintln!(
            "{} of {} shown texts fully in {language} ({:.1}%)",
            shown.len() - left.len(),
            shown.len(),
            share * 100.0
        );
        for (text, translated) in left.iter().take(40) {
            eprintln!("  {text}  =>  {translated}");
        }
        assert!(share >= 0.995, "only {:.1}% in {language}", share * 100.0);
    }

    /// Every person's name, which stays in Latin letters in Chinese.
    fn registry_names() -> std::collections::BTreeSet<&'static str> {
        tiny_society::people_names().into_iter().collect()
    }
}
