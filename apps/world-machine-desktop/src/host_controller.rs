//! What a World window holds onto: the World's file open in a session,
//! shared by every window onto it, and the controller through which the
//! renderer's intents reach it (and the demo's limits are kept). Moved
//! out of `main.rs` as it stood.

#[allow(unused_imports)]
use super::*;

pub(crate) struct SharedDocumentState {
    pub(crate) session: DurableWorldSession,
    pub(crate) registry: Arc<world_host::WorldRegistry>,
    pub(crate) library: Arc<WorldLibrary>,
    /// This sitting (v0.30 S4): every window onto the World shares it.
    /// `None` for a World kept without sittings.
    pub(crate) sitting: Option<SittingKept>,
}

/// Where a World's sitting is kept.
#[derive(Clone)]
pub(crate) enum SittingKept {
    /// In the ledger the World's file carries, written down day by day.
    Ledger(observer::Ledger),
    /// Nowhere: the ledger could not be read. One day, all the same, never
    /// days without end.
    Unreadable,
}

pub(crate) type SharedDocument = Rc<RefCell<SharedDocumentState>>;

pub(crate) struct HostProjectionController {
    pub(crate) document: SharedDocument,
    /// What the World held when last seen, for the sound to hear a letter
    /// that a turn brought.
    pub(crate) tally: std::cell::Cell<Option<ambience::Tally>>,
    /// Set when the demo held a day's passing back: the window then
    /// shows the wishlist card (see `demo`).
    pub(crate) demo_cue: DemoCue,
}

/// What the demo asks the window to show after a turn it held back.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DemoTurn {
    /// The wishlist card: the day after the last was asked for.
    Ending,
}

pub(crate) type DemoCue = Rc<std::cell::Cell<Option<DemoTurn>>>;

impl HostProjectionController {
    pub(crate) fn new(document: SharedDocument, demo_cue: DemoCue) -> Self {
        Self {
            document,
            tally: std::cell::Cell::new(None),
            demo_cue,
        }
    }
}

/// What the demo does with `intent` in this World: the day after its last
/// is answered with the wishlist card instead (the World and its sitting
/// left as they were), and a World of a Pack it does
/// not offer is not played at all (see `demo::gate`, which fails closed).
pub(crate) fn demo_holds(
    session: &DurableWorldSession,
    intent: &world_gpui::ProjectionIntent,
) -> demo::Gate {
    if !demo::ENABLED {
        return demo::Gate::Open;
    }
    let pack = &session.pack().id;
    let snapshot = session.snapshot();
    let passes_time = match intent {
        world_gpui::ProjectionIntent::InvokeCommand(command) => {
            demo::passes_time(&snapshot, command)
        }
        // Choosing, branching or talking moves no time, but a World the
        // demo does not offer is not played at all.
        _ if !demo::offers_pack(pack) => false,
        _ => return demo::Gate::Open,
    };
    let day = snapshot
        .calendar
        .as_ref()
        .map(|calendar| demo::day_of(snapshot.world_time, calendar.length));
    demo::gate(pack, day, passes_time)
}

/// What a World shows that its sound listens for.
pub(crate) fn tally(snapshot: &world_gpui::ProjectionSnapshot) -> ambience::Tally {
    ambience::Tally {
        built: snapshot.canvas.marks.len(),
        keepsakes: snapshot.keepsakes.len(),
        letters: snapshot.letters.len(),
    }
}

impl world_gpui::ProjectionController for HostProjectionController {
    fn snapshot(&self) -> world_gpui::ProjectionSnapshot {
        let snapshot = world_gpui::i18n::localize(self.document.borrow().session.snapshot());
        self.tally.set(Some(tally(&snapshot)));
        snapshot
    }

    fn cue(&mut self, cue: world_gpui::Cue) {
        ambience::player::cue(cue);
    }

    fn story(&mut self, request: world_gpui::StoryRequest) -> Option<world_gpui::StoryPage> {
        let page = self.document.borrow().session.story(request);
        match page {
            Ok(page) => page.map(world_gpui::i18n::localize_story),
            Err(error) => {
                diagnostics::error(format!("story: {error}"));
                None
            }
        }
    }

    /// With the World voice on, the model is asked here rather than inside
    /// the Pack, so the window can wait for it without freezing: the World
    /// gives the prompt, the model is asked off the window's thread, and
    /// the words are said with its response.
    fn listen(
        &mut self,
        to: world_gpui::SelectionId,
        words: &str,
    ) -> Option<world_gpui::Listening> {
        if !world_voice::voice_on() {
            return None;
        }
        // The World says what it knows, as data; the app builds the
        // prompt itself, so no World chooses what the player's key asks.
        let hearing = self
            .document
            .borrow()
            .session
            .voice_hearing(to, words)
            .ok()??;
        Some(Box::new(move || world_voice::ask_model(&hearing)))
    }

    fn handle(
        &mut self,
        intent: world_gpui::ProjectionIntent,
    ) -> Result<world_gpui::ProjectionSnapshot, String> {
        let mut document = self.document.borrow_mut();
        match demo_holds(&document.session, &intent) {
            demo::Gate::Open => {}
            demo::Gate::Ending => {
                self.demo_cue.set(Some(DemoTurn::Ending));
                return Ok(world_gpui::i18n::localize(document.session.snapshot()));
            }
            demo::Gate::NotOffered => return Err(ui::t(demo::NOT_OFFERED).to_string()),
        }
        let registry = Arc::clone(&document.registry);
        let library = Arc::clone(&document.library);
        let is_library_world = document.session.document_id().is_some();
        let result = document
            .session
            .handle(intent, &registry, &library)
            .map_err(|error| error.to_string());
        if result.is_ok() && is_library_world {
            mark_library_changed();
        }
        let result = result.map(world_gpui::i18n::localize);
        // A letter that came with the turn is heard just after its answer.
        if let Ok(snapshot) = &result {
            let after = tally(snapshot);
            if let Some(before) = self.tally.replace(Some(after)) {
                if ambience::letter_came(before, after) {
                    ambience::player::act_later(ambience::Act::Letter, 0.6);
                }
            }
        }
        result
    }
}
