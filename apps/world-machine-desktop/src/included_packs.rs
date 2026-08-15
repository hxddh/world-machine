use std::env;
use std::io;
use std::path::{Path, PathBuf};
use world_persistence::WorldPackRef;

const INCLUDED_PACKS_OVERRIDE_ENV: &str = "WORLD_MACHINE_INCLUDED_PACKS_DIR";
const INCLUDED_PACKS_RESOURCE_DIR: &str = "World Packs";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncludedPack {
    pub pack: WorldPackRef,
    pub title: &'static str,
    pub description: &'static str,
    pub experience: &'static str,
    pub featured: bool,
    pub path: PathBuf,
}

#[derive(Clone, Copy)]
struct IncludedPackSpec {
    id: &'static str,
    version: &'static str,
    title: &'static str,
    description: &'static str,
    experience: &'static str,
    featured: bool,
    file_name: &'static str,
}

const INCLUDED_PACKS: &[IncludedPackSpec] = &[
    IncludedPackSpec {
        id: "world-machine.pocket-universe",
        version: "0.15.0",
        title: "Pocket Universe",
        description: "Seed a tiny persistent world, let its inhabitants act, and watch choices, relationships, and repeated behavior compound into durable legacies.",
        experience: "Seed a place · Let it live · Branch what happens next",
        featured: true,
        file_name: "pocket-universe.worldpack",
    },
    IncludedPackSpec {
        id: "world-machine.micro-company",
        version: "0.1.0",
        title: "Micro Company",
        description: "Run a tiny product company where two actors make bounded decisions and the business can find traction or run out of cash.",
        experience: "Choose a direction · Watch demand and cash · Adapt",
        featured: false,
        file_name: "micro-company.worldpack",
    },
];

pub fn discover() -> io::Result<Vec<IncludedPack>> {
    if let Some(root) = env::var_os(INCLUDED_PACKS_OVERRIDE_ENV) {
        return Ok(discover_in(Path::new(&root)));
    }

    let executable = env::current_exe()?;
    let Some(root) = resource_root_for_executable(&executable) else {
        return Ok(Vec::new());
    };
    Ok(discover_in(&root))
}

fn resource_root_for_executable(executable: &Path) -> Option<PathBuf> {
    let macos = executable.parent()?;
    if macos.file_name()?.to_str()? != "MacOS" {
        return None;
    }
    let contents = macos.parent()?;
    Some(contents.join("Resources").join(INCLUDED_PACKS_RESOURCE_DIR))
}

fn discover_in(root: &Path) -> Vec<IncludedPack> {
    INCLUDED_PACKS
        .iter()
        .filter_map(|spec| {
            let path = root.join(spec.file_name);
            path.is_file().then(|| IncludedPack {
                pack: WorldPackRef::new(spec.id, spec.version),
                title: spec.title,
                description: spec.description,
                experience: spec.experience,
                featured: spec.featured,
                path,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NONCE: AtomicU64 = AtomicU64::new(1);

    fn scratch_dir() -> PathBuf {
        let nonce = NONCE.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        env::temp_dir().join(format!(
            "world-machine-included-packs-{}-{timestamp}-{nonce}",
            process::id()
        ))
    }

    #[test]
    fn bundled_resource_root_is_derived_only_from_a_macos_app_layout() {
        let executable =
            PathBuf::from("/tmp/World Machine.app/Contents/MacOS/world-machine-desktop");
        assert_eq!(
            resource_root_for_executable(&executable),
            Some(PathBuf::from(
                "/tmp/World Machine.app/Contents/Resources/World Packs"
            ))
        );
        assert_eq!(
            resource_root_for_executable(Path::new("/tmp/world-machine-desktop")),
            None
        );
    }

    #[test]
    fn discovery_uses_a_fixed_allowlist_instead_of_scanning_the_directory() {
        let root = scratch_dir();
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("pocket-universe.worldpack"), b"pocket").unwrap();
        fs::write(root.join("surprise.worldpack"), b"surprise").unwrap();

        let packs = discover_in(&root);
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].pack.id, "world-machine.pocket-universe");
        assert_eq!(packs[0].pack.version, "0.15.0");
        assert_eq!(packs[0].title, "Pocket Universe");
        assert!(packs[0].featured);
        assert_eq!(
            packs[0].experience,
            "Seed a place · Let it live · Branch what happens next"
        );
        assert_eq!(packs[0].path, root.join("pocket-universe.worldpack"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn included_pack_order_is_product_defined_and_stable() {
        let root = scratch_dir();
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("micro-company.worldpack"), b"company").unwrap();
        fs::write(root.join("pocket-universe.worldpack"), b"pocket").unwrap();

        let packs = discover_in(&root);
        assert_eq!(
            packs
                .iter()
                .map(|pack| pack.pack.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "world-machine.pocket-universe",
                "world-machine.micro-company"
            ]
        );
        assert_eq!(packs.iter().filter(|pack| pack.featured).count(), 1);
        assert!(packs[0].featured);
        assert!(!packs[1].featured);

        fs::remove_dir_all(root).unwrap();
    }
}
