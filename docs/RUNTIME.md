# Runtime overview

What the workspace implements today, one boundary at a time. This is the developer-facing companion to [ARCHITECTURE.md](../ARCHITECTURE.md); the user-facing summary lives in the [README](../README.md).

The repository implements:

`Entity / Relation -> Action -> Event -> State -> Scheduler / Behavior / Agent -> Replay -> Projection`

The renderer boundary is explicit:

`World -> ProjectionSnapshot -> GPUI`

- `world-projection` defines headless Collection / Timeline / Inspector / Semantic Canvas read models.
- World Packs produce their own projection data without introducing domain concepts into the renderer.
- `world-gpui` consumes only projection models; it does not own World truth and does not depend on `world-core` or a specific Pack.
- The generic GPUI renderer treats Briefing + Commands as the current focus: the next available continuation/choice appears before Canvas and Inspector, while `Explore the world` keeps the semantic state inspectable underneath.
- Projection layout uses three independent vertical scroll regions for Collection, Focus/Explore, and Timeline, so long worlds remain inspectable without moving the fixed World header or hiding current actions.
- Projection selection defaults to semantic Collection entities and preserves explicit user selection across snapshot updates when it remains valid; Timeline events become the fallback or an explicit investigation path rather than an automatic post-command focus.
- Empty/unseeded Worlds render as a focus-only Briefing/Commands surface: empty Collection, Timeline, Canvas, and Inspector chrome stay hidden until the Pack exposes semantic content.
- `world-machine-desktop` hosts durable `.world` documents, branching/lineage, external Pack installation, durable activation probing, and generic World creation.
- Tiny Society, Pocket Universe, and Micro Company exercise the same public Host/Pack boundaries.
- The macOS app bundle carries Pocket Universe and Micro Company as **included external Packs**. They are not built-ins: on first launch Home installs them through the same content-pin, durable-probe, and activation path as any other Pack, without a review dialog, because the bundle is one code-signed unit. User-supplied `.worldpack` files still require an explicit Review & Install action before any Pack code runs.
- A fresh packaged Home presents **Pocket Universe** as the primary `Start here` experience (`Seed a place · Let it live · Branch what happens next`) while keeping Micro Company as a secondary World and moving Pack management behind the World/product hierarchy.
- Pocket Universe 0.10 turns its opening generations into a guided first story: observe the first cycle, notice the central relationship forming, then choose whether to steer it or leave the World alone. Larger interventions are presented as optional branches rather than required progress.
- Pocket Universe 0.16 adds a third chapter. Once a World's legacy has reinforced itself, a seed-specific pressure rises against the World's anchor (the reclaimer, the arcade lease, the bridge span), warns for two generations, peaks, and after three more generations durably costs the anchor. The observer can hold with what the World has or reach beyond it; the answer is recorded as aligned or strained against the World's direction. A lost anchor can be recovered, at the cost of the legacy's reinforcement cycles.
- Portable `.worldpack` files are registered as a native macOS file type. Double-clicking a Pack, using Open With, or opening it through the app routes the file into the same static review surface; the open event itself never installs or executes Pack code.
- After a newly installed Pack passes the durable probe and becomes active, Home offers an explicit `Create <World>` handoff. The probe still does not create a user World automatically; the CTA is ephemeral and only remains valid while that exact Pack version is enabled, active, content-valid, and registered.
- Pi remains an optional out-of-process `world-pi-rpc` AgentRuntime adapter.
