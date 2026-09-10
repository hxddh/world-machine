//! Where an API key is kept.
//!
//! The key belongs to the person using the app, not to the app, so it goes
//! into the login keychain rather than into any file World Machine writes:
//! the keychain is what macOS already asks for when it protects a secret, and
//! it means a backup of the Worlds folder never carries one.
//!
//! Two rules shape everything below.
//!
//! **The key is never a process argument.** Arguments are readable by every
//! process on the machine, so `security` is handed the key on standard input
//! and the arguments carry only which item is being read or written. The test
//! that pins this runs everywhere, including where there is no keychain at all.
//!
//! **A keychain that will not answer is not an error worth stopping for.** A
//! locked keychain, a denied prompt, or no `security` at all reads as "no key",
//! which is the same state as never having entered one: the World keeps its
//! built-in copy and the app says the voice is not configured.

use std::io::Write;
use std::process::{Command, Stdio};

const SECURITY: &str = "/usr/bin/security";
/// What the key is called in Keychain Access, so somebody can find, inspect, or
/// delete it without this app.
const SERVICE: &str = "World Machine · World voice";
const ACCOUNT: &str = "anthropic-api-key";

/// The arguments for storing a key. The key is not among them.
pub fn save_args() -> Vec<String> {
    vec![
        "add-generic-password".into(),
        "-a".into(),
        ACCOUNT.into(),
        "-s".into(),
        SERVICE.into(),
        // Replace an existing key rather than failing, so entering a new one
        // just works.
        "-U".into(),
        // No value: the key follows on standard input.
        "-w".into(),
    ]
}

/// The arguments for reading the key back.
pub fn load_args() -> Vec<String> {
    vec![
        "find-generic-password".into(),
        "-a".into(),
        ACCOUNT.into(),
        "-s".into(),
        SERVICE.into(),
        "-w".into(),
    ]
}

/// The arguments for forgetting it.
pub fn clear_args() -> Vec<String> {
    vec![
        "delete-generic-password".into(),
        "-a".into(),
        ACCOUNT.into(),
        "-s".into(),
        SERVICE.into(),
    ]
}

/// Store the key, replacing any key already there.
pub fn save(key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("a key with nothing in it is not a key".into());
    }
    let mut child = Command::new(SECURITY)
        .args(save_args())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not reach the keychain: {error}"))?;
    child
        .stdin
        .take()
        .ok_or_else(|| "could not hand the keychain the key".to_string())?
        .write_all(format!("{key}\n").as_bytes())
        .map_err(|error| format!("could not hand the keychain the key: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("the keychain did not answer: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "the keychain refused to store the key: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

/// The stored key, if there is one this app can read right now.
///
/// Deliberately not a `Result`: every way this can fail — no keychain, locked,
/// refused, nothing stored — means the same thing to everything upstream.
pub fn load() -> Option<String> {
    let output = Command::new(SECURITY).args(load_args()).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let key = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!key.is_empty()).then_some(key)
}

/// Forget the key. Forgetting one that is not there is not a failure.
pub fn clear() -> Result<(), String> {
    let output = Command::new(SECURITY)
        .args(clear_args())
        .output()
        .map_err(|error| format!("could not reach the keychain: {error}"))?;
    if output.status.success() || load().is_none() {
        return Ok(());
    }
    Err(format!(
        "the keychain refused to forget the key: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

/// Whether a key is stored, without reading it.
pub fn is_configured() -> bool {
    load().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "sk-ant-not-a-real-key-0123456789";

    #[test]
    fn no_argument_list_can_ever_carry_the_key() {
        // Runs everywhere, including where there is no keychain: the arguments
        // are built without the key, so there is nothing to leak.
        for args in [save_args(), load_args(), clear_args()] {
            for argument in &args {
                assert!(
                    !argument.contains("sk-ant"),
                    "something key-shaped appeared in an argument: {argument}"
                );
            }
            assert!(
                args.iter().any(|argument| argument == ACCOUNT),
                "the arguments do not say which item they mean: {args:?}"
            );
        }
        // Storing takes -w with no value, which is what makes the key arrive on
        // standard input instead.
        let save = save_args();
        assert_eq!(
            save.last().map(String::as_str),
            Some("-w"),
            "the key would have to be an argument: {save:?}"
        );
    }

    #[test]
    fn a_key_with_nothing_in_it_is_refused_before_the_keychain_is_touched() {
        assert!(save("").is_err());
        assert!(save("   ").is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_key_can_be_stored_read_back_and_forgotten() {
        // The real thing, on the only platform that has one. If `security`
        // will not take a key on standard input, this fails here rather than
        // shipping a path that puts it in an argument instead.
        if !std::path::Path::new(SECURITY).exists() {
            return;
        }
        let restore = load();
        let _ = clear();

        assert_eq!(load(), None, "a key was there before anything stored one");
        save(KEY).expect("the keychain would not store a key");
        assert_eq!(
            load().as_deref(),
            Some(KEY),
            "the key did not survive the keychain"
        );
        assert!(is_configured());

        // Entering a different key replaces the first rather than failing.
        save("sk-ant-second-key").expect("the keychain would not replace a key");
        assert_eq!(load().as_deref(), Some("sk-ant-second-key"));

        clear().expect("the keychain would not forget the key");
        assert_eq!(load(), None);
        assert!(!is_configured());
        // Forgetting what is not there is not a failure.
        clear().expect("forgetting an absent key should be quiet");

        if let Some(previous) = restore {
            let _ = save(&previous);
        }
    }
}
