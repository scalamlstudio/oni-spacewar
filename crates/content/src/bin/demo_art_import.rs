//! Imports the First Playable demo art set (`design/art/demo/*-v1.png`, with
//! the restyled battle art from `design/art/demo-v2/` replacing what it
//! covers) into `assets/source/core/...`: keys out the flat backgrounds, cuts
//! the walk and icon sheets into frames, and scales everything to its in-game
//! size; Art's battle feedback strips are copied as they are. Run
//! `content-pipeline` afterwards to process, bundle and list the
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

/// Transparent margin (px, after scaling) around every single sprite, so no
/// opaque pixel touches the canvas edge (design/ART_GUIDELINES.md § Cropping;
/// checked by `crates/content/tests/asset_crop.rs`).
const SPRITE_MARGIN: u32 = 2;

/// Full-frame transparent scene images (no background to key): stem, output, max size.
/// (The battle background tile was dropped for the procedural nebula sky,
/// TAKOAI-58.) These demo-v3 transparent renders are 2000 x 1160, the largest
/// window they have to stay crisp in, with sky areas cut out for the runtime
/// nebula. The Title key art is a fresh transparent redraw (TAKOAI-85); do not
/// restore the old mask-and-recolour export path.
const BACKDROPS: [(&str, &str, u32, u32); 2] = [
    (
        "demo-v3/title-key-art-v4-transparent",
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

/// Animation strips copied unchanged: design file stem, pack-relative
/// output. Art's battle feedback pack (TAKOAI-111,
/// `design/art/battle/feedback/README-v1.md`) is already transparent, one
/// row of 256 x 256 frames with an 8 px margin; the client cuts the frames
/// (`fx::FX`).
const STRIPS: [(&str, &str); 8] = [
    ("battle/feedback/hit-enemy-v1", "battle/fx/hit_enemy"),
    ("battle/feedback/hit-ship-v1", "battle/fx/hit_ship"),
    (
        "battle/feedback/death-swarmer-v1",
        "battle/fx/death_swarmer",
    ),
    (
        "battle/feedback/death-spitter-v1",
        "battle/fx/death_spitter",
    ),
    (
        "battle/feedback/shockwave-impact-v1",
        "battle/fx/shockwave_impact",
    ),
    (
        "battle/feedback/scatter-impact-v1",
        "battle/fx/scatter_impact",
    ),
    // The Spitter's wind-up glow (4 frames) and its stretchable aim line
    // (one cell; the client stretches only its inner UV region).
    (
        "battle/feedback/spitter-windup-v1",
        "battle/fx/spitter_windup",
    ),
    (
        "battle/feedback/spitter-aim-line-v1",
        "battle/fx/spitter_aim_line",
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

    for (stem, out) in STRIPS {
        write(&root, out, open(&root, stem)?, &mut contact)?;
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
