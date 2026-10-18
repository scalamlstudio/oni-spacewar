//! Imports Art batch A for the carrier interior Revision 3 (TAKOAI-89,
//! TAKOAI-95): floor, wall and doorway tiles, the six stations, the scaffold
//! and the front marker. The selection is Art's catalogue
//! `design/art/carrier-furniture/batch-a-import-v2.json` (design file, stable
//! ID, destination under `assets/source/core/`, size, alpha bounds). The art
//! is already transparent and authored at 2×, so each file is copied
//! byte-for-byte after checking it against its catalogue row.
//!
//! ```sh
//! cargo run --release --bin carrier-tile-import -- .
//! cargo run --release --bin content-pipeline -- .
//! ```

use std::fs;
use std::path::PathBuf;

use image::GenericImageView;
use serde::Deserialize;

const CATALOGUE: &str = "design/art/carrier-furniture/batch-a-import-v2.json";
const ART_DIR: &str = "design/art/carrier-furniture";

#[derive(Deserialize)]
struct Entry {
    id: String,
    file: String,
    source: String,
    size: [u32; 2],
    alpha_bbox: [u32; 4],
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    let entries: Vec<Entry> = serde_json::from_slice(&fs::read(root.join(CATALOGUE))?)?;
    for e in &entries {
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
        let bbox = alpha_bbox(&image.to_rgba8());
        if bbox != e.alpha_bbox {
            return Err(format!(
                "{}: alpha bounds {bbox:?}, expected {:?}",
                e.file, e.alpha_bbox
            )
            .into());
        }
        let to = root.join("assets/source/core").join(&e.source);
        fs::create_dir_all(to.parent().unwrap())?;
        fs::copy(&from, &to)?;
        println!("{} -> {}", e.file, e.id);
    }
    println!("imported {} carrier assets", entries.len());
    Ok(())
}

/// `[left, top, right, bottom)` of the pixels with any alpha.
fn alpha_bbox(image: &image::RgbaImage) -> [u32; 4] {
    let mut b = [u32::MAX, u32::MAX, 0, 0];
    for (x, y, p) in image.enumerate_pixels() {
        if p[3] > 0 {
            b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
        }
    }
    b
}
