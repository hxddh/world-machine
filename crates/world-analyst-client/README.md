# world-analyst-client

`world-analyst-client` is the native Rust client for `world-machine-analyst-turns@1`.

It intentionally sits above the M220 process boundary. Callers provide a prompt and receive one completed analyst turn containing final text, canonical World Machine tool calls, and normalized runtime errors. The crate does not parse Pi events, register tools, inspect World state, or execute evidence queries itself.

The process wrapper binds the left/right archive pair and optional provider/model/thinking configuration when the M220 host starts. Individual `ask` calls contain only the prompt and optional timeout.

Session policy is fail-closed: malformed/unknown responses, protocol or version mismatch, wrong request correlation, EOF, and fatal remote errors poison the session. A correlated non-fatal `command` rejection is the only remote failure that permits reuse.
