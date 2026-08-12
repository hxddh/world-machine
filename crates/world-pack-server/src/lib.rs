use std::env;
use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};
use world_pack_protocol::{
    decode_request, encode_response, PackDescriptor, PackManifest, PackRequest, PackRequestEnvelope,
    PackResponse, PackResponseEnvelope, ProjectionSnapshotWire,
};
use world_persistence::WorldPackRef;

pub const DEFAULT_MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

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
}

impl PackServer {
    pub fn new(registration: WorldRegistration) -> Result<Self, PackServerError> {
        let descriptor = protocol_descriptor(&registration.descriptor);
        let pack = registration.descriptor.pack.clone();
        let mut registry = WorldRegistry::new();
        registry.register(registration).map_err(PackServerError::Host)?;
        Ok(Self {
            registry,
            descriptor,
            pack,
            session: None,
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
        let (response, shutdown) = match self.handle(envelope.request) {
            Ok(step) => step,
            Err(error) => (
                PackResponse::Error {
                    message: error.to_string(),
                },
                false,
            ),
        };
        (PackResponseEnvelope::new(request_id, response), shutdown)
    }

    fn handle(&mut self, request: PackRequest) -> Result<(PackResponse, bool), PackServerError> {
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
                let session = self
                    .registry
                    .open_archive(&archive)
                    .map_err(PackServerError::Host)?;
                let snapshot = ProjectionSnapshotWire::from(&session.snapshot());
                self.session = Some(session);
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
        }
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
        self.session
            .as_deref()
            .ok_or_else(|| PackServerError::InvalidSequence(format!(
                "cannot {operation}: create or open a World first"
            )))
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
        write_response(&mut writer, response)?;
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
    Ok(PackManifest::process(
        protocol_descriptor(descriptor),
        executable.to_string_lossy().into_owned(),
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

fn write_response<W: Write>(
    writer: &mut W,
    response: PackResponseEnvelope,
) -> Result<(), PackServerError> {
    let request_id = response.request_id;
    let mut encoded = encode_response(&response)
        .map_err(|error| PackServerError::Protocol(error.to_string()))?;
    if encoded.len().saturating_add(1) > DEFAULT_MAX_RESPONSE_BYTES {
        encoded = encode_response(&PackResponseEnvelope::new(
            request_id,
            PackResponse::Error {
                message: format!(
                    "Pack response exceeds {} byte protocol limit",
                    DEFAULT_MAX_RESPONSE_BYTES
                ),
            },
        ))
        .map_err(|error| PackServerError::Protocol(error.to_string()))?;
    }
    writer
        .write_all(encoded.as_bytes())
        .and_then(|_| writer.write_all(b"\n"))
        .and_then(|_| writer.flush())
        .map_err(PackServerError::Io)
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
    InvalidSequence(String),
}

impl fmt::Display for PackServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "Pack server I/O failed: {error}"),
            Self::Host(error) => write!(f, "Pack Host operation failed: {error}"),
            Self::Protocol(error) => write!(f, "Pack protocol failed: {error}"),
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

        fn handle(
            &mut self,
            intent: ProjectionIntent,
        ) -> Result<ProjectionSnapshot, HostError> {
            match intent {
                ProjectionIntent::InvokeCommand(command) if command == "increment" => {
                    self.world_time += 1;
                    Ok(self.snapshot())
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
            }))
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
            responses.iter().map(|response| response.request_id).collect::<Vec<_>>(),
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
