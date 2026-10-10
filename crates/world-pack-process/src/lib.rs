#![deny(unsafe_code)]

// The only `unsafe` in the package: the unix pipe calls that bound a
// write by a deadline (see the module).
#[allow(unsafe_code)]
mod deadline_stdin;

use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use world_host::{
    HostError, SessionCheckpoint, WorldDescriptor, WorldPackSource, WorldRegistration,
    WorldRegistry, WorldSession,
};
use world_pack_protocol::{
    decode_response, encode_open_request, encode_open_request_deflated, encode_request,
    pack_frame_limit, EarsWire, PackDescriptor, PackManifest, PackRequest, PackRequestEnvelope,
    PackResponse, PackRuntimeManifest, ProjectionIntentWire, ProtocolEncodeError, StoryPageWire,
    StoryRequestWire, PACK_FRAME_LIMIT, PACK_PROTOCOL_VERSION_V3, PACK_PROTOCOL_VERSION_V4,
    PACK_PROTOCOL_VERSION_V5, PACK_PROTOCOL_VERSION_V6, PACK_PROTOCOL_VERSION_V7,
    PACK_PROTOCOL_VERSION_V8,
};
use world_persistence::{CheckpointFit, WorldArchive, WorldPackRef};
use world_projection::{
    ProjectionIntent, ProjectionSnapshot, SelectionId, StoryPage, StoryRequest,
};

pub const PACK_MANIFEST_SUFFIX: &str = ".world-pack.json";
/// The most a request frame may hold. A Pack before v5 reads no more than
/// `PACK_FRAME_LIMIT_BEFORE_V5`, so its frames stop there.
pub const DEFAULT_MAX_REQUEST_BYTES: usize = PACK_FRAME_LIMIT;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = PACK_FRAME_LIMIT;
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
/// How many times a request's timeout `open` and `archive` may take: they
/// carry the World's whole history, which takes longer to read and write the
/// longer it is kept (a minute, by default).
const HISTORY_REQUEST_TIMEOUT_FACTOR: u32 = 12;

const RESPONSE_QUEUE_CAPACITY: usize = 1;
static LAUNCH_NONCE: AtomicU64 = AtomicU64::new(1);

/// What tells this run's launch images and scratch folders from those of
/// an earlier run that had the same process id: when this run first made
/// one, in nanoseconds since 1970, in hex. A crashed run's image is left
/// in the temporary folder; a later run given its process id once made the
/// same name, and could not open a World ("File exists").
fn run_stamp() -> &'static str {
    static STAMP: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    STAMP.get_or_init(|| {
        let since = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{:x}", since.as_nanos())
    })
}

/// The start of every launch image's name this run makes.
fn own_launch_prefix() -> String {
    format!(
        "world-machine-pack-launch-{}-{}-",
        process::id(),
        run_stamp()
    )
}

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
    settings: Vec<(String, String)>,
    crash_log_dir: Option<PathBuf>,
}

/// The prefix a host setting has to carry to reach a Pack process.
///
/// A host configures a Pack; it does not reshape the Pack's environment. The
/// prefix keeps that distinction enforceable rather than conventional: `PATH`,
/// `DYLD_*`, and everything else a Pack inherits stay out of reach.
pub const PACK_SETTING_PREFIX: &str = "WORLD_MACHINE_";

/// Whether an environment variable's name says it holds a secret: an API
/// key, a token, a password. A Pack is never given one, neither as a
/// setting nor from the host's own environment; a model is asked by the
/// app itself, never by a Pack with the player's key.
pub fn is_secret_name(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    [
        "API_KEY",
        "APIKEY",
        "SECRET",
        "TOKEN",
        "PASSWORD",
        "CREDENTIAL",
    ]
    .iter()
    .any(|word| name.contains(word))
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
            settings: Vec::new(),
            crash_log_dir: None,
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

    /// Settings the host hands this Pack's process when it is launched.
    ///
    /// Every name must carry [`PACK_SETTING_PREFIX`]; anything else is refused
    /// rather than quietly dropped, because a setting that silently fails to
    /// arrive is worse than one that never existed. Values reach the Pack as
    /// process environment, which is what an external Pack can read without a
    /// protocol change.
    pub fn with_settings<I, K, V>(mut self, settings: I) -> Result<Self, HostError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.settings = settings
            .into_iter()
            .map(|(name, value)| {
                let name = name.into();
                if !name.starts_with(PACK_SETTING_PREFIX) {
                    return Err(HostError::pack_source(format!(
                        "a Pack setting has to be named {PACK_SETTING_PREFIX}…, not {name}"
                    )));
                }
                if is_secret_name(&name) {
                    return Err(HostError::pack_source(format!(
                        "a Pack is never given a secret, such as {name}"
                    )));
                }
                Ok((name, value.into()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self)
    }

    pub fn settings(&self) -> &[(String, String)] {
        &self.settings
    }

    /// Where this Pack's process leaves a crash log when it exits with a
    /// failure on its own: what it wrote to standard error last (up to
    /// [`CRASH_LOG_TAIL_BYTES`]), its exit status and the time, in a file
    /// named after the Pack. Without one, a crash is only in the host's own
    /// standard error, where the Pack's is passed on as before.
    pub fn with_crash_log_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.crash_log_dir = Some(dir.into());
        self
    }

    pub fn crash_log_dir(&self) -> Option<&Path> {
        self.crash_log_dir.as_deref()
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
        let deflated_pack = self.clone();
        WorldRegistration::new(descriptor, move || {
            ProcessWorldSession::create(create_pack.clone())
                .map(|session| Box::new(session) as Box<dyn WorldSession>)
        })
        .with_archive_opener(move |archive| {
            ProcessWorldSession::open(open_pack.clone(), archive, None)
                .map(|session| Box::new(session) as Box<dyn WorldSession>)
        })
        .with_deflated_archive_opener(move |archive, deflated| {
            ProcessWorldSession::open(deflated_pack.clone(), archive, Some(deflated))
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
    Ok((bytes, lower_hex(&hasher.finalize()), permissions))
}

fn write_launch_image(
    source: &Path,
    bytes: &[u8],
    permissions: fs::Permissions,
) -> Result<PathBuf, HostError> {
    static SWEPT: std::sync::Once = std::sync::Once::new();
    SWEPT.call_once(|| sweep_stale_launch_images(&env::temp_dir()));
    let (path, mut file) = create_launch_image(&env::temp_dir(), source)?;
    // Not synced to disk: the image is run from here at once and never
    // needed again, and syncing costs every launch (on a Mac, a full flush
    // of the disk's cache).
    if let Err(error) = file.write_all(bytes) {
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

/// A new launch image in `temp_dir`, named for this run (see
/// [`run_stamp`]), opened for writing. A name that is somehow taken
/// already is never written over, nor a reason to fail: the next is tried.
fn create_launch_image(temp_dir: &Path, source: &Path) -> Result<(PathBuf, File), HostError> {
    let extension = source
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{extension}"))
        .unwrap_or_default();
    let prefix = own_launch_prefix();
    let mut tries = 0;
    loop {
        let nonce = LAUNCH_NONCE.fetch_add(1, Ordering::Relaxed);
        let path = temp_dir.join(format!("{prefix}{nonce}{extension}"));
        match OpenOptions::new().create_new(true).write(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && tries < 64 => {
                tries += 1;
            }
            Err(error) => {
                return Err(HostError::session(format!(
                    "could not create approved Pack launch image {}: {error}",
                    path.display()
                )))
            }
        }
    }
}

/// How old an earlier run's launch image must be before it is swept.
const STALE_LAUNCH_IMAGE: std::time::Duration = std::time::Duration::from_secs(12 * 60 * 60);

/// Removes launch images an earlier run left behind: a process that ends
/// without dropping its Pack (a crash, a forced quit, killed) leaves its
/// copy of the Pack's program in the temporary folder. An image is removed
/// when the run that made it is over, however new it is: its process is
/// gone, or the process with its id is this one, which did not make it.
/// Any other run's image is removed once untouched for half a day (where
/// whether a process is still running cannot be told, everywhere but
/// Linux and macOS, only that). `temp_dir` is the folder launch images are
/// written to (`env::temp_dir()` in the app; a scratch folder in tests).
fn sweep_stale_launch_images(temp_dir: &Path) {
    const LAUNCH: &str = "world-machine-pack-launch-";
    let own = own_launch_prefix();
    let Ok(entries) = fs::read_dir(temp_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(rest) = name.strip_prefix(LAUNCH) else {
            continue;
        };
        if name.starts_with(&own) {
            continue;
        }
        let Some(metadata) = entry.metadata().ok().filter(|metadata| metadata.is_file()) else {
            continue;
        };
        let owner = rest
            .split(['-', '.'])
            .next()
            .and_then(|pid| pid.parse::<u32>().ok());
        let over = owner.is_some_and(|pid| pid == process::id() || process_is_gone(pid));
        let stale = over
            || metadata
                .modified()
                .ok()
                .and_then(|modified| modified.elapsed().ok())
                .is_some_and(|age| age >= STALE_LAUNCH_IMAGE);
        if stale {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Whether no process with id `pid` is running, where that can be told
/// without `unsafe`: on Linux from `/proc`, on macOS by asking `kill -0`.
/// Elsewhere (and when it cannot be told) `false`: the image is left to
/// age.
fn process_is_gone(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        Path::new("/proc/self").exists() && !Path::new(&format!("/proc/{pid}")).exists()
    }
    #[cfg(target_os = "macos")]
    {
        // `kill -0` fails for a process that is gone, and also for one that
        // belongs to someone else; an image another user's run left in this
        // user's temporary folder cannot be, so only "No such process" counts.
        Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .is_ok_and(|output| {
                !output.status.success()
                    && String::from_utf8_lossy(&output.stderr).contains("No such process")
            })
    }
    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
    {
        false
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
    Ok(lower_hex(&hasher.finalize()))
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
    /// What the running Pack said it can do (v8); nothing for an older one.
    capabilities: Vec<String>,
}

impl ProcessWorldSession {
    fn create(pack: ProcessPack) -> Result<Self, HostError> {
        Self::start(pack, None)
    }

    /// Opens `archive` in a new Pack process; with `deflated`, its compact
    /// JSON as the World file it was read from keeps it, which a Pack on v5
    /// is handed as it is.
    fn open(
        pack: ProcessPack,
        archive: &WorldArchive,
        deflated: Option<&[u8]>,
    ) -> Result<Self, HostError> {
        let mut without_checkpoint = None;
        if archive.checkpoint.is_some() && pack.protocol_version < PACK_PROTOCOL_VERSION_V4 {
            // A Pack before v4 cannot restore from a checkpoint. It can
            // replay a whole history, only more slowly; a history that keeps
            // only what came after the checkpoint (a World code's) it
            // cannot open at all.
            let whole = archive
                .checkpoint
                .as_ref()
                .is_some_and(|checkpoint| checkpoint.fit(archive) == Some(CheckpointFit::Within));
            if !whole {
                return Err(HostError::session(format!(
                    "this World needs a newer {} Pack (protocol v{PACK_PROTOCOL_VERSION_V4}); the installed one speaks v{}",
                    pack.descriptor.title, pack.protocol_version
                )));
            }
            let mut whole = archive.clone();
            whole.checkpoint = None;
            without_checkpoint = Some(whole);
        }
        let archive = without_checkpoint.as_ref().unwrap_or(archive);
        if archive.pack != pack.descriptor.pack {
            return Err(HostError::session(format!(
                "external Pack {}@{} cannot open archive {}@{}",
                pack.descriptor.pack.id,
                pack.descriptor.pack.version,
                archive.pack.id,
                archive.pack.version
            )));
        }
        let deflated = deflated.filter(|_| {
            without_checkpoint.is_none() && pack.protocol_version >= PACK_PROTOCOL_VERSION_V5
        });
        Self::start(pack, Some((archive, deflated)))
    }

    fn start(
        pack: ProcessPack,
        archive: Option<(&WorldArchive, Option<&[u8]>)>,
    ) -> Result<Self, HostError> {
        let mut client = ProcessClient::spawn(&pack)?;
        let described = match client.request(PackRequest::Describe)? {
            PackResponse::Descriptor { descriptor } => descriptor,
            response => return Err(unexpected_response("describe", &response)),
        };
        // What the running Pack says it can do is what counts; a manifest
        // written before capabilities were said has none.
        if !described.same_pack(&pack.descriptor) {
            return Err(HostError::session(format!(
                "external Pack descriptor mismatch: manifest is {}@{}, process described {}@{}",
                pack.descriptor.pack.id,
                pack.descriptor.pack.version,
                described.pack.id,
                described.pack.version
            )));
        }

        let response = match archive {
            Some((archive, deflated)) => client.open(archive, deflated)?,
            None => client.request(PackRequest::Create)?,
        };
        let snapshot = snapshot_response("create/open", response)?;
        Ok(Self {
            pack: pack.descriptor.pack,
            client: RefCell::new(client),
            snapshot,
            capabilities: described.capabilities,
        })
    }

    /// Whether the running Pack speaks v8 and says it can do `capability`.
    fn can(&self, capability: &str) -> bool {
        self.client.borrow().protocol_version >= PACK_PROTOCOL_VERSION_V8
            && self.capabilities.iter().any(|known| known == capability)
    }

    /// Whether the Pack speaks a protocol with `hear` and `ears`.
    fn speaks_v3(&self) -> bool {
        self.client.borrow().protocol_version >= PACK_PROTOCOL_VERSION_V3
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
        // A design or a name goes typed only to a Pack that says it takes
        // them; any other hears it as the command it offered, with the
        // argument after `=`, as every Pack before v8 did.
        let typed = match &intent {
            ProjectionIntent::Design { .. } => self.can(world_projection::capability::DESIGNS),
            ProjectionIntent::Name { .. } => self.can(world_projection::capability::NAMES),
            _ => true,
        };
        let intent = if typed { intent } else { intent.as_command() };
        let mut intent = ProjectionIntentWire::from(intent);
        // A Pack on an older protocol hears everything in its own way, and
        // one that never said it reads an offered reply hears its words.
        if let ProjectionIntentWire::Say { ears, .. } = &mut intent {
            if !self.speaks_v3()
                || (*ears == EarsWire::Offered
                    && !self.can(world_projection::capability::OFFERED_REPLIES))
            {
                *ears = EarsWire::World;
            }
        }
        let snapshot = self.request_snapshot(PackRequest::Handle { intent }, "handle")?;
        self.snapshot = snapshot.clone();
        Ok(snapshot)
    }

    fn hearing(&self, to: SelectionId, words: &str) -> Result<Option<String>, HostError> {
        // Asking a Pack that never said it could answer would end it.
        if !self.speaks_v3() {
            return Ok(None);
        }
        let response = self.client.borrow_mut().request(PackRequest::Hear {
            to: to.into(),
            words: words.to_string(),
        })?;
        match response {
            PackResponse::Hearing { prompt, .. } => Ok(prompt),
            response => Err(unexpected_response("hear", &response)),
        }
    }

    fn voice_hearing(
        &self,
        to: SelectionId,
        words: &str,
    ) -> Result<Option<world_projection::VoiceHearing>, HostError> {
        if !self.speaks_v3() {
            return Ok(None);
        }
        let response = self.client.borrow_mut().request(PackRequest::Hear {
            to: to.into(),
            words: words.to_string(),
        })?;
        match response {
            PackResponse::Hearing { hearing, .. } => Ok(hearing.map(Into::into)),
            response => Err(unexpected_response("hear", &response)),
        }
    }

    fn story(&self, request: StoryRequest) -> Result<Option<StoryPage>, HostError> {
        // A Pack from before stories has none to tell.
        if !self.speaks_v7() {
            return Ok(None);
        }
        let response = self.client.borrow_mut().request(PackRequest::Story {
            request: StoryRequestWire::from(&request),
        })?;
        match response {
            PackResponse::Story { page } => Ok(page.and_then(StoryPageWire::into_page)),
            response => Err(unexpected_response("story", &response)),
        }
    }

    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
        let snapshot = self.request_snapshot(PackRequest::Advance { periods }, "advance")?;
        self.snapshot = snapshot.clone();
        Ok(snapshot)
    }

    fn checkpoint(&mut self) -> Result<Option<SessionCheckpoint>, HostError> {
        if !self.speaks_v6() {
            return Ok(None);
        }
        match self.client.borrow_mut().request(PackRequest::Checkpoint)? {
            PackResponse::Checkpointed { kept } => Ok(kept.then(|| {
                SessionCheckpoint::new(ProcessMark {
                    snapshot: self.snapshot.clone(),
                })
            })),
            response => Err(unexpected_response("checkpoint", &response)),
        }
    }

    fn rollback(&mut self, checkpoint: SessionCheckpoint) -> Result<(), HostError> {
        let mark = checkpoint.into_inner::<ProcessMark>()?;
        match self.client.borrow_mut().request(PackRequest::Rollback)? {
            PackResponse::Ok => {
                self.snapshot = mark.snapshot;
                Ok(())
            }
            response => Err(unexpected_response("rollback", &response)),
        }
    }

    fn archive_since(&self, from: usize) -> Result<Option<WorldArchive>, HostError> {
        if !self.speaks_v6() {
            return Ok(self.archive()?.map(|mut archive| {
                archive.events.drain(..from.min(archive.events.len()));
                archive
            }));
        }
        let response = self
            .client
            .borrow_mut()
            .request(PackRequest::ArchiveSince { events: from })?;
        self.archive_from(response)
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        let response = self.client.borrow_mut().request(PackRequest::Archive)?;
        self.archive_from(response)
    }
}

/// What the host keeps of a Pack process's World at a checkpoint: the Pack
/// keeps the rest.
struct ProcessMark {
    snapshot: ProjectionSnapshot,
}

impl ProcessWorldSession {
    /// Whether the Pack speaks a protocol with `checkpoint`, `rollback` and
    /// `archive_since`.
    fn speaks_v6(&self) -> bool {
        self.client.borrow().protocol_version >= PACK_PROTOCOL_VERSION_V6
    }

    fn speaks_v7(&self) -> bool {
        self.client.borrow().protocol_version >= PACK_PROTOCOL_VERSION_V7
    }

    fn archive_from(&self, response: PackResponse) -> Result<Option<WorldArchive>, HostError> {
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
        PackResponse::Hearing { .. } => "hearing",
        PackResponse::Checkpointed { .. } => "checkpointed",
        PackResponse::Story { .. } => "story",
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
    check_max_request_bytes(max_request_bytes)?;
    let opens = matches!(request, PackRequest::Open { .. });
    let envelope = PackRequestEnvelope::for_version(protocol_version, request_id, request)
        .map_err(|error| HostError::session(format!("invalid Pack protocol version: {error}")))?;
    let encoded = encode_request(&envelope)
        .map_err(|error| HostError::session(format!("could not encode Pack request: {error}")))?;
    finish_frame(protocol_version, encoded, opens, max_request_bytes)
}

/// The frame of an `open` request for an archive the host keeps, as
/// [`prepare_request_frame`] makes it for `PackRequest::Open`.
fn prepare_open_frame(
    protocol_version: u32,
    request_id: u64,
    archive: &WorldArchive,
    deflated: Option<&[u8]>,
    max_request_bytes: usize,
) -> Result<Vec<u8>, HostError> {
    check_max_request_bytes(max_request_bytes)?;
    let encoded = match deflated {
        Some(deflated) => {
            encode_open_request_deflated(protocol_version, request_id, archive, deflated)
        }
        None => encode_open_request(protocol_version, request_id, archive),
    }
    .map_err(|error| match error {
        ProtocolEncodeError::Protocol(error) => {
            HostError::session(format!("invalid Pack protocol version: {error}"))
        }
        ProtocolEncodeError::Json(error) => {
            HostError::session(format!("could not encode Pack request: {error}"))
        }
    })?;
    finish_frame(protocol_version, encoded, true, max_request_bytes)
}

fn check_max_request_bytes(max_request_bytes: usize) -> Result<(), HostError> {
    if max_request_bytes == 0 || max_request_bytes > DEFAULT_MAX_REQUEST_BYTES {
        return Err(HostError::session(format!(
            "external Pack max request bytes must be between 1 and the {DEFAULT_MAX_REQUEST_BYTES}-byte production ceiling"
        )));
    }
    Ok(())
}

fn finish_frame(
    protocol_version: u32,
    encoded: String,
    opens: bool,
    max_request_bytes: usize,
) -> Result<Vec<u8>, HostError> {
    // A Pack reads no more than its protocol lets it.
    let max_request_bytes = max_request_bytes.min(pack_frame_limit(protocol_version));
    let frame_bytes = encoded
        .len()
        .checked_add(1)
        .ok_or_else(|| HostError::session("external Pack request frame length overflow"))?;
    if frame_bytes > max_request_bytes {
        if opens && protocol_version < PACK_PROTOCOL_VERSION_V5 {
            return Err(HostError::session(format!(
                "this World's history is too long for a Pack on protocol v{protocol_version}: its request frame exceeds the {max_request_bytes}-byte protocol limit; a Pack that speaks v{PACK_PROTOCOL_VERSION_V5} opens it"
            )));
        }
        return Err(HostError::session(format!(
            "external Pack request frame exceeds the {max_request_bytes}-byte protocol limit"
        )));
    }
    let mut frame = encoded.into_bytes();
    frame.push(b'\n');
    Ok(frame)
}

/// What a Pack process inherits from the host's environment: what a program
/// needs to start and find its own files and language, by name, and nothing
/// else. Everything else (the host's own `WORLD_MACHINE_*` variables among
/// it) reaches a Pack only as a setting the host gives that Pack
/// ([`ProcessPack::with_settings`]), and a secret never does.
pub const PACK_ENVIRONMENT_ALLOWLIST: &[&str] = &[
    // Every platform.
    "PATH",
    "HOME",
    "LANG",
    "LANGUAGE",
    "LC_ALL",
    "LC_CTYPE",
    "LC_MESSAGES",
    "TZ",
    "RUST_BACKTRACE",
    // Unix: who is running it, and where a program keeps its files.
    "USER",
    "LOGNAME",
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
    "XDG_DATA_HOME",
    // Windows: what a process cannot start or find its folders without.
    "SYSTEMROOT",
    "SYSTEMDRIVE",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    "USERPROFILE",
    "USERNAME",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "HOMEDRIVE",
    "HOMEPATH",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
];

/// The temporary-folder variables, which a Pack is given pointing at its own
/// scratch folder, so what it leaves behind is removed with it.
const PACK_TEMP_VARIABLES: &[&str] = &["TMPDIR", "TMP", "TEMP"];

/// The whole environment a Pack process is started with, from the host's
/// own (`host`), the Pack's settings and its scratch folder: the allowed
/// names (compared without case, as Windows does), never a secret, then the
/// temporary-folder variables, then the settings.
pub fn pack_environment<I, K, V>(
    host: I,
    settings: &[(String, String)],
    scratch: Option<&Path>,
) -> Vec<(OsString, OsString)>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let mut environment = host
        .into_iter()
        .filter(|(name, _)| {
            let name = name.as_ref().to_string_lossy().to_ascii_uppercase();
            PACK_ENVIRONMENT_ALLOWLIST.contains(&name.as_str()) && !is_secret_name(&name)
        })
        .map(|(name, value)| (name.as_ref().to_owned(), value.as_ref().to_owned()))
        .collect::<Vec<_>>();
    if let Some(scratch) = scratch {
        for name in PACK_TEMP_VARIABLES {
            environment.push((OsString::from(name), scratch.as_os_str().to_owned()));
        }
    }
    for (name, value) in settings {
        if !is_secret_name(name) {
            environment.push((OsString::from(name), OsString::from(value)));
        }
    }
    environment
}

/// A Pack process's own working folder, made empty for it under the
/// temporary folder and removed with everything in it when the process is
/// done: a Pack does not work in (or write into) the host's folder.
fn make_scratch_dir() -> io::Result<PathBuf> {
    let nonce = LAUNCH_NONCE.fetch_add(1, Ordering::Relaxed);
    let path = env::temp_dir().join(format!(
        "world-machine-pack-scratch-{}-{}-{nonce}",
        process::id(),
        run_stamp()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

/// The most of a Pack's standard error a crash log keeps: the end of it.
pub const CRASH_LOG_TAIL_BYTES: usize = 64 * 1024;

/// A Pack's standard error, passed on to the host's as it arrives, with its
/// last [`CRASH_LOG_TAIL_BYTES`] kept for a crash log.
struct StderrTail {
    tail: Arc<Mutex<Vec<u8>>>,
    finished: Receiver<()>,
}

impl StderrTail {
    fn spawn(mut stderr: ChildStderr) -> Self {
        let tail = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&tail);
        let (done, finished) = mpsc::channel();
        thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            loop {
                match stderr.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => {
                        let _ = io::stderr().write_all(&buffer[..read]);
                        if let Ok(mut kept) = kept.lock() {
                            kept.extend_from_slice(&buffer[..read]);
                            let over = kept.len().saturating_sub(CRASH_LOG_TAIL_BYTES);
                            if over > 0 {
                                kept.drain(..over);
                            }
                        }
                    }
                }
            }
            let _ = done.send(());
        });
        Self { tail, finished }
    }

    /// What was kept, once the Pack has finished writing (or a moment has
    /// passed: a program the Pack started may still hold its standard error).
    fn take(&self) -> Vec<u8> {
        let _ = self.finished.recv_timeout(Duration::from_millis(500));
        self.tail
            .lock()
            .map(|tail| tail.clone())
            .unwrap_or_default()
    }
}

/// The file name a crash log of the Pack `id` is written to: its id with
/// anything a file name cannot hold made `_`, the time and the host's
/// process, so crashes never overwrite one another.
pub fn crash_log_file_name(id: &str, unix_seconds: u64, host_process: u32) -> String {
    let id = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("{id}-{unix_seconds}-{host_process}.log")
}

/// Writes `bytes` to a new file in `dir` named `name`, or, when a file of
/// that name is already there (a Pack that crashes twice in one second),
/// `name` with `-2`, `-3`… before its extension: an earlier crash log is
/// never overwritten.
fn write_new(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let (stem, extension) = name.rsplit_once('.').unwrap_or((name, ""));
    for attempt in 1..=1000u32 {
        let candidate = match (attempt, extension) {
            (1, _) => name.to_string(),
            (_, "") => format!("{stem}-{attempt}"),
            (_, extension) => format!("{stem}-{attempt}.{extension}"),
        };
        let path = dir.join(candidate);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                io::Write::write_all(&mut file, bytes)?;
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "every crash log name is taken",
    ))
}

struct ProcessClient {
    child: Child,
    stdin: Option<deadline_stdin::BoundedStdin>,
    responses: Receiver<io::Result<String>>,
    protocol_version: u32,
    next_request_id: u64,
    request_timeout: Duration,
    max_request_bytes: usize,
    launch_cleanup: Option<PathBuf>,
    scratch: Option<PathBuf>,
    stderr: Option<StderrTail>,
    crash_log_dir: Option<PathBuf>,
    pack: WorldPackRef,
}

impl ProcessClient {
    fn spawn(pack: &ProcessPack) -> Result<Self, HostError> {
        let (program, launch_cleanup) = pack.prepare_launch_program()?;
        let mut command = Command::new(&program);
        if pack.pin.is_none() {
            command.args(&pack.args);
        }
        // A Pack starts in a folder of its own, with only the variables it
        // needs and the settings it was given: nothing else the host was
        // started with reaches it, a secret least of all.
        let scratch = match make_scratch_dir() {
            Ok(scratch) => Some(scratch),
            Err(error) => {
                if let Some(path) = launch_cleanup.as_ref() {
                    let _ = fs::remove_file(path);
                }
                return Err(HostError::session(format!(
                    "could not make a folder for external Pack {}: {error}",
                    pack.descriptor.pack.id
                )));
            }
        };
        command.env_clear().envs(pack_environment(
            env::vars_os(),
            &pack.settings,
            scratch.as_deref(),
        ));
        if let Some(scratch) = scratch.as_ref() {
            command.current_dir(scratch);
        }
        // The app is a windowed program on Windows; without this, every
        // Pack it starts would open a console window of its own.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
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
                if let Some(scratch) = scratch.as_ref() {
                    let _ = fs::remove_dir_all(scratch);
                }
                return Err(error);
            }
        };
        let stderr = child.stderr.take().map(StderrTail::spawn);
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| HostError::session("external Pack stdin was not piped"))?;
        let stdin = match deadline_stdin::configure(stdin) {
            Ok(stdin) => stdin,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                if let Some(path) = launch_cleanup.as_ref() {
                    let _ = fs::remove_file(path);
                }
                if let Some(scratch) = scratch.as_ref() {
                    let _ = fs::remove_dir_all(scratch);
                }
                return Err(HostError::session(format!(
                    "could not configure external Pack stdin: {error}"
                )));
            }
        };
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
            scratch,
            stderr,
            crash_log_dir: pack.crash_log_dir.clone(),
            pack: pack.descriptor.pack.clone(),
        })
    }

    /// The child's exit status if it has exited or does within a moment
    /// (it may close its output just before it exits).
    fn exit_status_soon(&mut self) -> Option<ExitStatus> {
        for _ in 0..10 {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Some(status);
            }
            thread::sleep(Duration::from_millis(10));
        }
        None
    }

    /// The child exited on its own with `status`: if that is a failure,
    /// leave a crash log where the host asked for one.
    fn note_exit(&mut self, status: ExitStatus) {
        if status.success() {
            return;
        }
        let (Some(dir), Some(stderr)) = (self.crash_log_dir.take(), self.stderr.as_ref()) else {
            return;
        };
        let tail = stderr.take();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut log = format!(
            "Pack: {} {}\nExited: {status}\nAt: {now} (seconds since 1970, UTC)\n\nStandard error (the last {} bytes at most):\n",
            self.pack.id, self.pack.version, CRASH_LOG_TAIL_BYTES
        )
        .into_bytes();
        log.extend_from_slice(&tail);
        let name = crash_log_file_name(&self.pack.id, now, process::id());
        let _ = fs::create_dir_all(&dir).and_then(|_| write_new(&dir, &name, &log));
    }

    fn request(&mut self, request: PackRequest) -> Result<PackResponse, HostError> {
        let request_id = self.next_request_id;
        let carries_history = matches!(request, PackRequest::Open { .. } | PackRequest::Archive);
        let frame = prepare_request_frame(
            self.protocol_version,
            request_id,
            request,
            self.max_request_bytes,
        )?;
        self.exchange(request_id, frame, carries_history)
    }

    /// Opens `archive` in the Pack without a copy of it, handing it over
    /// as `deflated` when there is that.
    fn open(
        &mut self,
        archive: &WorldArchive,
        deflated: Option<&[u8]>,
    ) -> Result<PackResponse, HostError> {
        let request_id = self.next_request_id;
        let frame = prepare_open_frame(
            self.protocol_version,
            request_id,
            archive,
            deflated,
            self.max_request_bytes,
        )?;
        self.exchange(request_id, frame, true)
    }

    fn exchange(
        &mut self,
        request_id: u64,
        frame: Vec<u8>,
        carries_history: bool,
    ) -> Result<PackResponse, HostError> {
        let next_request_id = request_id
            .checked_add(1)
            .ok_or_else(|| HostError::session("external Pack request id overflow"))?;
        let request_timeout = if carries_history {
            self.request_timeout
                .saturating_mul(HISTORY_REQUEST_TIMEOUT_FACTOR)
        } else {
            self.request_timeout
        };
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
            let status = self.exit_status_soon();
            if let Some(status) = status {
                self.note_exit(status);
            }
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
            Ok(Some(status)) => self.note_exit(status),
            _ => {
                // Stopped by the host (a deadline passed): not a crash.
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
        if let Some(scratch) = self.scratch.take() {
            let _ = fs::remove_dir_all(scratch);
        }
    }
}

impl Drop for ProcessClient {
    fn drop(&mut self) {
        self.send_shutdown();
        for _ in 0..5 {
            if let Ok(Some(status)) = self.child.try_wait() {
                self.note_exit(status);
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

/// A digest written as lowercase hexadecimal.
fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
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

    /// A crashed run's launch image, half a day old, is swept; this
    /// process's own images, fresh ones of a run still going, folders and
    /// anything not named as a launch image are left alone.
    #[test]
    fn stale_launch_images_from_other_runs_are_swept() {
        let dir = temp_dir("sweep");
        // A run still going: where the sweep can tell (Linux, macOS), the
        // process that started this test; elsewhere any other id.
        #[cfg(unix)]
        let alive = std::os::unix::process::parent_id();
        #[cfg(not(unix))]
        let alive = 4294967295_u32;
        let other = format!("world-machine-pack-launch-{alive}");
        let own = own_launch_prefix();
        let aged = |name: &str, age: Duration| {
            let path = dir.join(name);
            let file = File::create(&path).unwrap();
            file.set_modified(SystemTime::now() - age).unwrap();
            path
        };
        let day = Duration::from_secs(24 * 60 * 60);
        let crashed = aged(&format!("{other}-0.worldpack"), day);
        let crashed_bare = aged(
            &format!("{other}-3"),
            STALE_LAUNCH_IMAGE + Duration::from_secs(60),
        );
        let running = aged(&format!("{other}-1"), Duration::from_secs(60));
        let just_under = aged(
            &format!("{other}-2"),
            STALE_LAUNCH_IMAGE - Duration::from_secs(60),
        );
        let mine = aged(&format!("{own}0"), day);
        let unrelated = aged("someone-elses-file", day);
        let folder = dir.join(format!("{other}-folder"));
        fs::create_dir(&folder).unwrap();
        // Aged too where the platform allows it; swept or not, a folder stays.
        let _ = File::open(&folder).and_then(|folder| folder.set_modified(SystemTime::now() - day));

        sweep_stale_launch_images(&dir);

        assert!(!crashed.exists(), "a day-old image from a crashed run");
        assert!(!crashed_bare.exists(), "one with no extension");
        assert!(running.exists(), "another run's image in use");
        assert!(just_under.exists(), "younger than half a day");
        assert!(mine.exists(), "this process's own image");
        assert!(unrelated.exists(), "not a launch image");
        assert!(folder.is_dir(), "folders are never swept");
        sweep_stale_launch_images(&dir.join("missing"));
        fs::remove_dir_all(&dir).unwrap();
    }

    /// An app that was killed leaves its launch image; the next run sweeps
    /// it however new it is, once the run that made it is over: its
    /// process gone, or its process id now this process's own (v0.29
    /// could not open a World then: "File exists").
    #[test]
    fn a_killed_runs_launch_image_is_swept_at_once() {
        let dir = temp_dir("killed");
        let mut child = Command::new(env::current_exe().unwrap())
            .arg("--list")
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let gone = child.id();
        child.wait().unwrap();
        let killed = dir.join(format!("world-machine-pack-launch-{gone}-1f-0.worldpack"));
        // v0.29's name for this process's first image, from an earlier
        // run that had the same id.
        let same_id = dir.join(format!("world-machine-pack-launch-{}-5", process::id()));
        for path in [&killed, &same_id] {
            File::create(path).unwrap();
        }
        sweep_stale_launch_images(&dir);
        if cfg!(any(target_os = "linux", target_os = "macos")) {
            assert!(!killed.exists(), "the killed run's image, new as it is");
        }
        assert!(!same_id.exists(), "an earlier run's image under this id");
        fs::remove_dir_all(&dir).unwrap();
    }

    /// A launch image is never written over and a name already taken is no
    /// reason to fail: an earlier run's images under this process's id, in
    /// v0.29's names and in this run's own, are left as they are and a new
    /// image is made beside them.
    #[test]
    fn a_launch_image_name_already_taken_is_passed_over() {
        let dir = temp_dir("taken");
        let source = Path::new("pack.worldpack");
        let mut taken = Vec::new();
        let next = LAUNCH_NONCE.load(Ordering::Relaxed);
        for nonce in next..next + 8 {
            for name in [
                format!(
                    "world-machine-pack-launch-{}-{nonce}.worldpack",
                    process::id()
                ),
                format!("{}{nonce}.worldpack", own_launch_prefix()),
            ] {
                let path = dir.join(name);
                fs::write(&path, b"an earlier image").unwrap();
                taken.push(path);
            }
        }
        let (path, mut file) = create_launch_image(&dir, source).expect("a new image");
        file.write_all(b"this run's").unwrap();
        drop(file);
        assert!(!taken.contains(&path), "{}", path.display());
        assert_eq!(fs::read(&path).unwrap(), b"this run's");
        for path in &taken {
            assert_eq!(fs::read(path).unwrap(), b"an earlier image");
        }
        fs::remove_dir_all(&dir).unwrap();
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
            capabilities: ProjectionCapabilitiesWire {
                fork: false,
                background: false,
                talk: false,
            },
            ..ProjectionSnapshotWire::default()
        }
    }

    fn response_line(request_id: u64, response: PackResponse) -> String {
        encode_response(&PackResponseEnvelope::new(request_id, response)).unwrap()
    }

    /// The setting that names a fixture Pack's script.
    const FIXTURE_SCRIPT: &str = "WORLD_MACHINE_FIXTURE_SCRIPT";

    /// The scripted stand-in Pack (`src/bin/world-pack-fixture.rs`), which
    /// Cargo builds beside this test binary for `cargo test` of the package
    /// (a test binary's `deps/` folder sits in the folder binaries go in).
    fn fixture_program() -> PathBuf {
        let exe = env::current_exe().unwrap();
        let program = exe
            .parent()
            .and_then(Path::parent)
            .unwrap()
            .join(format!("world-pack-fixture{}", env::consts::EXE_SUFFIX));
        assert!(
            program.is_file(),
            "{} is not built: run `cargo test -p world-pack-process` (all its targets), not `--lib` alone",
            program.display()
        );
        program
    }

    /// A Pack manifest in `root` whose process is the fixture, running
    /// `steps` (see the fixture's module for the verbs); load it with
    /// [`load_fixture`] so it is told where its script is.
    fn write_fixture_pack(root: &Path, steps: &[String]) -> PathBuf {
        fs::write(root.join("fixture.script"), steps.join("\n")).unwrap();
        let program = fixture_program();
        let manifest = PackManifest::process(descriptor(), program.to_str().unwrap(), Vec::new());
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();
        manifest_path
    }

    /// Steps that answer each request with one of `responses`, in order.
    fn respond(responses: &[String]) -> Vec<String> {
        responses
            .iter()
            .map(|response| format!("respond\t{response}"))
            .collect()
    }

    fn fixture_setting(root: &Path) -> (String, String) {
        (
            FIXTURE_SCRIPT.to_string(),
            root.join("fixture.script").to_string_lossy().into_owned(),
        )
    }

    fn load_fixture(manifest_path: &Path) -> ProcessPack {
        let root = manifest_path.parent().unwrap().to_path_buf();
        ProcessPack::load(manifest_path)
            .unwrap()
            .with_settings([fixture_setting(&root)])
            .unwrap()
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

    fn describe_and_create(title: &str) -> Vec<String> {
        vec![
            response_line(
                1,
                PackResponse::Descriptor {
                    descriptor: descriptor(),
                },
            ),
            response_line(
                2,
                PackResponse::Snapshot {
                    snapshot: wire_snapshot(0, title),
                },
            ),
        ]
    }

    #[test]
    fn a_host_setting_reaches_the_pack_process() {
        // The host configures a Pack by handing it settings when it launches.
        // Proven by a Pack that writes what it was given to a file.
        let root = temp_dir("settings");
        let observed = root.join("observed");
        let secret = root.join("secret");
        // A secret in the host's own environment, as a key exported in a
        // shell would be.
        std::env::set_var("WORLD_MACHINE_TEST_SECRET_API_KEY", "sk-test");
        let mut steps = vec![
            format!("env\tWORLD_MACHINE_TEST_VOICE\t{}", observed.display()),
            format!(
                "env\tWORLD_MACHINE_TEST_SECRET_API_KEY\t{}",
                secret.display()
            ),
        ];
        steps.extend(respond(&describe_and_create("Created externally")));
        let manifest_path = write_fixture_pack(&root, &steps);

        let pack = ProcessPack::load(&manifest_path)
            .unwrap()
            .with_settings([
                fixture_setting(&root),
                ("WORLD_MACHINE_TEST_VOICE".into(), "pi".into()),
            ])
            .unwrap();
        let source = ProcessPackSource::from_packs(vec![pack]);
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();
        let session = registry.create("fixture.external").unwrap();
        assert_eq!(session.snapshot().title, "Created externally");
        assert_eq!(
            fs::read_to_string(&observed).unwrap(),
            "pi",
            "the Pack process was not given the setting the host configured"
        );
        assert_eq!(
            fs::read_to_string(&secret).unwrap(),
            "",
            "a secret in the host's environment reached the Pack"
        );
        assert!(ProcessPack::load(&manifest_path)
            .unwrap()
            .with_settings([("WORLD_MACHINE_ANY_API_KEY", "sk-test")])
            .is_err());
        assert!(is_secret_name("world_machine_voice_token"));
        assert!(!is_secret_name("WORLD_MACHINE_POCKET_UNIVERSE_VOICE"));

        drop(session);
        let _ = fs::remove_dir_all(root);
    }

    /// A Pack inherits only the variables on the allowlist: not the host's
    /// own `WORLD_MACHINE_*` variables (a voice the host was started with
    /// reaches only the Pack it is given to, as a setting), nor anything
    /// else the host happens to have.
    #[test]
    fn a_pack_inherits_only_the_allowed_variables() {
        let host = [
            ("PATH", "/usr/bin"),
            ("Path", r"C:\Windows"),
            ("HOME", "/home/someone"),
            ("SystemRoot", r"C:\Windows"),
            ("LANG", "ja_JP.UTF-8"),
            ("WORLD_MACHINE_SOME_PACK_VOICE", "pi"),
            ("WORLD_MACHINE_LIBRARY_DIR", "/somewhere"),
            ("SOME_VENDOR_API_KEY", "sk-test"),
            ("DYLD_INSERT_LIBRARIES", "/tmp/x.dylib"),
            ("LD_PRELOAD", "/tmp/x.so"),
            ("SSH_AUTH_SOCK", "/tmp/agent"),
            ("TMPDIR", "/host/tmp"),
        ];
        let scratch = Path::new("/scratch/pack");
        let settings = vec![("WORLD_MACHINE_TEST_VOICE".to_string(), "fm".to_string())];
        let environment = pack_environment(host, &settings, Some(scratch));
        let names = environment
            .iter()
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        for kept in ["PATH", "Path", "HOME", "SystemRoot", "LANG"] {
            assert!(names.iter().any(|name| name == kept), "{kept} was not kept");
        }
        for dropped in [
            "WORLD_MACHINE_SOME_PACK_VOICE",
            "WORLD_MACHINE_LIBRARY_DIR",
            "SOME_VENDOR_API_KEY",
            "DYLD_INSERT_LIBRARIES",
            "LD_PRELOAD",
            "SSH_AUTH_SOCK",
        ] {
            assert!(
                !names.iter().any(|name| name == dropped),
                "{dropped} reached a Pack"
            );
        }
        // The temporary folder is the Pack's own scratch folder, not the host's.
        let temp = environment
            .iter()
            .filter(|(name, _)| name == "TMPDIR")
            .map(|(_, value)| value.clone())
            .collect::<Vec<_>>();
        assert_eq!(temp, vec![scratch.as_os_str().to_owned()]);
        // And a setting the host gave this Pack arrives.
        assert!(environment.contains(&("WORLD_MACHINE_TEST_VOICE".into(), "fm".into())));
    }

    /// A real Pack process: started in an empty folder of its own (removed
    /// when it is done), without the host's other variables.
    #[test]
    fn a_pack_process_runs_in_a_scratch_folder_with_only_the_allowed_variables() {
        let root = temp_dir("isolation");
        let cwd = root.join("cwd");
        let inherited = root.join("inherited");
        let path = root.join("path");
        std::env::set_var("WORLD_MACHINE_TEST_HOST_ONLY", "leaked");
        let mut steps = vec![
            format!("cwd\t{}", cwd.display()),
            format!("env\tWORLD_MACHINE_TEST_HOST_ONLY\t{}", inherited.display()),
            format!("env\tPATH\t{}", path.display()),
        ];
        steps.extend(respond(&describe_and_create("Isolated")));
        let manifest_path = write_fixture_pack(&root, &steps);
        let source = ProcessPackSource::from_packs(vec![load_fixture(&manifest_path)]);
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();
        let session = registry.create("fixture.external").unwrap();
        assert_eq!(session.snapshot().title, "Isolated");

        let scratch = PathBuf::from(fs::read_to_string(&cwd).unwrap());
        assert_ne!(
            scratch.canonicalize().ok(),
            env::current_dir().unwrap().canonicalize().ok(),
            "the Pack ran in the host's folder"
        );
        assert!(
            scratch
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("world-machine-pack-scratch-"),
            "{}",
            scratch.display()
        );
        assert_eq!(fs::read_to_string(&inherited).unwrap(), "");
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            env::var("PATH").unwrap_or_default(),
            "PATH is on the allowlist"
        );
        drop(session);
        drop(registry);
        assert!(!scratch.exists(), "the scratch folder outlived its Pack");
        let _ = fs::remove_dir_all(root);
    }

    /// A Pack process that exits with a failure on its own leaves a crash
    /// log: what it wrote to standard error, and how it exited.
    #[test]
    fn a_pack_that_crashes_leaves_a_crash_log_with_its_standard_error() {
        let root = temp_dir("crash");
        let logs = root.join("crash-logs");
        let steps = vec![
            "read".to_string(),
            "stderr\tthread 'main' panicked at src/main.rs:1:1: the fixture fell over".to_string(),
            "exit\t3".to_string(),
        ];
        let manifest_path = write_fixture_pack(&root, &steps);
        let pack = load_fixture(&manifest_path).with_crash_log_dir(&logs);
        let source = ProcessPackSource::from_packs(vec![pack]);
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();
        let error = registry.create("fixture.external").err().unwrap();
        assert!(error.to_string().contains("exited"), "{error}");

        let written = fs::read_dir(&logs)
            .expect("no crash log folder")
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(written.len(), 1, "{written:?}");
        let name = written[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert!(name.starts_with("fixture.external-"), "{name}");
        let log = fs::read_to_string(&written[0]).unwrap();
        assert!(log.contains("Pack: fixture.external 1"), "{log}");
        assert!(log.contains("the fixture fell over"), "{log}");
        assert!(log.contains('3'), "the exit status is missing: {log}");

        // One that exits cleanly leaves none.
        let clean = temp_dir("clean");
        let manifest_path = write_fixture_pack(&clean, &respond(&describe_and_create("Fine")));
        let pack = load_fixture(&manifest_path).with_crash_log_dir(clean.join("crash-logs"));
        let source = ProcessPackSource::from_packs(vec![pack]);
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();
        drop(registry.create("fixture.external").unwrap());
        assert!(!clean.join("crash-logs").exists());

        assert_eq!(
            crash_log_file_name("a/b:c", 7, 9),
            "a_b_c-7-9.log",
            "a Pack id cannot reach outside the crash log folder"
        );
        let logs = clean.join("same-second");
        fs::create_dir_all(&logs).unwrap();
        let first = write_new(&logs, "p-7-9.log", b"first").unwrap();
        let second = write_new(&logs, "p-7-9.log", b"second").unwrap();
        assert_eq!(second.file_name().unwrap(), "p-7-9-2.log");
        assert_eq!(fs::read(&first).unwrap(), b"first", "never overwritten");
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(clean);
    }

    #[test]
    fn a_host_cannot_reshape_a_pack_environment_under_the_guise_of_a_setting() {
        let root = temp_dir("settings-refused");
        let manifest_path = write_fixture_pack(&root, &[]);
        let pack = ProcessPack::load(manifest_path).unwrap();

        // A Pack is started with the allowed variables; the host may add
        // settings to them and nothing else. Refused loudly, because a
        // setting that silently fails to arrive is worse than one that
        // never existed.
        for name in [
            "PATH",
            "DYLD_INSERT_LIBRARIES",
            "HOME",
            "world_machine_voice",
        ] {
            let error = pack
                .clone()
                .with_settings([(name, "anything")])
                .expect_err("{name} was accepted as a Pack setting");
            assert!(
                format!("{error}").contains(PACK_SETTING_PREFIX),
                "the refusal does not say what a Pack setting has to look like: {error}"
            );
        }
        assert!(
            pack.settings().is_empty(),
            "a Pack carries no settings unless the host gives it some"
        );

        let _ = fs::remove_dir_all(root);
    }

    fn archive_at(world_time: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: descriptor().pack.clone(),
            world_time,
            events: Vec::new(),
            pending: Vec::new(),
            checkpoint: None,
        }
    }

    /// On v6 a change is tried on the Pack process that has the World open,
    /// and gone back from there: no process is started for it.
    #[test]
    fn a_v6_pack_marks_changes_and_goes_back_in_the_same_process() {
        let root = temp_dir("checkpoint");
        let archive = archive_at(7);
        let later = archive_at(8);
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
                    snapshot: wire_snapshot(7, "Opened"),
                },
            ),
            response_line(3, PackResponse::Checkpointed { kept: true }),
            response_line(
                4,
                PackResponse::Snapshot {
                    snapshot: wire_snapshot(8, "Changed"),
                },
            ),
            response_line(
                5,
                PackResponse::Archive {
                    archive: Some(later.clone()),
                },
            ),
            response_line(6, PackResponse::Ok),
        ];
        let manifest_path = write_fixture_pack(&root, &respond(&responses));
        let manifest = PackManifest::process(descriptor(), "runtime", Vec::new());
        assert!(manifest.protocol_version >= PACK_PROTOCOL_VERSION_V6);
        let source = ProcessPackSource::from_packs(vec![load_fixture(&manifest_path)]);
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();

        let mut session = registry.open_archive(&archive).unwrap();
        let mark = session
            .checkpoint()
            .unwrap()
            .expect("a v6 Pack keeps marks");
        let changed = session
            .handle(ProjectionIntent::InvokeCommand("fixture.act".into()))
            .unwrap();
        assert_eq!(changed.title, "Changed");
        assert_eq!(session.archive_since(0).unwrap(), Some(later));
        session.rollback(mark).unwrap();
        assert_eq!(session.snapshot().title, "Opened");
        drop(session);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn external_process_runs_as_a_normal_world_session() {
        let root = temp_dir("session");
        let archive = archive_at(7);
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
        let manifest_path = write_fixture_pack(&root, &respond(&responses));

        let pack = load_fixture(&manifest_path);
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
        drop(reopened);
        let _ = fs::remove_dir_all(root);
    }

    fn probe_responses(world_time: u64, title: &str, archive: Option<WorldArchive>) -> Vec<String> {
        vec![
            response_line(
                1,
                PackResponse::Descriptor {
                    descriptor: descriptor(),
                },
            ),
            response_line(
                2,
                PackResponse::Snapshot {
                    snapshot: wire_snapshot(world_time, title),
                },
            ),
            response_line(3, PackResponse::Archive { archive }),
        ]
    }

    fn pinned_fixture(root: &Path, steps: &[String]) -> ProcessPack {
        let manifest_path = write_fixture_pack(root, steps);
        let pack = load_fixture(&manifest_path);
        let pin = pack.current_pin().unwrap();
        pack.with_pin(pin)
    }

    #[test]
    fn durable_probe_creates_archives_and_reopens_in_a_fresh_process() {
        let root = temp_dir("durable-probe");
        let steps = respond(&probe_responses(
            3,
            "Created for probe",
            Some(archive_at(3)),
        ));
        let probe = pinned_fixture(&root, &steps).probe_durable().unwrap();
        assert_eq!(probe.pack, descriptor().pack);
        assert_eq!(probe.created_title, "Created for probe");
        assert_eq!(probe.created_world_time, 3);
        assert_eq!(probe.reopened_title, "Created for probe");
        assert_eq!(probe.reopened_world_time, 3);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn durable_probe_rejects_archive_state_drift() {
        let root = temp_dir("durable-probe-state-drift");
        let steps = respond(&probe_responses(
            3,
            "Created for probe",
            Some(archive_at(4)),
        ));
        let error = pinned_fixture(&root, &steps).probe_durable().unwrap_err();
        assert!(error.to_string().contains("archived World time 4"));
        assert!(error.to_string().contains("reported 3"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn durable_probe_rejects_reopened_archive_content_drift_at_same_world_time() {
        let root = temp_dir("durable-probe-rearchive-drift");
        let launch_marker = root.join("launched-once");
        let original = archive_at(3);
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
        let responses = probe_responses(3, "Created for probe", Some(original));
        let changed_archive = response_line(
            3,
            PackResponse::Archive {
                archive: Some(changed),
            },
        );
        // The second launch (the reopening) archives something else.
        let steps = vec![
            format!("marker\t{}", launch_marker.display()),
            format!("respond\t{}", responses[0]),
            format!("respond\t{}", responses[1]),
            format!("first\trespond\t{}", responses[2]),
            format!("second\trespond\t{changed_archive}"),
        ];
        let error = pinned_fixture(&root, &steps).probe_durable().unwrap_err();
        assert!(error
            .to_string()
            .contains("did not round-trip durable state exactly"));
        let _ = fs::remove_dir_all(root);
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

    #[test]
    fn durable_probe_rejects_packs_without_archives() {
        let root = temp_dir("durable-probe-no-archive");
        let steps = respond(&probe_responses(0, "Created without archive", None));
        let error = pinned_fixture(&root, &steps).probe_durable().unwrap_err();
        assert!(error
            .to_string()
            .contains("does not provide a durable archive"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hung_process_is_timed_out_and_terminated() {
        let root = temp_dir("timeout");
        let manifest_path =
            write_fixture_pack(&root, &["read".to_string(), "sleep\t2000".to_string()]);
        let pack = load_fixture(&manifest_path);
        let mut client = ProcessClient::spawn(&pack).unwrap();
        client.request_timeout = Duration::from_millis(50);

        let error = client.request(PackRequest::Describe).err().unwrap();
        assert!(error.to_string().contains("timed out"));
        assert!(client.child.try_wait().unwrap().is_some());
        drop(client);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn no_read_pack_request_write_is_timed_out_and_direct_child_is_reaped() {
        let root = temp_dir("write-timeout");
        let manifest_path = write_fixture_pack(&root, &["sleep\t2000".to_string()]);
        let pack = load_fixture(&manifest_path);
        let mut client = ProcessClient::spawn(&pack).unwrap();
        client.request_timeout = Duration::from_millis(50);

        // A request that is not an archive's goes over as written (an
        // archive's is packed, and takes longer).
        let big = || PackRequest::Handle {
            intent: ProjectionIntentWire::InvokeCommand {
                command: "x".repeat(2 * 1024 * 1024),
            },
        };
        let frame =
            prepare_request_frame(pack.protocol_version, 1, big(), DEFAULT_MAX_REQUEST_BYTES)
                .unwrap();
        assert!(frame.len() > 1024 * 1024);

        let started = Instant::now();
        let error = client.request(big()).err().unwrap();
        assert!(error.to_string().contains("timed out after 50 ms"));
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "request write deadline did not return promptly: {:?}",
            started.elapsed()
        );
        assert!(client.child.try_wait().unwrap().is_some());
        drop(client);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn process_descriptor_must_match_the_manifest() {
        let root = temp_dir("descriptor-mismatch");
        let wrong = PackDescriptor::new(
            WorldPackRef::new("fixture.other", "1"),
            "Wrong Pack",
            "fixture",
        );
        let manifest_path = write_fixture_pack(
            &root,
            &respond(&[response_line(
                1,
                PackResponse::Descriptor { descriptor: wrong },
            )]),
        );
        let source = ProcessPackSource::from_packs(vec![load_fixture(&manifest_path)]);
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();
        let error = registry.create("fixture.external").err().unwrap();
        assert!(error.to_string().contains("descriptor mismatch"));
        let _ = fs::remove_dir_all(root);
    }
}
