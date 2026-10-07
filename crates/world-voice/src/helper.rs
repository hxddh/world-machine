//! The on-device model helper (`apps/fm-helper`): a small program that asks
//! the model built into macOS through Apple's Foundation Models framework,
//! with one read-only tool that looks things up among the facts a World
//! gave it, and answers with what it says and the ids of the facts (the
//! World's recorded events) it rests on.
//!
//! What it answers is a proposal like any model's. It is taken only when
//! every fact it cites is one it was given, it cites at least one, and the
//! answer keeps to the World by the conversation System's bounds, grounded
//! in the facts it cites. Anything else, including no helper at all, is no
//! answer, and the World's own words stand.

use crate::fm::{run_with_input, Ran};
use std::time::Duration;

/// Where the helper is, when the app ships or finds one.
pub const PROGRAM_ENV: &str = "WORLD_MACHINE_FM_HELPER";

/// Somebody is waiting for the answer.
const TIMEOUT: Duration = Duration::from_secs(25);

/// One fact a World gives the helper: the event it was recorded as, and
/// what it says in the World's words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact {
    pub id: u64,
    pub text: String,
}

/// What the helper is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asked {
    /// Who answers, and where they live.
    pub speaker: String,
    pub settlement: String,
    /// What the player said.
    pub question: String,
    pub facts: Vec<Fact>,
    pub era: conversation::Era,
}

/// An answer the World may use: what was said, and the facts it rests on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grounded {
    pub reply: String,
    pub cited: Vec<u64>,
}

/// Why a helper's answer was not used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unused {
    /// Not the JSON the helper writes, or it said why it could not answer.
    Unreadable,
    /// It cited nothing, so nothing it said rests on the World.
    Uncited,
    /// It cited a fact it was never given.
    Invented(u64),
    /// It went out of the World.
    OutOfWorld(conversation::OutOfWorld),
}

impl Asked {
    /// The JSON the helper reads on its standard input.
    pub fn request(&self) -> String {
        serde_json::json!({
            "speaker": self.speaker,
            "settlement": self.settlement,
            "question": self.question,
            "instructions": crate::fm::INSTRUCTIONS,
            "facts": self
                .facts
                .iter()
                .map(|fact| serde_json::json!({ "id": fact.id, "text": fact.text }))
                .collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// Reads what the helper wrote and checks it: every citation one of
    /// the facts it was given, at least one, and an answer that keeps to
    /// the World as the cited facts have it.
    pub fn read(&self, stdout: &str) -> Result<Grounded, Unused> {
        let start = stdout.find('{').ok_or(Unused::Unreadable)?;
        let end = stdout.rfind('}').ok_or(Unused::Unreadable)?;
        let value: serde_json::Value = stdout
            .get(start..=end)
            .and_then(|json| serde_json::from_str(json).ok())
            .ok_or(Unused::Unreadable)?;
        if value.get("error").is_some_and(|error| !error.is_null()) {
            return Err(Unused::Unreadable);
        }
        let reply = value
            .get("reply")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|reply| !reply.is_empty())
            .ok_or(Unused::Unreadable)?
            .to_string();
        let mut cited = value
            .get("cited")
            .and_then(serde_json::Value::as_array)
            .ok_or(Unused::Unreadable)?
            .iter()
            .map(|id| id.as_u64().ok_or(Unused::Unreadable))
            .collect::<Result<Vec<_>, _>>()?;
        cited.sort_unstable();
        cited.dedup();
        if cited.is_empty() {
            return Err(Unused::Uncited);
        }
        let mut grounds = vec![self.speaker.as_str(), self.settlement.as_str()];
        for id in &cited {
            let fact = self
                .facts
                .iter()
                .find(|fact| fact.id == *id)
                .ok_or(Unused::Invented(*id))?;
            grounds.push(&fact.text);
        }
        let grounds = conversation::Grounds::new(grounds, &self.question, self.era);
        conversation::keeps_to(&reply, &grounds).map_err(Unused::OutOfWorld)?;
        Ok(Grounded { reply, cited })
    }

    /// Asks the helper at `program`; its answer if it gave one the World
    /// may use.
    pub fn ask(&self, program: &str) -> Option<Grounded> {
        if !std::path::Path::new(program).is_file() {
            return None;
        }
        let ran: Ran = run_with_input(program, &[], Some(&self.request()), TIMEOUT)?;
        if !ran.success {
            return None;
        }
        self.read(&ran.stdout).ok()
    }
}

/// The helper as a judge: asked the whole judge prompt a World wrote, it
/// answers `{"verdict", "kind"}`, which the World reads like any judge's.
pub struct HelperJudge {
    pub program: String,
}

impl HelperJudge {
    /// The JSON the helper reads on its standard input to judge.
    pub fn request(prompt: &str) -> String {
        serde_json::json!({
            "mode": "judge",
            "prompt": prompt,
            "instructions": crate::fm::JUDGE_INSTRUCTIONS,
        })
        .to_string()
    }
}

impl crate::Completion for HelperJudge {
    fn complete(&mut self, prompt: &str) -> Option<String> {
        if !std::path::Path::new(&self.program).is_file() {
            return None;
        }
        let ran = run_with_input(&self.program, &[], Some(&Self::request(prompt)), TIMEOUT)?;
        ran.success.then_some(ran.stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use conversation::OutOfWorld;

    fn asked() -> Asked {
        Asked {
            speaker: "Mara".into(),
            settlement: "the harbour".into(),
            question: "What happened at the quay?".into(),
            facts: vec![
                Fact {
                    id: 41,
                    text: "Jonas mended the Sea Finch on the quay".into(),
                },
                Fact {
                    id: 57,
                    text: "Leo opened the Anchor Pub late".into(),
                },
            ],
            era: conversation::Era::Radio,
        }
    }

    #[test]
    fn the_request_carries_every_fact_with_its_event() {
        let request: serde_json::Value = serde_json::from_str(&asked().request()).unwrap();
        assert_eq!(request["facts"][1]["id"], 57);
        assert_eq!(request["question"], "What happened at the quay?");
    }

    #[test]
    fn an_answer_is_taken_only_on_the_facts_it_cites() {
        let asked = asked();
        let good = r#"{"reply":"Jonas mended the Sea Finch down on the quay.","cited":[41,41]}"#;
        assert_eq!(
            asked.read(good),
            Ok(Grounded {
                reply: "Jonas mended the Sea Finch down on the quay.".into(),
                cited: vec![41],
            })
        );
        assert_eq!(
            asked.read(r#"{"reply":"Jonas was busy.","cited":[]}"#),
            Err(Unused::Uncited)
        );
        assert_eq!(
            asked.read(r#"{"reply":"Jonas was busy.","cited":[99]}"#),
            Err(Unused::Invented(99))
        );
        // Named only in a fact it did not cite: not grounded in what it
        // rests on.
        assert_eq!(
            asked.read(r#"{"reply":"Leo was at the Anchor Pub.","cited":[41]}"#),
            Err(Unused::OutOfWorld(OutOfWorld::Stranger))
        );
        assert_eq!(
            asked.read(r#"{"reply":"As an AI, I only know event 41.","cited":[41]}"#),
            Err(Unused::OutOfWorld(OutOfWorld::Machine))
        );
        for unreadable in [
            "",
            "Jonas mended it.",
            r#"{"error":"unavailable","cited":[]}"#,
        ] {
            assert_eq!(
                asked.read(unreadable),
                Err(Unused::Unreadable),
                "{unreadable}"
            );
        }
    }

    #[test]
    fn the_helper_judges_the_worlds_own_prompt() {
        let request: serde_json::Value =
            serde_json::from_str(&HelperJudge::request("Judge <answer>Hi</answer>")).unwrap();
        assert_eq!(request["mode"], "judge");
        assert_eq!(request["prompt"], "Judge <answer>Hi</answer>");
        let mut judge = crate::ModelJudge {
            completion: Box::new(HelperJudge {
                program: "/nonexistent/world-machine/fm-helper".into(),
            }),
            name: "apple-on-device".into(),
        };
        let hearing = conversation::Hearing {
            name: "Mara".into(),
            settlement: "the harbour".into(),
            traits: Vec::new(),
            facts: Vec::new(),
            people: Vec::new(),
            places: Vec::new(),
            words: "Hi".into(),
            answer: "Hello.".into(),
            known: Vec::new(),
            era: conversation::Era::Radio,
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        assert_eq!(judge.judged(&hearing, "Hi", deadline).verdict, None);
    }

    #[test]
    fn with_no_helper_there_is_no_answer() {
        assert_eq!(asked().ask("/nonexistent/world-machine/fm-helper"), None);
    }
}
