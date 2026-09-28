use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io::{Read, Write};
use world_persistence::{
    ArchivedCheckpoint, CheckpointFit, PersistenceError, WorldArchive, WorldPackRef,
};

pub const DOCUMENT_METADATA_FIELD: &str = "document";
/// How many of a World's own days make a season, for its checkpoints: a
/// World file carries one at the start of its latest season.
pub const CHECKPOINT_SEASON_UNITS: u64 = 30;
/// For a World that does not say what it counts time in, how much world
/// time a checkpoint's season is.
pub const CHECKPOINT_FALLBACK_SPAN: u64 = 30;
/// The largest World file a reader will unpack, so a hostile file cannot
/// fill memory: far more than years of any World's history.
pub const MAX_DOCUMENT_BYTES: u64 = 512 * 1024 * 1024;
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorldDocumentMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<WorldLineage>,
    /// How the World looks, as its Pack last said, so a list of Worlds can
    /// draw each one in its own colours without opening it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_scenery: Option<DocumentScenery>,
    /// What the World counts its time in, so a list can say "Sol 5".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_calendar: Option<DocumentCalendar>,
    /// The shapes of what the World has built, oldest first, so its cover
    /// can show it filling up.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub display_marks: Vec<String>,
    /// Whether the World moves on by itself between visits.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub display_moves_alone: bool,
    /// Who and what stands in the World, as its Pack last drew it, so its
    /// cover can show its own people and buildings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub display_cast: Vec<DocumentFigure>,
    /// The Pack's own drawings its cast is drawn with, as the Pack protocol
    /// writes them, so the cover draws each one as the World does.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub display_drawings: Vec<serde_json::Value>,
}

/// One person, place or thing on a World's stage, as its cover draws it.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentFigure {
    /// Stable within the World, such as `entity-11`.
    pub id: String,
    /// `place`, `person` or `thing`.
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    /// The id of wherever it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// Where the Pack put it left to right, in thousandths of the stage.
    #[serde(default)]
    pub x: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clothes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hair: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin: Option<u32>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bird: bool,
    /// What they carry, such as `fish` or `bread`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carries: Option<String>,
    /// Which of the World's drawings it is drawn with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawing: Option<String>,
}

impl WorldDocumentMetadata {
    pub fn is_empty(&self) -> bool {
        self.display_title.is_none()
            && self.display_summary.is_none()
            && self.lineage.is_none()
            && self.display_scenery.is_none()
            && self.display_calendar.is_none()
            && self.display_marks.is_empty()
            && !self.display_moves_alone
            && self.display_cast.is_empty()
            && self.display_drawings.is_empty()
    }
}

/// A World's unit of time and how much world time one of them is.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentCalendar {
    pub unit: String,
    pub length: u64,
}

/// A World's colours as `0xRRGGBB`: sky, far ridge, near ridge, sun.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentScenery {
    pub sky_top: u32,
    pub sky_bottom: u32,
    pub far: u32,
    pub near: u32,
    pub sun: u32,
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

    /// How much world time a checkpoint's season is for this World.
    pub fn season_span(&self) -> u64 {
        self.metadata
            .display_calendar
            .as_ref()
            .map(|calendar| calendar.length.max(1) * CHECKPOINT_SEASON_UNITS)
            .unwrap_or(CHECKPOINT_FALLBACK_SPAN)
    }

    /// The world time the latest season began at.
    pub fn season_start(&self) -> u64 {
        let span = self.season_span();
        self.archive.world_time / span * span
    }

    /// Brings the archive's checkpoint up to the start of the latest
    /// season, carrying the one it has on rather than summing up the whole
    /// history again. The checkpoint lets opening the World replay only the
    /// season since; it is derived from the archive alone.
    pub fn settle_checkpoint(&mut self) {
        if let Cow::Owned(settled) = self.settled_checkpoint() {
            self.archive.checkpoint = Some(settled);
        }
    }

    /// The checkpoint at the start of the latest season: the one the
    /// document has when it is already there, or else that one carried on
    /// (or, when it does not belong to this history, a new one).
    pub fn settled_checkpoint(&self) -> Cow<'_, ArchivedCheckpoint> {
        let start = self.season_start();
        let end = self
            .archive
            .events
            .partition_point(|event| event.world_time < start);
        match &self.archive.checkpoint {
            Some(checkpoint)
                if checkpoint.events == end
                    && checkpoint.fit(&self.archive) == Some(CheckpointFit::Within) =>
            {
                Cow::Borrowed(checkpoint)
            }
            previous => Cow::Owned(ArchivedCheckpoint::at(
                previous.as_ref(),
                &self.archive,
                start,
            )),
        }
    }

    /// The document as a World file writes it: its archive in the compact
    /// encoding with its metadata and its checkpoint at the start of the
    /// latest season alongside, gzipped.
    pub fn to_bytes(&self) -> Result<Vec<u8>, DocumentError> {
        // A checkpoint of nothing says nothing, so it is not written.
        let checkpoint = self.settled_checkpoint();
        let checkpoint = Some(checkpoint.as_ref()).filter(|checkpoint| checkpoint.events > 0);
        let json = self.compact_json_with(checkpoint)?;
        // A low level: it finds most of what a history repeats, and a
        // World is saved after every change.
        let mut encoder = GzEncoder::new(Vec::with_capacity(json.len() / 8), Compression::new(2));
        encoder.write_all(&json).map_err(DocumentError::Io)?;
        encoder.finish().map_err(DocumentError::Io)
    }

    /// The document as one line of compact JSON, with the checkpoint its
    /// archive has as it stands.
    pub fn to_compact_json(&self) -> Result<Vec<u8>, DocumentError> {
        self.compact_json_with(self.archive.checkpoint.as_ref())
    }

    fn compact_json_with(
        &self,
        checkpoint: Option<&ArchivedCheckpoint>,
    ) -> Result<Vec<u8>, DocumentError> {
        let mut extra = serde_json::Map::new();
        if !self.metadata.is_empty() {
            extra.insert(
                DOCUMENT_METADATA_FIELD.into(),
                serde_json::to_value(&self.metadata)?,
            );
        }
        Ok(self.archive.to_compact_json_with(checkpoint, &extra)?)
    }

    /// Reads a World file however it was written: gzipped or not, compact
    /// or tagged, with or without a checkpoint.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DocumentError> {
        if bytes.starts_with(&GZIP_MAGIC) {
            let mut json = Vec::new();
            GzDecoder::new(bytes)
                .take(MAX_DOCUMENT_BYTES + 1)
                .read_to_end(&mut json)
                .map_err(DocumentError::Io)?;
            if json.len() as u64 > MAX_DOCUMENT_BYTES {
                return Err(DocumentError::TooLarge);
            }
            Self::from_json_bytes(&json)
        } else {
            Self::from_json_bytes(bytes)
        }
    }

    fn from_json_bytes(json: &[u8]) -> Result<Self, DocumentError> {
        let value: serde_json::Value = serde_json::from_slice(json)?;
        Self::from_json_value(&value)
    }

    fn from_json_value(value: &serde_json::Value) -> Result<Self, DocumentError> {
        // The persistence layer deliberately ignores document-only extension
        // fields, so Packs and Host code continue to consume a pure archive.
        let archive = WorldArchive::from_json_value(value)?;
        let object = value.as_object().ok_or(DocumentError::InvalidRoot)?;
        let metadata = match object.get(DOCUMENT_METADATA_FIELD) {
            Some(value) => WorldDocumentMetadata::deserialize(value)?,
            None => WorldDocumentMetadata::default(),
        };
        Ok(Self { archive, metadata })
    }

    pub fn with_display_title(mut self, title: impl Into<String>) -> Self {
        self.metadata.display_title = Some(title.into());
        self
    }

    pub fn with_display_summary(mut self, summary: impl Into<String>) -> Self {
        self.metadata.display_summary = Some(summary.into());
        self
    }

    pub fn with_lineage(mut self, lineage: WorldLineage) -> Self {
        self.metadata.lineage = Some(lineage);
        self
    }

    /// The document in the tagged encoding World files were written in
    /// before the compact one, which older World Machines read (they pass
    /// over the checkpoint).
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

    /// Reads a document written as JSON, in either encoding.
    pub fn from_json(json: &str) -> Result<Self, DocumentError> {
        Self::from_json_bytes(json.as_bytes())
    }
}

#[derive(Debug)]
pub enum DocumentError {
    Persistence(PersistenceError),
    Json(serde_json::Error),
    InvalidRoot,
    Io(std::io::Error),
    TooLarge,
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Persistence(error) => error.fmt(f),
            Self::Json(error) => write!(f, "invalid World document JSON: {error}"),
            Self::InvalidRoot => write!(f, "World document JSON root must be an object"),
            Self::Io(error) => write!(f, "unreadable World document: {error}"),
            Self::TooLarge => write!(f, "World document is too large to open"),
        }
    }
}

impl Error for DocumentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Persistence(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::InvalidRoot | Self::TooLarge => None,
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
            checkpoint: None,
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
    fn display_summary_round_trips_as_document_only_metadata() {
        let document = WorldDocument::new(archive(8))
            .with_display_title("A Small Mars")
            .with_display_summary("Ridge Network · care-led");

        let json = document.to_json_pretty().unwrap();
        let decoded = WorldDocument::from_json(&json).unwrap();
        let pure_archive = WorldArchive::from_json(&json).unwrap();

        assert_eq!(
            decoded.metadata.display_summary.as_deref(),
            Some("Ridge Network · care-led")
        );
        assert_eq!(pure_archive.world_time, 8);
        assert!(json.contains("\"display_summary\""));
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

    fn lived_archive() -> WorldArchive {
        use world_persistence::{ArchivedEvent, ArchivedStateChange, ArchivedValue};
        let mut archive = archive(99);
        archive.events = (1..=12)
            .map(|id| ArchivedEvent {
                id,
                kind: "lived".into(),
                world_time: id * 8,
                actor: Some(1),
                targets: Vec::new(),
                caused_by: Vec::new(),
                payload: Default::default(),
                changes: vec![ArchivedStateChange::SetComponent {
                    entity: 1,
                    key: "day".into(),
                    value: ArchivedValue::Integer(id as i64),
                }],
            })
            .collect();
        archive
    }

    #[test]
    fn a_world_file_is_gzipped_compact_json_with_a_seasons_checkpoint() {
        let mut document = WorldDocument::new(lived_archive()).with_display_title("Harbor");
        document.metadata.display_calendar = Some(DocumentCalendar {
            unit: "Day".into(),
            length: 1,
        });
        let bytes = document.to_bytes().unwrap();
        assert_eq!(bytes[..2], GZIP_MAGIC);

        let read = WorldDocument::from_bytes(&bytes).unwrap();
        assert_eq!(read.archive.events, document.archive.events);
        assert_eq!(read.metadata, document.metadata);
        // Its checkpoint is at the start of the latest thirty days: day 90.
        let checkpoint = read.archive.checkpoint.clone().expect("a checkpoint");
        assert_eq!(checkpoint.events, 11);
        assert_eq!(checkpoint.last_event, 11);
        assert_eq!(checkpoint.changes.len(), 1, "only the latest value is kept");
        assert_eq!(document.season_start(), 90);

        // Carried on, it comes to the same.
        let mut settled = document.clone();
        settled.archive.checkpoint =
            Some(ArchivedCheckpoint::covering(&document.archive.events[..4]));
        settled.settle_checkpoint();
        assert_eq!(settled.archive.checkpoint.as_ref(), Some(&checkpoint));
    }

    #[test]
    fn older_world_files_still_open() {
        let document = WorldDocument::new(lived_archive()).with_display_title("Harbor");
        // Tagged and pretty, as World files were written before.
        let tagged = document.to_json_pretty().unwrap();
        assert!(tagged.contains("\"set_component\""));
        let read = WorldDocument::from_bytes(tagged.as_bytes()).unwrap();
        assert_eq!(read, document);
        assert_eq!(WorldDocument::from_json(&tagged).unwrap(), document);
        // Compact but not gzipped.
        let compact = document.to_compact_json().unwrap();
        assert_eq!(WorldDocument::from_bytes(&compact).unwrap(), document);
        // Tagged and gzipped.
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(tagged.as_bytes()).unwrap();
        let gzipped = encoder.finish().unwrap();
        assert_eq!(WorldDocument::from_bytes(&gzipped).unwrap(), document);
        // A cut file is refused.
        let bytes = document.to_bytes().unwrap();
        assert!(WorldDocument::from_bytes(&bytes[..bytes.len() / 2]).is_err());
    }
}
