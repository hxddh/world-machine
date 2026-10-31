//! How the app's memory allocator hands large blocks back.
//!
//! The window paints its still layers and bakes the living stage's looks
//! as pictures of a few kilobytes to tens of megabytes, on the painter's
//! and the bakers' threads, and lets them go on others. On Linux with
//! glibc, malloc serves a block of at least its mmap threshold (128 KiB at
//! first) with a mapping of its own and unmaps it on free. But the
//! threshold is dynamic: every such block freed raises it to the block's
//! size, up to 32 MiB. After the first look is let go, nearly every
//! picture is carved out of malloc's per-thread arenas instead, where the
//! freed pixels stay with the process: spread over a dozen arenas,
//! fragmented between pictures that are still shown, and given back only
//! from the top of each. Resident memory then grows for as long as the
//! World lives (the nightly soak: 253 MB an hour over two hours; nothing
//! the app holds grew).
//!
//! [`hand_back_large_blocks`] fixes the threshold at glibc's own default,
//! which also turns its raising off: a picture's pixels are mapped for it
//! and unmapped when it goes. Elsewhere (macOS, Windows, other C
//! libraries) it does nothing.
//!
//! This is the only `unsafe` in the package (one libc call), which is why
//! the crate denies `unsafe_code` and allows it in this module alone.

/// Blocks at least this large are mapped for themselves and unmapped when
/// freed: glibc's own default threshold, held where it starts.
pub const LARGE_BLOCK: usize = 128 * 1024;

/// Fixes the allocator's threshold for large blocks at [`LARGE_BLOCK`], so
/// freed pictures go back to the system. Called first thing in `main`,
/// before any thread starts; calling it again is harmless. Returns whether
/// the allocator took it (false where there is nothing to set).
pub fn hand_back_large_blocks() -> bool {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        // SAFETY: mallopt only sets one of malloc's parameters; it takes
        // malloc's own lock and touches no memory of ours.
        let threshold = libc::c_int::try_from(LARGE_BLOCK).unwrap_or(libc::c_int::MAX);
        unsafe { libc::mallopt(libc::M_MMAP_THRESHOLD, threshold) == 1 }
    }
    #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
    {
        false
    }
}
