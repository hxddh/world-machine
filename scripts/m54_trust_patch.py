from pathlib import Path

process_path = Path('crates/world-pack-process/src/lib.rs')
s = process_path.read_text()

s = s.replace('use std::fs::{self, File};', 'use std::env;\nuse std::fs::{self, File, OpenOptions};', 1)
s = s.replace('use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};', 'use std::process::{self, Child, ChildStdin, ChildStdout, Command, Stdio};', 1)
s = s.replace('use std::sync::mpsc::{self, Receiver, RecvTimeoutError};', 'use std::sync::atomic::{AtomicU64, Ordering};\nuse std::sync::mpsc::{self, Receiver, RecvTimeoutError};', 1)
s = s.replace('pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);', 'pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);\n\nstatic LAUNCH_NONCE: AtomicU64 = AtomicU64::new(1);', 1)

needle = '''    pub fn verify_pin(&self) -> Result<(), HostError> {
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
replacement = needle + '''
    fn prepare_launch_program(&self) -> Result<(PathBuf, Option<PathBuf>), HostError> {
        let Some(expected) = self.pin.as_ref() else {
            return Ok((self.command.clone(), None));
        };
        if !self.args.is_empty() {
            return Err(HostError::session(format!(
                "pinned external Pack {}@{} cannot use runtime arguments; package the approved program as the direct command",
                self.descriptor.pack.id, self.descriptor.pack.version
            )));
        }

        let manifest_sha256 = sha256_file(&self.manifest_path)?;
        if manifest_sha256 != expected.manifest_sha256() {
            return Err(content_pin_mismatch(self, expected, &manifest_sha256, "not-read"));
        }

        let (bytes, command_sha256, permissions) = read_command_image(&self.command)?;
        if command_sha256 != expected.command_sha256() {
            return Err(content_pin_mismatch(
                self,
                expected,
                &manifest_sha256,
                &command_sha256,
            ));
        }
        let launch_path = write_launch_image(&self.command, &bytes, permissions)?;
        Ok((launch_path.clone(), Some(launch_path)))
    }
'''
if needle not in s:
    raise SystemExit('verify_pin marker not found')
s = s.replace(needle, replacement, 1)

sha_marker = 'fn sha256_file(path: &Path) -> Result<String, HostError> {'
helpers = '''fn content_pin_mismatch(
    pack: &ProcessPack,
    expected: &ProcessPackPin,
    manifest_sha256: &str,
    command_sha256: &str,
) -> HostError {
    HostError::session(format!(
        "external Pack content pin mismatch for {}@{}: expected manifest sha256 {} and executable sha256 {}, found manifest sha256 {} and executable sha256 {}",
        pack.descriptor.pack.id,
        pack.descriptor.pack.version,
        expected.manifest_sha256(),
        expected.command_sha256(),
        manifest_sha256,
        command_sha256,
    ))
}

fn read_command_image(path: &Path) -> Result<(Vec<u8>, String, fs::Permissions), HostError> {
    let mut file = File::open(path).map_err(|error| {
        HostError::pack_source(format!("could not open {} for approved launch: {error}", path.display()))
    })?;
    let permissions = file.metadata().map_err(|error| {
        HostError::pack_source(format!("could not stat {} for approved launch: {error}", path.display()))
    })?.permissions();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|error| {
        HostError::pack_source(format!("could not read {} for approved launch: {error}", path.display()))
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok((bytes, format!("{:x}", hasher.finalize()), permissions))
}

fn write_launch_image(
    source: &Path,
    bytes: &[u8],
    permissions: fs::Permissions,
) -> Result<PathBuf, HostError> {
    let nonce = LAUNCH_NONCE.fetch_add(1, Ordering::Relaxed);
    let extension = source
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{extension}"))
        .unwrap_or_default();
    let path = env::temp_dir().join(format!(
        "world-machine-pack-launch-{}-{nonce}{extension}",
        process::id()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|error| {
            HostError::session(format!(
                "could not create approved Pack launch image {}: {error}",
                path.display()
            ))
        })?;
    if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(&path);
        return Err(HostError::session(format!(
            "could not write approved Pack launch image {}: {error}",
            path.display()
        )));
    }
    drop(file);
    if let Err(error) = fs::set_permissions(&path, permissions) {
        let _ = fs::remove_file(&path);
        return Err(HostError::session(format!(
            "could not set approved Pack launch permissions {}: {error}",
            path.display()
        )));
    }
    Ok(path)
}

'''
if 'fn read_command_image' not in s:
    s = s.replace(sha_marker, helpers + sha_marker, 1)

s = s.replace('        pack.verify_pin()?;\n        let mut client = ProcessClient::spawn(&pack)?;', '        let mut client = ProcessClient::spawn(&pack)?;', 1)
s = s.replace('    request_timeout: Duration,\n}', '    request_timeout: Duration,\n    launch_cleanup: Option<PathBuf>,\n}', 1)

spawn_start = '''    fn spawn(pack: &ProcessPack) -> Result<Self, HostError> {
        let mut child = Command::new(&pack.command)
            .args(&pack.args)'''
spawn_new = '''    fn spawn(pack: &ProcessPack) -> Result<Self, HostError> {
        let (program, launch_cleanup) = pack.prepare_launch_program()?;
        let mut command = Command::new(&program);
        if pack.pin.is_none() {
            command.args(&pack.args);
        }
        let mut child = command'''
if spawn_start not in s:
    raise SystemExit('spawn marker not found')
s = s.replace(spawn_start, spawn_new, 1)
s = s.replace('                    pack.command.display()\n                ))\n            })?;', '                    program.display()\n                ))\n            });\n        let mut child = match child {\n            Ok(child) => child,\n            Err(error) => {\n                if let Some(path) = launch_cleanup.as_ref() {\n                    let _ = fs::remove_file(path);\n                }\n                return Err(error);\n            }\n        };', 1)
s = s.replace('            request_timeout: DEFAULT_REQUEST_TIMEOUT,\n        })', '            request_timeout: DEFAULT_REQUEST_TIMEOUT,\n            launch_cleanup,\n        })', 1)

term = '''    fn terminate(&mut self) {
        self.stdin.take();
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            _ => {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }
}'''
term_new = '''    fn terminate(&mut self) {
        self.stdin.take();
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            _ => {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
        self.cleanup_launch_image();
    }

    fn cleanup_launch_image(&mut self) {
        if let Some(path) = self.launch_cleanup.take() {
            let _ = fs::remove_file(path);
        }
    }
}'''
if term not in s:
    raise SystemExit('terminate marker not found')
s = s.replace(term, term_new, 1)
s = s.replace('            if matches!(self.child.try_wait(), Ok(Some(_))) {\n                return;\n            }', '            if matches!(self.child.try_wait(), Ok(Some(_))) {\n                self.cleanup_launch_image();\n                return;\n            }', 1)

process_path.write_text(s)

catalog_path = Path('crates/world-pack-catalog/src/lib.rs')
c = catalog_path.read_text()
c = c.replace('        let identity = pack.current_pin().map_err(process_error)?;', '        if !pack.args.is_empty() {\n            return Err(CatalogError::RuntimeArgumentsNotPinnable(pack.descriptor.pack));\n        }\n        let identity = pack.current_pin().map_err(process_error)?;', 1)
c = c.replace('    AlreadyInstalled(WorldPackRef),\n    NotInstalled(WorldPackRef),', '    AlreadyInstalled(WorldPackRef),\n    RuntimeArgumentsNotPinnable(WorldPackRef),\n    NotInstalled(WorldPackRef),', 1)
needle = '            Self::AlreadyInstalled(pack) => {\n                write!(f, "Pack is already installed: {}@{}", pack.id, pack.version)\n            }\n'
if needle not in c:
    needle = '            Self::AlreadyInstalled(pack) => write!(f, "Pack is already installed: {}@{}", pack.id, pack.version),\n'
addition = needle + '''            Self::RuntimeArgumentsNotPinnable(pack) => write!(
                f,
                "installed Pack {}@{} uses runtime arguments that are outside the v1 content pin; package the approved program as the direct command",
                pack.id, pack.version
            ),
'''
if needle not in c:
    raise SystemExit('catalog display marker not found')
c = c.replace(needle, addition, 1)

test_marker = '''    #[test]
    fn tampered_executable_is_rejected_before_source_assembly() {'''
new_test = '''    #[test]
    fn launcher_style_runtime_arguments_are_rejected_at_install() {
        let root = temp_dir("launcher-args");
        let script = root.join("runtime.sh");
        fs::write(&script, "exit 0\n").unwrap();
        let descriptor = PackDescriptor::new(pack("fixture", "1"), "fixture", "fixture");
        let manifest = PackManifest::process(
            descriptor,
            "/bin/sh",
            vec![script.to_string_lossy().into_owned()],
        );
        let manifest_path = root.join("fixture.world-pack.json");
        fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();
        let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();

        assert!(matches!(
            catalog.install_manifest(&manifest_path),
            Err(CatalogError::RuntimeArgumentsNotPinnable(found)) if found == pack("fixture", "1")
        ));
        assert!(catalog.entries().is_empty());
    }

'''
if test_marker not in c:
    raise SystemExit('catalog test marker not found')
c = c.replace(test_marker, new_test + test_marker, 1)
catalog_path.write_text(c)
