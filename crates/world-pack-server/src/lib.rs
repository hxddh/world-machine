#![forbid(unsafe_code)]

use std::env;
use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};
use world_pack_bundle::{write_program_bundle, PackBundleHeader};
use world_pack_protocol::{
    decode_request, encode_response, pack_frame_limit, PackDescriptor, PackManifest, PackRequest,
    PackRequestEnvelope, PackResponse, PackResponseEnvelope, ProjectionSnapshotWire, StoryPageWire,
    PACK_FRAME_LIMIT,
};
use world_persistence::WorldPackRef;

/// The most a request frame may hold: as much as the newest protocol lets
/// a host write.
pub const DEFAULT_MAX_REQUEST_BYTES: usize = PACK_FRAME_LIMIT;
/// The most a response frame may hold. A response to a host on a protocol
/// before v5 stops at what that host reads (`PACK_FRAME_LIMIT_BEFORE_V5`).
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = PACK_FRAME_LIMIT;

/// Stateful stdio server for one exact World Pack registration.
///
/// External Pack authors keep implementing the ordinary Host `WorldRegistration`
/// / `WorldSession` surface. This adapter owns the JSONL process protocol and does
/// not expose wire details to the World implementation itself.
pub struct PackServer {
    registry: WorldRegistry,
    descriptor: PackDescriptor,
    pack: WorldPackRef,
    session: Option<Box<dyn WorldSession>>,
    /// The archive the World was opened from, when the World only read it:
    /// let go of once the answer to `open` is on its way, since freeing a
    /// long history takes a while and the host need not wait for it.
    opened_from: Option<world_persistence::WorldArchive>,
    /// Where the World stood when the host last asked to mark it.
    mark: Option<world_host::SessionCheckpoint>,
}

impl PackServer {
    pub fn new(registration: WorldRegistration) -> Result<Self, PackServerError> {
        let descriptor = protocol_descriptor(&registration.descriptor)
            .with_capabilities(registration.capabilities().iter().cloned());
        let pack = registration.descriptor.pack.clone();
        let mut registry = WorldRegistry::new();
        registry
            .register(registration)
            .map_err(PackServerError::Host)?;
        Ok(Self {
            registry,
            descriptor,
            pack,
            session: None,
            opened_from: None,
            mark: None,
        })
    }

    pub fn descriptor(&self) -> &PackDescriptor {
        &self.descriptor
    }

    pub fn has_session(&self) -> bool {
        self.session.is_some()
    }

    /// Handle one already-decoded protocol request. Host/session failures are
    /// returned as protocol `Error` responses so a well-formed peer can decide
    /// whether to continue. The bool is true only after `Shutdown`.
    pub fn handle_request(
        &mut self,
        envelope: PackRequestEnvelope,
    ) -> (PackResponseEnvelope, bool) {
        let request_id = envelope.request_id;
        let protocol_version = envelope.protocol_version;
        let (response, shutdown) = match self.handle(envelope.request) {
            Ok(step) => step,
            Err(error) => (
                PackResponse::Error {
                    message: error.to_string(),
                },
                false,
            ),
        };
        // Answered in the protocol it was asked in, so a host that knows
        // this Pack by an older manifest still reads it.
        let mut envelope = PackResponseEnvelope::new(request_id, response);
        let latest = envelope.protocol_version;
        envelope.protocol_version = protocol_version;
        if envelope.validate().is_err() {
            envelope.protocol_version = latest;
        }
        (envelope, shutdown)
    }

    fn handle(&mut self, request: PackRequest) -> Result<(PackResponse, bool), PackServerError> {
        self.let_go();
        match request {
            PackRequest::Describe => Ok((
                PackResponse::Descriptor {
                    descriptor: self.descriptor.clone(),
                },
                false,
            )),
            PackRequest::Create => {
                self.require_uninitialized("create")?;
                let session = self
                    .registry
                    .create_exact(&self.pack)
                    .map_err(PackServerError::Host)?;
                let snapshot = ProjectionSnapshotWire::from(&session.snapshot());
                self.session = Some(session);
                Ok((PackResponse::Snapshot { snapshot }, false))
            }
            PackRequest::Open { archive } => {
                self.require_uninitialized("open")?;
                let (session, lent) = self
                    .registry
                    .open_owned_archive(archive)
                    .map_err(PackServerError::Host)?;
                let snapshot = ProjectionSnapshotWire::from(&session.snapshot());
                self.session = Some(session);
                self.opened_from = lent;
                Ok((PackResponse::Snapshot { snapshot }, false))
            }
            PackRequest::Snapshot => {
                let session = self.session("snapshot")?;
                Ok((
                    PackResponse::Snapshot {
                        snapshot: ProjectionSnapshotWire::from(&session.snapshot()),
                    },
                    false,
                ))
            }
            PackRequest::Handle { intent } => {
                let session = self.session_mut("handle")?;
                let snapshot = session
                    .handle(intent.into())
                    .map_err(PackServerError::Host)?;
                Ok((
                    PackResponse::Snapshot {
                        snapshot: ProjectionSnapshotWire::from(&snapshot),
                    },
                    false,
                ))
            }
            PackRequest::Hear { to, words } => {
                let session = self.session("hear")?;
                let prompt = session
                    .hearing(to.into(), &words)
                    .map_err(PackServerError::Host)?;
                let hearing = session
                    .voice_hearing(to.into(), &words)
                    .map_err(PackServerError::Host)?
                    .map(world_pack_protocol::VoiceHearingWire::from);
                Ok((PackResponse::Hearing { prompt, hearing }, false))
            }
            PackRequest::Story { request } => {
                let session = self.session("story")?;
                let page = session
                    .story(request.into())
                    .map_err(PackServerError::Host)?;
                Ok((
                    PackResponse::Story {
                        page: page.as_ref().map(StoryPageWire::from),
                    },
                    false,
                ))
            }
            PackRequest::Advance { periods } => {
                let session = self.session_mut("advance")?;
                let snapshot = session
                    .advance_background(periods)
                    .map_err(PackServerError::Host)?;
                Ok((
                    PackResponse::Snapshot {
                        snapshot: ProjectionSnapshotWire::from(&snapshot),
                    },
                    false,
                ))
            }
            PackRequest::Archive => {
                let session = self.session("archive")?;
                let archive = session.archive().map_err(PackServerError::Host)?;
                Ok((PackResponse::Archive { archive }, false))
            }
            PackRequest::Shutdown => Ok((PackResponse::Ok, true)),
            PackRequest::Checkpoint => {
                let session = self.session_mut("checkpoint")?;
                let mark = session.checkpoint().map_err(PackServerError::Host)?;
                let kept = mark.is_some();
                self.mark = mark;
                Ok((PackResponse::Checkpointed { kept }, false))
            }
            PackRequest::Rollback => {
                let mark = self.mark.take().ok_or_else(|| {
                    PackServerError::InvalidSequence("cannot roll back: nothing was marked".into())
                })?;
                let session = self.session_mut("rollback")?;
                session.rollback(mark).map_err(PackServerError::Host)?;
                Ok((PackResponse::Ok, false))
            }
            PackRequest::ArchiveSince { events } => {
                let session = self.session("archive")?;
                let archive = session
                    .archive_since(events)
                    .map_err(PackServerError::Host)?;
                Ok((PackResponse::Archive { archive }, false))
            }
        }
    }

    /// Frees what was kept only until the last answer was sent.
    pub fn let_go(&mut self) {
        self.opened_from = None;
    }

    fn require_uninitialized(&self, operation: &'static str) -> Result<(), PackServerError> {
        if self.session.is_some() {
            Err(PackServerError::InvalidSequence(format!(
                "cannot {operation}: World session is already initialized"
            )))
        } else {
            Ok(())
        }
    }

    fn session(&self, operation: &'static str) -> Result<&dyn WorldSession, PackServerError> {
        self.session.as_deref().ok_or_else(|| {
            PackServerError::InvalidSequence(format!(
                "cannot {operation}: create or open a World first"
            ))
        })
    }

    fn session_mut(
        &mut self,
        operation: &'static str,
    ) -> Result<&mut (dyn WorldSession + '_), PackServerError> {
        match self.session.as_deref_mut() {
            Some(session) => Ok(session),
            None => Err(PackServerError::InvalidSequence(format!(
                "cannot {operation}: create or open a World first"
            ))),
        }
    }
}

/// Serve one exact Pack registration over stdin/stdout until Shutdown or EOF.
pub fn serve_stdio(registration: WorldRegistration) -> Result<(), PackServerError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    serve_jsonl(registration, stdin.lock(), stdout.lock())
}

/// Generic JSONL loop used by `serve_stdio` and deterministic tests.
pub fn serve_jsonl<R, W>(
    registration: WorldRegistration,
    reader: R,
    writer: W,
) -> Result<(), PackServerError>
where
    R: BufRead,
    W: Write,
{
    let mut server = PackServer::new(registration)?;
    serve_server_jsonl(&mut server, reader, writer)
}

pub fn serve_server_jsonl<R, W>(
    server: &mut PackServer,
    mut reader: R,
    writer: W,
) -> Result<(), PackServerError>
where
    R: BufRead,
    W: Write,
{
    let mut writer = BufWriter::new(writer);
    loop {
        let line = read_bounded_line(&mut reader, DEFAULT_MAX_REQUEST_BYTES)?;
        if line.is_empty() {
            writer.flush().map_err(PackServerError::Io)?;
            return Ok(());
        }
        let envelope = decode_request(line.trim_end())
            .map_err(|error| PackServerError::Protocol(error.to_string()))?;
        let (response, shutdown) = server.handle_request(envelope);
        let request_id = response.request_id;
        if let ResponseWrite::Oversized(max_bytes) = write_response(&mut writer, response)? {
            return Err(PackServerError::ResponseTooLarge {
                request_id,
                max_bytes,
            });
        }
        drop(line);
        server.let_go();
        if shutdown {
            writer.flush().map_err(PackServerError::Io)?;
            return Ok(());
        }
    }
}

/// Build a v1 direct-process manifest for the currently running Pack executable.
/// This is intended for a Pack binary's `--print-manifest` command.
pub fn manifest_for_current_exe(
    descriptor: &WorldDescriptor,
) -> Result<PackManifest, PackServerError> {
    let executable = env::current_exe()
        .map_err(PackServerError::Io)?
        .canonicalize()
        .map_err(PackServerError::Io)?;
    manifest_for_canonical_exe(descriptor, &executable)
}

/// Write a portable v1 `.worldpack` containing this Pack executable.
/// The bundle manifest always names the single embedded program and carries no
/// runtime arguments, so installing it does not expand the v1 trust surface.
pub fn write_current_exe_bundle(
    descriptor: &WorldDescriptor,
    destination: impl AsRef<Path>,
) -> Result<PackBundleHeader, PackServerError> {
    let executable = env::current_exe()
        .map_err(PackServerError::Io)?
        .canonicalize()
        .map_err(PackServerError::Io)?;
    write_program_bundle(destination, protocol_descriptor(descriptor), executable)
        .map_err(|error| PackServerError::Bundle(error.to_string()))
}

fn manifest_for_canonical_exe(
    descriptor: &WorldDescriptor,
    executable: &Path,
) -> Result<PackManifest, PackServerError> {
    let command = executable
        .to_str()
        .ok_or_else(|| PackServerError::ManifestPathNotUtf8(executable.to_path_buf()))?;
    Ok(PackManifest::process(
        protocol_descriptor(descriptor),
        command,
        Vec::new(),
    ))
}

fn protocol_descriptor(descriptor: &WorldDescriptor) -> PackDescriptor {
    PackDescriptor::new(
        descriptor.pack.clone(),
        descriptor.title.clone(),
        descriptor.description.clone(),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResponseWrite {
    Sent,
    /// Over the limit it names, so an error went in its place.
    Oversized(usize),
}

fn write_response<W: Write>(
    writer: &mut W,
    response: PackResponseEnvelope,
) -> Result<ResponseWrite, PackServerError> {
    let request_id = response.request_id;
    let protocol_version = response.protocol_version;
    let max_bytes = DEFAULT_MAX_RESPONSE_BYTES.min(pack_frame_limit(protocol_version));
    let mut encoded =
        encode_response(&response).map_err(|error| PackServerError::Protocol(error.to_string()))?;
    let outcome = if encoded.len().saturating_add(1) > max_bytes {
        let mut error = PackResponseEnvelope::new(
            request_id,
            PackResponse::Error {
                message: format!(
                    "Pack response exceeds {max_bytes} byte protocol limit; session terminated to avoid state desynchronization"
                ),
            },
        );
        error.protocol_version = protocol_version;
        encoded = encode_response(&error)
            .map_err(|error| PackServerError::Protocol(error.to_string()))?;
        ResponseWrite::Oversized(max_bytes)
    } else {
        ResponseWrite::Sent
    };
    writer
        .write_all(encoded.as_bytes())
        .and_then(|_| writer.write_all(b"\n"))
        .and_then(|_| writer.flush())
        .map_err(PackServerError::Io)?;
    Ok(outcome)
}

fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<String, PackServerError> {
    let mut bytes = Vec::new();
    let mut limited = reader.take(max_bytes.saturating_add(1) as u64);
    limited
        .read_until(b'\n', &mut bytes)
        .map_err(PackServerError::Io)?;
    if bytes.len() > max_bytes {
        return Err(PackServerError::Protocol(format!(
            "Pack request exceeds {max_bytes} byte protocol limit"
        )));
    }
    String::from_utf8(bytes)
        .map_err(|error| PackServerError::Protocol(format!("Pack request is not UTF-8: {error}")))
}

#[derive(Debug)]
pub enum PackServerError {
    Io(io::Error),
    Host(HostError),
    Protocol(String),
    Bundle(String),
    ResponseTooLarge { request_id: u64, max_bytes: usize },
    ManifestPathNotUtf8(PathBuf),
    InvalidSequence(String),
}

impl fmt::Display for PackServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "Pack server I/O failed: {error}"),
            Self::Host(error) => write!(f, "Pack Host operation failed: {error}"),
            Self::Protocol(error) => write!(f, "Pack protocol failed: {error}"),
            Self::Bundle(error) => write!(f, "Pack bundle failed: {error}"),
            Self::ResponseTooLarge {
                request_id,
                max_bytes,
            } => write!(
                f,
                "Pack response for request {request_id} exceeded {max_bytes} bytes; session terminated"
            ),
            Self::ManifestPathNotUtf8(path) => write!(
                f,
                "Pack executable path cannot be represented in the v1 manifest: {}",
                path.display()
            ),
            Self::InvalidSequence(error) => write!(f, "Pack request sequence is invalid: {error}"),
        }
    }
}

impl Error for PackServerError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use world_pack_protocol::{
        decode_response, encode_request, PackRequest, PackRequestEnvelope, PackResponse,
        ProjectionIntentWire,
    };
    use world_persistence::{WorldArchive, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
    use world_projection::{ProjectionIntent, ProjectionSnapshot};

    const PACK_ID: &str = "fixture.pack.server";
    const PACK_VERSION: &str = "one";

    struct FixtureSession {
        world_time: u64,
    }

    impl WorldSession for FixtureSession {
        fn pack(&self) -> WorldPackRef {
            WorldPackRef::new(PACK_ID, PACK_VERSION)
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: format!("Fixture @ {}", self.world_time),
                world_time: self.world_time,
                ..ProjectionSnapshot::default()
            }
        }

        fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            match intent {
                ProjectionIntent::InvokeCommand(command) if command == "increment" => {
                    self.world_time += 1;
                    Ok(self.snapshot())
                }
                ProjectionIntent::InvokeCommand(command) if command == "huge" => {
                    self.world_time += 1;
                    let mut snapshot = self.snapshot();
                    snapshot.title = "x".repeat(DEFAULT_MAX_RESPONSE_BYTES);
                    Ok(snapshot)
                }
                _ => Err(HostError::session("unsupported fixture intent")),
            }
        }

        fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
            self.world_time += periods;
            Ok(self.snapshot())
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            Ok(Some(WorldArchive {
                format: WORLD_ARCHIVE_FORMAT.into(),
                format_version: WORLD_ARCHIVE_VERSION,
                pack: self.pack(),
                world_time: self.world_time,
                events: Vec::new(),
                pending: Vec::new(),
                checkpoint: None,
            }))
        }
        fn checkpoint(&mut self) -> Result<Option<world_host::SessionCheckpoint>, HostError> {
            Ok(Some(world_host::SessionCheckpoint::new(self.world_time)))
        }

        fn rollback(&mut self, checkpoint: world_host::SessionCheckpoint) -> Result<(), HostError> {
            self.world_time = checkpoint.into_inner()?;
            Ok(())
        }
    }

    fn registration() -> WorldRegistration {
        WorldRegistration::new(
            WorldDescriptor {
                pack: WorldPackRef::new(PACK_ID, PACK_VERSION),
                title: "Fixture Pack".into(),
                description: "server fixture".into(),
            },
            || Ok(Box::new(FixtureSession { world_time: 0 })),
        )
        .with_archive_opener(|archive| {
            Ok(Box::new(FixtureSession {
                world_time: archive.world_time,
            }))
        })
    }

    fn request(id: u64, request: PackRequest) -> String {
        let mut line = encode_request(&PackRequestEnvelope::new(id, request)).unwrap();
        line.push('\n');
        line
    }

    fn responses(output: Vec<u8>) -> Vec<PackResponseEnvelope> {
        String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| decode_response(line).unwrap())
            .collect()
    }

    #[test]
    fn a_world_is_marked_changed_and_gone_back_in_the_same_process() {
        let increment = || PackRequest::Handle {
            intent: ProjectionIntentWire::InvokeCommand {
                command: "increment".into(),
            },
        };
        let input = [
            request(1, PackRequest::Create),
            request(2, PackRequest::Checkpoint),
            request(3, increment()),
            request(4, PackRequest::Rollback),
            request(5, PackRequest::Snapshot),
            request(6, PackRequest::Checkpoint),
            request(7, increment()),
            request(8, PackRequest::ArchiveSince { events: 0 }),
            request(9, PackRequest::Rollback),
            request(10, PackRequest::Rollback),
            request(11, PackRequest::Shutdown),
        ]
        .concat();
        let mut output = Vec::new();
        serve_jsonl(registration(), Cursor::new(input.into_bytes()), &mut output).unwrap();
        let responses = responses(output);
        let world_time = |index: usize| match &responses[index].response {
            PackResponse::Snapshot { snapshot } => snapshot.world_time,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            responses[1].response,
            PackResponse::Checkpointed { kept: true }
        );
        assert_eq!(world_time(2), 1);
        assert_eq!(responses[3].response, PackResponse::Ok);
        assert_eq!(world_time(4), 0);
        assert_eq!(world_time(6), 1);
        assert!(matches!(
            &responses[7].response,
            PackResponse::Archive { archive: Some(archive) } if archive.world_time == 1
        ));
        assert_eq!(responses[8].response, PackResponse::Ok);
        // A mark is gone back to once.
        assert!(matches!(responses[9].response, PackResponse::Error { .. }));
    }

    #[test]
    fn jsonl_server_drives_complete_world_session() {
        let input = [
            request(1, PackRequest::Describe),
            request(2, PackRequest::Create),
            request(
                3,
                PackRequest::Handle {
                    intent: ProjectionIntentWire::InvokeCommand {
                        command: "increment".into(),
                    },
                },
            ),
            request(4, PackRequest::Advance { periods: 4 }),
            request(5, PackRequest::Archive),
            request(6, PackRequest::Shutdown),
        ]
        .concat();
        let mut output = Vec::new();
        serve_jsonl(registration(), Cursor::new(input.into_bytes()), &mut output).unwrap();
        let responses = responses(output);

        assert_eq!(responses.len(), 6);
        assert!(matches!(
            &responses[0].response,
            PackResponse::Descriptor { descriptor }
                if descriptor.pack == WorldPackRef::new(PACK_ID, PACK_VERSION)
        ));
        assert!(matches!(
            &responses[1].response,
            PackResponse::Snapshot { snapshot } if snapshot.world_time == 0
        ));
        assert!(matches!(
            &responses[2].response,
            PackResponse::Snapshot { snapshot } if snapshot.world_time == 1
        ));
        assert!(matches!(
            &responses[3].response,
            PackResponse::Snapshot { snapshot } if snapshot.world_time == 5
        ));
        assert!(matches!(
            &responses[4].response,
            PackResponse::Archive { archive: Some(archive) } if archive.world_time == 5
        ));
        assert!(matches!(responses[5].response, PackResponse::Ok));
        assert_eq!(
            responses
                .iter()
                .map(|response| response.request_id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6]
        );
    }

    #[test]
    fn open_restores_exact_archive_through_host_integrity_gate() {
        let archive = WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new(PACK_ID, PACK_VERSION),
            world_time: 9,
            events: Vec::new(),
            pending: Vec::new(),
            checkpoint: None,
        };
        let input = [
            request(1, PackRequest::Open { archive }),
            request(2, PackRequest::Snapshot),
            request(3, PackRequest::Shutdown),
        ]
        .concat();
        let mut output = Vec::new();
        serve_jsonl(registration(), Cursor::new(input.into_bytes()), &mut output).unwrap();
        let responses = responses(output);

        assert!(matches!(
            &responses[0].response,
            PackResponse::Snapshot { snapshot } if snapshot.world_time == 9
        ));
        assert!(matches!(
            &responses[1].response,
            PackResponse::Snapshot { snapshot } if snapshot.world_time == 9
        ));
    }

    #[test]
    fn invalid_sequence_is_a_protocol_error_response_not_a_server_crash() {
        let input = [
            request(7, PackRequest::Snapshot),
            request(8, PackRequest::Shutdown),
        ]
        .concat();
        let mut output = Vec::new();
        serve_jsonl(registration(), Cursor::new(input.into_bytes()), &mut output).unwrap();
        let responses = responses(output);

        assert!(matches!(
            &responses[0].response,
            PackResponse::Error { message } if message.contains("create or open")
        ));
        assert!(matches!(responses[1].response, PackResponse::Ok));
    }

    #[test]
    fn oversized_mutating_response_is_fatal_before_any_followup_request() {
        let input = [
            request(1, PackRequest::Create),
            request(
                2,
                PackRequest::Handle {
                    intent: ProjectionIntentWire::InvokeCommand {
                        command: "huge".into(),
                    },
                },
            ),
            request(
                3,
                PackRequest::Handle {
                    intent: ProjectionIntentWire::InvokeCommand {
                        command: "increment".into(),
                    },
                },
            ),
        ]
        .concat();
        let mut server = PackServer::new(registration()).unwrap();
        let mut output = Vec::new();
        let error = serve_server_jsonl(&mut server, Cursor::new(input.into_bytes()), &mut output)
            .unwrap_err();

        assert!(matches!(
            error,
            PackServerError::ResponseTooLarge { request_id: 2, .. }
        ));
        let responses = responses(output);
        assert_eq!(responses.len(), 2);
        assert!(matches!(
            &responses[1].response,
            PackResponse::Error { message } if message.contains("session terminated")
        ));
        assert_eq!(server.session.as_ref().unwrap().snapshot().world_time, 1);
    }

    #[cfg(unix)]
    #[test]
    fn manifest_rejects_non_utf8_executable_paths() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let descriptor = WorldDescriptor {
            pack: WorldPackRef::new(PACK_ID, PACK_VERSION),
            title: "Fixture Pack".into(),
            description: "server fixture".into(),
        };
        let path = PathBuf::from(OsString::from_vec(vec![b'/', b't', b'm', b'p', b'/', 0xff]));
        let error = manifest_for_canonical_exe(&descriptor, &path).unwrap_err();
        assert!(matches!(error, PackServerError::ManifestPathNotUtf8(found) if found == path));
    }

    #[test]
    fn malformed_wire_input_terminates_the_server() {
        let mut output = Vec::new();
        let error = serve_jsonl(
            registration(),
            Cursor::new(b"not-json\n".to_vec()),
            &mut output,
        )
        .unwrap_err();
        assert!(matches!(error, PackServerError::Protocol(_)));
        assert!(output.is_empty());
    }

    #[test]
    fn current_executable_manifest_is_direct_and_exact() {
        let descriptor = WorldDescriptor {
            pack: WorldPackRef::new(PACK_ID, PACK_VERSION),
            title: "Fixture Pack".into(),
            description: "server fixture".into(),
        };
        let manifest = manifest_for_current_exe(&descriptor).unwrap();
        assert_eq!(manifest.descriptor.pack, descriptor.pack);
        assert_eq!(manifest.descriptor.title, descriptor.title);
        match manifest.runtime {
            world_pack_protocol::PackRuntimeManifest::Process { command, args } => {
                assert!(PathBuf::from(command).is_absolute());
                assert!(args.is_empty());
            }
        }
    }
}
