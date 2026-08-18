#[test]
fn production_manifest_has_only_transport_neutral_tool_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    let dependencies = manifest
        .split_once("[dependencies]")
        .expect("host manifest must declare production dependencies")
        .1
        .split_once("[dev-dependencies]")
        .expect("host manifest keeps test-only query dependencies separate")
        .0;

    assert!(dependencies.contains("serde ="));
    assert!(dependencies.contains("serde_json ="));
    assert!(dependencies.contains("world-agent-tools ="));

    for forbidden in [
        "world-agent =",
        "world-projection",
        "world-core",
        "world-pi-rpc",
        "gpui",
        "openai",
        "anthropic",
        "reqwest",
        "hyper",
        "axum",
        "tokio",
    ] {
        assert!(
            !dependencies.to_ascii_lowercase().contains(forbidden),
            "production host dependency boundary contains forbidden token {forbidden}"
        );
    }
}

#[test]
fn production_source_has_no_in_world_or_provider_authority() {
    let source = include_str!("../src/lib.rs");
    let production = source
        .split_once("#[cfg(test)]")
        .expect("host source keeps test-only integration fixtures behind cfg(test)")
        .0
        .to_ascii_lowercase();

    for forbidden in [
        "agentruntime",
        "agentobservation",
        "projectionsnapshot",
        "world_projection",
        "world_core",
        "pi_rpc",
        "openai",
        "anthropic",
        "reqwest",
        "hyper::",
        "axum",
        "tokio",
        "websocket",
    ] {
        assert!(
            !production.contains(forbidden),
            "production host source contains forbidden authority token {forbidden}"
        );
    }
}
