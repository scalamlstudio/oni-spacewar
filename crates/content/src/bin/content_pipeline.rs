use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use content::{pack_hash, sha256_hex, ContentAsset, ContentManifest, ContentPack};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let assets_root = PathBuf::from(repo_root).join("assets");
    let mut assets = Vec::new();

    let source = "source/core/ui/hud_status.txt";
    let source_path = assets_root.join(source);
    let mut text = fs::read_to_string(&source_path)?;
    if !text.ends_with('\n') {
        text.push('\n');
    }

    let processed = "packs/core/processed/ui/hud_status.txt";
    let compressed = "packs/core/bundles/ui/hud_status.txt.zst";
    write_bytes(&assets_root.join(processed), text.as_bytes())?;
    compress_zstd(&assets_root.join(processed), &assets_root.join(compressed))?;
    assets.push(ContentAsset {
        id: "core.ui.hud_status".into(),
        kind: "text".into(),
        source: source.into(),
        processed: processed.into(),
        compressed: compressed.into(),
        hash: sha256_hex(text.as_bytes()),
        gameplay_affecting: false,
    });

    for portrait in discover_portraits(&assets_root.join("source/core/portraits"))? {
        let source_path = assets_root.join(&portrait.source);
        let bytes = fs::read(&source_path)?;
        write_bytes(&assets_root.join(&portrait.processed), &bytes)?;
        compress_zstd(
            &assets_root.join(&portrait.processed),
            &assets_root.join(&portrait.compressed),
        )?;
        assets.push(ContentAsset {
            id: portrait.id,
            kind: "image/png".into(),
            source: portrait.source,
            processed: portrait.processed,
            compressed: portrait.compressed,
            hash: sha256_hex(&bytes),
            gameplay_affecting: false,
        });
    }

    assets.sort_by(|a, b| a.id.cmp(&b.id));
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

struct SourceAsset {
    id: String,
    source: String,
    processed: String,
    compressed: String,
}

fn discover_portraits(root: &Path) -> io::Result<Vec<SourceAsset>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    for character in fs::read_dir(root)? {
        let character = character?;
        if !character.file_type()?.is_dir() {
            continue;
        }
        let character_name = character.file_name().to_string_lossy().to_string();
        for expression in fs::read_dir(character.path())? {
            let expression = expression?;
            if !expression.file_type()?.is_file() {
                continue;
            }
            let path = expression.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("png") {
                continue;
            }
            let expression_name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "bad filename"))?;
            let relative = format!("portraits/{character_name}/{expression_name}.png");
            files.push(SourceAsset {
                id: format!("core.portraits.{character_name}.{expression_name}"),
                source: format!("source/core/{relative}"),
                processed: format!("packs/core/processed/{relative}"),
                compressed: format!("packs/core/bundles/{relative}.zst"),
            });
        }
    }
    files.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(files)
}

fn write_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
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
