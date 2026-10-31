//! Tiny Society's Steam achievements (`data/achievements.toml`), and
//! which of them a World has unlocked, worked out from the World alone:
//! the pages of the Book, the years it has closed, the town's projects,
//! the book of finds and the almanac, and each resident's heart. Nothing
//! here talks to Steam; the same World unlocks the same achievements on
//! any machine and on any replay.

use std::sync::OnceLock;
use world_core::World;

/// The achievements, as the data file lists them.
pub const ACHIEVEMENTS_TOML: &str = include_str!("../data/achievements.toml");

/// One achievement: its Steam id, its name in each language, the kinds
/// of recorded Event whose arrival can unlock it, and what must hold.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Achievement {
    pub id: String,
    pub en: String,
    pub zh: String,
    pub ja: String,
    pub events: Vec<String>,
    pub unlock: Unlock,
}

/// What must hold for an achievement to unlock.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Unlock {
    /// That season's page of the Book has been filled.
    Page(String),
    /// The Book has closed this many years.
    Years(u32),
    /// A town project has reached this step, or its last.
    Project(String, Option<u32>),
    /// This many entries of the book of finds are found, or all of them.
    Finds(Option<u32>),
    /// Every day of the almanac has been lived.
    Almanac,
    /// Every beat of this resident's heart is reached.
    Heart(String),
    /// This many residents' hearts are complete.
    Hearts(u32),
    #[default]
    Never,
}

impl Unlock {
    fn parse(text: &str) -> Result<Self, String> {
        let parts = text.split(':').collect::<Vec<_>>();
        let number = |text: &str| {
            text.parse::<u32>()
                .map_err(|_| format!("not a count: {text:?}"))
        };
        Ok(match parts.as_slice() {
            ["page", season] => Self::Page((*season).into()),
            ["years", n] => Self::Years(number(n)?),
            ["project", id] => Self::Project((*id).into(), None),
            ["project", id, step] => Self::Project((*id).into(), Some(number(step)?)),
            ["finds", "all"] => Self::Finds(None),
            ["finds", n] => Self::Finds(Some(number(n)?)),
            ["almanac", "all"] => Self::Almanac,
            ["heart", name] => Self::Heart((*name).into()),
            ["hearts", n] => Self::Hearts(number(n)?),
            _ => return Err(format!("no such condition: {text:?}")),
        })
    }
}

/// Reads the achievements file: a `[[achievement]]` table each, with
/// `key = "value"` lines.
pub fn parse(text: &str) -> Result<Vec<Achievement>, String> {
    let mut all = Vec::<Achievement>::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[achievement]]" {
            all.push(Achievement::default());
            continue;
        }
        let at = || format!("line {}", number + 1);
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("{}: not a key and value", at()))?;
        let value = value
            .trim()
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .ok_or_else(|| format!("{}: a value is quoted", at()))?;
        let achievement = all
            .last_mut()
            .ok_or_else(|| format!("{}: outside an [[achievement]]", at()))?;
        match key.trim() {
            "id" => achievement.id = value.into(),
            "en" => achievement.en = value.into(),
            "zh" => achievement.zh = value.into(),
            "ja" => achievement.ja = value.into(),
            "events" => {
                achievement.events = value.split(',').map(|kind| kind.trim().into()).collect()
            }
            "unlock" => {
                achievement.unlock = Unlock::parse(value).map_err(|e| format!("{}: {e}", at()))?
            }
            other => return Err(format!("{}: no such key {other:?}", at())),
        }
    }
    for achievement in &all {
        if achievement.id.is_empty()
            || achievement.en.is_empty()
            || achievement.zh.is_empty()
            || achievement.ja.is_empty()
            || achievement.events.is_empty()
            || achievement.unlock == Unlock::Never
        {
            return Err(format!("{:?} is missing something", achievement.id));
        }
    }
    Ok(all)
}

/// Every achievement, in the file's order.
pub fn all() -> &'static [Achievement] {
    static ALL: OnceLock<Vec<Achievement>> = OnceLock::new();
    ALL.get_or_init(|| parse(ACHIEVEMENTS_TOML).expect("the achievements file"))
}

/// How far a World has come on everything an achievement can ask.
struct Standing {
    pages: std::collections::BTreeSet<&'static str>,
    years: u32,
    projects: Vec<(&'static str, u32, u32)>,
    finds: (u32, u32),
    days: (u32, u32),
    hearts: Vec<(String, bool)>,
}

impl Standing {
    fn of(world: &World) -> Self {
        let state = world.state();
        let mut book = crate::book::book(world);
        book.extend(ritual::book(world));
        let count = |days: bool| {
            let shelf = book
                .iter()
                .filter(|entry| !days || entry.shelf == "Days")
                .collect::<Vec<_>>();
            let found = shelf.iter().filter(|entry| entry.found).count() as u32;
            (found, shelf.len() as u32)
        };
        Self {
            pages: crate::keeper::pages_ever(world),
            years: crate::keeper::years_closed(world),
            projects: crate::keeper::projects(world)
                .into_iter()
                .map(|(project, done)| (project.id, done, project.steps()))
                .collect(),
            finds: count(false),
            days: count(true),
            hearts: crate::keeper::hearts(world)
                .into_iter()
                .map(|(who, beats)| {
                    (
                        lives::first_name(state, who),
                        beats.iter().all(|beat| *beat),
                    )
                })
                .collect(),
        }
    }

    fn holds(&self, unlock: &Unlock) -> bool {
        match unlock {
            Unlock::Page(season) => self.pages.contains(season.as_str()),
            Unlock::Years(years) => self.years >= *years,
            Unlock::Project(id, step) => self
                .projects
                .iter()
                .any(|(project, done, steps)| project == id && *done >= step.unwrap_or(*steps)),
            Unlock::Finds(Some(n)) => self.finds.0 >= *n,
            Unlock::Finds(None) => self.finds.1 > 0 && self.finds.0 >= self.finds.1,
            Unlock::Almanac => self.days.1 > 0 && self.days.0 >= self.days.1,
            Unlock::Heart(name) => self
                .hearts
                .iter()
                .any(|(who, complete)| who == name && *complete),
            Unlock::Hearts(n) => {
                self.hearts.iter().filter(|(_, complete)| *complete).count() as u32 >= *n
            }
            Unlock::Never => false,
        }
    }
}

/// The achievements a World has unlocked, in the file's order: worked out
/// from its recorded history and the state it made, nothing else.
pub fn unlocked(world: &World) -> Vec<&'static Achievement> {
    let standing = Standing::of(world);
    all()
        .iter()
        .filter(|achievement| standing.holds(&achievement.unlock))
        .collect()
}

#[cfg(test)]
#[path = "achievements_tests.rs"]
mod tests;
