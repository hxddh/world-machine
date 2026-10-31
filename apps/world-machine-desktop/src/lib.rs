//! Non-View native product ownership for World Machine desktop features.
//!
//! UI code consumes the persisted settings, sound and window-state APIs here
//! instead of owning filesystem persistence or audio details directly.

#![deny(unsafe_code)]

// The only `unsafe` in the package: one libc call that tells the
// allocator to hand large freed blocks back (see the module).
#[allow(unsafe_code)]
pub mod allocator;
pub mod ambience;
pub mod app_settings;
pub mod channel;
pub mod crash_reports;
pub mod demo;
pub mod display;
pub mod key_store;
pub mod notices;
pub mod plain_errors;
pub mod platform;
pub mod restore;
pub mod settings_plan;
pub mod shell;
pub mod steam;
pub mod window_state;
