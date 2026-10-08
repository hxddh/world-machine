//! Sets `cfg(gui)` where the app has a window toolkit: macOS and Windows,
//! and Linux when the `linux-window` feature asks for it (the screenshot
//! harness and previews run the real window under Xvfb).
//!
//! The app's windowed code is written against `cfg(gui)` rather than
//! `target_os = "macos"`, because what it needs is "a GPUI platform", not
//! macOS. What is genuinely Apple-only is gated on `target_os = "macos"`
//! by itself, and what differs between operating systems goes through
//! `platform.rs`.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(gui)");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let linux_window = std::env::var_os("CARGO_FEATURE_LINUX_WINDOW").is_some();
    if os == "macos" || os == "windows" || (os == "linux" && linux_window) {
        println!("cargo::rustc-cfg=gui");
    }
}
