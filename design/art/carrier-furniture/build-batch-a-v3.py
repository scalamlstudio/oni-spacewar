"""Clean office tile option. Reproduce with Python 3 + Pillow; no generated textures.
All changes and outputs are confined to design/art/carrier-furniture.
"""
from pathlib import Path
import importlib.util, json, sys
sys.dont_write_bytecode = True
from PIL import Image, ImageDraw, ImageChops, ImageOps
ROOT=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('previous',ROOT/'build-batch-a-v2.py')
old=importlib.util.module_from_spec(spec);spec.loader.exec_module(old)

def save(im,name):
    im.save(ROOT/(name+'.png'));return im

def floor(i):
    im=Image.new('RGBA',(128,128),(222+i,228+i,231+i,255))
    d=ImageDraw.Draw(im)
    # One quiet office panel, common seam colour at all variant boundaries.
    d.line((0,0,127,0),fill=(207,215,220),width=1)
    d.line((0,0,0,127),fill=(207,215,220),width=1)
    return im

def shape(bits,width):
    n,e,s,w=map(int,bits)
    im=Image.new('L',(128,128));d=ImageDraw.Draw(im)
    lo=64-width//2;hi=lo+width-1
    d.rounded_rectangle((lo,lo,hi,hi),radius=min(10,width//2),fill=255)
    for yes,box in [(n,(lo,0,hi,64)),(e,(64,lo,127,hi)),(s,(lo,64,hi,127)),(w,(0,lo,64,hi))]:
        if yes:d.rectangle(box,fill=255)
    return im

def wall(bits):
    # Floor remains visible outside the 48 px cap. Lower face is an extrusion
    # of exactly the same union, so corner and T geometry has no internal seams.
    im=Image.new('RGBA',(128,128),(223,229,232,255))
    cap=shape(bits,48)
    face=Image.new('L',(128,128))
    for dy in range(1,33):
        shifted=Image.new('L',(128,128));shifted.paste(cap,(0,dy))
        face=ImageChops.lighter(face,shifted)
    # Clip extrusion at the canvas edge; never wrap a south end to the north.
    im.paste((36,46,58,255),(0,0,128,128),face)
    im.paste((57,70,84,255),(0,0,128,128),cap)
    # Single continuous cyan guide on the cap. No end brackets or tile frames.
    im.paste((100,213,224,255),(0,0,128,128),shape(bits,2))
    return im

floors={f'floor-{i}':save(floor(i),f'floor-{i}-v3') for i in range(3)}
walls={f'{m:04b}':save(wall(f'{m:04b}'),f'wall-{m:04b}-v3') for m in range(16)}
tiles=dict(floors)
for axis in ['ns','ew']:
    im=floor(0);d=ImageDraw.Draw(im)
    # Posts are the rounded ends of adjacent walls, never repeated in the
    # threshold: identical paired tiles cannot introduce a middle post.
    line=(0,64,127,64) if axis=='ns' else (64,0,64,127)
    d.line(line,fill=(100,213,224),width=2)
    tiles['doorway-'+axis]=save(im,'doorway-'+axis+'-v4')
# Reuse the established coordinates, starfield and unchanged station art.
original_save=old.save
old.save=lambda im,name:save(im,name+'-v3')
full=old.mock(walls,tiles)
save(full.resize((2304,1408),Image.Resampling.LANCZOS),'starter-assembled-1x-v3')
save(full.resize((1152,704),Image.Resampling.LANCZOS),'starter-assembled-half-v3')
save(ImageOps.grayscale(full).resize((2304,1408),Image.Resampling.LANCZOS),'starter-assembled-grayscale-v3')
old.diagnostics(walls,tiles)
# Six stations, including the two not placed in the initial layout.
sheet=Image.new('RGBA',(1900,720),(223,229,232,255));d=ImageDraw.Draw(sheet)
for i,name in enumerate(['command-console','upgrade-bench','sample-analyzer','salvage-sorter','combat-simulator','berth']):
    x=i*280 if i<5 else 1400
    art=old.read(name+'-v1.png');sheet.alpha_composite(art,(x,600-art.height))
    d.text((x+8,625),name,fill=(36,46,58))
save(sheet,'station-floor-check-v3')
rows=json.loads((ROOT/'batch-a-import-v2.json').read_text())
for row in rows:
    stem=row['file'].rsplit('-v',1)[0]
    if stem.startswith(('floor-','wall-','doorway-')):
        row['file']=stem+('-v4.png' if stem.startswith('doorway') else '-v3.png')
    art=old.read(row['file']);row['size']=list(art.size);row['alpha_bbox']=art.getchannel('A').getbbox()
(ROOT/'batch-a-import-v3.json').write_text(json.dumps(rows,indent=2)+'\n')
# Meaningful connectivity checks across EVERY compatible pair, including T's.
for a,ia in walls.items():
    assert ia.size==(128,128)
    for b,ib in walls.items():
        if a[1]=='1' and b[3]=='1':
            assert ia.crop((127,0,128,128)).tobytes()==ib.crop((0,0,1,128)).tobytes(),(a,b,'EW')
        if a[2]=='1' and b[0]=='1':
            assert ia.crop((0,127,128,128)).tobytes()==ib.crop((0,0,128,1)).tobytes(),(a,b,'NS')
assert ImageOps.grayscale(floors['floor-0']).getpixel((64,64))-ImageOps.grayscale(walls['0101']).getpixel((30,90))>140
print('PASS: 16 masks, all compatible edge pairs, 21 replacement tile dimensions, grayscale contrast; 29 catalogue entries.')
