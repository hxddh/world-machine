# world-pack-process

Process-isolated adapter for external World Packs. Discovery reads validated manifests only; Pack code is launched only when a World session is explicitly created or opened. The adapter preserves `WorldArchive` as durable truth and exposes the remote Pack through the generic `WorldSession` / `WorldPackSource` Host interfaces.
