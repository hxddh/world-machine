# world-pack-server

Authoring adapter for external Rust World Packs.

A Pack author keeps implementing the ordinary `WorldRegistration` / `WorldSession` Host surface. `serve_stdio(registration)` turns that registration into the versioned World Machine JSONL process protocol; the World implementation itself does not parse protocol messages or know about the external-process transport.

A Pack executable can expose `--print-manifest` with `manifest_for_current_exe(&registration.descriptor)`. The resulting v1 manifest points directly at the current executable with no runtime arguments, so it is compatible with the installed Pack catalog's explicit approval, managed-copy, and content-pin model.

`tiny-society-pack` is the executable reference implementation: the same Tiny Society registration used as a built-in World is served out-of-process without changing its World implementation.
