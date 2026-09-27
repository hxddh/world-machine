# v0.14.0 review: friendships that lead nowhere

`v0.14.0` made people hear far more and remember what you said. This review measures what now stands between World Machine and the best products of its kind, and learns from them. The answer is that nothing the player builds up pays off. Standing with a person is a mark in their card that nothing reads. People remember your words but not your deeds or your answers. The questions come round again worded exactly as before.

**Verdict:** the World is alive, but it doesn't reward you for knowing it. In *Stardew Valley* a friendship opens scenes you can't see any other way. In *Animal Crossing* a villager hands you their photo. In *Dwarf Fortress* your deeds are carved into the walls. World Machine counts the relationship and then does nothing with it.

## Where v0.14.0 stands

Measured on Tiny Society with `measure_a_year` (added in this review, ignored by default). A player who always says yes and a player who never answers each play 365 periods. A period is six real hours, so this is about three months of real use.

| | Always says yes | Never answers |
| --- | --- | --- |
| Questions raised in the year | 392, from 53 of the deck's 64 | 226, from 37 of 64 |
| Questions never raised at all | 11 | 27 |
| Median times a question came round | 3 | 3 |
| Most repeated | market day, 36 times, worded the same each time | the same |
| Questions raised 6 times or more | 17 | 8 |
| Chapters closed | 32, about one every 11 periods | 15 |
| Answers that changed what is on the scene | 232 of 365 | |
| What reads the player's standing with a person | only their card; no storylet, line or event depends on it | the same |
| What people remember of the player | the latest words worth recalling, within 30 periods | the same |
| What people remember of the player's deeds and answers | nothing; no line names a bench, garden or lamp the player made, or an answer the player gave | the same |
| Things to do with your hands | Build, Decorate, Give, Invite, Plant; two a period | the same |

**Hearing:** 61 phrases written for this review before any tuning (set four). 6 went unheard (10%): "sup", "what keeps you up at night", "here, take this", "who do you get on with", "anyone you're close to here", "明天见". One was plainly misheard: "anything on your mind" was taken as a question about what is coming up. Five more landed on a near meaning, such as "busy day?" heard as a question about work. The earlier three sets had 27%, 25% and 45% unheard. This set was written by someone who knows the System, so 10% is a floor, not a fair measure.

**First minutes:** a new World opens with a question card over the middle of the scene (screenshot in the v0.14 review). The first deed, the first talk and the place itself all wait behind that card.

## Learning from the best

| What the best do | Who does it best | World Machine v0.14.0 |
| --- | --- | --- |
| **A friendship opens doors.** Stardew Valley has ten hearts at 250 points each. Talking once a day is worth +20, and you can give two gifts a week. One-time heart scenes play at 2, 4, 6, 8 and 10 hearts, some with a choice that moves the heart either way. A full heart never decays. | *Stardew Valley* | Five marks in a card and nothing behind them |
| **People remember what you did.** CK3 characters keep memories of births, battles and deaths that others can read. Dwarf Fortress engraves the fort's own history on its walls and names artifacts after it. Wildermyth keeps a history tab of each hero's deeds. | *Crusader Kings III*, *Dwarf Fortress*, *Wildermyth* | Only words are remembered; deeds and answers are forgotten |
| **The same story, told differently.** Wildermyth recasts one authored event with whoever is in it. RimWorld's storytellers pace incidents (Cassandra: 4.6 days on, 6 off) so beats neither clump nor go quiet. | *Wildermyth*, *RimWorld* | Market day asked 36 times a year in the same words; 11 to 27 storylets never raised |
| **Each return has a reveal.** Neko Atsume shows who visited and what they left, with an occasional memento. Animal Crossing refreshes four fossils and a money rock every day. Wordle limits you to one puzzle a day, about three minutes. | *Neko Atsume*, *Animal Crossing*, *Wordle* | The return film tells what happened, but hands you nothing to keep |
| **The first minutes are a toy.** Townscaper has no goals and no failure; every click improves the town. Animal Crossing starts you with a tent and a campfire and lets you watch the island grow. | *Townscaper*, *Animal Crossing* | The first thing a new player meets is a two-button question card |

Sources: the Stardew Valley wiki (Friendship); the RimWorld wiki (Cassandra Classic); the Dwarf Fortress wiki (Legends, Engraving); the Wildermyth wiki (Legacy); the CK3 1.7 patch notes; Park et al., *Generative Agents* (2023); Game8 and TheGamer guides for Animal Crossing; Wikipedia on Townscaper, Wordle and Neko Atsume. The research notes behind this table are in the pull request that adds this review.

## v0.15: friendships that open doors

The bar, checked by tests on the real Packs:

- **A friendship opens doors.** At least three moments a person offers only at a warm enough standing: a scene, a favour or a keepsake. Each one happens once per person per World, and a test that plays a friendship up proves each is reached. A cold standing closes doors too: a person with a grudge won't ask the player for help.
- **People remember what you did.** Within three periods of the player answering someone's question, building, planting or decorating near them, or giving them something, that person mentions it by what it was. Things the player made are named as the player's ("the bench you put up by the pier") in speech, festivals and chapter endings.
- **The same question, told differently.** A question that comes round again never uses its previous wording, and says what happened last time ("Market day again. Last time you sent Leo; he sold out by noon."). For a player who answers, nothing that could come up waits more than 60 days, the calendar's days aside.
- **Each return has a keepsake.** Every return of a period or more hands the player one thing to keep: a note, a drawing, a photo, a pressed flower, or a line in the chapter book. It comes from someone they know, for something that happened. Keepsakes collect in a drawer.
- **The first minutes are a toy.** A new World opens on the place, not a card. The first deed can be done at once, can't fail, and someone reacts to it the same period. The first question comes after that, or when the first period passes.

1. **Bonds that open doors (a System, not a Pack rule).**
   - Standing is each person's regard for the player, which storylet conditions can already read. The doors belong to `systems/lives`, so both Packs have them without writing them: a confidence, a favour and a keepsake, each marked once so it never repeats, and a cold shoulder from anyone with a grudge.
   - Nothing wears a friendship away; standing moves only with what happens.
2. **Memory of deeds and answers.**
   - What a person remembers widens from talk to every event the player caused that touches them: an answer to their question, something made near where they live or work, a gift. The recollection names what it was, from the event itself, so replay needs no model.
   - Things the player made carry a maker, and speech, festival tellings and chapter summaries name them as the player's.
3. **Recast questions.**
   - A storylet may have several wordings. The one used last time is skipped, and its previous outcome can be told in the new one.
   - The storyteller favours storylets not yet raised this year, so the deck is used, and keeps its current pacing bars.
4. **Keepsakes and a gentler start.**
   - A keepsake is an event with a giver, a reason and a drawing, kept in a drawer. The return film ends on it.
   - A new World's first period offers a deed before a question. The first question is held until the player has acted or the first period has passed.

## How we'll know

- `a_warm_friendship_opens_doors` and `a_grudge_closes_them`, played up and down with one resident in each Pack; `doors_open_once` across replay and branches.
- `people_remember_what_you_did`: for each kind of deed and for answers, a mention within three periods, naming it.
- `what_you_made_is_named_as_yours` in speech, festival tellings and chapter summaries.
- No question asked in the same words twice running, and no wait over 60 days for a player who answers, checked in each Pack's year-long test next to the existing year-long and story-density bars.
- `every_return_brings_a_keepsake` for returns of 1 to 7 periods; `a_new_world_opens_on_the_place`.
- Hearing: a fifth set of unseen phrases, written before any tuning by someone other than the author of the rules, with the unheard share reported, not tuned away.
- As before, a year-old World's snapshot in 10 ms at most.

Out of scope by decision: Worlds saved by earlier releases. They are not carried forward.

Still to do, and outside the code: trying World voice against a real model on a Mac, a commissioned artist's set, the two-week diary on a real Mac, and the five signing secrets.

## Progress

`v0.15.0` ships all four. What changed from the measured start, on a year of Tiny Society (365 periods) and the same bars in Pocket Universe:

| The bar | Tiny Society | Pocket Universe |
| --- | --- | --- |
| What reads standing | the doors (a secret at "likes you", a favour and then a keepsake at "thinks the world of you") and the cold shoulder, each tested by playing one friendship up and one grudge down | the same, in every seed |
| Doors opened by one befriended person in 120 periods | a secret, a favour, a keepsake, once each, in that order | the same, in every seed |
| Asking for help while holding a grudge | never, over 60 periods of insults | the same System |
| What people remember of the player | words, answers to their questions, things they saw the player make, presents, evenings out, a kept secret | the same |
| An answer or a deed brought up | within three periods, naming it | the same, in every seed |
| What the player made, named as theirs | in festivals held where it stands, and in the chapter's ending | the same |
| A repeat question worded as last time | never, over a year, three ways of playing | the same, three seeds |
| Longest wait for a question that could come up, for a player who answers | at most 60 days, the calendar aside, saying yes or taking the last answer | not yet measured |
| A return's last beat | a keepsake, for every return of 1 to 7 periods | the same, in every seed |
| A new World | opens on the place; the first deed brings a reaction and then the first question | the same, in every seed |

What did not move: for a player who always says yes, 11 of the 64 storylets still never come up in a year. They are threads that only follow a refusal, plus two goals (the pier and the lamp) that are built by answers costing money a yes-to-what-you-can-afford player never has. That is a finding for the next release, not a deck that goes unused. A year-old World's snapshot measured the same as `v0.14.0` on the machine this was built on (12 to 20 ms for both, where `v0.14.0` had measured 9 ms on a quieter machine); it needs re-timing on a quiet machine or a real Mac.

These are checked by the tests below, next to every earlier release's tests, which still hold:

- `a_warm_friendship_opens_doors_once_each`, `a_grudge_closes_doors_and_shows` in `systems/lives` and Tiny Society; `a_warm_friendship_opens_doors_in_every_place` in Pocket Universe
- `people_remember_what_you_did_for_them` in `systems/conversation`; `people_remember_how_you_answered_them`, `someone_sees_what_you_made_and_remembers_it` (and `…_in_every_place`)
- `what_you_made_is_named_as_yours`
- `every_return_brings_a_keepsake` (and `…_in_every_place`), `a_return_brings_a_keepsake_from_someone_who_likes_you`
- `a_new_world_opens_on_the_place` in both Packs
- `it_remembers_how_often_each_came_up_and_how_it_ended`, `what_has_never_come_up_comes_first`, `nothing_comes_up_while_held` in `systems/storylets`
- `keepsakes_cross_the_wire_and_an_older_pack_sends_none`

Still to do: each person's own secrets and keepsakes instead of shared ones, goals a careful player can finish, a commissioned artist's set, trying World voice against a real model on a Mac, the two-week diary on a real Mac, and the five signing secrets.
