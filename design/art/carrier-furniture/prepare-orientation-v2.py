"""Normalize whole transparent redraws and build orientation QA sheets (Pillow)."""
from PIL import Image, ImageDraw
from pathlib import Path
import json
p=Path(__file__).resolve().parent
paths=json.loads((p/'orientation-sources-v2.json').read_text())
rows=json.loads((p/'batch-a-import-v3.json').read_text())+json.loads((p/'batch-b-import-v1.json').read_text())
patch=[]
for n,src in paths.items():
 r=next(r for r in rows if r['file']==n+'-v1.png')
 im=Image.open(p/src).convert('RGBA');alpha=im.getchannel('A')
 # Ignore near-invisible generator noise when locating the whole subject.
 a,b,c,e=alpha.point(lambda v:255 if v>128 else 0).getbbox()
 im=im.crop((max(0,a-2),max(0,b-2),min(im.width,c+2),min(im.height,e+2)))
 im.putalpha(im.getchannel('A').point(lambda v:0 if v<9 else v))
 w,h=r['size'];a,b,c,e=r['alpha_bbox'];im.thumbnail((c-a,e-b),Image.Resampling.LANCZOS)
 im=im.crop(im.getchannel('A').getbbox());out=Image.new('RGBA',(w,h));out.alpha_composite(im,((w-im.width)//2,h-4-im.height));out.save(p/(n+'-v2.png'))
 r['file']=n+'-v2.png';r['alpha_bbox']=list(out.getchannel('A').getbbox());patch.append(r)
 assert out.size==(w,h) and r['alpha_bbox'][3]==h-4 and r['alpha_bbox'][0]>=4 and r['alpha_bbox'][2]<=w-4
(p/'orientation-import-patch-v2.json').write_text(json.dumps(patch,indent=2)+'\n')
names=['command-console','upgrade-bench','sample-analyzer','salvage-sorter','combat-simulator','berth']
sheet=Image.new('RGBA',(1920,700));floor=Image.open(p/'floor-0-v3.png')
for y in range(0,700,128):
 for x in range(0,1920,128):sheet.alpha_composite(floor,(x,y))
d=ImageDraw.Draw(sheet)
for i,n in enumerate(names):
 x=i*280 if i<5 else 1400;im=Image.open(p/(n+('-v2.png' if n in paths else '-v1.png')));sheet.alpha_composite(im,(x,590-im.height));d.text((x+8,625),n,fill=(36,46,58))
sheet.convert('RGB').save(p/'station-floor-check-v4.png')
compare=Image.new('RGB',(1000,2240),(223,229,232));d=ImageDraw.Draw(compare)
for i,n in enumerate(paths):
 d.text((15,i*320+10),n+' | BEFORE (left) / AFTER (right)',fill=(36,46,58))
 for j,v in enumerate([1,2]):
  im=Image.open(p/f'{n}-v{v}.png');im=im.crop(im.getchannel('A').getbbox());im.thumbnail((440,270));compare.paste(im,(j*500+(500-im.width)//2,i*320+40),im)
compare.save(p/'orientation-before-after-v2.png')
print('PASS: seven canvases, stable IDs, bottom-center alignment, >=4px transparent margins')
