from pathlib import Path

p = Path('crates/world-pack-process/src/lib.rs')
s = p.read_text()
old = '''        let archive = created.archive()?.ok_or_else(|| {
            HostError::session(format!(
                "external Pack {}@{} does not provide a durable archive",
                self.descriptor.pack.id, self.descriptor.pack.version
            ))
        })?;
        drop(created);

        let reopened = registry.open_archive(&archive)?;
        let reopened_snapshot = reopened.snapshot();
        Ok(ProcessPackProbe {
'''
new = '''        let archive = created.archive()?.ok_or_else(|| {
            HostError::session(format!(
                "external Pack {}@{} does not provide a durable archive",
                self.descriptor.pack.id, self.descriptor.pack.version
            ))
        })?;
        if archive.world_time != created_snapshot.world_time {
            return Err(HostError::session(format!(
                "external Pack {}@{} archived World time {} after Create snapshot reported {}",
                self.descriptor.pack.id,
                self.descriptor.pack.version,
                archive.world_time,
                created_snapshot.world_time
            )));
        }
        drop(created);

        let reopened = registry.open_archive(&archive)?;
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
if old not in s:
    raise SystemExit('probe archive block not found')
s = s.replace(old, new, 1)

marker = '''    #[cfg(unix)]
    #[test]
    fn durable_probe_rejects_packs_without_archives() {
'''
extra = '''    #[cfg(unix)]
    #[test]
    fn durable_probe_rejects_archive_state_drift() {
        let root = temp_dir("durable-probe-state-drift");
        let runtime = root.join("runtime.sh");
        let archive = WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: descriptor().pack.clone(),
            world_time: 4,
            events: Vec::new(),
            pending: Vec::new(),
        };
        write_fixture_process(
            &runtime,
            &[
                response_line(
                    1,
                    PackResponse::Descriptor {
                        descriptor: descriptor(),
                    },
                ),
                response_line(
                    2,
                    PackResponse::Snapshot {
                        snapshot: wire_snapshot(3, "Created for probe"),
                    },
                ),
                response_line(
                    3,
                    PackResponse::Archive {
                        archive: Some(archive),
                    },
                ),
            ],
        );
        let manifest = PackManifest::process(descriptor(), "runtime.sh", Vec::new());
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();
        let pack = ProcessPack::load(manifest_path).unwrap();
        let pin = pack.current_pin().unwrap();

        let error = pack.with_pin(pin).probe_durable().unwrap_err();
        assert!(error.to_string().contains("archived World time 4"));
        assert!(error.to_string().contains("reported 3"));
    }

'''
if marker not in s:
    raise SystemExit('probe no archive test marker not found')
s = s.replace(marker, extra + marker, 1)
p.write_text(s)
