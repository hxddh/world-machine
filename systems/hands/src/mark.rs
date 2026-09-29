//! The player's mark: works built on plots along the paths, which the
//! place finishes; designs painted on what can wear one (a flag, a sail, a
//! sign, a quilt); and names given to what can be named (a boat, a work, a
//! newborn).
//!
//! A plot is somewhere the Pack keeps free for the player; what could stand
//! there is the Pack's to say. Building on one is a deed like any other: it
//! counts toward the period's deeds and costs from the purse. What was
//! built there records the plot, and stands until it is taken back.
//!
//! A design is written as `<cells>:<colours>`: 256 hex digits, each the
//! place of its colour among the design's colours, and then those colours
//! as one hex digit each, places in a fixed palette of sixteen the app
//! knows. A name is one to twenty-four characters on one line.

use world_core::{
    Action, ActionError, ActionRequest, EntityId, EventDraft, EventId, StateChange, Value, World,
    WorldError, WorldState,
};

use crate::{integer, made, name, period, text, Kit, Thing};

/// Somewhere free the player can build, and which works could stand there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plot {
    /// Stable for as long as the place is: "p7".
    pub id: String,
    /// The place it lies by, for how it is told.
    pub at: EntityId,
    /// What could be built there, by id among the kit's works.
    pub offers: Vec<&'static str>,
}

/// Which plot a work was built on.
pub const PLOT: &str = "hands.plot";
/// Set on a work while the place is still building it.
pub const BUILDING: &str = "hands.building";
/// A design painted on something: `<cells>:<colours>`.
pub const PATTERN: &str = "hands.pattern";
/// Set on something the player named.
pub const NAMED: &str = "hands.named";
/// What something was called before the player named it.
pub const WAS: &str = "hands.was";

/// A deed's key for building `work` on `plot`: `plot.<work>.<plot>`.
pub fn plot_key(work: &str, plot: &str) -> String {
    format!("plot.{work}.{plot}")
}

/// The work and plot of a plot deed's key.
pub(crate) fn parse_plot(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix("plot.")?;
    let (work, plot) = rest.split_once('.')?;
    (!work.is_empty() && !plot.is_empty() && !plot.contains('.')).then_some((work, plot))
}

/// The work a plot deed's key builds, if it is one.
pub fn plot_work(key: &str) -> Option<&str> {
    parse_plot(key).map(|(work, _)| work)
}

/// The plot a work stands on, if it was built on one.
pub fn plot_of(state: &WorldState, fixture: EntityId) -> Option<&str> {
    text(state, fixture, PLOT)
}

/// The period a work on a plot was finished, once it is: as many periods
/// after it was begun as the place takes to build.
pub fn finished_at(state: &WorldState, kit: &Kit, fixture: EntityId) -> Option<u64> {
    if plot_of(state, fixture).is_none() || building(state, fixture) {
        return None;
    }
    let since = integer(state, fixture, "hands.since")?.max(0) as u64;
    Some(since + kit.growing)
}

/// Whether a work is still being built.
pub fn building(state: &WorldState, fixture: EntityId) -> bool {
    matches!(
        state
            .entity(fixture)
            .and_then(|entity| entity.component(BUILDING)),
        Some(Value::Bool(true))
    )
}

/// Everything standing on a plot now: each plot's id and the work on it.
pub fn on_plots(state: &WorldState) -> Vec<(String, EntityId)> {
    made(state)
        .into_iter()
        .filter_map(|id| Some((plot_of(state, id)?.to_string(), id)))
        .collect()
}

/// Which works stand anywhere now, by id: each is built once.
fn standing_works(state: &WorldState) -> Vec<String> {
    on_plots(state)
        .into_iter()
        .filter_map(|(_, id)| text(state, id, "hands.thing").map(str::to_string))
        .collect()
}

pub(crate) fn work<'a>(kit: &'a Kit, id: &str) -> Option<&'a Thing> {
    kit.works.iter().find(|work| work.id == id)
}

/// What could be built on a plot now, if it is free: each work it offers
/// that does not already stand somewhere.
pub fn offers(state: &WorldState, kit: &Kit, plot: &Plot) -> Vec<&'static Thing> {
    if on_plots(state).iter().any(|(on, _)| *on == plot.id) {
        return Vec::new();
    }
    let standing = standing_works(state);
    plot.offers
        .iter()
        .filter(|id| !standing.iter().any(|done| done == *id))
        .filter_map(|id| kit.works.iter().find(|work| work.id == *id))
        .collect()
}

/// Why a work cannot be built on a plot, if it cannot: it is not offered
/// there, the plot is taken, or the work already stands.
pub(crate) fn plot_refused(
    state: &WorldState,
    kit: &Kit,
    work_id: &str,
    plot_id: &str,
) -> Result<(&'static Thing, EntityId), ActionError> {
    let plot = (kit.plots)(state)
        .into_iter()
        .find(|plot| plot.id == plot_id)
        .ok_or_else(|| ActionError::Invalid(format!("no plot {plot_id}")))?;
    if on_plots(state).iter().any(|(on, _)| *on == plot.id) {
        return Err(ActionError::Invalid(
            "something already stands there".into(),
        ));
    }
    if !plot.offers.contains(&work_id) {
        return Err(ActionError::Invalid(format!("no {work_id} there")));
    }
    if standing_works(state).iter().any(|done| done == work_id) {
        return Err(ActionError::Invalid(format!(
            "there is a {work_id} already"
        )));
    }
    let work = kit
        .works
        .iter()
        .find(|work| work.id == work_id)
        .ok_or_else(|| ActionError::Invalid(format!("no {work_id} to build")))?;
    Ok((work, plot.at))
}

/// The works the place has finished building by now and not yet said so.
fn due_finish(state: &WorldState, kit: &Kit) -> Vec<EntityId> {
    let now = period(state, kit);
    made(state)
        .into_iter()
        .filter(|id| building(state, *id))
        .filter(|id| {
            let since = integer(state, *id, "hands.since").unwrap_or(0).max(0) as u64;
            now.saturating_sub(since) >= kit.growing
        })
        .collect()
}

/// The place finishes a work the player began on a plot.
pub(crate) struct Finishes(pub(crate) fn(&WorldState) -> Kit);

impl Action for Finishes {
    fn name(&self) -> &'static str {
        "hands_finish"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let kit = (self.0)(state);
        let fixture = match request.args.get("work") {
            Some(Value::Entity(id)) => *id,
            _ => return Err(ActionError::Invalid("missing work".into())),
        };
        if !due_finish(state, &kit).contains(&fixture) {
            return Err(ActionError::Invalid("not finished yet".into()));
        }
        let at = match state
            .entity(fixture)
            .and_then(|entity| entity.component("at"))
        {
            Some(Value::Entity(at)) => name(state, *at),
            _ => "the path".into(),
        };
        let thing = text(state, fixture, "hands.thing")
            .and_then(|id| work(&kit, id))
            .map_or_else(|| name(state, fixture), |work| work.name.to_string());
        let mut draft = EventDraft::new("plot_finished");
        draft.targets = vec![fixture];
        draft.payload.insert(
            "told".into(),
            format!(
                "The {} you began by {at} was finished",
                thing.to_lowercase()
            )
            .into(),
        );
        if let Some(plot) = plot_of(state, fixture) {
            draft.payload.insert("plot".into(), plot.into());
        }
        draft.changes = vec![StateChange::RemoveComponent {
            entity: fixture,
            key: BUILDING.into(),
        }];
        Ok(draft)
    }
}

/// One period of the place building what the player began: whatever has
/// been going up long enough is finished.
pub(crate) fn finish(
    world: &mut World,
    actions: &world_core::ActionRegistry,
    kit: &Kit,
) -> Result<Vec<EventId>, WorldError> {
    let mut events = Vec::new();
    for work in due_finish(world.state(), kit) {
        let mut request = ActionRequest::new("hands_finish").arg("work", Value::Entity(work));
        if let Some(began) = world.history_index().changes_of(work).first().copied() {
            request = request.caused_by(began);
        }
        events.push(world.execute(actions, &request)?.id);
    }
    Ok(events)
}

/// A design as a World keeps it, if `text` is a whole one:
/// `<cells>:<colours>`, returned as its cells and its colours.
pub fn design_parts(text: &str) -> Option<(&str, &str)> {
    let (cells, colours) = text.split_once(':')?;
    let places = colours
        .chars()
        .map(|c| c.to_digit(16))
        .collect::<Option<Vec<_>>>()?;
    let mut seen = [false; 16];
    if places.is_empty()
        || places.len() > 8
        || places
            .iter()
            .any(|place| std::mem::replace(&mut seen[*place as usize], true))
    {
        return None;
    }
    (cells.len() == 256
        && cells.chars().all(|c| {
            c.to_digit(16)
                .is_some_and(|digit| (digit as usize) < places.len())
        }))
    .then_some((cells, colours))
}

/// A name as the player gave it, tidied, if it is one: one to twenty-four
/// characters on one line.
pub fn tidy_name(name: &str) -> Option<String> {
    let name = name.trim();
    let length = name.chars().count();
    (length > 0 && length <= 24 && !name.chars().any(char::is_control)).then(|| name.to_string())
}

/// The design painted on something, if any.
pub fn pattern_of(state: &WorldState, target: EntityId) -> Option<&str> {
    text(state, target, PATTERN)
}

/// The request that paints a design on something.
pub fn design_request(target: EntityId, design: &str) -> ActionRequest {
    ActionRequest::new("hands_design")
        .arg("target", Value::Entity(target))
        .arg("design", design)
}

/// The request that names something.
pub fn name_request(target: EntityId, name: &str) -> ActionRequest {
    ActionRequest::new("hands_name")
        .arg("target", Value::Entity(target))
        .arg("name", name)
}

fn target(request: &ActionRequest, state: &WorldState) -> Result<EntityId, ActionError> {
    match request.args.get("target") {
        Some(Value::Entity(id)) if state.entity(*id).is_some() => Ok(*id),
        _ => Err(ActionError::Invalid("nothing there".into())),
    }
}

fn argument<'a>(request: &'a ActionRequest, key: &str) -> Result<&'a str, ActionError> {
    match request.args.get(key) {
        Some(Value::Text(text)) => Ok(text.as_str()),
        _ => Err(ActionError::Invalid(format!("missing {key}"))),
    }
}

/// The player paints a design on a flag, a sail, a sign or a quilt.
pub(crate) struct Designs(pub(crate) fn(&WorldState) -> Kit);

impl Action for Designs {
    fn name(&self) -> &'static str {
        "hands_design"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let kit = (self.0)(state);
        let target = target(request, state)?;
        let wears = (kit.wears)(state, target)
            .ok_or_else(|| ActionError::Invalid("that can't wear a design".into()))?;
        let design = argument(request, "design")?.trim().to_ascii_lowercase();
        let (cells, colours) = design_parts(&design)
            .ok_or_else(|| ActionError::Invalid("not a whole design".into()))?;
        let mut draft = EventDraft::new("designed");
        draft.targets = vec![target];
        draft.payload.insert("pattern".into(), cells.into());
        draft.payload.insert("palette".into(), colours.into());
        draft.payload.insert("wears".into(), wears.into());
        draft.payload.insert(
            "told".into(),
            format!("You painted a design on the {wears}").into(),
        );
        draft.changes = vec![StateChange::SetComponent {
            entity: target,
            key: PATTERN.into(),
            value: design.clone().into(),
        }];
        Ok(draft)
    }
}

/// The player names a boat, a work or a newborn.
pub(crate) struct Names(pub(crate) fn(&WorldState) -> Kit);

impl Action for Names {
    fn name(&self) -> &'static str {
        "hands_name"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let kit = (self.0)(state);
        let target = target(request, state)?;
        let what = (kit.naming)(state, target)
            .ok_or_else(|| ActionError::Invalid("that can't be named".into()))?;
        let given = tidy_name(argument(request, "name")?).ok_or_else(|| {
            ActionError::Invalid("a name is one to twenty-four letters on one line".into())
        })?;
        let was = text(state, target, "name").unwrap_or_default().to_string();
        let mut draft = EventDraft::new("named");
        draft.targets = vec![target];
        draft.payload.insert("who".into(), Value::Entity(target));
        draft.payload.insert("name".into(), given.clone().into());
        draft.payload.insert("was".into(), was.clone().into());
        draft.payload.insert(
            "told".into(),
            format!("You named the {what} {given}").into(),
        );
        let mut changes = vec![
            StateChange::SetComponent {
                entity: target,
                key: "name".into(),
                value: given.into(),
            },
            StateChange::SetComponent {
                entity: target,
                key: NAMED.into(),
                value: true.into(),
            },
        ];
        if text(state, target, WAS).is_none() && !was.is_empty() {
            changes.push(StateChange::SetComponent {
                entity: target,
                key: WAS.into(),
                value: was.into(),
            });
        }
        draft.changes = changes;
        Ok(draft)
    }
}

/// Whether the player has named something.
pub fn named(state: &WorldState, target: EntityId) -> bool {
    matches!(
        state
            .entity(target)
            .and_then(|entity| entity.component(NAMED)),
        Some(Value::Bool(true))
    )
}

/// What something was called before the player named it, if they did.
pub fn was_called(state: &WorldState, target: EntityId) -> Option<&str> {
    text(state, target, WAS)
}

/// Whether a made thing is what the Pack builds on plots rather than an
/// ordinary deed's: it takes no room from what the player makes by hand.
pub(crate) fn on_a_plot(state: &WorldState, fixture: EntityId) -> bool {
    plot_of(state, fixture).is_some()
}
