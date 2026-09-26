# v0.9.0 review: a place that runs out of story

`v0.9.0` made the World a place you look into. People are drawn, they talk, they walk about between turns, you can ask them things, and a return plays as a film. This review sets the look aside and asks what top products get right after the first ten minutes: **is there still something to do, and something new to see, on day ten?**

To find out, I played each World for 30 turns through its real Pack, the same code the app runs, and counted what changed.

**Verdict:** both Worlds run dry. Tiny Society stops asking the player anything after one choice. Pocket Universe keeps asking but gets stuck, and its people repeat themselves. The window now promises a living place, and that raises the cost of a place where nothing new happens. The next version has to be about *story supply*: a steady stream of new situations, people who want things and remember things, and chapters that end.

## What 30 turns look like

| | Tiny Society | Pocket Universe (Mars) |
| --- | --- | --- |
| Turns with no choice to make | **29 of 30.** After Sea Finch is repaired, nothing is ever asked again | 0 of 30 |
| Distinct things people said | 11 | 11 |
| Most repeated line | "Bread's out of the oven." and "Class dismissed.", 29 times each | "Taking Kestrel Rover past the ridge.", 47 times |
| Where the stakes end up | In work 7 of 7 and money in town rising every day (1,052 → 2,142). Nothing can go wrong | Trust 0 of 10 and tension 10 of 10 for the last 15 turns, pinned at both ends |
| Headline | "While you were away" on all 30 days | "Life goes on" for 20 turns in a row |
| Something built | Nothing | 19 things in 25 turns, until the horizon is full |

The choices Pocket Universe offers differ in name only. "Let the world move", "Let the quiet stretch run", "Let it unfold without steering" and "See what the next sol changes" all mean the same thing: wait.

## Against the products that do it best

| What keeps people coming back | Best in class | How | World Machine v0.9.0 |
| --- | --- | --- | --- |
| Something always happens | *RimWorld*'s storytellers (Cassandra, Randy) | A director watches the colony's tension and, when things go quiet or too easy, sends a raid, a visitor, a sickness or a windfall | Whatever the rules do. Once a World settles, it stays settled |
| People want things | *The Sims* (wants and fears), *Animal Crossing* (villager requests) | Everyone has a small, visible want. Granting it matters to them, and they remember that you did | Only people the Pack scripted ask for anything, and each only once |
| People remember | *Hades*, *Wildermyth* | Lines react to what happened before ("After the storm…"). The same thing is rarely said twice | Fixed lines per kind of moment, repeated dozens of times |
| Days aren't all the same | *Animal Crossing*, *Stardew Valley* | Seasons, weather, festivals and birthdays. The calendar itself brings things | Every day is the same day |
| Stories end | *Reigns*, *Frostpunk*, *Wildermyth* | A run or chapter has a shape: it builds, peaks and ends on a card you remember, then the next one starts | Arcs have no ending. Gauges pin at an extreme and stay there |
| Progress you can see | *Animal Crossing*'s island rating, *Stardew*'s community centre | Goals you can see, finished one by one, that change the place | Built things pile up on the horizon, but none is a goal |

## v0.10: a World that always has a story

The bar: in either World, over 60 periods of play, **every period offers at least one real choice** (not just "wait"). No line is said more than three times in any ten periods. No gauge stays pinned at an end for more than five periods. The World reaches the end of a chapter within about 30 periods. All four are enforced by tests that play the real Packs, like the 40-word test does now.

1. **A storyteller.**
   - Each World gets a director, in the style of RimWorld's. It reads how the World stands (calm or tense, flush or broke, pinned or balanced) and, when things have gone quiet, too easy or stuck, it sets up a new situation. In the harbour that could be a storm, a newcomer, a sick child or a big order. On Mars: a dust front, a failing seal or a signal.
   - The director runs as rules inside the Pack. It is deterministic from the World's own state and seed, and what it does is recorded as events, so replaying a World never needs it again.
   - The pacing logic that isn't about any one World (tension curve, cooldowns, pinned-gauge detection) goes into a shared System crate outside `world-core`, and both Packs use it.
2. **Everyone wants something.**
   - Each person has one small want at a time, taken from their state: Emma wants the school roof fixed, Leo wants to put on a music night, Nia wants a spare seal.
   - Asking "What do you need?" gives their want. Granting it is a real Action with a cost, and they remember it. A want left too long turns into a grudge.
   - The *Sims* and *Animal Crossing* loop of small, personal goals.
3. **Every period brings a card.**
   - At least one choice each period that is more than "wait": a want, something the director set up, or a consequence coming due.
   - The four ways of saying "wait" become one.
4. **People remember, and don't repeat themselves.**
   - Lines come from small pools per kind of moment, chosen by the World's seed, and they refer back to what happened ("First sol since the storm.", "Thanks again for the loan.").
   - A line is rested once it has been said recently. It is still all written by the Pack and recorded with the moment.
5. **A calendar with seasons and days that matter.**
   - Seasons change the palette, the weather and what the director tends to send.
   - Each World has a few recurring days: the harbour's regatta and market day, the colony's launch window.
   - Birthdays come from each person's seed.
6. **Chapters that end.**
   - A World's story comes in chapters: a pressure that builds, a turning point, and an ending card that says how this one went ("The harbour kept its bakery. Jonas never went back to sea.").
   - The ending goes into a chapter book in the drawer and on Home, and the next chapter starts from where this one left the World.
   - Gauges can no longer pin at an end: reaching one ends the chapter or turns it.
7. **Goals you can see.**
   - A few standing goals per World, shown on the horizon as unbuilt outlines: rebuild the pier, open a second dome.
   - Each fills in as its parts are done. What gets built then means something, and the horizon stops filling with unnamed shapes.
8. **Being told, and a signed app.** This is still item 10 of the last plan, and it still waits on the five secrets in `RELEASE_SIGNING.md`. A storyteller makes it worth more: "A storm is coming to the harbour" is a notification worth getting.

As before, every concept is built in both Pocket Universe and Tiny Society before it counts. The director, wants, chapters and goals change World rules, so each Pack's version moves in v0.10, and Worlds from v0.9 will need exporting first. The changelog and known issues will say so.

## How we'll know

- **Story-density tests** in each Pack: 60 periods under three different ways of choosing, checked against the four numbers in the bar above.
- A replay test: a World played with the director replays to the same state without running the director again.
- A two-week diary on a real Mac: open one World once a day and note whether that day brought something new. The target is "yes" on at least 12 of 14 days.
