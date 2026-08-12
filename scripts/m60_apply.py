from pathlib import Path

# Process adapter: expose a headless durable activation probe over the same Host seam.
p = Path('crates/world-pack-process/src/lib.rs')
s = p.read_text()
s = s.replace(
    'use world_host::{HostError, WorldDescriptor, WorldPackSource, WorldRegistration, WorldSession};\n',
    'use world_host::{\n    HostError, WorldDescriptor, WorldPackSource, WorldRegistration, WorldRegistry, WorldSession,\n};\n',
    1,
)
pin_marker = '''#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessPackPin {
'''
probe_type = '''#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessPackProbe {
    pub pack: WorldPackRef,
    pub created_title: String,
    pub created_world_time: u64,
    pub reopened_title: String,
    pub reopened_world_time: u64,
}

'''
if pin_marker not in s:
    raise SystemExit('process pin marker missing')
s = s.replace(pin_marker, probe_type + pin_marker, 1)
method_marker = '''    pub fn verify_pin(&self) -> Result<(), HostError> {
        let Some(expected) = self.pin.as_ref() else {
            return Ok(());
        };
        let current = self.current_pin()?;
        if current != *expected {
            return Err(HostError::session(format!(
                "external Pack content pin mismatch for {}@{}: expected manifest sha256 {} and executable sha256 {}, found manifest sha256 {} and executable sha256 {}",
                self.descriptor.pack.id, self.descriptor.pack.version,
                expected.manifest_sha256(), expected.command_sha256(),
                current.manifest_sha256(), current.command_sha256(),
            )));
        }
        Ok(())
    }
'''
probe_method = method_marker + '''
    /// Launch the already-approved Pack and prove the minimum durable World contract:
    /// exact Describe handshake, Create/Snapshot, Archive, then a fresh-process Open/Snapshot.
    /// No business command is invoked and World time is never advanced by the probe itself.
    pub fn probe_durable(&self) -> Result<ProcessPackProbe, HostError> {
        self.verify_pin()?;
        let source = ProcessPackSource::from_packs(vec![self.clone()]);
        let mut registry = WorldRegistry::new();
        registry.install_source(&source)?;

        let created = registry.create_exact(&self.descriptor.pack)?;
        let created_snapshot = created.snapshot();
        let archive = created.archive()?.ok_or_else(|| {
            HostError::session(format!(
                "external Pack {}@{} does not provide a durable archive",
                self.descriptor.pack.id, self.descriptor.pack.version
            ))
        })?;
        drop(created);

        let reopened = registry.open_archive(&archive)?;
        let reopened_snapshot = reopened.snapshot();
        Ok(ProcessPackProbe {
            pack: self.descriptor.pack.clone(),
            created_title: created_snapshot.title,
            created_world_time: created_snapshot.world_time,
            reopened_title: reopened_snapshot.title,
            reopened_world_time: reopened_snapshot.world_time,
        })
    }
'''
if method_marker not in s:
    raise SystemExit('process verify pin marker missing')
s = s.replace(method_marker, probe_method, 1)
# Add probe tests before timeout test.
test_marker = '''    #[cfg(unix)]
    #[test]
    fn hung_process_is_timed_out_and_terminated() {
'''
probe_tests = '''    #[cfg(unix)]
    #[test]
    fn durable_probe_creates_archives_and_reopens_in_a_fresh_process() {
        let root = temp_dir("durable-probe");
        let runtime = root.join("runtime.sh");
        let archive = WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: descriptor().pack.clone(),
            world_time: 3,
            events: Vec::new(),
            pending: Vec::new(),
        };
        write_fixture_process(
            &runtime,
            &[
                response_line(
                    1,
                    PackResponse::Descriptor {
                        descriptor: descriptor(),
                    },
                ),
                response_line(
                    2,
                    PackResponse::Snapshot {
                        snapshot: wire_snapshot(3, "Created for probe"),
                    },
                ),
                response_line(
                    3,
                    PackResponse::Archive {
                        archive: Some(archive),
                    },
                ),
            ],
        );
        let manifest = PackManifest::process(descriptor(), "runtime.sh", Vec::new());
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();
        let pack = ProcessPack::load(manifest_path).unwrap();
        let pin = pack.current_pin().unwrap();

        let probe = pack.with_pin(pin).probe_durable().unwrap();
        assert_eq!(probe.pack, descriptor().pack);
        assert_eq!(probe.created_title, "Created for probe");
        assert_eq!(probe.created_world_time, 3);
        assert_eq!(probe.reopened_title, "Created for probe");
        assert_eq!(probe.reopened_world_time, 3);
    }

    #[cfg(unix)]
    #[test]
    fn durable_probe_rejects_packs_without_archives() {
        let root = temp_dir("durable-probe-no-archive");
        let runtime = root.join("runtime.sh");
        write_fixture_process(
            &runtime,
            &[
                response_line(
                    1,
                    PackResponse::Descriptor {
                        descriptor: descriptor(),
                    },
                ),
                response_line(
                    2,
                    PackResponse::Snapshot {
                        snapshot: wire_snapshot(0, "Created without archive"),
                    },
                ),
                response_line(3, PackResponse::Archive { archive: None }),
            ],
        );
        let manifest = PackManifest::process(descriptor(), "runtime.sh", Vec::new());
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();
        let pack = ProcessPack::load(manifest_path).unwrap();
        let pin = pack.current_pin().unwrap();

        let error = pack.with_pin(pin).probe_durable().unwrap_err();
        assert!(error.to_string().contains("does not provide a durable archive"));
    }

'''
if test_marker not in s:
    raise SystemExit('process test insertion marker missing')
s = s.replace(test_marker, probe_tests + test_marker, 1)
p.write_text(s)

# Catalog: install into durable quarantine, probe disabled Packs, preserve current active version.
p = Path('crates/world-pack-catalog/src/lib.rs')
s = p.read_text()
s = s.replace(
    'use world_pack_process::{ProcessPack, ProcessPackPin, ProcessPackSource};\n',
    'use world_pack_process::{ProcessPack, ProcessPackPin, ProcessPackProbe, ProcessPackSource};\n',
    1,
)
old_entry = '''    pub fn install_reviewed(
        &mut self,
        preview: &PackInstallPreview,
    ) -> Result<InstalledPack, CatalogError> {
        if self.entry(&preview.pack).is_some() {
            return Err(CatalogError::AlreadyInstalled(preview.pack.clone()));
        }

        match &preview.evidence {
'''
new_entry = '''    pub fn install_reviewed(
        &mut self,
        preview: &PackInstallPreview,
    ) -> Result<InstalledPack, CatalogError> {
        self.install_reviewed_with_activation(preview, true)
    }

    /// Persist trusted bytes in a disabled, non-active quarantine state. This is the
    /// Desktop install path: a crash before the activation probe finishes cannot make
    /// never-probed external code eligible for automatic registration on restart.
    pub fn install_reviewed_pending_probe(
        &mut self,
        preview: &PackInstallPreview,
    ) -> Result<InstalledPack, CatalogError> {
        self.install_reviewed_with_activation(preview, false)
    }

    fn install_reviewed_with_activation(
        &mut self,
        preview: &PackInstallPreview,
        activate_now: bool,
    ) -> Result<InstalledPack, CatalogError> {
        if self.entry(&preview.pack).is_some() {
            return Err(CatalogError::AlreadyInstalled(preview.pack.clone()));
        }

        match &preview.evidence {
'''
if old_entry not in s:
    raise SystemExit('catalog install_reviewed marker missing')
s = s.replace(old_entry, new_entry, 1)
s = s.replace('self.record_managed_install(managed)\n', 'self.record_managed_install(managed, activate_now)\n', 2)
# Probe method before set_enabled.
set_enabled_marker = '''    pub fn set_enabled(&mut self, pack: &WorldPackRef, enabled: bool) -> Result<(), CatalogError> {
'''
probe_catalog = '''    /// Execute a trusted managed Pack only for a bounded durable activation probe.
    /// Disabled Packs are intentionally probeable so the user can explicitly retry them.
    pub fn probe(&self, pack: &WorldPackRef) -> Result<ProcessPackProbe, CatalogError> {
        let entry = self
            .entry(pack)
            .ok_or_else(|| CatalogError::NotInstalled(pack.clone()))?;
        let verified = self.verified_pack(entry)?;
        verified.probe_durable().map_err(process_error)
    }

'''
if set_enabled_marker not in s:
    raise SystemExit('catalog set_enabled marker missing')
s = s.replace(set_enabled_marker, probe_catalog + set_enabled_marker, 1)
# record_managed_install activation flag/state.
s = s.replace(
'''    fn record_managed_install(
        &mut self,
        managed: ProcessPack,
    ) -> Result<InstalledPack, CatalogError> {
''',
'''    fn record_managed_install(
        &mut self,
        managed: ProcessPack,
        activate_now: bool,
    ) -> Result<InstalledPack, CatalogError> {
''',
1)
s = s.replace(
'''                enabled: true,
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
''',
'''                enabled: activate_now,
                active: activate_now,
                managed: true,
            };

            let mut entries = self.entries.clone();
            if activate_now {
                for entry in entries
                    .iter_mut()
                    .filter(|entry| entry.pack.id == installed.pack.id)
                {
                    entry.active = false;
                }
            }
''',
1)
# Existing in-crate calls in tests to record_managed_install, if any, need true.
s = s.replace('record_managed_install(managed).unwrap()', 'record_managed_install(managed, true).unwrap()')
# Add pending-state test before portable suffix test.
catalog_test_marker = '''    #[test]
    fn portable_bundle_preserves_executable_suffix_in_managed_manifest() {
'''
pending_test = '''    #[test]
    fn pending_probe_install_preserves_existing_active_version() {
        let root = temp_dir("pending-probe-version");
        let v1_manifest = write_pack(&root, "fixture.probe", "z-old");
        let v2_manifest = write_pack(&root, "fixture.probe", "a-new");
        let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();

        let v1 = catalog.install_manifest(&v1_manifest).unwrap();
        assert!(v1.enabled && v1.active);
        let preview = catalog.inspect_install(&v2_manifest).unwrap();
        let v2 = catalog.install_reviewed_pending_probe(&preview).unwrap();
        assert!(!v2.enabled && !v2.active);

        let stored_v1 = catalog.entry(&pack("fixture.probe", "z-old")).unwrap();
        let stored_v2 = catalog.entry(&pack("fixture.probe", "a-new")).unwrap();
        assert!(stored_v1.enabled && stored_v1.active);
        assert!(!stored_v2.enabled && !stored_v2.active);
    }

'''
if catalog_test_marker not in s:
    raise SystemExit('catalog test marker missing')
s = s.replace(catalog_test_marker, pending_test + catalog_test_marker, 1)
p.write_text(s)

# Real external Pack E2E: quarantine -> durable probe -> enable -> registry.
p = Path('apps/tiny-society-pack/tests/external_pack.rs')
s = p.read_text()
append = '''
#[test]
fn tiny_society_pending_install_is_probed_before_enablement() {
    let binary = env!("CARGO_BIN_EXE_tiny-society-pack");
    let root = temp_dir();
    let bundle_path = root.join("tiny-society-probe.worldpack");
    let status = Command::new(binary)
        .arg("--write-bundle")
        .arg(&bundle_path)
        .status()
        .unwrap();
    assert!(status.success());

    let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
    let preview = catalog.inspect_install(&bundle_path).unwrap();
    let installed = catalog.install_reviewed_pending_probe(&preview).unwrap();
    assert!(!installed.enabled);
    assert!(!installed.active);
    assert!(catalog.trusted_source().unwrap().packs().is_empty());

    fs::remove_file(&bundle_path).unwrap();
    let probe = catalog.probe(&installed.pack).unwrap();
    assert_eq!(probe.pack, installed.pack);
    assert_eq!(probe.created_world_time, probe.reopened_world_time);

    catalog.set_enabled(&installed.pack, true).unwrap();
    catalog.activate(&installed.pack).unwrap();
    let source = catalog.trusted_source().unwrap();
    assert_eq!(source.packs().len(), 1);
    let mut registry = WorldRegistry::new();
    registry.install_source(&source).unwrap();
    let session = registry.create(TINY_SOCIETY_PACK_ID).unwrap();
    assert_eq!(session.pack(), installed.pack);
}
'''
if 'tiny_society_pending_install_is_probed_before_enablement' not in s:
    s += append
p.write_text(s)

# Desktop: run probe on GPUI background executor and expose Test & Enable for disabled Packs.
p = Path('apps/world-machine-desktop/src/main.rs')
s = p.read_text()
s = s.replace(
    'use std::cell::RefCell;\n',
    'use std::cell::RefCell;\n',
    1,
)
s = s.replace(
'''    pending_pack_install: Option<PackInstallPreview>,
    status: Option<String>,
''',
'''    pending_pack_install: Option<PackInstallPreview>,
    probing_packs: Vec<WorldPackRef>,
    status: Option<String>,
''',
1)
old_confirm = '''    fn confirm_pack_install(&mut self, cx: &mut Context<Self>) {
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
'''
new_confirm = '''    fn confirm_pack_install(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = self.pending_pack_install.clone() else {
            return;
        };
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        let result = catalog.install_reviewed_pending_probe(&preview);
        self.pending_pack_install = None;
        match result {
            Ok(installed) => {
                self.start_pack_probe(installed.pack, true, cx);
            }
            Err(error) => {
                self.status = Some(format!(
                    "Pack was not installed. Re-open it to review current content: {error}"
                ));
                cx.notify();
            }
        }
    }

    fn is_pack_probing(&self, pack: &WorldPackRef) -> bool {
        self.probing_packs.iter().any(|candidate| candidate == pack)
    }

    fn start_pack_probe(
        &mut self,
        pack: WorldPackRef,
        activate_on_success: bool,
        cx: &mut Context<Self>,
    ) {
        if self.is_pack_probing(&pack) {
            return;
        }
        let Some(catalog) = self.pack_catalog.clone() else {
            return;
        };
        self.probing_packs.push(pack.clone());
        self.status = Some(format!(
            "Testing trusted Pack {} @ {} · Create → Archive → fresh-process Open…",
            pack.id, pack.version
        ));
        cx.notify();

        let probe_pack = pack.clone();
        let task = cx
            .background_executor()
            .spawn(async move { catalog.probe(&probe_pack) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.probing_packs.retain(|candidate| candidate != &pack);
                match result {
                    Ok(probe) => {
                        let transition = this
                            .pack_catalog
                            .as_mut()
                            .ok_or_else(|| "Installed Packs catalog is unavailable".to_string())
                            .and_then(|catalog| {
                                catalog
                                    .set_enabled(&pack, true)
                                    .map_err(|error| error.to_string())?;
                                if activate_on_success {
                                    catalog.activate(&pack).map_err(|error| error.to_string())?;
                                }
                                Ok(())
                            });
                        match transition {
                            Ok(()) => match this.rebuild_registry() {
                                Ok(()) => {
                                    this.status = Some(format!(
                                        "Trusted and tested {} @ {} · durable Create/Archive/Open succeeded · World time {} → {}",
                                        pack.id,
                                        pack.version,
                                        probe.created_world_time,
                                        probe.reopened_world_time
                                    ));
                                }
                                Err(error) => {
                                    this.status = Some(format!(
                                        "Pack {} @ {} passed its durable probe, but Registry rebuild failed: {error}",
                                        pack.id, pack.version
                                    ));
                                }
                            },
                            Err(error) => {
                                this.status = Some(format!(
                                    "Pack {} @ {} passed its durable probe, but could not be enabled: {error}",
                                    pack.id, pack.version
                                ));
                            }
                        }
                    }
                    Err(error) => {
                        let _ = this.rebuild_registry();
                        this.status = Some(format!(
                            "Installed and trusted {} @ {}, but its durable activation probe failed. The Pack remains disabled: {error}",
                            pack.id, pack.version
                        ));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
'''
if old_confirm not in s:
    raise SystemExit('desktop confirm marker missing')
s = s.replace(old_confirm, new_confirm, 1)
# Installed card state/actions.
old_state = '''        let state = match &availability {
            PackAvailability::Ready if pack.active => "Active".to_string(),
            PackAvailability::Ready => "Historical · available for saved Worlds".to_string(),
            PackAvailability::Disabled => "Disabled".to_string(),
            PackAvailability::Invalid { reason } => format!("Invalid · {reason}"),
            PackAvailability::MissingVersion { .. } => "Missing exact version".to_string(),
            PackAvailability::NotInstalled => "Not installed".to_string(),
        };
        let activate_pack = pack.pack.clone();
        let toggle_pack = pack.pack.clone();
        let enabled = pack.enabled;
        let active = pack.active;

        let mut actions = div().flex().gap_2();
        if enabled && !active {
'''
new_state = '''        let probing = self.is_pack_probing(&pack.pack);
        let state = if probing {
            "Testing durable round-trip…".to_string()
        } else {
            match &availability {
                PackAvailability::Ready if pack.active => "Active".to_string(),
                PackAvailability::Ready => "Historical · available for saved Worlds".to_string(),
                PackAvailability::Disabled => "Disabled · trusted, not runnable until tested".to_string(),
                PackAvailability::Invalid { reason } => format!("Invalid · {reason}"),
                PackAvailability::MissingVersion { .. } => "Missing exact version".to_string(),
                PackAvailability::NotInstalled => "Not installed".to_string(),
            }
        };
        let activate_pack = pack.pack.clone();
        let toggle_pack = pack.pack.clone();
        let test_pack = pack.pack.clone();
        let enabled = pack.enabled;
        let active = pack.active;

        let mut actions = div().flex().gap_2();
        if !probing && enabled && !active {
'''
if old_state not in s:
    raise SystemExit('desktop installed state marker missing')
s = s.replace(old_state, new_state, 1)
old_toggle = '''        actions = actions.child(
            div()
                .id(SharedString::from(format!(
                    "toggle-pack-{}-{}",
                    pack.pack.id, pack.pack.version
                )))
                .cursor_pointer()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xd9d9d3))
                .text_sm()
                .child(if enabled { "Disable" } else { "Enable" })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_pack_enabled(toggle_pack.clone(), !enabled, cx)
                })),
        );
'''
new_toggle = '''        if !probing && enabled {
            actions = actions.child(
                div()
                    .id(SharedString::from(format!(
                        "toggle-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xd9d9d3))
                    .text_sm()
                    .child("Disable")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_pack_enabled(toggle_pack.clone(), false, cx)
                    })),
            );
        } else if !probing && !enabled {
            actions = actions.child(
                div()
                    .id(SharedString::from(format!(
                        "test-enable-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xd9d9d3))
                    .text_sm()
                    .child("Test & Enable")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.start_pack_probe(test_pack.clone(), false, cx)
                    })),
            );
        }
'''
if old_toggle not in s:
    raise SystemExit('desktop toggle action marker missing')
s = s.replace(old_toggle, new_toggle, 1)
# Missing pack copy should say Test & Enable.
s = s.replace(
    '"Could not open World: {} @ {} is installed but disabled. Enable it under Installed Packs.",',
    '"Could not open World: {} @ {} is installed but disabled. Use Test & Enable under Installed Packs.",',
    1,
)
# Initialization.
s = s.replace(
'''                pending_pack_install: None,
                status,
''',
'''                pending_pack_install: None,
                probing_packs: Vec::new(),
                status,
''',
1)
p.write_text(s)
