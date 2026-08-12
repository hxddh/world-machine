use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use world_document::{WorldBranchCause, WorldDocument, WorldLineage, WorldParent};
use world_library::{LibraryError, WorldDocumentId, WorldLibrary};
use world_persistence::WorldPackRef;

const MAX_LINEAGE_DEPTH: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineageTrace {
    pub document: WorldDocumentId,
    pub steps: Vec<LineageStep>,
    pub terminal: LineageTerminal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineageStep {
    pub document: WorldDocumentId,
    pub parent: WorldParent,
    pub branch: WorldBranchCause,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LineageTerminal {
    Root {
        document: WorldDocumentId,
    },
    MissingParent {
        reference: String,
        parent: WorldParent,
    },
    ExternalParent {
        reference: Option<String>,
        parent: WorldParent,
    },
    Cycle {
        document: WorldDocumentId,
    },
    DepthLimit {
        document: WorldDocumentId,
    },
}

pub fn trace_library_lineage(
    library: &WorldLibrary,
    document_id: &WorldDocumentId,
) -> Result<LineageTrace, LineageError> {
    let first = library
        .load_document(document_id)?
        .ok_or_else(|| LineageError::UnknownDocument(document_id.clone()))?;
    let mut visited = BTreeSet::new();
    visited.insert(document_id.to_string());

    let mut current_id = document_id.clone();
    let mut current = first;
    let mut steps = Vec::new();

    for _ in 0..MAX_LINEAGE_DEPTH {
        let Some(lineage) = current.metadata.lineage.clone() else {
            return Ok(LineageTrace {
                document: document_id.clone(),
                steps,
                terminal: LineageTerminal::Root {
                    document: current_id,
                },
            });
        };

        steps.push(LineageStep {
            document: current_id.clone(),
            parent: lineage.parent.clone(),
            branch: lineage.branch.clone(),
        });

        let Some(reference) = lineage.parent.document.clone() else {
            return Ok(LineageTrace {
                document: document_id.clone(),
                steps,
                terminal: LineageTerminal::ExternalParent {
                    reference: None,
                    parent: lineage.parent,
                },
            });
        };

        let Some(parent_id) = library_document_id_from_reference(&reference) else {
            return Ok(LineageTrace {
                document: document_id.clone(),
                steps,
                terminal: LineageTerminal::ExternalParent {
                    reference: Some(reference),
                    parent: lineage.parent,
                },
            });
        };

        if !visited.insert(parent_id.to_string()) {
            return Ok(LineageTrace {
                document: document_id.clone(),
                steps,
                terminal: LineageTerminal::Cycle {
                    document: parent_id,
                },
            });
        }

        let Some(parent_document) = library.load_document(&parent_id)? else {
            return Ok(LineageTrace {
                document: document_id.clone(),
                steps,
                terminal: LineageTerminal::MissingParent {
                    reference,
                    parent: lineage.parent,
                },
            });
        };

        current_id = parent_id;
        current = parent_document;
    }

    Ok(LineageTrace {
        document: document_id.clone(),
        steps,
        terminal: LineageTerminal::DepthLimit {
            document: current_id,
        },
    })
}

pub fn compact_lineage_summary(lineage: &WorldLineage) -> String {
    let source = lineage
        .parent
        .document
        .clone()
        .unwrap_or_else(|| pack_label(&lineage.parent.pack));
    let branch = match &lineage.branch {
        WorldBranchCause::Strategy {
            choice_title,
            horizon,
            ..
        } => format!("Strategy: {choice_title} · {horizon} periods"),
        WorldBranchCause::Fork { label } => label
            .as_ref()
            .map(|label| format!("Fork: {label}"))
            .unwrap_or_else(|| "Fork".into()),
    };
    format!(
        "From {source} · {branch} · branched at t={}",
        lineage.parent.world_time
    )
}

fn library_document_id_from_reference(reference: &str) -> Option<WorldDocumentId> {
    let reference = reference
        .strip_suffix(".world.json")
        .or_else(|| reference.strip_suffix(".world"))
        .unwrap_or(reference);
    WorldDocumentId::new(reference.to_owned()).ok()
}

fn pack_label(pack: &WorldPackRef) -> String {
    format!("{}@{}", pack.id, pack.version)
}

#[derive(Debug)]
pub enum LineageError {
    Library(LibraryError),
    UnknownDocument(WorldDocumentId),
}

impl fmt::Display for LineageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Library(error) => error.fmt(f),
            Self::UnknownDocument(document) => write!(f, "unknown World document: {document}"),
        }
    }
}

impl Error for LineageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Library(error) => Some(error),
            Self::UnknownDocument(_) => None,
        }
    }
}

impl From<LibraryError> for LineageError {
    fn from(error: LibraryError) -> Self {
        Self::Library(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_document::{WorldDocument, WorldDocumentMetadata};
    use world_persistence::{WorldArchive, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};

    fn archive(time: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new("world-machine.lineage-test", "1"),
            world_time: time,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn strategy_lineage(parent: &str, time: u64, title: &str) -> WorldLineage {
        WorldLineage {
            parent: WorldParent {
                document: Some(parent.into()),
                pack: WorldPackRef::new("world-machine.lineage-test", "1"),
                world_time: time,
                event_count: 0,
            },
            branch: WorldBranchCause::Strategy {
                choice_id: format!("choice-{title}"),
                choice_title: title.into(),
                horizon: 20,
            },
        }
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        env::temp_dir().join(format!(
            "world-machine-lineage-{}-{nonce}-{label}",
            process::id()
        ))
    }

    #[test]
    fn traces_multiple_saved_futures_back_to_the_library_root() {
        let root = temp_root("chain");
        let library = WorldLibrary::new(root.clone());
        let source = WorldDocumentId::new("source").unwrap();
        let future_a = WorldDocumentId::new("future-a").unwrap();
        let future_b = WorldDocumentId::new("future-b").unwrap();

        library
            .create_from_document(source.clone(), &WorldDocument::new(archive(100)))
            .unwrap();
        library
            .create_from_document(
                future_a.clone(),
                &WorldDocument::new(archive(30))
                    .with_lineage(strategy_lineage("source", 10, "A")),
            )
            .unwrap();
        library
            .create_from_document(
                future_b.clone(),
                &WorldDocument::new(archive(50))
                    .with_lineage(strategy_lineage("future-a", 30, "B")),
            )
            .unwrap();

        let trace = trace_library_lineage(&library, &future_b).unwrap();

        assert_eq!(trace.steps.len(), 2);
        assert_eq!(trace.steps[0].document, future_b);
        assert_eq!(trace.steps[1].document, future_a);
        assert!(matches!(
            trace.terminal,
            LineageTerminal::Root { document } if document == source
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn parent_can_keep_evolving_after_the_branch_point() {
        let root = temp_root("evolved-parent");
        let library = WorldLibrary::new(root.clone());
        let source = WorldDocumentId::new("source").unwrap();
        let future = WorldDocumentId::new("future").unwrap();

        library
            .create_from_document(source.clone(), &WorldDocument::new(archive(100)))
            .unwrap();
        library
            .create_from_document(
                future.clone(),
                &WorldDocument::new(archive(30))
                    .with_lineage(strategy_lineage("source", 10, "Lean")),
            )
            .unwrap();

        let trace = trace_library_lineage(&library, &future).unwrap();

        assert_eq!(trace.steps[0].parent.world_time, 10);
        assert!(matches!(
            trace.terminal,
            LineageTerminal::Root { document } if document == source
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn world_suffix_parent_references_resolve_to_library_ids() {
        let root = temp_root("suffix");
        let library = WorldLibrary::new(root.clone());
        let source = WorldDocumentId::new("source").unwrap();
        let future = WorldDocumentId::new("future").unwrap();

        library
            .create_from_document(source.clone(), &WorldDocument::new(archive(0)))
            .unwrap();
        library
            .create_from_document(
                future.clone(),
                &WorldDocument::new(archive(20))
                    .with_lineage(strategy_lineage("source.world", 0, "A")),
            )
            .unwrap();

        let trace = trace_library_lineage(&library, &future).unwrap();
        assert!(matches!(
            trace.terminal,
            LineageTerminal::Root { document } if document == source
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reports_missing_and_external_parents_without_losing_the_edge() {
        let root = temp_root("detached");
        let library = WorldLibrary::new(root.clone());
        let missing = WorldDocumentId::new("missing-child").unwrap();
        let external = WorldDocumentId::new("external-child").unwrap();

        library
            .create_from_document(
                missing.clone(),
                &WorldDocument::new(archive(20))
                    .with_lineage(strategy_lineage("gone", 5, "A")),
            )
            .unwrap();
        let mut external_lineage = strategy_lineage("/Users/example/Shared World.world", 7, "B");
        external_lineage.parent.document = Some("/Users/example/Shared World.world".into());
        library
            .create_from_document(
                external.clone(),
                &WorldDocument::new(archive(30)).with_lineage(external_lineage),
            )
            .unwrap();

        let missing_trace = trace_library_lineage(&library, &missing).unwrap();
        assert!(matches!(
            missing_trace.terminal,
            LineageTerminal::MissingParent { reference, .. } if reference == "gone"
        ));
        let external_trace = trace_library_lineage(&library, &external).unwrap();
        assert!(matches!(
            external_trace.terminal,
            LineageTerminal::ExternalParent { reference: Some(reference), .. }
                if reference == "/Users/example/Shared World.world"
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn detects_lineage_cycles() {
        let root = temp_root("cycle");
        let library = WorldLibrary::new(root.clone());
        let a = WorldDocumentId::new("a").unwrap();
        let b = WorldDocumentId::new("b").unwrap();

        library
            .create_from_document(
                a.clone(),
                &WorldDocument::new(archive(1)).with_lineage(strategy_lineage("b", 0, "A")),
            )
            .unwrap();
        library
            .create_from_document(
                b.clone(),
                &WorldDocument::new(archive(2)).with_lineage(strategy_lineage("a", 1, "B")),
            )
            .unwrap();

        let trace = trace_library_lineage(&library, &a).unwrap();
        assert!(matches!(
            trace.terminal,
            LineageTerminal::Cycle { document } if document == a
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn compact_summary_is_generic_and_stable() {
        let lineage = strategy_lineage("Source.world", 42, "Lean reopen");
        assert_eq!(
            compact_lineage_summary(&lineage),
            "From Source.world · Strategy: Lean reopen · 20 periods · branched at t=42"
        );

        let detached = WorldLineage {
            parent: WorldParent {
                document: None,
                pack: WorldPackRef::new("world-machine.mock", "3"),
                world_time: 8,
                event_count: 2,
            },
            branch: WorldBranchCause::Fork { label: None },
        };
        assert_eq!(
            compact_lineage_summary(&detached),
            "From world-machine.mock@3 · Fork · branched at t=8"
        );
    }

    #[test]
    fn unknown_starting_document_is_an_error() {
        let root = temp_root("unknown");
        let library = WorldLibrary::new(root.clone());
        let missing = WorldDocumentId::new("missing").unwrap();

        assert!(matches!(
            trace_library_lineage(&library, &missing),
            Err(LineageError::UnknownDocument(document)) if document == missing
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn metadata_without_lineage_is_a_root() {
        let root = temp_root("root");
        let library = WorldLibrary::new(root.clone());
        let id = WorldDocumentId::new("root").unwrap();
        let document = WorldDocument {
            archive: archive(0),
            metadata: WorldDocumentMetadata::default(),
        };
        library.create_from_document(id.clone(), &document).unwrap();

        let trace = trace_library_lineage(&library, &id).unwrap();
        assert!(trace.steps.is_empty());
        assert!(matches!(
            trace.terminal,
            LineageTerminal::Root { document } if document == id
        ));
        let _ = fs::remove_dir_all(root);
    }
}
