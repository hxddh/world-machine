//! A new Tiny Society World's first session, as the full app plays it: one
//! sitting, from the town opening on Day 1 to Goodnight, walked at the
//! intended pace of `demo/pacing.json` (the game's own measure of how long
//! play takes; see `tests/walking`). The host's sitting ledger says when
//! the sitting is spent, as the app's does; the World is told only that a
//! sitting began, and passes its days by its own card.
//!
//! The first session is designed, not counted: before Goodnight it walks
//! the whole loop (someone asks something of the player, a build is marked
//! out, rises a stage a day and stands finished, a letter comes, the
//! first festival is got ready for, and something is found for the book),
//! and at the intended pace it takes 20 to 40 minutes.

use std::collections::BTreeMap;
use std::path::PathBuf;
use world_library::{DurableWorldSession, WorldDocumentId, WorldLibrary};
use world_machine_desktop::demo::{self, Pace};
use world_observer::SittingLedger;
use world_projection::{ProjectionIntent, ProjectionSnapshot, SelectionId, SITTING_BEGINS_COMMAND};

mod walking;
use walking::Walker;

/// The beats of a first session, in the order a player meets them.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Beat {
    /// Someone of the place asks something of the player.
    Request,
    /// Something the player had built is going up: its site marked out,
    /// or its frame in scaffolding.
    BuildBegun,
    /// What the player had built rose a stage on a later day: a site
    /// marked out one day stands in scaffolding the next.
    BuildRose,
    /// A letter is written to the player.
    Letter,
    /// Someone is getting ready for the first festival, days ahead.
    FestivalPreview,
    /// Something turned up for the book.
    FirstFind,
    /// What the player had built stands finished, a day or more after.
    BuildFinished,
    /// The sitting is spent, and the Goodnight card comes up.
    Goodnight,
}

fn temp_root(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-first-session-{}-{nonce}-{label}",
        std::process::id()
    ))
}

/// What a first session saw: when each beat came, in seconds of play, and
/// when each day began.
struct Session {
    beats: BTreeMap<Beat, f32>,
    days: Vec<(u64, f32)>,
}

/// A new World's first sitting, played at `pace` until Goodnight.
fn first_sitting(label: &str, pace: &Pace) -> Session {
    let root = temp_root(label);
    let library = WorldLibrary::new(root.join("Library"));
    let registry = world_builtins::registry().unwrap();
    let document = WorldDocumentId::new(format!("first-session-{label}")).unwrap();
    let session =
        DurableWorldSession::create(document, demo::PACK_ID, &registry, &library).unwrap();
    let mut walker = Walker {
        session,
        registry,
        library,
        pace,
        clock: 2.0,
        heard: Vec::new(),
        made: Vec::new(),
        plotted: false,
        mornings: Vec::new(),
    };
    // The app sits down with the World: a first sitting, and the World told
    // a sitting began (its first day opens on its welcome, not a morning).
    let mut ledger = SittingLedger::default();
    let today = 20_000;
    let sitting = ledger.sit(today);
    assert_eq!(sitting.number, 1);
    let _ = walker.session.handle(
        ProjectionIntent::InvokeCommand(SITTING_BEGINS_COMMAND.into()),
        &walker.registry,
        &walker.library,
    );
    let mut snapshot = walker.session.snapshot();
    let pass = demo::day_pass(&snapshot).expect("a day to pass").id.clone();
    let mut beats = BTreeMap::new();
    let mut days = Vec::new();
    // What the player had built, going up: by when it was first seen so,
    // and how it was drawn then.
    let mut going_up = BTreeMap::<SelectionId, (u64, Option<String>)>::new();
    loop {
        let day = snapshot.world_time;
        days.push((day, walker.clock));
        let mut see = |snapshot: &ProjectionSnapshot, clock: f32| {
            let mut note = |beat: Beat| {
                beats.entry(beat).or_insert(clock);
            };
            if snapshot
                .commands
                .iter()
                .any(|command| command.question.is_some() && command.asker.is_some())
                || snapshot.favour.is_some()
            {
                note(Beat::Request);
            }
            for item in &snapshot.canvas.items {
                let since = going_up.get(&item.id).map(|(since, _)| *since);
                let later = since.is_some_and(|since| since < snapshot.world_time);
                if item.going_up() {
                    let (_, first) = going_up
                        .entry(item.id)
                        .or_insert((snapshot.world_time, item.art.clone()));
                    note(Beat::BuildBegun);
                    if later && *first != item.art {
                        note(Beat::BuildRose);
                    }
                } else if item.built.is_some() && later {
                    note(Beat::BuildFinished);
                }
            }
            if !snapshot.letters.is_empty()
                || snapshot
                    .morning
                    .as_ref()
                    .is_some_and(|morning| morning.letter.is_some())
            {
                note(Beat::Letter);
            }
            let festival = snapshot
                .calendar
                .as_ref()
                .filter(|calendar| !calendar.festival_today)
                .and_then(|calendar| calendar.coming.as_deref())
                .and_then(|coming| coming.split(" in ").next())
                .map(str::to_string);
            // Said by someone getting ready, or told in the day's history.
            if festival.is_some_and(|festival| {
                snapshot
                    .voices
                    .iter()
                    .any(|voice| voice.line.contains(&festival))
                    || snapshot
                        .timeline
                        .items
                        .iter()
                        .any(|item| item.title.contains(&festival))
            }) {
                note(Beat::FestivalPreview);
            }
            if snapshot
                .book
                .iter()
                .any(|entry| entry.shelf == "Finds" && entry.found)
            {
                note(Beat::FirstFind);
            }
        };
        // The World as the day leaves it is the one whose day passes.
        walker.play_day(snapshot, &pass, &mut see);
        // Let the day pass: the day's own card, while the sitting has days
        // left, as the window lets it.
        walker.clock += 3.0;
        snapshot = walker
            .session
            .handle(
                ProjectionIntent::InvokeCommand(pass.clone()),
                &walker.registry,
                &walker.library,
            )
            .expect("a day passes");
        let sitting = ledger.spend_day(today);
        see(&snapshot, walker.clock);
        if sitting.spent() {
            // The Goodnight card, read and said.
            walker.clock += pace.choose_seconds;
            beats.insert(Beat::Goodnight, walker.clock);
            break;
        }
    }
    walker.session.flush().unwrap();
    drop(walker);
    let _ = std::fs::remove_dir_all(&root);
    Session { beats, days }
}

#[test]
fn a_first_session_walks_the_whole_loop_in_twenty_to_forty_minutes() {
    let pace = demo::pacing()
        .paces
        .get("intended")
        .expect("the intended pace");
    let session = first_sitting("intended", pace);
    let goodnight = session.beats[&Beat::Goodnight];
    println!(
        "first session: {goodnight:.0} s ({:.1} min); days began at {:?}; beats {:?}",
        goodnight / 60.0,
        session
            .days
            .iter()
            .map(|(time, at)| format!("{time}@{at:.0}"))
            .collect::<Vec<_>>(),
        session.beats
    );
    for beat in [
        Beat::Request,
        Beat::BuildBegun,
        Beat::BuildRose,
        Beat::Letter,
        Beat::FestivalPreview,
        Beat::FirstFind,
        Beat::BuildFinished,
    ] {
        let at = session
            .beats
            .get(&beat)
            .unwrap_or_else(|| panic!("{beat:?} never came before Goodnight"));
        assert!(*at < goodnight, "{beat:?} at {at:.0} s, after Goodnight");
    }
    let minutes = goodnight / 60.0;
    assert!(
        (20.0..=40.0).contains(&minutes),
        "the first session takes {minutes:.1} minutes"
    );
}
