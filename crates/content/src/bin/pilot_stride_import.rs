//! Import tiny generated boots underneath the original, pixel-identical body.
//! Foot identities follow opposite trajectories, including diagonal depth.
use image::{imageops, RgbaImage};

const HIP: u32 = 162;
const BASELINE: i64 = 181;
const BOOT_H: u32 = 19;

fn bounds(img: &RgbaImage) -> (u32, u32, u32, u32) {
    let mut b = (img.width(), img.height(), 0, 0);
    for (x, y, _) in img.enumerate_pixels().filter(|(_, _, p)| p[3] > 8) {
        b = (b.0.min(x), b.1.min(y), b.2.max(x + 1), b.3.max(y + 1));
    }
    assert!(b.2 > b.0 && b.3 > b.1, "empty boot");
    b
}

fn trim(img: &RgbaImage) -> RgbaImage {
    let (x, y, r, b) = bounds(img);
    imageops::crop_imm(img, x, y, r - x, b - y).to_image()
}

fn boots(dir: &str) -> Result<[RgbaImage; 2], Box<dyn std::error::Error>> {
    let sheet = image::open(format!("assets/authoring/pilot-short-walk-{dir}.png"))?.into_rgba8();
    let cell = imageops::crop_imm(&sheet, 0, 0, sheet.width() / 4, sheet.height()).to_image();
    let (l, t, r, b) = bounds(&cell);
    // Only the bottom nine percent is imported. Never fit a new full body.
    let top = b - (b - t) * 9 / 100;
    let strip = imageops::crop_imm(&cell, l, top, r - l, b - top).to_image();
    let (l, _, r, _) = bounds(&strip);
    let split = (l + r) / 2;
    let left = trim(&imageops::crop_imm(&strip, l, 0, split - l, strip.height()).to_image());
    let right = trim(&imageops::crop_imm(&strip, split, 0, r - split, strip.height()).to_image());
    Ok([left, right].map(|boot| {
        let w = (boot.width() * BOOT_H / boot.height()).clamp(16, 34);
        imageops::resize(&boot, w, BOOT_H, imageops::FilterType::Lanczos3)
    }))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Direction, each leg\'s hip centre, and the projected forward step.
    for (dir, hips, step) in [
        ("e", [100i64, 100], (12i64, 0i64)),
        ("se", [79, 106], (7, 3)),
        ("s", [70, 102], (0, 3)),
        ("ne", [82, 108], (7, -3)),
        ("n", [72, 99], (0, -3)),
    ] {
        let base =
            image::open(format!("assets/authoring/pilot-walk-bases/{dir}.png"))?.into_rgba8();
        let mut base = base;
        if dir == "n" {
            let head = image::open("assets/authoring/pilot-walk-bases/n-head.png")?.into_rgba8();
            for y in 0..96 {
                for x in 0..165 {
                    base.put_pixel(x, y, *head.get_pixel(x, y));
                }
            }
        }
        let feet = boots(dir)?;
        for phase in 0..4 {
            let mut out = RgbaImage::new(165, 192);
            // Same two boots throughout the cycle: swapping their positions
            // explicitly prevents the same leg from always staying in front.
            for leg in (0..2).rev() {
                let sign = if leg == 0 { 1 } else { -1 };
                let travel = [1, 0, -1, 0][phase] * sign;
                let lift = matches!((phase, leg), (1, 1) | (3, 0));
                let x = hips[leg] + travel * step.0 - feet[leg].width() as i64 / 2;
                let y = if phase % 2 == 0 && step.1 != 0 {
                    BASELINE - 3 + travel * step.1
                } else {
                    BASELINE
                } - if lift { 4 } else { 0 };
                imageops::overlay(&mut out, &feet[leg], x, y - BOOT_H as i64);
            }
            // Copy *all* original pixels, including transparency, above the
            // foot region. No generated head, torso, waist or longer thigh.
            for y in 0..HIP {
                for x in 0..base.width() {
                    out.put_pixel(x, y, *base.get_pixel(x, y));
                }
            }
            // Keep the original rounded waist/belly in front of the boots;
            // a straight horizontal cut would flatten the suit silhouette.
            let (cx, cy, rx, ry) = match dir {
                "e" => (96i64, 155i64, 50i64, 17i64),
                "se" => (91, 153, 43, 18),
                "s" => (83, 153, 42, 16),
                "ne" => (94, 153, 34, 15),
                _ => (87, 154, 32, 14),
            };
            for y in HIP..181 {
                for x in 0..base.width() {
                    let dx = x as i64 - cx;
                    let dy = y as i64 - cy;
                    if dx * dx * ry * ry + dy * dy * rx * rx <= rx * rx * ry * ry {
                        let p = base.get_pixel(x, y);
                        if p[3] > 0 {
                            out.put_pixel(x, y, *p);
                        }
                    }
                }
            }
            // The original back-view tail hangs in front of the left boot.
            if dir == "n" {
                for y in HIP..168 {
                    for x in 0..82 {
                        let p = base.get_pixel(x, y);
                        if p[3] > 0 {
                            out.put_pixel(x, y, *p);
                        }
                    }
                }
            }
            let path = format!(
                "assets/source/core/carrier/pilot/{dir}/walk_{}.png",
                phase + 1
            );
            out.save(&path)?;
            println!("{path}: original upper body, {BOOT_H} px boots");
        }
    }
    Ok(())
}
