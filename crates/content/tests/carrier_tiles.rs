//! Carrier interior Revision 3 art (Art batch A, TAKOAI-95): every ID the
//! scene needs is in the manifest, and the wall tiles picked by the n-e-s-w
//! neighbour mask join into one continuous wall.

use std::path::{Path, PathBuf};

use content::ContentManifest;
use image::RgbaImage;

fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn manifest() -> ContentManifest {
    ContentManifest::load(assets().join("manifest.json")).unwrap()
}

/// The wall tile's file stem: mask bits n, e, s, w (1, 2, 4, 8, as
/// `layout::TileMap::wall_mask`) written in n-e-s-w order.
fn wall_key(mask: u8) -> String {
    (0..4)
        .map(|i| if mask & 1 << i != 0 { '1' } else { '0' })
        .collect()
}

const N: u8 = 1;
const E: u8 = 2;
const S: u8 = 4;
const W: u8 = 8;

fn wall(mask: u8) -> RgbaImage {
    let path = assets().join(format!(
        "source/core/carrier/tile/wall/{}.png",
        wall_key(mask)
    ));
    image::open(path).unwrap().to_rgba8()
}

fn column(im: &RgbaImage, x: u32, rows: std::ops::Range<u32>) -> Vec<[u8; 4]> {
    rows.map(|y| im.get_pixel(x, y).0).collect()
}

fn row(im: &RgbaImage, y: u32, cols: std::ops::Range<u32>) -> Vec<[u8; 4]> {
    cols.map(|x| im.get_pixel(x, y).0).collect()
}

/// The brass rim along an exposed (unjoined) tile edge.
const RIM: u32 = 3;

#[test]
fn batch_a_ids_resolve() {
    let m = manifest();
    let mut ids: Vec<String> = (0..3)
        .map(|i| format!("core.carrier.tile.floor.{i}"))
        .collect();
    ids.extend((0..16).map(|m| format!("core.carrier.tile.wall.{}", wall_key(m))));
    ids.extend(["ns", "ew"].map(|a| format!("core.carrier.tile.doorway.{a}")));
    ids.extend(
        [
            "command_console",
            "upgrade_bench",
            "sample_analyzer",
            "salvage_sorter",
            "combat_simulator",
            "berth",
        ]
        .map(|i| format!("core.carrier.item.{i}")),
    );
    ids.extend(["core.carrier.scaffold", "core.carrier.front_marker"].map(String::from));
    assert_eq!(ids.len(), 29);
    for id in &ids {
        let asset = m.asset(id).unwrap_or_else(|e| panic!("{id}: {e:?}"));
        assert!(
            assets().join(&asset.processed).is_file(),
            "{id}: no processed file"
        );
    }
}

#[test]
fn tiles_are_one_grid_cell() {
    for m in 0..16 {
        assert_eq!(wall(m).dimensions(), (128, 128), "wall {}", wall_key(m));
    }
    for name in ["floor/0", "floor/1", "floor/2", "doorway/ns", "doorway/ew"] {
        let path = assets().join(format!("source/core/carrier/tile/{name}.png"));
        assert_eq!(image::image_dimensions(path).unwrap(), (128, 128), "{name}");
    }
}

/// Two walls side by side (the west one joined east, the east one joined
/// west) share their cap edge exactly, and their face edge too when both
/// show a face; one tile above another shares its whole edge. So a run of
/// mask-picked tiles reads as one wall with no seam or post at the joins.
/// The only exception is the rim of an outer edge that one tile exposes and
/// its neighbour doesn't (where the wall turns a corner): the rim ends there.
#[test]
fn joined_walls_are_continuous() {
    for a in 0..16u8 {
        for b in 0..16u8 {
            let (wa, wb) = (wall(a), wall(b));
            if a & E != 0 && b & W != 0 {
                let top = if a & N == b & N { 0 } else { RIM };
                let rows = if a & S == b & S { top..128 } else { top..32 };
                assert_eq!(
                    column(&wa, 127, rows.clone()),
                    column(&wb, 0, rows),
                    "wall {} | wall {}",
                    wall_key(a),
                    wall_key(b)
                );
            }
            if a & S != 0 && b & N != 0 {
                let left = if a & W == b & W { 0 } else { RIM };
                let right = if a & E == b & E { 128 } else { 128 - RIM };
                assert_eq!(
                    row(&wa, 127, left..right),
                    row(&wb, 0, left..right),
                    "wall {} above wall {}",
                    wall_key(a),
                    wall_key(b)
                );
            }
        }
    }
}
