//! Non-View native product ownership for World Machine desktop features.
//!
//! UI code consumes the analyst session and readiness APIs here instead of owning Pi, Node,
//! child-process, PATH resolution, or protocol lifecycle details directly.

pub mod analyst_readiness;
pub mod analyst_session;
