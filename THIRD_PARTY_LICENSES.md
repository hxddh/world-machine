# Third-party licenses

World Machine's `world-core`, `world-agent`, and `world-projection` crates remain free of third-party runtime dependencies beyond the standard library and internal workspace crates.

Current dependencies / integrations:

| Dependency / integration | Upstream license | Boundary |
| --- | --- | --- |
| serde_json | MIT OR Apache-2.0 | `world-pi-rpc` protocol parsing only |
| external pi_agent_rust binary | MIT-derived license with OpenAI/Anthropic rider | optional out-of-process runtime; not linked or redistributed by this crate |
| GPUI | Apache-2.0 | optional `world-gpui` renderer; pinned to Zed revision `4e8057d74db3570b3bd419ff296eb84c35b3a5a3` |
| gpui_platform | Apache-2.0 | macOS application bootstrap only; pinned to the same Zed revision as GPUI |

This file records World Machine's dependency boundary. Packaged distributions must include the complete notices required for every dependency or external runtime they actually redistribute.
