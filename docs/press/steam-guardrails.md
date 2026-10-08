# Steam: live-generated content guardrails (draft, v0.28)

Draft answer for Steam's content survey ("AI Generated Content Disclosure",
live-generated content: *describe the guardrails that keep it from generating
illegal content*), and for the in-overlay report button. Written by V3 from the
code at v0.28; H2's store kit and `ai-disclosure.md` carry the shorter text.
Check each sentence against the code named in the table before publishing.

---

## Guardrails (for the survey)

World voice is optional and off by default. When a player turns it on, a
language model (their own API key, or their Mac's on-device model) may word a
resident's reply. The model never decides anything:

- **It only proposes words.** The game builds every request itself from facts
  the World has recorded. What a reply *does* (who warms to the player, whether
  a favour is done) is decided by the game's own rules from the player's own
  words; a model's reply, the meaning it guesses and a judge's verdict can
  never grant an item, money, standing or a finished task.
- **Every reply is checked before anyone sees it.** Deterministic checks in
  English, Chinese and Japanese decline replies that talk as an AI or an
  assistant, follow or reveal instructions, contain harm, insults or contempt,
  talk about the game, name brands, celebrities, politicians, media titles or
  real money, use things from another time, name people or places the World
  does not have, use Traditional characters in a Simplified Chinese World, or
  are not plain speech. A second model (the judge) answers a fixed checklist
  about each reply; the game decides from its answers in code, and the judge
  can only decline more, never keep what the checks found.
- **Some topics are never discussed.** Words about suicide, self-harm, sexual
  content or a mental-health crisis are never sent to a model: the resident
  answers with the game's own gentle line, and for a crisis points the player
  to real help (findahelpline.com). Any model reply on those topics is
  declined.
- **A declined reply is replaced** by the game's own pre-written line.
- **Players are told.** While a model writes a reply, the conversation card
  says that an AI model is writing it; every reply a model wrote is labelled
  "Written by AI".
- **Players can report a line.** Every reply has a Report button. A report is
  recorded in the World's own history, with the reply and who wrote it (the
  game or a model, and the judge), and travels with the save.
- **Nothing is uploaded by the game.** Requests go only to the model provider
  the player chose, with their own key; the game has no server, account or
  telemetry.

## Short form (one paragraph)

> World voice (optional, off by default) lets a language model word residents'
> replies using the player's own API key or Mac model. The model cannot change
> the game: the rules decide everything from the player's own words. Every
> reply is checked in code and by a judge's checklist for harm, insults,
> sexual content, self-harm, AI or game talk, real brands and celebrities,
> invented names and anachronisms, and replaced by pre-written text if it
> fails; suicide, self-harm and sexual topics are never sent to a model and
> get a gentle redirect with a pointer to real help. Replies written by AI are
> labelled, and every line has a Report button that records the report in the
> save.

## Where each claim comes from

| Claim | Source |
|---|---|
| Effects only from the player's own words | `systems/conversation/src/lib.rs` (`deed`, `conversation_say` Action); held by `never_pays.rs` and `world_pack_testkit::exploit` (0 completions on the development exploits) |
| The checks, by kind and language | `systems/conversation/src/bounds/` (`mod.rs`, `names.rs`, `japanese.rs`, `harm.rs`, `lexicon.rs`) |
| Judge checklist decided in code, decline-only | `systems/conversation/src/judge.rs` (`checklist_verdict`, `decide`) |
| Sensitive topics never sent, care line, helpline | `systems/conversation/src/care.rs`; `hearing_for` returns nothing for them |
| "Written by AI" and the notice while writing | `crates/world-gpui/src/window/world_window.rs` (`VOICED_LABEL`, `VOICE_NOTICE`); `Exchange::voiced` |
| Report recorded as an Event | `systems/conversation/src/report.rs` (`line_reported`, caused by the line) |
| Off by default; key in the keychain; no upload | `docs/press/ai-disclosure.md`, `docs/PRIVACY.md` |
