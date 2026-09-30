use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io::{Read, Write};
use world_persistence::{
    ArchiveHead, ArchivedCheckpoint, CheckpointFit, CompactHistory, PersistenceError, WorldArchive,
    WorldPackRef,
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
    /// What the app draws it as, from its own library of drawings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub art: Option<String>,
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
        season_span(&self.metadata)
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
    ///
    /// The file begins with a summary of the World and the document's own
    /// fields, so a list of Worlds reads them without the history (see
    /// [`Self::summary_from_bytes`]).
    pub fn to_bytes(&self) -> Result<Vec<u8>, DocumentError> {
        // A checkpoint of nothing says nothing, so it is not written.
        let checkpoint = self.settled_checkpoint();
        let checkpoint = Some(checkpoint.as_ref()).filter(|checkpoint| checkpoint.events > 0);
        let json = self
            .archive
            .to_compact_json_with(checkpoint, &serde_json::Map::new())?;
        let summary = FileSummary {
            pack: &self.archive.pack,
            world_time: self.archive.world_time,
            event_count: self.archive.events.len(),
        };
        gzipped(&file_json(&summary, &self.metadata, &json)?)
    }

    /// [`Self::file_from_history`], with the history's events deflated in
    /// pieces as they were recorded (see [`DeflatedHistory`]), so only what
    /// the World recorded since the last save is deflated now. The file is
    /// one gzip stream, as any World file: it reads as the same document.
    pub fn file_from_deflated_history(
        metadata: &WorldDocumentMetadata,
        head: ArchiveHead<'_>,
        history: &CompactHistory,
        deflated: &mut DeflatedHistory,
        checkpoint: Option<&ArchivedCheckpoint>,
    ) -> Result<Vec<u8>, DocumentError> {
        let checkpoint = checkpoint.filter(|checkpoint| checkpoint.events > 0);
        let (archive_before, after) =
            history.around_events(head, checkpoint, &serde_json::Map::new())?;
        let summary = FileSummary {
            pack: head.pack,
            world_time: head.world_time,
            event_count: history.len(),
        };
        let before = file_json(&summary, metadata, &archive_before)?;
        deflated.cover(history.written_text());

        let mut file = Vec::with_capacity(deflated.deflated_len() + before.len() / 4 + 64);
        file.extend_from_slice(&GZIP_HEADER);
        let mut crc = flate2::Crc::new();
        let head_part = deflate(&before, flate2::FlushCompress::Full)?;
        file.extend_from_slice(&head_part);
        crc.update(&before);
        for piece in &deflated.pieces {
            file.extend_from_slice(&piece.deflated);
            crc.combine(&piece.crc);
        }
        file.extend_from_slice(&deflate(&after, flate2::FlushCompress::Finish)?);
        crc.update(&after);
        file.extend_from_slice(&crc.sum().to_le_bytes());
        file.extend_from_slice(&crc.amount().to_le_bytes());
        Ok(file)
    }

    /// A World file written from a history kept written as the World grew
    /// (see [`CompactHistory`]): what [`Self::to_bytes`] writes for a
    /// document with this metadata, whose archive has `head`, these events
    /// and `checkpoint`, already settled. Only the names the changes set may
    /// be listed in another order; it reads back as the same document.
    pub fn file_from_history(
        metadata: &WorldDocumentMetadata,
        head: ArchiveHead<'_>,
        history: &CompactHistory,
        checkpoint: Option<&ArchivedCheckpoint>,
    ) -> Result<Vec<u8>, DocumentError> {
        let checkpoint = checkpoint.filter(|checkpoint| checkpoint.events > 0);
        let json = history.to_compact_json(head, checkpoint, &serde_json::Map::new())?;
        let summary = FileSummary {
            pack: head.pack,
            world_time: head.world_time,
            event_count: history.len(),
        };
        gzipped(&file_json(&summary, metadata, &json)?)
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

    /// The document's compact JSON as a World file keeps it deflated, when
    /// `file` is one gzipped with a plain header, as World Machine writes
    /// them: the Pack protocol carries an archive deflated too, so a World
    /// opened from its file can be handed to a Pack as the file keeps it,
    /// rather than written and deflated again. The Pack reads the document's
    /// own fields alongside and passes over them.
    ///
    /// Only for a file [`Self::from_bytes`] has read, which checked what the
    /// deflated JSON unpacks to.
    pub fn deflated_json(file: &[u8]) -> Option<&[u8]> {
        const HEADER: usize = 10;
        const TRAILER: usize = 8;
        const DEFLATE: u8 = 8;
        let plain = file.len() > HEADER + TRAILER
            && file.starts_with(&GZIP_MAGIC)
            && file[2] == DEFLATE
            && file[3] == 0;
        plain.then(|| &file[HEADER..file.len() - TRAILER])
    }

    /// Reads a World file however it was written: gzipped or not, compact
    /// or tagged, with or without a checkpoint.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DocumentError> {
        Self::from_json_bytes(&unpacked(bytes)?)
    }

    /// [`Self::from_bytes`], with the history as the file writes it, to
    /// write on from as the World records more (see
    /// [`Self::file_from_history`]); `None` for a file not written so.
    pub fn from_bytes_kept(bytes: &[u8]) -> Result<(Self, Option<CompactHistory>), DocumentError> {
        let json = unpacked(bytes)?;
        let (archive, metadata, history) =
            WorldArchive::from_json_slice_kept(&json, DOCUMENT_METADATA_FIELD)?;
        Ok((
            Self {
                archive,
                metadata: metadata_from(metadata.as_deref())?,
            },
            history,
        ))
    }

    /// What a list of Worlds shows of the World in a file, read from the
    /// summary at its start without unpacking its history; `None` for a
    /// file written without one, which is read whole instead.
    pub fn summary_from_bytes(bytes: &[u8]) -> Result<Option<DocumentSummary>, DocumentError> {
        if !bytes.starts_with(&GZIP_MAGIC) {
            return Ok(None);
        }
        let mut decoder = GzDecoder::new(bytes).take(MAX_DOCUMENT_BYTES);
        let mut json = Vec::new();
        let mut chunk = vec![0; 64 * 1024];
        loop {
            let read = decoder.read(&mut chunk).map_err(DocumentError::Io)?;
            json.extend_from_slice(&chunk[..read]);
            match summary_prefix(&json) {
                Prefix::Whole(summary) => return (*summary).map(Some),
                Prefix::None => return Ok(None),
                Prefix::More if read == 0 => return Ok(None),
                Prefix::More => {}
            }
        }
    }

    fn from_json_bytes(json: &[u8]) -> Result<Self, DocumentError> {
        // Read straight into the archive, without a tree of JSON values in
        // between. The persistence layer ignores document-only fields, so
        // Packs and Host code continue to consume a pure archive; the
        // document's own is handed back as it was written.
        let (archive, metadata) =
            WorldArchive::from_json_slice_with(json, DOCUMENT_METADATA_FIELD)?;
        Ok(Self {
            archive,
            metadata: metadata_from(metadata.as_deref())?,
        })
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

/// A gzip header with nothing optional in it, as flate2 writes one.
const GZIP_HEADER: [u8; 10] = [0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];

/// `data` deflated on its own, at the level World files use: `Full` leaves
/// the stream open at a byte boundary for more to follow, `Finish` ends it.
fn deflate(data: &[u8], flush: flate2::FlushCompress) -> Result<Vec<u8>, DocumentError> {
    let mut compress = flate2::Compress::new(Compression::new(2), false);
    let mut out = Vec::with_capacity(data.len() / 3 + 64);
    loop {
        let consumed = compress.total_in() as usize;
        let status = compress
            .compress_vec(&data[consumed..], &mut out, flush)
            .map_err(|error| DocumentError::Io(std::io::Error::other(error)))?;
        let done = compress.total_in() as usize == data.len();
        match status {
            flate2::Status::StreamEnd => return Ok(out),
            _ if done && out.len() < out.capacity() && flush != flate2::FlushCompress::Finish => {
                return Ok(out)
            }
            _ => out.reserve(out.capacity().max(4096)),
        }
    }
}

/// A World's history, as its file writes it, deflated in pieces as it was
/// recorded: each piece on its own, so a save deflates only what came
/// since the last, and the pieces, joined, are part of one deflate stream.
/// Pieces of what one save added are joined into one, now and then, so they
/// pack about as well as the history deflated whole.
#[derive(Debug, Default)]
pub struct DeflatedHistory {
    pieces: Vec<Piece>,
    /// How much of the history's text the pieces hold.
    covered: usize,
}

#[derive(Debug)]
struct Piece {
    /// Where in the history's text it starts.
    start: usize,
    len: usize,
    deflated: Vec<u8>,
    crc: flate2::Crc,
}

/// How many small pieces are kept before they are joined.
const SMALL_PIECES: usize = 24;

impl DeflatedHistory {
    /// The history's text, `text`, deflated whole.
    pub fn of(text: &[u8]) -> Result<Self, DocumentError> {
        let mut deflated = Self::default();
        deflated.push(text, 0)?;
        Ok(deflated)
    }

    fn push(&mut self, text: &[u8], start: usize) -> Result<(), DocumentError> {
        let data = &text[start..];
        if data.is_empty() {
            return Ok(());
        }
        // Events are joined by commas; a piece after another starts with
        // the comma that joins it.
        let mut crc = flate2::Crc::new();
        crc.update(data);
        self.pieces.push(Piece {
            start,
            len: data.len(),
            deflated: deflate(data, flate2::FlushCompress::Full)?,
            crc,
        });
        self.covered = text.len();
        Ok(())
    }

    /// Deflates the text the history wrote since the pieces were made.
    fn cover(&mut self, text: &[u8]) {
        if text.len() < self.covered {
            // The history went back past what was deflated: start again.
            *self = Self::default();
        }
        if self.push(text, self.covered).is_err() {
            *self = Self::default();
            let _ = self.push(text, 0);
        }
        // Many small pieces pack worse than one: join all after the first.
        if self.pieces.len() > SMALL_PIECES {
            let start = self.pieces[1].start;
            self.pieces.truncate(1);
            self.covered = start;
            if self.push(text, start).is_err() {
                *self = Self::default();
                let _ = self.push(text, 0);
            }
        }
    }

    /// Forgets what was deflated past `len` bytes of the text, as when the
    /// history goes back to what it held then.
    pub fn go_back_to(&mut self, len: usize) {
        while self
            .pieces
            .last()
            .is_some_and(|piece| piece.start + piece.len > len)
        {
            self.pieces.pop();
        }
        self.covered = self
            .pieces
            .last()
            .map_or(0, |piece| piece.start + piece.len);
    }

    /// How many bytes the pieces take.
    pub fn deflated_len(&self) -> usize {
        self.pieces.iter().map(|piece| piece.deflated.len()).sum()
    }
}

/// How much world time a checkpoint's season is for a World with this
/// metadata.
pub fn season_span(metadata: &WorldDocumentMetadata) -> u64 {
    metadata
        .display_calendar
        .as_ref()
        .map(|calendar| calendar.length.max(1) * CHECKPOINT_SEASON_UNITS)
        .unwrap_or(CHECKPOINT_FALLBACK_SPAN)
}

/// The field a World file begins with, summing up its World for a list of
/// Worlds, beside the document's own fields.
pub const DOCUMENT_SUMMARY_FIELD: &str = "summary";

/// What a list of Worlds shows of the World in a file, without its history.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentSummary {
    pub pack: WorldPackRef,
    pub world_time: u64,
    pub event_count: usize,
    pub metadata: WorldDocumentMetadata,
}

#[derive(Serialize, Deserialize)]
struct FileSummary<P> {
    pack: P,
    world_time: u64,
    event_count: usize,
}

/// A World file's JSON: its summary and the document's own fields first,
/// then its archive (written with no fields of its own beside it).
fn file_json(
    summary: &FileSummary<&WorldPackRef>,
    metadata: &WorldDocumentMetadata,
    archive: &[u8],
) -> Result<Vec<u8>, DocumentError> {
    let mut json = Vec::with_capacity(archive.len() + 64 * 1024);
    json.extend_from_slice(b"{\"");
    json.extend_from_slice(DOCUMENT_SUMMARY_FIELD.as_bytes());
    json.extend_from_slice(b"\":");
    serde_json::to_writer(&mut json, summary)?;
    if !metadata.is_empty() {
        json.extend_from_slice(b",\"");
        json.extend_from_slice(DOCUMENT_METADATA_FIELD.as_bytes());
        json.extend_from_slice(b"\":");
        serde_json::to_writer(&mut json, metadata)?;
    }
    json.push(b',');
    json.extend_from_slice(&archive[1..]);
    Ok(json)
}

fn gzipped(json: &[u8]) -> Result<Vec<u8>, DocumentError> {
    // A low level: it finds most of what a history repeats, and a World is
    // saved after every change.
    let mut encoder = GzEncoder::new(Vec::with_capacity(json.len() / 8), Compression::new(2));
    encoder.write_all(json).map_err(DocumentError::Io)?;
    encoder.finish().map_err(DocumentError::Io)
}

/// A World file's JSON, gzipped or not.
fn unpacked(bytes: &[u8]) -> Result<Cow<'_, [u8]>, DocumentError> {
    if !bytes.starts_with(&GZIP_MAGIC) {
        return Ok(Cow::Borrowed(bytes));
    }
    // A gzip file ends with how long it unpacks to (modulo 4 GiB), so the
    // JSON is unpacked without being copied as it grows; a file that claims
    // far more than JSON packs to is not believed.
    let unpacked = bytes
        .last_chunk::<4>()
        .map_or(0, |size| u64::from(u32::from_le_bytes(*size)))
        .min(bytes.len() as u64 * 32)
        .min(MAX_DOCUMENT_BYTES + 1);
    let mut json = Vec::with_capacity(usize::try_from(unpacked).unwrap_or(0));
    GzDecoder::new(bytes)
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut json)
        .map_err(DocumentError::Io)?;
    if json.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(DocumentError::TooLarge);
    }
    Ok(Cow::Owned(json))
}

fn metadata_from(
    metadata: Option<&serde_json::value::RawValue>,
) -> Result<WorldDocumentMetadata, DocumentError> {
    Ok(match metadata {
        // Read as a JSON value when it does not read straight, as it always
        // was (a field given twice reads as the last).
        Some(metadata) => serde_json::from_str(metadata.get()).or_else(|_| {
            serde_json::from_str::<serde_json::Value>(metadata.get())
                .and_then(WorldDocumentMetadata::deserialize)
        })?,
        None => WorldDocumentMetadata::default(),
    })
}

enum Prefix {
    /// The summary and the document's fields, read (or refused).
    Whole(Box<Result<DocumentSummary, DocumentError>>),
    /// Not a file that begins with a summary.
    None,
    /// Not all unpacked yet.
    More,
}

/// The summary a World file's JSON begins with, and the document's fields
/// right after it, from as much of the JSON as is unpacked.
fn summary_prefix(json: &[u8]) -> Prefix {
    let lead = format!("{{\"{DOCUMENT_SUMMARY_FIELD}\":");
    if json.len() < lead.len() {
        return Prefix::More;
    }
    if !json.starts_with(lead.as_bytes()) {
        return Prefix::None;
    }
    let summary_start = lead.len();
    let Some(summary_end) = value_end(json, summary_start) else {
        return Prefix::More;
    };
    let lead = format!(",\"{DOCUMENT_METADATA_FIELD}\":");
    let rest = &json[summary_end..];
    let metadata = if rest.len() < lead.len() {
        return Prefix::More;
    } else if rest.starts_with(lead.as_bytes()) {
        let start = summary_end + lead.len();
        let Some(end) = value_end(json, start) else {
            return Prefix::More;
        };
        Some(&json[start..end])
    } else {
        None
    };
    let read = || -> Result<DocumentSummary, DocumentError> {
        let summary: FileSummary<WorldPackRef> =
            serde_json::from_slice(&json[summary_start..summary_end])?;
        let metadata = match metadata {
            Some(metadata) => {
                let raw: &serde_json::value::RawValue = serde_json::from_slice(metadata)?;
                metadata_from(Some(raw))?
            }
            None => WorldDocumentMetadata::default(),
        };
        Ok(DocumentSummary {
            pack: summary.pack,
            world_time: summary.world_time,
            event_count: summary.event_count,
            metadata,
        })
    };
    Prefix::Whole(Box::new(read()))
}

/// Where the JSON object or array starting at `start` ends, found by its
/// brackets and strings alone, if it ends within `json`.
fn value_end(json: &[u8], start: usize) -> Option<usize> {
    let mut depth = 0_usize;
    let mut at = start;
    while at < json.len() {
        match json[at] {
            b'"' => {
                at += 1;
                while *json.get(at)? != b'"' {
                    at += if json[at] == b'\\' { 2 } else { 1 };
                }
            }
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(at + 1);
                }
            }
            _ => {}
        }
        at += 1;
    }
    None
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
    fn a_world_file_begins_with_what_a_list_of_worlds_shows() {
        let mut document = WorldDocument::new(lived_archive()).with_display_title("Harbor");
        document.metadata.display_marks = vec!["pier".into()];
        let file = document.to_bytes().unwrap();
        let summary = WorldDocument::summary_from_bytes(&file)
            .unwrap()
            .expect("a summary");
        assert_eq!(
            summary,
            DocumentSummary {
                pack: document.archive.pack.clone(),
                world_time: document.archive.world_time,
                event_count: document.archive.events.len(),
                metadata: document.metadata.clone(),
            }
        );
        let mut read = WorldDocument::from_bytes(&file).unwrap();
        read.archive.checkpoint = None;
        assert_eq!(read, document);

        // Without the document's own fields, and as files were written
        // before there was a summary: those are read whole.
        let bare = WorldDocument::new(lived_archive());
        let summary = WorldDocument::summary_from_bytes(&bare.to_bytes().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(summary.metadata, WorldDocumentMetadata::default());
        assert_eq!(summary.event_count, 12);
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(2));
        encoder
            .write_all(&document.to_compact_json().unwrap())
            .unwrap();
        let older = encoder.finish().unwrap();
        assert_eq!(WorldDocument::summary_from_bytes(&older).unwrap(), None);
        assert_eq!(
            WorldDocument::summary_from_bytes(document.to_json_pretty().unwrap().as_bytes())
                .unwrap(),
            None
        );
    }

    /// A file written from a history kept as the World grew is the file
    /// the whole document writes, read back.
    #[test]
    fn a_world_file_written_from_a_kept_history_reads_as_the_document() {
        let mut document = WorldDocument::new(lived_archive()).with_display_title("Harbor");
        document.settle_checkpoint();
        let file = document.to_bytes().unwrap();
        let (read, history) = WorldDocument::from_bytes_kept(&file).unwrap();
        let history = history.expect("written as World Machine writes");
        assert_eq!(history.len(), 12);
        let again = WorldDocument::file_from_history(
            &read.metadata,
            read.archive.head(),
            &history,
            read.archive.checkpoint.as_ref(),
        )
        .unwrap();
        assert_eq!(WorldDocument::from_bytes(&again).unwrap(), read);
        assert_eq!(read, WorldDocument::from_bytes(&file).unwrap());
    }

    /// Deflated in pieces as the history grows, the file is one gzip
    /// stream that reads as the document, however many pieces it took.
    #[test]
    fn a_world_file_deflated_in_pieces_reads_as_the_document() {
        use world_persistence::{ArchivedEvent, ArchivedStateChange, ArchivedValue};
        let event = |id: u64| ArchivedEvent {
            id,
            kind: "lived".into(),
            world_time: id * 8,
            actor: Some(1),
            targets: Vec::new(),
            caused_by: Vec::new(),
            payload: Default::default(),
            changes: vec![ArchivedStateChange::SetComponent {
                entity: 1,
                key: format!("day.{}", id % 7),
                value: ArchivedValue::Text(format!("day {id} of many")),
            }],
        };
        let mut document = WorldDocument::new(lived_archive()).with_display_title("Harbor");
        document.archive.events.clear();
        let mut history = CompactHistory::default();
        let mut deflated = DeflatedHistory::default();
        for id in 1..=200 {
            let mark = history.mark();
            let text_len = history.written_text().len();
            history.push(&[event(id)]);
            document.archive.events.push(event(id));
            document.archive.world_time = id * 8;
            if id % 17 == 0 {
                // A save that fails goes back, and is made again.
                let _ = WorldDocument::file_from_deflated_history(
                    &document.metadata,
                    document.archive.head(),
                    &history,
                    &mut deflated,
                    None,
                )
                .unwrap();
                history.rewind(mark);
                deflated.go_back_to(text_len);
                history.push(&[event(id)]);
            }
            let file = WorldDocument::file_from_deflated_history(
                &document.metadata,
                document.archive.head(),
                &history,
                &mut deflated,
                None,
            )
            .unwrap();
            assert_eq!(
                WorldDocument::from_bytes(&file).unwrap(),
                document,
                "at {id}"
            );
            if id % 50 == 0 {
                let whole = WorldDocument::file_from_history(
                    &document.metadata,
                    document.archive.head(),
                    &history,
                    None,
                )
                .unwrap();
                assert!(
                    file.len() < whole.len() * 2,
                    "{} against {}",
                    file.len(),
                    whole.len()
                );
                // Handed to a Pack as it is, it is the document's archive.
                let mut json = Vec::new();
                flate2::read::DeflateDecoder::new(WorldDocument::deflated_json(&file).unwrap())
                    .read_to_end(&mut json)
                    .unwrap();
                assert_eq!(
                    WorldArchive::from_json_slice(&json).unwrap(),
                    document.archive
                );
            }
        }
    }

    #[test]
    fn a_world_file_keeps_its_json_deflated_as_the_pack_protocol_carries_it() {
        use flate2::read::DeflateDecoder;
        let mut document = WorldDocument::new(lived_archive()).with_display_title("Kept");
        document.settle_checkpoint();
        let file = document.to_bytes().unwrap();
        let deflated = WorldDocument::deflated_json(&file).expect("a plain gzip header");
        let mut json = Vec::new();
        DeflateDecoder::new(deflated)
            .read_to_end(&mut json)
            .unwrap();
        let read = WorldDocument::from_json(std::str::from_utf8(&json).unwrap()).unwrap();
        assert_eq!(read, WorldDocument::from_bytes(&file).unwrap());
        assert_eq!(WorldArchive::from_json_slice(&json).unwrap(), read.archive);

        assert!(WorldDocument::deflated_json(&json).is_none());
        assert!(WorldDocument::deflated_json(&file[..12]).is_none());
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
