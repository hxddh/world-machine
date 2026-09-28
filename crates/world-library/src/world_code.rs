//! World codes: a World's start and history as a short piece of text (or a
//! `.worldcode` file holding it) that anyone can open as a *visit*.
//!
//! A code is `wm1:` followed by the World document (its archive and its
//! name) as compact JSON, deflated and written in URL-safe base64 without
//! padding, then `.` and the CRC-32 of that JSON as eight hex digits:
//!
//! ```text
//! wm1:<base64url(deflate(json))>.<crc32 hex>
//! ```
//!
//! Whitespace anywhere in a code is ignored, so a code that a chat window
//! wrapped over several lines still opens. A code that is cut short or
//! changed fails its checksum and is refused with a plain sentence.
//!
//! A visit replays the archive through the World's own Pack: nothing is
//! decided again, only recorded events are applied. It has no file and no
//! place in the library, and it refuses every intent, so it can never
//! change the World it was made from.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::{Compression, Crc};
use world_document::{WorldDocument, WorldDocumentMetadata};
use world_host::{HostError, WorldRegistry, WorldSession};
use world_persistence::{WorldArchive, WorldPackRef};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

use crate::{required_archive, DurableWorldSession};

/// What every World code starts with; the digit is the code's format.
pub const WORLD_CODE_PREFIX: &str = "wm1:";
/// The file a World code is saved in.
pub const WORLD_CODE_SUFFIX: &str = ".worldcode";
/// The largest World a code will unpack to, so a hostile code cannot fill
/// memory: far more than years of any World's history.
const MAX_UNPACKED_BYTES: u64 = 256 * 1024 * 1024;

/// Why a World code could not be made or opened, in words a player can
/// read.
#[derive(Debug)]
pub enum WorldCodeError {
    /// The text is not a World code at all.
    NotACode,
    /// A World code from a newer World Machine.
    NewerFormat(String),
    /// Cut short, changed, or otherwise not whole.
    Damaged,
    /// The code's World needs a Pack that is not installed, or cannot be
    /// replayed by the one that is.
    CannotOpen(HostError),
    /// Someone tried to change a visit.
    ReadOnlyVisit,
    /// The World could not be written as a code.
    Unwritable(String),
    Io(std::io::Error),
}

impl fmt::Display for WorldCodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotACode => write!(
                f,
                "This is not a World code. A World code starts with {WORLD_CODE_PREFIX}"
            ),
            Self::NewerFormat(prefix) => write!(
                f,
                "This World code ({prefix}) was made by a newer World Machine"
            ),
            Self::Damaged => write!(
                f,
                "This World code is incomplete or damaged. Copy the whole code and try again"
            ),
            Self::CannotOpen(HostError::UnknownWorld(pack)) => write!(
                f,
                "This World needs the {pack} Pack, which is not installed"
            ),
            Self::CannotOpen(error) => write!(f, "This World cannot be opened here: {error}"),
            Self::ReadOnlyVisit => write!(
                f,
                "This is a visit: a copy that can be looked at but not changed"
            ),
            Self::Unwritable(reason) => write!(f, "Could not make a World code: {reason}"),
            Self::Io(error) => error.fmt(f),
        }
    }
}

impl Error for WorldCodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CannotOpen(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WorldCodeError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<HostError> for WorldCodeError {
    fn from(error: HostError) -> Self {
        Self::CannotOpen(error)
    }
}

/// The code for a World document: its archive, and its name if it has one.
/// Anything else the file keeps about it (where it came from, its summary
/// line) stays behind.
pub fn encode_world_code(document: &WorldDocument) -> Result<String, WorldCodeError> {
    let shared = WorldDocument {
        archive: document.archive.clone(),
        metadata: WorldDocumentMetadata {
            display_title: document.metadata.display_title.clone(),
            ..WorldDocumentMetadata::default()
        },
    };
    let pretty = shared
        .to_json_pretty()
        .map_err(|error| WorldCodeError::Unwritable(error.to_string()))?;
    let value: serde_json::Value = serde_json::from_str(&pretty)
        .map_err(|error| WorldCodeError::Unwritable(error.to_string()))?;
    let json = serde_json::to_vec(&value)
        .map_err(|error| WorldCodeError::Unwritable(error.to_string()))?;
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&json)?;
    let packed = encoder.finish()?;
    Ok(format!(
        "{WORLD_CODE_PREFIX}{}.{:08x}",
        URL_SAFE_NO_PAD.encode(packed),
        checksum(&json)
    ))
}

/// The code for an archive, under `name` if it has one.
pub fn world_code_for_archive(
    archive: WorldArchive,
    name: Option<String>,
) -> Result<String, WorldCodeError> {
    let mut document = WorldDocument::new(archive);
    document.metadata.display_title = name;
    encode_world_code(&document)
}

/// The World document a code holds. Refuses anything that is not a whole,
/// unchanged code.
pub fn decode_world_code(text: &str) -> Result<WorldDocument, WorldCodeError> {
    let code = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let Some(body) = code.strip_prefix(WORLD_CODE_PREFIX) else {
        return Err(newer_or_not_a_code(&code));
    };
    let (data, sum) = body.rsplit_once('.').ok_or(WorldCodeError::Damaged)?;
    if sum.len() != 8 {
        return Err(WorldCodeError::Damaged);
    }
    let expected = u32::from_str_radix(sum, 16).map_err(|_| WorldCodeError::Damaged)?;
    let packed = URL_SAFE_NO_PAD
        .decode(data)
        .map_err(|_| WorldCodeError::Damaged)?;
    let mut json = Vec::new();
    DeflateDecoder::new(packed.as_slice())
        .take(MAX_UNPACKED_BYTES + 1)
        .read_to_end(&mut json)
        .map_err(|_| WorldCodeError::Damaged)?;
    if json.len() as u64 > MAX_UNPACKED_BYTES || checksum(&json) != expected {
        return Err(WorldCodeError::Damaged);
    }
    let json = std::str::from_utf8(&json).map_err(|_| WorldCodeError::Damaged)?;
    WorldDocument::from_json(json).map_err(|_| WorldCodeError::Damaged)
}

/// Whether some text looks like a World code (of any format), so a pasted
/// line can be told apart from a path.
pub fn looks_like_world_code(text: &str) -> bool {
    let text = text.trim_start();
    text.starts_with(WORLD_CODE_PREFIX)
        || world_code_format(text).is_some_and(|version| version > 1)
}

/// Saves a code as a `.worldcode` file: the code and a newline.
pub fn write_world_code_file(path: &Path, code: &str) -> Result<(), WorldCodeError> {
    fs::write(path, format!("{code}\n"))?;
    Ok(())
}

/// Reads the code in a `.worldcode` file. Opening it never writes to it.
pub fn read_world_code_file(path: &Path) -> Result<WorldDocument, WorldCodeError> {
    let bytes = fs::read(path)?;
    let text = String::from_utf8(bytes).map_err(|_| WorldCodeError::NotACode)?;
    decode_world_code(&text)
}

fn checksum(bytes: &[u8]) -> u32 {
    let mut crc = Crc::new();
    crc.update(bytes);
    crc.sum()
}

/// `wmN:` → N.
fn world_code_format(code: &str) -> Option<u32> {
    let rest = code.strip_prefix("wm")?;
    let (digits, _) = rest.split_once(':')?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

fn newer_or_not_a_code(code: &str) -> WorldCodeError {
    match world_code_format(code) {
        Some(version) if version > 1 => WorldCodeError::NewerFormat(format!("wm{version}:")),
        _ => WorldCodeError::NotACode,
    }
}

impl DurableWorldSession {
    /// This World's code, as it stands now. Reads the live session only;
    /// the World's file is not touched.
    pub fn world_code(&self) -> Result<String, WorldCodeError> {
        let archive = required_archive(self.session.as_ref())
            .map_err(|error| WorldCodeError::Unwritable(error.to_string()))?;
        let document = WorldDocument {
            archive,
            metadata: WorldDocumentMetadata {
                display_title: self.metadata.display_title.clone(),
                ..WorldDocumentMetadata::default()
            },
        };
        encode_world_code(&document)
    }
}

/// A World opened from a code: a copy to look around, which cannot be
/// changed and is never saved anywhere.
pub struct WorldVisit {
    name: Option<String>,
    archive: WorldArchive,
    session: Box<dyn WorldSession>,
}

impl WorldVisit {
    /// Opens the World a code holds, replaying its recorded history.
    pub fn open_code(text: &str, registry: &WorldRegistry) -> Result<Self, WorldCodeError> {
        Self::open_document(decode_world_code(text)?, registry)
    }

    /// Opens the World a `.worldcode` file holds. The file is only read.
    pub fn open_file(path: &Path, registry: &WorldRegistry) -> Result<Self, WorldCodeError> {
        Self::open_document(read_world_code_file(path)?, registry)
    }

    pub fn open_document(
        document: WorldDocument,
        registry: &WorldRegistry,
    ) -> Result<Self, WorldCodeError> {
        let session = registry.open_archive(&document.archive)?;
        Ok(Self {
            name: document.metadata.display_title,
            archive: document.archive,
            session,
        })
    }

    /// What the World is called: its owner's name for it, or its own.
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| self.session.snapshot().title)
    }

    pub fn pack(&self) -> WorldPackRef {
        self.session.pack()
    }

    pub fn snapshot(&self) -> ProjectionSnapshot {
        self.session.snapshot()
    }

    /// The archive the visit was opened from: the World's start and
    /// history, as recorded.
    pub fn archive(&self) -> &WorldArchive {
        &self.archive
    }

    /// The visit's own code, the same World as the code it came from.
    pub fn world_code(&self) -> Result<String, WorldCodeError> {
        world_code_for_archive(self.archive.clone(), self.name.clone())
    }

    /// A visit is looked at, never played: every intent is refused, and the
    /// World is left exactly as it was.
    pub fn handle(
        &mut self,
        intent: ProjectionIntent,
    ) -> Result<ProjectionSnapshot, WorldCodeError> {
        let _ = intent;
        Err(WorldCodeError::ReadOnlyVisit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_persistence::{WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};

    fn document() -> WorldDocument {
        let archive = WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new("world-machine.code-mock", "1"),
            world_time: 0,
            events: Vec::new(),
            pending: Vec::new(),
        };
        WorldDocument::new(archive).with_display_title("Harbor Town")
    }

    #[test]
    fn a_code_decodes_to_the_same_document() {
        let document = document();
        let code = encode_world_code(&document).unwrap();
        assert!(code.starts_with("wm1:"), "{code}");
        assert!(looks_like_world_code(&code));
        assert_eq!(decode_world_code(&code).unwrap(), document);
        // Wrapped over lines by a chat window, it still opens.
        let wrapped = code
            .chars()
            .collect::<Vec<_>>()
            .chunks(20)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n  ");
        assert_eq!(decode_world_code(&wrapped).unwrap(), document);
    }

    #[test]
    fn only_the_name_travels_with_the_archive() {
        let mut document = document().with_display_summary("A private line");
        document.metadata.display_moves_alone = true;
        let decoded = decode_world_code(&encode_world_code(&document).unwrap()).unwrap();
        assert_eq!(decoded.archive, document.archive);
        assert_eq!(
            decoded.metadata.display_title.as_deref(),
            Some("Harbor Town")
        );
        assert_eq!(decoded.metadata.display_summary, None);
    }

    #[test]
    fn a_cut_or_changed_code_is_refused_plainly() {
        let code = encode_world_code(&document()).unwrap();
        for bad in [
            code[..code.len() - 1].to_string(),
            code[..code.len() / 2].to_string(),
            code.replacen(".", "", 1),
            {
                // One letter of the data changed.
                let mut chars = code.chars().collect::<Vec<_>>();
                let at = WORLD_CODE_PREFIX.len() + 3;
                chars[at] = if chars[at] == 'A' { 'B' } else { 'A' };
                chars.into_iter().collect()
            },
            "wm1:".to_string(),
            "wm1:.00000000".to_string(),
        ] {
            let error = decode_world_code(&bad).unwrap_err();
            assert!(matches!(error, WorldCodeError::Damaged), "{bad}: {error:?}");
            assert_eq!(
                error.to_string(),
                "This World code is incomplete or damaged. Copy the whole code and try again"
            );
        }
        assert!(matches!(
            decode_world_code("hello"),
            Err(WorldCodeError::NotACode)
        ));
        assert!(matches!(
            decode_world_code("wm7:abc.00000000"),
            Err(WorldCodeError::NewerFormat(_))
        ));
    }
}
