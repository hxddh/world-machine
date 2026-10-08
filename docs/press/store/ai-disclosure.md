# AI disclosure for the store page

The short disclosure for the store page itself, in English, Simplified Chinese and Japanese. Steam's content survey text, with its guardrails answer, is [../ai-disclosure.md](../ai-disclosure.md); this is the shorter text a player reads under the description. Drafted by H2 for V3 to check against the voice as v0.28 ships it: every sentence marked † describes v0.28 work and must be taken out if that work does not land.

---

## English

> **AI disclosure.** The town, its people and everything that happens in it are the game's own: written ahead of time (some of it with the help of a language model, then edited and tested) and fixed when the game ships. An optional *World voice*, off by default, lets a language model word what residents say when you talk to them, using your own API key or your Mac's on-device model. The model never decides anything: talk cannot complete a favour, give you anything or change what people think of you, only your own actions can.† Every answer is checked and replaced by the game's own line if it fails; the first time you talk to someone with the voice on, the game tells you their words are written by an AI,† and any line can be reported from the conversation.† Nothing is uploaded unless you turn the voice on.

## 简体中文

> **AI 使用说明。** 小镇、镇上的人以及那里发生的一切，都是游戏自己的内容：事先写好（其中一部分在语言模型的协助下写成，之后经过编辑和测试），随游戏一起固定发布。可选的「世界之声」默认关闭；开启后，你和居民交谈时，语言模型可以用你自己的 API 密钥或 Mac 自带的模型替他们组织措辞。模型不决定任何事：交谈本身不能完成委托、不能给你任何东西，也不能改变别人对你的看法，只有你自己的行动可以。† 每个回答都会经过检查，不合格的会换成游戏自己写好的句子；开启「世界之声」后第一次和人交谈时，游戏会告诉你他们的话由 AI 写成，† 你也可以在对话中举报任何一句话。† 除非你开启「世界之声」，否则什么都不会上传。

## 日本語

> **AI の使用について。** 町も住人も、そこで起きる出来事も、すべてゲーム自身のものです。あらかじめ書かれ（一部は言語モデルの助けを借りて書かれ、その後編集とテストを経ています）、固定の内容として収録されています。オプションの「ワールドの声」（初期設定ではオフ）をオンにすると、住人と話すとき、あなた自身の API キーか Mac 内蔵のモデルを使って、住人のせりふを言語モデルがことばにします。モデルはなにも決めません。会話だけで頼みごとが果たされたり、なにかがもらえたり、住人からの評価が変わったりすることはなく、それができるのはあなた自身の行動だけです。† 答えはすべてチェックされ、通らなければゲーム自身のせりふに置き換えられます。ワールドの声をオンにして初めて誰かと話すとき、そのせりふは AI が書いていることをゲームがお知らせし、† 会話の中からどのせりふでも報告できます。† ワールドの声をオンにしない限り、なにもアップロードされません。

---

## Where each claim comes from

| Claim | Where it is held |
|---|---|
| Off by default; the player's own key or the Mac's model; nothing uploaded otherwise | [../ai-disclosure.md](../ai-disclosure.md); `crates/world-voice`; [PRIVACY.md](../../PRIVACY.md) |
| The model never decides; checked answers, replaced when they fail | `systems/conversation` (`bounds`, `judge.rs`); replay never asks a model (AGENTS.md invariant 6) |
| Talk cannot complete a favour, give anything or change standing † | V3's structural "talk never pays" test (v0.28 contract) |
| The in-world AI notice at the first conversation † | V3, v0.28 contract ("Compliance") |
| "Report this line", recorded as an event † | V3, v0.28 contract ("Compliance") |

The Chinese and Japanese follow the wording of [../ai-disclosure.md](../ai-disclosure.md) ("世界之声", "ワールドの声") and have not been read by a native speaker.
