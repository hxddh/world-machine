mod actions;
mod behaviors;
mod interventions;
mod model;
mod projection;
mod seed;

use std::error::Error;
use world_agent::{
    AgentExecutor, AgentRuntime, AvailableAction, MockAgentRuntime, ScopedPerception,
};
use world_core::{
    ActionRegistry, ActionRequest, BehaviorRegistry, BehaviorRuntime, Event, EventId, World,
};
use world_projection::ProjectionSnapshot;

pub use model::{
    BAKERY, EMMA, EVAN, HARBOR, JONAS, JONAS_BOAT, LEO, MARA, MIA, NOAH, PUB, SCHOOL, SOFIA,
    WEDDING_ORDER,
};

pub const RETAIN_WORKER_COMMAND: &str = "tiny-society.retain-worker";

pub struct TinySociety {
    world: World,
    actions: ActionRegistry,
    behaviors: BehaviorRegistry,
}

#[derive(Clone)]
pub struct TinySocietyBranch {
    world: World,
}

impl TinySocietyBranch {
    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn projection_snapshot(&self) -> ProjectionSnapshot {
        projection::snapshot(&self.world)
    }

    pub fn fork_before_event(&mut self, event_id: EventId) -> Result<(), Box<dyn Error>> {
        let position = self
            .world
            .events()
            .iter()
            .position(|event| event.id == event_id)
            .ok_or_else(|| std::io::Error::other(format!("unknown event {event_id}")))?;
        self.world = self.world.fork_after(position)?;
        Ok(())
    }

    pub fn invoke_projection_command(
        &mut self,
        command_id: &str,
    ) -> Result<Vec<EventId>, Box<dyn Error>> {
        match command_id {
            RETAIN_WORKER_COMMAND => self.continue_with_retention(),
            _ => Err(std::io::Error::other(format!("unknown projection command: {command_id}")).into()),
        }
    }

    pub fn continue_with_retention(&mut self) -> Result<Vec<EventId>, Box<dyn Error>> {
        let order_loss = self
            .world
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "order_lost")
            .map(|event| event.id)
            .ok_or_else(|| std::io::Error::other("branch has no lost order to respond to"))?;
        if self
            .world
            .events()
            .iter()
            .any(|event| event.kind == "worker_retained")
        {
            return Err(std::io::Error::other("Jonas has already been retained").into());
        }

        let actions = build_action_registry()?;
        let retained = self
            .world
            .execute(
                &actions,
                &ActionRequest::new("retain_worker")
                    .actor(MARA)
                    .caused_by(order_loss),
            )?
            .id;

        let next_shift = self.world.world_time() + 5;
        self.world.schedule_at(
            next_shift,
            ActionRequest::new("work_shift")
                .actor(JONAS)
                .arg("worker", JONAS)
                .arg("workplace", BAKERY)
                .arg("wage", 18_i64)
                .caused_by(retained),
        )?;

        let mut events = vec![retained];
        events.extend(self.world.advance_to(&actions, next_shift)?);
        Ok(events)
    }
}

impl TinySociety {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let actions = build_action_registry()?;

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

    pub fn projection_snapshot(&self) -> ProjectionSnapshot {
        projection::snapshot(&self.world)
    }

    pub fn branch(&self) -> TinySocietyBranch {
        TinySocietyBranch {
            world: self.world.clone(),
        }
    }

    pub fn run_story(&mut self) -> Result<(), Box<dyn Error>> {
        let mut runtime = MockAgentRuntime::scripted(["assign_temporary_work"]);
        self.run_story_with_runtime(&mut runtime)
    }

    pub fn run_story_with_runtime<R>(&mut self, runtime: &mut R) -> Result<(), Box<dyn Error>>
    where
        R: AgentRuntime,
    {
        self.advance_checkpoint(5)?;
        self.advance_checkpoint(10)?;

        let loan_request = self
            .world
            .events()
            .iter()
            .find(|event| event.kind == "loan_requested")
            .map(|event| event.id)
            .ok_or_else(|| std::io::Error::other("loan request was not created"))?;

        let options = [
            AvailableAction::new(
                "Offer Jonas temporary work at the bakery",
                ActionRequest::new("assign_temporary_work").actor(MARA),
            ),
            AvailableAction::new(
                "Decline to offer temporary work",
                ActionRequest::new("decline_temporary_work").actor(MARA),
            ),
        ];
        let perception = ScopedPerception::new([MARA, JONAS, LEO, BAKERY]);
        let execution = AgentExecutor::decide_and_execute(
            runtime,
            &perception,
            &mut self.world,
            &self.actions,
            MARA,
            &options,
            &[loan_request],
        )?;

        let temporary_work = self
            .world
            .event(execution.outcome_event)
            .filter(|event| event.kind == "temporary_work_assigned")
            .map(|event| event.id)
            .ok_or_else(|| std::io::Error::other("Mara did not assign temporary work"))?;

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

fn build_action_registry() -> Result<ActionRegistry, Box<dyn Error>> {
    let mut actions = ActionRegistry::new();
    society_basic::register_actions(&mut actions)?;
    world_agent::register_actions(&mut actions)?;
    actions::register(&mut actions)?;
    interventions::register(&mut actions)?;
    Ok(actions)
}

#[cfg(test)]
mod tests;
