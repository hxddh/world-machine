use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use world_persistence::{PersistenceError, WorldArchive, WorldPackRef};

pub const DOCUMENT_METADATA_FIELD: &str = "document";

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorldDocumentMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<WorldLineage>,
}

impl WorldDocumentMetadata {
    pub fn is_empty(&self) -> bool {
        self.display_title.is_none() && self.lineage.is_none()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorldLineage {
    pub parent: WorldParent,
    pub branch: WorldBranchCause,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorldParent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
    pub pack: WorldPackRef,
    pub world_time: u64,
    pub event_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorldBranchCause {
    Strategy {
        choice_id: String,
        choice_title: String,
        horizon: u64,
    },
    Fork {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorldDocument {
    pub archive: WorldArchive,
    pub metadata: WorldDocumentMetadata,
}

impl WorldDocument {
    pub fn new(archive: WorldArchive) -> Self {
        Self {
            archive,
            metadata: WorldDocumentMetadata::default(),
        }
    }

    pub fn with_display_title(mut self, title: impl Into<String>) -> Self {
        self.metadata.display_title = Some(title.into());
        self
    }

    pub fn with_lineage(mut self, lineage: WorldLineage) -> Self {
        self.metadata.lineage = Some(lineage);
        self
    }

    pub fn to_json_pretty(&self) -> Result<String, DocumentError> {
        // Keep WorldArchive's existing header validation as the source of truth.
        let archive_json = self.archive.to_json_pretty()?;
        let mut value: serde_json::Value = serde_json::from_str(&archive_json)?;
        let object = value.as_object_mut().ok_or(DocumentError::InvalidRoot)?;
        if !self.metadata.is_empty() {
            object.insert(
                DOCUMENT_METADATA_FIELD.into(),
                serde_json::to_value(&self.metadata)?,
            );
        }
        Ok(serde_json::to_string_pretty(&value)?)
    }

    pub fn from_json(json: &str) -> Result<Self, DocumentError> {
        // The persistence layer deliberately ignores document-only extension
        // fields, so Packs and Host code continue to consume a pure archive.
        let archive = WorldArchive::from_json(json)?;
        let value: serde_json::Value = serde_json::from_str(json)?;
        let object = value.as_object().ok_or(DocumentError::InvalidRoot)?;
        let metadata = match object.get(DOCUMENT_METADATA_FIELD) {
            Some(value) => serde_json::from_value(value.clone())?,
            None => WorldDocumentMetadata::default(),
        };
        Ok(Self { archive, metadata })
    }
}

#[derive(Debug)]
pub enum DocumentError {
    Persistence(PersistenceError),
    Json(serde_json::Error),
    InvalidRoot,
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Persistence(error) => error.fmt(f),
            Self::Json(error) => write!(f, "invalid World document JSON: {error}"),
            Self::InvalidRoot => write!(f, "World document JSON root must be an object"),
        }
    }
}

impl Error for DocumentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Persistence(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::InvalidRoot => None,
        }
    }
}

impl From<PersistenceError> for DocumentError {
    fn from(error: PersistenceError) -> Self {
        Self::Persistence(error)
    }
}

impl From<serde_json::Error> for DocumentError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_persistence::{WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};

    fn archive(world_time: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new("world-machine.document-mock", "1"),
            world_time,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn lineage() -> WorldLineage {
        WorldLineage {
            parent: WorldParent {
                document: Some("source-world".into()),
                pack: WorldPackRef::new("world-machine.document-mock", "1"),
                world_time: 12,
                event_count: 4,
            },
            branch: WorldBranchCause::Strategy {
                choice_id: "mock.choose-a".into(),
                choice_title: "Choose A".into(),
                horizon: 20,
            },
        }
    }

    #[test]
    fn reads_legacy_bare_archives_without_metadata() {
        let bare = archive(12).to_json_pretty().unwrap();

        let document = WorldDocument::from_json(&bare).unwrap();

        assert_eq!(document.archive.world_time, 12);
        assert_eq!(document.metadata, WorldDocumentMetadata::default());
    }

    #[test]
    fn display_title_round_trips_as_document_only_metadata() {
        let document = WorldDocument::new(archive(8)).with_display_title("A Small Mars");

        let json = document.to_json_pretty().unwrap();
        let decoded = WorldDocument::from_json(&json).unwrap();
        let pure_archive = WorldArchive::from_json(&json).unwrap();

        assert_eq!(
            decoded.metadata.display_title.as_deref(),
            Some("A Small Mars")
        );
        assert_eq!(pure_archive.world_time, 8);
        assert!(json.contains("\"display_title\""));
    }

    #[test]
    fn lineage_round_trips_inside_the_same_world_file() {
        let document = WorldDocument::new(archive(32)).with_lineage(lineage());

        let json = document.to_json_pretty().unwrap();
        let decoded = WorldDocument::from_json(&json).unwrap();

        assert_eq!(decoded, document);
        assert!(json.contains("\"document\""));
        assert!(json.contains("\"strategy\""));
    }

    #[test]
    fn pure_archive_reader_ignores_document_metadata() {
        let document = WorldDocument::new(archive(32)).with_lineage(lineage());
        let json = document.to_json_pretty().unwrap();

        let archive = WorldArchive::from_json(&json).unwrap();

        assert_eq!(archive.world_time, 32);
        assert_eq!(archive.pack.id, "world-machine.document-mock");
    }

    #[test]
    fn empty_metadata_is_not_written() {
        let json = WorldDocument::new(archive(5)).to_json_pretty().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(value.get(DOCUMENT_METADATA_FIELD).is_none());
    }
}
