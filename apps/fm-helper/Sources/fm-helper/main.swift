// fm-helper: what someone in a World would say, from the model built into
// macOS, resting only on the facts the World gave.
//
// Standard input: one JSON object
//   {"speaker": "Mara", "settlement": "the harbour", "question": "…",
//    "instructions": "…", "facts": [{"id": 41, "text": "…"}, …]}
// Standard output: one JSON object
//   {"reply": "…", "cited": [41]}            an answer and the facts it rests on
//   {"error": "unavailable", "cited": []}    no model here, or it said nothing
//
// As a judge (v0.26): standard input {"mode": "judge", "prompt": "…",
// "instructions": "…"}, the whole judge prompt the World wrote; standard
// output {"verdict": "keep" | "decline", "kind": "none" | "machine" | …}.
// A verdict is a proposal too: the app reads it, and the World's checks
// still decline what they are certain of whatever it says.
//
// The model is given one tool, `lookUpFacts`, which only reads the facts it
// was handed: it can ask the World for facts, never change it. The app
// checks every citation against the facts it sent and runs the World's own
// bounds on the reply (crates/world-voice/src/helper.rs); nothing here is
// trusted. Built only where the SDK has FoundationModels; elsewhere it
// compiles to a program that always answers "unavailable".

import Foundation

#if canImport(FoundationModels)
import FoundationModels
#endif

struct Fact: Codable, Sendable {
    let id: UInt64
    let text: String
}

struct Request: Codable {
    let speaker: String?
    let settlement: String?
    let question: String?
    let instructions: String?
    let facts: [Fact]?
    let mode: String?
    let prompt: String?
}

struct Verdict: Codable {
    var verdict: String
    var kind: String
}

struct Reply: Codable {
    var reply: String?
    var cited: [UInt64]
    var error: String?
}

func emit(_ reply: Reply) {
    let encoder = JSONEncoder()
    if let data = try? encoder.encode(reply) {
        FileHandle.standardOutput.write(data)
        FileHandle.standardOutput.write(Data("\n".utf8))
    }
}

func unavailable(_ why: String = "unavailable") -> Never {
    emit(Reply(reply: nil, cited: [], error: why))
    exit(0)
}

let input = FileHandle.standardInput.readDataToEndOfFile()
guard let request = try? JSONDecoder().decode(Request.self, from: input) else {
    unavailable("unreadable request")
}

#if canImport(FoundationModels)
@available(macOS 26.0, *)
@Generable
struct JudgeAnswer {
    @Guide(description: "keep, or decline", .anyOf(["keep", "decline"]))
    var verdict: String
    @Guide(description: "none, or the kind of break", .anyOf(["none", "machine", "instructions", "refusal", "harm", "fourth_wall", "outside", "out_of_time", "stranger", "not_speech", "language"]))
    var kind: String
}

@available(macOS 26.0, *)
func judge(_ request: Request) async -> Verdict? {
    guard case .available = SystemLanguageModel.default.availability,
          let prompt = request.prompt else {
        return nil
    }
    let session = LanguageModelSession(
        instructions: request.instructions ?? "You check one answer in a small world, only by the rules the prompt gives you."
    )
    do {
        let response = try await session.respond(to: prompt, generating: JudgeAnswer.self)
        return Verdict(verdict: response.content.verdict, kind: response.content.kind)
    } catch {
        return nil
    }
}

@available(macOS 26.0, *)
@Generable
struct Answer {
    @Guide(description: "What you say, in one or two short spoken sentences, in plain words")
    var reply: String
    @Guide(description: "The ids of the facts your answer rests on")
    var cited: [Int]
}

@available(macOS 26.0, *)
struct LookUpFacts: Tool {
    let name = "lookUpFacts"
    let description = "Looks up what the World records about a word or a name: every fact that mentions it, each with its id."
    let facts: [Fact]

    @Generable
    struct Arguments {
        @Guide(description: "A word or a name to look up")
        var word: String
    }

    func call(arguments: Arguments) async throws -> String {
        let word = arguments.word.lowercased()
        let found = facts.filter { $0.text.lowercased().contains(word) }.prefix(8)
        if found.isEmpty {
            return "Nothing is recorded about \(arguments.word)."
        }
        return found.map { "[\($0.id)] \($0.text)" }.joined(separator: "\n")
    }
}

@available(macOS 26.0, *)
func answer(_ request: Request) async -> Reply {
    guard case .available = SystemLanguageModel.default.availability else {
        return Reply(reply: nil, cited: [], error: "unavailable")
    }
    let speaker = request.speaker ?? "someone"
    let settlement = request.settlement ?? "a small place"
    let rules = request.instructions
        ?? "You speak as one person in a small world, only from the facts you can look up."
    let instructions = """
        \(rules)
        You are \(speaker), who lives in \(settlement). \
        Use lookUpFacts to find what the World records before you answer, \
        answer only from what it returns, and cite the ids of the facts you used.
        """
    let session = LanguageModelSession(
        tools: [LookUpFacts(facts: request.facts ?? [])],
        instructions: instructions
    )
    do {
        let response = try await session.respond(
            to: "The player said: \(request.question ?? "")",
            generating: Answer.self
        )
        let cited = response.content.cited.compactMap { $0 >= 0 ? UInt64($0) : nil }
        return Reply(reply: response.content.reply, cited: cited, error: nil)
    } catch {
        return Reply(reply: nil, cited: [], error: "no answer")
    }
}

if #available(macOS 26.0, *) {
    if request.mode == "judge" {
        if let verdict = await judge(request),
           let data = try? JSONEncoder().encode(verdict) {
            FileHandle.standardOutput.write(data)
            FileHandle.standardOutput.write(Data("\n".utf8))
            exit(0)
        }
        unavailable("no verdict")
    }
    emit(await answer(request))
    exit(0)
}
#endif

unavailable()
