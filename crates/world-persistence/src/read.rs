//! Reading an archive from its JSON text.
//!
//! A compact archive as World Machine writes it is read by the reader made
//! for it (`fast.rs`), straight into the archive: a three-year World's
//! history is millions of values, and a tree of JSON values for them costs
//! several times what the archive itself does, in time and in memory.
//! Anything else (an archive in the tagged encoding, one written by hand,
//! or a damaged one) is read, or refused, by the JSON value reader, as
//! World Machine always has; the tests hold the two readers to each other.

use serde_json::value::RawValue;

use crate::{PersistenceError, WorldArchive};

/// Reads an archive, and the raw JSON of the top-level field `extra` when
/// it has one (a World document keeps its own there).
pub(crate) fn archive_from_slice(
    json: &[u8],
    extra: Option<&str>,
) -> Result<(WorldArchive, Option<Box<RawValue>>), PersistenceError> {
    let fast = std::str::from_utf8(json)
        .ok()
        .and_then(|text| crate::fast::archive(text, extra));
    if let Some((archive, extra)) = fast {
        archive.validate_header()?;
        return Ok((archive, extra.map(ToOwned::to_owned)));
    }
    general(json, extra)
}

/// [`archive_from_slice`], with the archive's history as it is written, to
/// write on from, when it is written as World Machine writes one.
pub(crate) fn archive_kept(
    json: &[u8],
    extra: Option<&str>,
) -> Result<crate::KeptArchive, PersistenceError> {
    let fast = std::str::from_utf8(json)
        .ok()
        .and_then(|text| crate::fast::archive_kept(text, extra));
    if let Some((archive, extra, history)) = fast {
        archive.validate_header()?;
        return Ok((archive, extra.map(ToOwned::to_owned), Some(history)));
    }
    let (archive, extra) = general(json, extra)?;
    Ok((archive, extra, None))
}

fn general(
    json: &[u8],
    extra: Option<&str>,
) -> Result<(WorldArchive, Option<Box<RawValue>>), PersistenceError> {
    let value: serde_json::Value = serde_json::from_slice(json).map_err(PersistenceError::Json)?;
    let archive = WorldArchive::from_json_value(&value)?;
    let extra = extra
        .and_then(|name| value.get(name))
        .map(|value| RawValue::from_string(value.to_string()))
        .transpose()
        .map_err(PersistenceError::Json)?;
    Ok((archive, extra))
}
