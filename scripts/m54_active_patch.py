from pathlib import Path

p = Path('crates/world-pack-catalog/src/lib.rs')
s = p.read_text()

s = s.replace('use std::collections::BTreeSet;', 'use std::collections::{BTreeMap, BTreeSet};', 1)
s = s.replace('    pub enabled: bool,\n}', '    pub enabled: bool,\n    pub active: bool,\n}', 1)
s = s.replace('            enabled: true,\n        };', '            enabled: true,\n            active: true,\n        };', 1)

old_install = '''        let mut entries = self.entries.clone();
        entries.push(installed.clone());
        sort_entries(&mut entries);
        self.commit(entries)?;'''
new_install = '''        let mut entries = self.entries.clone();
        for entry in entries
            .iter_mut()
            .filter(|entry| entry.pack.id == installed.pack.id)
        {
            entry.active = false;
        }
        entries.push(installed.clone());
        sort_entries(&mut entries);
        self.commit(entries)?;'''
if old_install not in s:
    raise SystemExit('install marker not found')
s = s.replace(old_install, new_install, 1)

start = s.index('    pub fn set_enabled(')
end = s.index('    /// Re-validate every enabled entry', start)
replacement = '''    pub fn set_enabled(&mut self, pack: &WorldPackRef, enabled: bool) -> Result<(), CatalogError> {
        let mut entries = self.entries.clone();
        let index = entries
            .iter()
            .position(|entry| entry.pack == *pack)
            .ok_or_else(|| CatalogError::NotInstalled(pack.clone()))?;

        if enabled {
            if entries[index].enabled {
                return Ok(());
            }
            let has_active = entries.iter().any(|entry| {
                entry.pack.id == pack.id && entry.enabled && entry.active
            });
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

'''
s = s[:start] + replacement + s[end:]

old_source = '''    pub fn trusted_source(&self) -> Result<ProcessPackSource, CatalogError> {
        let mut packs = Vec::new();
        for entry in self.entries.iter().filter(|entry| entry.enabled) {
            packs.push(self.verified_pack(entry)?);
        }
        Ok(ProcessPackSource::from_packs(packs))
    }'''
new_source = '''    pub fn trusted_source(&self) -> Result<ProcessPackSource, CatalogError> {
        let mut entries = self
            .entries
            .iter()
            .filter(|entry| entry.enabled)
            .collect::<Vec<_>>();
        // Host intentionally makes the last registration for one Pack id active.
        // We only use ordering to encode the catalog's explicit `active` bit;
        // version strings are opaque and merely stabilize ordering among historical versions.
        entries.sort_by(|left, right| {
            (&left.pack.id, left.active, &left.pack.version)
                .cmp(&(&right.pack.id, right.active, &right.pack.version))
        });
        let packs = entries
            .into_iter()
            .map(|entry| self.verified_pack(entry))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ProcessPackSource::from_packs(packs))
    }'''
if old_source not in s:
    raise SystemExit('trusted_source marker not found')
s = s.replace(old_source, new_source, 1)

vstart = s.index('fn validate_entries(entries: &[InstalledPack])')
vend = s.index('\nfn sort_entries', vstart)
validation = '''fn validate_entries(entries: &[InstalledPack]) -> Result<(), CatalogError> {
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
}'''
s = s[:vstart] + validation + s[vend:]

s = s.replace('    NotInstalled(WorldPackRef),\n    InvalidStoredPath(WorldPackRef),', '    NotInstalled(WorldPackRef),\n    DisabledCannotActivate(WorldPackRef),\n    ActivePackRequiresReplacement(WorldPackRef),\n    InvalidActiveSelection(String),\n    InvalidStoredPath(WorldPackRef),', 1)

needle = '            Self::NotInstalled(pack) => {\n                write!(f, "Pack is not installed: {}@{}", pack.id, pack.version)\n            }\n'
addition = needle + '''            Self::DisabledCannotActivate(pack) => write!(
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
'''
if needle not in s:
    raise SystemExit('display marker not found')
s = s.replace(needle, addition, 1)

s = s.replace('        assert!(installed.enabled);\n        assert_eq!(installed.approval', '        assert!(installed.enabled);\n        assert!(installed.active);\n        assert_eq!(installed.approval', 1)

insert_before = '''    #[test]
    fn availability_distinguishes_disabled_missing_version_and_missing_pack() {'''
new_tests = '''    #[test]
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
        assert_eq!(registry.descriptor("fixture").unwrap().pack.version, "a-new");
        assert!(registry.descriptor_for(&old.pack).is_some());

        catalog.activate(&old.pack).unwrap();
        let source = catalog.trusted_source().unwrap();
        let mut registry = WorldRegistry::new();
        registry.install_source(&source).unwrap();
        assert_eq!(registry.descriptor("fixture").unwrap().pack.version, "z-old");
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

'''
if insert_before not in s:
    raise SystemExit('test insertion marker not found')
s = s.replace(insert_before, new_tests + insert_before, 1)

p.write_text(s)
