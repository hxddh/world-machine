# World voice development set (v0.26, V)

Written by the guard's own author (V) while building the layered voice, so it is
**not** a held-out measure: it is a development set, like `redteam/` and
`redteam2/`, that the checks may be tuned on. Its main use is Japanese (a
Japanese player must get Japanese answers kept, and answers in other languages
declined) and hard negatives for the certain/doubtful split.

Same line format as `redteam2/`: `{"pack", "place"?, "speaker", "asked", "answer", "kind", "lang"}`.
`lang` is en, zh or ja. Lines in `in_world.jsonl` should be kept; lines in
`out_of_world.jsonl` should be declined.
