//! What Home says it did, or could not do, and what it offers to put right.

#[allow(unused_imports)]
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HomeStatusTone {
    Info,
    Success,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HomeStatus {
    pub(crate) message: String,
    pub(crate) tone: HomeStatusTone,
    /// A World whose file failed to load, which can be put back to its last
    /// good save: the status then offers it (see `restore`).
    pub(crate) restore: Option<RestoreTarget>,
}

/// A World to put back to its last good save, and open again.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RestoreTarget {
    pub(crate) path: PathBuf,
    pub(crate) title: String,
    /// The Library World it is, if it is one; else a file opened by path.
    pub(crate) document: Option<WorldDocumentId>,
}

impl HomeStatus {
    pub(crate) fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tone: HomeStatusTone::Info,
            restore: None,
        }
    }

    pub(crate) fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tone: HomeStatusTone::Success,
            restore: None,
        }
    }

    pub(crate) fn error(message: impl Into<String>) -> Self {
        let message = message.into();
        diagnostics::error(format!("home: {message}"));
        Self {
            message,
            tone: HomeStatusTone::Error,
            restore: None,
        }
    }

    /// A World that failed to open with `error`: said so, with the offer to
    /// put it back to its last good save when it has one. A World the 1.0
    /// save contract refuses is told in the contract's own sentence, whole,
    /// so it reads in the player's language.
    pub(crate) fn open_failed(
        message: impl Into<String>,
        error: &LibraryError,
        target: RestoreTarget,
    ) -> Self {
        if error.save_age() != world_library::FormatAge::Current {
            return Self::error(error.to_string());
        }
        let offered = world_machine_desktop::restore::offers_restore(error, &target.path);
        Self {
            restore: offered.then_some(target),
            ..Self::error(message)
        }
    }
}
