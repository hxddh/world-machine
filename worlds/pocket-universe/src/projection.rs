use crate::drift;
use crate::era::{self, EraStanding};
use crate::narrator;
use crate::pressure::{self, PRESSURE_OUTCOME};
use crate::succession::{self, SuccessorStanding, SUCCESSION, SUCCESSION_OUTCOME};
use crate::{
    seed_id, BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND, DECISION, ENTRUST_LEGACY_COMMAND, GENERATION,
    HOLD_PRESSURE_COMMAND, LAST_CHANGE, LEGACY, LEGACY_CYCLES, LEGACY_SUMMARY, NUDGE_COMMAND,
    OUTWARD_POSTURE_COMMAND, POSTURE, POSTURE_GENERATION, REACH_PRESSURE_COMMAND,
    RECOVER_ANCHOR_COMMAND, RELATIONSHIP, RELATIONSHIP_DIRECTION, RELATIONSHIP_LAST_DYNAMIC,
    RELATIONSHIP_SOCIAL_ARC, RELATIONSHIP_TENSION, RELATIONSHIP_TRUST, RELEASE_LEGACY_COMMAND,
    RIVALRY_COMMAND, ROOTED_POSTURE_COMMAND, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND, SHARED_PROJECT_COMMAND, SLOT_A, UNIVERSE,
};
use world_core::{Entity, EntityId, Event, StateChange, Value, World};
use world_projection::{
    entity_title, inspectors_from_world, timeline_from_world, value_text, why_map_from_world,
    BriefingItem, BriefingItemKind, BriefingProjection, CanvasItem, CanvasItemKind,
    CanvasItemState, CanvasProjection, CollectionItem, CollectionProjection,
    ProjectionCapabilities, ProjectionCommand, ProjectionSnapshot, SelectionId,
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
                concerns: Vec::new(),
                id: SEED_MARS_COLONY_COMMAND.into(),
                title: "Start a Mars colony".into(),
                detail: "A tiny habitat, one keeper, hydroponics, and a rover on a red horizon."
                    .into(),
            },
            ProjectionCommand {
                concerns: Vec::new(),
                id: SEED_1980S_TOWN_COMMAND.into(),
                title: "Start a town in 1987".into(),
                detail: "An arcade, local radio, a night bus, and a neighborhood that remembers."
                    .into(),
            },
            ProjectionCommand {
                concerns: Vec::new(),
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
    let pressure_stage = pressure::pressure_id_from_state(world.state());
    let succession_nudge = succession_nudge_copy(world);
    let (nudge_title, nudge_detail) = if let Some(copy) = succession_nudge {
        copy
    } else if let Some(copy) = era_calm_nudge_copy(world) {
        copy
    } else if let Some(copy) = pressure_nudge_copy(&pressure_stage) {
        copy
    } else if posture_choice_available {
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
        concerns: Vec::new(),
        id: NUDGE_COMMAND.into(),
        title: nudge_title.into(),
        detail: command_detail_with_signal(world, NUDGE_COMMAND, nudge_detail),
    }];

    if relationship_choice_available {
        commands.push(ProjectionCommand {
            concerns: Vec::new(),
            id: SHARED_PROJECT_COMMAND.into(),
            title: "Give them a shared project".into(),
            detail: command_detail_with_signal(
                world,
                SHARED_PROJECT_COMMAND,
                "Create a goal that neither actor can complete alone; future interactions will lean toward trust.",
            ),
        });
        commands.push(ProjectionCommand {
            concerns: Vec::new(),
            id: RIVALRY_COMMAND.into(),
            title: "Let rivalry sharpen them".into(),
            detail: command_detail_with_signal(
                world,
                RIVALRY_COMMAND,
                "Keep both actors independent and let competition add pressure to future interactions.",
            ),
        });
    }
    if intervention_choice_available {
        let (bold_title, bold_detail, careful_title, careful_detail) =
            intervention_copy(seed_id(world));
        commands.push(ProjectionCommand {
            concerns: Vec::new(),
            id: BOLD_PATH_COMMAND.into(),
            title: bold_title.into(),
            detail: command_detail_with_signal(world, BOLD_PATH_COMMAND, bold_detail),
        });
        commands.push(ProjectionCommand {
            concerns: Vec::new(),
            id: CAREFUL_PATH_COMMAND.into(),
            title: careful_title.into(),
            detail: command_detail_with_signal(world, CAREFUL_PATH_COMMAND, careful_detail),
        });
    }
    if posture_choice_available {
        let (outward_title, outward_detail, rooted_title, rooted_detail) =
            posture_command_copy(seed_id(world));
        commands.push(ProjectionCommand {
            concerns: Vec::new(),
            id: OUTWARD_POSTURE_COMMAND.into(),
            title: outward_title.into(),
            detail: command_detail_with_signal(world, OUTWARD_POSTURE_COMMAND, outward_detail),
        });
        commands.push(ProjectionCommand {
            concerns: Vec::new(),
            id: ROOTED_POSTURE_COMMAND.into(),
            title: rooted_title.into(),
            detail: command_detail_with_signal(world, ROOTED_POSTURE_COMMAND, rooted_detail),
        });
    }
    let copy = pressure::copy_for_state(world.state());
    if pressure::window_open(&pressure_stage) {
        commands.push(ProjectionCommand {
            concerns: Vec::new(),
            id: HOLD_PRESSURE_COMMAND.into(),
            title: copy.hold_title.into(),
            detail: command_detail_with_signal(world, HOLD_PRESSURE_COMMAND, copy.hold_detail),
        });
        commands.push(ProjectionCommand {
            concerns: Vec::new(),
            id: REACH_PRESSURE_COMMAND.into(),
            title: copy.reach_title.into(),
            detail: command_detail_with_signal(world, REACH_PRESSURE_COMMAND, copy.reach_detail),
        });
    } else if pressure_stage == "lost" {
        commands.push(ProjectionCommand {
            concerns: Vec::new(),
            id: RECOVER_ANCHOR_COMMAND.into(),
            title: copy.recover_title.into(),
            detail: command_detail_with_signal(world, RECOVER_ANCHOR_COMMAND, copy.recover_detail),
        });
    }
    let succession_stage = succession::succession_id_from_state(world.state());
    if succession::choice_open(&succession_stage) {
        let succession_copy = succession::copy_for_seed(seed_id(world));
        commands.push(ProjectionCommand {
            concerns: Vec::new(),
            id: ENTRUST_LEGACY_COMMAND.into(),
            title: succession_copy.entrust_title.into(),
            detail: command_detail_with_signal(
                world,
                ENTRUST_LEGACY_COMMAND,
                succession_copy.entrust_detail,
            ),
        });
        commands.push(ProjectionCommand {
            concerns: Vec::new(),
            id: RELEASE_LEGACY_COMMAND.into(),
            title: succession_copy.release_title.into(),
            detail: command_detail_with_signal(
                world,
                RELEASE_LEGACY_COMMAND,
                succession_copy.release_detail,
            ),
        });
    }
    commands
}

/// While a successor is waiting, letting a cycle pass is itself a decision:
/// the copy says what waiting is costing.
/// The stretch after an era opens and before its threat arrives. There is
/// genuinely nothing to decide yet, and the copy should say so rather than fall
/// back to first-visit language.
fn era_calm_nudge_copy(world: &World) -> Option<(&'static str, &'static str)> {
    if era::era_from_state(world.state()) < 2 {
        return None;
    }
    if pressure::pressure_id_from_state(world.state()) != "none" {
        return None;
    }
    Some((
        "Let the quiet stretch run",
        "This era has not met its trouble yet. Nothing needs deciding; the World is simply living in the meantime.",
    ))
}

fn succession_nudge_copy(world: &World) -> Option<(&'static str, &'static str)> {
    let stage = succession::succession_id_from_state(world.state());
    if !succession::choice_open(&stage) {
        return None;
    }
    let patience = succession::succession_patience_from_state(world.state());
    Some(match succession::standing_for_patience(patience) {
        SuccessorStanding::NewHands => (
            "Let the successor keep learning",
            "Nothing is decided yet. Every cycle you wait, they do more of the work their own way.",
        ),
        SuccessorStanding::OwnHabits => (
            "Let their habits settle further",
            "They have started doing the work their own way. Waiting longer makes handing it on unchanged harder to mean.",
        ),
        SuccessorStanding::AlreadyTheirs => (
            "Leave it as it already is",
            "The work is theirs in all but name. Entrusting now would be a formality; releasing would only say so out loud.",
        ),
    })
}

/// How many eras turned while nobody was looking, and where that leaves the
/// World. A long absence used to read as a list of periods; the era it crossed
/// is the thing that actually happened in it.
fn eras_turned_item(world: &World, events: &[Event]) -> Option<BriefingItem> {
    let turned = events
        .iter()
        .filter(|event| event.kind == "era_began")
        .collect::<Vec<_>>();
    let latest = turned.last()?;
    let era = era::era_from_state(world.state());
    let left_during = era - turned.len() as i64;
    let summary = payload_text(latest, "summary").unwrap_or("");
    let title = if turned.len() == 1 {
        "An era turned".to_string()
    } else {
        format!("{} eras turned", turned.len())
    };
    Some(BriefingItem {
        concerns: Vec::new(),
        kind: BriefingItemKind::Beat,
        selection: Some(SelectionId::Event(latest.id)),
        title,
        detail: format!("You left during era {left_during}; this is era {era}. {summary}")
            .trim_end()
            .to_string(),
    })
}

/// What the World settled while nobody was answering. Only on a return digest:
/// on a fresh visit there is no "since" to have missed anything in.
fn decided_without_you_item(events: &[Event]) -> Option<BriefingItem> {
    let drifted = drift::drifted_decisions(events);
    let last = drifted.last()?;
    let note = drift::drift_note(last)?;
    let detail = match drifted.len() - 1 {
        0 => note.to_string(),
        1 => format!("{note} One other decision was reached the same way."),
        more => format!("{note} {more} other decisions were reached the same way."),
    };
    Some(BriefingItem {
        concerns: Vec::new(),
        kind: BriefingItemKind::Beat,
        selection: Some(SelectionId::Event(last.id)),
        title: "Decided without you".into(),
        detail,
    })
}

/// Which era this is, and what it inherited. Only once a World has had more
/// than one: saying "Era 1" to somebody on their first visit is noise.
fn era_item(world: &World) -> Option<BriefingItem> {
    let era = era::era_from_state(world.state());
    if era < 2 {
        return None;
    }
    let event = world
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "era_began")?;
    let inherited = match payload_text(event, "inherited") {
        Some("renewed") => "began again on its successor's terms",
        _ => "kept what it was handed",
    };
    let summary = payload_text(event, "summary").unwrap_or("").to_string();
    // A run of the same ending is a fact about the World, not about this era,
    // so it is read from the log here rather than recorded in the Event.
    let detail = match era::standing(&era::era_endings(world)) {
        EraStanding::Settled(run) => format!(
            "{summary} {run} eras running have handed their habits on unchanged; nobody keeping them now chose them."
        ),
        EraStanding::Restless(run) => format!(
            "{summary} {run} eras running have been rewritten; nothing here is older than one keeper."
        ),
        EraStanding::Mixed => summary,
    };
    Some(BriefingItem {
        concerns: Vec::new(),
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("Era {era} · {inherited}"),
        detail,
    })
}

fn succession_consequence_item(world: &World) -> Option<BriefingItem> {
    let succession = succession::succession_id_from_state(world.state());
    if succession == "none" {
        return None;
    }
    let event = world.events().iter().rev().find(|event| {
        matches!(
            event.kind.as_str(),
            "successor_emerged" | "successor_waited" | "legacy_entrusted" | "legacy_released"
        )
    })?;
    let summary = payload_text(event, "summary").unwrap_or("").to_string();
    let label = match event.kind.as_str() {
        "legacy_entrusted" => "Handed on".to_string(),
        "legacy_released" => "Rewritten".to_string(),
        _ => {
            let patience = succession::succession_patience_from_state(world.state());
            match succession::standing_for_patience(patience) {
                SuccessorStanding::NewHands => "New hands".to_string(),
                SuccessorStanding::OwnHabits => "Own habits".to_string(),
                SuccessorStanding::AlreadyTheirs => "Already theirs".to_string(),
            }
        }
    };
    Some(BriefingItem {
        concerns: Vec::new(),
        kind: BriefingItemKind::Beat,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("Succession · {label}"),
        detail: summary,
    })
}

fn succession_choice_evidence(event: &Event) -> Option<BriefingItem> {
    let stage = event_text_component(event, UNIVERSE, SUCCESSION)?;
    let outcome = event_text_component(event, UNIVERSE, SUCCESSION_OUTCOME)?;
    let inheritance = payload_text(event, "inheritance").unwrap_or("");
    let (label, follow_on) = match event.kind.as_str() {
        "legacy_entrusted" => (
            "Handed on",
            "The succession is settled. Later growth reads this durable answer.",
        ),
        "legacy_released" => (
            "Rewritten",
            "Legacy cycles were reset to 0; the successor's version has to earn its own.",
        ),
        _ => return None,
    };
    Some(BriefingItem {
        concerns: Vec::new(),
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("Choice evidence · {label}"),
        detail: format!(
            "Verified by this Event: succession = {stage}; outcome = {outcome}. {inheritance} {follow_on}"
        ),
    })
}

fn pressure_nudge_copy(pressure: &str) -> Option<(&'static str, &'static str)> {
    match pressure {
        "warning" => Some((
            "Watch the pressure build",
            "Let one more cycle pass without answering. The World will not wait forever.",
        )),
        "crisis" => Some((
            "Risk one more cycle",
            "Let one more cycle pass in crisis. If the window closes, what this World depends on is lost.",
        )),
        "lost" => Some((
            "Let the loss settle",
            "Let the World live with what it lost before deciding whether to recover it.",
        )),
        _ => None,
    }
}

fn command_detail_with_signal(world: &World, command_id: &str, detail: &str) -> String {
    match command_choice_signal(world, command_id) {
        Some(signal) => format!("{detail} Choice signal: {signal}."),
        None => detail.into(),
    }
}

fn command_choice_signal(world: &World, command_id: &str) -> Option<String> {
    match command_id {
        NUDGE_COMMAND => Some(
            "one full cycle resolves under current rules: world growth, both actor turns, relationship update, then period consequences"
                .into(),
        ),
        SHARED_PROJECT_COMMAND | RIVALRY_COMMAND => {
            let relationship = world.state().entity(RELATIONSHIP);
            let trust = integer_entity_component(relationship, RELATIONSHIP_TRUST).unwrap_or_default();
            let tension = integer_entity_component(relationship, RELATIONSHIP_TENSION).unwrap_or_default();
            if command_id == SHARED_PROJECT_COMMAND {
                let next_trust = (trust + 2).clamp(0, 10);
                let next_tension = (tension - 1).clamp(0, 10);
                Some(format!(
                    "trust {trust} → {next_trust} · tension {tension} → {next_tension}; each later relationship shift also gains +1 trust and -1 tension"
                ))
            } else {
                let next_tension = (tension + 2).clamp(0, 10);
                Some(format!(
                    "trust {trust} → {trust} · tension {tension} → {next_tension}; each later relationship shift also gains +1 tension"
                ))
            }
        }
        BOLD_PATH_COMMAND => Some(intervention_choice_signal(seed_id(world), true).into()),
        CAREFUL_PATH_COMMAND => Some(intervention_choice_signal(seed_id(world), false).into()),
        OUTWARD_POSTURE_COMMAND => Some(
            "sets durable World direction to Outward; later growth and legacy formation read the outward posture"
                .into(),
        ),
        ROOTED_POSTURE_COMMAND => Some(
            "sets durable World direction to Rooted; later growth and legacy formation read the rooted posture"
                .into(),
        ),
        ENTRUST_LEGACY_COMMAND | RELEASE_LEGACY_COMMAND => {
            let continued = command_id == ENTRUST_LEGACY_COMMAND;
            let outcome = if continued { "continued" } else { "renewed" };
            let cost = if continued {
                "legacy cycles keep accumulating"
            } else {
                "legacy cycles reset to 0 and the successor's version starts earning its own"
            };
            Some(format!(
                "settles the succession now; the durable outcome becomes {outcome}; {cost}"
            ))
        }
        HOLD_PRESSURE_COMMAND | REACH_PRESSURE_COMMAND => {
            let posture = text_component(world.state().entity(UNIVERSE), POSTURE, "none");
            let aligned = matches!(
                (command_id, posture.as_str()),
                (HOLD_PRESSURE_COMMAND, "rooted") | (REACH_PRESSURE_COMMAND, "outward")
            );
            let copy = pressure::copy_for_state(world.state());
            let status = if command_id == HOLD_PRESSURE_COMMAND {
                copy.hold_status
            } else {
                copy.reach_status
            };
            let fit = if aligned {
                "this answer fits the World's durable direction; the outcome is recorded as aligned"
            } else {
                "this answer runs against the World's durable direction; the outcome is recorded as strained"
            };
            Some(format!(
                "closes the pressure window now; the anchor's durable status becomes {status}; {fit}"
            ))
        }
        RECOVER_ANCHOR_COMMAND => {
            let copy = pressure::copy_for_state(world.state());
            Some(format!(
                "the anchor's durable status becomes {}; legacy cycles reset to 0 and the legacy must reinforce itself again",
                copy.recover_status
            ))
        }
        _ => None,
    }
}

fn intervention_choice_signal(seed: &str, bold: bool) -> &'static str {
    match (seed, bold) {
        ("mars-colony", true) => {
            "locks the first intervention to Signal expedition; Kestrel's durable status becomes signal expedition"
        }
        ("mars-colony", false) => {
            "locks the first intervention to Fortified habitat; Ares Habitat's durable status becomes storm sealed"
        }
        ("1980s-town", true) => {
            "locks the first intervention to Community arcade; Maple Arcade's durable status becomes community nights"
        }
        ("1980s-town", false) => {
            "locks the first intervention to Steady business; Maple Arcade's durable status becomes steady business"
        }
        ("penguin-civilization", true) => {
            "locks the first intervention to Winter feast; Fish Vault's durable reserve becomes festival opened"
        }
        ("penguin-civilization", false) => {
            "locks the first intervention to Conserved reserves; Fish Vault's durable reserve becomes winter conserved"
        }
        (_, true) => "locks a durable bold intervention that later growth can read",
        (_, false) => "locks a durable careful intervention that later growth can read",
    }
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

fn pressure_stage_copy(
    seed: &str,
    pressure: &str,
) -> Option<(String, Option<(&'static str, &'static str)>)> {
    let anchor = match seed {
        "mars-colony" => "Ares Habitat",
        "1980s-town" => "Maple Arcade",
        "penguin-civilization" => "The ice bridge",
        _ => "What this World depends on",
    };
    match pressure {
        "warning" => Some((
            "Pressure is rising".into(),
            Some((
                "Your turn · Hold or reach",
                match seed {
                    "mars-colony" => "The water reclaimer is failing. Rebuild it from what Ares has, send Kestrel for a replacement, or wait and see how far it slips.",
                    "1980s-town" => "The rent is rising past what the arcade earns. Fund it from the neighborhood, put its story on the air, or wait and see.",
                    "penguin-civilization" => "The third span is cracking. Rebuild it through the dark season, send for the outer builders, or wait and see.",
                    _ => "Something the World depends on is failing. Hold with what the World has, reach beyond it, or wait and see.",
                },
            )),
        )),
        "crisis" => Some((
            format!("{anchor} is in crisis"),
            Some((
                "Your turn · Decide before it is lost",
                "The window is closing. Hold or reach now; a few more cycles of waiting and this loss becomes durable.",
            )),
        )),
        "lost" => Some((
            "Something was lost".into(),
            Some((
                "Your turn · Recover",
                "The World lives with the loss now. Recovering is possible, but it costs the routines and legacy the World had built.",
            )),
        )),
        _ => None,
    }
}

fn pressure_consequence_item(world: &World) -> Option<BriefingItem> {
    let pressure = pressure::pressure_id_from_state(world.state());
    if pressure == "none" {
        return None;
    }
    let event = world.events().iter().rev().find(|event| {
        matches!(
            event.kind.as_str(),
            "pressure_rising"
                | "pressure_peaked"
                | "anchor_lost"
                | "pressure_held"
                | "pressure_reached"
                | "anchor_recovered"
        )
    })?;
    let summary = payload_text(event, "summary").unwrap_or("").to_string();
    let label = match pressure.as_str() {
        "warning" => "Rising",
        "crisis" => "Crisis",
        "lost" => "Lost",
        "held" => "Held",
        "reached" => "Reached",
        "recovered" => "Recovered",
        other => other,
    };
    Some(BriefingItem {
        concerns: Vec::new(),
        kind: BriefingItemKind::Beat,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("World pressure · {label}"),
        detail: summary,
    })
}

fn pressure_choice_evidence(world: &World, event: &Event) -> Option<BriefingItem> {
    let status = event_text_component(event, SLOT_A, "status")?;
    let outcome = event_text_component(event, UNIVERSE, PRESSURE_OUTCOME)?;
    let anchor = world
        .state()
        .entity(SLOT_A)
        .map(entity_title)
        .unwrap_or_else(|| "The anchor".into());
    let (label, follow_on) = match event.kind.as_str() {
        "pressure_held" => (
            "Held",
            "The pressure window is closed. Later growth reads this durable answer.",
        ),
        "pressure_reached" => (
            "Reached",
            "The pressure window is closed. Later growth reads this durable answer.",
        ),
        "anchor_recovered" => (
            "Recovered",
            "Legacy cycles were reset to 0; the legacy must reinforce itself again.",
        ),
        _ => return None,
    };
    Some(BriefingItem {
        concerns: Vec::new(),
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("Choice evidence · {label}"),
        detail: format!(
            "Verified by this Event: {anchor} · status = {status}; outcome = {outcome}. {follow_on}"
        ),
    })
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
                    concerns: Vec::new(),
                    kind: BriefingItemKind::Status,
                    selection: Some(SelectionId::Entity(UNIVERSE)),
                    title: "Create".into(),
                    detail: "Choose one seed. The choice becomes the first durable event in this World."
                        .into(),
                },
                BriefingItem {
                    concerns: Vec::new(),
                    kind: BriefingItemKind::Status,
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
        // What the World settled for itself comes before the routine churn: it
        // is the thing a returning observer least expects.
        if let Some(item) = decided_without_you_item(events) {
            items.insert(0, item);
        }
        // And an era that turned frames everything else, so it goes above even
        // that.
        if let Some(item) = eras_turned_item(world, events) {
            items.insert(0, item);
        }
        items.push(return_compass_item(world));
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
    let pressure_stage = pressure::pressure_id_from_state(world.state());
    let (title, guidance) = if let Some(copy) = pressure_stage_copy(seed_id(world), &pressure_stage)
    {
        copy
    } else if posture_choice_available {
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
        concerns: Vec::new(),
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Entity(UNIVERSE)),
        title: "Current thread".into(),
        detail: last_change,
    }];
    if let Some((guidance_title, guidance_detail)) = guidance {
        items.push(BriefingItem {
            concerns: Vec::new(),
            kind: BriefingItemKind::Status,
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
    if let Some(item) = choice_evidence_item(world) {
        items.push(item);
    }
    let decision = text_component(world.state().entity(UNIVERSE), DECISION, "none");
    if let Some((title, detail)) = intervention_influence_copy(&decision) {
        items.push(BriefingItem {
            concerns: Vec::new(),
            kind: BriefingItemKind::Status,
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
    if let Some(item) = pressure_consequence_item(world) {
        items.push(item);
    }
    if let Some(item) = succession_consequence_item(world) {
        items.push(item);
    }
    if let Some(item) = era_item(world) {
        items.push(item);
    }
    items
}

fn choice_evidence_item(world: &World) -> Option<BriefingItem> {
    let event_index = world.events().iter().rposition(|event| {
        matches!(
            event.kind.as_str(),
            "relationship_steered"
                | "universe_intervened"
                | "world_posture_chosen"
                | "pressure_held"
                | "pressure_reached"
                | "anchor_recovered"
                | "legacy_entrusted"
                | "legacy_released"
        )
    })?;
    let event = &world.events()[event_index];
    match event.kind.as_str() {
        "relationship_steered" => relationship_choice_evidence(world, event, event_index),
        "universe_intervened" => intervention_choice_evidence(world, event),
        "world_posture_chosen" => posture_choice_evidence(event),
        "pressure_held" | "pressure_reached" | "anchor_recovered" => {
            pressure_choice_evidence(world, event)
        }
        "legacy_entrusted" | "legacy_released" => succession_choice_evidence(event),
        _ => None,
    }
}

fn relationship_choice_evidence(
    world: &World,
    event: &Event,
    event_index: usize,
) -> Option<BriefingItem> {
    let direction = payload_text(event, "direction")?;
    let before_trust = integer_value(component_value_before_event(
        world.events(),
        event_index,
        RELATIONSHIP,
        RELATIONSHIP_TRUST,
    )?)?;
    let before_tension = integer_value(component_value_before_event(
        world.events(),
        event_index,
        RELATIONSHIP,
        RELATIONSHIP_TENSION,
    )?)?;
    let after_trust = event_integer_component(event, RELATIONSHIP, RELATIONSHIP_TRUST)?;
    let after_tension = event_integer_component(event, RELATIONSHIP, RELATIONSHIP_TENSION)?;
    let durable_direction = event_text_component(event, RELATIONSHIP, RELATIONSHIP_DIRECTION)?;
    let (label, follow_on) = match direction {
        "shared-project" => (
            "Shared project",
            "Later relationship shifts read this durable direction and add +1 trust and -1 tension.",
        ),
        "rivalry" => (
            "Rivalry",
            "Later relationship shifts read this durable direction and add +1 tension.",
        ),
        _ => (
            "Relationship",
            "Later relationship shifts continue reading this durable direction.",
        ),
    };
    Some(BriefingItem {
        concerns: Vec::new(),
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("Choice evidence · {label}"),
        detail: format!(
            "Verified by this Event: trust {before_trust} → {after_trust} · tension {before_tension} → {after_tension}. Durable direction = {}. {follow_on}",
            durable_direction.replace('-', " ")
        ),
    })
}

fn intervention_choice_evidence(world: &World, event: &Event) -> Option<BriefingItem> {
    let decision = event_text_component(event, UNIVERSE, DECISION)?;
    let label = intervention_influence_copy(&decision)
        .map(|(title, _)| {
            title
                .strip_prefix("Your influence · ")
                .unwrap_or(title)
                .to_string()
        })
        .unwrap_or_else(|| legacy_label(&decision));
    let effect = event.changes.iter().find_map(|change| match change {
        StateChange::SetComponent { entity, key, value } if *entity != UNIVERSE => {
            let target = world
                .state()
                .entity(*entity)
                .map(entity_title)
                .unwrap_or_else(|| format!("Entity #{entity}"));
            Some(format!(
                "{target} · {} = {}",
                key.replace('_', " "),
                value_text(value, world)
            ))
        }
        _ => None,
    })?;
    Some(BriefingItem {
        concerns: Vec::new(),
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("Choice evidence · {label}"),
        detail: format!(
            "Verified by this Event: first intervention = {label}; {effect}. Later growth reads this durable intervention."
        ),
    })
}

fn posture_choice_evidence(event: &Event) -> Option<BriefingItem> {
    let posture = event_text_component(event, UNIVERSE, POSTURE)?;
    let generation = event_integer_component(event, UNIVERSE, POSTURE_GENERATION)?;
    let label = match posture.as_str() {
        "outward" => "Outward",
        "rooted" => "Rooted",
        _ => "World direction",
    };
    Some(BriefingItem {
        concerns: Vec::new(),
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("Choice evidence · {label}"),
        detail: format!(
            "Verified by this Event: World direction = {label} at generation {generation}. Later growth and legacy formation read this durable posture."
        ),
    })
}

fn payload_text<'a>(event: &'a Event, key: &str) -> Option<&'a str> {
    match event.payload.get(key) {
        Some(Value::Text(value)) => Some(value.as_str()),
        _ => None,
    }
}

fn event_component_value(event: &Event, entity: EntityId, key: &str) -> Option<Value> {
    event.changes.iter().rev().find_map(|change| match change {
        StateChange::SetComponent {
            entity: changed_entity,
            key: changed_key,
            value,
        } if *changed_entity == entity && changed_key == key => Some(value.clone()),
        StateChange::RemoveComponent {
            entity: changed_entity,
            key: changed_key,
        } if *changed_entity == entity && changed_key == key => Some(Value::Null),
        _ => None,
    })
}

fn event_integer_component(event: &Event, entity: EntityId, key: &str) -> Option<i64> {
    integer_value(event_component_value(event, entity, key)?)
}

fn event_text_component(event: &Event, entity: EntityId, key: &str) -> Option<String> {
    match event_component_value(event, entity, key)? {
        Value::Text(value) => Some(value),
        _ => None,
    }
}

fn integer_value(value: Value) -> Option<i64> {
    match value {
        Value::Integer(value) => Some(value),
        _ => None,
    }
}

fn component_value_before_event(
    events: &[Event],
    event_index: usize,
    entity_id: EntityId,
    key: &str,
) -> Option<Value> {
    let mut current = None;
    for event in &events[..event_index] {
        for change in &event.changes {
            match change {
                StateChange::CreateEntity(entity) if entity.id == entity_id => {
                    current = entity.component(key).cloned();
                }
                StateChange::RemoveEntity(entity) if *entity == entity_id => current = None,
                StateChange::SetComponent {
                    entity,
                    key: changed_key,
                    value,
                } if *entity == entity_id && changed_key == key => {
                    current = Some(value.clone());
                }
                StateChange::RemoveComponent {
                    entity,
                    key: changed_key,
                } if *entity == entity_id && changed_key == key => current = None,
                _ => {}
            }
        }
    }
    current
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
        concerns: Vec::new(),
        kind: BriefingItemKind::Status,
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
        concerns: Vec::new(),
        kind: BriefingItemKind::Status,
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
        concerns: Vec::new(),
        kind: BriefingItemKind::Status,
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

fn return_compass_item(world: &World) -> BriefingItem {
    let generation = integer_component(world, GENERATION).unwrap_or_default();
    let (relationship_choice_available, intervention_choice_available) =
        choice_state(world, generation);
    let posture_choice_available = posture_choice_state(world, generation);
    let legacy = text_component(world.state().entity(UNIVERSE), LEGACY, "forming");
    let available = commands(world, true);
    let nudge = available.iter().find(|command| command.id == NUDGE_COMMAND);
    let shaping = available
        .iter()
        .filter(|command| command.id != NUDGE_COMMAND)
        .collect::<Vec<_>>();

    let pressure_stage = pressure::pressure_id_from_state(world.state());
    let title = if pressure::window_open(&pressure_stage) {
        "Your turn · Hold or reach"
    } else if pressure_stage == "lost" {
        "Your turn · Recover"
    } else if posture_choice_available {
        "Your turn · World direction"
    } else if relationship_choice_available && intervention_choice_available {
        "Your turn · Shape the world"
    } else if relationship_choice_available {
        "Your turn · Relationship"
    } else if intervention_choice_available {
        "Your turn · Future"
    } else if legacy != "forming" {
        "Next · Living legacy"
    } else {
        "Next · Continue"
    };

    let action_detail = match (shaping.is_empty(), nudge) {
        (true, Some(nudge)) => format!("Continue with ‘{}’. {}", nudge.title, nudge.detail),
        (false, Some(nudge)) => {
            let choices = shaping
                .iter()
                .map(|command| {
                    let signal =
                        command_choice_signal(world, command.id.as_str()).unwrap_or_else(|| {
                            "changes durable World state through this action".into()
                        });
                    format!("‘{}’ — {signal}", command.title)
                })
                .collect::<Vec<_>>()
                .join(" · ");
            let nudge_signal = command_choice_signal(world, nudge.id.as_str())
                .unwrap_or_else(|| "continues from the current durable state".into());
            format!(
                "Choice signals: {choices}. ‘{}’ — {nudge_signal}.",
                nudge.title
            )
        }
        (_, None) => "This World can continue from its current durable state.".into(),
    };
    let why_now = return_compass_context(
        world,
        generation,
        relationship_choice_available,
        intervention_choice_available,
        posture_choice_available,
        &legacy,
    );
    let detail = format!("Why now: {why_now} {action_detail}");

    BriefingItem {
        concerns: Vec::new(),
        kind: BriefingItemKind::Status,
        selection: None,
        title: title.into(),
        detail,
    }
}

fn return_compass_context(
    world: &World,
    generation: i64,
    relationship_choice_available: bool,
    intervention_choice_available: bool,
    posture_choice_available: bool,
    legacy: &str,
) -> String {
    let pressure_stage = pressure::pressure_id_from_state(world.state());
    if pressure::window_open(&pressure_stage) || pressure_stage == "lost" {
        return pressure_return_context(world, &pressure_stage);
    }
    if posture_choice_available {
        return posture_return_context(world);
    }
    if relationship_choice_available && intervention_choice_available {
        return format!(
            "{} {}",
            relationship_return_context(world),
            intervention_return_context(world, generation)
        );
    }
    if relationship_choice_available {
        return relationship_return_context(world);
    }
    if intervention_choice_available {
        return intervention_return_context(world, generation);
    }
    if legacy != "forming" {
        return legacy_return_context(world, legacy);
    }

    let last_change = text_component(
        world.state().entity(UNIVERSE),
        LAST_CHANGE,
        "The world is quiet.",
    );
    format!("Generation {generation} is still carrying its current thread: {last_change}")
}

fn pressure_return_context(world: &World, pressure: &str) -> String {
    let last_change = text_component(
        world.state().entity(UNIVERSE),
        LAST_CHANGE,
        "The world is quiet.",
    );
    match pressure {
        "warning" => format!("Pressure is rising and the window to answer is open. {last_change}"),
        "crisis" => format!("The pressure has peaked and the window is closing. {last_change}"),
        _ => format!(
            "The World has lost what it depended on; recovery is possible at a cost. {last_change}"
        ),
    }
}

fn relationship_return_context(world: &World) -> String {
    let relationship = world.state().entity(RELATIONSHIP);
    let trust = integer_entity_component(relationship, RELATIONSHIP_TRUST).unwrap_or_default();
    let tension = integer_entity_component(relationship, RELATIONSHIP_TENSION).unwrap_or_default();
    let last_dynamic = text_component(relationship, RELATIONSHIP_LAST_DYNAMIC, "");
    let dynamic = if last_dynamic.trim().is_empty() {
        String::new()
    } else {
        format!(" {last_dynamic}")
    };
    format!(
        "The central relationship is still forming at trust {trust} · tension {tension}.{dynamic} Its durable direction is still open."
    )
}

fn intervention_return_context(world: &World, generation: i64) -> String {
    let last_change = text_component(
        world.state().entity(UNIVERSE),
        LAST_CHANGE,
        "The world is quiet.",
    );
    format!(
        "Generation {generation} has reached a larger intervention point. Current thread: {last_change}"
    )
}

fn posture_return_context(world: &World) -> String {
    let social_arc = text_component(
        world.state().entity(RELATIONSHIP),
        RELATIONSHIP_SOCIAL_ARC,
        "forming",
    );
    let social_arc = match social_arc.as_str() {
        "partnership" => "partnership".into(),
        "fracture" => "fracture".into(),
        other => other.replace('-', " "),
    };
    let decision = text_component(world.state().entity(UNIVERSE), DECISION, "none");
    let influence = intervention_influence_copy(&decision)
        .map(|(title, _)| {
            title
                .strip_prefix("Your influence · ")
                .unwrap_or(title)
                .to_string()
        })
        .unwrap_or_else(|| decision.replace('-', " "));
    format!(
        "The first arc has settled as {social_arc}, and {influence} is already durable. The next choice now decides whether this World reaches outward or deepens home."
    )
}

fn legacy_return_context(world: &World, legacy: &str) -> String {
    let cycles = integer_component(world, LEGACY_CYCLES).unwrap_or_default();
    let legacy = legacy_label(legacy);
    if cycles <= 0 {
        let summary = text_component(
            world.state().entity(UNIVERSE),
            LEGACY_SUMMARY,
            "This World now carries a durable legacy from its earlier choices.",
        );
        return format!(
            "World legacy · {legacy} has formed but has not yet reinforced through a later cycle. {summary}"
        );
    }

    let pattern = world
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "legacy_reinforced")
        .and_then(|event| match event.payload.get("pattern") {
            Some(Value::Text(pattern)) => Some(pattern.as_str()),
            _ => None,
        });
    let cycle_word = if cycles == 1 { "cycle" } else { "cycles" };
    match pattern {
        Some(pattern) => format!(
            "World legacy · {legacy} has reinforced through {cycles} later {cycle_word}; its current pattern is {pattern}. Another continuation feeds that established pattern back into the World."
        ),
        None => format!(
            "World legacy · {legacy} has reinforced through {cycles} later {cycle_word}. Another continuation feeds that established pattern back into the World."
        ),
    }
}

/// The Events a return digest will show, and how many of that kind it stands
/// for. Shared with the narrator so that what gets put into words is exactly
/// what gets read, and the two can never drift apart.
pub(crate) fn digest_events(events: &[Event]) -> Vec<(&Event, usize)> {
    let mut groups = Vec::<(&Event, usize)>::new();
    for event in events.iter().rev().filter(|event| {
        // Agent plumbing is not news, and a narrated line is not an event of
        // its own: it is how the event it re-words gets read.
        event.kind != "agent_decision_recorded" && event.kind != narrator::NARRATED
    }) {
        if let Some((_, count)) = groups
            .iter_mut()
            .find(|(latest, _)| latest.kind == event.kind)
        {
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

fn return_digest_items(events: &[Event]) -> Vec<BriefingItem> {
    digest_events(events)
        .into_iter()
        .map(|(event, occurrences)| return_item(events, event, occurrences))
        .collect()
}

fn return_digest_priority(kind: &str) -> u8 {
    match kind {
        "universe_seeded"
        | "universe_intervened"
        | "relationship_steered"
        | "partnership_formed"
        | "relationship_fractured"
        | "world_legacy_formed"
        | "pressure_rising"
        | "pressure_peaked"
        | "anchor_lost"
        | "pressure_held"
        | "pressure_reached"
        | "anchor_recovered" => 0,
        _ => 1,
    }
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

fn return_item(events: &[Event], event: &Event, occurrences: usize) -> BriefingItem {
    // This World's own words if it has them for this Event, and the table line
    // otherwise. The table is the floor: a World with no narrator, or one whose
    // narrator said nothing usable, reads exactly as it always did.
    let detail = narrator::narrated_text(events, event.id)
        .map(str::to_owned)
        .or_else(|| {
            ["change", "summary"]
                .into_iter()
                .find_map(|key| match event.payload.get(key) {
                    Some(Value::Text(value)) => Some(value.clone()),
                    _ => None,
                })
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
        "pressure_rising" => "Pressure is rising".into(),
        "pressure_peaked" => "The pressure peaked".into(),
        "anchor_lost" => "Something was lost".into(),
        "pressure_held" => "You held through the pressure".into(),
        "pressure_reached" => "You reached beyond the pressure".into(),
        "anchor_recovered" => "You recovered what was lost".into(),
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
        concerns: Vec::new(),
        kind: BriefingItemKind::Beat,
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
                at: None,
                state: CanvasItemState::Working,
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
