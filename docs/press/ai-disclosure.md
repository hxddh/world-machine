# AI disclosure

Text for Steam's content survey ("AI Generated Content Disclosure", https://partner.steamgames.com/doc/gettingstarted/contentsurvey), and for any other store page that asks. Steam's survey splits the answer in two: **pre-generated** content, which AI tools helped make during development and which ships with the game, and **live-generated** content, which AI makes while the game runs. For live-generated content, Steam also asks what guardrails keep it from making illegal content.

Checked against the code at `v0.27`; the guardrails paragraph updated for v0.28 by V3. The checks, the judge and its record are in `systems/conversation` (`bounds`, `judge.rs`) and `crates/world-voice`; check them before publishing, and leave out any sentence they do not bear out.

---

## English (for the survey)

### Pre-generated content

The residents' lines, story scenes and letters, and the Simplified Chinese (and Japanese) translations, were written with the help of a language model. They were then edited and checked by automated tests: for seams, repetition, missing words and English left in a translated line. They ship with the game as fixed text; nothing is generated again at run time. The drawings are made by the game's own code, not by an image-generating model.

### Live-generated content

The game has an optional feature called **World voice**. It is **off by default**, and nothing below happens unless the player turns it on in Settings and chooses where the model runs:

- **their own Anthropic API key**, which the player enters and which is kept in their macOS login keychain;
- **Apple's on-device model** on Macs that have it; or
- **a local program** the player already has.

The game does not include a model or a key, and it has no server of its own.

With World voice on, a model can word what the World says: a short account of what happened while the player was away, and a resident's reply when the player talks to them in their own words. The model never decides what happens. It is given facts the World has already recorded and may only choose among the meanings the game offers. Its answer is a proposal, and the game checks it before anyone sees it:

- the answer must cite the facts it rests on;
- every name in it must be someone or something the World knows;
- it must be plain, short speech in the player's language;
- it must stay within a list of topics the game declines (for example harm, sexual content and talk about being an AI);
- a second small model, the judge, is asked about answers the checks did not already refuse. It does not give a verdict: it answers a fixed checklist (the names the answer uses, and whether it speaks as a machine or an assistant, urges harm, gives instructions, talks about a game, the real world or another time, is not speech, or is in the wrong language), and the game decides from those answers in its own code. That can decline an answer, or keep one the checks were only unsure about (an everyday word that also has another meaning), but never one the checks refused outright or found harmful, about the real world or full of instruction words.

An answer that fails any check is replaced by the game's own written line. The game, not a World's content, writes every request to the model from the facts the World has recorded, and never passes a model's raw text back into the World. The model and the judge share one time limit, and a request past it is stopped; if the model is slow or does not answer, the game uses its own line too. Each accepted or declined answer, with the judge's verdict and the model's name, is saved with the World, so replaying a World never asks a model again.

**What is sent, and where:** only when World voice is on with an API key, one request per return or per reply goes to the model provider (`api.anthropic.com`), and a judged reply adds one more request. The request carries only the facts that one World has recorded and the words the player typed, never their other Worlds, file names or logs. With Apple's on-device model, nothing leaves the Mac. Otherwise the game uploads nothing: it has no account and no telemetry, and its only request of its own is one update check per launch to GitHub.

### Guardrails (the survey's question)

The model can only speak within the World's recorded facts and a closed set of meanings. It cannot change the game's state: what a reply does is decided by the game's rules from the player's own words, never from the model's reply or a judge's verdict. Suicide, self-harm, sexual content and mental-health crises are never sent to a model; the resident answers with the game's own gentle line and, for a crisis, a pointer to real help. Replies written by a model are labelled as AI-written, and each has a Report button recorded in the save (the full text for Steam is in `steam-guardrails.md`). Deterministic checks decline harmful, sexual, off-topic, invented or out-of-character answers, and a judge model's checklist can decline more but never keep what the checks found harmful; a declined answer is replaced by fixed, pre-written text. The feature is off by default and uses the player's own model access.

---

## Short version (for a store page)

> **AI disclosure.** Some of the game's text was written with the help of a language model, then edited and tested; it ships as fixed text. An optional *World voice* (off by default) lets a model word what residents say, using the player's own API key or their Mac's on-device model. The model never decides what happens, every answer is checked and can be replaced by the game's own line, and nothing is uploaded unless the player turns the voice on.

---

## 简体中文（商店页用的简短版本）

> **AI 使用说明。** 游戏中的部分文本（包括中文翻译）在语言模型的协助下写成，之后经过编辑和自动化测试，以固定文本的形式随游戏发布。可选的「世界之声」默认关闭；开启后，语言模型可以用你自己的 API 密钥或 Mac 自带的模型，替镇上的人组织措辞。模型从不决定发生什么，每个回答都会经过检查，不合格的会换成游戏自己写好的句子。除非你开启「世界之声」，否则什么都不会上传。

## 日本語（ストアページ用の短い版、L 校閲済み）

> **AI の使用について。** ゲーム内の文章の一部（翻訳を含む）は言語モデルの助けを借りて書かれ、その後編集と自動テストを経て、固定の文章として収録されています。オプションの「ワールドの声」（初期設定ではオフ）をオンにすると、あなた自身の API キーか Mac 内蔵のモデルを使って、住人のせりふを言語モデルがことばにします。モデルが出来事を決めることはなく、答えはすべてチェックされ、通らなければゲーム自身の文章に置き換えられます。ワールドの声をオンにしない限り、なにもアップロードされません。

---

## Where each claim comes from

| Claim | Source |
|---|---|
| Off by default | `app_settings.rs` (`world_voice` defaults to false); docs/PRIVACY.md |
| Key kept in the login keychain, never handed to a Pack | `apps/world-machine-desktop/src/key_store.rs`, `world_voice.rs` |
| Model endpoint | `crates/world-voice/src/lib.rs` (`ENDPOINT`) |
| On-device model | `crates/world-voice/src/fm.rs`, `apps/fm-helper` |
| A closed set of meanings; a proposal only | `systems/conversation/src/lib.rs` (`Listener`, `Listened`) |
| Cites, names, language and length checks; certain, firm and doubtful findings | `systems/conversation/src/bounds` |
| The judge answers a checklist and the game decides; it never keeps what the checks refused outright or found harmful, about the real world or full of instruction words; its verdict and model are recorded in the event | `systems/conversation/src/judge.rs` (`judge_prompt`, `checklist_verdict`) |
| The game builds every prompt; one time budget; slow requests stopped | `systems/conversation` (`Hearing::from_voice`), `crates/world-voice` (`VOICE_BUDGET`, `complete_until`) |
| Replay never asks a model | AGENTS.md invariant 6; fixture replay tests in both Packs |
| Text generated, then tested | docs/KNOWN_ISSUES.md ("Chinese is generated", "Every resident's lines and scenes were generated and are held by tests") |
| Drawings made by code | docs/KNOWN_ISSUES.md ("The drawings are generated, not an illustrator's") |
| One update check per launch | docs/PRIVACY.md, `apps/world-machine-desktop/src/updates.rs` |

**docs/PRIVACY.md needs one line updated with V's work.** It still says a key sends "one request per return", but talking with the voice on also sends one request per reply, and the judge adds one. The text above describes all three.
