# world-pack-server

Authoring adapter for external Rust World Packs.

A Pack author keeps implementing the ordinary `WorldRegistration` / `WorldSession` Host surface. `serve_stdio(registration)` turns that registration into the versioned World Machine JSONL process protocol; the World implementation itself does not parse protocol messages or know about the external-process transport.

A Pack executable can expose `--print-manifest` with `manifest_for_current_exe(&registration.descriptor)`. The resulting v1 manifest points directly at the current executable with no runtime arguments, so it is compatible with the installed Pack catalog's explicit approval, managed-copy, and content-pin model.

Executable examples live outside this generic crate so the authoring adapter remains independent of any concrete World implementation.

For distribution, `write_current_exe_bundle(&registration.descriptor, path)` writes a portable single-file `.worldpack`. Bundle v1 embeds exactly one executable, rewrites runtime identity to the bundle-owned `program`, and carries no launcher arguments. Desktop installation parses and verifies the bundle before materializing it into the managed Pack store; it does not execute source bundle code during installation.
