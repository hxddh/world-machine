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
    SEED_PENGUIN_CIVILIZATION_COMMAND, SHARED_PROJECT_COMMAND, SLOT_A, SLOT_B, SLOT_C, SLOT_D,
    SLOT_E, UNIVERSE,
};
use std::collections::BTreeMap;
use world_core::{Entity, EntityId, Event, StateChange, Value, World};
use world_projection::{
    entity_title, inspectors_from_world, timeline_from_world, value_text, why_map_from_world,
    BriefingItem, BriefingItemKind, BriefingProjection, CanvasChange, CanvasItem, CanvasItemKind,
    CanvasLink, CanvasLinkTone, CanvasProjection, CollectionItem, CollectionProjection,
    CommandEffect, EffectChange, InspectorProjection, InspectorRow, InspectorSection,
    ProjectionCapabilities, ProjectionCommand, ProjectionSnapshot, SelectionId, Tone,
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
        },
        briefing: Some(toned(world, briefing(world, seeded, since_event_count))),
        commands: commands(world, seeded)
            .into_iter()
            .map(|mut command| {
                command.effects = command_effects(world, &command.id);
                command.asker = asker(&command.id);
                command
            })
            .collect(),
        collection: collection(world),
        timeline: told_timeline(world),
        canvas: with_changes(world, canvas(world), since_event_count),
        inspectors: told_inspectors(world),
        why: why_map_from_world(world),
        scenery: seeded.then(|| seed_scenery(seed_id(world))).flatten(),
        calendar: seeded.then(|| world_projection::Calendar {
            unit: seed_time_unit(seed_id(world)).into(),
            length: crate::BACKGROUND_PERIOD,
        }),
        gauges: gauges(world),
    };
    snapshot.tell_events_as_history_does();
    snapshot
}

/// Good news, bad news, or neither, for each briefing line, read from the
/// kind of event the line reports.
fn toned(world: &World, mut briefing: BriefingProjection) -> BriefingProjection {
    for item in &mut briefing.items {
        item.tone = match item.selection {
            Some(SelectionId::Event(id)) => world
                .events()
                .iter()
                .find(|event| event.id == id)
                .map(|event| tone_for_event(&event.kind))
                .unwrap_or_default(),
            Some(SelectionId::Entity(RELATIONSHIP)) => {
                let relationship = world.state().entity(RELATIONSHIP);
                match text_component(relationship, RELATIONSHIP_SOCIAL_ARC, "forming").as_str() {
                    "partnership" => Tone::Good,
                    "fracture" => Tone::Bad,
                    _ => Tone::Neutral,
                }
            }
            _ => Tone::Neutral,
        };
    }
    briefing
}

fn tone_for_event(kind: &str) -> Tone {
    match kind {
        "pressure_rising" => Tone::Warning,
        "pressure_peaked" | "anchor_lost" | "relationship_fractured" => Tone::Bad,
        "pressure_held"
        | "pressure_reached"
        | "anchor_recovered"
        | "partnership_formed"
        | "world_legacy_formed"
        | "legacy_reinforced" => Tone::Good,
        _ => Tone::Neutral,
    }
}

/// What each choice would change, as facts a screen can show beside it and
/// point at on the scene before the choice is made.
/// Whose choice it is to put to the player. A choice about the pair is the
/// pair's; a bold or outward one is the explorer's, who would go; a careful
/// or rooted one the keeper's, who would stay. Letting time pass is nobody's.
fn asker(command_id: &str) -> Option<SelectionId> {
    let who = match command_id {
        SHARED_PROJECT_COMMAND | RIVALRY_COMMAND => RELATIONSHIP,
        BOLD_PATH_COMMAND | OUTWARD_POSTURE_COMMAND | REACH_PRESSURE_COMMAND => SLOT_E,
        CAREFUL_PATH_COMMAND
        | ROOTED_POSTURE_COMMAND
        | HOLD_PRESSURE_COMMAND
        | RECOVER_ANCHOR_COMMAND
        | ENTRUST_LEGACY_COMMAND
        | RELEASE_LEGACY_COMMAND => SLOT_B,
        _ => return None,
    };
    Some(SelectionId::Entity(who))
}

fn command_effects(world: &World, command_id: &str) -> Vec<CommandEffect> {
    let effect =
        |target: Option<EntityId>, label: &str, change: EffectChange, tone: Tone| CommandEffect {
            target: target.map(SelectionId::Entity),
            label: label.into(),
            change,
            tone,
        };
    let anchor = world
        .state()
        .entity(SLOT_A)
        .map(entity_title)
        .unwrap_or_else(|| "The anchor".into());
    let pressure = pressure::pressure_id_from_state(world.state());
    match command_id {
        NUDGE_COMMAND if pressure::window_open(&pressure) => vec![effect(
            Some(SLOT_A),
            &anchor,
            EffectChange::To("trouble grows".into()),
            Tone::Warning,
        )],
        SHARED_PROJECT_COMMAND => vec![
            effect(Some(RELATIONSHIP), "Trust", EffectChange::Up, Tone::Good),
            effect(
                Some(RELATIONSHIP),
                "Tension",
                EffectChange::Down,
                Tone::Good,
            ),
        ],
        RIVALRY_COMMAND => vec![effect(
            Some(RELATIONSHIP),
            "Tension",
            EffectChange::Up,
            Tone::Warning,
        )],
        BOLD_PATH_COMMAND | CAREFUL_PATH_COMMAND => {
            let bold = command_id == BOLD_PATH_COMMAND;
            crate::intervention_plan(seed_id(world), bold)
                .and_then(|(_, _, target, _, value)| {
                    let name = world.state().entity(target).map(entity_title)?;
                    Some(vec![effect(
                        Some(target),
                        &name,
                        EffectChange::To(value.into()),
                        Tone::Neutral,
                    )])
                })
                .unwrap_or_default()
        }
        OUTWARD_POSTURE_COMMAND => vec![effect(
            None,
            "World direction",
            EffectChange::To("outward".into()),
            Tone::Neutral,
        )],
        ROOTED_POSTURE_COMMAND => vec![effect(
            None,
            "World direction",
            EffectChange::To("rooted".into()),
            Tone::Neutral,
        )],
        HOLD_PRESSURE_COMMAND | REACH_PRESSURE_COMMAND => {
            let hold = command_id == HOLD_PRESSURE_COMMAND;
            let copy = pressure::copy_for_state(world.state());
            let posture = text_component(world.state().entity(UNIVERSE), POSTURE, "none");
            let fits = matches!(
                (hold, posture.as_str()),
                (true, "rooted") | (false, "outward")
            );
            vec![
                effect(
                    Some(SLOT_A),
                    &anchor,
                    EffectChange::To(
                        if hold {
                            copy.hold_status
                        } else {
                            copy.reach_status
                        }
                        .into(),
                    ),
                    Tone::Good,
                ),
                effect(
                    None,
                    if fits {
                        "Fits the World's direction"
                    } else {
                        "Against the World's direction"
                    },
                    EffectChange::To(if fits { "aligned" } else { "strained" }.into()),
                    if fits { Tone::Good } else { Tone::Warning },
                ),
            ]
        }
        RECOVER_ANCHOR_COMMAND => {
            let copy = pressure::copy_for_state(world.state());
            vec![
                effect(
                    Some(SLOT_A),
                    &anchor,
                    EffectChange::To(copy.recover_status.into()),
                    Tone::Good,
                ),
                effect(
                    None,
                    "Legacy",
                    EffectChange::To("starts over".into()),
                    Tone::Warning,
                ),
            ]
        }
        ENTRUST_LEGACY_COMMAND => vec![effect(
            None,
            "Legacy",
            EffectChange::To("carries on".into()),
            Tone::Good,
        )],
        RELEASE_LEGACY_COMMAND => vec![effect(
            None,
            "Legacy",
            EffectChange::To("starts over".into()),
            Tone::Warning,
        )],
        _ => Vec::new(),
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
                effects: Vec::new(),
                scenery: seed_scenery("mars-colony"),
                asker: None,
                moves: Vec::new(),
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
        id: NUDGE_COMMAND.into(),
        title: nudge_title.into(),
        detail: String::from(nudge_detail),
        effects: Vec::new(),
        scenery: None,
        asker: None,
        moves: Vec::new(),
    }];

    if relationship_choice_available {
        commands.push(ProjectionCommand {
            id: SHARED_PROJECT_COMMAND.into(),
            title: "Give them a shared project".into(),
            detail: String::from("Give them something neither can finish alone. From here on they lean toward trusting each other."), effects: Vec::new(),
            scenery: None, asker: None, moves: Vec::new(),
});
        commands.push(ProjectionCommand {
            id: RIVALRY_COMMAND.into(),
            title: "Let rivalry sharpen them".into(),
            detail: String::from("Keep them apart and let competition sharpen how they deal with each other from now on."), effects: Vec::new(),
            scenery: None, asker: None, moves: Vec::new(),
});
    }
    if intervention_choice_available {
        let (bold_title, bold_detail, careful_title, careful_detail) =
            intervention_copy(seed_id(world));
        commands.push(ProjectionCommand {
            id: BOLD_PATH_COMMAND.into(),
            title: bold_title.into(),
            detail: String::from(bold_detail),
            effects: Vec::new(),
            scenery: None,
            asker: None,
            moves: Vec::new(),
        });
        commands.push(ProjectionCommand {
            id: CAREFUL_PATH_COMMAND.into(),
            title: careful_title.into(),
            detail: String::from(careful_detail),
            effects: Vec::new(),
            scenery: None,
            asker: None,
            moves: Vec::new(),
        });
    }
    if posture_choice_available {
        let (outward_title, outward_detail, rooted_title, rooted_detail) =
            posture_command_copy(seed_id(world));
        commands.push(ProjectionCommand {
            id: OUTWARD_POSTURE_COMMAND.into(),
            title: outward_title.into(),
            detail: String::from(outward_detail),
            effects: Vec::new(),
            scenery: None,
            asker: None,
            moves: Vec::new(),
        });
        commands.push(ProjectionCommand {
            id: ROOTED_POSTURE_COMMAND.into(),
            title: rooted_title.into(),
            detail: String::from(rooted_detail),
            effects: Vec::new(),
            scenery: None,
            asker: None,
            moves: Vec::new(),
        });
    }
    let copy = pressure::copy_for_state(world.state());
    if pressure::window_open(&pressure_stage) {
        commands.push(ProjectionCommand {
            id: HOLD_PRESSURE_COMMAND.into(),
            title: copy.hold_title.into(),
            detail: String::from(copy.hold_detail),
            effects: Vec::new(),
            scenery: None,
            asker: None,
            moves: Vec::new(),
        });
        commands.push(ProjectionCommand {
            id: REACH_PRESSURE_COMMAND.into(),
            title: copy.reach_title.into(),
            detail: String::from(copy.reach_detail),
            effects: Vec::new(),
            scenery: None,
            asker: None,
            moves: Vec::new(),
        });
    } else if pressure_stage == "lost" {
        commands.push(ProjectionCommand {
            id: RECOVER_ANCHOR_COMMAND.into(),
            title: copy.recover_title.into(),
            detail: String::from(copy.recover_detail),
            effects: Vec::new(),
            scenery: None,
            asker: None,
            moves: Vec::new(),
        });
    }
    let succession_stage = succession::succession_id_from_state(world.state());
    if succession::choice_open(&succession_stage) {
        let succession_copy = succession::copy_for_seed(seed_id(world));
        commands.push(ProjectionCommand {
            id: ENTRUST_LEGACY_COMMAND.into(),
            title: succession_copy.entrust_title.into(),
            detail: String::from(succession_copy.entrust_detail),
            effects: Vec::new(),
            scenery: None,
            asker: None,
            moves: Vec::new(),
        });
        commands.push(ProjectionCommand {
            id: RELEASE_LEGACY_COMMAND.into(),
            title: succession_copy.release_title.into(),
            detail: String::from(succession_copy.release_detail),
            effects: Vec::new(),
            scenery: None,
            asker: None,
            moves: Vec::new(),
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
            "Nothing is decided yet. The longer you wait, the more they do the work their own way.",
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
        kind: BriefingItemKind::Beat,
        selection: Some(SelectionId::Event(latest.id)),
        title,
        detail: format!("You left during era {left_during}; this is era {era}. {summary}")
            .trim_end()
            .to_string(),
        tone: world_projection::Tone::Neutral,
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
        kind: BriefingItemKind::Beat,
        selection: Some(SelectionId::Event(last.id)),
        title: "Decided without you".into(),
        detail,
        tone: world_projection::Tone::Neutral,
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
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("Era {era} · {inherited}"),
        detail,
        tone: world_projection::Tone::Neutral,
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
        kind: BriefingItemKind::Beat,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("Succession · {label}"),
        detail: summary,
        tone: world_projection::Tone::Neutral,
    })
}

fn succession_choice_evidence(event: &Event) -> Option<BriefingItem> {
    event_text_component(event, UNIVERSE, SUCCESSION)?;
    event_text_component(event, UNIVERSE, SUCCESSION_OUTCOME)?;
    let inheritance = payload_text(event, "inheritance").unwrap_or("");
    let (label, follow_on) = match event.kind.as_str() {
        "legacy_entrusted" => (
            "Handed on",
            "The succession is settled, and what grows next builds on it.",
        ),
        "legacy_released" => (
            "Rewritten",
            "The old legacy was set aside; the successor's version has to earn its own.",
        ),
        _ => return None,
    };
    Some(BriefingItem {
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("You chose · {label}"),
        detail: format!("{inheritance} {follow_on}").trim().to_string(),
        tone: world_projection::Tone::Neutral,
    })
}

fn pressure_nudge_copy(pressure: &str) -> Option<(&'static str, &'static str)> {
    match pressure {
        "warning" => Some((
            "Watch the pressure build",
            "Wait without answering. The World will not wait forever.",
        )),
        "crisis" => Some((
            "Risk waiting longer",
            "Wait while the crisis runs. If the window closes, what this World depends on is lost.",
        )),
        "lost" => Some((
            "Let the loss settle",
            "Let the World live with what it lost before deciding whether to recover it.",
        )),
        _ => None,
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
            "The first chapter has settled. Decide whether the next one reaches outward or deepens the home this World already made."
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
                "The window is closing. Hold or reach now; wait much longer and the loss will be permanent.",
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
        kind: BriefingItemKind::Beat,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("World pressure · {label}"),
        detail: summary,
        tone: world_projection::Tone::Neutral,
    })
}

fn pressure_outcome_sentence(outcome: &str) -> &'static str {
    match outcome {
        "aligned" => "The answer fit the direction this World had chosen.",
        "strained" => "The answer ran against the direction this World had chosen, and it shows.",
        _ => "",
    }
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
            "The trouble is answered, and what grows next builds on it.",
        ),
        "pressure_reached" => (
            "Reached",
            "The trouble is answered, and what grows next builds on it.",
        ),
        "anchor_recovered" => (
            "Recovered",
            "The legacy has to prove itself all over again.",
        ),
        _ => return None,
    };
    Some(BriefingItem {
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("You chose · {label}"),
        detail: format!(
            "{anchor} is now {status}. {} {follow_on}",
            pressure_outcome_sentence(&outcome)
        )
        .replace("  ", " "),
        tone: world_projection::Tone::Neutral,
    })
}

fn legacy_nudge_copy(seed: &str, legacy: &str) -> (&'static str, &'static str) {
    match (seed, legacy) {
        ("mars-colony", "ridge-network") => (
            "Let the ridge network carry on",
            "Let another sol move through the ridge routes and see what the expedition network changes next.",
        ),
        ("mars-colony", "competing-frontiers") => (
            "Let the competing frontiers advance",
            "Let another sol pass while rival survey routes keep defining different edges of Ares.",
        ),
        ("mars-colony", "habitat-commons") => (
            "Let the habitat commons deepen",
            "Let another sol move through the commons and see what shared life inside Ares builds next.",
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
            "Let a little more time pass in the World this legacy has already shaped.",
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
            "Leave both choices open for now and let a little more time pass.",
        );
    }
    if relationship_choice_available {
        return (
            "Let it unfold without steering",
            "Leave them to it for now; the two of them keep finding their own way.",
        );
    }
    if intervention_choice_available {
        return match seed {
            "mars-colony" => (
                "Not yet",
                "Leave the decision open; Nia and Tomas carry on as they are for another sol.",
            ),
            "1980s-town" => (
                "Not yet",
                "Leave the decision open; Lena and Max carry on as they are for another night.",
            ),
            "penguin-civilization" => (
                "Not yet",
                "Leave the decision open; Piko and Miri carry on as they are for another aurora.",
            ),
            _ => (
                "Not yet",
                "Leave the decision open and let everyone carry on as they are.",
            ),
        };
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
            "Let time begin",
            "Watch the World move once before deciding how much to shape it.",
        ),
        (_, 1) => (
            "See what comes next",
            "Give the World a little more time; its central relationship is starting to take shape.",
        ),
        _ => (
            "Let the world move",
            "Let a little more time pass without making a bigger choice.",
        ),
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
            returned: true,
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
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Entity(UNIVERSE)),
        title: "Current thread".into(),
        detail: last_change,
        tone: world_projection::Tone::Neutral,
    }];
    if let Some((guidance_title, guidance_detail)) = guidance {
        items.push(BriefingItem {
            kind: BriefingItemKind::Status,
            selection: None,
            title: guidance_title.into(),
            detail: guidance_detail.into(),
            tone: world_projection::Tone::Neutral,
        });
    }
    items.extend(persistent_consequence_items(world));

    BriefingProjection {
        eyebrow: format!("Pocket Universe · {}", seed_label(seed_id(world))),
        title,
        items,
        returned: false,
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
                "Steer what happens between the two of them, decide where this place goes next, or leave both alone and watch.",
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
        let (title, detail) = match seed {
            "mars-colony" => (
                "Ares has a decision to make",
                "Follow the rover signal or fortify the habitat, or leave the colony as it is.",
            ),
            "1980s-town" => (
                "Maple Street has a decision to make",
                "Turn the arcade into a community hub or keep it a steady business, or leave the town as it is.",
            ),
            "penguin-civilization" => (
                "Icebridge has a decision to make",
                "Open the Fish Vault for a feast or save the winter reserves, or leave Icebridge as it is.",
            ),
            _ => (
                "A decision is waiting",
                "Decide where this place goes next, or leave it as it is.",
            ),
        };
        return (title.into(), Some(("Your turn · Future", detail)));
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
                _ => "Watch it move once before deciding how much to shape this World.",
            };
            let title = match seed {
                "mars-colony" => "Ares is waking up",
                "1980s-town" => "Maple Street is waking up",
                "penguin-civilization" => "Icebridge is waking up",
                _ => "A World is waking up",
            };
            (title.into(), Some(("Next · Watch", detail)))
        }
        1 => (
            "They are finding their rhythm".into(),
            Some((
                "Next · Notice",
                "Give it a little more time. After that, you can steer the relationship at the center of this World.",
            )),
        ),
        // Past the opening chapter with nothing open: the World simply goes
        // on. The generation number is the engine's count, not a headline.
        _ => ("Life goes on".into(), None),
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
            kind: BriefingItemKind::Status,
            selection: Some(SelectionId::Entity(UNIVERSE)),
            title: title.into(),
            detail: detail.into(),
            tone: world_projection::Tone::Neutral,
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

fn relationship_change_sentence(
    before_trust: i64,
    after_trust: i64,
    before_tension: i64,
    after_tension: i64,
) -> String {
    let part = |name: &str, before: i64, after: i64| {
        if before == after {
            format!("{name} held at {after}")
        } else {
            format!("{name} went from {before} to {after}")
        }
    };
    format!(
        "{} and {}.",
        part("Trust", before_trust, after_trust),
        part("tension", before_tension, after_tension)
    )
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
    event_text_component(event, RELATIONSHIP, RELATIONSHIP_DIRECTION)?;
    let (label, follow_on) = match direction {
        "shared-project" => (
            "Shared project",
            "From here on, each time they shift, it leans toward trust.",
        ),
        "rivalry" => (
            "Rivalry",
            "From here on, each time they shift, it leans toward tension.",
        ),
        _ => (
            "Relationship",
            "Later shifts between them keep following this direction.",
        ),
    };
    Some(BriefingItem {
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("You chose · {label}"),
        detail: format!(
            "{} {follow_on}",
            relationship_change_sentence(before_trust, after_trust, before_tension, after_tension)
        ),
        tone: world_projection::Tone::Neutral,
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
        StateChange::SetComponent { entity, value, .. } if *entity != UNIVERSE => {
            let target = world
                .state()
                .entity(*entity)
                .map(entity_title)
                .unwrap_or_else(|| format!("Entity #{entity}"));
            Some(format!("{target} is now {}.", value_text(value, world)))
        }
        _ => None,
    })?;
    Some(BriefingItem {
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("You chose · {label}"),
        detail: format!("{effect} What grows next builds on it."),
        tone: world_projection::Tone::Neutral,
    })
}

fn posture_choice_evidence(event: &Event) -> Option<BriefingItem> {
    let posture = event_text_component(event, UNIVERSE, POSTURE)?;
    event_integer_component(event, UNIVERSE, POSTURE_GENERATION)?;
    let label = match posture.as_str() {
        "outward" => "Outward",
        "rooted" => "Rooted",
        _ => "World direction",
    };
    Some(BriefingItem {
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Event(event.id)),
        title: format!("You chose · {label}"),
        detail: format!(
            "This World now leans {}. What grows next, and what it leaves behind, follows that direction.",
            label.to_lowercase()
        ), tone: world_projection::Tone::Neutral,
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
            "An earlier choice is still shaping what this World becomes.",
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
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Entity(UNIVERSE)),
        title: title.into(),
        detail: detail.into(),
        tone: world_projection::Tone::Neutral,
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
                "This World now carries a legacy of its earlier choices.",
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
        kind: BriefingItemKind::Status,
        selection: Some(selection),
        title: format!("World legacy · {}", legacy_label(&legacy)),
        detail: summary,
        tone: world_projection::Tone::Neutral,
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
            "They have settled into a lasting partnership.",
        ),
        "fracture" => (
            "Relationship fractured",
            "They have settled into a lasting rift.",
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
    // The latest shift already states trust and tension; saying them twice
    // in one card is how it ended up reading like a readout.
    let _ = last_dynamic;
    let detail = format!("{meaning} {}", crate::bond_phrase(trust, tension));
    Some(BriefingItem {
        kind: BriefingItemKind::Status,
        selection: Some(SelectionId::Entity(RELATIONSHIP)),
        title: title.into(),
        detail,
        tone: world_projection::Tone::Neutral,
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

    let why_now = return_compass_context(
        world,
        generation,
        relationship_choice_available,
        intervention_choice_available,
        posture_choice_available,
        &legacy,
    );
    // Why this choice is open now, in the World's own terms. What each
    // answer does is the choices' own business, one line below.
    let detail = why_now;

    BriefingItem {
        kind: BriefingItemKind::Status,
        selection: None,
        title: title.into(),
        detail,
        tone: world_projection::Tone::Neutral,
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
    let _ = generation;
    last_change
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
    let footing = match (trust, tension) {
        (t, n) if n > t => "They are not easy with each other yet.",
        (t, _) if t >= 4 => "They are starting to rely on each other.",
        _ => "They are still finding their footing with each other.",
    };
    format!("{footing}{dynamic} Where it goes is still open.")
}

fn intervention_return_context(world: &World, generation: i64) -> String {
    let last_change = text_component(
        world.state().entity(UNIVERSE),
        LAST_CHANGE,
        "The world is quiet.",
    );
    let _ = generation;
    format!("The World has grown enough for a bigger choice. {last_change}")
}

fn posture_return_context(world: &World) -> String {
    let social_arc = text_component(
        world.state().entity(RELATIONSHIP),
        RELATIONSHIP_SOCIAL_ARC,
        "forming",
    );
    let social_arc = match social_arc.as_str() {
        "partnership" => "a partnership".into(),
        "fracture" => "a rift".into(),
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
        "The first chapter has settled into {social_arc}, and {influence} is here to stay. The next choice decides whether this World reaches outward or deepens home."
    )
}

fn legacy_return_context(world: &World, legacy: &str) -> String {
    let cycles = integer_component(world, LEGACY_CYCLES).unwrap_or_default();
    let legacy = legacy_label(legacy);
    if cycles <= 0 {
        let summary = text_component(
            world.state().entity(UNIVERSE),
            LEGACY_SUMMARY,
            "This World now carries a legacy of its earlier choices.",
        );
        return format!("{legacy} has formed. {summary}");
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
    match pattern {
        Some(pattern) => format!(
            "{legacy} keeps growing stronger, carried by {pattern}. Letting time pass feeds it further."
        ),
        None => format!("{legacy} keeps growing stronger. Letting time pass feeds it further."),
    }
}

/// History in the World's own words. What happened to somebody is a story;
/// the everyday round (each day's tending and exploring, a decision logged,
/// the small shifts between two people, a legacy renewing itself) folds
/// under the moment it happened in.
fn told_timeline(world: &World) -> world_projection::TimelineProjection {
    let mut timeline = timeline_from_world(world);
    world_projection::retell_timeline(&mut timeline, world, |event| {
        let summary =
            ["summary", "change"]
                .into_iter()
                .find_map(|key| match event.payload.get(key) {
                    Some(Value::Text(text)) if !text.trim().is_empty() => Some(text.clone()),
                    _ => None,
                });
        let actor = event
            .actor
            .and_then(|id| world.state().entity(id))
            .map(entity_title);
        match event.kind.as_str() {
            "agent_decision_recorded" => world_projection::Telling::Routine(
                actor.map(|name| format!("{name} decided what to do next")),
            ),
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
        "agent_decision_recorded"
            | "agent_cared_for_world"
            | "agent_explored_world"
            | "relationship_shifted"
            | "legacy_reinforced"
            | "successor_waited"
    )
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
        // The everyday round fills whatever room the story leaves, people's
        // own doings first.
        "agent_cared_for_world" | "agent_explored_world" => 2,
        kind if is_routine(kind) => 3,
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

fn return_item(events: &[Event], event: &Event, _occurrences: usize) -> BriefingItem {
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
        .unwrap_or_else(|| event_kind_words(&event.kind));
    // How often something happened is not what happened; the title says what.
    let title: String = match event.kind.as_str() {
        "universe_grew" => "The world moved".into(),
        "universe_intervened" => "Your choice took hold".into(),
        "universe_seeded" => "A world began".into(),
        "agent_cared_for_world" => "Looking after home".into(),
        "agent_explored_world" => "Out exploring".into(),
        "relationship_shifted" => "Between the two of them".into(),
        "relationship_steered" => "You steered their relationship".into(),
        "partnership_formed" => "A partnership formed".into(),
        "relationship_fractured" => "Their relationship fractured".into(),
        "world_legacy_formed" => "A world legacy formed".into(),
        "legacy_reinforced" => "The legacy grew stronger".into(),
        "pressure_rising" => "Pressure is rising".into(),
        "pressure_peaked" => "The pressure peaked".into(),
        "anchor_lost" => "Something was lost".into(),
        "pressure_held" => "You held through the pressure".into(),
        "pressure_reached" => "You reached beyond the pressure".into(),
        "anchor_recovered" => "You recovered what was lost".into(),
        "world_posture_chosen" => "A direction was chosen".into(),
        "successor_emerged" => "A successor stepped in".into(),
        "successor_waited" => "The successor carried on".into(),
        "legacy_entrusted" => "The legacy was handed on".into(),
        "legacy_released" => "The legacy was set aside".into(),
        "era_began" => "A new era began".into(),
        other => event_kind_words(other),
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
                title: entity_title(entity),
                subtitle: entity.kind.replace('_', " "),
            })
            .collect(),
    }
}

/// Everything the World has built, one mark per time it grew, in the shapes
/// its place builds in: domes and masts on Mars, houses and street lamps on
/// Maple Street, domes and trees of ice on Icebridge.
fn growth_marks(world: &World) -> Vec<world_projection::CanvasMark> {
    use world_projection::MarkShape::{Dome, House, Lamp, Tower, Tree};
    let shapes: &[world_projection::MarkShape] = match seed_id(world) {
        "mars-colony" => &[Dome, Tower, Dome, Dome, Tower],
        "1980s-town" => &[House, Lamp, House, House, Lamp, Tree],
        "penguin-civilization" => &[Dome, Tree, Dome, Tower],
        _ => &[House],
    };
    world
        .events()
        .iter()
        .filter(|event| event.kind == "universe_grew")
        .enumerate()
        .map(|(index, event)| world_projection::CanvasMark {
            label: match event.payload.get("change") {
                Some(Value::Text(change)) => change.clone(),
                _ => "The World grew".into(),
            },
            shape: shapes[index % shapes.len()],
            selection: Some(SelectionId::Event(event.id)),
        })
        .collect()
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
    // Every seed casts the same five roles in the same slots: the anchor
    // everything depends on, a second place, the two people whose
    // relationship is the story, and the thing that lets them range out.
    // Placing by role rather than by list order lets the scene read the same
    // way in every World: home on the left, the pair in the middle, the way
    // out on the right.
    const LAYOUT: [(EntityId, f32, f32); 5] = [
        (SLOT_A, 0.12, 0.22),
        (SLOT_C, 0.12, 0.86),
        (SLOT_B, 0.42, 0.10),
        (SLOT_E, 0.62, 0.84),
        (SLOT_D, 0.90, 0.46),
    ];
    let items = LAYOUT
        .iter()
        .filter_map(|(id, x, y)| {
            let entity = world.state().entity(*id)?;
            Some(CanvasItem {
                id: SelectionId::Entity(*id),
                kind: canvas_kind(entity),
                label: entity_title(entity),
                detail: canvas_detail(world, entity),
                x: *x,
                y: *y,
                changes: Vec::new(),
                shape: place_shape(world, *id),
                at: whereabouts(world, entity),
            })
        })
        .collect();
    CanvasProjection {
        items,
        links: relationship_link(world).into_iter().collect(),
        marks: growth_marks(world),
    }
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
fn canvas_detail(world: &World, entity: &Entity) -> String {
    if entity.id == SLOT_A {
        let stage = pressure::pressure_id_from_state(world.state());
        let trouble = match stage.as_str() {
            "warning" => Some("trouble rising"),
            "crisis" => Some("in crisis"),
            "lost" => Some("lost"),
            _ => None,
        };
        if let Some(trouble) = trouble {
            return trouble.into();
        }
    }
    for key in ["status", "role"] {
        if let Some(Value::Text(value)) = entity.component(key) {
            if !value.trim().is_empty() {
                return value.clone();
            }
        }
    }
    entity.kind.replace('_', " ")
}

/// The relationship at the centre of the World, drawn between the two
/// people it belongs to rather than as a third thing beside them.
/// What a Pocket Universe keeps score of: how much the two people trust
/// each other, how strained they are, and how safe the place everyone
/// depends on is. Nothing before the World begins.
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
    let anchor = world
        .state()
        .entity(SLOT_A)
        .map(entity_title)
        .unwrap_or_else(|| "Home".into());
    let (safety, reading, tone) = match pressure::pressure_id_from_state(world.state()).as_str() {
        "warning" => (0.6, "Trouble rising", Tone::Warning),
        "crisis" => (0.3, "In crisis", Tone::Bad),
        "lost" => (0.0, "Lost", Tone::Bad),
        "held" | "reached" => (1.0, "Weathered it", Tone::Good),
        "recovered" => (0.8, "Rebuilt", Tone::Good),
        _ => (1.0, "Safe", Tone::Neutral),
    };
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
        Gauge {
            id: "anchor".into(),
            label: anchor,
            value: safety,
            reading: reading.into(),
            tone,
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
        let trouble = match pressure::pressure_id_from_state(world.state()).as_str() {
            "warning" => Some("Rising"),
            "crisis" => Some("A crisis"),
            "lost" => Some("Lost"),
            _ => None,
        };
        if let Some(trouble) = trouble {
            rows.push(row("Trouble", trouble.into()));
        }
        let decision = text(DECISION).unwrap_or_default();
        if let Some((title, _)) = intervention_influence_copy(&decision) {
            let choice = title
                .trim_start_matches("Your influence")
                .trim_start_matches(" · ");
            if !choice.is_empty() {
                rows.push(row("Your choice", choice.into()));
            }
        }
        if let Some(posture) = text(POSTURE).filter(|posture| posture != "none") {
            rows.push(row("Direction", capitalize_first(&posture)));
        }
        if let Some(legacy) = text(LEGACY).filter(|legacy| legacy != "none" && legacy != "forming")
        {
            rows.push(row("Legacy", capitalize_first(&legacy.replace('-', " "))));
        }
        if let Some(summary) = text(LEGACY_SUMMARY) {
            rows.push(row("What it is becoming", summary));
        }
        return (Some(seed_label(seed_id(world)).into()), rows);
    }
    if entity.id == RELATIONSHIP {
        let number = |key: &str| integer_entity_component(Some(entity), key).unwrap_or(0);
        rows.push(row("Trust", number(RELATIONSHIP_TRUST).to_string()));
        rows.push(row("Tension", number(RELATIONSHIP_TENSION).to_string()));
        if let Some(link) = relationship_link(world) {
            rows.push(row("Where it stands", link.label));
        }
        if let Some(lately) = text(RELATIONSHIP_LAST_DYNAMIC) {
            rows.push(row("Lately", lately));
        }
        return (None, rows);
    }
    if matches!(entity.kind.as_str(), "person" | "penguin") {
        if let Some(role) = text("role") {
            rows.push(row("Role", capitalize_first(&role)));
        }
        let lately = match text("last_intent").as_deref() {
            Some("care") => Some("Looking after the others"),
            Some("explore") => Some("Out exploring"),
            _ => None,
        };
        if let Some(lately) = lately {
            rows.push(row("Lately", lately.into()));
        }
        let times = |key: &str| match entity.component(key) {
            Some(Value::Integer(count)) if *count > 0 => Some(match count {
                1 => "Once".to_string(),
                count => format!("{count} times"),
            }),
            _ => None,
        };
        if let Some(count) = times("care_count") {
            rows.push(row("Looked after the others", count));
        }
        if let Some(count) = times("explore_count") {
            rows.push(row("Went exploring", count));
        }
        // Someone guided by an outside mind says so; the built-in one goes
        // without saying.
        if let Some(mind) = text("last_mind_profile").filter(|mind| mind != "deterministic") {
            rows.push(row("Guided by", capitalize_first(&mind)));
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
        let label = match key.as_str() {
            "pulse" => "Latest".to_string(),
            "water_cycles" => "Water loops".to_string(),
            other => capitalize_first(&other.replace('_', " ")),
        };
        rows.push(row(&label, capitalize_first(&value)));
    }
    (None, rows)
}

/// Components the rules keep for themselves.
fn is_bookkeeping(key: &str) -> bool {
    key.starts_with("legacy")
        || key.starts_with("pressure")
        || key.starts_with("posture")
        || key.starts_with("succession")
        || key.ends_with("_count")
        || key.ends_with("generation")
        || matches!(
            key,
            "seed" | "decision" | "era" | "custom" | "last_intent" | "last_mind_profile"
        )
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn relationship_link(world: &World) -> Option<CanvasLink> {
    let relationship = world.state().entity(RELATIONSHIP)?;
    world.state().entity(SLOT_B)?;
    world.state().entity(SLOT_E)?;
    let trust = integer_entity_component(Some(relationship), RELATIONSHIP_TRUST).unwrap_or(0);
    let tension = integer_entity_component(Some(relationship), RELATIONSHIP_TENSION).unwrap_or(0);
    let arc = text_component(Some(relationship), RELATIONSHIP_SOCIAL_ARC, "forming");
    let direction = text_component(Some(relationship), RELATIONSHIP_DIRECTION, "none");
    let (label, tone) = match (arc.as_str(), direction.as_str()) {
        ("partnership", _) => ("Partners", CanvasLinkTone::Warm),
        ("fracture", _) => ("Estranged", CanvasLinkTone::Strained),
        (_, "shared-project") => ("Working together", CanvasLinkTone::Warm),
        (_, "rivalry") => ("Rivals", CanvasLinkTone::Strained),
        _ if tension > trust => ("Uneasy", CanvasLinkTone::Strained),
        _ => ("Getting to know each other", CanvasLinkTone::Neutral),
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

/// Where someone is, as they last left things: looking after the others
/// keeps them at home; exploring takes them out with the thing that lets
/// them range (the rover, the night bus, the council's ice runs). Before
/// either has done anything they are both at home.
fn whereabouts(world: &World, entity: &Entity) -> Option<SelectionId> {
    if canvas_kind(entity) != CanvasItemKind::Actor {
        return None;
    }
    let out_exploring = matches!(
        entity.component("last_intent"),
        Some(Value::Text(intent)) if intent == "explore"
    );
    let place = if out_exploring { SLOT_D } else { SLOT_A };
    world
        .state()
        .entity(place)
        .map(|_| SelectionId::Entity(place))
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

/// What each place looks like from a distance: red dust and a pale sun for
/// Ares, a sodium-lit street at dusk for Maple Street, ice under the aurora
/// for Icebridge.
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
        "1980s-town" => Some(scenery(0x241d45, 0x6b4a7a, 0x3a2f55, 0x1b1630, 0xf4bf5c)),
        "penguin-civilization" => Some(scenery(0x14305a, 0x3f8f95, 0xa9cbdb, 0xe6f1f6, 0xb9f3d3)),
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
            "Ares is waking up"
        );
        assert_eq!(
            live_stage_copy("mars-colony", 1, false, false).0,
            "They are finding their rhythm"
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
