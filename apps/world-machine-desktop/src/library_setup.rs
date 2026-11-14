//! Where the Worlds folder and the Pack catalog are, and the registry of
//! Worlds the app can open. Moved out of `main.rs` as it stood; the
//! folders come from `platform.rs`.

#[allow(unused_imports)]
use super::*;

pub(crate) const LIBRARY_OVERRIDE_ENV: &str = "WORLD_MACHINE_LIBRARY_DIR";
pub(crate) const PACK_CATALOG_OVERRIDE_ENV: &str = "WORLD_MACHINE_PACK_CATALOG";

pub(crate) fn discover_library() -> std::io::Result<WorldLibrary> {
    if let Some(path) = env::var_os(LIBRARY_OVERRIDE_ENV) {
        return Ok(WorldLibrary::new(PathBuf::from(path)));
    }
    // On the Mac, `~/Library/Application Support/World Machine/Worlds`.
    let support = world_machine_desktop::platform::current()
        .support_dir()
        .ok_or_else(|| std::io::Error::other("the folder for the app's files is not known"))?;
    Ok(WorldLibrary::new(support.join("Worlds")))
}

pub(crate) fn discover_pack_catalog_path(library: &WorldLibrary) -> PathBuf {
    if let Some(path) = env::var_os(PACK_CATALOG_OVERRIDE_ENV) {
        return PathBuf::from(path);
    }
    if env::var_os(LIBRARY_OVERRIDE_ENV).is_some() {
        return library
            .root()
            .join(".world-machine-packs")
            .join("catalog.json");
    }
    library
        .root()
        .parent()
        .unwrap_or_else(|| library.root())
        .join("Packs")
        .join("catalog.json")
}

/// The Worlds this app can create and open.
///
/// The Worlds compiled into this binary run inside the app
/// and always win: a turn is a function call, never a round trip to a
/// child process that can time out and leave the World dead. Installed
/// Packs fill in the Worlds the app does not carry (the other included
/// Packs and any external one); an installed copy of a built-in World is never
/// verified or launched.
pub(crate) fn build_registry(
    catalog: Option<&PackCatalog>,
) -> Result<world_host::WorldRegistry, String> {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .install_source(&world_builtins::BuiltinWorlds)
        .map_err(|error| error.to_string())?;
    if let Some(catalog) = catalog {
        let source = catalog
            .trusted_source_except(builtin_pack_ids())
            .map_err(|error| error.to_string())?;
        let source = world_voice::with_settings(source, world_voice::pack_settings());
        let source = world_voice::with_crash_logs(
            source,
            crate::diagnostics::crash_log_dir().map(|dir| dir.join("Packs")),
        );
        registry
            .install_fallback_source(&source)
            .map_err(|error| error.to_string())?;
    }
    Ok(registry)
}

/// The ids of the Worlds this binary runs itself.
/// Worked out once: Home asks on every redraw.
pub(crate) fn builtin_pack_ids() -> &'static std::collections::BTreeSet<String> {
    use world_host::WorldPackSource;
    static IDS: std::sync::OnceLock<std::collections::BTreeSet<String>> =
        std::sync::OnceLock::new();
    IDS.get_or_init(|| {
        world_builtins::BuiltinWorlds
            .registrations()
            .map(|registrations| {
                registrations
                    .into_iter()
                    .map(|registration| registration.descriptor.pack.id)
                    .collect()
            })
            .unwrap_or_default()
    })
}
