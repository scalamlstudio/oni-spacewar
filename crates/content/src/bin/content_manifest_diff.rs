use content::{diff_manifests, ContentManifest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(old_path) = args.next() else {
        eprintln!("usage: content-manifest-diff OLD_MANIFEST NEW_MANIFEST");
        std::process::exit(2);
    };
    let Some(new_path) = args.next() else {
        eprintln!("usage: content-manifest-diff OLD_MANIFEST NEW_MANIFEST");
        std::process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("usage: content-manifest-diff OLD_MANIFEST NEW_MANIFEST");
        std::process::exit(2);
    }

    let old = ContentManifest::load(old_path)?;
    let new = ContentManifest::load(new_path)?;
    let patches = diff_manifests(&old, &new);
    if patches.is_empty() {
        println!("no content packs changed");
        return Ok(());
    }
    for patch in patches {
        let old_version = patch.old_version.as_deref().unwrap_or("<missing>");
        let new_version = patch.new_version.as_deref().unwrap_or("<missing>");
        println!(
            "{} {} -> {}: {}",
            patch.pack_id,
            old_version,
            new_version,
            patch.changed_assets.join(", ")
        );
    }
    Ok(())
}
