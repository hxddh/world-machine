from pathlib import Path

p = Path('crates/world-pack-process/src/lib.rs')
s = p.read_text()
s = s.replace('use std::fs;', 'use sha2::{Digest, Sha256};\nuse std::fs::{self, File};', 1)
s = s.replace('use std::io::{self, BufRead, BufReader, BufWriter, Write};', 'use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};', 1)

marker = '#[derive(Clone, Debug, Eq, PartialEq)]\npub struct ProcessPack {'
if 'pub struct ProcessPackPin' not in s:
    s = s.replace(marker, '''#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessPackPin {
    manifest_sha256: String,
    command_sha256: String,
}

impl ProcessPackPin {
    pub fn new(manifest_sha256: impl Into<String>, command_sha256: impl Into<String>) -> Self {
        Self { manifest_sha256: manifest_sha256.into(), command_sha256: command_sha256.into() }
    }
    pub fn manifest_sha256(&self) -> &str { &self.manifest_sha256 }
    pub fn command_sha256(&self) -> &str { &self.command_sha256 }
}

''' + marker, 1)

s = s.replace('    pub args: Vec<String>,\n}', '    pub args: Vec<String>,\n    pin: Option<ProcessPackPin>,\n}', 1)
s = s.replace('            command,\n            args,\n        })', '            command,\n            args,\n            pin: None,\n        })', 1)

needle = '    fn registration(&self) -> WorldRegistration {'
if 'pub fn current_pin' not in s:
    methods = '''    pub fn current_pin(&self) -> Result<ProcessPackPin, HostError> {
        Ok(ProcessPackPin::new(sha256_file(&self.manifest_path)?, sha256_file(&self.command)?))
    }

    pub fn with_pin(mut self, pin: ProcessPackPin) -> Self {
        self.pin = Some(pin);
        self
    }

    pub fn pin(&self) -> Option<&ProcessPackPin> { self.pin.as_ref() }

    pub fn verify_pin(&self) -> Result<(), HostError> {
        let Some(expected) = self.pin.as_ref() else { return Ok(()); };
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
    s = s.replace(needle, methods + needle, 1)

source = 'impl ProcessPackSource {\n    pub fn from_manifest_paths('
if 'pub fn from_packs' not in s:
    s = s.replace(source, 'impl ProcessPackSource {\n    pub fn from_packs(packs: Vec<ProcessPack>) -> Self { Self { packs } }\n\n    pub fn from_manifest_paths(', 1)

resolve = 'fn resolve_command(manifest_path: &Path, command: &str) -> Result<PathBuf, HostError> {'
if 'fn sha256_file' not in s:
    helper = '''fn sha256_file(path: &Path) -> Result<String, HostError> {
    let mut file = File::open(path).map_err(|error| HostError::pack_source(format!("could not open {} for sha256: {error}", path.display())))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| HostError::pack_source(format!("could not read {} for sha256: {error}", path.display())))?;
        if read == 0 { break; }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

'''
    s = s.replace(resolve, helper + resolve, 1)

start = '    fn start(pack: ProcessPack, archive: Option<WorldArchive>) -> Result<Self, HostError> {\n        let mut client = ProcessClient::spawn(&pack)?;'
s = s.replace(start, '    fn start(pack: ProcessPack, archive: Option<WorldArchive>) -> Result<Self, HostError> {\n        pack.verify_pin()?;\n        let mut client = ProcessClient::spawn(&pack)?;', 1)
p.write_text(s)
