# Next Coding Task — M10 Tiny Society Product Loop

Turn the M9 projection shell into the first product-shaped Tiny Society experience without weakening World Machine boundaries.

Requirements:

1. Add a **Society Today / While You Were Away** projection derived from real Events; it must summarize existing history rather than invent events.
2. Add a generic **Why?** projection that walks `Event.caused_by` and exposes a causal chain/graph for the selected Event.
3. Add **Fork here** at an Event boundary using the existing `World::fork_after` semantics; the UI must make the branch point explicit.
4. Keep `world-gpui` generic. Any Tiny Society story labels, resident emphasis, or layout choices belong in the Tiny Society projection adapter or app layer.
5. Do not give GPUI authoritative World mutation access. User commands must become explicit World actions/branch operations outside the renderer.
6. Keep Pi optional. M10 must remain fully usable with the deterministic/mock-agent story.
7. Add tests proving the Why projection follows persisted `caused_by` links and the forked world excludes post-branch events.
8. The first end-to-end demo should support: open Tiny Society -> inspect Mara/Jonas -> select `worker_dismissed` -> Why? -> fork before dismissal -> inspect the alternative state.
9. Do not add marriage, children, politics, crime, procedural map generation, or more residents yet.
10. Linux workspace CI and macOS GPUI compile CI must remain green.
