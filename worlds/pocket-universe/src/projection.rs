use crate::narrator;
use crate::{
    seed_id, NUDGE_COMMAND, RELATIONSHIP, RELATIONSHIP_TENSION, RELATIONSHIP_TRUST,
    SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND, SEED_PENGUIN_CIVILIZATION_COMMAND, SLOT_A,
    SLOT_B, SLOT_C, SLOT_D, SLOT_E, UNIVERSE,
};
use std::collections::BTreeMap;
use world_core::{Entity, EntityId, Event, Value, World};
use world_projection::{
    entity_title, inspectors_from_world, value_text, why_map_from_world, BriefingItem,
    BriefingItemKind, BriefingProjection, CanvasChange, CanvasItem, CanvasItemKind, CanvasLink,
    CanvasLinkTone, CanvasProjection, CollectionItem, CollectionProjection, InspectorProjection,
    InspectorRow, InspectorSection, ProjectionCapabilities, ProjectionCommand, ProjectionSnapshot,
    SelectionId, Tone,
};

/// The choices a World offers now, before any is previewed.
pub(crate) fn commands_on_offer(world: &World) -> Vec<ProjectionCommand> {
    commands(world, seed_id(world) != "unseeded")
}

/// What the player said to people today, and what they answered; a
/// request is offered only while its choice still is.
fn exchanges(world: &World, commands: &[ProjectionCommand]) -> Vec<world_projection::Exchange> {
    conversation::exchanges_today(world)
        .into_iter()
        .map(|exchange| world_projection::Exchange {
            who: SelectionId::Entity(exchange.who),
            words: exchange.words,
            answer: exchange.reply,
            moment: SelectionId::Event(exchange.event),
            asks_for: exchange.asks_for.filter(|id| {
                commands
                    .iter()
                    .any(|command| &command.id == id && command.unavailable.is_none())
            }),
            voiced: exchange.voiced,
            reported: exchange.reported,
        })
        .collect()
}

pub(crate) fn snapshot(world: &World) -> ProjectionSnapshot {
    snapshot_since(world, None)
}

pub(crate) fn snapshot_since(
    world: &World,
    since_event_count: Option<usize>,
) -> ProjectionSnapshot {
    let seed = seed_id(world);
    let seeded = seed != "unseeded";
    let commands = commands(world, seeded);
    let talks = crate::talk::talks(world, &commands);
    let exchanges = exchanges(world, &commands);
    let almanac = crate::almanac::almanac(world.state());
    let mut snapshot = ProjectionSnapshot {
        title: if seeded {
            universe_name(world)
        } else {
            "A new World".into()
        },
        world_time: world.world_time(),
        capabilities: ProjectionCapabilities {
            fork: !world.events().is_empty(),
            background: seeded,
            talk: seeded,
        },
        briefing: Some(toned(world, briefing(world, seeded, since_event_count))),
        commands,
        collection: collection(world),
        timeline: told_timeline(world),
        canvas: with_changes(world, canvas(world), since_event_count),
        inspectors: told_inspectors(world),
        why: why_map_from_world(world),
        scenery: seeded
            .then(|| seed_scenery(seed_id(world)))
            .flatten()
            // Only Maple Street has green hills to turn gold and frost:
            // Mars keeps its butterscotch and the ice its blue all year, and
            // their seasons show on the ground.
            .map(|scenery| match seed_id(world) {
                "1980s-town" => scenery.in_season(crate::story::season(world) as u64),
                _ => scenery,
            }),
        calendar: seeded.then(|| world_projection::Calendar {
            unit: seed_time_unit(seed_id(world)).into(),
            length: crate::BACKGROUND_PERIOD,
            season: Some(calendar::season_name(world.state(), &almanac).into()),
            coming: calendar::coming_up(world.state(), &almanac, 7),
            festival_today: calendar::festival_today(world.state(), &almanac),
            year: Some(crate::almanac::YEAR),
        }),
        gauges: gauges(world),
        voices: crate::talk::voices(world),
        talks,
        exchanges,
        openers: if seeded {
            conversation::openers::shown(world, &crate::speech::kit(world.state()))
        } else {
            Vec::new()
        },
        drawings: crate::drawings::drawings_for(world),
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
    // The place's moments: the latest few, and every one in the book.
    let moments = crate::moments::moments(world);
    snapshot.almanac = crate::almanac_page::new_year(world, &moments);
    snapshot.book.extend(crate::moments::book_entries(&moments));
    snapshot.moments = world_projection::latest_moments(&moments);
    // Whoever asked what was just answered goes over to what it made.
    world_projection::go_to_what_was_answered(world, &mut snapshot.canvas.items, |event| {
        matches!(crate::story::answer_words(world, event), Some(Some(_)))
    });
    snapshot.tell_events_as_history_does();
    snapshot.keep_voices_in_view();
    snapshot
}

/// Good news, bad news, or neither, for each briefing line, read from the
/// moment the line reports.
fn toned(world: &World, mut briefing: BriefingProjection) -> BriefingProjection {
    for item in &mut briefing.items {
        let told = match item.selection {
            Some(SelectionId::Event(id)) => world.event(id).and_then(crate::story::tone),
            _ => None,
        };
        if let Some(tone) = told {
            item.tone = tone;
        }
    }
    briefing
}

fn commands(world: &World, seeded: bool) -> Vec<ProjectionCommand> {
    if !seeded {
        return vec![
            ProjectionCommand {
                id: SEED_MARS_COLONY_COMMAND.into(),
                title: "Start a Mars colony".into(),
                detail: "A tiny habitat, one keeper, hydroponics, and a rover on a red horizon."
                    .into(),
                effects: Vec::new(),
                scenery: seed_scenery("mars-colony"),
                asker: None,
                moves: Vec::new(),
                question: None,
                unavailable: None,
                hand: None,
                preview: None,
                role: None,
            },
            ProjectionCommand {
                id: SEED_1980S_TOWN_COMMAND.into(),
                title: "Start a town in 1987".into(),
                detail: "An arcade, local radio, a night bus, and a neighborhood that remembers."
                    .into(),
                effects: Vec::new(),
                scenery: seed_scenery("1980s-town"),
                asker: None,
                moves: Vec::new(),
                question: None,
                unavailable: None,
                hand: None,
                preview: None,
                role: None,
            },
            ProjectionCommand {
                id: SEED_PENGUIN_CIVILIZATION_COMMAND.into(),
                title: "Start a penguin civilization".into(),
                detail: "An ice bridge, a fish vault, a moonrise council, and one bridge keeper."
                    .into(),
                effects: Vec::new(),
                scenery: seed_scenery("penguin-civilization"),
                asker: None,
                moves: Vec::new(),
                question: None,
                unavailable: None,
                hand: None,
                preview: None,
                role: None,
            },
        ];
    }

    let mut commands = vec![ProjectionCommand {
        id: NUDGE_COMMAND.into(),
        title: wait_title(seed_id(world)).into(),
        detail: "Let a little more time pass, and see what they do with it.".into(),
        effects: Vec::new(),
        scenery: None,
        asker: None,
        moves: Vec::new(),
        question: None,
        unavailable: None,
        hand: None,
        preview: None,
        role: Some(world_projection::CommandRole::PassesTime),
    }];
    commands.extend(crate::story::commands(world));
    // What the player can do with their own hands comes after every card;
    // a screen offers it apart from them.
    commands.extend(crate::handwork::commands(world));
    commands.extend(crate::life::suggestions(world));
    // A newborn's parents ask the player to choose a name, last of all.
    commands.extend(crate::plots::naming_cards(world));
    commands
}

/// The one way to wait, in each place's own time.
pub(crate) fn wait_title(seed: &str) -> &'static str {
    match seed {
        "mars-colony" => "Let the sol pass",
        "1980s-town" => "Let the night pass",
        "penguin-civilization" => "Let the aurora turn",
        _ => "Let time pass",
    }
}

fn briefing(world: &World, seeded: bool, since_event_count: Option<usize>) -> BriefingProjection {
    if !seeded {
        return BriefingProjection {
            eyebrow: "Pocket Universe".into(),
            title: "Where should this World begin?".into(),
            // The three places to begin are pictures; nothing needs saying
            // beside them.
            items: Vec::new(),
            returned: false,
        };
    }

    if let Some(since) = since_event_count.filter(|since| *since < world.events().len()) {
        let events = &world.events()[since..];
        let mut items = return_digest_items(world, events);
        // What someone left or wrote the player while they were away
        // closes the story of the return.
        if let Some(left) = events
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
                items.push(BriefingItem {
                    kind: BriefingItemKind::Beat,
                    selection: Some(SelectionId::Event(left.id)),
                    title,
                    detail: lives::said(left).map(|(_, note)| note).unwrap_or_default(),
                    tone: world_projection::Tone::Good,
                });
            }
        }
        return BriefingProjection {
            eyebrow: format!("Pocket Universe · {}", seed_label(seed_id(world))),
            title: "While you were away".into(),
            items,
            returned: true,
        };
    }

    // Headed by whatever the storyteller has just set going; a World that
    // has not yet lived a period is waking up.
    let title = crate::story::headline(world).unwrap_or_else(|| {
        if world.world_time() == 0 {
            match seed_id(world) {
                "mars-colony" => "Ares is waking up",
                "1980s-town" => "Maple Street is waking up",
                "penguin-civilization" => "Icebridge is waking up",
                _ => "A World is waking up",
            }
            .into()
        } else {
            QUIET_TITLE.into()
        }
    });
    BriefingProjection {
        eyebrow: format!("Pocket Universe · {}", seed_label(seed_id(world))),
        title,
        items: Vec::new(),
        returned: false,
    }
}

const QUIET_TITLE: &str = "Life goes on";

/// History in the World's own words. What happened to somebody is a story;
/// the everyday round folds under the moment it happened in.
fn told_timeline(world: &World) -> world_projection::TimelineProjection {
    // Everyday life is told as it happens, in what people say; History
    // keeps to what changed, and to today's, which today's words point at.
    let now = world.world_time();
    // A day beginning tells nothing: it is there to be the cause of what
    // the day brings.
    let mut timeline = world_projection::timeline_of(world, |event| {
        !calendar::is_day_begun(event)
            && (event.world_time == now
                || !matches!(
                    event.kind.as_str(),
                    "lived" | "life_began" | "lines_forgotten"
                ))
    });
    world_projection::retell_timeline(&mut timeline, world, |event| {
        let summary =
            ["summary", "change"]
                .into_iter()
                .find_map(|key| match event.payload.get(key) {
                    Some(Value::Text(text)) if !text.trim().is_empty() => Some(text.clone()),
                    _ => None,
                });
        if let Some(told) =
            conversation::told(event).or_else(|| conversation::favour::told(world.state(), event))
        {
            return world_projection::Telling::Routine(Some(told));
        }
        if event.kind == "festival_nears" {
            return world_projection::Telling::Routine(calendar::told(event));
        }
        if let Some(told) = crate::story::told(world, event) {
            return world_projection::Telling::Story(told);
        }
        match event.kind.as_str() {
            kind if is_routine(kind) => world_projection::Telling::Routine(None),
            "universe_seeded" => {
                world_projection::Telling::Story(format!("{} began", universe_name(world)))
            }
            _ => match summary {
                Some(summary) => world_projection::Telling::Story(summary),
                None => world_projection::Telling::Routine(None),
            },
        }
    });
    timeline
}

/// The everyday round: what happens every period whatever anyone chooses.
/// History folds it and a return leads with anything else first.
pub(crate) fn is_routine(kind: &str) -> bool {
    matches!(
        kind,
        "story_began" | "lived" | "life_began" | "lines_forgotten" | "bond_settled"
    )
}

/// The Events a return digest will show, and how many of that kind it stands
/// for. Shared with the narrator so that what gets put into words is exactly
/// what gets read, and the two can never drift apart.
pub(crate) fn digest_events(events: &[Event]) -> Vec<(&Event, usize)> {
    let mut groups = Vec::<(&Event, usize)>::new();
    for event in events.iter().rev().filter(|event| {
        // A narrated line is not an event of its own: it is how the event
        // it re-words gets read. What someone left or wrote the player
        // closes the return on its own, and what someone said of a deed is
        // heard as it happens. A question that came up is asked on the
        // card, or told by how it went, never also as it came up.
        !matches!(
            event.kind.as_str(),
            narrator::NARRATED | "keepsake_left" | "letter_written" | "reacted" | "situation_arose"
        ) && !conversation::favour::is_favour(event)
    }) {
        // One line a kind, and one a storylet: how a question ended is
        // told once, by the latest of it.
        let storylet = |event: &Event| match event.payload.get("storylet") {
            Some(Value::Text(id)) => Some(id.clone()),
            _ => None,
        };
        if let Some((_, count)) = groups.iter_mut().find(|(latest, _)| {
            latest.kind == event.kind
                || storylet(latest).is_some_and(|id| Some(id) == storylet(event))
        }) {
            *count += 1;
        } else {
            groups.push((event, 1));
        }
    }

    groups.sort_by_key(|(event, _)| return_digest_priority(event.kind.as_str()));
    groups.truncate(RETURN_DIGEST_ENTRIES);
    groups
}

/// How many kinds of thing a return digest reports before it stops.
pub(crate) const RETURN_DIGEST_ENTRIES: usize = 3;

fn return_digest_items(world: &World, events: &[Event]) -> Vec<BriefingItem> {
    digest_events(events)
        .into_iter()
        .map(|(event, _)| return_item(world, events, event))
        .collect()
}

fn return_digest_priority(kind: &str) -> u8 {
    match kind {
        "chapter_ended" | "universe_seeded" => 0,
        // Something coming up is not yet news; how it ended is. What the
        // player said themselves is not news to them.
        "situation_arose" | "festival_nears" | "spoken" => 3,
        kind if is_routine(kind) => 3,
        _ => 1,
    }
}

/// The line a return shows for an Event when the World has not put it
/// into its own words: what it records about itself, or what was said.
pub(crate) fn table_line(world: &World, event: &Event) -> Option<String> {
    ["change", "summary"]
        .into_iter()
        .find_map(|key| match event.payload.get(key) {
            Some(Value::Text(value)) if !value.trim().is_empty() => Some(value.clone()),
            _ => None,
        })
        .or_else(|| crate::story::line(world, event).map(|(_, line)| line))
}

fn return_item(world: &World, events: &[Event], event: &Event) -> BriefingItem {
    // This World's own words if it has them for this Event, and the table line
    // otherwise.
    let detail = narrator::narrated_text(events, event.id)
        .map(str::to_owned)
        .or_else(|| table_line(world, event))
        .unwrap_or_else(|| event_kind_words(&event.kind));
    let title = match event.kind.as_str() {
        "universe_seeded" => "A world began".into(),
        _ => crate::story::told(world, event).unwrap_or_else(|| event_kind_words(&event.kind)),
    };
    BriefingItem {
        kind: BriefingItemKind::Beat,
        selection: Some(SelectionId::Event(event.id)),
        title,
        detail,
        tone: world_projection::Tone::Neutral,
    }
}

/// Last resort for an Event nobody named: its kind, as a sentence.
fn event_kind_words(kind: &str) -> String {
    let words = kind.replace('_', " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => words,
    }
}

fn collection(world: &World) -> CollectionProjection {
    CollectionProjection {
        title: "World Contents".into(),
        items: world
            .state()
            .entities()
            .filter(|entity| entity.id != UNIVERSE)
            .map(|entity| CollectionItem {
                id: SelectionId::Entity(entity.id),
                title: crate::story::fill(world, &entity_title(entity)),
                subtitle: entity.kind.replace('_', " "),
            })
            .collect(),
    }
}

/// What a place looks like on its tile: the habitat a dome and its
/// greenhouse a tree, the arcade a shopfront, the colony the bridge it is
/// named for.
fn place_shape(world: &World, id: EntityId) -> Option<world_projection::MarkShape> {
    use world_projection::MarkShape::{Bridge, Dome, House, Shop, Tower, Tree};
    match (seed_id(world), id) {
        ("mars-colony", SLOT_A) => Some(Dome),
        ("mars-colony", SLOT_C) => Some(Tree),
        ("1980s-town", SLOT_A) => Some(Shop),
        ("1980s-town", SLOT_C) => Some(Tower),
        ("penguin-civilization", SLOT_A) => Some(Bridge),
        ("penguin-civilization", SLOT_C) => Some(Dome),
        ("penguin-civilization", SLOT_D) => Some(House),
        _ => None,
    }
}

fn canvas(world: &World) -> CanvasProjection {
    let askers = crate::speech::askers(world);
    // Every seed casts the same five roles in the same slots: the anchor
    // everything depends on, a second place, the two people whose
    // relationship is the story, and the thing that lets them range out.
    // Placing by role rather than by list order lets the scene read the same
    // way in every World: home on the left, the pair in the middle, the way
    // out on the right.
    const LAYOUT: [(EntityId, f32, f32); 6] = [
        (SLOT_A, 0.12, 0.22),
        (SLOT_C, 0.12, 0.86),
        (SLOT_B, 0.42, 0.10),
        (SLOT_E, 0.62, 0.84),
        (SLOT_D, 0.90, 0.46),
        (crate::story::NEWCOMER, 0.3, 0.5),
    ];
    let mut items: Vec<CanvasItem> = LAYOUT
        .iter()
        .filter_map(|(id, x, y)| {
            let entity = world.state().entity(*id)?;
            Some(CanvasItem {
                id: SelectionId::Entity(*id),
                kind: canvas_kind(entity),
                label: crate::story::fill(world, &entity_title(entity)),
                detail: canvas_detail(entity),
                x: *x,
                y: *y,
                changes: Vec::new(),
                shape: place_shape(world, *id).or_else(|| {
                    // The thing that lets them range out is drawn as a
                    // vehicle: the rover, the night bus.
                    (*id == SLOT_D && canvas_kind(entity) == CanvasItemKind::Object)
                        .then_some(world_projection::MarkShape::Rover)
                }),
                at: whereabouts(world, entity),
                look: crate::kin::look(world, *id),
                drawing: crate::drawings::drawing_of(
                    world,
                    *id,
                    canvas_kind(entity) == CanvasItemKind::Actor,
                ),
                stance: crate::drawings::stance_of(world, *id),
                standing: crate::speech::standing_of(world, *id),
                mood: crate::speech::mood_of(world, *id, &askers),
                spot: None,
                px: None,
                home: None,
                day: Vec::new(),
                built: None,
                ..Default::default()
            })
        })
        .collect();
    // Strangers who came to stay, and whoever the place's years brought.
    let cast = crate::life::cast(world.state());
    for (index, id) in crate::life::newcomers(world.state())
        .into_iter()
        .chain(lives::born_here(world.state(), &cast))
        .filter(|id| !lives::gone(world.state(), *id))
        .enumerate()
    {
        let Some(entity) = world.state().entity(id) else {
            continue;
        };
        items.push(CanvasItem {
            id: SelectionId::Entity(id),
            kind: CanvasItemKind::Actor,
            label: crate::story::fill(world, &entity_title(entity)),
            detail: canvas_detail(entity),
            x: 0.2 + 0.12 * (index % 6) as f32 + 0.06 * (index / 6 % 2) as f32,
            y: 0.55 + 0.08 * (index % 2) as f32 + 0.1 * (index / 6 % 2) as f32,
            changes: Vec::new(),
            shape: None,
            at: whereabouts(world, entity),
            look: crate::kin::look(world, id),
            drawing: crate::drawings::drawing_of(world, id, true),
            stance: crate::drawings::stance_of(world, id),
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
    items.extend(crate::story::fixtures(world));
    crate::town::lay_out(world, items, relationship_link(world).into_iter().collect())
}

/// On a return, what moved on each thing on stage since the visit.
fn with_changes(
    world: &World,
    mut canvas: CanvasProjection,
    since_event_count: Option<usize>,
) -> CanvasProjection {
    let Some(since) = since_event_count else {
        return canvas;
    };
    for item in &mut canvas.items {
        let SelectionId::Entity(id) = item.id else {
            continue;
        };
        if let Some((then, now)) =
            world_projection::component_change_since(world, since, id, "status")
        {
            let text = |value: Option<Value>| match value {
                Some(Value::Text(text)) => text,
                Some(other) => value_text(&other, world),
                None => "—".into(),
            };
            item.changes.push(CanvasChange {
                label: String::new(),
                before: text(then),
                after: text(now),
                tone: Tone::Neutral,
            });
        }
    }
    canvas
}

/// What a scene node says under its name: how it is, not what it is.
fn canvas_detail(entity: &Entity) -> String {
    for key in ["status", "role"] {
        if let Some(Value::Text(value)) = entity.component(key) {
            if !value.trim().is_empty() {
                return value.clone();
            }
        }
    }
    // What it is, in the player's words, never what the World keeps it as.
    match entity.kind.as_str() {
        "rover" => "A rover".into(),
        "bus" => "A night bus".into(),
        "place" | "person" | "penguin" | "fixture" => String::new(),
        kind => kind.replace('_', " "),
    }
}

/// What a Pocket Universe keeps score of: how much the two people trust
/// each other and how strained they are, which is what the questions they
/// ask move, and what the place has put by. Nothing before the World
/// begins.
pub(crate) fn gauges(world: &World) -> Vec<world_projection::Gauge> {
    use world_projection::Gauge;
    if seed_id(world) == "unseeded" {
        return Vec::new();
    }
    let relationship = world.state().entity(RELATIONSHIP);
    let out_of_ten = |key: &str| {
        integer_entity_component(relationship, key)
            .unwrap_or(0)
            .clamp(0, 10)
    };
    let trust = out_of_ten(RELATIONSHIP_TRUST);
    let tension = out_of_ten(RELATIONSHIP_TENSION);
    let stores = crate::years::stores(world.state());
    vec![
        Gauge {
            id: "trust".into(),
            label: "Trust".into(),
            value: trust as f32 / 10.0,
            reading: format!("{trust} of 10"),
            tone: if trust >= 5 {
                Tone::Good
            } else {
                Tone::Neutral
            },
        },
        Gauge {
            id: "tension".into(),
            label: "Tension".into(),
            value: tension as f32 / 10.0,
            reading: format!("{tension} of 10"),
            tone: match tension {
                7.. => Tone::Bad,
                4..=6 => Tone::Warning,
                _ => Tone::Neutral,
            },
        },
        // What the place has put by: what its works and its turns bring
        // in, and its people, parties and hard times take out.
        Gauge {
            id: "stores".into(),
            label: match seed_id(world) {
                "1980s-town" => "Street fund",
                "penguin-civilization" => "Fish vault",
                _ => "Stores",
            }
            .into(),
            value: stores as f32 / 100.0,
            reading: format!("{stores} of 100"),
            tone: match stores {
                50.. => Tone::Good,
                20..=49 => Tone::Neutral,
                _ => Tone::Warning,
            },
        },
    ]
}

/// Each thing's detail panel in the World's words: what a person does and
/// has been doing, where a relationship stands, how a place is. The
/// counters, generations and bookkeeping the rules keep stay out of sight.
fn told_inspectors(world: &World) -> BTreeMap<SelectionId, InspectorProjection> {
    let mut inspectors = inspectors_from_world(world);
    for (selection, inspector) in &mut inspectors {
        let SelectionId::Entity(id) = selection else {
            continue;
        };
        let Some(entity) = world.state().entity(*id) else {
            continue;
        };
        let (subtitle, rows) = told_state(world, entity);
        if let Some(subtitle) = subtitle {
            inspector.subtitle = subtitle;
        }
        inspector
            .sections
            .retain(|section| section.title != "State");
        if !rows.is_empty() {
            inspector.sections.insert(
                0,
                InspectorSection {
                    title: "Now".into(),
                    rows,
                },
            );
        }
    }
    inspectors
}

fn told_state(world: &World, entity: &Entity) -> (Option<String>, Vec<InspectorRow>) {
    let text = |key: &str| match entity.component(key) {
        Some(Value::Text(value)) if !value.trim().is_empty() => Some(value.clone()),
        _ => None,
    };
    let row = |label: &str, value: String| InspectorRow {
        label: label.into(),
        value,
    };
    let mut rows = Vec::new();
    if entity.id == UNIVERSE {
        return (Some(seed_label(seed_id(world)).into()), rows);
    }
    if entity.id == RELATIONSHIP {
        let number = |key: &str| integer_entity_component(Some(entity), key).unwrap_or(0);
        rows.push(row("Trust", number(RELATIONSHIP_TRUST).to_string()));
        rows.push(row("Tension", number(RELATIONSHIP_TENSION).to_string()));
        if let Some(link) = relationship_link(world) {
            rows.push(row("Where it stands", link.label));
        }
        return (None, rows);
    }
    if matches!(entity.kind.as_str(), "person" | "penguin") {
        if let Some(role) = text("role") {
            rows.push(row("Role", capitalize_first(&role)));
        }
        return (None, rows);
    }
    // A place or a thing: what it says about itself, minus the rules'
    // bookkeeping.
    for (key, value) in &entity.components {
        if key == "name" || is_bookkeeping(key) {
            continue;
        }
        let value = match value {
            Value::Text(value) if !value.trim().is_empty() => value.clone(),
            Value::Integer(value) => value.to_string(),
            _ => continue,
        };
        rows.push(row(
            &capitalize_first(&key.replace('_', " ")),
            capitalize_first(&value),
        ));
    }
    (None, rows)
}

/// Components the rules keep for themselves.
fn is_bookkeeping(key: &str) -> bool {
    key.ends_with("_count") || matches!(key, "seed" | "custom")
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The pair's bond, drawn between the two people it belongs to rather
/// than as a third thing beside them.
fn relationship_link(world: &World) -> Option<CanvasLink> {
    let relationship = world.state().entity(RELATIONSHIP)?;
    world.state().entity(SLOT_B)?;
    world.state().entity(SLOT_E)?;
    let trust = integer_entity_component(Some(relationship), RELATIONSHIP_TRUST).unwrap_or(0);
    let tension = integer_entity_component(Some(relationship), RELATIONSHIP_TENSION).unwrap_or(0);
    let (label, tone) = if tension >= 7 && tension > trust {
        ("At odds", CanvasLinkTone::Strained)
    } else if tension > trust {
        ("Uneasy", CanvasLinkTone::Strained)
    } else if trust >= 7 {
        ("Close", CanvasLinkTone::Warm)
    } else {
        ("Getting to know each other", CanvasLinkTone::Neutral)
    };
    Some(CanvasLink {
        from: SelectionId::Entity(SLOT_B),
        to: SelectionId::Entity(SLOT_E),
        label: label.into(),
        tone,
        strength: (trust.max(tension) as f32 / 10.0).clamp(0.15, 1.0),
        selection: Some(SelectionId::Entity(RELATIONSHIP)),
    })
}

fn canvas_kind(entity: &Entity) -> CanvasItemKind {
    match entity.kind.as_str() {
        "person" | "penguin" => CanvasItemKind::Actor,
        "place" | "habitat" | "colony" | "radio_station" | "storehouse" | "council" => {
            CanvasItemKind::Place
        }
        _ => CanvasItemKind::Object,
    }
}

/// Where someone is: wherever their day took them, where they were taken
/// in, or else at home.
fn whereabouts(world: &World, entity: &Entity) -> Option<SelectionId> {
    if canvas_kind(entity) != CanvasItemKind::Actor {
        return None;
    }
    if let Some(place) = lives::at(world.state(), entity.id) {
        return Some(SelectionId::Entity(place));
    }
    if let Some(Value::Entity(place)) = entity.component("location") {
        return Some(SelectionId::Entity(*place));
    }
    world
        .state()
        .entity(SLOT_A)
        .map(|_| SelectionId::Entity(SLOT_A))
}

fn universe_name(world: &World) -> String {
    world
        .state()
        .entity(UNIVERSE)
        .map(entity_title)
        .unwrap_or_else(|| "Pocket Universe".into())
}

fn integer_entity_component(entity: Option<&Entity>, key: &str) -> Option<i64> {
    match entity.and_then(|entity| entity.component(key)) {
        Some(Value::Integer(value)) => Some(*value),
        _ => None,
    }
}

/// What each place looks like from a distance, by day (the app grades it
/// for dusk and night): red dust under a pale butterscotch sky for Ares, a
/// hazy summer sky over lilac hills and asphalt for Maple Street, a cold
/// clear sky over the ice and a dark sea for Icebridge.
pub(crate) fn seed_scenery(seed: &str) -> Option<world_projection::Scenery> {
    let scenery = |sky_top, sky_bottom, far, near, sun| world_projection::Scenery {
        sky_top,
        sky_bottom,
        far,
        near,
        sun,
    };
    match seed {
        "mars-colony" => Some(scenery(0xe7b089, 0xf5d9bd, 0xc2663f, 0x8a3a22, 0xfff3dc)),
        "1980s-town" => Some(scenery(0x7ca6d6, 0xeedfcc, 0x857e9c, 0x46465a, 0xffd98a)),
        "penguin-civilization" => Some(scenery(0x8fbcd8, 0xeaf3f7, 0xbcd4e4, 0x2f6f8a, 0xfff6dc)),
        _ => None,
    }
}

/// What each place counts its days in.
fn seed_time_unit(seed: &str) -> &'static str {
    match seed {
        "mars-colony" => "Sol",
        "1980s-town" => "Night",
        "penguin-civilization" => "Aurora",
        _ => "Day",
    }
}

fn seed_label(seed: &str) -> &'static str {
    match seed {
        "mars-colony" => "Mars Colony",
        "1980s-town" => "1987 Town",
        "penguin-civilization" => "Penguin Civilization",
        _ => "Unseeded",
    }
}
