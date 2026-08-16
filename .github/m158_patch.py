from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    assert count == 1, f"{label}: expected 1 match, found {count}"
    return text.replace(old, new, 1)


projection_path = Path("crates/world-projection/src/lib.rs")
text = projection_path.read_text()
text = replace_once(
    text,
    """pub enum SelectionId {
    Entity(EntityId),
    Event(EventId),
}
""",
    """pub enum SelectionId {
    Entity(EntityId),
    Relation(RelationId),
    Event(EventId),
}
""",
    "projection selection variant",
)
text = replace_once(
    text,
    """        match self {
            Self::Entity(id) => format!(\"entity-{id}\"),
            Self::Event(id) => format!(\"event-{id}\"),
        }
""",
    """        match self {
            Self::Entity(id) => format!(\"entity-{id}\"),
            Self::Relation(id) => format!(\"relation-{id}\"),
            Self::Event(id) => format!(\"event-{id}\"),
        }
""",
    "projection stable key",
)
projection_path.write_text(text)

protocol_path = Path("crates/world-pack-protocol/src/lib.rs")
text = protocol_path.read_text()
text = replace_once(
    text,
    "use world_core::{EntityId, EventId};\n",
    "use world_core::{EntityId, EventId, RelationId};\n",
    "protocol core ids",
)
text = replace_once(
    text,
    """pub enum SelectionIdWire {
    Entity { id: u64 },
    Event { id: u64 },
}
""",
    """pub enum SelectionIdWire {
    Entity { id: u64 },
    Relation { id: u64 },
    Event { id: u64 },
}
""",
    "wire selection variant",
)
text = replace_once(
    text,
    """        match self {
            Self::Entity { id } => format!(\"entity-{id}\"),
            Self::Event { id } => format!(\"event-{id}\"),
        }
""",
    """        match self {
            Self::Entity { id } => format!(\"entity-{id}\"),
            Self::Relation { id } => format!(\"relation-{id}\"),
            Self::Event { id } => format!(\"event-{id}\"),
        }
""",
    "wire stable key",
)
text = replace_once(
    text,
    """        match selection {
            SelectionId::Entity(id) => Self::Entity { id: id.0 },
            SelectionId::Event(id) => Self::Event { id: id.0 },
        }
""",
    """        match selection {
            SelectionId::Entity(id) => Self::Entity { id: id.0 },
            SelectionId::Relation(id) => Self::Relation { id: id.0 },
            SelectionId::Event(id) => Self::Event { id: id.0 },
        }
""",
    "selection to wire",
)
text = replace_once(
    text,
    """        match selection {
            SelectionIdWire::Entity { id } => Self::Entity(EntityId::new(id)),
            SelectionIdWire::Event { id } => Self::Event(EventId::new(id)),
        }
""",
    """        match selection {
            SelectionIdWire::Entity { id } => Self::Entity(EntityId::new(id)),
            SelectionIdWire::Relation { id } => Self::Relation(RelationId::new(id)),
            SelectionIdWire::Event { id } => Self::Event(EventId::new(id)),
        }
""",
    "wire to selection",
)

text = replace_once(
    text,
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
""",
    """    pub fn for_version(
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
""",
    "response envelope version semantics",
)

marker = """fn validate_protocol_version(version: u32) -> Result<(), ProtocolError> {
    if matches!(version, PACK_PROTOCOL_VERSION_V1 | PACK_PROTOCOL_VERSION_V2) {
        Ok(())
    } else {
        Err(ProtocolError::UnsupportedProtocolVersion(version))
    }
}
"""
addition = marker + """

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
"""
text = replace_once(text, marker, addition, "protocol semantic validators")

snapshot_marker = """impl From<&ProjectionSnapshot> for ProjectionSnapshotWire {
"""
snapshot_impl = """impl ProjectionSnapshotWire {
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
        }
        for inspector in &self.inspectors {
            validate_selection_for_protocol(protocol_version, inspector.selection)?;
        }
        Ok(())
    }
}

""" + snapshot_marker
text = replace_once(text, snapshot_marker, snapshot_impl, "snapshot protocol validation")

text = replace_once(
    text,
    """    DuplicateInspector(String),
    DuplicateWhy(u64),
    DepthOverflow(u64),
""",
    """    DuplicateInspector(String),
    DuplicateWhy(u64),
    SelectionNotSupportedInProtocol {
        protocol_version: u32,
        selection: String,
    },
    DepthOverflow(u64),
""",
    "protocol error variant",
)
text = replace_once(
    text,
    """            Self::DuplicateWhy(event) => {
                write!(
                    f,
                    \"Pack snapshot contains duplicate why projection for event {event}\"
                )
            }
            Self::DepthOverflow(depth) => {
""",
    """            Self::DuplicateWhy(event) => {
                write!(
                    f,
                    \"Pack snapshot contains duplicate why projection for event {event}\"
                )
            }
            Self::SelectionNotSupportedInProtocol {
                protocol_version,
                selection,
            } => write!(
                f,
                \"selection {selection} is not supported by Pack protocol v{protocol_version}\"
            ),
            Self::DepthOverflow(depth) => {
""",
    "protocol error display",
)
protocol_path.write_text(text)
