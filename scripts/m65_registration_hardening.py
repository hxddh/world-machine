from pathlib import Path

lib = Path('worlds/pocket-universe/src/lib.rs')
s = lib.read_text()
old = '''pub fn pocket_universe_registration() -> WorldRegistration {\n    pocket_universe_registration_with_agent_runtime_profile(\n        || PocketMind,\n        DETERMINISTIC_MIND_PROFILE,\n    )\n}\n\npub fn pocket_universe_registration_with_agent_runtime<R, F>(factory: F) -> WorldRegistration\nwhere\n    R: AgentRuntime + 'static,\n    F: Fn() -> R + Send + Sync + 'static,\n{\n    pocket_universe_registration_with_agent_runtime_profile(factory, CUSTOM_MIND_PROFILE)\n}\n\npub fn pocket_universe_registration_with_agent_runtime_profile<R, F>(\n    factory: F,\n    mind_profile: impl Into<String>,\n) -> WorldRegistration\nwhere\n    R: AgentRuntime + 'static,\n    F: Fn() -> R + Send + Sync + 'static,\n{\n    let factory = Arc::new(factory);\n    let mind_profile = Arc::new(\n        validate_mind_profile(mind_profile.into())\n            .expect("Pocket Universe registration mind profile must be a safe non-secret label"),\n    );\n    let create_factory = Arc::clone(&factory);\n    let open_factory = Arc::clone(&factory);\n    let create_profile = Arc::clone(&mind_profile);\n    let open_profile = Arc::clone(&mind_profile);\n    WorldRegistration::new(pocket_universe_descriptor(), move || {\n        PocketUniverseSession::fresh(create_factory(), create_profile.as_str())\n    })\n    .with_archive_opener(move |archive| {\n        PocketUniverseSession::open_archive(archive, open_factory(), open_profile.as_str())\n    })\n}\n'''
new = '''pub fn pocket_universe_registration() -> WorldRegistration {\n    registration_with_validated_profile(|| PocketMind, DETERMINISTIC_MIND_PROFILE)\n}\n\npub fn pocket_universe_registration_with_agent_runtime<R, F>(factory: F) -> WorldRegistration\nwhere\n    R: AgentRuntime + 'static,\n    F: Fn() -> R + Send + Sync + 'static,\n{\n    registration_with_validated_profile(factory, CUSTOM_MIND_PROFILE)\n}\n\npub fn pocket_universe_registration_with_agent_runtime_profile<R, F>(\n    factory: F,\n    mind_profile: impl Into<String>,\n) -> Result<WorldRegistration, std::io::Error>\nwhere\n    R: AgentRuntime + 'static,\n    F: Fn() -> R + Send + Sync + 'static,\n{\n    let mind_profile = validate_mind_profile(mind_profile.into())?;\n    Ok(registration_with_validated_profile(factory, mind_profile))\n}\n\nfn registration_with_validated_profile<R, F>(\n    factory: F,\n    mind_profile: impl Into<String>,\n) -> WorldRegistration\nwhere\n    R: AgentRuntime + 'static,\n    F: Fn() -> R + Send + Sync + 'static,\n{\n    let factory = Arc::new(factory);\n    let mind_profile = Arc::new(mind_profile.into());\n    let create_factory = Arc::clone(&factory);\n    let open_factory = Arc::clone(&factory);\n    let create_profile = Arc::clone(&mind_profile);\n    let open_profile = Arc::clone(&mind_profile);\n    WorldRegistration::new(pocket_universe_descriptor(), move || {\n        PocketUniverseSession::fresh(create_factory(), create_profile.as_str())\n    })\n    .with_archive_opener(move |archive| {\n        PocketUniverseSession::open_archive(archive, open_factory(), open_profile.as_str())\n    })\n}\n'''
if old not in s:
    raise SystemExit('registration provenance block not found')
s = s.replace(old, new, 1)

# The compare test uses a direct World constructor and is unchanged. Add explicit public registration rejection.
marker = '''    #[test]\n    fn mind_profile_rejects_secret_shaped_or_freeform_values() {\n'''
extra = r'''    #[test]
    fn registration_profile_rejects_secret_shaped_or_freeform_values_without_panicking() {
        let error = pocket_universe_registration_with_agent_runtime_profile(
            || PocketMind,
            "pi api-key=secret",
        )
        .err()
        .expect("freeform registration mind profile must be rejected");
        assert!(error.to_string().contains("mind profile must be"));
    }

'''
if marker not in s:
    raise SystemExit('mind profile test marker missing')
s = s.replace(marker, extra + marker, 1)
lib.write_text(s)

main = Path('apps/pocket-universe-pack/src/main.rs')
a = main.read_text()
old_app = '''            serve_stdio(pocket_universe_registration_with_agent_runtime_profile(\n                move || PiRpcRuntime::new(ProcessPiRpcTransport::new(command.clone())),\n                "pi",\n            ))?;\n'''
new_app = '''            let registration = pocket_universe_registration_with_agent_runtime_profile(\n                move || PiRpcRuntime::new(ProcessPiRpcTransport::new(command.clone())),\n                "pi",\n            )?;\n            serve_stdio(registration)?;\n'''
if old_app not in a:
    raise SystemExit('Pi registration call not found')
main.write_text(a.replace(old_app, new_app, 1))
