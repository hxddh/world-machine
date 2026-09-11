# What the top of this category does, and what this one has not built

A study of the products World Machine is competing with, and an honest reading
of where it stands against them.

It exists because of a measurement in `NEXT_TASK.md`. Version 0.5 was planned,
built and judged on this:

```text
whole return page:  61% repeat  ->  40% repeat
```

That is a measurement of the **prose**. It says the sentences stopped repeating.
It cannot say whether anything happened worth writing a sentence about — and the
numbers below say that for long stretches, nothing did.

---

## What the Worlds actually do

Measured by advancing each shipped World one visit at a time and recording what
it offered and when it last had anything new to say. `worlds/*/examples/` has the
probe; these are the figures it printed.

| World | Distinct choices it will ever offer | Visits with something to decide | Last visit that produced a new timeline entry |
| --- | --- | --- | --- |
| Pocket Universe | 12 | 120 / 120 | still going at 120 |
| Tiny Society | 5 | 92 / 120 | **77** |
| Micro Company | 1 | 1 / 120 | **2** |

Three readings of that table, in order of how much they matter.

**Tiny Society goes quiet at visit 77 and stays quiet forever.** Not settled into
a rhythm — silent. Every visit after that opens a World that has nothing to
report. The line drawn across its life is flat at zero for the whole right-hand
side, which is an accurate picture of a town where nobody works, nobody buys and
nobody is paid.

**Micro Company is advertised in the README and is over after two visits.** One
command, one arc, then a document that never changes again.

**Pocket Universe is the only one that behaves like a product**, and the reason
is structural rather than a matter of having more content: it has an era engine.
When one arc resolves, the next era begins, facing a different trouble, shaped by
how the last was answered. It cannot run out in the way the other two do.

### And the one that keeps living is the one that seizes up

`worlds/*/examples/probe_cost.rs`, release build, timing one visit and one
projection as the history grows:

| | Pocket Universe | Tiny Society |
| --- | --- | --- |
| visit 60 | 444 events · **13 ms** | 436 events · 2.2 ms |
| visit 110 | 829 events · **45 ms** | 454 events · 2.2 ms |
| visit 210 | 1,599 events · **162 ms** | 454 events · 2.3 ms |
| visit 410 | 3,140 events · **772 ms** | 454 events · 2.2 ms |

Tiny Society is flat because it is dead: its event count stops moving at 454 and
never moves again, so there is nothing for the cost to grow with.

Pocket Universe is the World that keeps having something to say, and the cost of
a visit grows as the **square** of its history — the fitted exponent across those
three intervals is 1.94, 1.95, 2.31. Projecting a snapshot does the same thing,
13 ms to 763 ms.

Extrapolated, a World at visit 1,000 costs something like four and a half seconds
per visit advanced and the same again every time the window redraws, which makes a
week-long catch-up a two-minute freeze. Phase VII is called "Worlds that don't
end". **A World that does not end is precisely the case this cost curve forbids**,
and nothing in the repository measures it.

---

## What I studied

Direct page fetches are blocked by this environment's egress policy (HTTP 403 at
the proxy), so the sources below are search summaries rather than full texts.
Where I have leaned on my own knowledge of these products rather than on a
source, I say so.

### RimWorld — the single most important idea in the category

RimWorld is not a simulation you watch. It is an **AI Storyteller**, modelled on
Left 4 Dead's AI Director, that looks at the state of your colony and picks the
event it thinks makes the best story next. The simulation is its vocabulary, not
its purpose. Its creator's framing is that the game is not about winning and
losing but about "the drama, tragedy, and comedy that goes on in your colony".

What the director actually reads, per the community documentation: colony wealth
(converted into a points budget that scales threats to what you have to lose),
colonist and animal counts, whether someone has recently died or been badly hurt,
and **how long it has been since the last major event**.

That last input is the one this repository does not have anywhere. A director
that knows how long it has been quiet cannot go quiet, because quiet is a reason
to act. World Machine has a scheduler that runs rules and stops when the rules
stop firing. Visit 77 is what that difference looks like.

Three named storytellers — Cassandra Classic, Phoebe Chillax, Randy Random —
ship as the difficulty setting, so the *pacing* is the thing the player chooses,
not the numbers.

### Dwarf Fortress — and the flaw that is this product's opening

Dwarf Fortress is the category's deepest simulation and the source of most of its
famous stories. It is also the clearest case of the problem World Machine
proposes to solve. From the community discussion: players "need to dig in order
to find stories"; "if you're not one to dig through Legends and connect the
threads, it sure won't feel like a story generator"; and many simulated details
are modelled but "you'll never see them in the game in any meaningful way".
There is also survivorship bias — the fortresses that go smoothly are the norm
and nobody hears about them.

**This is the strategic asset.** The best product in the category cannot tell you
what happened; it makes you excavate it. World Machine's whole pitch is that it
tells you when you come back. That is a real gap and a real answer to it.

It only pays if the World has something to say. Legibility applied to a silent
town produces a well-typeset nothing, which is precisely what visits 78 onward
currently are.

### Wildermyth — how procedural people come to matter

Wildermyth is the counter-argument to "procedurally generated characters are
shallow". What it does, per the design writing on it: characters age, retire,
form bonds, have children who join the party, and die leaving a **legacy** that
carries into future playthroughs. Outcomes are written onto the person — you end
up with a specific hero who is what they are because of a specific event. The
narrative is assembled from authored modular pieces, so the result "feels
authored and personal, despite being generated by algorithms".

Tiny Society's eight residents are all named in the timeline at some point. But
nothing that happens marks them. They have a job, a purse and a location that is
written once when the World is seeded and never again. The one durable choice in
the World changes what happens to the *town*, not to a person you could describe
afterwards.

### Idle and incremental games — the absence loop, designed

The genre built entirely on leaving and returning treats offline progress as a
retention mechanism, and the consensus practice is the opposite of generosity:
progress is **deliberately capped**, "after which accumulation stops, creating a
feeling of lost opportunity", which is what actually brings people back.

World Machine caps catch-up at a week of World time. The reason recorded in
`apps/world-machine-desktop/src/observer.rs` is that "a return should be readable
rather than exhaustive" — a legibility decision, not a design of the reason to
return. Nothing in a World is waiting, degrading, or at risk of being missed.
There is no cost to never coming back.

### Football Manager — the shape of a check-in

The interface is a Continue button and an inbox, and the button's label changes
when something in the inbox needs an answer. The return is not a report; it is a
queue of things addressed to you.

### The competitors that did not exist when this started

There are now shipping AI life-simulators in exactly this space — Altworld,
AnyWorld, Infinite Life Simulation — advertising persistent structured state
(money, relationships, faction standing, rumours about you), memory across a
whole life, and **a shareable recap at the end of a life**. The recap is worth
noting: the retelling is the distribution.

Stanford's generative-agents work (Smallville) is the research ancestor of all of
this and did not become a product. The reported limitations — agents too formal,
too cooperative, hallucinating their own memories — are the reasons this
repository's decision to keep structure deterministic and let a model only narrate
is the right one. That decision is an asset and should be defended.

---

## The diagnosis

World Machine has built, well, the parts of this category that are
engineering: a deterministic event-sourced runtime, replay, fork, worlds as
files, Packs as separate processes, a projection protocol, a document library.
Those are hard and they work.

What it has not built is the part that is design:

1. **There is no director.** Rules fire until they stop. Nothing in the system
   treats "nothing has happened for a while" as a reason for something to happen.
   This is the whole difference between a simulation and a story generator, and it
   is measurable as visit 77.

2. **Nothing is written onto anybody.** The cast is a table of states. No
   resident carries a mark from a choice, nobody moves, nobody wants anything.
   There is no one to miss.

3. **Nothing is at stake.** A World cannot be lost and cannot be won. It stops.
   "Losing is fun" is the motto of the deepest product in this category; here
   there is no losing, and so no tension in any choice.

4. **There is no reason to return other than curiosity**, and curiosity is
   exhausted the first time a return says nothing.

5. **The one World that does keep going cannot afford to.** Quadratic cost in the
   length of its own history. This is an engineering problem rather than a design
   one, and it is the one item on this list that will not be fixed by better
   content: it is the ceiling under every other ambition here.

And one thing worth stating plainly about the work itself: **the project's
measurements are pointed at the narration rather than at the drama.** 61% → 40%
repeated lines is a real improvement to a real problem, and it is downstream of
the problem that matters. A World that has nothing to say can have its nothing
phrased forty different ways.

---

## What would make it a product

In order. Each with the measurement that would settle it, in the style this
repository already works in.

### 1. A director

Lift what Pocket Universe's era engine already proves and make it a first-class
thing a Pack owns: something that reads the World's state — including how long it
has been since anything happened — and decides what should happen next.

*Measure:* **time to silence.** Tiny Society is at visit 77. A World should still
be producing a consequence worth telling at visit 500. This number belongs in CI
next to the repeat-rate number, and it is the more important of the two.

### 1b. A World that can be visited a thousand times

Do this at the same time as the director, because the director is what will
create the histories that break it. Something is recomputing over the whole
event log per visit, on both the advance and the projection path.

*Measure:* the per-visit and per-projection cost at visit 1,000 is within a small
constant of the cost at visit 50. `probe_cost` already prints the curve; the
number belongs in CI.

### 2. Consequences written onto people

A choice should change a person, not only a ledger. Someone leaves, someone takes
over, someone will not speak to someone else again, someone's child appears years
later carrying the thing you decided.

*Measure:* how many of the cast carry a durable mark that a briefing can name,
and whether two Worlds forked at the same moment produce recognisably different
people thirty visits later. Today both forks produce the same eight rows with
different numbers in them.

### 3. Something to lose

A World needs a state the player does not want to reach, and a choice that
trades against it. Tiny Society already has the material — the town dies — but
dying is currently the default rather than the failure, so it costs nothing.

*Measure:* a World can be lost; and the fortune line can be driven to zero *by a
choice* rather than by the passage of time.

### 4. A reason to come back that is not curiosity

Something waiting, something closing, something that gets worse unheeded. The
idle genre's cap exists for this and is deliberately ungenerous.

*Measure:* a return after a week contains at least one decision whose window
closes, and the World is measurably different if it was missed.

### 5. Stop advertising Micro Company, or make it a World

It is in the README and it is over after two visits. One of those two has to
change.

### 6. The retelling

The competitors ship shareable recaps because the retelling is how the product
spreads. This one already draws a town and a line across its life; an exportable
account of what happened to your World is close, and it is the only item on this
list that is marketing rather than design.

### What not to change

The deterministic core, and the rule that a model narrates but never decides.
That is the difference between this and the research prototypes that stayed
prototypes, and between this and the AI life-sims whose worlds cannot be replayed
or forked. **Fork and compare are the genuine differentiator** — no product in
this category lets you go back and take the other choice and put the two futures
side by side. That is worth building the pitch on, and it is currently sitting on
top of Worlds that only ever offer one choice worth taking.

---

## Sources

Search summaries; direct fetches were blocked by this environment's egress policy.

- [The Story Generator: A Game Design Analysis of RimWorld](https://zaydqazi.substack.com/p/the-story-generator-a-game-design)
- [About RimWorld — RimWorld Wiki](https://rimworldwiki.com/wiki/About_RimWorld)
- [AI Storytellers — RimWorld Wiki](https://rimworldwiki.com/wiki/AI_Storytellers)
- [Wealth management — RimWorld Wiki](https://rimworldwiki.com/wiki/Wealth_management)
- [Rimworld, Dwarf Fortress, and procedurally generated story telling](https://www.gamedeveloper.com/design/rimworld-dwarf-fortress-and-procedurally-generated-story-telling)
- [Do you guys actually find interesting stories within this game? — Dwarf Fortress discussion](https://steamcommunity.com/app/975370/discussions/0/5570437336436974214/)
- [Legends — Dwarf Fortress Wiki](https://dwarffortresswiki.org/Legends)
- [Wildermyth: Strategic design using Emergent Story](https://medium.com/@1512909009a/wildermyth-strategic-design-using-emergent-story-to-customize-gaming-experience-3c43c2e0df91)
- ['Wildermyth' Embraces Storytelling Traditions in a Procedural Narrative](https://www.vice.com/en/article/pkbz78/wildermyth-review)
- [How to design idle games — Machinations.io](https://machinations.io/articles/idle-games-and-how-to-design-them)
- [Idle Clicker Games: Best Practices for Idle Game Design](https://games.themindstudios.com/post/idle-clicker-game-design-and-monetization/)
- [The User Interface — Football Manager 2024](https://community.sports-interactive.com/sigames-manual/football-manager-2024/the-user-interface-r4951/)
- [Generative Agents: Interactive Simulacra of Human Behavior](https://arxiv.org/pdf/2304.03442)
- [Story-Generating Games — Game Developer](https://www.gamedeveloper.com/design/story-generating-games)
- [Altworld](https://altworld.io/blog/best-ai-life-simulator-games), [AnyWorld](https://anyworldgame.com/best-ai-life-simulation-games-2026)
