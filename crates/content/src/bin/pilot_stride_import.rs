//! Import the authored four-phase side walk without per-pose recentering.
//! Run from the repo root, then run content-pipeline.
use image::{imageops, RgbaImage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sheet = image::open("assets/authoring/pilot-side-walk-v2.png")?.into_rgba8();
    assert_eq!(sheet.width() % 4, 0, "four equal-width cells");
    let cell = sheet.width() / 4;
    let mut bounds = (cell, sheet.height(), 0, 0);
    for (x, y, _) in sheet.enumerate_pixels().filter(|(_, _, p)| p[3] > 8) {
        let x = x % cell;
        bounds.0 = bounds.0.min(x);
        bounds.1 = bounds.1.min(y);
        bounds.2 = bounds.2.max(x + 1);
        bounds.3 = bounds.3.max(y + 1);
    }
    let (left, top, right, bottom) = bounds;
    let height = 150;
    let width = ((right - left) * height + (bottom - top) / 2) / (bottom - top);
    assert!(width < 153, "leave a transparent canvas margin");
    for i in 0..4 {
        let pose =
            imageops::crop_imm(&sheet, i * cell + left, top, right - left, bottom - top).to_image();
        // One shared scale and origin keeps the head and hips steady.
        let pose = imageops::resize(&pose, width, height, imageops::FilterType::Lanczos3);
        let mut out = RgbaImage::new(165, 192);
        imageops::replace(&mut out, &pose, ((165 - width) / 2) as i64, 31);
        let path = format!("assets/source/core/carrier/pilot/e/walk_{}.png", i + 1);
        out.save(&path)?;
        println!("{path}: 165x192, shared visible bounds {width}x{height}");
    }
    Ok(())
}
