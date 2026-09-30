//! Does a World keep having a story? Days played three ways and held to
//! the bar the v0.10 plan set: something to decide every day, no line said
//! more than three times in ten days, no gauge pinned at an end for more
//! than five days, a first chapter closed within a month, and a history
//! that replays without the storyteller.

use std::collections::{BTreeMap, BTreeSet};
use world_core::World;
use world_projection::ProjectionSnapshot;

#[derive(Clone, Copy, Debug)]
pub enum Policy {
    /// Always says yes, to the first thing on offer.
    Generous,
    /// Always takes the last answer on offer, which is often a no.
    Contrary,
    /// Never answers anything; lets every day pass.
    Absent,
}

/// A World as the density player plays it.
pub trait StoryWorld {
    fn world(&self) -> &World;
    /// The World's snapshot, as its story shows it.
    fn story_snapshot(&self) -> ProjectionSnapshot;
    fn invoke(&mut self, command: &str) -> Result<(), String>;
    /// The command that lets the day pass.
    fn pass_command(&self) -> &'static str;
    /// The two gauges that must not stay pinned at an end.
    fn gauges(&self) -> [&'static str; 2];
    /// What was said today.
    fn said_today(&self) -> Vec<String>;
    /// Checks the World holds to before each day.
    fn each_day(&self, _policy: Policy) {}
    /// The storylets that could come up today.
    fn could(&self) -> Vec<&'static str> {
        Vec::new()
    }
}

/// What the scene shows: everything on it, by name and where it stands.
pub fn scene(snapshot: &ProjectionSnapshot) -> BTreeSet<String> {
    snapshot
        .canvas
        .items
        .iter()
        .map(|item| format!("{} @ {:?}", item.label, item.at))
        .collect()
}

pub struct Played<W> {
    /// Days that offered something more than letting the day pass.
    pub days_with_a_choice: Vec<bool>,
    /// What was said on each day.
    pub lines: Vec<Vec<String>>,
    /// Each day's two gauges.
    pub gauges: Vec<Vec<(String, f32)>>,
    /// The day the first chapter closed.
    pub first_chapter: Option<usize>,
    /// Questions answered, and how many of those answers changed what is
    /// on the scene.
    pub answered: usize,
    pub answers_seen: usize,
    /// Each day, the storylets that could have come up.
    pub could: Vec<Vec<&'static str>>,
    pub world: W,
}

/// Plays `world` for `days` days as `policy`.
pub fn play<W: StoryWorld>(world: W, policy: Policy, days: usize) -> Played<W> {
    let pass = world.pass_command();
    let gauges = world.gauges();
    let mut played = Played {
        days_with_a_choice: Vec::new(),
        lines: Vec::new(),
        gauges: Vec::new(),
        first_chapter: None,
        answered: 0,
        answers_seen: 0,
        could: Vec::new(),
        world,
    };
    let world = &mut played.world;
    for day in 0..days {
        world.each_day(policy);
        let snapshot = world.story_snapshot();
        let choices = snapshot
            .commands
            .iter()
            .filter(|command| {
                command.id != pass && command.unavailable.is_none() && command.hand.is_none()
            })
            .map(|command| command.id.clone())
            .collect::<Vec<_>>();
        played.days_with_a_choice.push(!choices.is_empty());
        played.could.push(world.could());
        played.lines.push(world.said_today());
        played.gauges.push(
            snapshot
                .gauges
                .iter()
                .filter(|gauge| gauges.contains(&gauge.id.as_str()))
                .map(|gauge| (gauge.id.clone(), gauge.value))
                .collect(),
        );
        if played.first_chapter.is_none()
            && world
                .world()
                .events()
                .iter()
                .any(|event| event.kind == "chapter_ended")
        {
            played.first_chapter = Some(day);
        }
        let pick = match policy {
            Policy::Generous => choices.first(),
            Policy::Contrary => choices.last(),
            Policy::Absent => None,
        };
        if let Some(command) = pick {
            let before = scene(&world.story_snapshot());
            if let Err(error) = world.invoke(command) {
                panic!("{policy:?} day {day}: {command} was offered but failed: {error}");
            }
            if snapshot
                .command(command)
                .is_some_and(|command| command.question.is_some())
            {
                played.answered += 1;
                if scene(&world.story_snapshot()) != before {
                    played.answers_seen += 1;
                }
            }
        }
        world.invoke(pass).unwrap();
    }
    played
}

/// Holds sixty days of `world` played as `policy` to the bar.
pub fn check<W: StoryWorld>(world: W, policy: Policy) {
    let gauges = world.gauges();
    let played = play(world, policy, 60);
    let empty = played
        .days_with_a_choice
        .iter()
        .enumerate()
        .filter(|(_, choice)| !**choice)
        .map(|(day, _)| day)
        .collect::<Vec<_>>();
    assert!(
        empty.is_empty(),
        "{policy:?}: days with nothing to decide: {empty:?}"
    );

    for window in played.lines.windows(10) {
        let mut counts = BTreeMap::<&str, usize>::new();
        for line in window.iter().flatten() {
            *counts.entry(line).or_default() += 1;
        }
        let worst = counts.into_iter().max_by_key(|(_, count)| *count);
        if let Some((line, count)) = worst {
            assert!(
                count <= 3,
                "{policy:?}: {line:?} said {count} times in ten days"
            );
        }
    }

    for gauge in gauges {
        let mut run = 0;
        let mut worst = 0;
        for day in &played.gauges {
            let value = day
                .iter()
                .find(|(id, _)| id == gauge)
                .map(|(_, value)| *value)
                .unwrap();
            if !(0.05..=0.95).contains(&value) {
                run += 1;
            } else {
                run = 0;
            }
            worst = worst.max(run);
        }
        assert!(
            worst <= 5,
            "{policy:?}: {gauge} pinned at an end for {worst} days"
        );
    }

    assert!(
        played.first_chapter.is_some_and(|day| day <= 30),
        "{policy:?}: first chapter closed on {:?}",
        played.first_chapter
    );

    // Everything the storyteller did is history: the World replays to the
    // same place without it.
    let world = played.world.world();
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());

    // The chapters that ended are in the book, each in the World's words,
    // and the goals stand on the horizon.
    let snapshot = played.world.story_snapshot();
    assert!(!snapshot.chapters.is_empty());
    assert!(snapshot
        .chapters
        .iter()
        .all(|chapter| !chapter.title.is_empty() && !chapter.summary.is_empty()));
    assert!(!snapshot.goals.is_empty());
}

/// Of `voices`, the lines said today.
pub fn said_today(world: &World, voices: Vec<world_projection::Voice>) -> Vec<String> {
    let now = world.world_time();
    voices
        .into_iter()
        .filter(|voice| {
            let world_projection::SelectionId::Event(id) = voice.moment else {
                return false;
            };
            world
                .events()
                .iter()
                .any(|event| event.id == id && event.world_time == now)
        })
        .map(|voice| voice.line)
        .collect()
}
