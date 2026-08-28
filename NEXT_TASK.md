# Next Coding Task — M258 Bound world-cli Machine-Query Stdin

M254–M257 close the unbounded JSONL transport edges in the shipped Analyst path: Pi RPC child stdout, restricted tool-host stdout, Node turn-host stdin, and finally Rust sender pre-write request sizing. The next confirmed whole-input boundary is separate from Analyst transport: `crates/world-cli/src/main.rs::read_query_request()`.

Today all three machine-query commands accept `-` as the request argument and then do:

```rust
let mut json = String::new();
io::stdin().read_to_string(&mut json)?;
Ok(json)
```

That means an oversized or indefinitely unterminated stdin document can grow an unbounded `String` before UTF-8/JSON/request-shape validation.

## M258

Add a fixed **64 MiB stdin request-document byte ceiling** for the `-` path used by:

- `world-cli evidence-query <file.world> -`
- `world-cli evidence-compare-query <left.world> <right.world> -`
- `world-cli evidence-investigate-compare <left.world> <right.world> -`

This is a transport/runaway-memory guard, not a new semantic query-size recommendation. Keep direct JSON supplied as a normal command-line argument unchanged; M258 only bounds bytes read from stdin when the request argument is exactly `-`.

### Required semantics

- Count raw stdin bytes before UTF-8/JSON parsing.
- Maximum payload is exactly 64 MiB; there is no JSONL delimiter or framing byte to exclude because this input is one EOF-terminated JSON document.
- Exactly the configured limit at EOF is accepted.
- Limit + 1 bytes must fail as soon as that byte is observed; do not keep reading until EOF merely to report overflow.
- Never truncate an oversized document and parse the prefix.
- Do not preallocate 64 MiB for normal requests.
- Preserve current EOF behavior: stdin is one complete document and normal execution still waits for EOF when the document stays within the limit.
- Preserve UTF-8 validation before JSON decoding. Invalid UTF-8 within the size ceiling remains an input error rather than lossy conversion.
- Preserve the existing JSON request schemas, query execution, status envelopes, archive semantics, and exit behavior for valid requests.
- Ensure overflow is rejected before JSON deserialization and before any query execution that could depend on the supplied request.

A private constant such as:

```rust
const MAX_MACHINE_QUERY_STDIN_BYTES: usize = 64 * 1024 * 1024;
```

is sufficient. Do not add a CLI flag, environment variable, or user/runtime option to raise the production ceiling.

### Suggested implementation shape

Refactor the stdin-specific reading into a small testable helper that accepts a generic `Read`, for example:

```rust
fn read_bounded_query_request<R: Read>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<String, Box<dyn Error>>
```

or an equivalent private typed-error shape.

A simple bounded strategy is appropriate:

1. wrap the reader in `Read::take(max_bytes + 1)`;
2. `read_to_end` into a `Vec<u8>` without large preallocation;
3. if `len > max_bytes`, return a clear local CLI/input error immediately;
4. otherwise convert with `String::from_utf8` (or equivalent strict UTF-8 conversion) and return the full document.

Using `take(max + 1)` means an endless producer fails once the first impossible byte arrives instead of requiring EOF. Avoid `read_to_string` on an unbounded reader and avoid reading the full input only to check length afterward.

Production `read_query_request()` should call the helper only when `request == "-"`; non-stdin request strings must remain untouched.

### Required regressions

Use a small injected helper limit for deterministic unit tests; do not allocate tens of MiB.

1. below-limit EOF input is returned unchanged;
2. exact-limit EOF input is accepted;
3. limit + 1 bytes are rejected and the underlying reader is not consumed beyond `max + 1` bytes;
4. a reader that can continue indefinitely is rejected once overflow becomes certain rather than waiting for EOF;
5. invalid UTF-8 within the limit is rejected, not lossily converted;
6. valid multibyte UTF-8 is counted by bytes and returned intact;
7. direct `request-json` argument behavior is unchanged and does not go through the stdin ceiling helper;
8. stdin `evidence-query` still emits the same typed JSON status envelope for a valid request;
9. stdin `evidence-compare-query` still works for existing legacy/tagged causal comparison requests;
10. stdin `evidence-investigate-compare` still works for first-divergence investigation requests;
11. oversized stdin fails before request JSON deserialization/query execution and emits no successful machine-query envelope;
12. all existing machine-query transport/causal/investigation integration tests remain green.

Where practical, add the helper-level boundary tests inside `main.rs` and add one CLI integration regression in `crates/world-cli/tests/machine_query_transport.rs` proving the `-` path is actually wired to the bounded helper.

Current `machine_query_transport.rs` already protects temporary fixture paths with an `AtomicU64` sequence and has a cross-thread uniqueness regression. Preserve that existing fix; it is not part of M258.

### Validation

Run:

- `cargo fmt --all -- --check`;
- `cargo test -p world-cli` including `machine_query_transport` and causal/investigation suites;
- workspace Clippy/tests and Pack conformance;
- existing boundary and Pi checks to ensure the unrelated Analyst path remains untouched;
- full macOS Library/Packs/GPUI/desktop/`World Machine.app` build, packaged Analyst smoke, archive and artifact upload because repository merge policy requires the full gate even though M258 is CLI-only.

## M257 invariants to preserve

Do not mix Analyst transport changes into M258:

- Rust `AnalystTurnClient` retains its fixed 64 MiB serialized request pre-write ceiling;
- oversized Analyst requests remain local/non-fatal, write zero bytes, do not poison or consume request ids, and do not stop `AnalystTurnProcess`;
- M256 Node turn-host stdin remains independently bounded to 64 MiB JSON payload bytes;
- M254/M255 response-side framing and poisoning behavior remain unchanged.

## Later audit candidates

After M258, continue a repository-wide audit for whole-input or line-oriented external/process boundaries that can accumulate without a production cap. Treat each independently according to its protocol semantics rather than introducing a broad shared framing abstraction.

## Non-goals

No query protocol/version/schema changes, no archive input-size policy change, no direct argv JSON limit, no output truncation, no World/Pack/query algorithm changes, no Analyst transport changes, no UI changes, and no broad shared I/O framework refactor.
