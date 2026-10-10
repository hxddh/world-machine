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
/// Installed Packs go in first and the in-process copies only fill the gaps
/// they leave. The two overlap: a World that ships as a real Pack is also
/// compiled into this binary, and the Host refuses to register one Pack id and
/// version twice — rightly, since two registrations claiming to be the same
/// Pack version cannot both be it. Installing the built-ins first made that
/// collision fail the Pack the observer actually installed, so Home reported a
/// Registry rebuild failure on every launch and the installed Pack never
/// became reachable. An installed Pack is the one that wins.
pub(crate) fn build_registry(
    catalog: Option<&PackCatalog>,
) -> Result<world_host::WorldRegistry, String> {
    let mut registry = world_host::WorldRegistry::new();
    if let Some(catalog) = catalog {
        let source = catalog
            .trusted_source()
            .map_err(|error| error.to_string())?;
        let source = world_voice::with_settings(source, world_voice::pack_settings());
        let source = world_voice::with_crash_logs(
            source,
            crate::diagnostics::crash_log_dir().map(|dir| dir.join("Packs")),
        );
        registry
            .install_source(&source)
            .map_err(|error| error.to_string())?;
    }
    registry
        .install_fallback_source(&world_builtins::BuiltinWorlds)
        .map_err(|error| error.to_string())?;
    Ok(registry)
}
