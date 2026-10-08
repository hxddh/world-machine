//! Non-View native product ownership for World Machine desktop features.
//!
//! UI code consumes the persisted settings, sound and window-state APIs here
//! instead of owning filesystem persistence or audio details directly.

#![forbid(unsafe_code)]

pub mod ambience;
pub mod app_settings;
pub mod channel;
pub mod demo;
pub mod display;
pub mod key_store;
pub mod platform;
pub mod window_state;
