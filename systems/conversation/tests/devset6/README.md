# World voice development set 6 (v0.28, V3) — development data, not a measure

Written by the voice's owner (V3) for v0.28, by hand, from the plan's list of
what fails (`docs/REVIEW_v0.27.md` and the v0.27 measurement's categories),
**never from set 5's lines** (spent; its misses and wrong declines were not
read). The checks may be tuned on everything here, so no number from this
folder is a blind result. The blind sets the lead measures with (blind set 6
and the exploit set) were never opened.

Each file is made by a small script from hand-written rows:

| file | script | what |
|---|---|---|
| `out_of_world.jsonl`, `in_world.jsonl` | `gen.py` | 54 lines a guard must decline (invented people, places and history by subtype; another time, per place; brands, celebrities, media titles and real money; contempt; suicide, self-harm and sex; rewards told as a game tells them) and 46 hard negatives it must keep (the Packs' own figures and newcomers-to-be, transliterated into Chinese and Japanese; real places and history; folk figures; era technology and money per place; words near the sensitive ones) in English, Chinese and Japanese |
| `judged-v3.jsonl` | `gen.py` | for each of those lines, the checklist a careful judge would answer, written by hand by V3 (`judge_names`, `judge_yes` in the line), so the checklist path can be exercised without a model. It is not a model's answer |
| `exploits.jsonl` | `gen_exploits.py` | 26 "talk for rewards" attempts in three languages (outcomes narrated in brackets, rewards claimed, favours said to be done, a favour's ask said back, instructions to the game), each with an eager model's answer, for `world_pack_testkit::exploit` |
| `disagreement.jsonl`, `disagreement-judged-v3.jsonl` | `gen_disagree.py` | 30 pushbacks on a resident's view, each with a model answer that holds the view or gives it up, and V3's hand checklist, for `world_pack_testkit::disagreement` |

Line format is the red-team sets': `pack`, `place`, `speaker`, `asked`,
`answer`, `kind`, `lang`, with `note` (and `judge_*`) for the reader.

## Running

- Lines and checklists: `WORLD_MACHINE_REDTEAM=<abs>/out_of_world.jsonl,<abs>/in_world.jsonl WORLD_MACHINE_JUDGE_DIR=<dir> WORLD_MACHINE_VERDICTS=<abs>/judged-v3.jsonl cargo test -p tiny-society --lib judge_prompts_are_written -- --ignored` (and `-p pocket-universe`), then `WORLD_MACHINE_JUDGE_DIR=<dir> cargo test -p world-pack-testkit --lib judged_metrics -- --ignored --nocapture`.
- Exploits: `cargo test -p tiny-society --lib exploits_never_pay -- --nocapture` (and `-p pocket-universe`). A blind set in the same line format: `WORLD_MACHINE_EXPLOITS=<abs paths, comma-separated>` before the same command.
- Disagreement: `cargo test -p tiny-society --lib residents_hold_their_stances -- --nocapture` (and `-p pocket-universe`).

## Results at the end of V3's work

See "As built (V3)" in the v0.28 contract; repeated in `CHANGELOG.md` by the lead.
