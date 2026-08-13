use micro_company::{MICRO_COMPANY_PACK_ID, MICRO_COMPANY_PACK_VERSION, RUN_CYCLE_COMMAND};
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{self, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use world_host::WorldRegistry;
use world_pack_catalog::PackCatalog;
use world_persistence::ArchivedValue;

const MIND_ENV: &str = "WORLD_MACHINE_MICRO_COMPANY_MIND";
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
printf '%s\n' '{"type":"text_delta","delta":"WORLD_ACTION:company_agent.build"}'
printf '%s\n' '{"type":"response","command":"prompt","success":true}'
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

static TEMP_DIR_NONCE: AtomicU64 = AtomicU64::new(1);

fn temp_dir() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let nonce = TEMP_DIR_NONCE.fetch_add(1, Ordering::Relaxed);
    let root = env::temp_dir().join(format!(
        "world-machine-micro-company-external-{}-{timestamp}-{nonce}",
        process::id()
    ));
    fs::create_dir(&root).unwrap();
    root
}

fn inspector_row<'a>(
    snapshot: &'a world_projection::ProjectionSnapshot,
    title: &str,
    label: &str,
) -> Option<&'a str> {
    snapshot
        .inspectors
        .values()
        .find(|inspector| inspector.title == title)
        .and_then(|inspector| {
            inspector
                .sections
                .iter()
                .flat_map(|section| &section.rows)
                .find(|row| row.label == label)
        })
        .map(|row| row.value.as_str())
}

#[test]
fn micro_company_is_a_real_external_pack_with_distinct_deterministic_and_pi_futures() {
    let binary = env!("CARGO_BIN_EXE_micro-company-pack");
    let root = temp_dir();
    let bundle_path = root.join("micro-company.worldpack");
    let status = Command::new(binary)
        .arg("--write-bundle")
        .arg(&bundle_path)
        .status()
        .unwrap();
    assert!(status.success());

    let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
    let preview = catalog.inspect_install(&bundle_path).unwrap();
    assert_eq!(preview.pack().id, MICRO_COMPANY_PACK_ID);
    assert_eq!(preview.pack().version, MICRO_COMPANY_PACK_VERSION);
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

    let mut deterministic = registry.create(MICRO_COMPANY_PACK_ID).unwrap();
    let initial = deterministic.snapshot();
    assert_eq!(initial.title, "Northstar Micro Company");
    assert!(initial.command(RUN_CYCLE_COMMAND).is_some());
    let traction = deterministic.advance_background(2).unwrap();
    assert_eq!(traction.world_time, 20);
    assert_eq!(traction.title, "Northstar Micro Company · Traction");
    assert_eq!(
        inspector_row(&traction, "Northstar Micro Company", "Cash"),
        Some("6")
    );
    assert_eq!(inspector_row(&traction, "Northstar", "Quality"), Some("3"));
    assert_eq!(
        inspector_row(&traction, "First Customers", "Customers"),
        Some("3")
    );
    assert_eq!(inspector_row(&traction, "Maya ↔ Jon", "Trust"), Some("2"));
    assert_eq!(inspector_row(&traction, "Maya ↔ Jon", "Tension"), Some("0"));
    assert!(traction.command(RUN_CYCLE_COMMAND).is_none());

    let deterministic_archive = deterministic.archive().unwrap().unwrap();
    assert!(deterministic_archive
        .events
        .iter()
        .any(|event| event.kind == "company_found_traction"));
    assert!(deterministic_archive.events.iter().any(|event| {
        (event.kind == "agent_built_product" || event.kind == "agent_sold_product")
            && event.payload.get("mind_profile")
                == Some(&ArchivedValue::Text("deterministic".into()))
    }));
    drop(deterministic);
    let reopened = registry.open_archive(&deterministic_archive).unwrap();
    let reopened_snapshot = reopened.snapshot();
    assert_eq!(
        reopened_snapshot.title,
        "Northstar Micro Company · Traction"
    );
    assert_eq!(reopened_snapshot.world_time, 20);
    assert_eq!(
        reopened_snapshot
            .briefing
            .as_ref()
            .expect("current-state briefing")
            .title,
        "Traction found",
        "return briefing is session-local and must not become persistence truth"
    );
    assert_eq!(
        inspector_row(&reopened_snapshot, "Northstar Micro Company", "Cash"),
        Some("6")
    );
    assert_eq!(
        inspector_row(&reopened_snapshot, "Northstar Micro Company", "Status"),
        Some("traction")
    );
    assert_eq!(
        inspector_row(&reopened_snapshot, "Northstar", "Quality"),
        Some("3")
    );
    assert_eq!(
        inspector_row(&reopened_snapshot, "First Customers", "Customers"),
        Some("3")
    );
    assert_eq!(
        inspector_row(&reopened_snapshot, "Maya ↔ Jon", "Trust"),
        Some("2")
    );
    assert_eq!(
        inspector_row(&reopened_snapshot, "Maya ↔ Jon", "Tension"),
        Some("0")
    );
    assert!(reopened_snapshot.command(RUN_CYCLE_COMMAND).is_none());
    assert!(reopened_snapshot
        .timeline
        .items
        .iter()
        .any(|item| item.title == "Company Found Traction"));
    assert_eq!(reopened.archive().unwrap().unwrap(), deterministic_archive);
    drop(reopened);

    #[cfg(unix)]
    {
        let fake_pi = root.join("fake-pi.sh");
        write_fake_pi(&fake_pi);
        let _mind = EnvGuard::set(MIND_ENV, "pi");
        let pi_program = EnvGuard::set(PI_PROGRAM_ENV, &fake_pi);

        let mut shutdown = registry.create(MICRO_COMPANY_PACK_ID).unwrap();
        shutdown.advance_background(2).unwrap();
        let shutdown_archive = shutdown.archive().unwrap().unwrap();
        assert_eq!(
            shutdown_archive
                .events
                .iter()
                .filter(|event| event.kind == "agent_built_product")
                .count(),
            4
        );
        assert!(!shutdown_archive
            .events
            .iter()
            .any(|event| event.kind == "agent_sold_product"));
        assert!(shutdown_archive.events.iter().any(|event| {
            event.kind == "agent_built_product"
                && event.payload.get("mind_profile") == Some(&ArchivedValue::Text("pi".into()))
        }));
        assert!(shutdown_archive
            .events
            .iter()
            .any(|event| event.kind == "company_ran_out_of_cash"));
        drop(shutdown);

        let mut searching = registry.create(MICRO_COMPANY_PACK_ID).unwrap();
        searching.advance_background(1).unwrap();
        let searching_archive = searching.archive().unwrap().unwrap();
        drop(searching);

        drop(pi_program);
        let missing_pi = root.join("missing-pi");
        let _missing_program = EnvGuard::set(PI_PROGRAM_ENV, &missing_pi);

        let reopened_shutdown = registry.open_archive(&shutdown_archive).unwrap();
        let shutdown_snapshot = reopened_shutdown.snapshot();
        assert_eq!(
            inspector_row(&shutdown_snapshot, "Northstar Micro Company", "Cash"),
            Some("0")
        );
        assert_eq!(
            inspector_row(&shutdown_snapshot, "Northstar Micro Company", "Status"),
            Some("out-of-cash")
        );
        assert_eq!(
            inspector_row(&shutdown_snapshot, "Northstar", "Quality"),
            Some("5")
        );
        assert_eq!(
            inspector_row(&shutdown_snapshot, "First Customers", "Customers"),
            Some("1")
        );
        assert_eq!(
            inspector_row(&shutdown_snapshot, "Maya ↔ Jon", "Tension"),
            Some("4")
        );
        drop(reopened_shutdown);

        let mut reopened_searching = registry.open_archive(&searching_archive).unwrap();
        assert_eq!(
            reopened_searching.archive().unwrap().unwrap(),
            searching_archive,
            "Open must restore recorded company truth without invoking Pi"
        );
        let error = reopened_searching
            .advance_background(1)
            .expect_err("missing Pi should fail only when a new decision is required");
        assert!(error.to_string().to_lowercase().contains("pi"));
        assert_eq!(
            reopened_searching.archive().unwrap().unwrap(),
            searching_archive,
            "failed provider call must not commit a partial company cycle"
        );
    }

    drop(registry);
    drop(catalog);
    let _ = fs::remove_dir_all(root);
}
