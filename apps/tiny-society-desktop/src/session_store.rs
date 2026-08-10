use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const SAVE_OVERRIDE_ENV: &str = "WORLD_MACHINE_TINY_SOCIETY_SAVE";

#[derive(Clone, Debug)]
pub struct SessionStore {
    world_path: PathBuf,
    visit_path: PathBuf,
}

impl SessionStore {
    pub fn discover() -> io::Result<Self> {
        if let Some(path) = env::var_os(SAVE_OVERRIDE_ENV) {
            return Ok(Self::new(PathBuf::from(path)));
        }

        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::other("HOME is not set"))?;
        Ok(Self::new(
            home.join("Library")
                .join("Application Support")
                .join("World Machine")
                .join("Tiny Society")
                .join("current.world.json"),
        ))
    }

    pub fn new(world_path: PathBuf) -> Self {
        let visit_path = world_path.with_extension("visit");
        Self {
            world_path,
            visit_path,
        }
    }

    pub fn path(&self) -> &Path {
        &self.world_path
    }

    pub fn load(&self) -> io::Result<Option<String>> {
        read_optional(&self.world_path)
    }

    pub fn save(&self, json: &str) -> io::Result<()> {
        atomic_write(&self.world_path, json.as_bytes())
    }

    pub fn load_visit_cursor(&self) -> io::Result<Option<usize>> {
        let Some(value) = read_optional(&self.visit_path)? else {
            return Ok(None);
        };
        Ok(value.trim().parse::<usize>().ok())
    }

    pub fn save_visit_cursor(&self, event_count: usize) -> io::Result<()> {
        atomic_write(&self.visit_path, event_count.to_string().as_bytes())
    }
}

fn read_optional(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("save path has no parent directory"))?;
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::other("save path has no file name"))?
        .to_string_lossy();
    let temp_path = path.with_file_name(format!("{file_name}.tmp"));
    let mut file = File::create(&temp_path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);

    fs::rename(&temp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_store(label: &str) -> SessionStore {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        SessionStore::new(
            env::temp_dir()
                .join(format!(
                    "world-machine-session-store-{}-{nonce}-{label}",
                    process::id()
                ))
                .join("current.world.json"),
        )
    }

    fn cleanup(store: &SessionStore) {
        if let Some(directory) = store.path().parent() {
            let _ = fs::remove_dir_all(directory);
        }
    }

    #[test]
    fn missing_world_and_cursor_return_none() {
        let store = test_store("missing");
        cleanup(&store);

        assert_eq!(store.load().unwrap(), None);
        assert_eq!(store.load_visit_cursor().unwrap(), None);
    }

    #[test]
    fn save_round_trips_and_atomically_replaces_the_world() {
        let store = test_store("round-trip");
        cleanup(&store);

        store.save("first archive").unwrap();
        assert_eq!(store.load().unwrap().as_deref(), Some("first archive"));

        store.save("second archive").unwrap();
        assert_eq!(store.load().unwrap().as_deref(), Some("second archive"));
        assert!(!store
            .path()
            .with_file_name("current.world.json.tmp")
            .exists());

        cleanup(&store);
    }

    #[test]
    fn visit_cursor_round_trips_and_invalid_metadata_is_ignored() {
        let store = test_store("visit");
        cleanup(&store);

        store.save_visit_cursor(42).unwrap();
        assert_eq!(store.load_visit_cursor().unwrap(), Some(42));

        atomic_write(&store.visit_path, b"not-a-number").unwrap();
        assert_eq!(store.load_visit_cursor().unwrap(), None);

        cleanup(&store);
    }
}
