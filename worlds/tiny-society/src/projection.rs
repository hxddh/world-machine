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
    entity_title, inspectors_from_world, why_map_from_world, BriefingItem, BriefingItemKind,
    BriefingProjection, CanvasChange, CanvasItem, CanvasItemKind, CollectionItem,
    CollectionProjection, CommandEffect, EffectChange, MarkShape, ProjectionCapabilities,
    ProjectionCommand, ProjectionSnapshot, SelectionId, Telling, Tone,
};

pub(crate) fn snapshot(world: &World) -> ProjectionSnapshot {
    snapshot_since(world, None)
}

pub(crate) fn snapshot_since(
    world: &World,
    since_event_count: Option<usize>,
) -> ProjectionSnapshot {
    let commands = available_commands(world)
        .into_iter()
        .map(|mut command| {
            if command.asker.is_none() {
                command.asker = asker(&command.id);
            }
            if command.question.is_none() {
                command.question = question(&command.id);
            }
            command
        })
        .collect::<Vec<_>>();
    let talks = crate::talk::talks(world, &commands);
    let commands_on_offer = commands
        .iter()
        .filter(|command| command.unavailable.is_none())
        .map(|command| command.id.clone())
        .collect::<Vec<_>>();
    let mut snapshot = ProjectionSnapshot {
        title: "Tiny Society".into(),
        world_time: world.world_time(),
        capabilities: ProjectionCapabilities {
            fork: true,
            background: true,
            talk: true,
        },
        briefing: Some(society_briefing(world, since_event_count)),
        commands,
        collection: CollectionProjection {
            title: "Residents".into(),
            items: crate::story::people(world)
                .iter()
                .filter_map(|id| resident_item(world, *id))
                .collect(),
        },
        timeline: told_timeline(world),
        canvas: crate::town::lay_out(
            world,
            canvas_items(world)
                .into_iter()
                .map(|mut item| {
                    if let (Some(since), SelectionId::Entity(id)) = (since_event_count, item.id) {
                        item.changes = changes_since(world, since, id);
                    }
                    item
                })
                .collect(),
        ),
        inspectors: inspectors_from_world(world),
        why: why_map_from_world(world),
        // A small island harbour on a clear morning: sea, low hills, sun.
        scenery: Some(
            world_projection::Scenery {
                sky_top: 0xa9cfe6,
                sky_bottom: 0xe9f1ef,
                far: 0x7f9f8a,
                near: 0x2f6a86,
                sun: 0xffe2a0,
            }
            .in_season(crate::story::season(world) as u64),
        ),
        calendar: Some({
            let almanac = crate::almanac::almanac(world.state());
            world_projection::Calendar {
                unit: "Day".into(),
                length: crate::persistence::WORLD_DAY_TICKS,
                season: Some(calendar::season_name(world.state(), &almanac).into()),
                coming: calendar::coming_up(world.state(), &almanac, 7),
                festival_today: calendar::festival_today(world.state(), &almanac),
                year: Some(crate::almanac::YEAR_DAYS),
            }
        }),
        gauges: gauges(world),
        voices: crate::talk::voices(world),
        exchanges: exchanges(world, &commands_on_offer),
        drawings: crate::drawings::drawings_for(world),
        talks,
        goals: crate::story::goals(world),
        chapters: crate::story::chapters(world),
        weather: crate::story::weather(world),
        keepsakes: crate::life::keepsakes(world),
        letters: crate::life::letters(world),
        book: crate::book::book(world),
        moments: Vec::new(),
        almanac: None,
        almanac_years: crate::almanac_page::years(world),
        favour: conversation::favour::shown(world, &crate::speech::kit(world.state())),
    };
    // Whoever asked what was just answered goes over to what it made.
    world_projection::go_to_what_was_answered(world, &mut snapshot.canvas.items, |event| {
        matches!(crate::story::answer_words(event), Some(Some(_)))
    });
    // The harbour's moments: the latest few, and every one in the book.
    let moments = crate::moments::moments(world);
    snapshot.almanac = crate::almanac_page::new_year(world, &moments);
    snapshot.book.extend(crate::moments::book_entries(&moments));
    snapshot.moments = world_projection::latest_moments(&moments);
    snapshot.tell_events_as_history_does();
    snapshot.keep_voices_in_view();
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
fn tone_for_event(event: &Event) -> Tone {
    if let Some(tone) = crate::story::tone(event) {
        return tone;
    }
    match event.kind.as_str() {
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
        | "boat_mended_together"
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
            effect(BAKERY, "Harbour Bakery", to("open"), Tone::Good),
            effect(MARA, "Mara's savings", EffectChange::Down, Tone::Warning),
        ],
        crate::LEAN_REOPEN_BAKERY_COMMAND => vec![
            effect(BAKERY, "Harbour Bakery", to("owner-run"), Tone::Good),
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
        crate::MEND_BOAT_TOGETHER_COMMAND => vec![
            effect(JONAS_BOAT, "Sea Finch", to("mended"), Tone::Good),
            effect(JONAS, "Jonas", to("back at sea"), Tone::Good),
        ],
        crate::TAKE_JONAS_ON_COMMAND => vec![
            effect(JONAS, "Jonas", to("works the counter"), Tone::Good),
            effect(BAKERY, "Bakery till", EffectChange::Down, Tone::Warning),
        ],
        _ => Vec::new(),
    }
}

/// The question a pair of the town's own choices answer together.
fn question(command_id: &str) -> Option<world_projection::Question> {
    let (id, prompt) = match command_id {
        crate::REOPEN_BAKERY_COMMAND | crate::LEAN_REOPEN_BAKERY_COMMAND => {
            ("reopen", "The bakery's shut. How do I open again?")
        }
        crate::REPAIR_BOAT_COMMAND
        | crate::SELL_BOAT_COMMAND
        | crate::MEND_BOAT_TOGETHER_COMMAND => ("sea_finch", "What becomes of Sea Finch?"),
        _ => return None,
    };
    Some(world_projection::Question {
        id: id.into(),
        prompt: prompt.into(),
    })
}

/// Whose choice it is: Jonas's for his job and his boat, Mara's for her
/// bakery.
fn asker(command_id: &str) -> Option<SelectionId> {
    let who = match command_id {
        crate::RETAIN_WORKER_COMMAND
        | crate::REPAIR_BOAT_COMMAND
        | crate::SELL_BOAT_COMMAND
        | crate::MEND_BOAT_TOGETHER_COMMAND
        | crate::TAKE_JONAS_ON_COMMAND => JONAS,
        crate::REOPEN_BAKERY_COMMAND | crate::LEAN_REOPEN_BAKERY_COMMAND => MARA,
        _ => return None,
    };
    Some(SelectionId::Entity(who))
}

pub(crate) fn available_commands(world: &World) -> Vec<ProjectionCommand> {
    let mut commands = Vec::new();
    // Asked of the World's index of its history, not by reading all of it.
    let index = world.history_index();
    let has_order_loss = !index.of_kind("order_lost").is_empty();
    let has_dismissal = !index.of_kind("worker_dismissed").is_empty();
    let has_retention = !index.of_kind("worker_retained").is_empty();
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
            asker: None,
            moves: Vec::new(),
            question: None,
            unavailable: None,
            hand: None,
            preview: None,
        });
    }

    // Mara asks how to open again for a couple of days after a closure, and
    // not again within two months; otherwise she settles it herself.
    let bakery_closed = component_text(world, BAKERY, OPERATING_STATUS).as_deref()
        == Some("closed")
        && crate::drift::reopening_is_asked(world);
    let mara_can_reopen = component_integer(world, MARA, CASH)
        .is_some_and(|cash| cash >= crate::BAKERY_REOPEN_INVESTMENT);
    if bakery_closed && mara_can_reopen {
        commands.push(ProjectionCommand {
            id: crate::REOPEN_BAKERY_COMMAND.into(),
            title: "Reopen with Mara's savings".into(),
            detail: format!(
                "Mara puts {} of her savings into Harbour Bakery and goes back to work. The old staff aren't rehired.",
                crate::BAKERY_REOPEN_INVESTMENT
            ), effects: Vec::new(),
            scenery: None, asker: None, moves: Vec::new(), question: None, unavailable: None, hand: None,
    preview: None,
});
    }

    let mara_can_reopen_lean = component_integer(world, MARA, CASH)
        .is_some_and(|cash| cash >= crate::recovery::LEAN_REOPEN_INVESTMENT);
    if bakery_closed && mara_can_reopen_lean {
        commands.push(ProjectionCommand {
            id: crate::LEAN_REOPEN_BAKERY_COMMAND.into(),
            title: "Reopen as an owner-run counter".into(),
            detail: format!(
                "Mara puts in {} and runs the counter herself: cheap to keep open, but no steady wage.",
                crate::recovery::LEAN_REOPEN_INVESTMENT
            ), effects: Vec::new(),
            scenery: None, asker: None, moves: Vec::new(), question: None, unavailable: None, hand: None,
    preview: None,
});
    }

    if repair_offer_is_open(world) {
        commands.push(ProjectionCommand {
            id: crate::REPAIR_BOAT_COMMAND.into(),
            title: "Repair Sea Finch with Leo's backing".into(),
            detail: format!(
                "Leo pays Evan {} to mend Sea Finch, and Jonas goes back to fishing. Leo won't wait forever.",
                crate::social::SEA_FINCH_REPAIR_COST
            ), effects: Vec::new(),
            scenery: None, asker: None, moves: Vec::new(), question: None, unavailable: None, hand: None,
    preview: None,
});
    }

    if crate::drift::sea_finch_can_be_sold(world.state()) {
        commands.push(ProjectionCommand {
            id: crate::MEND_BOAT_TOGETHER_COMMAND.into(),
            title: "Mend her together".into(),
            detail: "Evan shows everyone how. No money changes hands, and it takes a week of everyone's evenings. Jonas goes back to sea.".into(),
            effects: Vec::new(),
            scenery: None, asker: None, moves: Vec::new(), question: None, unavailable: None, hand: None,
            preview: None,
        });
        commands.push(ProjectionCommand {
            id: crate::SELL_BOAT_COMMAND.into(),
            title: "Sell Sea Finch for what it will fetch".into(),
            detail: format!(
                "A broken boat fetches {}, against the {} it would take to make her sound. It ends the fishing life, and it is money today.",
                crate::drift::SEA_FINCH_SCRAP_VALUE,
                crate::social::SEA_FINCH_REPAIR_COST
            ), effects: Vec::new(),
            scenery: None, asker: None, moves: Vec::new(), question: None, unavailable: None, hand: None,
    preview: None,
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
            scenery: None, asker: None, moves: Vec::new(), question: None, unavailable: None, hand: None,
    preview: None,
});
    }

    for command in &mut commands {
        command.effects = command_effects(&command.id);
    }
    commands.extend(crate::story::commands(world));
    commands.push(ProjectionCommand {
        id: crate::story::WAIT_COMMAND.into(),
        title: "Let the day pass".into(),
        detail: "Nothing is decided. The harbour lives its day.".into(),
        effects: Vec::new(),
        scenery: None,
        asker: None,
        moves: Vec::new(),
        question: None,
        unavailable: None,
        hand: None,
        preview: None,
    });
    // What the player can do with their own hands comes after every
    // card; a screen offers it apart from them.
    commands.extend(crate::handwork::commands(world));
    commands.extend(crate::life::suggestions(world));
    // A newborn's parents ask the player to choose a name, last of all.
    commands.extend(crate::plots::naming_cards(world));
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
    briefing_from(world, since_event_count, true)
}

/// The briefing, found through the World's index of its history when
/// `indexed`, else by reading it event by event, as a reference for tests:
/// the two are the same briefing.
pub(crate) fn briefing_from(
    world: &World,
    since_event_count: Option<usize>,
    indexed: bool,
) -> BriefingProjection {
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
    // A plot cleared is somewhere new to build, not news of the harbour:
    // it fills what room is left, like the storyteller's small moments.
    let everyday = |event: &Event| crate::story::is_storylet(event) || event.kind == "plot_cleared";
    let beat = |event: &Event, title: String| BriefingItem {
        selection: Some(SelectionId::Event(event.id)),
        title,
        // The headline is the news; when it happened is the history's to
        // show, and an event number is nobody's.
        detail: String::new(),
        kind: BriefingItemKind::Beat,
        tone: tone_for_event(event),
    };
    // Over the whole history, the town's own beats are each kind's newest
    // telling, found through the World's index of its history; with enough
    // of them nothing older is read, however long the World has lived.
    let own = (indexed && since_event_count.is_none()).then(|| own_beats(world, &everyday));
    let mut beats = Vec::new();
    match &own {
        Some(own) if own.len() >= BEATS_PER_BRIEFING => {
            for (event, title) in own.iter().take(BEATS_PER_BRIEFING) {
                beats.push((*event, beat(event, title.clone())));
            }
        }
        _ => {
            // Newest first, only as far back as it takes to find the beats
            // kept: once the town's own life has filled the briefing, or
            // every one of its beats and the storyteller's that fill the
            // room left are found, nothing older would be kept.
            let own_total = own.as_ref().map(Vec::len);
            // The storylets whose questions were settled in this stretch.
            let settled = relevant_events
                .iter()
                .filter(|event| event.kind != "situation_arose")
                .map(storylet_of)
                .filter(|storylet| storylet.is_some())
                .collect::<std::collections::BTreeSet<_>>();
            let (mut own_beats, mut everyday_beats) = (0, 0);
            for event in relevant_events.iter().rev() {
                if own_beats >= BEATS_PER_BRIEFING
                    || own_total.is_some_and(|total| {
                        own_beats >= total && everyday_beats >= BEATS_PER_BRIEFING - total
                    })
                {
                    break;
                }
                // A question still waiting for the player is asked when
                // they are back, and one already settled is told by how it
                // went: the film does not tell either as it came up.
                if since_event_count.is_some()
                    && (still_asking(world, event)
                        || (event.kind == "situation_arose"
                            && settled.contains(&storylet_of(event)))
                        || own_work_going_on(event))
                {
                    continue;
                }
                let Some(title) = narrated_title(world, event) else {
                    continue;
                };
                // People's lives are told one line each; the town's
                // machinery one line a kind.
                let told_as = if lives::is_news(event) {
                    title.clone()
                } else {
                    event.kind.clone()
                };
                if !told.insert(told_as) {
                    continue;
                }
                if everyday(event) {
                    everyday_beats += 1;
                } else {
                    own_beats += 1;
                }
                beats.push((event, beat(event, title)));
            }
        }
    }
    // What the town's own life did comes first; the small asks and turns
    // of the storyteller fill whatever room is left, so a fortnight of
    // wants and birthdays never crowds out the bakery closing.
    let mut kept = beats
        .iter()
        .filter(|(event, _)| !everyday(event))
        .take(BEATS_PER_BRIEFING)
        .map(|(event, _)| event.id)
        .collect::<Vec<_>>();
    let room = BEATS_PER_BRIEFING - kept.len();
    kept.extend(
        beats
            .iter()
            .filter(|(event, _)| everyday(event))
            .take(room)
            .map(|(event, _)| event.id),
    );
    let mut items = beats
        .into_iter()
        .filter(|(event, _)| kept.contains(&event.id))
        .map(|(_, item)| item)
        .collect::<Vec<_>>();

    // Newest first is how you pick which beats to keep; oldest first is how
    // you read them. Left newest-first, a window reported "Harbour Bakery
    // closed its doors" above "The bakery couldn't find this week's wages" — the
    // consequence before its cause, which is a log. Turned around it is the
    // sentence the World actually wrote: the payroll failed, so the bakery
    // shut, so the school's income went with it.
    items.reverse();

    // Truncation used to be silent, and with a busier World it started losing
    // the thing a visitor most needed: a window holding a school's payroll
    // collapse and the bakery's closure dropped the household budget cut that
    // caused them, because the cut was older. If beats are left out, the
    // briefing says how many rather than pretending there were none.
    // What someone left or wrote the player while they were away closes
    // the story of the return.
    if let Some(left) = relevant_events
        .iter()
        .rev()
        .find(|event| matches!(event.kind.as_str(), "keepsake_left" | "letter_written"))
    {
        if let Some(title) = lives::told(left) {
            // What the note tells is not told again beside it.
            let note = lives::said(left)
                .map(|(_, note)| note.to_lowercase())
                .unwrap_or_default();
            items.retain(|item| {
                let told = item.title.trim_end_matches('.').to_lowercase();
                told.len() < 12 || !note.contains(&told)
            });
            // In its place in time, so nothing reads before its cause.
            let when = |item: &BriefingItem| match item.selection {
                Some(SelectionId::Event(id)) => world.event(id).map(|event| event.world_time),
                _ => None,
            };
            let at = items
                .iter()
                .position(|item| when(item).is_some_and(|time| time > left.world_time))
                .unwrap_or(items.len());
            items.insert(
                at,
                BriefingItem {
                    selection: Some(SelectionId::Event(left.id)),
                    title,
                    detail: lives::said(left).map(|(_, note)| note).unwrap_or_default(),
                    kind: BriefingItemKind::Beat,
                    tone: world_projection::Tone::Good,
                },
            );
        }
    }
    let told = items.len();
    let happened = if indexed {
        narratable_count(world, relevant_events)
    } else {
        narratable_count_of(world, relevant_events)
    };
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
    // "Harbour Bakery had customers · 40 purchases · 400 revenue" answers "was
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
            // Not a return: a newcomer has not been away, and a World
            // opened again tells its story whole.
            "The story so far".into()
        },
        items,
        returned: since_event_count.is_some(),
    }
}

/// The storylet an Event belongs to, if any.
fn storylet_of(event: &Event) -> Option<&str> {
    match event.payload.get("storylet") {
        Some(Value::Text(id)) => Some(id.as_str()),
        _ => None,
    }
}

/// Whether `event` is the harbour getting on with one of its own works by
/// itself: told by the scaffolding on the scene, and by the card when the
/// next part is asked, not by the film.
fn own_work_going_on(event: &Event) -> bool {
    storylet_of(event).is_some_and(|storylet| storylet.starts_with("own_"))
        && (event.kind.ends_with("_by_hand") || event.kind.ends_with("_went_on"))
}

/// Whether `event` raised a question the player has not answered yet.
fn still_asking(world: &World, event: &Event) -> bool {
    event.kind == "situation_arose"
        && match event.payload.get("storylet") {
            Some(Value::Text(id)) => {
                storylets::opened_at(world.state(), crate::story::deck(), id).is_some()
            }
            _ => false,
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
pub(crate) fn narrated_title(world: &World, event: &Event) -> Option<String> {
    if let Some(title) = crate::story::told(world, event) {
        return Some(title);
    }
    if event.kind == "payroll_shortfall" {
        let worker = event
            .targets
            .first()
            .and_then(|id| world.state().entity(*id))
            .map(entity_title);
        return Some(match worker {
            Some(name) => format!("The bakery could not pay {name}"),
            None => "The bakery couldn't find this week's wages".to_string(),
        });
    }
    Some(String::from(match event.kind.as_str() {
        "support_repaid" => "Jonas repaid Leo after returning to sea",
        "fish_sold" => "Jonas's catch reached the mainland",
        "boat_repaired" => "Sea Finch returned to the water",
        "bakery_reopened_lean" => "Mara reopened Harbour Bakery as an owner-run counter",
        "bakery_reopened" => "Mara reopened Harbour Bakery",
        "bakery_closed" => "Harbour Bakery closed its doors",
        "bread_budget_cut" if event.actor == Some(LEO) => "Leo started protecting his savings",
        "bread_budget_cut" if event.actor == Some(EMMA) => "Emma started protecting her savings",
        "income_disrupted" if event.actor == Some(LEO) => "Leo's takings at the pub took a knock",
        "income_disrupted" if event.actor == Some(EMMA) => {
            "Emma's pay from the school took a knock"
        }
        "payroll_reserve_exhausted" if event.targets.contains(&PUB) => {
            "The Anchor's wage tin ran dry"
        }
        "payroll_reserve_exhausted" if event.targets.contains(&SCHOOL) => {
            "The school's wage tin ran dry"
        }
        "payroll_reserve_exhausted" => "A wage tin in the harbour ran dry",
        "payroll_shortfall" => "The bakery couldn't find this week's wages",
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
        "boat_mended_together" => "The whole harbour mended Sea Finch together",
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
        "storm_started" => "A storm reached the harbour",
        "counter_help_hired" => "Mara took Mia on at the bakery counter",
        _ => return None,
    }))
}

/// History in the town's words: every headline the briefing would use, a
/// few more things that happened to somebody, and the everyday round (shifts,
/// bread, the cost of a day) folded under the moment it happened in.
fn told_timeline(world: &World) -> world_projection::TimelineProjection {
    // Everyday life is told as it happens, in what people say; History
    // keeps to what changed, and to today's, which today's words point at.
    let now = world.world_time();
    let mut timeline = world_projection::timeline_of(world, |event| {
        event.world_time == now
            || !matches!(
                event.kind.as_str(),
                "lived" | "life_began" | "lines_forgotten"
            )
    });
    world_projection::retell_timeline(&mut timeline, world, |event| telling(world, event));
    timeline
}

/// What the player said to people today, and what they answered; a
/// request is offered only while its choice still is.
fn exchanges(world: &World, on_offer: &[String]) -> Vec<world_projection::Exchange> {
    conversation::exchanges_today(world)
        .into_iter()
        .map(|exchange| world_projection::Exchange {
            who: SelectionId::Entity(exchange.who),
            words: exchange.words,
            answer: exchange.reply,
            moment: SelectionId::Event(exchange.event),
            asks_for: exchange
                .asks_for
                .filter(|command| on_offer.contains(command)),
        })
        .collect()
}

fn telling(world: &World, event: &Event) -> Telling {
    if let Some(told) =
        conversation::told(event).or_else(|| conversation::favour::told(world.state(), event))
    {
        return Telling::Routine(Some(told));
    }
    if event.kind == "festival_nears" {
        return Telling::Routine(calendar::told(event));
    }
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
        "fixture_passed" => Telling::Routine(match event.payload.get("name") {
            Some(Value::Text(name)) => Some(format!("{name} came down")),
            _ => None,
        }),
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
    // Over the whole history, each kind that can be told is looked up in
    // the World's index of its history, newest first, so a long-lived
    // World is not read event by event.
    if events.len() == world.events().len() {
        let index = world.history_index();
        return narrated_kinds()
            .iter()
            .filter(|kind| {
                index
                    .of_kind(kind)
                    .iter()
                    .rev()
                    .filter_map(|id| world.event(*id))
                    .any(|event| narrated_title(world, event).is_some())
            })
            .count();
    }
    narratable_count_of(world, events)
}

/// How many kinds of the `events` can be told, event by event.
fn narratable_count_of(world: &World, events: &[Event]) -> usize {
    let mut kinds = std::collections::BTreeSet::new();
    for event in events {
        // A kind already counted needs no more of its events told.
        if !kinds.contains(event.kind.as_str()) && narrated_title(world, event).is_some() {
            kinds.insert(event.kind.as_str());
        }
    }
    kinds.len()
}

/// The town's own beats over the whole history, newest first: of each kind
/// that is not news, its newest telling, when that is not one of the
/// storyteller's everyday moments. A kind is told once, by its newest
/// telling, so an older one of the same kind would never be a beat.
fn own_beats<'a>(world: &'a World, everyday: &dyn Fn(&Event) -> bool) -> Vec<(&'a Event, String)> {
    let index = world.history_index();
    let mut own = narrated_kinds()
        .iter()
        .filter(|kind| !lives::NEWS_KINDS.contains(kind))
        .filter_map(|kind| {
            index
                .of_kind(kind)
                .iter()
                .rev()
                .filter_map(|id| world.event(*id))
                .find_map(|event| Some((event, narrated_title(world, event)?)))
        })
        .filter(|(event, _)| !everyday(event))
        .collect::<Vec<_>>();
    own.sort_by_key(|(event, _)| std::cmp::Reverse(event.id));
    own
}

/// Every kind of Event [`narrated_title`] can tell: what the storyteller,
/// lives, hands and the calendar tell, and the town's own table. Any other
/// kind is the background hum.
fn narrated_kinds() -> &'static std::collections::BTreeSet<&'static str> {
    static KINDS: std::sync::OnceLock<std::collections::BTreeSet<&'static str>> =
        std::sync::OnceLock::new();
    KINDS.get_or_init(|| {
        // hands::is_hands, and the calendar's one day told.
        const HANDS: [&str; 13] = [
            "plot_cleared",
            "built_by_hand",
            "decorated_by_hand",
            "planted_by_hand",
            "moved_by_hand",
            "gift_given",
            "invited_out",
            "plant_grew",
            "undone_by_hand",
            "enjoyed",
            "plot_finished",
            "designed",
            "named",
        ];
        const TABLE: [&str; 32] = [
            "chapter_ended",
            "boat_mended_together",
            "hand_lent",
            "festival_held",
            "payroll_shortfall",
            "support_repaid",
            "fish_sold",
            "boat_repaired",
            "bakery_reopened_lean",
            "bakery_reopened",
            "bakery_closed",
            "bread_budget_cut",
            "income_disrupted",
            "payroll_reserve_exhausted",
            "backing_withdrawn",
            "work_sought",
            "jonas_taken_on",
            "boat_sold",
            "living_cost_unmet",
            "hardship_began",
            "hardship_eased",
            "support_received",
            "support_requested",
            "worker_retained",
            "worker_dismissed",
            "order_lost",
            "temporary_work_assigned",
            "loan_requested",
            "storm_started",
            "counter_help_hired",
            "situation_arose",
            // Being greeted is the first thing that happens to a newcomer.
            "greeted",
        ];
        HANDS
            .into_iter()
            .chain(TABLE)
            .chain(lives::NEWS_KINDS)
            .chain(crate::story::storylet_kinds())
            .collect()
    })
}

/// The "what is happening now" line every briefing opens with, so a return
/// digest and a fresh visit share the same shape: state first, then changes,
/// then the commands underneath.
fn harbor_today(world: &World) -> BriefingItem {
    let bakery = match component_text(world, BAKERY, OPERATING_STATUS).as_deref() {
        Some("open") => "Harbour Bakery is open".to_string(),
        Some("closed") => "Harbour Bakery is closed".to_string(),
        _ => "Harbour Bakery".to_string(),
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
        title: "Harbour today".into(),
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
        title: "Harbour Bakery had customers".into(),
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
    let askers = crate::speech::askers(world);
    let mut items = Vec::new();

    for (id, x, y) in [
        (HARBOR, 0.08, 0.48),
        (BAKERY, 0.62, 0.18),
        (SCHOOL, 0.62, 0.66),
        (PUB, 0.28, 0.16),
    ] {
        if let Some(entity) = world.state().entity(id) {
            // How it is, in the player's words: never what the World keeps
            // it as.
            let detail = if id == BAKERY {
                match component_text(world, BAKERY, OPERATING_STATUS).as_deref() {
                    Some("closed") => "Closed for now".to_string(),
                    _ => "Open".to_string(),
                }
            } else if id == HARBOR {
                component_integer(world, HARBOR, CASH)
                    .map(|cash| format!("The harbour fund holds {cash}"))
                    .unwrap_or_default()
            } else if id == SCHOOL {
                "The school on the hill".into()
            } else {
                "The pub on the square".into()
            };
            // "the harbour" in a sentence, "The harbour" on its label.
            let label = entity_title(entity);
            let mut letters = label.chars();
            let label = letters
                .next()
                .map(|first| first.to_uppercase().chain(letters).collect())
                .unwrap_or(label);
            items.push(CanvasItem {
                id: SelectionId::Entity(id),
                kind: CanvasItemKind::Place,
                label,
                detail,
                x,
                y,
                changes: Vec::new(),
                shape: Some(place_shape(id)),
                at: None,
                look: None,
                drawing: crate::drawings::drawing_of(id, false),
                stance: None,
                standing: None,
                mood: None,
                spot: None,
                px: None,
                home: None,
                day: Vec::new(),
                built: None,
                ..Default::default()
            });
        }
    }

    let living = crate::story::people(world);
    for (id, x, y) in [
        (JONAS, 0.12, 0.62),
        (MARA, 0.68, 0.32),
        (LEO, 0.34, 0.28),
        (EMMA, 0.70, 0.74),
        (MIA, 0.82, 0.67),
        (NOAH, 0.20, 0.52),
        (EVAN, 0.04, 0.72),
        (SOFIA, 0.42, 0.12),
        (crate::story::ADA, 0.36, 0.2),
        (crate::story::IVO, 0.1, 0.6),
    ] {
        if !living.contains(&id) {
            continue;
        }
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
                at: lives::at(world.state(), id)
                    .or_else(|| workplace(world, id))
                    .or_else(|| match entity.component("location") {
                        Some(Value::Entity(place)) => Some(*place),
                        _ => None,
                    })
                    .map(SelectionId::Entity),
                look: crate::kin::look(world.state(), id),
                drawing: crate::drawings::drawing_of(id, true),
                stance: crate::drawings::stance_of(world, id, workplace(world, id)),
                standing: crate::speech::standing_of(world, id),
                mood: crate::speech::mood_of(world, id, &askers),
                spot: None,
                px: None,
                home: None,
                day: Vec::new(),
                built: None,
                ..Default::default()
            });
        }
    }

    // Strangers who came to stay, where they spend their days.
    let children = lives::children(world.state(), &crate::life::cast());
    for (index, id) in lives::arrivals(world.state(), &crate::life::cast())
        .into_iter()
        .chain(lives::grown_here(world.state(), &crate::life::cast()))
        .chain(children.iter().copied())
        .enumerate()
    {
        if !living.contains(&id) && !children.contains(&id) {
            continue;
        }
        if let Some(entity) = world.state().entity(id) {
            items.push(CanvasItem {
                id: SelectionId::Entity(id),
                kind: CanvasItemKind::Actor,
                label: entity_title(entity),
                detail: component_text(world, id, JOB)
                    .map(|job| job.replace('_', " "))
                    .unwrap_or_else(|| "Resident".into()),
                x: 0.15 + 0.07 * (index % 10) as f32,
                y: 0.4 + 0.05 * (index % 3) as f32,
                changes: Vec::new(),
                shape: None,
                at: lives::at(world.state(), id)
                    .or_else(|| crate::life::work(world.state(), id))
                    .map(SelectionId::Entity),
                look: crate::kin::look(world.state(), id),
                drawing: crate::drawings::drawing_of(id, true),
                stance: crate::drawings::stance_of(world, id, crate::life::work(world.state(), id)),
                standing: crate::speech::standing_of(world, id),
                mood: crate::speech::mood_of(world, id, &askers),
                spot: None,
                px: None,
                home: None,
                day: Vec::new(),
                built: None,
                ..Default::default()
            });
        }
    }

    for (id, x, y) in [(JONAS_BOAT, 0.02, 0.42), (WEDDING_ORDER, 0.84, 0.22)] {
        if let Some(entity) = world.state().entity(id) {
            // How it is, in the player's words: never what the World
            // keeps it as.
            let detail = if id == JONAS_BOAT {
                match component_text(world, JONAS_BOAT, CONDITION).as_deref() {
                    Some("sound") => "A fishing boat in good repair",
                    Some("damaged") => "A fishing boat, damaged in the storm",
                    Some("sold") => "Sold for scrap",
                    _ => "A fishing boat",
                }
            } else {
                "Bread for a wedding"
            }
            .to_string();
            items.push(CanvasItem {
                id: SelectionId::Entity(id),
                kind: CanvasItemKind::Object,
                label: entity_title(entity),
                detail,
                x,
                y,
                changes: Vec::new(),
                shape: Some(if id == JONAS_BOAT {
                    MarkShape::Boat
                } else {
                    MarkShape::Parcel
                }),
                // The boat is moored at the harbour; the order waits at
                // the bakery that has to fill it.
                at: Some(SelectionId::Entity(if id == JONAS_BOAT {
                    HARBOR
                } else {
                    BAKERY
                })),
                look: None,
                drawing: None,
                stance: None,
                standing: None,
                mood: None,
                spot: None,
                px: None,
                home: None,
                day: Vec::new(),
                built: None,
                ..Default::default()
            });
        }
    }

    items.extend(crate::story::fixtures(world));
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

/// Where someone works, which is where they are found: the place their
/// job ties them to, if they have one.
fn workplace(world: &World, person: EntityId) -> Option<EntityId> {
    world
        .state()
        .relations()
        .find(|relation| relation.kind == "works_at" && relation.from == person)
        .map(|relation| relation.to)
        .filter(|place| world.state().entity(*place).is_some())
}

/// What the harbour town keeps score of: how many of its working people
/// have work, how much money its people hold against what they started
/// with, and whether the bakery at its heart is open.
pub(crate) fn gauges(world: &World) -> Vec<world_projection::Gauge> {
    use world_projection::Gauge;
    let people = crate::story::people(world);
    let workforce = people
        .iter()
        .filter(|id| component_text(world, **id, JOB).as_deref() != Some("student"))
        .count();
    let out_of_work = people
        .iter()
        .filter(|id| component_text(world, **id, JOB).as_deref() == Some("unemployed"))
        .count();
    let working = workforce - out_of_work;
    // The whole town's money, its people's and its places': wages and bread
    // only move it about, and what comes and goes is the mainland trade.
    let town = people.iter().chain(&[HARBOR, BAKERY, SCHOOL, PUB]);
    let money: i64 = town
        .clone()
        .filter_map(|id| component_integer(world, *id, CASH))
        .sum();
    let started_with: i64 = town
        .filter_map(
            |id| match world_projection::component_at(world, 0, *id, CASH) {
                Some(Value::Integer(cash)) => Some(cash),
                _ => None,
            },
        )
        .sum();
    let started_with = started_with.max(1);
    vec![
        Gauge {
            id: "work".into(),
            label: "In work".into(),
            value: working as f32 / workforce.max(1) as f32,
            reading: format!("{working} of {workforce}"),
            tone: match out_of_work {
                0 => Tone::Good,
                1 => Tone::Warning,
                _ => Tone::Bad,
            },
        },
        Gauge {
            id: "money".into(),
            label: "Money in town".into(),
            // Half full is what the town started with; empty is 40% less,
            // and full 40% more.
            value: ((money as f32 / started_with as f32 - 0.6) / 0.8).clamp(0.0, 1.0),
            reading: with_thousands(money),
            tone: if money * 10 < started_with * 7 {
                Tone::Bad
            } else if money < started_with {
                Tone::Warning
            } else {
                Tone::Good
            },
        },
        Gauge {
            id: "spirits".into(),
            label: "Spirits".into(),
            value: (crate::story::spirits(world) + 5) as f32 / 10.0,
            reading: match crate::story::spirits(world) {
                4.. => "High",
                2..=3 => "Good",
                0..=1 => "Steady",
                -2..=-1 => "Uneasy",
                _ => "Low",
            }
            .into(),
            tone: match crate::story::spirits(world) {
                1.. => Tone::Good,
                -2..=0 => Tone::Warning,
                _ => Tone::Bad,
            },
        },
    ]
}

#[cfg(test)]
pub(crate) fn with_thousands_for_test(amount: i64) -> String {
    with_thousands(amount)
}

/// "1,372", the way money is written.
fn with_thousands(amount: i64) -> String {
    let digits = amount.unsigned_abs().to_string();
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    if amount < 0 {
        format!("-{grouped}")
    } else {
        grouped
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

        assert!(
            briefing
                .items
                .iter()
                .any(|item| item.title == "Leo started protecting his savings"),
            "{briefing:#?}"
        );
        assert!(briefing
            .items
            .iter()
            .any(|item| item.title == "Emma's pay from the school took a knock"));
    }

    #[test]
    fn every_briefing_opens_with_the_harbor_state() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let fresh = snapshot(society.world());
        let first = &fresh.briefing.expect("briefing").items[0];
        assert_eq!(first.title, "Harbour today");
        assert!(first.detail.starts_with("Harbour Bakery"));
        assert!(first.detail.contains("Jonas"));

        let quiet = snapshot_since(society.world(), Some(society.world().events().len()));
        let items = quiet.briefing.expect("briefing").items;
        assert_eq!(items[0].title, "Harbour today");
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
        for title in ["Harbour today", "Harbour Bakery had customers"] {
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
            !beat_titles(&briefing).contains(&"Harbour Bakery had customers".to_string()),
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

#[cfg(test)]
mod probe_parts {
    use super::*;

    /// The briefing found through the World's index of its history is the
    /// one found by reading every event, on each of a season's days.
    #[test]
    fn the_indexed_briefing_is_the_briefing() {
        let mut society = crate::TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.begin_story().unwrap();
        for day in 0..90 {
            let world = branch.world();
            assert_eq!(
                briefing_from(world, None, true),
                briefing_from(world, None, false),
                "day {day}"
            );
            // What is open, read from the story's open storylets, is what
            // asking after every storylet in the deck finds.
            let deck = crate::story::deck();
            let mut asked = deck
                .storylets
                .iter()
                .filter_map(|s| Some((storylets::opened_at(world.state(), deck, s.id)?, s)))
                .collect::<Vec<_>>();
            asked.sort_by_key(|(at, storylet)| (*at, storylet.id));
            let open = storylets::open(world.state(), deck);
            assert_eq!(asked.len(), open.len(), "day {day}");
            assert!(
                asked
                    .iter()
                    .zip(&open)
                    .all(|((_, a), b)| std::ptr::eq(*a, *b)),
                "day {day}"
            );
            let snapshot = snapshot(world);
            if let Some(choice) = snapshot
                .choices()
                .find(|c| c.question.is_some() && c.unavailable.is_none())
            {
                let _ = branch.invoke_projection_command(&choice.id.clone());
            }
            if day % 3 == 0 {
                if let Some((_, deed, _)) = snapshot
                    .deeds()
                    .find(|(_, c, hand)| c.unavailable.is_none() && hand.verb != "Undo")
                {
                    let _ = branch.invoke_projection_command(&deed.id.clone());
                }
            }
            branch
                .invoke_projection_command(crate::story::WAIT_COMMAND)
                .unwrap();
        }
    }
    #[test]
    #[ignore]
    fn probe_parts() {
        let mut society = crate::TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.begin_story().unwrap();
        // `PROBE_DAYS` sets how long to play (a year by default), and
        // `PROBE_AT` the days, comma-separated, to time a snapshot on.
        let days = std::env::var("PROBE_DAYS")
            .ok()
            .and_then(|days| days.parse().ok())
            .unwrap_or(365_usize);
        let at = std::env::var("PROBE_AT")
            .unwrap_or_default()
            .split(',')
            .filter_map(|day| day.trim().parse().ok())
            .collect::<Vec<usize>>();
        for day in 1..=days {
            if at.contains(&day) {
                // The projection alone, and as a session shows it, with
                // what each choice would do.
                let time = |take: &dyn Fn()| {
                    let mut times = (0..7)
                        .map(|_| {
                            let started = std::time::Instant::now();
                            take();
                            started.elapsed()
                        })
                        .collect::<Vec<_>>();
                    times.sort();
                    times[3]
                };
                let alone = time(&|| {
                    std::hint::black_box(snapshot(branch.world()));
                });
                let shown = time(&|| {
                    std::hint::black_box(branch.projection_snapshot());
                });
                eprintln!(
                    "day {day}: snapshot median {alone:?}, with previews {shown:?} ({} events)",
                    branch.world().events().len()
                );
            }
            let snapshot = snapshot(branch.world());
            if let Some(c) = snapshot
                .choices()
                .find(|c| c.question.is_some() && c.unavailable.is_none())
            {
                let _ = branch.invoke_projection_command(&c.id.clone());
            }
            branch
                .invoke_projection_command(crate::story::WAIT_COMMAND)
                .unwrap();
        }
        let world = branch.world();
        macro_rules! once {
            ($name:expr, $e:expr) => {{
                let s = std::time::Instant::now();
                let r = $e;
                eprintln!("{:>14}: {:?} (once)", $name, s.elapsed());
                r
            }};
        }
        macro_rules! t {
            ($name:expr, $e:expr) => {{
                // The median of several takes, the first one discarded.
                let mut times = Vec::new();
                let mut r = $e;
                for _ in 0..9 {
                    let s = std::time::Instant::now();
                    r = std::hint::black_box($e);
                    times.push(s.elapsed());
                }
                times.sort();
                eprintln!("{:>14}: {:?}", $name, times[4]);
                r
            }};
        }
        t!("index", world.history_index());
        let cmds = t!("commands", available_commands(world));
        t!("talks", crate::talk::talks(world, &cmds));
        t!("briefing", society_briefing(world, None));
        t!("narratable", narratable_count(world, world.events()));
        t!("timeline", told_timeline(world));

        t!("canvas", canvas_items(world));
        t!("inspectors", inspectors_from_world(world));
        t!("why", why_map_from_world(world));
        t!("gauges", gauges(world));
        t!("voices", crate::talk::voices(world));
        t!("goals", crate::story::goals(world));
        t!("chapters", crate::story::chapters(world));
        t!("weather", crate::story::weather(world));
        t!("book", crate::book::book(world));
        let cast = crate::life::cast();
        t!("keepsakes", lives::keepsakes(world));
        t!("possible", lives::possible_keepsakes(world, &cast));
        t!("letters", lives::letters(world));
        t!("writers", lives::letter_writers(world, &cast));
        t!("firsts", lives::firsts(world));
        t!("met", lives::met(world));
        t!("to meet", lives::people_to_meet(world, &cast));
        let mut s = t!("snapshot", snapshot(world));
        t!("with previews", crate::with_previews(world, s.clone()));
        {
            let actions = t!("registry", crate::build_action_registry().unwrap());
            let mut sketch = world.sketch(world_projection::RECENT_EVENTS);
            let mut story_only = sketch.clone();
            once!(
                "story tick",
                crate::story::tick(&mut story_only, actions, false).unwrap()
            );
            let cast = t!("cast_in", crate::life::cast_in(sketch.state()));
            once!(
                "lives tick",
                lives::tick_with(
                    &mut sketch,
                    actions,
                    &cast,
                    false,
                    false,
                    &crate::firsts::quiet_days()
                )
                .unwrap()
            );
            let kit = crate::handwork::kit(sketch.state());
            once!(
                "hands tick",
                hands::tick(&mut sketch, actions, &kit).unwrap()
            );
            let almanac = crate::almanac::almanac(sketch.state());
            once!(
                "calendar tick",
                calendar::tick(&mut sketch, actions, &almanac).unwrap()
            );
            t!("gauges", gauges(&sketch));
            once!(
                "years tick",
                crate::years::tick(&mut sketch, actions).unwrap()
            );
        }
        t!("sketch", world.sketch(world_projection::RECENT_EVENTS));
        t!("state clone", world.state().clone());
        for command in s.commands.iter().filter(|command| command.hand.is_none()) {
            // A sketch of the World and the choice made on it, as a
            // preview makes it.
            t!(command.id.as_str(), {
                let mut copy = crate::TinySocietyBranch {
                    world: world.sketch(world_projection::RECENT_EVENTS),
                };
                copy.invoke_projection_command(&command.id).is_ok()
            });
        }
        t!("tell", s.tell_events_as_history_does());
        eprintln!("events {}", world.events().len());
        // What the lives System's notes hold of what was said.
        let notes = world.state().entity(crate::life::cast().notes).unwrap();
        let said = notes.components.iter().filter(|(key, _)| {
            key.starts_with("lives.lines.")
                || key.starts_with("lives.said.")
                || key.starts_with("lives.heard.")
        });
        let (count, chars) = said.fold((0, 0), |(count, chars), (key, value)| {
            let len = match value {
                world_core::Value::Text(text) => text.len(),
                _ => 8,
            };
            (count + 1, chars + key.len() + len)
        });
        eprintln!("heard notes: {count} entries, {chars} characters");
    }
}
