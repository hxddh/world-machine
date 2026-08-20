//! Non-View native product ownership for World Machine desktop features.
//!
//! UI code consumes the analyst session, readiness, and persisted runtime-settings APIs here
//! instead of owning Pi, Node, child-process, PATH resolution, filesystem persistence, or
//! protocol lifecycle details directly.

pub mod analyst_readiness;
pub mod analyst_session;
pub mod analyst_settings;
