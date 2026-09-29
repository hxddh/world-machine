//! A World's history kept written in the compact encoding as it grows, so a
//! World saved after every change writes out only what it recorded since
//! it was last saved, not every event again.

use serde::Serialize;

use crate::compact::{CompactChanges, CompactEvents, Keys};
use crate::{
    ArchivedCheckpoint, ArchivedEvent, ArchivedScheduledAction, PersistenceError, WorldPackRef,
    WORLD_ARCHIVE_COMPACT_VERSION, WORLD_ARCHIVE_FORMAT,
};

/// A history written event by event in the compact encoding, as a World
/// file keeps it: the names its changes set, and its events, each written
/// against the one before it.
#[derive(Clone, Debug, Default)]
pub struct CompactHistory {
    keys: Keys<'static>,
    /// The events written so far, separated by commas, without brackets.
    written: Vec<u8>,
    events: usize,
    /// The id and world time of the last event written.
    last: Option<(u64, u64)>,
}

/// How far a [`CompactHistory`] was written, to go back to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryMark {
    keys: usize,
    written: usize,
    events: usize,
    last: Option<(u64, u64)>,
}

/// What a World file says about a World beside its history.
#[derive(Clone, Copy, Debug)]
pub struct ArchiveHead<'a> {
    pub pack: &'a WorldPackRef,
    pub world_time: u64,
    pub pending: &'a [ArchivedScheduledAction],
}

impl CompactHistory {
    /// A history with these events written.
    pub fn of(events: &[ArchivedEvent]) -> Self {
        let mut history = Self::default();
        history.push(events);
        history
    }

    /// A history as a compact archive wrote it: its `keys`, and the text of
    /// its `events` between the brackets, holding `events` events of which
    /// the last had `last` as its id and world time.
    pub(crate) fn from_written(
        keys: Vec<String>,
        text: &str,
        events: usize,
        last: Option<(u64, u64)>,
    ) -> Self {
        Self {
            keys: Keys::listed(keys),
            written: text.as_bytes().to_vec(),
            events,
            last,
        }
    }

    /// How many events are written.
    pub fn len(&self) -> usize {
        self.events
    }

    pub fn is_empty(&self) -> bool {
        self.events == 0
    }

    /// The id and world time of the last event written.
    pub fn last_event(&self) -> Option<(u64, u64)> {
        self.last
    }

    /// How much the written history holds, in bytes.
    pub fn written_bytes(&self) -> usize {
        self.written.len()
    }

    /// Writes the events that came after those written.
    pub fn push(&mut self, events: &[ArchivedEvent]) {
        if events.is_empty() {
            return;
        }
        for event in events {
            self.keys.gather_owned(&event.changes);
        }
        let run = CompactEvents {
            events,
            keys: &self.keys,
            previous: self.last,
        };
        let text = serde_json::to_vec(&run).expect("an archive's events write as JSON");
        // Without the brackets of the list they were written as.
        let inner = &text[1..text.len() - 1];
        if !self.written.is_empty() {
            self.written.push(b',');
        }
        self.written.extend_from_slice(inner);
        self.events += events.len();
        let last = &events[events.len() - 1];
        self.last = Some((last.id, last.world_time));
    }

    /// Where the history stands, to go back to with [`Self::rewind`].
    pub fn mark(&self) -> HistoryMark {
        HistoryMark {
            keys: self.keys.names().len(),
            written: self.written.len(),
            events: self.events,
            last: self.last,
        }
    }

    /// Forgets everything written since `mark`.
    pub fn rewind(&mut self, mark: HistoryMark) {
        self.keys.truncate(mark.keys);
        self.written.truncate(mark.written);
        self.events = mark.events;
        self.last = mark.last;
    }

    /// The archive of this history in the compact encoding, as one JSON
    /// object, as [`crate::WorldArchive::to_compact_json_with`] writes an
    /// archive with these events, with `head` and `checkpoint`, and with
    /// `extra` fields alongside. Only the names may be listed in another
    /// order; it reads back as the same archive.
    pub fn to_compact_json(
        &self,
        head: ArchiveHead<'_>,
        checkpoint: Option<&ArchivedCheckpoint>,
        extra: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<Vec<u8>, PersistenceError> {
        let (mut json, rest) = self.around_events(head, checkpoint, extra)?;
        json.reserve(self.written.len() + rest.len());
        json.extend_from_slice(&self.written);
        json.extend_from_slice(&rest);
        Ok(json)
    }

    /// The events as written: their JSON between the brackets of the list.
    pub fn written_text(&self) -> &[u8] {
        &self.written
    }

    /// [`Self::to_compact_json`] without the events' text: what comes
    /// before it, and what after, so the events can be written (or packed)
    /// apart from the rest.
    pub fn around_events(
        &self,
        head: ArchiveHead<'_>,
        checkpoint: Option<&ArchivedCheckpoint>,
        extra: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<(Vec<u8>, Vec<u8>), PersistenceError> {
        if head.pack.id.trim().is_empty() || head.pack.version.trim().is_empty() {
            return Err(PersistenceError::InvalidPack);
        }
        // A checkpoint's changes may name keys no event written does (one
        // carried from a history kept elsewhere): those are listed after.
        let widened;
        let keys = match checkpoint {
            Some(checkpoint) => {
                let mut keys = self.keys.clone();
                keys.gather_owned(&checkpoint.changes);
                widened = keys;
                &widened
            }
            None => &self.keys,
        };
        let head_out = HeadOut {
            format: WORLD_ARCHIVE_FORMAT,
            format_version: WORLD_ARCHIVE_COMPACT_VERSION,
            pack: head.pack,
            world_time: head.world_time,
            pending: head.pending,
            keys: keys.names(),
            checkpoint: checkpoint.map(|checkpoint| CheckpointOut {
                events: checkpoint.events,
                last_event: checkpoint.last_event,
                world_time: checkpoint.world_time,
                changes: CompactChanges {
                    changes: &checkpoint.changes,
                    keys,
                },
            }),
        };
        let mut before = Vec::with_capacity(64 * 1024);
        serde_json::to_writer(&mut before, &head_out).map_err(PersistenceError::Json)?;
        // The head ends with its closing brace; the events and the extra
        // fields follow in its place.
        before.pop();
        before.extend_from_slice(b",\"events\":[");
        let mut after = vec![b']'];
        if !extra.is_empty() {
            let fields = serde_json::to_vec(extra).map_err(PersistenceError::Json)?;
            after.push(b',');
            after.extend_from_slice(&fields[1..fields.len() - 1]);
        }
        after.push(b'}');
        Ok((before, after))
    }
}

#[derive(Serialize)]
struct HeadOut<'a> {
    format: &'a str,
    format_version: u32,
    pack: &'a WorldPackRef,
    world_time: u64,
    pending: &'a [ArchivedScheduledAction],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    keys: &'a [std::borrow::Cow<'static, str>],
    #[serde(skip_serializing_if = "Option::is_none")]
    checkpoint: Option<CheckpointOut<'a>>,
}

#[derive(Serialize)]
struct CheckpointOut<'a> {
    events: usize,
    last_event: u64,
    world_time: u64,
    changes: CompactChanges<'a, 'a>,
}
