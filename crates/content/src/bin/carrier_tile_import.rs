//! Imports Art's carrier interior Revision 3 batches into
//! `assets/source/core/carrier/`: batch A (TAKOAI-89, TAKOAI-95), the floor,
//! wall and doorway tiles, the six stations, the scaffold and the front
//! marker; batch B (TAKOAI-92, TAKOAI-93), the 19 furniture and decor
//! pieces; the hull-edge wall tiles (TAKOAI-101, TAKOAI-102); and patch
//! catalogues that redraw some of those (TAKOAI-99). The selection is Art's
//! catalogues under `design/art/carrier-furniture/` (design file, stable ID,
//! destination under `assets/source/core/`, size, alpha bounds; batch B adds
//! footprint, headroom, anchor, layer and placement). The art is already
//! transparent and authored at 2×, so each file is copied byte-for-byte
//! after checking it against its catalogue row.
//!
//! ```sh
//! cargo run --release --bin carrier-tile-import -- .
//! cargo run --release --bin content-pipeline -- .
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use image::GenericImageView;
use serde::Deserialize;

/// How a catalogue's rows are read.
#[derive(Clone, Copy)]
enum Kind {
    /// A JSON list of new rows.
    Rows,
    /// A JSON list of rows that each redraw one an earlier catalogue
    /// already has (same ID and canvas; footprint rows keep their
    /// footprint): only the changed art.
    Patch,
    /// An object with its rows under `tiles` (the hull-edge catalogues).
    Tiles,
}

/// Each catalogue in import order, the directory its files are in, and
/// the alpha above which Art's inspector counted a pixel into
/// `alpha_bbox`: any alpha for batch A, the patches and the hull tiles,
/// above 0.02 (5 of 255) for batch B (`inspect-batch-b-v1.swift`).
const CATALOGUES: [(&str, &str, Kind, u8); 5] = [
    ("batch-a-import-v3.json", ART_DIR, Kind::Rows, 0),
    ("batch-b-import-v1.json", ART_DIR, Kind::Rows, 5),
    // TAKOAI-99: two stations and five furniture pieces turned to face
    // straight south.
    ("orientation-import-patch-v2.json", ART_DIR, Kind::Patch, 0),
    // TAKOAI-101: the starter hull edge, and (TAKOAI-102) every other hull
    // case a player-built layout reaches, drawn by Art's generator.
    ("outer-wall-import-v1.json", ART_DIR, Kind::Tiles, 0),
    ("catalogue.json", HULL_EXTRA_DIR, Kind::Tiles, 0),
];
const ART_DIR: &str = "design/art/carrier-furniture";
const HULL_EXTRA_DIR: &str = "crates/content/hull-wall/extra-v1";

#[derive(Deserialize)]
struct Tiles {
    tiles: Vec<Entry>,
}

#[derive(Clone, Deserialize)]
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

/// A row to import: its catalogue row, the directory its file is in and
/// its alpha threshold.
type Row = (Entry, &'static str, u8);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    // By ID, so a patch row replaces the row it redraws.
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    for (name, dir, kind, min_alpha) in CATALOGUES {
        let bytes = fs::read(root.join(dir).join(name))?;
        let entries: Vec<Entry> = match kind {
            Kind::Tiles => serde_json::from_slice::<Tiles>(&bytes)?.tiles,
            _ => serde_json::from_slice(&bytes)?,
        };
        for e in entries {
            let id = e.id.clone();
            match (kind, rows.get(&id)) {
                (Kind::Patch, None) => return Err(format!("{name}: {id} patches nothing").into()),
                (Kind::Patch, Some((old, ..))) => patches(old, &e)?,
                (_, Some(_)) => return Err(format!("{name}: {id} listed twice").into()),
                (_, None) => {}
            }
            rows.insert(id, (e, dir, min_alpha));
        }
    }
    for (e, dir, min_alpha) in rows.values() {
        import(&root, dir, e, *min_alpha)?;
    }
    println!("imported {} carrier assets", rows.len());
    Ok(())
}

/// A patch row may change the art only: same canvas, and a footprint row
/// keeps its footprint, headroom, anchor and layer.
fn patches(old: &Entry, new: &Entry) -> Result<(), String> {
    let same = old.size == new.size
        && (new.footprint.is_none()
            || (old.footprint, old.headroom, old.anchor, &old.layer)
                == (new.footprint, new.headroom, new.anchor, &new.layer));
    if same {
        Ok(())
    } else {
        Err(format!("{}: the patch changes more than the art", new.id))
    }
}

/// The destination under `assets/source/core/` the pipeline derives `id`
/// from (`core.a.b.c` is `a/b/c.png`).
fn destination(id: &str) -> Option<String> {
    Some(format!(
        "{}.png",
        id.strip_prefix("core.")?.replace('.', "/")
    ))
}

fn import(
    root: &Path,
    dir: &str,
    e: &Entry,
    min_alpha: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    // The pipeline derives the stable ID from the path, so the ID decides
    // where the file goes. Art's hull catalogue (TAKOAI-101) gives its
    // tiles a `hull_wall/` folder their IDs don't have; the ID wins.
    let source = destination(&e.id).ok_or_else(|| format!("{}: not a core ID", e.id))?;
    if source != e.source {
        println!("{}: catalogue source {} -> {source}", e.id, e.source);
    }
    let from = root.join(dir).join(&e.file);
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
    let to = root.join("assets/source/core").join(&source);
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
