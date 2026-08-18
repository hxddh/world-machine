use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;
use world_host::HostError;
use world_investigation::ComparisonQueryExecutor;
use world_persistence::{PersistenceError, WorldArchive};
use world_projection::ProjectionSnapshot;
use world_query::{
    execute_comparison_query_request, EvidenceComparisonQueryRequest,
    EvidenceComparisonQueryResponse, QueryError,
};

pub struct LocalArchiveComparisonExecutor {
    left: ProjectionSnapshot,
    right: ProjectionSnapshot,
}

impl LocalArchiveComparisonExecutor {
    pub fn new(left: ProjectionSnapshot, right: ProjectionSnapshot) -> Self {
        Self { left, right }
    }

    pub fn from_archive_paths(
        left_path: &Path,
        right_path: &Path,
    ) -> Result<Self, LocalArchiveComparisonOpenError> {
        let left_json =
            fs::read_to_string(left_path).map_err(LocalArchiveComparisonOpenError::ReadLeft)?;
        let right_json =
            fs::read_to_string(right_path).map_err(LocalArchiveComparisonOpenError::ReadRight)?;
        let left_archive = WorldArchive::from_json(&left_json)
            .map_err(LocalArchiveComparisonOpenError::ParseLeft)?;
        let right_archive = WorldArchive::from_json(&right_json)
            .map_err(LocalArchiveComparisonOpenError::ParseRight)?;
        Self::from_archives(&left_archive, &right_archive)
    }

    pub fn from_archives(
        left_archive: &WorldArchive,
        right_archive: &WorldArchive,
    ) -> Result<Self, LocalArchiveComparisonOpenError> {
        let registry =
            world_builtins::registry().map_err(LocalArchiveComparisonOpenError::Registry)?;
        let left_session = registry
            .open_archive(left_archive)
            .map_err(LocalArchiveComparisonOpenError::OpenLeft)?;
        let right_session = registry
            .open_archive(right_archive)
            .map_err(LocalArchiveComparisonOpenError::OpenRight)?;
        Ok(Self::new(left_session.snapshot(), right_session.snapshot()))
    }

    pub fn left(&self) -> &ProjectionSnapshot {
        &self.left
    }

    pub fn right(&self) -> &ProjectionSnapshot {
        &self.right
    }
}

impl ComparisonQueryExecutor for LocalArchiveComparisonExecutor {
    type Error = QueryError;

    fn execute(
        &mut self,
        request: &EvidenceComparisonQueryRequest,
    ) -> Result<EvidenceComparisonQueryResponse, Self::Error> {
        execute_comparison_query_request(&self.left, &self.right, request)
    }
}

#[derive(Debug)]
pub enum LocalArchiveComparisonOpenError {
    ReadLeft(std::io::Error),
    ReadRight(std::io::Error),
    ParseLeft(PersistenceError),
    ParseRight(PersistenceError),
    Registry(HostError),
    OpenLeft(HostError),
    OpenRight(HostError),
}

impl fmt::Display for LocalArchiveComparisonOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadLeft(error) => write!(f, "failed to read left World archive: {error}"),
            Self::ReadRight(error) => write!(f, "failed to read right World archive: {error}"),
            Self::ParseLeft(error) => write!(f, "failed to parse left World archive: {error}"),
            Self::ParseRight(error) => write!(f, "failed to parse right World archive: {error}"),
            Self::Registry(error) => write!(f, "failed to build local World registry: {error}"),
            Self::OpenLeft(error) => write!(f, "failed to open left World archive: {error}"),
            Self::OpenRight(error) => write!(f, "failed to open right World archive: {error}"),
        }
    }
}

impl Error for LocalArchiveComparisonOpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadLeft(error) | Self::ReadRight(error) => Some(error),
            Self::ParseLeft(error) | Self::ParseRight(error) => Some(error),
            Self::Registry(error) | Self::OpenLeft(error) | Self::OpenRight(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn opens_builtin_archive_pair_and_owns_snapshots() {
        let registry = world_builtins::registry().unwrap();
        let descriptor = registry.descriptors().into_iter().next().unwrap();
        let session = registry.create(&descriptor.pack.id).unwrap();
        let archive = session.archive().unwrap().unwrap();
        let path = temp_world_path("shared");
        fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();

        let executor = LocalArchiveComparisonExecutor::from_archive_paths(&path, &path).unwrap();
        assert_eq!(executor.left().title, executor.right().title);
        assert_eq!(executor.left().world_time, executor.right().world_time);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_errors_preserve_left_right_attribution() {
        let missing_left = temp_world_path("missing-left");
        let missing_right = temp_world_path("missing-right");
        let error = LocalArchiveComparisonExecutor::from_archive_paths(&missing_left, &missing_right)
            .err()
            .unwrap();
        assert!(matches!(
            error,
            LocalArchiveComparisonOpenError::ReadLeft(_)
        ));
    }

    fn temp_world_path(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "world-machine-m215-local-{}-{nonce}-{label}.world",
            std::process::id()
        ))
    }
}
