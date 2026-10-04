//! Imports the First Playable demo art set (`design/art/demo/*-v1.png`, plus
//! the restyled `design/art/demo-v2/` pieces in `V2_SPRITES`) into
//! `assets/source/core/...`: keys out the flat backgrounds, cuts the walk and
//! icon sheets into frames, and scales everything to its in-game size. Run
//! `content-pipeline` afterwards to process, bundle and list the results.

use std::fs;
use std::path::{Path, PathBuf};

use content::cutouts::{extract_expression_cutout_sheet, extract_sprite, fit_within};
use image::RgbaImage;

/// Single sprites: design file stem, pack-relative output, max width/height.
const SPRITES: [(&str, &str, u32, u32); 9] = [
    ("battleship-kite-v1", "battle/ship/kite", 256, 256),
    ("battleship-bulwark-v1", "battle/ship/bulwark", 256, 256),
    (
        "enemy-void-swarmer-v1",
        "battle/enemy/void_swarmer",
        192,
        192,
    ),
    (
        "enemy-void-spitter-v1",
        "battle/enemy/void_spitter",
        192,
        192,
    ),
    ("projectile-void-spitter-v1", "battle/fx/spit", 128, 128),
    ("carrier-room-bridge-v1", "carrier/room/bridge", 480, 360),
    (
        "carrier-room-crew-quarters-v1",
        "carrier/room/crew_quarters",
        480,
        360,
    ),
    (
        "carrier-room-workshop-v1",
        "carrier/room/workshop",
        480,
        360,
    ),
    ("carrier-interior-wide-v1", "carrier/interior", 1200, 450),
];

/// Single sprites from `design/art/demo-v2/`, same columns as `SPRITES`.
const V2_SPRITES: [(&str, &str, u32, u32); 1] =
    [("carrier-room-dock-empty-v2", "carrier/dock/berth", 480, 360)];

/// Full-frame images (no background to key): stem, output, max size.
const BACKDROPS: [(&str, &str, u32, u32); 1] = [("title-key-art-v1", "title/key_art", 1440, 540)];

/// Row sheets: stem, output directory, frame names left to right, max frame size.
const SHEETS: [(&str, &str, &[&str], u32, u32); 2] = [
    (
        "oni-pilot-walk-sheet-v1",
        "carrier/pilot",
        &["idle", "walk_1", "walk_2", "walk_3", "walk_4"],
        192,
        192,
    ),
    (
        "demo-icons-sheet-v1",
        "ui/icon",
        &[
            "credits",
            "void_crystal",
            "hull_plating",
            "weapon_tuning",
            "thruster_tuning",
        ],
        128,
        128,
    ),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    let mut contact = Vec::new();

    let sprites = SPRITES
        .iter()
        .map(|s| ("demo", s))
        .chain(V2_SPRITES.iter().map(|s| ("demo-v2", s)));
    for (dir, &(stem, out, max_w, max_h)) in sprites {
        let sheet = open_in(&root, dir, stem)?;
        let sprite = extract_sprite(&sheet, 4).map_err(|e| format!("{stem}: {e}"))?;
        write(&root, out, fit_within(&sprite, max_w, max_h), &mut contact)?;
    }
    for (stem, out, max_w, max_h) in BACKDROPS {
        write(
            &root,
            out,
            fit_within(&open(&root, stem)?, max_w, max_h),
            &mut contact,
        )?;
    }
    for (stem, dir, names, max_w, max_h) in SHEETS {
        let sheet = open(&root, stem)?;
        let cut = extract_expression_cutout_sheet(&sheet, names, None)
            .map_err(|e| format!("{stem}: {e}"))?;
        for frame in cut.cutouts {
            // Frames share one canvas, so they stay the same size after scaling.
            let out = format!("{dir}/{}", frame.expression);
            write(
                &root,
                &out,
                fit_within(&frame.image, max_w, max_h),
                &mut contact,
            )?;
        }
    }

    let small: Vec<_> = contact
        .into_iter()
        .map(|(name, image)| (name, fit_within(&image, 320, 240)))
        .collect();
    let sheet = content::cutouts::make_contact_sheet(&small, 5);
    let path = root.join("target/demo-art-contact-sheet.png");
    fs::create_dir_all(path.parent().unwrap())?;
    sheet.save(&path)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn open(root: &Path, stem: &str) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    open_in(root, "demo", stem)
}

fn open_in(root: &Path, dir: &str, stem: &str) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    let path = root
        .join("design/art")
        .join(dir)
        .join(format!("{stem}.png"));
    Ok(image::open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .into_rgba8())
}

fn write(
    root: &Path,
    out: &str,
    image: RgbaImage,
    contact: &mut Vec<(String, RgbaImage)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join("assets/source/core").join(format!("{out}.png"));
    fs::create_dir_all(path.parent().unwrap())?;
    image.save(&path)?;
    println!("{out}: {}x{}", image.width(), image.height());
    contact.push((out.to_string(), image));
    Ok(())
}
