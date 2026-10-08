//! Crop check (design/ART_GUIDELINES.md § Cropping): every cut-out sprite or
//! frame under `assets/source/core/` keeps a transparent margin, so no opaque
//! pixel touches the canvas edge. This catches clipped ears, tails and
//! antennas before they ship.

use std::fs;
use std::path::{Path, PathBuf};

/// Alpha above this counts as opaque (soft key edges stay below it).
const OPAQUE_ALPHA: u8 = 8;

/// Paths (relative to `assets/source/core/`) that are meant to fill their
/// canvas: full-bleed backdrops, room and corridor tiles that join their
/// neighbours, and 9-slice UI panels. A directory entry ends in `/`.
const FULL_BLEED: &[&str] = &[
    "title/",
    "result/",
    "carrier/room/",
    "carrier/corridor/",
    "carrier/door/",
    "carrier/dock/",
    "carrier/hull_floor.png",
    "carrier/build_slot.png",
    "ui/builder/panel/",
    "ui/builder/module/",
    "ui/builder/grid_cell_overlay.png",
];

/// Cut-outs that really are clipped and wait for an art fix. They are
/// reported, not hidden: the test fails if one of them stops clipping, so
/// the entry gets removed with the fix.
const KNOWN_CLIPPED: &[&str] = &[
    // The tail touches the Shocked pose on the expression sheet, so the
    // seam split cuts it off at the right edge.
    "portraits/engineer/confused.png",
];

fn core_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/source/core")
}

fn pngs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            pngs(&path, out);
        } else if path.extension().is_some_and(|e| e == "png") {
            out.push(path);
        }
    }
}

fn full_bleed(relative: &str) -> bool {
    FULL_BLEED.iter().any(|p| {
        if p.ends_with('/') {
            relative.starts_with(p)
        } else {
            relative == *p
        }
    })
}

/// The canvas edges ("top", "bottom", "left", "right") an opaque pixel
/// touches.
fn touched_edges(image: &image::RgbaImage) -> Vec<&'static str> {
    let (w, h) = image.dimensions();
    let opaque = |x: u32, y: u32| image.get_pixel(x, y)[3] > OPAQUE_ALPHA;
    let mut edges = Vec::new();
    if (0..w).any(|x| opaque(x, 0)) {
        edges.push("top");
    }
    if (0..w).any(|x| opaque(x, h - 1)) {
        edges.push("bottom");
    }
    if (0..h).any(|y| opaque(0, y)) {
        edges.push("left");
    }
    if (0..h).any(|y| opaque(w - 1, y)) {
        edges.push("right");
    }
    edges
}

#[test]
fn cutouts_keep_a_transparent_margin() {
    let root = core_root();
    let mut files = Vec::new();
    pngs(&root, &mut files);
    files.sort();
    let mut checked = 0;
    let mut clipped = Vec::new();
    let mut fixed = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if full_bleed(&relative) {
            continue;
        }
        let image = image::open(&path).unwrap().to_rgba8();
        checked += 1;
        let edges = touched_edges(&image);
        let known = KNOWN_CLIPPED.contains(&relative.as_str());
        if known && edges.is_empty() {
            fixed.push(relative);
        } else if known {
            eprintln!("known clipped: {relative} ({})", edges.join(", "));
        } else if !edges.is_empty() {
            clipped.push(format!("{relative} ({})", edges.join(", ")));
        }
    }
    assert!(
        checked > 0,
        "no cut-out sprites found under {}",
        root.display()
    );
    assert!(
        clipped.is_empty(),
        "opaque pixels touch the canvas edge:\n  {}",
        clipped.join("\n  ")
    );
    assert!(
        fixed.is_empty(),
        "no longer clipped, remove from KNOWN_CLIPPED: {fixed:?}"
    );
}

#[test]
fn edge_check_finds_clipped_pixels() {
    let mut image = image::RgbaImage::new(4, 4);
    image.put_pixel(1, 1, image::Rgba([255, 255, 255, 255]));
    assert!(touched_edges(&image).is_empty());
    image.put_pixel(3, 2, image::Rgba([255, 255, 255, OPAQUE_ALPHA + 1]));
    assert_eq!(touched_edges(&image), ["right"]);
    image.put_pixel(0, 0, image::Rgba([255, 255, 255, OPAQUE_ALPHA]));
    assert_eq!(touched_edges(&image), ["right"]);
}
