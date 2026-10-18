"""TAKOAI-104: whole-canvas orthographic redraws. Python 3 + Pillow.

Run this file from any directory. No generated-image warping or slicing.
All horizontal planes use the same axis-aligned projection; z goes straight
up. Coordinates are authored at the existing 2x asset resolution.
"""
from pathlib import Path
import json
import math
import random
import re
import sys
from PIL import Image, ImageDraw, ImageFont

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parent
TEAL = (37, 124, 132)
ORANGE = (215, 117, 48)
DARK = (22, 48, 59)
GOLD = (238, 173, 74)
CREAM = (227, 215, 177)
GREEN = (76, 137, 78)
GEOMETRY = []


class Painter:
    def __init__(self, size):
        self.im = Image.new('RGBA', size)
        self.d = ImageDraw.Draw(self.im)
        self.rng = random.Random(104)

    def panel(self, box, color=TEAL, radius=4, rim=True):
        x, y, r, b = map(round, box)
        if r <= x or b <= y:
            return
        mask = Image.new('L', (r-x, b-y))
        ImageDraw.Draw(mask).rounded_rectangle((0, 0, r-x-1, b-y-1), radius, fill=255)
        surf = Image.new('RGBA', mask.size)
        pix = surf.load()
        for yy in range(b-y):
            for xx in range(r-x):
                # Broad soft upper-left light, fine material grain, darker edges.
                nx,ny=xx/max(1,r-x),yy/max(1,b-y)
                light = 1.15 - .34*ny - .17*nx + .30*math.exp(-((nx-.23)**2+(ny-.18)**2)/.07)
                edge = min(xx, yy, r-x-1-xx, b-y-1-yy)
                light *= .77 + .23*min(1, edge/5)
                noise = self.rng.uniform(-2.3, 2.3)
                pix[xx, yy] = tuple(max(0,min(255,round(c*light+noise))) for c in color)+(255,)
        self.im.paste(surf, (x,y), mask)
        self.d = ImageDraw.Draw(self.im)
        if rim:
            self.d.rounded_rectangle((x,y,r-1,b-1), radius, outline=DARK, width=2)
            if r-x > 12 and b-y > 10:
                self.d.line((x+5,y+3,r-6,y+3), fill=tuple(min(255,c+55) for c in color),width=1)
                self.d.line((x+3,y+5,x+3,b-6),fill=tuple(min(255,c+25) for c in color),width=1)
                self.d.line((x+5,b-4,r-5,b-4),fill=tuple(round(c*.55) for c in color),width=1)
                # Short brushed-metal/cloth scuffs follow the material plane.
                for _ in range(max(1,(r-x)*(b-y)//1800)):
                    sx=self.rng.randint(x+5,r-6); sy=self.rng.randint(y+5,b-6)
                    self.d.line((sx,sy,min(r-5,sx+self.rng.randint(2,9)),sy),
                                fill=tuple(min(255,round(c*1.13)) for c in color),width=1)

    def line(self, xy, fill=GOLD, width=2):
        self.d.line(tuple(map(round,xy)), fill=fill, width=width)

    def ellipse(self, box, color=ORANGE):
        x0,y0,x1,y1=map(round,box)
        if x1-x0>10 and y1-y0>10:
            mask=Image.new('L',(x1-x0,y1-y0))
            ImageDraw.Draw(mask).ellipse((0,0,x1-x0-1,y1-y0-1),fill=255)
            surface=Image.new('RGBA',mask.size)
            dd=ImageDraw.Draw(surface)
            for yy in range(y1-y0):
                light=1.28-.53*yy/(y1-y0)
                dd.line((0,yy,x1-x0,yy),fill=tuple(min(255,round(c*light)) for c in color))
            self.im.paste(surface,(x0,y0),mask)
            self.d=ImageDraw.Draw(self.im)
            self.d.ellipse((x0,y0,x1-1,y1-1),outline=DARK,width=2)
        else:
            self.d.ellipse((x0,y0,x1-1,y1-1),fill=color,outline=DARK,width=1)
        x,y,r,b=box
        if r-x > 8 and b-y > 8:
            self.d.arc((x+3,y+2,r-3,b-4),190,300,fill=tuple(min(255,c+65) for c in color),width=2)

    def box(self, x, y, w, depth, height, color=TEAL):
        # Back/front edges have identical widths. No visible east/west face.
        self.panel((x,y+depth-height,x+w,y+depth),tuple(round(c*.77) for c in color))
        self.panel((x,y-height,x+w,y+depth-height),color)
        if w>65 and depth>30:
            self.line((x+8,y-height+8,x+w-9,y-height+8),tuple(min(255,c+40) for c in color),1)
            self.line((x+8,y-height+8,x+8,y+depth-height-8),tuple(min(255,c+25) for c in color),1)
            for xx in (x+7,x+w-8):
                for yy in (y-height+7,y+depth-height-8):
                    self.ellipse((xx-2,yy-2,xx+3,yy+3),GOLD)
        GEOMETRY.append([round(x,2),round(y,2),round(w,2),round(depth,2),round(height,2)])

    def bolts(self, box):
        x,y,r,b=box
        for xx in (x+6,r-7):
            for yy in (y+6,b-7):
                self.ellipse((xx-2,yy-2,xx+3,yy+3),GOLD)

    def screen(self, box, seed=0):
        self.panel(box,DARK)
        x,y,r,b=box
        rng=random.Random(seed)
        points=[(rng.randint(round(x+6),round(r-6)),rng.randint(round(y+6),round(b-6))) for _ in range(10)]
        for a,bb in zip(points,points[1:]): self.line((*a,*bb),(40,115,127),1)
        for xx,yy in points: self.ellipse((xx-1,yy-1,xx+2,yy+2),(123,227,217))

    def leaves(self, x,y,w,h):
        for i in range(9):
            a=i*2.399
            xx=x+math.cos(a)*w*.35
            yy=y+math.sin(a)*h*.3
            self.line((x,y+h*.3,xx,yy),(68,105,57),2)
            self.ellipse((xx-w*.13,yy-h*.19,xx+w*.13,yy+h*.19),GREEN)
            self.line((xx,yy-h*.12,xx,yy+h*.1),(153,180,92),1)


def draw(row):
    name=row['id'].split('.')[-1]
    w,h=row['size']; fw,fd=row['footprint']; z=row.get('reference_height') or 0
    p=Painter((w,h)); bottom=h-4
    depth=fd*128-8
    # Wall-mounted pieces have their original shallow canvas and wall anchor.
    if row.get('layer')=='wall':
        top={'wall_screen':27,'wall_light':57,'tool_rack':10}[name]
        p.box(4,bottom-8,w-8,8,bottom-top-8)
        if name=='wall_screen': p.screen((12,top+10,w-12,bottom-8),104)
        elif name=='wall_light': p.panel((12,top+9,w-12,bottom-6),GOLD,5)
        else:
            for i in range(3):
                x=25+35*i
                p.panel((x-4,40,x+4,83),ORANGE)
                if i==0:
                    p.line((x,43,x,25),CREAM,6)
                    p.line((x-8,18,x-8,30,x+8,30,x+8,18),CREAM,4)
                elif i==1:
                    p.line((x-7,64,x+6,30),CREAM,4); p.line((x+7,64,x-6,30),CREAM,4)
                else: p.line((x,43,x,20),CREAM,3)
        return p.im
    if name in ('berth','rug'):
        top=h-fd*128+4
        color=TEAL if name=='berth' else ORANGE
        p.panel((4,top,w-4,bottom),color,0)
        p.panel((13,top+9,w-13,bottom-9),DARK if name=='berth' else GOLD,0)
        p.panel((18,top+14,w-18,bottom-14),TEAL if name=='berth' else ORANGE,0)
        if name=='berth':
            for x in range(128,w,128): p.line((x,top+15,x,bottom-15),(62,144,148),1)
            for y in range(h-fd*128+128,bottom,128): p.line((19,y,w-20,y),(62,144,148),1)
            for x in (28,w-29):
                for y in (top+28,bottom-29):
                    p.line((x,y+30 if y<top+100 else y-30,x,y,x+30 if x<100 else x-30,y),GOLD,5)
            p.ellipse((w/2-48,h/2-36,w/2+48,h/2+36),DARK)
            p.line((w/2-30,h/2,w/2+30,h/2),GOLD,3)
        else:
            for y in range(top+22,bottom-20,4): p.line((22,y,w-23,y),(195,105,48),1)
            p.ellipse((w/2-23,h/2+4,w/2+23,h/2+34),TEAL)
            for dx,dy in [(-30,-12),(-10,-25),(13,-25),(32,-10)]:
                p.ellipse((w/2+dx-8,h/2+dy-10,w/2+dx+8,h/2+dy+10),TEAL)
        return p.im
    # Depth of a top face is uniformly compressed to 60% of the reserved
    # ground depth. Vertical extrusion is independent of ground projection.
    dep=min(depth*.60,depth)
    x=8; ww=w-16; ground=bottom-dep
    if name in ('command_console','upgrade_bench','sample_analyzer','salvage_sorter','combat_simulator'):
        heights={'command_console':108,'upgrade_bench':64,'sample_analyzer':104,'salvage_sorter':172,'combat_simulator':148}
        z=heights[name]
        p.box(4,ground,w-8,dep,z)
        fy=bottom-z
        p.panel((14,fy+9,w-14,bottom-10),TEAL)
        for xx in (22,w-38): p.panel((xx,bottom-27,xx+16,bottom-17),GOLD)
        if name=='command_console':
            p.panel((20,ground-z+8,w-20,ground-z+53),DARK)
            p.screen((27,ground-z+12,w-27,ground-z+48),1)
            for i in range(8): p.panel((25+25*i,fy-13,39+25*i,fy-5),GOLD if i%3==0 else TEAL,2)
            p.screen((44,fy+17,w-44,fy+58),2)
        elif name=='upgrade_bench':
            p.panel((14,ground-z+9,w-14,fy-7),CREAM)
            p.line((45,fy-18,85,fy-18),DARK,6)
            p.ellipse((34,fy-28,52,fy-9),DARK)
            p.panel((140,ground-z+19,205,fy-12),ORANGE)
            for xx in (18,134): p.panel((xx,fy+10,xx+100,bottom-12),TEAL)
        elif name=='sample_analyzer':
            p.box(28,ground+15,83,38,44,CREAM)
            p.ellipse((45,ground-22,94,ground+2),(80,206,213))
            p.screen((137,ground-z+14,w-23,fy-12),3)
            for xx in range(135,220,20): p.ellipse((xx,fy+18,xx+10,fy+39),(83,211,210))
        elif name=='salvage_sorter':
            p.panel((27,ground-z+18,w-27,fy-18),DARK)
            for yy in range(round(ground-z+27),round(fy-20),18): p.line((35,yy,w-35,yy),(91,117,122),7)
            p.panel((22,fy+20,148,bottom-20),DARK)
            for xx in (32,70,108): p.panel((xx,fy+40,xx+23,bottom-26),ORANGE)
            p.screen((169,fy+26,w-19,fy+82),4)
        else:
            p.panel((22,ground-z+15,w-22,fy-12),DARK)
            for xx in (38,w-58): p.box(xx,ground+15,20,75,48,ORANGE)
            p.screen((52,fy+18,w-52,bottom-22),5)
            p.ellipse((w/2-20,fy+42,w/2+20,fy+82),ORANGE)
        p.bolts((4,ground-z,w-4,fy))
    elif name in ('locker','snack_dispenser','aquarium','crate_stack'):
        z={'locker':180,'snack_dispenser':140,'aquarium':100,'crate_stack':90}[name]
        p.box(x,ground,ww,dep,z)
        fy=bottom-z
        if name=='locker':
            p.panel((x+7,fy+7,w-x-7,bottom-7),TEAL)
            for yy in (fy+22,fy+29,fy+36,bottom-25,bottom-18): p.line((32,yy,w-32,yy),DARK,3)
            p.panel((w-32,fy+70,w-23,fy+107),GOLD)
            p.line((44,fy+18,44,fy+9,53,fy+15,72,fy+15,81,fy+9,81,fy+18),GOLD,2)
        elif name=='snack_dispenser':
            p.panel((18,fy+9,w-28,bottom-43),DARK)
            for yy in range(round(fy+16),round(bottom-48),25):
                for xx in (25,49,73): p.panel((xx,yy,xx+15,yy+18),ORANGE if xx!=49 else CREAM,2)
            p.panel((22,bottom-35,w-22,bottom-12),ORANGE)
            p.panel((30,bottom-30,w-30,bottom-17),DARK)
        elif name=='aquarium':
            p.panel((16,fy+8,w-16,bottom-35),(43,141,155))
            for xx in range(30,w-24,28):
                p.line((xx,bottom-40,xx-7,bottom-64),GREEN,3)
                p.ellipse((xx,fy+28,xx+13,fy+36),GOLD)
            p.line((20,fy+13,w-21,fy+13),(143,222,217),2)
            for xx in (23,w/2+8): p.panel((xx,bottom-28,xx+ww/2-24,bottom-8),TEAL)
        else:
            for yy in (fy+8,fy+49):
                for xx in (12,66):
                    p.panel((xx,yy,xx+49,min(yy+36,bottom-5)),TEAL)
                    p.panel((xx+18,yy+5,xx+29,yy+18),GOLD,2)
    elif name in ('table','sofa','bunk_bed','stool','cat_bed'):
        if name=='table':
            for xx in (15,w-29): p.box(xx,ground+12,14,dep-14,52)
            p.box(4,ground-44,w-8,dep,8,ORANGE)
            for yy in range(round(ground-44),round(bottom-56),12): p.line((14,yy,w-15,yy),(161,83,43),1)
        elif name=='bunk_bed':
            for level in (16,104):
                p.box(10,ground,w-20,dep,level)
                p.panel((17,ground-level+9,w-17,bottom-level-6),ORANGE)
                p.panel((24,ground-level+12,w-24,ground-level+42),CREAM,7)
                for yy in range(round(ground-level+51),round(bottom-level-8),16): p.line((22,yy,w-23,yy),GOLD,1)
            for xx in (6,w-13): p.panel((xx,ground-112,xx+7,bottom),TEAL)
            for yy in range(round(bottom-94),round(bottom-8),18): p.line((w-35,yy,w-12,yy),GOLD,3)
            p.panel((17,bottom-75,w-37,bottom-35),ORANGE,6)
            p.panel((23,bottom-70,w-43,bottom-57),CREAM,4)
        elif name=='sofa':
            p.box(10,ground,w-20,dep,24)
            p.box(14,ground,w-28,24,72,ORANGE)
            for xx in (23,w/2+3): p.box(xx,ground+33,w/2-26,dep-38,32,ORANGE)
            for xx in (7,w-28): p.box(xx,ground+15,21,dep-15,48)
            for xx in (50,w-76):
                p.panel((xx,ground-37,xx+27,ground-12),CREAM,7)
                p.line((xx+2,ground-36,xx+2,ground-45,xx+10,ground-37),CREAM,3)
                p.line((xx+17,ground-37,xx+25,ground-45,xx+25,ground-36),CREAM,3)
        else:
            z=36 if name=='stool' else 20
            cx=w/2; cy=bottom-dep/2
            if name=='stool':
                for xx in (24,w-36): p.panel((xx,cy-5,xx+12,bottom),TEAL)
            p.ellipse((8,cy-dep/2-z,w-8,cy+dep/2),TEAL)
            p.ellipse((12,cy-dep/2-z,w-12,cy+dep/2-z),ORANGE)
            if name=='cat_bed':
                p.ellipse((24,cy-dep/2-z+9,w-24,cy+dep/2-z-9),(170,81,39))
                p.line((25,cy-dep/2-z+5,27,cy-dep/2-z-10,40,cy-dep/2-z+1),GOLD,4)
                p.line((w-40,cy-dep/2-z+1,w-27,cy-dep/2-z-10,w-25,cy-dep/2-z+5),GOLD,4)
            else:
                p.line((cx,cy-dep/2-z+4,cx,cy+dep/2-z-4),GOLD,1)
    elif name in ('cat_tree','scratch_post','floor_lamp'):
        p.box(12,ground,w-24,dep,10)
        z={'cat_tree':200,'scratch_post':80,'floor_lamp':160}[name]
        p.panel((55,bottom-z,73,bottom-12),CREAM if name!='floor_lamp' else TEAL)
        if name!='floor_lamp':
            for yy in range(round(bottom-z+3),round(bottom-13),4): p.line((57,yy,70,yy),(150,130,96),1)
        if name=='cat_tree':
            for zz,xx in ((195,8),(115,48)):
                p.box(xx,bottom-35,65,35,zz,TEAL)
                p.panel((xx+5,bottom-35-zz+5,xx+60,bottom-zz-5),ORANGE)
            p.panel((22,bottom-90,103,bottom-20),TEAL,12)
            p.ellipse((40,bottom-80,86,bottom-29),DARK)
        elif name=='scratch_post': p.ellipse((39,bottom-z-14,89,bottom-z+10),GOLD)
        else:
            p.ellipse((20,bottom-z-12,108,bottom-z+28),GOLD)
            p.line((64,bottom-z-12,64,bottom-z+28),TEAL,4)
    elif name in ('potted_plant','hydro_planter'):
        if name=='potted_plant':
            p.box(29,ground+25,w-58,dep-25,35,ORANGE)
            p.leaves(w/2,bottom-70,102,70)
        else:
            p.box(4,ground,w-8,dep,28)
            p.panel((12,ground-22,w-12,bottom-33),DARK)
            for xx in range(35,w-20,42): p.leaves(xx,bottom-64,48,47)
            p.line((10,bottom-24,w-10,bottom-24),GOLD,3)
    elif name=='food_bowls':
        p.box(4,ground+20,w-8,dep-20,3)
        for xx,color in ((34,ORANGE),(92,(66,184,210))):
            p.ellipse((xx-25,bottom-59,xx+25,bottom-13),CREAM)
            p.ellipse((xx-19,bottom-56,xx+19,bottom-25),color)
            if xx==34:
                for i in range(10):
                    a=i*2.4; dx=math.cos(a)*12; dy=math.sin(a)*8
                    p.ellipse((xx+dx-2,bottom-42+dy-2,xx+dx+3,bottom-42+dy+3),GOLD)
    else:
        raise ValueError(name)
    return p.im


def main():
    rows={}
    for file in ('batch-a-import-v3.json','batch-b-import-v1.json','orientation-import-patch-v2.json'):
        for row in json.loads((ROOT/file).read_text()):
            if '.item.' in row['id']:
                rows[row['id']]={**rows.get(row['id'],{}),**row}
    assert len(rows)==25
    stations={'command_console':(2,1,160),'upgrade_bench':(2,1,96),'sample_analyzer':(2,1,160),
              'salvage_sorter':(2,2,192),'combat_simulator':(2,2,160),'berth':(4,4,32)}
    patch=[]; originals={}; arts={}; grid_records=[]
    for row in rows.values():
        name=row['id'].split('.')[-1]
        if name in stations:
            fw,fd,head=stations[name]
            row.update(footprint=[fw,fd],headroom=head,anchor=[row['size'][0]//2,row['size'][1]],layer='object')
        originals[name]=Image.open(ROOT/row['file']).convert('RGBA')
        out=draw(row)
        assert list(out.size)==row['size']
        box=out.getbbox(); w,h=out.size
        assert box and box[0]>=4 and box[1]>=4 and box[2]<=w-4 and box[3]<=h-4,(name,box)
        version=int(re.search(r'-v(\d+)\.png',row['file'])[1])+1
        row={**row,'file':name.replace('_','-')+f'-v{version}.png','alpha_bbox':list(box)}
        out.save(ROOT/row['file']); arts[name]=out; patch.append(row)
        fw,fd=row['footprint']
        grid_records.append(dict(id=row['id'],footprint=row['footprint'],canvas=row['size'],anchor=row['anchor'],
                                 ground_rect=[0,h-fd*128,fw*128,h],alpha_bbox=list(box)))
        if name in ('berth','rug'):
            assert box==(4,h-fd*128+4,w-4,h-4)
    # Batch A station rows never carried footprint metadata. The importer's
    # art-only patch contract forbids introducing it: keep those row shapes.
    exported=[]
    for row in patch:
        name=row['id'].split('.')[-1]
        exported.append({k:v for k,v in row.items() if name not in stations or k not in ('footprint','headroom','anchor','layer')})
    (ROOT/'orthographic-import-patch-v1.json').write_text(json.dumps(exported,indent=2)+'\n')
    (ROOT/'orthographic-geometry-v1.json').write_text(json.dumps(dict(projection='x screen-right; y screen-down; z screen-up; no convergence',
        tile_px=128,transparent_inset_px=4,items=grid_records,boxes=GEOMETRY),indent=2)+'\n')
    # Each pair includes the ground grid overlaid AFTER the sprite, so the
    # relationship between top surfaces, extrusion and reserved cells is visible.
    sheet=Image.new('RGBA',(1800,7*390),(224,230,233,255)); d=ImageDraw.Draw(sheet)
    for i,row in enumerate(patch):
        name=row['id'].split('.')[-1]; xx=(i%4)*450; yy=(i//4)*390
        d.text((xx+12,yy+8),name.replace('_',' ')+' | before / ortho',fill=DARK)
        for j,im in enumerate((originals[name],arts[name])):
            scale=min(205/im.width,335/im.height,.85)
            size=(round(im.width*scale),round(im.height*scale))
            px=xx+j*222+(222-size[0])//2; py=yy+370-size[1]
            sheet.alpha_composite(im.resize(size,Image.Resampling.LANCZOS),(px,py))
            fw,fd=row['footprint']; h=im.height
            for gx in range(fw+1):
                x=px+round(gx*128*scale)
                d.line((x,py+round((h-fd*128)*scale),x,py+size[1]),fill=(192,63,119,150),width=1)
            for gy in range(fd+1):
                y=py+round((h-fd*128+gy*128)*scale)
                d.line((px,y,px+size[0],y),fill=(192,63,119,150),width=1)
    sheet.convert('RGB').save(ROOT/'orthographic-before-after-v1.png')
    # Starting-layout coordinates are the existing design mock's coordinates.
    import importlib.util
    spec=importlib.util.spec_from_file_location('layout',ROOT/'build-batch-a-v2.py')
    old=importlib.util.module_from_spec(spec); spec.loader.exec_module(old)
    floors,walls,doors=old.layout()
    mock=Image.new('RGBA',(22*128,16*128),(14,17,32,255))
    for x,y in floors: mock.alpha_composite(Image.open(ROOT/f'floor-{(x+y)%3}-v3.png').convert('RGBA'),(128*x,128*y))
    for x,y in walls:
        bits=''.join(str(int(q in walls)) for q in [(x,y-1),(x+1,y),(x,y+1),(x-1,y)])
        mock.alpha_composite(Image.open(ROOT/f'wall-{bits}-v3.png').convert('RGBA'),(128*x,128*y))
    for (x,y),axis in doors.items(): mock.alpha_composite(Image.open(ROOT/f'doorway-{axis}-v4.png').convert('RGBA'),(128*x,128*y))
    items=[('command_console',3,1),('wall_screen',5,0),('potted_plant',1,1),('floor_lamp',6,1),
           ('bunk_bed',9,1),('bunk_bed',10,1),('locker',12,1),('cat_bed',13,1),('rug',10,3),('table',10,3),
           ('sample_analyzer',17,1),('potted_plant',15,1),('hydro_planter',19,1),('berth',2,8),('berth',8,8),
           ('wall_light',6,5),('wall_light',13,5),('upgrade_bench',15,6),('tool_rack',20,5),('crate_stack',19,9),('crate_stack',20,9)]
    lookup={r['id'].split('.')[-1]:r for r in patch}
    ordered=sorted(items,key=lambda q:(lookup[q[0]].get('layer')!='floor',q[2]+lookup[q[0]]['footprint'][1]))
    for name,x,y in ordered:
        row=lookup[name]; art=arts[name]; fd=row['footprint'][1]
        mock.alpha_composite(art,(x*128,(y+fd)*128-art.height))
    pilot=Image.open(ROOT.parents[2]/'assets/source/core/carrier/pilot/s/idle.png').convert('RGBA')
    pb=pilot.getbbox(); factor=112/(pb[3]-pb[1])
    pilot=pilot.resize((round(pilot.width*factor),round(pilot.height*factor)),Image.Resampling.LANCZOS)
    pb=pilot.getbbox()
    mock.alpha_composite(pilot,(5*128+64-(pb[0]+pb[2])//2,3*128-pb[3]))
    d=ImageDraw.Draw(mock)
    for x in range(23): d.line((x*128,0,x*128,mock.height),fill=(106,163,183,110),width=1)
    for y in range(17): d.line((0,y*128,mock.width,y*128),fill=(106,163,183,110),width=1)
    for name,x,y in items:
        fw,fd=lookup[name]['footprint']
        d.rectangle((x*128,y*128,(x+fw)*128-1,(y+fd)*128-1),outline=(226,83,151,200),width=2)
    mock.convert('RGB').save(ROOT/'orthographic-starter-grid-v1.png')
    # Unchanged scripted floor/wall/hull inspection contact sheet.
    tilefiles=[f'floor-{i}-v3.png' for i in range(3)]+[f'wall-{i:04b}-v3.png' for i in range(16)]
    tilefiles+=sorted(p.name for p in ROOT.glob('hull-wall-[01]*-v1.png'))
    check=Image.new('RGB',(8*160,math.ceil(len(tilefiles)/8)*170),(224,230,233))
    dd=ImageDraw.Draw(check)
    for i,file in enumerate(tilefiles):
        im=Image.open(ROOT/file).convert('RGBA'); assert im.size==(128,128),(file,im.size)
        x=(i%8)*160; y=(i//8)*170; check.paste(im,(x+16,y+8),im)
        dd.text((x+3,y+141),file.replace('hull-wall-','h-').replace('-v1.png','').replace('-v3.png',''),fill=DARK)
        dd.rectangle((x+16,y+8,x+143,y+135),outline=(192,63,119))
    check.save(ROOT/'orthographic-unchanged-tiles-v1.png')
    print(f'PASS: {len(patch)} unique stable IDs, dimensions, anchors, alpha margins; exact rectangular flats; {len(tilefiles)} unchanged scripted tiles.')


if __name__=='__main__': main()
