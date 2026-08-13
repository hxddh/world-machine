# World Pack Check

`world-pack-check` is the developer-facing conformance command for external World Machine Packs.

It validates the same trust and durability boundary used by the World Machine desktop without installing the Pack into the user's real catalog.

## Static inspection

Use static inspection when you want to validate the Pack envelope/manifest and the exact executable identity without running Pack code:

```bash
cargo run -p world-pack-catalog --bin world-pack-check -- \
  --inspect-only path/to/example.worldpack
```

The command reports:

- exact Pack id and version;
- title;
- install format;
- executable name and size;
- executable SHA-256;
- resolved source path.

`--inspect-only` does not execute the Pack.

## Durable conformance check

Run the default mode before distributing a Pack:

```bash
cargo run -p world-pack-catalog --bin world-pack-check -- \
  path/to/example.worldpack
```

The command first performs the same static inspection, then:

1. creates a temporary isolated Pack catalog;
2. copies the reviewed bytes into that catalog's managed Pack store;
3. keeps the installed Pack disabled/non-active;
4. executes only the bounded durable activation probe;
5. verifies the exact Describe identity;
6. creates a World and snapshots it;
7. captures a durable archive;
8. drops the first process/session;
9. opens the archive through a fresh Pack process;
10. verifies the reopened World time and exact archive round-trip;
11. removes the temporary catalog and managed copy when the command exits.

The source `.worldpack` or developer manifest is never modified.

## What this proves

A passing durable check proves the minimum external Pack contract required by World Machine:

- the approved executable bytes can launch;
- the runtime identifies itself as the exact Pack id/version being checked;
- `Create` returns a valid projection snapshot;
- the created World can be archived;
- a fresh Pack process can reopen that archive;
- reopen preserves durable World time and archive bytes exactly.

It does **not** prove that every Pack command is correct, that background progression is useful, or that the Pack is safe. Executing an external Pack remains a trust decision.

## Authoring boundary

External Pack authors should continue to implement the ordinary public `WorldRegistration` / `WorldSession` surface and use `world-pack-server` for stdio serving and `.worldpack` generation. The check command deliberately reuses `world-pack-catalog` and `world-pack-process` rather than introducing a second protocol or a Pack-specific test API.

The repository's Tiny Society, Pocket Universe, and Micro Company external Pack tests remain deeper product regressions. `world-pack-check` is the common minimum gate that any unrelated Pack can run before those domain-specific tests exist.
