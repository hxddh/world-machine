use pocket_universe::{
    BOLD_PATH_COMMAND, POCKET_UNIVERSE_PACK_ID, POCKET_UNIVERSE_PACK_VERSION,
    SEED_MARS_COLONY_COMMAND,
};
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};
use world_host::WorldRegistry;
use world_pack_catalog::PackCatalog;
use world_projection::ProjectionIntent;

const MIND_ENV: &str = "WORLD_MACHINE_POCKET_UNIVERSE_MIND";
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

fn temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "world-machine-pocket-universe-external-{}-{nonce}",
        process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn pocket_universe_is_a_real_external_pack_with_durable_seed_and_growth() {
    let binary = env!("CARGO_BIN_EXE_pocket-universe-pack");
    let root = temp_dir();
    let bundle_path = root.join("pocket-universe.worldpack");
    let status = Command::new(binary)
        .arg("--write-bundle")
        .arg(&bundle_path)
        .status()
        .unwrap();
    assert!(status.success());

    let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
    let preview = catalog.inspect_install(&bundle_path).unwrap();
    assert_eq!(preview.pack().id, POCKET_UNIVERSE_PACK_ID);
    assert_eq!(preview.pack().version, POCKET_UNIVERSE_PACK_VERSION);
    let installed = catalog.install_reviewed_pending_probe(&preview).unwrap();
    assert!(!installed.enabled);
    assert!(!installed.active);
    fs::remove_file(&bundle_path).unwrap();

    let probe = catalog.probe(&installed.pack).unwrap();
    assert_eq!(probe.pack, installed.pack);
    assert_eq!(probe.created_world_time, 0);
    assert_eq!(probe.reopened_world_time, 0);

    catalog.set_enabled(&installed.pack, true).unwrap();
    catalog.activate(&installed.pack).unwrap();
    let source = catalog.trusted_source().unwrap();
    let mut registry = WorldRegistry::new();
    registry.install_source(&source).unwrap();

    let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
    let empty = session.snapshot();
    assert_eq!(empty.title, "Pocket Universe · Empty World");
    let seeded = session
        .handle(ProjectionIntent::InvokeCommand(
            SEED_MARS_COLONY_COMMAND.into(),
        ))
        .unwrap();
    assert_eq!(seeded.title, "Ares Pocket Colony");
    assert!(seeded
        .collection
        .items
        .iter()
        .any(|item| item.title == "Ares Habitat"));

    let grown = session.advance_background(3).unwrap();
    assert_eq!(grown.world_time, 30);
    assert_eq!(
        grown
            .briefing
            .as_ref()
            .expect("Pocket Universe has a return briefing")
            .title,
        "While you were away"
    );
    assert!(grown
        .commands
        .iter()
        .any(|command| command.id == BOLD_PATH_COMMAND));
    let chosen = session
        .handle(ProjectionIntent::InvokeCommand(BOLD_PATH_COMMAND.into()))
        .unwrap();
    assert_eq!(chosen.briefing.as_ref().unwrap().title, "Generation 3");

    let archive = session.archive().unwrap().unwrap();
    assert!(archive
        .events
        .iter()
        .any(|event| event.kind == "agent_decision_recorded"));
    assert!(archive.events.iter().any(|event| {
        event.kind == "agent_cared_for_world" || event.kind == "agent_explored_world"
    }));
    assert!(archive.events.iter().any(|event| {
        (event.kind == "agent_cared_for_world" || event.kind == "agent_explored_world")
            && event.payload.get("mind_profile")
                == Some(&world_persistence::ArchivedValue::Text(
                    "deterministic".into(),
                ))
    }));
    let before = session.snapshot();
    drop(session);

    let reopened = registry.open_archive(&archive).unwrap();
    assert_eq!(reopened.snapshot(), before);
    assert_eq!(reopened.archive().unwrap().unwrap(), archive);

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
                    == Some(&world_persistence::ArchivedValue::Text(
                        "pocket_agent.explore".into(),
                    ))
        }));
        assert_eq!(
            pi_archive
                .events
                .iter()
                .filter(|event| event.kind == "agent_explored_world")
                .count(),
            2
        );
        assert!(pi_archive.events.iter().any(|event| {
            event.kind == "agent_explored_world"
                && event.payload.get("mind_profile")
                    == Some(&world_persistence::ArchivedValue::Text("pi".into()))
        }));
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
        let reopened_snapshot = reopened_without_pi.snapshot();
        let relationship = reopened_snapshot
            .inspectors
            .values()
            .find(|inspector| inspector.title == "Nia ↔ Tomas")
            .expect("Pi relationship inspector");
        assert!(relationship
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .any(|row| row.label == "Trust" && row.value == "0"));
        assert!(relationship
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .any(|row| row.label == "Tension" && row.value == "2"));

        for actor_title in ["Nia Chen", "Tomas Vale"] {
            let actor = reopened_snapshot
                .inspectors
                .values()
                .find(|inspector| inspector.title == actor_title)
                .unwrap_or_else(|| panic!("missing Pi actor inspector: {actor_title}"));
            assert!(actor
                .sections
                .iter()
                .flat_map(|section| &section.rows)
                .any(|row| { row.label == "Last Mind Profile" && row.value == "pi" }));
        }

        let error = reopened_without_pi.advance_background(1).unwrap_err();
        assert!(error
            .to_string()
            .contains("failed to start external Pi runtime"));
        assert_eq!(
            reopened_without_pi.archive().unwrap().unwrap(),
            pi_archive,
            "Pi failure must preserve M63 world-atomic rollback"
        );
    }
}
