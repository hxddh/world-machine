//! The place's stories: legends whose every cause was recorded, moments
//! that come the same way every time the history is read, and an almanac
//! at each New Year, in each of the three places.

use crate::{PocketUniverse, NUDGE_COMMAND};
use std::collections::BTreeMap;
use world_core::World;
use world_projection::{engine_words_in, Moment, SelectionId, StoryPage, StoryRequest};

/// A warm player in one place for `periods`: the first question each
/// period answered, a hand lent every third. Returns the moments and
/// almanac years each snapshot carried as they came.
fn lived(seed: &str, periods: u64) -> (PocketUniverse, BTreeMap<String, Moment>, Vec<u32>) {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    let (mut came, mut almanacs) = (BTreeMap::new(), Vec::new());
    for period in 0..periods {
        let snapshot = universe.projection_snapshot();
        if let Some(pick) = snapshot.commands.iter().find(|command| {
            command.question.is_some()
                && command.unavailable.is_none()
                && command.id != NUDGE_COMMAND
        }) {
            let _ = universe.invoke_projection_command(&pick.id.clone());
        }
        if period % 3 == 2 {
            if let Some(deed) = snapshot
                .commands
                .iter()
                .find(|command| command.hand.is_some() && command.unavailable.is_none())
            {
                let _ = universe.invoke_projection_command(&deed.id.clone());
            }
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        let after = universe.projection_snapshot();
        assert!(after.moments.len() <= world_projection::MOST_MOMENTS_IN_SNAPSHOT);
        for moment in after.moments {
            came.insert(moment.id.clone(), moment);
        }
        almanacs.extend(after.almanac.map(|almanac| almanac.year));
    }
    (universe, came, almanacs)
}

/// Every legend line names a cause only with one the World recorded, no
/// later than the line; returns how many lines, and how many with one.
fn legends_hold(world: &World) -> (usize, usize) {
    let (mut lines, mut caused) = (0, 0);
    for entity in world.state().entities() {
        let Some(legend) = crate::legends::legend(world, SelectionId::Entity(entity.id)) else {
            continue;
        };
        assert!(legend.lines.len() <= chronicle::MOST_LEGEND_LINES);
        for line in &legend.lines {
            lines += 1;
            // The line is the World's own telling, as its history shows it.
            let event = world.event(line.event.unwrap()).unwrap();
            match (&line.because, line.cause) {
                (Some(because), Some(cause)) => {
                    caused += 1;
                    assert!(world.event(cause).is_some_and(|cause| cause.id <= event.id));
                    assert!(engine_words_in(frame(because)).is_empty(), "{because}");
                }
                (None, None) => {}
                other => panic!("words and cause apart: {other:?}"),
            }
        }
    }
    (lines, caused)
}

fn holds(seed: &str, periods: u64) {
    let (universe, came, almanacs) = lived(seed, periods);
    let world = universe.world();
    let moments = crate::moments::moments(world);
    assert!(!moments.is_empty(), "{seed}: no moments");
    let mut days = moments.iter().map(|moment| moment.day).collect::<Vec<_>>();
    days.dedup();
    assert_eq!(days.len(), moments.len(), "{seed}: two moments on one day");
    for moment in &moments {
        assert!(moment.cast().len() <= 4);
        for panel in &moment.panels {
            assert!(!panel.caption.is_empty() && !panel.caption.contains('{'));
            assert!(panel.caption.ends_with('.'), "{}", panel.caption);
        }
        // Before and after are in this module's words; the moment itself,
        // and a work's name, are the World's own ("A seed vault under the
        // rock").
        let own_words = moment.kind != world_projection::MomentKind::WorkOpened;
        for panel in [&moment.panels[0], &moment.panels[2]]
            .into_iter()
            .filter(|_| own_words)
        {
            assert!(
                engine_words_in(&panel.caption).is_empty(),
                "{}",
                panel.caption
            );
        }
        assert!(!moment.cast().is_empty() || moment.panels[1].place.is_some());
        assert_eq!(came.get(&moment.id), Some(moment), "{seed}: {}", moment.id);
        let Some(StoryPage::Moment(asked)) =
            universe.story(StoryRequest::Moment(moment.id.clone()))
        else {
            panic!("{seed}: cannot open {}", moment.id);
        };
        assert_eq!(&asked, moment);
    }
    assert_eq!(came.len(), moments.len(), "{seed}: a moment was taken back");
    assert_eq!(crate::moments::moments(&world.replay().unwrap()), moments);
    let years = crate::almanac_page::year_of(world.world_time()) - 1;
    assert_eq!(almanacs, (1..=years as u32).collect::<Vec<_>>(), "{seed}");
    for year in 1..=years as u32 {
        assert!(matches!(
            universe.story(StoryRequest::Almanac(year)),
            Some(StoryPage::Almanac(page)) if page.year == year
        ));
    }
    let (lines, caused) = legends_hold(world);
    assert!(
        lines > 20 && caused * 3 >= lines,
        "{seed}: {caused} of {lines}"
    );
    eprintln!(
        "{seed}: {} moments, {lines} legend lines, {caused} with a cause",
        moments.len()
    );
}

#[test]
fn a_first_year_on_mars_is_told() {
    holds(crate::SEED_MARS_COLONY_COMMAND, 125);
}

#[test]
#[ignore]
fn three_years_of_stories_in_every_place() {
    std::thread::scope(|scope| {
        for seed in [
            crate::SEED_MARS_COLONY_COMMAND,
            crate::SEED_1980S_TOWN_COMMAND,
            crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
        ] {
            scope.spawn(move || holds(seed, 1_080));
        }
    });
}

/// A cause's words without the player's own, quoted: "because you said".
fn frame(because: &str) -> &str {
    because.split('“').next().unwrap_or(because)
}
