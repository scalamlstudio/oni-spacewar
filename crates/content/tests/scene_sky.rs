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
