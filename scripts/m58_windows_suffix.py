from pathlib import Path

# Bundle format: allow only a fixed managed basename plus a safe single extension.
p = Path('crates/world-pack-bundle/src/lib.rs')
s = p.read_text()
s = s.replace(
'''        match &self.manifest.runtime {
            PackRuntimeManifest::Process { command, args }
                if command == PACK_BUNDLE_PROGRAM_NAME && args.is_empty() => {}
            _ => return Err(PackBundleError::NonPortableRuntime),
        }
''',
'''        match &self.manifest.runtime {
            PackRuntimeManifest::Process { command, args }
                if is_portable_program_name(command) && args.is_empty() => {}
            _ => return Err(PackBundleError::NonPortableRuntime),
        }
''',
1)
s = s.replace(
'''pub fn portable_process_manifest(descriptor: PackDescriptor) -> PackManifest {
    PackManifest::process(descriptor, PACK_BUNDLE_PROGRAM_NAME, Vec::new())
}
''',
'''pub fn portable_process_manifest(
    descriptor: PackDescriptor,
    program_name: impl Into<String>,
) -> Result<PackManifest, PackBundleError> {
    let program_name = program_name.into();
    if !is_portable_program_name(&program_name) {
        return Err(PackBundleError::InvalidProgramName(program_name));
    }
    Ok(PackManifest::process(descriptor, program_name, Vec::new()))
}

fn program_name_for_path(path: &Path) -> Result<String, PackBundleError> {
    match path.extension() {
        None => Ok(PACK_BUNDLE_PROGRAM_NAME.into()),
        Some(extension) => {
            let extension = extension
                .to_str()
                .ok_or_else(|| PackBundleError::InvalidProgramExtension(path.to_path_buf()))?;
            let name = format!("{PACK_BUNDLE_PROGRAM_NAME}.{extension}");
            if is_portable_program_name(&name) {
                Ok(name)
            } else {
                Err(PackBundleError::InvalidProgramExtension(path.to_path_buf()))
            }
        }
    }
}

fn is_portable_program_name(name: &str) -> bool {
    if name == PACK_BUNDLE_PROGRAM_NAME {
        return true;
    }
    let Some(extension) = name.strip_prefix("program.") else {
        return false;
    };
    !extension.is_empty()
        && extension.len() <= 32
        && extension
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
''',
1)
# Add accessor before extract_program.
s = s.replace(
'''    pub fn manifest(&self) -> &PackManifest {
        &self.header.manifest
    }

    pub fn extract_program(mut self, destination: impl AsRef<Path>) -> Result<(), PackBundleError> {
''',
'''    pub fn manifest(&self) -> &PackManifest {
        &self.header.manifest
    }

    pub fn program_name(&self) -> &str {
        match &self.header.manifest.runtime {
            PackRuntimeManifest::Process { command, .. } => command,
        }
    }

    pub fn extract_program(mut self, destination: impl AsRef<Path>) -> Result<(), PackBundleError> {
''',
1)
# Writer derives managed name from source suffix.
s = s.replace(
'''    write_bundle(
        destination,
        portable_process_manifest(descriptor),
        program_path,
    )
''',
'''    let program_path = program_path.as_ref();
    let program_name = program_name_for_path(program_path)?;
    write_bundle(
        destination,
        portable_process_manifest(descriptor, program_name)?,
        program_path,
    )
''',
1)
s = s.replace(
'''    match &manifest.runtime {
        PackRuntimeManifest::Process { command, args }
            if command == PACK_BUNDLE_PROGRAM_NAME && args.is_empty() => {}
        _ => return Err(PackBundleError::NonPortableRuntime),
    }
''',
'''    match &manifest.runtime {
        PackRuntimeManifest::Process { command, args }
            if is_portable_program_name(command) && args.is_empty() => {}
        _ => return Err(PackBundleError::NonPortableRuntime),
    }
''',
1)
# Errors.
s = s.replace(
'''    InvalidProgramDigest,
    InvalidLayout,
''',
'''    InvalidProgramDigest,
    InvalidProgramName(String),
    InvalidProgramExtension(PathBuf),
    InvalidLayout,
''',
1)
s = s.replace(
'''            Self::InvalidProgramDigest => write!(f, "invalid Pack bundle program digest"),
            Self::InvalidLayout => {
''',
'''            Self::InvalidProgramDigest => write!(f, "invalid Pack bundle program digest"),
            Self::InvalidProgramName(name) => {
                write!(f, "invalid portable Pack program name: {name}")
            }
            Self::InvalidProgramExtension(path) => write!(
                f,
                "Pack program extension cannot be represented safely in bundle v1: {}",
                path.display()
            ),
            Self::InvalidLayout => {
''',
1)
# Update existing round-trip expectation and add extension/security regression.
s = s.replace(
'''                assert_eq!(command, PACK_BUNDLE_PROGRAM_NAME);
                assert!(args.is_empty());
''',
'''                assert_eq!(command, PACK_BUNDLE_PROGRAM_NAME);
                assert!(args.is_empty());
''',
1)
insert_before = '''    #[test]
    fn bundle_rejects_trailing_data() {
'''
extra = '''    #[test]
    fn bundle_preserves_a_safe_executable_suffix_without_allowing_paths() {
        let root = temp_dir("suffix");
        let program = root.join("fixture.exe");
        fs::write(&program, b"windows-program").unwrap();
        let bundle_path = root.join(format!("fixture{PACK_BUNDLE_SUFFIX}"));
        write_program_bundle(&bundle_path, descriptor(), &program).unwrap();
        let bundle = PackBundle::open(&bundle_path).unwrap();
        assert_eq!(bundle.program_name(), "program.exe");

        assert!(matches!(
            portable_process_manifest(descriptor(), "../program.exe").unwrap_err(),
            PackBundleError::InvalidProgramName(_)
        ));
        assert!(matches!(
            portable_process_manifest(descriptor(), r"dir\\program.exe").unwrap_err(),
            PackBundleError::InvalidProgramName(_)
        ));
    }

'''
if insert_before not in s:
    raise SystemExit('bundle test insertion marker not found')
s = s.replace(insert_before, extra + insert_before, 1)
p.write_text(s)

# Catalog extracts and writes the exact validated managed basename from the bundle.
p = Path('crates/world-pack-catalog/src/lib.rs')
s = p.read_text()
s = s.replace(
'use world_pack_bundle::{PackBundle, PACK_BUNDLE_PROGRAM_NAME};\n',
'use world_pack_bundle::PackBundle;\n',
1)
s = s.replace(
'''        let descriptor = bundle.manifest().descriptor.clone();
        let final_dir = managed_pack_dir(&self.path, &descriptor.pack);
''',
'''        let descriptor = bundle.manifest().descriptor.clone();
        let program_name = bundle.program_name().to_owned();
        let final_dir = managed_pack_dir(&self.path, &descriptor.pack);
''',
1)
s = s.replace(
'''            let staged_program = stage.join(PACK_BUNDLE_PROGRAM_NAME);
            bundle
                .extract_program(&staged_program)
                .map_err(bundle_error)?;

            let managed_manifest =
                PackManifest::process(descriptor.clone(), PACK_BUNDLE_PROGRAM_NAME, Vec::new());
''',
'''            let staged_program = stage.join(&program_name);
            bundle
                .extract_program(&staged_program)
                .map_err(bundle_error)?;

            let managed_manifest =
                PackManifest::process(descriptor.clone(), program_name.clone(), Vec::new());
''',
1)
# Add catalog test proving .exe survives managed installation.
marker = '''    #[test]
    fn portable_bundle_install_owns_program_after_source_is_removed() {
'''
extra = '''    #[test]
    fn portable_bundle_preserves_executable_suffix_in_managed_manifest() {
        use world_pack_bundle::write_program_bundle;

        let root = temp_dir("bundle-suffix");
        let program = root.join("bundle-runtime.exe");
        fs::write(&program, b"portable-runtime").unwrap();
        let descriptor = PackDescriptor::new(pack("fixture.bundle.exe", "v1"), "Bundle", "fixture");
        let bundle_path = root.join("fixture.worldpack");
        write_program_bundle(&bundle_path, descriptor, &program).unwrap();

        let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
        let installed = catalog.install_bundle(&bundle_path).unwrap();
        assert_eq!(
            installed.command_path.file_name().and_then(|name| name.to_str()),
            Some("program.exe")
        );
        let managed = ProcessPack::load(&installed.manifest_path).unwrap();
        assert_eq!(
            managed.command.file_name().and_then(|name| name.to_str()),
            Some("program.exe")
        );
    }

'''
if marker not in s:
    raise SystemExit('catalog suffix test marker not found')
s = s.replace(marker, extra + marker, 1)
p.write_text(s)
