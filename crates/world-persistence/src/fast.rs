//! A reader for the compact encoding as World Machine writes it: one line,
//! no spaces, keys before events. It reads a long history straight into the
//! archive, several times faster than through a tree of JSON values.
//!
//! Anything it does not expect it leaves alone and says so (`None`), and
//! the general reader (`read.rs`) reads the archive instead, or says what
//! is wrong with it. So this reader only has to be right when it succeeds,
//! and it succeeds only on text it reads to the end, strictly.

use serde_json::value::RawValue;
use std::collections::BTreeMap;

use crate::{
    history::CompactHistory, ArchivedCheckpoint, ArchivedEntity, ArchivedEvent, ArchivedRelation,
    ArchivedScheduledAction, ArchivedStateChange, ArchivedValue, WorldArchive, WorldPackRef,
    WORLD_ARCHIVE_COMPACT_VERSION, WORLD_ARCHIVE_VERSION,
};

/// The archive `json` holds, and the raw JSON of its top-level field
/// `extra`, if it is a compact archive this reader reads through.
pub(crate) fn archive<'a>(
    json: &'a str,
    extra: Option<&str>,
) -> Option<(WorldArchive, Option<&'a RawValue>)> {
    archive_kept(json, extra).map(|(archive, extra, _)| (archive, extra))
}

/// [`archive`], with its history as it is written there, to write on from.
pub(crate) fn archive_kept<'a>(
    json: &'a str,
    extra: Option<&str>,
) -> Option<(WorldArchive, Option<&'a RawValue>, CompactHistory)> {
    let mut reader = Reader {
        text: json,
        bytes: json.as_bytes(),
        at: 0,
        scratch: Vec::new(),
    };
    let (archive, extra, written) = reader.top(extra)?;
    reader.whitespace();
    if reader.at != reader.bytes.len() {
        return None;
    }
    let (keys, events) = written;
    let history = CompactHistory::from_written(
        keys,
        json.get(events)?,
        archive.events.len(),
        archive
            .events
            .last()
            .map(|event| (event.id, event.world_time)),
    );
    Some((archive, extra, history))
}

/// The keys an archive's events were written with, and where their text is.
type Written = (Vec<String>, std::ops::Range<usize>);

struct Reader<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
    /// Changes read for the event at hand, kept between events.
    scratch: Vec<ArchivedStateChange>,
}

impl<'a> Reader<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn eat(&mut self, byte: u8) -> Option<()> {
        if self.peek()? == byte {
            self.at += 1;
            Some(())
        } else {
            None
        }
    }

    /// Whether the next byte is `byte`, taking it if so.
    fn next_is(&mut self, byte: u8) -> bool {
        let is = self.peek() == Some(byte);
        if is {
            self.at += 1;
        }
        is
    }

    fn word(&mut self, word: &[u8]) -> Option<()> {
        if self.bytes.get(self.at..self.at + word.len())? == word {
            self.at += word.len();
            Some(())
        } else {
            None
        }
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.at += 1;
        }
    }

    /// Where, from here, the next byte is that ends a string, begins an
    /// escape or may not be in a string at all.
    fn string_stop(&self) -> Option<usize> {
        self.bytes
            .get(self.at..)?
            .iter()
            .position(|&byte| byte == b'"' || byte == b'\\' || byte < 0x20)
            .map(|offset| self.at + offset)
    }

    /// A string written without escapes, as the text it is.
    fn plain(&mut self) -> Option<&'a str> {
        self.eat(b'"')?;
        let start = self.at;
        self.at = self.string_stop()?;
        if self.bytes[self.at] != b'"' {
            return None;
        }
        let plain = self.text.get(start..self.at)?;
        self.at += 1;
        Some(plain)
    }

    fn string(&mut self) -> Option<String> {
        self.eat(b'"')?;
        let start = self.at;
        self.at = self.string_stop()?;
        match self.bytes[self.at] {
            b'"' => {
                let text = self.text.get(start..self.at)?.to_owned();
                self.at += 1;
                return Some(text);
            }
            b'\\' => {}
            _ => return None,
        }
        // With escapes: the common ones are read here; `\u` is left to the
        // general reader.
        let mut text = String::from(self.text.get(start..self.at)?);
        loop {
            match self.bytes[self.at] {
                b'"' => {
                    self.at += 1;
                    return Some(text);
                }
                b'\\' => {}
                _ => return None,
            }
            self.at += 1;
            text.push(match self.peek()? {
                b'"' => '"',
                b'\\' => '\\',
                b'/' => '/',
                b'b' => '\u{8}',
                b'f' => '\u{c}',
                b'n' => '\n',
                b'r' => '\r',
                b't' => '\t',
                _ => return None,
            });
            self.at += 1;
            let run = self.at;
            self.at = self.string_stop()?;
            text.push_str(self.text.get(run..self.at)?);
        }
    }

    /// A whole number from 0, written as JSON writes one.
    fn id(&mut self) -> Option<u64> {
        let start = self.at;
        let mut value: u64 = 0;
        while let Some(digit @ b'0'..=b'9') = self.peek() {
            value = value
                .checked_mul(10)?
                .checked_add(u64::from(digit - b'0'))?;
            self.at += 1;
        }
        let digits = self.at - start;
        // No digits, a leading zero, or a fraction or exponent: not an id.
        if digits == 0
            || (digits > 1 && self.bytes[start] == b'0')
            || matches!(self.peek(), Some(b'.' | b'e' | b'E'))
        {
            return None;
        }
        Some(value)
    }

    fn integer(&mut self) -> Option<i64> {
        if self.next_is(b'-') {
            let magnitude = self.id()?;
            if magnitude == 0 {
                return None;
            }
            0_i64.checked_sub_unsigned(magnitude)
        } else {
            i64::try_from(self.id()?).ok()
        }
    }

    fn ids(&mut self) -> Option<Vec<u64>> {
        self.eat(b'[')?;
        let mut ids = Vec::new();
        if self.next_is(b']') {
            return Some(ids);
        }
        loop {
            ids.push(self.id()?);
            if self.next_is(b']') {
                return Some(ids);
            }
            self.eat(b',')?;
        }
    }

    fn value(&mut self) -> Option<ArchivedValue> {
        Some(match self.peek()? {
            b'n' => {
                self.word(b"null")?;
                ArchivedValue::Null
            }
            b't' => {
                self.word(b"true")?;
                ArchivedValue::Bool(true)
            }
            b'f' => {
                self.word(b"false")?;
                ArchivedValue::Bool(false)
            }
            b'"' => ArchivedValue::Text(self.string()?),
            b'[' => {
                self.at += 1;
                let mut values = Vec::new();
                if !self.next_is(b']') {
                    loop {
                        values.push(self.value()?);
                        if self.next_is(b']') {
                            break;
                        }
                        self.eat(b',')?;
                    }
                }
                ArchivedValue::List(values)
            }
            b'{' => self.map_value()?,
            b'-' | b'0'..=b'9' => ArchivedValue::Integer(self.integer()?),
            _ => return None,
        })
    }

    /// An object read as a value: an entity, a wrapped map or a map.
    fn map_value(&mut self) -> Option<ArchivedValue> {
        self.eat(b'{')?;
        let mut values = BTreeMap::new();
        if self.next_is(b'}') {
            return Some(ArchivedValue::Map(values));
        }
        let first = self.string()?;
        self.eat(b':')?;
        // A map led by `#` given it again reads as whatever it was given
        // last, which is the general reader's to tell.
        let led_by_hash = first == "#";
        if first == "#" && matches!(self.peek(), Some(b'0'..=b'9')) {
            let id = self.id()?;
            if self.next_is(b'}') {
                return Some(ArchivedValue::Entity(id));
            }
            values.insert(first, ArchivedValue::Integer(i64::try_from(id).ok()?));
        } else if first == "#" {
            // Not an entity, so there must be more to the map.
            let value = self.value()?;
            if self.peek()? == b'}' {
                return None;
            }
            values.insert(first, value);
        } else if first == "{}" {
            let inner = self.map()?;
            if self.next_is(b'}') {
                return Some(ArchivedValue::Map(inner));
            }
            // A wrapper beside other keys is read as a value of its own,
            // which is the general reader's to do.
            return None;
        } else {
            let value = self.value()?;
            values.insert(first, value);
            if self.next_is(b'}') {
                return Some(ArchivedValue::Map(values));
            }
        }
        loop {
            self.eat(b',')?;
            let key = self.string()?;
            if led_by_hash && key == "#" {
                return None;
            }
            self.eat(b':')?;
            let value = self.value()?;
            values.insert(key, value);
            if self.next_is(b'}') {
                return Some(ArchivedValue::Map(values));
            }
        }
    }

    /// An object whose values are each a value: components, properties and
    /// payloads.
    fn map(&mut self) -> Option<BTreeMap<String, ArchivedValue>> {
        self.eat(b'{')?;
        let mut values = BTreeMap::new();
        if self.next_is(b'}') {
            return Some(values);
        }
        loop {
            let key = self.string()?;
            self.eat(b':')?;
            let value = self.value()?;
            values.insert(key, value);
            if self.next_is(b'}') {
                return Some(values);
            }
            self.eat(b',')?;
        }
    }

    fn key(&mut self, keys: &[String]) -> Option<String> {
        if self.peek()? == b'"' {
            self.string()
        } else {
            keys.get(usize::try_from(self.id()?).ok()?).cloned()
        }
    }

    fn changes(&mut self, keys: &[String]) -> Option<Vec<ArchivedStateChange>> {
        self.eat(b'[')?;
        let mut out = std::mem::take(&mut self.scratch);
        out.clear();
        if !self.next_is(b']') {
            loop {
                self.change(keys, &mut out)?;
                if self.next_is(b']') {
                    break;
                }
                self.eat(b',')?;
            }
        }
        // In a list of their own length; the scratch keeps its room.
        let mut changes = Vec::with_capacity(out.len());
        changes.append(&mut out);
        self.scratch = out;
        Some(changes)
    }

    fn change(&mut self, keys: &[String], out: &mut Vec<ArchivedStateChange>) -> Option<()> {
        self.eat(b'[')?;
        let tag = self.plain()?;
        self.eat(b',')?;
        match tag {
            "e+" => {
                let id = self.id()?;
                self.eat(b',')?;
                let kind = self.string()?;
                self.eat(b',')?;
                let components = self.map()?;
                out.push(ArchivedStateChange::CreateEntity {
                    entity: ArchivedEntity {
                        id,
                        kind,
                        components,
                    },
                });
            }
            "e-" => out.push(ArchivedStateChange::RemoveEntity { entity: self.id()? }),
            "r+" => {
                let id = self.id()?;
                self.eat(b',')?;
                let kind = self.string()?;
                self.eat(b',')?;
                let from = self.id()?;
                self.eat(b',')?;
                let to = self.id()?;
                self.eat(b',')?;
                let properties = self.map()?;
                out.push(ArchivedStateChange::CreateRelation {
                    relation: ArchivedRelation {
                        id,
                        kind,
                        from,
                        to,
                        properties,
                    },
                });
            }
            "r-" => out.push(ArchivedStateChange::RemoveRelation {
                relation: self.id()?,
            }),
            "s" | "rs" => {
                let target = self.id()?;
                loop {
                    self.eat(b',')?;
                    let key = self.key(keys)?;
                    self.eat(b',')?;
                    let value = self.value()?;
                    out.push(if tag == "s" {
                        ArchivedStateChange::SetComponent {
                            entity: target,
                            key,
                            value,
                        }
                    } else {
                        ArchivedStateChange::SetRelationProperty {
                            relation: target,
                            key,
                            value,
                        }
                    });
                    if self.peek()? == b']' {
                        break;
                    }
                }
            }
            "u" | "ru" => {
                let target = self.id()?;
                loop {
                    self.eat(b',')?;
                    let key = self.key(keys)?;
                    out.push(if tag == "u" {
                        ArchivedStateChange::RemoveComponent {
                            entity: target,
                            key,
                        }
                    } else {
                        ArchivedStateChange::RemoveRelationProperty {
                            relation: target,
                            key,
                        }
                    });
                    if self.peek()? == b']' {
                        break;
                    }
                }
            }
            _ => return None,
        }
        self.eat(b']')
    }

    fn events(&mut self, keys: &[String]) -> Option<Vec<ArchivedEvent>> {
        self.eat(b'[')?;
        let mut events: Vec<ArchivedEvent> = Vec::new();
        if self.next_is(b']') {
            return Some(events);
        }
        loop {
            let previous = events.last().map(|event| (event.id, event.world_time));
            let event = self.event(keys, previous)?;
            events.push(event);
            if self.next_is(b']') {
                events.shrink_to_fit();
                return Some(events);
            }
            self.eat(b',')?;
        }
    }

    fn event(&mut self, keys: &[String], previous: Option<(u64, u64)>) -> Option<ArchivedEvent> {
        self.eat(b'{')?;
        let mut id = None;
        let mut kind = None;
        let mut world_time = None;
        let mut actor = None;
        let mut targets = Vec::new();
        let mut caused_by = Vec::new();
        let mut payload = BTreeMap::new();
        let mut changes = Vec::new();
        if !self.next_is(b'}') {
            loop {
                let field = self.plain()?;
                self.eat(b':')?;
                match field {
                    "i" => id = Some(self.id()?),
                    "k" => kind = Some(self.string()?),
                    "t" => world_time = Some(self.id()?),
                    "a" => actor = Some(self.id()?),
                    "g" => targets = self.ids()?,
                    "c" => caused_by = self.ids()?,
                    "p" => payload = self.map()?,
                    "x" => changes = self.changes(keys)?,
                    _ => return None,
                }
                if self.next_is(b'}') {
                    break;
                }
                self.eat(b',')?;
            }
        }
        Some(ArchivedEvent {
            id: match id {
                Some(id) => id,
                None => previous?.0.checked_add(1)?,
            },
            kind: kind?,
            world_time: match world_time {
                Some(time) => time,
                None => previous?.1,
            },
            actor,
            targets,
            caused_by,
            payload,
            changes,
        })
    }

    fn checkpoint(&mut self, keys: &[String]) -> Option<ArchivedCheckpoint> {
        self.eat(b'{')?;
        let mut events = None;
        let mut last_event = None;
        let mut world_time = None;
        let mut changes = None;
        if !self.next_is(b'}') {
            loop {
                let field = self.plain()?;
                self.eat(b':')?;
                match field {
                    "events" => events = Some(self.id()?),
                    "last_event" => last_event = Some(self.id()?),
                    "world_time" => world_time = Some(self.id()?),
                    "changes" => changes = Some(self.changes(keys)?),
                    _ => return None,
                }
                if self.next_is(b'}') {
                    break;
                }
                self.eat(b',')?;
            }
        }
        Some(ArchivedCheckpoint {
            events: usize::try_from(events?).ok()?,
            last_event: last_event?,
            world_time: world_time?,
            changes: changes?,
        })
    }

    /// The text of the JSON value that starts here, found by its brackets
    /// and strings alone: whoever reads it checks it.
    fn skip(&mut self) -> Option<&'a str> {
        let start = self.at;
        let mut open = Vec::new();
        loop {
            match self.peek()? {
                b'"' => {
                    self.at += 1;
                    loop {
                        match *self.bytes.get(self.at)? {
                            b'"' => break,
                            b'\\' => self.at += 2,
                            _ => self.at += 1,
                        }
                    }
                    self.at += 1;
                }
                byte @ (b'[' | b'{') => {
                    open.push(if byte == b'[' { b']' } else { b'}' });
                    self.at += 1;
                }
                byte @ (b']' | b'}') => {
                    if open.pop()? != byte {
                        return None;
                    }
                    self.at += 1;
                }
                b',' if open.is_empty() => break,
                _ => self.at += 1,
            }
            if open.is_empty() && matches!(self.peek(), Some(b',' | b'}') | None) {
                break;
            }
        }
        self.text.get(start..self.at)
    }

    fn top(
        &mut self,
        extra: Option<&str>,
    ) -> Option<(WorldArchive, Option<&'a RawValue>, Written)> {
        self.eat(b'{')?;
        let mut format = None;
        let mut compact = false;
        let mut pack = None;
        let mut world_time = None;
        let mut pending = None;
        let mut keys: Option<Vec<String>> = None;
        let mut checkpoint = None;
        let mut events = None;
        let mut written = None;
        let mut extra_value = None;
        if self.next_is(b'}') {
            return None;
        }
        loop {
            let field = self.plain()?;
            self.eat(b':')?;
            match field {
                "format" => format = Some(self.string()?),
                "format_version" => {
                    compact = self.id()? == u64::from(WORLD_ARCHIVE_COMPACT_VERSION);
                    if !compact {
                        return None;
                    }
                }
                "pack" => pack = Some(serde_json::from_str::<WorldPackRef>(self.skip()?).ok()?),
                "world_time" => world_time = Some(self.id()?),
                "pending" => {
                    pending = Some(
                        serde_json::from_str::<Vec<ArchivedScheduledAction>>(self.skip()?).ok()?,
                    )
                }
                // Given twice, which keys the changes mean is the general
                // reader's to tell.
                "keys" if keys.is_some() => return None,
                "keys" => {
                    self.eat(b'[')?;
                    let mut names = Vec::new();
                    if !self.next_is(b']') {
                        loop {
                            names.push(self.string()?);
                            if self.next_is(b']') {
                                break;
                            }
                            self.eat(b',')?;
                        }
                    }
                    keys = Some(names);
                }
                // Changes name their keys by place, so both need the keys
                // first, as they are written; without any, none is named so.
                "checkpoint" => {
                    checkpoint = Some(self.checkpoint(keys.as_deref().unwrap_or_default())?)
                }
                "events" => {
                    let start = self.at + 1;
                    events = Some(self.events(keys.as_deref().unwrap_or_default())?);
                    written = Some((keys.clone().unwrap_or_default(), start..self.at - 1));
                }
                name if Some(name) == extra => {
                    extra_value = Some(serde_json::from_str::<&RawValue>(self.skip()?).ok()?);
                }
                _ => {
                    serde_json::from_str::<serde::de::IgnoredAny>(self.skip()?).ok()?;
                }
            }
            if self.next_is(b'}') {
                break;
            }
            self.eat(b',')?;
        }
        if !compact {
            return None;
        }
        let archive = WorldArchive {
            format: format?,
            format_version: WORLD_ARCHIVE_VERSION,
            pack: pack?,
            world_time: world_time?,
            events: events?,
            pending: pending.unwrap_or_default(),
            checkpoint,
        };
        Some((archive, extra_value, written?))
    }
}
