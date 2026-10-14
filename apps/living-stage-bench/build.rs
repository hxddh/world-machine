//! Sets `cfg(gui)` where there is a window toolkit: macOS and Windows,
//! and Linux with the `linux-window` feature (as the desktop app does).

fn main() {
    println!("cargo::rustc-check-cfg=cfg(gui)");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let linux_window = std::env::var_os("CARGO_FEATURE_LINUX_WINDOW").is_some();
    if os == "macos" || os == "windows" || (os == "linux" && linux_window) {
        println!("cargo::rustc-cfg=gui");
    }
}
