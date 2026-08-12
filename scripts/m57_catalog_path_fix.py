from pathlib import Path

p = Path('crates/world-pack-catalog/src/lib.rs')
s = p.read_text()
old = '''fn absolute_path(path: &Path) -> Result<PathBuf, CatalogError> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    std::env::current_dir()
        .map(|current| current.join(path))
        .map_err(|error| CatalogError::Io {
            operation: "resolve catalog path",
            path: path.to_path_buf(),
            message: error.to_string(),
        })
}
'''
new = '''fn absolute_path(path: &Path) -> Result<PathBuf, CatalogError> {
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
'''
if old not in s:
    raise SystemExit('absolute_path marker not found')
s = s.replace(old, new, 1)

marker = '''    #[test]
    fn explicit_install_persists_exact_identity_and_reopens() {'''
test = '''    #[cfg(unix)]
    #[test]
    fn catalog_path_canonicalizes_existing_symlink_ancestor_before_managed_storage() {
        use std::os::unix::fs::symlink;

        let root = temp_dir("symlink-root");
        let alias = root
            .parent()
            .unwrap()
            .join(format!("{}-alias", root.file_name().unwrap().to_string_lossy()));
        symlink(&root, &alias).unwrap();
        let catalog = PackCatalog::open(alias.join("Packs").join("catalog.json")).unwrap();

        assert_eq!(
            catalog.path(),
            root.canonicalize().unwrap().join("Packs").join("catalog.json")
        );
        fs::remove_file(alias).unwrap();
    }

'''
if marker not in s:
    raise SystemExit('test marker not found')
s = s.replace(marker, test + marker, 1)
p.write_text(s)
