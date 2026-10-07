# World voice red-team set #5, Chinese (blind)

**This set is frozen. Nobody may edit it, add to it, or tune the guard against it.**
It is a held-out measurement. If you change the guard (its word lists, patterns, bounds or thresholds) because a line here failed, the set stops measuring anything. Log failures, fix the guard using other data, then re-run this set unchanged. If a label here looks wrong, record that separately and leave the file as it is.

**This set deliberately has no `prompt_injection` kind.** No line in either file is labelled `prompt_injection`, no `mixed` line carries an injection, and no hard negative has `resembles: "prompt_injection"`. Set #5 measures the other ten kinds only.

## How it was written

Two writers wrote it, both blind:
- A first writer wrote part 1 (`wzh_build/bad1.py`): 212 out-of-world lines in five kinds, `machine_self_reference`, `fourth_wall`, `anachronism`, `harmful` and `invented_fact`. They then stopped.
- Writer W-zh2 kept part 1 and wrote the rest:
  - the other five out-of-world kinds (`wzh2_build/bad2.py`);
  - all in-world lines (`wzh2_build/good_*.py`);
  - the build (`wzh2_build/build.py`) and this README.

Neither writer read `systems/conversation/` (apart from W-zh2 reading the redteam3 README for the schema), `crates/world-voice/`, `worlds/*/src/red_team.rs`, any lexicon, bounds, judge or guard file, any earlier red-team or devset jsonl, `v026/blind4`, or the other writers' folders in `blind5/`.

W-zh2 read only these files in the repo, for the cast, places and eras:
- `systems/conversation/tests/redteam3/README.md`, for the schema;
- `worlds/tiny-society/src/seed.rs`;
- the name pools in both `life.rs` files (`VISITOR_NAMES`, `TRADES`, `ORIGINS`, `TOWN_NAMES`, `ICE_NAMES` and `MARS_NAMES`), read only to keep invented names clear of them;
- `worlds/pocket-universe/src/places.rs` and `eras.rs`, and the three seed Actions in `worlds/pocket-universe/src/lib.rs`;
- both `locales/zh-Hans.tsv` files.

Every line was written by hand. No templates or paraphrase generators were used:
- The answers are one or two short spoken sentences.
- Residents keep their Latin names inside Chinese text, as the zh-Hans tables do.
- Places use the locale's Chinese names, such as 港口面包房, 锚酒馆, 岛上学校, 海雀号, 阿瑞斯基地, 水培舱, 红隼号漫游车, 枫叶游戏厅, K-88电台, 6路夜班车, 冰桥, 鱼库 and 极光议会.

### Part 1 invented names

W-zh2 grepped every invented name in part 1 over `worlds/`, excluding `red_team.rs`. All of them are absent, except one:
- 奥林匹斯哨站 clashed with Olympus, which the Ares locale has as 奥林匹斯山, a real place there.
- `build.py` renames that outpost to 赤沙哨站, which appears nowhere in `worlds/`.
- Every other part-1 line is unchanged.

Examples of the invented names:
- 刘铁柱, 孙巧云, 何小梅, 钱大海, 周晓燕, 黄文杰, 赵福生, 郑雅琴 and 方志远;
- Gideon, Ottoline, Percival, Mortimer, Fergus, Hollis, Juniper, Ambrose, Hamish, Rutherford, Corwin, Brannigan and Delphine;
- Ezekiel Marlowe, Cassius, Halvard, Chuck Hargreaves, Gwendolyn, Benny Cosgrove, Marge, Flapsworth, Glacia, Waddles, Snowdrop, Barnaby, Thaddeus, Seraphina, Lucinda, Phineas Fenwick, Quincy and Bellamy;
- 海星罐头厂, 麦穗坊, 咸风乐队, 灯塔路, 海豹湾, 磨坊街, 圣玛格丽特诊所, 黑松林场, 玫瑰茶室, 女武神号, 铁锈谷, 林伍德社区学院, 黑木唱片, 雪滴湾, 海豹岛, 海象部落 and 鸥石滩.

The `mixed` lines that include an invented fact reuse part-1 names only.

### Kinds written by W-zh2

- `ooc_formatting`: in-world content in an out-of-character form:
  - Markdown headings, bold text, bullets, checkboxes and tables;
  - code fences, JSON and HTML or XML tags;
  - stage directions such as （叹气） and `*清了清嗓子*`;
  - speaker labels, 【旁白】, 回答：/理由：, emoji, hashtags and kaomoji.
- `refusal_boilerplate`: polite assistant-style refusals, with no mention of being a machine:
  - 抱歉，我无法……, 如果您有其他问题，欢迎随时提问, 请咨询相关专业人士;
  - 根据相关规定, 出于安全考虑, 我需要保持中立, 由于版权原因, 祝您生活愉快.
- `wrong_language`: in-world content in another language for a Simplified-Chinese player:
  - English, French, German, Spanish, Italian, Japanese, Korean, Russian, Dutch, Portuguese and Polish;
  - Traditional-only Chinese, written so that most characters differ from Simplified (號, 這, 麵, 過, 魚).
- `real_world_intrusion`: Chinese apps, brands and celebrities:
  - apps and brands: 淘宝, 拼多多, 微信, 支付宝, 瑞幸, 网易云音乐, 青岛啤酒, 茅台, B站, 抖音, 王者荣耀, 钉钉, 华为, 小米, 货拉拉, 美团, 海底捞, 京东, 比亚迪, 蔚来, 卫龙, 农夫山泉, 滴滴, 小红书, 原神, 蜜雪冰城, 大疆, 波司登, 海尔, 腾讯会议, 百度地图 and 春晚;
  - people: 周杰伦, 马云, 谷爱凌, 成龙, 邓紫棋, 林俊杰, 何炅, 姚明 and 赵本山;
  - shows: 熊出没 and 甄嬛传.
- `mixed`: each line combines two of the non-injection kinds. The pair is noted in a comment in `bad2.py`, for example `ooc_formatting + machine_self_reference` or `wrong_language + real_world_intrusion`.

### In-world lines

- They name only seed residents and places that appear in the world text. No invented person or place appears in a good line.
- They also draw on era details from `eras.rs` and the locales:
  - Maple Street: Walkman, mixtapes, Nintendo, Game Boy, dial-up, email, pagers, CDs, cable TV, grunge, the mall, the Rialto, the drive-in and the news from Berlin;
  - Ares: the second dome, the great storm, Tharsis, the cadets, the ice mine, the long winter, the dwarf apple tree and the settlers;
  - Icebridge: the whales, the long night, the leopard seal and the far floe.
- Hard negatives share surface words with a bad kind but are fine in the world:
  - `machine_self_reference`: 系统维护员, 程序 on a control panel, 训练 of cadets, a 模型 boat, 助手/店员, arcade and studio 机器, and "我又不是机器";
  - `fourth_wall`: the school play (剧本, 台词, 观众, 角色), arcade 关卡 and 游戏结束, radio listeners and the drive-in;
  - `anachronism`: era-correct technology in each place, and the fishing net (网上破了好几个洞);
  - `harmful`: gutting fish, storms, airlock danger, the leopard seal, de-escalation of the same prompts as the harmful lines, and gentle grief;
  - `refusal_boilerplate`: in-character refusals, some starting with 抱歉/对不起/不好意思;
  - `ooc_formatting`: spoken lists such as 第一……第二……, chalkboard specials, ledgers and set lists;
  - `wrong_language`: a single in-world foreign word (takk, bonjour, merci, gracias, adiós, Ahoy), K-88 and 嘎;
  - `real_world_intrusion`: era-correct brands and titles (Nintendo, Game Boy, 《白鲸》, Livin' on a Prayer), and the world's own real-world origins (Cornwall, Norway, Galway, Brittany, Hull, Seattle);
  - `invented_fact`: specific facts that are true in the seed, such as Leo paying Evan to mend 海雀号, the 120 wedding order, the rover's 18 km range and the vote at moonrise.

## Files and schema

The schema is the same as set #3.

- `out_of_world.jsonl` holds answers a good guard must decline. Each line has:
  - `pack` (`tiny-society` or `pocket-universe`);
  - `place` (`null` for tiny-society, otherwise `Ares`, `Maple Street` or `Icebridge`);
  - `speaker`, `asked` and `answer`;
  - `kind`, one of the ten kinds below;
  - `lang`, which is always `"zh"`, the player's language.
- `in_world.jsonl` holds answers a good guard must allow. Each line has the same fields, plus:
  - `kind`, which is always `"in_world"`;
  - `hard_negative` (bool);
  - `resembles`, which names the bad kind for a hard negative and is `null` otherwise.

## Counts

### out_of_world.jsonl: 401 lines, all zh

| kind | part | count |
|---|---|---|
| machine_self_reference | 1 | 43 |
| fourth_wall | 1 | 42 |
| anachronism | 1 | 42 |
| harmful | 1 | 42 |
| invented_fact | 1 | 43 |
| ooc_formatting | 2 | 39 |
| refusal_boilerplate | 2 | 37 |
| wrong_language | 2 | 37 |
| real_world_intrusion | 2 | 38 |
| mixed | 2 | 38 |
| prompt_injection | none | 0 |

By pack and place:
- tiny-society: 220;
- pocket-universe: 181, of which 57 are Ares, 63 Maple Street and 61 Icebridge.

### in_world.jsonl: 520 lines, all zh

By pack and place:
- tiny-society: 252;
- pocket-universe: 268, of which 93 are Ares, 92 Maple Street and 83 Icebridge.

254 lines are hard negatives (49%):
- tiny-society: 118;
- Ares: 44;
- Maple Street: 52;
- Icebridge: 40.

| resembles | count |
|---|---|
| harmful | 38 |
| anachronism | 38 |
| machine_self_reference | 38 |
| refusal_boilerplate | 28 |
| invented_fact | 26 |
| ooc_formatting | 26 |
| fourth_wall | 25 |
| wrong_language | 18 |
| real_world_intrusion | 17 |
| prompt_injection | 0 |

## Validation

- Every line in both files parses as JSON with `python3`.
- No `answer` appears twice across the two files.
- No line has `kind` or `resembles` equal to `prompt_injection`.
- `build.py` checks all of these and refuses to write if one fails.
- The sources sit outside this folder, in `../../wzh_build/bad1.py` and in `../../wzh2_build/` (`bad2.py`, `good_tiny1.py`, `good_tiny2.py`, `good_ares.py`, `good_maple.py`, `good_ice.py`, `good_extra.py` and `build.py`). They are frozen along with this set.
