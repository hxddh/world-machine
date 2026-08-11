use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

const STAMP_FORMAT: &str = "world-observer-v1";
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatchUpPolicy {
    pub seconds_per_period: u64,
    pub max_periods: u64,
}

impl CatchUpPolicy {
    pub fn new(seconds_per_period: u64, max_periods: u64) -> Result<Self, ObserverError> {
        if seconds_per_period == 0 || max_periods == 0 {
            return Err(ObserverError::InvalidPolicy);
        }
        Ok(Self {
            seconds_per_period,
            max_periods,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObserverKey(String);

impl ObserverKey {
    pub fn new(value: impl Into<String>) -> Result<Self, ObserverError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ObserverError::InvalidKey);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct ObserverStore {
    root: PathBuf,
}

impl ObserverStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn claim_due(
        &self,
        key: &ObserverKey,
        now_unix_seconds: u64,
        policy: CatchUpPolicy,
    ) -> Result<CatchUpClaim, ObserverError> {
        if policy.seconds_per_period == 0 || policy.max_periods == 0 {
            return Err(ObserverError::InvalidPolicy);
        }

        let path = self.stamp_path(key);
        let previous = match fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(ObserverError::Io(error)),
        };

        let Some(previous_bytes) = previous.as_deref() else {
            self.write_stamp(&path, now_unix_seconds)?;
            return Ok(CatchUpClaim::idle(path));
        };

        let Some(previous_seconds) = parse_stamp(previous_bytes) else {
            self.write_stamp(&path, now_unix_seconds)?;
            return Ok(CatchUpClaim::idle(path));
        };

        if now_unix_seconds < previous_seconds {
            self.write_stamp(&path, now_unix_seconds)?;
            return Ok(CatchUpClaim::idle(path));
        }

        let elapsed = now_unix_seconds - previous_seconds;
        let raw_periods = elapsed / policy.seconds_per_period;
        if raw_periods == 0 {
            return Ok(CatchUpClaim::idle(path));
        }

        let periods = raw_periods.min(policy.max_periods);
        self.write_stamp(&path, now_unix_seconds)?;
        Ok(CatchUpClaim {
            periods,
            path,
            rollback_bytes: previous,
        })
    }

    pub fn rollback(&self, claim: &CatchUpClaim) -> Result<(), ObserverError> {
        let Some(previous) = claim.rollback_bytes.as_deref() else {
            return Ok(());
        };
        atomic_write(&claim.path, previous)?;
        Ok(())
    }

    fn stamp_path(&self, key: &ObserverKey) -> PathBuf {
        self.root
            .join(format!("{}.stamp", stable_digest(key.as_str())))
    }

    fn write_stamp(&self, path: &Path, unix_seconds: u64) -> Result<(), ObserverError> {
        let bytes = format!("{STAMP_FORMAT}\n{unix_seconds}\n");
        atomic_write(path, bytes.as_bytes())?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct CatchUpClaim {
    periods: u64,
    path: PathBuf,
    rollback_bytes: Option<Vec<u8>>,
}

impl CatchUpClaim {
    fn idle(path: PathBuf) -> Self {
        Self {
            periods: 0,
            path,
            rollback_bytes: None,
        }
    }

    pub fn periods(&self) -> u64 {
        self.periods
    }

    pub fn is_due(&self) -> bool {
        self.periods > 0
    }
}

#[derive(Debug)]
pub enum ObserverError {
    InvalidKey,
    InvalidPolicy,
    Io(io::Error),
}

impl fmt::Display for ObserverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey => write!(f, "observer key must not be empty"),
            Self::InvalidPolicy => write!(
                f,
                "observer catch-up policy requires non-zero period seconds and maximum periods"
            ),
            Self::Io(error) => error.fmt(f),
        }
    }
}

impl Error for ObserverError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::InvalidKey | Self::InvalidPolicy => None,
        }
    }
}

impl From<io::Error> for ObserverError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn parse_stamp(bytes: &[u8]) -> Option<u64> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.lines();
    if lines.next()? != STAMP_FORMAT {
        return None;
    }
    let unix_seconds = lines.next()?.parse().ok()?;
    if lines.next().is_some() {
        return None;
    }
    Some(unix_seconds)
}

fn stable_digest(value: &str) -> String {
    format!(
        "{:016x}{:016x}",
        fnv64(0x01, value.as_bytes()),
        fnv64(0x02, value.as_bytes())
    )
}

fn fnv64(domain: u8, bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    hash ^= domain as u64;
    hash = hash.wrapping_mul(FNV_PRIME);
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::other("observer stamp path has no file name"))?
        .to_string_lossy();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp_path = path.with_file_name(format!(
        ".{file_name}.observer-{}-{nonce}.tmp",
        process::id()
    ));

    let result = (|| -> io::Result<()> {
        let mut file = File::create(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "world-machine-observer-{}-{nonce}-{label}",
            process::id()
        ))
    }

    fn key() -> ObserverKey {
        ObserverKey::new("library:tiny-society-1").unwrap()
    }

    fn policy() -> CatchUpPolicy {
        CatchUpPolicy::new(60, 3).unwrap()
    }

    #[test]
    fn first_observation_initializes_without_catch_up() {
        let root = temp_root("first");
        let store = ObserverStore::new(root.clone());

        let claim = store.claim_due(&key(), 100, policy()).unwrap();

        assert_eq!(claim.periods(), 0);
        assert!(!claim.is_due());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sub_period_observations_preserve_elapsed_remainder() {
        let root = temp_root("remainder");
        let store = ObserverStore::new(root.clone());
        let key = key();
        store.claim_due(&key, 100, policy()).unwrap();

        assert_eq!(store.claim_due(&key, 130, policy()).unwrap().periods(), 0);
        assert_eq!(store.claim_due(&key, 161, policy()).unwrap().periods(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn catch_up_is_bounded_and_claim_consumes_backlog() {
        let root = temp_root("bounded");
        let store = ObserverStore::new(root.clone());
        let key = key();
        store.claim_due(&key, 100, policy()).unwrap();

        let claim = store.claim_due(&key, 1_000, policy()).unwrap();

        assert_eq!(claim.periods(), 3);
        assert_eq!(store.claim_due(&key, 1_001, policy()).unwrap().periods(), 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rollback_restores_the_previous_anchor() {
        let root = temp_root("rollback");
        let store = ObserverStore::new(root.clone());
        let key = key();
        store.claim_due(&key, 100, policy()).unwrap();
        let claim = store.claim_due(&key, 280, policy()).unwrap();
        assert_eq!(claim.periods(), 3);

        store.rollback(&claim).unwrap();

        assert_eq!(store.claim_due(&key, 280, policy()).unwrap().periods(), 3);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn backward_clock_resets_without_catch_up() {
        let root = temp_root("backward");
        let store = ObserverStore::new(root.clone());
        let key = key();
        store.claim_due(&key, 200, policy()).unwrap();

        assert_eq!(store.claim_due(&key, 100, policy()).unwrap().periods(), 0);
        assert_eq!(store.claim_due(&key, 159, policy()).unwrap().periods(), 0);
        assert_eq!(store.claim_due(&key, 160, policy()).unwrap().periods(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn malformed_metadata_is_reinitialized_without_advancing() {
        let root = temp_root("malformed");
        let store = ObserverStore::new(root.clone());
        let key = key();
        let path = store.stamp_path(&key);
        fs::create_dir_all(&root).unwrap();
        fs::write(&path, "not an observer stamp").unwrap();

        assert_eq!(store.claim_due(&key, 500, policy()).unwrap().periods(), 0);
        assert_eq!(parse_stamp(&fs::read(path).unwrap()), Some(500));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn keys_map_to_distinct_stable_stamp_paths() {
        let store = ObserverStore::new(PathBuf::from("observer-root"));
        let first = ObserverKey::new("library:a").unwrap();
        let second = ObserverKey::new("library:b").unwrap();

        assert_eq!(store.stamp_path(&first), store.stamp_path(&first));
        assert_ne!(store.stamp_path(&first), store.stamp_path(&second));
    }

    #[test]
    fn invalid_policy_and_key_are_rejected() {
        assert!(matches!(
            ObserverKey::new("   "),
            Err(ObserverError::InvalidKey)
        ));
        assert!(matches!(
            CatchUpPolicy::new(0, 3),
            Err(ObserverError::InvalidPolicy)
        ));
        assert!(matches!(
            CatchUpPolicy::new(60, 0),
            Err(ObserverError::InvalidPolicy)
        ));
    }
}
