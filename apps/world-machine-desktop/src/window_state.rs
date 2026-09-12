//! Where the app's windows were left, so they open there again.
//!
//! Only geometry lives here: the position and size of the Home window and of
//! the most recent World window. Nothing about a World's content is recorded,
//! and a missing, damaged, or now-impossible entry simply means the window
//! opens at its default place.

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const WINDOW_STATE_VERSION: u32 = 1;
const WINDOW_STATE_FILE_NAME: &str = "Window State.json";
static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// A window smaller than this is not usable, and one larger than this is not a
/// window anybody dragged: both mean the stored entry should be ignored.
const MIN_WINDOW_EDGE: f32 = 400.0;
const MAX_WINDOW_EDGE: f32 = 20_000.0;
/// How much of a restored window must land on some display for the window to
/// be reachable — enough of the title bar to grab.
const MIN_VISIBLE_WIDTH: f32 = 120.0;
const MIN_VISIBLE_HEIGHT: f32 = 40.0;

/// One window's rectangle in the global display space, as it is written to disk.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredWindowBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl StoredWindowBounds {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Whether this is a rectangle the app itself could have produced. Rejects
    /// the shapes a hand-edited or corrupted file can hold.
    pub fn is_plausible(&self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|value| value.is_finite())
            && (MIN_WINDOW_EDGE..=MAX_WINDOW_EDGE).contains(&self.width)
            && (MIN_WINDOW_EDGE..=MAX_WINDOW_EDGE).contains(&self.height)
            && self.x.abs() <= MAX_WINDOW_EDGE * 4.0
            && self.y.abs() <= MAX_WINDOW_EDGE * 4.0
    }

    fn overlap_with(&self, display: &StoredWindowBounds) -> (f32, f32) {
        let width = (self.x + self.width).min(display.x + display.width) - self.x.max(display.x);
        let height = (self.y + self.height).min(display.y + display.height) - self.y.max(display.y);
        (width.max(0.0), height.max(0.0))
    }

    /// Whether enough of this window would land on one of the displays that
    /// exist now. A window saved on a monitor that has since been unplugged
    /// must not reopen off-screen where nobody can reach it.
    pub fn is_reachable_on(&self, displays: &[StoredWindowBounds]) -> bool {
        displays.iter().any(|display| {
            let (width, height) = self.overlap_with(display);
            width >= MIN_VISIBLE_WIDTH && height >= MIN_VISIBLE_HEIGHT
        })
    }

    /// This rectangle brought inside the part of a display you can actually
    /// see: below the menu bar, above the Dock, and no wider than the glass.
    ///
    /// `is_reachable_on` only asks whether enough of a remembered window lands
    /// on some display to grab it, which a window hanging into the Dock does.
    /// So a World remembered at a size that does not fit — a different Mac, a
    /// bigger Dock, or a window something resized — reopened with its bottom
    /// row behind the Dock, and on this app that bottom row is "Your turn",
    /// which is where the decision is. Clamping only the default window fixed
    /// the first time a World is opened and nothing after it.
    pub fn fit_within(&self, display: &Self, top_inset: f32, bottom_inset: f32) -> Self {
        let top = display.y + top_inset;
        let usable_height = (display.height - top_inset - bottom_inset).max(MIN_WINDOW_EDGE);
        let height = self.height.min(usable_height);
        let width = self.width.min(display.width);
        Self {
            x: self.x.clamp(display.x, display.x + display.width - width),
            y: self.y.clamp(top, top + usable_height - height),
            width,
            height,
        }
    }

    /// The display this window sits most of itself on.
    pub fn home_display<'a>(&self, displays: &'a [Self]) -> Option<&'a Self> {
        displays.iter().max_by(|left, right| {
            let area = |display: &Self| {
                let (width, height) = self.overlap_with(display);
                width * height
            };
            area(left).total_cmp(&area(right))
        })
    }

    /// The stored rectangle to reopen a window at, or `None` to let the window
    /// open at its default place.
    pub fn restorable(
        stored: Option<StoredWindowBounds>,
        displays: &[StoredWindowBounds],
    ) -> Option<StoredWindowBounds> {
        let stored = stored?;
        if !stored.is_plausible() {
            return None;
        }
        // With no display list to check against, trust the stored entry rather
        // than throwing away a good position.
        if displays.is_empty() || stored.is_reachable_on(displays) {
            Some(stored)
        } else {
            None
        }
    }
}

/// Everything the app remembers about where its windows were.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopWindowState {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<StoredWindowBounds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world: Option<StoredWindowBounds>,
}

impl DesktopWindowState {
    pub fn empty() -> Self {
        Self {
            version: WINDOW_STATE_VERSION,
            home: None,
            world: None,
        }
    }

    pub fn is_supported(&self) -> bool {
        self.version == WINDOW_STATE_VERSION
    }
}

impl Default for DesktopWindowState {
    fn default() -> Self {
        Self::empty()
    }
}

pub fn window_state_path(root: &Path) -> PathBuf {
    root.join(WINDOW_STATE_FILE_NAME)
}

/// Read the remembered geometry. Window positions are a convenience, never
/// something worth an error in front of somebody, so anything unreadable,
/// malformed, or from a version this build does not know reads as "nothing
/// remembered" and the windows open at their defaults.
pub fn load(root: &Path) -> DesktopWindowState {
    let path = window_state_path(root);
    let Ok(mut file) = File::open(path) else {
        return DesktopWindowState::empty();
    };
    let mut contents = String::new();
    if file.read_to_string(&mut contents).is_err() {
        return DesktopWindowState::empty();
    }
    match serde_json::from_str::<DesktopWindowState>(&contents) {
        Ok(state) if state.is_supported() => state,
        _ => DesktopWindowState::empty(),
    }
}

/// Write the remembered geometry, replacing the file atomically so a crash
/// mid-write cannot leave a half-written file behind.
pub fn save(root: &Path, state: &DesktopWindowState) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("could not create {}: {error}", root.display()))?;
    let target = window_state_path(root);
    let sequence = SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp = root.join(format!(
        ".{WINDOW_STATE_FILE_NAME}.tmp-{}-{sequence}",
        std::process::id()
    ));
    let payload = serde_json::to_vec_pretty(state)
        .map_err(|error| format!("could not encode window state: {error}"))?;

    if let Err(error) = write_temporary(&temp, &payload) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    fs::rename(&temp, &target).map_err(|error| {
        let _ = fs::remove_file(&temp);
        format!("could not replace {}: {error}", target.display())
    })
}

fn write_temporary(path: &Path, payload: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    file.write_all(payload)
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("could not flush {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "world-machine-window-state-{}-{nonce}-{label}",
            process::id()
        ))
    }

    fn display() -> StoredWindowBounds {
        StoredWindowBounds::new(0.0, 0.0, 1920.0, 1080.0)
    }

    #[test]
    fn a_window_reopens_where_it_was_left() {
        let root = temp_root("round-trip");
        let mut state = DesktopWindowState::empty();
        state.home = Some(StoredWindowBounds::new(120.0, 80.0, 900.0, 700.0));
        state.world = Some(StoredWindowBounds::new(200.0, 100.0, 1200.0, 950.0));

        save(&root, &state).unwrap();
        let loaded = load(&root);

        assert_eq!(loaded, state);
        assert_eq!(
            StoredWindowBounds::restorable(loaded.home, &[display()]),
            state.home
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn nothing_remembered_yet_is_not_an_error() {
        let root = temp_root("absent");
        assert_eq!(load(&root), DesktopWindowState::empty());
        assert_eq!(StoredWindowBounds::restorable(None, &[display()]), None);
    }

    #[test]
    fn a_damaged_or_future_file_falls_back_to_the_default_place() {
        let root = temp_root("damaged");
        fs::create_dir_all(&root).unwrap();

        fs::write(window_state_path(&root), "{\"version\":").unwrap();
        assert_eq!(load(&root), DesktopWindowState::empty());

        fs::write(window_state_path(&root), "{\"version\":99}").unwrap();
        assert_eq!(load(&root), DesktopWindowState::empty());

        fs::write(window_state_path(&root), "{\"version\":1,\"future\":true}").unwrap();
        assert_eq!(load(&root), DesktopWindowState::empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn an_impossible_rectangle_is_never_restored() {
        let displays = [display()];
        for impossible in [
            StoredWindowBounds::new(0.0, 0.0, 10.0, 10.0),
            StoredWindowBounds::new(0.0, 0.0, 900.0, 0.0),
            StoredWindowBounds::new(0.0, 0.0, 900.0, 90_000.0),
            StoredWindowBounds::new(f32::NAN, 0.0, 900.0, 700.0),
            StoredWindowBounds::new(0.0, f32::INFINITY, 900.0, 700.0),
        ] {
            assert!(!impossible.is_plausible(), "{impossible:?}");
            assert_eq!(
                StoredWindowBounds::restorable(Some(impossible), &displays),
                None,
                "{impossible:?}"
            );
        }
    }

    #[test]
    fn a_window_saved_on_a_display_that_is_gone_opens_at_the_default_place() {
        let displays = [display()];
        let on_second_monitor = StoredWindowBounds::new(2400.0, 200.0, 900.0, 700.0);

        assert!(on_second_monitor.is_plausible());
        assert!(!on_second_monitor.is_reachable_on(&displays));
        assert_eq!(
            StoredWindowBounds::restorable(Some(on_second_monitor), &displays),
            None
        );

        let both = [
            display(),
            StoredWindowBounds::new(1920.0, 0.0, 1920.0, 1080.0),
        ];
        assert_eq!(
            StoredWindowBounds::restorable(Some(on_second_monitor), &both),
            Some(on_second_monitor)
        );
    }

    #[test]
    fn a_window_hanging_off_an_edge_is_kept_while_it_can_still_be_grabbed() {
        let displays = [display()];
        let mostly_off = StoredWindowBounds::new(-750.0, 20.0, 900.0, 700.0);
        let all_but_gone = StoredWindowBounds::new(-880.0, 20.0, 900.0, 700.0);

        assert!(mostly_off.is_reachable_on(&displays));
        assert!(!all_but_gone.is_reachable_on(&displays));
    }

    #[test]
    fn an_unknown_display_list_still_restores_a_plausible_window() {
        let remembered = StoredWindowBounds::new(120.0, 80.0, 900.0, 700.0);
        assert_eq!(
            StoredWindowBounds::restorable(Some(remembered), &[]),
            Some(remembered)
        );
    }

    #[test]
    fn a_remembered_window_is_brought_in_from_behind_the_dock() {
        let screen = StoredWindowBounds::new(0.0, 0.0, 1920.0, 1080.0);
        // Reachable — most of it is on the glass — but 940 tall on a band of
        // 930, so its last row sits under the Dock.
        let too_tall = StoredWindowBounds::new(410.0, 55.0, 1100.0, 940.0);
        assert!(too_tall.is_reachable_on(&[screen]));

        let fitted = too_tall.fit_within(&screen, 40.0, 110.0);

        assert_eq!(fitted.height, 930.0);
        assert!(fitted.y >= 40.0, "under the menu bar at y={}", fitted.y);
        assert!(
            fitted.y + fitted.height <= 1080.0 - 110.0,
            "into the Dock, ending at {}",
            fitted.y + fitted.height
        );
        assert_eq!(fitted.width, 1100.0, "a window that fits keeps its width");
    }

    #[test]
    fn a_window_that_already_fits_is_left_exactly_where_it_was() {
        let screen = StoredWindowBounds::new(0.0, 0.0, 1920.0, 1080.0);
        let fits = StoredWindowBounds::new(410.0, 100.0, 1100.0, 700.0);

        assert_eq!(fits.fit_within(&screen, 40.0, 110.0), fits);
    }

    #[test]
    fn a_window_is_fitted_to_the_display_it_is_actually_on() {
        let left = StoredWindowBounds::new(0.0, 0.0, 1280.0, 800.0);
        let right = StoredWindowBounds::new(1280.0, 0.0, 1920.0, 1080.0);
        let on_the_right = StoredWindowBounds::new(1400.0, 60.0, 1100.0, 900.0);

        assert_eq!(
            on_the_right.home_display(&[left, right]),
            Some(&right),
            "a window mostly on the second monitor is not fitted to the first"
        );
    }
}
