//! Non-View native product ownership for World Machine desktop features.
//!
//! UI code consumes the persisted settings, sound and window-state APIs here
//! instead of owning filesystem persistence or audio details directly.

pub mod ambience;
pub mod app_settings;
pub mod demo;
pub mod display;
pub mod key_store;
pub mod window_state;
