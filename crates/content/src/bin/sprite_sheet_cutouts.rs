use std::fs;
use std::path::{Path, PathBuf};

use content::cutouts::{extract_expression_cutouts, make_contact_sheet};

const CHARACTERS: [&str; 4] = ["oni-pilot", "oni-engineer", "oni-researcher", "oni-gunner"];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    let mut all_cutouts = Vec::new();

    for character in CHARACTERS {
        let sheet_path = repo_root
            .join("design/art/expressions")
            .join(format!("{character}-expression-sheet-v1.png"));
        let sheet = image::open(&sheet_path)?.into_rgba8();
        let cutouts = extract_expression_cutouts(&sheet)
            .map_err(|error| format!("{}: {error}", sheet_path.display()))?;

        let output_dir = repo_root
            .join("assets/source/core/portraits")
            .join(character.trim_start_matches("oni-"));
        fs::create_dir_all(&output_dir)?;

        for cutout in cutouts {
            let path = output_dir.join(format!("{}.png", cutout.expression));
            cutout.image.save(&path)?;
            all_cutouts.push((format!("{character}-{}", cutout.expression), cutout.image));
        }
    }

    remove_bad_strips(&repo_root)?;

    let contact_sheet = make_contact_sheet(&all_cutouts, 9);
    let contact_path = repo_root.join("target/expression-cutouts-contact-sheet.png");
    if let Some(parent) = contact_path.parent() {
        fs::create_dir_all(parent)?;
    }
    contact_sheet.save(&contact_path)?;
    println!("wrote {}", contact_path.display());
    Ok(())
}

fn remove_bad_strips(repo_root: &Path) -> std::io::Result<()> {
    let expressions_dir = repo_root.join("design/art/expressions");
    for entry in fs::read_dir(expressions_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.ends_with("-v1.png") && !name.contains("expression-sheet") {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}
