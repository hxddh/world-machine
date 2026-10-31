//! Large freed blocks go back to the system (src/allocator.rs): the soak's
//! growth was glibc keeping freed pictures once its dynamic mmap threshold
//! had risen past them. One test in its own process, so nothing else
//! allocates beside it.

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[test]
fn a_picture_let_go_after_a_larger_one_is_handed_back() {
    use world_machine_desktop::allocator::{hand_back_large_blocks, LARGE_BLOCK};

    const MB: usize = 1 << 20;
    /// Resident memory now, in bytes.
    fn resident() -> usize {
        let status = std::fs::read_to_string("/proc/self/status").expect("/proc/self/status");
        let kb = status
            .lines()
            .find_map(|line| line.strip_prefix("VmRSS:"))
            .and_then(|rest| {
                rest.trim()
                    .trim_end_matches("kB")
                    .trim()
                    .parse::<usize>()
                    .ok()
            })
            .expect("VmRSS");
        kb * 1024
    }
    /// A picture's pixels, every byte written (so all of it is resident).
    fn picture(bytes: usize) -> Vec<u8> {
        let pixels = vec![0x5a_u8; bytes];
        std::hint::black_box(pixels)
    }

    assert!(hand_back_large_blocks(), "glibc takes the threshold");
    const { assert!(LARGE_BLOCK <= 128 * 1024) };
    // A whole-place look let go: with glibc's dynamic threshold, freeing
    // it would raise the threshold to its size.
    drop(picture(30 * MB));
    // Then pictures smaller than it, each let go in turn: every one is
    // handed back, not kept by the allocator.
    let before = resident();
    let mut most = 0;
    for size in [24 * MB, 16 * MB, 9 * MB, 3 * MB, MB / 2] {
        drop(picture(size));
        most = most.max(resident().saturating_sub(before));
    }
    assert!(
        most < 2 * MB,
        "freed pictures were kept: resident memory {} MB above where it began",
        most / MB
    );
}
