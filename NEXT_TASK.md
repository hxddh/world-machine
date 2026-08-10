# Next Coding Task — M9 GPUI Projection Shell

Build the first renderer adapter without moving World truth into GPUI.

Requirements:

1. Add `world-gpui` as an adapter over generic projection/read models; it must not define authoritative World state.
2. Keep `world-core` and `world-agent` free of GPUI dependencies.
3. Start with four generic surfaces only: Collection, Inspector, Timeline, minimal Semantic Canvas.
4. Tiny Society may supply projection data/configuration but must not require Tiny-Society-specific renderer types.
5. The first visible vertical slice should render the existing causal story and allow selecting an Event/Entity for inspection.
6. Do not attempt a universal game engine, arbitrary generated GUI, or 3D renderer.
7. Add headless tests for projection models separately from graphical smoke tests.
8. Keep Pi integration optional; M9 must render the deterministic/mock-agent World without a model.
9. Document the GPUI Apache-2.0 dependency and its exact crate boundary.
10. GitHub CI must remain green; platform-specific GPUI build strategy should be explicit.
