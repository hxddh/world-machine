//! A new player's first minutes in the harbour: Day 1 in fair weather, a
//! hello before any question, an answer that is seen on the scene, a first
//! build that goes up in scaffolding and is finished in the first session,
//! and nothing sad or unkind said in the first days. A harbour made before
//! keeps its own story and pace.

use crate::{TinySociety, TinySocietyBranch, LEO};
use world_projection::{CanvasItem, ProjectionSnapshot, SelectionId, Weather};

/// Words a new player should not read in their first days: sorrow,
/// hardship, quarrels and being turned away.
const UNKIND: &[&str] = &[
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

/// The first build goes up in scaffolding and is finished the next day;
/// the next one takes the harbour's usual time.
#[test]
fn the_first_build_shows_scaffolding_and_is_finished_the_next_day() {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    let (build, plot) = first_offer(&branch.projection_snapshot());
    branch.invoke_projection_command(&build).unwrap();
    let work = work_on(&branch, &plot);
    let going_up = branch.projection_snapshot();
    let shown = item(&going_up, work).expect("on the scene");
    assert_eq!(shown.art.as_deref(), Some("scaffold"));
    assert_eq!(shown.built, None);
    pass(&mut branch);
    let finished = branch.projection_snapshot();
    let shown = item(&finished, work).expect("on the scene");
    assert_ne!(shown.art.as_deref(), Some("scaffold"));
    assert!(shown.built.is_some(), "finished on day 2");
    assert!(branch
        .world()
        .events()
        .iter()
        .any(|event| event.kind == "plot_finished"));

    let (build, plot) = first_offer(&finished);
    branch.invoke_projection_command(&build).unwrap();
    let next = work_on(&branch, &plot);
    pass(&mut branch);
    assert_eq!(
        item(&branch.projection_snapshot(), next)
            .unwrap()
            .art
            .as_deref(),
        Some("scaffold"),
        "the next takes longer"
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
    pass(&mut branch);
    assert_eq!(
        item(&branch.projection_snapshot(), work)
            .unwrap()
            .art
            .as_deref(),
        Some("scaffold"),
        "an old harbour's first build takes the usual time"
    );
}
