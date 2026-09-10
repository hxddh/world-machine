# Implementation Roadmap

Pocket Universe is the architecture north star. Tiny Society is the first product and reference World.

World Machine is no longer only a kernel experiment. The repository now has a durable macOS product shell, multiple Worlds, background progression, causal inspection, and a Tiny Society whose long-running choices produce materially different economic futures.

This roadmap separates the stable runtime foundation from the current product frontier.

## Phase I — Semantic World Runtime

### M0 — Repository and architecture guardrails

Status: implemented and enforced by GitHub CI.

- Rust workspace
- architecture constitution (`AGENTS.md` / architecture docs)
- boundary checker
- formatting, Clippy, semantic tests, and macOS product CI
- release `World Machine.app` artifact build

### M1 — Minimal World data model

Status: implemented.

- Entity
- Relation
- Value
- WorldState

### M2 — Action -> Event -> State

Status: implemented.

- ActionRegistry
- validated Action evaluation
- immutable Event history with causal references
- generic StateChange operations
- atomic event application
- failed Actions/Events do not partially mutate state

### M3 — Deterministic history / replay / fork

Status: implemented for the current product surface, with further branch hardening still possible.

- event log
- replay from baseline
- logical clock preserved by replay
- historical prefix fork
- durable archives reconstruct the same visible history

Known design frontier:

- richer explicit branch identity / lineage metadata
- more efficient snapshot + suffix replay for very large Worlds
- scheduler-history reconstruction beyond the current archive model

These are optimization/hardening tasks, not blockers for the current product loop.

### M4 — Clock / Scheduler

Status: implemented.

- logical World time
- scheduled `ActionRequest`s
- deterministic `(time, insertion order)` execution
- failed scheduled Actions do not corrupt state
- replay does not re-run historical scheduler decisions

Wall-clock time remains outside World truth.

### M5 — Behavior Runtime

Status: implemented.

- RuleBehavior / NativeBehavior
- event subscriptions
- event -> behavior -> action -> event loop
- causal trigger propagation
- deterministic ordering
- loop/action budget protection
- replay remains Event-only and does not re-run Behaviors

### M6 — Deterministic Tiny Society vertical slice

Status: implemented and substantially expanded beyond the original slice.

The original validation goal was that a coherent social World must work without an LLM. Tiny Society now proves that with residents, places, jobs, money, relationships, scheduled living, causal crises, recovery, and long-running economic feedback.

### M7 — AgentRuntime boundary

Status: implemented.

- provider-neutral AgentRuntime
- MockAgentRuntime
- scoped perception
- AgentDecisionEvent
- historical decisions replay without model calls
- Agent output remains a proposed World Action, never authoritative state mutation

### M8 — pi_agent_rust adapter

Status: implemented in the workspace as the out-of-process `world-pi-rpc` boundary and exercised by semantic CI.

- no `pi_agent_rust` dependency in `world-core`
- external decision transport
- strict offered-Action protocol
- filtered observation
- fail-closed handling for invalid decisions/tool attempts/protocol failures

Future optimization, not a prerequisite for product work:

- persistent RPC sessions
- richer dynamic World Action tool registration if/when the external Pi runtime surface makes that useful and stable

## Phase II — Generic Product Surface

### M9 — Projection + GPUI shell

Status: implemented and used by the product.

Generic projection surfaces include:

- Collection
- Inspector
- Timeline
- Semantic Canvas
- causal Why projection
- ProjectionCommand
- Pack-neutral ProjectionIntent

`world-gpui` renders generic Projection state. Tiny Society does not own a parallel application UI model.

### M10 — World Host + multiple Worlds

Status: implemented.

- WorldRegistry / WorldRegistration
- Pack descriptors and version checks
- integrity-checked archive opening
- generic WorldSession lifecycle
- Tiny Society
- Future Archaeologist
- built-in World registration

The presence of materially different Worlds is the current architecture canary: adding a World must not introduce Pack-specific concepts into `world-core` or generic GPUI rendering.

### M11 — Durable `.world` documents and Library

Status: implemented.

- WorldArchive persistence
- integrity validation
- World Library
- durable sessions
- optimistic revision/conflict checks
- candidate -> persist -> commit transaction shape
- Save As / external document targets
- no-op persistence detection

A persistence failure or stale document conflict does not advance the live World.

### M12 — macOS document product

Status: implemented and continuously validated on macOS CI.

- `World Machine.app`
- Home / World document windows
- native document identity/title behavior
- generic GPUI product shell
- Tiny Society and Future Archaeologist desktop regressions
- release `.app` archive artifact

## Phase III — Living Worlds

### Durable background living

Status: implemented.

- Pack-neutral `WorldSession::advance_background(periods)`
- Tiny Society maps a background period to deterministic living progression
- durable background candidate transaction
- static Worlds remain true persistence no-ops when their archive does not change
- background progression never reads wall-clock time from World/runtime truth

### Observer clock

Status: implemented outside `.world` truth.

- device-local observer metadata
- bounded wall-clock -> background-period policy
- claim / rollback semantics
- catch-up before document presentation
- failed durable catch-up leaves the live World untouched

### Return experience

Status: implemented.

- transient visit cursor
- `While you were away`
- return briefing survives the current visit but is not persisted as World truth
- ordinary World interaction clears the transient return mode
- causal events remain inspectable through Timeline / Inspector / Why

## Phase IV — Tiny Society as a Causal Living World

This phase is the current product proof that World Machine can support a persistent society rather than a scripted story.

### Economic circulation

Status: implemented.

- work transfers real workplace cash to residents
- resident purchases transfer real personal cash into Harbor Bakery
- Bakery revenue occurs before payroll in the deterministic living day
- no synthetic income is created merely to keep the simulation alive

### Long-run institutional risk

Status: implemented.

- payroll reserve exhaustion becomes durable history
- Pub and School can lose the ability to fund future payroll
- income disruption propagates to households
- residents spend savings before changing consumption
- `bread_budget_cut` protects emergency savings rather than deleting wealth
- reduced household demand can later produce Bakery payroll crisis and closure

### Consequences and recovery

Status: implemented.

- retaining Jonas has a real long-run payroll cost
- payroll shortfall causes Bakery closure through a persisted causal edge
- Mara can reopen with personal savings
- reopening does not erase prior employment/history consequences
- restored fishing can repair Jonas's income path
- support/reciprocity can complete and be repaid
- restored fishing income can spill back into local Bakery demand

### Adaptive recovery

Status: implemented through M38 and validated against the current local-economy model.

From the same durable Bakery closure:

- traditional salaried reopen: higher investment, restores Mara's fixed Bakery wage, and can fail again under weak demand;
- lean owner-run reopen: lower investment, changes Mara to owner-operator, removes the fixed Bakery payroll burden, and survives the same tested horizon.

This is an important product threshold: a fork now changes the World’s operating structure, not just one Event.

## Phase V — Strategy, Lineage, External Packs, and the Analyst

Status: implemented across M42 through M262.

The repository moved well past the original M42 target. The main capabilities landed since then:

- **Strategy comparison** (`world-compare`, `world-strategy`, `world-strategy-gpui`): deterministic side-by-side comparison of two branches from the same archive, with a generic comparison surface and saved comparisons.
- **Lineage** (`world-lineage`, `world-lineage-compare`, `world-lineage-gpui`): explicit branch identity, parent/child lineage index, and a lineage explorer.
- **External World Packs** (`world-pack-protocol`, `world-pack-bundle`, `world-pack-process`, `world-pack-catalog`, `world-pack-server`): out-of-process Packs distributed as `.worldpack`, static review before any code runs, quarantine, durable probe, activation, and a `world-pack-check` conformance command. Request and response records are bounded at 16 MiB in both directions.
- **Pocket Universe and Micro Company** as included external Packs; Pocket Universe is the packaged `Start here` experience.
- **Evidence query and investigation** (`world-query`, `world-investigation`, `world-investigation-local`, `world-cli`): machine-readable evidence, first-divergence investigation, and bounded stdin/stdout framing.
- **Read-only Analyst** (`world-agent-tools`, `world-agent-tool-host`, `world-agent-tool-stdio`, `world-analyst-client`, `integrations/pi`): an out-of-process Pi session with only catalog-derived read-only tools, a World Machine-owned analyst-turn protocol, and a desktop Analyst panel with readiness probing, virtualized history, and bounded framing on every hop.
- **Pre-alpha packaging**: repeatable `World Machine.app` build, ad-hoc signed and not notarized, with a validated package manifest.

## Phase VI — 0.2 "Usable" release

Status: in progress. This phase replaces milestone-by-milestone infrastructure work with acceptance criteria that describe what a user can do.

### The one usability problem this phase cannot fix by itself

A normal macOS app is download, open, use. That requires a Developer ID signature and Apple notarization; without them every install path needs a manual "Open Anyway" step, and no packaging trick removes it. The pipeline is now fully built for the signed path and switches on with five repository secrets ([RELEASE_SIGNING.md](RELEASE_SIGNING.md)); the Apple Developer Program costs USD 99 per year. Until then the release ships a DMG with a `Read Me First.txt` next to the app and states the limitation everywhere.

### Progress (updated 2026-09-08)

| Item | State |
| --- | --- |
| Stage 0: freeze, stale PRs closed, `pre.1` published | done |
| Stage 1: automated pre-release, install guide, universal binary | done (`pre.1` to `pre.5`) |
| Stage 1: log file, About window, Report a Problem, Help menu | done, ships in `pre.6` |
| Stage 1: Homebrew cask | dropped: Homebrew is a developer tool; the install is a DMG with an Applications shortcut, no commands |
| Stage 1: pipeline ready for a future Developer ID | done: signing, notarization, stapling, DMG, and validation all keyed on secrets; nothing to build once the certificate exists |
| Stage 2: included Packs activate on first launch, Analyst hidden without runtime | done |
| Stage 2: Home hierarchy (Start here / My Worlds / New World / Manage Packs) and first-launch copy in user language | done |
| Stage 2: usability test with three to five non-developers | needs a real Mac and testers |
| Stage 3: long-run consequence chains in Pocket Universe | done (chapters two and three) |
| Stage 3: branch comparison reachable from Home | done ("What if…" on every World card) |
| Stage 3: three-part briefing shape in every World | done: the briefing carries "now" and "what changed", the command panel underneath is "what you can do"; Tiny Society now opens with a Harbor today state line |
| Stage 4: user-first README, CHANGELOG, known issues, privacy note | done |
| Stage 4: screenshot in the README | needs a real Mac; the CI runner renders no text |
| Stage 4: `v0.2.0` | shipped 2026-09-07 at the owner's direction, before real-device verification and the usability test; both stay open below |
| Stage 4: `v0.2.1` | shipped 2026-09-08: update banner, prefilled problem report, dark mode, standard window and app menu behaviour |
| `v0.2.2` | shipped 2026-09-08: naming and removing Worlds, a World window that carries the name, and a Library that survives one damaged file |
| `v0.3.0` | shipped 2026-09-08: find and order My Worlds, windows that reopen where they were left, Pocket Universe chapter four, and a second Tiny Society consequence chain. Both Pack versions moved, which closes Worlds saved by earlier releases |
| `v0.4.0` | shipped 2026-09-10: the era engine, three troubles per seed, a history that reads back, a World that keeps living while nobody answers, and a week of catch-up that reads as the eras it crossed. Pocket Universe moves to `0.18`, which closes Pocket Universe Worlds saved by earlier releases |

### Why this phase exists

The runtime, persistence, branching, and Pack isolation layers are solid. What is missing is the path from "downloaded the app" to "kept a World alive for a week": there is no signed download, no first-run flow that avoids Pack review dialogs, not enough content to make returning worthwhile, and no feedback channel. M247 through M263 were all transport hardening and did not change anything a user sees. That class of work is **frozen** for 0.2 unless a real user-reported bug reopens it.

### Stage 0 — Close out and freeze (week 1)

- Merge M263 once its rustfmt diff is fixed; then freeze further transport-hardening milestones.
- Close or explicitly re-scope stale PRs (#50, #51, #200, #213).
- Tag `v0.1.0-pre.1` from the current pipeline as the baseline artifact.
- Keep `NEXT_TASK.md` pointing at this phase; every task must state what a user will see change.

Accepted when: `main` is green, there are no stale open PRs, and one downloadable `pre.1` package exists.

### Stage 1 — Installable without notarization (weeks 2–3)

There is no Apple Developer ID available in the 0.2 timeframe, so the package stays **ad-hoc signed and not notarized**, exactly as `package-release.sh` and `validate_release_package.py` already require. The stage therefore optimizes the unsigned path instead of waiting on a certificate: the audience for 0.2 is technical early adopters who will follow a two-step first-open, and every download surface must say so up front.

- A tag push creates the GitHub Release and attaches the zip, SHA-256, and `release-manifest.json`. The release notes lead with the not-notarized status and link the install guide.
- `docs/INSTALL.md`: the exact first-open steps for macOS 14 and macOS 15 (open, dismiss the Gatekeeper dialog, System Settings → Privacy & Security → Open Anyway), the `xattr -dr com.apple.quarantine` fallback for a Terminal user, and the verify-the-checksum step. Test it on both macOS versions before publishing; macOS 15 removed the right-click → Open shortcut, so the guide must not rely on it.
- Local rolling log file, an About window showing version and commit with a "copy diagnostics" action, and a "Report a problem" menu item pointing at an issue template.
- Keep `release-package.yml` ready to flip: signing identity and notarization become optional inputs so that a future Developer ID is a secrets change, not a pipeline rewrite. Do not build the notarization step until the certificate exists.
- Optional: an update check. Full auto-update can wait.

Accepted when: a Mac with no developer tooling downloads the Release, follows `docs/INSTALL.md` without help, and reaches Home within two minutes; the Release page and the app's About window both state that the build is not notarized.

### Stage 2 — Sixty seconds to a living World (weeks 3–5)

- First-party included Packs are verified at build time and activated on first launch; the Review & Install flow remains only for user-supplied `.worldpack` files.
- Home shows two entries: Pocket Universe (`Start here`) and Tiny Society. Micro Company and Pack management move to a secondary level.
- Empty states and first-World copy use user language: no `Pack`, `probe`, `activation`, or `SHA-256` in the primary path.
- The Analyst panel is labeled Experimental and hidden when no runtime is detected.
- Run a five-minute usability test with three to five non-developers and record where they stall.

Accepted when: testers reach a progressing World within sixty seconds without prompting and nobody is blocked by a dialog.

### Stage 3 — Worth coming back to (weeks 5–7)

- Pocket Universe gains two or three long-run consequence chains that span multiple visits so `While you were away` has substance.
- Branch comparison is reachable from Home: "try the other choice" on the same World, then compare the outcomes side by side. This productizes existing `world-compare` and strategy capabilities without new kernel work.
- Every World's briefing follows the same three-part shape: what is happening now, what changed, what you can do.

Accepted when: a returning tester can say what changed in their World and opens a branch unprompted.

### Stage 4 — Docs and release (weeks 7–8)

- README rewritten for users: one screenshot, three steps, a download link. Architecture material moves under `docs/`.
- `CHANGELOG.md`, a known-issues list, and a privacy note (local files, no telemetry, where data goes when the Analyst is used).
- Ship `v0.2.0` with release notes that explain the replay-never-reruns-AI guarantee.

Accepted when: the Release page explains itself to a stranger and at least one external issue arrives within a week.

### Explicitly out of scope for 0.2

- Windows and Linux desktop builds.
- A Pack marketplace, third-party Pack ecosystem, or Builder.
- In-process Pi, persistent RPC sessions, or dynamic World Action tool injection.
- The stdin write-deadline redesign that M263 defers, and any further audit-driven transport hardening.
- Remote telemetry or accounts.

### Risks

- **Single maintainer plus coding agents.** Agents generate hardening tasks readily and do not judge "enough". Mitigation: every `NEXT_TASK.md` entry names the user-visible change.
- **GPUI pinned to a Zed Git revision.** Upgrading is expensive. Mitigation: do not move the pin during 0.2; keep an upgrade checklist.
- **No Apple Developer ID in the 0.2 timeframe.** The app cannot be notarized, so Gatekeeper interrupts every first launch. Mitigation: Stage 1 ships the ad-hoc package with a tested install guide and a Homebrew tap, states the status everywhere, and keeps the pipeline ready for a later certificate. Revisit notarization before any release aimed at non-technical users.
- **Pi license rider.** Mitigation: keep the Analyst out-of-process and optional and never bundle the `pi` binary, which is already the case.

## Phase VII — 0.4 "Worlds that don't end"

Status: shipped 2026-09-10 as `v0.4.0`.

### What it produced, measured the same way it was planned

```text
before   all four chapters complete by period 13, then ["pocket-universe.nudge"] forever
after    13 eras in 80 attended periods, never inert

before   an unattended World stalls at period 18 and never moves again
after    the same World reaches its third era by period 60, on its own

before   catch-up capped at 7 periods — a week away equalled two days away
after    capped at one week of World time, and a return that crossed eras says so
```

### The measurement this phase exists because of

Driving a fresh Pocket Universe World and taking every choice the moment it is offered:

```text
period 3   chose careful-path
period 6   chose rooted-posture
period 9   legacy=habitat-commons formed
period 11  chose pressure-hold
period 13  chose succession-entrust   -> all four chapters complete
--- for the next 47 periods the command list is exactly ["pocket-universe.nudge"]
```

A World's entire content is thirteen periods. After that it is permanently inert. Two related measurements:

- **Nothing happens unless the observer acts.** Advancing sixty periods without making a choice leaves the World at `legacy=forming`. Every chapter is gated behind an observer decision, so "keeps living while you are away" is currently "waits for you".
- **Catch-up is capped** at one period per six hours, at most seven periods. Leaving for a week returns the same World as leaving for two days.

A fifth chapter would move the wall from thirteen periods to sixteen. This phase removes the wall instead.

### Pillar 1 — the era engine

The four chapters are hard-chained at their call sites (`legacy` calls `pressure` calls `succession`) and end. They become the stages of one **era**, and an era loops: the successor becomes the new anchor-keeper and the next era begins.

Three mechanisms keep the loop from being a replay:

- **A pressure pool per seed.** Each era draws a different threat, never the same one twice in a row. Selection is a deterministic function of era number and history — never a random draw — so replay stays exact.
- **An era record derived from the event log.** No new serialization: `era_began`, the pressure resolution, and the succession answer are already durable Events, so a World's history of eras is a projection over its own log.
- **History read back into the present.** A World that released its legacy three times running offers a different fourth era than one that entrusted three times. This is the only thing that keeps unbounded content from becoming wallpaper.

### Pillar 2 — a World that moves without you

Every decision point gains a drift: if the observer stays away, the World reaches the default itself, marks the outcome as one it decided, and says so on return. Chapter four's patience bands are the prototype; this generalizes them. A drifted decision is exactly as durable as one you made — that is the cost, and the reason to come back.

### Pillar 3 — pacing to match

The six-hour period and seven-period cap were tuned to ration finite content. With the wall gone, the cap should be about how much narration one return should carry, and `While you were away` should summarize an era rather than list periods.

### Versioning under this phase

No multi-version Packs, no World migration, no frozen crates, no protocol change: carrying older Worlds across a rules change is explicitly not this phase's problem. Instead the era engine is designed so that **new content never changes the meaning of Events already recorded**. Pack versions exist to protect replay determinism; if content is additive, one version carries it and a version bump becomes rare rather than routine. The kernel cannot enforce this — it is a discipline, not a mechanism — but it is cheaper and more honest than building migration machinery. `v0.4` itself bumps once, because the era engine does change what the rules mean.

### Explicitly out of scope for 0.4

Multi-version Packs and World migration; a background daemon or notifications (the app being closed is a macOS background-execution and notarization problem, separate from Worlds not moving); AI-generated content; Windows and Linux; a Pack marketplace.

### Accepted when

A World left running produces a next thing to decide indefinitely, two consecutive eras never face the same threat, an era's reading depends on the eras before it, and a World left alone still moves and reports what it decided without you. All four hold, each with a test that fails if it stops holding.

## After 0.2

Decide the next phase from real usage, not from architectural interest. Candidates, in rough order:

1. Developer ID signing and notarization as soon as a certificate exists; this is the gate for any release aimed beyond technical early adopters.
2. Real-device verification, README screenshots, and the usability test. These need a physical Mac and testers; the CI screenshot runner rasterizes no text and cannot substitute. Findings from them outrank the rest of this list, and [NEXT_TASK.md](../NEXT_TASK.md) carries the ordered plan while none exist yet.
3. Carrying a World across a Pack version change. Deliberately deferred by Phase VII, which instead keeps content additive so version bumps stay rare; revisit only if a rules change genuinely has to alter what past Events mean.
4. Windows or Linux support if download requests justify the GPUI cost.
5. A second-party Pack authoring guide once first-party content proves retention.
6. Persistent Pi sessions or direct model API access for the Analyst if the Experimental panel sees use.
7. Return to transport and scheduler hardening only against reported failures.

The project should resist adding infrastructure merely because it is architecturally interesting. New runtime primitives should be justified by a product behavior that at least two different Worlds can use.
