from pathlib import Path

lib = Path('worlds/pocket-universe/src/lib.rs')
s = lib.read_text()
s = s.replace('use std::error::Error;\n', 'use std::error::Error;\nuse std::sync::Arc;\n', 1)
s = s.replace('pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.3.0";', 'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.4.0";', 1)

old_session = '''struct PocketUniverseSession {\n    world: PocketUniverse,\n    return_since_event_count: Option<usize>,\n}\n\nimpl PocketUniverseSession {\n    fn fresh() -> Result<Box<dyn WorldSession>, HostError> {\n        Ok(Box::new(Self {\n            world: PocketUniverse::new().map_err(HostError::session)?,\n            return_since_event_count: None,\n        }))\n    }\n\n    fn open_archive(archive: &WorldArchive) -> Result<Box<dyn WorldSession>, HostError> {\n        Ok(Box::new(Self {\n            world: PocketUniverse::resume_archive(archive).map_err(HostError::session)?,\n            return_since_event_count: None,\n        }))\n    }\n}\n\nimpl WorldSession for PocketUniverseSession {\n'''
new_session = '''struct PocketUniverseSession<R>\nwhere\n    R: AgentRuntime,\n{\n    world: PocketUniverse<R>,\n    return_since_event_count: Option<usize>,\n}\n\nimpl<R> PocketUniverseSession<R>\nwhere\n    R: AgentRuntime + 'static,\n{\n    fn fresh(mind: R) -> Result<Box<dyn WorldSession>, HostError> {\n        Ok(Box::new(Self {\n            world: PocketUniverse::with_agent_runtime(mind).map_err(HostError::session)?,\n            return_since_event_count: None,\n        }))\n    }\n\n    fn open_archive(archive: &WorldArchive, mind: R) -> Result<Box<dyn WorldSession>, HostError> {\n        Ok(Box::new(Self {\n            world: PocketUniverse::resume_archive_with_agent_runtime(archive, mind)\n                .map_err(HostError::session)?,\n            return_since_event_count: None,\n        }))\n    }\n}\n\nimpl<R> WorldSession for PocketUniverseSession<R>\nwhere\n    R: AgentRuntime + 'static,\n{\n'''
if old_session not in s:
    raise SystemExit('PocketUniverseSession block not found')
s = s.replace(old_session, new_session, 1)

old_registration = '''pub fn pocket_universe_registration() -> WorldRegistration {\n    WorldRegistration::new(\n        WorldDescriptor {\n            pack: pocket_universe_pack_ref(),\n            title: "Pocket Universe".into(),\n            description:\n                "Create a tiny persistent world, let it grow, then return to see what changed."\n                    .into(),\n        },\n        PocketUniverseSession::fresh,\n    )\n    .with_archive_opener(PocketUniverseSession::open_archive)\n}\n'''
new_registration = '''pub fn pocket_universe_descriptor() -> WorldDescriptor {\n    WorldDescriptor {\n        pack: pocket_universe_pack_ref(),\n        title: "Pocket Universe".into(),\n        description:\n            "Create a tiny persistent world, let it grow, then return to see what changed.".into(),\n    }\n}\n\npub fn pocket_universe_registration() -> WorldRegistration {\n    pocket_universe_registration_with_agent_runtime(|| PocketMind)\n}\n\npub fn pocket_universe_registration_with_agent_runtime<R, F>(factory: F) -> WorldRegistration\nwhere\n    R: AgentRuntime + 'static,\n    F: Fn() -> R + Send + Sync + 'static,\n{\n    let factory = Arc::new(factory);\n    let create_factory = Arc::clone(&factory);\n    let open_factory = Arc::clone(&factory);\n    WorldRegistration::new(pocket_universe_descriptor(), move || {\n        PocketUniverseSession::fresh(create_factory())\n    })\n    .with_archive_opener(move |archive| {\n        PocketUniverseSession::open_archive(archive, open_factory())\n    })\n}\n'''
if old_registration not in s:
    raise SystemExit('registration block not found')
s = s.replace(old_registration, new_registration, 1)

marker = '''    #[test]\n    fn empty_universe_offers_multiple_world_seeds() {\n'''
extra = r'''    #[test]
    fn registration_factory_creates_a_fresh_runtime_for_create_and_open() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let created = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&created);
        let registration = pocket_universe_registration_with_agent_runtime(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            PanicMind
        });
        let mut registry = world_host::WorldRegistry::new();
        registry.register(registration).unwrap();

        let session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        assert_eq!(created.load(Ordering::SeqCst), 1);
        let archive = session.archive().unwrap().unwrap();
        drop(session);

        let reopened = registry.open_archive(&archive).unwrap();
        assert_eq!(created.load(Ordering::SeqCst), 2);
        assert_eq!(reopened.archive().unwrap().unwrap(), archive);
    }

'''
# PanicMind is declared later in test module. Rust item order doesn't matter.
if marker not in s:
    raise SystemExit('test marker not found')
s = s.replace(marker, extra + marker, 1)
lib.write_text(s)

world_cargo = Path('worlds/pocket-universe/Cargo.toml')
wc = world_cargo.read_text().replace('version = "0.3.0"', 'version = "0.4.0"', 1)
world_cargo.write_text(wc)

app_cargo = Path('apps/pocket-universe-pack/Cargo.toml')
ac = app_cargo.read_text().replace('version = "0.3.0"', 'version = "0.4.0"', 1)
ac = ac.replace('world-pack-server = { path = "../../crates/world-pack-server" }\n', 'world-pack-server = { path = "../../crates/world-pack-server" }\nworld-pi-rpc = { path = "../../crates/world-pi-rpc" }\n', 1)
app_cargo.write_text(ac)

main = Path('apps/pocket-universe-pack/src/main.rs')
main.write_text(r'''use std::env;
use std::error::Error;
use std::path::PathBuf;
use pocket_universe::{
    pocket_universe_descriptor, pocket_universe_registration,
    pocket_universe_registration_with_agent_runtime,
};
use world_pack_server::{manifest_for_current_exe, serve_stdio, write_current_exe_bundle};
use world_pi_rpc::{PiCommand, PiRpcRuntime, ProcessPiRpcTransport};

const MIND_ENV: &str = "WORLD_MACHINE_POCKET_UNIVERSE_MIND";
const PI_PROGRAM_ENV: &str = "WORLD_MACHINE_PI_PROGRAM";

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    let descriptor = pocket_universe_descriptor();
    if args.len() == 1 && args[0] == "--print-manifest" {
        let manifest = manifest_for_current_exe(&descriptor)?;
        println!("{}", manifest.to_json_pretty()?);
        return Ok(());
    }
    if args.len() == 2 && args[0] == "--write-bundle" {
        let destination = PathBuf::from(&args[1]);
        write_current_exe_bundle(&descriptor, destination)?;
        return Ok(());
    }
    if !args.is_empty() {
        return Err("unsupported arguments; run without arguments as a Pack server, use --print-manifest, or use --write-bundle PATH"
            .to_string()
            .into());
    }

    match env::var(MIND_ENV).as_deref().unwrap_or("deterministic") {
        "deterministic" => serve_stdio(pocket_universe_registration())?,
        "pi" => {
            let program = env::var(PI_PROGRAM_ENV).unwrap_or_else(|_| "pi".into());
            let command = PiCommand::decision_only(program);
            serve_stdio(pocket_universe_registration_with_agent_runtime(move || {
                PiRpcRuntime::new(ProcessPiRpcTransport::new(command.clone()))
            }))?;
        }
        other => {
            return Err(format!(
                "unsupported {MIND_ENV} value {other:?}; expected deterministic or pi"
            )
            .into())
        }
    }
    Ok(())
}
''')

external = Path('apps/pocket-universe-pack/tests/external_pack.rs')
e = external.read_text()
e = e.replace('use std::time::{SystemTime, UNIX_EPOCH};\n', 'use std::time::{SystemTime, UNIX_EPOCH};\n#[cfg(unix)]\nuse std::os::unix::fs::PermissionsExt;\n', 1)

helper_marker = '''fn temp_dir() -> PathBuf {\n'''
helpers = r'''const MIND_ENV: &str = "WORLD_MACHINE_POCKET_UNIVERSE_MIND";
const PI_PROGRAM_ENV: &str = "WORLD_MACHINE_PI_PROGRAM";

struct EnvGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = env::var_os(key);
        env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.as_ref() {
            env::set_var(self.key, previous);
        } else {
            env::remove_var(self.key);
        }
    }
}

#[cfg(unix)]
fn write_fake_pi(path: &PathBuf) {
    fs::write(
        path,
        r#"#!/bin/sh
IFS= read -r request || exit 2
printf '%s\n' '{"type":"text_delta","delta":"WORLD_ACTION:pocket_agent.explore"}'
printf '%s\n' '{"type":"response","command":"prompt","success":true}'
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

'''
if helper_marker not in e:
    raise SystemExit('external helper marker missing')
e = e.replace(helper_marker, helpers + helper_marker, 1)

insert = '''    let reopened = registry.open_archive(&archive).unwrap();\n    assert_eq!(reopened.snapshot(), before);\n    assert_eq!(reopened.archive().unwrap().unwrap(), archive);\n'''
replacement = insert + r'''

    #[cfg(unix)]
    {
        let fake_pi = root.join("fake-pi.sh");
        write_fake_pi(&fake_pi);
        let _mind = EnvGuard::set(MIND_ENV, "pi");
        let pi_program = EnvGuard::set(PI_PROGRAM_ENV, &fake_pi);

        let mut pi_session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
        pi_session
            .handle(ProjectionIntent::InvokeCommand(
                SEED_MARS_COLONY_COMMAND.into(),
            ))
            .unwrap();
        pi_session.advance_background(1).unwrap();
        let pi_archive = pi_session.archive().unwrap().unwrap();
        assert!(pi_archive.events.iter().any(|event| {
            event.kind == "agent_decision_recorded"
                && event.payload.get("selected_action")
                    == Some(&world_core::Value::Text("pocket_agent.explore".into()))
        }));
        assert!(pi_archive
            .events
            .iter()
            .any(|event| event.kind == "agent_explored_world"));
        assert!(!pi_archive
            .events
            .iter()
            .any(|event| event.kind == "agent_cared_for_world"));
        drop(pi_session);

        drop(pi_program);
        let missing_pi = root.join("missing-pi");
        let _missing_program = EnvGuard::set(PI_PROGRAM_ENV, &missing_pi);
        let mut reopened_without_pi = registry.open_archive(&pi_archive).unwrap();
        assert_eq!(
            reopened_without_pi.archive().unwrap().unwrap(),
            pi_archive,
            "fresh Open must restore recorded truth without invoking Pi"
        );

        let error = reopened_without_pi.advance_background(1).unwrap_err();
        assert!(error.to_string().contains("failed to start external Pi runtime"));
        assert_eq!(
            reopened_without_pi.archive().unwrap().unwrap(),
            pi_archive,
            "Pi failure must preserve M63 world-atomic rollback"
        );
    }
'''
if insert not in e:
    raise SystemExit('external test tail not found')
e = e.replace(insert, replacement, 1)
external.write_text(e)
