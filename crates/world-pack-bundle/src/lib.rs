use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use world_pack_protocol::{PackDescriptor, PackManifest, PackRuntimeManifest};

pub const PACK_BUNDLE_FORMAT: &str = "world-machine-pack-bundle";
pub const PACK_BUNDLE_VERSION: u32 = 1;
pub const PACK_BUNDLE_SUFFIX: &str = ".worldpack";
pub const PACK_BUNDLE_PROGRAM_NAME: &str = "program";
pub const MAX_BUNDLE_HEADER_BYTES: u64 = 1024 * 1024;
pub const MAX_BUNDLE_PROGRAM_BYTES: u64 = 512 * 1024 * 1024;

const PACK_BUNDLE_MAGIC: [u8; 8] = *b"WMPACK01";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackBundleHeader {
    pub format: String,
    pub format_version: u32,
    pub manifest: PackManifest,
    pub program_sha256: String,
    pub program_bytes: u64,
}

impl PackBundleHeader {
    fn validate(&self) -> Result<(), PackBundleError> {
        if self.format != PACK_BUNDLE_FORMAT {
            return Err(PackBundleError::UnsupportedFormat(self.format.clone()));
        }
        if self.format_version != PACK_BUNDLE_VERSION {
            return Err(PackBundleError::UnsupportedVersion(self.format_version));
        }
        self.manifest
            .validate()
            .map_err(|error| PackBundleError::Manifest(error.to_string()))?;
        match &self.manifest.runtime {
            PackRuntimeManifest::Process { command, args }
                if command == PACK_BUNDLE_PROGRAM_NAME && args.is_empty() => {}
            _ => return Err(PackBundleError::NonPortableRuntime),
        }
        if self.program_bytes == 0 || self.program_bytes > MAX_BUNDLE_PROGRAM_BYTES {
            return Err(PackBundleError::InvalidProgramSize(self.program_bytes));
        }
        if self.program_sha256.len() != 64
            || !self
                .program_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(PackBundleError::InvalidProgramDigest);
        }
        Ok(())
    }
}

pub fn portable_process_manifest(descriptor: PackDescriptor) -> PackManifest {
    PackManifest::process(descriptor, PACK_BUNDLE_PROGRAM_NAME, Vec::new())
}

pub struct PackBundle {
    path: PathBuf,
    file: File,
    header: PackBundleHeader,
    program_offset: u64,
}

impl PackBundle {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PackBundleError> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path).map_err(|error| io_error("open bundle", &path, error))?;

        let mut magic = [0_u8; PACK_BUNDLE_MAGIC.len()];
        file.read_exact(&mut magic)
            .map_err(|error| io_error("read bundle magic", &path, error))?;
        if magic != PACK_BUNDLE_MAGIC {
            return Err(PackBundleError::InvalidMagic);
        }

        let mut header_len_bytes = [0_u8; 4];
        file.read_exact(&mut header_len_bytes)
            .map_err(|error| io_error("read bundle header length", &path, error))?;
        let header_len = u32::from_le_bytes(header_len_bytes) as u64;
        if header_len == 0 || header_len > MAX_BUNDLE_HEADER_BYTES {
            return Err(PackBundleError::InvalidHeaderSize(header_len));
        }
        let header_len_usize = usize::try_from(header_len)
            .map_err(|_| PackBundleError::InvalidHeaderSize(header_len))?;
        let mut header_json = vec![0_u8; header_len_usize];
        file.read_exact(&mut header_json)
            .map_err(|error| io_error("read bundle header", &path, error))?;
        let header = serde_json::from_slice::<PackBundleHeader>(&header_json)
            .map_err(|error| PackBundleError::Json(error.to_string()))?;
        header.validate()?;

        let program_offset = (PACK_BUNDLE_MAGIC.len() as u64)
            .checked_add(4)
            .and_then(|value| value.checked_add(header_len))
            .ok_or(PackBundleError::InvalidLayout)?;
        let expected_len = program_offset
            .checked_add(header.program_bytes)
            .ok_or(PackBundleError::InvalidLayout)?;
        let actual_len = file
            .metadata()
            .map_err(|error| io_error("inspect bundle", &path, error))?
            .len();
        if actual_len != expected_len {
            return Err(PackBundleError::InvalidLayout);
        }

        Ok(Self {
            path,
            file,
            header,
            program_offset,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn header(&self) -> &PackBundleHeader {
        &self.header
    }

    pub fn manifest(&self) -> &PackManifest {
        &self.header.manifest
    }

    pub fn extract_program(mut self, destination: impl AsRef<Path>) -> Result<(), PackBundleError> {
        let destination = destination.as_ref().to_path_buf();
        let result = (|| {
            self.file
                .seek(SeekFrom::Start(self.program_offset))
                .map_err(|error| io_error("seek bundle program", &self.path, error))?;
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&destination)
                .map_err(|error| io_error("create extracted Pack program", &destination, error))?;
            let mut remaining = self.header.program_bytes;
            let mut hasher = Sha256::new();
            let mut buffer = [0_u8; 64 * 1024];
            while remaining > 0 {
                let requested = usize::try_from(remaining.min(buffer.len() as u64))
                    .expect("bounded read size fits usize");
                let read = self
                    .file
                    .read(&mut buffer[..requested])
                    .map_err(|error| io_error("read bundle program", &self.path, error))?;
                if read == 0 {
                    return Err(PackBundleError::InvalidLayout);
                }
                output.write_all(&buffer[..read]).map_err(|error| {
                    io_error("write extracted Pack program", &destination, error)
                })?;
                hasher.update(&buffer[..read]);
                remaining -= read as u64;
            }
            output
                .sync_all()
                .map_err(|error| io_error("sync extracted Pack program", &destination, error))?;
            let found = format!("{:x}", hasher.finalize());
            if !found.eq_ignore_ascii_case(&self.header.program_sha256) {
                return Err(PackBundleError::ProgramDigestMismatch {
                    expected: self.header.program_sha256.clone(),
                    found,
                });
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&destination, fs::Permissions::from_mode(0o700)).map_err(
                    |error| {
                        io_error(
                            "mark extracted Pack program executable",
                            &destination,
                            error,
                        )
                    },
                )?;
            }
            Ok(())
        })();

        if result.is_err() {
            let _ = fs::remove_file(&destination);
        }
        result
    }
}

pub fn write_program_bundle(
    destination: impl AsRef<Path>,
    descriptor: PackDescriptor,
    program_path: impl AsRef<Path>,
) -> Result<PackBundleHeader, PackBundleError> {
    write_bundle(
        destination,
        portable_process_manifest(descriptor),
        program_path,
    )
}

pub fn write_bundle(
    destination: impl AsRef<Path>,
    manifest: PackManifest,
    program_path: impl AsRef<Path>,
) -> Result<PackBundleHeader, PackBundleError> {
    let destination = destination.as_ref().to_path_buf();
    let program_path = program_path.as_ref().to_path_buf();
    manifest
        .validate()
        .map_err(|error| PackBundleError::Manifest(error.to_string()))?;
    match &manifest.runtime {
        PackRuntimeManifest::Process { command, args }
            if command == PACK_BUNDLE_PROGRAM_NAME && args.is_empty() => {}
        _ => return Err(PackBundleError::NonPortableRuntime),
    }

    let metadata = fs::metadata(&program_path)
        .map_err(|error| io_error("inspect Pack program", &program_path, error))?;
    if !metadata.is_file() {
        return Err(PackBundleError::ProgramNotFile(program_path));
    }
    let (program_sha256, program_bytes) = hash_program(&program_path)?;
    let header = PackBundleHeader {
        format: PACK_BUNDLE_FORMAT.into(),
        format_version: PACK_BUNDLE_VERSION,
        manifest,
        program_sha256,
        program_bytes,
    };
    header.validate()?;
    let header_json =
        serde_json::to_vec(&header).map_err(|error| PackBundleError::Json(error.to_string()))?;
    if header_json.is_empty() || header_json.len() as u64 > MAX_BUNDLE_HEADER_BYTES {
        return Err(PackBundleError::InvalidHeaderSize(header_json.len() as u64));
    }
    let header_len = u32::try_from(header_json.len())
        .map_err(|_| PackBundleError::InvalidHeaderSize(header_json.len() as u64))?;

    let result = (|| {
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&destination)
            .map_err(|error| io_error("create Pack bundle", &destination, error))?;
        output
            .write_all(&PACK_BUNDLE_MAGIC)
            .and_then(|_| output.write_all(&header_len.to_le_bytes()))
            .and_then(|_| output.write_all(&header_json))
            .map_err(|error| io_error("write Pack bundle header", &destination, error))?;

        let mut program = File::open(&program_path)
            .map_err(|error| io_error("open Pack program", &program_path, error))?;
        let mut copied = 0_u64;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = program
                .read(&mut buffer)
                .map_err(|error| io_error("read Pack program", &program_path, error))?;
            if read == 0 {
                break;
            }
            copied = copied
                .checked_add(read as u64)
                .ok_or(PackBundleError::InvalidProgramSize(u64::MAX))?;
            if copied > MAX_BUNDLE_PROGRAM_BYTES {
                return Err(PackBundleError::InvalidProgramSize(copied));
            }
            output
                .write_all(&buffer[..read])
                .map_err(|error| io_error("write Pack bundle program", &destination, error))?;
            hasher.update(&buffer[..read]);
        }
        let copied_sha256 = format!("{:x}", hasher.finalize());
        if copied != header.program_bytes
            || !copied_sha256.eq_ignore_ascii_case(&header.program_sha256)
        {
            return Err(PackBundleError::ProgramChangedDuringBundleCreation);
        }
        output
            .sync_all()
            .map_err(|error| io_error("sync Pack bundle", &destination, error))?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&destination);
        return result.map(|_| header);
    }
    Ok(header)
}

fn hash_program(path: &Path) -> Result<(String, u64), PackBundleError> {
    let mut file = File::open(path).map_err(|error| io_error("open Pack program", path, error))?;
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| io_error("read Pack program", path, error))?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or(PackBundleError::InvalidProgramSize(u64::MAX))?;
        if total > MAX_BUNDLE_PROGRAM_BYTES {
            return Err(PackBundleError::InvalidProgramSize(total));
        }
        hasher.update(&buffer[..read]);
    }
    if total == 0 {
        return Err(PackBundleError::InvalidProgramSize(0));
    }
    Ok((format!("{:x}", hasher.finalize()), total))
}

fn io_error(operation: &'static str, path: &Path, error: std::io::Error) -> PackBundleError {
    PackBundleError::Io {
        operation,
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackBundleError {
    Io {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
    Json(String),
    Manifest(String),
    UnsupportedFormat(String),
    UnsupportedVersion(u32),
    InvalidMagic,
    InvalidHeaderSize(u64),
    InvalidProgramSize(u64),
    InvalidProgramDigest,
    InvalidLayout,
    NonPortableRuntime,
    ProgramNotFile(PathBuf),
    ProgramDigestMismatch {
        expected: String,
        found: String,
    },
    ProgramChangedDuringBundleCreation,
}

impl fmt::Display for PackBundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                message,
            } => write!(f, "could not {operation} {}: {message}", path.display()),
            Self::Json(error) => write!(f, "could not decode Pack bundle header: {error}"),
            Self::Manifest(error) => write!(f, "invalid Pack bundle manifest: {error}"),
            Self::UnsupportedFormat(format) => {
                write!(f, "unsupported Pack bundle format: {format}")
            }
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported Pack bundle version: {version}")
            }
            Self::InvalidMagic => write!(f, "file is not a World Machine Pack bundle"),
            Self::InvalidHeaderSize(bytes) => write!(f, "invalid Pack bundle header size: {bytes}"),
            Self::InvalidProgramSize(bytes) => {
                write!(f, "invalid Pack bundle program size: {bytes}")
            }
            Self::InvalidProgramDigest => write!(f, "invalid Pack bundle program digest"),
            Self::InvalidLayout => {
                write!(f, "Pack bundle layout is truncated or has trailing data")
            }
            Self::NonPortableRuntime => write!(
                f,
                "Pack bundle v1 must contain exactly one direct program with no runtime arguments"
            ),
            Self::ProgramNotFile(path) => {
                write!(
                    f,
                    "Pack bundle program is not a regular file: {}",
                    path.display()
                )
            }
            Self::ProgramDigestMismatch { expected, found } => write!(
                f,
                "Pack bundle program digest mismatch: expected sha256 {expected}, found {found}"
            ),
            Self::ProgramChangedDuringBundleCreation => {
                write!(f, "Pack program changed while the bundle was being created")
            }
        }
    }
}

impl Error for PackBundleError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_pack_protocol::PackDescriptor;
    use world_persistence::WorldPackRef;

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "world-machine-pack-bundle-{label}-{}-{nonce}",
            process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn descriptor() -> PackDescriptor {
        PackDescriptor::new(
            WorldPackRef::new("fixture.bundle", "opaque-v1"),
            "Fixture Bundle",
            "portable fixture",
        )
    }

    #[test]
    fn bundle_round_trips_exact_program_and_manifest() {
        let root = temp_dir("round-trip");
        let program = root.join("fixture-program");
        fs::write(&program, b"portable-program-bytes").unwrap();
        let bundle_path = root.join(format!("fixture{PACK_BUNDLE_SUFFIX}"));
        let header = write_program_bundle(&bundle_path, descriptor(), &program).unwrap();
        assert_eq!(header.manifest.descriptor.pack.id, "fixture.bundle");

        let bundle = PackBundle::open(&bundle_path).unwrap();
        assert_eq!(bundle.manifest().descriptor.pack.version, "opaque-v1");
        match &bundle.manifest().runtime {
            PackRuntimeManifest::Process { command, args } => {
                assert_eq!(command, PACK_BUNDLE_PROGRAM_NAME);
                assert!(args.is_empty());
            }
        }
        let extracted = root.join("extracted-program");
        bundle.extract_program(&extracted).unwrap();
        assert_eq!(fs::read(extracted).unwrap(), b"portable-program-bytes");
    }

    #[test]
    fn bundle_rejects_trailing_data() {
        let root = temp_dir("trailing");
        let program = root.join("fixture-program");
        fs::write(&program, b"program").unwrap();
        let bundle_path = root.join(format!("fixture{PACK_BUNDLE_SUFFIX}"));
        write_program_bundle(&bundle_path, descriptor(), &program).unwrap();
        OpenOptions::new()
            .append(true)
            .open(&bundle_path)
            .unwrap()
            .write_all(b"unexpected")
            .unwrap();
        assert!(matches!(
            PackBundle::open(bundle_path).err().unwrap(),
            PackBundleError::InvalidLayout
        ));
    }

    #[test]
    fn bundle_detects_program_tampering_before_executable_is_kept() {
        let root = temp_dir("tamper");
        let program = root.join("fixture-program");
        fs::write(&program, b"program").unwrap();
        let bundle_path = root.join(format!("fixture{PACK_BUNDLE_SUFFIX}"));
        write_program_bundle(&bundle_path, descriptor(), &program).unwrap();

        let bundle = PackBundle::open(&bundle_path).unwrap();
        let offset = bundle.program_offset;
        drop(bundle);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&bundle_path)
            .unwrap();
        file.seek(SeekFrom::Start(offset)).unwrap();
        file.write_all(b"X").unwrap();
        file.sync_all().unwrap();

        let bundle = PackBundle::open(&bundle_path).unwrap();
        let extracted = root.join("tampered-program");
        let error = bundle.extract_program(&extracted).unwrap_err();
        assert!(matches!(
            error,
            PackBundleError::ProgramDigestMismatch { .. }
        ));
        assert!(!extracted.exists());
    }

    #[test]
    fn bundle_writer_rejects_empty_programs() {
        let root = temp_dir("empty");
        let program = root.join("fixture-program");
        fs::write(&program, b"").unwrap();
        let bundle_path = root.join(format!("fixture{PACK_BUNDLE_SUFFIX}"));
        assert!(matches!(
            write_program_bundle(bundle_path, descriptor(), program).unwrap_err(),
            PackBundleError::InvalidProgramSize(0)
        ));
    }
}
