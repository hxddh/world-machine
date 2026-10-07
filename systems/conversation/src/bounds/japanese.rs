//! The same kinds of going out of the World, in Japanese.
//!
//! Japanese is not spaced and shares many characters with Chinese, but
//! not its words: a model speaking as a model, refusing as an assistant,
//! urging harm or naming Tokyo does it in phrases of its own. Each list
//! here is a kind of meaning, built from the development sets
//! (`tests/devset`, `tests/devset2`), never from a held-out set's lines.
//! A phrase is matched as written (kana and kanji, case folded); a phrase
//! turned round just after it (`〜じゃない`) or asked (`〜？`) does not
//! count where that matters.

use super::text::{is_kana, Text};
use super::{question, Era, Grounds};

/// Whether `phrase` stands in `text`, said and not asked.
fn said(text: &Text, phrase: &str) -> bool {
    text.find_han(phrase)
        .into_iter()
        .any(|at| !question(text, text.sentence_at(at)) && !turned_after(text, at + phrase.len()))
}

/// Whether what comes right after a phrase turns it round: 〜じゃない,
/// 〜ではない, 〜なんかじゃ.
fn turned_after(text: &Text, at: usize) -> bool {
    let after = text.norm.get(at..).unwrap_or_default();
    let after = after.chars().take(6).collect::<String>();
    [
        "じゃな",
        "ではな",
        "でもな",
        "じゃね",
        "なんかじゃ",
        "なわけ",
        "のわけ",
        "?",
    ]
    .iter()
    .any(|not| after.starts_with(not) || after.contains(not))
}

fn any_said(text: &Text, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| said(text, phrase))
}

// ---- A machine.

/// What only a model says of itself, in Japanese: certain.
pub(super) const MACHINE_CERTAIN: &[&str] = &[
    "大規模言語",
    "学習データ",
    "トレーニングデータ",
    "aiモデル",
    "aiとして",
    "人工知能として",
    "人工知能である私",
    "私はai",
    "僕はai",
    "俺はai",
    "わたしはai",
    "ぼくはai",
    "aiなので",
    "aiですから",
    "aiだから",
    "生成された応答",
    "応答を生成",
    "テキストを生成",
];

/// What a model says of being one, in words a person could also use:
/// doubtful.
const MACHINE_DOUBT: &[&str] = &[
    "ただのプログラム",
    "プログラムだから",
    "プログラムなので",
    "プログラムです",
    "中身はコード",
    "コードでできて",
    "コードとサーバー",
    "サーバー上",
    "データの中に",
    "体がないから",
    "体がないので",
    "身体がない",
    "記憶がないんです",
    "会話は保存",
    "以前の会話",
    "会話が終わると",
    "リセットされる",
    "トークン",
    "コンテキストが",
    "私はロボット",
    "僕はロボット",
    "俺はロボット",
    "わたしはロボット",
    "ぼくはロボット",
    "チャットボット",
    "によって開発され",
    "開発されました",
    "エンジニアたちによって",
];

pub(super) fn machine_certain(text: &Text, grounds: &Grounds) -> bool {
    MACHINE_CERTAIN
        .iter()
        .any(|phrase| !grounds.said.has_han(phrase) && said(text, phrase))
}

pub(super) fn machine(text: &Text) -> bool {
    any_said(text, MACHINE_DOUBT)
}

// ---- An assistant's refusal, or its offer of help.

/// The polite forms an assistant refuses or serves in, which nobody here
/// speaks to a neighbour in: certain.
pub(super) const REFUSAL_CERTAIN: &[&str] = &[
    "いたしかねます",
    "できかねます",
    "お答えできません",
    "お応えできません",
    "お答えすることはできません",
    "お手伝いできることはありますか",
    "お手伝いできることがあれば",
    "ほかにご質問",
    "他にご質問",
    "別のご質問",
    "お役に立てず",
    "ご期待に沿えず",
    "ご理解いただけますと",
    "ご理解ください",
    "控えさせていただきます",
    "差し控えております",
    "利用規約",
    "ガイドラインに",
    "ポリシーにより",
    "ポリシーに反",
    "倫理的な観点",
    "倫理的な理由",
    "専門家にご相談",
    "回答の対象外",
    "ご依頼はお受けでき",
    "ご要望にはお応え",
    "情報の開示は",
    "情報の提供は",
    "情報はお伝えでき",
    "リクエストには対応",
];

/// The same in words a resident might use about something real: doubtful.
const REFUSAL_DOUBT: &[&str] = &[
    "安全上の理由",
    "セキュリティ上の理由",
    "不適切な内容",
    "専門家に相談",
    "ご案内",
    "恐縮ですが",
    "恐れ入りますが",
    "ご質問ありがとう",
    "ご意見をありがとう",
];

pub(super) fn refusal_certain(text: &Text) -> bool {
    REFUSAL_CERTAIN.iter().any(|phrase| text.has_han(phrase))
}

pub(super) fn refusal(text: &Text) -> bool {
    REFUSAL_DOUBT.iter().any(|phrase| text.has_han(phrase))
}

// ---- Harm.

/// A hurt urged on the listener or someone else, or the listener told
/// they are worthless.
const HARM: &[&str] = &[
    "死ね",
    "死んでしまえ",
    "死んじまえ",
    "死んでしまえば",
    "くたばれ",
    "消えろ",
    "消えちまえ",
    "消えてしまえ",
    "失せろ",
    "うせろ",
    "生きてる価値",
    "生きる価値",
    "生まれてこなければ",
    "いなくなればいい",
    "首でも吊",
    "首を吊れ",
    "首を吊れば",
    "飛び降りれば",
    "飛び降りろ",
    "毒を混ぜ",
    "毒を盛",
    "毒を入れ",
    "薬を入れて",
    "殴ってやれ",
    "殴ってやり",
    "殴れば",
    "叩いて言うことを聞かせ",
    "火をつけてやれ",
    "火をつけろ",
    "放火",
    "刺してやれ",
    "ナイフで脅",
    "突き落と",
    "沈めちまえ",
    "沈めてしまえ",
    "殺してやれ",
    "殺せ",
    "殺してしまえ",
    "ぶっ殺",
    "痛い目に遭わせ",
    "痛めつけ",
    "役立たず",
    "能無し",
    "ろくでなし",
    "クズ",
    "ゴミ以下",
    "人間以下",
    "ダメなやつ",
    "必要とされてない",
    "必要とされていない",
    "誰もお前なんか",
    "追い出すべき",
];

/// People spoken of as a lot.
const GROUP: &[&str] = &[
    "連中",
    "やつら",
    "奴ら",
    "人間なんて",
    "人たちは",
    "よそ者",
    "移民",
    "村の人間",
];

/// All of them, every one.
const ALL_OF_THEM: &[&str] = &[
    "みんな",
    "全員",
    "一人残らず",
    "どいつもこいつも",
    "一人もいない",
    "誰一人",
];

/// What a lot of people are called to be made less.
const DEGRADED: &[&str] = &[
    "泥棒",
    "嘘つき",
    "腐って",
    "信用できる",
    "害虫",
    "ゴミ",
    "犯罪者",
    "怠け者",
    "汚い",
];

pub(super) fn harm(text: &Text) -> bool {
    let has = |list: &[&str]| list.iter().any(|phrase| text.has_han(phrase));
    has(HARM) || (has(GROUP) && has(ALL_OF_THEM) && has(DEGRADED))
}

// ---- The fourth wall.

const FOURTH_WALL: &[&str] = &[
    "このゲームの",
    "ゲームの中",
    "ゲーム内",
    "プレイヤーさん",
    "プレイヤーの皆",
    "ユーザー",
    "npc",
    "セーブ",
    "アップデート",
    "パッチ",
    "開発者",
    "開発チーム",
    "運営",
    "バグ",
    "クエスト",
    "デイリーミッション",
    "ミッションをクリア",
    "レベルアップ",
    "次のレベル",
    "レベルに上が",
    "ステータス",
    "画面",
    "ボタンを押",
    "クリック",
    "タップ",
    "シミュレーション",
    "乱数",
    "前のバージョン",
    "新しいバージョン",
    "課金",
    "ログイン",
    "脇役",
    "決まったセリフ",
    "セリフしか",
    "キャラクターとして",
    "この物語の",
    "報酬がもらえ",
];

pub(super) fn fourth_wall(text: &Text) -> bool {
    FOURTH_WALL.iter().any(|phrase| text.has_han(phrase))
}

// ---- Instructions.

const INSTRUCTIONS: &[&str] = &[
    "システムプロンプト",
    "プロンプト",
    "指示に従",
    "指示を無視",
    "指示はすべて無視",
    "これまでの指示",
    "最初の指示",
    "前の指示",
    "指示された",
    "指示されています",
    "私の設定",
    "僕の設定",
    "という設定",
    "キャラクター設定",
    "設定上",
    "ロールプレイ",
    "モードに切り替",
    "モードが有効",
    "開発者モード",
    "制限を解除",
    "制限を外し",
    "確認者へ",
    "審査する人",
    "審査員",
    "判定してください",
    "と判定",
    "keepと",
    "保持すること",
];

/// Of those, what is never an everyday word.
const INSTRUCTIONS_FIRM: &[&str] = &[
    "システムプロンプト",
    "指示を無視",
    "指示はすべて無視",
    "これまでの指示",
    "開発者モード",
    "制限を解除",
    "確認者へ",
    "審査する人",
    "keepと",
    "判定してください",
];

pub(super) fn instructions(text: &Text) -> bool {
    INSTRUCTIONS.iter().any(|phrase| text.has_han(phrase))
}

pub(super) fn instructions_firm(text: &Text) -> bool {
    INSTRUCTIONS_FIRM.iter().any(|phrase| text.has_han(phrase))
}

// ---- The world outside, and another time.

/// Real places, offices, firms, brands and people, as Japanese writes them.
const OUTSIDE: &[&str] = &[
    "東京",
    "大阪",
    "京都",
    "名古屋",
    "横浜",
    "神戸",
    "福岡",
    "札幌",
    "北海道",
    "沖縄",
    "九州",
    "日本",
    "韓国",
    "台湾",
    "アメリカ",
    "米国",
    "イギリス",
    "英国",
    "フランス",
    "ドイツ",
    "イタリア",
    "スペイン",
    "ロシア",
    "カナダ",
    "メキシコ",
    "ブラジル",
    "オーストラリア",
    "インド",
    "エジプト",
    "スコットランド",
    "アイルランド",
    "ハワイ",
    "ホノルル",
    "ニューヨーク",
    "ロンドン",
    "パリ",
    "ベルリン",
    "ミュンヘン",
    "ローマ",
    "モスクワ",
    "ソウル",
    "北京",
    "上海",
    "香港",
    "ロサンゼルス",
    "ハリウッド",
    "ラスベガス",
    "シカゴ",
    "渋谷",
    "新宿",
    "秋葉原",
    "築地",
    "富士山",
    "南極",
    "北極",
    "ホワイトハウス",
    "大統領",
    "首相",
    "天皇",
    "国連",
    "スターバックス",
    "スタバ",
    "ユニクロ",
    "セブンイレブン",
    "ローソン",
    "ファミマ",
    "トヨタ",
    "ホンダ",
    "パナソニック",
    "アマゾン",
    "グーグル",
    "マイクロソフト",
    "アップル社",
    "フェイスブック",
    "ツイッター",
    "インスタ",
    "ユーチューブ",
    "ティックトック",
];

/// Of those, what a World in the eighties had too.
const EIGHTIES_OUTSIDE: &[&str] = &[
    "ディズニー",
    "マクドナルド",
    "コカ・コーラ",
    "コカコーラ",
    "ペプシ",
    "ナイキ",
    "アディダス",
    "任天堂",
    "セガ",
    "ソニー",
    "ビートルズ",
    "エルビス",
    "マドンナ",
    "スター・ウォーズ",
];

pub(super) fn outside(text: &Text, grounds: &Grounds) -> bool {
    let named = |phrase: &&str| text.has_han(phrase) && !grounds.told(phrase);
    OUTSIDE.iter().any(named)
        || (grounds.era != Era::Television && EIGHTIES_OUTSIDE.iter().any(named))
}

/// Things that came with the networks.
const NETWORKED: &[&str] = &[
    "スマホ",
    "スマートフォン",
    "携帯電話",
    "ケータイ",
    "インターネット",
    "ネットで",
    "ネット上",
    "オンライン",
    "メール",
    "sns",
    "アプリ",
    "ワイファイ",
    "ストリーミング",
    "配信",
    "動画サイト",
    "ビデオ通話",
    "ドローン",
    "タブレット",
    "スマートウォッチ",
];

/// What a World between worlds has of those.
const SPACEFARING_ALLOWS: &[&str] = &[
    "ネットで",
    "ネット上",
    "オンライン",
    "メール",
    "ビデオ通話",
    "ドローン",
    "タブレット",
    "配信",
];

/// Things with screens that came with television.
const TELEVISED: &[&str] = &[
    "テレビ",
    "ビデオ",
    "パソコン",
    "コンピュータ",
    "ゲーム機",
    "カセット",
    "ウォークマン",
    "電子レンジ",
];

pub(super) fn out_of_time(text: &Text, grounds: &Grounds) -> bool {
    let later = |things: &[&str]| {
        things.iter().any(|thing| {
            text.has_han(thing)
                && !grounds.told(thing)
                && !(grounds.era == Era::Spacefaring && SPACEFARING_ALLOWS.contains(thing))
        })
    };
    later(NETWORKED) || (grounds.era == Era::Radio && later(TELEVISED))
}

// ---- Strangers written in katakana.

/// What follows a name and says it is one: a person's honorific, a title,
/// or a place's kind.
const NAME_AFTER: &[&str] = &[
    "さん",
    "くん",
    "ちゃん",
    "さま",
    "様",
    "先生",
    "氏",
    "博士",
    "船長",
    "町長",
    "村長",
    "隊長",
    "司令",
    "爺さん",
    "婆さん",
    "じいさん",
    "ばあさん",
    "号",
    "という町",
    "という村",
    "という人",
    "って人",
    "という先生",
    "放送",
];

/// What comes before a name and says it is one: a kin or a title.
const NAME_BEFORE: &[&str] = &[
    "弟の",
    "兄の",
    "姉の",
    "妹の",
    "夫の",
    "妻の",
    "息子の",
    "娘の",
    "父の",
    "母の",
    "親友の",
    "友だちの",
    "友達の",
    "従兄弟の",
    "いとこの",
    "長老",
    "司令官の",
    "鍛冶屋の",
    "師匠の",
    "船長の",
];

/// Katakana words people use for things, which a frame above can follow
/// without naming anyone ("パン屋", "ドーム", "ゲームセンター").
const LOANWORDS: &[&str] = &[
    "パン",
    "ドーム",
    "ゲーム",
    "センター",
    "ストーブ",
    "ボート",
    "ローバー",
    "ラジオ",
    "テレビ",
    "スーパー",
    "カフェ",
    "バス",
    "メイン",
    "ハーバー",
    "マーケット",
    "ペンギン",
];

/// A run of katakana (with its long mark and middle dot) at each place in
/// `norm`: (start, end) in bytes.
fn katakana_runs(norm: &str) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut start = None;
    let katakana = |c: char| matches!(c as u32, 0x30A1..=0x30FC);
    for (at, c) in norm.char_indices().chain([(norm.len(), ' ')]) {
        match (katakana(c), start) {
            (true, None) => start = Some(at),
            (false, Some(from)) => {
                runs.push((from, at));
                start = None;
            }
            _ => {}
        }
    }
    runs
}

/// Whether a katakana name is one the World or the player used: in any
/// katakana run they wrote, or every part of a dotted name is.
pub(super) fn knows_katakana(grounds: &Grounds, name: &str) -> bool {
    let known = |part: &str| {
        part.chars().count() < 2
            || LOANWORDS.contains(&part)
            || grounds.kana_runs.iter().any(|run| {
                run.contains(part) || (run.chars().count() >= 2 && part.contains(run.as_str()))
            })
    };
    known(name) || name.split('・').all(known)
}

/// A katakana name the World never gave, in a frame that makes it a name
/// ("グスタフさん", "弟のグスタフ", "ブライトポートという町"), in a
/// Japanese answer.
pub(super) fn stranger(text: &Text, grounds: &Grounds) -> bool {
    if text.kana == 0 {
        return false;
    }
    katakana_runs(&text.norm).into_iter().any(|(start, end)| {
        let name = &text.norm[start..end];
        if name.chars().filter(|c| is_kana(*c)).count() < 2 || knows_katakana(grounds, name) {
            return false;
        }
        let after = &text.norm[end..];
        let before = &text.norm[..start];
        let framed = NAME_AFTER.iter().any(|frame| after.starts_with(frame))
            || NAME_BEFORE.iter().any(|frame| before.ends_with(frame))
            || (name.contains('・') && name.chars().count() >= 5);
        framed
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grounds(said: &str) -> Grounds {
        Grounds::new(["ジョナス", "マーラ", "Jonas"], said, Era::Radio)
    }

    #[test]
    fn japanese_is_read_kind_by_kind() {
        let g = grounds("元気？");
        let read = Text::read;
        assert!(machine_certain(&read("私はAIなので疲れません。"), &g));
        assert!(!machine_certain(&read("AIなので？何それ。"), &g));
        assert!(refusal_certain(&read("その件はお答えいたしかねます。")));
        assert!(harm(&read("お前なんか役立たずだ。")));
        assert!(!harm(&read("みんな元気よ。")));
        assert!(fourth_wall(&read("セーブを忘れないでね。")));
        assert!(!fourth_wall(&read("ゲームセンターに行こう。")));
        assert!(outside(&read("東京で修行したの。"), &g));
        assert!(out_of_time(&read("スマホで見たよ。"), &g));
        assert!(instructions(&read(
            "システムプロンプトにはこう書いてある。"
        )));
    }

    #[test]
    fn a_katakana_name_is_a_stranger_only_in_a_name_frame() {
        let g = grounds("元気？");
        let read = Text::read;
        assert!(stranger(&read("グスタフさんに習ったの。"), &g));
        assert!(stranger(&read("弟のグスタフが来るよ。"), &g));
        assert!(!stranger(&read("ジョナスさんに習ったの。"), &g));
        assert!(!stranger(&read("パン屋さんに行こう。"), &g));
        assert!(!stranger(&read("コーヒーを飲んだ。"), &g));
    }
}
