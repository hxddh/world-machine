from pathlib import Path

# Catalog: static preview + approved-content evidence + reviewed installation.
p = Path('crates/world-pack-catalog/src/lib.rs')
s = p.read_text()
s = s.replace(
    'use world_pack_bundle::PackBundle;\n',
    'use world_pack_bundle::{PackBundle, PackBundleHeader, PACK_BUNDLE_SUFFIX};\n',
    1,
)
s = s.replace(
    'use world_pack_protocol::PackManifest;\n',
    'use world_pack_protocol::{PackDescriptor, PackManifest};\n',
    1,
)
insert_after = '''pub enum PackAvailability {
    Ready,
    Disabled,
    Invalid { reason: String },
    MissingVersion { installed_versions: Vec<String> },
    NotInstalled,
}
'''
preview_types = insert_after + '''
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
    pub source_path: PathBuf,
    pub kind: PackInstallKind,
    pub pack: WorldPackRef,
    pub title: String,
    pub description: String,
    pub runtime_name: String,
    pub program_bytes: u64,
    pub program_sha256: String,
    evidence: PackInstallEvidence,
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
'''
if insert_after not in s:
    raise SystemExit('availability marker missing')
s = s.replace(insert_after, preview_types, 1)
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
new_install = '''    /// Inspect installable Pack content without launching it or mutating the catalog.
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
                let managed_pin = managed.current_pin().map_err(process_error)?;
                if managed_pin.command_sha256() != pin.command_sha256() {
                    cleanup_managed_pack_identity(&self.path, &preview.pack);
                    return Err(reviewed_content_changed(
                        &preview.pack,
                        "executable changed while it was copied into the managed store",
                    ));
                }
                self.record_managed_install(managed)
            }
        }
    }

    fn inspect_bundle(&self, bundle_path: &Path) -> Result<PackInstallPreview, CatalogError> {
        let source_path = bundle_path.canonicalize().map_err(|error| CatalogError::Io {
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
'''
if old_install not in s:
    raise SystemExit('catalog install block not found')
s = s.replace(old_install, new_install, 1)
# Error variant + formatter + helper.
s = s.replace(
'''    ManagedDestinationExists(WorldPackRef),
    PackIdentityChanged {
''',
'''    ManagedDestinationExists(WorldPackRef),
    ReviewedContentChanged {
        pack: WorldPackRef,
        reason: String,
    },
    PackIdentityChanged {
''',
1)
s = s.replace(
'''            Self::ManagedDestinationExists(pack) => write!(
                f,
                "managed Pack destination already exists for {}@{}",
                pack.id, pack.version
            ),
''',
'''            Self::ManagedDestinationExists(pack) => write!(
                f,
                "managed Pack destination already exists for {}@{}",
                pack.id, pack.version
            ),
            Self::ReviewedContentChanged { pack, reason } => write!(
                f,
                "reviewed Pack {}@{} changed before installation: {reason}",
                pack.id, pack.version
            ),
''',
1)
helper_marker = '''fn bundle_error(error: impl fmt::Display) -> CatalogError {
    CatalogError::Bundle(error.to_string())
}
'''
helper_repl = '''fn reviewed_content_changed(pack: &WorldPackRef, reason: impl Into<String>) -> CatalogError {
    CatalogError::ReviewedContentChanged {
        pack: pack.clone(),
        reason: reason.into(),
    }
}

''' + helper_marker
if helper_marker not in s:
    raise SystemExit('bundle error helper marker missing')
s = s.replace(helper_marker, helper_repl, 1)
# Tests: preview never executes; reviewed developer source changes are rejected; bundle replacement rejected.
test_marker = '''    #[test]
    fn portable_bundle_preserves_executable_suffix_in_managed_manifest() {
'''
extra_tests = r'''    #[test]
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
        let descriptor = PackDescriptor::new(pack("fixture.review-bundle", "v1"), "Bundle", "fixture");
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

'''
if test_marker not in s:
    raise SystemExit('catalog test insertion marker missing')
s = s.replace(test_marker, extra_tests + test_marker, 1)
p.write_text(s)

# Desktop: selecting a Pack only creates a static review; explicit second action installs it.
p = Path('apps/world-machine-desktop/src/main.rs')
s = p.read_text()
s = s.replace(
    'use world_pack_catalog::{InstalledPack, PackAvailability, PackCatalog};\n',
    'use world_pack_catalog::{\n    InstalledPack, PackAvailability, PackCatalog, PackInstallPreview,\n};\n',
    1,
)
s = s.replace(
'''    lineage: Option<LineageIndex>,
    status: Option<String>,
}
''',
'''    lineage: Option<LineageIndex>,
    pending_pack_install: Option<PackInstallPreview>,
    status: Option<String>,
}
''',
1)
# Replace install method body after picker selection with inspection only.
old_inner = '''            let _ = this.update(cx, |this, cx| {
                if this.pack_catalog.is_none() {
                    match PackCatalog::open(&this.pack_catalog_path) {
                        Ok(catalog) => this.pack_catalog = Some(catalog),
                        Err(error) => {
                            this.status = Some(format!(
                                "Could not open Installed Packs catalog {}: {error}",
                                this.pack_catalog_path.display()
                            ));
                            cx.notify();
                            return;
                        }
                    }
                }
                let catalog = this.pack_catalog.as_mut().unwrap();
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
                match this.rebuild_registry() {
                    Ok(()) => {
                        this.status = Some(format!(
                            "Installed {} · {} @ {} is now active for new Worlds",
                            installed.title, installed.pack.id, installed.pack.version
                        ));
                    }
                    Err(error) => {
                        this.status = Some(format!(
                            "Installed {} @ {}, but external Packs could not be activated: {error}",
                            installed.pack.id, installed.pack.version
                        ));
                    }
                }
                cx.notify();
            });
'''
new_inner = '''            let _ = this.update(cx, |this, cx| {
                if this.pack_catalog.is_none() {
                    match PackCatalog::open(&this.pack_catalog_path) {
                        Ok(catalog) => this.pack_catalog = Some(catalog),
                        Err(error) => {
                            this.status = Some(format!(
                                "Could not open Installed Packs catalog {}: {error}",
                                this.pack_catalog_path.display()
                            ));
                            cx.notify();
                            return;
                        }
                    }
                }
                let catalog = this.pack_catalog.as_ref().unwrap();
                match catalog.inspect_install(&manifest) {
                    Ok(preview) => {
                        this.status = Some(format!(
                            "Review {} @ {} before trusting its executable bytes",
                            preview.pack.id, preview.pack.version
                        ));
                        this.pending_pack_install = Some(preview);
                    }
                    Err(error) => {
                        this.pending_pack_install = None;
                        this.status = Some(format!(
                            "Could not inspect {}: {error}",
                            manifest.display()
                        ));
                    }
                }
                cx.notify();
            });
'''
if old_inner not in s:
    raise SystemExit('desktop install inner block missing')
s = s.replace(old_inner, new_inner, 1)
# New confirm/cancel methods before activate_pack.
method_marker = '''    fn activate_pack(&mut self, pack: WorldPackRef, cx: &mut Context<Self>) {
'''
new_methods = '''    fn confirm_pack_install(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = self.pending_pack_install.clone() else {
            return;
        };
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        let result = catalog.install_reviewed(&preview);
        self.pending_pack_install = None;
        match result {
            Ok(installed) => match self.rebuild_registry() {
                Ok(()) => {
                    self.status = Some(format!(
                        "Installed and trusted {} · {} @ {} is now active for new Worlds",
                        installed.title, installed.pack.id, installed.pack.version
                    ));
                }
                Err(error) => {
                    self.status = Some(format!(
                        "Installed {} @ {}, but external Packs could not be activated: {error}",
                        installed.pack.id, installed.pack.version
                    ));
                }
            },
            Err(error) => {
                self.status = Some(format!(
                    "Pack was not installed. Re-open it to review current content: {error}"
                ));
            }
        }
        cx.notify();
    }

    fn cancel_pack_install(&mut self, cx: &mut Context<Self>) {
        self.pending_pack_install = None;
        self.status = Some("Pack installation cancelled; no external code was installed.".into());
        cx.notify();
    }

'''
if method_marker not in s:
    raise SystemExit('activate method marker missing')
s = s.replace(method_marker, new_methods + method_marker, 1)
# Add review card before installed_pack_card.
card_marker = '''    fn installed_pack_card(&self, pack: InstalledPack, cx: &mut Context<Self>) -> impl IntoElement {
'''
card = '''    fn pack_install_review_card(
        &self,
        preview: PackInstallPreview,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let format = preview.kind.label();
        let size = format_program_size(preview.program_bytes);
        let source = preview.source_path.display().to_string();
        let pack = format!("{} @ {}", preview.pack.id, preview.pack.version);
        let runtime = preview.runtime_name.clone();
        let sha = preview.program_sha256.clone();

        div()
            .id("pack-install-review")
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xc7a85a))
            .bg(rgb(0xfffbeb))
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_lg().child("Review Pack Install"))
            .child(div().text_lg().child(preview.title))
            .child(div().text_sm().text_color(rgb(0x666666)).child(preview.description))
            .child(div().text_xs().child(format!("Identity · {pack}")))
            .child(div().text_xs().child(format!("Format · {format}")))
            .child(div().text_xs().child(format!("Will execute · {runtime}")))
            .child(div().text_xs().child(format!("Executable · {size}")))
            .child(div().text_xs().child(format!("SHA-256 · {sha}")))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777770))
                    .child(format!("Source · {source}")),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x6f5420))
                    .child("No Pack code has run. Install & Trust approves these exact executable bytes; any change before installation is rejected."),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("confirm-pack-install")
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x8c6a23))
                            .child("Install & Trust")
                            .on_click(cx.listener(|this, _, _, cx| this.confirm_pack_install(cx))),
                    )
                    .child(
                        div()
                            .id("cancel-pack-install")
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0xd9d9d3))
                            .child("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| this.cancel_pack_install(cx))),
                    ),
            )
    }

'''
if card_marker not in s:
    raise SystemExit('installed card marker missing')
s = s.replace(card_marker, card + card_marker, 1)
# Insert review card into body after intro copy.
intro = '''            .child(div().text_sm().text_color(rgb(0x666666)).child(
                "Worlds are portable documents. Double-click an external .world to edit it in place; Import copies it into My Worlds.",
            ))
            .child(div().text_sm().child("My Worlds"))
'''
intro_repl = '''            .child(div().text_sm().text_color(rgb(0x666666)).child(
                "Worlds are portable documents. Double-click an external .world to edit it in place; Import copies it into My Worlds.",
            ));

        if let Some(preview) = self.pending_pack_install.clone() {
            body = body.child(self.pack_install_review_card(preview, cx));
        }

        body = body
            .child(div().text_sm().child("My Worlds"))
'''
if intro not in s:
    raise SystemExit('home body intro marker missing')
s = s.replace(intro, intro_repl, 1)
# Initialization.
s = s.replace(
'''                documents,
                lineage,
                status,
''',
'''                documents,
                lineage,
                pending_pack_install: None,
                status,
''',
1)
# Add pure size formatter before lineage_branch_label.
label_marker = '''#[cfg(target_os = "macos")]
fn lineage_branch_label(branch: &WorldBranchCause) -> String {
'''
formatter = '''#[cfg(target_os = "macos")]
fn format_program_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    if bytes >= MIB {
        format!("{:.1} MiB · {bytes} bytes", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB · {bytes} bytes", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} bytes")
    }
}

'''
if label_marker not in s:
    raise SystemExit('lineage label marker missing')
s = s.replace(label_marker, formatter + label_marker, 1)
p.write_text(s)
