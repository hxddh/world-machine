//! A newcomer's day in a Tiny Society World, walked headlessly on a clock
//! at one of the paces of `demo/pacing.json`: the game's own measure of how
//! long play takes. Shared by the demo's walk and the full app's first
//! session.
//!
//! What is said now is shown at 4.6 s a page of two lines (the World
//! window's own `LINE_SECONDS`); reading takes a second for every
//! `chars_per_second` characters; choosing takes `choose_seconds`. Each day
//! the newcomer hears what is said, reads the morning (its letter, find,
//! what was tended and its question) and the news, looks about for
//! `look_seconds`, answers the cards, makes what their hands can and does
//! the favour asked (if their pace does), and has a word with someone.
//! Nothing here asks a model: the people answer in the World's own words,
//! as they do with the voice off.

use world_library::{DurableWorldSession, WorldLibrary};
use world_machine_desktop::demo::Pace;
use world_projection::{Ears, ProjectionIntent, ProjectionSnapshot, Voice};

/// A walk in progress: the World, the clock, and what the newcomer has
/// already heard and made.
pub struct Walker<'a> {
    pub session: DurableWorldSession,
    pub registry: world_host::WorldRegistry,
    pub library: WorldLibrary,
    pub pace: &'a Pace,
    /// Seconds of play so far.
    pub clock: f32,
    pub heard: Vec<Voice>,
    pub made: Vec<String>,
    /// Whether they have put something up on a plot.
    pub plotted: bool,
    /// The mornings read, by their moment.
    pub mornings: Vec<world_projection::SelectionId>,
}

impl<'a> Walker<'a> {
    /// Reading `text`, at the pace.
    pub fn read(&self, text: &str) -> f32 {
        text.chars().count() as f32 / self.pace.chars_per_second
    }

    /// `text` said on the scene, shown a page of two lines at a time.
    pub fn page(text: &str) -> f32 {
        4.6 * text.chars().count().div_ceil(72) as f32
    }

    fn handle(&mut self, intent: ProjectionIntent) -> Option<ProjectionSnapshot> {
        self.session
            .handle(intent, &self.registry, &self.library)
            .ok()
    }

    /// One day, up to letting it pass: what is said, the morning and the
    /// news, a look about, the cards and the deeds, the favour and a word
    /// with someone. `see` is shown the World after each step, with the
    /// clock. The World as the day leaves it.
    pub fn play_day(
        &mut self,
        mut snapshot: ProjectionSnapshot,
        pass: &str,
        see: &mut dyn FnMut(&ProjectionSnapshot, f32),
    ) -> ProjectionSnapshot {
        let pace = self.pace;
        // What is said now, in turn (waited for, or not), and the
        // morning's news.
        for voice in &snapshot.voices {
            if !self.heard.contains(voice) {
                if pace.waits_for_what_is_said {
                    self.clock += Self::page(&voice.line);
                }
                self.heard.push(voice.clone());
            }
        }
        if let Some(morning) = snapshot.morning.clone() {
            if !self.mornings.contains(&morning.moment) {
                self.mornings.push(morning.moment);
                // The morning card: read, and put away with "Begin the
                // day". Its question is answered with the cards.
                if pace.reads_the_news {
                    let letter = morning.letter.map(|letter| letter.note);
                    let lines = [morning.find, morning.tended]
                        .into_iter()
                        .flatten()
                        .map(|line| line.line);
                    for line in letter.into_iter().chain(lines) {
                        self.clock += self.read(&line);
                    }
                }
                self.clock += pace.choose_seconds / 2.0;
            }
        }
        if let Some(briefing) = &snapshot.briefing {
            if briefing.returned {
                see(&snapshot, self.clock);
                // The return film: each beat told big, read, and stepped on.
                assert!(!briefing.beats().is_empty(), "a film to show");
                for beat in briefing.beats() {
                    self.clock += 1.5 + self.read(&beat.title) + self.read(&beat.detail);
                }
            } else if pace.reads_the_news {
                for item in &briefing.items {
                    self.clock += self.read(&item.title) + self.read(&item.detail);
                }
            }
        }
        see(&snapshot, self.clock);
        self.clock += pace.look_seconds;
        // A first work on one of the place's plots, before anything else
        // the hands make: it goes up in scaffolding and takes its time.
        if pace.makes && !self.plotted {
            let offer = snapshot
                .canvas
                .plots
                .iter()
                .flat_map(|plot| plot.offers.iter())
                .find(|offer| offer.unavailable.is_none())
                .cloned();
            if let Some(offer) = offer {
                self.clock += self.read(&offer.label) + pace.choose_seconds;
                if let Some(after) = self.handle(ProjectionIntent::InvokeCommand(offer.command)) {
                    self.plotted = true;
                    snapshot = after;
                    see(&snapshot, self.clock);
                }
            }
        }
        // The cards, and the deeds the hands allow: something new made,
        // planted or put up, never something taken back or moved.
        let mut asked = 0;
        loop {
            let made = &self.made;
            let makes = |command: &world_projection::ProjectionCommand| {
                pace.makes
                    && command.hand.as_ref().is_some_and(|hand| {
                        ["Build", "Plant", "Decorate"].contains(&hand.verb.as_str())
                            && !made.contains(&hand.thing)
                    })
            };
            let next = snapshot
                .commands
                .iter()
                .find(|command| {
                    command.unavailable.is_none()
                        && command.id != pass
                        && (command.question.is_some() || makes(command))
                })
                .cloned();
            let Some(command) = next.filter(|_| asked < 6) else {
                break;
            };
            asked += 1;
            if let Some(question) = &command.question {
                self.clock += self.read(&question.prompt);
                for answer in snapshot
                    .commands
                    .iter()
                    .filter(|other| other.question.as_ref() == Some(question))
                {
                    self.clock += self.read(&answer.title);
                }
            }
            self.clock += pace.choose_seconds;
            if let Some(hand) = &command.hand {
                self.made.push(hand.thing.clone());
            }
            let before = snapshot.clone();
            let Some(after) = self.handle(ProjectionIntent::InvokeCommand(command.id.clone()))
            else {
                break;
            };
            snapshot = after;
            for voice in &snapshot.voices {
                if !self.heard.contains(voice) {
                    self.clock += 1.0 + self.read(&voice.line);
                    self.heard.push(voice.clone());
                }
            }
            for keepsake in snapshot.keepsakes.iter().skip(before.keepsakes.len()) {
                self.clock += self.read(&keepsake.what) + self.read(&keepsake.note);
            }
            for letter in snapshot.letters.iter().skip(before.letters.len()) {
                self.clock += self.read(&letter.note);
            }
            see(&snapshot, self.clock);
        }
        // The favour asked, done with its offered reply.
        if pace.favours {
            if let Some(favour) = snapshot.favour.clone().filter(|favour| !favour.done) {
                self.clock += self.read(&favour.note) + pace.choose_seconds;
                if let Some(after) = self.handle(ProjectionIntent::Say {
                    to: favour.whom,
                    words: favour.reply.clone(),
                    ears: Ears::Offered,
                }) {
                    if let Some(exchange) = after.exchanges.last() {
                        self.clock += self.read(&exchange.answer);
                    }
                    snapshot = after;
                }
            }
        }
        // A word with someone, in the player's own words, heard by the
        // World's own ears (the voice is off).
        let mut people = Vec::new();
        for talk in &snapshot.talks {
            if !people.contains(&talk.who) {
                people.push(talk.who);
            }
        }
        for (turn, who) in people.into_iter().take(pace.talks).enumerate() {
            let words = ["How are you today?", "What are you working on?"][turn % 2];
            self.clock += 15.0;
            if let Some(after) = self.handle(ProjectionIntent::Say {
                to: who,
                words: words.into(),
                ears: Ears::Own,
            }) {
                if let Some(exchange) = after.exchanges.last() {
                    self.clock += self.read(&exchange.answer);
                }
                snapshot = after;
            }
        }
        see(&snapshot, self.clock);
        snapshot
    }
}
