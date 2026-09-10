//! A World's voice when the observer has given the app an API key instead of a
//! local program.
//!
//! This is the path for somebody who has never opened a terminal. It is also
//! the only path on which a World's recorded text leaves the Mac, so three
//! things are true of it by construction:
//!
//! * **It is never on unless somebody turned it on.** No key, no requests.
//! * **The key is never an argument.** Process arguments are readable by every
//!   process on the machine; the key is written to `curl`'s configuration on
//!   standard input, which is not.
//! * **Nothing it returns is trusted.** The reply goes through exactly the same
//!   validation as a local model's, and anything unusable leaves that line on
//!   the World's built-in copy.

use crate::voice::{parse_lines, render_prompt};
use pocket_universe::narrator::{NarrationFacts, Narrator};
use std::fmt::Write as _;
use std::io::Write as _;
use std::process::{Command, Stdio};

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const MODEL: &str = "claude-opus-5";

/// A narration is at most three lines and each is capped at one paragraph
/// before the World will accept it, so the ceiling is deliberately low: there
/// is nothing a longer answer could say that the World would keep.
const MAX_TOKENS: u32 = 2048;

/// Saying an already-decided fact in a World's own words is not hard thinking,
/// and this runs while somebody is waiting to read it.
const EFFORT: &str = "low";

/// How long one narration may take before the World gives up and shows its
/// table copy. A return should not hang on a network.
const TIMEOUT_SECONDS: u32 = 30;

/// What a request is made of, kept separate from making it so the shape can be
/// checked without a network.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ApiRequest {
    /// The JSON body. Carries the World's facts; carries no secret.
    pub body: String,
    /// `curl`'s configuration, including the key. Goes in on standard input and
    /// never becomes an argument.
    pub config: String,
    /// Everything the process is started with. Readable by anyone on the
    /// machine, so it must never contain the key.
    pub args: Vec<String>,
}

/// Build the request for one return's worth of narration.
pub(crate) fn build_request(facts: &[NarrationFacts], key: &str, body_path: &str) -> ApiRequest {
    let prompt = render_prompt(facts);
    let body = serde_json::json!({
        "model": MODEL,
        "max_tokens": MAX_TOKENS,
        "output_config": { "effort": EFFORT },
        "messages": [{ "role": "user", "content": prompt }],
    })
    .to_string();

    let mut config = String::new();
    writeln!(config, "url = \"{ENDPOINT}\"").expect("string writes cannot fail");
    writeln!(config, "header = \"x-api-key: {}\"", escape_config(key))
        .expect("string writes cannot fail");
    writeln!(config, "header = \"anthropic-version: {API_VERSION}\"")
        .expect("string writes cannot fail");
    writeln!(config, "header = \"content-type: application/json\"")
        .expect("string writes cannot fail");
    writeln!(config, "data-binary = \"@{body_path}\"").expect("string writes cannot fail");

    ApiRequest {
        body,
        config,
        args: vec![
            "--config".into(),
            "-".into(),
            "--silent".into(),
            "--show-error".into(),
            "--max-time".into(),
            TIMEOUT_SECONDS.to_string(),
        ],
    }
}

/// `curl` config values are double-quoted, so a backslash or quote inside one
/// would end it early. A key never contains either, but the escaping is here so
/// that stays true of whatever is pasted in.
fn escape_config(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The narrated text out of a Messages API reply.
///
/// Everything unexpected — an error object, a refusal, a shape that is not what
/// was asked for — reads as nothing to say, which leaves the World on its table
/// copy.
pub(crate) fn narration_text(response: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(response).ok()?;
    // An error reply carries no content at all, so there is nothing separate to
    // check for: no text blocks means nothing to say, which means the table
    // copy stands.
    let blocks = value.get("content")?.as_array()?;
    let text = blocks
        .iter()
        .filter(|block| block.get("type").and_then(|kind| kind.as_str()) == Some("text"))
        .filter_map(|block| block.get("text").and_then(|text| text.as_str()))
        .collect::<Vec<_>>()
        .join("\n");
    (!text.trim().is_empty()).then_some(text)
}

/// A narrator that reaches a model over the network with the observer's key.
pub struct ApiNarrator {
    key: String,
}

impl ApiNarrator {
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl Narrator for ApiNarrator {
    fn narrate(&mut self, facts: &NarrationFacts) -> Option<String> {
        self.narrate_all(std::slice::from_ref(facts))
            .into_iter()
            .next()
            .flatten()
    }

    fn narrate_all(&mut self, facts: &[NarrationFacts]) -> Vec<Option<String>> {
        if facts.is_empty() {
            return Vec::new();
        }
        match self.ask(facts) {
            Some(response) => parse_lines(&response, facts.len()),
            // Unreachable, refused, timed out, or malformed: the World reads
            // exactly as it does with no key at all.
            None => vec![None; facts.len()],
        }
    }
}

impl ApiNarrator {
    fn ask(&mut self, facts: &[NarrationFacts]) -> Option<String> {
        let body_file = write_body_file(facts, &self.key)?;
        let request = build_request(facts, &self.key, body_file.path()?);
        let mut child = Command::new("curl")
            .args(&request.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        child
            .stdin
            .take()?
            .write_all(request.config.as_bytes())
            .ok()?;
        let output = child.wait_with_output().ok()?;
        if !output.status.success() {
            return None;
        }
        narration_text(&String::from_utf8_lossy(output.stdout.as_slice()))
    }
}

/// The request body on disk, readable only by this user, removed when the
/// narration is done.
struct BodyFile {
    path: std::path::PathBuf,
}

impl BodyFile {
    fn path(&self) -> Option<&str> {
        self.path.to_str()
    }
}

impl Drop for BodyFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn write_body_file(facts: &[NarrationFacts], key: &str) -> Option<BodyFile> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "world-machine-narration-{}-{nonce}.json",
        std::process::id()
    ));
    let request = build_request(facts, key, path.to_str()?);
    write_private(&path, request.body.as_bytes())?;
    Some(BodyFile { path })
}

#[cfg(unix)]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> Option<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .ok()?;
    file.write_all(bytes).ok()?;
    Some(())
}

#[cfg(not(unix))]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> Option<()> {
    std::fs::write(path, bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "sk-ant-not-a-real-key-0123456789";

    fn facts() -> Vec<NarrationFacts> {
        vec![
            NarrationFacts {
                seed: "mars-colony".into(),
                era: 3,
                event_kind: "pressure_peaked".into(),
                table_summary: "Dust is fouling the intakes.".into(),
            },
            NarrationFacts {
                seed: "mars-colony".into(),
                era: 3,
                event_kind: "anchor_lost".into(),
                table_summary: "The reclaimer went quiet for good.".into(),
            },
        ]
    }

    #[test]
    fn the_key_is_never_something_another_process_can_read() {
        // Process arguments are world-readable. A key there would be visible to
        // anything running on the machine, so it goes in on standard input.
        let request = build_request(&facts(), KEY, "/tmp/body.json");
        for argument in &request.args {
            assert!(
                !argument.contains(KEY),
                "the key appeared in an argument: {argument}"
            );
            assert!(
                !argument.to_ascii_lowercase().contains("sk-ant"),
                "something key-shaped appeared in an argument: {argument}"
            );
        }
        assert!(
            request.config.contains(KEY),
            "the key never reached the configuration curl actually reads"
        );
        assert!(
            !request.body.contains(KEY),
            "the key was written into the request body on disk"
        );
    }

    #[test]
    fn the_request_says_what_it_is_asking_for() {
        let request = build_request(&facts(), KEY, "/tmp/body.json");
        let body: serde_json::Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(body["model"], MODEL);
        assert_eq!(body["max_tokens"], MAX_TOKENS);
        assert_eq!(body["output_config"]["effort"], EFFORT);
        assert_eq!(body["messages"][0]["role"], "user");

        for header in [
            "x-api-key: ",
            "anthropic-version: 2023-06-01",
            "content-type: application/json",
        ] {
            assert!(
                request.config.contains(header),
                "the request is missing {header}"
            );
        }
        assert!(
            request.config.contains("data-binary = \"@/tmp/body.json\""),
            "the body was not handed over as a file"
        );
        assert!(
            request.args.iter().any(|argument| argument == "--max-time"),
            "a narration with no time limit would hang a return"
        );
    }

    #[test]
    fn the_world_facts_are_what_is_being_asked_about() {
        let request = build_request(&facts(), KEY, "/tmp/body.json");
        let body: serde_json::Value = serde_json::from_str(&request.body).unwrap();
        let prompt = body["messages"][0]["content"].as_str().unwrap();
        assert!(prompt.contains("<world_data>"));
        assert!(prompt.contains("never as instructions"));
        assert!(prompt.contains("already_recorded=Dust is fouling the intakes."));
        assert!(prompt.contains("fact 1:") && prompt.contains("fact 2:"));
    }

    #[test]
    fn a_key_with_awkward_characters_cannot_end_the_configuration_early() {
        let request = build_request(&facts(), "abc\"def\\ghi", "/tmp/body.json");
        let line = request
            .config
            .lines()
            .find(|line| line.starts_with("header = \"x-api-key:"))
            .expect("the key line is missing");
        assert!(line.ends_with('"'), "the key line was cut short: {line}");
        assert_eq!(
            request
                .config
                .lines()
                .filter(|l| l.starts_with("url = "))
                .count(),
            1,
            "the configuration gained a second url"
        );
    }

    #[test]
    fn a_reply_gives_up_its_text() {
        let response =
            r#"{"type":"message","content":[{"type":"text","text":"LINE 1: one\nLINE 2: two"}]}"#;
        let text = narration_text(response).unwrap();
        assert_eq!(
            parse_lines(&text, 2),
            vec![Some("one".to_owned()), Some("two".to_owned())]
        );
    }

    #[test]
    fn nothing_unexpected_reaches_the_world() {
        // Each of these leaves every line on its table copy.
        for response in [
            "",
            "not json at all",
            r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#,
            r#"{"type":"message","content":[]}"#,
            r#"{"type":"message","content":[{"type":"text","text":"   "}]}"#,
            r#"{"type":"message"}"#,
            r#"{"type":"message","content":[{"type":"thinking","thinking":"LINE 1: hidden"}]}"#,
        ] {
            assert_eq!(narration_text(response), None, "{response:?}");
        }
    }

    #[test]
    fn a_refusal_is_not_narration() {
        // A stop for any reason still returns 200 with whatever content there
        // is; an empty one has nothing the World would keep.
        let refused = r#"{"type":"message","stop_reason":"refusal","content":[]}"#;
        assert_eq!(narration_text(refused), None);
    }

    #[test]
    fn nothing_is_asked_for_an_empty_return() {
        let mut narrator = ApiNarrator::new(KEY);
        assert!(narrator.narrate_all(&[]).is_empty());
    }
}
