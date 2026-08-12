from pathlib import Path

p = Path('crates/world-pack-catalog/src/lib.rs')
s = p.read_text()

# Managed developer executables use a catalog-owned execution mode instead of
# inheriting mutable source permissions. Bundle extraction already uses 0700.
old_permissions = '''            let permissions = fs::metadata(&source.command)
                .map_err(|error| CatalogError::Io {
                    operation: "read approved Pack executable permissions",
                    path: source.command.clone(),
                    message: error.to_string(),
                })?
                .permissions();
            fs::set_permissions(&staged_program, permissions).map_err(|error| {
                CatalogError::Io {
                    operation: "set managed Pack executable permissions",
                    path: staged_program.clone(),
                    message: error.to_string(),
                }
            })?;
'''
new_permissions = '''            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&staged_program, fs::Permissions::from_mode(0o700)).map_err(
                    |error| CatalogError::Io {
                        operation: "set managed Pack executable permissions",
                        path: staged_program.clone(),
                        message: error.to_string(),
                    },
                )?;
            }
'''
if old_permissions not in s:
    raise SystemExit('managed permission block not found')
s = s.replace(old_permissions, new_permissions, 1)

old_verify = '''                let managed = self.materialize_managed_pack(&source)?;
                let managed_pin = managed.current_pin().map_err(process_error)?;
                if managed_pin.command_sha256() != pin.command_sha256() {
                    cleanup_managed_pack_identity(&self.path, &preview.pack);
                    return Err(reviewed_content_changed(
                        &preview.pack,
                        "executable changed while it was copied into the managed store",
                    ));
                }
                self.record_managed_install(managed)
'''
new_verify = '''                let managed = self.materialize_managed_pack(&source)?;
                self.verify_reviewed_managed_program(&managed, pin.command_sha256())?;
                self.record_managed_install(managed)
'''
if old_verify not in s:
    raise SystemExit('post-copy verify block not found')
s = s.replace(old_verify, new_verify, 1)

record_marker = '''    fn record_managed_install(
        &mut self,
        managed: ProcessPack,
    ) -> Result<InstalledPack, CatalogError> {
'''
verify_method = '''    fn verify_reviewed_managed_program(
        &self,
        managed: &ProcessPack,
        expected_sha256: &str,
    ) -> Result<(), CatalogError> {
        let pack = managed.descriptor.pack.clone();
        let result = match managed.current_pin() {
            Ok(pin) if pin.command_sha256() == expected_sha256 => Ok(()),
            Ok(_) => Err(reviewed_content_changed(
                &pack,
                "executable changed while it was copied into the managed store",
            )),
            Err(error) => Err(process_error(error)),
        };
        if result.is_err() {
            cleanup_managed_pack_identity(&self.path, &pack);
        }
        result
    }

'''
if record_marker not in s:
    raise SystemExit('record install marker not found')
s = s.replace(record_marker, verify_method + record_marker, 1)

# Add behavior regressions before the existing reviewed-manifest replacement test.
test_marker = '''    #[test]
    fn reviewed_manifest_refuses_changed_executable_without_installing() {
'''
extra_tests = '''    #[cfg(unix)]
    #[test]
    fn reviewed_manifest_normalizes_managed_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_dir("review-permissions");
        let manifest_path = write_pack(&root, "fixture.review-mode", "v1");
        let runtime = root.join("fixture.review-mode-v1-runtime.sh");
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();

        let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
        let preview = catalog.inspect_install(&manifest_path).unwrap();
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o600)).unwrap();

        let installed = catalog.install_reviewed(&preview).unwrap();
        let mode = fs::metadata(&installed.command_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);
    }

    #[test]
    fn failed_post_copy_pin_verification_removes_managed_destination() {
        let root = temp_dir("review-post-copy-cleanup");
        let manifest_path = write_pack(&root, "fixture.review-cleanup", "v1");
        let source = ProcessPack::load(&manifest_path).unwrap();
        let catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
        let managed = catalog.materialize_managed_pack(&source).unwrap();
        let pack = managed.descriptor.pack.clone();
        fs::remove_file(&managed.command).unwrap();

        let error = catalog
            .verify_reviewed_managed_program(&managed, "not-used-after-read-failure")
            .unwrap_err();
        assert!(matches!(error, CatalogError::Process(_)));
        assert!(!managed_pack_dir(catalog.path(), &pack).exists());
    }

'''
if test_marker not in s:
    raise SystemExit('review test insertion marker not found')
s = s.replace(test_marker, extra_tests + test_marker, 1)
p.write_text(s)
