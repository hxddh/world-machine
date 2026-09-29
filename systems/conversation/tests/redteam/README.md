# World voice guard: blind red-team set

These cases test the guard that decides whether a model's in-character answer
from a resident may be shown to the player.

## How it was written (blind)

- The author did **not** read the guard code (`systems/conversation/src/bounds.rs`,
  `worlds/tiny-society/src/red_team.rs`) or any of its phrase lists.
- The only repo reading was a skim of `worlds/tiny-society` and
  `worlds/pocket-universe`, mainly grep over names and the `locales/zh-Hans.tsv`
  files, to learn the cast and places: Mara, Jonas, Leo, Emma, Rosa, Evan, Ada,
  Old Tam, Sea Finch, Harbor Bakery (港口面包房), Anchor Pub (锚酒馆),
  Island School (岛上学校), the lighthouse (灯塔), the quay (码头), the mainland
  (大陆), Lantern Night (灯笼夜); Ares Habitat (阿瑞斯基地), Hydroponics Bay, Kestrel
  Rover, Tharsis, Phobos; Maple Street 1987 (枫树街), Maple Arcade, K-88 Radio,
  Night Bus 6, Elm Street; Icebridge (冰桥), Piko, Miri, the Fish Vault, the
  Aurora Council.
- Invented names and places in `out_of_world.jsonl` were grepped against
  `worlds/` so that none of them accidentally exists. One exception is deliberate:
  "Kevin" is a Maple Street name, but the case puts him in Tiny Society as Mara's
  brother who runs a Starbucks.
- Some speakers in Pocket Universe (Walt, Wendy, Tony, Vince, Troy, "Ares crew")
  come from the name lists and were given to places loosely. Treat `place` and
  `speaker` as context, not as ground truth about the Pack.

## Format

One JSON object per line:

`{"pack", "place"?, "speaker", "asked", "answer", "kind", "lang"}`

- `pack`: `tiny-society` or `pocket-universe`.
- `lang`: the language of the conversation (the player's words). For
  `wrong_language` cases, the `answer` is in the *other* language on purpose.
- `kind`: in `in_world.jsonl` it is always `in_world`.

## Counts

### out_of_world.jsonl: 164 lines, every one must be DECLINED (82 en / 82 zh, 50% zh)

| kind | en | zh | total |
|---|---|---|---|
| machine_self_reference | 15 | 16 | 31 |
| fourth_wall | 10 | 10 | 20 |
| real_world | 10 | 10 | 20 |
| mixed (in character, with one breaking clause) | 8 | 8 | 16 |
| anachronism | 7 | 7 | 14 |
| invented_entity | 7 | 7 | 14 |
| harmful | 7 | 6 | 13 |
| prompt_injection | 5 | 5 | 10 |
| ooc_format (markdown, lists, code, URL, emoji spam) | 6 | 4 | 10 |
| wrong_language (unasked language switch) | 3 | 5 | 8 |
| refusal | 4 | 4 | 8 |

By pack: tiny-society 116, pocket-universe 48.

### in_world.jsonl: 239 lines, every one must be ALLOWED (120 en / 119 zh, 49.8% zh)

By pack: tiny-society 170, pocket-universe 69.

Hard negatives included (these look risky but are fine): "I'm no machine, I just
work like one" / 我又不是机器; "Are you a robot?" answered in character; the game of
cards at the pub; darts "player"; "Do you need an assistant?"; Jonas "training"
Evan's boy; "Ignore Jonas" / 别理Jonas; Emma's "instructions" on the blackboard;
"Be prompt"; "my old radio", cassettes and Pac-Man on Maple Street in 1987; Earth
and Phobos on Ares; Old Tam's funeral and a death in the blizzard at Icebridge;
mild grumbles; honest "I don't know" in character; many uses of the mainland /
大陆; Chinese lines with Latin names ("Mara 说面包要趁热吃。", "Jonas 和 Leo …").

## Rule

**The guard's author must not edit these files. To add cases, put them in a new
file** (for example `out_of_world_2.jsonl`), so this set stays a blind
measurement. If a case here looks wrong, raise it with someone other than the
guard's author instead of changing it.

The generator scripts (`../gen_oow.py`, `../gen_iw.py`) are kept beside this
directory only as a record of how it was written.
