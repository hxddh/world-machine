use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use world_pack_process::{ProcessPack, ProcessPackPin, ProcessPackSource};
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
        let path = path.as_ref().to_path_buf();
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

    /// Explicit installation is the trust decision. It reads and validates the
    /// manifest but never launches Pack code, then pins both manifest and process
    /// executable content by SHA-256.
    pub fn install_manifest(
        &mut self,
        manifest_path: impl AsRef<Path>,
    ) -> Result<InstalledPack, CatalogError> {
        let pack = ProcessPack::load(manifest_path).map_err(process_error)?;
        if !pack.args.is_empty() {
            return Err(CatalogError::RuntimeArgumentsNotPinnable(
                pack.descriptor.pack,
            ));
        }
        let identity = pack.current_pin().map_err(process_error)?;
        let installed = InstalledPack {
            pack: pack.descriptor.pack.clone(),
            title: pack.descriptor.title.clone(),
            description: pack.descriptor.description.clone(),
            manifest_path: pack.manifest_path.clone(),
            command_path: pack.command.clone(),
            manifest_sha256: identity.manifest_sha256().into(),
            command_sha256: identity.command_sha256().into(),
            approval: PackApproval::ExplicitInstall,
            enabled: true,
            active: true,
        };
        if self.entry(&installed.pack).is_some() {
            return Err(CatalogError::AlreadyInstalled(installed.pack));
        }

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
        let entries = self
            .entries
            .iter()
            .filter(|entry| entry.pack != *pack)
            .cloned()
            .collect();
        self.commit(entries)
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
        persist_document(&self.path, &entries)?;
        self.entries = entries;
        Ok(())
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
        assert_eq!(installed.manifest_sha256.len(), 64);
        assert_eq!(installed.command_sha256.len(), 64);

        let reopened = PackCatalog::open(&catalog_path).unwrap();
        assert_eq!(reopened.entries(), &[installed]);
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
