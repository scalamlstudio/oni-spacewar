use std::collections::VecDeque;

use image::{imageops::FilterType, Rgba, RgbaImage};

pub const EXPRESSIONS: [&str; 9] = [
    "angry",
    "happy",
    "sleepy",
    "confused",
    "shocked",
    "excited",
    "sad",
    "surprised",
    "shy",
];

#[derive(Clone, Debug)]
pub struct Cutout {
    pub expression: String,
    pub image: RgbaImage,
}

#[derive(Clone, Copy, Debug)]
pub struct CutoutLayout {
    width: u32,
    height: u32,
    baseline: u32,
}

#[derive(Clone, Debug)]
pub struct CutoutSheet {
    pub layout: CutoutLayout,
    pub cutouts: Vec<Cutout>,
}

#[derive(Clone, Debug)]
struct Region {
    pixels: Vec<(u32, u32)>,
    bbox: BBox,
}

#[derive(Clone, Copy, Debug)]
struct BBox {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

#[derive(Clone, Debug)]
struct Pose {
    body: Region,
    attachments: Vec<Region>,
}

#[derive(Clone, Copy, Debug)]
struct AlphaKey {
    bg: [u8; 3],
    full_alpha_distance: u32,
    zero_alpha_distance: u32,
}

pub fn extract_expression_cutouts(sheet: &RgbaImage) -> Result<Vec<Cutout>, String> {
    extract_expression_cutout_sheet(sheet, &EXPRESSIONS, None).map(|sheet| sheet.cutouts)
}

pub fn extract_expression_cutout_sheet<N: AsRef<str>>(
    sheet: &RgbaImage,
    names: &[N],
    layout_override: Option<CutoutLayout>,
) -> Result<CutoutSheet, String> {
    let expected_count = names.len();
    if expected_count == 0 || names.len() != expected_count {
        return Err("cutout names must be non-empty".into());
    }

    let key = AlphaKey {
        bg: sample_background(sheet),
        zero_alpha_distance: 10,
        full_alpha_distance: 44,
    };
    let keyed = key_background(sheet, key);
    let mask = foreground_mask(&keyed);
    let mut regions = connected_regions(&mask, sheet.width(), sheet.height());
    regions.retain(|region| region.pixels.len() >= 12);

    let mut bodies = take_body_regions(regions.clone(), sheet.height());
    split_until_expected(&mut bodies, expected_count)?;
    if bodies.len() != expected_count {
        return Err(format!(
            "expected {expected_count} character regions, found {}",
            bodies.len()
        ));
    }
    bodies.sort_by_key(|region| (region.bbox.min_x + region.bbox.max_x, region.bbox.min_y));

    let mut poses: Vec<Pose> = bodies
        .into_iter()
        .map(|body| Pose {
            body,
            attachments: Vec::new(),
        })
        .collect();
    attach_small_regions(&mut poses, regions);

    let layout = layout_override.unwrap_or_else(|| compute_layout(&poses));
    let cutouts = poses
        .iter()
        .zip(names.iter().map(AsRef::as_ref))
        .map(|(pose, expression)| {
            let image = render_pose(&keyed, pose, layout)?;
            Ok(Cutout {
                expression: expression.to_string(),
                image,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(CutoutSheet { layout, cutouts })
}

pub fn make_contact_sheet(cutouts: &[(String, RgbaImage)], columns: u32) -> RgbaImage {
    let columns = columns.max(1);
    let cell_w = cutouts
        .iter()
        .map(|(_, image)| image.width())
        .max()
        .unwrap_or(1)
        + 16;
    let cell_h = cutouts
        .iter()
        .map(|(_, image)| image.height())
        .max()
        .unwrap_or(1)
        + 16;
    let rows = (cutouts.len() as u32).div_ceil(columns);
    let mut out = RgbaImage::new(cell_w * columns, cell_h * rows.max(1));
    for y in 0..out.height() {
        for x in 0..out.width() {
            let v = if ((x / 8) + (y / 8)) % 2 == 0 {
                216
            } else {
                168
            };
            out.put_pixel(x, y, Rgba([v, v, v, 255]));
        }
    }
    for (index, (_, image)) in cutouts.iter().enumerate() {
        let x0 = (index as u32 % columns) * cell_w + 8 + (cell_w - 16 - image.width()) / 2;
        let y0 = (index as u32 / columns) * cell_h + 8 + (cell_h - 16 - image.height()) / 2;
        overlay(&mut out, image, x0, y0);
    }
    out
}

/// Cut one sprite out of a flat-background image: key the border-connected
/// background to alpha, then crop to every foreground region (body plus
/// detached bits such as sparks or a trail) with `padding` px around it.
/// Images that already have a transparent background are only cropped.
pub fn extract_sprite(image: &RgbaImage, padding: u32) -> Result<RgbaImage, String> {
    let key = AlphaKey {
        bg: sample_background(image),
        zero_alpha_distance: 10,
        full_alpha_distance: 44,
    };
    let keyed = key_background(image, key);
    let mask = foreground_mask(&keyed);
    let regions = connected_regions(&mask, image.width(), image.height());
    let bbox = regions
        .iter()
        .filter(|region| region.pixels.len() >= 40)
        .map(|region| region.bbox)
        .reduce(BBox::union)
        .ok_or("no foreground found")?;
    // The padding is transparent canvas, even where the subject touches
    // the source image's edge (design/ART_GUIDELINES.md § Cropping).
    let cropped = image::imageops::crop_imm(
        &keyed,
        bbox.min_x,
        bbox.min_y,
        bbox.max_x - bbox.min_x + 1,
        bbox.max_y - bbox.min_y + 1,
    )
    .to_image();
    Ok(pad(&cropped, padding))
}

/// `image` on a canvas `margin` px larger on every side, the new border
/// transparent.
pub fn pad(image: &RgbaImage, margin: u32) -> RgbaImage {
    let mut out = RgbaImage::new(image.width() + margin * 2, image.height() + margin * 2);
    image::imageops::replace(&mut out, image, margin as i64, margin as i64);
    out
}

/// Scale `image` down (never up) so it fits inside `max_w` x `max_h`,
/// keeping its aspect ratio.
pub fn fit_within(image: &RgbaImage, max_w: u32, max_h: u32) -> RgbaImage {
    let scale = (max_w as f32 / image.width() as f32)
        .min(max_h as f32 / image.height() as f32)
        .min(1.0);
    if scale >= 1.0 {
        return image.clone();
    }
    let w = ((image.width() as f32 * scale).round() as u32).max(1);
    let h = ((image.height() as f32 * scale).round() as u32).max(1);
    image::imageops::resize(image, w, h, FilterType::Lanczos3)
}

fn sample_background(image: &RgbaImage) -> [u8; 3] {
    let mut totals = [0u64; 3];
    let mut count = 0u64;
    let w = image.width();
    let h = image.height();
    for y in 0..h {
        for x in 0..w {
            if x < 3 || y < 3 || x + 3 >= w || y + 3 >= h {
                let p = image.get_pixel(x, y).0;
                totals[0] += p[0] as u64;
                totals[1] += p[1] as u64;
                totals[2] += p[2] as u64;
                count += 1;
            }
        }
    }
    [
        (totals[0] / count) as u8,
        (totals[1] / count) as u8,
        (totals[2] / count) as u8,
    ]
}

/// Whether the art already ships with a transparent background (almost every
/// border pixel fully transparent), so colour keying must be skipped.
fn has_transparent_border(image: &RgbaImage) -> bool {
    let (w, h) = (image.width(), image.height());
    let mut total = 0u64;
    let mut clear = 0u64;
    for y in 0..h {
        for x in 0..w {
            if x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                total += 1;
                if image.get_pixel(x, y).0[3] == 0 {
                    clear += 1;
                }
            }
        }
    }
    clear * 10 >= total * 9
}

fn key_background(image: &RgbaImage, key: AlphaKey) -> RgbaImage {
    if has_transparent_border(image) {
        return image.clone();
    }
    let mut out = image.clone();
    let width = image.width();
    let height = image.height();
    let bg_like: Vec<bool> = image
        .pixels()
        .map(|pixel| color_distance(pixel.0, key.bg) < key.full_alpha_distance)
        .collect();
    let border_bg = border_connected(&bg_like, width, height);

    for (index, pixel) in out.pixels_mut().enumerate() {
        if !border_bg[index] {
            pixel.0[3] = 255;
            continue;
        }
        let d = color_distance(pixel.0, key.bg);
        let alpha = if d <= key.zero_alpha_distance {
            0
        } else if d >= key.full_alpha_distance {
            255
        } else {
            let n = d - key.zero_alpha_distance;
            let den = key.full_alpha_distance - key.zero_alpha_distance;
            ((n * 255) / den) as u8
        };
        pixel.0[3] = alpha;
        if alpha == 0 {
            pixel.0[0] = key.bg[0];
            pixel.0[1] = key.bg[1];
            pixel.0[2] = key.bg[2];
        }
    }
    out
}

fn border_connected(mask: &[bool], width: u32, height: u32) -> Vec<bool> {
    let mut seen = vec![false; mask.len()];
    let mut queue = VecDeque::new();
    for x in 0..width {
        push_if_masked(mask, &mut seen, &mut queue, x, 0, width);
        push_if_masked(mask, &mut seen, &mut queue, x, height - 1, width);
    }
    for y in 0..height {
        push_if_masked(mask, &mut seen, &mut queue, 0, y, width);
        push_if_masked(mask, &mut seen, &mut queue, width - 1, y, width);
    }
    while let Some((x, y)) = queue.pop_front() {
        if x > 0 {
            push_if_masked(mask, &mut seen, &mut queue, x - 1, y, width);
        }
        if x + 1 < width {
            push_if_masked(mask, &mut seen, &mut queue, x + 1, y, width);
        }
        if y > 0 {
            push_if_masked(mask, &mut seen, &mut queue, x, y - 1, width);
        }
        if y + 1 < height {
            push_if_masked(mask, &mut seen, &mut queue, x, y + 1, width);
        }
    }
    seen
}

fn push_if_masked(
    mask: &[bool],
    seen: &mut [bool],
    queue: &mut VecDeque<(u32, u32)>,
    x: u32,
    y: u32,
    width: u32,
) {
    let index = idx(x, y, width);
    if mask[index] && !seen[index] {
        seen[index] = true;
        queue.push_back((x, y));
    }
}

fn color_distance(pixel: [u8; 4], bg: [u8; 3]) -> u32 {
    let dr = pixel[0].abs_diff(bg[0]) as u32;
    let dg = pixel[1].abs_diff(bg[1]) as u32;
    let db = pixel[2].abs_diff(bg[2]) as u32;
    dr.max(dg).max(db)
}

fn foreground_mask(image: &RgbaImage) -> Vec<bool> {
    image.pixels().map(|pixel| pixel.0[3] > 24).collect()
}

fn connected_regions(mask: &[bool], width: u32, height: u32) -> Vec<Region> {
    let mut seen = vec![false; mask.len()];
    let mut regions = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let index = idx(x, y, width);
            if !mask[index] || seen[index] {
                continue;
            }
            seen[index] = true;
            let mut queue = VecDeque::from([(x, y)]);
            let mut pixels = Vec::new();
            let mut bbox = BBox {
                min_x: x,
                min_y: y,
                max_x: x,
                max_y: y,
            };
            while let Some((px, py)) = queue.pop_front() {
                pixels.push((px, py));
                bbox.min_x = bbox.min_x.min(px);
                bbox.min_y = bbox.min_y.min(py);
                bbox.max_x = bbox.max_x.max(px);
                bbox.max_y = bbox.max_y.max(py);
                let y0 = py.saturating_sub(1);
                let y1 = (py + 1).min(height - 1);
                let x0 = px.saturating_sub(1);
                let x1 = (px + 1).min(width - 1);
                for ny in y0..=y1 {
                    for nx in x0..=x1 {
                        let next = idx(nx, ny, width);
                        if mask[next] && !seen[next] {
                            seen[next] = true;
                            queue.push_back((nx, ny));
                        }
                    }
                }
            }
            regions.push(Region { pixels, bbox });
        }
    }
    regions
}

fn take_body_regions(regions: Vec<Region>, sheet_height: u32) -> Vec<Region> {
    let min_height = (sheet_height / 7).max(24);
    let min_area = ((sheet_height * sheet_height) / 300).max(500) as usize;
    regions
        .into_iter()
        .filter(|region| region.bbox.height() >= min_height && region.pixels.len() >= min_area)
        .collect()
}

fn split_until_expected(regions: &mut Vec<Region>, expected: usize) -> Result<(), String> {
    while regions.len() < expected {
        let Some((index, _)) = regions
            .iter()
            .enumerate()
            .filter(|(_, region)| region.bbox.width() > 10)
            .max_by_key(|(_, region)| region.bbox.width() * region.bbox.height())
        else {
            break;
        };
        let region = regions.remove(index);
        let Some((left, right)) = split_region_at_low_density_seam(&region) else {
            regions.push(region);
            break;
        };
        regions.push(left);
        regions.push(right);
    }
    if regions.len() > expected {
        return Err(format!(
            "expected {expected} character regions, found {} before attachment",
            regions.len()
        ));
    }
    Ok(())
}

fn split_region_at_low_density_seam(region: &Region) -> Option<(Region, Region)> {
    let width = region.bbox.width();
    if width < 20 {
        return None;
    }
    let mut counts = vec![0u32; width as usize];
    for &(x, _) in &region.pixels {
        counts[(x - region.bbox.min_x) as usize] += 1;
    }
    let start = width / 5;
    let end = width - start;
    let seam_offset = (start..end).min_by_key(|&offset| counts[offset as usize])?;
    let seam_x = region.bbox.min_x + seam_offset;
    let mut left = Vec::new();
    let mut right = Vec::new();
    for &pixel in &region.pixels {
        if pixel.0 <= seam_x {
            left.push(pixel);
        } else {
            right.push(pixel);
        }
    }
    if left.is_empty() || right.is_empty() {
        return None;
    }
    Some((region_from_pixels(left), region_from_pixels(right)))
}

fn attach_small_regions(poses: &mut [Pose], regions: Vec<Region>) {
    let body_boxes: Vec<BBox> = poses.iter().map(|pose| pose.body.bbox).collect();
    let min_body_area = poses
        .iter()
        .map(|pose| pose.body.pixels.len())
        .min()
        .unwrap_or(usize::MAX);
    let min_body_height = poses
        .iter()
        .map(|pose| pose.body.bbox.height())
        .min()
        .unwrap_or(u32::MAX);
    for region in regions {
        if body_boxes.contains(&region.bbox) {
            continue;
        }
        let region_is_body_sized =
            region.pixels.len() > min_body_area / 4 || region.bbox.height() > min_body_height / 2;
        if region_is_body_sized {
            continue;
        }
        let center = region.bbox.center();
        let Some((pose_index, distance)) = body_boxes
            .iter()
            .enumerate()
            .map(|(index, bbox)| {
                let body_center = bbox.center();
                let dx = (center.0 - body_center.0).round() as i64;
                let dy = (center.1 - body_center.1).round() as i64;
                (index, dx * dx + dy * dy)
            })
            .min_by_key(|(_, distance)| *distance)
        else {
            continue;
        };
        let body = body_boxes[pose_index];
        let below_body = region.bbox.min_y > body.max_y + 8;
        let far_away = distance > 260 * 260;
        if !below_body && !far_away {
            poses[pose_index].attachments.push(region);
        }
    }
}

fn compute_layout(poses: &[Pose]) -> CutoutLayout {
    let mut left_span = 0i32;
    let mut right_span = 0i32;
    let mut above = 0i32;
    let mut below = 0i32;
    for pose in poses {
        let bbox = pose_bbox(pose);
        let body_center = pose.body.bbox.center().0.round() as i32;
        let body_base = pose.body.bbox.max_y as i32;
        left_span = left_span.max(body_center - bbox.min_x as i32);
        right_span = right_span.max(bbox.max_x as i32 - body_center);
        above = above.max(body_base - bbox.min_y as i32);
        below = below.max(bbox.max_y as i32 - body_base);
    }
    let padding = 6;
    CutoutLayout {
        width: (left_span + right_span + padding * 2 + 1).max(1) as u32,
        height: (above + below + padding * 2 + 1).max(1) as u32,
        baseline: (above + padding) as u32,
    }
}

fn render_pose(source: &RgbaImage, pose: &Pose, layout: CutoutLayout) -> Result<RgbaImage, String> {
    if !pose_fits(pose, layout) {
        return render_pose_scaled_to_layout(source, pose, layout);
    }
    render_pose_unscaled(source, pose, layout)
}

fn render_pose_unscaled(
    source: &RgbaImage,
    pose: &Pose,
    layout: CutoutLayout,
) -> Result<RgbaImage, String> {
    let mut out = RgbaImage::new(layout.width, layout.height);
    let source_center = pose.body.bbox.center().0.round() as i32;
    let target_center = (layout.width / 2) as i32;
    let dy = layout.baseline as i32 - pose.body.bbox.max_y as i32;
    let dx = target_center - source_center;
    ensure_region_fits(&pose.body, dx, dy, layout)?;
    blit_region(&mut out, source, &pose.body, dx, dy);
    for attachment in &pose.attachments {
        ensure_region_fits(attachment, dx, dy, layout)?;
        blit_region(&mut out, source, attachment, dx, dy);
    }
    Ok(out)
}

fn pose_fits(pose: &Pose, layout: CutoutLayout) -> bool {
    let source_center = pose.body.bbox.center().0.round() as i32;
    let target_center = (layout.width / 2) as i32;
    let dy = layout.baseline as i32 - pose.body.bbox.max_y as i32;
    let dx = target_center - source_center;
    std::iter::once(&pose.body)
        .chain(pose.attachments.iter())
        .all(|region| region_fits(region, dx, dy, layout))
}

fn render_pose_scaled_to_layout(
    source: &RgbaImage,
    pose: &Pose,
    layout: CutoutLayout,
) -> Result<RgbaImage, String> {
    let source_layout = compute_layout(std::slice::from_ref(pose));
    let unscaled = render_pose_unscaled(source, pose, source_layout)?;
    let above = source_layout.baseline as f32;
    let below = (source_layout.height - source_layout.baseline) as f32;
    let mut scale = (layout.width as f32 / source_layout.width as f32).min(1.0);
    if above > 0.0 {
        scale = scale.min(layout.baseline as f32 / above);
    }
    if below > 0.0 {
        scale = scale.min((layout.height - layout.baseline) as f32 / below);
    }
    scale *= 0.98;
    if scale <= 0.0 {
        return Err(format!(
            "pose cannot be scaled into target layout {}x{} baseline {}",
            layout.width, layout.height, layout.baseline
        ));
    }
    let scaled_w = ((source_layout.width as f32 * scale).floor() as u32).max(1);
    let scaled_h = ((source_layout.height as f32 * scale).floor() as u32).max(1);
    let scaled_baseline = ((source_layout.baseline as f32 * scale).round() as u32).min(scaled_h);
    let scaled = image::imageops::resize(&unscaled, scaled_w, scaled_h, FilterType::Lanczos3);
    let mut out = RgbaImage::new(layout.width, layout.height);
    let x0 = (layout.width - scaled_w) / 2;
    let y0 = layout
        .baseline
        .checked_sub(scaled_baseline)
        .ok_or_else(|| "scaled pose baseline does not fit target layout".to_string())?;
    if y0 + scaled_h > layout.height {
        return Err(format!(
            "scaled pose does not fit target layout {}x{} baseline {}",
            layout.width, layout.height, layout.baseline
        ));
    }
    overlay(&mut out, &scaled, x0, y0);
    Ok(out)
}

fn ensure_region_fits(
    region: &Region,
    dx: i32,
    dy: i32,
    layout: CutoutLayout,
) -> Result<(), String> {
    if region_fits(region, dx, dy, layout) {
        return Ok(());
    }
    Err(format!(
        "pose does not fit target layout {}x{} baseline {}",
        layout.width, layout.height, layout.baseline
    ))
}

fn region_fits(region: &Region, dx: i32, dy: i32, layout: CutoutLayout) -> bool {
    let min_x = region.bbox.min_x as i32 + dx;
    let max_x = region.bbox.max_x as i32 + dx;
    let min_y = region.bbox.min_y as i32 + dy;
    let max_y = region.bbox.max_y as i32 + dy;
    min_x >= 0 && min_y >= 0 && max_x < layout.width as i32 && max_y < layout.height as i32
}

fn blit_region(target: &mut RgbaImage, source: &RgbaImage, region: &Region, dx: i32, dy: i32) {
    for &(sx, sy) in &region.pixels {
        let tx = sx as i32 + dx;
        let ty = sy as i32 + dy;
        if tx >= 0 && ty >= 0 && tx < target.width() as i32 && ty < target.height() as i32 {
            target.put_pixel(tx as u32, ty as u32, *source.get_pixel(sx, sy));
        }
    }
}

fn overlay(target: &mut RgbaImage, source: &RgbaImage, x0: u32, y0: u32) {
    for y in 0..source.height() {
        for x in 0..source.width() {
            let src = source.get_pixel(x, y).0;
            if src[3] == 0 {
                continue;
            }
            target.put_pixel(x0 + x, y0 + y, Rgba(src));
        }
    }
}

fn pose_bbox(pose: &Pose) -> BBox {
    let mut bbox = pose.body.bbox;
    for attachment in &pose.attachments {
        bbox = bbox.union(attachment.bbox);
    }
    bbox
}

fn region_from_pixels(pixels: Vec<(u32, u32)>) -> Region {
    let mut bbox = BBox {
        min_x: u32::MAX,
        min_y: u32::MAX,
        max_x: 0,
        max_y: 0,
    };
    for &(x, y) in &pixels {
        bbox.min_x = bbox.min_x.min(x);
        bbox.min_y = bbox.min_y.min(y);
        bbox.max_x = bbox.max_x.max(x);
        bbox.max_y = bbox.max_y.max(y);
    }
    Region { pixels, bbox }
}

fn idx(x: u32, y: u32, width: u32) -> usize {
    (y * width + x) as usize
}

impl BBox {
    fn width(self) -> u32 {
        self.max_x - self.min_x + 1
    }

    fn height(self) -> u32 {
        self.max_y - self.min_y + 1
    }

    fn center(self) -> (f32, f32) {
        (
            (self.min_x + self.max_x) as f32 * 0.5,
            (self.min_y + self.max_y) as f32 * 0.5,
        )
    }

    fn union(self, other: BBox) -> BBox {
        BBox {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }
}

impl PartialEq for BBox {
    fn eq(&self, other: &Self) -> bool {
        self.min_x == other.min_x
            && self.min_y == other.min_y
            && self.max_x == other.max_x
            && self.max_y == other.max_y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_sheet_extracts_alpha_marks_and_drops_labels() {
        let bg = Rgba([244, 241, 232, 255]);
        let mut sheet = RgbaImage::from_pixel(180, 96, bg);
        rect(&mut sheet, 14, 24, 42, 62, [216, 48, 48, 255]);
        rect(&mut sheet, 22, 14, 28, 20, [216, 48, 48, 255]);
        rect(&mut sheet, 72, 22, 104, 62, [48, 160, 72, 255]);
        rect(&mut sheet, 94, 12, 100, 18, [48, 160, 72, 255]);
        rect(&mut sheet, 132, 24, 164, 62, [64, 96, 224, 255]);
        rect(&mut sheet, 52, 28, 55, 62, [180, 64, 64, 255]);
        rect(&mut sheet, 92, 68, 98, 74, [20, 20, 20, 255]);

        let cutouts = extract_expression_cutout_sheet(&sheet, &["a", "b", "c"], None)
            .unwrap()
            .cutouts;
        assert_eq!(cutouts.len(), 3);
        let size = (cutouts[0].image.width(), cutouts[0].image.height());
        assert!(cutouts
            .iter()
            .all(|cutout| { cutout.image.width() == size.0 && cutout.image.height() == size.1 }));
        assert!(cutouts
            .iter()
            .all(|cutout| cutout.image.pixels().any(|pixel| pixel.0[3] == 0)));
        assert!(!has_color(&cutouts[0].image, [64, 96, 224]));
        assert!(has_color(&cutouts[1].image, [48, 160, 72]));
        assert!(has_color(&cutouts[1].image, [20, 20, 20]));
        assert!(!has_color(&cutouts[1].image, [244, 241, 232]));
    }

    #[test]
    fn two_pose_sheet_uses_custom_expression_names() {
        let bg = Rgba([244, 241, 232, 255]);
        let mut sheet = RgbaImage::from_pixel(128, 96, bg);
        rect(&mut sheet, 16, 22, 46, 64, [216, 48, 48, 255]);
        rect(&mut sheet, 76, 24, 108, 64, [48, 160, 72, 255]);

        let cutouts = extract_expression_cutout_sheet(&sheet, &["normal", "serious"], None)
            .unwrap()
            .cutouts;

        assert_eq!(cutouts.len(), 2);
        assert_eq!(cutouts[0].expression, "normal");
        assert_eq!(cutouts[1].expression, "serious");
        assert!(has_color(&cutouts[0].image, [216, 48, 48]));
        assert!(has_color(&cutouts[1].image, [48, 160, 72]));
    }

    #[test]
    fn single_sprite_is_keyed_and_cropped_with_its_detached_bits() {
        let bg = Rgba([16, 20, 32, 255]);
        let mut image = RgbaImage::from_pixel(120, 80, bg);
        rect(&mut image, 40, 20, 70, 50, [230, 140, 40, 255]);
        rect(&mut image, 80, 30, 86, 36, [200, 60, 220, 255]);
        let sprite = extract_sprite(&image, 2).unwrap();
        assert_eq!((sprite.width(), sprite.height()), (51, 35));
        assert_eq!(sprite.get_pixel(0, 0).0[3], 0);
        assert!(has_color(&sprite, [200, 60, 220]));
    }

    #[test]
    fn padding_is_added_where_the_subject_touches_the_source_edge() {
        let mut image = RgbaImage::from_pixel(60, 40, Rgba([16, 20, 32, 255]));
        rect(&mut image, 0, 16, 20, 19, [230, 140, 40, 255]);
        let sprite = extract_sprite(&image, 3).unwrap();
        assert_eq!((sprite.width(), sprite.height()), (27, 10));
        assert_eq!(sprite.get_pixel(0, 5).0[3], 0);
        assert_eq!(sprite.get_pixel(3, 5).0[3], 255);
    }

    #[test]
    fn transparent_art_keeps_its_alpha() {
        let mut image = RgbaImage::new(60, 40);
        rect(&mut image, 10, 10, 30, 30, [255, 255, 255, 255]);
        rect(&mut image, 14, 14, 16, 16, [0, 0, 0, 0]);
        let sprite = extract_sprite(&image, 0).unwrap();
        assert_eq!((sprite.width(), sprite.height()), (21, 21));
        assert_eq!(sprite.get_pixel(5, 5).0[3], 0);
        assert_eq!(sprite.get_pixel(0, 0).0, [255, 255, 255, 255]);
    }

    #[test]
    fn fit_within_only_shrinks() {
        let image = RgbaImage::new(400, 200);
        let small = fit_within(&image, 100, 100);
        assert_eq!((small.width(), small.height()), (100, 50));
        assert_eq!(fit_within(&small, 500, 500).dimensions(), (100, 50));
    }

    fn rect(image: &mut RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32, color: [u8; 4]) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                image.put_pixel(x, y, Rgba(color));
            }
        }
    }

    fn has_color(image: &RgbaImage, color: [u8; 3]) -> bool {
        image.pixels().any(|pixel| {
            pixel.0[3] > 0
                && pixel.0[0] == color[0]
                && pixel.0[1] == color[1]
                && pixel.0[2] == color[2]
        })
    }
}
