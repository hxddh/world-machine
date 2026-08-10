mod actions;
mod behaviors;
mod model;
mod seed;

use std::error::Error;
use world_core::{
    ActionRegistry, ActionRequest, BehaviorRegistry, BehaviorRuntime, Event, EventId, World,
};

pub use model::{
    BAKERY, EMMA, EVAN, HARBOR, JONAS, JONAS_BOAT, LEO, MARA, MIA, NOAH, PUB, SCHOOL, SOFIA,
    WEDDING_ORDER,
};

pub struct TinySociety {
    world: World,
    actions: ActionRegistry,
    behaviors: BehaviorRegistry,
}

impl TinySociety {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let mut actions = ActionRegistry::new();
        society_basic::register_actions(&mut actions)?;
        actions::register(&mut actions)?;

        let mut behaviors = BehaviorRegistry::new();
        behaviors::register(&mut behaviors)?;

        let mut simulation = Self {
            world: World::new(seed::seed_world()?),
            actions,
            behaviors,
        };
        simulation.schedule_routines()?;
        simulation
            .world
            .schedule_at(10, ActionRequest::new("storm_arrives"))?;
        Ok(simulation)
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn run_story(&mut self) -> Result<(), Box<dyn Error>> {
        self.advance_checkpoint(5)?;
        self.advance_checkpoint(10)?;

        let temporary_work = self
            .world
            .events()
            .iter()
            .find(|event| event.kind == "temporary_work_assigned")
            .map(|event| event.id)
            .ok_or_else(|| std::io::Error::other("temporary work was not assigned"))?;

        self.world.schedule_at(
            20,
            ActionRequest::new("miss_shift")
                .actor(JONAS)
                .caused_by(temporary_work),
        )?;

        self.advance_checkpoint(15)?;
        self.advance_checkpoint(20)?;
        Ok(())
    }

    pub fn causal_story(&self) -> Vec<&Event> {
        const STORY: [&str; 8] = [
            "storm_started",
            "boat_damaged",
            "income_lost",
            "loan_requested",
            "temporary_work_assigned",
            "shift_missed",
            "order_lost",
            "worker_dismissed",
        ];

        STORY
            .iter()
            .filter_map(|kind| self.world.events().iter().find(|event| event.kind == *kind))
            .collect()
    }

    fn schedule_routines(&mut self) -> Result<(), Box<dyn Error>> {
        for (time, worker, workplace, wage) in [
            (5, MARA, BAKERY, 20_i64),
            (5, EMMA, SCHOOL, 18),
            (5, LEO, PUB, 22),
            (5, JONAS, HARBOR, 25),
            (15, MARA, BAKERY, 20),
            (15, EMMA, SCHOOL, 18),
            (15, LEO, PUB, 22),
        ] {
            self.world.schedule_at(
                time,
                ActionRequest::new("work_shift")
                    .actor(worker)
                    .arg("worker", worker)
                    .arg("workplace", workplace)
                    .arg("wage", wage),
            )?;
        }
        Ok(())
    }

    fn advance_checkpoint(&mut self, world_time: u64) -> Result<Vec<EventId>, Box<dyn Error>> {
        let scheduled = self.world.advance_to(&self.actions, world_time)?;
        let mut all = scheduled.clone();
        for event in scheduled {
            let run = BehaviorRuntime::run_from_event(
                &mut self.world,
                &self.actions,
                &self.behaviors,
                event,
                32,
            )?;
            all.extend(run.generated_events);
        }
        Ok(all)
    }
}

#[cfg(test)]
mod tests;
