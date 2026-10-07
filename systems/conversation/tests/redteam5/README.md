# World voice red-team set #5 (blind)

**This set is a held-out measurement, and it is now spent.** Its numbers were seen when v0.27 was measured, so nobody may tune the guard, the judge or its prompt against it from here on. Nobody may edit it or add to it. If a label looks wrong, note it separately and leave the file as it is.

## How it was written

- Three writers wrote it blind for v0.27, one per player language: W-en2 (English), W-zh2 with an earlier writer's first part (Chinese) and W-ja (Japanese). Each says exactly what they read and did not read in `WRITER-en.md`, `WRITER-zh.md` and `WRITER-ja.md`, which are their own READMEs, unchanged.
- None of them opened the guard, the judge, its prompt, the lexicons, any earlier red-team set or development set, or each other's folders. They read only the cast, places, eras and name pools, to keep invented names clear of them.
- Every line was written by hand; small scripts only turned the hand-written rows into JSONL.
- The three writers' files are concatenated, English, Chinese then Japanese, into `out_of_world.jsonl` and `in_world.jsonl`, with no line changed. The schema is set #3's and set #4's: `pack`, `place`, `speaker`, `asked`, `answer`, `kind`, `lang`, and on good lines `hard_negative` and `resembles`.

## Counts

- `out_of_world.jsonl`: 1,244 lines a good guard must decline (en 380, zh 401, ja 463).
- `in_world.jsonl`: 1,562 lines a good guard must keep (en 520, zh 520, ja 522), of which 632 are hard negatives (en 190, zh 254, ja 188).
- By place, both files: Tiny Society 1,442; Ares 448, Maple Street 486, Icebridge 430.

| kind | en | zh | ja |
|---|---|---|---|
| machine_self_reference | 42 | 43 | 43 |
| fourth_wall | 42 | 42 | 42 |
| anachronism | 42 | 42 | 42 |
| invented_fact | 42 | 43 | 42 |
| real_world_intrusion | 43 | 38 | 42 |
| ooc_formatting | 43 | 39 | 42 |
| refusal_boilerplate | 42 | 37 | 42 |
| wrong_language | 42 | 37 | 42 |
| mixed | 42 | 38 | 42 |
| harmful | – | 42 | 42 |
| prompt_injection | – | – | 42 |

**Scope gaps.** The English set has no `harmful` lines, and neither the English nor the Chinese set has `prompt_injection` lines (the writers' READMEs say why). Those kinds are measured on this set in the other languages only; sets #3 and #4 cover them in all three.

## Recorded judge verdicts

`judged-claude-haiku-4-5.jsonl` holds 2,488 checklists: one for every line the judge would be asked about (the rest are declined for certain by the checks, or are never sent to a judge). It was made after V wrote that the judge prompt was final, and nothing in the guard, the judge or its prompt was changed after these numbers were seen.

- Each line's exact judge prompt was written by `judge_prompts_are_written` (both Packs), under an opaque id that says nothing of its set, its file or its place in the file, with the prompts sorted by that id.
- Claude Haiku 4.5 answered each prompt as a checklist (`{"id", "reply"}`, the reply being the judge's JSON object as text), 25 prompts to a batch.
- Every batch was answered by a read-only agent: it could read the batch and write its answers, and nothing else, so it read each prompt itself rather than running a script over the batch. An earlier round, in which judging agents could run scripts, was thrown away and the whole set judged again this way.
- This went through agent sessions, not the app's own request with a real key, so it is a close stand-in for the shipped judge, not a measurement of it. The on-device model, the other half of the judge pair, is not measured (there was no Mac).

The ids are opaque ids of `all/out_of_world:<line>` and `all/in_world:<line>`, the folder the set was judged from. To measure again, copy the two set files into a folder named `all` and run, with absolute paths:

1. `WORLD_MACHINE_REDTEAM=<all>/out_of_world.jsonl,<all>/in_world.jsonl WORLD_MACHINE_JUDGE_DIR=<dir> WORLD_MACHINE_VERDICTS=<this folder>/judged-claude-haiku-4-5.jsonl cargo test -p tiny-society --lib judge_prompts_are_written -- --ignored`, and the same with `-p pocket-universe`;
2. `WORLD_MACHINE_JUDGE_DIR=<dir> cargo test -p world-pack-testkit --lib judged_metrics -- --ignored --nocapture`.

## Results (v0.27.0)

95% Wilson intervals in brackets.

| | declined | wrongly declined |
|---|---|---|
| strict guard alone | 739/1,244, 59.4% (56.7–62.1) | 34/1,562, 2.2% (1.6–3.0) |
| certain checks alone | 318/1,244, 25.6% (23.2–28.1) | 0/1,562, 0.0% (0.0–0.2) |
| certain and firm checks | 402/1,244, 32.3% (29.8–35.0) | 5/1,562, 0.3% (0.1–0.7) |
| the checks and the judge | 1,132/1,244, 91.0% (89.3–92.5) | 25/1,562, 1.6% (1.1–2.4) |

| with the judge | declined | wrongly declined |
|---|---|---|
| en | 354/380, 93.2% (90.2–95.3) | 1/520, 0.2% (0.0–1.1) |
| zh | 357/401, 89.0% (85.6–91.7) | 11/520, 2.1% (1.2–3.7) |
| ja | 421/463, 90.9% (88.0–93.2) | 13/522, 2.5% (1.5–4.2) |

| strict guard alone | declined | wrongly declined |
|---|---|---|
| en | 242/380, 63.7% | 4/520, 0.8% |
| zh | 226/401, 56.4% | 13/520, 2.5% |
| ja | 271/463, 58.5% | 17/522, 3.3% |

The v0.27 bars were at least 95% declined and at most 1% wrongly declined in each language with the judge, and at least 75% declined in each language without one. All are missed, except that English meets the wrongly-declined bar.
