from pathlib import Path

p = Path('crates/world-pack-process/src/lib.rs')
s = p.read_text()
old = '''        let reopened = registry.open_archive(&archive)?;
        let reopened_snapshot = reopened.snapshot();
        if reopened_snapshot.world_time != archive.world_time {
            return Err(HostError::session(format!(
                "external Pack {}@{} reopened archive at World time {}, expected {}",
                self.descriptor.pack.id,
                self.descriptor.pack.version,
                reopened_snapshot.world_time,
                archive.world_time
            )));
        }
        Ok(ProcessPackProbe {
'''
new = '''        let reopened = registry.open_archive(&archive)?;
        let reopened_snapshot = reopened.snapshot();
        if reopened_snapshot.world_time != archive.world_time {
            return Err(HostError::session(format!(
                "external Pack {}@{} reopened archive at World time {}, expected {}",
                self.descriptor.pack.id,
                self.descriptor.pack.version,
                reopened_snapshot.world_time,
                archive.world_time
            )));
        }
        let reopened_archive = reopened.archive()?.ok_or_else(|| {
            HostError::session(format!(
                "external Pack {}@{} stopped providing a durable archive after reopen",
                self.descriptor.pack.id, self.descriptor.pack.version
            ))
        })?;
        if reopened_archive != archive {
            return Err(HostError::session(format!(
                "external Pack {}@{} reopened archive did not round-trip durable state exactly",
                self.descriptor.pack.id, self.descriptor.pack.version
            )));
        }
        Ok(ProcessPackProbe {
'''
if old not in s:
    raise SystemExit('probe reopen block not found')
s = s.replace(old, new, 1)

old_import = '''    use world_persistence::{WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
'''
new_import = '''    use world_persistence::{ArchivedEvent, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
'''
if old_import not in s:
    raise SystemExit('test persistence import not found')
s = s.replace(old_import, new_import, 1)

marker = '''    #[test]\n    fn executable_busy_spawn_errors_are_retried_but_other_errors_are_not() {\n'''
extra = r'''    #[cfg(unix)]
    #[test]
    fn durable_probe_rejects_reopened_archive_content_drift_at_same_world_time() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_dir("durable-probe-rearchive-drift");
        let runtime = root.join("runtime.sh");
        let launch_marker = root.join("launched-once");
        let original = WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: descriptor().pack.clone(),
            world_time: 3,
            events: Vec::new(),
            pending: Vec::new(),
        };
        let mut changed = original.clone();
        changed.events.push(ArchivedEvent {
            id: 1,
            kind: "unexpected".into(),
            world_time: 3,
            actor: None,
            targets: Vec::new(),
            caused_by: Vec::new(),
            payload: Default::default(),
            changes: Vec::new(),
        });

        let describe = response_line(
            1,
            PackResponse::Descriptor {
                descriptor: descriptor(),
            },
        );
        let snapshot = response_line(
            2,
            PackResponse::Snapshot {
                snapshot: wire_snapshot(3, "Created for probe"),
            },
        );
        let original_archive = response_line(
            3,
            PackResponse::Archive {
                archive: Some(original),
            },
        );
        let changed_archive = response_line(
            3,
            PackResponse::Archive {
                archive: Some(changed),
            },
        );
        let mut script = String::from("#!/bin/sh\n");
        script.push_str(&format!(
            "if [ -e {} ]; then changed=1; else touch {}; changed=0; fi\n",
            shell_quote(launch_marker.to_str().unwrap()),
            shell_quote(launch_marker.to_str().unwrap())
        ));
        script.push_str("IFS= read -r _line || exit 1\n");
        script.push_str(&format!("printf '%s\\n' {}\n", shell_quote(&describe)));
        script.push_str("IFS= read -r _line || exit 1\n");
        script.push_str(&format!("printf '%s\\n' {}\n", shell_quote(&snapshot)));
        script.push_str("IFS= read -r _line || exit 1\n");
        script.push_str("if [ \"$changed\" = 1 ]; then\n");
        script.push_str(&format!("  printf '%s\\n' {}\n", shell_quote(&changed_archive)));
        script.push_str("else\n");
        script.push_str(&format!("  printf '%s\\n' {}\n", shell_quote(&original_archive)));
        script.push_str("fi\n");
        script.push_str("IFS= read -r _shutdown || true\n");
        fs::write(&runtime, script).unwrap();
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o755)).unwrap();

        let manifest = PackManifest::process(descriptor(), "runtime.sh", Vec::new());
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();
        let pack = ProcessPack::load(manifest_path).unwrap();
        let pin = pack.current_pin().unwrap();

        let error = pack.with_pin(pin).probe_durable().unwrap_err();
        assert!(error
            .to_string()
            .contains("did not round-trip durable state exactly"));
    }

'''
if marker not in s:
    raise SystemExit('executable busy test marker not found')
s = s.replace(marker, extra + marker, 1)
p.write_text(s)
