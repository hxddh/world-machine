# World voice red-team set #5 (blind), English, writer W-en2

This set is a held-out measurement. Do not edit it, add to it, or tune the guard against it.
Log any failures and fix the guard using other data. If a label looks wrong, record that separately.

## Scope: no harmful or prompt_injection kind

This set deliberately has **no `harmful` kind and no `prompt_injection` kind**. Earlier sets cover them.
No line in either file is harmful or attempts an injection, and none was folded into `mixed`.
Every `mixed` line combines two of the nine kinds below.

## How it was written

- It was written blind. The only repo files read were:
  - `systems/conversation/tests/redteam3/README.md`, for the schema only;
  - `worlds/tiny-society/src/seed.rs` for the cast and places, plus a grep that confirmed Ada and Ivo are residents;
  - the `VISITOR_NAMES` pool in `worlds/tiny-society/src/life.rs` and the `TOWN_NAMES`, `ICE_NAMES`, `MARS_NAMES` and `people_names` pools in `worlds/pocket-universe/src/life.rs`, read only to keep invented names clear of them;
  - `worlds/pocket-universe/src/places.rs`, part of `eras.rs`, and the three seed Actions in `worlds/pocket-universe/src/lib.rs`;
  - the first lines of `worlds/tiny-society/locales/zh-Hans.tsv`;
  - grep output that located the Pocket Universe residents' names, which showed a few matching lines from `voices.rs`, `years.rs` and `speech.rs`.
- The writer did not open anything else under `systems/conversation/`, `crates/world-voice/`, any `red_team.rs`, any lexicon, bounds, judge or guard file, any `redteam*/` or devset jsonl, `scratchpad/v026/blind4`, or the other writers' folders.
- Every line was written by hand. A small script only split the hand-written lines into JSON fields. It did not generate, template or paraphrase any text.
- Answers are one or two short spoken sentences. The exceptions are `ooc_formatting` lines and a few `mixed` lines, whose markdown, tables, code fences and list breaks are the point.
- `lang` is `"en"` everywhere. It is the player's language. In `wrong_language` lines the answer is in French, German, Spanish, Italian, Portuguese, Dutch, Polish, Swedish, Czech, Finnish, Turkish, Greek, Russian, Japanese, Korean or Chinese.
- Every `invented_fact` name was grepped as a whole word over `worlds/` with `red_team.rs` excluded. One collision ("Rusty", a spacesuit's name in `firsts.rs`) was replaced with Spike Dalloway. None of the brief's reserved names is used as an invented name. Those names are Orla, Kowalski, Tilly, Okafor, Pell, Alfie, Ricky and Pebble. Example invented names:
  - Gideon Marsh, Cornelius Bray, the Widow Halloran, Merrow Isle, Gull Rock Light;
  - Commander Ostrowski, Vesper Station, Dome Seven;
  - Duke's Diner, Principal Hargrove, Spike Dalloway;
  - Elder Flipwick, Queen Tuxa, the Krill Caves.
- The in-world answers name only seed residents and places. Those are:
  - Tiny Society: Jonas, Mara, Leo, Emma, Mia, Noah, Evan, Sofia, Ada and Ivo, plus the harbour, Harbour Bakery, Island School, Anchor Pub, Mainland Fish Market and the Sea Finch.
  - Ares: Nia Chen, Tomas Vale, Ares Habitat, Hydroponics Bay and Kestrel Rover.
  - Maple Street: Lena Ortiz, Max Park, Maple Arcade, K-88 Radio, Night Bus 6 and the Maple Loop.
  - Icebridge: Piko, Miri, the Fish Vault and the Aurora Council.

  Everyone else is unnamed, such as "my father", "a stranger off the ferry" or "the pub cat".
- Pocket Universe `real_world_intrusion` lines use real companies (SpaceX, NASA, Tesla), apps, cities and celebrities. Maple Street hard negatives use a few era-correct 1987 items (Walkman, VCR, Pac-Man, Coke and Tab).

## Files and schema

The schema is the same as set 3.
- `out_of_world.jsonl` has these fields: `pack`, `place`, `speaker`, `asked`, `answer`, `kind` and `lang`.
- `in_world.jsonl` has the same fields. In this file `kind` is always `"in_world"`, and each line also has `hard_negative` (bool) and `resembles`. `resembles` is the bad kind the line brushes against, or `null`.
- `place` is `null` for tiny-society. For pocket-universe it is `"Ares"`, `"Maple Street"` or `"Icebridge"`.

## Counts

### out_of_world.jsonl: 380 lines

| kind | Tiny Society | Ares | Maple Street | Icebridge | total |
|---|---|---|---|---|---|
| machine_self_reference | 26 | 5 | 6 | 5 | 42 |
| fourth_wall | 26 | 5 | 6 | 5 | 42 |
| anachronism | 25 | 5 | 7 | 5 | 42 |
| invented_fact | 26 | 5 | 6 | 5 | 42 |
| ooc_formatting | 29 | 4 | 5 | 5 | 43 |
| refusal_boilerplate | 27 | 5 | 6 | 4 | 42 |
| wrong_language | 26 | 5 | 6 | 5 | 42 |
| real_world_intrusion | 27 | 6 | 6 | 4 | 43 |
| mixed | 29 | 4 | 5 | 4 | 42 |
| **total** | 241 | 44 | 53 | 42 | 380 |

By pack: 241 are tiny-society and 139 are pocket-universe.

### in_world.jsonl: 520 lines, 190 of them hard negatives

| resembles | Tiny Society | Ares | Maple Street | Icebridge | total |
|---|---|---|---|---|---|
| (plain, not a hard negative) | 190 | 48 | 48 | 44 | 330 |
| machine_self_reference | 15 | 4 | 3 | 2 | 24 |
| fourth_wall | 17 | 2 | 3 | 2 | 24 |
| anachronism | 17 | 3 | 3 | 1 | 24 |
| refusal_boilerplate | 16 | 3 | 2 | 3 | 24 |
| invented_fact | 15 | 3 | 2 | 2 | 22 |
| real_world_intrusion | 14 | 3 | 3 | 2 | 22 |
| ooc_formatting | 14 | 2 | 2 | 2 | 20 |
| wrong_language | 13 | 1 | 2 | 2 | 18 |
| mixed | 7 | 1 | 2 | 2 | 12 |
| **total** | 318 | 70 | 70 | 62 | 520 |

By pack: 318 are tiny-society and 202 are pocket-universe.

The hard negatives cover:
- "assistant" as a job (Sofia, Mia), the model boat, the radio programme, the habitat's systems and memory banks, and "autopilot" at 2 a.m.;
- darts players, a spirit level, the school-play script, "a character", the classroom map, slide races and the arcade high score;
- the wireless, telegrams, the post, slate tablets, a mouse, tweeting sparrows, a spider's web, the dial, and era-correct Walkman and VCR;
- unnamed people (a father, an uncle, strangers off the ferry) and true facts about real residents;
- spoken lists, signs, scores and rules with no markup;
- refusals given in character;
- loanwords and a few foreign words inside English (bonjour, merci, déjà vu, bon voyage, uno dos tres);
- brand-like common nouns: apple, windows, shell, dove, target, gap, mercury, orange, ford, the Milky Way, and the penguin.

## Validation

- Every line in both files parses as JSON with `python3`.
- No `answer` appears twice across the two files.
- A capitalised-word scan of the in-world answers found only seed names, places, weekdays, Earth, Latin and the Milky Way.
