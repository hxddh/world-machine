pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const TARGET_ARCH: &str = std::env::consts::ARCH;
pub const BUILD_COMMIT: &str = match option_env!("WORLD_MACHINE_BUILD_COMMIT") {
    Some(commit) => commit,
    None => "dev",
};

pub fn display_label() -> String {
    format_build_identity(APP_VERSION, BUILD_COMMIT, TARGET_ARCH)
}

fn format_build_identity(version: &str, commit: &str, architecture: &str) -> String {
    let commit = commit.trim();
    let commit = if commit.is_empty() { "dev" } else { commit };
    format!("Pre-alpha {version} · build {commit} · {architecture}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_identity_keeps_version_commit_and_architecture_explicit() {
        assert_eq!(
            format_build_identity("0.1.0", "abc123def456", "aarch64"),
            "Pre-alpha 0.1.0 · build abc123def456 · aarch64"
        );
    }

    #[test]
    fn blank_build_commit_is_a_development_build() {
        assert_eq!(
            format_build_identity("0.1.0", "   ", "x86_64"),
            "Pre-alpha 0.1.0 · build dev · x86_64"
        );
    }

    #[test]
    fn compiled_build_identity_exposes_the_compiled_constants() {
        let label = display_label();
        assert!(label.contains(APP_VERSION));
        assert!(label.contains(BUILD_COMMIT.trim()));
        assert!(label.ends_with(TARGET_ARCH));
    }
}
