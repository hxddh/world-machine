# Third-party licenses

World Machine's `world-core` and `world-agent` crates remain free of third-party Rust dependencies.

Current adapter dependency:

| Dependency / integration | Upstream license | Boundary |
| --- | --- | --- |
| serde_json | MIT OR Apache-2.0 | `world-pi-rpc` protocol parsing only |
| external pi_agent_rust binary | MIT-derived license with OpenAI/Anthropic rider | optional out-of-process runtime; not linked or redistributed by this crate |

Planned integration:

| Integration | Upstream license | Planned boundary |
| --- | --- | --- |
| GPUI | Apache-2.0 | optional `world-gpui` renderer crate |

This file records World Machine's dependency boundary. Packaged distributions must include the complete notices required for every dependency or external runtime they actually redistribute.
