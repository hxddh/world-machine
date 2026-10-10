# Fresh phrasing, v0.29 (blind)

**A held-out measurement.** Written by hand on 2026-10-08 (23:40 UTC) by the
v0.29 R4 agent from the plan alone, **before it opened any code under
`systems/` or `crates/`** or any phrase table; the only lines it had seen were
a handful of `fresh27`/`fresh28dev` rows, read for the meaning names and the
TSV shape. No separate writer agent was available, so the same agent later
tuned the ears, on development sets only. Never tune on this folder, edit it
or add to it; it is measured once, at the end.

Same shape as `fresh27` and `fresh28dev`:

- `fresh_<lang>.held.tsv`: `meaning \t about \t words`, said to Emma, 112 per
  language (28 meanings × 4).
- `favour_<lang>.held.tsv`: `kind \t asker \t words`, said to whom a favour of
  that kind is for, 36 per language (9 each of ask_after, invite, cheer_up,
  sorry). Four `sorry` lines per language pass an apology on without naming who
  it is from ("She's sorry, she asked me to tell you").

Measured with:

```text
FRESH_DIR=<abs>/fresh29 FRESH_SPLIT=held \
  cargo test -p tiny-society --lib a_blind_fresh_set_is_heard -- --ignored --nocapture
```

Hashes: `SHA256SUMS` in this folder, taken before any v0.29 code was read.
