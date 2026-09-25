use crate::model::{
    CONDITION, JONAS_HARBOR_JOB, JONAS_LEO_TRUST, OPERATING_STATUS, SUPPORT_STATUS,
};
use crate::{
    BAKERY, EMMA, EVAN, HARBOR, JONAS, JONAS_BOAT, LEO, MARA, MIA, NOAH, PUB, SCHOOL, SOFIA,
    WEDDING_ORDER,
};
use society_basic::{CASH, JOB};
use world_core::{EntityId, Event, RelationId, Value, World};
use world_projection::{
    entity_title, inspectors_from_world, timeline_from_world, why_map_from_world, BriefingItem,
    BriefingItemKind, BriefingProjection, CanvasChange, CanvasItem, CanvasItemKind,
    CanvasProjection, CollectionItem, CollectionProjection, CommandEffect, EffectChange, MarkShape,
    ProjectionCapabilities, ProjectionCommand, ProjectionSnapshot, SelectionId, Telling, Tone,
};

const RESIDENTS: [EntityId; 8] = [JONAS, MARA, LEO, EMMA, MIA, NOAH, EVAN, SOFIA];

pub(crate) fn snapshot(world: &World) -> ProjectionSnapshot {
    snapshot_since(world, None)
}

pub(crate) fn snapshot_since(
    world: &World,
    since_event_count: Option<usize>,
) -> ProjectionSnapshot {
    let mut snapshot = ProjectionSnapshot {
        title: "Tiny Society".into(),
        world_time: world.world_time(),
        capabilities: ProjectionCapabilities {
            fork: true,
            background: true,
        },
        briefing: Some(society_briefing(world, since_event_count)),
        commands: available_commands(world),
        collection: CollectionProjection {
            title: "Residents".into(),
            items: RESIDENTS
                .iter()
                .filter_map(|id| resident_item(world, *id))
                .collect(),
        },
        timeline: told_timeline(world),
        canvas: CanvasProjection {
            items: canvas_items(world)
                .into_iter()
                .map(|mut item| {
                    if let (Some(since), SelectionId::Entity(id)) = (since_event_count, item.id) {
                        item.changes = changes_since(world, since, id);
                    }
                    item
                })
                .collect(),
            links: Vec::new(),
            marks: Vec::new(),
        },
        inspectors: inspectors_from_world(world),
        why: why_map_from_world(world),
        // A small island harbour on a clear morning: sea, low hills, sun.
        scenery: Some(world_projection::Scenery {
            sky_top: 0xa9cfe6,
            sky_bottom: 0xe9f1ef,
            far: 0x7f9f8a,
            near: 0x2f6a86,
            sun: 0xffe2a0,
        }),
        calendar: Some(world_projection::Calendar {
            unit: "Day".into(),
            length: crate::persistence::WORLD_DAY_TICKS,
        }),
    };
    snapshot.tell_events_as_history_does();
    snapshot
}

/// What moved on one person, place or thing since the visit: money, work,
/// whether a place is open, what shape a boat is in.
fn changes_since(world: &World, since: usize, id: EntityId) -> Vec<CanvasChange> {
    let text = |value: &Option<Value>| match value {
        Some(Value::Integer(n)) => n.to_string(),
        Some(Value::Text(t)) => t.replace('_', " "),
        Some(Value::Bool(b)) => b.to_string(),
        _ => "—".into(),
    };
    [
        (CASH, "cash"),
        (JOB, "work"),
        (OPERATING_STATUS, ""),
        (CONDITION, ""),
    ]
    .into_iter()
    .filter_map(|(key, label)| {
        let (then, now) = world_projection::component_change_since(world, since, id, key)?;
        let tone = match (&then, &now) {
            (Some(Value::Integer(a)), Some(Value::Integer(b))) if b > a => Tone::Good,
            (Some(Value::Integer(a)), Some(Value::Integer(b))) if b < a => Tone::Warning,
            (_, Some(Value::Text(t))) if t == "closed" || t == "unemployed" || t == "damaged" => {
                Tone::Bad
            }
            (_, Some(Value::Text(t))) if t == "open" || t == "sound" => Tone::Good,
            _ => Tone::Neutral,
        };
        Some(CanvasChange {
            label: label.into(),
            before: text(&then),
            after: text(&now),
            tone,
        })
    })
    .collect()
}

/// Good news, bad news, or neither, for each kind of thing the harbour
/// reports.
fn tone_for_event(kind: &str) -> Tone {
    match kind {
        "bakery_closed"
        | "payroll_shortfall"
        | "worker_dismissed"
        | "order_lost"
        | "boat_sold"
        | "living_cost_unmet"
        | "income_disrupted"
        | "payroll_reserve_exhausted"
        | "backing_withdrawn" => Tone::Bad,
        "storm_started" | "hardship_began" | "bread_budget_cut" | "loan_requested"
        | "support_requested" | "work_sought" => Tone::Warning,
        "support_repaid"
        | "boat_repaired"
        | "bakery_reopened_lean"
        | "bakery_reopened"
        | "hardship_eased"
        | "support_received"
        | "worker_retained"
        | "jonas_taken_on"
        | "counter_help_hired"
        | "fish_sold" => Tone::Good,
        _ => Tone::Neutral,
    }
}

/// What each choice would change, for the screen to show beside it and
/// point at on the scene before it is made.
fn command_effects(command_id: &str) -> Vec<CommandEffect> {
    let effect = |target: EntityId, label: &str, change: EffectChange, tone: Tone| CommandEffect {
        target: Some(SelectionId::Entity(target)),
        label: label.into(),
        change,
        tone,
    };
    let to = |value: &str| EffectChange::To(value.into());
    match command_id {
        crate::RETAIN_WORKER_COMMAND => {
            vec![effect(JONAS, "Jonas", to("keeps his job"), Tone::Good)]
        }
        crate::REOPEN_BAKERY_COMMAND => vec![
            effect(BAKERY, "Harbor Bakery", to("open"), Tone::Good),
            effect(MARA, "Mara's savings", EffectChange::Down, Tone::Warning),
        ],
        crate::LEAN_REOPEN_BAKERY_COMMAND => vec![
            effect(BAKERY, "Harbor Bakery", to("owner-run"), Tone::Good),
            effect(MARA, "Mara's savings", EffectChange::Down, Tone::Warning),
        ],
        crate::REPAIR_BOAT_COMMAND => vec![
            effect(JONAS_BOAT, "Sea Finch", to("repaired"), Tone::Good),
            effect(LEO, "Leo's savings", EffectChange::Down, Tone::Neutral),
        ],
        crate::SELL_BOAT_COMMAND => vec![
            effect(JONAS_BOAT, "Sea Finch", to("sold for scrap"), Tone::Bad),
            effect(JONAS, "Jonas's cash", EffectChange::Up, Tone::Neutral),
        ],
        crate::TAKE_JONAS_ON_COMMAND => vec![
            effect(JONAS, "Jonas", to("works the counter"), Tone::Good),
            effect(BAKERY, "Bakery till", EffectChange::Down, Tone::Warning),
        ],
        _ => Vec::new(),
    }
}

fn available_commands(world: &World) -> Vec<ProjectionCommand> {
    let mut commands = Vec::new();
    let has_order_loss = world
        .events()
        .iter()
        .any(|event| event.kind == "order_lost");
    let has_dismissal = world
        .events()
        .iter()
        .any(|event| event.kind == "worker_dismissed");
    let has_retention = world
        .events()
        .iter()
        .any(|event| event.kind == "worker_retained");
    let jonas_is_temp = component_text(world, JONAS, JOB).as_deref() == Some("bakery_temp");

    if has_order_loss && !has_dismissal && !has_retention && jonas_is_temp {
        commands.push(ProjectionCommand {
            id: crate::RETAIN_WORKER_COMMAND.into(),
            title: "Give Jonas another chance".into(),
            detail:
                "Keep Jonas at the bakery and let this branch continue into a different future."
                    .into(),
            effects: Vec::new(),
            scenery: None,
        });
    }

    let bakery_closed =
        component_text(world, BAKERY, OPERATING_STATUS).as_deref() == Some("closed");
    let mara_can_reopen = component_integer(world, MARA, CASH)
        .is_some_and(|cash| cash >= crate::BAKERY_REOPEN_INVESTMENT);
    if bakery_closed && mara_can_reopen {
        commands.push(ProjectionCommand {
            id: crate::REOPEN_BAKERY_COMMAND.into(),
            title: "Reopen with Mara's savings".into(),
            detail: format!(
                "Invest {} of Mara's cash to reopen Harbor Bakery. Mara returns to work; former workers are not automatically rehired.",
                crate::BAKERY_REOPEN_INVESTMENT
            ), effects: Vec::new(),
            scenery: None,
});
    }

    let mara_can_reopen_lean = component_integer(world, MARA, CASH)
        .is_some_and(|cash| cash >= crate::recovery::LEAN_REOPEN_INVESTMENT);
    if bakery_closed && mara_can_reopen_lean {
        commands.push(ProjectionCommand {
            id: crate::LEAN_REOPEN_BAKERY_COMMAND.into(),
            title: "Reopen as an owner-run counter".into(),
            detail: format!(
                "Invest {} of Mara's cash and reopen Harbor Bakery without a fixed daily Bakery wage. Lower overhead can survive weak demand, but Mara gives up predictable pay.",
                crate::recovery::LEAN_REOPEN_INVESTMENT
            ), effects: Vec::new(),
            scenery: None,
});
    }

    if repair_offer_is_open(world) {
        commands.push(ProjectionCommand {
            id: crate::REPAIR_BOAT_COMMAND.into(),
            title: "Repair Sea Finch with Leo's backing".into(),
            detail: format!(
                "Leo pays Evan {} to repair Sea Finch. Jonas returns to Harbor fishing once the boat is sound. Leo's backing does not stand indefinitely.",
                crate::social::SEA_FINCH_REPAIR_COST
            ), effects: Vec::new(),
            scenery: None,
});
    }

    if crate::drift::sea_finch_can_be_sold(world.state()) {
        commands.push(ProjectionCommand {
            id: crate::SELL_BOAT_COMMAND.into(),
            title: "Sell Sea Finch for what it will fetch".into(),
            detail: format!(
                "A broken boat fetches {}, against the {} it would take to make her sound. It ends the fishing life, and it is money today.",
                crate::drift::SEA_FINCH_SCRAP_VALUE,
                crate::social::SEA_FINCH_REPAIR_COST
            ), effects: Vec::new(),
            scenery: None,
});
    }

    if crate::livelihood::work_ask_is_open(world.state()) {
        commands.push(ProjectionCommand {
            id: crate::TAKE_JONAS_ON_COMMAND.into(),
            title: "Take Jonas back at the bakery".into(),
            detail: format!(
                "Jonas works the counter for {} a day. It is a second wage against the same island trade, and the bakery has to carry it.",
                crate::livelihood::COUNTER_WAGE
            ), effects: Vec::new(),
            scenery: None,
});
    }

    for command in &mut commands {
        command.effects = command_effects(&command.id);
    }
    commands
}

/// Whether Leo's backing is on the table right now.
///
/// Drift measures its deadline against this, so the question the World answers
/// for itself is exactly the question it was offering.
pub(crate) fn repair_offer_is_open(world: &World) -> bool {
    let sea_finch_damaged =
        component_text(world, JONAS_BOAT, CONDITION).as_deref() == Some("damaged");
    let jonas_unemployed = component_text(world, JONAS, JOB).as_deref() == Some("unemployed");
    let support_received =
        component_text(world, JONAS, SUPPORT_STATUS).as_deref() == Some("received");
    let trust_ready = relation_integer(world, JONAS_LEO_TRUST, "trust")
        .is_some_and(|trust| trust >= crate::social::SEA_FINCH_REPAIR_TRUST);
    let leo_can_afford = component_integer(world, LEO, CASH)
        .is_some_and(|cash| cash >= crate::social::SEA_FINCH_REPAIR_COST);
    let harbor_job_missing = world.state().relation(JONAS_HARBOR_JOB).is_none();
    let backing_stands =
        crate::drift::backing_status(world.state()) != crate::drift::BACKING_WITHDRAWN;

    sea_finch_damaged
        && jonas_unemployed
        && support_received
        && trust_ready
        && leo_can_afford
        && harbor_job_missing
        && backing_stands
}

/// Beats a return briefing tells before it starts counting.
///
/// This was four, from when the briefing was a small block in a corner of the
/// window. Real consequence chains produce more than four notable events in a
/// long absence — a household budget cut, the demand loss it causes, a
/// payroll shortfall, a closure, and a second workplace going the same way is
/// five before anything happens to anybody by name — and a visitor who has
/// been away a fortnight is owed the story rather than its last four lines.
const BEATS_PER_BRIEFING: usize = 8;

fn society_briefing(world: &World, since_event_count: Option<usize>) -> BriefingProjection {
    let start = since_event_count.unwrap_or(0).min(world.events().len());
    let relevant_events = if since_event_count.is_some() {
        &world.events()[start..]
    } else {
        world.events()
    };

    // One line per kind of thing that happened. Once Jonas is fishing again he
    // sells a catch every single day, and three identical "Jonas's catch
    // reached the mainland" lines are a counter wearing a sentence's clothes.
    // The most recent occurrence stands for the rest; how many there were is
    // what the Status counters are for.
    let mut told = std::collections::BTreeSet::<String>::new();
    let mut items = relevant_events
        .iter()
        .rev()
        .filter_map(|event| {
            let title = narrated_title(world, event)?;
            if !told.insert(event.kind.clone()) {
                return None;
            }
            Some(BriefingItem {
                selection: Some(SelectionId::Event(event.id)),
                title,
                // The headline is the news; when it happened is the
                // history's to show, and an event number is nobody's.
                detail: String::new(),
                kind: BriefingItemKind::Beat,
                tone: tone_for_event(&event.kind),
            })
        })
        .take(BEATS_PER_BRIEFING)
        .collect::<Vec<_>>();

    // Newest first is how you pick which beats to keep; oldest first is how
    // you read them. Left newest-first, a window reported "Harbor Bakery
    // closed its doors" above "The bakery could not cover payroll" — the
    // consequence before its cause, which is a log. Turned around it is the
    // sentence the World actually wrote: the payroll failed, so the bakery
    // shut, so the school's income went with it.
    items.reverse();

    // Truncation used to be silent, and with a busier World it started losing
    // the thing a visitor most needed: a window holding a school's payroll
    // collapse and the bakery's closure dropped the household budget cut that
    // caused them, because the cut was older. If beats are left out, the
    // briefing says how many rather than pretending there were none.
    let told = items.len();
    let happened = narratable_count(world, relevant_events);
    if happened > told {
        items.push(BriefingItem {
            selection: None,
            title: format!("{} more things happened", happened - told),
            detail: "The whole history is in the timeline.".into(),
            kind: BriefingItemKind::Status,
            tone: world_projection::Tone::Neutral,
        });
    }

    // Counters travel with the briefing but are marked Status, not Beat:
    // "Harbor Bakery had customers · 40 purchases · 400 revenue" answers "was
    // anything happening at all?", never "what happened?". Keeping them here
    // costs nothing now that the distinction is carried in the projection, and
    // dropping them would throw away the one number that makes a quiet stretch
    // legible.
    if since_event_count.is_some() {
        if let Some(sales) = bakery_sales_summary(world, relevant_events) {
            items.push(sales);
        }
        if let Some(activity) = living_activity_summary(world, relevant_events) {
            items.push(activity);
        }
    }

    items.insert(0, harbor_today(world));

    if since_event_count.is_some() && items.len() == 1 {
        let (title, detail) = if relevant_events.is_empty() {
            (
                "A quiet stretch",
                "Nothing changed in the town since your last visit.".to_string(),
            )
        } else {
            (
                "The town kept working",
                match relevant_events.len() {
                    1 => "One small thing happened, and nothing stood out.".to_string(),
                    count => format!("{count} small things happened, and nothing stood out."),
                },
            )
        };
        items.push(BriefingItem {
            selection: None,
            title: title.into(),
            detail,
            kind: BriefingItemKind::Status,
            tone: world_projection::Tone::Neutral,
        });
    }

    BriefingProjection {
        eyebrow: "Society Today".into(),
        title: if since_event_count.is_some() {
            "While you were away".into()
        } else {
            "Life happened while you were away".into()
        },
        items,
    }
}

/// This World's own words for an Event, or `None` when the Event is part of
/// the background hum. One table, used both to write the beats and to count
/// how many were left out, so the two can never disagree.
///
/// It takes the World because some of these sentences name somebody, and a
/// sentence that leaves the person out reads as being about nobody while the
/// Event beside it is filed under their name: "The bakery could not cover
/// payroll" sat next to Jonas, because it was his wage, and never said so.
fn narrated_title(world: &World, event: &Event) -> Option<String> {
    if event.kind == "payroll_shortfall" {
        let worker = event
            .targets
            .first()
            .and_then(|id| world.state().entity(*id))
            .map(entity_title);
        return Some(match worker {
            Some(name) => format!("The bakery could not pay {name}"),
            None => "The bakery could not cover payroll".to_string(),
        });
    }
    Some(String::from(match event.kind.as_str() {
        "support_repaid" => "Jonas repaid Leo after returning to sea",
        "fish_sold" => "Jonas's catch reached the mainland",
        "boat_repaired" => "Sea Finch returned to the water",
        "bakery_reopened_lean" => "Mara reopened Harbor Bakery as an owner-run counter",
        "bakery_reopened" => "Mara reopened Harbor Bakery",
        "bakery_closed" => "Harbor Bakery closed its doors",
        "bread_budget_cut" if event.actor == Some(LEO) => "Leo started protecting his savings",
        "bread_budget_cut" if event.actor == Some(EMMA) => "Emma started protecting her savings",
        "income_disrupted" if event.actor == Some(LEO) => "Leo's Pub income was disrupted",
        "income_disrupted" if event.actor == Some(EMMA) => "Emma's School income was disrupted",
        "payroll_reserve_exhausted" if event.targets.contains(&PUB) => {
            "Anchor Pub exhausted its payroll reserve"
        }
        "payroll_reserve_exhausted" if event.targets.contains(&SCHOOL) => {
            "Island School exhausted its payroll reserve"
        }
        "payroll_reserve_exhausted" => "A workplace exhausted its payroll reserve",
        "payroll_shortfall" => "The bakery could not cover payroll",
        "backing_withdrawn" => "Leo put his backing elsewhere",
        "work_sought" => "Jonas asked Mara for work",
        "jonas_taken_on" if crate::drift::was_drifted(event) => {
            "Mara took Jonas back on while nobody was watching"
        }
        "jonas_taken_on" => "Mara took Jonas back on",
        "boat_sold" if crate::drift::was_drifted(event) => {
            "Jonas sold Sea Finch for scrap while nobody was watching"
        }
        "boat_sold" => "Jonas sold Sea Finch for scrap",
        "living_cost_unmet" => "Jonas could not cover his day",
        "hardship_began" => "Jonas started eating into his savings",
        "hardship_eased" => "Jonas is covering his own days again",
        "support_received" => "Leo helped Jonas stay afloat",
        "support_requested" => "Jonas asked Leo for help",
        "worker_retained" => "Mara gave Jonas another chance",
        "worker_dismissed" => "Mara dismissed Jonas",
        "order_lost" => "The bakery lost the wedding order",
        "temporary_work_assigned" => "Jonas took temporary work at the bakery",
        "loan_requested" => "Jonas asked Leo for a loan",
        "storm_started" => "A storm reached the harbor",
        "counter_help_hired" => "Mara took Mia on at the bakery counter",
        _ => return None,
    }))
}

/// History in the town's words: every headline the briefing would use, a
/// few more things that happened to somebody, and the everyday round (shifts,
/// bread, the cost of a day) folded under the moment it happened in.
fn told_timeline(world: &World) -> world_projection::TimelineProjection {
    let mut timeline = timeline_from_world(world);
    world_projection::retell_timeline(&mut timeline, world, |event| telling(world, event));
    timeline
}

fn telling(world: &World, event: &Event) -> Telling {
    if let Some(title) = narrated_title(world, event) {
        return Telling::Story(title);
    }
    let name = |id: Option<&EntityId>| {
        id.and_then(|id| world.state().entity(*id))
            .map(entity_title)
            .unwrap_or_else(|| "Someone".into())
    };
    let actor = name(event.actor.as_ref());
    let place = name(event.targets.last());
    match event.kind.as_str() {
        "boat_damaged" => Telling::Story("The storm damaged Sea Finch".into()),
        "income_lost" => Telling::Story(format!("{actor}'s income stopped")),
        "shift_missed" => Telling::Story(format!("{actor} missed a shift at {place}")),
        "catch_landed" => Telling::Routine(Some(format!("{actor} landed a catch"))),
        "work_shift_completed" => {
            Telling::Routine(Some(format!("{actor} worked a shift at {place}")))
        }
        "bread_purchased" => Telling::Routine(Some(format!("{actor} bought bread"))),
        "living_cost_paid" => Telling::Routine(Some(format!("{actor} paid for the day"))),
        "agent_decision_recorded" => Telling::Routine(Some(format!("{actor} decided"))),
        _ => Telling::Routine(None),
    }
}

/// How many of these Events the briefing would tell, before the cap. One per
/// kind, matching what the beats themselves collapse to, so "3 more things
/// happened" counts things rather than repetitions of one thing.
fn narratable_count(world: &World, events: &[Event]) -> usize {
    let mut kinds = std::collections::BTreeSet::new();
    for event in events {
        if narrated_title(world, event).is_some() {
            kinds.insert(event.kind.as_str());
        }
    }
    kinds.len()
}

/// The "what is happening now" line every briefing opens with, so a return
/// digest and a fresh visit share the same shape: state first, then changes,
/// then the commands underneath.
fn harbor_today(world: &World) -> BriefingItem {
    let bakery = match component_text(world, BAKERY, OPERATING_STATUS).as_deref() {
        Some("open") => "Harbor Bakery is open".to_string(),
        Some("closed") => "Harbor Bakery is closed".to_string(),
        _ => "Harbor Bakery".to_string(),
    };
    let bakery_cash = component_integer(world, BAKERY, CASH)
        .map(|cash| format!(" · till {cash}"))
        .unwrap_or_default();
    // After a lean reopening the bakery is one pair of hands until recovered
    // demand earns a second, so the state line says which it is.
    let counter = match component_text(world, BAKERY, crate::staffing::STAFFING_STATUS).as_deref() {
        Some("lean") => " · counter run alone".to_string(),
        Some("staffed") => " · counter shared with Mia".to_string(),
        _ => String::new(),
    };
    let jonas = component_text(world, JONAS, JOB)
        .map(|job| format!("Jonas: {}", job.replace('_', " ")))
        .unwrap_or_else(|| "Jonas".to_string());
    let jonas_cash = component_integer(world, JONAS, CASH)
        .map(|cash| format!(", cash {cash}"))
        .unwrap_or_default();
    BriefingItem {
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Entity(BAKERY)),
        title: "Harbor today".into(),
        detail: format!("{bakery}{bakery_cash}{counter} · {jonas}{jonas_cash}"),
        tone: world_projection::Tone::Neutral,
    }
}

fn bakery_sales_summary(world: &World, events: &[Event]) -> Option<BriefingItem> {
    let purchases = events
        .iter()
        .filter(|event| event.kind == "bread_purchased")
        .collect::<Vec<_>>();
    let latest = purchases.last()?;

    let mut customers = Vec::<String>::new();
    let mut total_revenue = 0_i64;
    for event in &purchases {
        if let Some(actor) = event.actor {
            if let Some(entity) = world.state().entity(actor) {
                let name = entity_title(entity);
                if !customers.contains(&name) {
                    customers.push(name);
                }
            }
        }
        if let Some(Value::Integer(amount)) = event.payload.get("amount") {
            total_revenue += amount;
        }
    }

    let people = if customers.is_empty() {
        "Residents".into()
    } else {
        customers.join(", ")
    };
    let purchase_label = if purchases.len() == 1 {
        "purchase"
    } else {
        "purchases"
    };

    Some(BriefingItem {
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(latest.id)),
        title: "Harbor Bakery had customers".into(),
        detail: format!(
            "{people} bought bread · {} {purchase_label} · {total_revenue} earned · last on day {}",
            purchases.len(),
            town_day(latest.world_time)
        ),
        tone: world_projection::Tone::Neutral,
    })
}

/// The day a moment falls on, counted the way the town's calendar counts.
fn town_day(world_time: u64) -> u64 {
    world_time
        .div_ceil(crate::persistence::WORLD_DAY_TICKS)
        .max(1)
}

fn living_activity_summary(world: &World, events: &[Event]) -> Option<BriefingItem> {
    let shifts = events
        .iter()
        .filter(|event| event.kind == "work_shift_completed")
        .collect::<Vec<_>>();
    let latest = shifts.last()?;

    let mut residents = Vec::<String>::new();
    let mut total_wages = 0_i64;
    for event in &shifts {
        if let Some(actor) = event.actor {
            if let Some(entity) = world.state().entity(actor) {
                let name = entity_title(entity);
                if !residents.contains(&name) {
                    residents.push(name);
                }
            }
        }
        if let Some(Value::Integer(wage)) = event.payload.get("wage") {
            total_wages += wage;
        }
    }

    let people = if residents.is_empty() {
        "Residents".into()
    } else {
        residents.join(", ")
    };
    let shift_label = if shifts.len() == 1 { "shift" } else { "shifts" };

    Some(BriefingItem {
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(latest.id)),
        title: "The town kept working".into(),
        detail: format!(
            "{people} worked · {} {shift_label} · {total_wages} paid in wages · last on day {}",
            shifts.len(),
            town_day(latest.world_time)
        ),
        tone: world_projection::Tone::Neutral,
    })
}

fn resident_item(world: &World, id: EntityId) -> Option<CollectionItem> {
    let entity = world.state().entity(id)?;
    let job = component_text(world, id, JOB)
        .map(|job| capitalized(&job.replace('_', " ")))
        .unwrap_or_else(|| "Unknown job".into());
    let cash = component_text(world, id, CASH).unwrap_or_else(|| "?".into());
    Some(CollectionItem {
        id: SelectionId::Entity(id),
        title: entity_title(entity),
        subtitle: format!("{job} · cash {cash}"),
    })
}

/// What each of the harbour town's places looks like on its tile.
fn place_shape(id: EntityId) -> MarkShape {
    match id {
        HARBOR => MarkShape::Tower,
        BAKERY => MarkShape::Shop,
        _ => MarkShape::House,
    }
}

fn canvas_items(world: &World) -> Vec<CanvasItem> {
    let mut items = Vec::new();

    for (id, x, y) in [
        (HARBOR, 0.08, 0.48),
        (BAKERY, 0.62, 0.18),
        (SCHOOL, 0.62, 0.66),
        (PUB, 0.28, 0.16),
    ] {
        if let Some(entity) = world.state().entity(id) {
            let detail = if id == BAKERY {
                component_text(world, BAKERY, OPERATING_STATUS)
                    .map(|status| format!("Place · {status}"))
                    .unwrap_or_else(|| "Place".into())
            } else if id == HARBOR {
                component_integer(world, HARBOR, CASH)
                    .map(|cash| format!("Place · cash {cash}"))
                    .unwrap_or_else(|| "Place".into())
            } else {
                "Place".into()
            };
            items.push(CanvasItem {
                id: SelectionId::Entity(id),
                kind: CanvasItemKind::Place,
                label: entity_title(entity),
                detail,
                x,
                y,
                changes: Vec::new(),
                shape: Some(place_shape(id)),
            });
        }
    }

    for (id, x, y) in [
        (JONAS, 0.12, 0.62),
        (MARA, 0.68, 0.32),
        (LEO, 0.34, 0.28),
        (EMMA, 0.70, 0.74),
        (MIA, 0.82, 0.67),
        (NOAH, 0.20, 0.52),
        (EVAN, 0.04, 0.72),
        (SOFIA, 0.42, 0.12),
    ] {
        if let Some(entity) = world.state().entity(id) {
            items.push(CanvasItem {
                id: SelectionId::Entity(id),
                kind: CanvasItemKind::Actor,
                label: entity_title(entity),
                detail: component_text(world, id, JOB)
                    .map(|job| job.replace('_', " "))
                    .unwrap_or_else(|| "Resident".into()),
                x,
                y,
                changes: Vec::new(),
                shape: None,
            });
        }
    }

    for (id, x, y) in [(JONAS_BOAT, 0.02, 0.42), (WEDDING_ORDER, 0.84, 0.22)] {
        if let Some(entity) = world.state().entity(id) {
            let detail = if id == JONAS_BOAT {
                component_text(world, JONAS_BOAT, CONDITION)
                    .map(|condition| format!("asset · {condition}"))
                    .unwrap_or_else(|| entity.kind.clone())
            } else {
                entity.kind.clone()
            };
            items.push(CanvasItem {
                id: SelectionId::Entity(id),
                kind: CanvasItemKind::Object,
                label: entity_title(entity),
                detail,
                x,
                y,
                changes: Vec::new(),
                shape: None,
            });
        }
    }

    items
}

fn component_text(world: &World, id: EntityId, key: &str) -> Option<String> {
    match world.state().entity(id)?.component(key)? {
        Value::Text(value) => Some(value.clone()),
        Value::Integer(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Entity(value) => world.state().entity(*value).map(entity_title),
        Value::Null | Value::List(_) | Value::Map(_) => None,
    }
}

fn component_integer(world: &World, id: EntityId, key: &str) -> Option<i64> {
    match world.state().entity(id)?.component(key)? {
        Value::Integer(value) => Some(*value),
        _ => None,
    }
}

fn relation_integer(world: &World, id: RelationId, key: &str) -> Option<i64> {
    match world.state().relation(id)?.properties.get(key)? {
        Value::Integer(value) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TinySociety;

    #[test]
    fn household_savings_cut_is_highlighted_in_return_briefing() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(70).unwrap();

        let cut_position = branch
            .world()
            .events()
            .iter()
            .position(|event| event.kind == "bread_budget_cut" && event.actor == Some(LEO))
            .expect("Leo eventually cuts his bread budget");
        let snapshot = snapshot_since(branch.world(), Some(cut_position));
        let briefing = snapshot.briefing.expect("Tiny Society has a briefing");

        assert!(briefing
            .items
            .iter()
            .any(|item| item.title == "Leo started protecting his savings"));
        assert!(briefing
            .items
            .iter()
            .any(|item| item.title == "Emma's School income was disrupted"));
    }

    #[test]
    fn every_briefing_opens_with_the_harbor_state() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let fresh = snapshot(society.world());
        let first = &fresh.briefing.expect("briefing").items[0];
        assert_eq!(first.title, "Harbor today");
        assert!(first.detail.starts_with("Harbor Bakery"));
        assert!(first.detail.contains("Jonas"));

        let quiet = snapshot_since(society.world(), Some(society.world().events().len()));
        let items = quiet.briefing.expect("briefing").items;
        assert_eq!(items[0].title, "Harbor today");
        assert_eq!(items[1].title, "A quiet stretch");
    }
}

#[cfg(test)]
mod running_out_tests {
    use super::*;
    use crate::TinySociety;

    /// Visiting the World the way the app does: a stretch of background time,
    /// then a briefing covering only that stretch.
    fn visit(branch: &mut crate::TinySocietyBranch, days: u64) -> BriefingProjection {
        let cursor = branch.visit_cursor();
        branch.advance_days(days).unwrap();
        branch
            .projection_snapshot_since(cursor)
            .briefing
            .expect("Tiny Society has a return briefing")
    }

    fn beat_titles(briefing: &BriefingProjection) -> Vec<String> {
        briefing
            .beats()
            .into_iter()
            .map(|item| item.title.clone())
            .collect()
    }

    #[test]
    fn running_out_of_money_is_something_the_visitor_is_told_about() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();

        // Jonas leaves the opening story unemployed with savings to burn. The
        // burning used to be invisible: his cash fell from 85 towards nothing
        // over twenty periods and every briefing in between said only that the
        // bakery had customers.
        let mut told = Vec::new();
        for _ in 0..8 {
            told.extend(beat_titles(&visit(&mut branch, 4)));
        }

        assert!(
            told.iter()
                .any(|title| title == "Jonas started eating into his savings"),
            "a visitor is told when Jonas starts spending savings he cannot replace, got {told:?}"
        );
        assert!(
            told.iter()
                .any(|title| title == "Jonas could not cover his day"),
            "a visitor is told when Jonas can no longer cover a day at all, got {told:?}"
        );
    }

    #[test]
    fn running_out_of_money_is_told_once_rather_than_every_day() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(120).unwrap();

        // Once per spell, not once a day, and not once ever: a man who gets a
        // wage, climbs out, and slides again has had two spells, and the
        // second is news too. What must never happen is two crossings in the
        // same direction with nothing in between.
        let mut spells = Vec::new();
        for event in branch.world().events() {
            match event.kind.as_str() {
                "hardship_began" | "living_cost_unmet" | "hardship_eased" => {
                    spells.push(event.kind.as_str())
                }
                _ => {}
            }
        }
        assert!(
            spells.len() >= 2,
            "a World this long has more than one crossing to report, got {spells:?}"
        );
        for window in spells.windows(2) {
            assert_ne!(
                window[0], window[1],
                "the same crossing was reported twice running: {spells:?}"
            );
        }
    }

    #[test]
    fn a_day_jonas_cannot_pay_for_no_longer_passes_in_silence() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(120).unwrap();

        let unmet = branch
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "living_cost_unmet")
            .expect("Jonas eventually cannot cover a day");
        // The scheduler used to return early here and record nothing at all,
        // which is why the World appeared to freeze with Jonas at 5 cash.
        assert_eq!(unmet.actor, Some(JONAS));
        assert!(matches!(
            unmet.payload.get("shortfall"),
            Some(Value::Integer(shortfall)) if *shortfall > 0
        ));
    }

    #[test]
    fn counters_are_never_told_as_news() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();

        // A stretch long enough that the bakery and the shifts both have
        // something to total up.
        let briefing = visit(&mut branch, 8);
        for title in ["Harbor today", "Harbor Bakery had customers"] {
            let item = briefing
                .items
                .iter()
                .find(|item| item.title == title)
                .unwrap_or_else(|| panic!("{title} is part of a return briefing"));
            assert_eq!(
                item.kind,
                BriefingItemKind::Status,
                "{title} totals up the routine and is not news"
            );
        }
        assert!(
            !beat_titles(&briefing).contains(&"Harbor Bakery had customers".to_string()),
            "a counter must never reach the visitor as a beat"
        );
    }

    #[test]
    fn a_thing_that_happens_every_day_is_told_once() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(10).unwrap();
        branch
            .invoke_projection_command(crate::REPAIR_BOAT_COMMAND)
            .unwrap();

        // Once Sea Finch is back in the water Jonas sells a catch every single
        // day. Three identical lines about it are a counter wearing a
        // sentence's clothes.
        let briefing = visit(&mut branch, 6);
        let titles = beat_titles(&briefing);
        let mut unique = titles.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            titles.len(),
            unique.len(),
            "each kind of thing that happened is told once, got {titles:?}"
        );
    }
}

#[cfg(test)]
mod reading_order_tests {
    use super::*;
    use crate::TinySociety;

    #[test]
    fn a_briefing_reads_forwards_even_though_it_keeps_the_newest() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        let cursor = branch.visit_cursor();
        branch.advance_days(60).unwrap();

        let briefing = branch
            .projection_snapshot_since(cursor)
            .briefing
            .expect("Tiny Society has a return briefing");
        let times = briefing
            .beats()
            .into_iter()
            .filter_map(|item| match item.selection {
                Some(SelectionId::Event(id)) => branch.world().event(id),
                _ => None,
            })
            .map(|event| (event.world_time, event.id))
            .collect::<Vec<_>>();

        assert!(
            times.len() >= 3,
            "a long absence has a story, got {times:?}"
        );
        let mut sorted = times.clone();
        sorted.sort();
        assert_eq!(
            times, sorted,
            "beats read in the order they happened, so a cause is never printed \
             below its own consequence"
        );
    }
}

#[cfg(test)]
mod naming_tests {
    use super::*;
    use crate::TinySociety;

    #[test]
    fn a_line_filed_under_somebody_says_who_it_is_about() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        for _ in 0..250 {
            if branch
                .world()
                .events()
                .iter()
                .any(|event| event.kind == "payroll_shortfall")
            {
                break;
            }
            branch.advance_days(1).unwrap();
        }

        let shortfall = branch
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "payroll_shortfall")
            .expect("the bakery eventually cannot cover a wage");
        // The Event carries no actor, so a timeline and any UI that shows one
        // fall back to the first target — the worker. The sentence has to be
        // about the same person, or the line reads as being about nobody with
        // somebody's name printed beside it.
        let worker = branch
            .world()
            .state()
            .entity(shortfall.targets[0])
            .map(entity_title)
            .expect("the shortfall names the worker it could not pay");
        let told = narrated_title(branch.world(), shortfall).expect("this is news");
        assert!(
            told.contains(&worker),
            "{told:?} is filed under {worker} and does not mention them"
        );
    }
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
