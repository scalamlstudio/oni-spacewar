"""TAKOAI-89: deterministic wall assembly, Python 3 + Pillow.

Run from any directory. All outputs stay alongside this script. Originals
are immutable; v2 PNGs and a stable-ID import catalogue are regenerated.
No painted-room extraction, sprite-sheet slicing, or background keying.
"""
from pathlib import Path
import json
import random
from PIL import Image, ImageOps, ImageDraw

ROOT = Path(__file__).resolve().parent
R = Image.Resampling.LANCZOS


def read(name):
    return Image.open(ROOT / name).convert('RGBA')


def save(im, name):
    version = 3 if name in ['doorway-ns', 'doorway-ew'] else 2
    im.save(ROOT / (name + f'-v{version}.png'))
    return im


def periodic(im, size):
    # Blend opposing material edges to their common average. Preserve the
    # original panel scale rather than reflecting four miniature panels.
    w, h = size
    out = im.resize(size, R)
    for axis in [0, 1]:
        length, other = (w, h) if axis == 0 else (h, w)
        for t in range(other):
            coords = lambda k: (k, t) if axis == 0 else (t, k)
            a, b = out.getpixel(coords(0)), out.getpixel(coords(length-1))
            avg = tuple((a[c]+b[c])//2 for c in range(4))
            for k in range(8):
                blend = k / 8
                for pos in [k, length-1-k]:
                    old = out.getpixel(coords(pos))
                    out.putpixel(coords(pos), tuple(round(avg[c]*(1-blend)+old[c]*blend)
                                                   for c in range(4)))
    return out


def whole(name, size):
    im = read(name)
    box = im.getchannel('A').getbbox()
    assert box, name
    return ImageOps.contain(im.crop(box), size, R)


def build():
    cap_source = read('wall-cap-source-v2.png')
    cap = periodic(cap_source.crop((0, 40, cap_source.width, cap_source.height-40)), (128, 128))
    # Sample one panel of the deliberately generated material swatch.
    face = periodic(read('wall-face-source-v2.png').crop((135, 235, 665, 800)), (128, 96))
    inner = whole('wall-inner-corner-source-v2.png', (24, 24))
    outer = whole('wall-outer-corner-source-v2.png', (24, 24))
    jamb = whole('doorway-jamb-source-v2.png', (12, 88))
    save(face, 'wall-face')
    save(cap.crop((0, 0, 128, 32)), 'wall-cap')
    save(inner, 'wall-inner-corner')
    save(outer, 'wall-outer-corner')
    save(jamb, 'doorway-jamb')
    walls = {}
    for mask in range(16):
        bits = f'{mask:04b}'
        n, e, s, w = map(int, bits)
        im = cap.copy()
        d = ImageDraw.Draw(im)
        # Only exterior edges receive trim: no gold crossbars at tile
        # boundaries along a continuous vertical wall or at a T-junction.
        if not n:
            d.line((0, 1, 127, 1), fill=(180,145,72,255), width=3)
        if not w:
            d.line((1, 0, 1, 127 if s else 31), fill=(180,145,72,255), width=3)
        if not e:
            d.line((126, 0, 126, 127 if s else 31), fill=(180,145,72,255), width=3)
        if not s:
            im.paste(face, (0, 32))
            # Jambs belong to terminating wall ends, never the repeated
            # doorway threshold. A two-tile passage has no centre post.
            if not w:
                im.alpha_composite(jamb, (4, 36))
            if not e:
                im.alpha_composite(ImageOps.mirror(jamb), (112, 36))
        # Keep texture seams untouched. Corner trim sits inside the tile;
        # the shared material below stays opaque through all junctions.
        for joined, x, flip in [(w, 4, False), (e, 100, True)]:
            trim = inner if joined and n else outer
            if flip:
                trim = ImageOps.mirror(trim)
            if not n or not joined:
                im.alpha_composite(trim, (x, 4))
        walls[bits] = save(im, 'wall-' + bits)

    # Preserve the approved Batch A floors/stations/scaffold/marker.
    tiles = {f'floor-{i}': read(f'floor-{i}-v1.png') for i in range(3)}
    for axis in ['ns', 'ew']:
        im = tiles['floor-0'].copy()
        d = ImageDraw.Draw(im)
        if axis == 'ns':
            for y in [28, 31, 96, 99]:
                d.line((0, y, 127, y), fill=(179, 143, 67, 255), width=2)
        else:
            for x in [28, 31, 96, 99]:
                d.line((x, 0, x, 127), fill=(179, 143, 67, 255), width=2)
        tiles['doorway-' + axis] = save(im, 'doorway-' + axis)
    return walls, tiles


def layout():
    floors, walls = set(), set()
    for x, y, w, h in [(0, 0, 8, 6), (7, 0, 8, 6), (14, 0, 8, 6),
                        (0, 5, 15, 11), (14, 5, 8, 6)]:
        for yy in range(y, y + h):
            for xx in range(x, x + w):
                (walls if xx in (x, x+w-1) or yy in (y, y+h-1)
                 else floors).add((xx, yy))
    doors = {(7, 2): 'ew', (7, 3): 'ew', (14, 2): 'ew', (14, 3): 'ew',
             (3, 5): 'ns', (4, 5): 'ns', (10, 5): 'ns', (11, 5): 'ns',
             (17, 5): 'ns', (18, 5): 'ns', (14, 7): 'ew', (14, 8): 'ew'}
    walls -= doors.keys()
    floors |= doors.keys()
    return floors, walls, doors


def mock(wall_images, tiles):
    # Full 2x resolution master; preview is downscaled only after assembly.
    im = Image.new('RGBA', (36*128, 22*128), (8, 9, 24, 255))
    d = ImageDraw.Draw(im)
    rng = random.Random(89)
    for _ in range(1600):
        x, y = rng.randrange(im.width), rng.randrange(im.height)
        d.point((x, y), fill=(80, 70, rng.randrange(100, 180), 255))
    floors, walls, doors = layout()
    for x, y in sorted(floors):
        im.alpha_composite(tiles[f'floor-{(x+y)%3}'], (128*x, 128*y))
    for x, y in sorted(walls):
        bits = ''.join(str(int(p in walls)) for p in
                       [(x,y-1), (x+1,y), (x,y+1), (x-1,y)])
        im.alpha_composite(wall_images[bits], (128*x, 128*y))
    for (x, y), axis in doors.items():
        im.alpha_composite(tiles['doorway-'+axis], (128*x, 128*y))
    for name, x, y, depth in [('command-console',3,1,1),
                             ('sample-analyzer',17,1,1),
                             ('upgrade-bench',15,6,1),
                             ('berth',2,8,4), ('berth',8,8,4)]:
        art = read(name+'-v1.png')
        im.alpha_composite(art, (128*x, 128*(y+depth)-art.height))
    save(im, 'starter-assembled-full')
    save(im.resize((1728, 1056), R), 'starter-assembled')
    return im


def diagnostics(walls, tiles):
    # Each mask plus horizontal/vertical paired thresholds for inspection.
    sheet = Image.new('RGBA', (1024, 1024), (25, 34, 43, 255))
    d = ImageDraw.Draw(sheet)
    for i, (bits, art) in enumerate(walls.items()):
        x, y = (i % 4)*256, (i // 4)*200
        sheet.alpha_composite(art, (x+64, y+24))
        d.text((x+92, y+160), bits, fill='white')
    for i, axis in enumerate(['ns', 'ew']):
        x = i*512 + 32
        for k in range(2):
            art = tiles['doorway-'+axis]
            # EW passage spans two rows; show it at half scale to fit.
            if axis == 'ew':
                art = art.resize((64,64), R)
                pos = (x, 850+64*k)
            else:
                pos = (x+128*k, 850)
            sheet.alpha_composite(art, pos)
        d.text((x, 820), 'Two repeated doorway-'+axis+' tiles; no central jamb', fill='white')
    save(sheet, 'wall-diagnostics')


def catalogue():
    rows = []
    for name in [*(f'floor-{i}' for i in range(3)),
                 *(f'wall-{i:04b}' for i in range(16)), 'doorway-ns', 'doorway-ew']:
        stem = name.replace('-', '.')
        version = 1 if name.startswith('floor') else (3 if name.startswith('doorway') else 2)
        rows.append({'id': 'core.carrier.tile.'+stem,
                     'file': f'{name}-v{version}.png',
                     'source': 'carrier/tile/'+stem.replace('.', '/')+'.png'})
    for name in ['command-console', 'upgrade-bench', 'sample-analyzer',
                 'salvage-sorter', 'combat-simulator', 'berth']:
        key = name.replace('-', '_')
        rows.append({'id': 'core.carrier.item.'+key, 'file': name+'-v1.png',
                     'source': 'carrier/item/'+key+'.png'})
    for name in ['scaffold', 'front-marker']:
        key = name.replace('-', '_')
        rows.append({'id': 'core.carrier.'+key, 'file': name+'-v1.png',
                     'source': 'carrier/'+key+'.png'})
    for row in rows:
        art = read(row['file'])
        row['size'] = list(art.size)
        row['alpha_bbox'] = art.getchannel('A').getbbox()
    (ROOT/'batch-a-import-v2.json').write_text(json.dumps(rows, indent=2)+'\n')
    return rows


def verify(walls, tiles):
    for bits, im in walls.items():
        assert im.size == (128, 128)
        assert im.getchannel('A').getextrema() == (255, 255), bits
        # Every connected wall edge has zero transparent gaps.
    horizontal = walls['0101']
    assert horizontal.crop((0,0,1,128)).tobytes() == horizontal.crop((127,0,128,128)).tobytes()
    vertical = walls['1010']
    assert vertical.crop((0,0,128,1)).tobytes() == vertical.crop((0,127,128,128)).tobytes()
    for axis in ['ns', 'ew']:
        im = tiles['doorway-'+axis]
        # The middle of the passage is the original unobstructed floor.
        assert im.getpixel((64, 64)) == tiles['floor-0'].getpixel((64, 64))
    for name in ['command-console','upgrade-bench','sample-analyzer',
                 'salvage-sorter','combat-simulator','berth']:
        im = read(name+'-v1.png')
        x0,y0,x1,y1 = im.getchannel('A').getbbox()
        assert min(x0,y0,im.width-x1,im.height-y1) >= 3, name


def scale_sheet():
    sheet = Image.new('RGBA', (1900, 650), (25,34,43,255))
    d = ImageDraw.Draw(sheet)
    pilot_path = ROOT.parents[2] / 'assets/source/core/carrier/pilot/s/idle.png'
    pilot = Image.open(pilot_path).convert('RGBA')
    pilot = pilot.crop(pilot.getchannel('A').getbbox())
    pilot = pilot.resize((round(pilot.width*112/pilot.height),112), R)
    for i, name in enumerate(['command-console','upgrade-bench','sample-analyzer',
                              'salvage-sorter','combat-simulator','berth']):
        x = i*280 if i < 5 else 1400
        art = read(name+'-v1.png')
        sheet.alpha_composite(art,(x,560-art.height))
        sheet.alpha_composite(pilot,(x+12,560-112))
        d.text((x+8,585), name, fill='white')
    d.text((12,620),'Same native 2x scale. Pilot alpha height = 112 px. Station art retained from Batch A.',fill='white')
    save(sheet,'station-scale')


if __name__ == '__main__':
    walls, tiles = build()
    verify(walls, tiles)
    mock(walls, tiles)
    diagnostics(walls, tiles)
    scale_sheet()
    rows = catalogue()
    print(f'PASS: 16 opaque connected wall masks, 2 clear doorway tiles, '
          f'6 whole station sprites; {len(rows)} stable-ID catalogue entries.')
