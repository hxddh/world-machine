//! Words without seams: every line a World says, and every line its
//! people's own voices can make, reads as written, never as a template
//! filled in. No slot or its name, no clause told twice, no sentence glued
//! on with a colon, no letter whose writer speaks of themselves by name,
//! and no moment that fails to show what it is.

use std::collections::BTreeSet;
use world_core::{EntityId, World};
use world_projection::{
    seams_in, speaks_of_self, Moment, MomentKind, ProjectionSnapshot, Prop, SelectionId, StoryPage,
    StoryRequest,
};

/// Everything a player can read now in the World's own words: the scene,
/// what people say, the questions, the drawer and the book (the details
/// panels, which show what the World records, are left out), and the
/// briefing if `briefing`.
pub fn readable(snapshot: &ProjectionSnapshot, briefing: bool) -> Vec<String> {
    let mut text: Vec<String> = Vec::new();
    if let Some(shown) = snapshot.briefing.as_ref().filter(|_| briefing) {
        for item in &shown.items {
            text.push(item.title.clone());
            text.push(item.detail.clone());
        }
    }
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

/// Everything a World can tell now: what a player reads (`lines`), and
/// the legend of each of `subjects`, every moment, and every almanac of
/// `years`, as `story` tells them.
pub fn told(
    mut lines: Vec<String>,
    subjects: impl IntoIterator<Item = EntityId>,
    moments: Vec<Moment>,
    years: Vec<u32>,
    story: impl Fn(StoryRequest) -> Option<StoryPage>,
) -> Vec<String> {
    for subject in subjects {
        if let Some(StoryPage::Legend(legend)) =
            story(StoryRequest::Legend(SelectionId::Entity(subject)))
        {
            lines.push(legend.title);
            for line in legend.lines {
                lines.push(line.text);
                lines.extend(line.because);
            }
        }
    }
    for moment in moments {
        lines.push(moment.title);
        lines.extend(moment.panels.into_iter().map(|panel| panel.caption));
    }
    for year in years {
        if let Some(StoryPage::Almanac(almanac)) = story(StoryRequest::Almanac(year)) {
            lines.push(almanac.title);
            lines.extend(almanac.built);
        }
    }
    lines
}

/// Notes in `found`, as `who`, every seam in a line not seen before.
pub fn note_seams(
    who: &str,
    lines: Vec<String>,
    seen: &mut BTreeSet<String>,
    found: &mut BTreeSet<String>,
) {
    for line in lines {
        if seen.insert(line.clone()) {
            for seam in seams_in(&line) {
                found.insert(format!("{who}: {seam:?} in {line:?}"));
            }
        }
    }
}

/// Letters whose writer speaks of themselves by name.
pub fn self_told(world: &World, snapshot: &ProjectionSnapshot) -> Vec<String> {
    let state = world.state();
    snapshot
        .letters
        .iter()
        .filter_map(|letter| {
            let SelectionId::Entity(writer) = letter.from else {
                return None;
            };
            let name = lives::first_name(state, writer);
            speaks_of_self(&name, &letter.note).then(|| format!("{name} wrote {:?}", letter.note))
        })
        .collect()
}

/// Of `people`, anyone who has lived in the World a year (of `year` days,
/// by `today`) with a life of fewer than five lines.
pub fn short_lives(
    who: &str,
    people: impl IntoIterator<Item = EntityId>,
    today: u32,
    year: u64,
    story: impl Fn(StoryRequest) -> Option<StoryPage>,
) -> Vec<String> {
    let mut short = Vec::new();
    for person in people {
        let Some(StoryPage::Legend(legend)) =
            story(StoryRequest::Legend(SelectionId::Entity(person)))
        else {
            continue;
        };
        let since = legend.lines.first().map_or(today, |line| line.day);
        if u64::from(today.saturating_sub(since)) >= year
            && legend.lines.len() < chronicle::FEWEST_LIFE_LINES
        {
            short.push(format!(
                "{who}: {} has lived a year in {} lines",
                legend.title,
                legend.lines.len()
            ));
        }
    }
    short
}

/// What a moment's middle panel fails to show of what every moment of its
/// kind shows: bunting at a wedding, a cradle at a birth. A Pack checks
/// its own farewells first.
pub fn unshown(moment: &Moment) -> Option<&'static str> {
    let shown = &moment.panels[1].props;
    let needs = match moment.kind {
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

/// Every seam in the lines a voice can make: its own lines, and every
/// scene's prompt and replies, the gift filled in as it is given.
pub fn voice_seams(voice: &lives::Voice) -> Vec<String> {
    let scenes = voice
        .scenes
        .iter()
        .flat_map(|scene| std::iter::once(scene.prompt).chain(scene.replies))
        .map(|line| line.replace("{keepsake}", voice.keepsake));
    lives::own_lines(voice)
        .into_iter()
        .chain(scenes)
        .flat_map(|line| {
            seams_in(&line)
                .into_iter()
                .map(move |seam| format!("voice: {seam:?} in {line:?}"))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Fails with every seam found.
pub fn assert_no_seams(found: BTreeSet<String>) {
    assert!(
        found.is_empty(),
        "{} seams:\n{}",
        found.len(),
        found.into_iter().collect::<Vec<_>>().join("\n")
    );
}
