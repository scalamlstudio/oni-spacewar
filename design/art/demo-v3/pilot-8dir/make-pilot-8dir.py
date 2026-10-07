from pathlib import Path

from PIL import Image, ImageDraw, ImageFont, ImageOps


ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
ASSET_ROOT = ROOT / "assets/source/core/carrier/pilot"

W = 165
H = 192
BASELINE = 181
TARGET_HEIGHT = 150

FRAMES = ["idle", "walk_1", "walk_2", "walk_3", "walk_4"]
ASSET_DIRS = ["s", "se", "e", "ne", "n"]
SHEET_DIRS = ["s", "se", "e", "ne", "n", "sw", "w", "nw"]

GENERATED_SHEET = HERE / "oni-pilot-8dir-generated-source-v2.png"
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


def add_back_ear_tips(cell: Image.Image) -> Image.Image:
    cell = cell.convert("RGBA")
    box = bbox_from_alpha(cell)
    if box[1] > 0:
        return cell

    top_pad = 30
    padded = Image.new("RGBA", (cell.width, cell.height + top_pad), (0, 0, 0, 0))
    draw = ImageDraw.Draw(padded, "RGBA")

    x1, _, x2, _ = box
    span = x2 - x1
    head_left = x1 + round(span * 0.08)
    head_right = x2 - round(span * 0.02)
    base_y = top_pad + 7
    tip_y = 5
    outline = (73, 50, 38, 230)
    dark = (150, 75, 31, 255)
    orange = (238, 126, 37, 255)
    light = (255, 171, 67, 245)

    ears = [
        ((head_left, base_y + 3), (head_left + 34, tip_y), (head_left + 72, base_y + 2)),
        ((head_right - 76, base_y + 2), (head_right - 38, tip_y), (head_right, base_y + 3)),
    ]
    for points in ears:
        draw.polygon(points, fill=orange, outline=outline)
        draw.line(points + (points[0],), fill=outline, width=4, joint="curve")
        draw.line((points[0], points[1], points[2]), fill=light, width=2, joint="curve")
        draw.line((points[0][0] + 11, base_y, points[1][0], tip_y + 12), fill=dark, width=2)
        draw.line((points[2][0] - 13, base_y, points[1][0], tip_y + 12), fill=dark, width=2)

    padded.alpha_composite(cell, (0, top_pad))
    return padded


def draw_scaled(base: Image.Image, painter) -> None:
    scale = 4
    layer = Image.new("RGBA", (base.width * scale, base.height * scale), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer, "RGBA")

    def sx(points):
        if isinstance(points[0], tuple):
            return tuple((x * scale, y * scale) for x, y in points)
        return tuple(v * scale for v in points)

    painter(draw, sx, scale)
    layer = layer.resize(base.size, Image.Resampling.LANCZOS)
    base.alpha_composite(layer)


def make_ne_three_quarter(frame: Image.Image) -> Image.Image:
    frame = frame.convert("RGBA")

    body = frame.copy()
    backpack = body.crop((52, 101, 108, 151))

    out = Image.new("RGBA", frame.size, (0, 0, 0, 0))
    out.alpha_composite(body)
    out.alpha_composite(backpack, (47, 101))

    def painter(draw, sx, scale):
        outline = (71, 47, 34, 210)
        orange = (240, 125, 36, 245)
        light = (255, 182, 78, 220)
        cream = (255, 231, 194, 230)
        pink = (255, 121, 118, 190)
        strap = (31, 39, 45, 235)
        lens_dark = (18, 82, 88, 235)
        lens = (64, 214, 203, 230)
        shine = (199, 255, 239, 190)

        draw.ellipse(sx((114, 69, 137, 96)), fill=outline)
        draw.ellipse(sx((112, 68, 134, 95)), fill=orange)
        draw.pieslice(sx((120, 78, 141, 101)), 90, 260, fill=cream)
        draw.ellipse(sx((124, 90, 136, 100)), fill=pink)
        draw.arc(sx((111, 65, 136, 96)), 285, 65, fill=light, width=2 * scale)

        draw.line(sx(((91, 66), (111, 62), (137, 65))), fill=strap, width=5 * scale)
        draw.rounded_rectangle(sx((116, 52, 139, 68)), radius=7 * scale, fill=(23, 29, 35, 245))
        draw.rounded_rectangle(sx((120, 55, 136, 66)), radius=5 * scale, fill=lens_dark)
        draw.ellipse(sx((121, 56, 134, 65)), fill=lens)
        draw.ellipse(sx((126, 56, 136, 60)), fill=shine)

        draw.arc(sx((36, 105, 85, 166)), 150, 250, fill=outline, width=9 * scale)
        draw.arc(sx((38, 106, 83, 164)), 150, 250, fill=orange, width=7 * scale)
        draw.arc(sx((41, 109, 79, 158)), 150, 235, fill=light, width=2 * scale)

    draw_scaled(out, painter)
    return out


def borrow_front_ears(frame: Image.Image, front: Image.Image, turn: str) -> Image.Image:
    frame = frame.convert("RGBA")
    front = front.convert("RGBA")

    pixels = frame.load()
    for y in range(22, 69):
        for x in range(30, 146):
            r, g, b, a = pixels[x, y]
            if a:
                pixels[x, y] = (r, g, b, 0)

    def head_painter(draw, sx, scale):
        outline = (76, 49, 33, 210)
        orange = (237, 119, 33, 255)
        mid = (246, 143, 45, 255)
        light = (255, 177, 69, 235)
        head = (44, 47, 123, 90) if turn == "n" else (47, 47, 128, 90)
        draw.ellipse(sx(head), fill=outline)
        draw.ellipse(sx((head[0] + 3, head[1] + 2, head[2] - 3, head[3] + 5)), fill=orange)
        draw.pieslice(sx((head[0] + 6, head[1] + 3, head[2] - 5, head[3] + 6)), 188, 352, fill=mid)
        draw.ellipse(sx((head[0] + 7, head[1] + 16, head[2] - 7, head[3] + 9)), fill=(241, 132, 39, 238))
        draw.arc(sx((head[0] + 5, head[1] + 4, head[2] - 5, head[3] + 7)), 190, 350, fill=light, width=2 * scale)
        stripe_xs = [61, 72, 84, 96, 108] if turn == "n" else [65, 78, 92, 106, 118]
        for i, x in enumerate(stripe_xs):
            y0 = 51 + (i % 2) * 2
            draw.polygon(sx(((x, y0), (x + 4, y0 + 2), (x + 1, y0 + 10), (x - 3, y0 + 3))), fill=(215, 93, 28, 70))

    draw_scaled(frame, head_painter)

    ear_layer = Image.new("RGBA", front.size, (0, 0, 0, 0))
    ear_pixels = ear_layer.load()
    src_pixels = front.load()
    for y in range(26, 76):
        for x in list(range(33, 76)) + list(range(94, 138)):
            r, g, b, a = src_pixels[x, y]
            if a > 20 and (r > 120 and g > 45 and b < 190):
                ear_pixels[x, y] = (r, g, b, a)

    if turn == "ne":
        ear_layer = ear_layer.transform(
            ear_layer.size,
            Image.Transform.AFFINE,
            (1.0, 0.0, 4.0, 0.0, 1.0, 0.0),
            resample=Image.Resampling.BICUBIC,
        )
    frame.alpha_composite(ear_layer)
    return frame


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
            if direction in {"ne", "n"}:
                cell = add_back_ear_tips(cell)
            fitted = fit_to_frame(cell)
            if direction == "n":
                fitted = borrow_front_ears(fitted, result[("s", frame)], "n")
            elif direction == "ne":
                fitted = make_ne_three_quarter(fitted)
                fitted = borrow_front_ears(fitted, result[("s", frame)], "ne")
            result[(direction, frame)] = fitted
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
    sheet = checkerboard((W * len(FRAMES) + margin * 2, (H + label_h) * len(SHEET_DIRS) + margin * 2))
    draw = ImageDraw.Draw(sheet)
    font = ImageFont.load_default()
    for row, direction in enumerate(SHEET_DIRS):
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
    for direction in ASSET_DIRS:
        (ASSET_ROOT / direction).mkdir(parents=True, exist_ok=True)
        (HERE / "frames" / direction).mkdir(parents=True, exist_ok=True)
        for frame in FRAMES:
            image = east[frame] if direction == "e" else generated[(direction, frame)]
            frames[(direction, frame)] = image
            image.save(ASSET_ROOT / direction / f"{frame}.png")
            image.save(HERE / "frames" / direction / f"{frame}.png")
    for frame in FRAMES:
        frames[("w", frame)] = ImageOps.mirror(frames[("e", frame)])
        frames[("sw", frame)] = ImageOps.mirror(frames[("se", frame)])
        frames[("nw", frame)] = ImageOps.mirror(frames[("ne", frame)])
    make_contact_sheet(frames)
    make_crew_compare(frames)


if __name__ == "__main__":
    main()
