//! The demo's scripted path, walked headlessly on a clock.
//!
//! A newcomer plays a new Tiny Society World in the ordinary Library, the
//! way the demo window plays it: the window takes 2 s to open; what is said
//! now is shown at 4.6 s a page of two lines; reading takes a second for
//! every `chars_per_second` characters; choosing takes `choose_seconds`.
//! Each day they hear what is said, answer the cards, make what their
//! hands can and do the favour asked (if their pace does), have a word with
//! someone, look about, and let the day pass, which the demo's own gate
//! (`demo::gate`) answers: the full app's day, the time away (the World's
//! background time, then the return film) or the ending card.
//!
//! The beats and their budgets are the demo's pacing (`demo/pacing.json`):
//! they must arrive in order and in time. Nothing here asks a model: the
//! people answer in the World's own words, as they do with the voice off.

use std::collections::BTreeMap;
use std::path::PathBuf;
use world_library::{DurableWorldSession, WorldDocumentId, WorldLibrary};
use world_machine_desktop::demo::{self, Beat, Pace};
use world_projection::{CanvasItemKind, Ears, ProjectionIntent, ProjectionSnapshot, SelectionId};

fn day(snapshot: &ProjectionSnapshot) -> u32 {
    let length = snapshot
        .calendar
        .as_ref()
        .map(|calendar| calendar.length)
        .expect("Tiny Society keeps a calendar");
    demo::day_of(snapshot.world_time, length)
}

fn temp_root(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-demo-walk-{}-{nonce}-{label}",
        std::process::id()
    ))
}

/// What a walk saw.
struct Walked {
    /// When each beat arrived, in seconds of play.
    beats: BTreeMap<Beat, f32>,
    /// The seconds at which each day began.
    days: Vec<(u32, f32)>,
    /// The whole demo, to the end of reading the ending card.
    total: f32,
    /// The World as the ending card showed it.
    snapshot: ProjectionSnapshot,
    /// The ending card's recap.
    recap: Vec<String>,
    root: PathBuf,
    document: WorldDocumentId,
}

/// The demo, played at `pace` from a new World to its ending card.
fn walk(label: &str, pace: &Pace) -> Walked {
    let read = |text: &str| text.chars().count() as f32 / pace.chars_per_second;
    let page = |text: &str| 4.6 * text.chars().count().div_ceil(72) as f32;
    let root = temp_root(label);
    let library = WorldLibrary::new(root.join("Library"));
    let registry = world_builtins::registry().unwrap();
    let document = WorldDocumentId::new(format!("demo-walk-{label}")).unwrap();
    let mut session =
        DurableWorldSession::create(document.clone(), demo::PACK_ID, &registry, &library).unwrap();
    let mut snapshot = session.snapshot();
    let pass = demo::day_pass(&snapshot).expect("a day to pass").id.clone();
    let mut beats = BTreeMap::new();
    let mut days = Vec::new();
    let mut clock = 2.0;

    // The town, as the window opens: a sky, its places and its people.
    let places = |snapshot: &ProjectionSnapshot, kind| {
        snapshot
            .canvas
            .items
            .iter()
            .filter(|item| item.kind == kind)
            .count()
    };
    assert!(snapshot.scenery.is_some(), "a sky to paint");
    assert!(places(&snapshot, CanvasItemKind::Place) >= 3);
    assert!(places(&snapshot, CanvasItemKind::Actor) >= 3);
    beats.insert(Beat::Painted, clock);
    let standing = snapshot
        .canvas
        .items
        .iter()
        .map(|item| item.id)
        .collect::<Vec<SelectionId>>();
    let begun = snapshot
        .goals
        .iter()
        .map(|goal| (goal.id.clone(), goal.done))
        .collect::<BTreeMap<_, _>>();

    let mut heard = Vec::new();
    let mut made = Vec::<String>::new();
    fn note(beats: &mut BTreeMap<Beat, f32>, beat: Beat, at: f32) {
        beats.entry(beat).or_insert(at);
    }
    let see = |beats: &mut BTreeMap<Beat, f32>, snapshot: &ProjectionSnapshot, clock: f32| {
        if snapshot.favour.is_some() {
            note(beats, Beat::Favour, clock);
        }
        // A work going up on the scene (its scaffolding a part higher
        // than when the town opened), or something made that was not there
        // then, standing on the scene.
        let rose = snapshot
            .goals
            .iter()
            .any(|goal| goal.done > begun.get(&goal.id).copied().unwrap_or(0));
        let made = snapshot
            .canvas
            .items
            .iter()
            .any(|item| item.built.is_some() && !standing.contains(&item.id));
        if rose || made {
            note(beats, Beat::Build, clock);
        }
    };
    loop {
        days.push((day(&snapshot), clock));
        // What is said now, in turn (waited for, or not), and the
        // morning's news.
        for voice in &snapshot.voices {
            if !heard.contains(voice) {
                if pace.waits_for_what_is_said {
                    clock += page(&voice.line);
                }
                heard.push(voice.clone());
            }
        }
        if let Some(briefing) = &snapshot.briefing {
            if briefing.returned {
                // The return film: each beat told big, read, and stepped on.
                assert!(!briefing.beats().is_empty(), "a film to show");
                note(&mut beats, Beat::ReturnFilm, clock);
                for beat in briefing.beats() {
                    clock += 1.5 + read(&beat.title) + read(&beat.detail);
                }
            } else if pace.reads_the_news {
                for item in &briefing.items {
                    clock += read(&item.title) + read(&item.detail);
                }
            }
        }
        see(&mut beats, &snapshot, clock);
        clock += pace.look_seconds;
        // The cards, and the deeds the hands allow: something new made,
        // planted or put up, never something taken back or moved.
        let mut asked = 0;
        loop {
            let makes = |command: &world_projection::ProjectionCommand| {
                pace.makes
                    && command.hand.as_ref().is_some_and(|hand| {
                        ["Build", "Plant", "Decorate"].contains(&hand.verb.as_str())
                            && !made.contains(&hand.thing)
                    })
            };
            let next = snapshot
                .commands
                .iter()
                .find(|command| {
                    command.unavailable.is_none()
                        && command.id != pass
                        && (command.question.is_some() || makes(command))
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
            clock += pace.choose_seconds;
            if let Some(hand) = &command.hand {
                made.push(hand.thing.clone());
            }
            let before = snapshot.clone();
            let Ok(after) = session.handle(
                ProjectionIntent::InvokeCommand(command.id.clone()),
                &registry,
                &library,
            ) else {
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
            see(&mut beats, &snapshot, clock);
        }
        // The favour asked, done with its offered reply.
        if pace.favours {
            if let Some(favour) = snapshot.favour.clone().filter(|favour| !favour.done) {
                clock += read(&favour.note) + pace.choose_seconds;
                if let Ok(after) = session.handle(
                    ProjectionIntent::Say {
                        to: favour.whom,
                        words: favour.reply.clone(),
                        ears: Ears::Offered,
                    },
                    &registry,
                    &library,
                ) {
                    if let Some(exchange) = after.exchanges.last() {
                        clock += read(&exchange.answer);
                    }
                    snapshot = after;
                }
            }
        }
        // A word with someone, in the player's own words, heard by the
        // World's own ears (the voice is off).
        let mut people = Vec::new();
        for talk in &snapshot.talks {
            if !people.contains(&talk.who) {
                people.push(talk.who);
            }
        }
        for (turn, who) in people.into_iter().take(pace.talks).enumerate() {
            let words = ["How are you today?", "What are you working on?"][turn % 2];
            clock += 15.0;
            if let Ok(after) = session.handle(
                ProjectionIntent::Say {
                    to: who,
                    words: words.into(),
                    ears: Ears::Own,
                },
                &registry,
                &library,
            ) {
                if let Some(exchange) = after.exchanges.last() {
                    clock += read(&exchange.answer);
                }
                snapshot = after;
            }
        }
        see(&mut beats, &snapshot, clock);
        // Let the day pass, as the demo answers it.
        clock += 3.0;
        match demo::gate(
            demo::PACK_ID,
            Some(day(&snapshot)),
            demo::passes_time(&snapshot, &pass),
        ) {
            demo::Gate::Open => {
                snapshot = session
                    .handle(
                        ProjectionIntent::InvokeCommand(pass.clone()),
                        &registry,
                        &library,
                    )
                    .expect("a day passes");
                note(&mut beats, Beat::DayPassed, clock);
            }
            demo::Gate::Away(days) => {
                let before = day(&snapshot);
                snapshot = session
                    .advance_background(u64::from(days), &registry, &library)
                    .expect("the World lives days away");
                assert_eq!(day(&snapshot), before + days, "every day lived");
                // The window opens anew on the World, as after time away.
                snapshot = session.snapshot();
                assert!(
                    snapshot.briefing.as_ref().is_some_and(|b| b.returned),
                    "the next morning is a return"
                );
                // The window's note on it.
                clock += read(demo::AWAY_NOTE);
            }
            demo::Gate::Ending => {
                note(&mut beats, Beat::Ending, clock);
                break;
            }
            demo::Gate::NotOffered => panic!("the demo offers Tiny Society"),
        }
    }
    // The ending card, read.
    let recap = demo::recap(&snapshot);
    for line in recap.iter().chain([
        &demo::ENDING_TITLE.to_string(),
        &demo::GOODBYE.to_string(),
        &demo::ENDING_BODY.to_string(),
        &demo::ENDING_KEPT.to_string(),
    ]) {
        clock += read(line);
    }
    clock += pace.choose_seconds;
    session.flush().unwrap();
    Walked {
        beats,
        days,
        total: clock,
        snapshot,
        recap,
        root,
        document,
    }
}

fn pace(name: &str) -> &'static Pace {
    demo::pacing()
        .paces
        .get(name)
        .unwrap_or_else(|| panic!("no pace {name}"))
}

/// The bars: every beat arrives, in order, within its budget, at the
/// pace it names; at the intended pace the whole demo is 20 to 30 minutes.
#[test]
fn the_demo_walks_its_beats_in_order_and_in_time() {
    let pacing = demo::pacing();
    let mut walks = BTreeMap::new();
    for name in pacing.paces.keys() {
        let walked = walk(name, pace(name));
        println!(
            "{name}: {:.0} s in all ({:.1} min), days began at {:?}",
            walked.total,
            walked.total / 60.0,
            walked
                .days
                .iter()
                .map(|(day, at)| format!("{day}@{at:.0}"))
                .collect::<Vec<_>>()
        );
        println!("{name}: beats {:?}", walked.beats);
        println!("{name}: recap {:#?}", walked.recap);
        walks.insert(name.clone(), walked);
    }
    for budget in &pacing.beats {
        let walked = &walks[&budget.pace];
        let at = *walked
            .beats
            .get(&budget.beat)
            .unwrap_or_else(|| panic!("{:?} never came at {}", budget.beat, budget.pace));
        let at = if budget.beat == Beat::Ending {
            walked.total
        } else {
            at
        };
        if let Some(by) = budget.by_seconds {
            assert!(at <= by, "{:?} at {at:.0} s, after {by} s", budget.beat);
        }
        if let Some(from) = budget.from_seconds {
            assert!(
                at >= from,
                "{:?} at {at:.0} s, before {from} s",
                budget.beat
            );
        }
    }
    // In order, at every pace: the beats come as `Beat` lists them.
    for (name, walked) in &walks {
        let mut seen = walked.beats.iter().collect::<Vec<_>>();
        seen.sort_by(|a, b| a.1.total_cmp(b.1));
        let order = seen.iter().map(|(beat, _)| **beat).collect::<Vec<_>>();
        let mut expected = order.clone();
        expected.sort();
        assert_eq!(order, expected, "{name}: {seen:?}");
        assert_eq!(order.len(), 6, "{name}: every beat: {seen:?}");
    }
    for walked in walks.into_values() {
        let _ = std::fs::remove_dir_all(&walked.root);
    }
}

/// The ending card recaps what the player did, in their own deeds, and
/// someone out at dusk says goodbye.
#[test]
fn the_ending_recaps_the_players_own_deeds() {
    let walked = walk("recap", pace("intended"));
    let recap = &walked.recap;
    assert!(
        (demo::RECAP_LEAST..=demo::RECAP_MOST).contains(&recap.len()),
        "{} lines: {recap:?}",
        recap.len()
    );
    let deeds = recap.iter().filter(|line| line.starts_with("You ")).count();
    assert!(deeds >= 2, "the player's own deeds: {recap:?}");
    for line in recap {
        assert!(line.contains("you") || line.starts_with("You "), "{line}");
    }
    assert!(demo::goodbye_from(&walked.snapshot).is_some());
    let _ = std::fs::remove_dir_all(&walked.root);
}

/// The demo's World is an ordinary World: the full app opens it from the
/// same Library and it plays on past the demo's last day, with everything
/// the demo recorded.
#[test]
fn the_demo_world_opens_in_the_full_app_and_plays_on() {
    let walked = walk("carry", pace("answer_first"));
    let library = WorldLibrary::new(walked.root.join("Library"));
    // The full app's registry is the demo's: the demo adds no Pack.
    let registry = world_builtins::registry().unwrap();
    let mut session = DurableWorldSession::open(walked.document.clone(), &registry, &library)
        .expect("the full app opens the demo's World");
    let opened = session.snapshot();
    assert_eq!(day(&opened), demo::last_day());
    assert_eq!(opened.world_time, walked.snapshot.world_time);
    assert_eq!(
        opened.timeline.items.len(),
        walked.snapshot.timeline.items.len(),
        "everything the demo recorded"
    );
    assert_eq!(demo::recap(&opened), walked.recap);
    // The full app does not ask the demo's gate: the next day comes.
    let pass = demo::day_pass(&opened).unwrap().id.clone();
    let next = session
        .handle(ProjectionIntent::InvokeCommand(pass), &registry, &library)
        .expect("the full app plays on");
    assert_eq!(day(&next), demo::last_day() + 1);
    session.flush().unwrap();
    drop(session);
    let _ = std::fs::remove_dir_all(&walked.root);
}

#[test]
fn the_day_pass_command_is_the_one_the_demo_holds() {
    let registry = world_builtins::registry().unwrap();
    let session = registry.create(demo::PACK_ID).unwrap();
    let snapshot = session.snapshot();
    assert_eq!(day(&snapshot), 1, "a new World opens on day 1");
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

/// The demo's own words speak Chinese and Japanese, whole, like every
/// window.
#[test]
fn the_demos_words_are_in_the_catalogs() {
    for (language, text) in [
        ("zh-Hans", world_gpui::i18n::APP_ZH_HANS),
        ("ja", world_gpui::i18n::APP_JA),
    ] {
        let catalog = world_i18n::Catalog::parse(text);
        for line in [
            demo::ENDING_TITLE,
            demo::ENDING_BODY,
            demo::ENDING_KEPT,
            demo::ENDING_STAY,
            demo::ENDING_WISHLIST,
            demo::RECAP_TITLE,
            demo::KEEP_POSTCARD,
            demo::GOODBYE,
            demo::AWAY_NOTE,
        ] {
            let shown = catalog
                .translate(line)
                .unwrap_or_else(|| panic!("not in {language}: {line}"));
            let english = shown
                .split(|c: char| !c.is_ascii_alphabetic())
                .filter(|word| word.len() >= 2 && !["World", "Machine"].contains(word))
                .collect::<Vec<_>>();
            assert!(english.is_empty(), "{line} → {shown}");
        }
    }
}
