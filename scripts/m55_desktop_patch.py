from pathlib import Path

p = Path('apps/world-machine-desktop/src/main.rs')
s = p.read_text()

# Imports and constants.
s = s.replace(
    'use world_lineage::LineageIndex;\n',
    'use world_lineage::LineageIndex;\n#[cfg(target_os = "macos")]\nuse world_pack_catalog::{InstalledPack, PackAvailability, PackCatalog};\n#[cfg(target_os = "macos")]\nuse world_persistence::WorldPackRef;\n',
    1,
)
s = s.replace(
    'const LIBRARY_OVERRIDE_ENV: &str = "WORLD_MACHINE_LIBRARY_DIR";\n',
    'const LIBRARY_OVERRIDE_ENV: &str = "WORLD_MACHINE_LIBRARY_DIR";\n#[cfg(target_os = "macos")]\nconst PACK_CATALOG_OVERRIDE_ENV: &str = "WORLD_MACHINE_PACK_CATALOG";\n',
    1,
)

# Home state.
s = s.replace(
    '''struct WorldMachineHome {
    registry: Arc<world_host::WorldRegistry>,
    library: Arc<WorldLibrary>,
    documents: Vec<WorldDocumentSummary>,''',
    '''struct WorldMachineHome {
    registry: Arc<world_host::WorldRegistry>,
    library: Arc<WorldLibrary>,
    pack_catalog: Option<PackCatalog>,
    pack_catalog_path: PathBuf,
    documents: Vec<WorldDocumentSummary>,''',
    1,
)

# Add Pack management methods before refresh_documents.
marker = '    fn refresh_documents(&mut self) {'
methods = '''    fn rebuild_registry(&mut self) -> Result<(), String> {
        let registry = build_registry(self.pack_catalog.as_ref())?;
        self.registry = Arc::new(registry);
        Ok(())
    }

    fn install_pack(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Install World Pack".into()),
        });
        cx.spawn(async move |this, cx| {
            let manifest = match picker.await {
                Ok(Ok(Some(mut paths))) => paths.pop(),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(format!("Could not open Install Pack dialog: {error}"));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(format!("Install Pack dialog was interrupted: {error}"));
                        cx.notify();
                    });
                    return;
                }
            };
            let Some(manifest) = manifest else { return; };
            let _ = this.update(cx, |this, cx| {
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
                let installed = match this.pack_catalog.as_mut().unwrap().install_manifest(&manifest) {
                    Ok(installed) => installed,
                    Err(error) => {
                        this.status = Some(format!("Could not install {}: {error}", manifest.display()));
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
        })
        .detach();
    }

    fn activate_pack(&mut self, pack: WorldPackRef, cx: &mut Context<Self>) {
        let Some(catalog) = self.pack_catalog.as_mut() else { return; };
        match catalog.activate(&pack) {
            Ok(()) => match self.rebuild_registry() {
                Ok(()) => self.status = Some(format!(
                    "Activated {} @ {} for new Worlds",
                    pack.id, pack.version
                )),
                Err(error) => self.status = Some(format!(
                    "Changed active Pack to {} @ {}, but Registry rebuild failed: {error}",
                    pack.id, pack.version
                )),
            },
            Err(error) => self.status = Some(format!(
                "Could not activate {} @ {}: {error}", pack.id, pack.version
            )),
        }
        cx.notify();
    }

    fn set_pack_enabled(&mut self, pack: WorldPackRef, enabled: bool, cx: &mut Context<Self>) {
        let Some(catalog) = self.pack_catalog.as_mut() else { return; };
        match catalog.set_enabled(&pack, enabled) {
            Ok(()) => match self.rebuild_registry() {
                Ok(()) => self.status = Some(format!(
                    "{} {} @ {}",
                    if enabled { "Enabled" } else { "Disabled" },
                    pack.id,
                    pack.version
                )),
                Err(error) => self.status = Some(format!(
                    "Updated {} @ {}, but Registry rebuild failed: {error}",
                    pack.id, pack.version
                )),
            },
            Err(error) => self.status = Some(format!(
                "Could not {} {} @ {}: {error}",
                if enabled { "enable" } else { "disable" },
                pack.id,
                pack.version
            )),
        }
        cx.notify();
    }

    fn missing_pack_message(&self, pack: &WorldPackRef) -> String {
        let availability = self
            .pack_catalog
            .as_ref()
            .map(|catalog| catalog.availability(pack))
            .unwrap_or(PackAvailability::NotInstalled);
        match availability {
            PackAvailability::Ready => format!(
                "Could not open World: {} @ {} should be available, but is not registered",
                pack.id, pack.version
            ),
            PackAvailability::Disabled => format!(
                "Could not open World: {} @ {} is installed but disabled. Enable it under Installed Packs.",
                pack.id, pack.version
            ),
            PackAvailability::Invalid { reason } => format!(
                "Could not open World: {} @ {} is installed but no longer matches its approved content: {reason}",
                pack.id, pack.version
            ),
            PackAvailability::MissingVersion { installed_versions } => format!(
                "Could not open World: it requires {} @ {}, but installed versions are {}. Install that exact Pack version.",
                pack.id,
                pack.version,
                installed_versions.join(", ")
            ),
            PackAvailability::NotInstalled => format!(
                "Could not open World: it requires {} @ {}, which is not installed. Use Install Pack… to add that exact version.",
                pack.id, pack.version
            ),
        }
    }

'''
if marker not in s:
    raise SystemExit('refresh_documents marker not found')
s = s.replace(marker, methods + marker, 1)

# Exact missing-Pack diagnostic before library open.
old_open = '''    fn open_document(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        let title = self
            .documents
            .iter()
            .find(|document| document.id == document_id)
            .and_then(|document| self.registry.descriptor_for(&document.pack))
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| document_id.to_string());'''
new_open = '''    fn open_document(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        let summary = self.documents.iter().find(|document| document.id == document_id);
        if let Some(document) = summary {
            if self.registry.descriptor_for(&document.pack).is_none() {
                self.status = Some(self.missing_pack_message(&document.pack));
                cx.notify();
                return;
            }
        }
        let title = summary
            .and_then(|document| self.registry.descriptor_for(&document.pack))
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| document_id.to_string());'''
if old_open not in s:
    raise SystemExit('open_document marker not found')
s = s.replace(old_open, new_open, 1)

# Installed Pack card before new_world_card.
marker = '    fn new_world_card(\n'
card = '''    fn installed_pack_card(&self, pack: InstalledPack, cx: &mut Context<Self>) -> impl IntoElement {
        let availability = self
            .pack_catalog
            .as_ref()
            .map(|catalog| catalog.availability(&pack.pack))
            .unwrap_or(PackAvailability::NotInstalled);
        let state = match &availability {
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
            actions = actions.child(
                div()
                    .id(SharedString::from(format!("activate-pack-{}-{}", pack.pack.id, pack.pack.version)))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xd9d9d3))
                    .text_sm()
                    .child("Activate")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.activate_pack(activate_pack.clone(), cx)
                    })),
            );
        }
        actions = actions.child(
            div()
                .id(SharedString::from(format!("toggle-pack-{}-{}", pack.pack.id, pack.pack.version)))
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

        div()
            .id(SharedString::from(format!("installed-pack-{}-{}", pack.pack.id, pack.pack.version)))
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd9d9d3))
            .bg(rgb(0xffffff))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child(pack.title))
                    .child(div().text_sm().text_color(rgb(0x666666)).child(pack.description))
                    .child(div().text_xs().text_color(rgb(0x8a8a82)).child(format!(
                        "{} @ {} · {state}", pack.pack.id, pack.pack.version
                    ))),
            )
            .child(actions)
    }

'''
if marker not in s:
    raise SystemExit('new_world_card marker not found')
s = s.replace(marker, card + marker, 1)

# Render installed Pack section and Install button.
old = '''        let mut available = div().w_full().flex().flex_col().gap_3();
        for descriptor in descriptors {
            available = available.child(self.new_world_card(descriptor, cx));
        }

        let refresh = div()'''
new = '''        let installed_packs = self
            .pack_catalog
            .as_ref()
            .map(|catalog| catalog.entries().to_vec())
            .unwrap_or_default();
        let mut installed = div().w_full().flex().flex_col().gap_3();
        if installed_packs.is_empty() {
            installed = installed.child(
                div()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xe1e1dc))
                    .text_sm()
                    .text_color(rgb(0x777770))
                    .child("No external Packs installed. Built-in Worlds remain available below."),
            );
        } else {
            for pack in installed_packs {
                installed = installed.child(self.installed_pack_card(pack, cx));
            }
        }

        let mut available = div().w_full().flex().flex_col().gap_3();
        for descriptor in descriptors {
            available = available.child(self.new_world_card(descriptor, cx));
        }

        let refresh = div()'''
if old not in s:
    raise SystemExit('render available marker not found')
s = s.replace(old, new, 1)

old_import = '''        let import = div()
            .id("import-world-file")'''
install_button = '''        let install_pack = div()
            .id("install-world-pack")
            .cursor_pointer()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd9d9d3))
            .text_sm()
            .child("Install Pack…")
            .on_click(cx.listener(|this, _, _, cx| this.install_pack(cx)));
        let import = div()
            .id("import-world-file")'''
if old_import not in s:
    raise SystemExit('import button marker not found')
s = s.replace(old_import, install_button, 1)

s = s.replace(
    '.child(div().flex().gap_2().child(import).child(refresh)),',
    '.child(div().flex().gap_2().child(install_pack).child(import).child(refresh)),',
    1,
)
s = s.replace(
    '''            .child(div().text_sm().child("My Worlds"))
            .child(saved)
            .child(div().text_sm().child("New World"))''',
    '''            .child(div().text_sm().child("My Worlds"))
            .child(saved)
            .child(div().text_sm().child("Installed Packs"))
            .child(installed)
            .child(div().text_sm().child("New World"))''',
    1,
)

# Catalog discovery/build helpers before new_document_id.
marker = '#[cfg(target_os = "macos")]\nfn new_document_id('
helpers = '''#[cfg(target_os = "macos")]
fn discover_pack_catalog_path(library: &WorldLibrary) -> PathBuf {
    if let Some(path) = env::var_os(PACK_CATALOG_OVERRIDE_ENV) {
        return PathBuf::from(path);
    }
    if env::var_os(LIBRARY_OVERRIDE_ENV).is_some() {
        return library.root().join(".world-machine-packs").join("catalog.json");
    }
    library
        .root()
        .parent()
        .unwrap_or_else(|| library.root())
        .join("Packs")
        .join("catalog.json")
}

#[cfg(target_os = "macos")]
fn build_registry(catalog: Option<&PackCatalog>) -> Result<world_host::WorldRegistry, String> {
    let mut registry = world_builtins::registry().map_err(|error| error.to_string())?;
    if let Some(catalog) = catalog {
        let source = catalog.trusted_source().map_err(|error| error.to_string())?;
        registry.install_source(&source).map_err(|error| error.to_string())?;
    }
    Ok(registry)
}

'''
if marker not in s:
    raise SystemExit('new_document_id helper marker not found')
s = s.replace(marker, helpers + marker, 1)

# Startup: library first, catalog second, registry assembled from catalog.
old_main = '''    let application = application();
    system_open::install(&application);
    let registry = Arc::new(world_builtins::registry()?);
    let library = Arc::new(discover_library()?);
    let (documents, lineage, status) = match library.list() {'''
new_main = '''    let application = application();
    system_open::install(&application);
    let library = Arc::new(discover_library()?);
    let pack_catalog_path = discover_pack_catalog_path(library.as_ref());
    let (pack_catalog, registry, pack_status) = match PackCatalog::open(&pack_catalog_path) {
        Ok(catalog) => match build_registry(Some(&catalog)) {
            Ok(registry) => (Some(catalog), Arc::new(registry), None),
            Err(error) => (
                Some(catalog),
                Arc::new(world_builtins::registry()?),
                Some(format!("Installed Packs were not activated: {error}")),
            ),
        },
        Err(error) => (
            None,
            Arc::new(world_builtins::registry()?),
            Some(format!(
                "Could not open Installed Packs catalog {}: {error}",
                pack_catalog_path.display()
            )),
        ),
    };
    let (documents, lineage, library_status) = match library.list() {'''
if old_main not in s:
    raise SystemExit('main startup marker not found')
s = s.replace(old_main, new_main, 1)

# Rename inner status occurrences in startup tuple only and combine statuses.
# Limit region between new startup and application.run.
start = s.index('    let (documents, lineage, library_status) = match library.list() {')
end = s.index('    application.run(move |cx: &mut App| {', start)
region = s[start:end]
region = region.replace('Some(format!("Could not build World lineage: {error}"))', 'Some(format!("Could not build World lineage: {error}"))')
# Tuple names need no inner change; add merge after match.
region += '    let status = pack_status.or(library_status);\n\n'
s = s[:start] + region + s[end:]

s = s.replace(
    '''            let mut home = WorldMachineHome {
                registry,
                library,
                documents,''',
    '''            let mut home = WorldMachineHome {
                registry,
                library,
                pack_catalog,
                pack_catalog_path,
                documents,''',
    1,
)

p.write_text(s)
