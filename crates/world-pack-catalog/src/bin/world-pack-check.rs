use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use world_pack_catalog::{PackCatalog, PackInstallPreview};

static SCRATCH_NONCE: AtomicU64 = AtomicU64::new(1);

const USAGE: &str = "Usage: world-pack-check [--inspect-only] <pack.worldpack|pack.world-pack.json>\n\n\
Checks a World Machine Pack without mutating the source.\n\
\n\
By default the Pack is copied into an isolated temporary catalog and executed only for\n\
the durable activation probe: Create -> Archive -> fresh-process Open.\n\
Use --inspect-only to validate metadata/content identity without executing Pack code.";

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Check {
        source: PathBuf,
        inspect_only: bool,
    },
    Help,
}

fn main() {
    match run(env::args_os().skip(1)) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("world-pack-check: {error}");
            process::exit(1);
        }
    }
}

fn run<I>(args: I) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = OsString>,
{
    match parse_args(args)? {
        Command::Help => {
            println!("{USAGE}");
            Ok(())
        }
        Command::Check {
            source,
            inspect_only,
        } => check_pack(&source, inspect_only),
    }
}

fn parse_args<I>(args: I) -> Result<Command, io::Error>
where
    I: IntoIterator<Item = OsString>,
{
    let mut inspect_only = false;
    let mut source = None;

    for arg in args {
        if arg == "-h" || arg == "--help" {
            return Ok(Command::Help);
        }
        if arg == "--inspect-only" {
            inspect_only = true;
            continue;
        }
        if arg.to_string_lossy().starts_with('-') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown option {}\n\n{USAGE}", arg.to_string_lossy()),
            ));
        }
        if source.replace(PathBuf::from(arg)).is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("expected exactly one Pack path\n\n{USAGE}"),
            ));
        }
    }

    let source = source.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("missing Pack path\n\n{USAGE}"),
        )
    })?;

    Ok(Command::Check {
        source,
        inspect_only,
    })
}

fn check_pack(source: &Path, inspect_only: bool) -> Result<(), Box<dyn Error>> {
    let scratch = ScratchDir::new()?;
    let mut catalog = PackCatalog::open(scratch.catalog_path())?;
    let preview = catalog.inspect_install(source)?;

    print_preview(&preview);
    println!("Inspection passed. No Pack code has run.");

    if inspect_only {
        println!("PASS: static Pack inspection only.");
        return Ok(());
    }

    println!("Installing reviewed bytes into an isolated temporary catalog…");
    let installed = catalog.install_reviewed_pending_probe(&preview)?;
    if installed.pack != *preview.pack() {
        return Err(io::Error::other(format!(
            "isolated install changed Pack identity from {} @ {} to {} @ {}",
            preview.pack().id,
            preview.pack().version,
            installed.pack.id,
            installed.pack.version
        ))
        .into());
    }

    println!("Running durable activation probe from the isolated managed copy…");
    let probe = catalog.probe(preview.pack())?;
    if probe.pack != *preview.pack() {
        return Err(io::Error::other(format!(
            "probe reported unexpected Pack identity {} @ {}",
            probe.pack.id, probe.pack.version
        ))
        .into());
    }

    println!("PASS: durable Pack contract verified.");
    println!(
        "Created: {} · World time {}",
        probe.created_title, probe.created_world_time
    );
    println!(
        "Reopened: {} · World time {}",
        probe.reopened_title, probe.reopened_world_time
    );
    Ok(())
}

fn print_preview(preview: &PackInstallPreview) {
    println!("Pack: {} @ {}", preview.pack().id, preview.pack().version);
    println!("Title: {}", preview.title());
    println!("Format: {}", preview.kind().label());
    println!("Will execute: {}", preview.runtime_name());
    println!("Executable bytes: {}", preview.program_bytes());
    println!("SHA-256: {}", preview.program_sha256());
    println!("Source: {}", preview.source_path().display());
}

struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new() -> io::Result<Self> {
        let root = env::temp_dir();
        let process_id = process::id();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        for _ in 0..32 {
            let nonce = SCRATCH_NONCE.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!(
                "world-machine-pack-check-{process_id}-{timestamp}-{nonce}"
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a unique temporary Pack check directory",
        ))
    }

    fn catalog_path(&self) -> PathBuf {
        self.path.join("catalog.json")
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_is_supported() {
        assert_eq!(
            parse_args([OsString::from("--help")]).unwrap(),
            Command::Help
        );
    }

    #[test]
    fn parses_default_durable_check() {
        assert_eq!(
            parse_args([OsString::from("example.worldpack")]).unwrap(),
            Command::Check {
                source: PathBuf::from("example.worldpack"),
                inspect_only: false,
            }
        );
    }

    #[test]
    fn parses_inspect_only_before_or_after_path() {
        let expected = Command::Check {
            source: PathBuf::from("pack.world-pack.json"),
            inspect_only: true,
        };
        assert_eq!(
            parse_args([
                OsString::from("--inspect-only"),
                OsString::from("pack.world-pack.json")
            ])
            .unwrap(),
            expected
        );
        assert_eq!(
            parse_args([
                OsString::from("pack.world-pack.json"),
                OsString::from("--inspect-only")
            ])
            .unwrap(),
            expected
        );
    }

    #[test]
    fn rejects_missing_or_multiple_paths_and_unknown_options() {
        assert!(parse_args(Vec::<OsString>::new()).is_err());
        assert!(parse_args([
            OsString::from("one.worldpack"),
            OsString::from("two.worldpack")
        ])
        .is_err());
        assert!(parse_args([OsString::from("--json")]).is_err());
    }

    #[test]
    fn scratch_directory_is_removed_on_drop() {
        let path = {
            let scratch = ScratchDir::new().unwrap();
            let path = scratch.path.clone();
            assert!(path.is_dir());
            path
        };
        assert!(!path.exists());
    }
}
