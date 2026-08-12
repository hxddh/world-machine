use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DocumentRevision {
    len: u64,
    first: u64,
    second: u64,
}

impl DocumentRevision {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        let mut first = DefaultHasher::new();
        0x574f_524c_445f_3031_u64.hash(&mut first);
        bytes.hash(&mut first);

        let mut second = DefaultHasher::new();
        0x574f_524c_445f_3032_u64.hash(&mut second);
        bytes.hash(&mut second);

        Self {
            len: bytes.len() as u64,
            first: first.finish(),
            second: second.finish(),
        }
    }
}

// Keep Save As as a private implementation detail of the durable document
// state machinery. The public API remains an inherent method on
// DurableWorldSession, not a new public module or Host/Projection concept.
#[path = "save_as.rs"]
mod save_as;

// Keep Pack-defined background progression inside the same private durable
// document state machinery. The public surface remains an inherent method on
// DurableWorldSession and preserves the existing candidate/persist/commit model.
#[path = "revision/background.rs"]
mod background;

// Expose the current live archive through the same private durable document
// machinery. The archive still comes from the Host's integrity-checked session
// boundary and does not read or mutate the document target.
#[path = "revision/archive.rs"]
mod archive;

// Materialize branch/strategy documents through Library's existing atomic write
// machinery while preserving no-clobber creation semantics.
#[path = "revision/archive_create.rs"]
mod archive_create;

// Fork a durable World through the same document revision boundary. Forking is
// a Library/document operation: it snapshots the current World and records a new
// immediate-parent lineage without changing Host, Projection, or Pack semantics.
#[path = "revision/fork.rs"]
mod fork;

#[cfg(test)]
#[path = "revision/metadata_regression.rs"]
mod metadata_regression;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_changes_change_the_revision() {
        assert_ne!(
            DocumentRevision::from_bytes(b"world-a"),
            DocumentRevision::from_bytes(b"world-b")
        );
    }

    #[test]
    fn identical_content_has_the_same_revision() {
        assert_eq!(
            DocumentRevision::from_bytes(b"same-world"),
            DocumentRevision::from_bytes(b"same-world")
        );
    }
}
