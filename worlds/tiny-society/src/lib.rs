mod actions;
mod almanac;
mod behaviors;
mod book;
mod drawings;
mod drift;
mod fishing;
mod handwork;
mod hardship;
mod host;
mod interventions;
mod life;
mod livelihood;
mod local_economy;
mod model;
mod payroll;
mod persistence;
mod projection;
mod reciprocity;
mod recovery;
#[cfg(test)]
mod red_team;
mod seed;
mod social;
mod speech;
mod staffing;
mod story;
mod talk;
mod voices;

use std::error::Error;
use world_agent::{
    AgentExecutor, AgentRuntime, AvailableAction, MockAgentRuntime, ScopedPerception,
};
use world_core::{
    ActionRegistry, ActionRequest, BehaviorRegistry, BehaviorRuntime, Event, EventId, World,
};
use world_projection::ProjectionSnapshot;

#[cfg(test)]
pub(crate) use world_core::Value;

pub use host::{
    tiny_society_registration, tiny_society_registration_with_listener, ListenerFactory,
};
pub use model::{
    BAKERY, EMMA, EVAN, HARBOR, JONAS, JONAS_BOAT, LEO, MARA, MIA, NOAH, PUB, SCHOOL, SOFIA,
    WEDDING_ORDER,
};
pub use persistence::{
    tiny_society_pack_ref, VisitCursor, TINY_SOCIETY_PACK_ID, TINY_SOCIETY_PACK_VERSION,
};

/// Tiny Society in Simplified Chinese: English, a tab, then the
/// translation, a line each.
pub const ZH_HANS: &str = include_str!("../locales/zh-Hans.tsv");

/// The core residents' own lines as templates and what fills them, for
/// showing every one of them in another language.
pub type VoiceTemplates = Vec<(
    &'static [&'static str],
    &'static [(&'static str, &'static [&'static str])],
)>;

pub fn voice_templates() -> VoiceTemplates {
    [JONAS, MARA, LEO, EMMA, MIA, NOAH, EVAN, SOFIA]
        .into_iter()
        .filter_map(voices::voice)
        .map(|voice| (voice.lines, voice.slots))
        .collect()
}

pub const RETAIN_WORKER_COMMAND: &str = "tiny-society.retain-worker";
pub const REOPEN_BAKERY_COMMAND: &str = "tiny-society.reopen-bakery";
pub const LEAN_REOPEN_BAKERY_COMMAND: &str = "tiny-society.reopen-bakery-lean";
pub const REPAIR_BOAT_COMMAND: &str = "tiny-society.repair-sea-finch";
pub const SELL_BOAT_COMMAND: &str = "tiny-society.sell-sea-finch";
pub const TAKE_JONAS_ON_COMMAND: &str = "tiny-society.take-jonas-on";
pub const BAKERY_REOPEN_INVESTMENT: i64 = 120;

pub struct TinySociety {
    world: World,
    actions: ActionRegistry,
    behaviors: BehaviorRegistry,
}

#[derive(Clone)]
pub struct TinySocietyBranch {
    world: World,
}

/// Mark each choice with how it would move the town's gauges, by making it
/// on a copy of the town and reading them again. The town's rules answer the
/// same way twice, so the mark is what will happen, not a guess.
pub(crate) fn with_previews(world: &World, mut snapshot: ProjectionSnapshot) -> ProjectionSnapshot {
    let before = snapshot.gauges.clone();
    for command in &mut snapshot.commands {
        // A deed of the player's own hands is not a choice to weigh.
        if command.hand.is_some() {
            continue;
        }
        let mut copy = TinySocietyBranch {
            world: world.sketch(world_projection::RECENT_EVENTS),
        };
        if copy.invoke_projection_command(&command.id).is_ok() {
            command.moves =
                world_projection::gauge_moves(&before, &projection::gauges(&copy.world));
        }
    }
    snapshot
}

impl TinySocietyBranch {
    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn projection_snapshot(&self) -> ProjectionSnapshot {
        with_previews(&self.world, projection::snapshot(&self.world))
    }

    /// On the player's return, someone who thinks well of them leaves them
    /// something, with a line about the latest of what happened since
    /// `since`.
    pub fn leave_keepsake(&mut self, since: VisitCursor) -> Result<Vec<EventId>, Box<dyn Error>> {
        let start = since.event_count.min(self.world.events().len());
        let why = self.world.events()[start..]
            .iter()
            .rev()
            .filter(|event| {
                lives::is_news(event) || event.kind == "festival_held" || story::is_storylet(event)
            })
            .find_map(|event| story::told(&self.world, event))
            .map(|told| format!("{told}."))
            .unwrap_or_default();
        let actions = build_action_registry()?;
        Ok(
            lives::leave_keepsake(&mut self.world, &actions, &life::cast(), &why)?
                .into_iter()
                .collect(),
        )
    }

    /// Starts the storyteller. A new World opens on the place, and its
    /// first question waits for the player's first deed.
    pub fn begin_story(&mut self) -> Result<Vec<EventId>, Box<dyn Error>> {
        let actions = build_action_registry()?;
        let mut events = story::tick(&mut self.world, &actions, false)?;
        // Someone comes over to say hello.
        events.extend(lives::greet(&mut self.world, &actions, &life::cast())?);
        Ok(events)
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
            REOPEN_BAKERY_COMMAND => self.reopen_bakery(),
            LEAN_REOPEN_BAKERY_COMMAND => recovery::reopen_lean(self),
            REPAIR_BOAT_COMMAND => self.repair_boat_with_leo(),
            SELL_BOAT_COMMAND => self.sell_sea_finch(),
            TAKE_JONAS_ON_COMMAND => self.take_jonas_on(),
            story::WAIT_COMMAND => self.pass_days(1, false),
            _ if story::parse_command(command_id).is_some() => self.answer(command_id),
            _ if life::parse_command(command_id).is_some() => self.answer_life(command_id),
            _ if handwork::parse_command(command_id).is_some() => self.do_deed(command_id),
            _ => Err(
                std::io::Error::other(format!("unknown projection command: {command_id}")).into(),
            ),
        }
    }

    /// Answers one of the storyteller's storylets, and lets the town react.
    fn answer(&mut self, command_id: &str) -> Result<Vec<EventId>, Box<dyn Error>> {
        let (storylet, choice) = story::parse_command(command_id)
            .ok_or_else(|| std::io::Error::other(format!("not a storylet: {command_id}")))?;
        let actions = build_action_registry()?;
        let mut behaviors = BehaviorRegistry::new();
        behaviors::register(&mut behaviors)?;
        let event = self
            .world
            .execute(&actions, &storylets::choose_request(storylet, choice))?
            .id;
        let run =
            BehaviorRuntime::run_from_event(&mut self.world, &actions, &behaviors, event, 32)?;
        let mut events = vec![event];
        events.extend(run.generated_events);
        Ok(events)
    }

    fn answer_life(&mut self, command_id: &str) -> Result<Vec<EventId>, Box<dyn Error>> {
        let (situation, answer) = life::parse_command(command_id)
            .ok_or_else(|| std::io::Error::other(format!("not a situation: {command_id}")))?;
        let actions = build_action_registry()?;
        let event = self
            .world
            .execute(&actions, &lives::answer_request(situation, answer))?
            .id;
        Ok(vec![event])
    }

    /// Says something to someone in the player's own words, and records
    /// what they were heard to mean and what was answered. The day does
    /// not pass.
    pub fn say(
        &mut self,
        who: world_core::EntityId,
        words: &str,
    ) -> Result<Vec<EventId>, Box<dyn Error>> {
        self.say_with(who, words, &mut conversation::OwnEars)
    }

    /// Says something to someone, heard by a listener of the player's
    /// choosing, such as a language model; what it hears is only a
    /// proposal the rules check.
    pub fn say_with(
        &mut self,
        who: world_core::EntityId,
        words: &str,
        listener: &mut dyn conversation::Listener,
    ) -> Result<Vec<EventId>, Box<dyn Error>> {
        let request =
            speech::say(&self.world, who, words, listener).map_err(std::io::Error::other)?;
        let actions = build_action_registry()?;
        let event = self.world.execute(&actions, &request)?.id;
        let mut events = vec![event];
        events.extend(story::after_first_deed(&mut self.world, &actions)?);
        Ok(events)
    }

    fn do_deed(&mut self, command_id: &str) -> Result<Vec<EventId>, Box<dyn Error>> {
        let deed = handwork::parse_command(command_id)
            .ok_or_else(|| std::io::Error::other(format!("not a deed: {command_id}")))?;
        let actions = build_action_registry()?;
        if deed == handwork::UNDO {
            return Ok(vec![
                self.world.execute(&actions, &hands::undo_request())?.id,
            ]);
        }
        let event = self.world.execute(&actions, &hands::do_request(deed))?.id;
        let mut events = vec![event];
        // Someone nearby says what they make of it, and in a new harbour
        // the first question follows.
        events.extend(lives::react_to(
            &mut self.world,
            &actions,
            &life::cast(),
            event,
        )?);
        events.extend(story::after_first_deed(&mut self.world, &actions)?);
        Ok(events)
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

    pub fn reopen_bakery(&mut self) -> Result<Vec<EventId>, Box<dyn Error>> {
        let closure = self
            .world
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "bakery_closed")
            .map(|event| event.id)
            .ok_or_else(|| std::io::Error::other("the bakery has not closed"))?;
        let actions = build_action_registry()?;
        let reopened = self
            .world
            .execute(
                &actions,
                &ActionRequest::new("reopen_bakery")
                    .actor(MARA)
                    .caused_by(closure),
            )?
            .id;
        Ok(vec![reopened])
    }

    pub fn sell_sea_finch(&mut self) -> Result<Vec<EventId>, Box<dyn Error>> {
        let withdrawal = self
            .world
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "backing_withdrawn")
            .map(|event| event.id)
            .ok_or_else(|| {
                std::io::Error::other("Sea Finch is not for sale while her repair is still backed")
            })?;
        let actions = build_action_registry()?;
        let sold = self
            .world
            .execute(
                &actions,
                &ActionRequest::new("sell_sea_finch")
                    .actor(JONAS)
                    .caused_by(withdrawal),
            )?
            .id;
        Ok(vec![sold])
    }

    pub fn take_jonas_on(&mut self) -> Result<Vec<EventId>, Box<dyn Error>> {
        let asked = self
            .world
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "work_sought")
            .map(|event| event.id)
            .ok_or_else(|| std::io::Error::other("Jonas has not asked the bakery for work"))?;
        let actions = build_action_registry()?;
        let taken_on = self
            .world
            .execute(
                &actions,
                &ActionRequest::new("take_jonas_on")
                    .actor(MARA)
                    .caused_by(asked),
            )?
            .id;
        Ok(vec![taken_on])
    }

    pub fn repair_boat_with_leo(&mut self) -> Result<Vec<EventId>, Box<dyn Error>> {
        let support = self
            .world
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "support_received")
            .map(|event| event.id)
            .ok_or_else(|| std::io::Error::other("Leo has not supported Jonas yet"))?;
        let actions = build_action_registry()?;
        let repaired = self
            .world
            .execute(
                &actions,
                &ActionRequest::new("repair_jonas_boat")
                    .actor(LEO)
                    .caused_by(support),
            )?
            .id;
        Ok(vec![repaired])
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
        with_previews(&self.world, projection::snapshot(&self.world))
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
    fishing::register_actions(&mut actions)?;
    drift::register_actions(&mut actions)?;
    hardship::register_actions(&mut actions)?;
    livelihood::register_actions(&mut actions)?;
    interventions::register(&mut actions)?;
    local_economy::register_actions(&mut actions)?;
    payroll::register_actions(&mut actions)?;
    reciprocity::register_actions(&mut actions)?;
    recovery::register_actions(&mut actions)?;
    social::register_actions(&mut actions)?;
    staffing::register_actions(&mut actions)?;
    story::register_actions(&mut actions)?;
    Ok(actions)
}

#[cfg(test)]
mod density;
#[cfg(test)]
mod friendship;
#[cfg(test)]
mod tests;
