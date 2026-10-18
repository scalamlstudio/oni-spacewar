"""TAKOAI-101. Python 3 + Pillow; deterministic, no image-generation dependency.
Writes only beside this script. Run to rebuild art, catalogue, mocks and checks.
"""
from pathlib import Path
import importlib.util
import json
import sys
sys.dont_write_bytecode = True
from PIL import Image, ImageDraw, ImageChops, ImageFilter
ROOT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('layout_v2', ROOT/'build-batch-a-v2.py')
old = importlib.util.module_from_spec(spec)
spec.loader.exec_module(old)
DIRS = [(0,-1),(1,0),(0,1),(-1,0)]
DIAGS = [(1,-1),(1,1),(-1,1),(-1,-1)]
FLOOR = (223,229,232,255)
CAP = (57,70,84,255)
FACE = (36,46,58,255)
CYAN = (100,213,224,255)

def save(im, name):
    im.save(ROOT/(name+'.png'))
    return im

def canonical(bits):
    b = list(bits)
    for k, (a,c) in enumerate([(0,1),(1,2),(2,3),(3,0)]):
        if b[a]=='1' or b[c]=='1':
            b[4+k]='0'  # Cardinal strips already cover this corner.
    return ''.join(b)

def key_at(p, occupied, walls):
    x,y=p
    ext=canonical(''.join(str(int((x+dx,y+dy) not in occupied)) for dx,dy in DIRS+DIAGS))
    joins=''.join(str(int((x+dx,y+dy) in walls)) for dx,dy in DIRS)
    return joins,ext

def branch(bits, width):
    im=Image.new('L',(128,128)); d=ImageDraw.Draw(im)
    lo=64-width//2; hi=lo+width-1
    d.rounded_rectangle((lo,lo,hi,hi),radius=min(10,width//2),fill=255)
    for yes,box in zip(bits,[(lo,0,hi,64),(64,lo,127,hi),(lo,64,hi,127),(0,lo,64,hi)]):
        if yes=='1': d.rectangle(box,fill=255)
    return im

def render(joins, ext):
    # Padding permits boundary extraction without false seams at tile edges.
    hull=Image.new('L',(384,384)); d=ImageDraw.Draw(hull)
    for yes,(dx,dy) in zip(ext,DIRS+DIAGS):
        if yes=='1':
            d.rectangle((128+128*dx-64,128+128*dy-64,
                         128+128*dx+191,128+128*dy+191),fill=255)
    inner=ImageChops.subtract(hull.filter(ImageFilter.MaxFilter(5)),hull)
    # Only the inner side gets cyan; shell itself reaches the canvas edge.
    shell=hull.crop((128,128,256,256))
    guide=inner.crop((128,128,256,256))
    im=Image.new('RGBA',(128,128),FLOOR)
    cap=branch(joins,48)
    face=Image.new('L',(128,128))
    for dy in range(1,33):
        shifted=Image.new('L',(128,128)); shifted.paste(cap,(0,dy))
        face=ImageChops.lighter(face,shifted)
    im.paste(FACE,(0,0,128,128),face)
    im.paste(CAP,(0,0,128,128),cap)
    im.paste(CYAN,(0,0,128,128),branch(joins,2))
    im.paste(CAP,(0,0,128,128),shell)
    # South-facing shell surface stays INSIDE its 64 px silhouette.
    lower=ImageChops.subtract(hull,ImageChops.offset(hull,0,-16)).crop((128,128,256,256))
    im.paste(FACE,(0,0,128,128),lower)
    im.paste(CYAN,(0,0,128,128),guide)
    # Quiet top/left highlight, also continuous along straight runs.
    rim=ImageChops.subtract(hull,ImageChops.offset(hull,1,1)).crop((128,128,256,256))
    im.paste((68,82,95,255),(0,0,128,128),rim)
    return im

def name(k): return 'hull-wall-'+k[0]+'-x'+k[1]+'-v1'

floors,walls,doors=old.layout()
occupied=floors|walls
placed={p:key_at(p,occupied,walls) for p in walls}
keys={k for k in placed.values() if '1' in k[1]}
# All four straight sides, inward T connections, convex and concave corners.
for side in range(4):
    ext=''.join('1' if i==side else '0' for i in range(8))
    tangents={(side+1)%4,(side+3)%4}
    for tee in [False,True]:
        j=tangents|({(side+2)%4} if tee else set())
        keys.add((''.join(str(int(i in j)) for i in range(4)),ext))
    adj={side,(side+1)%4}
    keys.add((''.join(str(int(i not in adj)) for i in range(4)),
              ''.join(str(int(i in adj)) for i in range(8))))
    # A concave step has an exterior diagonal but no exterior cardinal.
    keys.add(('1111','0000'+''.join('1' if i==side else '0' for i in range(4))))
arts={k:save(render(*k),name(k)) for k in sorted(keys)}
rows=[]
for k,im in arts.items():
    rows.append({'id':'core.carrier.tile.hull_wall_'+k[0]+'_x'+k[1],
                 'file':name(k)+'.png','size':[128,128],'alpha_bbox':list(im.getbbox()),
                 'connections_nesw':k[0],'exterior_nesw_ne_se_sw_nw':k[1],
                 'source':'carrier/tile/hull_wall/'+k[0]+'_x'+k[1]+'.png'})
catalogue={'schema_version':1,'tile_world_px':64,'art_scale':2,
 'rules':{'connections':'NESW bits indicate adjacent WALL cells, exactly as v3. Doorways are not wall connections.',
 'exterior':'8 bits N E S W NE SE SW NW: 1 means neighbor is SPACE, outside union of finished floor/wall/doorway cells. Include diagonals. Clear each diagonal bit if either adjoining cardinal bit is 1.',
 'selection':'Compute both masks; select exact catalogue key. No exterior bits: retain v3 inner wall. Derive from the union of rooms, never individual room borders.',
 'rendering':'Opaque replacement tile with its interior floor included. Replace the centered wall on hull cells. Shell touches every exposed edge; no floor outside. Space cells receive no floor or hull tile. Do not rotate sprites: lighting is baked from top left.',
 'coverage':'Starter keys plus four straight runs, four inward T joins, four convex and four concave corners. For additional layouts run render(connections, canonical(exterior)) and catalogue the new key; do not silently substitute a different mask.',
 'opening':'Skipped: all six two-cell starter doorways join rooms, not space. Berths are interior objects; there is no exterior hangar opening.'},
 'tiles':rows,'starter_placements':[{'tile':list(p),'id':'core.carrier.tile.hull_wall_'+k[0]+'_x'+k[1]} for p,k in sorted(placed.items()) if k in arts]}
(ROOT/'outer-wall-import-v1.json').write_text(json.dumps(catalogue,indent=2)+'\n')

def mock(after):
    im=Image.new('RGBA',(24*128,18*128),(8,9,24,255))
    # Same deterministic backdrop in both comparison images.
    import random
    rng=random.Random(101); d=ImageDraw.Draw(im)
    for _ in range(900): d.point((rng.randrange(im.width),rng.randrange(im.height)),fill=(72,62,112,255))
    for x,y in sorted(occupied):
        p=(x,y); pos=((x+1)*128,(y+1)*128)
        if p in walls:
            k=placed[p]
            art=arts[k] if after and k in arts else old.read('wall-'+k[0]+'-v3.png')
        elif p in doors: art=old.read('doorway-'+doors[p]+'-v4.png')
        else: art=old.read(f'floor-{(x+y)%3}-v3.png')
        im.alpha_composite(art,pos)
    # Current station orientations, from TAKOAI-99. Floor geometry is exact starter.
    for n,x,y,depth in [('command-console',3,1,1),('sample-analyzer',17,1,1),
                         ('upgrade-bench',15,6,1),('berth',2,8,4),('berth',8,8,4)]:
        version=2 if n=='command-console' else 1
        art=old.read(n+f'-v{version}.png')
        im.alpha_composite(art,((x+1)*128,(y+1+depth)*128-art.height))
    return im
before=mock(False); after=mock(True)
save(after,'starter-hull-full-v1')
for scale,tag in [(2,'1x'),(4,'half')]:
    save(after.resize((after.width//scale,after.height//scale),Image.Resampling.LANCZOS),'starter-hull-'+tag+'-v1')
comparison=Image.new('RGBA',(1280,768),(8,9,24,255))
# North edge includes a shared-room inner wall T at x=7.
for i,im in enumerate([before,after]):
    comparison.alpha_composite(im.crop((5*128,64,15*128,448)),(0,i*384))
save(comparison,'hull-edge-before-after-v1')
sheet=Image.new('RGBA',(768,((len(arts)+5)//6)*128),(8,9,24,255))
for i,im in enumerate(arts.values()): sheet.alpha_composite(im,(i%6*128,i//6*128))
save(sheet,'hull-wall-contact-sheet-v1')
# Every exposed cardinal edge must be shell, never white floor or cyan.
for p,k in placed.items():
    if k not in arts: continue
    art=arts[k]
    for bit,box in zip(k[1][:4],[(0,0,128,1),(127,0,128,128),(0,127,128,128),(0,0,1,128)]):
        if bit=='1':
            assert all(max(c[:3])<110 and c[3]==255 for c in list(art.crop(box).get_flattened_data())),(p,k,box)
# Every adjacent pair along the real hull shares exactly matching edge pixels.
for (x,y),k in placed.items():
    if k not in arts: continue
    for q,a,b in [((x+1,y),(127,0,128,128),(0,0,1,128)),((x,y+1),(0,127,128,128),(0,0,128,1))]:
        if q in placed and placed[q] in arts:
            assert arts[k].crop(a).tobytes()==arts[placed[q]].crop(b).tobytes(),((x,y),q,k,placed[q])
# Hull-to-inner joins must also match the unchanged v3 silhouettes and cyan.
for (x,y),k in placed.items():
    if k not in arts: continue
    for direction,(dx,dy) in enumerate(DIRS):
        q=(x+dx,y+dy)
        if q in placed and placed[q] not in arts and k[0][direction]=='1':
            boxes=[((0,0,128,1),(0,127,128,128)),((127,0,128,128),(0,0,1,128)),((0,127,128,128),(0,0,128,1)),((0,0,1,128),(127,0,128,128))]
            a,b=boxes[direction]
            inner=old.read('wall-'+placed[q][0]+'-v3.png')
            assert arts[k].crop(a).tobytes()==inner.crop(b).tobytes(),('inner join',(x,y),q)
assert not any(any((x+dx,y+dy) not in occupied for dx,dy in DIRS) for x,y in doors)
print(f'PASS: {len(arts)} hull tiles; {sum(k in arts for k in placed.values())} starter hull cells; exposed edges contain no floor; all adjacent hull seams exact; no exterior openings.')
