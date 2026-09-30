//! Words without seams, in every place: every line a place says over a
//! year and more for two kinds of player, and every line its people's own
//! voices can make, reads as written, never as a template filled in. No
//! slot or its name, no "Harbor", no clause told twice, no sentence glued
//! on with a colon, and no letter whose writer speaks of themselves by
//! name. And every moment shows what it is.

use crate::{
    PocketUniverse, NUDGE_COMMAND, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use std::collections::BTreeSet;
use world_core::EntityId;
use world_projection::{
    seams_in, speaks_of_self, Moment, MomentKind, ProjectionSnapshot, Prop, SelectionId, StoryPage,
    StoryRequest,
};

/// Periods each player plays: a year of the place's and a season more.
const PERIODS: u64 = crate::almanac::YEAR + 30;

const SEEDS: [&str; 3] = [
    SEED_MARS_COLONY_COMMAND,
    SEED_1980S_TOWN_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
];

/// Everything a player can read now in the World's own words.
pub(crate) fn readable(snapshot: &ProjectionSnapshot) -> Vec<String> {
    let mut text: Vec<String> = Vec::new();
    for command in &snapshot.commands {
        text.push(command.title.clone());
        text.push(command.detail.clone());
        if let Some(question) = &command.question {
            text.push(question.prompt.clone());
        }
    }
    for item in &snapshot.timeline.items {
        text.push(item.title.clone());
    }
    for item in &snapshot.canvas.items {
        text.push(item.label.clone());
        text.push(item.detail.clone());
    }
    text.extend(snapshot.voices.iter().map(|voice| voice.line.clone()));
    for talk in &snapshot.talks {
        text.push(talk.question.clone());
        text.push(talk.answer.clone());
    }
    text.extend(snapshot.goals.iter().map(|goal| goal.label.clone()));
    for chapter in &snapshot.chapters {
        text.push(chapter.title.clone());
        text.push(chapter.summary.clone());
    }
    for keepsake in &snapshot.keepsakes {
        text.push(keepsake.what.clone());
        text.push(keepsake.note.clone());
    }
    text.extend(snapshot.letters.iter().map(|letter| letter.note.clone()));
    for moment in &snapshot.moments {
        text.push(moment.title.clone());
        text.extend(moment.panels.iter().map(|panel| panel.caption.clone()));
    }
    for entry in &snapshot.book {
        text.push(entry.name.clone());
        text.push(entry.hint.clone());
    }
    text.retain(|line| !line.trim().is_empty());
    text
}

/// Everything a place can tell now: what a player reads, and every
/// legend, moment and almanac it keeps.
fn told(universe: &PocketUniverse) -> Vec<String> {
    let mut lines = readable(&universe.projection_snapshot());
    let world = universe.world();
    let subjects: Vec<EntityId> = world.state().entities().map(|entity| entity.id).collect();
    for subject in subjects {
        if let Some(StoryPage::Legend(legend)) =
            universe.story(StoryRequest::Legend(SelectionId::Entity(subject)))
        {
            lines.push(legend.title);
            for line in legend.lines {
                lines.push(line.text);
                lines.extend(line.because);
            }
        }
    }
    for moment in crate::moments::moments(world) {
        lines.push(moment.title);
        lines.extend(moment.panels.into_iter().map(|panel| panel.caption));
    }
    for year in crate::almanac_page::years(world) {
        if let Some(StoryPage::Almanac(almanac)) = universe.story(StoryRequest::Almanac(year)) {
            lines.push(almanac.title);
            lines.extend(almanac.built);
        }
    }
    lines
}

/// What a moment's panels fail to show of what it is.
fn unshown(moment: &Moment) -> Option<&'static str> {
    let shown = &moment.panels[1].props;
    let needs = match moment.kind {
        MomentKind::Farewell if moment.title.ends_with("farewell") => {
            if [Prop::Shuttle, Prop::Bus, Prop::Sled]
                .iter()
                .any(|way| shown.contains(way))
            {
                return None;
            }
            return Some("way to leave");
        }
        MomentKind::Wedding => Prop::Bunting,
        MomentKind::Birth => Prop::Cradle,
        MomentKind::Death => Prop::Wreath,
        MomentKind::Storm => Prop::Rain,
        MomentKind::WorkOpened => Prop::Ribbon,
        MomentKind::Festival => Prop::Bunting,
        _ => return None,
    };
    (!shown.contains(&needs)).then_some(needs.id())
}

fn play(seed: &str, refusing: bool, found: &mut BTreeSet<String>) {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    let who = if refusing { "Refusing" } else { "Warm" };
    let place = seed.trim_start_matches("pocket-universe.seed-");
    let mut seen = BTreeSet::new();
    for period in 0..PERIODS {
        let snapshot = universe.projection_snapshot();
        let asked = snapshot
            .commands
            .iter()
            .filter(|command| command.question.is_some() && command.unavailable.is_none())
            .map(|command| command.id.clone())
            .collect::<Vec<_>>();
        let answer = if refusing {
            asked.last().cloned()
        } else {
            asked.first().cloned()
        };
        if let Some(answer) = answer {
            let _ = universe.invoke_projection_command(&answer);
        }
        let lines = if period % 10 == 9 || period + 1 == PERIODS {
            told(&universe)
        } else {
            readable(&universe.projection_snapshot())
        };
        for line in lines {
            if seen.insert(line.clone()) {
                for seam in seams_in(&line) {
                    found.insert(format!("{place} {who}: {seam:?} in {line:?}"));
                }
            }
        }
        let state = universe.world().state();
        for letter in universe.projection_snapshot().letters {
            let SelectionId::Entity(writer) = letter.from else {
                continue;
            };
            let name = lives::first_name(state, writer);
            if speaks_of_self(&name, &letter.note) {
                found.insert(format!("{place} {who}: {name} wrote {:?}", letter.note));
            }
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
    // Anyone who has lived here a year has a life of five lines or more.
    let today = crate::arrival::today(universe.world().state()) as u32 + 1;
    let people: Vec<EntityId> = universe
        .world()
        .state()
        .entities()
        .map(|entity| entity.id)
        .filter(|id| crate::legends::is_person(universe.world(), *id))
        .collect();
    for person in people {
        let Some(StoryPage::Legend(legend)) =
            universe.story(StoryRequest::Legend(SelectionId::Entity(person)))
        else {
            continue;
        };
        let since = legend.lines.first().map_or(today, |line| line.day);
        if u64::from(today.saturating_sub(since)) >= crate::almanac::YEAR
            && legend.lines.len() < chronicle::FEWEST_LIFE_LINES
        {
            found.insert(format!(
                "{place} {who}: {} has lived a year in {} lines",
                legend.title,
                legend.lines.len()
            ));
        }
    }
    for moment in crate::moments::moments(universe.world()) {
        if let Some(missing) = unshown(&moment) {
            found.insert(format!(
                "{place} {who}: {:?} shows no {missing}",
                moment.title
            ));
        }
    }
}

#[test]
fn every_line_every_place_says_reads_without_seams() {
    let mut found = BTreeSet::new();
    for seed in SEEDS {
        for refusing in [false, true] {
            play(seed, refusing, &mut found);
        }
    }
    for voice in crate::voices::ALL {
        let scenes = voice
            .scenes
            .iter()
            .flat_map(|scene| std::iter::once(scene.prompt).chain(scene.replies))
            // A scene's gift is filled in as it is given.
            .map(|line| line.replace("{keepsake}", voice.keepsake));
        for line in lives::own_lines(voice).into_iter().chain(scenes) {
            for seam in seams_in(&line) {
                found.insert(format!("voice: {seam:?} in {line:?}"));
            }
        }
    }
    assert!(
        found.is_empty(),
        "{} seams:\n{}",
        found.len(),
        found.into_iter().collect::<Vec<_>>().join("\n")
    );
}
