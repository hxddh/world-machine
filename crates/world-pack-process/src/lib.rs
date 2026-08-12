use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;
use world_host::{HostError, WorldDescriptor, WorldPackSource, WorldRegistration, WorldSession};
use world_pack_protocol::{
    decode_response, encode_request, PackDescriptor, PackManifest, PackRequest,
    PackRequestEnvelope, PackResponse, PackRuntimeManifest, ProjectionIntentWire,
};
use world_persistence::{WorldArchive, WorldPackRef};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub const PACK_MANIFEST_SUFFIX: &str = ".world-pack.json";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessPackPin {
    manifest_sha256: String,
    command_sha256: String,
}

impl ProcessPackPin {
    pub fn new(manifest_sha256: impl Into<String>, command_sha256: impl Into<String>) -> Self {
        Self {
            manifest_sha256: manifest_sha256.into(),
            command_sha256: command_sha256.into(),
        }
    }
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }
    pub fn command_sha256(&self) -> &str {
        &self.command_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessPack {
    pub manifest_path: PathBuf,
    pub descriptor: PackDescriptor,
    pub command: PathBuf,
    pub args: Vec<String>,
    pin: Option<ProcessPackPin>,
}

impl ProcessPack {
    pub fn load(manifest_path: impl AsRef<Path>) -> Result<Self, HostError> {
        let requested_manifest_path = manifest_path.as_ref();
        let manifest_path = requested_manifest_path.canonicalize().map_err(|error| {
            HostError::pack_source(format!(
                "could not resolve Pack manifest {}: {error}",
                requested_manifest_path.display()
            ))
        })?;
        let json = fs::read_to_string(&manifest_path).map_err(|error| {
            HostError::pack_source(format!(
                "could not read {}: {error}",
                manifest_path.display()
            ))
        })?;
        let manifest = PackManifest::from_json(&json).map_err(|error| {
            HostError::pack_source(format!(
                "could not decode {}: {error}",
                manifest_path.display()
            ))
        })?;
        let PackRuntimeManifest::Process { command, args } = manifest.runtime;
        let command = resolve_command(&manifest_path, &command)?;
        Ok(Self {
            manifest_path,
            descriptor: manifest.descriptor,
            command,
            args,
            pin: None,
        })
    }

    pub fn current_pin(&self) -> Result<ProcessPackPin, HostError> {
        Ok(ProcessPackPin::new(
            sha256_file(&self.manifest_path)?,
            sha256_file(&self.command)?,
        ))
    }

    pub fn with_pin(mut self, pin: ProcessPackPin) -> Self {
        self.pin = Some(pin);
        self
    }

    pub fn pin(&self) -> Option<&ProcessPackPin> {
        self.pin.as_ref()
    }

    pub fn verify_pin(&self) -> Result<(), HostError> {
        let Some(expected) = self.pin.as_ref() else {
            return Ok(());
        };
        let current = self.current_pin()?;
        if current != *expected {
            return Err(HostError::session(format!(
                "external Pack content pin mismatch for {}@{}: expected manifest sha256 {} and executable sha256 {}, found manifest sha256 {} and executable sha256 {}",
                self.descriptor.pack.id, self.descriptor.pack.version,
                expected.manifest_sha256(), expected.command_sha256(),
                current.manifest_sha256(), current.command_sha256(),
            )));
        }
        Ok(())
    }

    fn registration(&self) -> WorldRegistration {
        let descriptor = WorldDescriptor {
            pack: self.descriptor.pack.clone(),
            title: self.descriptor.title.clone(),
            description: self.descriptor.description.clone(),
        };
        let create_pack = self.clone();
        let open_pack = self.clone();
        WorldRegistration::new(descriptor, move || {
            ProcessWorldSession::create(create_pack.clone())
                .map(|session| Box::new(session) as Box<dyn WorldSession>)
        })
        .with_archive_opener(move |archive| {
            ProcessWorldSession::open(open_pack.clone(), archive.clone())
                .map(|session| Box::new(session) as Box<dyn WorldSession>)
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProcessPackSource {
    packs: Vec<ProcessPack>,
}

impl ProcessPackSource {
    pub fn from_packs(packs: Vec<ProcessPack>) -> Self {
        Self { packs }
    }

    pub fn from_manifest_paths(
        paths: impl IntoIterator<Item = PathBuf>,
    ) -> Result<Self, HostError> {
        let mut packs = Vec::new();
        for path in paths {
            packs.push(ProcessPack::load(path)?);
        }
        Ok(Self { packs })
    }

    /// Discover direct child manifests only. Discovery never launches Pack code;
    /// processes are spawned only when a registered World session is created/opened.
    pub fn discover(directory: impl AsRef<Path>) -> Result<Self, HostError> {
        let directory = directory.as_ref();
        let entries = fs::read_dir(directory).map_err(|error| {
            HostError::pack_source(format!(
                "could not scan Pack directory {}: {error}",
                directory.display()
            ))
        })?;
        let mut manifests = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| {
                HostError::pack_source(format!(
                    "could not read Pack directory entry in {}: {error}",
                    directory.display()
                ))
            })?;
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(PACK_MANIFEST_SUFFIX))
            {
                manifests.push(path);
            }
        }
        manifests.sort();
        Self::from_manifest_paths(manifests)
    }

    pub fn packs(&self) -> &[ProcessPack] {
        &self.packs
    }
}

impl WorldPackSource for ProcessPackSource {
    fn registrations(&self) -> Result<Vec<WorldRegistration>, HostError> {
        Ok(self.packs.iter().map(ProcessPack::registration).collect())
    }
}

fn sha256_file(path: &Path) -> Result<String, HostError> {
    let mut file = File::open(path).map_err(|error| {
        HostError::pack_source(format!(
            "could not open {} for sha256: {error}",
            path.display()
        ))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            HostError::pack_source(format!(
                "could not read {} for sha256: {error}",
                path.display()
            ))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn resolve_command(manifest_path: &Path, command: &str) -> Result<PathBuf, HostError> {
    let command = PathBuf::from(command);
    let resolved = if command.is_absolute() {
        command
    } else {
        manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(command)
    };
    let resolved = resolved.canonicalize().map_err(|error| {
        HostError::pack_source(format!(
            "could not resolve Pack process command {}: {error}",
            resolved.display()
        ))
    })?;
    if !resolved.is_file() {
        return Err(HostError::pack_source(format!(
            "Pack process command is not a file: {}",
            resolved.display()
        )));
    }
    Ok(resolved)
}

pub struct ProcessWorldSession {
    pack: WorldPackRef,
    client: RefCell<ProcessClient>,
    snapshot: ProjectionSnapshot,
}

impl ProcessWorldSession {
    fn create(pack: ProcessPack) -> Result<Self, HostError> {
        Self::start(pack, None)
    }

    fn open(pack: ProcessPack, archive: WorldArchive) -> Result<Self, HostError> {
        if archive.pack != pack.descriptor.pack {
            return Err(HostError::session(format!(
                "external Pack {}@{} cannot open archive {}@{}",
                pack.descriptor.pack.id,
                pack.descriptor.pack.version,
                archive.pack.id,
                archive.pack.version
            )));
        }
        Self::start(pack, Some(archive))
    }

    fn start(pack: ProcessPack, archive: Option<WorldArchive>) -> Result<Self, HostError> {
        pack.verify_pin()?;
        let mut client = ProcessClient::spawn(&pack)?;
        let described = match client.request(PackRequest::Describe)? {
            PackResponse::Descriptor { descriptor } => descriptor,
            response => return Err(unexpected_response("describe", &response)),
        };
        if described != pack.descriptor {
            return Err(HostError::session(format!(
                "external Pack descriptor mismatch: manifest is {}@{}, process described {}@{}",
                pack.descriptor.pack.id,
                pack.descriptor.pack.version,
                described.pack.id,
                described.pack.version
            )));
        }

        let response = match archive {
            Some(archive) => client.request(PackRequest::Open { archive })?,
            None => client.request(PackRequest::Create)?,
        };
        let snapshot = snapshot_response("create/open", response)?;
        Ok(Self {
            pack: pack.descriptor.pack,
            client: RefCell::new(client),
            snapshot,
        })
    }

    fn request_snapshot(
        &self,
        request: PackRequest,
        operation: &str,
    ) -> Result<ProjectionSnapshot, HostError> {
        let response = self.client.borrow_mut().request(request)?;
        snapshot_response(operation, response)
    }
}

impl WorldSession for ProcessWorldSession {
    fn pack(&self) -> WorldPackRef {
        self.pack.clone()
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        self.snapshot.clone()
    }

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
        let snapshot = self.request_snapshot(
            PackRequest::Handle {
                intent: ProjectionIntentWire::from(intent),
            },
            "handle",
        )?;
        self.snapshot = snapshot.clone();
        Ok(snapshot)
    }

    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
        let snapshot = self.request_snapshot(PackRequest::Advance { periods }, "advance")?;
        self.snapshot = snapshot.clone();
        Ok(snapshot)
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        let response = self.client.borrow_mut().request(PackRequest::Archive)?;
        let archive = match response {
            PackResponse::Archive { archive } => archive,
            response => return Err(unexpected_response("archive", &response)),
        };
        if let Some(archive) = archive.as_ref() {
            if archive.pack != self.pack {
                return Err(HostError::session(format!(
                    "external Pack archive changed identity: session is {}@{}, archive is {}@{}",
                    self.pack.id, self.pack.version, archive.pack.id, archive.pack.version
                )));
            }
        }
        Ok(archive)
    }
}

fn snapshot_response(
    operation: &str,
    response: PackResponse,
) -> Result<ProjectionSnapshot, HostError> {
    match response {
        PackResponse::Snapshot { snapshot } => {
            ProjectionSnapshot::try_from(snapshot).map_err(|error| {
                HostError::session(format!(
                    "external Pack {operation} snapshot is invalid: {error}"
                ))
            })
        }
        response => Err(unexpected_response(operation, &response)),
    }
}

fn unexpected_response(operation: &str, response: &PackResponse) -> HostError {
    HostError::session(format!(
        "external Pack returned unexpected response to {operation}: {}",
        response_kind(response)
    ))
}

fn response_kind(response: &PackResponse) -> &'static str {
    match response {
        PackResponse::Descriptor { .. } => "descriptor",
        PackResponse::Snapshot { .. } => "snapshot",
        PackResponse::Archive { .. } => "archive",
        PackResponse::Ok => "ok",
        PackResponse::Error { .. } => "error",
    }
}

struct ProcessClient {
    child: Child,
    stdin: Option<BufWriter<ChildStdin>>,
    responses: Receiver<io::Result<String>>,
    next_request_id: u64,
    request_timeout: Duration,
}

impl ProcessClient {
    fn spawn(pack: &ProcessPack) -> Result<Self, HostError> {
        let mut child = Command::new(&pack.command)
            .args(&pack.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| {
                HostError::session(format!(
                    "could not launch external Pack {}: {error}",
                    pack.command.display()
                ))
            })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| HostError::session("external Pack stdin was not piped"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| HostError::session("external Pack stdout was not piped"))?;
        Ok(Self {
            child,
            stdin: Some(BufWriter::new(stdin)),
            responses: spawn_response_reader(stdout, DEFAULT_MAX_RESPONSE_BYTES),
            next_request_id: 1,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
        })
    }

    fn request(&mut self, request: PackRequest) -> Result<PackResponse, HostError> {
        let request_id = self.next_request_id;
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .ok_or_else(|| HostError::session("external Pack request id overflow"))?;
        let envelope = PackRequestEnvelope::new(request_id, request);
        let encoded = encode_request(&envelope).map_err(|error| {
            HostError::session(format!("could not encode Pack request: {error}"))
        })?;
        let send_result = self
            .stdin
            .as_mut()
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::BrokenPipe, "external Pack stdin is closed")
            })
            .and_then(|stdin| {
                stdin
                    .write_all(encoded.as_bytes())
                    .and_then(|_| stdin.write_all(b"\n"))
                    .and_then(|_| stdin.flush())
            });
        if let Err(error) = send_result {
            self.terminate();
            return Err(HostError::session(format!(
                "could not send Pack request: {error}"
            )));
        }

        let line = match self.responses.recv_timeout(self.request_timeout) {
            Ok(Ok(line)) => line,
            Ok(Err(error)) => {
                self.terminate();
                return Err(HostError::session(format!(
                    "could not read Pack response: {error}"
                )));
            }
            Err(RecvTimeoutError::Timeout) => {
                let timeout_ms = self.request_timeout.as_millis();
                self.terminate();
                return Err(HostError::session(format!(
                    "external Pack timed out after {timeout_ms} ms"
                )));
            }
            Err(RecvTimeoutError::Disconnected) => {
                self.terminate();
                return Err(HostError::session(
                    "external Pack response reader disconnected",
                ));
            }
        };
        if line.is_empty() {
            let status = self.child.try_wait().ok().flatten();
            self.terminate();
            return Err(HostError::session(match status {
                Some(status) => format!("external Pack exited before responding: {status}"),
                None => "external Pack closed stdout before responding".into(),
            }));
        }
        let response = match decode_response(line.trim_end()) {
            Ok(response) => response,
            Err(error) => {
                self.terminate();
                return Err(HostError::session(format!(
                    "could not decode Pack response: {error}"
                )));
            }
        };
        if response.request_id != request_id {
            let actual = response.request_id;
            self.terminate();
            return Err(HostError::session(format!(
                "external Pack response id mismatch: expected {request_id}, got {actual}"
            )));
        }
        match response.response {
            PackResponse::Error { message } => Err(HostError::session(format!(
                "external Pack rejected request: {message}"
            ))),
            response => Ok(response),
        }
    }

    fn send_shutdown(&mut self) {
        let request_id = self.next_request_id;
        let envelope = PackRequestEnvelope::new(request_id, PackRequest::Shutdown);
        if let (Ok(encoded), Some(stdin)) = (encode_request(&envelope), self.stdin.as_mut()) {
            let _ = stdin.write_all(encoded.as_bytes());
            let _ = stdin.write_all(b"\n");
            let _ = stdin.flush();
        }
        self.stdin.take();
    }

    fn terminate(&mut self) {
        self.stdin.take();
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            _ => {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }
}

impl Drop for ProcessClient {
    fn drop(&mut self) {
        self.send_shutdown();
        for _ in 0..5 {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        self.terminate();
    }
}

fn spawn_response_reader(
    stdout: ChildStdout,
    max_response_bytes: usize,
) -> Receiver<io::Result<String>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        loop {
            let line = read_bounded_line(&mut stdout, max_response_bytes);
            let finished = match &line {
                Ok(line) => line.is_empty(),
                Err(_) => true,
            };
            if sender.send(line).is_err() || finished {
                break;
            }
        }
    });
    receiver
}

fn read_bounded_line(reader: &mut impl BufRead, max_bytes: usize) -> io::Result<String> {
    let mut bytes = Vec::new();
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            break;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.map_or(buffer.len(), |index| index + 1);
        if bytes.len().saturating_add(take) > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Pack response exceeds {max_bytes} bytes"),
            ));
        }
        bytes.extend_from_slice(&buffer[..take]);
        reader.consume(take);
        if newline.is_some() {
            break;
        }
    }
    String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_host::WorldRegistry;
    use world_pack_protocol::{
        encode_response, PackResponseEnvelope, ProjectionCapabilitiesWire, ProjectionSnapshotWire,
    };
    use world_persistence::{WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "world-pack-process-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn descriptor() -> PackDescriptor {
        PackDescriptor::new(
            WorldPackRef::new("fixture.external", "1"),
            "External Fixture",
            "A process-backed fixture World",
        )
    }

    fn wire_snapshot(world_time: u64, title: &str) -> ProjectionSnapshotWire {
        ProjectionSnapshotWire {
            title: title.into(),
            world_time,
            capabilities: ProjectionCapabilitiesWire { fork: false },
            ..ProjectionSnapshotWire::default()
        }
    }

    fn response_line(request_id: u64, response: PackResponse) -> String {
        encode_response(&PackResponseEnvelope::new(request_id, response)).unwrap()
    }

    #[cfg(unix)]
    fn write_fixture_process(path: &Path, responses: &[String]) {
        use std::os::unix::fs::PermissionsExt;

        let mut script = String::from("#!/bin/sh\n");
        for response in responses {
            script.push_str("IFS= read -r _line || exit 1\n");
            script.push_str("printf '%s\\n' ");
            script.push_str(&shell_quote(response));
            script.push('\n');
        }
        script.push_str("IFS= read -r _shutdown || true\n");
        fs::write(path, script).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    fn shell_quote(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\\''"))
    }

    #[test]
    fn bounded_line_reader_rejects_oversized_protocol_messages() {
        let mut reader = BufReader::new("123456\n".as_bytes());
        let error = read_bounded_line(&mut reader, 4).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn source_discovery_is_sorted_and_does_not_recurse() {
        let root = temp_dir("discover");
        let command = root.join("runtime");
        fs::write(&command, "fixture").unwrap();
        let nested = root.join("nested");
        fs::create_dir_all(&nested).unwrap();

        for (name, id) in [
            ("b.world-pack.json", "pack.b"),
            ("a.world-pack.json", "pack.a"),
        ] {
            let manifest = PackManifest::process(
                PackDescriptor::new(WorldPackRef::new(id, "1"), id, "fixture"),
                "runtime",
                Vec::new(),
            );
            fs::write(root.join(name), manifest.to_json_pretty().unwrap()).unwrap();
        }
        let nested_manifest = PackManifest::process(
            PackDescriptor::new(WorldPackRef::new("pack.nested", "1"), "Nested", "fixture"),
            "../runtime",
            Vec::new(),
        );
        fs::write(
            nested.join("nested.world-pack.json"),
            nested_manifest.to_json_pretty().unwrap(),
        )
        .unwrap();

        let source = ProcessPackSource::discover(&root).unwrap();
        let ids = source
            .packs()
            .iter()
            .map(|pack| pack.descriptor.pack.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["pack.a", "pack.b"]);
    }

    #[cfg(unix)]
    #[test]
    fn external_process_runs_as_a_normal_world_session() {
        let root = temp_dir("session");
        let runtime = root.join("runtime.sh");
        let archive = WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: descriptor().pack.clone(),
            world_time: 7,
            events: Vec::new(),
            pending: Vec::new(),
        };
        let responses = vec![
            response_line(
                1,
                PackResponse::Descriptor {
                    descriptor: descriptor(),
                },
            ),
            response_line(
                2,
                PackResponse::Snapshot {
                    snapshot: wire_snapshot(0, "Created externally"),
                },
            ),
            response_line(
                3,
                PackResponse::Snapshot {
                    snapshot: wire_snapshot(1, "Handled externally"),
                },
            ),
            response_line(
                4,
                PackResponse::Snapshot {
                    snapshot: wire_snapshot(7, "Advanced externally"),
                },
            ),
            response_line(
                5,
                PackResponse::Archive {
                    archive: Some(archive.clone()),
                },
            ),
        ];
        write_fixture_process(&runtime, &responses);
        let manifest = PackManifest::process(descriptor(), "runtime.sh", Vec::new());
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();

        let source = ProcessPackSource::from_manifest_paths([manifest_path]).unwrap();
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();
        let mut session = registry.create("fixture.external").unwrap();

        assert_eq!(session.snapshot().title, "Created externally");
        assert_eq!(
            session
                .handle(ProjectionIntent::InvokeCommand("fixture.act".into()))
                .unwrap()
                .title,
            "Handled externally"
        );
        assert_eq!(
            session.advance_background(6).unwrap().title,
            "Advanced externally"
        );
        assert_eq!(session.archive().unwrap(), Some(archive.clone()));
        drop(session);

        let reopened = registry.open_archive(&archive).unwrap();
        assert_eq!(reopened.pack(), descriptor().pack);
        assert_eq!(reopened.snapshot().title, "Created externally");
    }

    #[cfg(unix)]
    #[test]
    fn hung_process_is_timed_out_and_terminated() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_dir("timeout");
        let runtime = root.join("runtime.sh");
        fs::write(
            &runtime,
            "#!/bin/sh\nIFS= read -r _line || exit 1\nsleep 2\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&runtime).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&runtime, permissions).unwrap();

        let manifest = PackManifest::process(descriptor(), "runtime.sh", Vec::new());
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();
        let pack = ProcessPack::load(manifest_path).unwrap();
        let mut client = ProcessClient::spawn(&pack).unwrap();
        client.request_timeout = Duration::from_millis(50);

        let error = client.request(PackRequest::Describe).err().unwrap();
        assert!(error.to_string().contains("timed out"));
        assert!(client.child.try_wait().unwrap().is_some());
    }

    #[cfg(unix)]
    #[test]
    fn process_descriptor_must_match_the_manifest() {
        let root = temp_dir("descriptor-mismatch");
        let runtime = root.join("runtime.sh");
        let wrong = PackDescriptor::new(
            WorldPackRef::new("fixture.other", "1"),
            "Wrong Pack",
            "fixture",
        );
        write_fixture_process(
            &runtime,
            &[response_line(
                1,
                PackResponse::Descriptor { descriptor: wrong },
            )],
        );
        let manifest = PackManifest::process(descriptor(), "runtime.sh", Vec::new());
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();

        let source = ProcessPackSource::from_manifest_paths([manifest_path]).unwrap();
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();
        let error = registry.create("fixture.external").err().unwrap();
        assert!(error.to_string().contains("descriptor mismatch"));
    }
}
