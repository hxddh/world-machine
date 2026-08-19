#[test]
fn analyst_client_has_only_transport_and_wire_dependencies() {
    let manifest = include_str!("../Cargo.toml").to_ascii_lowercase();
    let production = include_str!("../src/lib.rs").to_ascii_lowercase();

    for forbidden in [
        "world-agent =",
        "world-core",
        "world-projection",
        "world-query",
        "world-investigation",
        "world-pi-rpc",
        "gpui",
        "openai",
        "anthropic",
        "reqwest",
        "hyper",
        "tokio",
        "websocket",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "analyst client manifest contains forbidden dependency token {forbidden}"
        );
    }

    for forbidden in [
        "agentruntime",
        "world_action",
        "projectionsnapshot",
        "world_projection",
        "world_core",
        "world_query",
        "pi_rpc",
        "tool_execution_start",
        "agent_settled",
        "world_first_divergence",
        "openai",
        "anthropic",
        "reqwest",
        "hyper::",
        "tokio",
        "websocket",
    ] {
        assert!(
            !production.contains(forbidden),
            "analyst client production source contains forbidden authority/provider token {forbidden}"
        );
    }
}
