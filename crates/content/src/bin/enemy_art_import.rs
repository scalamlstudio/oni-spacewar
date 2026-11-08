//! Imports Art's new void enemy source sheets (TAKOAI-114,
//! `design/art/enemies/README-v1.md`) into `assets/source/core/battle/...`.
//! Each sheet is one row of loose items (poses, rings, lines, bursts) on a
//! white background; they are cut out by what they show
//! (`cutouts::extract_row_items`), never by equal widths, and centred on
//! the suggested padded canvas. Run `content-pipeline` afterwards.
//!
//! The Step 3 roles (Lancer, Bloater, Brood, Warden) and the Step 4 elite
//! sheets (affix icons, aura, burning patch, teleport mark) are imported;
//! the Fissure Guardian comes with its own step. Each enemy's resting body
//! goes to `battle/enemy/` (one codex entry each); its other poses to
//! `battle/pose/`; elite pieces to `battle/elite/`.

use std::fs;
use std::path::PathBuf;

use content::cutouts::{centre_on_canvas, extract_row_items, make_contact_sheet};

/// Enclosed background patches at least this big are keyed out (ring
/// interiors); smaller ones are white highlights and stay.
const HOLE_MIN_AREA: usize = 6000;

/// Transparent margin inside every canvas (design/ART_GUIDELINES.md § Cropping).
const MARGIN: u32 = 6;

/// Canvas sizes from the README's suggestions.
const BODY: (u32, u32) = (256, 256);
const LINE: (u32, u32) = (512, 64);
const TETHER: (u32, u32) = (256, 32);
const ICON: (u32, u32) = (64, 64);

type Item = (&'static str, (u32, u32));

/// Sheet stem under `design/art/enemies/`, then each item left to right:
/// pack-relative output (or "" to skip) and canvas size.
const SHEETS: [(&str, &[Item]); 6] = [
    (
        "lancer-source-v1",
        &[
            ("battle/enemy/void_lancer", BODY),
            ("battle/pose/void_lancer_windup", BODY),
            ("battle/pose/void_lancer_dash", BODY),
            ("battle/fx/lancer_charge_line", LINE),
            ("battle/fx/death_lancer", BODY),
        ],
    ),
    (
        "bloater-source-v1",
        &[
            ("battle/enemy/void_bloater", BODY),
            ("battle/pose/void_bloater_swelling", BODY),
            ("battle/fx/bloater_warning_ring", BODY),
            ("battle/fx/bloater_explosion", BODY),
            ("battle/fx/death_bloater", BODY),
        ],
    ),
    (
        "brood-source-v1",
        &[
            ("battle/enemy/void_brood", BODY),
            ("battle/pose/void_brood_windup", BODY),
            // The standalone pulsing sac: the client throbs the wind-up pose instead.
            ("", BODY),
            ("battle/fx/death_brood", BODY),
        ],
    ),
    (
        "warden-source-v1",
        &[
            ("battle/enemy/void_warden", BODY),
            ("battle/fx/warden_tether", TETHER),
            ("battle/fx/warden_shield", BODY),
            ("battle/fx/death_warden", BODY),
        ],
    ),
    (
        "elite-affixes-source-v1",
        &[
            ("battle/elite/affix_swift", ICON),
            ("battle/elite/affix_volatile", ICON),
            ("battle/elite/affix_warding", ICON),
            ("battle/elite/affix_blinking", ICON),
            ("battle/elite/affix_splitting", ICON),
            ("battle/elite/affix_scorching", ICON),
            ("battle/elite/affix_chilling", ICON),
        ],
    ),
    (
        "elite-effects-source-v1",
        &[
            // Three aura options, none final (README-v1.md): ship the first
            // (plain) one as the elite outline; the others stay in design/.
            ("battle/elite/aura", BODY),
            ("", BODY),
            ("", BODY),
            ("battle/elite/burning_patch", BODY),
            ("battle/elite/teleport_mark", BODY),
        ],
    ),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    let mut contact = Vec::new();
    for (stem, items) in SHEETS {
        let path = root.join(format!("design/art/enemies/{stem}.png"));
        let sheet = image::open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .into_rgba8();
        let cut = extract_row_items(&sheet, items.len(), HOLE_MIN_AREA)
            .map_err(|e| format!("{stem}: {e}"))?;
        for (image, (out, (w, h))) in cut.into_iter().zip(items) {
            if out.is_empty() {
                continue;
            }
            let image = centre_on_canvas(&image, *w, *h, MARGIN);
            let path = root.join("assets/source/core").join(format!("{out}.png"));
            fs::create_dir_all(path.parent().unwrap())?;
            image.save(&path)?;
            println!("{out}: {}x{}", image.width(), image.height());
            contact.push((out.to_string(), image));
        }
    }
    let sheet = make_contact_sheet(&contact, 5);
    // A review sheet, next to the build output rather than in the checkout.
    let dir =
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let path = dir.join("enemy-art-contact-sheet.png");
    fs::create_dir_all(path.parent().unwrap())?;
    sheet.save(&path)?;
    println!("wrote {}", path.display());
    Ok(())
}
