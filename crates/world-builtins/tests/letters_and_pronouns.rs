//! What a letter says the player did, and the lines that speak of someone
//! as he, she or they, read in each language the app installs: English as
//! it is written, and Chinese and Japanese through their catalogs, whole.

fn catalog(texts: &[&str]) -> world_i18n::Catalog {
    texts
        .iter()
        .fold(world_i18n::Catalog::default(), |mut all, text| {
            all.extend(text);
            all
        })
}

/// Each English line, and what Chinese and Japanese show for it.
const LINES: &[(&str, &str, &str)] = &[
    // A letter cites what the player did, by hand or in an answer.
    (
        "All because you built a bench by the quay.",
        "这都是因为你在码头边建了一张长椅。",
        "それも、波止場のそばにベンチを建てたからなんだよ。",
    ),
    (
        "And to think it started when you built a bench by the quay.",
        "想想看，这一切都始于你在码头边建了一张长椅。",
        "思えば、きっかけは波止場のそばにベンチを建てたことだったね。",
    ),
    (
        "All because you said “Make peace”.",
        "这都是因为你说了“劝和”。",
        "それも、あなたが「仲を取り持つ」と言ってくれたからなんだよ。",
    ),
    // He, she and they, each in its own words.
    (
        "Could I learn from him?",
        "我能跟他学吗？",
        "教えてもらえないかな？",
    ),
    (
        "Could I learn from her?",
        "我能跟她学吗？",
        "教えてもらえないかな？",
    ),
    (
        "Could I learn from them?",
        "我能跟他们学吗？",
        "教えてもらえないかな？",
    ),
    (
        "Leo kept to himself",
        "利奥独自待着",
        "レオはひとりで過ごした",
    ),
    (
        "Mara kept to herself",
        "玛拉独自待着",
        "マーラはひとりで過ごした",
    ),
    (
        "Mara confided she was sweet on Leo",
        "玛拉悄悄说自己喜欢上了利奥",
        "マーラはレオに想いを寄せていると打ち明けた",
    ),
    (
        "Leo told you a secret, and you told him to share it",
        "利奥告诉了你一个秘密，你让他说出去",
        "レオは秘密を打ち明け、あなたはみんなに話してみたらと勧めた",
    ),
    (
        "Mara told you a secret, and you told her to share it",
        "玛拉告诉了你一个秘密，你让她说出去",
        "マーラは秘密を打ち明け、あなたはみんなに話してみたらと勧めた",
    ),
    (
        "She makes every day easier.",
        "有她在，每天都轻松些。",
        "あの人がいると毎日が楽になるんだ。",
    ),
];

#[test]
fn a_letters_citation_and_each_pronoun_read_whole_in_every_language() {
    let zh = catalog(&world_builtins::ZH_HANS);
    let ja = catalog(&world_builtins::JA);
    let mut wrong = Vec::new();
    for (english, chinese, japanese) in LINES {
        for (catalog, want) in [(&zh, chinese), (&ja, japanese)] {
            let shown = catalog.translate(english);
            if shown.as_deref() != Some(*want) {
                wrong.push(format!("{english} => {shown:?}, not {want}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
