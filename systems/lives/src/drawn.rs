//! Drawn: what the player built draws someone to come and live there. A
//! bandstand draws a musician, a boathouse a boatwright, a garden a
//! beekeeper. The Pack says which work draws which trade; this System
//! brings the newcomer, some periods after the work was finished, and
//! records what drew them, so their arrival names it and their legend can
//! say so.

use super::*;

/// Set on someone who came because of a work: the work.
pub const DRAWN_BY: &str = "lives.drawn_by";
/// How many periods after a work is finished someone comes because of it.
pub const DRAWN_AFTER: u64 = 4;
/// How many may come because of what was built beyond the most the place
/// otherwise holds: a place that builds draws people it would not have.
pub const DRAWN_ROOM: usize = 1;

/// A work that could draw someone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draw {
    /// The work, as the World records it.
    pub by: EntityId,
    /// What it is, in lower case as told: "bandstand".
    pub what: String,
    /// The trade it draws, as told and in the Pack's own job word.
    pub trade: &'static str,
    pub job: &'static str,
    /// The period it was finished.
    pub since: u64,
    /// The Event that finished it.
    pub cause: Option<EventId>,
}

/// The request that brings someone a work drew.
pub fn drawn_request(draw: &Draw) -> ActionRequest {
    let mut request = ActionRequest::new("lives_drawn")
        .arg("by", Value::Entity(draw.by))
        .arg("what", draw.what.as_str())
        .arg("trade", draw.trade)
        .arg("job", draw.job);
    if let Some(cause) = draw.cause {
        request = request.caused_by(cause);
    }
    request
}

/// Who a work drew, if anyone yet: whoever came because of it.
pub fn drew(state: &WorldState, work: EntityId) -> Option<EntityId> {
    state
        .entities()
        .find(|entity| entity.component(DRAWN_BY) == Some(&Value::Entity(work)))
        .map(|entity| entity.id)
}

/// The work that drew someone, if one did.
pub fn drawn_by(state: &WorldState, person: EntityId) -> Option<EntityId> {
    match state.entity(person)?.component(DRAWN_BY)? {
        Value::Entity(work) => Some(*work),
        _ => None,
    }
}

/// What someone says on arriving, drawn by what the player built.
const ARRIVING: [&str; 8] = [
    "I heard about the {what} all the way from {origin}.",
    "The {what} needs looking after. I'm good at that.",
    "Word travels. Somebody said you'd built the {what}.",
    "I saw the {what} from the road and knew I'd stay.",
    "The {what} is why I came. Honestly.",
    "I came for the {what}, and I think I'll stay for the people.",
    "I've wanted to work somewhere like the {what} all my life.",
    "They told me about your {what}. I had to see it.",
];

fn starts_with_a_vowel(word: &str) -> bool {
    word.chars()
        .next()
        .is_some_and(|c| matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
}

/// Someone comes to live in the place because of a work.
pub(crate) struct Drawn(pub(crate) fn(&WorldState) -> Cast);

impl Action for Drawn {
    fn name(&self) -> &'static str {
        "lives_drawn"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let visitors = cast
            .visitors
            .ok_or_else(|| ActionError::Invalid("nobody comes here".into()))?;
        let by = arg_entity(request, "by")?;
        if state.entity(by).is_none() {
            return Err(ActionError::Invalid("no such work".into()));
        }
        if drew(state, by).is_some() {
            return Err(ActionError::Invalid("it drew someone already".into()));
        }
        let what = arg_text(request, "what")?.trim().to_string();
        let trade = arg_text(request, "trade")?.trim().to_string();
        let job = arg_text(request, "job")?.trim().to_string();
        if what.is_empty() || trade.is_empty() || job.is_empty() {
            return Err(ActionError::Invalid("drawn by what, to do what?".into()));
        }
        let who = next_visitor(state, &cast)
            .ok_or_else(|| ActionError::Invalid("no room for anyone".into()))?;
        let name = next_names(state, &cast, 1)
            .first()
            .copied()
            .ok_or_else(|| ActionError::Invalid("nobody left to come".into()))?;
        let seed = mix(&[who.0, by.0, 41]);
        let origin = pick(visitors.origins, seed).copied().unwrap_or("far away");
        let mut newcomer = Entity::new(who, visitors.kind).with_component("name", name);
        for (key, value) in (visitors.components)(name, &job) {
            newcomer = newcomer.with_component(key, value);
        }
        newcomer = newcomer
            .with_component("lives.newcomer", true)
            .with_component(AT, Value::Entity(by))
            .with_component(DRAWN_BY, Value::Entity(by));
        // Someone who comes to ply a trade is of an age to: grown, and
        // years from retiring.
        if let Some(kin) = cast.kin {
            let working = kin.retire_at.saturating_sub(kin.grown_at).max(2);
            let age = kin.grown_at + 1 + mix(&[who.0, 43]) % (working / 2).max(1);
            let year = kin.year.max(1) as i64;
            let now = period(state, &cast) as i64;
            newcomer = newcomer.with_component(
                generations::BORN,
                now - age as i64 * year - (mix(&[who.0, 47]) % year as u64) as i64,
            );
        }
        let fill = |line: &str| {
            line.replace("{name}", name)
                .replace("{what}", &what)
                .replace("{trade}", &trade)
                .replace("{origin}", origin)
        };
        let said = fill(pick(&ARRIVING, seed / 3).copied().unwrap_or(ARRIVING[0]));
        let told = fill(if starts_with_a_vowel(&trade) {
            "{name}, an {trade} from {origin}, came because of the {what} you built"
        } else {
            "{name}, a {trade} from {origin}, came because of the {what} you built"
        });
        let mut draft = EventDraft::new("drawn_here");
        draft.actor = Some(who);
        draft.targets = vec![who, by];
        draft.payload.insert("who".into(), Value::Entity(who));
        draft.payload.insert("drawn_by".into(), Value::Entity(by));
        draft.payload.insert("trade".into(), trade.as_str().into());
        draft.payload.insert("told".into(), told.into());
        draft.payload.insert("said".into(), said.into());
        draft.changes = vec![StateChange::CreateEntity(newcomer)];
        Ok(draft)
    }
}

/// One period of what was built drawing people: of the works finished long
/// enough ago that have drawn nobody yet, the one finished first draws
/// someone, while there is room. At most one a period.
pub fn draw_newcomers(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    draws: &[Draw],
) -> Result<Vec<EventId>, WorldError> {
    let state = world.state();
    let now = period(state, cast);
    let people = living(world, cast).len();
    if people >= cast.most_people + DRAWN_ROOM {
        return Ok(Vec::new());
    }
    let Some(draw) = draws
        .iter()
        .filter(|draw| state.entity(draw.by).is_some() && drew(state, draw.by).is_none())
        .filter(|draw| now >= draw.since + DRAWN_AFTER)
        .min_by_key(|draw| (draw.since, draw.by))
    else {
        return Ok(Vec::new());
    };
    match world.execute(actions, &drawn_request(draw)) {
        Ok(event) => Ok(vec![event.id]),
        Err(WorldError::Action(_)) => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

/// How many periods after a birth the parents ask the player to name the
/// baby.
pub const NAMING_DAYS: u64 = 3;

/// The babies born here in the last [`NAMING_DAYS`] periods, living, whom
/// `named` says the player has not named yet: whose parents would like the
/// player to choose a name.
pub fn to_be_named(
    state: &WorldState,
    cast: &Cast,
    named: impl Fn(EntityId) -> bool,
) -> Vec<EntityId> {
    let now = period(state, cast) as i64;
    generations::born_here(state, cast)
        .into_iter()
        .filter(|child| !gone(state, *child) && !named(*child))
        .filter(|child| {
            integer(state, *child, generations::BORN)
                .is_some_and(|born| (0..NAMING_DAYS as i64).contains(&(now - born)))
        })
        .collect()
}

/// The names a baby's parents propose: the one they gave, then the next
/// few of the place's names for children that nobody here has.
pub fn name_proposals(
    state: &WorldState,
    cast: &Cast,
    child: EntityId,
    count: usize,
) -> Vec<String> {
    let given = first_name(state, child);
    let Some(kin) = cast.kin else {
        return vec![given];
    };
    let taken = state
        .entities()
        .map(|entity| first_name(state, entity.id))
        .collect::<BTreeSet<_>>();
    let start = kin.names.iter().position(|name| *name == given).map_or(
        (mix(&[child.0, 5]) % kin.names.len().max(1) as u64) as usize,
        |at| at + 1,
    );
    std::iter::once(given.clone())
        .chain(
            (0..kin.names.len())
                .map(|at| kin.names[(start + at) % kin.names.len()])
                .filter(|name| *name != given && !taken.contains(*name))
                .map(str::to_string),
        )
        .take(count)
        .collect()
}
