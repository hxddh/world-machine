from pathlib import Path

p = Path('crates/world-pack-server/src/lib.rs')
s = p.read_text()
s = s.replace('use std::io::{self, BufRead, BufWriter, Read, Write};\n', 'use std::io::{self, BufRead, BufWriter, Read, Write};\nuse std::path::{Path, PathBuf};\n', 1)
s = s.replace('    use std::path::PathBuf;\n', '', 1)

old_loop = '''        let (response, shutdown) = server.handle_request(envelope);
        write_response(&mut writer, response)?;
        if shutdown {
            writer.flush().map_err(PackServerError::Io)?;
            return Ok(());
        }
'''
new_loop = '''        let (response, shutdown) = server.handle_request(envelope);
        let request_id = response.request_id;
        if write_response(&mut writer, response)? == ResponseWrite::Oversized {
            return Err(PackServerError::ResponseTooLarge {
                request_id,
                max_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            });
        }
        if shutdown {
            writer.flush().map_err(PackServerError::Io)?;
            return Ok(());
        }
'''
if old_loop not in s:
    raise SystemExit('serve loop marker not found')
s = s.replace(old_loop, new_loop, 1)

old_manifest = '''    Ok(PackManifest::process(
        protocol_descriptor(descriptor),
        executable.to_string_lossy().into_owned(),
        Vec::new(),
    ))
}
'''
new_manifest = '''    manifest_for_canonical_exe(descriptor, &executable)
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
'''
if old_manifest not in s:
    raise SystemExit('manifest marker not found')
s = s.replace(old_manifest, new_manifest, 1)

old_write = '''fn write_response<W: Write>(
    writer: &mut W,
    response: PackResponseEnvelope,
) -> Result<(), PackServerError> {
    let request_id = response.request_id;
    let mut encoded =
        encode_response(&response).map_err(|error| PackServerError::Protocol(error.to_string()))?;
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
        .and_then(|_| writer.write_all(b"\\n"))
        .and_then(|_| writer.flush())
        .map_err(PackServerError::Io)
}
'''
new_write = '''#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResponseWrite {
    Sent,
    Oversized,
}

fn write_response<W: Write>(
    writer: &mut W,
    response: PackResponseEnvelope,
) -> Result<ResponseWrite, PackServerError> {
    let request_id = response.request_id;
    let mut encoded =
        encode_response(&response).map_err(|error| PackServerError::Protocol(error.to_string()))?;
    let outcome = if encoded.len().saturating_add(1) > DEFAULT_MAX_RESPONSE_BYTES {
        encoded = encode_response(&PackResponseEnvelope::new(
            request_id,
            PackResponse::Error {
                message: format!(
                    "Pack response exceeds {} byte protocol limit; session terminated to avoid state desynchronization",
                    DEFAULT_MAX_RESPONSE_BYTES
                ),
            },
        ))
        .map_err(|error| PackServerError::Protocol(error.to_string()))?;
        ResponseWrite::Oversized
    } else {
        ResponseWrite::Sent
    };
    writer
        .write_all(encoded.as_bytes())
        .and_then(|_| writer.write_all(b"\\n"))
        .and_then(|_| writer.flush())
        .map_err(PackServerError::Io)?;
    Ok(outcome)
}
'''
if old_write not in s:
    raise SystemExit('write_response marker not found')
s = s.replace(old_write, new_write, 1)

s = s.replace('    Protocol(String),\n    InvalidSequence(String),\n', '    Protocol(String),\n    ResponseTooLarge { request_id: u64, max_bytes: usize },\n    ManifestPathNotUtf8(PathBuf),\n    InvalidSequence(String),\n', 1)
old_display = '''            Self::Protocol(error) => write!(f, "Pack protocol failed: {error}"),
            Self::InvalidSequence(error) => write!(f, "Pack request sequence is invalid: {error}"),
'''
new_display = '''            Self::Protocol(error) => write!(f, "Pack protocol failed: {error}"),
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
'''
if old_display not in s:
    raise SystemExit('display marker not found')
s = s.replace(old_display, new_display, 1)

old_handle = '''                ProjectionIntent::InvokeCommand(command) if command == "increment" => {
                    self.world_time += 1;
                    Ok(self.snapshot())
                }
                _ => Err(HostError::session("unsupported fixture intent")),
'''
new_handle = '''                ProjectionIntent::InvokeCommand(command) if command == "increment" => {
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
'''
if old_handle not in s:
    raise SystemExit('fixture handle marker not found')
s = s.replace(old_handle, new_handle, 1)

marker = '''    #[test]
    fn malformed_wire_input_terminates_the_server() {'''
tests = '''    #[test]
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
        let error = serve_server_jsonl(
            &mut server,
            Cursor::new(input.into_bytes()),
            &mut output,
        )
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

'''
if marker not in s:
    raise SystemExit('test insertion marker not found')
s = s.replace(marker, tests + marker, 1)
p.write_text(s)
