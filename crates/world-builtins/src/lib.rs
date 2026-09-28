use world_host::{HostError, WorldPackSource, WorldRegistration, WorldRegistry};

pub struct BuiltinWorlds;

/// The Simplified Chinese the built-in Worlds are shown in: the Systems'
/// words every World shares, Tiny Society's own and Pocket Universe's.
pub const ZH_HANS: [&str; 3] = [
    include_str!("../locales/systems.zh-Hans.tsv"),
    tiny_society::ZH_HANS,
    include_str!("../../../worlds/pocket-universe/locales/zh-Hans.tsv"),
];

/// Every line the core residents of Tiny Society and Pocket Universe can
/// say of their own, in Simplified Chinese: their templates filled in
/// every way, each word translated. Worked out from `ZH_HANS`, to install
/// beside it.
pub fn zh_hans_voices() -> String {
    let catalog = ZH_HANS
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
        Ok(vec![
            tiny_society::tiny_society_registration(),
            future_archaeologist::future_archaeologist_registration(),
        ])
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
    fn builtin_source_exposes_both_benchmark_worlds() {
        let registrations = BuiltinWorlds.registrations().unwrap();
        let ids = registrations
            .iter()
            .map(|registration| registration.descriptor.pack.id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(tiny_society::TINY_SOCIETY_PACK_ID));
        assert!(ids.contains(future_archaeologist::FUTURE_ARCHAEOLOGIST_PACK_ID));
    }

    #[test]
    fn catalog_lists_both_benchmark_worlds() {
        let registry = registry().unwrap();
        let ids = registry
            .descriptors()
            .into_iter()
            .map(|descriptor| descriptor.pack.id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(tiny_society::TINY_SOCIETY_PACK_ID));
        assert!(ids.contains(future_archaeologist::FUTURE_ARCHAEOLOGIST_PACK_ID));
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

    /// A year of Tiny Society in Chinese: nearly every sentence it shows
    /// is in the catalogs, and what is left is only names.
    #[test]
    fn a_year_of_tiny_society_is_shown_in_chinese() {
        for catalog in ZH_HANS {
            world_i18n::install(catalog);
        }
        let mut catalog = ZH_HANS
            .iter()
            .fold(world_i18n::Catalog::default(), |mut all, text| {
                all.extend(text);
                all
            });
        let voices = zh_hans_voices();
        assert!(voices.lines().count() >= 900, "{}", voices.lines().count());
        catalog.extend(&voices);
        let registry = registry().unwrap();
        let mut session = registry.create(tiny_society::TINY_SOCIETY_PACK_ID).unwrap();
        let mut shown = std::collections::BTreeSet::new();
        for day in 0..100 {
            let snapshot = session.snapshot();
            let mut texts = Vec::new();
            texts.extend(snapshot.voices.iter().map(|voice| voice.line.clone()));
            for command in &snapshot.commands {
                texts.push(command.title.clone());
                texts.push(command.detail.clone());
            }
            texts.extend(snapshot.keepsakes.iter().map(|kept| kept.what.clone()));
            texts.extend(snapshot.keepsakes.iter().map(|kept| kept.note.clone()));
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
            "{} of {} shown texts fully in Chinese ({:.1}%)",
            shown.len() - left.len(),
            shown.len(),
            share * 100.0
        );
        for (text, translated) in left.iter().take(40) {
            eprintln!("  {text}  =>  {translated}");
        }
        assert!(share >= 0.95, "only {:.1}% in Chinese", share * 100.0);
    }

    fn registry_names() -> std::collections::BTreeSet<&'static str> {
        [
            "Jonas", "Mara", "Leo", "Emma", "Mia", "Noah", "Evan", "Sofia", "Ada", "Ivo", "Rosa",
            "Tobias", "Hana", "Olek", "Maeve", "Arun", "Lise", "Pim", "Greta", "Kofi", "Ines",
            "Bram", "Nell", "Soren", "Yara", "Dario", "Ffion", "Mateo", "Wren", "Anouk", "Casimir",
            "Lotte", "Ravi", "Esme", "Tam",
        ]
        .into_iter()
        .collect()
    }
}
