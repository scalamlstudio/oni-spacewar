"""TAKOAI-102. Emits every carrier hull-wall tile a player-built layout can
reach that Art's starter catalogue (TAKOAI-101) does not ship, drawn by Art's
own deterministic `render()` from `build-outer-wall-v1.py`.

Python 3 + Pillow. Art's script writes beside itself, so it runs on a scratch
copy of design/art/carrier-furniture (its own checks run there too); nothing
under design/art/ is touched. Writes `extra-v1/` beside this file: one PNG
per key and `catalogue.json` in Art's catalogue format, which
`carrier-tile-import` reads.

    python3 crates/content/hull-wall/build-hull-wall-extra.py

Reachable keys come from a local enumeration: every set of Small and Large
rooms touching the 3 x 3 window round a wall cell that overlap only on their
walls (layout.rs's rule), doorways from each pair's shared wall. That is a
superset of what valid layouts reach; `layout::tests` checks the same from
the Rust side, so the two can't drift apart unnoticed.
"""
from pathlib import Path
import importlib.util
import itertools
import json
import shutil
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
ART = REPO / 'design/art/carrier-furniture'
OUT = HERE / 'extra-v1'

# layout::RoomSize::floor, (width, depth) in tiles; the wall ring adds one.
SIZES = [(6, 4), (13, 9)]
DIRS = [(0, -1), (1, 0), (0, 1), (-1, 0)]
DIAGS = [(1, -1), (1, 1), (-1, 1), (-1, -1)]


def is_floor(b, t):
    return b[0] < t[0] < b[2] and b[1] < t[1] < b[3]


def contains(b, t):
    return b[0] <= t[0] <= b[2] and b[1] <= t[1] <= b[3]


def compatible(a, b):
    """Overlapping only on walls (layout::Carrier::can_place_room)."""
    xs = range(max(a[0], b[0]), min(a[2], b[2]) + 1)
    ys = range(max(a[1], b[1]), min(a[3], b[3]) + 1)
    return not any(is_floor(a, (x, y)) or is_floor(b, (x, y)) for x in xs for y in ys)


def doorway(a, b):
    """layout::doorway_between."""
    if a[0] == b[2] or a[2] == b[0]:
        at, lo, hi, vertical = a[0] if a[0] == b[2] else a[2], max(a[1], b[1]) + 1, min(a[3], b[3]) - 1, True
    elif a[1] == b[3] or a[3] == b[1]:
        at, lo, hi, vertical = a[1] if a[1] == b[3] else a[3], max(a[0], b[0]) + 1, min(a[2], b[2]) - 1, False
    else:
        return []
    if hi - lo + 1 < 2:
        return []
    s = lo + (hi - lo + 1 - 2) // 2
    return [(at, s), (at, s + 1)] if vertical else [(s, at), (s + 1, at)]


def reachable_keys(canonical):
    rooms = [(x, y, x + w + 1, y + d + 1)
             for w, d in SIZES
             for x in range(-w - 2, 2)
             for y in range(-d - 2, 2)]
    fits = [{j for j, b in enumerate(rooms) if j != i and compatible(a, b)}
            for i, a in enumerate(rooms)]
    keys = set()

    def kind(chosen, doors, t):
        if t in doors:
            return 'doorway'
        k = None
        for b in chosen:
            if is_floor(b, t):
                return 'floor'
            if contains(b, t):
                k = 'wall'
        return k

    def visit(chosen):
        doors = {t for a, b in itertools.combinations(chosen, 2) for t in doorway(a, b)}
        if kind(chosen, doors, (0, 0)) != 'wall':
            return
        ext = canonical(''.join(str(int(kind(chosen, doors, d) is None)) for d in DIRS + DIAGS))
        if '1' in ext:
            keys.add((''.join(str(int(kind(chosen, doors, d) == 'wall')) for d in DIRS), ext))

    def grow(chosen, candidates):
        if chosen:
            visit([rooms[i] for i in chosen])
        for i in sorted(candidates):
            grow(chosen + [i], {j for j in candidates if j > i} & fits[i])

    grow([], set(range(len(rooms))))
    return keys


def main():
    with tempfile.TemporaryDirectory() as tmp:
        scratch = Path(tmp) / 'carrier-furniture'
        shutil.copytree(ART, scratch)
        spec = importlib.util.spec_from_file_location('outer_wall', scratch / 'build-outer-wall-v1.py')
        art = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(art)
        shipped = json.loads((ART / 'outer-wall-import-v1.json').read_text())['tiles']
        have = {(t['connections_nesw'], t['exterior_nesw_ne_se_sw_nw']) for t in shipped}
        keys = reachable_keys(art.canonical)
        if OUT.exists():
            shutil.rmtree(OUT)
        OUT.mkdir()
        rows = []
        for joins, ext in sorted(keys - have):
            name = f'hull-wall-{joins}-x{ext}-v1.png'
            im = art.render(joins, ext)
            im.save(OUT / name)
            rows.append({'id': f'core.carrier.tile.hull_wall_{joins}_x{ext}',
                         'file': name, 'size': [128, 128], 'alpha_bbox': list(im.getbbox()),
                         'connections_nesw': joins, 'exterior_nesw_ne_se_sw_nw': ext,
                         'source': f'carrier/tile/hull_wall_{joins}_x{ext}.png'})
    catalogue = {'schema_version': 1, 'generator': 'design/art/carrier-furniture/build-outer-wall-v1.py render()',
                 'rules': 'Same keys and rules as design/art/carrier-furniture/outer-wall-import-v1.json',
                 'tiles': rows}
    (OUT / 'catalogue.json').write_text(json.dumps(catalogue, indent=2) + '\n')
    print(f'{len(keys)} reachable hull keys: {len(keys & have)} in Art\'s catalogue, '
          f'{len(rows)} generated into {OUT.relative_to(REPO)}; '
          f'{len(have - keys)} catalogued keys unreachable')


main()
