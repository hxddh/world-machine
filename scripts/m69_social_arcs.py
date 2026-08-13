from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


lib_path = ROOT / "worlds/pocket-universe/src/lib.rs"
text = lib_path.read_text()
text = replace_once(text, 'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.8.0";', 'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.9.0";', "pack version")
text = replace_once(
    text,
    'const RELATIONSHIP_LAST_DYNAMIC: &str = "last_dynamic";\n',
    'const RELATIONSHIP_LAST_DYNAMIC: &str = "last_dynamic";\nconst RELATIONSHIP_SOCIAL_ARC: &str = "social_arc";\n',
    "social arc constant",
)
text = replace_once(
    text,
    '    actions.register(UpdateRelationship)?;\n    actions.register(SteerSharedProject)?;\n',
    '    actions.register(UpdateRelationship)?;\n    actions.register(ResolveSocialArc)?;\n    actions.register(SteerSharedProject)?;\n',
    "register social arc action",
)
text = replace_once(
    text,
    'struct UpdateRelationship;\nstruct SteerSharedProject;\n',
    'struct UpdateRelationship;\nstruct ResolveSocialArc;\nstruct SteerSharedProject;\n',
    "declare social arc action",
)
old_nudge = '''            let relationship = candidate
                .execute(
                    &self.actions,
                    &ActionRequest::new("update_relationship")
                        .caused_by(primary_outcome)
                        .caused_by(secondary_outcome),
                )?
                .id;
            self.world = candidate;
            return Ok(relationship);
'''
new_nudge = '''            let relationship = candidate
                .execute(
                    &self.actions,
                    &ActionRequest::new("update_relationship")
                        .caused_by(primary_outcome)
                        .caused_by(secondary_outcome),
                )?
                .id;
            let returned = if social_arc_candidate(candidate.state())?.is_some() {
                candidate
                    .execute(
                        &self.actions,
                        &ActionRequest::new("resolve_social_arc").caused_by(relationship),
                    )?
                    .id
            } else {
                relationship
            };
            self.world = candidate;
            return Ok(returned);
'''
text = replace_once(text, old_nudge, new_nudge, "nudge arc resolution")
old_advance = '''            candidate.execute(
                &self.actions,
                &ActionRequest::new("update_relationship")
                    .caused_by(primary_outcome)
                    .caused_by(secondary_outcome),
            )?;
'''
new_advance = '''            let relationship = candidate
                .execute(
                    &self.actions,
                    &ActionRequest::new("update_relationship")
                        .caused_by(primary_outcome)
                        .caused_by(secondary_outcome),
                )?
                .id;
            if social_arc_candidate(candidate.state())?.is_some() {
                candidate.execute(
                    &self.actions,
                    &ActionRequest::new("resolve_social_arc").caused_by(relationship),
                )?;
            }
'''
text = replace_once(text, old_advance, new_advance, "background arc resolution")
text = replace_once(
    text,
    '        let decision = decision_id_from_state(state)?;\n        let change = growth_message(&seed, next, &decision);\n',
    '        let decision = decision_id_from_state(state)?;\n        let social_arc = text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC)?;\n        let change = growth_message(&seed, next, &decision, &social_arc);\n',
    "growth reads social arc",
)
resolve_impl = r'''

fn social_arc_candidate(state: &WorldState) -> Result<Option<&'static str>, ActionError> {
    if text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC)? != "forming" {
        return Ok(None);
    }
    let direction = text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_DIRECTION)?;
    let trust = integer_component(state, RELATIONSHIP, RELATIONSHIP_TRUST)?;
    let tension = integer_component(state, RELATIONSHIP, RELATIONSHIP_TENSION)?;

    if direction == "shared-project" && trust >= 5 {
        return Ok(Some("partnership"));
    }
    if direction == "rivalry" && tension >= 5 {
        return Ok(Some("fracture"));
    }
    if trust >= 5 && trust >= tension + 2 {
        return Ok(Some("partnership"));
    }
    if tension >= 5 && tension >= trust + 2 {
        return Ok(Some("fracture"));
    }
    Ok(None)
}

impl Action for ResolveSocialArc {
    fn name(&self) -> &'static str {
        "resolve_social_arc"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let arc = social_arc_candidate(state)?.ok_or_else(|| {
            ActionError::Invalid("relationship has not reached a social-arc threshold".into())
        })?;
        let seed = seed_id_from_state(state)?;
        let trust = integer_component(state, RELATIONSHIP, RELATIONSHIP_TRUST)?;
        let tension = integer_component(state, RELATIONSHIP, RELATIONSHIP_TENSION)?;
        let (kind, summary, target, key, value) = match (seed.as_str(), arc) {
            ("mars-colony", "partnership") => (
                "partnership_formed",
                "Nia and Tomas stopped dividing the work into separate turns. Kestrel now launches with them as one expedition crew.",
                SLOT_D,
                "status",
                "joint expedition crew",
            ),
            ("mars-colony", "fracture") => (
                "relationship_fractured",
                "Nia and Tomas stopped trusting the same route. Kestrel now runs split survey plans with competing priorities.",
                SLOT_D,
                "status",
                "split survey routes",
            ),
            ("1980s-town", "partnership") => (
                "partnership_formed",
                "Lena and Max turned their late-night improvisation into a real partnership. K-88 now carries a shared neighborhood show.",
                SLOT_C,
                "format",
                "Lena + Max neighborhood show",
            ),
            ("1980s-town", "fracture") => (
                "relationship_fractured",
                "Lena and Max began pulling the same audience in different directions. K-88 now schedules competing late shows.",
                SLOT_C,
                "format",
                "competing late shows",
            ),
            ("penguin-civilization", "partnership") => (
                "partnership_formed",
                "Piko and Miri turned their different duties into one shared watch. The Aurora Council now plans around their joint reports.",
                SLOT_D,
                "custom",
                "shared watch council",
            ),
            ("penguin-civilization", "fracture") => (
                "relationship_fractured",
                "Piko and Miri split the colony's priorities into rival camps. The Aurora Council now meets as two moonrise caucuses.",
                SLOT_D,
                "custom",
                "split moonrise caucuses",
            ),
            _ => {
                return Err(ActionError::Invalid(format!(
                    "unsupported Pocket Universe social arc: seed={seed}, arc={arc}"
                )))
            }
        };
        let mut draft = EventDraft::new(kind);
        draft.targets = vec![RELATIONSHIP, SLOT_B, SLOT_E, target];
        draft.payload.insert("social_arc".into(), arc.into());
        draft.payload.insert("trust".into(), trust.into());
        draft.payload.insert("tension".into(), tension.into());
        draft.payload.insert("summary".into(), summary.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: RELATIONSHIP_SOCIAL_ARC.into(),
                value: arc.into(),
            },
            StateChange::SetComponent {
                entity: RELATIONSHIP,
                key: RELATIONSHIP_LAST_DYNAMIC.into(),
                value: summary.into(),
            },
            StateChange::SetComponent {
                entity: target,
                key: key.into(),
                value: value.into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LAST_CHANGE.into(),
                value: summary.into(),
            },
        ];
        Ok(draft)
    }
}
'''
text = replace_once(
    text,
    '\nimpl Action for SteerSharedProject {\n',
    resolve_impl + '\nimpl Action for SteerSharedProject {\n',
    "insert social arc resolver",
)
text = replace_once(
    text,
    '        .with_component(RELATIONSHIP_DIRECTION, "none")\n        .with_component(RELATIONSHIP_LAST_DYNAMIC, "forming")\n',
    '        .with_component(RELATIONSHIP_DIRECTION, "none")\n        .with_component(RELATIONSHIP_SOCIAL_ARC, "forming")\n        .with_component(RELATIONSHIP_LAST_DYNAMIC, "forming")\n',
    "seed social arc state",
)
old_growth = '''fn growth_message(seed: &str, generation: i64, decision: &str) -> String {
'''
new_growth = '''fn growth_message(seed: &str, generation: i64, decision: &str, social_arc: &str) -> String {
'''
text = replace_once(text, old_growth, new_growth, "growth signature")
old_growth_tail = '''    if decision == "none" {
        return base.into();
    }
    let consequence = match decision {
        "follow-signal" => "The signal expedition keeps pulling attention beyond the safe ridge.",
        "fortify-habitat" => "The stronger habitat makes every later risk feel more deliberate.",
        "community-arcade" => {
            "The arcade is becoming a place people organize their evenings around."
        }
        "steady-business" => "The arcade survives by staying small, predictable, and open.",
        "winter-feast" => {
            "The feast has turned Icebridge into a meeting point for distant colonies."
        }
        "conserve-reserves" => {
            "The sealed reserve gives the council more room to plan for the dark season."
        }
        _ => "The earlier intervention is still shaping what happens next.",
    };
    format!("{base} {consequence}")
}
'''
new_growth_tail = '''    let mut story = base.to_owned();
    if decision != "none" {
        let consequence = match decision {
            "follow-signal" => "The signal expedition keeps pulling attention beyond the safe ridge.",
            "fortify-habitat" => "The stronger habitat makes every later risk feel more deliberate.",
            "community-arcade" => {
                "The arcade is becoming a place people organize their evenings around."
            }
            "steady-business" => "The arcade survives by staying small, predictable, and open.",
            "winter-feast" => {
                "The feast has turned Icebridge into a meeting point for distant colonies."
            }
            "conserve-reserves" => {
                "The sealed reserve gives the council more room to plan for the dark season."
            }
            _ => "The earlier intervention is still shaping what happens next.",
        };
        story.push(' ');
        story.push_str(consequence);
    }
    let social_consequence = match (seed, social_arc) {
        (_, "forming") => None,
        ("mars-colony", "partnership") => {
            Some("Nia and Tomas now plan each rover cycle as one crew.")
        }
        ("mars-colony", "fracture") => {
            Some("Nia and Tomas now divide rover access into competing routes.")
        }
        ("1980s-town", "partnership") => {
            Some("Lena and Max now turn late-night discoveries into one shared broadcast.")
        }
        ("1980s-town", "fracture") => {
            Some("Lena and Max now compete to define the neighborhood's late-night rhythm.")
        }
        ("penguin-civilization", "partnership") => {
            Some("Piko and Miri now bring one shared watch report to the council.")
        }
        ("penguin-civilization", "fracture") => {
            Some("Piko and Miri now bring rival priorities to each moonrise council.")
        }
        (_, _) => Some("The relationship between the world's actors is now shaping later events."),
    };
    if let Some(social_consequence) = social_consequence {
        story.push(' ');
        story.push_str(social_consequence);
    }
    story
}
'''
text = replace_once(text, old_growth_tail, new_growth_tail, "growth social consequence")

test_anchor = '''    #[test]
    fn deterministic_default_mind_keeps_identical_worlds_reproducible() {
'''
new_tests = r'''    #[test]
    fn shared_project_cascades_into_a_partnership_that_changes_the_world() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(2).unwrap();
        universe
            .invoke_projection_command(SHARED_PROJECT_COMMAND)
            .unwrap();
        universe.advance_periods(1).unwrap();

        let relationship = universe.world().state().entity(RELATIONSHIP).unwrap();
        assert_eq!(
            relationship.component(RELATIONSHIP_SOCIAL_ARC),
            Some(&Value::Text("partnership".into()))
        );
        assert_eq!(
            universe
                .world()
                .state()
                .entity(SLOT_D)
                .unwrap()
                .component("status"),
            Some(&Value::Text("joint expedition crew".into()))
        );
        let partnership = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "partnership_formed")
            .expect("partnership event");
        assert_eq!(partnership.caused_by.len(), 1);
        let relationship_shift = partnership.caused_by[0];
        assert_eq!(
            universe
                .world()
                .events()
                .iter()
                .find(|event| event.id == relationship_shift)
                .map(|event| event.kind.as_str()),
            Some("relationship_shifted")
        );
        let why = universe.projection_snapshot().why(partnership.id).unwrap();
        let growth = universe
            .world()
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "universe_grew")
            .unwrap()
            .id;
        assert!(why.nodes.iter().any(|node| node.event == growth));

        universe.advance_periods(1).unwrap();
        let later_growth = universe
            .world()
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "universe_grew")
            .unwrap();
        assert!(matches!(
            later_growth.payload.get("change"),
            Some(Value::Text(change)) if change.contains("one crew")
        ));
    }

    #[test]
    fn rivalry_cascades_into_a_fracture_that_changes_the_world_and_is_forkable() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(2).unwrap();
        universe.invoke_projection_command(RIVALRY_COMMAND).unwrap();
        universe.advance_periods(2).unwrap();

        let relationship = universe.world().state().entity(RELATIONSHIP).unwrap();
        assert_eq!(
            relationship.component(RELATIONSHIP_SOCIAL_ARC),
            Some(&Value::Text("fracture".into()))
        );
        assert_eq!(
            universe
                .world()
                .state()
                .entity(SLOT_D)
                .unwrap()
                .component("status"),
            Some(&Value::Text("split survey routes".into()))
        );
        let fractured = universe
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "relationship_fractured")
            .expect("fracture event")
            .id;

        universe.fork_before_event(fractured).unwrap();
        assert_eq!(
            universe
                .world()
                .state()
                .entity(RELATIONSHIP)
                .unwrap()
                .component(RELATIONSHIP_SOCIAL_ARC),
            Some(&Value::Text("forming".into()))
        );
        assert_ne!(
            universe
                .world()
                .state()
                .entity(SLOT_D)
                .unwrap()
                .component("status"),
            Some(&Value::Text("split survey routes".into()))
        );
    }

'''
text = replace_once(text, test_anchor, new_tests + test_anchor, "social arc tests")
lib_path.write_text(text)

projection_path = ROOT / "worlds/pocket-universe/src/projection.rs"
projection = projection_path.read_text()
projection = replace_once(
    projection,
    '            "relationship_shifted" => "Their relationship changed".into(),\n            "relationship_steered" => "You changed their direction".into(),\n',
    '            "relationship_shifted" => "Their relationship changed".into(),\n            "relationship_steered" => "You changed their direction".into(),\n            "partnership_formed" => "A partnership formed".into(),\n            "relationship_fractured" => "Their relationship fractured".into(),\n',
    "briefing social arc titles",
)
projection_path.write_text(projection)

for rel in ["worlds/pocket-universe/Cargo.toml", "apps/pocket-universe-pack/Cargo.toml"]:
    path = ROOT / rel
    cargo = path.read_text()
    cargo = replace_once(cargo, 'version = "0.8.0"', 'version = "0.9.0"', f"{rel} version")
    path.write_text(cargo)

external_path = ROOT / "apps/pocket-universe-pack/tests/external_pack.rs"
external = external_path.read_text()
external = replace_once(
    external,
    '        pi_session.advance_background(1).unwrap();\n',
    '        pi_session.advance_background(3).unwrap();\n',
    "Pi periods",
)
external = replace_once(
    external,
    '''        assert_eq!(
            pi_archive
                .events
                .iter()
                .filter(|event| event.kind == "agent_explored_world")
                .count(),
            2
        );
''',
    '''        assert_eq!(
            pi_archive
                .events
                .iter()
                .filter(|event| event.kind == "agent_explored_world")
                .count(),
            6
        );
''',
    "Pi explore count",
)
external = replace_once(
    external,
    '''        assert!(relationship
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .any(|row| row.label == "Tension" && row.value == "2"));

        for actor_title in ["Nia Chen", "Tomas Vale"] {
''',
    '''        assert!(relationship
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .any(|row| row.label == "Tension" && row.value == "6"));
        assert!(relationship
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .any(|row| row.label == "Social Arc" && row.value == "fracture"));
        let rover = reopened_snapshot
            .inspectors
            .values()
            .find(|inspector| inspector.title == "Kestrel Rover")
            .expect("Pi rover inspector");
        assert!(rover
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .any(|row| row.label == "Status" && row.value == "split survey routes"));

        for actor_title in ["Nia Chen", "Tomas Vale"] {
''',
    "Pi social arc assertions",
)
external_path.write_text(external)
