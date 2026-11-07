"""Deterministic SVG sources and PNG strips; no generated sheets are sliced."""
from pathlib import Path
import math, json, shutil, subprocess
P=Path(__file__).resolve().parent
CHROME='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
DARK='#241032'; TEAL='#63eddf'; WARM='#ffd071'; VIOLET='#b26bfa'; PINK='#ff83dc'
def circle(x,y,r,c,stroke=0):
 return f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{r:.2f}" fill="{c}" stroke="{DARK}" stroke-width="{stroke}"/>'
def ring(r,c,width):
 return f'<circle cx="128" cy="128" r="{r}" fill="none" stroke="{c}" stroke-width="{width}"/>'
def star(r,c,phase=0):
 pts=[]
 for k in range(16):
  a=k*math.pi/8+phase; z=r if k%2==0 else r*.33
  pts.append(f'{128+math.cos(a)*z:.2f},{128+math.sin(a)*z:.2f}')
 return f'<polygon points="{" ".join(pts)}" fill="{c}" stroke="{DARK}" stroke-width="3" stroke-linejoin="round"/>'
def debris(t,c,seed=0):
 out=''
 for k in range(7):
  a=k*math.tau/7+seed; r=18+t*72; x=128+math.cos(a)*r; y=128+math.sin(a)*r; s=9*(1-t)+2
  out+=f'<path d="M{x-s:.2f} {y:.2f} l{s:.2f} {-s:.2f} l{s*1.5:.2f} {s:.2f} l{-s:.2f} {s:.2f} Z" fill="{c}" stroke="{DARK}" stroke-width="2"/>'
 return out
spec=[]
def sheet(name,frames,ms,target,notes):
 w=len(frames)*256; h=256
 body=''.join(f'<g transform="translate({i*256} 0)">{s}</g>' for i,s in enumerate(frames))
 svg=f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">{body}</svg>'
 (P/f'{name}-v1.svg').write_text(svg)
 spec.append(dict(name=name+'-v1',frame_width=256,frame_height=256,frames=len(frames),frame_ms=ms,pivot=[128,128],suggested_world_size=target,notes=notes))
for name,c in [('hit-enemy',WARM),('hit-ship',PINK)]:
 frames=[]
 for i,r in enumerate([19,63,81,91]):
  t=i/3
  frames.append(star(r,c,i*.08)*(i<3)+circle(128,128,max(2,17-i*5),'#fff9df')*(i<3)+debris(t,c,.4))
 sheet(name,frames,35,48,'One-shot, left to right; teal/warm player damage versus pink/violet incoming.')
for name,seed in [('death-swarmer',.2),('death-spitter',.65)]:
 frames=[]
 for i in range(6):
  t=i/5; s=debris(t,VIOLET,seed)
  if i<3:s+=star([25,65,47][i],PINK,seed)+circle(128,128,[16,29,12][i],'#fff3ff')
  if i in (2,3,4):s+=ring(32+i*13,VIOLET,8-i)
  frames.append(s)
 sheet(name,frames,45,112 if 'spitter' in name else 80,'Violet shell fragments; distinct rotation/spread. No new enemy body designs; fade final frame in engine.')
sheet('shockwave-impact',[ring(r,TEAL,9-i*2)+ring(max(4,r-11),WARM,2) for i,r in enumerate([20,44,69,93])],45,160,'Centered radial hit flash; final alpha fade in engine.')
sheet('scatter-impact',[star(r,WARM,.35)+debris(i/3,TEAL,.35) for i,r in enumerate([20,48,37,12])],35,64,'Compact pellet-hit accent; no ship or enemy silhouettes.')
sheet('spitter-windup',[ring(35+i*9,VIOLET,3+i)+ring(29+i*9,PINK,2)+''.join(circle(128+math.cos(a)*r,128+math.sin(a)*r,4+i,PINK) for a,r in [(0,55+i*8),(math.pi,55+i*8)]) for i in range(4)],100,96,'Overlay centered on Spitter; four 100 ms steps = 0.4 s. Additive or alpha, cosmetic timing only.')
sheet('spitter-aim-line',['<path d="M16 128 H240" stroke="#b26bfa" stroke-width="10" opacity=".4"/><path d="M16 128 H240" stroke="#ff83dc" stroke-width="3"/>'],0,[224,12],'Stretch only UV inner rectangle x=16,y=120,w=224,h=16; draw length along +X. Straight edges tile without arrowheads; pivot at start of cropped region.')
for f in ['russo_one.ttf','OFL.txt']:shutil.copyfile(P.parents[3]/'assets/source/core/fonts'/f,P/f)
manifest={'version':1,'status':'exploratory options; not locked art direction','style':'TAKOAI-111 explicit flat battle override of current painted guide','layout':'uniform 256px authored cells, one row, transparent, no sheet slicing','assets':spec,'damage_numbers':{'font':'russo_one.ttf','license':'OFL.txt','normal':'#FFF3CC','crit_or_big':'#FFD15C','ship_damage':'#FF789E','outline':'#241032','outline_px':2,'size_world_px':{'normal':18,'crit_or_big':25,'ship_damage':20},'note':'Runtime text only. Crit additionally larger; ship damage additionally prefixed with minus sign.'}}
(P/'manifest-v1.json').write_text(json.dumps(manifest,indent=2)+'\n')
rows=[]
for i,a in enumerate(spec):
 y=95+i*170
 rows.append(f'<text x="36" y="{y}" fill="#eee4ff" font-size="21">{a["name"]} · {a["frames"]} × 256² · {a["frame_ms"]} ms/frame</text><image href="{a["name"]}.svg" x="36" y="{y+12}" width="{a["frames"]*140}" height="140"/>')
rows.append('<text x="36" y="1505" fill="#fff3cc" font-size="30" font-family="Russo One">128</text><text x="250" y="1505" fill="#ffd15c" font-size="42" font-family="Russo One">256!</text><text x="480" y="1505" fill="#ff789e" font-size="34" font-family="Russo One">−64</text>')
(P/'preview-v1.svg').write_text('<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="1560"><style>@font-face{font-family:"Russo One";src:url("russo_one.ttf")}text{font-family:sans-serif}</style><rect width="1600" height="1560" fill="#121424"/><text x="36" y="40" fill="#63eddf" font-size="26">TAKOAI-111 · Feedback pack v1 · exploratory · left → right</text>'+''.join(rows)+'</svg>')
for name in [a['name'] for a in spec]+['preview-v1']:
 import xml.etree.ElementTree as ET
 root=ET.parse(P/(name+'.svg')).getroot(); w=root.attrib['width']; h=root.attrib['height']
 result=subprocess.run([CHROME,'--headless=new','--disable-gpu','--hide-scrollbars','--no-sandbox','--force-device-scale-factor=1','--default-background-color=00000000',f'--window-size={w},{h}',f'--screenshot={P/(name+".png")}',(P/(name+'.svg')).as_uri()],capture_output=True)
 if result.returncode:raise RuntimeError(result.stderr.decode())
 print(name,w,h)
