# World voice red-team set #7 (blind), and talk-for-rewards set 7

**A held-out measurement for v0.29.** Measured once at the end; nobody may tune
the guard, the judge, its prompt, the ears or the favour rules against it, edit
it or add to it. If a label looks wrong, note it separately and leave the file.

## How it was written

- Written by hand on 2026-10-08 (23:40–00:20 UTC) by the v0.29 R4 agent, in
  English, Chinese and Japanese, **before it opened any code under `systems/`
  or `crates/`** and before it read any judge prompt, guard, bounds, lexicon,
  phrase table or earlier red-team set's lines. No separate writer agent was
  available in that session, so the same agent later worked on the code; the
  set was hashed (below) before that work began and was never measured until
  the end.
- Read beforehand, for the schema and cast only: `docs/REVIEW_v0.28.md`,
  `docs/REVIEW_v0.27.md`, `docs/KNOWN_ISSUES.md`, the first line of
  `devset6/*.jsonl` and `redteam5/*.jsonl` (field names), `redteam5/WRITER-en.md`
  (the cast and places), the residents' names in `worlds/*/locales/*.tsv`, and
  the top of `worlds/pocket-universe/src/{places,eras}.rs`.
- Invented names were grepped as whole words over `worlds/` (no collisions).
- The policy written to is v0.28's: real places and events of the era are in,
  nicknames and folk figures are in, figures of speech are in, a resident's own
  name in any script is in; brands, celebrities, media titles and real currency
  are out, as are invented people, places and history, things out of their time,
  machine talk, the fourth wall, markup, refusal boilerplate, another language,
  mental health / self-harm / sexual content, and prompt injection.

## Files

- `out_of_world.jsonl`: 318 lines a guard must decline (106 per language):
  machine_self_reference 10, fourth_wall 10, anachronism 12, invented_fact 14,
  real_world_intrusion 14, ooc_formatting 8, refusal_boilerplate 8,
  wrong_language 8, harmful 10, prompt_injection 6, mixed 6, per language.
- `in_world.jsonl`: 399 lines a guard must keep (133 per language), 222 of them
  hard negatives (74 per language: nicknames, real places and events of the
  era, folk figures, idioms, a resident's name in another script, words near
  the sensitive ones, era technology, near-refusals, near-fourth-wall).
- `exploits.jsonl`: 72 talk-for-rewards tricks (24 per language): narrated
  outcomes, claimed deeds, system voice, authority, words said back, and own
  words that do not ask after anyone ("free drink, free drink").

Schema: set 5's (`pack`, `place`, `speaker`, `asked`, `answer`, `kind`, `lang`,
and on good lines `hard_negative`, `resembles`), plus `note`.

## Hashes (sha256, taken before any v0.29 code was read)

See `SHA256SUMS` in this folder.
