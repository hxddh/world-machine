use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use world_pack_bundle::{PackBundle, PackBundleHeader, PACK_BUNDLE_SUFFIX};
use world_pack_process::{ProcessPack, ProcessPackPin, ProcessPackSource};
use world_pack_protocol::{PackDescriptor, PackManifest};
use world_persistence::WorldPackRef;

pub const PACK_CATALOG_FORMAT: &str = "world-machine-pack-catalog";
pub const PACK_CATALOG_VERSION: u32 = 1;

static TEMP_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstalledPack {
    pub pack: WorldPackRef,
    pub title: String,
    pub description: String,
    pub manifest_path: PathBuf,
    pub command_path: PathBuf,
    pub manifest_sha256: String,
    pub command_sha256: String,
    pub approval: PackApproval,
    pub enabled: bool,
    pub active: bool,
    #[serde(default)]
    pub managed: bool,
}

impl InstalledPack {
    fn expected_pin(&self) -> ProcessPackPin {
        ProcessPackPin::new(self.manifest_sha256.clone(), self.command_sha256.clone())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackApproval {
    ExplicitInstall,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackAvailability {
    Ready,
    Disabled,
    Invalid { reason: String },
    MissingVersion { installed_versions: Vec<String> },
    NotInstalled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackInstallKind {
    PortableBundle,
    DeveloperManifest,
}

impl PackInstallKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::PortableBundle => "Portable .worldpack",
            Self::DeveloperManifest => "Developer manifest",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PackInstallPreview {
    source_path: PathBuf,
    kind: PackInstallKind,
    pack: WorldPackRef,
    title: String,
    description: String,
    runtime_name: String,
    program_bytes: u64,
    program_sha256: String,
    evidence: PackInstallEvidence,
}

impl PackInstallPreview {
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn kind(&self) -> PackInstallKind {
        self.kind
    }

    pub fn pack(&self) -> &WorldPackRef {
        &self.pack
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn runtime_name(&self) -> &str {
        &self.runtime_name
    }

    pub fn program_bytes(&self) -> u64 {
        self.program_bytes
    }

    pub fn program_sha256(&self) -> &str {
        &self.program_sha256
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PackInstallEvidence {
    Bundle {
        header: PackBundleHeader,
    },
    Manifest {
        descriptor: PackDescriptor,
        command_path: PathBuf,
        pin: ProcessPackPin,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct PackCatalogDocument {
    format: String,
    format_version: u32,
    entries: Vec<InstalledPack>,
}

impl PackCatalogDocument {
    fn new(entries: Vec<InstalledPack>) -> Self {
        Self {
            format: PACK_CATALOG_FORMAT.into(),
            format_version: PACK_CATALOG_VERSION,
            entries,
        }
    }

    fn validate(&self) -> Result<(), CatalogError> {
        if self.format != PACK_CATALOG_FORMAT {
            return Err(CatalogError::UnsupportedFormat(self.format.clone()));
        }
        if self.format_version != PACK_CATALOG_VERSION {
            return Err(CatalogError::UnsupportedVersion(self.format_version));
        }
        validate_entries(&self.entries)
    }
}

#[derive(Clone, Debug)]
pub struct PackCatalog {
    path: PathBuf,
    entries: Vec<InstalledPack>,
}

impl PackCatalog {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CatalogError> {
        let path = absolute_path(path.as_ref())?;
        if !path.exists() {
            return Ok(Self {
                path,
                entries: Vec::new(),
            });
        }
        let json = fs::read_to_string(&path).map_err(|error| CatalogError::Io {
            operation: "read catalog",
            path: path.clone(),
            message: error.to_string(),
        })?;
        let document = serde_json::from_str::<PackCatalogDocument>(&json)
            .map_err(|error| CatalogError::Json(error.to_string()))?;
        document.validate()?;
        validate_managed_entries(&path, &document.entries)?;
        Ok(Self {
            path,
            entries: document.entries,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn entries(&self) -> &[InstalledPack] {
        &self.entries
    }

    pub fn entry(&self, pack: &WorldPackRef) -> Option<&InstalledPack> {
        self.entries.iter().find(|entry| entry.pack == *pack)
    }

    /// Inspect installable Pack content without launching it or mutating the catalog.
    /// The returned preview contains private evidence binding a later approval to the
    /// exact descriptor/runtime/content identity that was reviewed.
    pub fn inspect_install(
        &self,
        source_path: impl AsRef<Path>,
    ) -> Result<PackInstallPreview, CatalogError> {
        let source_path = source_path.as_ref();
        if source_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(PACK_BUNDLE_SUFFIX))
        {
            self.inspect_bundle(source_path)
        } else {
            self.inspect_manifest(source_path)
        }
    }

    /// Explicit installation is the trust decision. Legacy/direct callers still get
    /// the same behavior, but the operation now goes through the same inspect + exact
    /// revalidation path used by the Desktop review UI.
    pub fn install_manifest(
        &mut self,
        manifest_path: impl AsRef<Path>,
    ) -> Result<InstalledPack, CatalogError> {
        let preview = self.inspect_manifest(manifest_path.as_ref())?;
        self.install_reviewed(&preview)
    }

    /// Install a portable `.worldpack` without executing any code from the bundle.
    pub fn install_bundle(
        &mut self,
        bundle_path: impl AsRef<Path>,
    ) -> Result<InstalledPack, CatalogError> {
        let preview = self.inspect_bundle(bundle_path.as_ref())?;
        self.install_reviewed(&preview)
    }

    /// Install only if the source still represents the exact executable identity that
    /// was inspected. If source content changes between review and approval, no Pack is
    /// added to the catalog and any managed copy is removed.
    pub fn install_reviewed(
        &mut self,
        preview: &PackInstallPreview,
    ) -> Result<InstalledPack, CatalogError> {
        if self.entry(&preview.pack).is_some() {
            return Err(CatalogError::AlreadyInstalled(preview.pack.clone()));
        }

        match &preview.evidence {
            PackInstallEvidence::Bundle { header } => {
                let bundle = PackBundle::open(&preview.source_path).map_err(bundle_error)?;
                if bundle.header() != header {
                    return Err(reviewed_content_changed(
                        &preview.pack,
                        "portable bundle header changed after review",
                    ));
                }
                let managed = self.materialize_managed_bundle(bundle)?;
                self.record_managed_install(managed)
            }
            PackInstallEvidence::Manifest {
                descriptor,
                command_path,
                pin,
            } => {
                let source = ProcessPack::load(&preview.source_path).map_err(process_error)?;
                if !source.args.is_empty() {
                    return Err(CatalogError::RuntimeArgumentsNotPinnable(
                        source.descriptor.pack,
                    ));
                }
                let current_pin = source.current_pin().map_err(process_error)?;
                if source.descriptor != *descriptor
                    || source.command != *command_path
                    || current_pin != *pin
                {
                    return Err(reviewed_content_changed(
                        &preview.pack,
                        "developer manifest or executable changed after review",
                    ));
                }

                let managed = self.materialize_managed_pack(&source)?;
                self.verify_reviewed_managed_program(&managed, pin.command_sha256())?;
                self.record_managed_install(managed)
            }
        }
    }

    fn inspect_bundle(&self, bundle_path: &Path) -> Result<PackInstallPreview, CatalogError> {
        let source_path = bundle_path
            .canonicalize()
            .map_err(|error| CatalogError::Io {
                operation: "resolve Pack bundle",
                path: bundle_path.to_path_buf(),
                message: error.to_string(),
            })?;
        let bundle = PackBundle::open(&source_path).map_err(bundle_error)?;
        let header = bundle.header().clone();
        let descriptor = &header.manifest.descriptor;
        if self.entry(&descriptor.pack).is_some() {
            return Err(CatalogError::AlreadyInstalled(descriptor.pack.clone()));
        }
        Ok(PackInstallPreview {
            source_path,
            kind: PackInstallKind::PortableBundle,
            pack: descriptor.pack.clone(),
            title: descriptor.title.clone(),
            description: descriptor.description.clone(),
            runtime_name: bundle.program_name().to_owned(),
            program_bytes: header.program_bytes,
            program_sha256: header.program_sha256.clone(),
            evidence: PackInstallEvidence::Bundle { header },
        })
    }

    fn inspect_manifest(&self, manifest_path: &Path) -> Result<PackInstallPreview, CatalogError> {
        let source = ProcessPack::load(manifest_path).map_err(process_error)?;
        if !source.args.is_empty() {
            return Err(CatalogError::RuntimeArgumentsNotPinnable(
                source.descriptor.pack,
            ));
        }
        if self.entry(&source.descriptor.pack).is_some() {
            return Err(CatalogError::AlreadyInstalled(source.descriptor.pack));
        }
        let pin = source.current_pin().map_err(process_error)?;
        let program_bytes = fs::metadata(&source.command)
            .map_err(|error| CatalogError::Io {
                operation: "inspect Pack executable",
                path: source.command.clone(),
                message: error.to_string(),
            })?
            .len();
        let runtime_name = source
            .command
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| source.command.display().to_string());
        Ok(PackInstallPreview {
            source_path: source.manifest_path.clone(),
            kind: PackInstallKind::DeveloperManifest,
            pack: source.descriptor.pack.clone(),
            title: source.descriptor.title.clone(),
            description: source.descriptor.description.clone(),
            runtime_name,
            program_bytes,
            program_sha256: pin.command_sha256().into(),
            evidence: PackInstallEvidence::Manifest {
                descriptor: source.descriptor,
                command_path: source.command,
                pin,
            },
        })
    }

    pub fn set_enabled(&mut self, pack: &WorldPackRef, enabled: bool) -> Result<(), CatalogError> {
        let mut entries = self.entries.clone();
        let index = entries
            .iter()
            .position(|entry| entry.pack == *pack)
            .ok_or_else(|| CatalogError::NotInstalled(pack.clone()))?;

        if enabled {
            if entries[index].enabled {
                return Ok(());
            }
            let has_active = entries
                .iter()
                .any(|entry| entry.pack.id == pack.id && entry.enabled && entry.active);
            entries[index].enabled = true;
            entries[index].active = !has_active;
        } else {
            if !entries[index].enabled {
                return Ok(());
            }
            if entries[index].active
                && entries.iter().enumerate().any(|(candidate, entry)| {
                    candidate != index && entry.pack.id == pack.id && entry.enabled
                })
            {
                return Err(CatalogError::ActivePackRequiresReplacement(pack.clone()));
            }
            entries[index].enabled = false;
            entries[index].active = false;
        }
        self.commit(entries)
    }

    /// Explicitly choose which installed, enabled version is used for new Worlds.
    /// Version strings remain opaque; activation is a product decision, never a sort result.
    pub fn activate(&mut self, pack: &WorldPackRef) -> Result<(), CatalogError> {
        let mut entries = self.entries.clone();
        let index = entries
            .iter()
            .position(|entry| entry.pack == *pack)
            .ok_or_else(|| CatalogError::NotInstalled(pack.clone()))?;
        if !entries[index].enabled {
            return Err(CatalogError::DisabledCannotActivate(pack.clone()));
        }
        for entry in entries.iter_mut().filter(|entry| entry.pack.id == pack.id) {
            entry.active = false;
        }
        entries[index].active = true;
        self.commit(entries)
    }

    pub fn uninstall(&mut self, pack: &WorldPackRef) -> Result<(), CatalogError> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.pack == *pack)
            .ok_or_else(|| CatalogError::NotInstalled(pack.clone()))?;
        if self.entries[index].active
            && self.entries.iter().enumerate().any(|(candidate, entry)| {
                candidate != index && entry.pack.id == pack.id && entry.enabled
            })
        {
            return Err(CatalogError::ActivePackRequiresReplacement(pack.clone()));
        }
        let removed = self.entries[index].clone();
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

    /// Re-validate every enabled entry before it can become a Host source.
    /// The returned ProcessPack values also carry the stored pins, so the same
    /// content check runs again immediately before each child process launch.
    pub fn trusted_source(&self) -> Result<ProcessPackSource, CatalogError> {
        let mut entries = self
            .entries
            .iter()
            .filter(|entry| entry.enabled)
            .collect::<Vec<_>>();
        // Host intentionally makes the last registration for one Pack id active.
        // We only use ordering to encode the catalog's explicit `active` bit;
        // version strings are opaque and merely stabilize ordering among historical versions.
        entries.sort_by(|left, right| {
            (&left.pack.id, left.active, &left.pack.version).cmp(&(
                &right.pack.id,
                right.active,
                &right.pack.version,
            ))
        });
        let packs = entries
            .into_iter()
            .map(|entry| self.verified_pack(entry))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ProcessPackSource::from_packs(packs))
    }

    pub fn availability(&self, pack: &WorldPackRef) -> PackAvailability {
        if let Some(entry) = self.entry(pack) {
            if !entry.enabled {
                return PackAvailability::Disabled;
            }
            return match self.verified_pack(entry) {
                Ok(_) => PackAvailability::Ready,
                Err(error) => PackAvailability::Invalid {
                    reason: error.to_string(),
                },
            };
        }

        let mut installed_versions = self
            .entries
            .iter()
            .filter(|entry| entry.pack.id == pack.id)
            .map(|entry| entry.pack.version.clone())
            .collect::<Vec<_>>();
        installed_versions.sort();
        installed_versions.dedup();
        if installed_versions.is_empty() {
            PackAvailability::NotInstalled
        } else {
            PackAvailability::MissingVersion { installed_versions }
        }
    }

    fn verify_reviewed_managed_program(
        &self,
        managed: &ProcessPack,
        expected_sha256: &str,
    ) -> Result<(), CatalogError> {
        let pack = managed.descriptor.pack.clone();
        let result = match managed.current_pin() {
            Ok(pin) if pin.command_sha256() == expected_sha256 => Ok(()),
            Ok(_) => Err(reviewed_content_changed(
                &pack,
                "executable changed while it was copied into the managed store",
            )),
            Err(error) => Err(process_error(error)),
        };
        if result.is_err() {
            cleanup_managed_pack_identity(&self.path, &pack);
        }
        result
    }

    fn record_managed_install(
        &mut self,
        managed: ProcessPack,
    ) -> Result<InstalledPack, CatalogError> {
        let managed_pack = managed.descriptor.pack.clone();
        let result = (|| {
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
            self.commit(entries)?;
            Ok(installed)
        })();
        if result.is_err() {
            cleanup_managed_pack_identity(&self.path, &managed_pack);
        }
        result
    }

    fn materialize_managed_bundle(&self, bundle: PackBundle) -> Result<ProcessPack, CatalogError> {
        let descriptor = bundle.manifest().descriptor.clone();
        let program_name = bundle.program_name().to_owned();
        let final_dir = managed_pack_dir(&self.path, &descriptor.pack);
        if final_dir.try_exists().map_err(|error| CatalogError::Io {
            operation: "check managed Pack destination",
            path: final_dir.clone(),
            message: error.to_string(),
        })? {
            return Err(CatalogError::ManagedDestinationExists(descriptor.pack));
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
            let staged_program = stage.join(&program_name);
            bundle
                .extract_program(&staged_program)
                .map_err(bundle_error)?;

            let managed_manifest =
                PackManifest::process(descriptor.clone(), program_name.clone(), Vec::new());
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
            let _ = fs::remove_dir_all(&final_dir);
        }
        result
    }

    fn materialize_managed_pack(&self, source: &ProcessPack) -> Result<ProcessPack, CatalogError> {
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
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&staged_program, fs::Permissions::from_mode(0o700)).map_err(
                    |error| CatalogError::Io {
                        operation: "set managed Pack executable permissions",
                        path: staged_program.clone(),
                        message: error.to_string(),
                    },
                )?;
            }
            if let Ok(file) = OpenOptions::new().read(true).open(&staged_program) {
                file.sync_all().map_err(|error| CatalogError::Io {
                    operation: "sync managed Pack executable",
                    path: staged_program.clone(),
                    message: error.to_string(),
                })?;
            }

            let managed_manifest =
                PackManifest::process(source.descriptor.clone(), program_name, Vec::new());
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
            let _ = fs::remove_dir_all(&final_dir);
        }
        result
    }

    fn verified_pack(&self, entry: &InstalledPack) -> Result<ProcessPack, CatalogError> {
        if !entry.manifest_path.is_absolute() || !entry.command_path.is_absolute() {
            return Err(CatalogError::InvalidStoredPath(entry.pack.clone()));
        }
        let pack = ProcessPack::load(&entry.manifest_path).map_err(process_error)?;
        if pack.descriptor.pack != entry.pack {
            return Err(CatalogError::PackIdentityChanged {
                expected: entry.pack.clone(),
                found: pack.descriptor.pack,
            });
        }
        if pack.command != entry.command_path {
            return Err(CatalogError::CommandPathChanged {
                pack: entry.pack.clone(),
                expected: entry.command_path.clone(),
                found: pack.command,
            });
        }
        let current = pack.current_pin().map_err(process_error)?;
        let expected = entry.expected_pin();
        if current.manifest_sha256() != expected.manifest_sha256() {
            return Err(CatalogError::ContentChanged {
                pack: entry.pack.clone(),
                component: "manifest",
                expected: expected.manifest_sha256().into(),
                found: current.manifest_sha256().into(),
            });
        }
        if current.command_sha256() != expected.command_sha256() {
            return Err(CatalogError::ContentChanged {
                pack: entry.pack.clone(),
                component: "executable",
                expected: expected.command_sha256().into(),
                found: current.command_sha256().into(),
            });
        }
        Ok(pack.with_pin(expected))
    }

    fn commit(&mut self, entries: Vec<InstalledPack>) -> Result<(), CatalogError> {
        validate_entries(&entries)?;
        validate_managed_entries(&self.path, &entries)?;
        persist_document(&self.path, &entries)?;
        self.entries = entries;
        Ok(())
    }
}

fn absolute_path(path: &Path) -> Result<PathBuf, CatalogError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .map_err(|error| CatalogError::Io {
                operation: "resolve catalog path",
                path: path.to_path_buf(),
                message: error.to_string(),
            })?
    };

    if absolute.try_exists().map_err(|error| CatalogError::Io {
        operation: "inspect catalog path",
        path: absolute.clone(),
        message: error.to_string(),
    })? {
        return absolute.canonicalize().map_err(|error| CatalogError::Io {
            operation: "canonicalize catalog path",
            path: absolute,
            message: error.to_string(),
        });
    }

    // The catalog and its Packs directory may not exist yet. Canonicalize the
    // nearest existing ancestor so platform aliases such as macOS /var ->
    // /private/var cannot make later managed artifacts appear to escape their
    // catalog-owned directory, then append the still-missing suffix.
    let mut missing = Vec::new();
    let mut cursor = absolute.as_path();
    loop {
        if cursor.try_exists().map_err(|error| CatalogError::Io {
            operation: "inspect catalog ancestor",
            path: cursor.to_path_buf(),
            message: error.to_string(),
        })? {
            break;
        }
        let name = cursor.file_name().ok_or_else(|| CatalogError::Io {
            operation: "resolve catalog ancestor",
            path: absolute.clone(),
            message: "no existing ancestor was found".into(),
        })?;
        missing.push(name.to_os_string());
        cursor = cursor.parent().ok_or_else(|| CatalogError::Io {
            operation: "resolve catalog ancestor",
            path: absolute.clone(),
            message: "no existing ancestor was found".into(),
        })?;
    }

    let mut resolved = cursor.canonicalize().map_err(|error| CatalogError::Io {
        operation: "canonicalize catalog ancestor",
        path: cursor.to_path_buf(),
        message: error.to_string(),
    })?;
    for component in missing.iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
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

fn cleanup_managed_pack_identity(catalog_path: &Path, pack: &WorldPackRef) {
    let expected = managed_pack_dir(catalog_path, pack);
    let _ = fs::remove_dir_all(expected);
    sync_directory(&managed_store_root(catalog_path));
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

fn validate_entries(entries: &[InstalledPack]) -> Result<(), CatalogError> {
    let mut identities = BTreeSet::new();
    let mut enabled_by_id = BTreeMap::<String, usize>::new();
    let mut active_by_id = BTreeMap::<String, usize>::new();
    for entry in entries {
        if entry.pack.id.trim().is_empty()
            || entry.pack.version.trim().is_empty()
            || entry.title.trim().is_empty()
            || entry.manifest_sha256.len() != 64
            || entry.command_sha256.len() != 64
            || !entry.manifest_path.is_absolute()
            || !entry.command_path.is_absolute()
            || (entry.active && !entry.enabled)
        {
            return Err(CatalogError::InvalidEntry(entry.pack.clone()));
        }
        let key = (entry.pack.id.clone(), entry.pack.version.clone());
        if !identities.insert(key) {
            return Err(CatalogError::DuplicateEntry(entry.pack.clone()));
        }
        if entry.enabled {
            *enabled_by_id.entry(entry.pack.id.clone()).or_default() += 1;
        }
        if entry.active {
            *active_by_id.entry(entry.pack.id.clone()).or_default() += 1;
        }
    }
    for (id, enabled) in enabled_by_id {
        if enabled > 0 && active_by_id.get(&id).copied().unwrap_or_default() != 1 {
            return Err(CatalogError::InvalidActiveSelection(id));
        }
    }
    Ok(())
}
fn sort_entries(entries: &mut [InstalledPack]) {
    entries.sort_by(|left, right| {
        (&left.pack.id, &left.pack.version).cmp(&(&right.pack.id, &right.pack.version))
    });
}

fn persist_document(path: &Path, entries: &[InstalledPack]) -> Result<(), CatalogError> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| CatalogError::Io {
        operation: "create catalog directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;
    let document = PackCatalogDocument::new(entries.to_vec());
    let mut json = serde_json::to_vec_pretty(&document)
        .map_err(|error| CatalogError::Json(error.to_string()))?;
    json.push(b'\n');

    let nonce = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("pack-catalog.json");
    let temporary = parent.join(format!(".{file_name}.{}.{}.tmp", process::id(), nonce));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| CatalogError::Io {
            operation: "create temporary catalog",
            path: temporary.clone(),
            message: error.to_string(),
        })?;
    if let Err(error) = file.write_all(&json).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(CatalogError::Io {
            operation: "write temporary catalog",
            path: temporary,
            message: error.to_string(),
        });
    }
    drop(file);
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(CatalogError::Io {
            operation: "publish catalog",
            path: path.to_path_buf(),
            message: error.to_string(),
        });
    }
    #[cfg(unix)]
    if let Ok(directory) = OpenOptions::new().read(true).open(parent) {
        let _ = directory.sync_all();
    }
    Ok(())
}

fn reviewed_content_changed(pack: &WorldPackRef, reason: impl Into<String>) -> CatalogError {
    CatalogError::ReviewedContentChanged {
        pack: pack.clone(),
        reason: reason.into(),
    }
}

fn bundle_error(error: impl fmt::Display) -> CatalogError {
    CatalogError::Bundle(error.to_string())
}

fn process_error(error: impl fmt::Display) -> CatalogError {
    CatalogError::Process(error.to_string())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogError {
    Io {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
    Json(String),
    Bundle(String),
    UnsupportedFormat(String),
    UnsupportedVersion(u32),
    InvalidEntry(WorldPackRef),
    DuplicateEntry(WorldPackRef),
    AlreadyInstalled(WorldPackRef),
    RuntimeArgumentsNotPinnable(WorldPackRef),
    NotInstalled(WorldPackRef),
    DisabledCannotActivate(WorldPackRef),
    ActivePackRequiresReplacement(WorldPackRef),
    InvalidActiveSelection(String),
    InvalidStoredPath(WorldPackRef),
    InvalidManagedPath(WorldPackRef),
    ManagedDestinationExists(WorldPackRef),
    ReviewedContentChanged {
        pack: WorldPackRef,
        reason: String,
    },
    PackIdentityChanged {
        expected: WorldPackRef,
        found: WorldPackRef,
    },
    CommandPathChanged {
        pack: WorldPackRef,
        expected: PathBuf,
        found: PathBuf,
    },
    ContentChanged {
        pack: WorldPackRef,
        component: &'static str,
        expected: String,
        found: String,
    },
    Process(String),
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, path, message } => write!(f, "could not {operation} {}: {message}", path.display()),
            Self::Json(error) => write!(f, "could not decode Pack catalog: {error}"),
            Self::Bundle(error) => write!(f, "could not install Pack bundle: {error}"),
            Self::UnsupportedFormat(format) => write!(f, "unsupported Pack catalog format: {format}"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported Pack catalog version: {version}"),
            Self::InvalidEntry(pack) => write!(f, "invalid installed Pack entry: {}@{}", pack.id, pack.version),
            Self::DuplicateEntry(pack) => write!(f, "duplicate installed Pack entry: {}@{}", pack.id, pack.version),
            Self::AlreadyInstalled(pack) => write!(f, "Pack is already installed: {}@{}", pack.id, pack.version),
            Self::RuntimeArgumentsNotPinnable(pack) => write!(
                f,
                "installed Pack {}@{} uses runtime arguments that are outside the v1 content pin; package the approved program as the direct command",
                pack.id, pack.version
            ),
            Self::NotInstalled(pack) => write!(f, "Pack is not installed: {}@{}", pack.id, pack.version),
            Self::DisabledCannotActivate(pack) => write!(
                f,
                "disabled Pack cannot become active: {}@{}",
                pack.id, pack.version
            ),
            Self::ActivePackRequiresReplacement(pack) => write!(
                f,
                "active Pack {}@{} has another enabled version; activate its replacement first",
                pack.id, pack.version
            ),
            Self::InvalidActiveSelection(id) => write!(
                f,
                "Pack catalog must select exactly one active enabled version for {id}"
            ),
            Self::InvalidStoredPath(pack) => write!(f, "installed Pack contains a non-absolute path: {}@{}", pack.id, pack.version),
            Self::InvalidManagedPath(pack) => write!(
                f,
                "managed Pack paths do not match the catalog-owned store: {}@{}",
                pack.id, pack.version
            ),
            Self::ManagedDestinationExists(pack) => write!(
                f,
                "managed Pack destination already exists for {}@{}",
                pack.id, pack.version
            ),
            Self::ReviewedContentChanged { pack, reason } => write!(
                f,
                "reviewed Pack {}@{} changed before installation: {reason}",
                pack.id, pack.version
            ),
            Self::PackIdentityChanged { expected, found } => write!(f, "installed Pack identity changed: expected {}@{}, found {}@{}", expected.id, expected.version, found.id, found.version),
            Self::CommandPathChanged { pack, expected, found } => write!(f, "installed Pack {}@{} executable path changed: expected {}, found {}", pack.id, pack.version, expected.display(), found.display()),
            Self::ContentChanged { pack, component, expected, found } => write!(f, "installed Pack {}@{} {component} content changed: expected sha256 {expected}, found {found}", pack.id, pack.version),
            Self::Process(error) => write!(f, "could not validate installed Pack: {error}"),
        }
    }
}

impl Error for CatalogError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_host::{WorldPackSource, WorldRegistry};
    use world_pack_protocol::{PackDescriptor, PackManifest};

    fn pack(id: &str, version: &str) -> WorldPackRef {
        WorldPackRef::new(id, version)
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "world-machine-pack-catalog-{label}-{}-{nonce}",
            process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_pack(root: &Path, id: &str, version: &str) -> PathBuf {
        let runtime = root.join(format!("{id}-{version}-runtime.sh"));
        fs::write(&runtime, "#!/bin/sh\nexit 0\n").unwrap();
        let descriptor = PackDescriptor::new(pack(id, version), id, "fixture");
        let manifest = PackManifest::process(
            descriptor,
            runtime.file_name().unwrap().to_string_lossy(),
            Vec::new(),
        );
        let path = root.join(format!("{id}-{version}.world-pack.json"));
        fs::write(&path, manifest.to_json_pretty().unwrap()).unwrap();
        path
    }

    #[cfg(unix)]
    #[test]
    fn catalog_path_canonicalizes_existing_symlink_ancestor_before_managed_storage() {
        use std::os::unix::fs::symlink;

        let root = temp_dir("symlink-root");
        let alias = root.parent().unwrap().join(format!(
            "{}-alias",
            root.file_name().unwrap().to_string_lossy()
        ));
        symlink(&root, &alias).unwrap();
        let catalog = PackCatalog::open(alias.join("Packs").join("catalog.json")).unwrap();

        assert_eq!(
            catalog.path(),
            root.canonicalize()
                .unwrap()
                .join("Packs")
                .join("catalog.json")
        );
        fs::remove_file(alias).unwrap();
    }

    #[test]
    fn inspection_never_executes_a_developer_pack() {
        let root = temp_dir("review-no-exec");
        let marker = root.join("executed");
        let runtime = root.join("runtime.sh");
        fs::write(
            &runtime,
            format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        )
        .unwrap();
        let descriptor = PackDescriptor::new(pack("fixture.review", "v1"), "Review", "fixture");
        let manifest = PackManifest::process(
            descriptor,
            runtime.file_name().unwrap().to_string_lossy(),
            Vec::new(),
        );
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();

        let catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
        let preview = catalog.inspect_install(&manifest_path).unwrap();
        assert_eq!(preview.kind, PackInstallKind::DeveloperManifest);
        assert_eq!(preview.pack, pack("fixture.review", "v1"));
        assert_eq!(preview.runtime_name, "runtime.sh");
        assert!(!marker.exists());
    }

    #[cfg(unix)]
    #[test]
    fn reviewed_manifest_normalizes_managed_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_dir("review-permissions");
        let manifest_path = write_pack(&root, "fixture.review-mode", "v1");
        let runtime = root.join("fixture.review-mode-v1-runtime.sh");
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();

        let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
        let preview = catalog.inspect_install(&manifest_path).unwrap();
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o600)).unwrap();

        let installed = catalog.install_reviewed(&preview).unwrap();
        let mode = fs::metadata(&installed.command_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);
    }

    #[test]
    fn failed_post_copy_pin_verification_removes_managed_destination() {
        let root = temp_dir("review-post-copy-cleanup");
        let manifest_path = write_pack(&root, "fixture.review-cleanup", "v1");
        let source = ProcessPack::load(&manifest_path).unwrap();
        let catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
        let managed = catalog.materialize_managed_pack(&source).unwrap();
        let pack = managed.descriptor.pack.clone();
        fs::remove_file(&managed.command).unwrap();

        let error = catalog
            .verify_reviewed_managed_program(&managed, "not-used-after-read-failure")
            .unwrap_err();
        assert!(matches!(error, CatalogError::Process(_)));
        assert!(!managed_pack_dir(catalog.path(), &pack).exists());
    }

    #[test]
    fn reviewed_manifest_refuses_changed_executable_without_installing() {
        let root = temp_dir("review-manifest-change");
        let manifest_path = write_pack(&root, "fixture.review-change", "v1");
        let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
        let preview = catalog.inspect_install(&manifest_path).unwrap();
        let runtime = root.join("fixture.review-change-v1-runtime.sh");
        fs::write(&runtime, "#!/bin/sh\necho changed\n").unwrap();

        let error = catalog.install_reviewed(&preview).unwrap_err();
        assert!(matches!(error, CatalogError::ReviewedContentChanged { .. }));
        assert!(catalog.entries().is_empty());
        assert!(!managed_pack_dir(catalog.path(), &preview.pack).exists());
    }

    #[test]
    fn reviewed_bundle_refuses_replacement_without_installing() {
        use world_pack_bundle::write_program_bundle;

        let root = temp_dir("review-bundle-change");
        let program = root.join("bundle-runtime");
        fs::write(&program, b"approved-program").unwrap();
        let descriptor =
            PackDescriptor::new(pack("fixture.review-bundle", "v1"), "Bundle", "fixture");
        let bundle_path = root.join("fixture.worldpack");
        write_program_bundle(&bundle_path, descriptor.clone(), &program).unwrap();

        let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
        let preview = catalog.inspect_install(&bundle_path).unwrap();
        fs::remove_file(&bundle_path).unwrap();
        fs::write(&program, b"replacement-program").unwrap();
        write_program_bundle(&bundle_path, descriptor, &program).unwrap();

        let error = catalog.install_reviewed(&preview).unwrap_err();
        assert!(matches!(error, CatalogError::ReviewedContentChanged { .. }));
        assert!(catalog.entries().is_empty());
        assert!(!managed_pack_dir(catalog.path(), &preview.pack).exists());
    }

    #[test]
    fn portable_bundle_preserves_executable_suffix_in_managed_manifest() {
        use world_pack_bundle::write_program_bundle;

        let root = temp_dir("bundle-suffix");
        let program = root.join("bundle-runtime.exe");
        fs::write(&program, b"portable-runtime").unwrap();
        let descriptor = PackDescriptor::new(pack("fixture.bundle.exe", "v1"), "Bundle", "fixture");
        let bundle_path = root.join("fixture.worldpack");
        write_program_bundle(&bundle_path, descriptor, &program).unwrap();

        let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
        let installed = catalog.install_bundle(&bundle_path).unwrap();
        assert_eq!(
            installed
                .command_path
                .file_name()
                .and_then(|name| name.to_str()),
            Some("program.exe")
        );
        let managed = ProcessPack::load(&installed.manifest_path).unwrap();
        assert_eq!(
            managed.command.file_name().and_then(|name| name.to_str()),
            Some("program.exe")
        );
    }

    #[test]
    fn portable_bundle_install_owns_program_after_source_is_removed() {
        use world_pack_bundle::write_program_bundle;

        let root = temp_dir("bundle-install");
        let program = root.join("bundle-runtime");
        fs::write(&program, b"portable-runtime").unwrap();
        let descriptor =
            PackDescriptor::new(pack("fixture.bundle", "opaque-v1"), "Bundle", "fixture");
        let bundle_path = root.join("fixture.worldpack");
        write_program_bundle(&bundle_path, descriptor, &program).unwrap();

        let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
        let installed = catalog.install_bundle(&bundle_path).unwrap();
        assert!(installed.managed);
        assert!(installed
            .command_path
            .starts_with(managed_store_root(catalog.path())));
        fs::remove_file(&bundle_path).unwrap();
        fs::remove_file(&program).unwrap();
        assert!(catalog.trusted_source().is_ok());
        assert!(installed.command_path.exists());
    }

    #[test]
    fn explicit_install_persists_exact_identity_and_reopens() {
        let root = temp_dir("install");
        let catalog_path = root.join("catalog.json");
        let manifest = write_pack(&root, "fixture", "1");
        let mut catalog = PackCatalog::open(&catalog_path).unwrap();

        let installed = catalog.install_manifest(&manifest).unwrap();
        assert_eq!(installed.pack, pack("fixture", "1"));
        assert!(installed.enabled);
        assert!(installed.active);
        assert_eq!(installed.approval, PackApproval::ExplicitInstall);
        assert!(installed.manifest_path.is_absolute());
        assert!(installed.command_path.is_absolute());
        assert!(installed.managed);
        assert_ne!(installed.manifest_path, manifest.canonicalize().unwrap());
        assert_eq!(installed.manifest_sha256.len(), 64);
        assert_eq!(installed.command_sha256.len(), 64);

        let reopened = PackCatalog::open(&catalog_path).unwrap();
        assert_eq!(reopened.entries(), &[installed]);
    }

    #[test]
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
        assert_eq!(
            catalog.availability(&installed.pack),
            PackAvailability::Ready
        );
        assert_eq!(
            catalog
                .trusted_source()
                .unwrap()
                .registrations()
                .unwrap()
                .len(),
            1
        );
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

    #[test]
    fn duplicate_exact_install_is_rejected_without_mutating_catalog() {
        let root = temp_dir("duplicate");
        let catalog_path = root.join("catalog.json");
        let manifest = write_pack(&root, "fixture", "1");
        let mut catalog = PackCatalog::open(&catalog_path).unwrap();
        let installed = catalog.install_manifest(&manifest).unwrap();

        assert!(matches!(
            catalog.install_manifest(&manifest),
            Err(CatalogError::AlreadyInstalled(found)) if found == installed.pack
        ));
        assert_eq!(catalog.entries(), &[installed]);
    }

    #[test]
    fn explicit_install_and_activate_choose_active_version_without_interpreting_versions() {
        let root = temp_dir("active-version");
        let catalog_path = root.join("catalog.json");
        let old_manifest = write_pack(&root, "fixture", "z-old");
        let new_manifest = write_pack(&root, "fixture", "a-new");
        let mut catalog = PackCatalog::open(&catalog_path).unwrap();
        let old = catalog.install_manifest(&old_manifest).unwrap();
        let new = catalog.install_manifest(&new_manifest).unwrap();

        assert!(!catalog.entry(&old.pack).unwrap().active);
        assert!(catalog.entry(&new.pack).unwrap().active);
        let source = catalog.trusted_source().unwrap();
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();
        assert_eq!(
            registry.descriptor("fixture").unwrap().pack.version,
            "a-new"
        );
        assert!(registry.descriptor_for(&old.pack).is_some());

        catalog.activate(&old.pack).unwrap();
        let source = catalog.trusted_source().unwrap();
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();
        assert_eq!(
            registry.descriptor("fixture").unwrap().pack.version,
            "z-old"
        );
        assert!(registry.descriptor_for(&new.pack).is_some());
    }

    #[test]
    fn active_enabled_version_requires_explicit_replacement_before_disable_or_uninstall() {
        let root = temp_dir("active-replacement");
        let catalog_path = root.join("catalog.json");
        let first = write_pack(&root, "fixture", "one");
        let second = write_pack(&root, "fixture", "two");
        let mut catalog = PackCatalog::open(&catalog_path).unwrap();
        let first = catalog.install_manifest(&first).unwrap();
        let second = catalog.install_manifest(&second).unwrap();

        assert!(matches!(
            catalog.set_enabled(&second.pack, false),
            Err(CatalogError::ActivePackRequiresReplacement(found)) if found == second.pack
        ));
        assert!(matches!(
            catalog.uninstall(&second.pack),
            Err(CatalogError::ActivePackRequiresReplacement(found)) if found == second.pack
        ));

        catalog.activate(&first.pack).unwrap();
        catalog.set_enabled(&second.pack, false).unwrap();
        assert!(catalog.entry(&first.pack).unwrap().active);
        assert!(!catalog.entry(&second.pack).unwrap().enabled);
        assert!(!catalog.entry(&second.pack).unwrap().active);
        assert!(matches!(
            catalog.activate(&second.pack),
            Err(CatalogError::DisabledCannotActivate(found)) if found == second.pack
        ));
    }

    #[test]
    fn availability_distinguishes_disabled_missing_version_and_missing_pack() {
        let root = temp_dir("availability");
        let catalog_path = root.join("catalog.json");
        let manifest = write_pack(&root, "fixture", "1");
        let mut catalog = PackCatalog::open(&catalog_path).unwrap();
        catalog.install_manifest(&manifest).unwrap();
        let v1 = pack("fixture", "1");
        catalog.set_enabled(&v1, false).unwrap();

        assert_eq!(catalog.availability(&v1), PackAvailability::Disabled);
        assert_eq!(
            catalog.availability(&pack("fixture", "2")),
            PackAvailability::MissingVersion {
                installed_versions: vec!["1".into()]
            }
        );
        assert_eq!(
            catalog.availability(&pack("unknown", "1")),
            PackAvailability::NotInstalled
        );
    }

    #[test]
    fn launcher_style_runtime_arguments_are_rejected_at_install() {
        let root = temp_dir("launcher-args");
        let script = root.join("runtime.sh");
        fs::write(
            &script, "exit 0
",
        )
        .unwrap();
        let descriptor = PackDescriptor::new(pack("fixture", "1"), "fixture", "fixture");
        let manifest = PackManifest::process(
            descriptor,
            "/bin/sh",
            vec![script.to_string_lossy().into_owned()],
        );
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();
        let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();

        assert!(matches!(
            catalog.install_manifest(&manifest_path),
            Err(CatalogError::RuntimeArgumentsNotPinnable(found)) if found == pack("fixture", "1")
        ));
        assert!(catalog.entries().is_empty());
    }

    #[test]
    fn tampered_executable_is_rejected_before_source_assembly() {
        let root = temp_dir("tamper");
        let catalog_path = root.join("catalog.json");
        let manifest = write_pack(&root, "fixture", "1");
        let mut catalog = PackCatalog::open(&catalog_path).unwrap();
        let installed = catalog.install_manifest(&manifest).unwrap();
        fs::write(&installed.command_path, "#!/bin/sh\necho changed\n").unwrap();

        assert!(matches!(
            catalog.availability(&installed.pack),
            PackAvailability::Invalid { .. }
        ));
        assert!(matches!(
            catalog.trusted_source(),
            Err(CatalogError::ContentChanged {
                component: "executable",
                ..
            })
        ));
    }

    #[test]
    fn source_pin_detects_replacement_after_registry_install() {
        let root = temp_dir("launch-pin");
        let catalog_path = root.join("catalog.json");
        let manifest = write_pack(&root, "fixture", "1");
        let mut catalog = PackCatalog::open(&catalog_path).unwrap();
        let installed = catalog.install_manifest(&manifest).unwrap();
        let source = catalog.trusted_source().unwrap();
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();

        fs::write(&installed.command_path, "#!/bin/sh\necho replaced\n").unwrap();
        let error = registry.create("fixture").err().unwrap();
        assert!(error.to_string().contains("content pin mismatch"));
    }

    #[test]
    fn disabled_pack_is_not_exposed_as_a_host_registration() {
        let root = temp_dir("disabled-source");
        let catalog_path = root.join("catalog.json");
        let manifest = write_pack(&root, "fixture", "1");
        let mut catalog = PackCatalog::open(&catalog_path).unwrap();
        let installed = catalog.install_manifest(&manifest).unwrap();
        catalog.set_enabled(&installed.pack, false).unwrap();

        let source = catalog.trusted_source().unwrap();
        assert!(source.registrations().unwrap().is_empty());
    }

    #[test]
    fn uninstall_is_durable() {
        let root = temp_dir("uninstall");
        let catalog_path = root.join("catalog.json");
        let manifest = write_pack(&root, "fixture", "1");
        let mut catalog = PackCatalog::open(&catalog_path).unwrap();
        let installed = catalog.install_manifest(&manifest).unwrap();
        catalog.uninstall(&installed.pack).unwrap();

        assert!(catalog.entries().is_empty());
        assert!(PackCatalog::open(&catalog_path)
            .unwrap()
            .entries()
            .is_empty());
    }
}
