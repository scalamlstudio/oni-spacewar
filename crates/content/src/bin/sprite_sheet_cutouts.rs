use std::fs;
use std::path::{Path, PathBuf};

use content::cutouts::{extract_expression_cutout_sheet, make_contact_sheet, EXPRESSIONS};

const CHARACTERS: [&str; 4] = ["oni-pilot", "oni-engineer", "oni-researcher", "oni-gunner"];
const DEFAULT_SHEET_SUFFIX: &str = "v1";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse(std::env::args().skip(1))?;
    let repo_root = args.repo_root;
    let mut all_cutouts = Vec::new();

    for character in CHARACTERS {
        let output_dir = repo_root
            .join("assets/source/core/portraits")
            .join(character.trim_start_matches("oni-"));
        fs::create_dir_all(&output_dir)?;

        let default_path = sheet_path(&repo_root, character, DEFAULT_SHEET_SUFFIX);
        let sheet = image::open(&default_path)?.into_rgba8();
        let default_sheet = extract_expression_cutout_sheet(&sheet, &EXPRESSIONS, None)
            .map_err(|error| format!("{}: {error}", default_path.display()))?;

        for cutout in default_sheet.cutouts {
            write_cutout(&output_dir, character, cutout, &mut all_cutouts)?;
        }

        if let Some(extra_sheet) = &args.extra_sheet {
            let extra_path = sheet_path(&repo_root, character, &extra_sheet.suffix);
            let sheet = image::open(&extra_path)?.into_rgba8();
            let cutout_sheet = extract_expression_cutout_sheet(
                &sheet,
                &extra_sheet.expressions,
                Some(default_sheet.layout),
            )
            .map_err(|error| format!("{}: {error}", extra_path.display()))?;
            for cutout in cutout_sheet.cutouts {
                write_cutout(&output_dir, character, cutout, &mut all_cutouts)?;
            }
        }
    }

    remove_bad_strips(&repo_root)?;

    let contact_columns = if args.extra_sheet.is_some() { 11 } else { 9 };
    let contact_sheet = make_contact_sheet(&all_cutouts, contact_columns);
    let contact_path = repo_root.join("target/expression-cutouts-contact-sheet.png");
    if let Some(parent) = contact_path.parent() {
        fs::create_dir_all(parent)?;
    }
    contact_sheet.save(&contact_path)?;
    println!("wrote {}", contact_path.display());
    Ok(())
}

struct Args {
    repo_root: PathBuf,
    extra_sheet: Option<ExtraSheet>,
}

struct ExtraSheet {
    suffix: String,
    expressions: Vec<String>,
}

impl Args {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut repo_root = PathBuf::from(".");
        let mut saw_repo_root = false;
        let mut extra_suffix = None;
        let mut extra_expressions = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--extra-sheet-suffix" => {
                    extra_suffix =
                        Some(args.next().ok_or("--extra-sheet-suffix requires a value")?);
                }
                "--extra-expressions" | "--expressions" => {
                    let value = args
                        .next()
                        .ok_or("--extra-expressions requires a comma-separated value")?;
                    extra_expressions = Some(parse_expressions(&value)?);
                }
                "-h" | "--help" => return Err(usage()),
                _ if arg.starts_with('-') => {
                    return Err(format!("{arg}: unknown option\n{}", usage()))
                }
                _ if !saw_repo_root => {
                    repo_root = PathBuf::from(arg);
                    saw_repo_root = true;
                }
                _ => return Err(format!("{arg}: unexpected argument\n{}", usage())),
            }
        }

        let extra_sheet = match (extra_suffix, extra_expressions) {
            (None, None) => None,
            (Some(suffix), Some(expressions)) => Some(ExtraSheet {
                suffix,
                expressions,
            }),
            _ => {
                return Err(
                    "--extra-sheet-suffix and --extra-expressions must be provided together".into(),
                )
            }
        };

        Ok(Self {
            repo_root,
            extra_sheet,
        })
    }
}

fn parse_expressions(value: &str) -> Result<Vec<String>, String> {
    let expressions = value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if expressions.is_empty() {
        return Err("expression list must not be empty".into());
    }
    Ok(expressions)
}

fn usage() -> String {
    "usage: sprite-sheet-cutouts [repo-root] [--extra-sheet-suffix SUFFIX --extra-expressions a,b]"
        .into()
}

fn sheet_path(repo_root: &Path, character: &str, suffix: &str) -> PathBuf {
    repo_root
        .join("design/art/expressions")
        .join(format!("{character}-expression-sheet-{suffix}.png"))
}

fn write_cutout(
    output_dir: &Path,
    character: &str,
    cutout: content::cutouts::Cutout,
    all_cutouts: &mut Vec<(String, image::RgbaImage)>,
) -> Result<(), image::ImageError> {
    let path = output_dir.join(format!("{}.png", cutout.expression));
    cutout.image.save(&path)?;
    all_cutouts.push((format!("{character}-{}", cutout.expression), cutout.image));
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
