from pathlib import Path

p = Path('crates/world-pack-catalog/src/lib.rs')
s = p.read_text()

s = s.replace('use serde::{Deserialize, Serialize};', 'use serde::{Deserialize, Serialize};\nuse sha2::{Digest, Sha256};', 1)
s = s.replace('use world_pack_process::{ProcessPack, ProcessPackPin, ProcessPackSource};', 'use world_pack_process::{ProcessPack, ProcessPackPin, ProcessPackSource};\nuse world_pack_protocol::PackManifest;', 1)

s = s.replace('    pub active: bool,\n}', '    pub active: bool,\n    #[serde(default)]\n    pub managed: bool,\n}', 1)

# Normalize catalog path to an absolute path even before the file exists.
old_open = '''    pub fn open(path: impl AsRef<Path>) -> Result<Self, CatalogError> {
        let path = path.as_ref().to_path_buf();'''
new_open = '''    pub fn open(path: impl AsRef<Path>) -> Result<Self, CatalogError> {
        let path = absolute_path(path.as_ref())?;'''
if old_open not in s:
    raise SystemExit('open marker not found')
s = s.replace(old_open, new_open, 1)

old_validate = '''        document.validate()?;
        Ok(Self {
            path,
            entries: document.entries,
        })'''
new_validate = '''        document.validate()?;
        validate_managed_entries(&path, &document.entries)?;
        Ok(Self {
            path,
            entries: document.entries,
        })'''
if old_validate not in s:
    raise SystemExit('open validation marker not found')
s = s.replace(old_validate, new_validate, 1)

# Replace install_manifest body while preserving signature/doc comment.
start = s.index('    pub fn install_manifest(')
end = s.index('\n    pub fn set_enabled', start)
install = '''    pub fn install_manifest(
        &mut self,
        manifest_path: impl AsRef<Path>,
    ) -> Result<InstalledPack, CatalogError> {
        let source = ProcessPack::load(manifest_path).map_err(process_error)?;
        if !source.args.is_empty() {
            return Err(CatalogError::RuntimeArgumentsNotPinnable(
                source.descriptor.pack,
            ));
        }
        if self.entry(&source.descriptor.pack).is_some() {
            return Err(CatalogError::AlreadyInstalled(source.descriptor.pack));
        }

        // The explicit approval is materialized into a World Machine-owned copy.
        // The catalog never relies on the user's download/source path after this point.
        let managed = self.materialize_managed_pack(&source)?;
        let identity = managed.current_pin().map_err(process_error)?;
        let installed = InstalledPack {
            pack: managed.descriptor.pack.clone(),
            title: managed.descriptor.title.clone(),
            description: managed.descriptor.description.clone(),
            manifest_path: managed.manifest_path.clone(),
            command_path: managed.command.clone(),
            manifest_sha256: identity.manifest_sha256().into(),
            command_sha256: identity.command_sha256().into(),
            approval: PackApproval::ExplicitInstall,
            enabled: true,
            active: true,
            managed: true,
        };

        let mut entries = self.entries.clone();
        for entry in entries
            .iter_mut()
            .filter(|entry| entry.pack.id == installed.pack.id)
        {
            entry.active = false;
        }
        entries.push(installed.clone());
        sort_entries(&mut entries);
        if let Err(error) = self.commit(entries) {
            cleanup_managed_pack(&self.path, &installed);
            return Err(error);
        }
        Ok(installed)
    }
'''
s = s[:start] + install + s[end:]

# Uninstall: retain entry for cleanup, commit removal first, then best-effort remove only verified managed dir.
old_uninstall_tail = '''        let entries = self
            .entries
            .iter()
            .filter(|entry| entry.pack != *pack)
            .cloned()
            .collect();
        self.commit(entries)
    }
'''
new_uninstall_tail = '''        let removed = self.entries[index].clone();
        let entries = self
            .entries
            .iter()
            .filter(|entry| entry.pack != *pack)
            .cloned()
            .collect();
        self.commit(entries)?;
        cleanup_managed_pack(&self.path, &removed);
        Ok(())
    }
'''
if old_uninstall_tail not in s:
    raise SystemExit('uninstall marker not found')
s = s.replace(old_uninstall_tail, new_uninstall_tail, 1)

# Insert materialization method before verified_pack.
marker = '    fn verified_pack(&self, entry: &InstalledPack) -> Result<ProcessPack, CatalogError> {'
method = '''    fn materialize_managed_pack(&self, source: &ProcessPack) -> Result<ProcessPack, CatalogError> {
        let final_dir = managed_pack_dir(&self.path, &source.descriptor.pack);
        if final_dir.try_exists().map_err(|error| CatalogError::Io {
            operation: "check managed Pack destination",
            path: final_dir.clone(),
            message: error.to_string(),
        })? {
            return Err(CatalogError::ManagedDestinationExists(
                source.descriptor.pack.clone(),
            ));
        }
        let store = managed_store_root(&self.path);
        fs::create_dir_all(&store).map_err(|error| CatalogError::Io {
            operation: "create managed Pack store",
            path: store.clone(),
            message: error.to_string(),
        })?;
        let nonce = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);
        let stage = store.join(format!(".install-{}-{nonce}.tmp", process::id()));
        fs::create_dir(&stage).map_err(|error| CatalogError::Io {
            operation: "create managed Pack staging directory",
            path: stage.clone(),
            message: error.to_string(),
        })?;

        let result = (|| {
            let program_name = managed_program_name(&source.command);
            let staged_program = stage.join(&program_name);
            fs::copy(&source.command, &staged_program).map_err(|error| CatalogError::Io {
                operation: "copy approved Pack executable",
                path: staged_program.clone(),
                message: error.to_string(),
            })?;
            let permissions = fs::metadata(&source.command)
                .map_err(|error| CatalogError::Io {
                    operation: "read approved Pack executable permissions",
                    path: source.command.clone(),
                    message: error.to_string(),
                })?
                .permissions();
            fs::set_permissions(&staged_program, permissions).map_err(|error| CatalogError::Io {
                operation: "set managed Pack executable permissions",
                path: staged_program.clone(),
                message: error.to_string(),
            })?;
            if let Ok(file) = OpenOptions::new().read(true).open(&staged_program) {
                file.sync_all().map_err(|error| CatalogError::Io {
                    operation: "sync managed Pack executable",
                    path: staged_program.clone(),
                    message: error.to_string(),
                })?;
            }

            let managed_manifest = PackManifest::process(
                source.descriptor.clone(),
                program_name,
                Vec::new(),
            );
            let manifest_path = stage.join("pack.world-pack.json");
            let mut manifest_json = managed_manifest
                .to_json_pretty()
                .map_err(|error| CatalogError::Json(error.to_string()))?
                .into_bytes();
            manifest_json.push(b'\n');
            let mut manifest_file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&manifest_path)
                .map_err(|error| CatalogError::Io {
                    operation: "create managed Pack manifest",
                    path: manifest_path.clone(),
                    message: error.to_string(),
                })?;
            manifest_file
                .write_all(&manifest_json)
                .and_then(|_| manifest_file.sync_all())
                .map_err(|error| CatalogError::Io {
                    operation: "write managed Pack manifest",
                    path: manifest_path.clone(),
                    message: error.to_string(),
                })?;
            drop(manifest_file);

            fs::rename(&stage, &final_dir).map_err(|error| CatalogError::Io {
                operation: "publish managed Pack",
                path: final_dir.clone(),
                message: error.to_string(),
            })?;
            sync_directory(&store);
            ProcessPack::load(final_dir.join("pack.world-pack.json")).map_err(process_error)
        })();

        if result.is_err() {
            let _ = fs::remove_dir_all(&stage);
        }
        result
    }

'''
if marker not in s:
    raise SystemExit('verified_pack marker not found')
s = s.replace(marker, method + marker, 1)

# Managed validation before generic entry validation helper.
marker = 'fn validate_entries(entries: &[InstalledPack]) -> Result<(), CatalogError> {'
helpers = '''fn absolute_path(path: &Path) -> Result<PathBuf, CatalogError> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    std::env::current_dir()
        .map(|current| current.join(path))
        .map_err(|error| CatalogError::Io {
            operation: "resolve catalog path",
            path: path.to_path_buf(),
            message: error.to_string(),
        })
}

fn managed_store_root(catalog_path: &Path) -> PathBuf {
    catalog_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .join("Installed")
}

fn managed_pack_key(pack: &WorldPackRef) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pack.id.as_bytes());
    hasher.update([0]);
    hasher.update(pack.version.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn managed_pack_dir(catalog_path: &Path, pack: &WorldPackRef) -> PathBuf {
    managed_store_root(catalog_path).join(managed_pack_key(pack))
}

fn managed_program_name(source: &Path) -> String {
    source
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!("program.{extension}"))
        .unwrap_or_else(|| "program".into())
}

fn validate_managed_entries(
    catalog_path: &Path,
    entries: &[InstalledPack],
) -> Result<(), CatalogError> {
    for entry in entries.iter().filter(|entry| entry.managed) {
        let expected = managed_pack_dir(catalog_path, &entry.pack);
        if entry.manifest_path != expected.join("pack.world-pack.json")
            || entry.command_path.parent() != Some(expected.as_path())
        {
            return Err(CatalogError::InvalidManagedPath(entry.pack.clone()));
        }
    }
    Ok(())
}

fn cleanup_managed_pack(catalog_path: &Path, entry: &InstalledPack) {
    if !entry.managed {
        return;
    }
    let expected = managed_pack_dir(catalog_path, &entry.pack);
    if entry.manifest_path.parent() == Some(expected.as_path())
        && entry.command_path.parent() == Some(expected.as_path())
    {
        let _ = fs::remove_dir_all(expected);
        sync_directory(&managed_store_root(catalog_path));
    }
}

fn sync_directory(path: &Path) {
    #[cfg(unix)]
    if let Ok(directory) = OpenOptions::new().read(true).open(path) {
        let _ = directory.sync_all();
    }
}

'''
if marker not in s:
    raise SystemExit('validate_entries marker not found')
s = s.replace(marker, helpers + marker, 1)

# validate_entries: managed may be either true/false; nothing else required.
# Extend errors.
s = s.replace('    InvalidManagedPath(WorldPackRef),', '    InvalidManagedPath(WorldPackRef),') if 'InvalidManagedPath' in s else s.replace(
    '    InvalidStoredPath(WorldPackRef),',
    '    InvalidStoredPath(WorldPackRef),\n    InvalidManagedPath(WorldPackRef),\n    ManagedDestinationExists(WorldPackRef),',
    1,
)

# Add display arms after InvalidStoredPath.
needle = '''            Self::InvalidStoredPath(pack) => write!(
                f,
                "installed Pack contains a non-absolute path: {}@{}",
                pack.id, pack.version
            ),
'''
addition = needle + '''            Self::InvalidManagedPath(pack) => write!(
                f,
                "managed Pack paths do not match the catalog-owned store: {}@{}",
                pack.id, pack.version
            ),
            Self::ManagedDestinationExists(pack) => write!(
                f,
                "managed Pack destination already exists for {}@{}",
                pack.id, pack.version
            ),
'''
if needle not in s:
    raise SystemExit('error display marker not found')
s = s.replace(needle, addition, 1)

# Existing install persistence assertion gets managed semantics.
s = s.replace(
    '        assert!(installed.command_path.is_absolute());\n        assert_eq!(installed.manifest_sha256.len(), 64);',
    '        assert!(installed.command_path.is_absolute());\n        assert!(installed.managed);\n        assert_ne!(installed.manifest_path, manifest.canonicalize().unwrap());\n        assert_eq!(installed.manifest_sha256.len(), 64);',
    1,
)

# Add managed-store tests before duplicate install.
marker = '''    #[test]
    fn duplicate_exact_install_is_rejected_without_mutating_catalog() {'''
new_tests = '''    #[test]
    fn managed_install_survives_source_files_being_removed() {
        let root = temp_dir("managed-survives-source");
        let catalog_path = root.join("catalog.json");
        let manifest = write_pack(&root, "fixture", "1");
        let source = ProcessPack::load(&manifest).unwrap();
        let source_command = source.command.clone();
        let mut catalog = PackCatalog::open(&catalog_path).unwrap();
        let installed = catalog.install_manifest(&manifest).unwrap();

        fs::remove_file(&manifest).unwrap();
        fs::remove_file(&source_command).unwrap();

        assert!(installed.manifest_path.exists());
        assert!(installed.command_path.exists());
        assert_eq!(catalog.availability(&installed.pack), PackAvailability::Ready);
        assert_eq!(catalog.trusted_source().unwrap().packs().len(), 1);
    }

    #[test]
    fn uninstall_removes_only_managed_copy_and_preserves_source() {
        let root = temp_dir("managed-uninstall");
        let catalog_path = root.join("catalog.json");
        let manifest = write_pack(&root, "fixture", "1");
        let source = ProcessPack::load(&manifest).unwrap();
        let source_command = source.command.clone();
        let mut catalog = PackCatalog::open(&catalog_path).unwrap();
        let installed = catalog.install_manifest(&manifest).unwrap();
        let managed_dir = installed.manifest_path.parent().unwrap().to_path_buf();

        catalog.uninstall(&installed.pack).unwrap();

        assert!(!managed_dir.exists());
        assert!(manifest.exists());
        assert!(source_command.exists());
    }

'''
if marker not in s:
    raise SystemExit('test insertion marker not found')
s = s.replace(marker, new_tests + marker, 1)

p.write_text(s)
