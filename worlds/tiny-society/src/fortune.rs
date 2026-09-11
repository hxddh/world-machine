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

pub(crate) fn of(world: &World) -> Fortune {
    let now = world.world_time();
    let since = now.saturating_sub(WINDOW_TICKS);
    let value = world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time > since)
        .map(moved)
        .sum();
    Fortune {
        label: "money changing hands".into(),
        value,
    }
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

    fn fortune_after(command: Option<&str>, days: u64) -> i64 {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(55).unwrap();
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

        let offers = [
            crate::REOPEN_BAKERY_COMMAND,
            crate::LEAN_REOPEN_BAKERY_COMMAND,
        ];
        let scored: Vec<(&str, i64)> = offers
            .iter()
            .map(|id| (*id, fortune_after(Some(id), HORIZON)))
            .collect();

        assert!(
            scored.iter().any(|(_, value)| *value > nothing),
            "nothing on offer beats doing nothing ({nothing}): {scored:?}"
        );
    }
}
