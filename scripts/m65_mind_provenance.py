from pathlib import Path

lib = Path('worlds/pocket-universe/src/lib.rs')
s = lib.read_text()
s = s.replace('pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.4.0";', 'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.5.0";', 1)
s = s.replace(
    'const AGENT_EXPLORE_COUNT: &str = "explore_count";\n',
    'const AGENT_EXPLORE_COUNT: &str = "explore_count";\nconst MIND_PROFILE_ARG: &str = "mind_profile";\nconst LAST_MIND_PROFILE: &str = "last_mind_profile";\nconst DETERMINISTIC_MIND_PROFILE: &str = "deterministic";\nconst CUSTOM_MIND_PROFILE: &str = "custom";\n',
    1,
)
s = s.replace(
    '''    actions: ActionRegistry,\n    mind: R,\n}\n''',
    '''    actions: ActionRegistry,\n    mind: R,\n    mind_profile: String,\n}\n''',
    1,
)
s = s.replace(
    '''    pub fn new() -> Result<Self, Box<dyn Error>> {\n        Self::with_agent_runtime(PocketMind)\n    }\n\n    pub fn resume_archive(archive: &WorldArchive) -> Result<Self, Box<dyn Error>> {\n        Self::resume_archive_with_agent_runtime(archive, PocketMind)\n    }\n''',
    '''    pub fn new() -> Result<Self, Box<dyn Error>> {\n        Self::with_agent_runtime_profile(PocketMind, DETERMINISTIC_MIND_PROFILE)\n    }\n\n    pub fn resume_archive(archive: &WorldArchive) -> Result<Self, Box<dyn Error>> {\n        Self::resume_archive_with_agent_runtime_profile(\n            archive,\n            PocketMind,\n            DETERMINISTIC_MIND_PROFILE,\n        )\n    }\n''',
    1,
)
s = s.replace(
    '''    pub fn with_agent_runtime(mind: R) -> Result<Self, Box<dyn Error>> {\n        Ok(Self {\n            world: World::new(baseline()?),\n            actions: build_action_registry()?,\n            mind,\n        })\n    }\n''',
    '''    pub fn with_agent_runtime(mind: R) -> Result<Self, Box<dyn Error>> {\n        Self::with_agent_runtime_profile(mind, CUSTOM_MIND_PROFILE)\n    }\n\n    pub fn with_agent_runtime_profile(\n        mind: R,\n        mind_profile: impl Into<String>,\n    ) -> Result<Self, Box<dyn Error>> {\n        Ok(Self {\n            world: World::new(baseline()?),\n            actions: build_action_registry()?,\n            mind,\n            mind_profile: validate_mind_profile(mind_profile.into())?,\n        })\n    }\n''',
    1,
)
s = s.replace(
    '''            let outcome =\n                Self::run_agent_turn_on(&mut self.mind, &mut candidate, &self.actions, &[growth])?;\n''',
    '''            let outcome = Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &self.mind_profile,\n                &[growth],\n            )?;\n''',
    1,
)
s = s.replace(
    '''            Self::run_agent_turn_on(&mut self.mind, &mut candidate, &self.actions, &[growth])?;\n''',
    '''            Self::run_agent_turn_on(\n                &mut self.mind,\n                &mut candidate,\n                &self.actions,\n                &self.mind_profile,\n                &[growth],\n            )?;\n''',
    1,
)
s = s.replace(
    '''        registry: &ActionRegistry,\n        caused_by: &[EventId],\n    ) -> Result<EventId, Box<dyn Error>> {\n''',
    '''        registry: &ActionRegistry,\n        mind_profile: &str,\n        caused_by: &[EventId],\n    ) -> Result<EventId, Box<dyn Error>> {\n''',
    1,
)
s = s.replace(
    '''                ActionRequest::new(AGENT_CARE_ACTION),\n''',
    '''                ActionRequest::new(AGENT_CARE_ACTION).arg(MIND_PROFILE_ARG, mind_profile),\n''',
    1,
)
s = s.replace(
    '''                ActionRequest::new(AGENT_EXPLORE_ACTION),\n''',
    '''                ActionRequest::new(AGENT_EXPLORE_ACTION).arg(MIND_PROFILE_ARG, mind_profile),\n''',
    1,
)
s = s.replace(
    '''    pub fn resume_archive_with_agent_runtime(\n        archive: &WorldArchive,\n        mind: R,\n    ) -> Result<Self, Box<dyn Error>> {\n        Ok(Self {\n            world: archive.restore(&pocket_universe_pack_ref(), baseline()?)?,\n            actions: build_action_registry()?,\n            mind,\n        })\n    }\n''',
    '''    pub fn resume_archive_with_agent_runtime(\n        archive: &WorldArchive,\n        mind: R,\n    ) -> Result<Self, Box<dyn Error>> {\n        Self::resume_archive_with_agent_runtime_profile(archive, mind, CUSTOM_MIND_PROFILE)\n    }\n\n    pub fn resume_archive_with_agent_runtime_profile(\n        archive: &WorldArchive,\n        mind: R,\n        mind_profile: impl Into<String>,\n    ) -> Result<Self, Box<dyn Error>> {\n        Ok(Self {\n            world: archive.restore(&pocket_universe_pack_ref(), baseline()?)?,\n            actions: build_action_registry()?,\n            mind,\n            mind_profile: validate_mind_profile(mind_profile.into())?,\n        })\n    }\n''',
    1,
)

# Session takes explicit profile so factory-based create/open preserve the same non-secret provenance label.
s = s.replace(
    '''    fn fresh(mind: R) -> Result<Box<dyn WorldSession>, HostError> {\n        Ok(Box::new(Self {\n            world: PocketUniverse::with_agent_runtime(mind).map_err(HostError::session)?,\n''',
    '''    fn fresh(mind: R, mind_profile: &str) -> Result<Box<dyn WorldSession>, HostError> {\n        Ok(Box::new(Self {\n            world: PocketUniverse::with_agent_runtime_profile(mind, mind_profile)\n                .map_err(HostError::session)?,\n''',
    1,
)
s = s.replace(
    '''    fn open_archive(archive: &WorldArchive, mind: R) -> Result<Box<dyn WorldSession>, HostError> {\n        Ok(Box::new(Self {\n            world: PocketUniverse::resume_archive_with_agent_runtime(archive, mind)\n                .map_err(HostError::session)?,\n''',
    '''    fn open_archive(\n        archive: &WorldArchive,\n        mind: R,\n        mind_profile: &str,\n    ) -> Result<Box<dyn WorldSession>, HostError> {\n        Ok(Box::new(Self {\n            world: PocketUniverse::resume_archive_with_agent_runtime_profile(\n                archive,\n                mind,\n                mind_profile,\n            )\n            .map_err(HostError::session)?,\n''',
    1,
)

old_reg = '''pub fn pocket_universe_registration() -> WorldRegistration {\n    pocket_universe_registration_with_agent_runtime(|| PocketMind)\n}\n\npub fn pocket_universe_registration_with_agent_runtime<R, F>(factory: F) -> WorldRegistration\nwhere\n    R: AgentRuntime + 'static,\n    F: Fn() -> R + Send + Sync + 'static,\n{\n    let factory = Arc::new(factory);\n    let create_factory = Arc::clone(&factory);\n    let open_factory = Arc::clone(&factory);\n    WorldRegistration::new(pocket_universe_descriptor(), move || {\n        PocketUniverseSession::fresh(create_factory())\n    })\n    .with_archive_opener(move |archive| {\n        PocketUniverseSession::open_archive(archive, open_factory())\n    })\n}\n'''
new_reg = '''pub fn pocket_universe_registration() -> WorldRegistration {\n    pocket_universe_registration_with_agent_runtime_profile(\n        || PocketMind,\n        DETERMINISTIC_MIND_PROFILE,\n    )\n}\n\npub fn pocket_universe_registration_with_agent_runtime<R, F>(factory: F) -> WorldRegistration\nwhere\n    R: AgentRuntime + 'static,\n    F: Fn() -> R + Send + Sync + 'static,\n{\n    pocket_universe_registration_with_agent_runtime_profile(factory, CUSTOM_MIND_PROFILE)\n}\n\npub fn pocket_universe_registration_with_agent_runtime_profile<R, F>(\n    factory: F,\n    mind_profile: impl Into<String>,\n) -> WorldRegistration\nwhere\n    R: AgentRuntime + 'static,\n    F: Fn() -> R + Send + Sync + 'static,\n{\n    let factory = Arc::new(factory);\n    let mind_profile = Arc::new(\n        validate_mind_profile(mind_profile.into())\n            .expect("Pocket Universe registration mind profile must be a safe non-secret label"),\n    );\n    let create_factory = Arc::clone(&factory);\n    let open_factory = Arc::clone(&factory);\n    let create_profile = Arc::clone(&mind_profile);\n    let open_profile = Arc::clone(&mind_profile);\n    WorldRegistration::new(pocket_universe_descriptor(), move || {\n        PocketUniverseSession::fresh(create_factory(), create_profile.as_str())\n    })\n    .with_archive_opener(move |archive| {\n        PocketUniverseSession::open_archive(archive, open_factory(), open_profile.as_str())\n    })\n}\n'''
if old_reg not in s:
    raise SystemExit('registration block not found')
s = s.replace(old_reg, new_reg, 1)

# Initial actor state exposes a stable row before any mind turn.
s = s.replace('.with_component(AGENT_EXPLORE_COUNT, 0_i64),', '.with_component(AGENT_EXPLORE_COUNT, 0_i64)\n                    .with_component(LAST_MIND_PROFILE, "none"),')

# Outcome actions require and persist the profile.
needle = '''    let seed = seed_id_from_state(state)?;\n    if seed == UNSEEDED {\n'''
replacement = '''    let mind_profile = match request.args.get(MIND_PROFILE_ARG) {\n        Some(Value::Text(profile)) if is_valid_mind_profile(profile) => profile.clone(),\n        _ => {\n            return Err(ActionError::Invalid(\n                "Pocket Mind action requires a valid mind_profile label".into(),\n            ))\n        }\n    };\n    let seed = seed_id_from_state(state)?;\n    if seed == UNSEEDED {\n'''
# Apply only in mind_action_draft: find after its actor validation.
pos = s.index('fn mind_action_draft(')
idx = s.index(needle, pos)
s = s[:idx] + s[idx:].replace(needle, replacement, 1)

s = s.replace(
    '''    draft.payload.insert("turn".into(), next.into());\n''',
    '''    draft.payload.insert("turn".into(), next.into());\n    draft\n        .payload\n        .insert(MIND_PROFILE_ARG.into(), mind_profile.clone().into());\n''',
    1,
)
s = s.replace(
    '''        StateChange::SetComponent {\n            entity: actor,\n            key: "last_intent".into(),\n            value: if care { "care" } else { "explore" }.into(),\n        },\n''',
    '''        StateChange::SetComponent {\n            entity: actor,\n            key: "last_intent".into(),\n            value: if care { "care" } else { "explore" }.into(),\n        },\n        StateChange::SetComponent {\n            entity: actor,\n            key: LAST_MIND_PROFILE.into(),\n            value: mind_profile.into(),\n        },\n''',
    1,
)

# Add profile validation helpers before seed_id.
marker = '''pub(crate) fn seed_id(world: &World) -> &str {\n'''
helpers = '''fn validate_mind_profile(profile: String) -> Result<String, std::io::Error> {\n    if is_valid_mind_profile(&profile) {\n        Ok(profile)\n    } else {\n        Err(std::io::Error::other(\n            "mind profile must be a non-empty <=64 character label using a-z, 0-9, '.', '_' or '-'",\n        ))\n    }\n}\n\nfn is_valid_mind_profile(profile: &str) -> bool {\n    !profile.is_empty()\n        && profile.len() <= 64\n        && profile\n            .bytes()\n            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-'))\n}\n\n'''
if marker not in s:
    raise SystemExit('seed_id marker missing')
s = s.replace(marker, helpers + marker, 1)

# Tests: compare same action under two mind profiles and validate persisted provenance.
test_marker = '''    #[test]\n    fn deterministic_mind_uses_durable_actor_memory_even_without_time_advancing() {\n'''
new_test = r'''    #[test]
    fn mind_profile_is_durable_and_visible_to_snapshot_compare() {
        use world_compare::{compare_snapshots, DifferenceKind};

        let mut left = PocketUniverse::with_agent_runtime_profile(
            MockAgentRuntime::scripted([AGENT_CARE_ACTION]),
            "mind-a",
        )
        .unwrap();
        let mut right = PocketUniverse::with_agent_runtime_profile(
            MockAgentRuntime::scripted([AGENT_CARE_ACTION]),
            "mind-b",
        )
        .unwrap();
        left.invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        right
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        left.advance_periods(1).unwrap();
        right.advance_periods(1).unwrap();

        let left_outcome = left
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "agent_cared_for_world")
            .unwrap();
        assert_eq!(
            left_outcome.payload.get(MIND_PROFILE_ARG),
            Some(&Value::Text("mind-a".into()))
        );

        let comparison = compare_snapshots(&left.projection_snapshot(), &right.projection_snapshot());
        let actor = comparison
            .entities
            .iter()
            .find(|difference| difference.id == world_projection::SelectionId::Entity(SLOT_B))
            .unwrap();
        assert_eq!(actor.kind, DifferenceKind::Changed);
        let profile = actor
            .inspector_rows
            .iter()
            .find(|row| row.key.label == "Last Mind Profile")
            .unwrap();
        assert_eq!(profile.left.as_deref(), Some("mind-a"));
        assert_eq!(profile.right.as_deref(), Some("mind-b"));
    }

    #[test]
    fn mind_profile_rejects_secret_shaped_or_freeform_values() {
        let error = PocketUniverse::with_agent_runtime_profile(PocketMind, "pi api-key=secret")
            .unwrap_err();
        assert!(error.to_string().contains("mind profile must be"));
    }

'''
if test_marker not in s:
    raise SystemExit('test marker not found')
s = s.replace(test_marker, new_test + test_marker, 1)
lib.write_text(s)

# Add compare as a dev-only dependency.
cargo = Path('worlds/pocket-universe/Cargo.toml')
c = cargo.read_text().replace('version = "0.4.0"', 'version = "0.5.0"', 1)
if '[dev-dependencies]' not in c:
    c += '\n\n[dev-dependencies]\nworld-compare = { path = "../../crates/world-compare" }\n'
cargo.write_text(c)

# App: Pi gets a stable public provenance label; deterministic path already gets its profile from default registration.
app_main = Path('apps/pocket-universe-pack/src/main.rs')
a = app_main.read_text()
a = a.replace(
    'pocket_universe_registration_with_agent_runtime,\n',
    'pocket_universe_registration_with_agent_runtime_profile,\n',
    1,
)
a = a.replace(
    '''            serve_stdio(pocket_universe_registration_with_agent_runtime(move || {\n                PiRpcRuntime::new(ProcessPiRpcTransport::new(command.clone()))\n            }))?;\n''',
    '''            serve_stdio(pocket_universe_registration_with_agent_runtime_profile(\n                move || PiRpcRuntime::new(ProcessPiRpcTransport::new(command.clone())),\n                "pi",\n            ))?;\n''',
    1,
)
app_main.write_text(a)

app_cargo = Path('apps/pocket-universe-pack/Cargo.toml')
app_cargo.write_text(app_cargo.read_text().replace('version = "0.4.0"', 'version = "0.5.0"', 1))

# External E2E checks both history-level and current-state provenance.
ext = Path('apps/pocket-universe-pack/tests/external_pack.rs')
e = ext.read_text()
# deterministic archive should contain deterministic profile on at least one agent outcome.
needle = '''    assert!(archive.events.iter().any(|event| {\n        event.kind == "agent_cared_for_world" || event.kind == "agent_explored_world"\n    }));\n'''
replacement = needle + '''    assert!(archive.events.iter().any(|event| {\n        (event.kind == "agent_cared_for_world" || event.kind == "agent_explored_world")\n            && event.payload.get("mind_profile")\n                == Some(&world_persistence::ArchivedValue::Text("deterministic".into()))\n    }));\n'''
if needle not in e:
    raise SystemExit('deterministic outcome assertion missing')
e = e.replace(needle, replacement, 1)
# Pi archive outcome payload profile.
needle2 = '''        assert!(pi_archive\n            .events\n            .iter()\n            .any(|event| event.kind == "agent_explored_world"));\n'''
replacement2 = needle2 + '''        assert!(pi_archive.events.iter().any(|event| {\n            event.kind == "agent_explored_world"\n                && event.payload.get("mind_profile")\n                    == Some(&world_persistence::ArchivedValue::Text("pi".into()))\n        }));\n'''
if needle2 not in e:
    raise SystemExit('pi outcome assertion missing')
e = e.replace(needle2, replacement2, 1)
# Reopened snapshot actor inspector exposes last mind profile for Compare.
needle3 = '''        let mut reopened_without_pi = registry.open_archive(&pi_archive).unwrap();\n        assert_eq!(\n            reopened_without_pi.archive().unwrap().unwrap(),\n            pi_archive,\n            "fresh Open must restore recorded truth without invoking Pi"\n        );\n'''
replacement3 = needle3 + '''        let reopened_snapshot = reopened_without_pi.snapshot();\n        let actor = reopened_snapshot\n            .inspectors\n            .values()\n            .find(|inspector| inspector.title == "Nia Chen")\n            .expect("Pi actor inspector");\n        assert!(actor.sections.iter().flat_map(|section| &section.rows).any(|row| {\n            row.label == "Last Mind Profile" && row.value == "pi"\n        }));\n'''
if needle3 not in e:
    raise SystemExit('reopen assertion missing')
e = e.replace(needle3, replacement3, 1)
ext.write_text(e)
