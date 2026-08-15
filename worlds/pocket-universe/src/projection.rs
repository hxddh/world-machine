use crate::{
    seed_id, BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND, DECISION, GENERATION, LAST_CHANGE, LEGACY,
    LEGACY_SUMMARY, NUDGE_COMMAND, OUTWARD_POSTURE_COMMAND, POSTURE, RELATIONSHIP,
    RELATIONSHIP_DIRECTION, RELATIONSHIP_LAST_DYNAMIC, RELATIONSHIP_SOCIAL_ARC,
    RELATIONSHIP_TENSION, RELATIONSHIP_TRUST, RIVALRY_COMMAND, ROOTED_POSTURE_COMMAND,
    SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND, SEED_PENGUIN_CIVILIZATION_COMMAND,
    SHARED_PROJECT_COMMAND, UNIVERSE,
};
use world_core::{Entity, Event, Value, World};
use world_projection::{
    entity_title, inspectors_from_world, timeline_from_world, why_map_from_world, BriefingItem,
    BriefingProjection, CanvasItem, CanvasItemKind, CanvasProjection, CollectionItem,
    CollectionProjection, ProjectionCapabilities, ProjectionCommand, ProjectionSnapshot,
    SelectionId,
};

pub(crate) fn snapshot(world: &World) -> ProjectionSnapshot {
    snapshot_since(world, None)
}

pub(crate) fn snapshot_since(
    world: &World,
    since_event_count: Option<usize>,
) -> ProjectionSnapshot {
    let seed = seed_id(world);
    let seeded = seed != "unseeded";
    ProjectionSnapshot {
        title: if seeded {
            universe_name(world)
        } else {
            "Pocket Universe · Empty World".into()
        },
        world_time: world.world_time(),
        capabilities: ProjectionCapabilities {
            fork: !world.events().is_empty(),
        },
        briefing: Some(briefing(world, seeded, since_event_count)),
        commands: commands(world, seeded),
        collection: collection(world),
        timeline: timeline_from_world(world),
        canvas: canvas(world),
        inspectors: inspectors_from_world(world),
        why: why_map_from_world(world),
    }
}

fn commands(world: &World, seeded: bool) -> Vec<ProjectionCommand> {
    if !seeded {
        return vec![
            ProjectionCommand {
                id: SEED_MARS_COLONY_COMMAND.into(),
                title: "Start a Mars colony".into(),
                detail: "A tiny habitat, one keeper, hydroponics, and a rover on a red horizon."
                    .into(),
            },
            ProjectionCommand {
                id: SEED_1980S_TOWN_COMMAND.into(),
                title: "Start a town in 1987".into(),
                detail: "An arcade, local radio, a night bus, and a neighborhood that remembers."
                    .into(),
            },
            ProjectionCommand {
                id: SEED_PENGUIN_CIVILIZATION_COMMAND.into(),
                title: "Start a penguin civilization".into(),
                detail: "An ice bridge, a fish vault, a moonrise council, and one bridge keeper."
                    .into(),
            },
        ];
    }

    let generation = integer_component(world, GENERATION).unwrap_or_default();
    let (relationship_choice_available, intervention_choice_available) =
        choice_state(world, generation);
    let posture_choice_available = posture_choice_state(world, generation);
    let legacy = text_component(world.state().entity(UNIVERSE), LEGACY, "forming");
    let (nudge_title, nudge_detail) = if posture_choice_available {
        (
            "Let the next chapter wait",
            "Keep watching before deciding whether this World reaches outward or roots itself more deeply.",
        )
    } else if legacy != "forming" {
        legacy_nudge_copy(seed_id(world), &legacy)
    } else {
        nudge_copy(
            seed_id(world),
            generation,
            relationship_choice_available,
            intervention_choice_available,
        )
    };
    let mut commands = vec![ProjectionCommand {
        id: NUDGE_COMMAND.into(),
        title: nudge_title.into(),
        detail: nudge_detail.into(),
    }];

    if relationship_choice_available {
        commands.push(ProjectionCommand {
            id: SHARED_PROJECT_COMMAND.into(),
            title: "Give them a shared project".into(),
            detail: "Create a goal that neither actor can complete alone; future interactions will lean toward trust.".into(),
        });
        commands.push(ProjectionCommand {
            id: RIVALRY_COMMAND.into(),
            title: "Let rivalry sharpen them".into(),
            detail: "Keep both actors independent and let competition add pressure to future interactions.".into(),
        });
    }
    if intervention_choice_available {
        let (bold_title, bold_detail, careful_title, careful_detail) =
            intervention_copy(seed_id(world));
        commands.push(ProjectionCommand {
            id: BOLD_PATH_COMMAND.into(),
            title: bold_title.into(),
            detail: bold_detail.into(),
        });
        commands.push(ProjectionCommand {
            id: CAREFUL_PATH_COMMAND.into(),
            title: careful_title.into(),
            detail: careful_detail.into(),
        });
    }
    if posture_choice_available {
        let (outward_title, outward_detail, rooted_title, rooted_detail) =
            posture_command_copy(seed_id(world));
        commands.push(ProjectionCommand {
            id: OUTWARD_POSTURE_COMMAND.into(),
            title: outward_title.into(),
            detail: outward_detail.into(),
        });
        commands.push(ProjectionCommand {
            id: ROOTED_POSTURE_COMMAND.into(),
            title: rooted_title.into(),
            detail: rooted_detail.into(),
        });
    }
    commands
}

fn choice_state(world: &World, generation: i64) -> (bool, bool) {
    let relationship_direction = text_component(
        world.state().entity(RELATIONSHIP),
        RELATIONSHIP_DIRECTION,
        "none",
    );
    let relationship_social_arc = text_component(
        world.state().entity(RELATIONSHIP),
        RELATIONSHIP_SOCIAL_ARC,
        "forming",
    );
    let relationship_choice_available =
        generation >= 2 && relationship_direction == "none" && relationship_social_arc == "forming";
    let decision = text_component(world.state().entity(UNIVERSE), DECISION, "none");
    let intervention_choice_available = generation >= 3 && decision == "none";
    (relationship_choice_available, intervention_choice_available)
}

fn posture_choice_state(world: &World, generation: i64) -> bool {
    if generation < 6 {
        return false;
    }
    let decision = text_component(world.state().entity(UNIVERSE), DECISION, "none");
    let posture = text_component(world.state().entity(UNIVERSE), POSTURE, "none");
    let social_arc = text_component(
        world.state().entity(RELATIONSHIP),
        RELATIONSHIP_SOCIAL_ARC,
        "forming",
    );
    decision != "none" && posture == "none" && social_arc != "forming"
}

fn posture_command_copy(seed: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    match seed {
        "mars-colony" => (
            "Open the ridge routes",
            "Turn Kestrel's reach into routes the colony keeps extending beyond the familiar ridge.",
            "Build a deeper home",
            "Make Ares Habitat the center of the next chapter and deepen what the colony already depends on.",
        ),
        "1980s-town" => (
            "Let Maple Street draw a crowd",
            "Let the arcade, radio, and night bus pull new people into Maple Street's orbit.",
            "Keep it neighborhood-sized",
            "Deepen the local places and rituals already holding the neighborhood together.",
        ),
        "penguin-civilization" => (
            "Invite the outer colonies",
            "Widen Icebridge's circle and keep carrying routes and reports beyond the familiar bridge.",
            "Deepen Icebridge's winter life",
            "Invest the next chapter in the winter systems and local routines that make home resilient.",
        ),
        _ => (
            "Open the World outward",
            "Carry the next chapter toward new edges and unfamiliar threads.",
            "Deepen the World at home",
            "Invest the next chapter in what this World already depends on.",
        ),
    }
}

fn second_arc_stage_copy(seed: &str) -> (String, Option<(&'static str, &'static str)>) {
    let detail = match seed {
        "mars-colony" => {
            "The first expedition and central relationship have left a real shape behind. Decide whether Ares opens its routes outward or turns the next chapter into a deeper home."
        }
        "1980s-town" => {
            "Maple Street now has history and a settled central relationship. Decide whether its next chapter draws a wider crowd or stays deliberately local."
        }
        "penguin-civilization" => {
            "Icebridge now has history and a settled central relationship. Decide whether its next chapter widens the colony network or deepens winter life at home."
        }
        _ => {
            "The first arc has settled. Decide whether the next chapter reaches outward or deepens the home this World already made."
        }
    };
    (
        "A second chapter is ready".into(),
        Some(("Your turn · World direction", detail)),
    )
}

fn legacy_nudge_copy(seed: &str, legacy: &str) -> (&'static str, &'static str) {
    match (seed, legacy) {
        ("mars-colony", "ridge-network") => (
            "Let the ridge network carry on",
            "Let another sol move through the ridge routes and see what this durable expedition network changes next.",
        ),
        ("mars-colony", "competing-frontiers") => (
            "Let the competing frontiers advance",
            "Let another sol pass while rival survey routes keep defining different edges of Ares.",
        ),
        ("mars-colony", "habitat-commons") => (
            "Let the habitat commons deepen",
            "Let another sol move through the commons and see what shared life inside Ares makes durable next.",
        ),
        ("mars-colony", "sealed-districts") => (
            "Let the sealed districts settle",
            "Let another sol pass while Ares keeps organizing safety and trust around its separated districts.",
        ),
        ("1980s-town", "night-network") => (
            "Let the night network carry on",
            "Let another night move through the radio, arcade, bus, and people now connected by the network.",
        ),
        ("1980s-town", "rival-scenes") => (
            "Let the rival scenes keep moving",
            "Let another night pass while Maple Street's competing scenes keep pulling the neighborhood in different directions.",
        ),
        ("1980s-town", "neighborhood-commons") => (
            "Let the neighborhood commons deepen",
            "Let another night pass through the shared places and routines that now hold Maple Street together.",
        ),
        ("1980s-town", "split-blocks") => (
            "Let the split blocks settle",
            "Let another night pass while different blocks keep carrying different versions of neighborhood life.",
        ),
        ("penguin-civilization", "aurora-league") => (
            "Let the aurora league carry on",
            "Let another aurora move through the routes now coordinated between Icebridge and the outer colonies.",
        ),
        ("penguin-civilization", "rival-routes") => (
            "Let the rival routes advance",
            "Let another aurora pass while competing colony routes keep redrawing cooperation beyond Icebridge.",
        ),
        ("penguin-civilization", "winter-commons") => (
            "Let the winter commons deepen",
            "Let another aurora pass through the shared systems that now carry Icebridge through the dark season.",
        ),
        ("penguin-civilization", "divided-houses") => (
            "Let the divided houses settle",
            "Let another aurora pass while Icebridge's winter houses keep organizing life around separate loyalties.",
        ),
        _ => (
            "Let this legacy carry on",
            "Let one more persistent change unfold inside the World this legacy has already shaped.",
        ),
    }
}

fn nudge_copy(
    seed: &str,
    generation: i64,
    relationship_choice_available: bool,
    intervention_choice_available: bool,
) -> (&'static str, &'static str) {
    if relationship_choice_available && intervention_choice_available {
        return (
            "Let it unfold without choosing",
            "Leave both open choices alone for now and let one more persistent change happen.",
        );
    }
    if relationship_choice_available {
        return (
            "Let it unfold without steering",
            "Skip the relationship choice for now and let the two actors keep finding their own direction.",
        );
    }
    if intervention_choice_available {
        return (
            "Let it unfold without intervening",
            "Leave the larger intervention alone for now and let existing dynamics keep working.",
        );
    }

    match (seed, generation) {
        ("mars-colony", 0) => (
            "Let the first sol unfold",
            "Watch Nia, Tomas, and Ares Habitat react before you steer anything.",
        ),
        ("1980s-town", 0) => (
            "Let the first night unfold",
            "Watch Lena, Max, and Maple Street find a rhythm before you steer anything.",
        ),
        ("penguin-civilization", 0) => (
            "Let the first aurora unfold",
            "Watch Piko, Miri, and Icebridge settle into motion before you steer anything.",
        ),
        ("mars-colony", 1) => (
            "See what the next sol changes",
            "Give the colony one more sol; its central relationship is starting to take shape.",
        ),
        ("1980s-town", 1) => (
            "See what the next night changes",
            "Give Maple Street one more night; its central relationship is starting to take shape.",
        ),
        ("penguin-civilization", 1) => (
            "See what the next aurora changes",
            "Give Icebridge one more aurora; its central relationship is starting to take shape.",
        ),
        (_, 0) => (
            "Let the first cycle unfold",
            "Watch the World move once before deciding how much to shape it.",
        ),
        (_, 1) => (
            "See what the next cycle changes",
            "Give the World one more cycle; its central relationship is starting to take shape.",
        ),
        _ => (
            "Let the world move",
            "Let one small, persistent change happen without making a larger choice.",
        ),
    }
}

fn briefing(world: &World, seeded: bool, since_event_count: Option<usize>) -> BriefingProjection {
    if !seeded {
        return BriefingProjection {
            eyebrow: "Pocket Universe".into(),
            title: "What kind of world should exist here?".into(),
            items: vec![
                BriefingItem {
                    selection: Some(SelectionId::Entity(UNIVERSE)),
                    title: "Create".into(),
                    detail: "Choose one seed. The choice becomes the first durable event in this World."
                        .into(),
                },
                BriefingItem {
                    selection: None,
                    title: "Keep · Grow · Return".into(),
                    detail: "Save it like a document, let time move, then come back to a world with history."
                        .into(),
                },
            ],
        };
    }

    if let Some(since) = since_event_count.filter(|since| *since < world.events().len()) {
        let events = &world.events()[since..];
        let mut items = return_digest_items(events);
        extend_with_persistent_consequences(world, &mut items);
        return BriefingProjection {
            eyebrow: format!("Pocket Universe · {}", seed_label(seed_id(world))),
            title: "While you were away".into(),
            items,
        };
    }

    let generation = integer_component(world, GENERATION).unwrap_or_default();
    let last_change = text_component(
        world.state().entity(UNIVERSE),
        LAST_CHANGE,
        "The world is quiet.",
    );
    let (relationship_choice_available, intervention_choice_available) =
        choice_state(world, generation);
    let posture_choice_available = posture_choice_state(world, generation);
    let (title, guidance) = if posture_choice_available {
        second_arc_stage_copy(seed_id(world))
    } else {
        live_stage_copy(
            seed_id(world),
            generation,
            relationship_choice_available,
            intervention_choice_available,
        )
    };
    let mut items = vec![BriefingItem {
        selection: Some(SelectionId::Entity(UNIVERSE)),
        title: "Current thread".into(),
        detail: last_change,
    }];
    if let Some((guidance_title, guidance_detail)) = guidance {
        items.push(BriefingItem {
            selection: None,
            title: guidance_title.into(),
            detail: guidance_detail.into(),
        });
    }
    items.extend(persistent_consequence_items(world));

    BriefingProjection {
        eyebrow: format!("Pocket Universe · {}", seed_label(seed_id(world))),
        title,
        items,
    }
}

fn live_stage_copy(
    seed: &str,
    generation: i64,
    relationship_choice_available: bool,
    intervention_choice_available: bool,
) -> (String, Option<(&'static str, &'static str)>) {
    if relationship_choice_available && intervention_choice_available {
        return (
            "Two choices are open".into(),
            Some((
                "Your turn · Shape the world",
                "You can steer the central relationship and make a larger intervention—or leave both alone and watch what happens.",
            )),
        );
    }
    if relationship_choice_available {
        return (
            "Their relationship is taking shape".into(),
            Some((
                "Your turn · Relationship",
                "Choose a shared project or rivalry—or leave them alone and let the World continue without steering.",
            )),
        );
    }
    if intervention_choice_available {
        let detail = match seed {
            "mars-colony" => {
                "A larger choice is ready: follow the rover signal or fortify the habitat. You can also leave the colony alone."
            }
            "1980s-town" => {
                "A larger choice is ready: turn the arcade into a community hub or keep it a steady business. You can also leave the town alone."
            }
            "penguin-civilization" => {
                "A larger choice is ready: open the Fish Vault for a feast or conserve the winter reserves. You can also leave Icebridge alone."
            }
            _ => "A larger intervention is available, but the World can keep moving without it.",
        };
        return (
            "A larger choice is here".into(),
            Some(("Your turn · Future", detail)),
        );
    }

    match generation {
        0 => {
            let detail = match seed {
                "mars-colony" => {
                    "Let the first sol unfold and see what Nia and Tomas do before deciding what this colony should become."
                }
                "1980s-town" => {
                    "Let the first night unfold and see how Lena and Max begin shaping Maple Street."
                }
                "penguin-civilization" => {
                    "Let the first aurora unfold and see how Piko and Miri settle into Icebridge."
                }
                _ => "Let the first cycle unfold before deciding how much to shape this World.",
            };
            ("The world is alive".into(), Some(("Next · Watch", detail)))
        }
        1 => (
            "Patterns are forming".into(),
            Some((
                "Next · Notice",
                "Let one more cycle pass. After that, you can steer the relationship at the center of this World.",
            )),
        ),
        _ => (format!("Generation {generation}"), None),
    }
}

fn persistent_consequence_items(world: &World) -> Vec<BriefingItem> {
    let mut items = Vec::new();
    let decision = text_component(world.state().entity(UNIVERSE), DECISION, "none");
    if let Some((title, detail)) = intervention_influence_copy(&decision) {
        items.push(BriefingItem {
            selection: Some(SelectionId::Entity(UNIVERSE)),
            title: title.into(),
            detail: detail.into(),
        });
    }
    if let Some(item) = relationship_consequence_item(world) {
        items.push(item);
    }
    if let Some(item) = posture_consequence_item(world) {
        items.push(item);
    }
    if let Some(item) = legacy_consequence_item(world) {
        items.push(item);
    }
    items
}

fn intervention_influence_copy(decision: &str) -> Option<(&'static str, &'static str)> {
    match decision {
        "follow-signal" => Some((
            "Your influence · Signal expedition",
            "Kestrel's signal expedition is still pulling the colony beyond the safe ridge.",
        )),
        "fortify-habitat" => Some((
            "Your influence · Fortified habitat",
            "Ares Habitat's stronger shell is making every later risk feel more deliberate.",
        )),
        "community-arcade" => Some((
            "Your influence · Community arcade",
            "Maple Arcade is becoming a place the neighborhood organizes its evenings around.",
        )),
        "steady-business" => Some((
            "Your influence · Steady business",
            "Maple Arcade is surviving by staying small, predictable, and open.",
        )),
        "winter-feast" => Some((
            "Your influence · Winter feast",
            "The feast is still turning Icebridge into a meeting point for distant colonies.",
        )),
        "conserve-reserves" => Some((
            "Your influence · Conserved reserves",
            "The sealed Fish Vault is still giving the council more room to plan for the dark season.",
        )),
        "none" => None,
        _ => Some((
            "Your influence",
            "An earlier intervention is still shaping what this World becomes.",
        )),
    }
}

fn posture_consequence_item(world: &World) -> Option<BriefingItem> {
    let posture = text_component(world.state().entity(UNIVERSE), POSTURE, "none");
    let seed = seed_id(world);
    let (title, detail) = match (seed, posture.as_str()) {
        (_, "none") => return None,
        ("mars-colony", "outward") => (
            "World direction · Outward",
            "Ares is carrying its next chapter beyond the familiar ridge. Nia keeps looking outward; Tomas still answers through the relationship they built.",
        ),
        ("mars-colony", "rooted") => (
            "World direction · Rooted",
            "Ares is deepening the home it already made. Nia keeps reinforcing it; Tomas still answers through the relationship they built.",
        ),
        ("1980s-town", "outward") => (
            "World direction · Outward",
            "Maple Street is widening its orbit. Lena keeps chasing new threads; Max still answers through the relationship they built.",
        ),
        ("1980s-town", "rooted") => (
            "World direction · Rooted",
            "Maple Street is deepening its local life. Lena keeps investing in familiar places; Max still answers through the relationship they built.",
        ),
        ("penguin-civilization", "outward") => (
            "World direction · Outward",
            "Icebridge is widening its colony network. Piko keeps looking beyond the bridge; Miri still answers through the relationship they built.",
        ),
        ("penguin-civilization", "rooted") => (
            "World direction · Rooted",
            "Icebridge is deepening winter life at home. Piko keeps reinforcing local systems; Miri still answers through the relationship they built.",
        ),
        (_, "outward") => (
            "World direction · Outward",
            "This World is carrying its next chapter toward unfamiliar edges.",
        ),
        (_, "rooted") => (
            "World direction · Rooted",
            "This World is deepening the home it has already made.",
        ),
        (_, _) => (
            "World direction",
            "A second-chapter choice is still shaping this World.",
        ),
    };
    Some(BriefingItem {
        selection: Some(SelectionId::Entity(UNIVERSE)),
        title: title.into(),
        detail: detail.into(),
    })
}

fn legacy_consequence_item(world: &World) -> Option<BriefingItem> {
    let legacy = text_component(world.state().entity(UNIVERSE), LEGACY, "forming");
    if legacy == "forming" {
        return None;
    }

    let latest_reinforcement = world
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "legacy_reinforced");
    let summary = latest_reinforcement
        .and_then(|event| match event.payload.get("summary") {
            Some(Value::Text(summary)) => Some(summary.clone()),
            _ => None,
        })
        .unwrap_or_else(|| {
            text_component(
                world.state().entity(UNIVERSE),
                LEGACY_SUMMARY,
                "This World now carries a durable legacy from its earlier choices.",
            )
        });
    let selection = latest_reinforcement
        .map(|event| SelectionId::Event(event.id))
        .or_else(|| {
            world
                .events()
                .iter()
                .rev()
                .find(|event| event.kind == "world_legacy_formed")
                .map(|event| SelectionId::Event(event.id))
        })
        .unwrap_or(SelectionId::Entity(UNIVERSE));
    Some(BriefingItem {
        selection: Some(selection),
        title: format!("World legacy · {}", legacy_label(&legacy)),
        detail: summary,
    })
}

fn legacy_label(legacy: &str) -> String {
    legacy
        .split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn relationship_consequence_item(world: &World) -> Option<BriefingItem> {
    let relationship = world.state().entity(RELATIONSHIP);
    let direction = text_component(relationship, RELATIONSHIP_DIRECTION, "none");
    let social_arc = text_component(relationship, RELATIONSHIP_SOCIAL_ARC, "forming");
    let trust = integer_entity_component(relationship, RELATIONSHIP_TRUST).unwrap_or_default();
    let tension = integer_entity_component(relationship, RELATIONSHIP_TENSION).unwrap_or_default();
    let last_dynamic = text_component(relationship, RELATIONSHIP_LAST_DYNAMIC, "");

    let (title, meaning) = match social_arc.as_str() {
        "partnership" => (
            "Partnership formed",
            "This relationship has resolved into a durable partnership.",
        ),
        "fracture" => (
            "Relationship fractured",
            "This relationship has resolved into a durable fracture.",
        ),
        "forming" if direction == "shared-project" => (
            "Relationship · Shared project",
            "Your shared-project choice is still shaping how they act together.",
        ),
        "forming" if direction == "rivalry" => (
            "Relationship · Rivalry",
            "Your rivalry choice is still adding pressure to how they respond to each other.",
        ),
        _ => return None,
    };
    let detail = if last_dynamic.trim().is_empty() {
        format!("{meaning} Trust {trust} · tension {tension}.")
    } else {
        format!("{meaning} Trust {trust} · tension {tension}. {last_dynamic}")
    };
    Some(BriefingItem {
        selection: Some(SelectionId::Entity(RELATIONSHIP)),
        title: title.into(),
        detail,
    })
}

fn integer_entity_component(entity: Option<&Entity>, key: &str) -> Option<i64> {
    match entity.and_then(|entity| entity.component(key)) {
        Some(Value::Integer(value)) => Some(*value),
        _ => None,
    }
}

fn return_digest_items(events: &[Event]) -> Vec<BriefingItem> {
    let mut groups = Vec::<(&Event, usize)>::new();
    for event in events
        .iter()
        .rev()
        .filter(|event| event.kind != "agent_decision_recorded")
    {
        if let Some((_, count)) = groups
            .iter_mut()
            .find(|(latest, _)| latest.kind == event.kind)
        {
            *count += 1;
        } else {
            groups.push((event, 1));
        }
    }

    groups
        .into_iter()
        .take(3)
        .map(|(event, occurrences)| return_item(event, occurrences))
        .collect()
}

fn extend_with_persistent_consequences(world: &World, items: &mut Vec<BriefingItem>) {
    let represented_events = items
        .iter()
        .filter_map(|item| match item.selection.as_ref() {
            Some(SelectionId::Event(event)) => Some(*event),
            _ => None,
        })
        .collect::<Vec<_>>();
    items.extend(
        persistent_consequence_items(world)
            .into_iter()
            .filter(|item| match item.selection.as_ref() {
                Some(SelectionId::Event(event)) => !represented_events.contains(event),
                _ => true,
            }),
    );
}

fn return_item(event: &Event, occurrences: usize) -> BriefingItem {
    let detail = ["change", "summary"]
        .into_iter()
        .find_map(|key| match event.payload.get(key) {
            Some(Value::Text(value)) => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_else(|| event.kind.replace('_', " "));
    let base_title: String = match event.kind.as_str() {
        "universe_grew" => "The world moved".into(),
        "universe_intervened" => "Your choice took hold".into(),
        "universe_seeded" => "A world began".into(),
        "agent_cared_for_world" => "Someone cared for the world".into(),
        "agent_explored_world" => "Someone explored beyond routine".into(),
        "relationship_shifted" => "Their relationship changed".into(),
        "relationship_steered" => "You steered their relationship".into(),
        "partnership_formed" => "A partnership formed".into(),
        "relationship_fractured" => "Their relationship fractured".into(),
        "world_legacy_formed" => "A world legacy formed".into(),
        "legacy_reinforced" => "A legacy reinforced itself".into(),
        _ => event.kind.replace('_', " "),
    };
    let title = if occurrences <= 1 {
        base_title
    } else {
        match event.kind.as_str() {
            "universe_grew" => format!("The world moved · {occurrences} cycles"),
            "legacy_reinforced" => {
                format!("A legacy reinforced itself · {occurrences} cycles")
            }
            "relationship_shifted" => {
                format!("Their relationship changed · {occurrences} times")
            }
            "agent_cared_for_world" => {
                format!("Someone cared for the world · {occurrences} times")
            }
            "agent_explored_world" => {
                format!("Someone explored beyond routine · {occurrences} times")
            }
            _ => format!("{base_title} · {occurrences} updates"),
        }
    };
    BriefingItem {
        selection: Some(SelectionId::Event(event.id)),
        title,
        detail,
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
                title: entity_title(entity),
                subtitle: entity.kind.replace('_', " "),
            })
            .collect(),
    }
}

fn canvas(world: &World) -> CanvasProjection {
    const POSITIONS: [(f32, f32); 6] = [
        (0.14, 0.24),
        (0.72, 0.22),
        (0.16, 0.78),
        (0.78, 0.74),
        (0.50, 0.48),
        (0.50, 0.82),
    ];
    let items = world
        .state()
        .entities()
        .filter(|entity| entity.id != UNIVERSE)
        .enumerate()
        .map(|(index, entity)| {
            let (x, y) = POSITIONS[index.min(POSITIONS.len() - 1)];
            CanvasItem {
                id: SelectionId::Entity(entity.id),
                kind: canvas_kind(entity),
                label: entity_title(entity),
                detail: entity.kind.replace('_', " "),
                x,
                y,
            }
        })
        .collect();
    CanvasProjection { items }
}

fn canvas_kind(entity: &Entity) -> CanvasItemKind {
    match entity.kind.as_str() {
        "person" | "penguin" => CanvasItemKind::Actor,
        "place" | "habitat" | "colony" => CanvasItemKind::Place,
        _ => CanvasItemKind::Object,
    }
}

fn universe_name(world: &World) -> String {
    world
        .state()
        .entity(UNIVERSE)
        .map(entity_title)
        .unwrap_or_else(|| "Pocket Universe".into())
}

fn integer_component(world: &World, key: &str) -> Option<i64> {
    match world
        .state()
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(key))
    {
        Some(Value::Integer(value)) => Some(*value),
        _ => None,
    }
}

fn text_component(entity: Option<&Entity>, key: &str, fallback: &str) -> String {
    match entity.and_then(|entity| entity.component(key)) {
        Some(Value::Text(value)) => value.clone(),
        _ => fallback.into(),
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

fn intervention_copy(seed: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    match seed {
        "mars-colony" => (
            "Follow the rover signal",
            "Send Kestrel beyond the safe ridge after a repeating signal.",
            "Fortify Ares Habitat",
            "Spend the colony's spare capacity sealing the habitat before the next dust front.",
        ),
        "1980s-town" => (
            "Make the arcade a community hub",
            "Keep Maple Arcade open late as a neighborhood club.",
            "Keep the arcade a steady business",
            "Protect its small cash buffer and avoid becoming the town's unofficial clubhouse.",
        ),
        "penguin-civilization" => (
            "Open the Fish Vault for a feast",
            "Invite distant colonies across Icebridge for a winter feast.",
            "Conserve the winter reserves",
            "Keep the Fish Vault sealed and plan for the dark season.",
        ),
        _ => (
            "Take the bold path",
            "Choose a visible change with uncertain consequences.",
            "Take the careful path",
            "Protect what already exists and reduce immediate risk.",
        ),
    }
}

#[cfg(test)]
mod first_story_copy_tests {
    use super::*;

    #[test]
    fn opening_cycles_are_seed_specific_before_relationship_agency_opens() {
        assert_eq!(
            nudge_copy("mars-colony", 0, false, false).0,
            "Let the first sol unfold"
        );
        assert_eq!(
            nudge_copy("1980s-town", 0, false, false).0,
            "Let the first night unfold"
        );
        assert_eq!(
            nudge_copy("penguin-civilization", 0, false, false).0,
            "Let the first aurora unfold"
        );
        assert_eq!(
            live_stage_copy("mars-colony", 0, false, false).0,
            "The world is alive"
        );
        assert_eq!(
            live_stage_copy("mars-colony", 1, false, false).0,
            "Patterns are forming"
        );
    }

    #[test]
    fn open_choices_are_explicit_and_always_optional() {
        assert_eq!(
            nudge_copy("mars-colony", 2, true, false).0,
            "Let it unfold without steering"
        );
        let relationship_stage = live_stage_copy("mars-colony", 2, true, false);
        assert_eq!(relationship_stage.0, "Their relationship is taking shape");
        assert!(relationship_stage.1.unwrap().1.contains("leave them alone"));

        assert_eq!(
            nudge_copy("mars-colony", 3, true, true).0,
            "Let it unfold without choosing"
        );
        let two_choices = live_stage_copy("mars-colony", 3, true, true);
        assert_eq!(two_choices.0, "Two choices are open");
        assert!(two_choices.1.unwrap().1.contains("leave both alone"));
    }

    #[test]
    fn intervention_guidance_keeps_each_seed_distinct() {
        let mars = live_stage_copy("mars-colony", 3, false, true).1.unwrap().1;
        let town = live_stage_copy("1980s-town", 3, false, true).1.unwrap().1;
        let penguins = live_stage_copy("penguin-civilization", 3, false, true)
            .1
            .unwrap()
            .1;
        assert!(mars.contains("rover signal"));
        assert!(town.contains("arcade"));
        assert!(penguins.contains("Fish Vault"));
    }
}
