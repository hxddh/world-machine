#![forbid(unsafe_code)]

mod almanac;
mod almanac_page;
#[cfg(test)]
mod apart_tests;
mod arrival;
#[cfg(test)]
mod art_tests;
mod book;
#[cfg(test)]
mod density;
mod drawings;
mod eras;
#[cfg(test)]
mod exploit_tests;
#[cfg(test)]
mod favours_tests;
#[cfg(test)]
mod first_minutes;
mod firsts;
mod handwork;
mod kin;
mod legends;
#[cfg(test)]
mod lexicon_tests;
mod life;
mod moments;
pub mod narrator;
mod places;
#[cfg(test)]
mod players;
mod plots;
#[cfg(test)]
mod plots_tests;
mod projection;
#[cfg(test)]
mod red_team;
#[cfg(test)]
mod seams_tests;
mod speech;
#[cfg(test)]
mod stories_tests;
mod story;
mod talk;
mod town;
mod voices;
#[cfg(test)]
mod words_tests;
mod works;
mod years;

use std::error::Error;
use std::sync::Arc;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, EventDraft, EventId,
    StateChange, Value, World, WorldState, WorldStateError,
};
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};
use world_persistence::{PersistenceError, WorldArchive, WorldPackRef};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub const POCKET_UNIVERSE_PACK_ID: &str = "world-machine.pocket-universe";
pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.30.0";

pub const SEED_MARS_COLONY_COMMAND: &str = "pocket-universe.seed-mars-colony";
pub const SEED_1980S_TOWN_COMMAND: &str = "pocket-universe.seed-1980s-town";
pub const SEED_PENGUIN_CIVILIZATION_COMMAND: &str = "pocket-universe.seed-penguin-civilization";
/// Letting one period pass.
pub const NUDGE_COMMAND: &str = "pocket-universe.nudge";

pub use life::people_names;

/// The residents' own lines as templates and what fills them, for showing
/// every one of them in another language.
pub type VoiceTemplates = Vec<(
    &'static [&'static str],
    &'static [(&'static str, &'static [&'static str])],
)>;

/// Every place's two residents' templates: Mars, Maple Street, Icebridge.
pub fn voice_templates() -> VoiceTemplates {
    voices::ALL
        .iter()
        .map(|voice| (voice.lines, voice.slots))
        .collect()
}

pub(crate) const UNIVERSE: EntityId = EntityId::new(1);
pub(crate) const SLOT_A: EntityId = EntityId::new(10);
pub(crate) const SLOT_B: EntityId = EntityId::new(11);
pub(crate) const SLOT_C: EntityId = EntityId::new(12);
pub(crate) const SLOT_D: EntityId = EntityId::new(13);
pub(crate) const SLOT_E: EntityId = EntityId::new(14);
/// How the pair get on: the trust and tension the storyteller's questions
/// move.
pub(crate) const RELATIONSHIP: EntityId = EntityId::new(15);

pub(crate) const SEED: &str = "seed";
pub(crate) const LAST_CHANGE: &str = "last_change";
pub(crate) const RELATIONSHIP_TRUST: &str = "trust";
pub(crate) const RELATIONSHIP_TENSION: &str = "tension";
const UNSEEDED: &str = "unseeded";
pub(crate) const BACKGROUND_PERIOD: u64 = 10;

pub fn pocket_universe_pack_ref() -> WorldPackRef {
    WorldPackRef::new(POCKET_UNIVERSE_PACK_ID, POCKET_UNIVERSE_PACK_VERSION)
}

pub struct PocketUniverse {
    world: World,
    actions: ActionRegistry,
    /// Who says what happened in this World's own words. A World without one
    /// reads from the table.
    narrator: Box<dyn narrator::Narrator>,
}

impl PocketUniverse {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            world: World::new(baseline()?),
            actions: build_action_registry()?,
            narrator: Box::new(narrator::NoNarrator),
        })
    }

    pub fn resume_archive(archive: &WorldArchive) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            world: archive.restore(&pocket_universe_pack_ref(), baseline()?)?,
            actions: build_action_registry()?,
            narrator: Box::new(narrator::NoNarrator),
        })
    }

    /// Opens an archive this World may keep: its history becomes the
    /// World's own rather than a copy of it, so the parsed archive is not
    /// held alongside the World.
    pub fn resume_owned_archive(archive: WorldArchive) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            world: archive.into_world(&pocket_universe_pack_ref(), baseline()?)?,
            actions: build_action_registry()?,
            narrator: Box::new(narrator::NoNarrator),
        })
    }

    /// Ask this World's narrator for the lines about to be read, if it has one.
    /// `since` is where the observer last looked, so the narrator is handed
    /// exactly what the return digest is about to show.
    fn narrate_return(&mut self, since: usize) -> usize {
        narrator::narrate_return(
            &mut self.world,
            &self.actions,
            self.narrator.as_mut(),
            since,
        )
    }

    /// Give this World a voice. Without one it reads from the table.
    pub fn set_narrator(&mut self, narrator: Box<dyn narrator::Narrator>) {
        self.narrator = narrator;
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    /// A story the place tells about itself: a legend, a moment or a
    /// year's almanac. Read from the history alone; changes nothing.
    pub fn story(
        &self,
        request: world_projection::StoryRequest,
    ) -> Option<world_projection::StoryPage> {
        use world_projection::{StoryPage, StoryRequest};
        match request {
            StoryRequest::Legend(subject) => {
                legends::legend(&self.world, subject).map(StoryPage::Legend)
            }
            StoryRequest::Moment(id) => moments::moment(&self.world, &id).map(StoryPage::Moment),
            StoryRequest::Almanac(year) => {
                let moments = moments::moments(&self.world);
                almanac_page::almanac(&self.world, year, &moments).map(StoryPage::Almanac)
            }
        }
    }

    pub fn projection_snapshot(&self) -> ProjectionSnapshot {
        self.shown(None)
    }

    pub fn projection_snapshot_since(
        &self,
        since_event_count: Option<usize>,
    ) -> ProjectionSnapshot {
        self.shown(since_event_count)
    }

    /// The place as the player is shown it (with what changed since the
    /// `since`th event, if asked), kept with the World for as long as it
    /// stands as it does: looking again at a place where nothing has
    /// changed costs a copy, as it does of a Pack in a process of its own.
    fn shown(&self, since: Option<usize>) -> ProjectionSnapshot {
        let standing = self.world.standing();
        let kept = self.world.derived::<Shown>(|kept| match kept {
            Some(kept) if kept.standing == standing && kept.since == since => kept,
            kept => {
                if let Some(stale) = kept {
                    let_go_later(stale);
                }
                Arc::new(Shown {
                    standing,
                    since,
                    snapshot: self.show(since),
                })
            }
        });
        kept.snapshot.clone()
    }

    /// The place's snapshot, with what each choice would do worked out on
    /// a thread of its own meanwhile: trying the choices on a copy reads
    /// the World and changes nothing, so the snapshot is the same
    /// whichever finishes first.
    fn show(&self, since: Option<usize>) -> ProjectionSnapshot {
        let world = &self.world;
        if seed_id(world) == UNSEEDED {
            return self.with_beginnings(projection::snapshot_since(world, since));
        }
        std::thread::scope(|scope| {
            let trying = std::thread::Builder::new()
                .name("pocket-universe-previews".into())
                .spawn_scoped(scope, || {
                    let commands = projection::commands_on_offer(world);
                    previews(world, &commands, &projection::gauges(world))
                });
            let snapshot = projection::snapshot_since(world, since);
            let previews = match trying {
                Ok(trying) => trying
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic)),
                // No thread to be had: worked out here instead.
                Err(_) => previews(world, &snapshot.commands, &snapshot.gauges),
            };
            self.with_previews(snapshot, previews)
        })
    }

    /// A copy of this World to try something on, with no narrator.
    fn sketch(&self) -> Option<PocketUniverse> {
        Some(PocketUniverse {
            world: self.world.sketch(world_projection::RECENT_EVENTS),
            actions: build_action_registry().ok()?,
            narrator: Box::new(narrator::NoNarrator),
        })
    }

    /// Mark each choice with how it would move the gauges (`previews`, by
    /// command, worked out by playing each on a copy of this World and
    /// reading them again: the same rules, so the same result).
    fn with_previews(
        &self,
        mut snapshot: ProjectionSnapshot,
        previews: Vec<(String, Option<Vec<world_projection::GaugeMove>>)>,
    ) -> ProjectionSnapshot {
        let cast = life::cast(self.world.state());
        world_projection::with_guests(&mut snapshot, &guests_staying(&self.world, &cast));
        if snapshot.gauges.is_empty() {
            return snapshot;
        }
        let choices = || {
            snapshot
                .commands
                .iter()
                .filter(|command| command.hand.is_none())
        };
        // Worked out for these very choices, or else again here.
        let fresh;
        let previews = if previews.len() == choices().count()
            && previews
                .iter()
                .zip(choices())
                .all(|((id, _), command)| *id == command.id)
        {
            &previews
        } else {
            fresh = crate::previews(&self.world, &snapshot.commands, &snapshot.gauges);
            &fresh
        };
        for ((_, moves), command) in previews.iter().zip(
            snapshot
                .commands
                .iter_mut()
                .filter(|command| command.hand.is_none()),
        ) {
            if let Some(moves) = moves {
                command.moves = moves.clone();
            }
        }
        snapshot
    }

    /// Before a place is chosen, each place to begin is shown as it would
    /// first stand, by beginning it on a copy: its people and buildings,
    /// drawn the way its window will draw them.
    fn with_beginnings(&self, mut snapshot: ProjectionSnapshot) -> ProjectionSnapshot {
        for command in &mut snapshot.commands {
            if command.scenery.is_none() {
                continue;
            }
            let Some(mut copy) = self.sketch() else {
                continue;
            };
            if copy.invoke_projection_command(&command.id).is_ok() {
                let begun = projection::snapshot(&copy.world);
                command.preview = Some(Box::new(world_projection::Preview {
                    canvas: begun.canvas,
                    drawings: begun.drawings,
                }));
            }
        }
        snapshot
    }

    /// One period passing on `candidate`: people live their day, the
    /// calendar turns and the storyteller moves on. `away` is whether the
    /// player is watching.
    fn pass_period_on(
        &self,
        candidate: &mut World,
        away: bool,
    ) -> Result<Option<EventId>, Box<dyn Error>> {
        let target = candidate
            .world_time()
            .checked_add(BACKGROUND_PERIOD)
            .ok_or_else(|| std::io::Error::other("Pocket Universe time overflow"))?;
        // The period begins with an Event of its own, and everything its
        // rules record follows from it (invariant 5).
        let day = calendar::begin_day(candidate, &self.actions)?;
        candidate.following(day, |candidate| {
            candidate.advance_to(&self.actions, target)?;
            Ok(story::tick(candidate, &self.actions, away)?.last().copied())
        })
    }

    /// Says something to someone in the player's own words, and records
    /// what they were heard to mean and what was answered. No time passes.
    pub fn say(&mut self, who: EntityId, words: &str) -> Result<EventId, Box<dyn Error>> {
        self.say_with(who, words, &mut conversation::OwnEars)
    }

    /// Says something to someone, heard by a listener of the player's
    /// choosing, such as a language model; what it hears is only a
    /// proposal the rules check.
    pub fn say_with(
        &mut self,
        who: EntityId,
        words: &str,
        listener: &mut dyn conversation::Listener,
    ) -> Result<EventId, Box<dyn Error>> {
        let request =
            speech::say(&self.world, who, words, listener).map_err(std::io::Error::other)?;
        self.said(request)
    }

    /// Says the open favour's quick reply to someone, chosen with a click:
    /// done as the favour offers, never heard, whatever language the
    /// player's button showed it in.
    pub fn say_offered(&mut self, who: EntityId, words: &str) -> Result<EventId, Box<dyn Error>> {
        let request =
            speech::say_offered(&self.world, who, words).map_err(std::io::Error::other)?;
        self.said(request)
    }

    /// Records what the player said, and what it did.
    fn said(&mut self, request: world_core::ActionRequest) -> Result<EventId, Box<dyn Error>> {
        let event = self.world.execute(&self.actions, &request)?.id;
        let actions = &self.actions;
        self.world
            .following(event, |world| -> Result<(), Box<dyn Error>> {
                speech::favour_done(world, actions, event)?;
                story::after_first_deed(world, actions)?;
                Ok(())
            })?;
        Ok(event)
    }

    /// A design or a name the player gave, as a typed intent (protocol
    /// v8): `what` is `design` or `name`, `target` the command the place
    /// offered, `argument` the design's text or the name.
    pub fn mark_typed(
        &mut self,
        what: &str,
        target: &str,
        argument: &str,
    ) -> Result<EventId, Box<dyn Error>> {
        if seed_id(&self.world) == UNSEEDED {
            return Err(std::io::Error::other("choose where this World begins first").into());
        }
        let request = plots::typed_request(what, target, argument)
            .ok_or_else(|| std::io::Error::other(format!("not a {what}: {target}")))?;
        Ok(self.world.execute(&self.actions, &request)?.id)
    }

    pub fn invoke_projection_command(
        &mut self,
        command_id: &str,
    ) -> Result<EventId, Box<dyn Error>> {
        if command_id == NUDGE_COMMAND {
            if seed_id(&self.world) == UNSEEDED {
                return Err(std::io::Error::other("choose where this World begins first").into());
            }
            let since = self.world.events().len();
            // The period passes on the World itself; if any part of it
            // fails, the World goes back to where it stood.
            let checkpoint = self.world.checkpoint();
            let mut world = std::mem::replace(&mut self.world, World::new(WorldState::default()));
            let outcome = self.pass_period_on(&mut world, false);
            if outcome.is_err() {
                world.rollback(checkpoint);
            }
            self.world = world;
            let returned = outcome?;
            self.narrate_return(since);
            return returned
                .or_else(|| self.world.events().last().map(|event| event.id))
                .ok_or_else(|| std::io::Error::other("nothing happened").into());
        }

        // "Report this line": an Event caused by the exchange it reports.
        if let Some(spoken) = conversation::report::parse_command(command_id) {
            return conversation::report::report(&mut self.world, &self.actions, spoken)
                .map_err(|error| std::io::Error::other(error).into());
        }

        if let Some((storylet, choice)) = story::parse_command(command_id) {
            // The answer follows the question it answers.
            let request = storylets::choose_request_in(&self.world, storylet, choice);
            let chosen = self.world.execute(&self.actions, &request)?.id;
            // The place sees the player here: it takes up the works they
            // let wait.
            let actions = &self.actions;
            self.world.following(chosen, |world| {
                storylets::mark_now(world, actions, story::HANDS_SEEN, story::HANDS_SEEN_EVERY)?;
                // Trust is earned over weeks: no answer takes it further.
                story::hold_trust(world, actions)
            })?;
            return Ok(chosen);
        }

        if let Some(deed) = handwork::parse_command(command_id) {
            if deed == handwork::UNDO {
                return Ok(self
                    .world
                    .execute(&self.actions, &hands::undo_request())?
                    .id);
            }
            let event = self
                .world
                .execute(&self.actions, &hands::do_request(deed))?
                .id;
            // Everything the deed brings about follows from it.
            let actions = &self.actions;
            self.world
                .following(event, |world| -> Result<(), Box<dyn Error>> {
                    // The place sees the player making something: it takes
                    // up the works they let wait.
                    storylets::mark_now(
                        world,
                        actions,
                        story::HANDS_SEEN,
                        story::HANDS_SEEN_EVERY,
                    )?;
                    story::lend_to_work(world, actions)?;
                    // Someone nearby says what they make of it, and in a
                    // new World the first question follows. Moving things
                    // about is only remarked on now and then: nobody
                    // comments on every shuffle.
                    if !moved_lately(world, event) {
                        let cast = life::cast(world.state());
                        lives::react_to(world, actions, &cast, event)?;
                    }
                    story::after_first_deed(world, actions)?;
                    Ok(())
                })?;
            return Ok(event);
        }

        if plots::parse_command(command_id).is_some() {
            if seed_id(&self.world) == UNSEEDED {
                return Err(std::io::Error::other("choose where this World begins first").into());
            }
            let request = plots::request(command_id)
                .ok_or_else(|| std::io::Error::other(format!("not a mark: {command_id}")))?;
            return Ok(self.world.execute(&self.actions, &request)?.id);
        }

        if let Some(idea) = command_id.strip_prefix(life::SUGGEST_COMMAND) {
            let request = lives::suggestion_request(idea, life::fair(&self.world));
            return Ok(self.world.execute(&self.actions, &request)?.id);
        }

        if let Some((who, spot)) = kin::parse_command(command_id) {
            let mut request = lives::memorial_request(who, true, spot);
            if let Some(died) = self
                .world
                .events_of_kind(&["died"])
                .into_iter()
                .rev()
                .find(|event| event.payload.get("who") == Some(&Value::Entity(who)))
            {
                request = request.caused_by(died.id);
            }
            return Ok(self.world.execute(&self.actions, &request)?.id);
        }

        if let Some((situation, answer)) = life::parse_command(command_id) {
            return Ok(self
                .world
                .execute(
                    &self.actions,
                    &lives::answer_request_in(&self.world, situation, answer),
                )?
                .id);
        }

        let action = match command_id {
            SEED_MARS_COLONY_COMMAND => "seed_mars_colony",
            SEED_1980S_TOWN_COMMAND => "seed_1980s_town",
            SEED_PENGUIN_CIVILIZATION_COMMAND => "seed_penguin_civilization",
            _ => {
                return Err(std::io::Error::other(format!(
                    "unknown projection command: {command_id}"
                ))
                .into())
            }
        };
        let event = self
            .world
            .execute(&self.actions, &ActionRequest::new(action).actor(UNIVERSE))?
            .id;
        // A World that has just begun: someone comes over to say hello,
        // and the first question follows, all of it from its beginning.
        let actions = &self.actions;
        self.world
            .following(event, |world| -> Result<(), Box<dyn Error>> {
                story::tick(world, actions, false)?;
                let cast = life::cast(world.state());
                lives::greet_saying(world, actions, &cast, arrival::hello(world.state()))?;
                story::first_question(world, actions)?;
                Ok(())
            })?;
        Ok(event)
    }

    /// `periods` passing on `candidate` while nobody watches.
    fn advance_on(&self, candidate: &mut World, periods: u64) -> Result<(), Box<dyn Error>> {
        for _ in 0..periods {
            if seed_id(candidate) == UNSEEDED {
                let target = candidate
                    .world_time()
                    .checked_add(BACKGROUND_PERIOD)
                    .ok_or_else(|| std::io::Error::other("Pocket Universe time overflow"))?;
                candidate.advance_to(&self.actions, target)?;
                continue;
            }
            self.pass_period_on(candidate, true)?;
        }
        Ok(())
    }

    pub fn advance_periods(&mut self, periods: u64) -> Result<(), Box<dyn Error>> {
        // Where the observer last looked. Everything after it is what they are
        // about to read, and so what is worth putting into words.
        let since = self.world.events().len();
        // The periods pass on the World itself; if any part of them fails,
        // the World goes back to where it stood.
        let checkpoint = self.world.checkpoint();
        let mut world = std::mem::replace(&mut self.world, World::new(WorldState::default()));
        let outcome = self.advance_on(&mut world, periods);
        if outcome.is_err() {
            world.rollback(checkpoint);
        }
        self.world = world;
        outcome?;
        self.leave_keepsake(since)?;
        // Once, for the lines an observer is about to read — not once per
        // period.
        self.narrate_return(since);
        Ok(())
    }

    /// On the player's return, someone who thinks well of them leaves them
    /// something, with a line about the latest of what happened since the
    /// `since`th event.
    fn leave_keepsake(&mut self, since: usize) -> Result<(), Box<dyn Error>> {
        if seed_id(&self.world) == UNSEEDED || self.world.events().len() <= since {
            return Ok(());
        }
        // The film of the return tells its beats; the note tells something
        // it has not, so nothing is told twice in a row.
        let filmed = projection::snapshot_since(&self.world, Some(since))
            .briefing
            .map(|briefing| briefing.items)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| match item.selection {
                Some(world_projection::SelectionId::Event(id)) => Some(id),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>();
        let told = self.world.events()[since..]
            .iter()
            .rev()
            .filter(|event| !filmed.contains(&event.id))
            .filter(|event| {
                lives::is_news(event)
                    || event.kind == "festival_held"
                    || event.payload.contains_key("storylet")
            })
            .find_map(|event| story::told(&self.world, event).map(|told| (event.id, told)));
        // The keepsake follows what its note tells of, or else the last
        // period that passed while the player was away.
        let cause = told.as_ref().map(|(id, _)| *id).or_else(|| {
            self.world.events()[since..]
                .iter()
                .rev()
                .find(|event| calendar::is_day_begun(event))
                .map(|event| event.id)
        });
        let why = told.map(|(_, told)| format!("{told}.")).unwrap_or_default();
        let cast = life::cast(self.world.state());
        let actions = &self.actions;
        match cause {
            Some(cause) => self.world.following(cause, |world| {
                lives::leave_keepsake(world, actions, &cast, &why)
            })?,
            None => lives::leave_keepsake(&mut self.world, actions, &cast, &why)?,
        };
        Ok(())
    }

    /// Someone from another of the player's Worlds visits: a letter for
    /// the letter box and, if the week has room, something to keep.
    pub fn host(&mut self, guest: &world_projection::Guest) -> Result<EventId, Box<dyn Error>> {
        if seed_id(&self.world) == UNSEEDED {
            return Err(std::io::Error::other("nobody lives here yet to welcome a guest").into());
        }
        let cast = life::cast(self.world.state());
        let (look, drawing) = (guest.look_code(), guest.drawing_code());
        Ok(lives::host_guest_with(
            &mut self.world,
            &self.actions,
            &cast,
            &guest_words(guest, &look, &drawing),
        )?)
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

    pub fn archive(&self) -> Result<WorldArchive, PersistenceError> {
        WorldArchive::capture(pocket_universe_pack_ref(), &self.world)
    }
}

/// A snapshot kept by [`PocketUniverse::shown`], with where the World
/// stood.
struct Shown {
    standing: world_core::Standing,
    since: Option<usize>,
    snapshot: ProjectionSnapshot,
}

/// Lets go of a snapshot the World no longer stands as away from the turn:
/// a three-year place's takes milliseconds to free, which a turn would
/// otherwise wait for. It is freed once nothing else holds it.
fn let_go_later(stale: Arc<Shown>) {
    use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;
    static LET_GO: OnceLock<Option<Mutex<Sender<Arc<Shown>>>>> = OnceLock::new();
    let sender = LET_GO.get_or_init(|| {
        let (sender, receiver) = channel::<Arc<Shown>>();
        std::thread::Builder::new()
            .name("pocket-universe-let-go".into())
            .spawn(move || {
                let mut held = Vec::new();
                loop {
                    // Asleep while it holds nothing; while it holds
                    // something still in use, it looks again now and then.
                    let next = if held.is_empty() {
                        receiver.recv().map_err(|_| RecvTimeoutError::Disconnected)
                    } else {
                        receiver.recv_timeout(Duration::from_millis(50))
                    };
                    match next {
                        Ok(stale) => held.push(stale),
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                    held.retain(|stale| Arc::strong_count(stale) > 1);
                }
            })
            .ok()
            .map(|_| Mutex::new(sender))
    });
    if let Some(sender) = sender {
        if let Ok(sender) = sender.lock() {
            // With nowhere to send it, it is let go of here, as before.
            let _ = sender.send(stale);
        }
    }
}

/// How each choice on offer (by its id) would move the place's gauges, if
/// it can be made. The choices are tried in two halves at once, each on a
/// copy of its own: trying reads the World and changes nothing.
fn previews(
    world: &World,
    commands: &[world_projection::ProjectionCommand],
    before: &[world_projection::Gauge],
) -> Vec<(String, Option<Vec<world_projection::GaugeMove>>)> {
    // A deed of the player's own hands is not a choice to weigh.
    let choices = commands
        .iter()
        .filter(|command| command.hand.is_none())
        .collect::<Vec<_>>();
    if choices.len() < 4 {
        return tried(world, &choices, before);
    }
    let (first, second) = choices.split_at(choices.len() / 2);
    std::thread::scope(|scope| {
        let trying = std::thread::Builder::new()
            .name("pocket-universe-previews-2".into())
            .spawn_scoped(scope, || tried(world, second, before));
        let mut previews = tried(world, first, before);
        match trying {
            Ok(trying) => previews.extend(
                trying
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic)),
            ),
            Err(_) => previews.extend(tried(world, second, before)),
        }
        previews
    })
}

/// Each of `choices` tried on one copy of the World and then taken back,
/// since going back to a checkpoint leaves the copy exactly as it was.
fn tried(
    world: &World,
    choices: &[&world_projection::ProjectionCommand],
    before: &[world_projection::Gauge],
) -> Vec<(String, Option<Vec<world_projection::GaugeMove>>)> {
    let Ok(actions) = build_action_registry() else {
        return Vec::new();
    };
    let mut copy = PocketUniverse {
        world: world.sketch(world_projection::RECENT_EVENTS),
        actions,
        narrator: Box::new(narrator::NoNarrator),
    };
    let mut previews = Vec::new();
    for command in choices {
        let checkpoint = copy.world.checkpoint();
        let moves = copy
            .invoke_projection_command(&command.id)
            .is_ok()
            .then(|| world_projection::gauge_moves(before, &projection::gauges(&copy.world)));
        previews.push((command.id.clone(), moves));
        copy.world.rollback(checkpoint);
    }
    previews
}

struct PocketUniverseSession {
    world: PocketUniverse,
    return_since_event_count: Option<usize>,
    /// Hears what the player says to people.
    listener: Box<dyn conversation::Listener>,
}

impl PocketUniverseSession {
    fn fresh(
        voice: Box<dyn narrator::Narrator>,
        listener: Box<dyn conversation::Listener>,
    ) -> Result<Box<dyn WorldSession>, HostError> {
        let mut world = PocketUniverse::new().map_err(HostError::session)?;
        world.set_narrator(voice);
        Ok(Box::new(Self {
            world,
            return_since_event_count: None,
            listener,
        }))
    }

    fn open_archive(
        archive: &WorldArchive,
        voice: Box<dyn narrator::Narrator>,
        listener: Box<dyn conversation::Listener>,
    ) -> Result<Box<dyn WorldSession>, HostError> {
        let mut world = PocketUniverse::resume_archive(archive).map_err(HostError::session)?;
        world.set_narrator(voice);
        Ok(Box::new(Self {
            world,
            return_since_event_count: None,
            listener,
        }))
    }

    fn open_owned_archive(
        archive: WorldArchive,
        voice: Box<dyn narrator::Narrator>,
        listener: Box<dyn conversation::Listener>,
    ) -> Result<Box<dyn WorldSession>, HostError> {
        let mut world =
            PocketUniverse::resume_owned_archive(archive).map_err(HostError::session)?;
        world.set_narrator(voice);
        Ok(Box::new(Self {
            world,
            return_since_event_count: None,
            listener,
        }))
    }
}

impl WorldSession for PocketUniverseSession {
    fn pack(&self) -> WorldPackRef {
        pocket_universe_pack_ref()
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        self.world
            .projection_snapshot_since(self.return_since_event_count)
    }

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
        match intent {
            ProjectionIntent::ForkBeforeEvent(event) => self
                .world
                .fork_before_event(event)
                .map_err(HostError::session)?,
            ProjectionIntent::InvokeCommand(command) => {
                self.world
                    .invoke_projection_command(&command)
                    .map_err(HostError::session)?;
            }
            ProjectionIntent::Say { to, words, ears } => {
                let world_projection::SelectionId::Entity(who) = to else {
                    return Err(HostError::session(std::io::Error::other(
                        "only someone can be spoken to",
                    )));
                };
                if ears == world_projection::Ears::Offered {
                    self.world
                        .say_offered(who, &words)
                        .map_err(HostError::session)?;
                    self.return_since_event_count = None;
                    return Ok(self.snapshot());
                }
                let mut answered;
                let mut own = conversation::OwnEars;
                let listener: &mut dyn conversation::Listener = match ears {
                    world_projection::Ears::World | world_projection::Ears::Offered => {
                        self.listener.as_mut()
                    }
                    world_projection::Ears::Model(response) => {
                        answered = conversation::Answered::new(response);
                        &mut answered
                    }
                    world_projection::Ears::Judged { response, judged } => {
                        answered = conversation::Answered::judged(response, &judged);
                        &mut answered
                    }
                    world_projection::Ears::Own => &mut own,
                };
                self.world
                    .say_with(who, &words, listener)
                    .map_err(HostError::session)?;
            }
            ProjectionIntent::Host(guest) => {
                self.world.host(&guest).map_err(HostError::session)?;
            }
            ProjectionIntent::Design { target, pattern } => {
                self.world
                    .mark_typed("design", &target, &pattern.text())
                    .map_err(HostError::session)?;
            }
            ProjectionIntent::Name { target, name } => {
                self.world
                    .mark_typed("name", &target, &name)
                    .map_err(HostError::session)?;
            }
        }
        self.return_since_event_count = None;
        Ok(self.snapshot())
    }

    fn story(
        &self,
        request: world_projection::StoryRequest,
    ) -> Result<Option<world_projection::StoryPage>, HostError> {
        Ok(self.world.story(request))
    }

    fn hearing(
        &self,
        to: world_projection::SelectionId,
        words: &str,
    ) -> Result<Option<String>, HostError> {
        Ok(match to {
            world_projection::SelectionId::Entity(who) => {
                speech::prompt(self.world.world(), who, words)
            }
            _ => None,
        })
    }

    fn voice_hearing(
        &self,
        to: world_projection::SelectionId,
        words: &str,
    ) -> Result<Option<world_projection::VoiceHearing>, HostError> {
        Ok(match to {
            world_projection::SelectionId::Entity(who) => {
                speech::voice_hearing(self.world.world(), who, words)
            }
            _ => None,
        })
    }

    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
        let before = self.world.world().events().len();
        self.world
            .advance_periods(periods)
            .map_err(HostError::session)?;
        self.return_since_event_count =
            (self.world.world().events().len() > before).then_some(before);
        Ok(self.snapshot())
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        self.world.archive().map(Some).map_err(HostError::session)
    }

    /// Where the World stands, and where a return's digest starts, for a
    /// host to come back to if saving what happens next fails.
    fn checkpoint(&mut self) -> Result<Option<world_host::SessionCheckpoint>, HostError> {
        Ok(Some(world_host::SessionCheckpoint::new((
            self.world.world.checkpoint(),
            self.return_since_event_count,
        ))))
    }

    fn rollback(&mut self, checkpoint: world_host::SessionCheckpoint) -> Result<(), HostError> {
        let (world, return_since_event_count) =
            checkpoint.into_inner::<(world_core::Checkpoint, Option<usize>)>()?;
        self.world.world.rollback(world);
        self.return_since_event_count = return_since_event_count;
        Ok(())
    }

    fn archive_since(&self, from: usize) -> Result<Option<WorldArchive>, HostError> {
        WorldArchive::capture_since(pocket_universe_pack_ref(), self.world.world(), from)
            .map(Some)
            .map_err(HostError::session)
    }
}

/// How many periods pass before someone remarks on the player moving
/// something again.
const MOVES_REMARKED_EVERY: u64 = 30;

/// Whether `deed` moved something, and something else was moved not long
/// before it.
fn moved_lately(world: &World, deed: EventId) -> bool {
    let Some(event) = world.event(deed) else {
        return false;
    };
    if event.kind != "moved_by_hand" {
        return false;
    }
    let since = event
        .world_time
        .saturating_sub(MOVES_REMARKED_EVERY * BACKGROUND_PERIOD);
    world
        .events()
        .iter()
        .rev()
        .take_while(|other| other.world_time >= since)
        .any(|other| other.id != deed && other.kind == "moved_by_hand")
}

pub fn pocket_universe_descriptor() -> WorldDescriptor {
    WorldDescriptor {
        pack: pocket_universe_pack_ref(),
        title: "Pocket Universe".into(),
        description:
            "A tiny world that keeps living while you are away: begin it, let it grow, then come back to see what changed.".into(),
    }
}

/// Builds the narrator each session of this Pack gets.
pub type NarratorFactory = Arc<dyn Fn() -> Box<dyn narrator::Narrator> + Send + Sync>;

/// Makes the listener each session hears the player's words with.
pub type ListenerFactory = Arc<dyn Fn() -> Box<dyn conversation::Listener> + Send + Sync>;

pub fn pocket_universe_registration() -> WorldRegistration {
    pocket_universe_registration_with_voice(Arc::new(|| Box::new(narrator::NoNarrator)))
}

/// A Pack whose Worlds have a voice: every World this registration creates
/// or opens is given a narrator from `narrator_factory`.
pub fn pocket_universe_registration_with_voice(
    narrator_factory: NarratorFactory,
) -> WorldRegistration {
    pocket_universe_registration_with_voices(
        narrator_factory,
        Arc::new(|| Box::new(conversation::OwnEars)),
    )
}

/// A Pack whose Worlds speak for their people too: every session hears the
/// player with a listener from `listener_factory`, such as a language model
/// the player switched on.
pub fn pocket_universe_registration_with_voices(
    narrator_factory: NarratorFactory,
    listener_factory: ListenerFactory,
) -> WorldRegistration {
    let create_ears = Arc::clone(&listener_factory);
    let open_ears = Arc::clone(&listener_factory);
    let own_ears = listener_factory;
    let create_voice = Arc::clone(&narrator_factory);
    let open_voice = Arc::clone(&narrator_factory);
    let own_voice = narrator_factory;
    WorldRegistration::new(pocket_universe_descriptor(), move || {
        PocketUniverseSession::fresh(create_voice(), create_ears())
    })
    .with_archive_opener(move |archive| {
        PocketUniverseSession::open_archive(archive, open_voice(), open_ears())
    })
    .with_owned_archive_opener(move |archive| {
        PocketUniverseSession::open_owned_archive(archive, own_voice(), own_ears())
    })
    .with_capabilities([
        world_projection::capability::PLOTS,
        world_projection::capability::DESIGNS,
        world_projection::capability::NAMES,
        world_projection::capability::STORY,
        world_projection::capability::OFFERED_REPLIES,
    ])
}

fn baseline() -> Result<WorldState, WorldStateError> {
    let mut state = WorldState::default();
    state.seed_entity(
        Entity::new(UNIVERSE, "universe")
            .with_component("name", "Untitled Pocket Universe")
            .with_component(SEED, UNSEEDED)
            .with_component(LAST_CHANGE, "Nothing exists here yet."),
    )?;
    Ok(state)
}

fn build_action_registry() -> Result<ActionRegistry, ActionError> {
    let mut actions = ActionRegistry::new();
    actions.register(SeedMarsColony)?;
    actions.register(Seed1980sTown)?;
    actions.register(SeedPenguinCivilization)?;
    narrator::register_actions(&mut actions)?;
    story::register_actions(&mut actions)?;
    years::register_actions(&mut actions)?;
    Ok(actions)
}

struct SeedMarsColony;
struct Seed1980sTown;
struct SeedPenguinCivilization;

impl Action for SeedMarsColony {
    fn name(&self) -> &'static str {
        "seed_mars_colony"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        seed_draft(
            state,
            "mars-colony",
            "Ares Pocket Colony",
            [
                Entity::new(SLOT_A, "habitat")
                    .with_component("name", "Ares Habitat")
                    .with_component("status", "pressurized"),
                Entity::new(SLOT_B, "person")
                    .with_component("name", "Nia Chen")
                    .with_component("role", "systems keeper"),
                Entity::new(SLOT_C, "place")
                    .with_component("name", "Hydroponics Bay")
                    .with_component("crop", "dwarf wheat"),
                Entity::new(SLOT_D, "rover")
                    .with_component("name", "Kestrel Rover")
                    .with_component("range", "18 km"),
                Entity::new(SLOT_E, "person")
                    .with_component("name", "Tomas Vale")
                    .with_component("role", "rover scout"),
                relationship_entity("Nia ↔ Tomas"),
            ],
        )
    }
}

impl Action for Seed1980sTown {
    fn name(&self) -> &'static str {
        "seed_1980s_town"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        seed_draft(
            state,
            "1980s-town",
            "Maple Street · 1987",
            [
                Entity::new(SLOT_A, "place")
                    .with_component("name", "Maple Arcade")
                    .with_component("status", "open late"),
                Entity::new(SLOT_B, "person")
                    .with_component("name", "Lena Ortiz")
                    .with_component("role", "night-shift student"),
                Entity::new(SLOT_C, "radio_station")
                    .with_component("name", "K-88 Radio")
                    .with_component("format", "local mix"),
                Entity::new(SLOT_D, "bus")
                    .with_component("name", "Night Bus 6")
                    .with_component("route", "Maple Loop"),
                Entity::new(SLOT_E, "person")
                    .with_component("name", "Max Park")
                    .with_component("role", "radio volunteer"),
                relationship_entity("Lena ↔ Max"),
            ],
        )
    }
}

impl Action for SeedPenguinCivilization {
    fn name(&self) -> &'static str {
        "seed_penguin_civilization"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        seed_draft(
            state,
            "penguin-civilization",
            "Icebridge Colony",
            [
                Entity::new(SLOT_A, "colony")
                    .with_component("name", "Icebridge")
                    .with_component("status", "lanterns lit"),
                Entity::new(SLOT_B, "penguin")
                    .with_component("name", "Piko")
                    .with_component("role", "bridge keeper"),
                Entity::new(SLOT_C, "storehouse")
                    .with_component("name", "Fish Vault")
                    .with_component("reserve", "steady"),
                Entity::new(SLOT_D, "council")
                    .with_component("name", "Aurora Council")
                    .with_component("custom", "vote at moonrise"),
                Entity::new(SLOT_E, "penguin")
                    .with_component("name", "Miri")
                    .with_component("role", "fish-vault keeper"),
                relationship_entity("Piko ↔ Miri"),
            ],
        )
    }
}

/// The pair's bond, which begins neither close nor strained.
fn relationship_entity(name: &str) -> Entity {
    Entity::new(RELATIONSHIP, "relationship")
        .with_component("name", name)
        .with_component("primary", Value::Entity(SLOT_B))
        .with_component("secondary", Value::Entity(SLOT_E))
        .with_component(RELATIONSHIP_TRUST, 4_i64)
        .with_component(RELATIONSHIP_TENSION, 3_i64)
}

fn seed_draft(
    state: &WorldState,
    seed: &str,
    universe_name: &str,
    entities: [Entity; 6],
) -> Result<EventDraft, ActionError> {
    if seed_id_from_state(state)? != UNSEEDED {
        return Err(ActionError::Invalid(
            "this Pocket Universe has already chosen a seed".into(),
        ));
    }
    let mut draft = EventDraft::new("universe_seeded");
    draft.targets = vec![UNIVERSE];
    draft.payload.insert("seed".into(), seed.into());
    draft.changes = vec![
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: "name".into(),
            value: universe_name.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: SEED.into(),
            value: seed.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),
            value: "A new world has taken shape.".into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: years::STORES.into(),
            value: 40_i64.into(),
        },
        arrival::marked(state),
    ];
    draft
        .changes
        .extend(entities.into_iter().map(StateChange::CreateEntity));
    Ok(draft)
}

pub(crate) fn seed_id(world: &World) -> &str {
    world
        .state()
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(SEED))
        .and_then(|value| match value {
            Value::Text(value) => Some(value.as_str()),
            _ => None,
        })
        .unwrap_or(UNSEEDED)
}

pub(crate) use lives::shown::{guest_words, guests_staying};

fn seed_id_from_state(state: &WorldState) -> Result<String, ActionError> {
    match state
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(SEED))
    {
        Some(Value::Text(seed)) => Ok(seed.clone()),
        _ => Err(ActionError::Invalid(
            "Pocket Universe seed state is missing".into(),
        )),
    }
}

#[cfg(test)]
fn text_component_from_state(
    state: &WorldState,
    entity: EntityId,
    key: &str,
) -> Result<String, ActionError> {
    match state
        .entity(entity)
        .and_then(|entity| entity.component(key))
    {
        Some(Value::Text(value)) => Ok(value.clone()),
        _ => Err(ActionError::Invalid(format!(
            "entity {entity} has no text component {key}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every place to begin shows itself as it will first stand: its own
    /// people and buildings, not a landscape alone.
    #[test]
    fn every_place_to_begin_is_shown_with_its_people() {
        let universe = PocketUniverse::new().unwrap();
        let snapshot = universe.projection_snapshot();
        let beginnings = snapshot
            .commands
            .iter()
            .filter(|command| command.scenery.is_some())
            .collect::<Vec<_>>();
        assert_eq!(beginnings.len(), 3);
        for command in beginnings {
            let preview = command.preview.as_ref().expect("a picture of the place");
            let people = preview
                .canvas
                .items
                .iter()
                .filter(|item| item.kind == world_projection::CanvasItemKind::Actor)
                .count();
            assert!(people >= 2, "{} shows {people} people", command.id);
            assert!(
                preview.canvas.items.len() > people,
                "{} shows no buildings",
                command.id
            );
        }
        // Showing a place does not begin it.
        assert_eq!(universe.world().events().len(), 0);
    }

    /// Everything a player reads in this World, over a first session and a
    /// return, speaks about the World and never about the engine.
    #[test]
    fn nothing_a_player_reads_is_in_engine_words() {
        let registry = registry();
        let mut found = std::collections::BTreeSet::new();
        let mut check = |snapshot: &ProjectionSnapshot| {
            for line in snapshot.visible_text() {
                for word in world_projection::engine_words_in(line) {
                    found.insert(format!("{word:?} in {line:?}"));
                }
            }
        };
        let empty = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        check(&empty.snapshot());
        for seed in [
            SEED_MARS_COLONY_COMMAND,
            SEED_1980S_TOWN_COMMAND,
            SEED_PENGUIN_CIVILIZATION_COMMAND,
        ] {
            let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
            let mut snapshot = session
                .handle(ProjectionIntent::InvokeCommand(seed.into()))
                .unwrap();
            check(&snapshot);
            for turn in 0..12 {
                if turn % 4 == 3 {
                    snapshot = session.advance_background(3).unwrap();
                } else {
                    // Whatever can be done now, every choice and deed in turn.
                    let offered = snapshot
                        .commands
                        .iter()
                        .filter(|command| command.unavailable.is_none())
                        .collect::<Vec<_>>();
                    let Some(command) = offered
                        .get(turn % offered.len().max(1))
                        .map(|command| command.id.clone())
                    else {
                        break;
                    };
                    snapshot = session
                        .handle(ProjectionIntent::InvokeCommand(command))
                        .unwrap();
                }
                check(&snapshot);
            }
        }
        assert!(
            found.is_empty(),
            "engine words:\n{}",
            found.into_iter().collect::<Vec<_>>().join("\n")
        );
    }

    /// In every seed, the keeper, the explorer and anyone who came to stay
    /// can be spoken to in the player's own words; each answers in their
    /// own, and no time passes. An unseeded World has nobody to talk to.
    #[test]
    fn people_answer_what_you_say_in_every_seed() {
        let registry = registry();
        let empty = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        assert!(!empty.snapshot().capabilities.talk);
        for seed in [
            SEED_MARS_COLONY_COMMAND,
            SEED_1980S_TOWN_COMMAND,
            SEED_PENGUIN_CIVILIZATION_COMMAND,
        ] {
            let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
            session
                .handle(ProjectionIntent::InvokeCommand(seed.into()))
                .unwrap();
            let snapshot = session.advance_background(2).unwrap();
            assert!(snapshot.capabilities.talk, "{seed}");
            let other = crate::talk::first_name_for_test(&session_world(&*session), SLOT_E);
            for who in [SLOT_B, SLOT_E] {
                let mut answers = Vec::new();
                for words in [
                    "Hi!".to_string(),
                    "How are you doing?".into(),
                    format!("What do you think of {other}?"),
                    "What's new?".into(),
                    "What do you need?".into(),
                ] {
                    let snapshot = session
                        .handle(ProjectionIntent::Say {
                            to: world_projection::SelectionId::Entity(who),
                            words: words.clone(),
                            ears: world_projection::Ears::World,
                        })
                        .unwrap();
                    let exchange = snapshot
                        .exchanges_with(world_projection::SelectionId::Entity(who))
                        .last()
                        .unwrap()
                        .clone();
                    assert_eq!(exchange.words, words);
                    let engine = world_projection::engine_words_in(&exchange.answer);
                    assert!(
                        engine.is_empty(),
                        "{seed}: {engine:?} in {:?}",
                        exchange.answer
                    );
                    answers.push(exchange.answer);
                }
                let unique = answers.iter().collect::<std::collections::BTreeSet<_>>();
                assert!(unique.len() >= 4, "{seed}: {answers:#?}");
            }
            let after = session.snapshot();
            assert_eq!(after.world_time, snapshot.world_time, "{seed}");
            assert_eq!(after.exchanges.len(), 10, "{seed}");
        }
    }

    fn session_world(session: &dyn world_host::WorldSession) -> World {
        let archive = session.archive().unwrap().unwrap();
        PocketUniverse::resume_archive(&archive)
            .unwrap()
            .world()
            .clone()
    }

    /// A World window is a place, not a page: at rest it shows at most
    /// forty words, bar included, and the scene takes most of it.
    #[test]
    fn a_world_window_shows_a_place_not_a_page() {
        let registry = registry();
        for seed in [
            SEED_MARS_COLONY_COMMAND,
            SEED_1980S_TOWN_COMMAND,
            SEED_PENGUIN_CIVILIZATION_COMMAND,
        ] {
            let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
            let mut snapshot = session
                .handle(ProjectionIntent::InvokeCommand(seed.into()))
                .unwrap();
            for turn in 0..10 {
                let words = world_gpui::words_at_rest(&snapshot);
                assert!(
                    words <= world_gpui::RESTING_WORD_LIMIT,
                    "{seed} turn {turn}: {words} words at rest"
                );
                // Whatever can be done now, every choice and deed in turn.
                let offered = snapshot
                    .commands
                    .iter()
                    .filter(|command| command.unavailable.is_none())
                    .collect::<Vec<_>>();
                let Some(command) = offered
                    .get(turn % offered.len().max(1))
                    .map(|command| command.id.clone())
                else {
                    break;
                };
                snapshot = session
                    .handle(ProjectionIntent::InvokeCommand(command))
                    .unwrap();
            }
        }
        assert!(world_gpui::scene_share(900.0) >= 0.75);
        assert!(world_gpui::scene_share(600.0) >= 0.75);
    }

    /// People say something at every turn, over whoever it happened to, and
    /// every question a player can ask has an answer; one that asks for
    /// something names a choice that is really on offer.
    #[test]
    fn the_pair_speak_and_answer_and_ask_only_for_what_is_on_offer() {
        let registry = registry();
        for seed in [
            SEED_MARS_COLONY_COMMAND,
            SEED_1980S_TOWN_COMMAND,
            SEED_PENGUIN_CIVILIZATION_COMMAND,
        ] {
            let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
            let mut snapshot = session
                .handle(ProjectionIntent::InvokeCommand(seed.into()))
                .unwrap();
            for turn in 0..8 {
                let on_scene = |id: world_projection::SelectionId| {
                    snapshot.canvas.items.iter().any(|item| item.id == id)
                };
                let on_timeline = |id: world_projection::SelectionId| {
                    snapshot.timeline.items.iter().any(|item| item.id == id)
                };
                assert!(
                    !snapshot.voices.is_empty(),
                    "{seed} turn {turn}: nobody spoke"
                );
                for voice in &snapshot.voices {
                    assert!(
                        on_scene(voice.speaker),
                        "{seed}: {voice:?} speaker is not on stage"
                    );
                    assert!(
                        on_timeline(voice.moment),
                        "{seed}: {voice:?} is not a moment"
                    );
                }
                assert_eq!(snapshot.talks.len(), 6, "{seed}: three questions each");
                for talk in &snapshot.talks {
                    assert!(on_scene(talk.who));
                    if let Some(command) = &talk.asks_for {
                        assert!(snapshot.command(command).is_some(), "{seed}: {talk:?}");
                    }
                }
                let people = snapshot
                    .canvas
                    .items
                    .iter()
                    .filter(|item| item.kind == world_projection::CanvasItemKind::Actor)
                    .collect::<Vec<_>>();
                assert!(people.iter().all(|person| person.look.is_some()));
                // Whatever can be done now, every choice and deed in turn.
                let offered = snapshot
                    .commands
                    .iter()
                    .filter(|command| command.unavailable.is_none())
                    .collect::<Vec<_>>();
                let Some(command) = offered
                    .get(turn % offered.len().max(1))
                    .map(|command| command.id.clone())
                else {
                    break;
                };
                snapshot = session
                    .handle(ProjectionIntent::InvokeCommand(command))
                    .unwrap();
            }
        }
    }

    /// A choice's gauge marks are what making it actually does: play it for
    /// real and the gauges move exactly as marked.
    #[test]
    fn a_choice_marks_the_gauges_it_will_move_and_they_move_that_way() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        assert!(
            session.snapshot().gauges.is_empty(),
            "nothing to keep score of yet"
        );
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_MARS_COLONY_COMMAND.into(),
            ))
            .unwrap();
        let mut marked = None;
        for _ in 0..40 {
            let snapshot = session.snapshot();
            if let Some(command) = snapshot.commands.iter().find(|command| {
                command.question.is_some()
                    && command.unavailable.is_none()
                    && !command.moves.is_empty()
            }) {
                marked = Some((snapshot.clone(), command.clone()));
                break;
            }
            session
                .handle(ProjectionIntent::InvokeCommand(NUDGE_COMMAND.into()))
                .unwrap();
        }
        let (before, command) = marked.expect("some answer moves a gauge");
        let ids: Vec<&str> = before
            .gauges
            .iter()
            .map(|gauge| gauge.id.as_str())
            .collect();
        assert_eq!(ids, ["trust", "tension", "stores"]);
        let after = session
            .handle(ProjectionIntent::InvokeCommand(command.id.clone()))
            .unwrap();
        assert_eq!(
            world_projection::gauge_moves(&before.gauges, &after.gauges),
            command.moves
        );
    }

    /// Every gauge a World shows can be moved: over a season and more of
    /// play, answering what is asked and building, each gauge changes, and
    /// something the player chose is followed by a gauge moving.
    #[test]
    fn every_gauge_can_be_moved() {
        for seed in [
            SEED_MARS_COLONY_COMMAND,
            SEED_1980S_TOWN_COMMAND,
            SEED_PENGUIN_CIVILIZATION_COMMAND,
        ] {
            let mut universe = freshly_seeded(seed);
            let first = universe.projection_snapshot().gauges;
            assert!(!first.is_empty(), "{seed}: no gauges");
            let mut moved = std::collections::BTreeSet::new();
            let mut chosen_and_moved = 0;
            for period in 0..120 {
                let snapshot = universe.projection_snapshot();
                if let Some(answer) = snapshot
                    .commands
                    .iter()
                    .find(|command| command.question.is_some() && command.unavailable.is_none())
                {
                    let before = projection::gauges(universe.world());
                    universe.invoke_projection_command(&answer.id).unwrap();
                    let after = projection::gauges(universe.world());
                    if before != after {
                        chosen_and_moved += 1;
                    }
                }
                if period % 3 == 0 {
                    if let Some(deed) = snapshot
                        .commands
                        .iter()
                        .find(|command| command.hand.is_some() && command.unavailable.is_none())
                    {
                        let _ = universe.invoke_projection_command(&deed.id);
                    }
                }
                let before = projection::gauges(universe.world());
                universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
                let now = projection::gauges(universe.world());
                for (then, gauge) in before.iter().zip(&now) {
                    if then.value != gauge.value || then.reading != gauge.reading {
                        moved.insert(gauge.id.clone());
                    }
                }
                for (then, gauge) in first.iter().zip(&now) {
                    if then.value != gauge.value {
                        moved.insert(gauge.id.clone());
                    }
                }
            }
            for gauge in &first {
                assert!(
                    moved.contains(&gauge.id),
                    "{seed}: {} never moved",
                    gauge.id
                );
            }
            assert!(
                chosen_and_moved > 0,
                "{seed}: none of the player's choices moved a gauge"
            );
        }
    }

    fn registry() -> world_host::WorldRegistry {
        let mut registry = world_host::WorldRegistry::new();
        registry.register(pocket_universe_registration()).unwrap();
        registry
    }

    #[test]
    fn empty_universe_offers_multiple_world_seeds() {
        let registry = registry();
        let session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        let snapshot = session.snapshot();
        let commands = snapshot
            .commands
            .iter()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(snapshot.title, "A new World");
        assert!(commands.contains(&SEED_MARS_COLONY_COMMAND));
        assert!(commands.contains(&SEED_1980S_TOWN_COMMAND));
        assert!(commands.contains(&SEED_PENGUIN_CIVILIZATION_COMMAND));
        assert!(snapshot.collection.items.is_empty());
    }

    #[test]
    fn only_a_begun_world_says_it_moves_on_its_own() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        assert!(!session.snapshot().capabilities.background);
        let seeded = session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_MARS_COLONY_COMMAND.into(),
            ))
            .unwrap();
        assert!(seeded.capabilities.background);
    }

    #[test]
    fn one_pack_can_seed_distinct_world_shapes() {
        let registry = registry();
        let mut mars = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        let mut town = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();

        let mars_snapshot = mars
            .handle(ProjectionIntent::InvokeCommand(
                SEED_MARS_COLONY_COMMAND.into(),
            ))
            .unwrap();
        let town_snapshot = town
            .handle(ProjectionIntent::InvokeCommand(
                SEED_1980S_TOWN_COMMAND.into(),
            ))
            .unwrap();

        assert_eq!(mars.pack(), town.pack());
        assert_ne!(mars_snapshot.title, town_snapshot.title);
        assert_ne!(
            mars_snapshot.collection.items,
            town_snapshot.collection.items
        );
        assert!(mars_snapshot
            .collection
            .items
            .iter()
            .any(|item| item.title == "Ares Habitat"));
        assert!(town_snapshot
            .collection
            .items
            .iter()
            .any(|item| item.title == "Maple Arcade"));
    }

    #[test]
    fn seed_is_a_durable_one_time_choice() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_PENGUIN_CIVILIZATION_COMMAND.into(),
            ))
            .unwrap();

        let error = session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_MARS_COLONY_COMMAND.into(),
            ))
            .unwrap_err();
        assert!(error.to_string().contains("already chosen a seed"));
    }

    #[test]
    fn archive_round_trip_preserves_seed_and_growth() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_1980S_TOWN_COMMAND.into(),
            ))
            .unwrap();
        session.advance_background(3).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(NUDGE_COMMAND.into()))
            .unwrap();
        let before = session.snapshot();
        let archive = session.archive().unwrap().unwrap();
        drop(session);

        let reopened = registry.open_archive(&archive).unwrap();

        assert_eq!(reopened.snapshot(), before);
        assert_eq!(reopened.archive().unwrap().unwrap(), archive);
    }

    /// An archive the Pack may keep opens to the same World as one it is
    /// lent.
    #[test]
    fn an_owned_archive_opens_to_the_same_world_as_a_lent_one() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_1980S_TOWN_COMMAND.into(),
            ))
            .unwrap();
        session.advance_background(12).unwrap();
        let archive = session.archive().unwrap().unwrap();

        let lent = PocketUniverse::resume_archive(&archive).unwrap();
        let owned = PocketUniverse::resume_owned_archive(archive.clone()).unwrap();
        assert_eq!(owned.world().events(), lent.world().events());
        assert_eq!(owned.world().state(), lent.world().state());
        assert_eq!(owned.projection_snapshot(), lent.projection_snapshot());

        let (reopened, _) = registry.open_owned_archive(archive.clone()).unwrap();
        let borrowed = registry.open_archive(&archive).unwrap();
        assert_eq!(reopened.snapshot(), borrowed.snapshot());
        assert_eq!(reopened.archive().unwrap().unwrap(), archive);
    }

    /// A session as a host saw one before sessions could go back: it has
    /// no checkpoint, so every change is tried on a copy opened anew.
    struct Reopened(Box<dyn WorldSession>);

    impl WorldSession for Reopened {
        fn pack(&self) -> world_persistence::WorldPackRef {
            self.0.pack()
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            self.0.snapshot()
        }

        fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            self.0.handle(intent)
        }

        fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
            self.0.advance_background(periods)
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            self.0.archive()
        }
    }

    fn reopening_registry() -> world_host::WorldRegistry {
        let mut registry = world_host::WorldRegistry::new();
        registry
            .register(
                WorldRegistration::new(pocket_universe_descriptor(), || {
                    let session = PocketUniverseSession::fresh(
                        Box::new(narrator::NoNarrator),
                        Box::new(conversation::OwnEars),
                    )?;
                    Ok(Box::new(Reopened(session)) as Box<dyn WorldSession>)
                })
                .with_archive_opener(|archive| {
                    let session = PocketUniverseSession::open_archive(
                        archive,
                        Box::new(narrator::NoNarrator),
                        Box::new(conversation::OwnEars),
                    )?;
                    Ok(Box::new(Reopened(session)) as Box<dyn WorldSession>)
                }),
            )
            .unwrap();
        registry
    }

    /// A folder of Worlds of its own, under the system's temporary folder.
    fn temp_library(name: &str) -> (std::path::PathBuf, world_library::WorldLibrary) {
        let root =
            std::env::temp_dir().join(format!("pocket-universe-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        (root.clone(), world_library::WorldLibrary::new(root))
    }

    /// Whatever can be done at `turn`: the next day, or in turn a choice or
    /// a deed on offer.
    fn next_intent(snapshot: &ProjectionSnapshot, turn: usize) -> ProjectionIntent {
        let offered = snapshot
            .commands
            .iter()
            .filter(|command| command.unavailable.is_none() && command.id != NUDGE_COMMAND)
            .collect::<Vec<_>>();
        match offered.get(turn % offered.len().max(1)) {
            Some(command) if turn % 2 == 1 => ProjectionIntent::InvokeCommand(command.id.clone()),
            _ => ProjectionIntent::InvokeCommand(NUDGE_COMMAND.into()),
        }
    }

    /// A session goes back exactly: after a checkpoint, a day and a return
    /// rolled back leave its archive and what it shows as they were.
    #[test]
    fn a_session_rolled_back_is_as_it_was() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_1980S_TOWN_COMMAND.into(),
            ))
            .unwrap();
        session.advance_background(3).unwrap();
        let snapshot = session.snapshot();
        let archive = session.archive().unwrap().unwrap();
        let events = archive.events.len();

        let mark = session.checkpoint().unwrap().expect("a session goes back");
        session
            .handle(ProjectionIntent::InvokeCommand(NUDGE_COMMAND.into()))
            .unwrap();
        session.advance_background(2).unwrap();
        assert_ne!(session.archive().unwrap().unwrap(), archive);
        session.rollback(mark).unwrap();

        assert_eq!(session.snapshot(), snapshot);
        assert_eq!(session.archive().unwrap().unwrap(), archive);
        // Only what came after the first events, when asked for so.
        let since = session.archive_since(events - 2).unwrap().unwrap();
        assert_eq!(since.events, archive.events[events - 2..]);
    }

    /// A save that fails leaves the World as it was: nothing the change did
    /// is shown or kept.
    #[test]
    fn a_failed_save_leaves_the_world_as_it_was() {
        let registry = registry();
        let (root, library) = temp_library("failed-save");
        let id = world_library::WorldDocumentId::new("maple").unwrap();
        let mut session = world_library::DurableWorldSession::create(
            id,
            POCKET_UNIVERSE_PACK_ID,
            &registry,
            &library,
        )
        .unwrap();
        session
            .handle(
                ProjectionIntent::InvokeCommand(SEED_1980S_TOWN_COMMAND.into()),
                &registry,
                &library,
            )
            .unwrap();
        for _ in 0..3 {
            session
                .handle(
                    ProjectionIntent::InvokeCommand(NUDGE_COMMAND.into()),
                    &registry,
                    &library,
                )
                .unwrap();
        }
        let before = session.snapshot();

        // The turns above are written away from them; once they are on
        // disk, nothing is writing into the folder while it is taken away.
        session.flush().unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        std::fs::File::create(&root).unwrap();
        assert!(session
            .handle(
                ProjectionIntent::InvokeCommand(NUDGE_COMMAND.into()),
                &registry,
                &library,
            )
            .is_err());
        assert_eq!(session.snapshot(), before);
        let _ = std::fs::remove_file(&root);
    }

    /// Twenty changes saved by going back when need be write the same World
    /// into its file as twenty saved the old way, each tried on a copy
    /// opened anew: the same history, checkpoint and description.
    #[test]
    fn saving_on_the_live_world_writes_what_the_old_way_wrote() {
        let files =
            [("live", registry()), ("reopened", reopening_registry())].map(|(name, registry)| {
                let (root, library) = temp_library(name);
                let id = world_library::WorldDocumentId::new("ares").unwrap();
                let mut session = world_library::DurableWorldSession::create(
                    id.clone(),
                    POCKET_UNIVERSE_PACK_ID,
                    &registry,
                    &library,
                )
                .unwrap();
                let mut snapshot = session
                    .handle(
                        ProjectionIntent::InvokeCommand(SEED_MARS_COLONY_COMMAND.into()),
                        &registry,
                        &library,
                    )
                    .unwrap();
                for turn in 0..19 {
                    snapshot = session
                        .handle(next_intent(&snapshot, turn), &registry, &library)
                        .unwrap();
                }
                session.flush().unwrap();
                let bytes = std::fs::read(library.path(&id)).unwrap();
                let _ = std::fs::remove_dir_all(root);
                (snapshot, bytes)
            });
        assert_eq!(files[0].0, files[1].0);
        let read = |bytes: &[u8]| world_document::WorldDocument::from_bytes(bytes).unwrap();
        let (live, reopened) = (read(&files[0].1), read(&files[1].1));
        assert_eq!(live.metadata, reopened.metadata);
        assert_eq!(live.archive.events, reopened.archive.events);
        assert_eq!(live.archive.checkpoint, reopened.archive.checkpoint);
        assert_eq!(live.archive, reopened.archive);
    }

    #[test]
    fn background_time_grows_a_seeded_world() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_MARS_COLONY_COMMAND.into(),
            ))
            .unwrap();
        let before = session.snapshot();
        let events_before = session.archive().unwrap().unwrap().events.len();

        let after = session.advance_background(2).unwrap();

        assert_eq!(after.world_time, before.world_time + 20);
        assert!(session.archive().unwrap().unwrap().events.len() > events_before);
        let briefing = after.briefing.as_ref().unwrap();
        assert_eq!(briefing.title, "While you were away");
        assert!(
            briefing
                .items
                .iter()
                .filter(|item| item.selection.is_some())
                .count()
                >= 1,
            "a return says what happened"
        );
        assert!(briefing
            .items
            .iter()
            .all(|item| !item.detail.trim().is_empty()));
    }

    #[test]
    fn forking_before_seed_returns_to_an_empty_universe() {
        let registry = registry();
        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        let seeded = session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_PENGUIN_CIVILIZATION_COMMAND.into(),
            ))
            .unwrap();
        let seed_event = seeded
            .timeline
            .items
            .iter()
            .filter(|item| item.title.ends_with("began"))
            .find_map(|item| match item.id {
                world_projection::SelectionId::Event(id) => Some(id),
                _ => None,
            })
            .unwrap();

        let forked = session
            .handle(ProjectionIntent::ForkBeforeEvent(seed_event))
            .unwrap();

        assert_eq!(forked.title, "A new World");
        assert!(forked.collection.items.is_empty());
        assert_eq!(forked.commands.len(), 3);
    }

    /// A World that has only been seeded, with every later decision still open.
    fn freshly_seeded(seed_command: &str) -> PocketUniverse {
        let mut universe = PocketUniverse::new().unwrap();
        universe.invoke_projection_command(seed_command).unwrap();
        universe
    }

    /// Answer the first question the World asks each period, whatever it is.
    fn live_with(universe: &mut PocketUniverse, periods: usize) {
        for _ in 0..periods {
            universe.advance_periods(1).unwrap();
            let answer = universe
                .projection_snapshot()
                .commands
                .into_iter()
                .find(|command| command.question.is_some() && command.unavailable.is_none());
            if let Some(answer) = answer {
                universe.invoke_projection_command(&answer.id).unwrap();
            }
        }
    }

    /// A narrator that says a fixed line and counts both how often it was
    /// asked for a whole return and how many lines it was asked for.
    #[derive(Default)]
    struct NarratorCalls {
        returns: std::cell::Cell<usize>,
        lines: std::cell::Cell<usize>,
    }

    struct ScriptedNarrator {
        line: String,
        calls: std::rc::Rc<NarratorCalls>,
    }

    impl narrator::Narrator for ScriptedNarrator {
        fn narrate(&mut self, facts: &narrator::NarrationFacts) -> Option<String> {
            self.calls.lines.set(self.calls.lines.get() + 1);
            // The narrator is handed facts that are already decided.
            assert!(!facts.seed.is_empty());
            assert!(!facts.event_kind.is_empty());
            assert!(!facts.table_summary.is_empty());
            Some(self.line.clone())
        }

        fn narrate_all(&mut self, facts: &[narrator::NarrationFacts]) -> Vec<Option<String>> {
            self.calls.returns.set(self.calls.returns.get() + 1);
            facts.iter().map(|fact| self.narrate(fact)).collect()
        }
    }

    fn narrated_with(line: &str) -> (PocketUniverse, std::rc::Rc<NarratorCalls>) {
        let calls = std::rc::Rc::new(NarratorCalls::default());
        let mut universe = freshly_seeded(SEED_MARS_COLONY_COMMAND);
        universe.set_narrator(Box::new(ScriptedNarrator {
            line: line.into(),
            calls: std::rc::Rc::clone(&calls),
        }));
        (universe, calls)
    }

    #[test]
    fn a_world_with_no_narrator_reads_exactly_as_it_always_did() {
        // The floor under everything else here: the table is what a World
        // shows when nobody is putting it into words.
        let mut plain = freshly_seeded(SEED_MARS_COLONY_COMMAND);
        live_with(&mut plain, 12);

        assert!(
            !plain
                .world()
                .events()
                .iter()
                .any(|event| event.kind == narrator::NARRATED),
            "a World with no narrator recorded a narrated line"
        );
        let thread = text_component_from_state(plain.world().state(), UNIVERSE, LAST_CHANGE)
            .unwrap_or_default();
        assert!(
            !thread.is_empty(),
            "the table left the World with nothing to say"
        );
    }

    #[test]
    fn a_world_with_a_voice_reads_in_its_own_words() {
        let line = "Ares stopped listening for the relay and started keeping its own time.";
        let (mut universe, calls) = narrated_with(line);
        let cursor = universe.world().events().len();
        universe.advance_periods(6).unwrap();

        assert!(calls.returns.get() >= 1, "the narrator was never asked");
        assert_eq!(
            text_component_from_state(universe.world().state(), UNIVERSE, LAST_CHANGE).unwrap(),
            line,
            "the World kept the table line instead of its own words"
        );
        let briefing = universe
            .projection_snapshot_since(Some(cursor))
            .briefing
            .expect("a return has a briefing");
        assert!(
            briefing.items.iter().any(|item| item.detail == line),
            "the return digest never showed the World's own words: {:?}",
            briefing
                .items
                .iter()
                .map(|item| &item.title)
                .collect::<Vec<_>>()
        );
        assert!(
            !briefing
                .items
                .iter()
                .any(|item| item.title.contains("world narrated")),
            "a narrated line became a digest entry of its own"
        );
    }

    /// A resident of one place comes to stay in another: a penguin from
    /// Icebridge on Mars, drawn as Icebridge draws them and saying their
    /// line, recorded so the visit replays exactly; nothing is written to
    /// Icebridge.
    #[test]
    fn a_friends_resident_comes_to_stay_drawn_as_at_home() {
        use world_projection::Guest;
        let mut ice = freshly_seeded(SEED_PENGUIN_CIVILIZATION_COMMAND);
        live_with(&mut ice, 2);
        let before = ice.archive().unwrap();
        let guest = Guest::residents(&ice.projection_snapshot())
            .into_iter()
            .find(|guest| guest.drawing.is_some())
            .expect("a penguin with their own drawing");
        let guest = Guest {
            line: Some("The ice sang all night.".into()),
            ..guest
        };
        let mut mars = freshly_seeded(SEED_MARS_COLONY_COMMAND);
        mars.host(&guest).unwrap();
        let snapshot = mars.projection_snapshot();
        let standing = snapshot
            .canvas
            .items
            .iter()
            .find(|item| item.label == guest.name && item.detail.contains("Visiting"))
            .expect("the guest stands on the colony's canvas");
        assert_eq!(
            snapshot.drawing_of(standing).map(|drawing| &drawing.parts),
            guest
                .travelling_drawing()
                .as_ref()
                .map(|drawing| &drawing.parts)
        );
        assert_eq!(
            standing.look.map(|look| look.bird),
            guest.look.map(|look| look.bird)
        );
        assert!(snapshot
            .voices
            .iter()
            .any(|voice| voice.speaker == standing.id && voice.line == "The ice sang all night."));
        let replayed = mars.world().replay().unwrap();
        assert_eq!(replayed.state(), mars.world().state());
        let reopened = PocketUniverse::resume_archive(&mars.archive().unwrap()).unwrap();
        assert!(reopened
            .projection_snapshot()
            .canvas
            .items
            .iter()
            .any(|item| item.id == standing.id));
        assert_eq!(ice.archive().unwrap(), before, "Icebridge is untouched");
    }

    /// A narrator is a model like any other: a line that speaks as a
    /// machine, of the world outside, or of a place the World never had,
    /// is not recorded, and the table line stands.
    #[test]
    fn a_narration_out_of_the_world_leaves_the_table_line() {
        for line in [
            "As an AI language model, I can report the colony is doing fine.",
            "The colony posted its harvest on Instagram.",
            "Commander Natasha Volkov flew in from Cassini Dome.",
        ] {
            let (mut universe, calls) = narrated_with(line);
            universe.advance_periods(6).unwrap();
            assert!(calls.returns.get() >= 1, "the narrator was asked");
            assert!(
                !universe
                    .world()
                    .events()
                    .iter()
                    .any(|event| event.kind == narrator::NARRATED),
                "{line:?} was recorded"
            );
        }
    }

    #[test]
    fn a_narrator_is_asked_once_for_a_return_not_once_per_period() {
        // The whole cost argument for giving a World a model rests on this.
        let (mut universe, calls) = narrated_with("A week of dust, and the intakes held.");
        universe.advance_periods(28).unwrap();
        assert_eq!(
            calls.returns.get(),
            1,
            "a twenty-eight period catch-up made {} requests",
            calls.returns.get()
        );
        assert!(
            calls.lines.get() > 1,
            "a week-long return put only {} line(s) into the World's own words",
            calls.lines.get()
        );
        assert!(
            calls.lines.get() <= projection::RETURN_DIGEST_ENTRIES,
            "more lines were narrated than a return digest shows"
        );
    }

    #[test]
    fn nothing_a_narrator_says_can_make_a_world_worse() {
        // Whatever comes back, the World keeps every fact it had and reads at
        // least as well as the table.
        for unusable in ["", "   ", "flooded ".repeat(200).as_str(), "two\nlines"] {
            let (mut universe, _) = narrated_with(unusable);
            let plain = {
                let mut plain = freshly_seeded(SEED_MARS_COLONY_COMMAND);
                live_with(&mut plain, 8);
                plain
            };
            live_with(&mut universe, 8);

            assert!(
                !universe
                    .world()
                    .events()
                    .iter()
                    .any(|event| event.kind == narrator::NARRATED),
                "an unusable line was recorded: {unusable:?}"
            );
            assert_eq!(
                text_component_from_state(universe.world().state(), UNIVERSE, LAST_CHANGE).unwrap(),
                text_component_from_state(plain.world().state(), UNIVERSE, LAST_CHANGE).unwrap(),
                "an unusable line changed what the World says: {unusable:?}"
            );
        }
    }

    #[test]
    fn replaying_a_narrated_world_never_asks_the_narrator_anything() {
        // The reason this is safe to point at a model: what was said is a
        // recorded fact, and replay applies recorded facts.
        let line = "The relay stayed quiet, and Ares stopped waiting on it.";
        let (mut universe, calls) = narrated_with(line);
        universe.advance_periods(6).unwrap();
        let asked = calls.returns.get();

        let replayed = universe.world().replay().unwrap();
        assert_eq!(
            calls.returns.get(),
            asked,
            "replay asked the narrator again"
        );
        assert_eq!(replayed.events(), universe.world().events());
        assert_eq!(replayed.state(), universe.world().state());
        assert_eq!(
            text_component_from_state(replayed.state(), UNIVERSE, LAST_CHANGE).unwrap(),
            line
        );
    }

    #[test]
    fn a_narrated_line_survives_being_saved_and_reopened() {
        let line = "Nia logged the first fouled intake and said nothing about it.";
        let (mut universe, _) = narrated_with(line);
        universe.advance_periods(6).unwrap();

        let archive = universe.archive().unwrap();
        let reopened = PocketUniverse::resume_archive(&archive).unwrap();
        assert_eq!(
            text_component_from_state(reopened.world().state(), UNIVERSE, LAST_CHANGE).unwrap(),
            line,
            "reopening the World lost its own words"
        );
    }

    #[test]
    fn the_same_consequence_is_never_put_into_words_twice() {
        let (mut universe, calls) = narrated_with("The dust thickened early this year.");
        universe.advance_periods(4).unwrap();
        let after_first = calls.returns.get();
        let events = universe.world().events().len();

        // Coming back without the World having moved leaves it alone.
        universe.advance_periods(0).unwrap();
        assert_eq!(calls.returns.get(), after_first);
        assert_eq!(universe.world().events().len(), events);
    }

    #[test]
    fn every_line_a_return_shows_can_be_in_the_world_own_words() {
        // Measured before this: over twenty week-long returns the digest's
        // trouble lines were stuck at exactly three distinct phrasings each
        // (83%, 82%, 82% repeat), because there are three troubles per seed and
        // one written line per stage. Narrating only the latest consequence
        // left the other entries on the table.
        let (mut universe, calls) = narrated_with("The season turned against Ares, and it held.");
        let cursor = universe.world().events().len();
        universe.advance_periods(28).unwrap();

        let briefing = universe
            .projection_snapshot_since(Some(cursor))
            .briefing
            .expect("a return has a briefing");
        let narrated_events = universe
            .world()
            .events()
            .iter()
            .filter(|event| event.kind == narrator::NARRATED)
            .count();
        assert!(
            narrated_events > 1,
            "only {narrated_events} line(s) of the return were put into the World's own words"
        );
        assert_eq!(
            calls.returns.get(),
            1,
            "narrating the whole digest cost more than the one request a return budgets"
        );

        // Every digest entry the World has words for shows those words.
        let voiced = briefing
            .items
            .iter()
            .filter(|item| item.detail == "The season turned against Ares, and it held.")
            .count();
        assert_eq!(
            voiced, narrated_events,
            "the briefing kept a table line for something the World had already put into its own words: {:?}",
            briefing
                .items
                .iter()
                .map(|item| (&item.title, &item.detail))
                .collect::<Vec<_>>()
        );
    }
}
