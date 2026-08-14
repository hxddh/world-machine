from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    file.write_text(text.replace(old, new, 1))


path = "crates/world-library/src/lib.rs"
replace_once(
    path,
    "use std::path::{Path, PathBuf};\n",
    "use std::path::{Path, PathBuf};\nuse std::time::{SystemTime, UNIX_EPOCH};\n",
    "time imports",
)
replace_once(
    path,
    '''    fn current_revision(
        &self,
        id: &WorldDocumentId,
    ) -> Result<Option<DocumentRevision>, LibraryError> {
        for path in [self.path(id), self.legacy_path(id)] {
            if let Some(revision) = revision_if_exists(&path)? {
                return Ok(Some(revision));
            }
        }
        Ok(None)
    }

    pub fn list(&self) -> Result<Vec<WorldDocumentSummary>, LibraryError> {''',
    '''    fn current_revision(
        &self,
        id: &WorldDocumentId,
    ) -> Result<Option<DocumentRevision>, LibraryError> {
        for path in [self.path(id), self.legacy_path(id)] {
            if let Some(revision) = revision_if_exists(&path)? {
                return Ok(Some(revision));
            }
        }
        Ok(None)
    }

    fn document_modified_time(
        &self,
        id: &WorldDocumentId,
    ) -> Result<Option<SystemTime>, LibraryError> {
        for path in [self.path(id), self.legacy_path(id)] {
            match fs::metadata(path) {
                Ok(metadata) => return Ok(Some(metadata.modified()?)),
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(LibraryError::Io(error)),
            }
        }
        Ok(None)
    }

    /// List Library Worlds with the most recently persisted document first.
    /// File modification time is Library browsing metadata only; it is never
    /// written into World state or used by replay. Ties are ordered by stable
    /// document id so the result remains deterministic for equal timestamps.
    pub fn list(&self) -> Result<Vec<WorldDocumentSummary>, LibraryError> {''',
    "modified-time helper",
)
replace_once(
    path,
    '''        let mut documents = Vec::new();
        for id in ids {
            let Some(document) = self.load_document(&id)? else {
                continue;
            };
            documents.push(summary(id, &document));
        }
        Ok(documents)
    }''',
    '''        let mut documents = Vec::new();
        for id in ids {
            let Some(document) = self.load_document(&id)? else {
                continue;
            };
            let modified = self.document_modified_time(&id)?.unwrap_or(UNIX_EPOCH);
            documents.push((modified, summary(id, &document)));
        }
        documents.sort_by(|(left_modified, left), (right_modified, right)| {
            right_modified
                .cmp(left_modified)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(documents
            .into_iter()
            .map(|(_modified, document)| document)
            .collect())
    }''',
    "recent-first list",
)
replace_once(
    path,
    '''    #[test]
    fn export_and_import_round_trip_a_portable_world_file() {''',
    '''    #[test]
    fn library_lists_most_recently_persisted_world_first() {
        let root = temp_root("recent-first");
        let library = WorldLibrary::new(root.clone());
        let older = WorldDocumentId::new("older").unwrap();
        let recent = WorldDocumentId::new("recent").unwrap();

        library.save(&older, &mock_archive(1)).unwrap();
        library.save(&recent, &mock_archive(2)).unwrap();
        let older_file = File::options()
            .write(true)
            .open(library.path(&older))
            .unwrap();
        older_file
            .set_times(
                fs::FileTimes::new()
                    .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(10)),
            )
            .unwrap();
        let recent_file = File::options()
            .write(true)
            .open(library.path(&recent))
            .unwrap();
        recent_file
            .set_times(
                fs::FileTimes::new()
                    .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(20)),
            )
            .unwrap();

        assert_eq!(
            library
                .list()
                .unwrap()
                .iter()
                .map(|document| document.id.as_str())
                .collect::<Vec<_>>(),
            vec!["recent", "older"]
        );

        library.save(&older, &mock_archive(3)).unwrap();
        assert_eq!(
            library
                .list()
                .unwrap()
                .iter()
                .map(|document| document.id.as_str())
                .collect::<Vec<_>>(),
            vec!["older", "recent"]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn equal_library_modification_times_fall_back_to_document_id() {
        let root = temp_root("recent-tie");
        let library = WorldLibrary::new(root.clone());
        let beta = WorldDocumentId::new("beta").unwrap();
        let alpha = WorldDocumentId::new("alpha").unwrap();

        library.save(&beta, &mock_archive(2)).unwrap();
        library.save(&alpha, &mock_archive(1)).unwrap();
        let tied = UNIX_EPOCH + std::time::Duration::from_secs(30);
        for id in [&alpha, &beta] {
            File::options()
                .write(true)
                .open(library.path(id))
                .unwrap()
                .set_times(fs::FileTimes::new().set_modified(tied))
                .unwrap();
        }

        assert_eq!(
            library
                .list()
                .unwrap()
                .iter()
                .map(|document| document.id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "beta"]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn export_and_import_round_trip_a_portable_world_file() {''',
    "recent-order tests",
)
