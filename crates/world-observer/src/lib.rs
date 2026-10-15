#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

const STAMP_FORMAT: &str = "world-observer-v1";
const LEDGER_FORMAT: &str = "world-observer-sitting-v2";

mod sitting;
pub use sitting::{
    local_day, sitting_at, Sitting, SittingLedger, FIRST_SITTING_DAYS, SECONDS_PER_DAY,
    SITTING_DAYS,
};
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

    /// When the World was last caught up to, in Unix seconds, if it ever
    /// was: the clock its next period is counted from.
    pub fn last_seen(&self, key: &ObserverKey) -> Result<Option<u64>, ObserverError> {
        match fs::read(self.stamp_path(key)) {
            Ok(bytes) => Ok(parse_stamp(&bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(ObserverError::Io(error)),
        }
    }

    pub fn rollback(&self, claim: &CatchUpClaim) -> Result<(), ObserverError> {
        let Some(previous) = claim.rollback_bytes.as_deref() else {
            return Ok(());
        };
        atomic_write(&claim.path, previous)?;
        Ok(())
    }

    /// This World's sittings on this device, as its ledger has them: the
    /// empty ledger if it was never sat with here (or the ledger cannot be
    /// read, which counts as never).
    pub fn ledger(&self, key: &ObserverKey) -> Result<SittingLedger, ObserverError> {
        match fs::read(self.ledger_path(key)) {
            Ok(bytes) => Ok(parse_ledger(&bytes).unwrap_or_default()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(SittingLedger::default()),
            Err(error) => Err(ObserverError::Io(error)),
        }
    }

    /// The sitting that opening the World on local day `today` is: a new
    /// one, written down as begun, or the one already begun that day.
    pub fn sit(&self, key: &ObserverKey, today: i64) -> Result<Sitting, ObserverError> {
        let ledger = self.ledger(key)?;
        let sitting = sitting_at(&ledger, today);
        if sitting.new {
            self.write_ledger(key, &sitting.begun(&ledger, today))?;
        }
        Ok(sitting)
    }

    /// Writes down that the sitting begun on `today` passed one of its
    /// World days: nothing past its allowance, and nothing for a sitting
    /// that is not today's. Returns the sitting as it now stands.
    pub fn spend_day(&self, key: &ObserverKey, today: i64) -> Result<Sitting, ObserverError> {
        let ledger = self.ledger(key)?;
        let spent = ledger.spent(today);
        if spent != ledger {
            self.write_ledger(key, &spent)?;
        }
        Ok(sitting_at(&spent, today))
    }

    /// Writes `ledger` as this World's, whatever it held: for previews and
    /// demos that pretend time away (`examples/pretend_away.rs`).
    pub fn set_ledger(
        &self,
        key: &ObserverKey,
        ledger: &SittingLedger,
    ) -> Result<(), ObserverError> {
        self.write_ledger(key, ledger)
    }

    fn ledger_path(&self, key: &ObserverKey) -> PathBuf {
        self.root
            .join(format!("{}.sitting", stable_digest(key.as_str())))
    }

    fn write_ledger(&self, key: &ObserverKey, ledger: &SittingLedger) -> Result<(), ObserverError> {
        let began = ledger
            .began_on
            .map_or_else(|| "-".to_string(), |day| day.to_string());
        let bytes = format!(
            "{LEDGER_FORMAT}\n{}\n{began}\n{}\n",
            ledger.sittings, ledger.days_passed
        );
        atomic_write(&self.ledger_path(key), bytes.as_bytes())?;
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

fn parse_ledger(bytes: &[u8]) -> Option<SittingLedger> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.lines();
    if lines.next()? != LEDGER_FORMAT {
        return None;
    }
    let sittings = lines.next()?.parse().ok()?;
    let began_on = match lines.next()? {
        "-" => None,
        day => Some(day.parse().ok()?),
    };
    let days_passed = lines.next()?.parse().ok()?;
    if lines.next().is_some() {
        return None;
    }
    Some(SittingLedger {
        sittings,
        began_on,
        days_passed,
    })
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
    fn last_seen_is_where_the_next_period_is_counted_from() {
        let root = temp_root("last-seen");
        let store = ObserverStore::new(root.clone());
        let key = key();
        assert_eq!(store.last_seen(&key).unwrap(), None);
        store.claim_due(&key, 100, policy()).unwrap();
        assert_eq!(store.last_seen(&key).unwrap(), Some(100));
        // A look before a whole period passes leaves the clock alone.
        store.claim_due(&key, 130, policy()).unwrap();
        assert_eq!(store.last_seen(&key).unwrap(), Some(100));
        let _ = fs::remove_dir_all(root);
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
    fn a_ledger_is_written_on_a_new_sitting_and_read_back() {
        let root = temp_root("ledger");
        let store = ObserverStore::new(root.clone());
        let key = key();
        assert_eq!(store.ledger(&key).unwrap(), SittingLedger::default());

        let first = store.sit(&key, 20_000).unwrap();
        assert!(first.new);
        assert_eq!((first.number, first.left), (1, FIRST_SITTING_DAYS));
        // Opened again the same day: the same sitting, not a new allowance.
        let again = store.sit(&key, 20_000).unwrap();
        assert!(!again.new);
        assert_eq!(again.left, FIRST_SITTING_DAYS);
        assert_eq!(store.spend_day(&key, 20_000).unwrap().left, 2);
        assert_eq!(store.sit(&key, 20_000).unwrap().left, 2);
        for _ in 0..5 {
            store.spend_day(&key, 20_000).unwrap();
        }
        assert_eq!(store.sit(&key, 20_000).unwrap().left, 0);
        // A month on: one day, never the month.
        let back = store.sit(&key, 20_030).unwrap();
        assert!(back.new);
        assert_eq!(
            (back.number, back.allowance, back.left),
            (2, SITTING_DAYS, 1)
        );
        assert_eq!(back.days_away, 30);
        // Spending on a day that is not the sitting's changes nothing.
        assert_eq!(store.spend_day(&key, 20_031).unwrap().left, 1);
        assert_eq!(store.ledger(&key).unwrap().days_passed, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn an_unreadable_ledger_is_a_first_sitting() {
        let root = temp_root("bad-ledger");
        let store = ObserverStore::new(root.clone());
        let key = key();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            store.ledger_path(&key),
            "world-observer-sitting-v2\nnot a number\n",
        )
        .unwrap();
        assert_eq!(store.ledger(&key).unwrap(), SittingLedger::default());
        assert_eq!(store.sit(&key, 9).unwrap().number, 1);
        // The stamp and the ledger are files of their own.
        assert_ne!(store.ledger_path(&key), store.stamp_path(&key));
        let _ = fs::remove_dir_all(root);
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
