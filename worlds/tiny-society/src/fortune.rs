//! What Harbour Town says about how it is doing: money changing hands.
//!
//! The obvious figure is the wrong one. Total household cash *rises* for as
//! long as the workplaces are draining into people's pockets, so by that
//! measure the town looks richest a few weeks before it stops entirely — it
//! peaked at 2,460 on the way down. Wealth that has pooled and stopped moving
//! is not a town doing well; it is a town that has finished.
//!
//! So the figure is flow, not stock: what changed hands over the last ten
//! days. It is high while Jonas is landing catches and the wages and the
//! bread money go round, it falls as each part of that circle breaks, and it
//! is zero in a town that has come to rest — which is the truth about a town
//! where nobody works, nobody buys and nobody is paid.

use world_core::{Event, Value, World};
use world_projection::Fortune;

/// Ten days. Long enough that one quiet morning does not read as collapse,
/// short enough that a town which stopped a month ago does not still look
/// busy.
const WINDOW_TICKS: u64 = 10 * crate::persistence::WORLD_DAY_TICKS;

/// The Events that move money, and the payload key each one carries it in.
///
/// Listed rather than guessed: an Event that happens to carry an `amount` is
/// not necessarily a payment, and a payment whose key is called something
/// else would be silently missed by a rule that went looking for `amount`.
const MONEY_MOVED: [(&str, &str); 6] = [
    ("work_shift_completed", "wage"),
    ("bread_purchased", "amount"),
    ("living_cost_paid", "amount"),
    ("fish_sold", "revenue"),
    ("support_received", "amount"),
    ("support_repaid", "amount"),
];

/// How many readings of the past to offer. Enough to show the shape of a
/// long absence without handing a renderer a point per tick to draw.
const HISTORY_POINTS: usize = 60;

pub(crate) fn of(world: &World) -> Fortune {
    let now = world.world_time();
    Fortune {
        label: "money changing hands".into(),
        value: at(world, now),
        history: history(world, now),
    }
}

/// What was changing hands in the ten days up to `moment`.
fn at(world: &World, moment: u64) -> i64 {
    let since = moment.saturating_sub(WINDOW_TICKS);
    world
        .events()
        .iter()
        .filter(|event| event.world_time > since && event.world_time <= moment)
        .map(moved)
        .sum()
}

/// The same reading, taken at even intervals across the World's whole life.
///
/// Evenly spaced rather than one per event: a stretch where nothing happened
/// is exactly the stretch a person most needs to see, and sampling events
/// would draw it as a gap between two points instead of as the flat line it
/// is.
fn history(world: &World, now: u64) -> Vec<world_projection::FortunePoint> {
    let first = world
        .events()
        .first()
        .map(|event| event.world_time)
        .unwrap_or(0);
    if now <= first {
        return Vec::new();
    }
    let step = ((now - first) / HISTORY_POINTS as u64).max(1);
    (0..=HISTORY_POINTS)
        .map(|index| first + index as u64 * step)
        .take_while(|moment| *moment <= now)
        .map(|moment| world_projection::FortunePoint {
            world_time: moment,
            value: at(world, moment),
        })
        .collect()
}

fn moved(event: &Event) -> i64 {
    MONEY_MOVED
        .iter()
        .find(|(kind, _)| *kind == event.kind)
        .and_then(|(_, key)| match event.payload.get(*key) {
            Some(Value::Integer(amount)) => Some(*amount),
            _ => None,
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use crate::{TinySociety, REOPEN_BAKERY_COMMAND};

    fn fortune(branch: &crate::TinySocietyBranch) -> i64 {
        branch
            .projection_snapshot()
            .fortune
            .expect("Harbour Town says how it is doing")
            .value
    }

    /// The measure tells the truth about a town that has stopped.
    ///
    /// Total household cash does not: it is near its highest when the town is
    /// nearly finished. This one goes to zero, because nothing is moving.
    #[test]
    fn a_town_at_rest_has_nothing_changing_hands() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();

        branch.advance_days(20).unwrap();
        let working = fortune(&branch);
        assert!(
            working > 0,
            "a town with wages and bread money in it is not still"
        );

        branch.advance_days(100).unwrap();
        assert_eq!(fortune(&branch), 0, "a town at rest has nothing moving");
    }

    /// And it recovers when the town is woken, so it can rank two futures
    /// rather than only report a death.
    #[test]
    fn waking_the_town_shows_up_as_money_moving_again() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(120).unwrap();
        assert_eq!(fortune(&branch), 0);

        branch
            .invoke_projection_command(REOPEN_BAKERY_COMMAND)
            .unwrap();
        branch.advance_days(5).unwrap();
        assert!(
            fortune(&branch) > 0,
            "reopening put money back into circulation"
        );
    }
}

#[cfg(test)]
mod is_there_anything_worth_doing {
    use crate::TinySociety;

    /// Run to the moment the World is actually asking something, rather than
    /// to a day number. A hardcoded horizon has to be re-guessed every time
    /// the economy changes, and re-guessing it is indistinguishable from
    /// tuning the test until it agrees.
    fn at_the_decision() -> crate::TinySocietyBranch {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        for _ in 0..400 {
            if !branch.projection_snapshot().commands.is_empty() {
                return branch;
            }
            branch.advance_days(1).unwrap();
        }
        panic!("this World never asked the player anything");
    }

    fn fortune_after(command: Option<&str>, days: u64) -> i64 {
        let mut branch = at_the_decision();
        if let Some(command) = command {
            branch
                .invoke_projection_command(command)
                .unwrap_or_else(|error| panic!("{command} was on the table: {error}"));
        }
        branch.advance_days(days).unwrap();
        branch
            .projection_snapshot()
            .fortune
            .expect("Harbour Town says how it is doing")
            .value
    }

    /// The product's core promise, as an assertion: forking to try the other
    /// choice has to be worth doing.
    ///
    /// A World whose every offered move leaves it worse off than being ignored
    /// has no second future worth comparing, and every screen built on top of
    /// that is rendering a decision that does not matter.
    ///
    /// This nearly went the other way. Measured by the total cash in the
    /// residents' pockets, every choice here loses — reopening costs Mara 120
    /// and the figure drops by 120. But that money did not vanish, it went
    /// back into circulation, and on the measure the World actually reports
    /// both choices beat inaction by a wide margin. A bad measure had this
    /// World condemned.
    #[test]
    fn some_choice_on_the_table_beats_ignoring_the_world() {
        const HORIZON: u64 = 30;
        let nothing = fortune_after(None, HORIZON);

        // Whatever the World is actually offering, not a list of command names
        // kept in step by hand. Naming them meant the test asked about the
        // bakery at a moment when the World was asking about a boat.
        let offers: Vec<String> = at_the_decision()
            .projection_snapshot()
            .commands
            .into_iter()
            .map(|command| command.id)
            .collect();
        assert!(!offers.is_empty());
        let scored: Vec<(String, i64)> = offers
            .iter()
            .map(|id| (id.clone(), fortune_after(Some(id), HORIZON)))
            .collect();

        assert!(
            scored.iter().any(|(_, value)| *value > nothing),
            "nothing on offer beats doing nothing ({nothing}): {scored:?}"
        );
    }
}

#[cfg(test)]
mod the_shape_of_an_absence {
    use crate::TinySociety;

    /// A return says when it began, so the line can shade the stretch.
    ///
    /// This is the half of that feature a screenshot cannot check: the
    /// screenshot harness opens an archive rather than leaving a World and
    /// coming back, so it never has an absence to shade and the shading has
    /// no pixels to prove. The data it needs is checked here instead.
    #[test]
    fn a_return_knows_when_it_began() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(20).unwrap();

        let cursor = branch.visit_cursor();
        let left_at = branch.world().world_time();
        branch.advance_days(30).unwrap();

        let briefing = branch
            .projection_snapshot_since(cursor)
            .briefing
            .expect("a return briefing");
        let since = briefing
            .since_world_time
            .expect("a return knows when the reader left");
        assert!(
            since <= left_at,
            "the absence starts no later than the moment they left: {since} against {left_at}"
        );

        let fortune = branch
            .projection_snapshot_since(cursor)
            .fortune
            .expect("Harbour Town says how it is doing");
        assert!(
            fortune
                .history
                .iter()
                .any(|point| point.world_time >= since),
            "and the line has readings inside the stretch it is being asked to shade"
        );
    }

    /// Looking at a World you never left has nothing to shade.
    #[test]
    fn a_world_you_did_not_leave_has_no_absence() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let branch = society.branch();
        assert_eq!(
            branch
                .projection_snapshot()
                .briefing
                .expect("a briefing")
                .since_world_time,
            None
        );
    }
}
