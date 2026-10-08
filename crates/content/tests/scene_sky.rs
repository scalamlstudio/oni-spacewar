//! Transparent-sky contract (TAKOAI-83): the Title key art and the Result
//! backdrop carry no painted sky, so the shared nebula shader shows through
//! behind them. A re-export with an opaque sky fails here.

use std::path::Path;

/// Side of each probed square, in px.
const PROBE: u32 = 32;

/// A named probe square centre, as fractions of the image size.
type Probe = (&'static str, f32, f32);

/// Scene art drawn over the nebula sky (relative to `assets/source/core/`)
/// and where its sky must be clear: probe square centres as fractions of the
/// image size. The key art is open sky in all four corners; the Result
/// bridge frame fills the edges, so its sky is the three window panes.
const SKY_SCENES: &[(&str, &[Probe])] = &[
    (
        "title/key_art.png",
        &[
            ("top-left corner", 0.0, 0.0),
            ("top-right corner", 1.0, 0.0),
            ("bottom-left corner", 0.0, 1.0),
            ("bottom-right corner", 1.0, 1.0),
        ],
    ),
    (
        "result/backdrop.png",
        &[
            ("left pane", 0.17, 0.4),
            ("centre pane", 0.5, 0.4),
            ("right pane", 0.83, 0.4),
        ],
    ),
];

#[test]
fn scene_art_has_a_transparent_sky() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/source/core");
    let mut failures = Vec::new();
    for (rel, probes) in SKY_SCENES {
        let decoded = image::open(root.join(rel)).unwrap();
        if !decoded.color().has_alpha() {
            failures.push(format!("{rel}: no alpha channel"));
            continue;
        }
        let image = decoded.to_rgba8();
        let (w, h) = image.dimensions();
        for (name, fx, fy) in *probes {
            // The probe square, centred on the point and kept on the canvas.
            let at = |f: f32, size: u32| {
                ((f * size as f32) as u32).clamp(PROBE / 2, size - PROBE / 2) - PROBE / 2
            };
            let (x0, y0) = (at(*fx, w), at(*fy, h));
            let max = (y0..y0 + PROBE)
                .flat_map(|y| (x0..x0 + PROBE).map(move |x| (x, y)))
                .map(|(x, y)| image.get_pixel(x, y)[3])
                .max()
                .unwrap();
            if max != 0 {
                failures.push(format!("{rel}: {name} has alpha up to {max}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "scene art must leave the sky transparent:\n{}",
        failures.join("\n")
    );
}

#[test]
fn title_key_art_has_no_opaque_old_sky_pixels() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/source/core/title/key_art.png");
    let image = image::open(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .to_rgba8();
    let mut offenders = Vec::new();
    for (x, y, pixel) in image.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        if a > 0 && is_old_sky_color(r, g, b) {
            offenders.push((x, y, r, g, b, a));
            if offenders.len() >= 12 {
                break;
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "title/key_art.png contains opaque pixels in the old blue-violet sky range: {offenders:?}"
    );
}

fn is_old_sky_color(r: u8, g: u8, b: u8) -> bool {
    let (h, s, v) = rgb_to_hsv(r, g, b);
    // The prior painted sky lived in saturated royal-blue through violet.
    // Teal/cyan engine light is intentionally outside this hue band.
    (220.0..=292.0).contains(&h) && s >= 0.32 && v >= 0.18
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };
    let saturation = if max == 0.0 { 0.0 } else { delta / max };
    (hue, saturation, max)
}
