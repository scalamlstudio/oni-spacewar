//! Frame-consistency check for 8-direction characters (TAKOAI-80): every
//! shipped `<character>.<dir>.<frame>` image of one character, with `<dir>`
//! one of the 8 compass directions, must share one canvas contract, so the
//! client can size and anchor all of them alike:
//!
//! - the same canvas size;
//! - the feet (lowest opaque row) on the same baseline, within
//!   [`BASELINE_PX`];
//! - the same visible height (head top to feet) within [`HEIGHT_SHARE`], so
//!   the head isn't bigger or smaller facing away than facing the camera;
//! - a transparent margin at every edge (nothing clipped).
//!
//! Characters are found in the manifest, so a new 8-direction character is
//! checked as soon as it ships.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use content::ContentManifest;

/// Alpha above this counts as opaque (soft key edges stay below it), as in
/// `asset_crop.rs`.
const OPAQUE_ALPHA: u8 = 8;
/// How far a frame's feet may sit from the character's median baseline.
const BASELINE_PX: u32 = 2;
/// How far a frame's visible height may stray from the character's median,
/// as a share of that median.
const HEIGHT_SHARE: f32 = 0.04;

const DIRECTIONS: [&str; 8] = ["n", "ne", "e", "se", "s", "sw", "w", "nw"];

/// Frames that break the contract and wait for an art fix (`id`, reason).
/// The test fails if one of them starts passing, so the entry gets removed
/// with the fix.
const KNOWN_BAD: &[(&str, &str)] = &[
    // TAKOAI-79's back-view repaint draws the ears lower than every other
    // facing: 141 px tall against 150 px, so the head reads smaller facing
    // away. Waits for an Art follow-up.
    ("core.carrier.pilot.n.idle", "N ears 9 px low"),
    ("core.carrier.pilot.n.walk_4", "N ears 9 px low"),
];

fn assets_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

/// Opaque bounds of one frame.
#[derive(Debug)]
struct Frame {
    id: String,
    size: (u32, u32),
    /// First opaque row.
    top: u32,
    /// One past the last opaque row: the feet baseline.
    feet: u32,
    left: u32,
    right: u32,
}

impl Frame {
    fn measure(id: &str, image: &image::RgbaImage) -> Option<Self> {
        let (w, h) = image.dimensions();
        let opaque = |x: u32, y: u32| image.get_pixel(x, y)[3] > OPAQUE_ALPHA;
        let row = |y: u32| (0..w).any(|x| opaque(x, y));
        let col = |x: u32| (0..h).any(|y| opaque(x, y));
        Some(Self {
            id: id.into(),
            size: (w, h),
            top: (0..h).find(|&y| row(y))?,
            feet: (0..h).rev().find(|&y| row(y))? + 1,
            left: (0..w).find(|&x| col(x))?,
            right: (0..w).rev().find(|&x| col(x))? + 1,
        })
    }

    fn height(&self) -> u32 {
        self.feet - self.top
    }

    fn clipped(&self) -> bool {
        self.top == 0 || self.left == 0 || self.feet == self.size.1 || self.right == self.size.0
    }
}

/// Every 8-direction character in the manifest: character prefix -> its
/// frame IDs.
fn characters(manifest: &ContentManifest) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for pack in &manifest.packs {
        for asset in &pack.assets {
            if asset.kind != "image/png" {
                continue;
            }
            let mut parts = asset.id.rsplitn(3, '.');
            let (Some(_frame), Some(dir), Some(character)) =
                (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            if DIRECTIONS.contains(&dir) {
                out.entry(character.into())
                    .or_default()
                    .push(asset.id.clone());
            }
        }
    }
    // A lone `n` or `e` segment isn't a character; it needs several facings.
    out.retain(|_, ids| {
        let mut dirs: Vec<&str> = ids
            .iter()
            .map(|id| id.rsplit('.').nth(1).unwrap())
            .collect();
        dirs.dedup();
        dirs.len() >= 3
    });
    out
}

fn median(mut values: Vec<u32>) -> u32 {
    values.sort_unstable();
    values[values.len() / 2]
}

/// Contract violations of one character's frames.
fn violations(frames: &[Frame]) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let size = frames[0].size;
    let feet = median(frames.iter().map(|f| f.feet).collect());
    let height = median(frames.iter().map(Frame::height).collect());
    for f in frames {
        let mut add = |m: String| out.entry(f.id.clone()).or_default().push(m);
        if f.size != size {
            add(format!("canvas {:?}, others {:?}", f.size, size));
        }
        if f.feet.abs_diff(feet) > BASELINE_PX {
            add(format!("feet at row {}, baseline {feet}", f.feet));
        }
        let off = f.height().abs_diff(height) as f32 / height as f32;
        if off > HEIGHT_SHARE {
            add(format!(
                "visible height {} px, median {height} px ({:+.0}%)",
                f.height(),
                (f.height() as f32 / height as f32 - 1.0) * 100.0
            ));
        }
        if f.clipped() {
            add("opaque pixels touch the canvas edge".into());
        }
    }
    out
}

#[test]
fn eight_direction_frames_share_one_canvas_contract() {
    let root = assets_root();
    let manifest = ContentManifest::load(root.join("manifest.json")).unwrap();
    let characters = characters(&manifest);
    assert!(
        characters.contains_key("core.carrier.pilot"),
        "no Pilot frames found: {:?}",
        characters.keys()
    );
    let mut bad = Vec::new();
    let mut fixed = Vec::new();
    for (character, ids) in &characters {
        let frames: Vec<Frame> = ids
            .iter()
            .map(|id| {
                let path = manifest.processed_path(&root, id).unwrap();
                let image = image::open(&path).unwrap().to_rgba8();
                Frame::measure(id, &image).unwrap_or_else(|| panic!("{id} is empty"))
            })
            .collect();
        for f in &frames {
            eprintln!(
                "{:28} {}x{} top {:3} feet {:3} height {:3} x {:3}..{:3}",
                f.id.strip_prefix(character.as_str()).unwrap_or(&f.id),
                f.size.0,
                f.size.1,
                f.top,
                f.feet,
                f.height(),
                f.left,
                f.right
            );
        }
        let found = violations(&frames);
        for id in ids {
            let known = KNOWN_BAD.iter().any(|(k, _)| k == id);
            match (found.get(id), known) {
                (Some(why), true) => eprintln!("known bad: {id}: {}", why.join("; ")),
                (Some(why), false) => bad.push(format!("{id}: {}", why.join("; "))),
                (None, true) => fixed.push(id.clone()),
                (None, false) => {}
            }
        }
    }
    assert!(
        bad.is_empty(),
        "8-direction frames break the canvas contract:\n  {}",
        bad.join("\n  ")
    );
    assert!(
        fixed.is_empty(),
        "now within the contract, remove from KNOWN_BAD: {fixed:?}"
    );
}

#[test]
fn contract_flags_baseline_height_and_clipping() {
    let mut images = vec![image::RgbaImage::new(20, 20); 4];
    let ink = image::Rgba([255, 255, 255, 255]);
    // Three good frames: rows 2..18, columns 5..15.
    for image in &mut images {
        for y in 2..18 {
            image.put_pixel(5, y, ink);
            image.put_pixel(14, y, ink);
        }
    }
    // The fourth starts at row 6: shorter, but feet on the baseline.
    for y in 2..6 {
        images[3].put_pixel(5, y, image::Rgba([0, 0, 0, 0]));
        images[3].put_pixel(14, y, image::Rgba([0, 0, 0, 0]));
    }
    let frames: Vec<Frame> = images
        .iter()
        .enumerate()
        .map(|(i, im)| Frame::measure(&format!("t.s.{i}"), im).unwrap())
        .collect();
    let found = violations(&frames);
    assert_eq!(found.keys().collect::<Vec<_>>(), ["t.s.3"]);
    assert!(found["t.s.3"][0].starts_with("visible height 12 px"));

    // Feet three rows up, and a pixel on the right edge.
    let mut lifted = image::RgbaImage::new(20, 20);
    for y in 2..15 {
        lifted.put_pixel(5, y, ink);
    }
    lifted.put_pixel(19, 10, ink);
    let mut frames = frames;
    frames.pop();
    frames.push(Frame::measure("t.n.0", &lifted).unwrap());
    let found = violations(&frames);
    let why = &found["t.n.0"];
    assert!(
        why.iter().any(|m| m.starts_with("feet at row 15")),
        "{why:?}"
    );
    assert!(why.iter().any(|m| m.contains("touch")), "{why:?}");
}

/// The pipeline copies PNGs byte for byte, so source, processed file and
/// manifest hash must agree. A source edit without a pipeline re-run ships
/// the old art silently (TAKOAI-80: the N/NE repaint never reached the game).
#[test]
fn processed_images_match_their_source() {
    let root = assets_root();
    let manifest = ContentManifest::load(root.join("manifest.json")).unwrap();
    let mut stale = Vec::new();
    for pack in &manifest.packs {
        for asset in &pack.assets {
            if asset.kind != "image/png" {
                continue;
            }
            let source = std::fs::read(root.join(&asset.source)).unwrap();
            let processed = std::fs::read(root.join(&asset.processed)).unwrap();
            if content::sha256_hex(&source) != asset.hash || source != processed {
                stale.push(asset.id.clone());
            }
        }
    }
    assert!(
        stale.is_empty(),
        "processed images are stale, re-run `cargo run --bin content-pipeline`:\n  {}",
        stale.join("\n  ")
    );
}
