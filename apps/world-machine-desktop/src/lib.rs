//! Non-View native product ownership for World Machine desktop features.
//!
//! UI code consumes the analyst session API here instead of owning Pi, Node,
//! child-process, or protocol lifecycle details directly.

pub mod analyst_session;
