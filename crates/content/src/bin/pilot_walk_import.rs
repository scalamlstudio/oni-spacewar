//! Import the Pilot's idle and walk frames from whole-character sheets
//! (TAKOAI-130). Each `assets/authoring/pilot-walk-<dir>.png` is one
//! generated row of five complete drawings on flat magenta: idle, then
//! walk_1..4. This importer only cuts, keys out the background, scales and
//! places frames. It never erases, repaints or composites regions, and it
//! never borrows pixels from another frame or direction
//! (design/ART_GUIDELINES.md § Animation frames).
use image::{imageops, Rgba, RgbaImage};

const CANVAS: (u32, u32) = (165, 192);
/// Feet row of every frame: one past the last opaque row.
const BASELINE: u32 = 181;
/// Visible idle height (head top to feet) in every direction, matching the
/// S and E frames the client's `standing()` contract was tuned on.
const IDLE_H: f32 = 150.0;
/// Alpha above this counts as part of the character when cutting cells.
const SOLID: u8 = 40;
const FRAMES: [&str; 5] = ["idle", "walk_1", "walk_2", "walk_3", "walk_4"];

/// Magenta (#FF00FF) chroma key: alpha from how magenta a pixel is, then
/// the background colour is unmixed from partially transparent edges so
/// they don't keep a pink fringe.
fn key(sheet: &RgbaImage) -> RgbaImage {
    let mut out = RgbaImage::new(sheet.width(), sheet.height());
    for (x, y, p) in sheet.enumerate_pixels() {
        let [r, g, b] = [p[0], p[1], p[2]].map(f32::from);
        let magenta = r.min(b) - g;
        let a = ((200.0 - magenta) / 110.0).clamp(0.0, 1.0);
        if a <= 0.01 {
            continue;
        }
        let unmix = |c: f32, bg: f32| ((c - (1.0 - a) * bg) / a).clamp(0.0, 255.0) as u8;
        out.put_pixel(
            x,
            y,
            Rgba([
                unmix(r, 255.0),
                unmix(g, 0.0),
                unmix(b, 255.0),
                (a * 255.0) as u8,
            ]),
        );
    }
    out
}

/// Opaque bounds `(left, top, right, bottom)`, exclusive right/bottom.
fn bounds(img: &RgbaImage) -> (u32, u32, u32, u32) {
    let mut b = (img.width(), img.height(), 0, 0);
    for (x, y, _) in img.enumerate_pixels().filter(|(_, _, p)| p[3] > SOLID) {
        b = (b.0.min(x), b.1.min(y), b.2.max(x + 1), b.3.max(y + 1));
    }
    assert!(b.2 > b.0 && b.3 > b.1, "empty cell");
    b
}

/// The five characters, cut at the empty columns between them.
fn cells(keyed: &RgbaImage) -> Vec<RgbaImage> {
    let used: Vec<bool> = (0..keyed.width())
        .map(|x| (0..keyed.height()).any(|y| keyed.get_pixel(x, y)[3] > SOLID))
        .collect();
    let mut spans = Vec::new();
    let mut x = 0;
    while x < used.len() {
        if used[x] {
            let start = x;
            while x < used.len() && used[x] {
                x += 1;
            }
            // Ignore specks; a character is far wider than this.
            if x - start > 20 {
                spans.push((start as u32, x as u32));
            }
        }
        x += 1;
    }
    spans
        .into_iter()
        .map(|(l, r)| {
            let column = imageops::crop_imm(keyed, l, 0, r - l, keyed.height()).to_image();
            let (x0, y0, x1, y1) = bounds(&column);
            imageops::crop_imm(&column, x0, y0, x1 - x0, y1 - y0).to_image()
        })
        .collect()
}

/// Mean opaque column of the top 45% (the head), which stays put while the
/// legs and tail swing; aligning on it keeps a walk from sliding sideways.
fn head_x(img: &RgbaImage) -> f32 {
    let rows = img.height() * 45 / 100;
    let (mut sum, mut n) = (0.0, 0.0);
    for y in 0..rows {
        for x in 0..img.width() {
            if img.get_pixel(x, y)[3] > SOLID {
                sum += x as f32;
                n += 1.0;
            }
        }
    }
    sum / n
}

/// Lanczos resize in premultiplied alpha, so colour doesn't bleed in from
/// the transparent (black) background and darken the outline.
fn resize(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    let mut pre = image::Rgba32FImage::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        let a = f32::from(p[3]) / 255.0;
        let c = |i: usize| f32::from(p[i]) / 255.0 * a;
        pre.put_pixel(x, y, Rgba([c(0), c(1), c(2), a]));
    }
    let pre = imageops::resize(&pre, w, h, imageops::FilterType::Lanczos3);
    let mut out = RgbaImage::new(w, h);
    for (x, y, p) in pre.enumerate_pixels() {
        let a = p[3].clamp(0.0, 1.0);
        if a < 1.0 / 255.0 {
            continue;
        }
        let c = |i: usize| ((p[i] / a).clamp(0.0, 1.0) * 255.0).round() as u8;
        out.put_pixel(x, y, Rgba([c(0), c(1), c(2), (a * 255.0).round() as u8]));
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // W, SW and NW mirror E, SE and NE in the client.
    for dir in ["e", "se", "s", "ne", "n"] {
        let sheet = image::open(format!("assets/authoring/pilot-walk-{dir}.png"))?.into_rgba8();
        let cells = cells(&key(&sheet));
        assert_eq!(cells.len(), FRAMES.len(), "{dir}: expected 5 drawings");
        // One scale for the whole sheet, from the idle drawing: walk frames
        // keep exactly the proportions they were drawn with.
        let scale = IDLE_H / cells[0].height() as f32;
        let mut target_x = None;
        for (cell, frame) in cells.iter().zip(FRAMES) {
            let w = (cell.width() as f32 * scale).round() as u32;
            let h = (cell.height() as f32 * scale).round() as u32;
            let sprite = resize(cell, w, h);
            // Idle is centred on the canvas; walk frames put the head where
            // idle has it.
            let at = *target_x.get_or_insert((CANVAS.0 as f32 - w as f32) / 2.0 + head_x(&sprite));
            let mut out = RgbaImage::new(CANVAS.0, CANVAS.1);
            let x = (at - head_x(&sprite)).round() as i64;
            imageops::overlay(&mut out, &sprite, x, BASELINE as i64 - h as i64);
            let path = format!("assets/source/core/carrier/pilot/{dir}/{frame}.png");
            out.save(&path)?;
            println!("{path}: {w}x{h} at scale {scale:.3}");
        }
    }
    Ok(())
}
