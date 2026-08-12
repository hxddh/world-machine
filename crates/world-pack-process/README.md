# world-pack-process

Process-isolated adapter for external World Packs. Discovery reads validated manifests only; Pack code is launched only when a World session is explicitly created or opened. Manifest and executable paths are canonicalized before registration so process launch never falls back to PATH lookup.

The adapter preserves `WorldArchive` as durable truth and exposes the remote Pack through the generic `WorldSession` / `WorldPackSource` Host interfaces. Protocol responses are size-bounded and requests use a bounded timeout; hung, malformed, disconnected, or request-id-desynchronized children are terminated instead of leaving a poisoned session alive.

This crate is a first-class workspace member, so its process integration tests and `-D warnings` Clippy checks run under the repository's normal CI rather than a separate plugin-only path.
