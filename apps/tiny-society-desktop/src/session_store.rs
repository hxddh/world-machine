use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const SAVE_OVERRIDE_ENV: &str = "WORLD_MACHINE_TINY_SOCIETY_SAVE";

#[derive(Clone, Debug)]
pub struct SessionStore {
    world_path: PathBuf,
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
        Self { world_path }
    }

    pub fn path(&self) -> &Path {
        &self.world_path
    }

    pub fn load(&self) -> io::Result<Option<String>> {
        match fs::read_to_string(&self.world_path) {
            Ok(json) => Ok(Some(json)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn save(&self, json: &str) -> io::Result<()> {
        let parent = self
            .world_path
            .parent()
            .ok_or_else(|| io::Error::other("save path has no parent directory"))?;
        fs::create_dir_all(parent)?;

        let temp_path = self.world_path.with_extension("json.tmp");
        let mut file = File::create(&temp_path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
        drop(file);

        fs::rename(&temp_path, &self.world_path)?;
        Ok(())
    }
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
    fn missing_world_returns_none() {
        let store = test_store("missing");
        cleanup(&store);

        assert_eq!(store.load().unwrap(), None);
    }

    #[test]
    fn save_round_trips_and_atomically_replaces_the_world() {
        let store = test_store("round-trip");
        cleanup(&store);

        store.save("first archive").unwrap();
        assert_eq!(store.load().unwrap().as_deref(), Some("first archive"));

        store.save("second archive").unwrap();
        assert_eq!(store.load().unwrap().as_deref(), Some("second archive"));
        assert!(!store.path().with_extension("json.tmp").exists());

        cleanup(&store);
    }
}
