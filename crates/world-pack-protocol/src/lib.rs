use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use world_core::{EntityId, EventId, RelationId};
use world_persistence::{WorldArchive, WorldPackRef};
use world_projection::{
    BriefingItem, BriefingItemKind, BriefingProjection, CanvasChange, CanvasItem, CanvasItemKind,
    CanvasLink, CanvasLinkTone, CanvasMark, CanvasProjection, CollectionItem, CollectionProjection,
    CommandEffect, EffectChange, InspectorProjection, InspectorRow, InspectorSection, MarkShape,
    ProjectionCapabilities, ProjectionCommand, ProjectionIntent, ProjectionSnapshot, Scenery,
    SelectionId, TimelineItem, TimelineProjection, Tone, WhyNode, WhyProjection,
};

pub const PACK_MANIFEST_FORMAT: &str = "world-machine-pack";
pub const PACK_MANIFEST_VERSION: u32 = 1;
pub const PACK_PROTOCOL_VERSION_V1: u32 = 1;
pub const PACK_PROTOCOL_VERSION_V2: u32 = 2;
pub const PACK_PROTOCOL_VERSION: u32 = PACK_PROTOCOL_VERSION_V2;

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

    pub fn for_version(
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
    Open { archive: WorldArchive },
    Snapshot,
    Handle { intent: ProjectionIntentWire },
    Advance { periods: u64 },
    Archive,
    Shutdown,
}

// One response is built per message and serialized at once, so how much
// larger a snapshot is than an acknowledgement costs nothing worth an extra
// indirection at every one of the call sites that build one.
#[allow(clippy::large_enum_variant)]
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
    if matches!(version, PACK_PROTOCOL_VERSION_V1 | PACK_PROTOCOL_VERSION_V2) {
        Ok(())
    } else {
        Err(ProtocolError::UnsupportedProtocolVersion(version))
    }
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
    pub goals: Vec<GoalWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chapters: Vec<ChapterWire>,
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
        for chapter in &self.chapters {
            if let Some(moment) = chapter.moment {
                validate_selection_for_protocol(protocol_version, moment)?;
            }
        }
        for inspector in &self.inspectors {
            validate_selection_for_protocol(protocol_version, inspector.selection)?;
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
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectionCapabilitiesWire {
    pub fork: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub background: bool,
}

impl From<ProjectionCapabilities> for ProjectionCapabilitiesWire {
    fn from(capabilities: ProjectionCapabilities) -> Self {
        Self {
            fork: capabilities.fork,
            background: capabilities.background,
        }
    }
}

impl From<ProjectionCapabilitiesWire> for ProjectionCapabilities {
    fn from(capabilities: ProjectionCapabilitiesWire) -> Self {
        Self {
            fork: capabilities.fork,
            background: capabilities.background,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
            ToneWire::Neutral => Self::Neutral,
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
        }
    }
}

impl From<ProjectionCommandWire> for ProjectionCommand {
    fn from(command: ProjectionCommandWire) -> Self {
        Self {
            id: command.id,
            title: command.title,
            detail: command.detail,
            effects: command.effects.into_iter().map(Into::into).collect(),
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
            BriefingItemKindWire::Beat => Self::Beat,
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
            CanvasLinkToneWire::Neutral => Self::Neutral,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<CanvasChangeWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<MarkShapeWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<SelectionIdWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<LookWire>,
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
    fn moving_on_its_own_is_declared_and_absent_means_it_does_not() {
        let old: ProjectionCapabilitiesWire =
            serde_json::from_str(r#"{"fork":true}"#).expect("an older Pack decodes");
        assert!(!ProjectionCapabilities::from(old).background);
        let live = ProjectionCapabilities {
            fork: false,
            background: true,
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
}
