use std::path::PathBuf;
use std::process::Command;

#[test]
fn pi_analyst_extension_passes_transport_and_authority_checks() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("world-agent-tool-stdio should live under crates/")
        .to_path_buf();
    let script = repo_root.join("scripts/check-pi-analyst.sh");

    let output = Command::new("bash")
        .arg(&script)
        .current_dir(&repo_root)
        .output()
        .expect("Pi analyst boundary test requires bash and Node.js");

    assert!(
        output.status.success(),
        "Pi analyst checks failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
