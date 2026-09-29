use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use content::{pack_hash, sha256_hex, ContentAsset, ContentManifest, ContentPack};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let assets_root = PathBuf::from(repo_root).join("assets");
    let source = "source/core/ui/hud_status.txt";
    let processed = "packs/core/processed/ui/hud_status.txt";
    let compressed = "packs/core/bundles/ui/hud_status.txt.zst";

    let source_path = assets_root.join(source);
    let mut text = fs::read_to_string(&source_path)?;
    if !text.ends_with('\n') {
        text.push('\n');
    }

    write_text(&assets_root.join(processed), &text)?;
    compress_zstd(&assets_root.join(processed), &assets_root.join(compressed))?;

    let asset = ContentAsset {
        id: "core.ui.hud_status".into(),
        kind: "text".into(),
        source: source.into(),
        processed: processed.into(),
        compressed: compressed.into(),
        hash: sha256_hex(text.as_bytes()),
        gameplay_affecting: false,
    };
    let assets = vec![asset];
    let manifest = ContentManifest {
        schema_version: 1,
        sim_version: env!("CARGO_PKG_VERSION").into(),
        packs: vec![ContentPack {
            id: "core".into(),
            version: "0.1.0".into(),
            gameplay_affecting: false,
            content_hash: pack_hash(&assets),
            assets,
        }],
    };
    manifest.save(assets_root.join("manifest.json"))?;
    println!("wrote assets/manifest.json");
    Ok(())
}

fn write_text(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

fn compress_zstd(input: &Path, output: &Path) -> io::Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let status = Command::new("zstd")
        .args(["-q", "-f", "-10", "-o"])
        .arg(output)
        .arg(input)
        .status()?;
    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("zstd failed with status {status}"),
        ));
    }
    Ok(())
}
