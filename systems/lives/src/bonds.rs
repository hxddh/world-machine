//! Friendships, feuds and couples: how two people come to stand with
//! each other, and the Action that records it.

use crate::*;

/// The fewest periods a friendship or a feud stands before it can change.
pub(crate) const BOND_HOLDS: i64 = 90;
/// The fewest periods between two changes in how anyone stands with
/// anyone: each is a thing people talk about, not the weather.
pub(crate) const BOND_GAP: i64 = 6;
/// Whom someone last had words with, and over what: `"<id>:<topic>"`.
pub const ROW: &str = "lives.row";
/// The changes in how people stand of the last [`BOND_HOLDS`] periods, on
/// the notes, as `"<a>.<b>.<period>"` with `a` the lower id: kept this
/// short so a World's saves and codes stay small.
pub(crate) const BONDS_LATELY: &str = "lives.bonds_lately";

/// The changes in how people stand lately: each pair and when.
pub(crate) fn bonds_lately(state: &WorldState, cast: &Cast) -> Vec<(u64, u64, i64)> {
    match state
        .entity(cast.notes)
        .and_then(|notes| notes.component(BONDS_LATELY))
    {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| {
                let Value::Text(item) = item else {
                    return None;
                };
                let mut parts = item.split('.');
                Some((
                    parts.next()?.parse().ok()?,
                    parts.next()?.parse().ok()?,
                    parts.next()?.parse().ok()?,
                ))
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Why two people's standing changed, in words.
pub(crate) fn because(state: &WorldState, a: EntityId, b: EntityId, bond: &str) -> String {
    let row = text(state, a, ROW)
        .and_then(|row| row.split_once(':'))
        .filter(|(with, _)| *with == b.to_string())
        .map(|(_, topic)| topic.to_string());
    let (an, bn) = (first_name(state, a), first_name(state, b));
    match (bond, row) {
        ("fell_out", Some(row)) => format!("It came to a head over {row}."),
        ("fell_out", None) => format!("Too many cross words with {bn}."),
        ("made_up", Some(row)) => format!("They let {row} go at last."),
        ("made_up", None) => "Time mended it.".into(),
        ("drifted", _) => "They stopped making time for each other.".into(),
        _ => format!("{an} and {bn} kept finding each other's company."),
    }
}

pub(crate) fn bond_key(other: EntityId) -> String {
    format!("{BOND}.{other}")
}

/// Two people's standing changes: they become friends, fall out, make up.
pub(crate) struct Bonds(pub(crate) fn(&WorldState) -> Cast);

impl Action for Bonds {
    fn name(&self) -> &'static str {
        "lives_bond"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let a = arg_entity(request, "a")?;
        let b = arg_entity(request, "b")?;
        let was = text(state, a, &bond_key(b)).unwrap_or("");
        let now = standing(state, a, b, was);
        if was == now {
            return Err(ActionError::Invalid("nothing has changed".into()));
        }
        // The cast only for a bond that does change: most are asked after
        // and have not, and a Pack's cast is not free to work out.
        let cast = (self.0)(state);
        // A friendship or a feud holds a while, and the place has one
        // change of heart at a time. Partners are told as they happen.
        let today = period(state, &cast) as i64;
        if now != "partners" && was != "partners" {
            let lately = bonds_lately(state, &cast);
            let pair = (a.0.min(b.0), a.0.max(b.0));
            if lately
                .iter()
                .any(|(x, y, at)| (*x, *y) == pair && today - at < BOND_HOLDS)
            {
                return Err(ActionError::Invalid("too soon".into()));
            }
            if lately.iter().any(|(_, _, at)| today - at < BOND_GAP) {
                return Err(ActionError::Invalid("one at a time".into()));
            }
        }
        // In the place's first kind days nobody falls out or drifts apart.
        if (cast.kind)(state) && (now == "foes" || (was == "friends" && now.is_empty())) {
            return Err(ActionError::Invalid("not in the first days".into()));
        }
        // What was said lately, read only for a bond that does change.
        let heard = Heard::of(state, &cast);
        let (an, bn) = (name(state, a), name(state, b));
        let (kind, told, said) = match (was, now) {
            (_, "friends") if was == "foes" => (
                "made_up",
                format!("{an} and {bn} made it up, and more"),
                pick_line(
                    &[
                        format!("{bn} and I are thick as thieves now. Who'd have thought?"),
                        format!("Funny how things turn out. {bn}'s my best friend now."),
                        format!("Me and {bn}? Old news. We're friends."),
                    ],
                    &heard,
                    a,
                ),
            ),
            (_, "friends") => (
                "became_friends",
                format!("{an} and {bn} became firm friends"),
                pick_line(
                    &[
                        format!("{bn}'s a proper friend."),
                        format!("Me and {bn}? Friends for life."),
                        format!("{bn} gets me. Rare, that."),
                        format!("Glad I've got {bn}."),
                    ],
                    &heard,
                    a,
                ),
            ),
            (_, "foes") => (
                "fell_out",
                format!("{an} and {bn} fell out"),
                pick_line(
                    &[
                        format!("I'm done with {bn}."),
                        format!("Don't talk to me about {bn}."),
                        format!("{bn} knows what they did."),
                        format!("{bn} and I are finished."),
                    ],
                    &heard,
                    a,
                ),
            ),
            ("foes", "") => (
                "made_up",
                format!("{an} and {bn} made it up"),
                pick_line(
                    &[
                        format!("{bn} and I are all right again."),
                        format!("Made it up with {bn}. Life's too short."),
                        format!("{bn} said sorry. So did I."),
                    ],
                    &heard,
                    a,
                ),
            ),
            ("friends", "") => (
                "drifted",
                format!("{an} and {bn} drifted apart"),
                pick_line(
                    &[
                        format!("I hardly see {bn} these days."),
                        format!("{bn} and I used to be close."),
                        format!("Must look {bn} up sometime."),
                    ],
                    &heard,
                    a,
                ),
            ),
            _ => return Err(ActionError::Invalid("nothing worth telling".into())),
        };
        let said = personal(
            state,
            a,
            &said,
            &heard,
            mix(&[a.0, b.0, period(state, &cast)]),
        );
        let mut moves = Moves::default();
        remember_saying(&mut moves, &heard, &said);
        let mut draft = EventDraft::new("bond_changed");
        draft.actor = Some(a);
        draft.targets = vec![b];
        draft
            .payload
            .insert("because".into(), because(state, a, b, kind).into());
        draft.payload.insert("bond".into(), kind.into());
        draft.payload.insert("told".into(), told.into());
        draft.payload.insert("said".into(), said.into());
        draft.changes.extend(moves.changes);
        for (x, y) in [(a, b), (b, a)] {
            draft.changes.push(StateChange::SetComponent {
                entity: x,
                key: bond_key(y),
                value: now.into(),
            });
        }
        let mut lately = bonds_lately(state, &cast)
            .into_iter()
            .filter(|(_, _, at)| today - at < BOND_HOLDS)
            .map(|(x, y, at)| Value::Text(format!("{x}.{y}.{at}")))
            .collect::<Vec<_>>();
        lately.push(Value::Text(format!(
            "{}.{}.{today}",
            a.0.min(b.0),
            a.0.max(b.0)
        )));
        draft.changes.push(StateChange::SetComponent {
            entity: cast.notes,
            key: BONDS_LATELY.into(),
            value: Value::List(lately),
        });
        Ok(draft)
    }
}
