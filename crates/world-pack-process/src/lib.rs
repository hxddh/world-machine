mod deadline_stdin;

use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};
use world_host::{
    HostError, WorldDescriptor, WorldPackSource, WorldRegistration, WorldRegistry, WorldSession,
};
use world_pack_protocol::{
    decode_response, encode_request, PackDescriptor, PackManifest, PackRequest,
    PackRequestEnvelope, PackResponse, PackRuntimeManifest, ProjectionIntentWire,
};
use world_persistence::{WorldArchive, WorldPackRef};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

pub const PACK_MANIFEST_SUFFIX: &str = ".world-pack.json";
pub const DEFAULT_MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

const RESPONSE_QUEUE_CAPACITY: usize = 1;
static LAUNCH_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessPackProbe {
    pub pack: WorldPackRef,
    pub created_title: String,
    pub created_world_time: u64,
    pub reopened_title: String,
    pub reopened_world_time: u64,
}

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
    pub protocol_version: u32,
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
        let protocol_version = manifest.protocol_version;
        let PackRuntimeManifest::Process { command, args } = manifest.runtime;
        let command = resolve_command(&manifest_path, &command)?;
        Ok(Self {
            manifest_path,
            descriptor: manifest.descriptor,
            protocol_version,
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

    /// Launch the already-approved Pack and prove the minimum durable World contract:
    /// exact Describe handshake, Create/Snapshot, Archive, then a fresh-process Open/Snapshot.
    /// No business command is invoked and World time is never advanced by the probe itself.
    pub fn probe_durable(&self) -> Result<ProcessPackProbe, HostError> {
        self.verify_pin()?;
        let source = ProcessPackSource::from_packs(vec![self.clone()]);
        let mut registry = WorldRegistry::new();
        registry.install_source(&source)?;

        let created = registry.create_exact(&self.descriptor.pack)?;
        let created_snapshot = created.snapshot();
        let archive = created.archive()?.ok_or_else(|| {
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
            pack: self.descriptor.pack.clone(),
            created_title: created_snapshot.title,
            created_world_time: created_snapshot.world_time,
            reopened_title: reopened_snapshot.title,
            reopened_world_time: reopened_snapshot.world_time,
        })
    }

    fn prepare_launch_program(&self) -> Result<(PathBuf, Option<PathBuf>), HostError> {
        let Some(expected) = self.pin.as_ref() else {
            return Ok((self.command.clone(), None));
        };
        if !self.args.is_empty() {
            return Err(HostError::session(format!(
                "pinned external Pack {}@{} cannot use runtime arguments; package the approved program as the direct command",
                self.descriptor.pack.id, self.descriptor.pack.version
            )));
        }

        let manifest_sha256 = sha256_file(&self.manifest_path)?;
        if manifest_sha256 != expected.manifest_sha256() {
            return Err(content_pin_mismatch(
                self,
                expected,
                &manifest_sha256,
                "not-read",
            ));
        }

        let (bytes, command_sha256, permissions) = read_command_image(&self.command)?;
        if command_sha256 != expected.command_sha256() {
            return Err(content_pin_mismatch(
                self,
                expected,
                &manifest_sha256,
                &command_sha256,
            ));
        }
        let launch_path = write_launch_image(&self.command, &bytes, permissions)?;
        Ok((launch_path.clone(), Some(launch_path)))
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

fn content_pin_mismatch(
    pack: &ProcessPack,
    expected: &ProcessPackPin,
    manifest_sha256: &str,
    command_sha256: &str,
) -> HostError {
    HostError::session(format!(
        "external Pack content pin mismatch for {}@{}: expected manifest sha256 {} and executable sha256 {}, found manifest sha256 {} and executable sha256 {}",
        pack.descriptor.pack.id,
        pack.descriptor.pack.version,
        expected.manifest_sha256(),
        expected.command_sha256(),
        manifest_sha256,
        command_sha256,
    ))
}

fn read_command_image(path: &Path) -> Result<(Vec<u8>, String, fs::Permissions), HostError> {
    let mut file = File::open(path).map_err(|error| {
        HostError::pack_source(format!(
            "could not open {} for approved launch: {error}",
            path.display()
        ))
    })?;
    let permissions = file
        .metadata()
        .map_err(|error| {
            HostError::pack_source(format!(
                "could not stat {} for approved launch: {error}",
                path.display()
            ))
        })?
        .permissions();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|error| {
        HostError::pack_source(format!(
            "could not read {} for approved launch: {error}",
            path.display()
        ))
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok((bytes, format!("{:x}", hasher.finalize()), permissions))
}

fn write_launch_image(
    source: &Path,
    bytes: &[u8],
    permissions: fs::Permissions,
) -> Result<PathBuf, HostError> {
    let nonce = LAUNCH_NONCE.fetch_add(1, Ordering::Relaxed);
    let extension = source
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{extension}"))
        .unwrap_or_default();
    let path = env::temp_dir().join(format!(
        "world-machine-pack-launch-{}-{nonce}{extension}",
        process::id()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|error| {
            HostError::session(format!(
                "could not create approved Pack launch image {}: {error}",
                path.display()
            ))
        })?;
    if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(&path);
        return Err(HostError::session(format!(
            "could not write approved Pack launch image {}: {error}",
            path.display()
        )));
    }
    drop(file);
    if let Err(error) = fs::set_permissions(&path, permissions) {
        let _ = fs::remove_file(&path);
        return Err(HostError::session(format!(
            "could not set approved Pack launch permissions {}: {error}",
            path.display()
        )));
    }
    Ok(path)
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

fn prepare_request_frame(
    protocol_version: u32,
    request_id: u64,
    request: PackRequest,
    max_request_bytes: usize,
) -> Result<Vec<u8>, HostError> {
    if max_request_bytes == 0 || max_request_bytes > DEFAULT_MAX_REQUEST_BYTES {
        return Err(HostError::session(format!(
            "external Pack max request bytes must be between 1 and the {DEFAULT_MAX_REQUEST_BYTES}-byte production ceiling"
        )));
    }
    let envelope = PackRequestEnvelope::for_version(protocol_version, request_id, request)
        .map_err(|error| HostError::session(format!("invalid Pack protocol version: {error}")))?;
    let encoded = encode_request(&envelope)
        .map_err(|error| HostError::session(format!("could not encode Pack request: {error}")))?;
    let frame_bytes = encoded
        .len()
        .checked_add(1)
        .ok_or_else(|| HostError::session("external Pack request frame length overflow"))?;
    if frame_bytes > max_request_bytes {
        return Err(HostError::session(format!(
            "external Pack request frame exceeds the {max_request_bytes}-byte protocol limit"
        )));
    }
    let mut frame = encoded.into_bytes();
    frame.push(b'\n');
    Ok(frame)
}

struct ProcessClient {
    child: Child,
    stdin: Option<ChildStdin>,
    responses: Receiver<io::Result<String>>,
    protocol_version: u32,
    next_request_id: u64,
    request_timeout: Duration,
    max_request_bytes: usize,
    launch_cleanup: Option<PathBuf>,
}

impl ProcessClient {
    fn spawn(pack: &ProcessPack) -> Result<Self, HostError> {
        let (program, launch_cleanup) = pack.prepare_launch_program()?;
        let mut command = Command::new(&program);
        if pack.pin.is_none() {
            command.args(&pack.args);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let child = retry_executable_busy(|| command.spawn()).map_err(|error| {
            HostError::session(format!(
                "could not launch external Pack {}: {error}",
                program.display()
            ))
        });
        let mut child = match child {
            Ok(child) => child,
            Err(error) => {
                if let Some(path) = launch_cleanup.as_ref() {
                    let _ = fs::remove_file(path);
                }
                return Err(error);
            }
        };
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| HostError::session("external Pack stdin was not piped"))?;
        if let Err(error) = deadline_stdin::configure(&stdin) {
            let _ = child.kill();
            let _ = child.wait();
            if let Some(path) = launch_cleanup.as_ref() {
                let _ = fs::remove_file(path);
            }
            return Err(HostError::session(format!(
                "could not configure external Pack stdin: {error}"
            )));
        }
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| HostError::session("external Pack stdout was not piped"))?;
        Ok(Self {
            child,
            stdin: Some(stdin),
            responses: spawn_response_reader(stdout, DEFAULT_MAX_RESPONSE_BYTES),
            protocol_version: pack.protocol_version,
            next_request_id: 1,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            max_request_bytes: DEFAULT_MAX_REQUEST_BYTES,
            launch_cleanup,
        })
    }

    fn request(&mut self, request: PackRequest) -> Result<PackResponse, HostError> {
        let request_id = self.next_request_id;
        let frame = prepare_request_frame(
            self.protocol_version,
            request_id,
            request,
            self.max_request_bytes,
        )?;
        let next_request_id = request_id
            .checked_add(1)
            .ok_or_else(|| HostError::session("external Pack request id overflow"))?;
        let request_timeout = self.request_timeout;
        let deadline = Instant::now() + request_timeout;
        self.next_request_id = next_request_id;

        let send_result = self
            .stdin
            .as_mut()
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::BrokenPipe, "external Pack stdin is closed")
            })
            .and_then(|stdin| deadline_stdin::write_all_until(stdin, &frame, deadline));
        if let Err(error) = send_result {
            self.terminate();
            if error.kind() == io::ErrorKind::TimedOut {
                return Err(request_timeout_error(request_timeout));
            }
            return Err(HostError::session(format!(
                "could not send Pack request: {error}"
            )));
        }

        let response_timeout = match deadline_stdin::remaining(deadline) {
            Ok(remaining) => remaining,
            Err(_) => {
                self.terminate();
                return Err(request_timeout_error(request_timeout));
            }
        };
        let line = match self.responses.recv_timeout(response_timeout) {
            Ok(Ok(line)) => line,
            Ok(Err(error)) => {
                self.terminate();
                return Err(HostError::session(format!(
                    "could not read Pack response: {error}"
                )));
            }
            Err(RecvTimeoutError::Timeout) => {
                self.terminate();
                return Err(request_timeout_error(request_timeout));
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
        if response.protocol_version != self.protocol_version {
            let actual = response.protocol_version;
            let expected = self.protocol_version;
            self.terminate();
            return Err(HostError::session(format!(
                "external Pack response protocol version mismatch: expected {expected}, got {actual}"
            )));
        }
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
        let Ok(frame) = prepare_request_frame(
            self.protocol_version,
            request_id,
            PackRequest::Shutdown,
            self.max_request_bytes,
        ) else {
            self.stdin.take();
            return;
        };
        if let Some(stdin) = self.stdin.as_mut() {
            let deadline = Instant::now() + self.request_timeout;
            let _ = deadline_stdin::write_all_until(stdin, &frame, deadline);
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
        self.cleanup_launch_image();
    }

    fn cleanup_launch_image(&mut self) {
        if let Some(path) = self.launch_cleanup.take() {
            let _ = fs::remove_file(path);
        }
    }
}

impl Drop for ProcessClient {
    fn drop(&mut self) {
        self.send_shutdown();
        for _ in 0..5 {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                self.cleanup_launch_image();
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        self.terminate();
    }
}

fn request_timeout_error(timeout: Duration) -> HostError {
    let timeout_ms = timeout.as_millis();
    HostError::session(format!("external Pack timed out after {timeout_ms} ms"))
}

const EXECUTABLE_BUSY_RETRIES: usize = 3;

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

fn spawn_response_reader(
    stdout: ChildStdout,
    max_response_bytes: usize,
) -> Receiver<io::Result<String>> {
    let (sender, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
    thread::spawn(move || {
        run_response_reader(BufReader::new(stdout), sender, max_response_bytes);
    });
    receiver
}

fn run_response_reader<R: BufRead>(
    mut reader: R,
    sender: mpsc::SyncSender<io::Result<String>>,
    max_response_bytes: usize,
) {
    loop {
        let line = read_bounded_line(&mut reader, max_response_bytes);
        let finished = match &line {
            Ok(line) => line.is_empty(),
            Err(_) => true,
        };
        if sender.send(line).is_err() || finished {
            break;
        }
    }
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
    use std::collections::VecDeque;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_host::WorldRegistry;
    use world_pack_protocol::{
        encode_response, PackResponseEnvelope, ProjectionCapabilitiesWire, ProjectionSnapshotWire,
    };
    use world_persistence::{ArchivedEvent, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};

    struct ObservedRead {
        chunks: VecDeque<Vec<u8>>,
        reads: mpsc::Sender<usize>,
        read_count: usize,
    }

    impl ObservedRead {
        fn new(chunks: &[&[u8]], reads: mpsc::Sender<usize>) -> Self {
            Self {
                chunks: chunks.iter().map(|chunk| chunk.to_vec()).collect(),
                reads,
                read_count: 0,
            }
        }
    }

    impl Read for ObservedRead {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let Some(chunk) = self.chunks.pop_front() else {
                return Ok(0);
            };
            assert!(chunk.len() <= buffer.len());
            buffer[..chunk.len()].copy_from_slice(&chunk);
            self.read_count += 1;
            let _ = self.reads.send(self.read_count);
            Ok(chunk.len())
        }
    }

    fn spawn_observed_response_reader(
        chunks: &[&[u8]],
        max_response_bytes: usize,
    ) -> (
        Receiver<io::Result<String>>,
        Receiver<usize>,
        thread::JoinHandle<()>,
    ) {
        let (read_sender, read_receiver) = mpsc::channel();
        let reader = BufReader::new(ObservedRead::new(chunks, read_sender));
        let (sender, receiver) = mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY);
        let handle = thread::spawn(move || {
            run_response_reader(reader, sender, max_response_bytes);
        });
        (receiver, read_receiver, handle)
    }

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
    fn response_reader_applies_backpressure_before_consuming_a_third_record() {
        let (responses, reads, handle) =
            spawn_observed_response_reader(&[b"one\n", b"two\n", b"three\n"], 64);

        assert_eq!(reads.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
        assert_eq!(reads.recv_timeout(Duration::from_secs(1)).unwrap(), 2);
        assert!(matches!(
            reads.recv_timeout(Duration::from_millis(50)),
            Err(RecvTimeoutError::Timeout)
        ));

        assert_eq!(responses.recv().unwrap().unwrap(), "one\n");
        assert_eq!(reads.recv_timeout(Duration::from_secs(1)).unwrap(), 3);
        assert_eq!(responses.recv().unwrap().unwrap(), "two\n");
        assert_eq!(responses.recv().unwrap().unwrap(), "three\n");
        assert_eq!(responses.recv().unwrap().unwrap(), "");
        handle.join().unwrap();
    }

    #[test]
    fn response_reader_exits_when_receiver_is_dropped_while_send_is_blocked() {
        let (responses, reads, handle) =
            spawn_observed_response_reader(&[b"one\n", b"two\n", b"three\n"], 64);

        assert_eq!(reads.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
        assert_eq!(reads.recv_timeout(Duration::from_secs(1)).unwrap(), 2);
        drop(responses);

        handle.join().unwrap();
        assert!(matches!(
            reads.recv_timeout(Duration::from_millis(50)),
            Err(RecvTimeoutError::Disconnected)
        ));
    }

    #[test]
    fn response_reader_preserves_eof_and_terminal_read_errors() {
        let (responses, _reads, handle) = spawn_observed_response_reader(&[b"one\n"], 64);
        assert_eq!(responses.recv().unwrap().unwrap(), "one\n");
        assert_eq!(responses.recv().unwrap().unwrap(), "");
        handle.join().unwrap();

        let (responses, _reads, handle) = spawn_observed_response_reader(&[b"123456\n"], 4);
        let error = responses.recv().unwrap().unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        handle.join().unwrap();
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

        let pack = ProcessPack::load(manifest_path).unwrap();
        let pin = pack.current_pin().unwrap();
        let source = ProcessPackSource::from_packs(vec![pack.with_pin(pin)]);
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
    fn durable_probe_creates_archives_and_reopens_in_a_fresh_process() {
        let root = temp_dir("durable-probe");
        let runtime = root.join("runtime.sh");
        let archive = WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: descriptor().pack.clone(),
            world_time: 3,
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

        let probe = pack.with_pin(pin).probe_durable().unwrap();
        assert_eq!(probe.pack, descriptor().pack);
        assert_eq!(probe.created_title, "Created for probe");
        assert_eq!(probe.created_world_time, 3);
        assert_eq!(probe.reopened_title, "Created for probe");
        assert_eq!(probe.reopened_world_time, 3);
    }

    #[cfg(unix)]
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

    #[cfg(unix)]
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
        script.push_str(&format!(
            "  printf '%s\\n' {}\n",
            shell_quote(&changed_archive)
        ));
        script.push_str("else\n");
        script.push_str(&format!(
            "  printf '%s\\n' {}\n",
            shell_quote(&original_archive)
        ));
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

    #[cfg(unix)]
    #[test]
    fn durable_probe_rejects_packs_without_archives() {
        let root = temp_dir("durable-probe-no-archive");
        let runtime = root.join("runtime.sh");
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
                        snapshot: wire_snapshot(0, "Created without archive"),
                    },
                ),
                response_line(3, PackResponse::Archive { archive: None }),
            ],
        );
        let manifest = PackManifest::process(descriptor(), "runtime.sh", Vec::new());
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();
        let pack = ProcessPack::load(manifest_path).unwrap();
        let pin = pack.current_pin().unwrap();

        let error = pack.with_pin(pin).probe_durable().unwrap_err();
        assert!(error
            .to_string()
            .contains("does not provide a durable archive"));
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
    fn no_read_pack_request_write_is_timed_out_and_direct_child_is_reaped() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_dir("write-timeout");
        let runtime = root.join("runtime.sh");
        fs::write(&runtime, "#!/bin/sh\nsleep 2\n").unwrap();
        let mut permissions = fs::metadata(&runtime).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&runtime, permissions).unwrap();

        let manifest = PackManifest::process(descriptor(), "runtime.sh", Vec::new());
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();
        let pack = ProcessPack::load(manifest_path).unwrap();
        let mut client = ProcessClient::spawn(&pack).unwrap();
        client.request_timeout = Duration::from_millis(50);

        let archive = WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: descriptor().pack,
            world_time: 0,
            events: vec![ArchivedEvent {
                id: 1,
                kind: "x".repeat(12 * 1024 * 1024),
                world_time: 0,
                actor: None,
                targets: Vec::new(),
                caused_by: Vec::new(),
                payload: Default::default(),
                changes: Vec::new(),
            }],
            pending: Vec::new(),
        };
        let frame = prepare_request_frame(
            pack.protocol_version,
            1,
            PackRequest::Open {
                archive: archive.clone(),
            },
            DEFAULT_MAX_REQUEST_BYTES,
        )
        .unwrap();
        assert!(frame.len() > 8 * 1024 * 1024);

        let started = Instant::now();
        let error = client.request(PackRequest::Open { archive }).err().unwrap();
        assert!(error.to_string().contains("timed out after 50 ms"));
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "request write deadline did not return promptly: {:?}",
            started.elapsed()
        );
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
