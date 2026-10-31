//! The demo's whole slice, walked headlessly on a clock.
//!
//! A newcomer plays a new Tiny Society World in the ordinary Library, the
//! way the demo window plays it: the window takes 2 s to open; what is said
//! now is shown at 4.6 s a page of two lines; reading takes a second for
//! every `chars_per_second` characters; choosing takes `choose_seconds`.
//! Each day they hear what is said, answer the cards, make what their
//! hands can and do the favour asked (if their pace does), have a word with
//! someone, look about, and let the day pass, which the sitting ledger and
//! the demo's own gate (`demo::gate`) answer as the window does:
//!
//! - the first sitting passes its six World days (the designed first
//!   session, `tests/first_session.rs`), then the Goodnight card comes up;
//! - the demo plays freely, so "Sleep now" begins the second sitting at
//!   once through the ledger (`SittingLedger::sleep_now`, no wall-clock
//!   time in the World), and the World, told a sitting began, opens on its
//!   morning card;
//! - that sitting's one day is the demo's last: letting it pass shows the
//!   wishlist card, and the day is held, not spent.
//!
//! The beats and their budgets are the demo's pacing (`demo/pacing.json`):
//! they must arrive in order and in time. Nothing here asks a model: the
//! people answer in the World's own words, as they do with the voice off.

use std::collections::BTreeMap;
use std::path::PathBuf;
use world_document::DocumentSittings;
use world_library::{DurableWorldSession, WorldDocumentId, WorldLibrary};
use world_machine_desktop::demo::{self, Beat, Pace};
use world_observer::{SittingLedger, FIRST_SITTING_DAYS};
use world_projection::{
    CanvasItemKind, ProjectionIntent, ProjectionSnapshot, SelectionId, SITTING_BEGINS_COMMAND,
};

mod walking;
use walking::Walker;

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

/// The ledger as the World's file carries it, as the app writes it.
fn keep(walker: &mut Walker, ledger: &SittingLedger) {
    walker
        .session
        .set_sittings(
            DocumentSittings {
                sittings: ledger.sittings,
                began_on: ledger.began_on,
                days_passed: ledger.days_passed,
                morning_read: ledger.morning_read.clone(),
            },
            &walker.library,
        )
        .expect("the sittings written down");
}

/// The day the walk sits on: any one day of the player's will do, since
/// the demo's second sitting comes at once, the same day.
const TODAY: i64 = 20_000;

/// What a walk saw.
struct Walked {
    /// When each beat arrived, in seconds of play.
    beats: BTreeMap<Beat, f32>,
    /// The seconds at which each day began.
    days: Vec<(u32, f32)>,
    /// The whole demo, to the end of reading the wishlist card.
    total: f32,
    /// The World as the wishlist card showed it.
    snapshot: ProjectionSnapshot,
    /// The wishlist card's recap.
    recap: Vec<String>,
    /// The ledger as the demo left it.
    ledger: SittingLedger,
    root: PathBuf,
    document: WorldDocumentId,
}

/// The demo, played at `pace` from a new World to its wishlist card.
fn walk(label: &str, pace: &Pace) -> Walked {
    let root = temp_root(label);
    let library = WorldLibrary::new(root.join("Library"));
    let registry = world_builtins::registry().unwrap();
    let document = WorldDocumentId::new(format!("demo-walk-{label}")).unwrap();
    let session =
        DurableWorldSession::create(document.clone(), demo::PACK_ID, &registry, &library).unwrap();
    let mut beats = BTreeMap::new();
    let mut days = Vec::new();
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
    // Home opens the World on its first sitting, written in its file, and
    // the World is told a sitting began (its first day opens on its
    // welcome, not a morning).
    let mut ledger = SittingLedger::default();
    assert_eq!(ledger.sit(TODAY).number, 1);
    keep(&mut walker, &ledger);
    let _ = walker.session.handle(
        ProjectionIntent::InvokeCommand(SITTING_BEGINS_COMMAND.into()),
        &walker.registry,
        &walker.library,
    );
    let mut snapshot = walker.session.snapshot();
    let pass = demo::day_pass(&snapshot).expect("a day to pass").id.clone();

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
    beats.insert(Beat::Painted, walker.clock);
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

    fn note(beats: &mut BTreeMap<Beat, f32>, beat: Beat, at: f32) {
        beats.entry(beat).or_insert(at);
    }
    loop {
        days.push((day(&snapshot), walker.clock));
        snapshot = walker.play_day(snapshot, &pass, &mut |snapshot, clock| {
            if snapshot.favour.is_some() {
                note(&mut beats, Beat::Favour, clock);
            }
            // A work going up on the scene (its scaffolding a part higher
            // than when the town opened), or something made that was not
            // there then, standing on the scene.
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
                note(&mut beats, Beat::Build, clock);
            }
        });
        // Let the day pass, as the window and the demo answer it: the
        // sitting has a day left (the window asks first), then the gate.
        walker.clock += 3.0;
        assert!(
            !world_observer::sitting_at(&ledger, TODAY).spent(),
            "a day asked of a spent sitting"
        );
        match demo::gate(
            demo::PACK_ID,
            Some(day(&snapshot)),
            demo::passes_time(&snapshot, &pass),
        ) {
            demo::Gate::Open => {
                snapshot = walker
                    .session
                    .handle(
                        ProjectionIntent::InvokeCommand(pass.clone()),
                        &walker.registry,
                        &walker.library,
                    )
                    .expect("a day passes");
                note(&mut beats, Beat::DayPassed, walker.clock);
                let sitting = ledger.spend_day(TODAY);
                keep(&mut walker, &ledger);
                if sitting.spent() {
                    // The Goodnight card, read, and "Sleep now" chosen.
                    assert_eq!(sitting.number, 1, "only the first sitting ends");
                    assert_eq!(day(&snapshot), 1 + FIRST_SITTING_DAYS);
                    note(&mut beats, Beat::Goodnight, walker.clock);
                    walker.clock += pace.choose_seconds;
                    let second = ledger.sleep_now(TODAY);
                    assert_eq!((second.number, second.left), (2, 1));
                    keep(&mut walker, &ledger);
                    snapshot = walker
                        .session
                        .handle(
                            ProjectionIntent::InvokeCommand(SITTING_BEGINS_COMMAND.into()),
                            &walker.registry,
                            &walker.library,
                        )
                        .expect("the World told a sitting began");
                    assert!(snapshot.morning.is_some(), "the second sitting's morning");
                    note(&mut beats, Beat::SecondMorning, walker.clock);
                }
            }
            demo::Gate::Ending => {
                note(&mut beats, Beat::Ending, walker.clock);
                break;
            }
            demo::Gate::NotOffered => panic!("the demo offers Tiny Society"),
        }
    }
    // The day held is not spent: the sitting keeps it.
    let now = world_observer::sitting_at(&ledger, TODAY);
    assert_eq!(
        (now.number, now.left),
        (2, 1),
        "the last day held, not spent"
    );
    // The wishlist card, read, and the wishlist chosen.
    let recap = demo::recap(&snapshot);
    let game = walker
        .registry
        .descriptor(demo::PACK_ID)
        .expect("the demo's Pack")
        .title
        .clone();
    for line in recap.iter().cloned().chain([
        demo::ENDING_TITLE.to_string(),
        demo::GOODBYE.to_string(),
        demo::ENDING_BODY.to_string(),
        demo::ENDING_KEPT.to_string(),
        demo::wishlist_label(&game),
    ]) {
        walker.clock += walker.read(&line);
    }
    walker.clock += pace.choose_seconds;
    walker.session.flush().unwrap();
    Walked {
        beats,
        days,
        total: walker.clock,
        snapshot,
        recap,
        ledger,
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

/// The bars: the whole slice (the first session, Goodnight, the second
/// morning and the wishlist card) is played, every beat arrives, in order,
/// within its budget, at the pace it names; at the intended pace the whole
/// demo is 25 to 45 minutes.
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
    // In order, at every pace: the beats come as `Beat` lists them, and
    // every one of them, to the wishlist card.
    for (name, walked) in &walks {
        let mut seen = walked.beats.iter().collect::<Vec<_>>();
        seen.sort_by(|a, b| a.1.total_cmp(b.1));
        let order = seen.iter().map(|(beat, _)| **beat).collect::<Vec<_>>();
        let mut expected = order.clone();
        expected.sort();
        assert_eq!(order, expected, "{name}: {seen:?}");
        assert_eq!(order.len(), 7, "{name}: every beat: {seen:?}");
        assert_eq!(
            walked.days.last().map(|(day, _)| *day),
            Some(demo::last_day()),
            "{name}: played to the last day"
        );
        assert_eq!(walked.ledger.sittings, 2, "{name}: two sittings");
    }
    let minutes = walks["intended"].total / 60.0;
    assert!(
        (25.0..=45.0).contains(&minutes),
        "the demo takes {minutes:.1} minutes at the intended pace"
    );
    for walked in walks.into_values() {
        let _ = std::fs::remove_dir_all(&walked.root);
    }
}

/// The wishlist card recaps what the player did, in their own deeds, and
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

/// Demo saves carry over: the demo's World is an ordinary World, its
/// sittings in its file. The full app opens it from the same Library and
/// plays on past the demo's last day, with everything the demo recorded,
/// on the sitting the demo left (its last day still to pass that day).
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
    let kept = session.sittings().expect("the sittings carried over");
    assert_eq!((kept.sittings, kept.days_passed), (2, 0));
    assert_eq!(kept.began_on, walked.ledger.began_on);
    // The full app does not ask the demo's gate: the held day comes.
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

/// Every word the demo says of its own speaks Chinese and Japanese, whole,
/// like every window: the wishlist card, its button with the game's name
/// (its Pack's own, in each language) and Steam's, and what the demo says
/// of a World it does not offer.
#[test]
fn the_demos_words_are_in_the_catalogs() {
    let registry = world_builtins::registry().unwrap();
    let game = registry
        .descriptor(demo::PACK_ID)
        .expect("the demo's Pack")
        .title
        .clone();
    assert_eq!(
        demo::wishlist_label(&game),
        format!("Wishlist {game} on Steam"),
        "in English, the Pack's own name"
    );
    for (language, text) in [
        ("zh-Hans", world_gpui::i18n::APP_ZH_HANS),
        ("ja", world_gpui::i18n::APP_JA),
    ] {
        let catalog = world_i18n::Catalog::parse(text);
        let named = catalog
            .translate(&game)
            .unwrap_or_else(|| panic!("the game's name is not in {language}: {game}"));
        assert!(!named.is_empty());
        // The button, its slot filled with the game's name as the
        // language writes it.
        let filled = catalog.expand(demo::ENDING_WISHLIST, &[("game", &[game.as_str()])]);
        let [(english, wishlist)] = filled.as_slice() else {
            panic!("not in {language}: {} ({filled:?})", demo::ENDING_WISHLIST);
        };
        assert_eq!(*english, demo::wishlist_label(&game));
        assert!(wishlist.contains(&named), "{wishlist}");
        assert!(wishlist.contains("Steam"), "{wishlist}");
        let english_words = |shown: &str| {
            shown
                .split(|c: char| !c.is_ascii_alphabetic())
                .filter(|word| {
                    word.len() >= 2
                        && !["World", "Machine", "Steam", "Tiny", "Society"].contains(word)
                })
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        assert!(english_words(wishlist).is_empty(), "{wishlist}");
        for line in [
            demo::ENDING_TITLE,
            demo::ENDING_BODY,
            demo::ENDING_KEPT,
            demo::ENDING_STAY,
            demo::RECAP_TITLE,
            demo::KEEP_POSTCARD,
            demo::GOODBYE,
            demo::NOT_OFFERED,
            demo::HOME_TITLE,
        ] {
            let shown = catalog
                .translate(line)
                .unwrap_or_else(|| panic!("not in {language}: {line}"));
            // Names read the same in any language: the app's, the store's
            // and the game's.
            let english = english_words(&shown);
            assert!(english.is_empty(), "{line} → {shown}");
        }
    }
}
