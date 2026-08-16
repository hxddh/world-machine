from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    assert count == 1, f"{label}: expected 1 match, found {count}"
    return text.replace(old, new, 1)


protocol_path = Path("crates/world-pack-protocol/src/lib.rs")
text = protocol_path.read_text()
text = replace_once(
    text,
    "pub const PACK_PROTOCOL_VERSION: u32 = 1;\n",
    "pub const PACK_PROTOCOL_VERSION_V1: u32 = 1;\n"
    "pub const PACK_PROTOCOL_VERSION_V2: u32 = 2;\n"
    "pub const PACK_PROTOCOL_VERSION: u32 = PACK_PROTOCOL_VERSION_V2;\n",
    "protocol constants",
)
text = replace_once(
    text,
    """    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_protocol_version(self.protocol_version)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackResponseEnvelope {
""",
    """    pub fn for_version(
        protocol_version: u32,
        request_id: u64,
        request: PackRequest,
    ) -> Result<Self, ProtocolError> {
        validate_protocol_version(protocol_version)?;
        Ok(Self {
            protocol_version,
            request_id,
            request,
        })
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_protocol_version(self.protocol_version)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackResponseEnvelope {
""",
    "request for_version",
)
text = replace_once(
    text,
    """    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_protocol_version(self.protocol_version)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = \"type\", rename_all = \"snake_case\")]
pub enum PackRequest {
""",
    """    pub fn for_version(
        protocol_version: u32,
        request_id: u64,
        response: PackResponse,
    ) -> Result<Self, ProtocolError> {
        validate_protocol_version(protocol_version)?;
        Ok(Self {
            protocol_version,
            request_id,
            response,
        })
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_protocol_version(self.protocol_version)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = \"type\", rename_all = \"snake_case\")]
pub enum PackRequest {
""",
    "response for_version",
)
text = replace_once(
    text,
    """fn validate_protocol_version(version: u32) -> Result<(), ProtocolError> {
    if version == PACK_PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(ProtocolError::UnsupportedProtocolVersion(version))
    }
}
""",
    """fn validate_protocol_version(version: u32) -> Result<(), ProtocolError> {
    if matches!(version, PACK_PROTOCOL_VERSION_V1 | PACK_PROTOCOL_VERSION_V2) {
        Ok(())
    } else {
        Err(ProtocolError::UnsupportedProtocolVersion(version))
    }
}
""",
    "protocol validation",
)
protocol_path.write_text(text)

process_path = Path("crates/world-pack-process/src/lib.rs")
text = process_path.read_text()
text = replace_once(
    text,
    """pub struct ProcessPack {
    pub manifest_path: PathBuf,
    pub descriptor: PackDescriptor,
    pub command: PathBuf,
    pub args: Vec<String>,
    pin: Option<ProcessPackPin>,
}
""",
    """pub struct ProcessPack {
    pub manifest_path: PathBuf,
    pub descriptor: PackDescriptor,
    pub protocol_version: u32,
    pub command: PathBuf,
    pub args: Vec<String>,
    pin: Option<ProcessPackPin>,
}
""",
    "ProcessPack field",
)
text = replace_once(
    text,
    """        let PackRuntimeManifest::Process { command, args } = manifest.runtime;
        let command = resolve_command(&manifest_path, &command)?;
        Ok(Self {
            manifest_path,
            descriptor: manifest.descriptor,
            command,
            args,
            pin: None,
        })
""",
    """        let protocol_version = manifest.protocol_version;
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
""",
    "ProcessPack load",
)
text = replace_once(
    text,
    """struct ProcessClient {
    child: Child,
    stdin: Option<BufWriter<ChildStdin>>,
    responses: Receiver<io::Result<String>>,
    next_request_id: u64,
    request_timeout: Duration,
    launch_cleanup: Option<PathBuf>,
}
""",
    """struct ProcessClient {
    child: Child,
    stdin: Option<BufWriter<ChildStdin>>,
    responses: Receiver<io::Result<String>>,
    protocol_version: u32,
    next_request_id: u64,
    request_timeout: Duration,
    launch_cleanup: Option<PathBuf>,
}
""",
    "ProcessClient field",
)
text = replace_once(
    text,
    """        Ok(Self {
            child,
            stdin: Some(BufWriter::new(stdin)),
            responses: spawn_response_reader(stdout, DEFAULT_MAX_RESPONSE_BYTES),
            next_request_id: 1,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            launch_cleanup,
        })
""",
    """        Ok(Self {
            child,
            stdin: Some(BufWriter::new(stdin)),
            responses: spawn_response_reader(stdout, DEFAULT_MAX_RESPONSE_BYTES),
            protocol_version: pack.protocol_version,
            next_request_id: 1,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            launch_cleanup,
        })
""",
    "ProcessClient spawn",
)
text = replace_once(
    text,
    "        let envelope = PackRequestEnvelope::new(request_id, request);\n",
    """        let envelope = PackRequestEnvelope::for_version(
            self.protocol_version,
            request_id,
            request,
        )
        .map_err(|error| HostError::session(format!(\"invalid Pack protocol version: {error}\")))?;
""",
    "request envelope",
)
text = replace_once(
    text,
    """        if response.request_id != request_id {
            let actual = response.request_id;
            self.terminate();
            return Err(HostError::session(format!(
                \"external Pack response id mismatch: expected {request_id}, got {actual}\"
            )));
        }
""",
    """        if response.protocol_version != self.protocol_version {
            let actual = response.protocol_version;
            let expected = self.protocol_version;
            self.terminate();
            return Err(HostError::session(format!(
                \"external Pack response protocol version mismatch: expected {expected}, got {actual}\"
            )));
        }
        if response.request_id != request_id {
            let actual = response.request_id;
            self.terminate();
            return Err(HostError::session(format!(
                \"external Pack response id mismatch: expected {request_id}, got {actual}\"
            )));
        }
""",
    "response version check",
)
text = replace_once(
    text,
    """        let envelope = PackRequestEnvelope::new(request_id, PackRequest::Shutdown);
        if let (Ok(encoded), Some(stdin)) = (encode_request(&envelope), self.stdin.as_mut()) {
""",
    """        let Ok(envelope) = PackRequestEnvelope::for_version(
            self.protocol_version,
            request_id,
            PackRequest::Shutdown,
        ) else {
            self.stdin.take();
            return;
        };
        if let (Ok(encoded), Some(stdin)) = (encode_request(&envelope), self.stdin.as_mut()) {
""",
    "shutdown envelope",
)
process_path.write_text(text)
