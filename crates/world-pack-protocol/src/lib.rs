use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::io::{Read, Write};
use world_core::{EntityId, EventId, RelationId};
use world_persistence::{WorldArchive, WorldPackRef};
use world_projection::{
    BriefingItem, BriefingItemKind, BriefingProjection, CanvasChange, CanvasItem, CanvasItemKind,
    CanvasLink, CanvasLinkTone, CanvasMark, CanvasProjection, CollectionItem, CollectionProjection,
    CommandEffect, EffectChange, GroundCover, InspectorProjection, InspectorRow, InspectorSection,
    MarkShape, ProjectionCapabilities, ProjectionCommand, ProjectionIntent, ProjectionSnapshot,
    Scenery, Season, SelectionId, TimelineItem, TimelineProjection, Tone, WhyNode, WhyProjection,
};
use world_projection::{DrawPart, DrawShape, Drawing, Ears, Ink, Stance};

mod mark;
pub use mark::{
    DesignableWire, NamingWire, PatternWire, PlotOfferWire, PlotWire, VariantWire,
    MOST_MARK_COMMAND, MOST_PLOTS, MOST_PLOT_OFFERS, MOST_PROPOSALS,
};
mod stories;
pub use stories::{
    AlmanacWire, LegendLineWire, LegendWire, MomentWire, NamedWire, PanelWire, StoryPageWire,
    StoryRequestWire, MOST_ALMANAC_NAMES, MOST_ALMANAC_YEARS, MOST_LEGEND_LINES, MOST_PANEL_CAST,
    MOST_PANEL_PROPS,
};

pub const PACK_MANIFEST_FORMAT: &str = "world-machine-pack";
pub const PACK_MANIFEST_VERSION: u32 = 1;
pub const PACK_PROTOCOL_VERSION_V1: u32 = 1;
pub const PACK_PROTOCOL_VERSION_V2: u32 = 2;
/// Adds `hear`, and `ears` on `say`: an app that asks a model itself.
pub const PACK_PROTOCOL_VERSION_V3: u32 = 3;
/// Adds a checkpoint to the archive `open` hands over: the Pack restores
/// from it, and opens a history that keeps only what came after it (a World
/// code's).
pub const PACK_PROTOCOL_VERSION_V4: u32 = 4;
/// Packs the archive `open` hands over and `archive` hands back: written in
/// the compact encoding, deflated and in base64 (see [`pack_archive`]), so a
/// history of years crosses in a few megabytes, and frames may be up to
/// [`PACK_FRAME_LIMIT`].
pub const PACK_PROTOCOL_VERSION_V5: u32 = 5;
/// Adds `checkpoint`, `rollback` and `archive_since`: a host that saves after
/// every change tries it on the World it has open and goes back if the save
/// fails, and asks for only the events it has not saved, so one Pack
/// process serves an open World for as long as it is open.
pub const PACK_PROTOCOL_VERSION_V6: u32 = 6;
/// Adds `story`: a World asked for a legend, a moment or an almanac, and
/// the snapshot's latest moments and New Year's almanac (which, like every
/// presentation hint, an older app passes over).
pub const PACK_PROTOCOL_VERSION_V7: u32 = 7;
/// Adds what a Pack can do, said in its descriptor (`capabilities`:
/// [`CAPABILITY_PLOTS`], [`CAPABILITY_DESIGNS`], [`CAPABILITY_NAMES`],
/// [`CAPABILITY_STORY`]); typed `design` and `name` intents in place of a
/// command with `=<argument>` after it; and wire words a newer Pack may
/// send that this build does not know (a tone, a shape, a kind of page),
/// read as `unknown` and passed over rather than failing the snapshot.
///
/// A host still sends a Pack on v7 a design or a name as the command it
/// offered with `=<argument>` (see [`ProjectionIntent::as_command`]), and
/// a Pack still reads that form, as older apps send it.
pub const PACK_PROTOCOL_VERSION_V8: u32 = 8;
pub const PACK_PROTOCOL_VERSION: u32 = PACK_PROTOCOL_VERSION_V8;

pub use world_projection::capability::{
    DESIGNS as CAPABILITY_DESIGNS, NAMES as CAPABILITY_NAMES, PLOTS as CAPABILITY_PLOTS,
    STORY as CAPABILITY_STORY,
};

/// The most a frame (one line, with its newline) may hold between a host
/// and a Pack that speaks v5 or later.
pub const PACK_FRAME_LIMIT: usize = 64 * 1024 * 1024;
/// The most a frame may hold for a Pack before v5, which reads no more.
pub const PACK_FRAME_LIMIT_BEFORE_V5: usize = 16 * 1024 * 1024;
/// The most a packed archive may unpack to: as much as a World file may.
pub const MAX_UNPACKED_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
/// What a packed archive starts with, before its base64.
pub const PACKED_ARCHIVE_PREFIX: &str = "deflate:";

/// The most a frame may hold in `protocol_version`.
pub fn pack_frame_limit(protocol_version: u32) -> usize {
    if protocol_version >= PACK_PROTOCOL_VERSION_V5 {
        PACK_FRAME_LIMIT
    } else {
        PACK_FRAME_LIMIT_BEFORE_V5
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackManifest {
    pub format: String,
    pub format_version: u32,
    pub protocol_version: u32,
    pub descriptor: PackDescriptor,
    pub runtime: PackRuntimeManifest,
}

impl PackManifest {
    pub fn process(
        descriptor: PackDescriptor,
        command: impl Into<String>,
        args: Vec<String>,
    ) -> Self {
        Self {
            format: PACK_MANIFEST_FORMAT.into(),
            format_version: PACK_MANIFEST_VERSION,
            protocol_version: PACK_PROTOCOL_VERSION,
            descriptor,
            runtime: PackRuntimeManifest::Process {
                command: command.into(),
                args,
            },
        }
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.format != PACK_MANIFEST_FORMAT {
            return Err(ProtocolError::UnsupportedManifestFormat(
                self.format.clone(),
            ));
        }
        if self.format_version != PACK_MANIFEST_VERSION {
            return Err(ProtocolError::UnsupportedManifestVersion(
                self.format_version,
            ));
        }
        validate_protocol_version(self.protocol_version)?;
        self.descriptor.validate()?;
        match &self.runtime {
            PackRuntimeManifest::Process { command, .. } if command.trim().is_empty() => {
                Err(ProtocolError::InvalidProcessCommand)
            }
            PackRuntimeManifest::Process { .. } => Ok(()),
        }
    }

    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, ManifestDecodeError> {
        let manifest = serde_json::from_str::<Self>(json).map_err(ManifestDecodeError::Json)?;
        manifest.validate().map_err(ManifestDecodeError::Protocol)?;
        Ok(manifest)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDescriptor {
    pub pack: WorldPackRef,
    pub title: String,
    pub description: String,
    /// What the Pack can do beyond the protocol's core (v8), such as
    /// [`CAPABILITY_DESIGNS`]. Words this build does not know are kept and
    /// passed over, so a newer Pack's capabilities never fail a handshake.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
}

impl PackDescriptor {
    pub fn new(
        pack: WorldPackRef,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            pack,
            title: title.into(),
            description: description.into(),
            capabilities: Vec::new(),
        }
    }

    /// The descriptor saying it can do `capabilities`.
    pub fn with_capabilities<I, S>(mut self, capabilities: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.capabilities = capabilities.into_iter().map(Into::into).collect();
        self.capabilities.sort();
        self.capabilities.dedup();
        self
    }

    /// Whether the Pack says it can do `capability`.
    pub fn can(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|known| known == capability)
    }

    /// The same Pack, whatever it says it can do: what a host compares a
    /// running Pack's own descriptor against its manifest by.
    pub fn same_pack(&self, other: &PackDescriptor) -> bool {
        self.pack == other.pack
            && self.title == other.title
            && self.description == other.description
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.pack.id.trim().is_empty() || self.pack.version.trim().is_empty() {
            return Err(ProtocolError::InvalidPack);
        }
        if self.title.trim().is_empty() {
            return Err(ProtocolError::InvalidTitle);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PackRuntimeManifest {
    Process {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackRequestEnvelope {
    pub protocol_version: u32,
    pub request_id: u64,
    pub request: PackRequest,
}

impl PackRequestEnvelope {
    pub fn new(request_id: u64, request: PackRequest) -> Self {
        Self {
            protocol_version: PACK_PROTOCOL_VERSION,
            request_id,
            request,
        }
    }

    pub fn for_version(
        protocol_version: u32,
        request_id: u64,
        request: PackRequest,
    ) -> Result<Self, ProtocolError> {
        let envelope = Self {
            protocol_version,
            request_id,
            request,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_protocol_version(self.protocol_version)?;
        validate_request_for_protocol(self.protocol_version, &self.request)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackResponseEnvelope {
    pub protocol_version: u32,
    pub request_id: u64,
    pub response: PackResponse,
}

impl PackResponseEnvelope {
    pub fn new(request_id: u64, response: PackResponse) -> Self {
        Self {
            protocol_version: PACK_PROTOCOL_VERSION,
            request_id,
            response,
        }
    }

    pub fn for_version(
        protocol_version: u32,
        request_id: u64,
        response: PackResponse,
    ) -> Result<Self, ProtocolError> {
        let envelope = Self {
            protocol_version,
            request_id,
            response,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_protocol_version(self.protocol_version)?;
        validate_response_for_protocol(self.protocol_version, &self.response)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PackRequest {
    Describe,
    Create,
    Open {
        /// Tagged before v5; packed from v5 (see [`pack_archive`]). Either
        /// reads back.
        #[serde(deserialize_with = "wire_archive")]
        archive: WorldArchive,
    },
    Snapshot,
    Handle {
        intent: ProjectionIntentWire,
    },
    /// What a language model should be asked, to hear the player's words to
    /// someone: nothing changes, and a World without talk has no prompt.
    Hear {
        to: SelectionIdWire,
        words: String,
    },
    Advance {
        periods: u64,
    },
    Archive,
    Shutdown,
    /// Mark where the World stands, replacing any mark before (v6).
    Checkpoint,
    /// Go back to the mark, forgetting everything since (v6).
    Rollback,
    /// The archive with only the events after the first `events` (v6).
    ArchiveSince {
        events: usize,
    },
    /// A legend, a moment or an almanac: nothing changes (v7).
    Story {
        request: StoryRequestWire,
    },
}

// One response is built per message and serialized at once, so how much
// larger a snapshot is than an acknowledgement costs nothing worth an extra
// indirection at every one of the call sites that build one.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PackResponse {
    Descriptor {
        descriptor: PackDescriptor,
    },
    Snapshot {
        snapshot: ProjectionSnapshotWire,
    },
    Archive {
        /// Tagged before v5; packed from v5, as `open`'s.
        #[serde(deserialize_with = "wire_optional_archive")]
        archive: Option<WorldArchive>,
    },
    Hearing {
        prompt: Option<String>,
    },
    /// Whether the World could mark where it stands (v6).
    Checkpointed {
        kept: bool,
    },
    /// What the World told, if it has that story (v7).
    Story {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        page: Option<StoryPageWire>,
    },
    Ok,
    Error {
        message: String,
    },
}

/// The request as one line of JSON. From v5, an archive in it is packed.
pub fn encode_request(request: &PackRequestEnvelope) -> Result<String, serde_json::Error> {
    match &request.request {
        PackRequest::Open { archive } if request.protocol_version >= PACK_PROTOCOL_VERSION_V5 => {
            let packed = pack_archive(archive).map_err(serde::ser::Error::custom)?;
            serde_json::to_string(&PackedEnvelopeOut {
                protocol_version: request.protocol_version,
                request_id: request.request_id,
                request: PackedRequestOut::Open { archive: &packed },
            })
        }
        _ => serde_json::to_string(request),
    }
}

/// An `open` request as one line of JSON, as [`encode_request`] writes
/// `PackRequest::Open` with this archive, for a host that keeps the archive
/// rather than handing over a copy of a long history to be encoded.
pub fn encode_open_request(
    protocol_version: u32,
    request_id: u64,
    archive: &WorldArchive,
) -> Result<String, ProtocolEncodeError> {
    validate_open(protocol_version, archive)?;
    if protocol_version >= PACK_PROTOCOL_VERSION_V5 {
        let packed = pack_archive(archive)
            .map_err(|error| ProtocolEncodeError::Json(serde::ser::Error::custom(error)))?;
        return open_request_with_packed(protocol_version, request_id, &packed);
    }
    serde_json::to_string(&OpenEnvelopeOut {
        protocol_version,
        request_id,
        request: OpenRequestOut::Open { archive },
    })
    .map_err(ProtocolEncodeError::Json)
}

/// [`encode_open_request`] for a Pack on v5 or later, with the archive's
/// compact JSON already deflated, as a World file keeps it: it is packed as
/// it is, rather than written and deflated again. The JSON may hold fields
/// beside the archive's own (a World file's document), which a Pack reading
/// the archive passes over. The caller vouches that it reads as `archive`.
pub fn encode_open_request_deflated(
    protocol_version: u32,
    request_id: u64,
    archive: &WorldArchive,
    deflated: &[u8],
) -> Result<String, ProtocolEncodeError> {
    validate_open(protocol_version, archive)?;
    if protocol_version < PACK_PROTOCOL_VERSION_V5 {
        return Err(ProtocolEncodeError::Protocol(
            ProtocolError::RequestNotSupportedInProtocol {
                protocol_version,
                request: "open with a packed archive",
            },
        ));
    }
    let mut packed =
        String::with_capacity(PACKED_ARCHIVE_PREFIX.len() + deflated.len() * 4 / 3 + 4);
    packed.push_str(PACKED_ARCHIVE_PREFIX);
    STANDARD.encode_string(deflated, &mut packed);
    open_request_with_packed(protocol_version, request_id, &packed)
}

fn validate_open(protocol_version: u32, archive: &WorldArchive) -> Result<(), ProtocolError> {
    validate_protocol_version(protocol_version)?;
    if archive.checkpoint.is_some() && protocol_version < PACK_PROTOCOL_VERSION_V4 {
        return Err(ProtocolError::RequestNotSupportedInProtocol {
            protocol_version,
            request: "open with a checkpoint",
        });
    }
    Ok(())
}

fn open_request_with_packed(
    protocol_version: u32,
    request_id: u64,
    packed: &str,
) -> Result<String, ProtocolEncodeError> {
    serde_json::to_string(&PackedEnvelopeOut {
        protocol_version,
        request_id,
        request: PackedRequestOut::Open { archive: packed },
    })
    .map_err(ProtocolEncodeError::Json)
}

#[derive(Serialize)]
struct OpenEnvelopeOut<'a> {
    protocol_version: u32,
    request_id: u64,
    request: OpenRequestOut<'a>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OpenRequestOut<'a> {
    Open { archive: &'a WorldArchive },
}

/// Why a request could not be written.
#[derive(Debug)]
pub enum ProtocolEncodeError {
    Json(serde_json::Error),
    Protocol(ProtocolError),
}

impl From<ProtocolError> for ProtocolEncodeError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl fmt::Display for ProtocolEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => error.fmt(f),
            Self::Protocol(error) => error.fmt(f),
        }
    }
}

impl Error for ProtocolEncodeError {}

pub fn decode_request(json: &str) -> Result<PackRequestEnvelope, ProtocolDecodeError> {
    let request =
        serde_json::from_str::<PackRequestEnvelope>(json).map_err(ProtocolDecodeError::Json)?;
    request.validate().map_err(ProtocolDecodeError::Protocol)?;
    Ok(request)
}

/// The response as one line of JSON. From v5, an archive in it is packed.
pub fn encode_response(response: &PackResponseEnvelope) -> Result<String, serde_json::Error> {
    match &response.response {
        PackResponse::Archive {
            archive: Some(archive),
        } if response.protocol_version >= PACK_PROTOCOL_VERSION_V5 => {
            let packed = pack_archive(archive).map_err(serde::ser::Error::custom)?;
            serde_json::to_string(&PackedResponseEnvelopeOut {
                protocol_version: response.protocol_version,
                request_id: response.request_id,
                response: PackedResponseOut::Archive { archive: &packed },
            })
        }
        _ => serde_json::to_string(response),
    }
}

#[derive(Serialize)]
struct PackedEnvelopeOut<'a> {
    protocol_version: u32,
    request_id: u64,
    request: PackedRequestOut<'a>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PackedRequestOut<'a> {
    Open { archive: &'a str },
}

#[derive(Serialize)]
struct PackedResponseEnvelopeOut<'a> {
    protocol_version: u32,
    request_id: u64,
    response: PackedResponseOut<'a>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PackedResponseOut<'a> {
    Archive { archive: &'a str },
}

/// An archive as v5 carries it: its compact encoding (the one a World file
/// keeps, which reads back as exactly this archive, checkpoint and all),
/// deflated, in standard base64, after [`PACKED_ARCHIVE_PREFIX`].
pub fn pack_archive(archive: &WorldArchive) -> Result<String, ArchivePackError> {
    let json = archive
        .to_compact_json(&serde_json::Map::new())
        .map_err(|error| ArchivePackError(error.to_string()))?;
    // The fastest level: most of what a history repeats is found by any,
    // the archive only crosses a pipe, and a World is saved after every
    // change.
    let mut encoder = DeflateEncoder::new(
        Vec::with_capacity(json.len() / 6),
        Compression::new(ARCHIVE_DEFLATE_LEVEL),
    );
    encoder
        .write_all(&json)
        .map_err(|error| ArchivePackError(error.to_string()))?;
    let deflated = encoder
        .finish()
        .map_err(|error| ArchivePackError(error.to_string()))?;
    let mut packed =
        String::with_capacity(PACKED_ARCHIVE_PREFIX.len() + deflated.len() * 4 / 3 + 4);
    packed.push_str(PACKED_ARCHIVE_PREFIX);
    STANDARD.encode_string(&deflated, &mut packed);
    Ok(packed)
}

const ARCHIVE_DEFLATE_LEVEL: u32 = 1;

/// Reads an archive [`pack_archive`] wrote.
pub fn unpack_archive(packed: &str) -> Result<WorldArchive, ArchivePackError> {
    let encoded = packed.strip_prefix(PACKED_ARCHIVE_PREFIX).ok_or_else(|| {
        ArchivePackError(format!(
            "a packed archive starts with {PACKED_ARCHIVE_PREFIX:?}"
        ))
    })?;
    let deflated = STANDARD
        .decode(encoded)
        .map_err(|error| ArchivePackError(format!("packed archive is not base64: {error}")))?;
    let mut json = Vec::with_capacity(deflated.len() * 6);
    DeflateDecoder::new(deflated.as_slice())
        .take(MAX_UNPACKED_ARCHIVE_BYTES + 1)
        .read_to_end(&mut json)
        .map_err(|error| ArchivePackError(format!("packed archive does not inflate: {error}")))?;
    if json.len() as u64 > MAX_UNPACKED_ARCHIVE_BYTES {
        return Err(ArchivePackError(format!(
            "packed archive unpacks to more than {MAX_UNPACKED_ARCHIVE_BYTES} bytes"
        )));
    }
    WorldArchive::from_json_slice(&json).map_err(|error| match error {
        world_persistence::PersistenceError::Json(error) => {
            ArchivePackError(format!("packed archive is not JSON: {error}"))
        }
        error => ArchivePackError(error.to_string()),
    })
}

/// Why an archive could not be packed or unpacked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchivePackError(String);

impl fmt::Display for ArchivePackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for ArchivePackError {}

/// An archive on the wire: packed (a string), or an object, tagged as
/// before v5 (or compact).
fn archive_from_wire(value: serde_json::Value) -> Result<WorldArchive, String> {
    match value {
        serde_json::Value::String(packed) => unpack_archive(&packed).map_err(|e| e.to_string()),
        value => WorldArchive::from_json_value(&value).map_err(|e| e.to_string()),
    }
}

fn wire_archive<'de, D>(deserializer: D) -> Result<WorldArchive, D::Error>
where
    D: serde::Deserializer<'de>,
{
    wire_optional_archive(deserializer)?
        .ok_or_else(|| serde::de::Error::custom("an archive is required"))
}

/// A packed archive is unpacked from the frame's own text, without a copy
/// of it; an archive written out as an object is read as before.
fn wire_optional_archive<'de, D>(deserializer: D) -> Result<Option<WorldArchive>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct WireVisitor;
    impl<'de> serde::de::Visitor<'de> for WireVisitor {
        type Value = Option<WorldArchive>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a packed archive, an archive or null")
        }

        fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_str<E: serde::de::Error>(self, packed: &str) -> Result<Self::Value, E> {
            unpack_archive(packed).map(Some).map_err(E::custom)
        }

        fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
            let value =
                serde_json::Value::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
            archive_from_wire(value)
                .map(Some)
                .map_err(serde::de::Error::custom)
        }
    }
    deserializer.deserialize_any(WireVisitor)
}

pub fn decode_response(json: &str) -> Result<PackResponseEnvelope, ProtocolDecodeError> {
    let response =
        serde_json::from_str::<PackResponseEnvelope>(json).map_err(ProtocolDecodeError::Json)?;
    response.validate().map_err(ProtocolDecodeError::Protocol)?;
    Ok(response)
}

fn validate_protocol_version(version: u32) -> Result<(), ProtocolError> {
    if matches!(
        version,
        PACK_PROTOCOL_VERSION_V1
            | PACK_PROTOCOL_VERSION_V2
            | PACK_PROTOCOL_VERSION_V3
            | PACK_PROTOCOL_VERSION_V4
            | PACK_PROTOCOL_VERSION_V5
            | PACK_PROTOCOL_VERSION_V6
            | PACK_PROTOCOL_VERSION_V7
            | PACK_PROTOCOL_VERSION_V8
    ) {
        Ok(())
    } else {
        Err(ProtocolError::UnsupportedProtocolVersion(version))
    }
}

/// What a Pack speaking an older protocol cannot read: asking it what to
/// ask a model, handing it a model's response, or opening an archive from a
/// checkpoint (which it would pass over, replaying the wrong history).
fn validate_request_for_protocol(
    protocol_version: u32,
    request: &PackRequest,
) -> Result<(), ProtocolError> {
    if let PackRequest::Open { archive } = request {
        if archive.checkpoint.is_some() && protocol_version < PACK_PROTOCOL_VERSION_V4 {
            return Err(ProtocolError::RequestNotSupportedInProtocol {
                protocol_version,
                request: "open with a checkpoint",
            });
        }
    }
    if matches!(
        request,
        PackRequest::Checkpoint | PackRequest::Rollback | PackRequest::ArchiveSince { .. }
    ) && protocol_version < PACK_PROTOCOL_VERSION_V6
    {
        return Err(ProtocolError::RequestNotSupportedInProtocol {
            protocol_version,
            request: "checkpoint",
        });
    }
    if matches!(
        request,
        PackRequest::Handle {
            intent: ProjectionIntentWire::Design { .. } | ProjectionIntentWire::Name { .. }
        }
    ) && protocol_version < PACK_PROTOCOL_VERSION_V8
    {
        return Err(ProtocolError::RequestNotSupportedInProtocol {
            protocol_version,
            request: "design or name",
        });
    }
    if matches!(request, PackRequest::Story { .. }) && protocol_version < PACK_PROTOCOL_VERSION_V7 {
        return Err(ProtocolError::RequestNotSupportedInProtocol {
            protocol_version,
            request: "story",
        });
    }
    let needs_v3 = match request {
        PackRequest::Hear { .. } => true,
        PackRequest::Handle {
            intent: ProjectionIntentWire::Say { ears, .. },
        } => *ears != EarsWire::World,
        _ => false,
    };
    if needs_v3 && protocol_version < PACK_PROTOCOL_VERSION_V3 {
        return Err(ProtocolError::RequestNotSupportedInProtocol {
            protocol_version,
            request: "hear",
        });
    }
    Ok(())
}

fn validate_response_for_protocol(
    protocol_version: u32,
    response: &PackResponse,
) -> Result<(), ProtocolError> {
    match response {
        PackResponse::Snapshot { snapshot } => snapshot.validate_for_protocol(protocol_version),
        _ => Ok(()),
    }
}

fn validate_selection_for_protocol(
    protocol_version: u32,
    selection: SelectionIdWire,
) -> Result<(), ProtocolError> {
    if protocol_version == PACK_PROTOCOL_VERSION_V1
        && matches!(selection, SelectionIdWire::Relation { .. })
    {
        return Err(ProtocolError::SelectionNotSupportedInProtocol {
            protocol_version,
            selection: selection.stable_key(),
        });
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProjectionIntentWire {
    ForkBeforeEvent {
        event: u64,
    },
    InvokeCommand {
        command: String,
    },
    Say {
        to: SelectionIdWire,
        words: String,
        #[serde(default, skip_serializing_if = "EarsWire::is_world")]
        ears: EarsWire,
    },
    Host {
        name: String,
        from: String,
        letter: String,
        #[serde(default)]
        gift: String,
        /// How they look at home.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        look: Option<LookWire>,
        /// The drawing their own World draws them with.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        drawing: Option<DrawingWire>,
        /// Something they say, in their own World's words.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        line: Option<String>,
    },
    /// A design painted on what the World offered as `target` (v8):
    /// `<cells>:<colours>`, as [`world_projection::Design::text`] writes it.
    Design {
        target: String,
        pattern: String,
    },
    /// A name given to what the World offered as `target` (v8).
    Name {
        target: String,
        name: String,
    },
}

/// The longest a guest's name, home, letter or gift is read: anything
/// longer is cut, and the World checks what it gets like anything else.
pub const MOST_GUEST_TEXT: usize = 280;

fn guest_text(text: String) -> String {
    text.chars().take(MOST_GUEST_TEXT).collect()
}

/// The longest model response a Pack is sent to read.
pub const MOST_MODEL_RESPONSE: usize = 8 * 1024;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EarsWire {
    #[default]
    World,
    Model {
        response: String,
    },
    Own,
}

impl EarsWire {
    fn is_world(&self) -> bool {
        *self == EarsWire::World
    }
}

impl From<Ears> for EarsWire {
    fn from(ears: Ears) -> Self {
        match ears {
            Ears::World => Self::World,
            Ears::Model(response) => Self::Model { response },
            Ears::Own => Self::Own,
        }
    }
}

impl From<EarsWire> for Ears {
    fn from(ears: EarsWire) -> Self {
        match ears {
            EarsWire::World => Self::World,
            // A response too long to be an answer is not read at all.
            EarsWire::Model { response } if response.len() > MOST_MODEL_RESPONSE => Self::Own,
            EarsWire::Model { response } => Self::Model(response),
            EarsWire::Own => Self::Own,
        }
    }
}

impl From<ProjectionIntent> for ProjectionIntentWire {
    fn from(intent: ProjectionIntent) -> Self {
        match intent {
            ProjectionIntent::ForkBeforeEvent(event) => Self::ForkBeforeEvent { event: event.0 },
            ProjectionIntent::InvokeCommand(command) => Self::InvokeCommand { command },
            ProjectionIntent::Say { to, words, ears } => Self::Say {
                to: to.into(),
                words,
                ears: ears.into(),
            },
            ProjectionIntent::Host(guest) => Self::Host {
                look: guest.look.map(LookWire::from),
                drawing: guest.drawing.as_ref().map(DrawingWire::from),
                line: guest.line,
                name: guest.name,
                from: guest.from,
                letter: guest.letter,
                gift: guest.gift,
            },
            ProjectionIntent::Design { target, pattern } => Self::Design {
                target,
                pattern: pattern.text(),
            },
            ProjectionIntent::Name { target, name } => Self::Name { target, name },
        }
    }
}

impl From<ProjectionIntentWire> for ProjectionIntent {
    fn from(intent: ProjectionIntentWire) -> Self {
        match intent {
            ProjectionIntentWire::ForkBeforeEvent { event } => {
                Self::ForkBeforeEvent(EventId::new(event))
            }
            ProjectionIntentWire::InvokeCommand { command } => Self::InvokeCommand(command),
            ProjectionIntentWire::Say { to, words, ears } => Self::Say {
                to: to.into(),
                words,
                ears: ears.into(),
            },
            ProjectionIntentWire::Host {
                name,
                from,
                letter,
                gift,
                look,
                drawing,
                line,
            } => Self::Host(world_projection::Guest {
                name: guest_text(name),
                from: guest_text(from),
                letter: guest_text(letter),
                gift: guest_text(gift),
                look: look.map(world_projection::Look::from),
                // A drawing too big to be anyone's is not read at all, and
                // one read is kept only within a guest's bounds.
                drawing: drawing
                    .filter(|drawing| drawing.parts.len() <= world_projection::MOST_GUEST_PARTS)
                    .map(Drawing::from)
                    .filter(world_projection::guest_drawing_is_sound),
                line: line.map(guest_text).filter(|line| !line.trim().is_empty()),
            }),
            // A design that is not one goes to the World as the old form
            // would, which refuses it in its own words.
            ProjectionIntentWire::Design { target, pattern } => {
                match world_projection::Design::parse(&pattern) {
                    Ok(pattern) => Self::Design { target, pattern },
                    Err(_) => {
                        Self::InvokeCommand(world_projection::command_with(&target, &pattern))
                    }
                }
            }
            ProjectionIntentWire::Name { target, name } => Self::Name { target, name },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SelectionIdWire {
    Entity { id: u64 },
    Relation { id: u64 },
    Event { id: u64 },
}

impl SelectionIdWire {
    fn stable_key(self) -> String {
        match self {
            Self::Entity { id } => format!("entity-{id}"),
            Self::Relation { id } => format!("relation-{id}"),
            Self::Event { id } => format!("event-{id}"),
        }
    }
}

impl From<SelectionId> for SelectionIdWire {
    fn from(selection: SelectionId) -> Self {
        match selection {
            SelectionId::Entity(id) => Self::Entity { id: id.0 },
            SelectionId::Relation(id) => Self::Relation { id: id.0 },
            SelectionId::Event(id) => Self::Event { id: id.0 },
        }
    }
}

impl From<SelectionIdWire> for SelectionId {
    fn from(selection: SelectionIdWire) -> Self {
        match selection {
            SelectionIdWire::Entity { id } => Self::Entity(EntityId::new(id)),
            SelectionIdWire::Relation { id } => Self::Relation(RelationId::new(id)),
            SelectionIdWire::Event { id } => Self::Event(EventId::new(id)),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectionSnapshotWire {
    pub title: String,
    pub world_time: u64,
    pub capabilities: ProjectionCapabilitiesWire,
    pub briefing: Option<BriefingProjectionWire>,
    pub commands: Vec<ProjectionCommandWire>,
    pub collection: CollectionProjectionWire,
    pub timeline: TimelineProjectionWire,
    pub canvas: CanvasProjectionWire,
    pub inspectors: Vec<InspectorProjectionWire>,
    pub why: Vec<WhyProjectionWire>,
    /// Optional both ways, like every presentation hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenery: Option<SceneryWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar: Option<CalendarWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gauges: Vec<GaugeWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub voices: Vec<VoiceWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub talks: Vec<TalkWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exchanges: Vec<ExchangeWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub goals: Vec<GoalWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chapters: Vec<ChapterWire>,
    /// Optional both ways: the weather over the scene; clear if absent.
    #[serde(default, skip_serializing_if = "is_clear")]
    pub weather: WeatherWire,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drawings: Vec<DrawingWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keepsakes: Vec<KeepsakeWire>,
    /// Optional both ways: the letter box.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub letters: Vec<LetterWire>,
    /// Optional both ways: the book of everything to find. An older Pack
    /// sends none, and the drawer shows no book.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub book: Vec<BookEntryWire>,
    /// Optional both ways: the latest moments, at most
    /// [`world_projection::MOST_MOMENTS_IN_SNAPSHOT`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub moments: Vec<MomentWire>,
    /// Optional both ways: the year in review, on New Year's day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub almanac: Option<AlmanacWire>,
    /// Optional both ways: every year whose almanac can be asked for now.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub almanac_years: Vec<u32>,
    /// Optional both ways: a favour someone asked of the player, open or
    /// done this period. An older Pack sends none, and the drawer shows no
    /// note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favour: Option<FavourWire>,
}

/// A favour someone asked of the player, as it crosses the boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavourWire {
    pub asker: SelectionIdWire,
    pub whom: SelectionIdWire,
    pub note: String,
    #[serde(default)]
    pub hint: String,
    #[serde(default)]
    pub done: bool,
}

/// One entry in a World's book, as it crosses the boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BookEntryWire {
    pub shelf: String,
    pub name: String,
    #[serde(default)]
    pub found: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<MarkShapeWire>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hint: String,
    /// The moment it keeps, by id (v7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moment: Option<String>,
    /// Who or what it shows (v7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cast: Vec<SelectionIdWire>,
}

/// The most entries one book carries.
pub const MOST_BOOK_ENTRIES: usize = 4_000;

/// Something someone gave the player to keep.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KeepsakeWire {
    pub from: SelectionIdWire,
    pub what: String,
    #[serde(default)]
    pub note: String,
    pub moment: SelectionIdWire,
}

/// A letter someone wrote the player, as it crosses the boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LetterWire {
    pub from: SelectionIdWire,
    pub note: String,
    pub moment: SelectionIdWire,
}

/// The most letters one snapshot carries: the latest.
pub const MOST_LETTERS: usize = 400;

/// The most drawings one snapshot carries.
pub const MOST_DRAWINGS: usize = 64;

/// A drawing a Pack ships, as it crosses the boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawingWire {
    pub id: String,
    pub aspect: f32,
    pub parts: Vec<DrawPartWire>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawPartWire {
    pub shape: DrawShapeWire,
    /// A role (`wall`, `clothes`, …) or a colour (`#rrggbb`); anything else
    /// is drawn in the walls' colour.
    pub ink: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub tone: f32,
    /// Stances this build does not know are left out of the list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stances: Vec<String>,
    /// Moods this build does not know are left out of the list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub moods: Vec<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub swing: f32,
}

fn is_zero(value: &f32) -> bool {
    *value == 0.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DrawShapeWire {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        #[serde(default, skip_serializing_if = "is_zero")]
        round: f32,
    },
    Ellipse {
        x: f32,
        y: f32,
        rx: f32,
        ry: f32,
    },
    Polygon {
        points: Vec<(f32, f32)>,
    },
    Line {
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
    },
    /// A shape from a newer Pack (v8): the part is left out.
    #[serde(other)]
    Unknown,
}

impl From<&Drawing> for DrawingWire {
    fn from(drawing: &Drawing) -> Self {
        Self {
            id: drawing.id.clone(),
            aspect: drawing.aspect,
            parts: drawing
                .parts
                .iter()
                .map(|part| DrawPartWire {
                    shape: match &part.shape {
                        DrawShape::Rect { x, y, w, h, round } => DrawShapeWire::Rect {
                            x: *x,
                            y: *y,
                            w: *w,
                            h: *h,
                            round: *round,
                        },
                        DrawShape::Ellipse { x, y, rx, ry } => DrawShapeWire::Ellipse {
                            x: *x,
                            y: *y,
                            rx: *rx,
                            ry: *ry,
                        },
                        DrawShape::Polygon { points } => DrawShapeWire::Polygon {
                            points: points.clone(),
                        },
                        DrawShape::Line { from, to, width } => DrawShapeWire::Line {
                            from: *from,
                            to: *to,
                            width: *width,
                        },
                    },
                    ink: part.ink.id(),
                    tone: part.tone,
                    stances: part
                        .stances
                        .iter()
                        .map(|stance| stance.id().to_string())
                        .collect(),
                    moods: part
                        .moods
                        .iter()
                        .map(|mood| mood.id().to_string())
                        .collect(),
                    swing: part.swing,
                })
                .collect(),
        }
    }
}

impl From<DrawingWire> for Drawing {
    fn from(drawing: DrawingWire) -> Self {
        Self {
            id: drawing.id,
            aspect: drawing.aspect,
            parts: drawing
                .parts
                .into_iter()
                .filter_map(|part| {
                    Some(DrawPart {
                        shape: match part.shape {
                            DrawShapeWire::Rect { x, y, w, h, round } => {
                                DrawShape::Rect { x, y, w, h, round }
                            }
                            DrawShapeWire::Ellipse { x, y, rx, ry } => {
                                DrawShape::Ellipse { x, y, rx, ry }
                            }
                            DrawShapeWire::Polygon { points } => DrawShape::Polygon { points },
                            DrawShapeWire::Line { from, to, width } => {
                                DrawShape::Line { from, to, width }
                            }
                            DrawShapeWire::Unknown => return None,
                        },
                        ink: Ink::from_id(&part.ink).unwrap_or(Ink::Wall),
                        tone: part.tone,
                        stances: part
                            .stances
                            .iter()
                            .filter_map(|stance| Stance::from_id(stance))
                            .collect(),
                        moods: part
                            .moods
                            .iter()
                            .filter_map(|mood| world_projection::Mood::from_id(mood))
                            .collect(),
                        swing: part.swing,
                    })
                })
                .collect(),
        }
    }
}

/// The weather over a World's scene. A Pack's word this build does not
/// know reads as clear.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherWire {
    #[default]
    Clear,
    Cloudy,
    Rain,
    Storm,
    Snow,
    Fog,
    Dust,
    #[serde(other)]
    Unknown,
}

fn is_clear(weather: &WeatherWire) -> bool {
    matches!(weather, WeatherWire::Clear | WeatherWire::Unknown)
}

impl From<world_projection::Weather> for WeatherWire {
    fn from(weather: world_projection::Weather) -> Self {
        use world_projection::Weather;
        match weather {
            Weather::Clear => Self::Clear,
            Weather::Cloudy => Self::Cloudy,
            Weather::Rain => Self::Rain,
            Weather::Storm => Self::Storm,
            Weather::Snow => Self::Snow,
            Weather::Fog => Self::Fog,
            Weather::Dust => Self::Dust,
        }
    }
}

impl From<WeatherWire> for world_projection::Weather {
    fn from(weather: WeatherWire) -> Self {
        match weather {
            WeatherWire::Clear | WeatherWire::Unknown => Self::Clear,
            WeatherWire::Cloudy => Self::Cloudy,
            WeatherWire::Rain => Self::Rain,
            WeatherWire::Storm => Self::Storm,
            WeatherWire::Snow => Self::Snow,
            WeatherWire::Fog => Self::Fog,
            WeatherWire::Dust => Self::Dust,
        }
    }
}

/// A standing goal, drawn as an outline until its parts are built.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalWire {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub shape: MarkShapeWire,
    pub done: u32,
    pub parts: u32,
}

/// A chapter of the story that has ended.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChapterWire {
    pub number: u32,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moment: Option<SelectionIdWire>,
}

/// Something someone said aloud at a moment. Narration only.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VoiceWire {
    pub moment: SelectionIdWire,
    pub speaker: SelectionIdWire,
    pub line: String,
}

/// A question a player can put to someone, and the answer; `asks_for` names
/// one of the snapshot's commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TalkWire {
    pub who: SelectionIdWire,
    pub question: String,
    pub answer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asks_for: Option<String>,
}

/// Something the player said to someone, and the answer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExchangeWire {
    pub who: SelectionIdWire,
    pub words: String,
    pub answer: String,
    pub moment: SelectionIdWire,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asks_for: Option<String>,
}

/// How someone looks. Every part is optional; colours are 0xRRGGBB.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LookWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clothes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hair: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carries: Option<CarryWire>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bird: bool,
    /// How far through life someone is; one this build does not know is
    /// drawn grown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age: Option<AgeStageWire>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub grey: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stoop: bool,
}

/// How far through life someone is, on the wire.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgeStageWire {
    Baby,
    Child,
    Teen,
    Adult,
    Elder,
    #[serde(other)]
    Unknown,
}

impl From<world_projection::AgeStage> for AgeStageWire {
    fn from(age: world_projection::AgeStage) -> Self {
        use world_projection::AgeStage;
        match age {
            AgeStage::Baby => Self::Baby,
            AgeStage::Child => Self::Child,
            AgeStage::Teen => Self::Teen,
            AgeStage::Adult => Self::Adult,
            AgeStage::Elder => Self::Elder,
        }
    }
}

impl AgeStageWire {
    fn stage(self) -> Option<world_projection::AgeStage> {
        use world_projection::AgeStage;
        Some(match self {
            Self::Baby => AgeStage::Baby,
            Self::Child => AgeStage::Child,
            Self::Teen => AgeStage::Teen,
            Self::Adult => AgeStage::Adult,
            Self::Elder => AgeStage::Elder,
            Self::Unknown => return None,
        })
    }
}

/// What someone carries. Something a newer Pack names and this build does
/// not know is carried as nothing.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarryWire {
    Tool,
    Book,
    Bread,
    Fish,
    Basket,
    Satchel,
    Plant,
    Mug,
    #[serde(other)]
    Unknown,
}

impl From<world_projection::Carry> for CarryWire {
    fn from(carry: world_projection::Carry) -> Self {
        use world_projection::Carry;
        match carry {
            Carry::Tool => Self::Tool,
            Carry::Book => Self::Book,
            Carry::Bread => Self::Bread,
            Carry::Fish => Self::Fish,
            Carry::Basket => Self::Basket,
            Carry::Satchel => Self::Satchel,
            Carry::Plant => Self::Plant,
            Carry::Mug => Self::Mug,
        }
    }
}

impl CarryWire {
    fn carried(self) -> Option<world_projection::Carry> {
        use world_projection::Carry;
        Some(match self {
            Self::Tool => Carry::Tool,
            Self::Book => Carry::Book,
            Self::Bread => Carry::Bread,
            Self::Fish => Carry::Fish,
            Self::Basket => Carry::Basket,
            Self::Satchel => Carry::Satchel,
            Self::Plant => Carry::Plant,
            Self::Mug => Carry::Mug,
            Self::Unknown => return None,
        })
    }
}

impl From<world_projection::Look> for LookWire {
    fn from(look: world_projection::Look) -> Self {
        Self {
            clothes: look.clothes,
            hair: look.hair,
            skin: look.skin,
            carries: look.carries.map(Into::into),
            bird: look.bird,
            age: look.age.map(Into::into),
            grey: look.grey,
            stoop: look.stoop,
        }
    }
}

impl From<LookWire> for world_projection::Look {
    fn from(look: LookWire) -> Self {
        let colour = |value: Option<u32>| value.filter(|value| *value <= 0xff_ffff);
        Self {
            clothes: colour(look.clothes),
            hair: colour(look.hair),
            skin: colour(look.skin),
            carries: look.carries.and_then(CarryWire::carried),
            bird: look.bird,
            age: look.age.and_then(AgeStageWire::stage),
            grey: look.grey,
            stoop: look.stoop,
        }
    }
}

/// Something a World keeps score of. `value` runs from 0 to 1; anything
/// else (or not a number) is clamped into that range on the way in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GaugeWire {
    pub id: String,
    pub label: String,
    pub value: f32,
    #[serde(default)]
    pub reading: String,
    #[serde(default)]
    pub tone: ToneWire,
}

/// How far a choice moves one gauge, in thousandths of its range.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GaugeMoveWire {
    pub gauge: String,
    pub by: i32,
}

/// What a World counts its time in: `unit` names one, `length` is how much
/// world time it is.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CalendarWire {
    pub unit: String,
    pub length: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coming: Option<String>,
    /// Optional both ways: whether today is a festival day.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub festival_today: bool,
    /// Optional both ways: how many of `unit` make the World's year (v7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<u64>,
}

/// How a World looks from a distance, as `0xRRGGBB` colours.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SceneryWire {
    pub sky_top: u32,
    pub sky_bottom: u32,
    pub far: u32,
    pub near: u32,
    pub sun: u32,
}

impl From<Scenery> for SceneryWire {
    fn from(scenery: Scenery) -> Self {
        Self {
            sky_top: scenery.sky_top,
            sky_bottom: scenery.sky_bottom,
            far: scenery.far,
            near: scenery.near,
            sun: scenery.sun,
        }
    }
}

impl From<SceneryWire> for Scenery {
    fn from(scenery: SceneryWire) -> Self {
        // Anything above 24 bits is not a colour; keep the colour part.
        let colour = |value: u32| value & 0x00ff_ffff;
        Self {
            sky_top: colour(scenery.sky_top),
            sky_bottom: colour(scenery.sky_bottom),
            far: colour(scenery.far),
            near: colour(scenery.near),
            sun: colour(scenery.sun),
        }
    }
}

impl ProjectionSnapshotWire {
    fn validate_for_protocol(&self, protocol_version: u32) -> Result<(), ProtocolError> {
        if let Some(briefing) = &self.briefing {
            for item in &briefing.items {
                if let Some(selection) = item.selection {
                    validate_selection_for_protocol(protocol_version, selection)?;
                }
            }
        }
        for item in &self.collection.items {
            validate_selection_for_protocol(protocol_version, item.id)?;
        }
        for item in &self.timeline.items {
            validate_selection_for_protocol(protocol_version, item.id)?;
        }
        for item in &self.canvas.items {
            validate_selection_for_protocol(protocol_version, item.id)?;
            if let Some(at) = item.at {
                validate_selection_for_protocol(protocol_version, at)?;
            }
            if let Some(home) = item.home {
                validate_selection_for_protocol(protocol_version, home)?;
            }
            for stop in &item.day {
                validate_selection_for_protocol(protocol_version, stop.at)?;
            }
        }
        for command in &self.commands {
            if let Some(asker) = command.asker {
                validate_selection_for_protocol(protocol_version, asker)?;
            }
            for effect in &command.effects {
                if let Some(target) = effect.target {
                    validate_selection_for_protocol(protocol_version, target)?;
                }
            }
        }
        for link in &self.canvas.links {
            validate_selection_for_protocol(protocol_version, link.from)?;
            validate_selection_for_protocol(protocol_version, link.to)?;
            if let Some(selection) = link.selection {
                validate_selection_for_protocol(protocol_version, selection)?;
            }
        }
        for mark in &self.canvas.marks {
            if let Some(selection) = mark.selection {
                validate_selection_for_protocol(protocol_version, selection)?;
            }
        }
        for voice in &self.voices {
            validate_selection_for_protocol(protocol_version, voice.moment)?;
            validate_selection_for_protocol(protocol_version, voice.speaker)?;
        }
        for talk in &self.talks {
            validate_selection_for_protocol(protocol_version, talk.who)?;
        }
        for exchange in &self.exchanges {
            validate_selection_for_protocol(protocol_version, exchange.who)?;
            validate_selection_for_protocol(protocol_version, exchange.moment)?;
        }
        for chapter in &self.chapters {
            if let Some(moment) = chapter.moment {
                validate_selection_for_protocol(protocol_version, moment)?;
            }
        }
        for keepsake in &self.keepsakes {
            validate_selection_for_protocol(protocol_version, keepsake.from)?;
            validate_selection_for_protocol(protocol_version, keepsake.moment)?;
        }
        for inspector in &self.inspectors {
            validate_selection_for_protocol(protocol_version, inspector.selection)?;
        }
        let moments = self.moments.iter().flat_map(MomentWire::selections);
        let almanac = self.almanac.iter().flat_map(AlmanacWire::selections);
        let book = self
            .book
            .iter()
            .flat_map(|entry| entry.cast.iter().copied());
        for selection in moments.chain(almanac).chain(book) {
            validate_selection_for_protocol(protocol_version, selection)?;
        }
        Ok(())
    }
}

impl From<&ProjectionSnapshot> for ProjectionSnapshotWire {
    fn from(snapshot: &ProjectionSnapshot) -> Self {
        Self {
            title: snapshot.title.clone(),
            world_time: snapshot.world_time,
            capabilities: snapshot.capabilities.into(),
            briefing: snapshot.briefing.as_ref().map(Into::into),
            commands: snapshot.commands.iter().map(Into::into).collect(),
            collection: (&snapshot.collection).into(),
            timeline: (&snapshot.timeline).into(),
            canvas: (&snapshot.canvas).into(),
            inspectors: snapshot.inspectors.values().map(Into::into).collect(),
            why: snapshot.why.values().map(Into::into).collect(),
            scenery: snapshot.scenery.map(Into::into),
            calendar: snapshot.calendar.as_ref().map(|calendar| CalendarWire {
                unit: calendar.unit.clone(),
                length: calendar.length,
                season: calendar.season.clone(),
                coming: calendar.coming.clone(),
                festival_today: calendar.festival_today,
                year: calendar.year,
            }),
            gauges: snapshot
                .gauges
                .iter()
                .map(|gauge| GaugeWire {
                    id: gauge.id.clone(),
                    label: gauge.label.clone(),
                    value: gauge.value,
                    reading: gauge.reading.clone(),
                    tone: gauge.tone.into(),
                })
                .collect(),
            voices: snapshot
                .voices
                .iter()
                .map(|voice| VoiceWire {
                    moment: voice.moment.into(),
                    speaker: voice.speaker.into(),
                    line: voice.line.clone(),
                })
                .collect(),
            talks: snapshot
                .talks
                .iter()
                .map(|talk| TalkWire {
                    who: talk.who.into(),
                    question: talk.question.clone(),
                    answer: talk.answer.clone(),
                    asks_for: talk.asks_for.clone(),
                })
                .collect(),
            exchanges: snapshot
                .exchanges
                .iter()
                .map(|exchange| ExchangeWire {
                    who: exchange.who.into(),
                    words: exchange.words.clone(),
                    answer: exchange.answer.clone(),
                    moment: exchange.moment.into(),
                    asks_for: exchange.asks_for.clone(),
                })
                .collect(),
            goals: snapshot
                .goals
                .iter()
                .map(|goal| GoalWire {
                    id: goal.id.clone(),
                    label: goal.label.clone(),
                    shape: goal.shape.into(),
                    done: goal.done,
                    parts: goal.parts,
                })
                .collect(),
            chapters: snapshot
                .chapters
                .iter()
                .map(|chapter| ChapterWire {
                    number: chapter.number,
                    title: chapter.title.clone(),
                    summary: chapter.summary.clone(),
                    moment: chapter.moment.map(Into::into),
                })
                .collect(),
            weather: snapshot.weather.into(),
            drawings: snapshot.drawings.iter().map(Into::into).collect(),
            keepsakes: snapshot
                .keepsakes
                .iter()
                .map(|keepsake| KeepsakeWire {
                    from: keepsake.from.into(),
                    what: keepsake.what.clone(),
                    note: keepsake.note.clone(),
                    moment: keepsake.moment.into(),
                })
                .collect(),
            letters: snapshot
                .letters
                .iter()
                .map(|letter| LetterWire {
                    from: letter.from.into(),
                    note: letter.note.clone(),
                    moment: letter.moment.into(),
                })
                .collect(),
            book: snapshot
                .book
                .iter()
                .map(|entry| BookEntryWire {
                    shelf: entry.shelf.clone(),
                    name: entry.name.clone(),
                    found: entry.found,
                    shape: entry.shape.map(Into::into),
                    hint: entry.hint.clone(),
                    moment: entry.moment.clone(),
                    cast: entry.cast.iter().copied().map(Into::into).collect(),
                })
                .collect(),
            moments: world_projection::latest_moments(&snapshot.moments)
                .iter()
                .map(Into::into)
                .collect(),
            almanac: snapshot.almanac.as_ref().map(Into::into),
            almanac_years: snapshot
                .almanac_years
                .iter()
                .copied()
                .take(MOST_ALMANAC_YEARS)
                .collect(),
            favour: snapshot.favour.as_ref().map(|favour| FavourWire {
                asker: favour.asker.into(),
                whom: favour.whom.into(),
                note: favour.note.clone(),
                hint: favour.hint.clone(),
                done: favour.done,
            }),
        }
    }
}

impl TryFrom<ProjectionSnapshotWire> for ProjectionSnapshot {
    type Error = ProtocolError;

    fn try_from(snapshot: ProjectionSnapshotWire) -> Result<Self, Self::Error> {
        let mut inspectors = BTreeMap::new();
        for inspector in snapshot.inspectors {
            let key = SelectionId::from(inspector.selection);
            let wire_key = inspector.selection;
            if inspectors
                .insert(key, InspectorProjection::from(inspector))
                .is_some()
            {
                return Err(ProtocolError::DuplicateInspector(wire_key.stable_key()));
            }
        }

        let mut why = BTreeMap::new();
        for projection in snapshot.why {
            let event = EventId::new(projection.event);
            if why
                .insert(event, WhyProjection::try_from(projection)?)
                .is_some()
            {
                return Err(ProtocolError::DuplicateWhy(event.0));
            }
        }

        Ok(Self {
            title: snapshot.title,
            world_time: snapshot.world_time,
            capabilities: snapshot.capabilities.into(),
            briefing: snapshot.briefing.map(Into::into),
            commands: snapshot.commands.into_iter().map(Into::into).collect(),
            collection: snapshot.collection.into(),
            timeline: snapshot.timeline.into(),
            canvas: snapshot.canvas.into(),
            inspectors,
            why,
            scenery: snapshot.scenery.map(Into::into),
            // A calendar that cannot count (no name, or no length) is no
            // calendar: time falls back to plain numbers.
            calendar: snapshot
                .calendar
                .filter(|calendar| calendar.length > 0 && !calendar.unit.trim().is_empty())
                .map(|calendar| world_projection::Calendar {
                    unit: calendar.unit,
                    length: calendar.length,
                    season: calendar.season.filter(|season| !season.trim().is_empty()),
                    coming: calendar.coming.filter(|coming| !coming.trim().is_empty()),
                    festival_today: calendar.festival_today,
                    year: calendar.year.filter(|year| *year > 0),
                }),
            gauges: snapshot
                .gauges
                .into_iter()
                .filter(|gauge| !gauge.id.trim().is_empty())
                .map(|gauge| world_projection::Gauge {
                    id: gauge.id,
                    label: gauge.label,
                    value: if gauge.value.is_finite() {
                        gauge.value.clamp(0.0, 1.0)
                    } else {
                        0.0
                    },
                    reading: gauge.reading,
                    tone: gauge.tone.into(),
                })
                .collect(),
            // A line nobody could read is no line.
            voices: snapshot
                .voices
                .into_iter()
                .filter(|voice| !voice.line.trim().is_empty())
                .map(|voice| world_projection::Voice {
                    moment: voice.moment.into(),
                    speaker: voice.speaker.into(),
                    line: voice.line,
                })
                .collect(),
            talks: snapshot
                .talks
                .into_iter()
                .filter(|talk| !talk.question.trim().is_empty() && !talk.answer.trim().is_empty())
                .map(|talk| world_projection::Talk {
                    who: talk.who.into(),
                    question: talk.question,
                    answer: talk.answer,
                    asks_for: talk.asks_for.filter(|command| !command.trim().is_empty()),
                })
                .collect(),
            exchanges: snapshot
                .exchanges
                .into_iter()
                .filter(|exchange| !exchange.answer.trim().is_empty())
                .map(|exchange| world_projection::Exchange {
                    who: exchange.who.into(),
                    words: exchange.words,
                    answer: exchange.answer,
                    moment: exchange.moment.into(),
                    asks_for: exchange
                        .asks_for
                        .filter(|command| !command.trim().is_empty()),
                })
                .collect(),
            // A goal needs a name and at least one part; no more can be done
            // than it takes.
            goals: snapshot
                .goals
                .into_iter()
                .filter(|goal| !goal.label.trim().is_empty() && goal.parts > 0)
                .map(|goal| world_projection::Goal {
                    id: goal.id,
                    label: goal.label,
                    shape: goal.shape.into(),
                    done: goal.done.min(goal.parts),
                    parts: goal.parts,
                })
                .collect(),
            chapters: snapshot
                .chapters
                .into_iter()
                .filter(|chapter| !chapter.title.trim().is_empty())
                .map(|chapter| world_projection::Chapter {
                    number: chapter.number,
                    title: chapter.title,
                    summary: chapter.summary,
                    moment: chapter.moment.map(Into::into),
                })
                .collect(),
            weather: snapshot.weather.into(),
            // A drawing the app could not draw is no drawing: its items
            // are drawn with the app's own shapes.
            drawings: snapshot
                .drawings
                .into_iter()
                .map(Drawing::from)
                .filter(Drawing::is_drawable)
                .take(MOST_DRAWINGS)
                .collect(),
            keepsakes: snapshot
                .keepsakes
                .into_iter()
                .filter(|keepsake| !keepsake.what.trim().is_empty())
                .map(|keepsake| world_projection::Keepsake {
                    from: keepsake.from.into(),
                    what: keepsake.what,
                    note: keepsake.note,
                    moment: keepsake.moment.into(),
                })
                .collect(),
            // The latest letters, as many as the app keeps, each with words.
            letters: {
                let letters = snapshot
                    .letters
                    .into_iter()
                    .filter(|letter| !letter.note.trim().is_empty())
                    .collect::<Vec<_>>();
                letters[letters.len().saturating_sub(MOST_LETTERS)..]
                    .iter()
                    .map(|letter| world_projection::Letter {
                        from: letter.from.into(),
                        note: letter.note.clone(),
                        moment: letter.moment.into(),
                    })
                    .collect()
            },
            // A book no longer than the app keeps, of entries with names.
            book: snapshot
                .book
                .into_iter()
                .filter(|entry| !entry.name.trim().is_empty() && !entry.shelf.trim().is_empty())
                .take(MOST_BOOK_ENTRIES)
                .map(|entry| world_projection::BookEntry {
                    shelf: entry.shelf,
                    name: entry.name,
                    found: entry.found,
                    shape: entry.shape.map(Into::into),
                    hint: entry.hint,
                    moment: entry.moment.filter(|moment| !moment.trim().is_empty()),
                    cast: entry.cast.into_iter().map(Into::into).collect(),
                })
                .collect(),
            // The latest few moments, each with its three panels.
            moments: {
                let moments = snapshot
                    .moments
                    .into_iter()
                    .filter_map(MomentWire::into_moment)
                    .collect::<Vec<_>>();
                world_projection::latest_moments(&moments)
            },
            almanac: snapshot.almanac.map(Into::into),
            almanac_years: {
                let mut years = snapshot
                    .almanac_years
                    .into_iter()
                    .filter(|year| *year > 0)
                    .take(MOST_ALMANAC_YEARS)
                    .collect::<Vec<_>>();
                years.sort_unstable();
                years.dedup();
                years
            },
            // A favour with no words to show is no favour.
            favour: snapshot
                .favour
                .filter(|favour| !favour.note.trim().is_empty())
                .map(|favour| world_projection::Favour {
                    asker: favour.asker.into(),
                    whom: favour.whom.into(),
                    note: favour.note,
                    hint: favour.hint,
                    done: favour.done,
                }),
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectionCapabilitiesWire {
    pub fork: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub background: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub talk: bool,
}

impl From<ProjectionCapabilities> for ProjectionCapabilitiesWire {
    fn from(capabilities: ProjectionCapabilities) -> Self {
        Self {
            fork: capabilities.fork,
            background: capabilities.background,
            talk: capabilities.talk,
        }
    }
}

impl From<ProjectionCapabilitiesWire> for ProjectionCapabilities {
    fn from(capabilities: ProjectionCapabilitiesWire) -> Self {
        Self {
            fork: capabilities.fork,
            background: capabilities.background,
            talk: capabilities.talk,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectionCommandWire {
    pub id: String,
    pub title: String,
    pub detail: String,
    /// Optional both ways: a Pack that predates effects sends none, and a
    /// host that predates them ignores the field.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<CommandEffectWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenery: Option<SceneryWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub moves: Vec<GaugeMoveWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asker: Option<SelectionIdWire>,
    /// Optional both ways: the question this choice answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<QuestionWire>,
    /// Optional both ways: why this cannot be chosen now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<String>,
    /// Optional both ways: something done with the player's own hands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand: Option<HandWire>,
    /// Optional both ways: the World this choice starts, as it first stands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<PreviewWire>,
}

/// How a World a choice would start first stands.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PreviewWire {
    #[serde(default)]
    pub canvas: CanvasProjectionWire,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drawings: Vec<DrawingWire>,
}

/// Something the player does in the place with their own hands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HandWire {
    pub verb: String,
    pub thing: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<SelectionIdWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
}

/// A question several choices answer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuestionWire {
    pub id: String,
    pub prompt: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToneWire {
    #[default]
    Neutral,
    Good,
    Warning,
    Bad,
    /// A tone from a newer Pack (v8): read as neutral.
    #[serde(other)]
    Unknown,
}

impl From<Tone> for ToneWire {
    fn from(tone: Tone) -> Self {
        match tone {
            Tone::Neutral => Self::Neutral,
            Tone::Good => Self::Good,
            Tone::Warning => Self::Warning,
            Tone::Bad => Self::Bad,
        }
    }
}

impl From<ToneWire> for Tone {
    fn from(tone: ToneWire) -> Self {
        match tone {
            ToneWire::Neutral | ToneWire::Unknown => Self::Neutral,
            ToneWire::Good => Self::Good,
            ToneWire::Warning => Self::Warning,
            ToneWire::Bad => Self::Bad,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum EffectChangeWire {
    Up,
    Down,
    To(String),
    /// A change from a newer Pack (v8): the effect is left out.
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommandEffectWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<SelectionIdWire>,
    pub label: String,
    pub change: EffectChangeWire,
    #[serde(default)]
    pub tone: ToneWire,
}

impl From<&CommandEffect> for CommandEffectWire {
    fn from(effect: &CommandEffect) -> Self {
        Self {
            target: effect.target.map(Into::into),
            label: effect.label.clone(),
            change: match &effect.change {
                EffectChange::Up => EffectChangeWire::Up,
                EffectChange::Down => EffectChangeWire::Down,
                EffectChange::To(value) => EffectChangeWire::To(value.clone()),
            },
            tone: effect.tone.into(),
        }
    }
}

impl From<CommandEffectWire> for CommandEffect {
    fn from(effect: CommandEffectWire) -> Self {
        Self {
            target: effect.target.map(Into::into),
            label: effect.label,
            change: match effect.change {
                EffectChangeWire::Up => EffectChange::Up,
                EffectChangeWire::Down => EffectChange::Down,
                EffectChangeWire::To(value) => EffectChange::To(value),
                // Left out of a command's effects before it gets here.
                EffectChangeWire::Unknown => EffectChange::To(String::new()),
            },
            tone: effect.tone.into(),
        }
    }
}

impl From<&ProjectionCommand> for ProjectionCommandWire {
    fn from(command: &ProjectionCommand) -> Self {
        Self {
            id: command.id.clone(),
            title: command.title.clone(),
            detail: command.detail.clone(),
            effects: command.effects.iter().map(Into::into).collect(),
            scenery: command.scenery.map(Into::into),
            moves: command
                .moves
                .iter()
                .map(|step| GaugeMoveWire {
                    gauge: step.gauge.clone(),
                    by: step.by,
                })
                .collect(),
            asker: command.asker.map(Into::into),
            question: command.question.as_ref().map(|question| QuestionWire {
                id: question.id.clone(),
                prompt: question.prompt.clone(),
            }),
            unavailable: command.unavailable.clone(),
            hand: command.hand.as_ref().map(|hand| HandWire {
                verb: hand.verb.clone(),
                thing: hand.thing.clone(),
                at: hand.at.map(Into::into),
                cost: hand.cost.clone(),
            }),
            preview: command.preview.as_ref().map(|preview| PreviewWire {
                canvas: (&preview.canvas).into(),
                drawings: preview.drawings.iter().map(Into::into).collect(),
            }),
        }
    }
}

impl From<ProjectionCommandWire> for ProjectionCommand {
    fn from(command: ProjectionCommandWire) -> Self {
        Self {
            id: command.id,
            title: command.title,
            detail: command.detail,
            effects: command
                .effects
                .into_iter()
                .filter(|effect| effect.change != EffectChangeWire::Unknown)
                .map(Into::into)
                .collect(),
            scenery: command.scenery.map(Into::into),
            asker: command.asker.map(Into::into),
            // A question with no id or nothing asked is no question: the
            // choice stands on its own.
            question: command
                .question
                .filter(|question| {
                    !question.id.trim().is_empty() && !question.prompt.trim().is_empty()
                })
                .map(|question| world_projection::Question {
                    id: question.id,
                    prompt: question.prompt,
                }),
            // An empty reason still means it cannot be chosen.
            unavailable: command.unavailable.map(|reason| reason.trim().to_string()),
            // A deed with no verb is no deed: the choice stands on its own.
            hand: command
                .hand
                .filter(|hand| !hand.verb.trim().is_empty())
                .map(|hand| world_projection::Hand {
                    verb: hand.verb,
                    thing: hand.thing,
                    at: hand.at.map(Into::into),
                    cost: hand.cost,
                }),
            preview: command.preview.map(|preview| {
                Box::new(world_projection::Preview {
                    canvas: preview.canvas.into(),
                    drawings: preview
                        .drawings
                        .into_iter()
                        .map(Drawing::from)
                        .filter(Drawing::is_drawable)
                        .take(MOST_DRAWINGS)
                        .collect(),
                })
            }),
            moves: command
                .moves
                .into_iter()
                .map(|step| world_projection::GaugeMove {
                    gauge: step.gauge,
                    by: step.by.clamp(-1000, 1000),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BriefingProjectionWire {
    pub eyebrow: String,
    pub title: String,
    pub items: Vec<BriefingItemWire>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub returned: bool,
}

impl From<&BriefingProjection> for BriefingProjectionWire {
    fn from(briefing: &BriefingProjection) -> Self {
        Self {
            eyebrow: briefing.eyebrow.clone(),
            title: briefing.title.clone(),
            items: briefing.items.iter().map(Into::into).collect(),
            returned: briefing.returned,
        }
    }
}

impl From<BriefingProjectionWire> for BriefingProjection {
    fn from(briefing: BriefingProjectionWire) -> Self {
        Self {
            eyebrow: briefing.eyebrow,
            title: briefing.title,
            items: briefing.items.into_iter().map(Into::into).collect(),
            returned: briefing.returned,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BriefingItemWire {
    pub selection: Option<SelectionIdWire>,
    pub title: String,
    pub detail: String,
    /// Absent in snapshots written before briefings distinguished news from
    /// counters; those lines were all presented as news, so that is what they
    /// decode back to.
    #[serde(default)]
    pub kind: BriefingItemKindWire,
    /// Absent before briefings carried a tone; those lines read as neutral.
    #[serde(default, skip_serializing_if = "is_neutral")]
    pub tone: ToneWire,
}

fn is_neutral(tone: &ToneWire) -> bool {
    *tone == ToneWire::Neutral
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BriefingItemKindWire {
    #[default]
    Beat,
    Status,
    /// A kind from a newer Pack (v8): read as a beat.
    #[serde(other)]
    Unknown,
}

impl From<BriefingItemKind> for BriefingItemKindWire {
    fn from(kind: BriefingItemKind) -> Self {
        match kind {
            BriefingItemKind::Beat => Self::Beat,
            BriefingItemKind::Status => Self::Status,
        }
    }
}

impl From<BriefingItemKindWire> for BriefingItemKind {
    fn from(kind: BriefingItemKindWire) -> Self {
        match kind {
            BriefingItemKindWire::Beat | BriefingItemKindWire::Unknown => Self::Beat,
            BriefingItemKindWire::Status => Self::Status,
        }
    }
}

impl From<&BriefingItem> for BriefingItemWire {
    fn from(item: &BriefingItem) -> Self {
        Self {
            selection: item.selection.map(Into::into),
            title: item.title.clone(),
            detail: item.detail.clone(),
            kind: item.kind.into(),
            tone: item.tone.into(),
        }
    }
}

impl From<BriefingItemWire> for BriefingItem {
    fn from(item: BriefingItemWire) -> Self {
        Self {
            selection: item.selection.map(Into::into),
            title: item.title,
            detail: item.detail,
            kind: item.kind.into(),
            tone: item.tone.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CollectionProjectionWire {
    pub title: String,
    pub items: Vec<CollectionItemWire>,
}

impl From<&CollectionProjection> for CollectionProjectionWire {
    fn from(collection: &CollectionProjection) -> Self {
        Self {
            title: collection.title.clone(),
            items: collection.items.iter().map(Into::into).collect(),
        }
    }
}

impl From<CollectionProjectionWire> for CollectionProjection {
    fn from(collection: CollectionProjectionWire) -> Self {
        Self {
            title: collection.title,
            items: collection.items.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollectionItemWire {
    pub id: SelectionIdWire,
    pub title: String,
    pub subtitle: String,
}

impl From<&CollectionItem> for CollectionItemWire {
    fn from(item: &CollectionItem) -> Self {
        Self {
            id: item.id.into(),
            title: item.title.clone(),
            subtitle: item.subtitle.clone(),
        }
    }
}

impl From<CollectionItemWire> for CollectionItem {
    fn from(item: CollectionItemWire) -> Self {
        Self {
            id: item.id.into(),
            title: item.title,
            subtitle: item.subtitle,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TimelineProjectionWire {
    pub items: Vec<TimelineItemWire>,
}

impl From<&TimelineProjection> for TimelineProjectionWire {
    fn from(timeline: &TimelineProjection) -> Self {
        Self {
            items: timeline.items.iter().map(Into::into).collect(),
        }
    }
}

impl From<TimelineProjectionWire> for TimelineProjection {
    fn from(timeline: TimelineProjectionWire) -> Self {
        Self {
            items: timeline.items.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimelineItemWire {
    pub id: SelectionIdWire,
    pub world_time: u64,
    pub title: String,
    pub subtitle: String,
    pub caused_by: Vec<u64>,
    /// Optional in both directions, like the other presentation hints.
    #[serde(default, skip_serializing_if = "is_false")]
    pub routine: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl From<&TimelineItem> for TimelineItemWire {
    fn from(item: &TimelineItem) -> Self {
        Self {
            id: item.id.into(),
            world_time: item.world_time,
            title: item.title.clone(),
            subtitle: item.subtitle.clone(),
            caused_by: item.caused_by.iter().map(|event| event.0).collect(),
            routine: item.routine,
        }
    }
}

impl From<TimelineItemWire> for TimelineItem {
    fn from(item: TimelineItemWire) -> Self {
        Self {
            id: item.id.into(),
            world_time: item.world_time,
            title: item.title,
            subtitle: item.subtitle,
            caused_by: item.caused_by.into_iter().map(EventId::new).collect(),
            routine: item.routine,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CanvasProjectionWire {
    pub items: Vec<CanvasItemWire>,
    /// Optional in both directions: a Pack that predates links sends none,
    /// and a host that predates them ignores the field.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<CanvasLinkWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<CanvasMarkWire>,
    /// Optional both ways, like everything below: how wide the panorama
    /// is, in screen-widths. An older Pack sends none, and the place is one
    /// screen wide.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub districts: Vec<DistrictWire>,
    /// A season this build does not know reads as none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<SeasonWire>,
    /// A ground cover this build does not know reads as none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground: Option<GroundCoverWire>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ice: bool,
    /// Plots the player can build on; an older Pack sends none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plots: Vec<PlotWire>,
    /// What kind of place it is to look at; an older Pack sends none, and
    /// a setting the app does not know is drawn as none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
}

/// The widest panorama a snapshot may ask for, in screen-widths.
pub const MOST_CANVAS_WIDTH: f32 = 16.0;
/// The most districts one panorama is cut into.
pub const MOST_DISTRICTS: usize = 32;
/// The most stops one person's day holds.
pub const MOST_ROUTINE_STOPS: usize = 24;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DistrictWire {
    pub id: String,
    pub label: String,
    pub from: f32,
    pub to: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeasonWire {
    Spring,
    Summer,
    Autumn,
    Winter,
    #[serde(other)]
    Unknown,
}

impl From<Season> for SeasonWire {
    fn from(season: Season) -> Self {
        match season {
            Season::Spring => Self::Spring,
            Season::Summer => Self::Summer,
            Season::Autumn => Self::Autumn,
            Season::Winter => Self::Winter,
        }
    }
}

impl SeasonWire {
    fn known(self) -> Option<Season> {
        match self {
            Self::Spring => Some(Season::Spring),
            Self::Summer => Some(Season::Summer),
            Self::Autumn => Some(Season::Autumn),
            Self::Winter => Some(Season::Winter),
            Self::Unknown => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundCoverWire {
    Blossom,
    Leaves,
    Snow,
    Frost,
    Dust,
    #[serde(other)]
    Unknown,
}

impl From<GroundCover> for GroundCoverWire {
    fn from(ground: GroundCover) -> Self {
        match ground {
            GroundCover::Blossom => Self::Blossom,
            GroundCover::Leaves => Self::Leaves,
            GroundCover::Snow => Self::Snow,
            GroundCover::Frost => Self::Frost,
            GroundCover::Dust => Self::Dust,
        }
    }
}

impl GroundCoverWire {
    fn known(self) -> Option<GroundCover> {
        match self {
            Self::Blossom => Some(GroundCover::Blossom),
            Self::Leaves => Some(GroundCover::Leaves),
            Self::Snow => Some(GroundCover::Snow),
            Self::Frost => Some(GroundCover::Frost),
            Self::Dust => Some(GroundCover::Dust),
            Self::Unknown => None,
        }
    }
}

/// Where a person is from one hour of the day on, as it crosses the
/// boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutineStopWire {
    pub from_hour: u8,
    pub at: SelectionIdWire,
    #[serde(default)]
    pub inside: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanvasMarkWire {
    pub label: String,
    #[serde(default)]
    pub shape: MarkShapeWire,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<SelectionIdWire>,
}

/// Unknown shapes from a newer Pack read as the default rather than
/// failing the whole snapshot.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkShapeWire {
    #[default]
    House,
    Dome,
    Tower,
    Tree,
    Lamp,
    Shop,
    Bridge,
    Rover,
    Boat,
    Parcel,
    Stall,
    Bunting,
    Pier,
    Garden,
    Flag,
    Lantern,
    Tent,
    Bench,
    Sprouts,
    Well,
    Swing,
    Fountain,
    Signpost,
    Birdhouse,
    Planter,
    Statue,
    Postbox,
    #[serde(other)]
    Unknown,
}

impl From<MarkShape> for MarkShapeWire {
    fn from(shape: MarkShape) -> Self {
        match shape {
            MarkShape::House => Self::House,
            MarkShape::Dome => Self::Dome,
            MarkShape::Tower => Self::Tower,
            MarkShape::Tree => Self::Tree,
            MarkShape::Lamp => Self::Lamp,
            MarkShape::Shop => Self::Shop,
            MarkShape::Bridge => Self::Bridge,
            MarkShape::Rover => Self::Rover,
            MarkShape::Boat => Self::Boat,
            MarkShape::Parcel => Self::Parcel,
            MarkShape::Stall => Self::Stall,
            MarkShape::Bunting => Self::Bunting,
            MarkShape::Pier => Self::Pier,
            MarkShape::Garden => Self::Garden,
            MarkShape::Flag => Self::Flag,
            MarkShape::Lantern => Self::Lantern,
            MarkShape::Tent => Self::Tent,
            MarkShape::Bench => Self::Bench,
            MarkShape::Sprouts => Self::Sprouts,
            MarkShape::Well => Self::Well,
            MarkShape::Swing => Self::Swing,
            MarkShape::Fountain => Self::Fountain,
            MarkShape::Signpost => Self::Signpost,
            MarkShape::Birdhouse => Self::Birdhouse,
            MarkShape::Planter => Self::Planter,
            MarkShape::Statue => Self::Statue,
            MarkShape::Postbox => Self::Postbox,
        }
    }
}

impl From<MarkShapeWire> for MarkShape {
    fn from(shape: MarkShapeWire) -> Self {
        match shape {
            MarkShapeWire::House | MarkShapeWire::Unknown => Self::House,
            MarkShapeWire::Dome => Self::Dome,
            MarkShapeWire::Tower => Self::Tower,
            MarkShapeWire::Tree => Self::Tree,
            MarkShapeWire::Lamp => Self::Lamp,
            MarkShapeWire::Shop => Self::Shop,
            MarkShapeWire::Bridge => Self::Bridge,
            MarkShapeWire::Rover => Self::Rover,
            MarkShapeWire::Boat => Self::Boat,
            MarkShapeWire::Parcel => Self::Parcel,
            MarkShapeWire::Stall => Self::Stall,
            MarkShapeWire::Bunting => Self::Bunting,
            MarkShapeWire::Pier => Self::Pier,
            MarkShapeWire::Garden => Self::Garden,
            MarkShapeWire::Flag => Self::Flag,
            MarkShapeWire::Lantern => Self::Lantern,
            MarkShapeWire::Tent => Self::Tent,
            MarkShapeWire::Bench => Self::Bench,
            MarkShapeWire::Sprouts => Self::Sprouts,
            MarkShapeWire::Well => Self::Well,
            MarkShapeWire::Swing => Self::Swing,
            MarkShapeWire::Fountain => Self::Fountain,
            MarkShapeWire::Signpost => Self::Signpost,
            MarkShapeWire::Birdhouse => Self::Birdhouse,
            MarkShapeWire::Planter => Self::Planter,
            MarkShapeWire::Statue => Self::Statue,
            MarkShapeWire::Postbox => Self::Postbox,
        }
    }
}

impl From<&CanvasProjection> for CanvasProjectionWire {
    fn from(canvas: &CanvasProjection) -> Self {
        Self {
            items: canvas.items.iter().map(Into::into).collect(),
            links: canvas.links.iter().map(Into::into).collect(),
            marks: canvas
                .marks
                .iter()
                .map(|mark| CanvasMarkWire {
                    label: mark.label.clone(),
                    shape: mark.shape.into(),
                    selection: mark.selection.map(Into::into),
                })
                .collect(),
            width: canvas.width,
            districts: canvas
                .districts
                .iter()
                .map(|district| DistrictWire {
                    id: district.id.clone(),
                    label: district.label.clone(),
                    from: district.from,
                    to: district.to,
                })
                .collect(),
            season: canvas.season.map(Into::into),
            ground: canvas.ground.map(Into::into),
            ice: canvas.ice,
            plots: canvas.plots.iter().map(Into::into).collect(),
            setting: canvas.setting.clone(),
        }
    }
}

impl From<CanvasProjectionWire> for CanvasProjection {
    fn from(canvas: CanvasProjectionWire) -> Self {
        Self {
            items: canvas.items.into_iter().map(Into::into).collect(),
            links: canvas.links.into_iter().map(Into::into).collect(),
            marks: canvas
                .marks
                .into_iter()
                .map(|mark| CanvasMark {
                    label: mark.label,
                    shape: mark.shape.into(),
                    selection: mark.selection.map(Into::into),
                })
                .collect(),
            // A panorama is at least one screen and at most a few dozen;
            // anything else is one screen.
            width: canvas
                .width
                .filter(|width| width.is_finite())
                .map(|width| width.clamp(1.0, MOST_CANVAS_WIDTH)),
            districts: canvas
                .districts
                .into_iter()
                .filter(|district| {
                    district.from.is_finite()
                        && district.to.is_finite()
                        && district.from < district.to
                        && !district.id.trim().is_empty()
                })
                .take(MOST_DISTRICTS)
                .map(|district| world_projection::District {
                    id: district.id,
                    label: district.label,
                    from: district.from,
                    to: district.to,
                })
                .collect(),
            season: canvas.season.and_then(SeasonWire::known),
            ground: canvas.ground.and_then(GroundCoverWire::known),
            ice: canvas.ice,
            plots: canvas
                .plots
                .into_iter()
                .filter_map(PlotWire::known)
                .take(MOST_PLOTS)
                .collect(),
            setting: art_key(canvas.setting),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanvasLinkToneWire {
    #[default]
    Neutral,
    Warm,
    Strained,
    /// A tone from a newer Pack (v8): read as neutral.
    #[serde(other)]
    Unknown,
}

impl From<CanvasLinkTone> for CanvasLinkToneWire {
    fn from(tone: CanvasLinkTone) -> Self {
        match tone {
            CanvasLinkTone::Neutral => Self::Neutral,
            CanvasLinkTone::Warm => Self::Warm,
            CanvasLinkTone::Strained => Self::Strained,
        }
    }
}

impl From<CanvasLinkToneWire> for CanvasLinkTone {
    fn from(tone: CanvasLinkToneWire) -> Self {
        match tone {
            CanvasLinkToneWire::Neutral | CanvasLinkToneWire::Unknown => Self::Neutral,
            CanvasLinkToneWire::Warm => Self::Warm,
            CanvasLinkToneWire::Strained => Self::Strained,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanvasLinkWire {
    pub from: SelectionIdWire,
    pub to: SelectionIdWire,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub tone: CanvasLinkToneWire,
    #[serde(default)]
    pub strength: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<SelectionIdWire>,
}

impl From<&CanvasLink> for CanvasLinkWire {
    fn from(link: &CanvasLink) -> Self {
        Self {
            from: link.from.into(),
            to: link.to.into(),
            label: link.label.clone(),
            tone: link.tone.into(),
            strength: link.strength,
            selection: link.selection.map(Into::into),
        }
    }
}

impl From<CanvasLinkWire> for CanvasLink {
    fn from(link: CanvasLinkWire) -> Self {
        Self {
            from: link.from.into(),
            to: link.to.into(),
            label: link.label,
            tone: link.tone.into(),
            strength: link.strength.clamp(0.0, 1.0),
            selection: link.selection.map(Into::into),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanvasItemKindWire {
    Place,
    Actor,
    Object,
    /// A kind from a newer Pack (v8): drawn as a thing.
    #[serde(other)]
    Unknown,
}

impl From<CanvasItemKind> for CanvasItemKindWire {
    fn from(kind: CanvasItemKind) -> Self {
        match kind {
            CanvasItemKind::Place => Self::Place,
            CanvasItemKind::Actor => Self::Actor,
            CanvasItemKind::Object => Self::Object,
        }
    }
}

impl From<CanvasItemKindWire> for CanvasItemKind {
    fn from(kind: CanvasItemKindWire) -> Self {
        match kind {
            CanvasItemKindWire::Place => Self::Place,
            CanvasItemKindWire::Actor => Self::Actor,
            CanvasItemKindWire::Object | CanvasItemKindWire::Unknown => Self::Object,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanvasItemWire {
    pub id: SelectionIdWire,
    pub kind: CanvasItemKindWire,
    pub label: String,
    pub detail: String,
    pub x: f32,
    pub y: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<CanvasChangeWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<MarkShapeWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<SelectionIdWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<LookWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawing: Option<String>,
    /// A stance this build does not know is drawn standing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing: Option<StandingWire>,
    /// A mood this build does not know is drawn content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
    /// Where along the ground a thing stands (0 to 1), when the player
    /// chose; an older Pack sends none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spot: Option<f32>,
    /// Where along the panorama it stands; an older Pack sends none, and
    /// `x` places it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub px: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<SelectionIdWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub day: Vec<RoutineStopWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built: Option<u32>,
    /// A design painted on it; one not of the fixed palette reads as none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<PatternWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design: Option<DesignableWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub naming: Option<NamingWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<VariantWire>,
    /// What the app draws it as; an older Pack sends none, and a key the
    /// app does not know is drawn by its shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub art: Option<String>,
}

/// The longest name of a drawing in the app's own library, or of a setting.
pub const MOST_ART_KEY: usize = 48;

/// An art key or a setting, if it is one: a short name of lower-case
/// letters, digits and dashes. Anything else reads as none.
pub fn art_key(key: Option<String>) -> Option<String> {
    key.filter(|key| {
        !key.is_empty()
            && key.len() <= MOST_ART_KEY
            && key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    })
}

/// The longest few words a standing is told in.
pub const MOST_STANDING_WORDS: usize = 60;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StandingWire {
    pub level: i8,
    pub words: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CanvasChangeWire {
    pub label: String,
    pub before: String,
    pub after: String,
    #[serde(default)]
    pub tone: ToneWire,
}

impl From<&CanvasItem> for CanvasItemWire {
    fn from(item: &CanvasItem) -> Self {
        Self {
            id: item.id.into(),
            kind: item.kind.into(),
            label: item.label.clone(),
            detail: item.detail.clone(),
            x: item.x,
            y: item.y,
            changes: item
                .changes
                .iter()
                .map(|change| CanvasChangeWire {
                    label: change.label.clone(),
                    before: change.before.clone(),
                    after: change.after.clone(),
                    tone: change.tone.into(),
                })
                .collect(),
            shape: item.shape.map(Into::into),
            at: item.at.map(Into::into),
            look: item.look.map(Into::into),
            drawing: item.drawing.clone(),
            stance: item.stance.map(|stance| stance.id().to_string()),
            standing: item.standing.as_ref().map(|standing| StandingWire {
                level: standing.level,
                words: standing.words.clone(),
            }),
            mood: item.mood.map(|mood| mood.id().to_string()),
            spot: item.spot,
            px: item.px,
            home: item.home.map(Into::into),
            day: item
                .day
                .iter()
                .map(|stop| RoutineStopWire {
                    from_hour: stop.from_hour,
                    at: stop.at.into(),
                    inside: stop.inside,
                })
                .collect(),
            built: item.built,
            pattern: item.pattern.as_ref().map(Into::into),
            design: item.design.as_ref().map(Into::into),
            naming: item.naming.as_ref().map(Into::into),
            variant: item.variant.map(Into::into),
            art: item.art.clone(),
        }
    }
}

impl From<CanvasItemWire> for CanvasItem {
    fn from(item: CanvasItemWire) -> Self {
        Self {
            id: item.id.into(),
            kind: item.kind.into(),
            label: item.label,
            detail: item.detail,
            x: item.x,
            y: item.y,
            changes: item
                .changes
                .into_iter()
                .map(|change| CanvasChange {
                    label: change.label,
                    before: change.before,
                    after: change.after,
                    tone: change.tone.into(),
                })
                .collect(),
            shape: item.shape.map(Into::into),
            at: item.at.map(Into::into),
            look: item.look.map(Into::into),
            drawing: item.drawing.filter(|drawing| !drawing.trim().is_empty()),
            stance: item
                .stance
                .map(|stance| Stance::from_id(&stance).unwrap_or_default()),
            // A standing is only ever a mark and a few plain words.
            standing: item
                .standing
                .filter(|standing| {
                    let words = standing.words.trim();
                    !words.is_empty()
                        && words.chars().count() <= MOST_STANDING_WORDS
                        && !words.chars().any(char::is_control)
                })
                .map(|standing| world_projection::Standing {
                    level: standing.level.clamp(-2, 2),
                    words: standing.words.trim().to_string(),
                }),
            mood: item
                .mood
                .map(|mood| world_projection::Mood::from_id(&mood).unwrap_or_default()),
            spot: item
                .spot
                .filter(|spot| spot.is_finite())
                .map(|spot| spot.clamp(0.0, 1.0)),
            px: item
                .px
                .filter(|px| px.is_finite())
                .map(|px| px.clamp(0.0, MOST_CANVAS_WIDTH)),
            home: item.home.map(Into::into),
            // A day is a few stops within the hours of one day, in order.
            day: {
                let mut day: Vec<world_projection::RoutineStop> = item
                    .day
                    .into_iter()
                    .filter(|stop| stop.from_hour < 24)
                    .take(MOST_ROUTINE_STOPS)
                    .map(|stop| world_projection::RoutineStop {
                        from_hour: stop.from_hour,
                        at: stop.at.into(),
                        inside: stop.inside,
                    })
                    .collect();
                day.sort_by_key(|stop| stop.from_hour);
                day
            },
            built: item.built,
            pattern: item.pattern.and_then(PatternWire::known),
            design: item.design.and_then(DesignableWire::known),
            naming: item.naming.and_then(NamingWire::known),
            variant: item.variant.map(Into::into),
            art: art_key(item.art),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InspectorProjectionWire {
    pub selection: SelectionIdWire,
    pub title: String,
    pub subtitle: String,
    pub sections: Vec<InspectorSectionWire>,
}

impl From<&InspectorProjection> for InspectorProjectionWire {
    fn from(inspector: &InspectorProjection) -> Self {
        Self {
            selection: inspector.selection.into(),
            title: inspector.title.clone(),
            subtitle: inspector.subtitle.clone(),
            sections: inspector.sections.iter().map(Into::into).collect(),
        }
    }
}

impl From<InspectorProjectionWire> for InspectorProjection {
    fn from(inspector: InspectorProjectionWire) -> Self {
        Self {
            selection: inspector.selection.into(),
            title: inspector.title,
            subtitle: inspector.subtitle,
            sections: inspector.sections.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InspectorSectionWire {
    pub title: String,
    pub rows: Vec<InspectorRowWire>,
}

impl From<&InspectorSection> for InspectorSectionWire {
    fn from(section: &InspectorSection) -> Self {
        Self {
            title: section.title.clone(),
            rows: section.rows.iter().map(Into::into).collect(),
        }
    }
}

impl From<InspectorSectionWire> for InspectorSection {
    fn from(section: InspectorSectionWire) -> Self {
        Self {
            title: section.title,
            rows: section.rows.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InspectorRowWire {
    pub label: String,
    pub value: String,
}

impl From<&InspectorRow> for InspectorRowWire {
    fn from(row: &InspectorRow) -> Self {
        Self {
            label: row.label.clone(),
            value: row.value.clone(),
        }
    }
}

impl From<InspectorRowWire> for InspectorRow {
    fn from(row: InspectorRowWire) -> Self {
        Self {
            label: row.label,
            value: row.value,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WhyProjectionWire {
    pub event: u64,
    pub nodes: Vec<WhyNodeWire>,
}

impl From<&WhyProjection> for WhyProjectionWire {
    fn from(why: &WhyProjection) -> Self {
        Self {
            event: why.event.0,
            nodes: why.nodes.iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<WhyProjectionWire> for WhyProjection {
    type Error = ProtocolError;

    fn try_from(why: WhyProjectionWire) -> Result<Self, Self::Error> {
        Ok(Self {
            event: EventId::new(why.event),
            nodes: why
                .nodes
                .into_iter()
                .map(WhyNode::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WhyNodeWire {
    pub event: u64,
    pub depth: u64,
    pub world_time: u64,
    pub title: String,
    pub subtitle: String,
    pub caused_by: Vec<u64>,
}

impl From<&WhyNode> for WhyNodeWire {
    fn from(node: &WhyNode) -> Self {
        Self {
            event: node.event.0,
            depth: node.depth as u64,
            world_time: node.world_time,
            title: node.title.clone(),
            subtitle: node.subtitle.clone(),
            caused_by: node.caused_by.iter().map(|event| event.0).collect(),
        }
    }
}

impl TryFrom<WhyNodeWire> for WhyNode {
    type Error = ProtocolError;

    fn try_from(node: WhyNodeWire) -> Result<Self, Self::Error> {
        let depth =
            usize::try_from(node.depth).map_err(|_| ProtocolError::DepthOverflow(node.depth))?;
        Ok(Self {
            event: EventId::new(node.event),
            depth,
            world_time: node.world_time,
            title: node.title,
            subtitle: node.subtitle,
            caused_by: node.caused_by.into_iter().map(EventId::new).collect(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    UnsupportedManifestFormat(String),
    UnsupportedManifestVersion(u32),
    UnsupportedProtocolVersion(u32),
    InvalidPack,
    InvalidTitle,
    InvalidProcessCommand,
    DuplicateInspector(String),
    DuplicateWhy(u64),
    SelectionNotSupportedInProtocol {
        protocol_version: u32,
        selection: String,
    },
    RequestNotSupportedInProtocol {
        protocol_version: u32,
        request: &'static str,
    },
    DepthOverflow(u64),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedManifestFormat(format) => {
                write!(f, "unsupported Pack manifest format: {format}")
            }
            Self::UnsupportedManifestVersion(version) => {
                write!(f, "unsupported Pack manifest version: {version}")
            }
            Self::UnsupportedProtocolVersion(version) => {
                write!(f, "unsupported Pack protocol version: {version}")
            }
            Self::InvalidPack => write!(f, "Pack id and version must be non-empty"),
            Self::InvalidTitle => write!(f, "Pack title must be non-empty"),
            Self::InvalidProcessCommand => write!(f, "Pack process command must be non-empty"),
            Self::DuplicateInspector(selection) => {
                write!(f, "Pack snapshot contains duplicate inspector: {selection}")
            }
            Self::DuplicateWhy(event) => {
                write!(
                    f,
                    "Pack snapshot contains duplicate why projection for event {event}"
                )
            }
            Self::SelectionNotSupportedInProtocol {
                protocol_version,
                selection,
            } => write!(
                f,
                "selection {selection} is not supported by Pack protocol v{protocol_version}"
            ),
            Self::RequestNotSupportedInProtocol {
                protocol_version,
                request,
            } => write!(
                f,
                "request {request} is not supported by Pack protocol v{protocol_version}"
            ),
            Self::DepthOverflow(depth) => {
                write!(f, "Pack why-node depth does not fit this platform: {depth}")
            }
        }
    }
}

impl Error for ProtocolError {}

#[derive(Debug)]
pub enum ManifestDecodeError {
    Json(serde_json::Error),
    Protocol(ProtocolError),
}

impl fmt::Display for ManifestDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "invalid Pack manifest JSON: {error}"),
            Self::Protocol(error) => error.fmt(f),
        }
    }
}

impl Error for ManifestDecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Protocol(error) => Some(error),
        }
    }
}

#[derive(Debug)]
pub enum ProtocolDecodeError {
    Json(serde_json::Error),
    Protocol(ProtocolError),
}

impl fmt::Display for ProtocolDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "invalid Pack protocol JSON: {error}"),
            Self::Protocol(error) => error.fmt(f),
        }
    }
}

impl Error for ProtocolDecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Protocol(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_projection::{
        BriefingItem, BriefingProjection, CanvasItem, CanvasItemKind, CanvasLink, CanvasLinkTone,
        CanvasProjection, CollectionItem, CollectionProjection, CommandEffect, EffectChange,
        InspectorProjection, InspectorRow, InspectorSection, ProjectionCapabilities,
        ProjectionCommand, TimelineItem, TimelineProjection, Tone, WhyNode, WhyProjection,
    };

    fn descriptor() -> PackDescriptor {
        PackDescriptor::new(
            WorldPackRef::new("example.external-world", "1"),
            "External World",
            "Protocol fixture",
        )
    }

    fn sample_snapshot() -> ProjectionSnapshot {
        let entity = SelectionId::Entity(EntityId::new(7));
        let event = SelectionId::Event(EventId::new(9));
        ProjectionSnapshot {
            title: "External World".into(),
            world_time: 42,
            capabilities: ProjectionCapabilities {
                fork: true,
                background: false,
                talk: false,
            },
            briefing: Some(BriefingProjection {
                eyebrow: "Status".into(),
                title: "World briefing".into(),
                items: vec![BriefingItem {
                    kind: BriefingItemKind::Status,
                    selection: Some(entity),
                    title: "Entity seven".into(),
                    detail: "A selected entity".into(),
                    tone: Tone::Warning,
                }],
                returned: false,
            }),
            commands: vec![ProjectionCommand {
                id: "external.advance".into(),
                title: "Advance".into(),
                detail: "Advance the external world".into(),
                effects: vec![
                    CommandEffect {
                        target: Some(entity),
                        label: "Seven".into(),
                        change: EffectChange::To("advanced".into()),
                        tone: Tone::Good,
                    },
                    CommandEffect {
                        target: None,
                        label: "Time".into(),
                        change: EffectChange::Up,
                        tone: Tone::Neutral,
                    },
                ],
                scenery: None,
                asker: None,
                moves: Vec::new(),
                question: None,
                unavailable: None,
                hand: None,
                preview: None,
            }],
            collection: CollectionProjection {
                title: "Entities".into(),
                items: vec![CollectionItem {
                    id: entity,
                    title: "Seven".into(),
                    subtitle: "Actor".into(),
                }],
            },
            timeline: TimelineProjection {
                items: vec![TimelineItem {
                    id: event,
                    world_time: 41,
                    title: "Changed".into(),
                    subtitle: "Event nine".into(),
                    caused_by: vec![EventId::new(8)],
                    routine: false,
                }],
            },
            canvas: CanvasProjection {
                items: vec![CanvasItem {
                    id: entity,
                    kind: CanvasItemKind::Actor,
                    label: "Seven".into(),
                    detail: "On the canvas".into(),
                    x: 0.25,
                    y: 0.75,
                    changes: Vec::new(),
                    shape: None,
                    at: None,
                    look: None,
                    drawing: None,
                    stance: None,
                    standing: None,
                    mood: None,
                    spot: None,
                    px: None,
                    home: None,
                    day: Vec::new(),
                    built: None,
                    ..Default::default()
                }],
                links: vec![CanvasLink {
                    from: entity,
                    to: entity,
                    label: "Partnership".into(),
                    tone: CanvasLinkTone::Warm,
                    strength: 0.8,
                    selection: Some(entity),
                }],
                marks: Vec::new(),
                ..CanvasProjection::default()
            },
            inspectors: BTreeMap::from([(
                entity,
                InspectorProjection {
                    selection: entity,
                    title: "Seven".into(),
                    subtitle: "Actor".into(),
                    sections: vec![InspectorSection {
                        title: "State".into(),
                        rows: vec![InspectorRow {
                            label: "Mood".into(),
                            value: "Curious".into(),
                        }],
                    }],
                },
            )]),
            why: BTreeMap::from([(
                EventId::new(9),
                WhyProjection {
                    event: EventId::new(9),
                    nodes: vec![WhyNode {
                        event: EventId::new(9),
                        depth: 0,
                        world_time: 41,
                        title: "Changed".into(),
                        subtitle: "Event nine".into(),
                        caused_by: vec![EventId::new(8)],
                    }],
                },
            )]),
            scenery: None,
            calendar: None,
            gauges: Vec::new(),
            talks: Vec::new(),
            voices: Vec::new(),
            chapters: Vec::new(),
            goals: Vec::new(),
            weather: Default::default(),
            exchanges: Vec::new(),
            drawings: Vec::new(),
            keepsakes: Vec::new(),
            letters: Vec::new(),
            book: Vec::new(),
            moments: Vec::new(),
            almanac: None,
            almanac_years: Vec::new(),
            favour: None,
        }
    }

    #[test]
    fn manifest_round_trip_preserves_process_contract() {
        let manifest =
            PackManifest::process(descriptor(), "bin/external-world", vec!["--stdio".into()]);
        manifest.validate().unwrap();
        let json = manifest.to_json_pretty().unwrap();
        let decoded = PackManifest::from_json(&json).unwrap();
        assert_eq!(decoded, manifest);
    }

    #[test]
    fn manifest_rejects_unknown_protocol_or_empty_process_command() {
        let mut manifest = PackManifest::process(descriptor(), "bin/world", Vec::new());
        manifest.protocol_version = PACK_PROTOCOL_VERSION + 1;
        assert!(matches!(
            manifest.validate(),
            Err(ProtocolError::UnsupportedProtocolVersion(_))
        ));

        manifest.protocol_version = PACK_PROTOCOL_VERSION;
        manifest.runtime = PackRuntimeManifest::Process {
            command: "   ".into(),
            args: Vec::new(),
        };
        assert_eq!(
            manifest.validate(),
            Err(ProtocolError::InvalidProcessCommand)
        );
    }

    #[test]
    fn request_and_response_envelopes_are_versioned_and_round_trip() {
        let request = PackRequestEnvelope::new(
            17,
            PackRequest::Handle {
                intent: ProjectionIntentWire::InvokeCommand {
                    command: "external.advance".into(),
                },
            },
        );
        let request_json = encode_request(&request).unwrap();
        assert_eq!(decode_request(&request_json).unwrap(), request);

        let response = PackResponseEnvelope::new(
            17,
            PackResponse::Descriptor {
                descriptor: descriptor(),
            },
        );
        let response_json = encode_response(&response).unwrap();
        assert_eq!(decode_response(&response_json).unwrap(), response);
    }

    #[test]
    fn envelope_rejects_unknown_protocol_version() {
        let mut request = PackRequestEnvelope::new(1, PackRequest::Describe);
        request.protocol_version += 1;
        let json = serde_json::to_string(&request).unwrap();
        assert!(matches!(
            decode_request(&json),
            Err(ProtocolDecodeError::Protocol(
                ProtocolError::UnsupportedProtocolVersion(_)
            ))
        ));
    }

    #[test]
    fn projection_snapshot_wire_round_trip_preserves_all_generic_surfaces() {
        let snapshot = sample_snapshot();
        let wire = ProjectionSnapshotWire::from(&snapshot);
        let json = serde_json::to_string(&wire).unwrap();
        let decoded = serde_json::from_str::<ProjectionSnapshotWire>(&json).unwrap();
        let restored = ProjectionSnapshot::try_from(decoded).unwrap();
        assert_eq!(restored, snapshot);
    }

    #[test]
    fn projection_wire_rejects_duplicate_map_keys() {
        let snapshot = sample_snapshot();
        let mut wire = ProjectionSnapshotWire::from(&snapshot);
        wire.inspectors.push(wire.inspectors[0].clone());
        assert!(matches!(
            ProjectionSnapshot::try_from(wire),
            Err(ProtocolError::DuplicateInspector(_))
        ));

        let snapshot = sample_snapshot();
        let mut wire = ProjectionSnapshotWire::from(&snapshot);
        wire.why.push(wire.why[0].clone());
        assert!(matches!(
            ProjectionSnapshot::try_from(wire),
            Err(ProtocolError::DuplicateWhy(9))
        ));
    }

    #[test]
    fn projection_intent_wire_round_trip_is_lossless() {
        let intents = [
            ProjectionIntent::ForkBeforeEvent(EventId::new(12)),
            ProjectionIntent::InvokeCommand("external.run".into()),
        ];
        for intent in intents {
            let wire = ProjectionIntentWire::from(intent.clone());
            assert_eq!(ProjectionIntent::from(wire), intent);
        }
    }

    #[test]
    fn a_snapshot_from_before_tones_and_effects_still_decodes() {
        let item: BriefingItemWire = serde_json::from_str(
            r#"{"selection":null,"title":"Old news","detail":"","kind":"beat"}"#,
        )
        .expect("an item without a tone decodes");
        assert_eq!(item.tone, ToneWire::Neutral);
        let command: ProjectionCommandWire =
            serde_json::from_str(r#"{"id":"old","title":"Old","detail":""}"#)
                .expect("a command without effects decodes");
        assert!(command.effects.is_empty());
        let encoded = serde_json::to_value(&command).expect("encodes");
        assert!(
            encoded.get("effects").is_none(),
            "an old host never sees the new field when there is nothing to say"
        );
    }

    #[test]
    fn a_place_keeps_its_shape_and_an_unknown_shape_reads_as_a_house() {
        let item: CanvasItemWire = serde_json::from_str(
            r#"{"id":{"type":"entity","id":1},"kind":"place","label":"Icebridge","detail":"","x":0.1,"y":0.2,"shape":"bridge"}"#,
        )
        .expect("a shaped place decodes");
        assert_eq!(item.shape, Some(MarkShapeWire::Bridge));
        assert_eq!(CanvasItem::from(item).shape, Some(MarkShape::Bridge));
        let newer: MarkShapeWire =
            serde_json::from_str(r#""lighthouse""#).expect("a newer shape still decodes");
        assert_eq!(MarkShape::from(newer), MarkShape::House);
    }

    #[test]
    fn what_the_player_says_crosses_the_boundary_as_said() {
        for ears in [
            Ears::World,
            Ears::Model("MEANING: greet\nABOUT: none\nREPLY: Hello!".into()),
            Ears::Own,
        ] {
            let said = ProjectionIntent::Say {
                to: SelectionId::Entity(EntityId::new(7)),
                words: "How's the \"bakery\"? 你好".into(),
                ears,
            };
            let json = serde_json::to_string(&ProjectionIntentWire::from(said.clone())).unwrap();
            let back: ProjectionIntentWire = serde_json::from_str(&json).unwrap();
            assert_eq!(ProjectionIntent::from(back), said);
        }
        // Said by an app that never heard of ears, the World hears it.
        let older: ProjectionIntentWire =
            serde_json::from_str(r#"{"type":"say","to":{"type":"entity","id":7},"words":"hi"}"#)
                .unwrap();
        assert!(matches!(
            ProjectionIntent::from(older),
            ProjectionIntent::Say {
                ears: Ears::World,
                ..
            }
        ));
        // A response too long to be an answer is never read.
        let long = ProjectionIntentWire::Say {
            to: SelectionIdWire::Entity { id: 7 },
            words: "hi".into(),
            ears: EarsWire::Model {
                response: "x".repeat(MOST_MODEL_RESPONSE + 1),
            },
        };
        assert!(matches!(
            ProjectionIntent::from(long),
            ProjectionIntent::Say {
                ears: Ears::Own,
                ..
            }
        ));
    }

    #[test]
    fn a_place_to_begin_crosses_with_its_picture() {
        let person = world_projection::person_base("someone")
            .with("someone", world_projection::short_hair());
        let command = ProjectionCommand {
            id: "begin".into(),
            title: "Start a town".into(),
            detail: String::new(),
            effects: Vec::new(),
            scenery: None,
            moves: Vec::new(),
            asker: None,
            question: None,
            unavailable: None,
            hand: None,
            preview: Some(Box::new(world_projection::Preview {
                canvas: CanvasProjection {
                    items: Vec::new(),
                    links: Vec::new(),
                    marks: vec![CanvasMark {
                        label: "Well".into(),
                        shape: world_projection::MarkShape::Well,
                        selection: None,
                    }],
                    ..CanvasProjection::default()
                },
                drawings: vec![person],
            })),
        };
        let json = serde_json::to_string(&ProjectionCommandWire::from(&command)).unwrap();
        let back =
            ProjectionCommand::from(serde_json::from_str::<ProjectionCommandWire>(&json).unwrap());
        assert_eq!(back, command);
        // From a Pack that never heard of pictures, a choice has none.
        let older: ProjectionCommandWire =
            serde_json::from_str(r#"{"id":"a","title":"A","detail":""}"#).unwrap();
        assert!(ProjectionCommand::from(older).preview.is_none());
    }

    #[test]
    fn drawings_cross_the_boundary_and_one_the_app_cannot_draw_is_dropped() {
        let person = world_projection::person_base("someone")
            .with("someone", world_projection::short_hair());
        let wire = DrawingWire::from(&person);
        let json = serde_json::to_string(&wire).unwrap();
        let back = Drawing::from(serde_json::from_str::<DrawingWire>(&json).unwrap());
        assert_eq!(back, person);

        let mut stray = wire.clone();
        stray.id = "stray".into();
        stray.parts[0].shape = DrawShapeWire::Rect {
            x: 90.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
            round: 0.0,
        };
        let mut unknown = wire.clone();
        unknown.parts[0].ink = "sparkle".into();
        unknown.parts[0].stances = vec!["dancing".into(), "talking".into()];
        let unknown = Drawing::from(unknown);
        assert_eq!(unknown.parts[0].ink, Ink::Wall);
        assert_eq!(unknown.parts[0].stances, vec![Stance::Talking]);
        assert!(!Drawing::from(stray).is_drawable());

        let old: CanvasItemWire = serde_json::from_str(
            r#"{"id":{"type":"entity","id":1},"kind":"actor","label":"Ann","detail":"","x":0.1,"y":0.2,"stance":"juggling"}"#,
        )
        .unwrap();
        assert_eq!(CanvasItem::from(old).stance, Some(Stance::Standing));
    }

    #[test]
    fn moving_on_its_own_is_declared_and_absent_means_it_does_not() {
        let old: ProjectionCapabilitiesWire =
            serde_json::from_str(r#"{"fork":true}"#).expect("an older Pack decodes");
        assert!(!ProjectionCapabilities::from(old).background);
        let live = ProjectionCapabilities {
            fork: false,
            background: true,
            talk: false,
        };
        let wire = ProjectionCapabilitiesWire::from(live);
        assert_eq!(ProjectionCapabilities::from(wire), live);
    }

    #[test]
    fn gauges_and_their_moves_cross_the_boundary_and_bad_values_are_tamed() {
        let snapshot: ProjectionSnapshotWire = serde_json::from_str(
            r#"{"title":"T","world_time":0,"capabilities":{"fork":false},"briefing":null,
                "commands":[{"id":"c","title":"C","detail":"","moves":[{"gauge":"trust","by":5000}]}],
                "collection":{"title":"","items":[]},"timeline":{"items":[]},
                "canvas":{"items":[]},"inspectors":[],"why":[],
                "gauges":[{"id":"trust","label":"Trust","value":7.5,"reading":"high"},
                          {"id":"","label":"Nameless","value":0.5}]}"#,
        )
        .expect("a snapshot with gauges decodes");
        let snapshot = ProjectionSnapshot::try_from(snapshot).expect("valid");
        assert_eq!(snapshot.gauges.len(), 1, "a gauge with no id is dropped");
        assert_eq!(snapshot.gauges[0].value, 1.0);
        assert_eq!(snapshot.commands[0].moves[0].by, 1000);
        let again = ProjectionSnapshot::try_from(ProjectionSnapshotWire::from(&snapshot)).unwrap();
        assert_eq!(again.gauges, snapshot.gauges);
    }

    /// Replaces, anywhere in `value`, a string under `key` with `word`.
    fn rename_all(value: &mut serde_json::Value, key: &str, word: &str) {
        match value {
            serde_json::Value::Object(map) => {
                for (name, inner) in map.iter_mut() {
                    if name == key && inner.is_string() {
                        *inner = serde_json::Value::String(word.into());
                    } else {
                        rename_all(inner, key, word);
                    }
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    rename_all(item, key, word);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn words_from_a_newer_pack_no_longer_fail_a_snapshot() {
        let mut snapshot = sample_snapshot();
        snapshot.drawings = vec![Drawing::new(
            "resident",
            0.5,
            vec![
                DrawPart::rect(0.0, 0.0, 1.0, 1.0, Ink::Clothes),
                DrawPart::ellipse(0.0, 0.5, 0.1, 0.1, Ink::Skin),
            ],
        )];
        let mut json = serde_json::to_value(ProjectionSnapshotWire::from(&snapshot)).unwrap();
        // A tone, a kind of item and a kind of briefing this build has
        // never heard of, a change it cannot show and a shape it cannot
        // draw.
        rename_all(&mut json, "tone", "iridescent");
        rename_all(&mut json, "kind", "omen");
        let commands = json["commands"].as_array_mut().unwrap();
        commands[0]["effects"][0]["change"] = serde_json::json!({"type": "sideways"});
        json["drawings"][0]["parts"][0]["shape"] = serde_json::json!({"type": "star", "arms": 5});
        let wire: ProjectionSnapshotWire = serde_json::from_value(json).unwrap();
        wire.validate_for_protocol(PACK_PROTOCOL_VERSION).unwrap();
        let read = ProjectionSnapshot::try_from(wire).unwrap();
        assert_eq!(read.title, snapshot.title);
        // What could not be read is read as the plainest thing it could
        // be, or left out.
        assert_eq!(read.briefing.unwrap().items[0].kind, BriefingItemKind::Beat);
        assert!(read
            .canvas
            .items
            .iter()
            .all(|item| item.kind == CanvasItemKind::Object));
        assert!(read
            .canvas
            .links
            .iter()
            .all(|link| link.tone == CanvasLinkTone::Neutral));
        assert_eq!(
            read.commands[0].effects.len(),
            snapshot.commands[0].effects.len() - 1
        );
        assert_eq!(read.drawings[0].parts.len(), 1);
        // A story page of a kind this build does not know is no page.
        let page: stories::StoryPageWire =
            serde_json::from_str(r#"{"type":"ballad","verses":[]}"#).unwrap();
        assert_eq!(page.into_page(), None);
    }

    #[test]
    fn a_design_or_a_name_is_not_sent_typed_to_a_pack_before_v8() {
        let design = PackRequest::Handle {
            intent: ProjectionIntentWire::Name {
                target: "pack.mark.name.7".into(),
                name: "Ann".into(),
            },
        };
        assert!(
            PackRequestEnvelope::for_version(PACK_PROTOCOL_VERSION_V7, 1, design.clone()).is_err()
        );
        assert!(PackRequestEnvelope::for_version(PACK_PROTOCOL_VERSION_V8, 1, design).is_ok());
    }

    #[test]
    fn typed_intents_cross_and_a_bad_design_goes_as_the_old_form() {
        let pattern = world_projection::Design::parse(&format!("{}:2f", "01".repeat(128))).unwrap();
        for intent in [
            ProjectionIntent::Design {
                target: "pack.mark.design.7".into(),
                pattern,
            },
            ProjectionIntent::Name {
                target: "pack.mark.name.7".into(),
                name: "Ann = Bea".into(),
            },
        ] {
            let json = serde_json::to_string(&ProjectionIntentWire::from(intent.clone())).unwrap();
            let back: ProjectionIntentWire = serde_json::from_str(&json).unwrap();
            assert_eq!(ProjectionIntent::from(back), intent);
        }
        let bad: ProjectionIntentWire = serde_json::from_str(
            r#"{"type":"design","target":"pack.mark.design.7","pattern":"no"}"#,
        )
        .unwrap();
        assert_eq!(
            ProjectionIntent::from(bad),
            ProjectionIntent::InvokeCommand("pack.mark.design.7=no".into())
        );
    }

    #[test]
    fn a_descriptor_says_what_a_pack_can_do_and_an_older_host_reads_it() {
        let descriptor =
            descriptor().with_capabilities([CAPABILITY_STORY, CAPABILITY_PLOTS, "telepathy"]);
        assert!(descriptor.can(CAPABILITY_PLOTS));
        assert!(!descriptor.can(CAPABILITY_NAMES));
        let json = serde_json::to_string(&descriptor).unwrap();
        let back: PackDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, descriptor);
        assert!(back.same_pack(&self::descriptor()));
        // Without capabilities, a descriptor is written as before.
        assert!(!serde_json::to_string(&self::descriptor())
            .unwrap()
            .contains("capabilities"));
    }
}
