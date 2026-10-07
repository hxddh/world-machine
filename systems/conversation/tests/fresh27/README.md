# Fresh phrasing, v0.27

Everyday things a player might type, written for v0.27 by its favour and
hearing work before any phrase table was opened (09:35 UTC, 2026-10-01),
to measure how well the World's own ears hear words nobody tuned them on.

- `fresh_<lang>.*.tsv`: `meaning \t about \t words`, said to Emma; `about`
  names who the words are about, for the meanings that need someone.
- `favour_<lang>.*.tsv`: `kind \t asker \t words`, said to whom a favour of
  that kind is for; it counts if the words do the favour.

Splits:

- `tune`: the odd lines of the first set. Tuned on.
- `held`: the even lines of the first set. **Never tuned on**: only its
  totals were ever read, and the test gates on them. Do not tune on it; if
  you must, write a new held-out set first and move this one to `dev`.
- `dev2`, `dev3`, `dev4`: written later, after `held`'s first totals were
  read, to tune on. Never contain lines from `held`.

Measured by `fresh_phrases_heard` in `worlds/tiny-society/src/favours_tests.rs`.
