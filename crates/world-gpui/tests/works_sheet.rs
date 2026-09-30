//! The library of drawings, looked over: every drawing on a contact sheet
//! per family, kept as golden pictures under `tests/golden/`.
//!
//! Run with `WORLD_GPUI_UPDATE_GOLDEN=1` to draw the sheets afresh after a
//! change meant to alter them, and look at them before committing. With
//! `WORLD_GPUI_CONTACT_SHEET=<dir>` the sheets are also written there,
//! larger, with a list of which drawing is where.

use world_gpui::works::{contact_sheet, Art, Family};

const FAMILIES: [Family; 5] = [
    Family::Any,
    Family::Harbour,
    Family::Mars,
    Family::Street,
    Family::Ice,
];

fn png(pixmap: &tiny_skia::Pixmap) -> image::RgbaImage {
    image::RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
        .expect("a picture")
}

/// How many pixels of two pictures differ by more than a little.
fn differing(a: &image::RgbaImage, b: &image::RgbaImage) -> usize {
    a.pixels()
        .zip(b.pixels())
        .filter(|(a, b)| a.0.iter().zip(b.0).any(|(x, y)| x.abs_diff(y) > 6))
        .count()
}

#[test]
fn every_drawing_matches_its_contact_sheet() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let update = std::env::var_os("WORLD_GPUI_UPDATE_GOLDEN").is_some();
    let out = std::env::var("WORLD_GPUI_CONTACT_SHEET").ok();
    for family in FAMILIES {
        let arts = Art::all()
            .filter(|art| art.family() == family)
            .collect::<Vec<_>>();
        let name = format!("{family:?}").to_lowercase();
        if let Some(out) = &out {
            png(&contact_sheet(&arts, 10, 132))
                .save(format!("{out}/{name}.png"))
                .unwrap();
            let keys = arts
                .iter()
                .enumerate()
                .map(|(index, art)| format!("{:>2},{:<2} {}", index % 10, index / 10, art.key()))
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(format!("{out}/{name}.txt"), keys).unwrap();
        }
        let sheet = png(&contact_sheet(&arts, 12, 64));
        let path = dir.join(format!("works-{name}.png"));
        if update || !path.exists() {
            sheet.save(&path).unwrap();
            continue;
        }
        let kept = image::open(&path).unwrap().to_rgba8();
        assert_eq!(
            kept.dimensions(),
            sheet.dimensions(),
            "{name}: the sheet changed size"
        );
        let changed = differing(&kept, &sheet);
        assert!(
            (changed as f64) < (sheet.len() / 4) as f64 * 0.002,
            "{name}: {changed} pixels differ from {}",
            path.display()
        );
    }
}
