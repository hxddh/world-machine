use crate::{
    behaviors, build_action_registry, projection, seed, TinySociety, TinySocietyBranch, BAKERY,
    EMMA, HARBOR, JONAS, LEO, MARA, PUB, SCHOOL,
};
use society_basic::{integer_component, CASH, JOB};
use std::error::Error;
use world_core::{
    ActionRequest, BehaviorRegistry, BehaviorRuntime, EntityId, EventId, Value, World,
};
use world_persistence::{PersistenceError, WorldArchive, WorldPackRef};
use world_projection::ProjectionSnapshot;

pub const TINY_SOCIETY_PACK_ID: &str = "world-machine.tiny-society";
pub const TINY_SOCIETY_PACK_VERSION: &str = "0.1.0";

const WORLD_DAY_TICKS: u64 = 10;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VisitCursor {
    pub event_count: usize,
}

impl VisitCursor {
    pub const fn new(event_count: usize) -> Self {
        Self { event_count }
    }
}

pub fn tiny_society_pack_ref() -> WorldPackRef {
    WorldPackRef::new(TINY_SOCIETY_PACK_ID, TINY_SOCIETY_PACK_VERSION)
}

impl TinySociety {
    pub fn archive(&self) -> Result<WorldArchive, PersistenceError> {
        WorldArchive::capture(tiny_society_pack_ref(), &self.world)
    }

    pub fn archive_json(&self) -> Result<String, PersistenceError> {
        self.archive()?.to_json_pretty()
    }

    pub fn resume_archive(archive: &WorldArchive) -> Result<Self, Box<dyn Error>> {
        let baseline = seed::seed_world()?;
        let world = archive.restore(&tiny_society_pack_ref(), baseline)?;
        restored_simulation(world)
    }

    pub fn resume_json(json: &str) -> Result<Self, Box<dyn Error>> {
        let archive = WorldArchive::from_json(json)?;
        Self::resume_archive(&archive)
    }

    pub fn visit_cursor(&self) -> VisitCursor {
        VisitCursor::new(self.world.events().len())
    }

    pub fn projection_snapshot_since(&self, cursor: VisitCursor) -> ProjectionSnapshot {
        projection::snapshot_since(&self.world, Some(cursor.event_count))
    }
}

impl TinySocietyBranch {
    pub fn archive(&self) -> Result<WorldArchive, PersistenceError> {
        WorldArchive::capture(tiny_society_pack_ref(), &self.world)
    }

    pub fn archive_json(&self) -> Result<String, PersistenceError> {
        self.archive()?.to_json_pretty()
    }

    pub fn visit_cursor(&self) -> VisitCursor {
        VisitCursor::new(self.world.events().len())
    }

    pub fn projection_snapshot_since(&self, cursor: VisitCursor) -> ProjectionSnapshot {
        projection::snapshot_since(&self.world, Some(cursor.event_count))
    }

    pub fn advance_days(&mut self, days: u64) -> Result<Vec<EventId>, Box<dyn Error>> {
        if days == 0 {
            return Ok(Vec::new());
        }

        let actions = build_action_registry()?;
        let mut behavior_registry = BehaviorRegistry::new();
        behaviors::register(&mut behavior_registry)?;
        let mut generated_events = Vec::new();

        for _ in 0..days {
            let next_time = self
                .world
                .world_time()
                .checked_add(WORLD_DAY_TICKS)
                .ok_or_else(|| std::io::Error::other("Tiny Society world time overflow"))?;
            schedule_daily_shifts(&mut self.world, next_time)?;
            let scheduled = self.world.advance_to(&actions, next_time)?;
            generated_events.extend(scheduled.iter().copied());

            for event in scheduled {
                let run = BehaviorRuntime::run_from_event(
                    &mut self.world,
                    &actions,
                    &behavior_registry,
                    event,
                    32,
                )?;
                generated_events.extend(run.generated_events);
            }
        }

        Ok(generated_events)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WageReservation {
    Reserved,
    Shortfall,
}

fn schedule_daily_shifts(world: &mut World, world_time: u64) -> Result<(), Box<dyn Error>> {
    let mut shifts = Vec::new();
    if current_job(world, MARA) == Some("baker") {
        shifts.push((MARA, BAKERY, 20_i64));
    }
    if current_job(world, EMMA) == Some("teacher") {
        shifts.push((EMMA, SCHOOL, 18_i64));
    }
    if current_job(world, LEO) == Some("pub_owner") {
        shifts.push((LEO, PUB, 22_i64));
    }

    match current_job(world, JONAS) {
        Some("fisher") => shifts.push((JONAS, HARBOR, 25_i64)),
        Some("bakery_temp") => shifts.push((JONAS, BAKERY, 18_i64)),
        _ => {}
    }

    let mut budgets = Vec::<(EntityId, i64)>::new();
    let mut shortfall_workplaces = Vec::<EntityId>::new();
    for (worker, workplace, wage) in shifts {
        if shortfall_workplaces.contains(&workplace) {
            continue;
        }
        match reserve_wage(world, &mut budgets, workplace, wage)? {
            WageReservation::Reserved => {
                world.schedule_at(
                    world_time,
                    ActionRequest::new("work_shift")
                        .actor(worker)
                        .arg("worker", worker)
                        .arg("workplace", workplace)
                        .arg("wage", wage),
                )?;
            }
            WageReservation::Shortfall if workplace == BAKERY => {
                shortfall_workplaces.push(workplace);
                world.schedule_at(
                    world_time,
                    ActionRequest::new("record_payroll_shortfall")
                        .actor(worker)
                        .arg("worker", worker)
                        .arg("workplace", workplace)
                        .arg("wage", wage),
                )?;
            }
            WageReservation::Shortfall => {}
        }
    }
    Ok(())
}

fn reserve_wage(
    world: &World,
    budgets: &mut Vec<(EntityId, i64)>,
    workplace: EntityId,
    wage: i64,
) -> Result<WageReservation, Box<dyn Error>> {
    let position = budgets
        .iter()
        .position(|(candidate, _)| *candidate == workplace);
    let budget = match position {
        Some(position) => &mut budgets[position].1,
        None => {
            budgets.push((
                workplace,
                integer_component(world.state(), workplace, CASH)?,
            ));
            &mut budgets.last_mut().expect("budget was just inserted").1
        }
    };

    if *budget < wage {
        return Ok(WageReservation::Shortfall);
    }
    *budget -= wage;
    Ok(WageReservation::Reserved)
}

fn current_job(world: &World, resident: EntityId) -> Option<&str> {
    match world.state().entity(resident)?.component(JOB)? {
        Value::Text(job) => Some(job.as_str()),
        _ => None,
    }
}

fn restored_simulation(world: World) -> Result<TinySociety, Box<dyn Error>> {
    let actions = build_action_registry()?;
    let mut behaviors = BehaviorRegistry::new();
    behaviors::register(&mut behaviors)?;

    Ok(TinySociety {
        world,
        actions,
        behaviors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dismissed_worker_does_not_resume_work_when_days_advance() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        let cursor = branch.visit_cursor();
        let starting_time = branch.world.world_time();

        branch.advance_days(2).unwrap();

        assert_eq!(
            branch.world.world_time(),
            starting_time + 2 * WORLD_DAY_TICKS
        );
        let new_events = &branch.world.events()[cursor.event_count..];
        let routine_shifts = new_events
            .iter()
            .filter(|event| event.kind == "work_shift_completed")
            .collect::<Vec<_>>();
        assert_eq!(routine_shifts.len(), 6);
        assert!(routine_shifts
            .iter()
            .all(|event| event.actor != Some(JONAS)));

        let snapshot = branch.projection_snapshot_since(cursor);
        let briefing = snapshot.briefing.expect("Tiny Society has a briefing");
        assert_eq!(briefing.title, "While you were away");
        assert!(briefing
            .items
            .iter()
            .any(|item| item.title == "The world moved forward"));
    }

    #[test]
    fn retained_worker_keeps_working_on_future_days() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let dismissal = society
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "worker_dismissed")
            .expect("story has a dismissal")
            .id;
        let mut branch = society.branch();
        branch.fork_before_event(dismissal).unwrap();
        branch.continue_with_retention().unwrap();
        let cursor = branch.visit_cursor();
        let starting_time = branch.world.world_time();

        branch.advance_days(1).unwrap();

        assert_eq!(branch.world.world_time(), starting_time + WORLD_DAY_TICKS);
        let new_events = &branch.world.events()[cursor.event_count..];
        assert!(new_events
            .iter()
            .any(|event| { event.kind == "work_shift_completed" && event.actor == Some(JONAS) }));
    }

    #[test]
    fn retaining_jonas_brings_the_bakery_crisis_forward() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let dismissal = society
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "worker_dismissed")
            .expect("story has a dismissal")
            .id;

        let mut dismissed_branch = society.branch();
        let mut retained_branch = society.branch();
        retained_branch.fork_before_event(dismissal).unwrap();
        retained_branch.continue_with_retention().unwrap();

        let dismissed_days = days_until_event(&mut dismissed_branch, "bakery_closed", 30);
        let retained_days = days_until_event(&mut retained_branch, "bakery_closed", 30);

        assert!(retained_days < dismissed_days);
        let closure = retained_branch
            .world
            .events()
            .iter()
            .find(|event| event.kind == "bakery_closed")
            .expect("retained branch eventually closes the bakery");
        assert_eq!(closure.caused_by.len(), 1);
        assert_eq!(
            retained_branch
                .world
                .event(closure.caused_by[0])
                .expect("closure cause remains in history")
                .kind,
            "payroll_shortfall"
        );
        assert_eq!(
            current_job(&retained_branch.world, MARA),
            Some("bakery_closed")
        );
        assert_eq!(
            current_job(&retained_branch.world, JONAS),
            Some("unemployed")
        );
    }

    #[test]
    fn bakery_stays_closed_after_the_payroll_crisis() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        days_until_event(&mut branch, "bakery_closed", 30);
        let cursor = branch.visit_cursor();

        branch.advance_days(2).unwrap();

        assert!(branch.world.events()[cursor.event_count..]
            .iter()
            .filter(|event| event.kind == "work_shift_completed")
            .all(|event| !event.targets.contains(&BAKERY)));
    }

    #[test]
    fn mara_can_reopen_the_bakery_without_automatically_rehiring_jonas() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let dismissal = society
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "worker_dismissed")
            .expect("story has a dismissal")
            .id;
        let mut branch = society.branch();
        branch.fork_before_event(dismissal).unwrap();
        branch.continue_with_retention().unwrap();
        days_until_event(&mut branch, "bakery_closed", 30);

        let closure = branch
            .world
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "bakery_closed")
            .expect("retained branch closes the bakery")
            .id;
        let mara_before = integer_component(branch.world.state(), MARA, CASH).unwrap();
        let bakery_before = integer_component(branch.world.state(), BAKERY, CASH).unwrap();
        assert_eq!(current_job(&branch.world, JONAS), Some("unemployed"));
        assert!(branch
            .projection_snapshot()
            .commands
            .iter()
            .any(|command| command.id == crate::REOPEN_BAKERY_COMMAND));

        let reopened = branch
            .invoke_projection_command(crate::REOPEN_BAKERY_COMMAND)
            .unwrap();

        assert_eq!(reopened.len(), 1);
        let event = branch
            .world
            .event(reopened[0])
            .expect("reopen command creates an event");
        assert_eq!(event.kind, "bakery_reopened");
        assert_eq!(event.caused_by, vec![closure]);
        assert_eq!(
            integer_component(branch.world.state(), MARA, CASH).unwrap(),
            mara_before - crate::BAKERY_REOPEN_INVESTMENT
        );
        assert_eq!(
            integer_component(branch.world.state(), BAKERY, CASH).unwrap(),
            bakery_before + crate::BAKERY_REOPEN_INVESTMENT
        );
        assert_eq!(current_job(&branch.world, MARA), Some("baker"));
        assert_eq!(current_job(&branch.world, JONAS), Some("unemployed"));
        assert!(!branch
            .projection_snapshot()
            .commands
            .iter()
            .any(|command| command.id == crate::REOPEN_BAKERY_COMMAND));

        let archive = branch.archive().unwrap();
        let resumed = TinySociety::resume_archive(&archive).unwrap();
        assert_eq!(current_job(resumed.world(), MARA), Some("baker"));
        assert_eq!(current_job(resumed.world(), JONAS), Some("unemployed"));

        let cursor = branch.visit_cursor();
        branch.advance_days(1).unwrap();
        assert!(branch.world.events()[cursor.event_count..]
            .iter()
            .any(|event| {
                event.kind == "work_shift_completed"
                    && event.actor == Some(MARA)
                    && event.targets.contains(&BAKERY)
            }));
    }

    #[test]
    fn zero_days_is_a_noop() {
        let society = TinySociety::new().unwrap();
        let mut branch = society.branch();
        let starting_time = branch.world.world_time();
        let starting_events = branch.world.events().len();

        let generated = branch.advance_days(0).unwrap();

        assert!(generated.is_empty());
        assert_eq!(branch.world.world_time(), starting_time);
        assert_eq!(branch.world.events().len(), starting_events);
    }

    fn days_until_event(branch: &mut TinySocietyBranch, kind: &str, max_days: u64) -> u64 {
        for day in 1..=max_days {
            branch.advance_days(1).unwrap();
            if branch.world.events().iter().any(|event| event.kind == kind) {
                return day;
            }
        }
        panic!("event {kind} did not occur within {max_days} days");
    }
}
