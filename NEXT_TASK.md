# Next Coding Task — M253 Collision-Proof world-cli Integration Test Fixtures

M214–M252 now provide the installed World Analyst path through immutable saved-World evidence, restricted long-lived Pi execution, provider-neutral turns, complete retained session evidence, bounded 4096-byte UI previews, incremental panel projection, variable-height virtualized history, retained/no-copy successful turns, and a bounded 64 MiB Analyst transport response frame before JSON/protocol validation.

While validating M251, the authoritative Linux workspace gate exposed a separate test-infrastructure reliability problem in `crates/world-cli/tests/machine_query_transport.rs`. The first run failed one integration test with `ENOENT`; rerunning the exact same commit passed the entire workspace and Pack conformance gate. This should be fixed separately rather than treated as acceptable CI noise.

## M253 — make temporary World archive paths collision-proof within the test process

The observed failure was:

- test: `stdin_neighborhood_and_shortest_path_queries_emit_typed_json`;
- the first neighborhood invocation using the fixture path succeeded;
- the immediately following shortest-path invocation using the same `path` returned a CLI error whose stderr was `Os { code: 2, kind: NotFound, message: "No such file or directory" }`;
- specifically, the failure was the second `assert!(output.status.success(), ...)` after `run_query` for `ShortestPath`, so the CLI binary had started successfully and the same archive path had already worked for the first query;
- a same-head rerun passed without any code change.

The fixture helper currently builds names only from process ID plus the current wall-clock nanosecond value:

```rust
fn temp_world_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-m188-{}-{nonce}.world",
        std::process::id()
    ))
}
```

Rust integration tests in this binary can execute concurrently in the same process. Every test also removes its fixture path when finished. If two fixture calls observe the same clock value, they produce the same filename; one test can then delete the archive while another still intends to reuse it. The exact M251 failure is consistent with that mechanism: the archive existed for the first CLI invocation and was gone before the second invocation in the same test.

### Product / test behavior

This milestone is test-infrastructure only:

- every call to `temp_world_path()` within one test process must produce a distinct path, even if multiple calls observe the exact same wall-clock timestamp;
- preserve the existing temp-directory location and `.world` archive shape;
- keep cleanup behavior best-effort and local to the tests;
- do not serialize the entire test suite as a workaround;
- do not add sleeps, retries around missing files, or ignore the failure;
- do not change `world-cli`, World archive semantics, query behavior, Pack behavior, or production code.

### Recommended implementation

Keep the change local to `crates/world-cli/tests/machine_query_transport.rs` and add a process-local atomic sequence to the filename, for example:

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_WORLD_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn temp_world_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = TEMP_WORLD_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "world-machine-m188-{}-{nonce}-{sequence}.world",
        std::process::id()
    ))
}
```

`Relaxed` ordering is sufficient because the atomic is only an identity allocator; no memory synchronization depends on it. Keeping PID + timestamp + sequence also protects reasonably against stale files from prior processes while guaranteeing uniqueness for concurrent calls in the current process.

Do not introduce a new dependency solely for this fix unless the existing workspace already has an intentionally shared temp-file abstraction that is clearly preferable.

### Validation

Required regressions:

- add a deterministic or source-level regression proving a process-local monotonic discriminator participates in the path, so equal timestamps cannot alias;
- preferably add a focused uniqueness test that generates many fixture paths concurrently and proves there are no duplicates without creating/removing archive contents;
- `stdin_neighborhood_and_shortest_path_queries_emit_typed_json` remains green and continues reusing its own one fixture path for both CLI requests;
- all other `machine_query_transport` tests remain green;
- run the affected integration test repeatedly or otherwise exercise parallel test execution enough to make the original cross-test deletion mechanism observable if it regresses;
- Linux boundary/Pi/fmt/Clippy/workspace/Pack gates remain green;
- full macOS Library/Packs/GPUI/desktop tests plus `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

### Scope audit

Before editing, inspect the rest of `crates/world-cli/tests` for another helper with the same PID + wall-clock-only naming pattern. If an identical local pattern exists, either share the same narrow test helper or fix the directly equivalent collision in the same milestone. Do not expand into a repository-wide test utility refactor without concrete evidence.

## Non-goals

No production temp-file behavior changes, no CLI query redesign, no World archive changes, no test-suite global mutex, no disabling parallel tests, no retries/sleeps, no ignored failures, no Analyst protocol/session/UI changes, no provider/model/Pi changes, and no Pack/World product behavior changes.
