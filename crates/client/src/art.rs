//! Shipped images and fonts by stable content ID. Each ID is resolved through
//! `assets/manifest.json` to its processed file, decoded once and cached.
//! Client only; the sim never sees these.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use content::ContentManifest;

#[derive(Resource, Default)]
pub struct ContentImages {
    manifest: Option<ContentManifest>,
    loaded: HashMap<String, Option<Handle<Image>>>,
    fonts: HashMap<String, Option<Handle<Font>>>,
}

impl ContentImages {
    /// The image shipped under `id`, or `None` (logged once) if it is missing.
    pub fn get(&mut self, images: &mut Assets<Image>, id: &str) -> Option<Handle<Image>> {
        if let Some(handle) = self.loaded.get(id) {
            return handle.clone();
        }
        let handle = self
            .read(id)
            .and_then(|bytes| {
                Image::from_buffer(
                    &bytes,
                    ImageType::Extension("png"),
                    CompressedImageFormats::NONE,
                    true,
                    ImageSampler::Default,
                    RenderAssetUsages::default(),
                )
                .ok()
            })
            .map(|image| images.add(image));
        if handle.is_none() {
            warn!("image {id} unavailable");
        }
        self.loaded.insert(id.to_string(), handle.clone());
        handle
    }

    /// The font shipped under `id`, or `None` (logged once) if it is missing.
    pub fn font(&mut self, fonts: &mut Assets<Font>, id: &str) -> Option<Handle<Font>> {
        if let Some(handle) = self.fonts.get(id) {
            return handle.clone();
        }
        let handle = self
            .read(id)
            .map(|bytes| fonts.add(Font::from_bytes(bytes)));
        if handle.is_none() {
            warn!("font {id} unavailable");
        }
        self.fonts.insert(id.to_string(), handle.clone());
        handle
    }

    /// The processed bytes shipped under `id`.
    fn read(&mut self, id: &str) -> Option<Vec<u8>> {
        if self.manifest.is_none() {
            self.manifest = ContentManifest::load("assets/manifest.json").ok();
        }
        self.manifest
            .as_ref()
            .and_then(|m| m.processed_path("assets", id).ok())
            .and_then(|path| std::fs::read(path).ok())
    }

    /// Pixel size of a loaded image (for keeping aspect ratios).
    pub fn size(images: &Assets<Image>, handle: &Handle<Image>) -> Vec2 {
        images
            .get(handle)
            .map(|i| i.size().as_vec2())
            .unwrap_or(Vec2::ONE)
    }
}

/// Image IDs shared by several scenes (`assets/source/core/ui/icon/`).
pub mod ids {
    pub const ICON_CREDITS: &str = "core.ui.icon.credits";
    pub const ICON_VOID_CRYSTAL: &str = "core.ui.icon.void_crystal";
    pub const TITLE_KEY_ART: &str = "core.title.key_art";
    pub const RESULT_BACKDROP: &str = "core.result.backdrop";
    /// Display font for the game name (Russo One, SIL OFL 1.1, see
    /// `assets/source/core/fonts/OFL.txt`).
    pub const FONT_DISPLAY: &str = "core.fonts.russo_one";
}

/// A small inline icon for UI rows.
pub fn icon_node(image: Handle<Image>, size: f32) -> impl Bundle {
    (
        ImageNode::new(image),
        Node {
            width: px(size),
            height: px(size * 120.0 / 128.0),
            flex_shrink: 0.0,
            ..default()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every image ID the client asks for ships in the manifest.
    #[test]
    fn every_referenced_image_ships() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");
        let manifest = ContentManifest::load(format!("{root}/manifest.json")).unwrap();
        let mut wanted: Vec<String> = vec![
            ids::ICON_CREDITS.into(),
            ids::ICON_VOID_CRYSTAL.into(),
            ids::TITLE_KEY_ART.into(),
            ids::RESULT_BACKDROP.into(),
            ids::FONT_DISPLAY.into(),
        ];
        wanted.extend(crate::render::sprite_ids().iter().map(|s| s.to_string()));
        wanted.extend(crate::carrier::image_ids());
        wanted.extend(
            crate::workshop::UPGRADES
                .iter()
                .map(|u| u.icon_id().to_string()),
        );
        for id in wanted {
            let path = manifest.processed_path(root, &id).unwrap();
            assert!(path.exists(), "{id}: {}", path.display());
        }
    }
}
