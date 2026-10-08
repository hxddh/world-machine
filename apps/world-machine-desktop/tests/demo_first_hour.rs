//! How far into Tiny Society a newcomer's first hour goes, on a clock.
//!
//! This extends the first-session harness in Tiny Society
//! (`a_first_session_greets_offers_and_gives_in_time`, whose pacing it
//! reuses: the window takes 2 s to open, reading takes a second for every
//! `chars_per_second` characters, choosing takes 12 s, and the window says
//! what is said now at 4.6 s a page of two lines) to a whole hour: each day
//! the player hears what is said, reads the morning's news, answers every
//! question, makes what their hands can, talks to someone, looks in the
//! drawer and at the scene, and then lets the day pass. The demo's cut
//! (`demo::LAST_DAY`) is chosen from what it prints.

use world_machine_desktop::demo;
use world_projection::{Ears, ProjectionIntent, ProjectionSnapshot};

/// How one newcomer spends their time.
#[derive(Clone, Copy, Debug)]
struct Pace {
    name: &'static str,
    /// Characters read a second.
    chars_per_second: f32,
    /// Seconds to find and choose an answer or a deed.
    choose: f32,
    /// Seconds spent each day just watching the place, the book or the
    /// drawer.
    look: f32,
    /// How many times a day they talk to someone in their own words, each
    /// taking 15 s to type.
    talks: usize,
}

const PACES: [Pace; 3] = [
    Pace {
        name: "brisk",
        chars_per_second: 20.0,
        choose: 8.0,
        look: 20.0,
        talks: 1,
    },
    Pace {
        name: "steady",
        chars_per_second: 15.0,
        choose: 12.0,
        look: 45.0,
        talks: 1,
    },
    Pace {
        name: "lingering",
        chars_per_second: 12.0,
        choose: 15.0,
        look: 90.0,
        talks: 2,
    },
];

const HOUR: f32 = 3600.0;

fn day(snapshot: &ProjectionSnapshot) -> u32 {
    let length = snapshot
        .calendar
        .as_ref()
        .map(|calendar| calendar.length)
        .expect("Tiny Society keeps a calendar");
    demo::day_of(snapshot.world_time, length)
}

/// What a player at `pace` saw in `seconds` of play.
struct Played {
    /// The day they were on when the time was up.
    day: u32,
    /// How many days they let pass.
    passed: u32,
    /// The days on which a chapter closed.
    chapters: Vec<u32>,
}

/// A newcomer playing at `pace` for `seconds`, or until `until` if that
/// day comes first.
fn play(pace: Pace, seconds: f32, until: u32) -> Played {
    let read = |text: &str| text.chars().count() as f32 / pace.chars_per_second;
    let page = |text: &str| 4.6 * text.chars().count().div_ceil(72) as f32;
    let registry = world_builtins::registry().unwrap();
    let mut session = registry.create(demo::PACK_ID).unwrap();
    let mut snapshot = session.snapshot();
    let pass = demo::day_pass(&snapshot).expect("a day to pass").id.clone();
    let mut clock = 2.0;
    let mut heard = Vec::new();
    let mut passed = 0;
    let mut chapters = Vec::new();
    loop {
        // What is said now, in turn, and the morning's news.
        for voice in &snapshot.voices {
            if !heard.contains(voice) {
                clock += page(&voice.line);
                heard.push(voice.clone());
            }
        }
        if let Some(briefing) = &snapshot.briefing {
            for item in &briefing.items {
                clock += read(&item.title) + read(&item.detail);
            }
        }
        clock += pace.look;
        // Every question, and the deeds the hands allow.
        let mut asked = 0;
        loop {
            let next = snapshot
                .commands
                .iter()
                .find(|command| {
                    command.unavailable.is_none()
                        && (command.question.is_some() || command.hand.is_some())
                        && command.id != pass
                })
                .cloned();
            let Some(command) = next.filter(|_| asked < 6) else {
                break;
            };
            asked += 1;
            if let Some(question) = &command.question {
                clock += read(&question.prompt);
                for answer in snapshot
                    .commands
                    .iter()
                    .filter(|other| other.question.as_ref() == Some(question))
                {
                    clock += read(&answer.title);
                }
            }
            clock += pace.choose;
            let before = snapshot.clone();
            let Ok(after) = session.handle(ProjectionIntent::InvokeCommand(command.id.clone()))
            else {
                break;
            };
            snapshot = after;
            for voice in &snapshot.voices {
                if !heard.contains(voice) {
                    clock += 1.0 + read(&voice.line);
                    heard.push(voice.clone());
                }
            }
            for keepsake in snapshot.keepsakes.iter().skip(before.keepsakes.len()) {
                clock += read(&keepsake.what) + read(&keepsake.note);
            }
            for letter in snapshot.letters.iter().skip(before.letters.len()) {
                clock += read(&letter.note);
            }
        }
        // A word with someone on the scene.
        let mut people = Vec::new();
        for talk in &snapshot.talks {
            if !people.contains(&talk.who) {
                people.push(talk.who);
            }
        }
        for (turn, who) in people.into_iter().take(pace.talks).enumerate() {
            let words = ["How are you today?", "What are you working on?"][turn % 2];
            clock += 15.0;
            if let Ok(after) = session.handle(ProjectionIntent::Say {
                to: who,
                words: words.into(),
                ears: Ears::Own,
            }) {
                if let Some(exchange) = after.exchanges.last() {
                    clock += read(&exchange.answer);
                }
                snapshot = after;
            }
        }
        if clock >= seconds || day(&snapshot) >= until {
            return Played {
                day: day(&snapshot),
                passed,
                chapters,
            };
        }
        // Let the day pass.
        clock += 3.0;
        snapshot = session
            .handle(ProjectionIntent::InvokeCommand(pass.clone()))
            .expect("a day passes");
        passed += 1;
        if snapshot.chapters.len() > chapters.len() {
            chapters.push(day(&snapshot));
        }
    }
}

#[test]
fn the_day_pass_command_is_the_one_the_demo_holds() {
    let registry = world_builtins::registry().unwrap();
    let session = registry.create(demo::PACK_ID).unwrap();
    let snapshot = session.snapshot();
    assert_eq!(day(&snapshot), 1, "a new harbour opens on day 1");
    // The Pack says which choice lets the day pass, with its role; nothing
    // in the app knows its id.
    let pass = demo::day_pass(&snapshot).expect("Tiny Society lets a day pass");
    assert!(demo::passes_time(&snapshot, &pass.id));
    let others = snapshot
        .commands
        .iter()
        .filter(|command| command.id != pass.id)
        .collect::<Vec<_>>();
    assert!(!others.is_empty());
    for command in others {
        assert!(!demo::passes_time(&snapshot, &command.id), "{}", command.id);
    }
}

/// The demo holds the whole first hour at every pace, a brisk one's too.
#[test]
fn the_demo_holds_the_first_hour() {
    let mut latest = 0;
    for pace in PACES {
        let played = play(pace, HOUR, u32::MAX);
        println!(
            "{}: day {} after an hour ({} days let pass; chapters closed on days {:?})",
            pace.name, played.day, played.passed, played.chapters
        );
        latest = latest.max(played.day);
    }
    assert!(
        demo::LAST_DAY >= latest,
        "a brisk first hour reaches day {latest}, past the demo's day {}",
        demo::LAST_DAY
    );
}

/// The demo ends on the day a chapter closes, the first such day after a
/// brisk first hour, so its last evening follows a chapter's ending card
/// rather than stopping mid-story.
#[test]
fn the_demo_ends_as_a_chapter_closes() {
    let brisk = PACES[0];
    let hour = play(brisk, HOUR, u32::MAX).day;
    let played = play(brisk, f32::MAX, demo::LAST_DAY);
    println!("chapters closed on days {:?}", played.chapters);
    assert_eq!(played.day, demo::LAST_DAY);
    assert_eq!(
        played.chapters.last(),
        Some(&demo::LAST_DAY),
        "a chapter closes as the demo's last day begins"
    );
    assert!(
        !played
            .chapters
            .iter()
            .any(|day| (hour..demo::LAST_DAY).contains(day)),
        "an earlier chapter's close after the first hour would do: {:?}",
        played.chapters
    );
}

/// The ending card speaks Chinese in Chinese, whole, like every window.
#[test]
fn the_ending_card_is_in_the_catalogs() {
    let catalog = world_i18n::Catalog::parse(world_gpui::i18n::APP_ZH_HANS);
    for line in [
        demo::ENDING_TITLE,
        demo::ENDING_BODY,
        demo::ENDING_KEPT,
        demo::ENDING_STAY,
        demo::ENDING_FULL_APP,
    ] {
        let shown = catalog
            .translate(line)
            .unwrap_or_else(|| panic!("not in zh-Hans: {line}"));
        let english = shown
            .split(|c: char| !c.is_ascii_alphabetic())
            .filter(|word| word.len() >= 2 && !["World", "Machine"].contains(word))
            .collect::<Vec<_>>();
        assert!(english.is_empty(), "{line} → {shown}");
    }
}
