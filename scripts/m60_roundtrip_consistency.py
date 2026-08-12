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

# ETXTBSY/ExecutableFileBusy means spawn failed before a child exists, so a
# short bounded retry is safe and does not risk executing a Pack twice.
old_spawn = '''        let child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| {
                HostError::session(format!(
                    "could not launch external Pack {}: {error}",
                    program.display()
                ))
            });
'''
new_spawn = '''        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let child = retry_executable_busy(|| command.spawn()).map_err(|error| {
            HostError::session(format!(
                "could not launch external Pack {}: {error}",
                program.display()
            ))
        });
'''
if old_spawn not in s:
    raise SystemExit('process spawn block not found')
s = s.replace(old_spawn, new_spawn, 1)

reader_marker = '''fn spawn_response_reader(
    stdout: ChildStdout,
    max_response_bytes: usize,
) -> Receiver<io::Result<String>> {
'''
retry_helper = '''const EXECUTABLE_BUSY_RETRIES: usize = 3;

fn retry_executable_busy<T>(mut operation: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    for attempt in 0..=EXECUTABLE_BUSY_RETRIES {
        match operation() {
            Err(error)
                if error.kind() == io::ErrorKind::ExecutableFileBusy
                    && attempt < EXECUTABLE_BUSY_RETRIES =>
            {
                thread::sleep(Duration::from_millis(10 * (attempt as u64 + 1)));
            }
            result => return result,
        }
    }
    unreachable!("bounded executable-busy retry loop always returns")
}

'''
if reader_marker not in s:
    raise SystemExit('response reader marker not found')
s = s.replace(reader_marker, retry_helper + reader_marker, 1)

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

    #[test]
    fn executable_busy_spawn_errors_are_retried_but_other_errors_are_not() {
        let mut busy_attempts = 0;
        let value = retry_executable_busy(|| {
            busy_attempts += 1;
            if busy_attempts < 3 {
                Err(io::Error::from(io::ErrorKind::ExecutableFileBusy))
            } else {
                Ok(7_u8)
            }
        })
        .unwrap();
        assert_eq!(value, 7);
        assert_eq!(busy_attempts, 3);

        let mut other_attempts = 0;
        let error = retry_executable_busy(|| -> io::Result<()> {
            other_attempts += 1;
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(other_attempts, 1);
    }

'''
if marker not in s:
    raise SystemExit('probe no archive test marker not found')
s = s.replace(marker, extra + marker, 1)
p.write_text(s)
