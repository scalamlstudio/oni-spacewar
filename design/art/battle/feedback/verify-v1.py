"""Check Chromium PNG dimensions, nonempty frames, and transparent gutters."""
from pathlib import Path
import json, struct, zlib
p=Path(__file__).resolve().parent
m=json.loads((p/'manifest-v1.json').read_text())
for a in m['assets']:
 raw=(p/(a['name']+'.png')).read_bytes(); pos=8; data=b''
 while pos<len(raw):
  n=struct.unpack('>I',raw[pos:pos+4])[0]; typ=raw[pos+4:pos+8]; chunk=raw[pos+8:pos+8+n]; pos+=12+n
  if typ==b'IHDR':w,h,depth,color,_,_,inter=struct.unpack('>IIBBBBB',chunk)
  if typ==b'IDAT':data+=chunk
 assert (w,h,depth,color,inter)==(256*a['frames'],256,8,6,0),(a['name'],w,h,depth,color,inter)
 unpack=zlib.decompress(data); stride=w*4; last=bytearray(stride); rows=[]; off=0
 def paeth(x,y,z):
  q=x+y-z; ds=[abs(q-x),abs(q-y),abs(q-z)]; return [x,y,z][ds.index(min(ds))]
 for y in range(h):
  filt=unpack[off]; off+=1; row=bytearray(unpack[off:off+stride]); off+=stride
  for x in range(stride):
   left=row[x-4] if x>=4 else 0; up=last[x]; ul=last[x-4] if x>=4 else 0
   pred=[0,left,up,(left+up)//2,paeth(left,up,ul)][filt]
   row[x]=(row[x]+pred)&255
  rows.append(row); last=row
 for f in range(a['frames']):
  points=[(x,y) for y,row in enumerate(rows) for x in range(256) if row[(f*256+x)*4+3]>0]
  assert points,(a['name'],f,'empty')
  box=[min(x for x,y in points),min(y for x,y in points),max(x for x,y in points),max(y for x,y in points)]
  assert min(box[:2])>=8 and max(box[2:])<=247,(a['name'],f,'clipping',box)
 print(a['name'],f'{w}x{h}',a['frames'],'frames: nonempty, alpha, >=8px margins OK')
