#[test]
fn local_investigation_adapter_has_no_agent_or_provider_dependencies() {
    let manifest = include_str!("../Cargo.toml").to_ascii_lowercase();
    for forbidden in [
        "world-agent =",
        "world-agent-tools",
        "world-agent-tool-host",
        "world-pi-rpc",
        "gpui",
        "openai",
        "anthropic",
        "reqwest",
        "hyper",
        "axum",
        "tokio",
        "websocket",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "local investigation adapter contains forbidden dependency {forbidden}"
        );
    }
}
