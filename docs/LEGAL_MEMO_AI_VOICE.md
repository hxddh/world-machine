# Memo for counsel: the optional AI voice, and companion-chatbot law

**Prepared for one review by counsel before the demo is public. Not legal advice.** Written for v0.29 on 2026-10-09. Every fact about the law below comes from the research in [REVIEW_v0.28.md](REVIEW_v0.28.md) ("What the research found"), which was gathered from web-search summaries on 2026-10-08 without opening the pages. **All of it is unverified** and must be read against the statutes themselves.

## What the product does (verifiable in this repository)

- **World voice is optional and off by default.** With it on, a language model (the player's own API key, or the Mac's on-device model) words a resident's reply. The model never decides what happens: what talk does is decided by the game's rules from the player's own words (`systems/conversation`, `never_pays.rs`).
- **Mental health, self-harm, suicide and sexual content are never sent to a model.** The player's words on these topics are answered with the game's own fixed line, which for a crisis points to real help (findahelpline.com) (`crates/world-voice-prompt/src/care.rs`). A model's answer that touches them is declined for certain and replaced by the game's own line.
- **Answers off the game's world are declined**: the real world's brands, celebrities, titles and money, other times, machine and assistant talk, game talk, other languages (`bounds`, and a judge model's checklist).
- **Replies written by a model are labelled as AI-written** in the game, and each has a Report button. Saved exchanges let residents "recall" the player for a month.
- A test pins this behaviour over a fixed set of prompts in English, Chinese and Japanese: `the_california_exemption_holds` in `systems/conversation/src/never_pays.rs` (see below).

## The questions for counsel

**1. California SB 243, and SB 1119 ("Adam's Law").** *(unverified)* SB 1119 was signed on 10 September 2026, takes effect on 1 January 2027, with most duties from 1 July 2027. It keeps SB 243's exemption for characters in a video game only while they cannot discuss mental health, self-harm, sexual content, or things unrelated to the game. Because saved exchanges make a resident "recall" the player, the game may otherwise look like a companion chatbot; the exemption is then the whole defence.
- *Ask:* Does the exemption apply to an optional, off-by-default feature in which the topics are refused before any model sees them, and off-world answers are replaced? Does "cannot discuss" mean the system must refuse (as built), or that a character must never so much as acknowledge the topic (the fixed line acknowledges and redirects, and for a crisis gives a helpline)? Is a one-month memory "recall" in the statute's sense? Are the duties that apply outside the exemption (disclosure, crisis protocols, reporting) ones the game could meet if the exemption failed?

**2. EU AI Act, Article 50(1).** *(unverified)* It was not delayed. It requires telling people they are interacting with an AI system unless that is obvious. The review recommends keeping the in-game "Written by AI" label rather than relying on the "obviously AI" exception.
- *Ask:* Is the per-reply label, together with the store-page disclosure (`docs/press/ai-disclosure.md`) and the Settings switch, sufficient? Does Article 50 reach the player's own API key, when the game runs no model or server of its own? Is the on-device model treated differently?

**3. China.** *(unverified)* China's generative-AI rules reach services offered from abroad. The game ships a Chinese translation and may be sold on Steam to players in China.
- *Ask:* Is a feature that uses the player's own foreign API key, or an on-device model, a "service offered" to the public in China? If so, what follows (filing, labelling, content duties), and is it enough to disable World voice for the zh-Hans locale or for Chinese storefronts?

**4. Japan and Korea, for completeness.** *(unverified)* Japan has no binding duty. Korea's AI Act (January 2026) wants AI-generated text labelled, which the game already does.

## What the code guarantees, and what it does not

- **Guaranteed and tested:** the listed topics never reach a model in the three languages, for the fixed prompts in the test and the lists in `bounds`; a model's reply or judge's verdict never changes what talk does; the decision and the judge's verdict are recorded, so replay never asks a model.
- **Not guaranteed:** detection is by word lists, not understanding, so new phrasing of a sensitive topic can still reach the model, and then only the answer checks and the judge stand between it and the player. The judge (Claude Haiku 4.5 by default) has been measured only through recorded agent sessions, not through the app with a real key. On the last blind measurement (v0.28's set 6) 94.0% of out-of-world answers were declined; the bar was 95%. The Mac's on-device model is untested.

## Recommended before the demo is public

One review of this memo by counsel; until then, keep World voice off by default and opt-in, keep the AI label on every model-written reply, and lead the store page with the handmade town.
