//! Where this build is distributed, and so what it may ask the internet.
//!
//! A build from GitHub (the default) checks GitHub's releases once a launch
//! and offers a newer one on Home. A store build (`--features
//! channel-store`, for Steam) is updated by the store, so it never asks
//! GitHub anything: the check is compiled off, not just hidden.

/// Where a build is distributed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channel {
    /// Downloaded from GitHub's releases (or built from source).
    Direct,
    /// Installed and updated by a store, such as Steam.
    Store,
}

/// This build's channel.
pub const CURRENT: Channel = if cfg!(feature = "channel-store") {
    Channel::Store
} else {
    Channel::Direct
};

impl Channel {
    /// Whether a build on this channel may ask GitHub for a newer release.
    pub const fn checks_github_for_updates(self) -> bool {
        matches!(self, Channel::Direct)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_direct_build_asks_github_for_updates() {
        assert!(Channel::Direct.checks_github_for_updates());
        assert!(!Channel::Store.checks_github_for_updates());
        assert_eq!(
            CURRENT == Channel::Store,
            cfg!(feature = "channel-store"),
            "the feature and the channel disagree"
        );
    }
}
