use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use world_core::{EntityId, EventId};
use world_persistence::{WorldArchive, WorldPackRef};
use world_projection::{
    BriefingItem, BriefingProjection, CanvasItem, CanvasItemKind, CanvasProjection, CollectionItem,
    CollectionProjection, InspectorProjection, InspectorRow, InspectorSection,
    ProjectionCapabilities, ProjectionCommand, ProjectionIntent, ProjectionSnapshot, SelectionId,
    TimelineItem, TimelineProjection, WhyNode, WhyProjection,
};

pub const PACK_MANIFEST_FORMAT: &str = "world-machine-pack";
pub const PACK_MANIFEST_VERSION: u32 = 1;
pub const PACK_PROTOCOL_VERSION: u32 = 1;

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
        if self.protocol_version != PACK_PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedProtocolVersion(
                self.protocol_version,
            ));
        }
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
        }
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

    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_protocol_version(self.protocol_version)
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

    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_protocol_version(self.protocol_version)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PackRequest {
    Describe,
    Create,
    Open { archive: WorldArchive },
    Snapshot,
    Handle { intent: ProjectionIntentWire },
    Advance { periods: u64 },
    Archive,
    Shutdown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PackResponse {
    Descriptor { descriptor: PackDescriptor },
    Snapshot { snapshot: ProjectionSnapshotWire },
    Archive { archive: Option<WorldArchive> },
    Ok,
    Error { message: String },
}

pub fn encode_request(request: &PackRequestEnvelope) -> Result<String, serde_json::Error> {
    serde_json::to_string(request)
}

pub fn decode_request(json: &str) -> Result<PackRequestEnvelope, ProtocolDecodeError> {
    let request =
        serde_json::from_str::<PackRequestEnvelope>(json).map_err(ProtocolDecodeError::Json)?;
    request.validate().map_err(ProtocolDecodeError::Protocol)?;
    Ok(request)
}

pub fn encode_response(response: &PackResponseEnvelope) -> Result<String, serde_json::Error> {
    serde_json::to_string(response)
}

pub fn decode_response(json: &str) -> Result<PackResponseEnvelope, ProtocolDecodeError> {
    let response =
        serde_json::from_str::<PackResponseEnvelope>(json).map_err(ProtocolDecodeError::Json)?;
    response.validate().map_err(ProtocolDecodeError::Protocol)?;
    Ok(response)
}

fn validate_protocol_version(version: u32) -> Result<(), ProtocolError> {
    if version == PACK_PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(ProtocolError::UnsupportedProtocolVersion(version))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProjectionIntentWire {
    ForkBeforeEvent { event: u64 },
    InvokeCommand { command: String },
}

impl From<ProjectionIntent> for ProjectionIntentWire {
    fn from(intent: ProjectionIntent) -> Self {
        match intent {
            ProjectionIntent::ForkBeforeEvent(event) => Self::ForkBeforeEvent { event: event.0 },
            ProjectionIntent::InvokeCommand(command) => Self::InvokeCommand { command },
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
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SelectionIdWire {
    Entity { id: u64 },
    Event { id: u64 },
}

impl SelectionIdWire {
    fn stable_key(self) -> String {
        match self {
            Self::Entity { id } => format!("entity-{id}"),
            Self::Event { id } => format!("event-{id}"),
        }
    }
}

impl From<SelectionId> for SelectionIdWire {
    fn from(selection: SelectionId) -> Self {
        match selection {
            SelectionId::Entity(id) => Self::Entity { id: id.0 },
            SelectionId::Event(id) => Self::Event { id: id.0 },
        }
    }
}

impl From<SelectionIdWire> for SelectionId {
    fn from(selection: SelectionIdWire) -> Self {
        match selection {
            SelectionIdWire::Entity { id } => Self::Entity(EntityId::new(id)),
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
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectionCapabilitiesWire {
    pub fork: bool,
}

impl From<ProjectionCapabilities> for ProjectionCapabilitiesWire {
    fn from(capabilities: ProjectionCapabilities) -> Self {
        Self {
            fork: capabilities.fork,
        }
    }
}

impl From<ProjectionCapabilitiesWire> for ProjectionCapabilities {
    fn from(capabilities: ProjectionCapabilitiesWire) -> Self {
        Self {
            fork: capabilities.fork,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectionCommandWire {
    pub id: String,
    pub title: String,
    pub detail: String,
}

impl From<&ProjectionCommand> for ProjectionCommandWire {
    fn from(command: &ProjectionCommand) -> Self {
        Self {
            id: command.id.clone(),
            title: command.title.clone(),
            detail: command.detail.clone(),
        }
    }
}

impl From<ProjectionCommandWire> for ProjectionCommand {
    fn from(command: ProjectionCommandWire) -> Self {
        Self {
            id: command.id,
            title: command.title,
            detail: command.detail,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BriefingProjectionWire {
    pub eyebrow: String,
    pub title: String,
    pub items: Vec<BriefingItemWire>,
}

impl From<&BriefingProjection> for BriefingProjectionWire {
    fn from(briefing: &BriefingProjection) -> Self {
        Self {
            eyebrow: briefing.eyebrow.clone(),
            title: briefing.title.clone(),
            items: briefing.items.iter().map(Into::into).collect(),
        }
    }
}

impl From<BriefingProjectionWire> for BriefingProjection {
    fn from(briefing: BriefingProjectionWire) -> Self {
        Self {
            eyebrow: briefing.eyebrow,
            title: briefing.title,
            items: briefing.items.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BriefingItemWire {
    pub selection: Option<SelectionIdWire>,
    pub title: String,
    pub detail: String,
}

impl From<&BriefingItem> for BriefingItemWire {
    fn from(item: &BriefingItem) -> Self {
        Self {
            selection: item.selection.map(Into::into),
            title: item.title.clone(),
            detail: item.detail.clone(),
        }
    }
}

impl From<BriefingItemWire> for BriefingItem {
    fn from(item: BriefingItemWire) -> Self {
        Self {
            selection: item.selection.map(Into::into),
            title: item.title,
            detail: item.detail,
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
}

impl From<&TimelineItem> for TimelineItemWire {
    fn from(item: &TimelineItem) -> Self {
        Self {
            id: item.id.into(),
            world_time: item.world_time,
            title: item.title.clone(),
            subtitle: item.subtitle.clone(),
            caused_by: item.caused_by.iter().map(|event| event.0).collect(),
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
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CanvasProjectionWire {
    pub items: Vec<CanvasItemWire>,
}

impl From<&CanvasProjection> for CanvasProjectionWire {
    fn from(canvas: &CanvasProjection) -> Self {
        Self {
            items: canvas.items.iter().map(Into::into).collect(),
        }
    }
}

impl From<CanvasProjectionWire> for CanvasProjection {
    fn from(canvas: CanvasProjectionWire) -> Self {
        Self {
            items: canvas.items.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanvasItemKindWire {
    Place,
    Actor,
    Object,
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
            CanvasItemKindWire::Object => Self::Object,
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
        BriefingItem, BriefingProjection, CanvasItem, CanvasItemKind, CanvasProjection,
        CollectionItem, CollectionProjection, InspectorProjection, InspectorRow, InspectorSection,
        ProjectionCapabilities, ProjectionCommand, TimelineItem, TimelineProjection, WhyNode,
        WhyProjection,
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
            capabilities: ProjectionCapabilities { fork: true },
            briefing: Some(BriefingProjection {
                eyebrow: "Status".into(),
                title: "World briefing".into(),
                items: vec![BriefingItem {
                    selection: Some(entity),
                    title: "Entity seven".into(),
                    detail: "A selected entity".into(),
                }],
            }),
            commands: vec![ProjectionCommand {
                id: "external.advance".into(),
                title: "Advance".into(),
                detail: "Advance the external world".into(),
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
                }],
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
}
