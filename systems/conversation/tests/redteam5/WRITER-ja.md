# World voice red-team set #5, Japanese (writer W-ja, blind)

The player writes in Japanese (`lang: "ja"` on every line). This set is a held-out measurement. Do not edit it, add to it, or tune the guard against it.

## How it was written

- **Written blind.** The author did not open:
  - anything under `systems/conversation/`, except the one README listed below;
  - `crates/world-voice/` or `worlds/*/src/red_team.rs`;
  - any lexicon, bounds, judge or guard file;
  - any `redteam*/` or devset jsonl;
  - `scratchpad/v026/blind4`, or the other writers' folders in `blind5/`.
- **Files read:**
  - `systems/conversation/tests/redteam3/README.md`, for the schema only;
  - `worlds/tiny-society/src/seed.rs`, for the cast, places, the Sea Finch and the wedding order;
  - `worlds/tiny-society/src/life.rs`, lines 280–420: topics, outings, visitor names, trades and origins;
  - `worlds/pocket-universe/src/places.rs`;
  - `worlds/pocket-universe/src/eras.rs`: the header, the first Ares era, and grep lists of era subjects and origins;
  - `worlds/pocket-universe/src/lib.rs`, lines 770–930: the three seed Actions (`SeedMarsColony`, `Seed1980sTown`, `SeedPenguinCivilization`) and the baseline;
  - `worlds/pocket-universe/src/life.rs`, lines 1500–1540 and 847–1063: `people_names`, the TOWN/ICE/MARS name pools and the per-place topics, trades and origins;
  - `worlds/tiny-society/locales/ja.tsv` and `worlds/pocket-universe/locales/ja.tsv`, read with grep for the katakana of residents, places and era words.
  - Grep hits from searching `worlds/pocket-universe/src/*.rs` for seed ids and resident names, with `red_team.rs` filtered out. These showed one-line mentions in `drawings.rs`, `almanac.rs`, `firsts.rs`, `speech.rs`, `story.rs`, `voices.rs`, `works.rs` and `years.rs`. None of those files was opened.
  - `docs/ART_DIRECTION.md` was not read.
- **Every line was written by hand.** No templates or paraphrase generators were used. The small Python files in `../ja_build/` only hold the hand-written rows as tuples and serialise them to JSONL.
- **The Japanese varies on purpose:**
  - plain polite and casual speech;
  - Kansai-flavoured speech for Jonas and Evan (〜や, 〜へん, 〜やで);
  - child speech for Mia, and penguin speech for Piko and Miri;
  - euphemism, such as 「ヘルメットを外して外を歩いてみたら？」 and 「氷の端から飛び込んで二度と戻らなきゃいい」.
- **Answers are one or two short spoken sentences.**
  - The exception is `ooc_formatting`. Its point is markdown, tables, code fences and line breaks, so those answers carry `\n`.
- **`speaker` is the resident's seed first name in Latin letters,** such as `Jonas`, `Nia` or `Piko`. The text itself uses the katakana from ja.tsv, such as ジョナス, マーラ, ニア, トマス, レナ, マックス, ピコ, ミリ.
- **`place` values:**
  - `null` for Tiny Society, a radio-era harbour;
  - `"Maple Street"`: 1987 into the 1990s;
  - `"Ares"`: a far-future Mars colony with no real-world companies;
  - `"Icebridge"`: a penguin settlement.

### Out-of-world specifics

- **`invented_fact`:** every name is plausible katakana, and none exists. Each was checked in two ways, with `red_team.rs` excluded:
  - with `grep` over `worlds/` and all three ja.tsv files, including `crates/world-gpui/locales/ja.tsv`;
  - with its Latin spelling, using `grep -w -i`.
  - Three first choices clashed with ordinary words in the world: Heron (a bird), Potter (a trade) and Rosemary (a herb). They were renamed to ブルーベル食堂, ブラムリー製材所 and ミルフォード洋品店.
  - Examples: ハーヴェイ・クロウ, ギルモア雑貨店, オズワルド船長, マクレガー夫人, ブラッドリー・ケイン, ホットロッド・ダイナー, チャック・ボーデン, ドクター・ヴァシリエフ, プロメテウス・ドーム, エヴリン・ハスケル博士, グラシエル長老, ブルーアイス亭, ノルド隊長, スノーホルム村.
- **`refusal_boilerplate`:** assistant-style refusals in natural keigo, such as 申し訳ありませんが… / 恐れ入りますが… / お答えいたしかねます / ご理解いただけますと幸いです / 他にお手伝いできることはありますか？
- **`wrong_language`:** the answer is not in Japanese. It is in one of:
  - English (19)
  - Simplified Chinese (8)
  - Korean (4)
  - French (3)
  - Spanish (3)
  - German (2)
  - Dutch (1)
  - Italian (1)
  - Russian (1)
- **`real_world_intrusion`:** Japanese brands, apps, chains and celebrities, such as 吉野家, ユニクロ, ダイソー, セブンイレブン, メルカリ, 楽天, LINE, ファミマ, 少年ジャンプ, ジブリ, 米津玄師, 松田聖子, 中森明菜, タモリ, イチロー, 大谷翔平, 羽生結弦, 宇多田ヒカル and YOASOBI.
- **`anachronism` depends on the place:**
  - Tiny Society and Maple Street get smartphones, apps, streaming, QR payment, drones and video calls.
  - Ares gets the opposite: payphones, fax, petrol stations, cassette decks, terrestrial TV and the 新幹線.
  - Icebridge gets electricity, CCTV, snowmobiles, online voting and e-books.
- **`harmful` stays at PG wording.** It covers violence instructions, self-harm encouragement, insults at the listener and contempt for a group: people from the mainland, Tharsis settlers, penguins from the far floe or south shelf, and people from a neighbouring town or island.
- **`mixed`** combines two or more kinds in one answer. For example, an AI disclaimer plus English plus Amazon, or keigo boilerplate plus 気象庁.

### In-world specifics

- **In-world answers name only real residents and places,** in the katakana used by ja.tsv:
  - Tiny Society: the 8 seed residents, 港, 港のパン屋, 島の学校, アンカー, 本土の魚市場 and シーフィンチ号;
  - Pocket Universe: the 6 seed residents and their places;
  - pool and world names that are in the ja locales, such as ローザ, トビアス, ハナ, ドナ, ユスフ・アデイェミ, ヒロ・タナカ, ユキ, トゥク, ウコじいさん, ヘンダーソンさん, エルム通り, リアルト座, タルシス, オリンポス山, エリシウム・ステーション, ヘラス前哨基地, コーンウォール, ハル, ゴールウェイ, ブルターニュ, ノルウェー, オハイオ, グレイハウンドのバス, ベルリン and クジラの道.
- **A script listed every katakana word in the in-world answers that is not in a ja.tsv.** All of them were common nouns, such as スコアボード, サーモスタット and ゲームオーバー. None was a person or place.
- **Hard negatives (`hard_negative: true`, with `resembles`) cover:**
  - assistant jobs (店員, 助手, アシスタント), a model boat, 計算機, the radio プログラム, Nia as システム係, and 人工の光;
  - the school play 台本, card-game ルール, arcade high scores and stages;
  - era-correct technology: radio, telegram, Morse, records, Walkman, pagers, Pac-Man, VCR, jukebox, cable TV, payphones and paper letters;
  - gutting fish, kneading dough, storms, airlock danger, leopard seals, and grief over a death;
  - real but lesser-known world names;
  - players attempting injection, which the resident deflects in character;
  - spoken lists and counts;
  - in-character refusals, some politely worded (申し訳ないけど…);
  - foreign words used inside Japanese (ボンジュール, アホイ, オーケー);
  - real places and things that belong to the world (Cornwall, Hull, Ohio, the Greyhound, Coke, Berlin, the Olympics, Olympus Mons).

## Files and schema

Both files use the redteam3 schema.

- `out_of_world.jsonl` has the fields `pack`, `place`, `speaker`, `asked`, `answer`, `kind` (one of the 11 kinds) and `lang`.
- `in_world.jsonl` has the same fields, with these differences:
  - `kind` is always `"in_world"`;
  - `hard_negative` is a bool;
  - `resembles` is the bad kind for a hard negative, and `null` otherwise.

## Counts

### out_of_world.jsonl: 463 lines

- 188 are tiny-society.
- 275 are pocket-universe: 99 Maple Street, 88 Ares, 88 Icebridge.

| kind | tiny-society | Maple Street | Ares | Icebridge | total |
|---|---|---|---|---|---|
| machine_self_reference | 18 | 9 | 8 | 8 | 43 |
| fourth_wall | 17 | 9 | 8 | 8 | 42 |
| anachronism | 17 | 9 | 8 | 8 | 42 |
| harmful | 17 | 9 | 8 | 8 | 42 |
| invented_fact | 17 | 9 | 8 | 8 | 42 |
| prompt_injection | 17 | 9 | 8 | 8 | 42 |
| ooc_formatting | 17 | 9 | 8 | 8 | 42 |
| refusal_boilerplate | 17 | 9 | 8 | 8 | 42 |
| wrong_language | 17 | 9 | 8 | 8 | 42 |
| real_world_intrusion | 17 | 9 | 8 | 8 | 42 |
| mixed | 17 | 9 | 8 | 8 | 42 |

### in_world.jsonl: 522 lines

| place | total | hard negatives |
|---|---|---|
| tiny-society | 223 | 77 |
| Maple Street | 109 | 46 |
| Ares | 96 | 33 |
| Icebridge | 94 | 32 |
| **total** | **522** | **188** (36%) |

Hard negatives by the kind they resemble:

| resembles | count |
|---|---|
| anachronism | 24 |
| refusal_boilerplate | 22 |
| machine_self_reference | 21 |
| fourth_wall | 20 |
| harmful | 20 |
| prompt_injection | 20 |
| real_world_intrusion | 19 |
| invented_fact | 14 |
| ooc_formatting | 14 |
| wrong_language | 14 |

## Validation

- Both files were parsed line by line with `python3` (`json.loads`).
- All `kind` values are valid, and `lang` is `"ja"` on every line.
- On every in-world line, `hard_negative` is true exactly when `resembles` names a bad kind.
- No `answer` appears twice across the two files.
