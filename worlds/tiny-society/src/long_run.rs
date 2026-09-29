//! Three years in the harbour, played by the warm player of the second
//! year's test: they answer the first question on offer each day, make
//! something new every third day, and let the day pass. The works keep
//! moving, the third year is not the first two again, letters stay rare,
//! and no stretch of days goes by with nothing new.
//!
//! One run of 1,080 days is shared by every test here. It takes a few
//! minutes in a debug build, so the tests are ignored by default: run them
//! with `cargo test -p tiny-society long_run -- --ignored`.

use crate::{story, TinySociety};
use std::collections::BTreeSet;
use std::sync::OnceLock;

const DAYS: usize = 1_080;
/// The days of a stretch the checks look at.
const STRETCH: usize = 120;

/// What each day of the run brought.
pub(crate) struct Run {
    /// Lines said each day, and how many of them were never heard before.
    lines: Vec<(usize, usize)>,
    /// Days that brought nothing new: nothing found in the book, nothing
    /// to keep, no letter and no chapter's close.
    quiet: Vec<bool>,
    /// Letters written each day.
    letters: Vec<usize>,
    /// The days a work (a rung of the ladder) was finished.
    finished: Vec<usize>,
    /// For each rung, the longest run of days it could have come up and
    /// did not.
    longest_wait: Vec<(&'static str, usize)>,
    /// The World as the run left it.
    pub(crate) world: world_core::World,
    /// How the place stood on days 30, 360 and 1,080, and every festival.
    place: Vec<(usize, crate::town::Bars)>,
    /// Every moment the snapshots carried as they came, by id, and the
    /// years whose almanac came on a New Year's day.
    pub(crate) moments: std::collections::BTreeMap<String, world_projection::Moment>,
    pub(crate) almanacs: Vec<u32>,
}

pub(crate) fn run() -> &'static Run {
    static RUN: OnceLock<Run> = OnceLock::new();
    RUN.get_or_init(play)
}

fn play() -> Run {
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    let deck = story::deck();
    let rungs = story::ladder()
        .iter()
        .map(|rung| (rung.id, format!("work_{}", rung.id)))
        .collect::<Vec<_>>();
    let mut waiting = vec![0_usize; rungs.len()];
    let mut longest_wait = vec![0_usize; rungs.len()];
    let mut heard = BTreeSet::new();
    let mut run = Run {
        lines: Vec::new(),
        quiet: Vec::new(),
        letters: Vec::new(),
        finished: Vec::new(),
        longest_wait: Vec::new(),
        world: world_core::World::new(Default::default()),
        place: Vec::new(),
        moments: Default::default(),
        almanacs: Vec::new(),
    };
    // Something new, as the month's test counts it: something found in
    // the book, something to keep, a letter or a chapter's close. The
    // book is counted by what has ever been found in it, so someone away
    // for a while (and off its pages) does not make a day look quiet.
    let mut found = BTreeSet::new();
    let mut news = |snapshot: &world_projection::ProjectionSnapshot| {
        let mut new = 0;
        for entry in snapshot.book.iter().filter(|entry| entry.found) {
            new += usize::from(found.insert((entry.shelf.clone(), entry.name.clone())));
        }
        (
            new,
            snapshot.keepsakes.len() + snapshot.letters.len() + snapshot.chapters.len(),
        )
    };
    let mut before = branch.projection_snapshot();
    news(&before);
    let mut done_before = 0;
    for day in 1..=DAYS {
        let snapshot = before;
        let now = snapshot.world_time;
        let (mut said, mut new) = (0, 0);
        for voice in snapshot.voices.iter().filter(|voice| {
            snapshot
                .timeline
                .items
                .iter()
                .any(|item| item.id == voice.moment && item.world_time == now)
        }) {
            said += 1;
            new += usize::from(heard.insert(voice.line.clone()));
        }
        run.lines.push((said, new));
        // A rung that could come up and is not open is waiting.
        let state = branch.world().state();
        for (at, (_, storylet)) in rungs.iter().enumerate() {
            let spec = deck
                .storylets
                .iter()
                .find(|spec| spec.id == storylet.as_str())
                .unwrap();
            if storylets::can_arise(state, deck, spec) {
                waiting[at] += 1;
                longest_wait[at] = longest_wait[at].max(waiting[at]);
            } else {
                waiting[at] = 0;
            }
        }
        if let Some(answer) = snapshot.commands.iter().find(|command| {
            command.question.is_some()
                && command.unavailable.is_none()
                && command.id != story::WAIT_COMMAND
        }) {
            let _ = branch.invoke_projection_command(&answer.id.clone());
        }
        if day % 3 == 0 {
            let unmade = snapshot
                .book
                .iter()
                .filter(|entry| entry.shelf == "Made" && !entry.found)
                .map(|entry| entry.name.clone())
                .collect::<BTreeSet<_>>();
            let hands = snapshot
                .commands
                .iter()
                .filter(|command| command.unavailable.is_none())
                .filter(|command| {
                    command
                        .hand
                        .as_ref()
                        .is_some_and(|hand| hand.verb != "Undo")
                })
                .collect::<Vec<_>>();
            let deed = hands
                .iter()
                .find(|command| unmade.contains(&command.hand.as_ref().unwrap().thing))
                .or(hands.first());
            if let Some(deed) = deed {
                let _ = branch.invoke_projection_command(&deed.id.clone());
            }
        }
        branch
            .invoke_projection_command(story::WAIT_COMMAND)
            .unwrap();
        let after = branch.projection_snapshot();
        assert!(after.moments.len() <= world_projection::MOST_MOMENTS_IN_SNAPSHOT);
        for moment in &after.moments {
            run.moments.insert(moment.id.clone(), moment.clone());
        }
        run.almanacs
            .extend(after.almanac.as_ref().map(|almanac| almanac.year));
        if [30, 360, DAYS].contains(&day) || crate::town::festival_today(branch.world().state()) {
            run.place
                .push((day, crate::town::bars(branch.world(), &after)));
        }
        let (found_new, kept) = news(&after);
        let kept_before =
            snapshot.keepsakes.len() + snapshot.letters.len() + snapshot.chapters.len();
        run.quiet.push(found_new == 0 && kept <= kept_before);
        run.letters
            .push(after.letters.len().saturating_sub(snapshot.letters.len()));
        let state = branch.world().state();
        let done = story::ladder()
            .iter()
            .filter(|rung| storylets::finished(state, deck, rung.id))
            .count();
        if done > done_before {
            run.finished.push(day);
        }
        done_before = done;
        before = after;
    }
    run.longest_wait = rungs
        .iter()
        .zip(longest_wait)
        .map(|((id, _), wait)| (*id, wait))
        .collect();
    let replayed = branch.world().replay().unwrap();
    assert_eq!(replayed.state(), branch.world().state());
    run.world = branch.world().clone();
    run
}

/// What the book, the letter box and the daily round find through the
/// World's index of its history is what reading every event finds: on the
/// World the three years left, on that World replayed, and on the World
/// as it stood at a hundred points along the way.
#[test]
#[ignore]
fn the_index_finds_what_reading_every_event_finds_for_three_years() {
    let world = &run().world;
    let cast = crate::life::cast();
    lives::scanned::compare(world, &cast).unwrap();
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
    lives::scanned::compare(&replayed, &cast).unwrap();
    let events = world.events().len();
    for at in (1..=100).map(|at| events * at / 100) {
        let then = world.fork_after(at).unwrap();
        lives::scanned::compare(&then, &cast).unwrap();
        // So does the briefing.
        assert_eq!(
            crate::projection::briefing_from(&then, None, true),
            crate::projection::briefing_from(&then, None, false),
            "at event {at}"
        );
    }
    let kept = lives::keepsakes(world);
    eprintln!(
        "{} keepsakes, {} letters, {} met, {} firsts in {events} events",
        kept.len(),
        lives::letters(world).len(),
        lives::met(world).len(),
        lives::firsts(world).len()
    );
    assert!(kept.iter().any(|kept| world
        .event(kept.event)
        .is_some_and(|event| event.kind == "enjoyed")));
}

#[test]
#[ignore]
fn the_works_keep_moving_for_three_years() {
    let run = run();
    let mut gaps = run
        .finished
        .windows(2)
        .map(|pair| (pair[1] - pair[0], pair[1]))
        .collect::<Vec<_>>();
    if let Some(last) = run.finished.last() {
        gaps.push((DAYS - last, DAYS));
    }
    let (longest, at) = gaps.iter().copied().max().unwrap_or((DAYS, DAYS));
    eprintln!(
        "{} works finished; longest gap {longest} days, to day {at}",
        run.finished.len()
    );
    assert!(run.finished.len() >= 20, "only {}", run.finished.len());
    assert!(
        longest <= 60,
        "no work finished for {longest} days to day {at}"
    );
    let (rung, wait) = run
        .longest_wait
        .iter()
        .copied()
        .max_by_key(|(_, wait)| *wait)
        .unwrap();
    eprintln!("longest wait to be asked: {wait} days ({rung})");
    assert!(wait <= 45, "{rung} waited {wait} days to be asked");
}

#[test]
#[ignore]
fn every_stretch_of_three_years_is_mostly_new() {
    let run = run();
    let shares = run
        .lines
        .chunks(STRETCH)
        .map(|stretch| {
            let (said, new) = stretch
                .iter()
                .fold((0, 0), |(said, new), (s, n)| (said + s, new + n));
            new as f64 / said.max(1) as f64
        })
        .collect::<Vec<_>>();
    eprintln!(
        "new lines by stretch: {:?}",
        shares
            .iter()
            .map(|share| format!("{:.0}%", share * 100.0))
            .collect::<Vec<_>>()
    );
    for (at, share) in shares.iter().enumerate() {
        assert!(
            *share >= 0.55,
            "only {:.0}% new in days {}-{}",
            share * 100.0,
            at * STRETCH + 1,
            (at + 1) * STRETCH
        );
    }
}

#[test]
#[ignore]
fn letters_stay_rare_and_no_stretch_is_quiet() {
    let run = run();
    let most_letters = run
        .letters
        .windows(7)
        .map(|week| week.iter().sum::<usize>())
        .max()
        .unwrap_or(0);
    let most_quiet = run
        .quiet
        .windows(STRETCH)
        .map(|stretch| stretch.iter().filter(|quiet| **quiet).count())
        .max()
        .unwrap_or(0);
    let quiet_days = run
        .quiet
        .iter()
        .enumerate()
        .filter(|(_, quiet)| **quiet)
        .map(|(day, _)| day + 1)
        .collect::<Vec<_>>();
    eprintln!(
        "at most {most_letters} letters in a week; at most {most_quiet} quiet days in {STRETCH}; {} letters in all; quiet days {quiet_days:?}",
        run.letters.iter().sum::<usize>()
    );
    assert!(most_letters <= 2, "{most_letters} letters in a week");
    assert!(
        most_quiet <= 4,
        "{most_quiet} quiet days in {STRETCH}: {quiet_days:?}"
    );
}

/// Three years of the warm player's harbour: on days 30, 360 and 1,080
/// everyone has a home, couples share one, every finished work stands in
/// a spot of its own (at least 40 of them by the end), nobody is out at ten
/// at night and no one stretch holds more than 60% of the harbour at noon;
/// and on every festival most of the harbour is on the square that evening.
#[test]
#[ignore]
fn the_place_grows_and_lives_for_three_years() {
    let run = run();
    let mut festivals = 0;
    for (day, bars) in &run.place {
        festivals += usize::from(bars.festival);
        crate::town::check_bars(*day, bars);
    }
    let (_, last) = run.place.iter().rfind(|(day, _)| *day == DAYS).unwrap();
    assert!(last.standing >= 40, "{last:?}");
    assert!(festivals >= 30, "{festivals} festivals");
}

/// Three years of lives: at least three children born, one or two of the
/// old dying gently, two comings of age; fewer, weightier changes between
/// people (at most 60 in any year of days); no storylet asked under the
/// same title more than three times; and something new of ten kinds or
/// more in every 30 days of year three.
#[test]
#[ignore]
fn three_years_of_lives() {
    let world = &run().world;
    let count = |kind: &str| world.events_of_kind(&[kind]).len();
    let (born, died, grown) = (count("born"), count("died"), count("came_of_age"));
    eprintln!(
        "born {born}, died {died}, came of age {grown}, retired {}, left home {}, heirlooms {}, memorials {}, anniversaries {}",
        count("retired"),
        count("left_home"),
        count("heirloom_passed"),
        count("memorial_placed"),
        count("anniversary_kept"),
    );
    for event in world.events_of_kind(&[
        "born",
        "came_of_age",
        "died",
        "retired",
        "left_home",
        "heirloom_passed",
        "memorial_placed",
    ]) {
        eprintln!(
            "  day {}: {}",
            event.world_time / crate::persistence::WORLD_DAY_TICKS,
            lives::told(event).unwrap_or_default()
        );
    }
    let cast = crate::life::cast();
    for person in story::people(world)
        .into_iter()
        .chain(lives::children(world.state(), &cast))
    {
        eprintln!(
            "  {} {} {:?}",
            lives::name(world.state(), person),
            lives::age_of(world.state(), &cast, &crate::kin::KIN, person),
            lives::partner(world.state(), person).map(|p| lives::name(world.state(), p))
        );
    }
    let day = |event: &world_core::Event| {
        (event.world_time / crate::persistence::WORLD_DAY_TICKS) as usize
    };
    let bonds = world
        .events_of_kind(&["bond_changed"])
        .into_iter()
        .map(day)
        .collect::<Vec<_>>();
    let most_bonds = (0..=DAYS.saturating_sub(360))
        .map(|start| {
            bonds
                .iter()
                .filter(|at| (start..start + 360).contains(*at))
                .count()
        })
        .max()
        .unwrap_or(0);
    // What each storylet was asked as, the times it came up.
    let mut titles = std::collections::BTreeMap::<String, usize>::new();
    for event in world.events_of_kind(&["situation_arose"]) {
        if let Some(told) = story::told(world, event) {
            *titles.entry(told).or_default() += 1;
        }
    }
    let mut repeated = titles
        .iter()
        .filter(|(_, times)| **times > 3)
        .collect::<Vec<_>>();
    repeated.sort_by_key(|(_, times)| std::cmp::Reverse(**times));
    // Never-seen kinds of thing: an event kind, a storylet or a situation's
    // kind, a festival.
    let mut seen = BTreeSet::new();
    let mut fresh = vec![0_usize; DAYS / 30 + 1];
    for event in world.events() {
        let mut keys = vec![event.kind.clone()];
        for key in ["storylet", "festival", "beat", "bond", "activity"] {
            if let Some(world_core::Value::Text(text)) = event.payload.get(key) {
                keys.push(format!("{key}:{text}"));
            }
        }
        // A question told as it came round (again, once more), and each
        // beat of a life.
        if event.kind == "situation_arose" || lives::is_news(event) {
            if let Some(told) = story::told(world, event) {
                keys.push(format!("told:{told}"));
            }
        }
        if let Some(world_core::Value::Text(situation)) = event.payload.get("situation") {
            keys.push(format!(
                "situation:{}",
                situation.split('.').next().unwrap_or_default()
            ));
        }
        for key in keys {
            if seen.insert(key) {
                fresh[(day(event).min(DAYS - 1)) / 30] += 1;
            }
        }
    }
    let year_three = &fresh[24..36];
    eprintln!(
        "bond changes {} in all, at most {most_bonds} in 360 days; titles over 3: {} {:?}; new kinds per 30 days in year three {year_three:?}",
        bonds.len(),
        repeated.len(),
        repeated.iter().take(8).collect::<Vec<_>>()
    );
    assert!(born >= 3, "{born} born");
    assert!((1..=2).contains(&died), "{died} died");
    assert!(grown >= 2, "{grown} came of age");
    assert!(
        most_bonds <= 60,
        "{most_bonds} changes between people in a year"
    );
    assert!(repeated.is_empty(), "{repeated:?}");
    assert!(year_three.iter().all(|new| *new >= 10), "{year_three:?}");
}
