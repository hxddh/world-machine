from pathlib import Path

lib = Path('worlds/pocket-universe/src/lib.rs')
s = lib.read_text()

s = s.replace('pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.1.0";', 'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.2.0";', 1)
s = s.replace(
    'pub const NUDGE_COMMAND: &str = "pocket-universe.nudge";\n',
    'pub const NUDGE_COMMAND: &str = "pocket-universe.nudge";\n'
    'pub const BOLD_PATH_COMMAND: &str = "pocket-universe.choose-bold-path";\n'
    'pub const CAREFUL_PATH_COMMAND: &str = "pocket-universe.choose-careful-path";\n',
    1,
)
s = s.replace(
    'pub(crate) const LAST_CHANGE: &str = "last_change";\n',
    'pub(crate) const LAST_CHANGE: &str = "last_change";\n'
    'pub(crate) const DECISION: &str = "decision";\n',
    1,
)
s = s.replace(
    '''    pub fn projection_snapshot(&self) -> ProjectionSnapshot {\n        projection::snapshot(&self.world)\n    }\n''',
    '''    pub fn projection_snapshot(&self) -> ProjectionSnapshot {\n        projection::snapshot(&self.world)\n    }\n\n    pub fn projection_snapshot_since(&self, since_event_count: Option<usize>) -> ProjectionSnapshot {\n        projection::snapshot_since(&self.world, since_event_count)\n    }\n''',
    1,
)
s = s.replace(
    '''            NUDGE_COMMAND => "grow_universe",\n            _ => {\n''',
    '''            NUDGE_COMMAND => "grow_universe",\n            BOLD_PATH_COMMAND => "choose_bold_path",\n            CAREFUL_PATH_COMMAND => "choose_careful_path",\n            _ => {\n''',
    1,
)
s = s.replace(
    '''struct PocketUniverseSession {\n    world: PocketUniverse,\n}\n''',
    '''struct PocketUniverseSession {\n    world: PocketUniverse,\n    return_since_event_count: Option<usize>,\n}\n''',
    1,
)
s = s.replace(
    '''        Ok(Box::new(Self {\n            world: PocketUniverse::new().map_err(HostError::session)?,\n        }))\n''',
    '''        Ok(Box::new(Self {\n            world: PocketUniverse::new().map_err(HostError::session)?,\n            return_since_event_count: None,\n        }))\n''',
    1,
)
s = s.replace(
    '''        Ok(Box::new(Self {\n            world: PocketUniverse::resume_archive(archive).map_err(HostError::session)?,\n        }))\n''',
    '''        Ok(Box::new(Self {\n            world: PocketUniverse::resume_archive(archive).map_err(HostError::session)?,\n            return_since_event_count: None,\n        }))\n''',
    1,
)
s = s.replace(
    '''    fn snapshot(&self) -> ProjectionSnapshot {\n        self.world.projection_snapshot()\n    }\n''',
    '''    fn snapshot(&self) -> ProjectionSnapshot {\n        self.world\n            .projection_snapshot_since(self.return_since_event_count)\n    }\n''',
    1,
)
s = s.replace(
    '''        match intent {\n            ProjectionIntent::ForkBeforeEvent(event) => self\n                .world\n                .fork_before_event(event)\n                .map_err(HostError::session)?,\n            ProjectionIntent::InvokeCommand(command) => {\n                self.world\n                    .invoke_projection_command(&command)\n                    .map_err(HostError::session)?;\n            }\n        }\n        Ok(self.snapshot())\n''',
    '''        match intent {\n            ProjectionIntent::ForkBeforeEvent(event) => self\n                .world\n                .fork_before_event(event)\n                .map_err(HostError::session)?,\n            ProjectionIntent::InvokeCommand(command) => {\n                self.world\n                    .invoke_projection_command(&command)\n                    .map_err(HostError::session)?;\n            }\n        }\n        self.return_since_event_count = None;\n        Ok(self.snapshot())\n''',
    1,
)
s = s.replace(
    '''    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {\n        self.world\n            .advance_periods(periods)\n            .map_err(HostError::session)?;\n        Ok(self.snapshot())\n    }\n''',
    '''    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {\n        let before = self.world.world().events().len();\n        self.world\n            .advance_periods(periods)\n            .map_err(HostError::session)?;\n        self.return_since_event_count =\n            (self.world.world().events().len() > before).then_some(before);\n        Ok(self.snapshot())\n    }\n''',
    1,
)
s = s.replace(
    '''            .with_component(GENERATION, 0_i64)\n            .with_component(LAST_CHANGE, "Nothing exists here yet."),\n''',
    '''            .with_component(GENERATION, 0_i64)\n            .with_component(DECISION, "none")\n            .with_component(LAST_CHANGE, "Nothing exists here yet."),\n''',
    1,
)
s = s.replace(
    '''    actions.register(GrowUniverse)?;\n    Ok(actions)\n}\n\nstruct SeedMarsColony;\nstruct Seed1980sTown;\nstruct SeedPenguinCivilization;\nstruct GrowUniverse;\n''',
    '''    actions.register(GrowUniverse)?;\n    actions.register(ChooseBoldPath)?;\n    actions.register(ChooseCarefulPath)?;\n    Ok(actions)\n}\n\nstruct SeedMarsColony;\nstruct Seed1980sTown;\nstruct SeedPenguinCivilization;\nstruct GrowUniverse;\nstruct ChooseBoldPath;\nstruct ChooseCarefulPath;\n''',
    1,
)
s = s.replace(
    '.with_component(ANCHOR_PULSE, "first lights"),',
    '.with_component(ANCHOR_PULSE, "first lights")\n                    .with_component("water_cycles", 0_i64),',
    1,
)
s = s.replace(
    '.with_component(ANCHOR_PULSE, "new high score"),',
    '.with_component(ANCHOR_PULSE, "new high score")\n                    .with_component("high_scores", 0_i64),',
    1,
)
s = s.replace(
    '.with_component(ANCHOR_PULSE, "first fish bell"),',
    '.with_component(ANCHOR_PULSE, "first fish bell")\n                    .with_component("bridge_spans", 1_i64),',
    1,
)
old_grow = '''        let generation = integer_component(state, UNIVERSE, GENERATION)?;\n        let next = generation + 1;\n        let change = growth_message(&seed, next);\n        let pulse = anchor_pulse(&seed, next);\n        let mut draft = EventDraft::new("universe_grew");\n        draft.targets = vec![UNIVERSE, SLOT_A];\n        draft.payload.insert("seed".into(), seed.into());\n        draft.payload.insert("generation".into(), next.into());\n        draft.changes = vec![\n            StateChange::SetComponent {\n                entity: UNIVERSE,\n                key: GENERATION.into(),\n                value: next.into(),\n            },\n            StateChange::SetComponent {\n                entity: UNIVERSE,\n                key: LAST_CHANGE.into(),\n                value: change.into(),\n            },\n            StateChange::SetComponent {\n                entity: SLOT_A,\n                key: ANCHOR_PULSE.into(),\n                value: pulse.into(),\n            },\n        ];\n        Ok(draft)\n'''
new_grow = '''        let generation = integer_component(state, UNIVERSE, GENERATION)?;\n        let next = generation + 1;\n        let decision = decision_id_from_state(state)?;\n        let change = growth_message(&seed, next, &decision);\n        let pulse = anchor_pulse(&seed, next);\n        let (metric_key, metric_value) = growth_metric(state, &seed)?;\n        let mut draft = EventDraft::new("universe_grew");\n        draft.targets = vec![UNIVERSE, SLOT_A];\n        draft.payload.insert("seed".into(), seed.into());\n        draft.payload.insert("generation".into(), next.into());\n        draft.payload.insert("change".into(), change.clone().into());\n        draft.changes = vec![\n            StateChange::SetComponent {\n                entity: UNIVERSE,\n                key: GENERATION.into(),\n                value: next.into(),\n            },\n            StateChange::SetComponent {\n                entity: UNIVERSE,\n                key: LAST_CHANGE.into(),\n                value: change.into(),\n            },\n            StateChange::SetComponent {\n                entity: SLOT_A,\n                key: ANCHOR_PULSE.into(),\n                value: pulse.into(),\n            },\n            StateChange::SetComponent {\n                entity: SLOT_A,\n                key: metric_key.into(),\n                value: metric_value.into(),\n            },\n        ];\n        Ok(draft)\n'''
if old_grow not in s:
    raise SystemExit('grow block not found')
s = s.replace(old_grow, new_grow, 1)

marker = '''fn seed_draft(\n'''
choice_impl = '''impl Action for ChooseBoldPath {\n    fn name(&self) -> &'static str {\n        "choose_bold_path"\n    }\n\n    fn evaluate(\n        &self,\n        state: &WorldState,\n        _request: &ActionRequest,\n    ) -> Result<EventDraft, ActionError> {\n        choice_draft(state, true)\n    }\n}\n\nimpl Action for ChooseCarefulPath {\n    fn name(&self) -> &'static str {\n        "choose_careful_path"\n    }\n\n    fn evaluate(\n        &self,\n        state: &WorldState,\n        _request: &ActionRequest,\n    ) -> Result<EventDraft, ActionError> {\n        choice_draft(state, false)\n    }\n}\n\nfn choice_draft(state: &WorldState, bold: bool) -> Result<EventDraft, ActionError> {\n    let seed = seed_id_from_state(state)?;\n    if seed == UNSEEDED {\n        return Err(ActionError::Invalid(\n            "choose a Pocket Universe seed before intervening".into(),\n        ));\n    }\n    if integer_component(state, UNIVERSE, GENERATION)? < 3 {\n        return Err(ActionError::Invalid(\n            "this Pocket Universe has not grown enough for that choice yet".into(),\n        ));\n    }\n    if decision_id_from_state(state)? != "none" {\n        return Err(ActionError::Invalid(\n            "this Pocket Universe has already crossed its first intervention point".into(),\n        ));\n    }\n\n    let (choice, summary, target, key, value) = match (seed.as_str(), bold) {\n        ("mars-colony", true) => (\n            "follow-signal",\n            "Kestrel leaves the safe route to follow a repeating signal beyond the ridge.",\n            SLOT_D,\n            "status",\n            "signal expedition",\n        ),\n        ("mars-colony", false) => (\n            "fortify-habitat",\n            "The colony diverts its spare capacity into sealing Ares Habitat before the next dust front.",\n            SLOT_A,\n            "status",\n            "storm sealed",\n        ),\n        ("1980s-town", true) => (\n            "community-arcade",\n            "Maple Arcade turns its late hours into a neighborhood club instead of closing the shutters.",\n            SLOT_A,\n            "status",\n            "community nights",\n        ),\n        ("1980s-town", false) => (\n            "steady-business",\n            "Maple Arcade keeps a quieter commercial rhythm and protects its small cash buffer.",\n            SLOT_A,\n            "status",\n            "steady business",\n        ),\n        ("penguin-civilization", true) => (\n            "winter-feast",\n            "Icebridge opens the Fish Vault for a winter feast that brings distant colonies onto the bridge.",\n            SLOT_C,\n            "reserve",\n            "festival opened",\n        ),\n        ("penguin-civilization", false) => (\n            "conserve-reserves",\n            "The Aurora Council keeps the Fish Vault sealed and stores extra reserves for the dark season.",\n            SLOT_C,\n            "reserve",\n            "winter conserved",\n        ),\n        _ => {\n            return Err(ActionError::Invalid(format!(\n                "unsupported Pocket Universe seed: {seed}"\n            )))\n        }\n    };\n\n    let mut draft = EventDraft::new("universe_intervened");\n    draft.targets = vec![UNIVERSE, target];\n    draft.payload.insert("choice".into(), choice.into());\n    draft.payload.insert("summary".into(), summary.into());\n    draft.changes = vec![\n        StateChange::SetComponent {\n            entity: UNIVERSE,\n            key: DECISION.into(),\n            value: choice.into(),\n        },\n        StateChange::SetComponent {\n            entity: UNIVERSE,\n            key: LAST_CHANGE.into(),\n            value: summary.into(),\n        },\n        StateChange::SetComponent {\n            entity: target,\n            key: key.into(),\n            value: value.into(),\n        },\n    ];\n    Ok(draft)\n}\n\n'''
if marker not in s:
    raise SystemExit('seed_draft marker missing')
s = s.replace(marker, choice_impl + marker, 1)
s = s.replace(
    '''        StateChange::SetComponent {\n            entity: UNIVERSE,\n            key: GENERATION.into(),\n            value: 0_i64.into(),\n        },\n        StateChange::SetComponent {\n            entity: UNIVERSE,\n            key: LAST_CHANGE.into(),\n''',
    '''        StateChange::SetComponent {\n            entity: UNIVERSE,\n            key: GENERATION.into(),\n            value: 0_i64.into(),\n        },\n        StateChange::SetComponent {\n            entity: UNIVERSE,\n            key: DECISION.into(),\n            value: "none".into(),\n        },\n        StateChange::SetComponent {\n            entity: UNIVERSE,\n            key: LAST_CHANGE.into(),\n''',
    1,
)
marker2 = '''fn integer_component(state: &WorldState, entity: EntityId, key: &str) -> Result<i64, ActionError> {\n'''
helpers = '''fn decision_id_from_state(state: &WorldState) -> Result<String, ActionError> {\n    match state\n        .entity(UNIVERSE)\n        .and_then(|entity| entity.component(DECISION))\n    {\n        Some(Value::Text(decision)) => Ok(decision.clone()),\n        _ => Err(ActionError::Invalid(\n            "Pocket Universe decision state is missing".into(),\n        )),\n    }\n}\n\nfn growth_metric(state: &WorldState, seed: &str) -> Result<(&'static str, i64), ActionError> {\n    let key = match seed {\n        "mars-colony" => "water_cycles",\n        "1980s-town" => "high_scores",\n        "penguin-civilization" => "bridge_spans",\n        _ => return Err(ActionError::Invalid(format!("unsupported Pocket Universe seed: {seed}"))),\n    };\n    Ok((key, integer_component(state, SLOT_A, key)? + 1))\n}\n\n'''
if marker2 not in s:
    raise SystemExit('integer helper marker missing')
s = s.replace(marker2, helpers + marker2, 1)

start = s.index('fn growth_message(')
end = s.index('\nfn anchor_pulse', start)
new_growth = '''fn growth_message(seed: &str, generation: i64, decision: &str) -> String {\n    let cycle = ((generation - 1).rem_euclid(3)) as usize;\n    let messages: [&[&str]; 3] = [\n        &[\n            "The colony opened a new water-recovery loop.",\n            "A dust front changed the rover routes overnight.",\n            "The hydroponics crew harvested its first shared meal.",\n        ],\n        &[\n            "A handwritten tournament bracket appeared at the arcade.",\n            "K-88 dedicated an hour to calls from the neighborhood.",\n            "Night Bus 6 added an unscheduled stop after the rain.",\n        ],\n        &[\n            "A new ice bridge shortened the walk to the Fish Vault.",\n            "Piko rang the fish bell early after spotting a silver shoal.",\n            "The Aurora Council adopted a new moonrise signal.",\n        ],\n    ];\n    let base = match seed {\n        "mars-colony" => messages[0][cycle],\n        "1980s-town" => messages[1][cycle],\n        "penguin-civilization" => messages[2][cycle],\n        _ => "The world changed in a small but persistent way.",\n    };\n    if decision == "none" {\n        return base.into();\n    }\n    let consequence = match decision {\n        "follow-signal" => "The signal expedition keeps pulling attention beyond the safe ridge.",\n        "fortify-habitat" => "The stronger habitat makes every later risk feel more deliberate.",\n        "community-arcade" => "The arcade is becoming a place people organize their evenings around.",\n        "steady-business" => "The arcade survives by staying small, predictable, and open.",\n        "winter-feast" => "The feast has turned Icebridge into a meeting point for distant colonies.",\n        "conserve-reserves" => "The sealed reserve gives the council more room to plan for the dark season.",\n        _ => "The earlier intervention is still shaping what happens next.",\n    };\n    format!("{base} {consequence}")\n}\n'''
s = s[:start] + new_growth + s[end:]

s = s.replace(
    '''        assert!(after\n            .briefing\n            .as_ref()\n            .unwrap()\n            .title\n            .contains("Generation 2"));\n''',
    '''        let briefing = after.briefing.as_ref().unwrap();\n        assert_eq!(briefing.title, "While you were away");\n        assert_eq!(briefing.items.len(), 2);\n        assert!(briefing\n            .items\n            .iter()\n            .all(|item| !item.detail.trim().is_empty()));\n''',
    1,
)
s = s.replace(
    '''        session.advance_background(3).unwrap();\n        let before = session.snapshot();\n        let archive = session.archive().unwrap().unwrap();\n''',
    '''        session.advance_background(3).unwrap();\n        session\n            .handle(ProjectionIntent::InvokeCommand(NUDGE_COMMAND.into()))\n            .unwrap();\n        let before = session.snapshot();\n        let archive = session.archive().unwrap().unwrap();\n''',
    1,
)
insert_before = '''    #[test]\n    fn forking_before_seed_returns_to_an_empty_universe() {\n'''
new_tests = '''    #[test]\n    fn generation_three_exposes_a_durable_intervention() {\n        let registry = registry();\n        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();\n        session\n            .handle(ProjectionIntent::InvokeCommand(\n                SEED_1980S_TOWN_COMMAND.into(),\n            ))\n            .unwrap();\n        let grown = session.advance_background(3).unwrap();\n        let command_ids = grown\n            .commands\n            .iter()\n            .map(|command| command.id.as_str())\n            .collect::<Vec<_>>();\n        assert!(command_ids.contains(&BOLD_PATH_COMMAND));\n        assert!(command_ids.contains(&CAREFUL_PATH_COMMAND));\n\n        let chosen = session\n            .handle(ProjectionIntent::InvokeCommand(BOLD_PATH_COMMAND.into()))\n            .unwrap();\n        assert_eq!(chosen.briefing.as_ref().unwrap().title, "Generation 3");\n        assert!(!chosen\n            .commands\n            .iter()\n            .any(|command| command.id == BOLD_PATH_COMMAND || command.id == CAREFUL_PATH_COMMAND));\n        let universe = chosen\n            .inspectors\n            .get(&world_projection::SelectionId::Entity(UNIVERSE))\n            .unwrap();\n        assert!(universe.sections.iter().flat_map(|section| &section.rows).any(|row| {\n            row.label == "Decision" && row.value == "community-arcade"\n        }));\n\n        let archive = session.archive().unwrap().unwrap();\n        drop(session);\n        let reopened = registry.open_archive(&archive).unwrap();\n        assert_eq!(reopened.archive().unwrap().unwrap(), archive);\n        assert!(!reopened\n            .snapshot()\n            .commands\n            .iter()\n            .any(|command| command.id == BOLD_PATH_COMMAND || command.id == CAREFUL_PATH_COMMAND));\n    }\n\n    #[test]\n    fn forking_before_intervention_reopens_the_choice() {\n        let registry = registry();\n        let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();\n        session\n            .handle(ProjectionIntent::InvokeCommand(\n                SEED_PENGUIN_CIVILIZATION_COMMAND.into(),\n            ))\n            .unwrap();\n        session.advance_background(3).unwrap();\n        let chosen = session\n            .handle(ProjectionIntent::InvokeCommand(CAREFUL_PATH_COMMAND.into()))\n            .unwrap();\n        let intervention = chosen\n            .timeline\n            .items\n            .iter()\n            .find(|item| item.title == "Universe Intervened")\n            .and_then(|item| match item.id {\n                world_projection::SelectionId::Event(id) => Some(id),\n                _ => None,\n            })\n            .unwrap();\n\n        let forked = session\n            .handle(ProjectionIntent::ForkBeforeEvent(intervention))\n            .unwrap();\n        assert!(forked\n            .commands\n            .iter()\n            .any(|command| command.id == BOLD_PATH_COMMAND));\n        assert!(forked\n            .commands\n            .iter()\n            .any(|command| command.id == CAREFUL_PATH_COMMAND));\n    }\n\n'''
if insert_before not in s:
    raise SystemExit('test insertion marker missing')
s = s.replace(insert_before, new_tests + insert_before, 1)
lib.write_text(s)

projection = Path('worlds/pocket-universe/src/projection.rs')
projection.write_text(r'''use crate::{
    seed_id, BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND, DECISION, GENERATION, LAST_CHANGE,
    NUDGE_COMMAND, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND, UNIVERSE,
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

    let mut commands = vec![ProjectionCommand {
        id: NUDGE_COMMAND.into(),
        title: "Nudge the world".into(),
        detail: "Let one small, persistent change happen now.".into(),
    }];
    let generation = integer_component(world, GENERATION).unwrap_or_default();
    let decision = text_component(world.state().entity(UNIVERSE), DECISION, "none");
    if generation >= 3 && decision == "none" {
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
    commands
}

fn briefing(
    world: &World,
    seeded: bool,
    since_event_count: Option<usize>,
) -> BriefingProjection {
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
        return BriefingProjection {
            eyebrow: format!("Pocket Universe · {}", seed_label(seed_id(world))),
            title: "While you were away".into(),
            items: events
                .iter()
                .rev()
                .take(3)
                .map(return_item)
                .collect(),
        };
    }

    let generation = integer_component(world, GENERATION).unwrap_or_default();
    let last_change = text_component(
        world.state().entity(UNIVERSE),
        LAST_CHANGE,
        "The world is quiet.",
    );
    BriefingProjection {
        eyebrow: format!("Pocket Universe · {}", seed_label(seed_id(world))),
        title: format!("Generation {generation}"),
        items: vec![BriefingItem {
            selection: Some(SelectionId::Entity(UNIVERSE)),
            title: "Current thread".into(),
            detail: last_change,
        }],
    }
}

fn return_item(event: &Event) -> BriefingItem {
    let detail = ["change", "summary"]
        .into_iter()
        .find_map(|key| match event.payload.get(key) {
            Some(Value::Text(value)) => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_else(|| event.kind.replace('_', " "));
    BriefingItem {
        selection: Some(SelectionId::Event(event.id)),
        title: match event.kind.as_str() {
            "universe_grew" => "The world moved".into(),
            "universe_intervened" => "Your choice took hold".into(),
            "universe_seeded" => "A world began".into(),
            _ => event.kind.replace('_', " "),
        },
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
    const POSITIONS: [(f32, f32); 4] = [(0.18, 0.30), (0.72, 0.26), (0.25, 0.74), (0.70, 0.70)];
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
''')

for cargo in [Path('worlds/pocket-universe/Cargo.toml'), Path('apps/pocket-universe-pack/Cargo.toml')]:
    text = cargo.read_text().replace('version = "0.1.0"', 'version = "0.2.0"', 1)
    cargo.write_text(text)

external = Path('apps/pocket-universe-pack/tests/external_pack.rs')
e = external.read_text()
e = e.replace(
    'use pocket_universe::{POCKET_UNIVERSE_PACK_ID, SEED_MARS_COLONY_COMMAND};',
    'use pocket_universe::{BOLD_PATH_COMMAND, POCKET_UNIVERSE_PACK_ID, SEED_MARS_COLONY_COMMAND};',
    1,
)
e = e.replace(
    '''    let grown = session.advance_background(2).unwrap();\n    assert_eq!(grown.world_time, 20);\n    assert!(grown\n        .briefing\n        .as_ref()\n        .expect("Pocket Universe has a briefing")\n        .title\n        .contains("Generation 2"));\n\n    let archive = session.archive().unwrap().unwrap();\n    let before = session.snapshot();\n''',
    '''    let grown = session.advance_background(3).unwrap();\n    assert_eq!(grown.world_time, 30);\n    assert_eq!(\n        grown\n            .briefing\n            .as_ref()\n            .expect("Pocket Universe has a return briefing")\n            .title,\n        "While you were away"\n    );\n    assert!(grown\n        .commands\n        .iter()\n        .any(|command| command.id == BOLD_PATH_COMMAND));\n    let chosen = session\n        .handle(ProjectionIntent::InvokeCommand(BOLD_PATH_COMMAND.into()))\n        .unwrap();\n    assert_eq!(chosen.briefing.as_ref().unwrap().title, "Generation 3");\n\n    let archive = session.archive().unwrap().unwrap();\n    let before = session.snapshot();\n''',
    1,
)
external.write_text(e)
