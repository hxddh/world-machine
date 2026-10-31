//! A new player's first minutes in the harbour: Day 1 in fair weather, a
//! hello before any question, an answer that is seen on the scene, a first
//! build that rises a stage a day and is finished in the first session,
//! and nothing sad or unkind said in the first days. A harbour made before
//! keeps its own story and pace.

use crate::{TinySociety, TinySocietyBranch, LEO};
use world_projection::{CanvasItem, ProjectionSnapshot, SelectionId, Stance, Weather};

/// Words a new player should not read in their first days: sorrow,
/// hardship, quarrels and being turned away.
pub(crate) const UNKIND: &[&str] = &[
    "sorry",
    "can't keep",
    "fell out",
    "falling out",
    "had words",
    "shouting",
    "quarrel",
    "knows what they did",
    "done with",
    "don't talk to me",
    "are finished",
    "dip into my savings",
    "can't cover",
    "worried",
    "i need work",
    "died",
    "passed away",
    "funeral",
    "goodbye",
    "farewell",
    "storm",
    "wrecked",
    "broken",
    "can't keep you",
    "owed me",
    "side with",
    "glum",
    "nobody listens",
    "short with each other",
    "can't make the wages",
    "money's tight",
    "no more fishing",
    "closed its doors",
    "can't keep the doors",
    "lonely",
    "cried",
];

/// Everything a player can read now in the World's own words, the
/// briefing among them.
pub(crate) fn readable(snapshot: &ProjectionSnapshot) -> Vec<String> {
    world_pack_testkit::seams::readable(snapshot, true)
}

fn unkind(line: &str) -> Option<&'static str> {
    let lower = line.to_lowercase();
    UNKIND.iter().copied().find(|word| lower.contains(word))
}

fn item(snapshot: &ProjectionSnapshot, id: SelectionId) -> Option<&CanvasItem> {
    snapshot.canvas.items.iter().find(|item| item.id == id)
}

fn questions(snapshot: &ProjectionSnapshot) -> Vec<String> {
    snapshot
        .commands
        .iter()
        .filter(|command| command.question.is_some() && command.unavailable.is_none())
        .map(|command| command.id.clone())
        .collect()
}

fn pass(branch: &mut TinySocietyBranch) {
    branch
        .invoke_projection_command(crate::story::WAIT_COMMAND)
        .unwrap();
}

/// The first plot offer that can be built now: its command, and the plot.
fn first_offer(snapshot: &ProjectionSnapshot) -> (String, String) {
    snapshot
        .canvas
        .plots
        .iter()
        .find_map(|plot| {
            plot.offers
                .iter()
                .find(|offer| offer.unavailable.is_none())
                .map(|offer| (offer.command.clone(), plot.id.clone()))
        })
        .expect("a plot to build on")
}

/// The work standing on a plot now.
fn work_on(branch: &TinySocietyBranch, plot: &str) -> SelectionId {
    let state = branch.world().state();
    hands::on_plots(state)
        .into_iter()
        .find(|(on, _)| on == plot)
        .map(|(_, id)| SelectionId::Entity(id))
        .expect("something stands on the plot")
}

#[test]
fn a_new_harbour_opens_on_day_one_in_fair_weather_with_a_hello_first() {
    let branch = TinySocietyBranch::new_world().unwrap();
    let snapshot = branch.projection_snapshot();
    assert_eq!(snapshot.moment_label(snapshot.world_time), "Day 1");
    assert_eq!(snapshot.weather, Weather::Clear);
    // Leo's hello is the first thing said, before anything is asked.
    let first = snapshot.voices.first().expect("someone speaks");
    assert_eq!(first.speaker, SelectionId::Entity(LEO));
    assert!(first.line.contains("Welcome"), "{}", first.line);
    let events = branch.world().events();
    let hello = events.iter().position(|event| event.kind == "greeted");
    let asked = events
        .iter()
        .position(|event| crate::story::is_storylet(event) || event.kind == "situation_arose");
    assert!(hello.is_some());
    assert!(asked.is_none_or(|asked| hello < Some(asked)));
    // The old storm and what came of it are not a new harbour's story.
    for kind in [
        "storm_started",
        "boat_damaged",
        "loan_requested",
        "worker_dismissed",
        "hardship_began",
    ] {
        assert!(
            events.iter().all(|event| event.kind != kind),
            "a new harbour opens with {kind}"
        );
    }
    // And the host opens a new World the same way.
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(crate::tiny_society_registration())
        .unwrap();
    let session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
    let opened = session.snapshot();
    assert_eq!(opened.moment_label(opened.world_time), "Day 1");
    assert_eq!(opened.voices.first(), snapshot.voices.first());
}

/// Whatever the player answers first, something new stands on the scene
/// at once, and whoever asked goes over to it.
#[test]
fn the_first_answer_is_seen_on_the_scene() {
    let opened = TinySocietyBranch::new_world().unwrap();
    let asked = questions(&opened.projection_snapshot());
    assert!(asked.len() >= 2, "{asked:?}");
    for answer in asked {
        let mut branch = opened.clone();
        let before = branch.projection_snapshot();
        branch.invoke_projection_command(&answer).unwrap();
        let after = branch.projection_snapshot();
        let new = after
            .canvas
            .items
            .iter()
            .filter(|item| self::item(&before, item.id).is_none())
            .map(|item| item.id)
            .collect::<Vec<_>>();
        assert!(!new.is_empty(), "{answer}: nothing new on the scene");
        let asker = before
            .commands
            .iter()
            .find(|command| command.id == answer)
            .and_then(|command| command.asker)
            .expect("someone asked");
        let asker_now = item(&after, asker).expect("the asker is on the scene");
        assert!(
            asker_now.at.is_some_and(|at| new.contains(&at)),
            "{answer}: {:?} stays at {:?}",
            asker_now.label,
            asker_now.at
        );
        assert_ne!(item(&before, asker).unwrap().at, asker_now.at);
    }
}

/// The first build rises a stage a day: its site is marked out the day it
/// is begun, its frame is up in scaffolding the next, and it is finished on
/// the third day, each stage a recorded Event told in the harbour's
/// history. The next takes the harbour's usual time, and is nearly done
/// before it is finished.
#[test]
fn the_first_build_rises_a_stage_a_day_and_is_finished_on_the_third() {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    let (build, plot) = first_offer(&branch.projection_snapshot());
    branch.invoke_projection_command(&build).unwrap();
    let work = work_on(&branch, &plot);
    let mut arts = Vec::new();
    for day in 0..3 {
        if day > 0 {
            pass(&mut branch);
        }
        let snapshot = branch.projection_snapshot();
        let shown = item(&snapshot, work).expect("on the scene");
        assert_eq!(shown.built.is_some(), day == 2, "day {day}: {shown:?}");
        arts.push(shown.going_up().then(|| shown.art.clone()).flatten());
    }
    assert_eq!(arts[0].as_deref(), Some("site"));
    assert_eq!(arts[1].as_deref(), Some("scaffold"));
    assert_eq!(arts[2], None, "finished, it is drawn as itself");
    let told = branch
        .world()
        .events()
        .iter()
        .filter(|event| matches!(event.kind.as_str(), "work_rose" | "plot_finished"))
        .map(|event| (event.kind.as_str(), hands::told(event).unwrap_or_default()))
        .collect::<Vec<_>>();
    assert_eq!(told.len(), 2, "{told:?}");
    assert!(told[0].1.starts_with("The frame of"), "{told:?}");
    assert_eq!(told[1].0, "plot_finished");

    let (build, plot) = first_offer(&branch.projection_snapshot());
    branch.invoke_projection_command(&build).unwrap();
    let next = work_on(&branch, &plot);
    for _ in 0..4 {
        pass(&mut branch);
    }
    let shown = item(&branch.projection_snapshot(), next).cloned().unwrap();
    assert!(shown.built.is_none(), "the next takes longer: {shown:?}");
    assert!(
        hands::rising(
            branch.world().state(),
            hands::made(branch.world().state())
                .into_iter()
                .max()
                .unwrap()
        ) == Some(2),
        "and is nearly done first"
    );
}

/// How a scripted first session goes, one day at a time.
#[derive(Clone, Copy, Debug)]
enum Player {
    /// Answers every question with its first answer and builds once.
    Warm,
    /// Answers every question with its last answer, often a no.
    Refusing,
    /// Builds on a plot every day and answers nothing.
    Builder,
    /// Only lets the days pass.
    Idle,
}

/// Nothing sad or unkind is said to a new player in their first days
/// (a first session and more), whatever they do.
#[test]
fn nothing_sad_or_unkind_in_a_new_players_first_days() {
    let mut found = Vec::new();
    for player in [
        Player::Warm,
        Player::Refusing,
        Player::Builder,
        Player::Idle,
    ] {
        let mut branch = TinySocietyBranch::new_world().unwrap();
        let read = |branch: &TinySocietyBranch, found: &mut Vec<String>| {
            for line in readable(&branch.projection_snapshot()) {
                if let Some(word) = unkind(&line) {
                    found.push(format!("{player:?}: {word:?} in {line:?}"));
                }
            }
        };
        for day in 0..crate::arrival::FIRST_DAYS {
            read(&branch, &mut found);
            let snapshot = branch.projection_snapshot();
            let asked = questions(&snapshot);
            let choice = match player {
                Player::Warm => asked.first().cloned(),
                Player::Refusing => asked.last().cloned(),
                Player::Builder => snapshot
                    .canvas
                    .plots
                    .iter()
                    .flat_map(|plot| &plot.offers)
                    .find(|offer| offer.unavailable.is_none())
                    .map(|offer| offer.command.clone()),
                Player::Idle => None,
            };
            if let Some(choice) = choice {
                branch.invoke_projection_command(&choice).unwrap();
                read(&branch, &mut found);
            }
            if matches!(player, Player::Warm) && day == 0 {
                let (build, _) = first_offer(&branch.projection_snapshot());
                branch.invoke_projection_command(&build).unwrap();
                read(&branch, &mut found);
            }
            pass(&mut branch);
        }
    }
    found.sort();
    found.dedup();
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// A harbour begun the old way keeps its storm, its pace and its story:
/// nothing here reaches a World that did not begin with a new player.
#[test]
fn a_harbour_from_before_keeps_its_story_and_pace() {
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    let state = branch.world().state();
    assert_eq!(crate::arrival::arrived(state), None);
    assert!(!crate::arrival::first_days(state));
    assert_eq!(crate::handwork::kit(state).first_growing, None);
    assert!(branch
        .world()
        .events()
        .iter()
        .any(|event| event.kind == "worker_dismissed"));
    let (build, plot) = first_offer(&branch.projection_snapshot());
    branch.invoke_projection_command(&build).unwrap();
    let work = work_on(&branch, &plot);
    for _ in 0..3 {
        pass(&mut branch);
    }
    let shown = item(&branch.projection_snapshot(), work).cloned().unwrap();
    assert!(
        shown.going_up() && shown.built.is_none(),
        "an old harbour's first build takes the usual time: {shown:?}"
    );
}

/// Whether someone's day takes them to `at`, standing as `stance` there.
fn someone_at(snapshot: &ProjectionSnapshot, at: SelectionId, stance: Stance) -> bool {
    snapshot.canvas.items.iter().any(|person| {
        person
            .day
            .iter()
            .any(|stop| stop.at == at && stop.stance == Some(stance))
    })
}

/// The hand command that does `deed` (`build.bench`, `decorate.flowerboxes`)
/// somewhere it can be done now.
fn hand_command(snapshot: &ProjectionSnapshot, deed: &str) -> String {
    snapshot
        .commands
        .iter()
        .find(|command| {
            command.unavailable.is_none()
                && crate::handwork::parse_command(&command.id)
                    .is_some_and(|key| key.starts_with(&format!("{deed}.")))
        })
        .map(|command| command.id.clone())
        .unwrap_or_else(|| panic!("no {deed} to do"))
}

/// What the player has just made by hand, the newest.
fn newest_made(branch: &TinySocietyBranch) -> SelectionId {
    let made = hands::made(branch.world().state());
    SelectionId::Entity(*made.iter().max().expect("something made"))
}

/// Residents are seen at what the player does: while a build goes up one
/// works at its site and another carries a load over to it; a bench put
/// down is sat on, and flower boxes watered, the same day or the next.
/// It is all in their day plans, the same every time, and nothing of it is
/// recorded.
#[test]
fn residents_lend_a_hand_and_use_what_the_player_puts_down() {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    let (build, plot) = first_offer(&branch.projection_snapshot());
    branch.invoke_projection_command(&build).unwrap();
    let site = work_on(&branch, &plot);
    let bench = hand_command(&branch.projection_snapshot(), "build.bench");
    branch.invoke_projection_command(&bench).unwrap();
    let bench = newest_made(&branch);
    let events = branch.world().events().len();
    let today = branch.projection_snapshot();
    assert_eq!(branch.world().events().len(), events, "nothing recorded");
    assert_eq!(today, branch.projection_snapshot(), "the same every time");
    assert!(
        someone_at(&today, site, Stance::Working),
        "nobody works at the site"
    );
    assert!(
        someone_at(&today, site, Stance::Carrying),
        "nobody carries to the site"
    );
    pass(&mut branch);
    let boxes = hand_command(&branch.projection_snapshot(), "decorate.flowerboxes");
    branch.invoke_projection_command(&boxes).unwrap();
    let boxes = newest_made(&branch);
    let next = branch.projection_snapshot();
    assert!(
        someone_at(&next, site, Stance::Working),
        "the site the next day"
    );
    assert!(
        someone_at(&today, bench, Stance::Sitting) || someone_at(&next, bench, Stance::Sitting),
        "nobody sits on the new bench"
    );
    pass(&mut branch);
    let after = branch.projection_snapshot();
    assert!(
        someone_at(&next, boxes, Stance::Working) || someone_at(&after, boxes, Stance::Working),
        "nobody waters the new flower boxes"
    );
}
