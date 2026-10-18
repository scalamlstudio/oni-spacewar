"""Author native SVG tiles over an ImageGen painted material; render with Chrome.
No sprite-sheet slicing. All writes are in design/art/carrier-furniture.
"""
from pathlib import Path
import base64, json, subprocess, tempfile

ROOT = Path(__file__).resolve().parent
CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'

def img(name,x,y,w,h):
    p=ROOT/name
    mime='image/svg+xml' if p.suffix=='.svg' else 'image/png'
    data=base64.b64encode(p.read_bytes()).decode()
    return f'<image x="{x}" y="{y}" width="{w}" height="{h}" href="data:{mime};base64,{data}"/>'

def save(name,w,h,body):
    svg=ROOT/(name+'.svg')
    svg.write_text(f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">{body}</svg>')
    # Each render completes before the next; no service is left running.
    with tempfile.TemporaryDirectory(dir=ROOT,prefix='.berth-render-') as profile:
        subprocess.run([CHROME,'--headless=new','--disable-gpu','--no-sandbox','--hide-scrollbars',
            '--force-device-scale-factor=1','--default-background-color=00000000',
            '--disable-dev-shm-usage','--user-data-dir='+profile,f'--window-size={w},{h+200}',
            '--screenshot='+str(ROOT/(name+'.png')),svg.as_uri()],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)

def line(x1,y1,x2,y2,color,width):
    return f'<path d="M{x1} {y1}L{x2} {y2}" fill="none" stroke="{color}" stroke-width="{width}"/>'

pieces={'fill':'','edge-n':'n','edge-e':'e','edge-s':'s','edge-w':'w',
        'corner-ne':'ne','corner-se':'se','corner-sw':'sw','corner-nw':'nw'}
rows=[]
for name,edges in pieces.items():
    body='<rect width="128" height="128" fill="#25494d"/>'
    body+=img('berth-floor-material-v1.png',3,3,122,122)
    body+='<path d="M3 124V3H124" fill="none" stroke="#92b4ac" stroke-width="1.5" opacity=".8"/>'
    body+='<path d="M4 125H125V4" fill="none" stroke="#183b40" stroke-width="2"/>'
    body+='<path d="M8 28V8H28 M100 8H120V28 M8 100V120H28 M100 120H120V100" fill="none" stroke="#356168" stroke-width="2"/>'
    for edge in edges:
        angle={'n':0,'e':90,'s':180,'w':270}[edge]
        trim='<path d="M0 9H128" stroke="#142d34" stroke-width="18"/>'
        trim+='<path d="M0 4H128 M0 15H128" stroke="#779b96" stroke-width="1.5"/>'
        trim+='<rect x="49" y="6" width="30" height="5" rx="1" fill="#ffcf73" stroke="#9e6736" stroke-width="1"/>'
        body+=f'<g transform="rotate({angle} 64 64)">{trim}</g>'
    if len(edges)==2:
        angle={'nw':0,'ne':90,'se':180,'sw':270}[edges]
        body+=f'<g transform="rotate({angle} 64 64)"><path d="M22 44V22H44" fill="none" stroke="#e9bd6b" stroke-width="5"/><path d="M22 20L40 20L20 40Z" fill="#f6d58c"/></g>'
    filename='berth-floor-'+name+'-v1'
    save(filename,128,128,body)
    rows.append({'piece':name,'file':filename+'.png','size':[128,128],'edges':edges})

ring='<circle cx="128" cy="128" r="92" fill="none" stroke="#e8b862" stroke-width="5" stroke-dasharray="135 9"/>'
for angle in (45,135,225,315):
    ring+=f'<g transform="rotate({angle} 128 128)">{line(128,14,128,25,"#f5cb7b",4)}</g>'
save('berth-floor-marking-2x2-v1',256,256,ring)

def pad(x,y):
    out=''
    for yy in range(4):
        for xx in range(4):
            edges=('n' if yy==0 else 's' if yy==3 else '')+('w' if xx==0 else 'e' if xx==3 else '')
            # corners use clockwise naming.
            edges={'en':'ne','sn':'ns','es':'se','ws':'sw'}.get(edges,edges)
            name='fill' if not edges else ('corner-' if len(edges)==2 else 'edge-')+edges
            out+=img('berth-floor-'+name+'-v1.svg',x+xx*128,y+yy*128,128,128)
    return out+img('berth-floor-marking-2x2-v1.svg',x+128,y+128,256,256)

save('berth-floor-comparison-v1',1120,620,
    '<rect width="1120" height="620" fill="#dfe6e9"/><g font-family="sans-serif" font-size="20" fill="#233b43"><text x="24" y="30">Original painted berth sprite</text><text x="580" y="30">Flat 4×4 floor tiles · exploration v1</text></g>'+
    img('berth-v1.png',24,55,512,544)+pad(580,70))

# Match the existing starting-layout coordinates. Keep original painted items.
rooms=[(0,0,7,5),(7,0,7,5),(14,0,7,5),(0,5,14,10),(14,5,7,5)]
body='<rect width="2816" height="2048" fill="#0e1120"/>'
walls=set(); floors=set()
for x,y,w,h in rooms:
    for yy in range(y,y+h+1):
        for xx in range(x,x+w+1):
            (walls if xx in (x,x+w) or yy in (y,y+h) else floors).add((xx,yy))
doors={(7,2):'ew',(7,3):'ew',(14,2):'ew',(14,3):'ew',(3,5):'ns',(4,5):'ns',(10,5):'ns',(11,5):'ns',(17,5):'ns',(18,5):'ns',(14,7):'ew',(14,8):'ew'}
walls -= doors.keys()
floors |= doors.keys()
for x,y in floors|walls:
    bits=''.join(str(int(q in walls)) for q in [(x,y-1),(x+1,y),(x,y+1),(x-1,y)])
    file=f'wall-{bits}-v3.png' if (x,y) in walls else f'floor-{(x+y)%3}-v3.png'
    if (x,y) in doors:file=f'doorway-{doors[x,y]}-v4.png'
    body+=img(file,x*128,y*128,128,128)
body+=pad(2*128,8*128)+pad(8*128,8*128)
items=[('wall-light-v1.png',6,5,1,1,0),('wall-light-v1.png',13,5,1,1,0),('tool-rack-v1.png',20,5,1,1,0),('potted-plant-v1.png',15,1,1,1,0),('command-console-v2.png',3,1,2,1,160),('wall-screen-v1.png',5,0,1,1,0),('potted-plant-v1.png',1,1,1,1,0),('floor-lamp-v1.png',6,1,1,1,0),('bunk-bed-v1.png',9,1,1,2,0),('bunk-bed-v1.png',10,1,1,2,0),('locker-v2.png',12,1,1,1,0),('cat-bed-v1.png',13,1,1,1,0),('rug-v1.png',10,3,2,2,0),('table-v1.png',10,3,2,1,0),('sample-analyzer-v1.png',17,1,2,1,160),('hydro-planter-v1.png',19,1,2,1,0),('upgrade-bench-v1.png',15,6,2,1,96),('crate-stack-v2.png',19,9,1,1,0),('crate-stack-v2.png',20,9,1,1,0)]
# Read PNG dimensions with the standard library; preserve original whole sprites.
import struct
for file,x,y,fw,fd,head in items:
    if not (ROOT/file).exists():continue
    w,h=struct.unpack('>II',(ROOT/file).read_bytes()[16:24])
    body+=img(file,x*128,(y+fd)*128-h,w,h)
for x in range(23):body+=line(x*128,0,x*128,2048,'#78bdd0',1)
for y in range(17):body+=line(0,y*128,2816,y*128,'#78bdd0',1)
save('berth-floor-starter-grid-v1',2816,2048,body)
(ROOT/'berth-floor-import-v1.json').write_text(json.dumps({'tile_px':128,'pieces':rows,'marking':{'file':'berth-floor-marking-2x2-v1.png','size':[256,256],'offset_cells':[1,1]},'example_footprint':[4,4],'starting_origins':[[2,8],[8,8]],'layer':'floor','status':'exploratory'},indent=2)+'\n')
subprocess.run(['swift',str(ROOT/'crop-berth-renders-v1.swift'),str(ROOT)],check=True)
print('Rendered nine tiles, ring overlay, comparison and starting-grid mock.')
