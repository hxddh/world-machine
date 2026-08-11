use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use world_persistence::WorldArchive;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Inspect(PathBuf),
    Validate(PathBuf),
    ListPacks,
    Help,
}

#[derive(Debug)]
struct CliError(String);

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for CliError {}

fn main() -> Result<(), Box<dyn Error>> {
    let command = parse_command(env::args().skip(1))?;
    match command {
        Command::Inspect(path) => println!("{}", inspect_report(&path)?),
        Command::Validate(path) => println!("{}", validate_report(&path)?),
        Command::ListPacks => println!("{}", pack_report()?),
        Command::Help => println!("{}", usage()),
    }
    Ok(())
}

fn parse_command<I, S>(args: I) -> Result<Command, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<String>>();
    match args.as_slice() {
        [command, path] if command == "inspect" => Ok(Command::Inspect(PathBuf::from(path))),
        [command, path] if command == "validate" => Ok(Command::Validate(PathBuf::from(path))),
        [command] if command == "list-packs" => Ok(Command::ListPacks),
        [command] if matches!(command.as_str(), "help" | "--help" | "-h") => Ok(Command::Help),
        [] => Ok(Command::Help),
        _ => Err(CliError(format!("invalid arguments\n\n{}", usage()))),
    }
}

fn usage() -> &'static str {
    "World Machine document tools\n\n\
Usage:\n\
  world-cli inspect <file.world>\n\
  world-cli validate <file.world>\n\
  world-cli list-packs\n\n\
inspect     Parse and summarize a World archive without requiring its Pack.\n\
validate    Parse the archive and open it through the currently installed Pack registry.\n\
list-packs  List World Packs this build can create and restore."
}

fn load_archive(path: &Path) -> Result<WorldArchive, Box<dyn Error>> {
    let json = fs::read_to_string(path)?;
    Ok(WorldArchive::from_json(&json)?)
}

fn inspect_report(path: &Path) -> Result<String, Box<dyn Error>> {
    let archive = load_archive(path)?;
    Ok(format_archive_report(path, &archive))
}

fn format_archive_report(path: &Path, archive: &WorldArchive) -> String {
    let mut lines = vec![
        format!("file: {}", path.display()),
        format!("format: {}@{}", archive.format, archive.format_version),
        format!("pack: {}@{}", archive.pack.id, archive.pack.version),
        format!("world_time: {}", archive.world_time),
        format!("events: {}", archive.events.len()),
        format!("pending: {}", archive.pending.len()),
    ];

    if let Some(event) = archive.events.last() {
        lines.push(format!(
            "last_event: #{} {} @ t={}",
            event.id, event.kind, event.world_time
        ));
    }
    lines.join("\n")
}

fn validate_report(path: &Path) -> Result<String, Box<dyn Error>> {
    let archive = load_archive(path)?;
    let registry = world_builtins::registry()?;
    let session = registry.open_archive(&archive)?;
    let snapshot = session.snapshot();

    Ok(format!(
        "{}\nvalidation: ok\nruntime_title: {}\nprojection_world_time: {}",
        format_archive_report(path, &archive),
        snapshot.title,
        snapshot.world_time
    ))
}

fn pack_report() -> Result<String, Box<dyn Error>> {
    let registry = world_builtins::registry()?;
    let descriptors = registry.descriptors();
    let mut lines = vec![format!("packs: {}", descriptors.len())];
    for descriptor in descriptors {
        lines.push(format!(
            "{}@{}\t{}\t{}",
            descriptor.pack.id, descriptor.pack.version, descriptor.title, descriptor.description
        ));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_document_commands() {
        assert_eq!(
            parse_command(["inspect", "sample.world"]).unwrap(),
            Command::Inspect(PathBuf::from("sample.world"))
        );
        assert_eq!(
            parse_command(["validate", "sample.world"]).unwrap(),
            Command::Validate(PathBuf::from("sample.world"))
        );
        assert_eq!(parse_command(["list-packs"]).unwrap(), Command::ListPacks);
        assert_eq!(parse_command(Vec::<String>::new()).unwrap(), Command::Help);
        assert!(parse_command(["inspect"]).is_err());
    }

    #[test]
    fn inspect_report_is_pack_independent() {
        let archive = WorldArchive {
            format: world_persistence::WORLD_ARCHIVE_FORMAT.into(),
            format_version: world_persistence::WORLD_ARCHIVE_VERSION,
            pack: world_persistence::WorldPackRef::new("example.uninstalled", "7"),
            world_time: 42,
            events: Vec::new(),
            pending: Vec::new(),
        };
        let path = Path::new("sample.world");
        let report = format_archive_report(path, &archive);

        assert!(report.contains("file: sample.world"));
        assert!(report.contains("pack: example.uninstalled@7"));
        assert!(report.contains("world_time: 42"));
    }

    #[test]
    fn validate_opens_a_builtin_world_archive() {
        let registry = world_builtins::registry().unwrap();
        let pack_id = registry.descriptors()[0].pack.id.clone();
        let session = registry.create(&pack_id).unwrap();
        let archive = session.archive().unwrap().unwrap();
        let path = temp_world_path("validate");
        fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();

        let report = validate_report(&path).unwrap();

        assert!(report.contains("validation: ok"));
        assert!(report.contains(&format!("pack: {pack_id}@")));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn pack_report_lists_registered_worlds() {
        let report = pack_report().unwrap();
        assert!(report.starts_with("packs: "));
        assert!(report.lines().count() >= 2);
    }

    fn temp_world_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        env::temp_dir().join(format!(
            "world-machine-cli-{label}-{}-{nonce}.world",
            process::id()
        ))
    }
}
