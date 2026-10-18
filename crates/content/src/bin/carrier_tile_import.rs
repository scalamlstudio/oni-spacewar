//! Imports Art's carrier interior Revision 3 batches into
//! `assets/source/core/carrier/`: batch A (TAKOAI-89, TAKOAI-95), the floor,
//! wall and doorway tiles, the six stations, the scaffold and the front
//! marker; and batch B (TAKOAI-92, TAKOAI-93), the 19 furniture and decor
//! pieces. The selection is Art's catalogues under
//! `design/art/carrier-furniture/` (design file, stable ID, destination
//! under `assets/source/core/`, size, alpha bounds; batch B adds footprint,
//! headroom, anchor, layer and placement). The art is already transparent
//! and authored at 2×, so each file is copied byte-for-byte after checking
//! it against its catalogue row.
//!
//! ```sh
//! cargo run --release --bin carrier-tile-import -- .
//! cargo run --release --bin content-pipeline -- .
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use image::GenericImageView;
use serde::Deserialize;

/// Each catalogue, with the alpha above which Art's inspector counted a
/// pixel into `alpha_bbox`: any alpha for batch A, above 0.02 (5 of 255)
/// for batch B (`inspect-batch-b-v1.swift`).
const CATALOGUES: [(&str, u8); 2] = [
    ("design/art/carrier-furniture/batch-a-import-v3.json", 0),
    ("design/art/carrier-furniture/batch-b-import-v1.json", 5),
];
const ART_DIR: &str = "design/art/carrier-furniture";

#[derive(Deserialize)]
struct Entry {
    id: String,
    file: String,
    source: String,
    size: [u32; 2],
    alpha_bbox: [u32; 4],
    /// Batch B only: width × depth in tiles.
    footprint: Option<[u32; 2]>,
    #[serde(default)]
    headroom: u32,
    anchor: Option<[u32; 2]>,
    layer: Option<String>,
}

/// One tile at 2×, art px.
const TILE: u32 = 128;
/// A wall-mounted piece's canvas height, art px.
const WALL_ITEM_H: u32 = 96;

impl Entry {
    /// The canvas a batch B row's footprint calls for (§ Art spec): one
    /// tile wide per footprint column; a wall piece 96 px tall, anything
    /// else its footprint depth plus its headroom (a multiple of 32), with
    /// the anchor at the bottom centre.
    fn check_footprint(&self) -> Result<(), String> {
        let Some([w, d]) = self.footprint else {
            return Ok(());
        };
        let height = match self.layer.as_deref() {
            Some("wall") => WALL_ITEM_H,
            _ => d * TILE + self.headroom,
        };
        if !self.headroom.is_multiple_of(32) || self.size != [w * TILE, height] {
            return Err(format!(
                "{}: canvas {:?} for footprint {w}x{d}, headroom {}",
                self.id, self.size, self.headroom
            ));
        }
        if self.anchor != Some([self.size[0] / 2, self.size[1]]) {
            return Err(format!(
                "{}: anchor {:?} not bottom centre",
                self.id, self.anchor
            ));
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    let mut count = 0;
    for (catalogue, min_alpha) in CATALOGUES {
        let entries: Vec<Entry> = serde_json::from_slice(&fs::read(root.join(catalogue))?)?;
        for e in &entries {
            import(&root, e, min_alpha)?;
        }
        count += entries.len();
    }
    println!("imported {count} carrier assets");
    Ok(())
}

fn import(root: &Path, e: &Entry, min_alpha: u8) -> Result<(), Box<dyn std::error::Error>> {
    // The pipeline derives the stable ID from the path; they must agree.
    let derived = format!(
        "core.{}",
        e.source.trim_end_matches(".png").replace('/', ".")
    );
    if derived != e.id {
        return Err(format!("{}: source {} gives ID {derived}", e.id, e.source).into());
    }
    let from = root.join(ART_DIR).join(&e.file);
    let image = image::open(&from).map_err(|err| format!("{}: {err}", from.display()))?;
    if image.dimensions() != (e.size[0], e.size[1]) {
        return Err(format!(
            "{}: {:?}, expected {:?}",
            e.file,
            image.dimensions(),
            e.size
        )
        .into());
    }
    let bbox = alpha_bbox(&image.to_rgba8(), min_alpha);
    if bbox != e.alpha_bbox {
        return Err(format!(
            "{}: alpha bounds {bbox:?}, expected {:?}",
            e.file, e.alpha_bbox
        )
        .into());
    }
    e.check_footprint()?;
    let to = root.join("assets/source/core").join(&e.source);
    fs::create_dir_all(to.parent().unwrap())?;
    fs::copy(&from, &to)?;
    println!("{} -> {}", e.file, e.id);
    Ok(())
}

/// `[left, top, right, bottom)` of the pixels with alpha above `min_alpha`.
fn alpha_bbox(image: &image::RgbaImage, min_alpha: u8) -> [u32; 4] {
    let mut b = [u32::MAX, u32::MAX, 0, 0];
    for (x, y, p) in image.enumerate_pixels() {
        if p[3] > min_alpha {
            b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
        }
    }
    b
}
