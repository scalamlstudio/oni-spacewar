//! Shipped images and fonts by stable content ID. Each ID is resolved through
//! `assets/manifest.json` to its processed file, decoded once and cached.
//! Client only; the sim never sees these.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
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
                    ImageSampler::linear(),
                    RenderAssetUsages::default(),
                )
                .ok()
            })
            .map(|mut image| {
                add_mips(&mut image);
                images.add(image)
            });
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

/// Appends a full mip chain to an RGBA8 image. Painted sprites ship at a
/// higher resolution than they are drawn (the Pilot's 192 px canvas is ~90
/// px tall at Carrier zoom 1 and ~58 px at 0.65 on a 1x display), and a
/// linear sampler without mips skips texels past 2x minification, so
/// outlines shimmer while a character walks. Each level is a 2x2 box filter
/// on premultiplied alpha, so the black RGB of fully transparent pixels
/// doesn't darken the edges of smaller levels. Level 0 stays first in
/// `data`, so CPU readers of the full-size pixels are unaffected.
fn add_mips(image: &mut Image) {
    if !matches!(
        image.texture_descriptor.format,
        TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm
    ) {
        return;
    }
    let (mut w, mut h) = (image.width() as usize, image.height() as usize);
    let Some(data) = image.data.as_mut() else {
        return;
    };
    if data.len() != w * h * 4 {
        return;
    }
    let mut level = data.clone();
    let mut count = 1;
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                let mut sum = [0u32; 4];
                for (sx, sy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let i = (((2 * y + sy).min(h - 1)) * w + (2 * x + sx).min(w - 1)) * 4;
                    let a = level[i + 3] as u32;
                    for c in 0..3 {
                        sum[c] += level[i + c] as u32 * a;
                    }
                    sum[3] += a;
                }
                let o = (y * nw + x) * 4;
                for c in 0..3 {
                    next[o + c] = (sum[c] + sum[3] / 2).checked_div(sum[3]).unwrap_or(0) as u8;
                }
                next[o + 3] = ((sum[3] + 2) / 4) as u8;
            }
        }
        data.extend_from_slice(&next);
        (level, w, h) = (next, nw, nh);
        count += 1;
    }
    image.texture_descriptor.mip_level_count = count;
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
    use bevy::render::render_resource::{Extent3d, TextureDimension};

    #[test]
    fn mips_ignore_the_colour_of_transparent_pixels() {
        // 4x2: a white opaque pixel next to three transparent black ones.
        let mut data = vec![0u8; 4 * 2 * 4];
        data[..4].copy_from_slice(&[255, 255, 255, 255]);
        let mut image = Image::new(
            Extent3d {
                width: 4,
                height: 2,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data.clone(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        add_mips(&mut image);
        // 4x2 -> 2x1 -> 1x1.
        assert_eq!(image.texture_descriptor.mip_level_count, 3);
        let all = image.data.as_ref().unwrap();
        assert_eq!(all.len(), (8 + 2 + 1) * 4);
        assert_eq!(&all[..32], &data[..], "level 0 unchanged");
        // Quarter coverage, but still white, not grey.
        assert_eq!(&all[32..36], &[255, 255, 255, 64]);
        assert_eq!(&all[36..40], &[0, 0, 0, 0]);
        assert_eq!(&all[40..44], &[255, 255, 255, 32]);
    }

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
