//! The first look at a three-year harbour after a day: the snapshot the
//! player sees as the day ends, made afresh because the World has changed
//! (a second look at an unchanged World is a copy, barred at 15 ms in
//! world-library's `three_years`). Only the look is timed, not the day.
//! In release:
//!
//! ```text
//! cargo test --release -p tiny-society --test first_look -- --ignored --nocapture
//! ```
//!
//! `WORLD_MACHINE_THREE_YEARS=<file>` keeps the lived World between runs.

use std::time::{Duration, Instant};

use tiny_society::{TinySociety, TinySocietyBranch};
use world_document::WorldDocument;

const PASS: &str = "tiny-society.let-day-pass";
const DAYS: u64 = 1_080;

/// A warm player's harbour after three years: the first question answered
/// each day, something made every third day, the day let pass.
fn three_years() -> TinySocietyBranch {
    let cache = std::env::var_os("WORLD_MACHINE_THREE_YEARS").map(std::path::PathBuf::from);
    if let Some(bytes) = cache.as_ref().and_then(|path| std::fs::read(path).ok()) {
        let archive = WorldDocument::from_bytes(&bytes).unwrap().archive;
        return TinySociety::resume_archive(&archive).unwrap().branch();
    }
    let mut harbour = TinySocietyBranch::new_world().unwrap();
    for day in 1..=DAYS {
        let snapshot = harbour.projection_snapshot();
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != PASS)
        {
            let _ = harbour.invoke_projection_command(&answer.id.clone());
        }
        if day % 3 == 0 {
            let snapshot = harbour.projection_snapshot();
            if let Some(deed) = snapshot
                .commands
                .iter()
                .filter(|c| c.unavailable.is_none())
                .find(|c| c.hand.as_ref().is_some_and(|hand| hand.verb != "Undo"))
            {
                let _ = harbour.invoke_projection_command(&deed.id.clone());
            }
        }
        harbour.invoke_projection_command(PASS).unwrap();
    }
    if let Some(path) = cache {
        let document = WorldDocument::new(harbour.archive().unwrap());
        std::fs::write(path, document.to_bytes().unwrap()).unwrap();
    }
    harbour
}

/// Its median over 21 days must stay under 20 ms (25–38 ms with the day
/// itself at v0.26, which timed them together).
#[test]
#[ignore]
fn a_three_year_first_look_after_a_day_takes_under_twenty_milliseconds() {
    let mut harbour = three_years();
    let mut snapshot = harbour.projection_snapshot();
    let count = std::env::var("WORLD_MACHINE_SNAPSHOTS")
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(21);
    let mut looks = Vec::new();
    let mut days = Vec::new();
    for _ in 0..count {
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != PASS)
        {
            let _ = harbour.invoke_projection_command(&answer.id.clone());
        }
        let started = Instant::now();
        harbour.invoke_projection_command(PASS).unwrap();
        days.push(started.elapsed());
        let started = Instant::now();
        snapshot = harbour.projection_snapshot();
        looks.push(started.elapsed());
    }
    looks.sort();
    days.sort();
    let median = looks[looks.len() / 2];
    eprintln!(
        "first look on {count} days after day {DAYS}: median {median:?}, fastest {:?}, slowest {:?}; \
         the day before it, alone: median {:?}",
        looks[0],
        looks[looks.len() - 1],
        days[days.len() / 2],
    );
    assert!(median < Duration::from_millis(20), "median {median:?}");
}
