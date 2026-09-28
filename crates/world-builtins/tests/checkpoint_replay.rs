//! A Tiny Society World replayed from a checkpoint is exactly the World its
//! full replay is: the same state, the same history, nothing decided again.

use tiny_society::TinySociety;
use world_persistence::{ArchivedCheckpoint, WorldArchive};
use world_projection::ProjectionIntent::InvokeCommand;

const PASS: &str = "tiny-society.let-day-pass";
const DAYS: u64 = 150;
/// A season of Tiny Society's days, as a World file checkpoints it.
const SEASON_DAYS: u64 = 30;

/// A player who answers what is asked and makes something now and then.
fn played() -> (WorldArchive, u64) {
    let registry = world_builtins::registry().unwrap();
    let mut session = registry.create(tiny_society::TINY_SOCIETY_PACK_ID).unwrap();
    let mut snapshot = session.snapshot();
    for day in 1..=DAYS {
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != PASS)
        {
            if let Ok(next) = session.handle(InvokeCommand(answer.id.clone())) {
                snapshot = next;
            }
        }
        if day % 4 == 0 {
            let deed = snapshot
                .commands
                .iter()
                .filter(|c| c.unavailable.is_none())
                .find(|c| c.hand.is_some())
                .map(|c| c.id.clone());
            if let Some(deed) = deed {
                let _ = session.handle(InvokeCommand(deed));
            }
        }
        snapshot = session.handle(InvokeCommand(PASS.into())).unwrap();
    }
    let day = snapshot.calendar.expect("a calendar").length;
    (session.archive().unwrap().unwrap(), day)
}

#[test]
fn replay_from_every_seasons_checkpoint_is_the_full_replay() {
    let (archive, day) = played();
    let season = SEASON_DAYS * day;
    let full = TinySociety::resume_archive(&archive).unwrap();
    let mut carried: Option<ArchivedCheckpoint> = None;
    let mut seasons = 0;
    for start in (season..=archive.world_time).step_by(season as usize) {
        // Carried on from the season before, as a World file does, it is
        // the checkpoint summed up afresh.
        let checkpoint = ArchivedCheckpoint::at(carried.as_ref(), &archive, start);
        assert_eq!(
            checkpoint,
            ArchivedCheckpoint::covering(
                &archive.events[..archive
                    .events
                    .partition_point(|event| event.world_time < start)]
            )
        );
        assert!(checkpoint.events > 0);
        // Far fewer changes than the events it sums up hold.
        let changes = archive.events[..checkpoint.events]
            .iter()
            .map(|event| event.changes.len())
            .sum::<usize>();
        assert!(
            checkpoint.changes.len() < changes,
            "{} of {changes}",
            checkpoint.changes.len()
        );

        // With the whole history.
        let mut whole = archive.clone();
        whole.checkpoint = Some(checkpoint.clone());
        let resumed = TinySociety::resume_archive(&whole).unwrap();
        assert_eq!(resumed.world().state(), full.world().state(), "at {start}");
        assert_eq!(resumed.world().events(), full.world().events());
        assert_eq!(
            resumed.world().baseline_state(),
            full.world().baseline_state()
        );
        assert_eq!(resumed.archive().unwrap(), archive);

        // With only the history since, as a World code carries it.
        let mut since = whole.clone();
        since.events = archive.events[checkpoint.events..].to_vec();
        let visited = TinySociety::resume_archive(&since).unwrap();
        assert_eq!(visited.world().state(), full.world().state(), "at {start}");
        assert_eq!(
            visited.world().events(),
            &full.world().events()[checkpoint.events..]
        );
        assert_eq!(
            visited.world().baseline_state(),
            full.world().baseline_state()
        );

        carried = Some(checkpoint);
        seasons += 1;
    }
    assert!(seasons >= 4, "{seasons} seasons");
}
