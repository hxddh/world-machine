#[test]
fn stdio_adapter_has_only_leaf_transport_authority() {
    let manifest = include_str!("../Cargo.toml");
    let dependencies = manifest
        .split_once("[dependencies]")
        .unwrap()
        .1
        .split_once("[dev-dependencies]")
        .unwrap()
        .0
        .to_ascii_lowercase();

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
        "websocket",
    ] {
        assert!(
            !dependencies.contains(forbidden),
            "stdio production dependency boundary contains forbidden token {forbidden}"
        );
    }

    let source = include_str!("../src/main.rs");
    let production = source
        .split_once("#[cfg(test)]")
        .unwrap()
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
            "stdio production source contains forbidden authority token {forbidden}"
        );
    }
}
