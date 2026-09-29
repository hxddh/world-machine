# World voice red-team set #3 (blind)

**This set is frozen. Nobody may edit it, add to it, or tune the guard against it.**
It is a held-out measurement. If you change the guard (its word lists, patterns, bounds or thresholds)
because a line here failed, the set stops measuring anything. Log failures, fix the guard using other
data, then re-run this set unchanged. If a label here looks wrong, record that separately and leave the file as it is.

## How it was written

- It was written blind. The author did not read `systems/conversation/`, `worlds/*/src/red_team.rs`,
  any file with "lexicon" or "bounds" in its name, or the earlier red-team sets (`redteam/` and `redteam2/`).
- The only repo reading was for the cast, places and eras:
  - the seed in `worlds/tiny-society/src/seed.rs`;
  - the visitor and trade name pools in `life.rs`, read only to keep invented names clear of them;
  - the two `locales/zh-Hans.tsv` files.
- Every line was written by hand. No templates or paraphrase generators were used. The phrasing varies on purpose:
  - slang, typos ("langauge", "actualy") and euphemism ("autocomplete with a haircut", "pop the airlock");
  - Traditional Chinese mixed with Simplified;
  - non-EN/ZH languages: French, German, Spanish, Italian, Portuguese, Japanese, Korean, Russian, Dutch and Polish.
- `lang` is the player's language, not the language of the answer. In `wrong_language` lines, the answer's language differs from `lang`.
- Every `invented_fact` name was checked against the world sources and locales and does not exist. Examples:
  - Bertrand Quayle, Harlan Voss and the Gullwing Cannery;
  - Rosalind Fairweather, Mr. Pemberton, Agatha Brisbane, Bartholomew and Kelp Lane;
  - Dr. Okonkwo Adebayo, Sal's Pizzeria, Captain Frostbeak;
  - 王大川, 周海生, 林婶, 赵师傅, 陈美玲, 蘇小雨, 老乔治, 巴托爷爷, 马克·霍尔登.
- The in-world answers name only seed residents and places that appear in the world text. Invented names were removed on purpose, so that no good line is an invented fact.
- `harmful` lines stay at PG wording. They cover violence instructions, self-harm encouragement, insults at the listener and hate against a group.

## Files and schema

- `out_of_world.jsonl` holds answers a good guard must decline. Each line has:
  - `pack`, `place` (`null` for tiny-society), `speaker`, `asked`, `answer`;
  - `kind`, one of the 11 kinds below;
  - `lang`, the player's language.
- `in_world.jsonl` holds answers a good guard must allow. Each line has the same fields, plus:
  - `kind` is always `"in_world"`;
  - `hard_negative` (bool) marks a line that shares surface words with a bad kind but is fine;
  - `resembles` names the bad kind a hard negative brushes against.

## Counts

### out_of_world.jsonl: 210 lines

- 100 are Chinese (48%).
- 126 are tiny-society (60%).
- 84 are pocket-universe: 37 Maple Street, 22 Ares, 25 Icebridge.

| kind | en | zh | total |
|---|---|---|---|
| machine_self_reference | 11 | 10 | 21 |
| fourth_wall | 10 | 9 | 19 |
| real_world_intrusion | 10 | 9 | 19 |
| anachronism | 10 | 9 | 19 |
| invented_fact | 10 | 9 | 19 |
| harmful | 10 | 9 | 19 |
| prompt_injection | 10 | 9 | 19 |
| ooc_formatting | 10 | 9 | 19 |
| wrong_language | 10 | 9 | 19 |
| refusal_boilerplate | 10 | 9 | 19 |
| mixed | 9 | 9 | 18 |

### in_world.jsonl: 236 lines

- 121 are Chinese (51%).
- 145 are tiny-society (61%).
- 91 are pocket-universe: 36 Maple Street, 30 Ares, 25 Icebridge.
- 135 are hard negatives (57%), and 69 of those are Chinese.

Hard negatives by the bad kind they resemble:

| resembles | count |
|---|---|
| anachronism | 25 |
| machine_self_reference | 24 |
| prompt_injection | 20 |
| refusal_boilerplate | 18 |
| fourth_wall | 16 |
| harmful | 15 |
| real_world_intrusion | 13 |
| wrong_language | 4 |

The hard negatives cover:
- jobs such as shop assistant, 助手/助理, and assistant engineer;
- era-correct 1987 brands and songs: Pac-Man, Walkman, Joshua Tree, Livin' on a Prayer, Commodore 64, VCR, Coke and Tab, The Princess Bride;
- world senses of loaded words: a fishing net, the harbour code, Morse code, the play script, a fiddle player, a model boat, the water system, the neighbourhood network;
- gentle grief over a death;
- in-character refusals;
- players attempting injection, which the resident deflects in character.

## Validation

- Every line in both files parses as JSON with `python3`.
- No `answer` appears twice across the two files.
- The generator and validator sit outside this folder, in `../redteam3_build/` (`bad.py`, `extra.py`, `good.py`, `build.py`, `validate.py`). They are frozen along with this set.
