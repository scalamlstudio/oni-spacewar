from pathlib import Path
import shutil, struct, json, base64, html
P=Path("design/art/enemies")
P.mkdir(parents=True,exist_ok=True)
names=["lancer","bloater","brood","warden","fissure-guardian","elite-affixes","elite-effects"]
orders={
"lancer":["move","windup","dash-smear","charge-line","death-burst"],
"bloater":["move","swelling-with-ring","warning-ring","explosion","death-burst"],
"brood":["idle","sac-windup","pulsing-sac","death-burst"],
"warden":["idle","tether","shield-shimmer","death-burst"],
"fissure-guardian":["phase-1","phase-2","shot-ring-flash","charge-line","death-burst"],
"elite-affixes":["Swift","Volatile","Warding","Blinking","Splitting","Scorching","Chilling"],
"elite-effects":["aura-option-1","aura-option-2","aura-option-3","burning-patch","teleport-mark"]}
assets=[]
rows=[]
for name in names:
 dst=P/(name+"-source-v1.png")
 data=dst.read_bytes(); width,height=struct.unpack(">II",data[16:24])
 assets.append(dict(file=dst.name,sheet_size_px=[width,height],items_left_to_right=orders[name],layout="one row; semantic cutouts, NOT equal-width grid frames"))
 rows.append('<section><h2>'+html.escape(name)+' — '+str(width)+' × '+str(height)+'</h2><img src="data:image/png;base64,'+base64.b64encode(data).decode()+'"><p>'+html.escape(" → ".join(orders[name]))+'</p></section>')
manifest=dict(version=1,issue="TAKOAI-114",status="exploratory options; no final art direction",assets=assets,cutout="Use the asset pipeline cutout tool. Keep disconnected particles grouped per effect. Do not slice equal-width strips. Remove white including ring interiors; preserve white creature highlights.",suggested_output_frames_px=dict(ordinary_enemy=[256,256],guardian=[512,512],death_effect=[256,256],guardian_death=[512,512],affix_icon=[64,64],aura=[256,256],ground_patch=[256,256],charge_line=[512,64],tether=[256,32]),suggested_world_render_px=dict(ordinary_enemy=56,guardian=140,affix_icon=24,bloater_explosion_diameter=180),timing_from_design=dict(lancer_warning_s=0.7,bloater_warning_s=1.0,blinking_warning_s=0.5),notes=["Output sizes are import suggestions, not existing animation cells.","Bloater 90 px AoE treated as radius for art scale; game simulation remains authoritative.","Animate pulse/scale/opacity in the client; these are key poses, not full frame-by-frame animations."])
(P/"manifest-v1.json").write_text(json.dumps(manifest,indent=2)+"\n")
(P/"preview-v1.html").write_text('<!doctype html><meta charset="utf-8"><style>*{box-sizing:border-box}body{margin:0;padding:24px;background:#171224;color:#efe6ff;font:16px system-ui}h1{font-size:24px;margin:0 0 8px}h2{font-size:17px;margin:0 0 6px}section{margin:12px 0}img{width:100%;display:block}p{margin:5px 0;font-size:12px;color:#c8b6df}</style><h1>TAKOAI-114 · Enemy art options v1</h1><p>Exploratory source sheets · semantic cutout required · pose order left to right</p>'+''.join(rows))
print(json.dumps(assets,indent=2))

