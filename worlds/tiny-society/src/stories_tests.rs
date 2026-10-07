//! The harbour's stories: legends whose every cause was recorded, moments
//! that come the same way every time the history is read, and an almanac
//! at each New Year.

use crate::{story, TinySociety, TinySocietyBranch};
use std::collections::BTreeMap;
use world_core::World;
use world_projection::{
    engine_words_in, Moment, ProjectionSnapshot, SelectionId, StoryPage, StoryRequest,
};

/// The warm player for `days` days: the first question each day
/// answered, something made every third day. Returns every snapshot's
/// moments and almanac as they came.
fn lived(days: u64) -> (TinySocietyBranch, Vec<ProjectionSnapshot>) {
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    let mut seen = Vec::new();
    for day in 1..=days {
        let snapshot = branch.projection_snapshot();
        if let Some(answer) = snapshot.commands.iter().find(|command| {
            command.question.is_some()
                && command.unavailable.is_none()
                && command.id != story::WAIT_COMMAND
        }) {
            let _ = branch.invoke_projection_command(&answer.id.clone());
        }
        if day % 3 == 0 {
            if let Some(deed) = snapshot.commands.iter().find(|command| {
                command.unavailable.is_none()
                    && command
                        .hand
                        .as_ref()
                        .is_some_and(|hand| hand.verb != "Undo")
            }) {
                let _ = branch.invoke_projection_command(&deed.id.clone());
            }
        }
        branch
            .invoke_projection_command(story::WAIT_COMMAND)
            .unwrap();
        let mut after = branch.projection_snapshot();
        after.book.retain(|entry| entry.moment.is_some());
        seen.push(ProjectionSnapshot {
            moments: after.moments,
            almanac: after.almanac,
            book: after.book,
            world_time: after.world_time,
            ..ProjectionSnapshot::default()
        });
    }
    (branch, seen)
}

fn short() -> &'static (TinySocietyBranch, Vec<ProjectionSnapshot>) {
    static SHORT: std::sync::OnceLock<(TinySocietyBranch, Vec<ProjectionSnapshot>)> =
        std::sync::OnceLock::new();
    SHORT.get_or_init(|| lived(125))
}

/// Everyone and every place the harbour could tell the life of.
fn subjects(world: &World) -> Vec<SelectionId> {
    world
        .state()
        .entities()
        .filter(|entity| matches!(entity.kind.as_str(), "resident" | "location"))
        .map(|entity| SelectionId::Entity(entity.id))
        .collect()
}

/// Every legend's lines name a cause only with one the World recorded,
/// no later than the line, and in words a player uses.
fn legends_hold(world: &World) -> (usize, usize) {
    let (mut lines, mut caused) = (0, 0);
    for subject in subjects(world) {
        let Some(legend) = crate::legends::legend(world, subject) else {
            continue;
        };
        assert!(!legend.title.is_empty());
        assert!(legend.lines.len() <= chronicle::MOST_LEGEND_LINES);
        for line in &legend.lines {
            lines += 1;
            let event = world.event(line.event.unwrap()).unwrap();
            assert_eq!(line.day, world_projection::day_of(event.world_time, 10));
            assert!(engine_words_in(&line.text).is_empty(), "{}", line.text);
            match (&line.because, line.cause) {
                (Some(because), Some(cause)) => {
                    caused += 1;
                    let cause = world.event(cause).expect("a recorded cause");
                    assert!(cause.id <= event.id, "{because}: a cause after the line");
                    assert!(engine_words_in(frame(because)).is_empty(), "{because}");
                    assert!(!because.contains("event") && !because.contains('#'));
                }
                (None, None) => {}
                other => panic!("words and cause apart: {other:?}"),
            }
        }
    }
    (lines, caused)
}

fn assert_drawable(moment: &Moment) {
    assert!(moment.id.starts_with("moment-"));
    assert!(!moment.title.is_empty());
    assert!(moment.cast().len() <= 4, "{moment:?}");
    for panel in &moment.panels {
        assert!(panel.caption.ends_with('.'), "{}", panel.caption);
        assert!(!panel.caption.is_empty(), "{moment:?}");
        assert!(!panel.caption.contains('{'), "{}", panel.caption);
        assert!(
            engine_words_in(&panel.caption).is_empty(),
            "{}",
            panel.caption
        );
        assert!(panel.place.is_some());
    }
}

#[test]
fn a_first_season_is_told_with_its_causes() {
    let (branch, _) = short();
    let (lines, caused) = legends_hold(branch.world());
    assert!(lines > 30, "{lines} lines");
    assert!(
        caused * 3 >= lines,
        "{caused} of {lines} lines name a cause"
    );
    // A story is asked for through the branch, and the host, the same way.
    let mara = SelectionId::Entity(crate::MARA);
    let Some(StoryPage::Legend(legend)) = branch.story(StoryRequest::Legend(mara)) else {
        panic!("no legend for Mara");
    };
    assert_eq!(legend.title, "Mara");
    assert!(branch
        .story(StoryRequest::Legend(SelectionId::Event(
            world_core::EventId::new(1)
        )))
        .is_none());
}

#[test]
fn moments_are_kept_in_the_book_with_faces_and_the_snapshot_keeps_three() {
    let (branch, seen) = short();
    let world = branch.world();
    let moments = crate::moments::moments(world);
    assert!(moments.len() >= 12, "{} moments", moments.len());
    for moment in &moments {
        assert_drawable(moment);
    }
    let mut days = moments.iter().map(|moment| moment.day).collect::<Vec<_>>();
    days.dedup();
    assert_eq!(days.len(), moments.len(), "two moments on one day");
    // Each one came in a snapshot as it happened, as it is told now.
    let mut came = BTreeMap::new();
    for snapshot in seen {
        assert!(snapshot.moments.len() <= 3);
        for moment in &snapshot.moments {
            came.insert(moment.id.clone(), moment.clone());
        }
    }
    for moment in &moments {
        assert_eq!(came.get(&moment.id), Some(moment), "{}", moment.id);
    }
    // The book keeps all of them, each with its cast to draw.
    let last = seen.last().unwrap();
    assert_eq!(last.book.len(), moments.len());
    for entry in &last.book {
        let id = entry.moment.clone().unwrap();
        assert!(!entry.cast.is_empty(), "{} has no faces", entry.name);
        let Some(StoryPage::Moment(moment)) = branch.story(StoryRequest::Moment(id.clone())) else {
            panic!("the book cannot open {id}");
        };
        assert_eq!(moment.title, entry.name);
    }
}

#[test]
fn a_new_year_brings_the_almanac_of_the_year_before() {
    let (branch, seen) = short();
    let pages = seen
        .iter()
        .filter_map(|snapshot| Some((snapshot.world_time, snapshot.almanac.clone()?)))
        .collect::<Vec<_>>();
    assert_eq!(pages.len(), 1, "one New Year's day, one page");
    let (when, page) = &pages[0];
    assert_eq!(crate::almanac_page::year_of(*when), 2);
    assert_eq!(page.year, 1);
    assert!(page.best.is_some());
    assert!(!page.arrived.is_empty(), "{page:?}");
    assert!(!page.built.is_empty(), "{page:?}");
    for name in page.arrived.iter().chain(&page.built) {
        assert!(page.cast.iter().any(|named| &named.name == name), "{name}");
    }
    assert_eq!(
        branch.story(StoryRequest::Almanac(1)),
        Some(StoryPage::Almanac(page.clone()))
    );
    assert_eq!(branch.story(StoryRequest::Almanac(2)), None);
    assert_eq!(branch.story(StoryRequest::Almanac(0)), None);
}

#[test]
fn a_replayed_world_regenerates_every_moment_identically() {
    let (branch, _) = short();
    let world = branch.world();
    let replayed = world.replay().unwrap();
    assert_eq!(
        crate::moments::moments(&replayed),
        crate::moments::moments(world)
    );
    let reopened = TinySociety::resume_json(&branch.archive_json().unwrap()).unwrap();
    assert_eq!(
        crate::moments::moments(reopened.branch().world()),
        crate::moments::moments(world)
    );
}

/// Over three years (the shared long run, in `long_run`): at least twelve
/// moments in each of the harbour's years and never two a day, each the
/// same as the snapshot carried it when it happened and as a replay tells
/// it; an almanac every New Year; every legend's causes recorded; and a
/// legend at day 1,080 told in under 20 ms in a release build.
#[test]
#[ignore]
fn three_years_of_stories() {
    let run = crate::long_run::run();
    let world = &run.world;
    let moments = crate::moments::moments(world);
    let mut per_year = BTreeMap::<u64, usize>::new();
    for moment in &moments {
        assert_drawable(moment);
        let event = world.event(moment.event.unwrap()).unwrap();
        *per_year
            .entry(crate::almanac_page::year_of(event.world_time))
            .or_default() += 1;
    }
    let mut days = moments.iter().map(|moment| moment.day).collect::<Vec<_>>();
    days.dedup();
    assert_eq!(days.len(), moments.len(), "two moments on one day");
    eprintln!("{} moments; per year {per_year:?}", moments.len());
    let whole_years = crate::almanac_page::year_of(world.world_time()) - 1;
    for year in 1..=whole_years {
        let count = per_year.get(&year).copied().unwrap_or_default();
        assert!(count >= 12, "year {year} had {count} moments");
    }
    for moment in &moments {
        assert_eq!(run.moments.get(&moment.id), Some(moment), "{}", moment.id);
    }
    assert_eq!(run.moments.len(), moments.len());
    let replayed = world.replay().unwrap();
    assert_eq!(crate::moments::moments(&replayed), moments);
    assert_eq!(
        run.almanacs,
        (1..=whole_years as u32).collect::<Vec<_>>(),
        "an almanac each New Year"
    );
    let (lines, caused) = legends_hold(world);
    eprintln!("{lines} legend lines, {caused} with a recorded cause");
    assert!(caused * 3 >= lines, "{caused} of {lines}");
    // The longest legend, asked for at the end: timed. Each legend is told
    // five times and its fastest telling counts, so a busy machine pausing
    // the test for another process does not read as a slow legend; the
    // slowest legend must still be told in under 20 ms.
    let slowest = subjects(world)
        .into_iter()
        .map(|subject| {
            let mut fastest = std::time::Duration::MAX;
            let mut lines = 0;
            for _ in 0..5 {
                let started = std::time::Instant::now();
                let legend = crate::legends::legend(world, subject);
                fastest = fastest.min(started.elapsed());
                lines = legend.map_or(0, |legend| legend.lines.len());
            }
            (fastest, lines)
        })
        .max()
        .unwrap();
    eprintln!("slowest legend: {:?} for {} lines", slowest.0, slowest.1);
    if !cfg!(debug_assertions) {
        assert!(slowest.0 < std::time::Duration::from_millis(20));
    }
}

/// A cause's words without the player's own, quoted: "because you said".
fn frame(because: &str) -> &str {
    because.split('“').next().unwrap_or(because)
}

/// Prints two legends and three moments from a 360-day World and from the
/// three-year run, to read:
/// `cargo test --release -p tiny-society --lib show_stories -- --ignored --nocapture`
#[test]
#[ignore]
fn show_stories() {
    let year = lived(360).0.world().clone();
    for (label, world) in [
        ("360 days", &year),
        ("1,080 days", &crate::long_run::run().world),
    ] {
        eprintln!("===== {label}");
        for who in [crate::MARA, crate::JONAS] {
            let legend = crate::legends::legend(world, SelectionId::Entity(who)).unwrap();
            eprintln!("--- {} ({} lines)", legend.title, legend.lines.len());
            for line in &legend.lines {
                match &line.because {
                    Some(because) => eprintln!("  Day {}: {} — {because}", line.day, line.text),
                    None => eprintln!("  Day {}: {}", line.day, line.text),
                }
            }
        }
        let moments = crate::moments::moments(world);
        for moment in moments.iter().rev().take(3) {
            eprintln!(
                "--- Day {}: {} ({:?}, cast {})",
                moment.day,
                moment.title,
                moment.kind,
                moment.cast().len()
            );
            for panel in &moment.panels {
                eprintln!("  [{}] {}", panel.beat.id(), panel.caption);
            }
        }
    }
}
