//! Words without seams: every line the harbour says, over a year and more
//! for three kinds of player, and every line its people's own voices can
//! make, reads as written, never as a template filled in. No slot or its
//! name, no "Harbor", no clause told twice, no sentence glued on with a
//! colon, and no letter whose writer speaks of themselves by name.

use crate::TinySocietyBranch;
use std::collections::BTreeSet;
use world_core::EntityId;
use world_pack_testkit::seams::{self, readable};

/// Days each player plays: a year of the harbour's and a season more.
const DAYS: u64 = crate::almanac::YEAR_DAYS + 30;

#[derive(Clone, Copy, Debug)]
enum Player {
    /// Answers with the first answer, and makes something every third day.
    Warm,
    /// Answers with the last answer, often a no.
    Refusing,
    /// Builds on a plot whenever one is free, and answers nothing.
    Builder,
}

/// Everything a harbour can tell now: what a player reads, and every
/// legend, moment and almanac it keeps.
pub(crate) fn told(branch: &TinySocietyBranch) -> Vec<String> {
    let world = branch.world();
    let subjects: Vec<EntityId> = world
        .state()
        .entities()
        .filter(|entity| matches!(entity.kind.as_str(), "resident" | "fixture" | "location"))
        .map(|entity| entity.id)
        .collect();
    seams::told(
        readable(&branch.projection_snapshot(), true),
        subjects,
        crate::moments::moments(world),
        crate::almanac_page::years(world),
        |request| branch.story(request),
    )
}

fn play(player: Player, found: &mut BTreeSet<String>) {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    let mut seen = BTreeSet::new();
    for day in 0..DAYS {
        let snapshot = branch.projection_snapshot();
        let asked = snapshot
            .commands
            .iter()
            .filter(|command| command.question.is_some() && command.unavailable.is_none())
            .map(|command| command.id.clone())
            .collect::<Vec<_>>();
        let build = snapshot
            .canvas
            .plots
            .iter()
            .flat_map(|plot| &plot.offers)
            .find(|offer| offer.unavailable.is_none())
            .map(|offer| offer.command.clone());
        let made = snapshot
            .commands
            .iter()
            .find(|command| {
                command.unavailable.is_none()
                    && command
                        .hand
                        .as_ref()
                        .is_some_and(|hand| hand.verb != "Undo")
            })
            .map(|command| command.id.clone());
        let choices = match player {
            Player::Warm => [asked.first().cloned(), made.filter(|_| day % 3 == 0)],
            Player::Refusing => [asked.last().cloned(), None],
            Player::Builder => [build, None],
        };
        for choice in choices.into_iter().flatten() {
            let _ = branch.invoke_projection_command(&choice);
        }
        let lines = if day % 10 == 9 || day + 1 == DAYS {
            told(&branch)
        } else {
            readable(&branch.projection_snapshot(), true)
        };
        seams::note_seams(&format!("{player:?}"), lines, &mut seen, found);
        for letter in seams::self_told(branch.world(), &branch.projection_snapshot()) {
            found.insert(format!("{player:?}: {letter}"));
        }
        branch
            .invoke_projection_command(crate::story::WAIT_COMMAND)
            .unwrap();
    }
    // Anyone who has lived here a year has a life of five lines or more.
    let today = crate::arrival::today(branch.world().state()) as u32;
    let residents = branch
        .world()
        .state()
        .entities()
        .filter(|entity| entity.kind == "resident")
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    found.extend(seams::short_lives(
        &format!("{player:?}"),
        residents,
        today,
        crate::almanac::YEAR_DAYS,
        |request| branch.story(request),
    ));
    for moment in crate::moments::moments(branch.world()) {
        if let Some(missing) = unshown(&moment) {
            found.insert(format!("{player:?}: {:?} shows no {missing}", moment.title));
        }
    }
}

/// What a moment's panels fail to show of what it is: a farewell with no
/// ferry, a wedding with no bunting, a caption that names the ferry with
/// none drawn.
pub(crate) fn unshown(moment: &world_projection::Moment) -> Option<&'static str> {
    use world_projection::{MomentKind, Prop};
    for panel in &moment.panels {
        if panel.caption.to_lowercase().contains("ferry") && !panel.props.contains(&Prop::Ferry) {
            return Some("ferry for its caption");
        }
    }
    let shown = &moment.panels[1].props;
    if moment.kind == MomentKind::Farewell
        && moment.title.ends_with("farewell")
        && !shown.contains(&Prop::Ferry)
    {
        return Some(Prop::Ferry.id());
    }
    seams::unshown(moment)
}

#[test]
fn every_line_the_harbour_says_reads_without_seams() {
    let mut found = BTreeSet::new();
    for player in [Player::Warm, Player::Refusing, Player::Builder] {
        play(player, &mut found);
    }
    // And every line the core residents' own voices can make.
    for voice in crate::talk::RESIDENTS
        .into_iter()
        .filter_map(crate::voices::voice)
    {
        found.extend(seams::voice_seams(voice));
    }
    seams::assert_no_seams(found);
}

/// A legend counts its years as the harbour does: the year the calendar,
/// the almanac and everyone's age are counted in is one and the same, so
/// "a year in the town" on a legend is a year of the harbour's.
#[test]
fn a_legend_counts_the_harbours_own_years() {
    let branch = TinySocietyBranch::new_world().unwrap();
    let snapshot = branch.projection_snapshot();
    let calendar = snapshot.calendar.expect("the harbour keeps a calendar");
    assert_eq!(calendar.year, Some(crate::almanac::YEAR_DAYS));
    assert_eq!(crate::kin::KIN.year, crate::almanac::YEAR_DAYS);
    assert_eq!(
        crate::almanac_page::year_of(crate::almanac::YEAR_DAYS * calendar.length),
        2
    );
}
