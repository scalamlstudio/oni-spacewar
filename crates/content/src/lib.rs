//! Versioned content-pack manifests and stable asset ID resolution.
//!
//! This crate is intentionally independent from `sim`. Client/rendering code can
//! read shipped assets from disk, while deterministic gameplay receives only
//! validated data that has already been agreed by manifest hash/version.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub enum ContentError {
    Io(io::Error),
    Json(serde_json::Error),
    MissingPack(String),
    MissingAsset(String),
    BadManifest(String),
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Json(e) => write!(f, "{e}"),
            Self::MissingPack(id) => write!(f, "missing content pack `{id}`"),
            Self::MissingAsset(id) => write!(f, "missing asset id `{id}`"),
            Self::BadManifest(msg) => write!(f, "bad content manifest: {msg}"),
        }
    }
}

impl std::error::Error for ContentError {}

impl From<io::Error> for ContentError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ContentError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentManifest {
    pub schema_version: u32,
    pub sim_version: String,
    pub packs: Vec<ContentPack>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentPack {
    pub id: String,
    pub version: String,
    pub gameplay_affecting: bool,
    pub content_hash: String,
    pub assets: Vec<ContentAsset>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentAsset {
    pub id: String,
    pub kind: String,
    pub source: String,
    pub processed: String,
    pub compressed: String,
    pub hash: String,
    pub gameplay_affecting: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackPatch {
    pub pack_id: String,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub changed_assets: Vec<String>,
}

impl ContentManifest {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ContentError> {
        let manifest = serde_json::from_slice(&fs::read(path)?)?;
        validate_manifest(&manifest)?;
        Ok(manifest)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ContentError> {
        validate_manifest(self)?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn pack(&self, id: &str) -> Result<&ContentPack, ContentError> {
        self.packs
            .iter()
            .find(|pack| pack.id == id)
            .ok_or_else(|| ContentError::MissingPack(id.to_owned()))
    }

    pub fn asset(&self, id: &str) -> Result<&ContentAsset, ContentError> {
        self.packs
            .iter()
            .flat_map(|pack| &pack.assets)
            .find(|asset| asset.id == id)
            .ok_or_else(|| ContentError::MissingAsset(id.to_owned()))
    }

    pub fn processed_path(
        &self,
        assets_root: impl AsRef<Path>,
        id: &str,
    ) -> Result<PathBuf, ContentError> {
        Ok(assets_root.as_ref().join(&self.asset(id)?.processed))
    }

    pub fn load_text(
        &self,
        assets_root: impl AsRef<Path>,
        id: &str,
    ) -> Result<String, ContentError> {
        Ok(fs::read_to_string(self.processed_path(assets_root, id)?)?)
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn pack_hash(assets: &[ContentAsset]) -> String {
    let mut hasher = Sha256::new();
    for asset in assets {
        hasher.update(asset.id.as_bytes());
        hasher.update([0]);
        hasher.update(asset.hash.as_bytes());
        hasher.update([0]);
        hasher.update(asset.processed.as_bytes());
        hasher.update([0]);
        hasher.update(asset.compressed.as_bytes());
        hasher.update([0]);
        hasher.update([asset.gameplay_affecting as u8]);
    }
    format!("{:x}", hasher.finalize())
}

pub fn diff_manifests(old: &ContentManifest, new: &ContentManifest) -> Vec<PackPatch> {
    let old_packs: BTreeMap<_, _> = old
        .packs
        .iter()
        .map(|pack| (pack.id.as_str(), pack))
        .collect();
    let new_packs: BTreeMap<_, _> = new
        .packs
        .iter()
        .map(|pack| (pack.id.as_str(), pack))
        .collect();
    let ids: BTreeSet<_> = old_packs.keys().chain(new_packs.keys()).copied().collect();

    ids.into_iter()
        .filter_map(|pack_id| {
            let old_pack = old_packs.get(pack_id).copied();
            let new_pack = new_packs.get(pack_id).copied();
            if old_pack == new_pack {
                return None;
            }
            let changed_assets = changed_asset_ids(old_pack, new_pack);
            Some(PackPatch {
                pack_id: pack_id.to_owned(),
                old_version: old_pack.map(|pack| pack.version.clone()),
                new_version: new_pack.map(|pack| pack.version.clone()),
                changed_assets,
            })
        })
        .collect()
}

fn changed_asset_ids(
    old_pack: Option<&ContentPack>,
    new_pack: Option<&ContentPack>,
) -> Vec<String> {
    let old_assets: BTreeMap<_, _> = old_pack
        .into_iter()
        .flat_map(|pack| &pack.assets)
        .map(|asset| (asset.id.as_str(), asset))
        .collect();
    let new_assets: BTreeMap<_, _> = new_pack
        .into_iter()
        .flat_map(|pack| &pack.assets)
        .map(|asset| (asset.id.as_str(), asset))
        .collect();
    let ids: BTreeSet<_> = old_assets
        .keys()
        .chain(new_assets.keys())
        .copied()
        .collect();
    ids.into_iter()
        .filter(|id| old_assets.get(id).copied() != new_assets.get(id).copied())
        .map(str::to_owned)
        .collect()
}

pub fn validate_manifest(manifest: &ContentManifest) -> Result<(), ContentError> {
    if manifest.schema_version == 0 {
        return Err(ContentError::BadManifest(
            "schema_version must be non-zero".into(),
        ));
    }
    let mut pack_ids = BTreeSet::new();
    let mut asset_ids = BTreeSet::new();
    for pack in &manifest.packs {
        if !pack_ids.insert(pack.id.as_str()) {
            return Err(ContentError::BadManifest(format!(
                "duplicate pack id `{}`",
                pack.id
            )));
        }
        if pack.content_hash != pack_hash(&pack.assets) {
            return Err(ContentError::BadManifest(format!(
                "pack `{}` hash mismatch",
                pack.id
            )));
        }
        for asset in &pack.assets {
            if !asset_ids.insert(asset.id.as_str()) {
                return Err(ContentError::BadManifest(format!(
                    "duplicate asset id `{}`",
                    asset.id
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str, asset_hash: &str) -> ContentManifest {
        let asset = ContentAsset {
            id: "core.ui.hud_status".into(),
            kind: "text".into(),
            source: "source/core/ui/hud_status.txt".into(),
            processed: "packs/core/processed/ui/hud_status.txt".into(),
            compressed: "packs/core/bundles/ui/hud_status.txt.zst".into(),
            hash: asset_hash.into(),
            gameplay_affecting: false,
        };
        let assets = vec![asset];
        ContentManifest {
            schema_version: 1,
            sim_version: "0.1.0".into(),
            packs: vec![ContentPack {
                id: "core".into(),
                version: version.into(),
                gameplay_affecting: false,
                content_hash: pack_hash(&assets),
                assets,
            }],
        }
    }

    #[test]
    fn unchanged_manifests_need_no_patch() {
        assert!(diff_manifests(&manifest("0.1.0", "a"), &manifest("0.1.0", "a")).is_empty());
    }

    #[test]
    fn changed_asset_lists_pack_for_patch() {
        let diff = diff_manifests(&manifest("0.1.0", "a"), &manifest("0.1.1", "b"));
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].pack_id, "core");
        assert_eq!(diff[0].changed_assets, ["core.ui.hud_status"]);
    }
}
