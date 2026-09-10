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
//! process on the machine, so storing goes through `security -i`, which reads
//! the command itself from standard input: the key travels inside that command
//! text and never appears in the process table. The test that pins this runs
//! everywhere, including where there is no keychain at all.
//!
//! **A key is only stored once it has been read back.** The first attempt at
//! this passed the key with a bare `-w` and trusted the exit status; the
//! keychain reported success and had stored nothing, which is the worst way for
//! this to fail. Storing now proves itself by reading the key back, so
//! "success" means the key is really there.
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

/// The arguments for storing a key: read the command from standard input, so
/// nothing about it — least of all the key — becomes an argument.
pub fn save_args() -> Vec<String> {
    vec!["-i".into()]
}

/// The command `security -i` is given on standard input.
///
/// `-U` replaces an existing key rather than failing, so entering a new one
/// just works.
pub fn save_command(key: &str) -> String {
    format!(
        "add-generic-password -a {} -s {} -U -w {}\n",
        quote(ACCOUNT),
        quote(SERVICE),
        quote(key)
    )
}

/// `security -i` splits its commands on whitespace and honours double quotes,
/// so every value is quoted and anything that could end a quoted value early is
/// escaped.
fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
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
///
/// Proves itself: a keychain that reports success and stored nothing is not a
/// success, and that is exactly how this failed the first time.
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
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "could not hand the keychain the key".to_string())?;
        stdin
            .write_all(save_command(key).as_bytes())
            .map_err(|error| format!("could not hand the keychain the key: {error}"))?;
        // Closing standard input is what ends the interactive session.
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("the keychain did not answer: {error}"))?;
    if load().as_deref() == Some(key) {
        return Ok(());
    }
    let complaint = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(if complaint.is_empty() {
        "the keychain did not keep the key".to_string()
    } else {
        format!("the keychain refused to store the key: {complaint}")
    })
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
                !args.is_empty(),
                "an empty argument list would run the wrong thing: {args:?}"
            );
        }
        // Storing takes only -i, so the whole command — key included — arrives
        // on standard input instead of in the process table.
        assert_eq!(save_args(), vec!["-i".to_string()]);
        let command = save_command(KEY);
        assert!(
            command.contains(KEY),
            "the key never reached the command the keychain actually reads"
        );
        assert!(
            command.starts_with("add-generic-password ") && command.ends_with('\n'),
            "the interactive command is malformed: {command:?}"
        );
    }

    #[test]
    fn a_key_cannot_smuggle_a_second_command_into_the_keychain() {
        // The command is read from standard input, so a key that could end its
        // own quoted value could append instructions of its own.
        let command = save_command("abc\" delete-generic-password -a x");
        assert_eq!(
            command.matches("add-generic-password").count(),
            1,
            "the key wrote a command of its own: {command:?}"
        );
        // The quote the key carried is escaped, so it cannot end the value it
        // sits inside.
        assert!(
            command.contains("\\\""),
            "the key's quote was not escaped: {command:?}"
        );
        assert!(
            !command.contains("-w \"abc\" "),
            "the key ended its own value: {command:?}"
        );
        assert_eq!(
            command.lines().count(),
            1,
            "the key added a line: {command:?}"
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
