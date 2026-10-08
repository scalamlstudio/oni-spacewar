//! Imports the First Playable demo art set (`design/art/demo/*-v1.png`, with
//! the restyled battle art from `design/art/demo-v2/` replacing what it
//! covers) into `assets/source/core/...`: keys out the flat backgrounds, cuts
//! the walk and icon sheets into frames, and scales everything to its in-game
//! size. Run `content-pipeline` afterwards to process, bundle and list the
//! results. Design files are named relative to `design/art/`.

use std::fs;
use std::path::{Path, PathBuf};

use content::cutouts::{extract_expression_cutout_sheet, extract_sprite, fit_within, pad};
use image::RgbaImage;

/// Single sprites: design file stem, pack-relative output, max width/height.
const SPRITES: [(&str, &str, u32, u32); 6] = [
    ("demo/battleship-kite-v1", "battle/ship/kite", 256, 256),
    (
        "demo/battleship-bulwark-v1",
        "battle/ship/bulwark",
        256,
        256,
    ),
    (
        "demo-v2/enemy-void-swarmer-v2",
        "battle/enemy/void_swarmer",
        192,
        192,
    ),
    (
        "demo-v2/enemy-void-spitter-v2",
        "battle/enemy/void_spitter",
        192,
        192,
    ),
    (
        "demo-v2/projectile-void-spitter-v2",
        "battle/fx/spit",
        128,
        128,
    ),
    (
        "demo-v2/void-fissure-v2",
        "battle/env/void_fissure",
        192,
        192,
    ),
];

/// The 2.5D carrier's modular pieces: the detailed painted set
/// (`design/art/carrier-2_5d-v2/`, TAKOAI-69) on the grid TAKOAI-55 set up.
/// Already transparent and authored at 2× on a shared 256 px-per-cell grid,
/// so they are copied as they are (the size is checked). Design file stem,
/// pack-relative output, expected width × height.
const CARRIER_DIR: &str = "carrier-2_5d-v2";
const CARRIER_2_5D: [(&str, &str, u32, u32); 28] = [
    ("room-bridge", "carrier/room/bridge", 768, 512),
    // The one-berth Dock (TAKOAI-60), drawn as one by Art since TAKOAI-69.
    ("room-dock", "carrier/room/dock", 512, 768),
    ("dock-berth", "carrier/dock/berth", 480, 244),
    ("room-crew-quarters", "carrier/room/crew_quarters", 512, 512),
    ("room-workshop", "carrier/room/workshop", 768, 512),
    ("room-salvage-bay", "carrier/room/salvage_bay", 512, 512),
    ("room-training-room", "carrier/room/training_room", 512, 512),
    ("door-n", "carrier/door/n", 256, 96),
    ("door-e", "carrier/door/e", 24, 256),
    ("door-s", "carrier/door/s", 256, 24),
    ("door-w", "carrier/door/w", 24, 256),
    ("corridor-n", "carrier/corridor/n", 256, 256),
    ("corridor-e", "carrier/corridor/e", 256, 256),
    ("corridor-s", "carrier/corridor/s", 256, 256),
    ("corridor-w", "carrier/corridor/w", 256, 256),
    ("corridor-ns", "carrier/corridor/ns", 256, 256),
    ("corridor-ew", "carrier/corridor/ew", 256, 256),
    ("corridor-ne", "carrier/corridor/ne", 256, 256),
    ("corridor-es", "carrier/corridor/es", 256, 256),
    ("corridor-sw", "carrier/corridor/sw", 256, 256),
    ("corridor-nw", "carrier/corridor/nw", 256, 256),
    ("corridor-nes", "carrier/corridor/nes", 256, 256),
    ("corridor-esw", "carrier/corridor/esw", 256, 256),
    ("corridor-nsw", "carrier/corridor/nsw", 256, 256),
    ("corridor-new", "carrier/corridor/new", 256, 256),
    ("corridor-nesw", "carrier/corridor/nesw", 256, 256),
    ("hull-floor", "carrier/hull_floor", 256, 256),
    ("build-slot", "carrier/build_slot", 256, 256),
];

/// Transparent margin (px, after scaling) around every single sprite, so no
/// opaque pixel touches the canvas edge (design/ART_GUIDELINES.md § Cropping;
/// checked by `crates/content/tests/asset_crop.rs`).
const SPRITE_MARGIN: u32 = 2;

/// Full-frame transparent scene images (no background to key): stem, output, max size.
/// (The battle background tile was dropped for the procedural nebula sky,
/// TAKOAI-58.) These demo-v3 transparent renders are 2000 x 1160, the largest
/// window they have to stay crisp in, with sky areas cut out for the runtime
/// nebula.
const BACKDROPS: [(&str, &str, u32, u32); 2] = [
    (
        "demo-v3/title-key-art-v3-transparent",
        "title/key_art",
        2000,
        1160,
    ),
    (
        "demo-v3/result-backdrop-v2-transparent",
        "result/backdrop",
        2000,
        1160,
    ),
];

/// Row sheets: stem, output directory, frame names left to right, max frame
/// size. A later sheet overwrites frames of the same name (the v2 loot icons
/// replace the v1 credits / Void Crystal icons). The v1 side-view Pilot walk
/// sheet is gone: the painted 8-direction Pilot
/// (`core.carrier.pilot.<dir>.*`, TAKOAI-68) is exported straight into
/// `assets/source/` by `design/art/demo-v3/pilot-8dir/make-pilot-8dir.py`.
const SHEETS: [(&str, &str, &[&str], u32, u32); 2] = [
    (
        "demo/demo-icons-sheet-v1",
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
    (
        "demo-v2/loot-icons-v2",
        "ui/icon",
        &["credits", "void_crystal"],
        128,
        128,
    ),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    let mut contact = Vec::new();

    for (stem, out, max_w, max_h) in SPRITES {
        let sheet = open(&root, stem)?;
        let sprite = extract_sprite(&sheet, 4).map_err(|e| format!("{stem}: {e}"))?;
        let m = SPRITE_MARGIN;
        let sprite = pad(&fit_within(&sprite, max_w - 2 * m, max_h - 2 * m), m);
        write(&root, out, sprite, &mut contact)?;
    }
    for (stem, out, max_w, max_h) in BACKDROPS {
        write(
            &root,
            out,
            fit_within(&open(&root, stem)?, max_w, max_h),
            &mut contact,
        )?;
    }
    for (stem, out, w, h) in CARRIER_2_5D {
        let image = open(&root, &format!("{CARRIER_DIR}/{stem}"))?;
        if image.dimensions() != (w, h) {
            return Err(format!(
                "{CARRIER_DIR}/{stem}: {:?}, expected {w}x{h}",
                image.dimensions()
            )
            .into());
        }
        write(&root, out, image, &mut contact)?;
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
    let path = root.join("design/art").join(format!("{stem}.png"));
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
