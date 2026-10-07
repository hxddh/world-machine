//! The harbour's words over three years, for the five players: nothing
//! said too often in year three, nothing told more than twice in a week
//! across cards, the return film, notes and strips, nothing sad in the
//! first five days, no seam by day 1,080 and no engine word anywhere
//! (`world_pack_testkit::words`).
//!
//! The three years take a while:
//! `cargo test -p tiny-society --lib words_tests -- --ignored --nocapture`;
//! `PLAYERS=Last` plays only that player, `WORDS_DAYS=360` fewer days.

use crate::TinySocietyBranch;
use world_pack_testkit::players::{self, Player, PlayerWorld};
use world_pack_testkit::seams;
use world_pack_testkit::words::Words;

impl TinySocietyBranch {
    /// The film a player returning now would be shown, having last seen
    /// the harbour with `since` Events.
    pub(crate) fn film_since(&self, since: usize) -> world_projection::BriefingProjection {
        crate::projection::briefing_from(self.world(), Some(since), true)
    }
}

/// Plays `player` for `days` days, watching every word.
pub(crate) fn words_of(player: Player, days: usize) -> Words {
    let mut words = Words::new(format!("{player:?}"), crate::first_minutes::UNKIND);
    let branch = TinySocietyBranch::new_world().unwrap();
    players::play_watched(
        format!("{player:?}"),
        player,
        days,
        branch,
        |day, branch, after| {
            let told =
                (day.day % 30 == 0 || day.day == days).then(|| crate::seams_tests::told(branch));
            words.watch(
                day.day,
                day.began,
                day.events_before,
                branch.world(),
                after,
                told,
                |since| branch.returned(since),
            );
            for letter in seams::self_told(branch.world(), after) {
                words.seams.insert(format!("{player:?}: {letter}"));
            }
            if day.day == days {
                words.look_at_strips(&crate::moments::moments(branch.world()), after);
                for moment in crate::moments::moments(branch.world()) {
                    if let Some(missing) = crate::seams_tests::unshown(&moment) {
                        words
                            .seams
                            .insert(format!("{player:?}: {:?} shows no {missing}", moment.title));
                    }
                }
            }
        },
    );
    words
}

fn days() -> usize {
    std::env::var("WORDS_DAYS")
        .ok()
        .and_then(|days| days.parse().ok())
        .unwrap_or(players::DAYS)
}

#[test]
#[ignore]
fn five_players_words_over_three_years() {
    let days = days();
    let asked = players::players_asked();
    let words = std::thread::scope(|scope| {
        asked
            .iter()
            .map(|player| {
                std::thread::Builder::new()
                    .stack_size(64 << 20)
                    .spawn_scoped(scope, move || words_of(*player, days))
                    .unwrap()
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    let mut failed = Vec::new();
    for words in &words {
        failed.extend(words.report());
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// The five players' first month reads well: nothing sad or unkind in
/// the first days, no engine word and no seam.
#[test]
fn the_first_month_reads_well_for_every_player() {
    let mut failed = Vec::new();
    for player in players::PLAYERS {
        let words = words_of(player, 30);
        failed.extend(words.sad.iter().cloned());
        failed.extend(words.engine.iter().cloned());
        failed.extend(words.seams.iter().cloned());
        assert!(
            words.said[0].values().sum::<usize>() > 100,
            "{player:?} heard too little to judge"
        );
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// Coming back after a few days away, nothing is told twice in a row: the
/// film tells no question the player is about to be asked, nor one whose
/// answer it tells, and the note left for the player tells something the
/// film did not.
#[test]
fn a_return_tells_each_thing_once() {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    let mut checked = 0;
    for day in 1..=90_u32 {
        let snapshot = branch.projection_snapshot();
        if day % 3 != 0 {
            if let Some(answer) = snapshot
                .commands
                .iter()
                .find(|command| command.question.is_some() && command.unavailable.is_none())
            {
                let _ = branch.invoke_projection_command(&answer.id.clone());
            }
            branch
                .invoke_projection_command(crate::story::WAIT_COMMAND)
                .unwrap();
            continue;
        }
        // Away for three days, then back.
        let cursor = branch.visit_cursor();
        branch.advance_days(3).unwrap();
        branch.leave_keepsake(cursor).unwrap();
        let film = branch.film_since(cursor.event_count);
        let beats = film.beats();
        let world = branch.world();
        for beat in &beats {
            let Some(world_projection::SelectionId::Event(id)) = beat.selection else {
                continue;
            };
            let event = world.event(id).unwrap();
            if event.kind == "situation_arose" {
                let storylet = event.payload.get("storylet");
                let settled = world.events()[cursor.event_count..].iter().any(|other| {
                    other.kind != "situation_arose" && other.payload.get("storylet") == storylet
                });
                let open = match storylet {
                    Some(world_core::Value::Text(id)) => {
                        storylets::opened_at(world.state(), crate::story::deck(), id).is_some()
                    }
                    _ => false,
                };
                assert!(
                    !settled && !open,
                    "day {day}: the film tells {:?} as it came up",
                    beat.title
                );
            }
        }
        for (at, kept) in beats
            .iter()
            .enumerate()
            .filter(|(_, beat)| !beat.detail.is_empty())
        {
            for (_, beat) in beats.iter().enumerate().filter(|(other, _)| *other != at) {
                let told = beat.title.trim_end_matches('.').to_lowercase();
                assert!(
                    told.len() < 12 || !kept.detail.to_lowercase().contains(&told),
                    "day {day}: the note {:?} tells the film's {:?} again",
                    kept.detail,
                    beat.title
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "no note was left on any return");
}
