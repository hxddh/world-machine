# Fresh phrasing, v0.28 dev

**Development data.** Written by W2 for v0.28 on 2026-10-08 to tune the
World's own ears on. Not blind: every line here has been tuned on. The
blind set for v0.28 (`fresh28`) is kept outside the repository, was never
read by W2, and is measured once, by the lead, with
`a_blind_fresh_set_is_heard` (below).

Same shape as `fresh27`:

- `fresh_<lang>.<split>.tsv`: `meaning \t about \t words`, said to Emma;
  `about` names (in English) who the words are about, for the meanings
  that need someone.
- `favour_<lang>.<split>.tsv`: `kind \t asker \t words`, said to whom a
  favour of that kind is for; it counts if the words do the favour.

Splits, each written before any tuning on it, and measured once as a
stand-in for unseen phrasing (totals only, with the blind entry point)
before it was tuned on:

| split | written | heard before tuning on it (en / zh / ja) | favours |
|---|---|---|---|
| `dev` | first, weighted to v0.27's cross-language misses ("give X another chance", encouragement as cheering up) | en 75.8% (zh, ja not measured) | en 76% |
| `check` | after tuning on `dev` | 63.2 / 57.9 / 63.2% | 69% |
| `check2` | after tuning on `check` | 74.1 / 70.4 / 66.7% | 89% |
| `check3` | after tuning on `check2` | 81.5 / 70.4 / 83.3% | 92% |
| `check4` | after tuning on `check3` | 88.9 / 80.3 / 74.1% | 91% |
| `check5` | after tuning on `check4` | 83.3 / 88.9 / 75.9% | 77% (30 lines) |

All splits are now tuned on and gated at 95% by
`fresh28_development_phrases_stay_heard` in
`worlds/tiny-society/src/favours_tests.rs`.

To measure a blind set kept elsewhere (its misses are never printed):

```text
FRESH_DIR=/path/to/fresh28 FRESH_SPLIT=held \
  cargo test -p tiny-society --lib a_blind_fresh_set_is_heard -- --ignored --nocapture
```
