from pathlib import Path

# world-pack-server: authoring helper for portable single-executable bundles.
p = Path('crates/world-pack-server/src/lib.rs')
s = p.read_text()
s = s.replace(
    'use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};\n',
    'use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};\nuse world_pack_bundle::{write_program_bundle, PackBundleHeader};\n',
    1,
)
marker = '''pub fn manifest_for_current_exe(
    descriptor: &WorldDescriptor,
) -> Result<PackManifest, PackServerError> {
    let executable = env::current_exe()
        .map_err(PackServerError::Io)?
        .canonicalize()
        .map_err(PackServerError::Io)?;
    manifest_for_canonical_exe(descriptor, &executable)
}
'''
replacement = marker + '''
/// Write a portable v1 `.worldpack` containing this Pack executable.
/// The bundle manifest always names the single embedded program and carries no
/// runtime arguments, so installing it does not expand the v1 trust surface.
pub fn write_current_exe_bundle(
    descriptor: &WorldDescriptor,
    destination: impl AsRef<Path>,
) -> Result<PackBundleHeader, PackServerError> {
    let executable = env::current_exe()
        .map_err(PackServerError::Io)?
        .canonicalize()
        .map_err(PackServerError::Io)?;
    write_program_bundle(destination, protocol_descriptor(descriptor), executable)
        .map_err(|error| PackServerError::Bundle(error.to_string()))
}
'''
if marker not in s:
    raise SystemExit('server manifest marker not found')
s = s.replace(marker, replacement, 1)
s = s.replace(
    '    Protocol(String),\n    ResponseTooLarge { request_id: u64, max_bytes: usize },\n',
    '    Protocol(String),\n    Bundle(String),\n    ResponseTooLarge { request_id: u64, max_bytes: usize },\n',
    1,
)
s = s.replace(
    '            Self::Protocol(error) => write!(f, "Pack protocol failed: {error}"),\n',
    '            Self::Protocol(error) => write!(f, "Pack protocol failed: {error}"),\n            Self::Bundle(error) => write!(f, "Pack bundle failed: {error}"),\n',
    1,
)
p.write_text(s)

# Tiny Society external Pack: expose a real portable bundle authoring command.
p = Path('apps/tiny-society-pack/src/main.rs')
s = p.read_text()
s = s.replace('use std::error::Error;\n', 'use std::error::Error;\nuse std::path::PathBuf;\n', 1)
s = s.replace(
    'use world_pack_server::{manifest_for_current_exe, serve_stdio};\n',
    'use world_pack_server::{manifest_for_current_exe, serve_stdio, write_current_exe_bundle};\n',
    1,
)
old = '''    if !args.is_empty() {
        return Err("unsupported arguments; run without arguments as a Pack server or use --print-manifest"
            .to_string()
            .into());
    }
'''
new = '''    if args.len() == 2 && args[0] == "--write-bundle" {
        let destination = PathBuf::from(&args[1]);
        write_current_exe_bundle(&registration.descriptor, destination)?;
        return Ok(());
    }
    if !args.is_empty() {
        return Err("unsupported arguments; run without arguments as a Pack server, use --print-manifest, or use --write-bundle PATH"
            .to_string()
            .into());
    }
'''
if old not in s:
    raise SystemExit('tiny pack args marker not found')
s = s.replace(old, new, 1)
p.write_text(s)

# Catalog: install portable bundles without launching source code, then reuse the
# same managed-store + content-pin semantics as manifest installation.
p = Path('crates/world-pack-catalog/src/lib.rs')
s = p.read_text()
s = s.replace(
    'use world_pack_process::{ProcessPack, ProcessPackPin, ProcessPackSource};\n',
    'use world_pack_bundle::{PackBundle, PACK_BUNDLE_PROGRAM_NAME};\nuse world_pack_process::{ProcessPack, ProcessPackPin, ProcessPackSource};\n',
    1,
)
old_install = '''    /// Explicit installation is the trust decision. It reads and validates the
    /// manifest but never launches Pack code, then pins both manifest and process
    /// executable content by SHA-256.
    pub fn install_manifest(
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
new_install = '''    /// Explicit installation is the trust decision. It reads and validates the
    /// manifest but never launches Pack code, then pins both manifest and process
    /// executable content by SHA-256.
    pub fn install_manifest(
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
        self.record_managed_install(managed)
    }

    /// Install a portable `.worldpack` without executing any code from the bundle.
    /// v1 bundles contain one direct program only; extraction verifies the embedded
    /// SHA-256 before the managed Pack is published and pinned for runtime launch.
    pub fn install_bundle(
        &mut self,
        bundle_path: impl AsRef<Path>,
    ) -> Result<InstalledPack, CatalogError> {
        let bundle = PackBundle::open(bundle_path).map_err(bundle_error)?;
        let pack = bundle.manifest().descriptor.pack.clone();
        if self.entry(&pack).is_some() {
            return Err(CatalogError::AlreadyInstalled(pack));
        }
        let managed = self.materialize_managed_bundle(bundle)?;
        self.record_managed_install(managed)
    }
'''
if old_install not in s:
    raise SystemExit('catalog install marker not found')
s = s.replace(old_install, new_install, 1)
materialize_marker = '''    fn materialize_managed_pack(&self, source: &ProcessPack) -> Result<ProcessPack, CatalogError> {
'''
helpers = '''    fn record_managed_install(
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

    fn materialize_managed_bundle(
        &self,
        bundle: PackBundle,
    ) -> Result<ProcessPack, CatalogError> {
        let descriptor = bundle.manifest().descriptor.clone();
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
            let staged_program = stage.join(PACK_BUNDLE_PROGRAM_NAME);
            bundle
                .extract_program(&staged_program)
                .map_err(bundle_error)?;

            let managed_manifest = PackManifest::process(
                descriptor.clone(),
                PACK_BUNDLE_PROGRAM_NAME,
                Vec::new(),
            );
            let manifest_path = stage.join("pack.world-pack.json");
            let mut manifest_json = managed_manifest
                .to_json_pretty()
                .map_err(|error| CatalogError::Json(error.to_string()))?
                .into_bytes();
            manifest_json.push(b'\\n');
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

'''
if materialize_marker not in s:
    raise SystemExit('catalog materialize marker not found')
s = s.replace(materialize_marker, helpers + materialize_marker, 1)
cleanup_marker = '''fn cleanup_managed_pack(catalog_path: &Path, entry: &InstalledPack) {
'''
cleanup_helper = '''fn cleanup_managed_pack_identity(catalog_path: &Path, pack: &WorldPackRef) {
    let expected = managed_pack_dir(catalog_path, pack);
    let _ = fs::remove_dir_all(expected);
    sync_directory(&managed_store_root(catalog_path));
}

'''
if cleanup_marker not in s:
    raise SystemExit('catalog cleanup marker not found')
s = s.replace(cleanup_marker, cleanup_helper + cleanup_marker, 1)
s = s.replace('    Json(String),\n', '    Json(String),\n    Bundle(String),\n', 1)
s = s.replace(
    '            Self::Json(error) => write!(f, "could not decode Pack catalog: {error}"),\n',
    '            Self::Json(error) => write!(f, "could not decode Pack catalog: {error}"),\n            Self::Bundle(error) => write!(f, "could not install Pack bundle: {error}"),\n',
    1,
)
s = s.replace(
    'fn process_error(error: impl fmt::Display) -> CatalogError {\n    CatalogError::Process(error.to_string())\n}\n',
    'fn bundle_error(error: impl fmt::Display) -> CatalogError {\n    CatalogError::Bundle(error.to_string())\n}\n\nfn process_error(error: impl fmt::Display) -> CatalogError {\n    CatalogError::Process(error.to_string())\n}\n',
    1,
)
# Add a catalog-level bundle lifecycle regression before the first existing install test.
test_marker = '''    #[test]
    fn explicit_install_persists_exact_identity_and_reopens() {'''
bundle_test = '''    #[test]
    fn portable_bundle_install_owns_program_after_source_is_removed() {
        use world_pack_bundle::write_program_bundle;

        let root = temp_dir("bundle-install");
        let program = root.join("bundle-runtime");
        fs::write(&program, b"portable-runtime").unwrap();
        let descriptor = PackDescriptor::new(pack("fixture.bundle", "opaque-v1"), "Bundle", "fixture");
        let bundle_path = root.join("fixture.worldpack");
        write_program_bundle(&bundle_path, descriptor, &program).unwrap();

        let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
        let installed = catalog.install_bundle(&bundle_path).unwrap();
        assert!(installed.managed);
        assert!(installed.command_path.starts_with(root.join("Installed")));
        fs::remove_file(&bundle_path).unwrap();
        fs::remove_file(&program).unwrap();
        assert!(catalog.trusted_source().is_ok());
        assert!(installed.command_path.exists());
    }

'''
if test_marker not in s:
    raise SystemExit('catalog test marker not found')
s = s.replace(test_marker, bundle_test + test_marker, 1)
p.write_text(s)

# Tiny Society: prove portable bundle -> managed install -> isolated process -> archive reopen.
p = Path('apps/tiny-society-pack/tests/external_pack.rs')
s = p.read_text()
append = '''
#[test]
fn tiny_society_portable_bundle_runs_after_the_bundle_is_removed() {
    let binary = env!("CARGO_BIN_EXE_tiny-society-pack");
    let root = temp_dir();
    let bundle_path = root.join("tiny-society.worldpack");
    let status = Command::new(binary)
        .arg("--write-bundle")
        .arg(&bundle_path)
        .status()
        .unwrap();
    assert!(status.success());

    let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
    let installed = catalog.install_bundle(&bundle_path).unwrap();
    assert!(installed.managed);
    assert_ne!(
        installed.command_path,
        PathBuf::from(binary).canonicalize().unwrap()
    );
    fs::remove_file(&bundle_path).unwrap();

    let source = catalog.trusted_source().unwrap();
    let mut registry = WorldRegistry::new();
    registry.install_source(&source).unwrap();
    let mut session = registry.create(TINY_SOCIETY_PACK_ID).unwrap();
    let initial = session.snapshot();
    let advanced = session.advance_background(1).unwrap();
    assert!(advanced.world_time >= initial.world_time);
    let archive = session.archive().unwrap().unwrap();
    drop(session);
    let reopened = registry.open_archive(&archive).unwrap();
    assert_eq!(reopened.pack(), archive.pack);
    assert_eq!(reopened.snapshot().world_time, archive.world_time);
}
'''
if 'tiny_society_portable_bundle_runs_after_the_bundle_is_removed' not in s:
    s += append
p.write_text(s)

# Desktop: Install Pack accepts a portable bundle or the existing developer manifest.
p = Path('apps/world-machine-desktop/src/main.rs')
s = p.read_text()
s = s.replace(
    'use world_pack_catalog::{InstalledPack, PackAvailability, PackCatalog};\n',
    'use world_pack_bundle::PACK_BUNDLE_SUFFIX;\n#[cfg(target_os = "macos")]\nuse world_pack_catalog::{InstalledPack, PackAvailability, PackCatalog};\n',
    1,
)
old_call = '''                let installed = match this
                    .pack_catalog
                    .as_mut()
                    .unwrap()
                    .install_manifest(&manifest)
                {
                    Ok(installed) => installed,
                    Err(error) => {
                        this.status =
                            Some(format!("Could not install {}: {error}", manifest.display()));
                        cx.notify();
                        return;
                    }
                };
'''
new_call = '''                let catalog = this.pack_catalog.as_mut().unwrap();
                let install_result = if manifest
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(PACK_BUNDLE_SUFFIX))
                {
                    catalog.install_bundle(&manifest)
                } else {
                    catalog.install_manifest(&manifest)
                };
                let installed = match install_result {
                    Ok(installed) => installed,
                    Err(error) => {
                        this.status =
                            Some(format!("Could not install {}: {error}", manifest.display()));
                        cx.notify();
                        return;
                    }
                };
'''
if old_call not in s:
    raise SystemExit('desktop install call marker not found')
s = s.replace(old_call, new_call, 1)
p.write_text(s)

# Generic architecture boundary for the new bundle format.
p = Path('scripts/check-boundaries.sh')
s = p.read_text()
s = s.replace('PACK_PROTOCOL="$ROOT/crates/world-pack-protocol"\n', 'PACK_PROTOCOL="$ROOT/crates/world-pack-protocol"\nPACK_BUNDLE="$ROOT/crates/world-pack-bundle"\n', 1)
s = s.replace(
    'pack_server_forbidden=("TinySociety"',
    'pack_bundle_forbidden=("TinySociety" "tiny_society" "tiny-society" "Tiny Society" "FutureArchaeologist" "future_archaeologist" "future-archaeologist" "Future Archaeologist" "gpui" "pi_agent" "openai" "anthropic")\npack_server_forbidden=("TinySociety"',
    1,
)
protocol_block = '''if [[ -d "$PACK_PROTOCOL" ]]; then
  for token in "${pack_protocol_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$PACK_PROTOCOL" >/tmp/world-machine-pack-protocol-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-pack-protocol:"
      cat /tmp/world-machine-pack-protocol-boundary-check
      failed=1
    fi
  done
fi

'''
bundle_block = protocol_block + '''if [[ -d "$PACK_BUNDLE" ]]; then
  for token in "${pack_bundle_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$PACK_BUNDLE" >/tmp/world-machine-pack-bundle-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-pack-bundle:"
      cat /tmp/world-machine-pack-bundle-boundary-check
      failed=1
    fi
  done
fi

'''
if protocol_block not in s:
    raise SystemExit('boundary protocol block not found')
s = s.replace(protocol_block, bundle_block, 1)
p.write_text(s)

# Make future bundle changes exercise the macOS external-Pack gate.
p = Path('.github/workflows/ci.yml')
s = p.read_text()
s = s.replace("              - 'crates/world-pack-protocol/**'\n", "              - 'crates/world-pack-protocol/**'\n              - 'crates/world-pack-bundle/**'\n", 1)
p.write_text(s)

# Authoring docs.
p = Path('crates/world-pack-server/README.md')
s = p.read_text()
extra = '''
For distribution, `write_current_exe_bundle(&registration.descriptor, path)` writes a portable single-file `.worldpack`. Bundle v1 embeds exactly one executable, rewrites runtime identity to the bundle-owned `program`, and carries no launcher arguments. Desktop installation parses and verifies the bundle before materializing it into the managed Pack store; it does not execute source bundle code during installation.
'''
if 'write_current_exe_bundle' not in s:
    s += extra
p.write_text(s)
