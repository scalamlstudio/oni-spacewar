from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
ASSET_ROOT = ROOT / "assets/source/core/carrier/pilot"

W = 165
H = 192
BASELINE = 181
TARGET_HEIGHT = 150

FRAMES = ["idle", "walk_1", "walk_2", "walk_3", "walk_4"]
DIRS = ["s", "se", "e", "ne", "n"]

GENERATED_SHEET = HERE / "oni-pilot-8dir-generated-source-v1.png"
EAST_SHEET = ROOT / "design/art/demo/oni-pilot-walk-sheet-v1.png"


def bbox_from_alpha(image: Image.Image, threshold: int = 12) -> tuple[int, int, int, int]:
    alpha = image.getchannel("A")
    mask = alpha.point(lambda a: 255 if a > threshold else 0)
    box = mask.getbbox()
    if not box:
        raise ValueError("no visible pixels found")
    return box


def keep_largest_alpha_component(image: Image.Image, threshold: int = 24) -> Image.Image:
    image = image.convert("RGBA")
    alpha = image.getchannel("A")
    pixels = alpha.load()
    width, height = image.size
    seen: set[tuple[int, int]] = set()
    best: list[tuple[int, int]] = []

    for y in range(height):
        for x in range(width):
            if (x, y) in seen or pixels[x, y] <= threshold:
                continue
            stack = [(x, y)]
            seen.add((x, y))
            component = []
            while stack:
                px, py = stack.pop()
                component.append((px, py))
                for nx, ny in ((px + 1, py), (px - 1, py), (px, py + 1), (px, py - 1)):
                    if nx < 0 or ny < 0 or nx >= width or ny >= height or (nx, ny) in seen:
                        continue
                    if pixels[nx, ny] > threshold:
                        seen.add((nx, ny))
                        stack.append((nx, ny))
            if len(component) > len(best):
                best = component

    if not best:
        return image

    keep = set(best)
    out = image.copy()
    out_pixels = out.load()
    for y in range(height):
        for x in range(width):
            if (x, y) not in keep:
                r, g, b, _ = out_pixels[x, y]
                out_pixels[x, y] = (r, g, b, 0)
    return out


def remove_flat_background(image: Image.Image, color: tuple[int, int, int], tolerance: int) -> Image.Image:
    image = image.convert("RGBA")
    pixels = image.load()
    for y in range(image.height):
        for x in range(image.width):
            r, g, b, a = pixels[x, y]
            dist = max(abs(r - color[0]), abs(g - color[1]), abs(b - color[2]))
            if dist <= tolerance:
                pixels[x, y] = (r, g, b, 0)
    return image


def fit_to_frame(sprite: Image.Image, target_height: int = TARGET_HEIGHT) -> Image.Image:
    box = bbox_from_alpha(sprite)
    sprite = sprite.crop(box)
    scale = target_height / sprite.height
    new_size = (round(sprite.width * scale), round(sprite.height * scale))
    sprite = sprite.resize(new_size, Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    x = (W - sprite.width) // 2
    y = BASELINE - sprite.height
    canvas.alpha_composite(sprite, (x, y))
    return canvas


def cut_generated_sheet() -> dict[tuple[str, str], Image.Image]:
    source = Image.open(GENERATED_SHEET).convert("RGBA")
    cell_w = source.width / 5
    cell_h = source.height / 4
    result = {}
    row_dirs = ["s", "se", "ne", "n"]
    for row, direction in enumerate(row_dirs):
        for col, frame in enumerate(FRAMES):
            cell = source.crop(
                (
                    round(col * cell_w),
                    round(row * cell_h),
                    round((col + 1) * cell_w),
                    round((row + 1) * cell_h),
                )
            )
            # The generated image is already transparent; this only removes tiny preview artifacts.
            alpha = cell.getchannel("A")
            cell.putalpha(alpha.point(lambda a: 0 if a < 24 else a))
            cell = keep_largest_alpha_component(cell)
            result[(direction, frame)] = fit_to_frame(cell)
    return result


def cut_east_sheet() -> dict[str, Image.Image]:
    source = Image.open(EAST_SHEET).convert("RGBA")
    bg = source.getpixel((0, 0))[:3]
    source = remove_flat_background(source, bg, 14)
    cell_w = source.width / 5
    result = {}
    for col, frame in enumerate(FRAMES):
        cell = source.crop((round(col * cell_w), 0, round((col + 1) * cell_w), source.height))
        result[frame] = fit_to_frame(cell, target_height=151)
    return result


def checkerboard(size: tuple[int, int], cell: int = 16) -> Image.Image:
    image = Image.new("RGBA", size, (21, 35, 46, 255))
    draw = ImageDraw.Draw(image)
    for y in range(0, size[1], cell):
        for x in range(0, size[0], cell):
            if (x // cell + y // cell) % 2:
                draw.rectangle((x, y, x + cell - 1, y + cell - 1), fill=(27, 48, 61, 255))
    return image


def make_contact_sheet(frames: dict[tuple[str, str], Image.Image]) -> None:
    margin = 18
    label_h = 22
    sheet = checkerboard((W * len(FRAMES) + margin * 2, (H + label_h) * len(DIRS) + margin * 2))
    draw = ImageDraw.Draw(sheet)
    font = ImageFont.load_default()
    for row, direction in enumerate(DIRS):
        y = margin + row * (H + label_h)
        draw.text((6, y + 6), direction.upper(), fill=(226, 238, 232, 255), font=font)
        for col, frame in enumerate(FRAMES):
            x = margin + col * W
            sheet.alpha_composite(frames[(direction, frame)], (x, y + label_h))
            draw.text((x + 44, y + 4), frame, fill=(226, 238, 232, 255), font=font)
    sheet.save(HERE / "carrier-pilot-8dir-contact-sheet.png")


def portrait_thumb(path: Path, height: int = 192) -> Image.Image:
    image = Image.open(path).convert("RGBA")
    box = bbox_from_alpha(image)
    image = image.crop(box)
    scale = height / image.height
    image = image.resize((round(image.width * scale), height), Image.Resampling.LANCZOS)
    return image


def make_crew_compare(frames: dict[tuple[str, str], Image.Image]) -> None:
    items = [
        ("S idle", frames[("s", "idle")]),
        ("E idle", frames[("e", "idle")]),
        ("Gunner", portrait_thumb(ROOT / "assets/source/core/portraits/gunner/normal.png")),
        ("Researcher", portrait_thumb(ROOT / "assets/source/core/portraits/researcher/normal.png")),
        ("Pilot portrait", portrait_thumb(ROOT / "assets/source/core/portraits/pilot/normal.png")),
    ]
    width = 5 * 188 + 28
    height = 250
    sheet = Image.new("RGBA", (width, height), (246, 241, 229, 255))
    draw = ImageDraw.Draw(sheet)
    font = ImageFont.load_default()
    for i, (label, image) in enumerate(items):
        x = 14 + i * 188
        y = 38
        if image.width > 170:
            scale = 170 / image.width
            image = image.resize((170, round(image.height * scale)), Image.Resampling.LANCZOS)
        sheet.alpha_composite(image, (x + (170 - image.width) // 2, y + (192 - image.height)))
        draw.text((x + 8, 14), label, fill=(24, 39, 51, 255), font=font)
    sheet.save(HERE / "pilot-crew-side-by-side-v1.png")


def main() -> None:
    generated = cut_generated_sheet()
    east = cut_east_sheet()
    frames = {}
    for direction in DIRS:
        (ASSET_ROOT / direction).mkdir(parents=True, exist_ok=True)
        (HERE / "frames" / direction).mkdir(parents=True, exist_ok=True)
        for frame in FRAMES:
            image = east[frame] if direction == "e" else generated[(direction, frame)]
            frames[(direction, frame)] = image
            image.save(ASSET_ROOT / direction / f"{frame}.png")
            image.save(HERE / "frames" / direction / f"{frame}.png")
    make_contact_sheet(frames)
    make_crew_compare(frames)


if __name__ == "__main__":
    main()
