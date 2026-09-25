"""Generates the exploratory concept-art SVG sheets in this folder.

Run: python3 design/concept-art/generate.py
Then render PNGs with design/concept-art/render.sh (headless Chrome).
"""
import math
import os
import random

HERE = os.path.dirname(os.path.abspath(__file__))
W, H = 1600, 1000

# Palette (see sheet 4)
P = {
    "space": "#0b1026", "space2": "#151c3d", "void": "#3a1d5c", "voidglow": "#c63fd6",
    "oni": "#3fd0c9", "amber": "#ffb347", "hull": "#6d7c93", "hulldk": "#3c4659",
    "hulllt": "#a9b6c8", "cream": "#f3e7d3", "ginger": "#e38b3f", "charcoal": "#3b3b48",
    "snow": "#e9eef3", "ink": "#1b1f2e", "paper": "#f6f1e7", "red": "#ff5b5b",
}
FONT = "font-family='Helvetica Neue, Helvetica, Arial, sans-serif'"


def svg(body, bg):
    return (f"<svg xmlns='http://www.w3.org/2000/svg' width='{W}' height='{H}' "
            f"viewBox='0 0 {W} {H}'>\n<rect width='{W}' height='{H}' fill='{bg}'/>\n"
            f"{body}\n</svg>\n")


def text(x, y, s, size=18, fill=P["ink"], weight="normal", anchor="start", opacity=1):
    s = s.replace("&", "&amp;").replace("&amp;lt;", "&lt;")
    return (f"<text x='{x}' y='{y}' {FONT} font-size='{size}' fill='{fill}' "
            f"font-weight='{weight}' text-anchor='{anchor}' opacity='{opacity}'>{s}</text>")


def header(title, sub, dark=False):
    c = P["snow"] if dark else P["ink"]
    return (text(50, 62, title, 38, c, "bold") + text(50, 94, sub, 18, c, opacity=0.7)
            + text(W - 50, 62, "ONI SPACEWAR · exploratory concept · not a decided art direction",
                   14, c, anchor="end", opacity=0.55))


def stars(n, seed, x0=0, y0=0, w=W, h=H):
    r = random.Random(seed)
    out = []
    for _ in range(n):
        x, y = x0 + r.random() * w, y0 + r.random() * h
        rad = r.choice([0.6, 0.8, 1, 1, 1.4, 2])
        out.append(f"<circle cx='{x:.1f}' cy='{y:.1f}' r='{rad}' fill='#fff' opacity='{r.uniform(.3, .9):.2f}'/>")
    return "".join(out)


# ---------------------------------------------------------------- Oni (cats)

def oni(x, y, s=1.0, fur="#e38b3f", belly="#f3e7d3", suit="#3fd0c9", suit2="#2a8f8a",
        ears="pointy", eyes="#ffb347", accessory="", stripe=None, tail="curl", flip=False):
    """A chubby, upright cat-like Oni. Origin = feet centre."""
    sx = -s if flip else s
    g = [f"<g transform='translate({x},{y}) scale({sx},{s})'>"]
    # tail
    if tail == "curl":
        g.append(f"<path d='M40,-60 C110,-60 120,-150 80,-170 C60,-180 55,-160 70,-150 C95,-135 85,-85 40,-85' fill='{fur}' stroke='{P['ink']}' stroke-width='4'/>")
    elif tail == "long":
        g.append(f"<path d='M40,-50 C120,-40 140,-120 150,-190' fill='none' stroke='{P['ink']}' stroke-width='22' stroke-linecap='round'/>"
                 f"<path d='M40,-50 C120,-40 140,-120 150,-190' fill='none' stroke='{fur}' stroke-width='14' stroke-linecap='round'/>")
    elif tail == "stub":
        g.append(f"<ellipse cx='55' cy='-60' rx='22' ry='18' fill='{fur}' stroke='{P['ink']}' stroke-width='4'/>")
    # feet
    for fx in (-30, 30):
        g.append(f"<ellipse cx='{fx}' cy='-8' rx='26' ry='14' fill='{suit2}' stroke='{P['ink']}' stroke-width='4'/>")
    # body (suit)
    g.append(f"<path d='M-55,-15 C-70,-90 -50,-150 0,-150 C50,-150 70,-90 55,-15 Z' fill='{suit}' stroke='{P['ink']}' stroke-width='4'/>")
    g.append(f"<path d='M-28,-20 C-35,-80 -20,-120 0,-120 C20,-120 35,-80 28,-20 Z' fill='{belly}' opacity='.9'/>")
    g.append(f"<path d='M-55,-40 L55,-40' stroke='{suit2}' stroke-width='8'/>")
    # arms
    g.append(f"<ellipse cx='-58' cy='-85' rx='16' ry='30' fill='{suit}' stroke='{P['ink']}' stroke-width='4' transform='rotate(15 -58 -85)'/>")
    g.append(f"<ellipse cx='58' cy='-85' rx='16' ry='30' fill='{suit}' stroke='{P['ink']}' stroke-width='4' transform='rotate(-15 58 -85)'/>")
    g.append(f"<circle cx='-64' cy='-58' r='12' fill='{fur}' stroke='{P['ink']}' stroke-width='4'/>")
    g.append(f"<circle cx='64' cy='-58' r='12' fill='{fur}' stroke='{P['ink']}' stroke-width='4'/>")
    # head
    hy = -200
    if ears == "pointy":
        e = f"M-60,{hy-10} L-52,{hy-95} L-12,{hy-50} Z"
        ei = f"M-50,{hy-25} L-47,{hy-75} L-24,{hy-50} Z"
    elif ears == "round":
        e = f"M-66,{hy-20} C-75,{hy-90} -25,{hy-90} -18,{hy-50} Z"
        ei = f"M-56,{hy-30} C-60,{hy-72} -32,{hy-72} -28,{hy-50} Z"
    elif ears == "tall":
        e = f"M-50,{hy-20} L-40,{hy-130} L-10,{hy-50} Z"
        ei = f"M-42,{hy-35} L-36,{hy-105} L-20,{hy-52} Z"
    else:  # folded
        e = f"M-66,{hy-30} C-62,{hy-80} -20,{hy-75} -15,{hy-50} C-35,{hy-55} -55,{hy-45} -66,{hy-30} Z"
        ei = ""
    for m in (1, -1):
        g.append(f"<g transform='scale({m},1)'><path d='{e}' fill='{fur}' stroke='{P['ink']}' stroke-width='4' stroke-linejoin='round'/>"
                 + (f"<path d='{ei}' fill='#f2a7b0'/>" if ei else "") + "</g>")
    g.append(f"<ellipse cx='0' cy='{hy}' rx='74' ry='60' fill='{fur}' stroke='{P['ink']}' stroke-width='4'/>")
    if stripe:
        for dx in (-14, 0, 14):
            g.append(f"<path d='M{dx},{hy-58} L{dx*0.7},{hy-35}' stroke='{stripe}' stroke-width='7' stroke-linecap='round'/>")
        for m in (1, -1):
            g.append(f"<path d='M{m*72},{hy-5} L{m*52},{hy} M{m*70},{hy+14} L{m*52},{hy+12}' stroke='{stripe}' stroke-width='6' stroke-linecap='round'/>")
    g.append(f"<ellipse cx='0' cy='{hy+25}' rx='38' ry='24' fill='{belly}'/>")
    # eyes
    for ex in (-30, 30):
        g.append(f"<ellipse cx='{ex}' cy='{hy-5}' rx='15' ry='18' fill='{eyes}' stroke='{P['ink']}' stroke-width='3'/>"
                 f"<ellipse cx='{ex}' cy='{hy-3}' rx='5' ry='13' fill='{P['ink']}'/>"
                 f"<circle cx='{ex+5}' cy='{hy-11}' r='4' fill='#fff'/>")
    g.append(f"<path d='M-8,{hy+14} L8,{hy+14} L0,{hy+23} Z' fill='#e0707e'/>")
    g.append(f"<path d='M-14,{hy+30} Q-7,{hy+37} 0,{hy+26} Q7,{hy+37} 14,{hy+30}' fill='none' stroke='{P['ink']}' stroke-width='3' stroke-linecap='round'/>")
    for m in (1, -1):
        g.append(f"<path d='M{m*40},{hy+22} L{m*95},{hy+14} M{m*40},{hy+30} L{m*95},{hy+34}' stroke='{P['ink']}' stroke-width='2' opacity='.6'/>")
    g.append(accessory)
    g.append("</g>")
    return "".join(g)


def acc_pilot():
    hy = -200
    return (f"<path d='M-78,{hy-30} C-70,{hy-70} 70,{hy-70} 78,{hy-30}' fill='none' stroke='{P['hulldk']}' stroke-width='10'/>"
            f"<rect x='-66' y='{hy-48}' width='56' height='30' rx='12' fill='{P['oni']}' fill-opacity='.55' stroke='{P['ink']}' stroke-width='4'/>"
            f"<rect x='10' y='{hy-48}' width='56' height='30' rx='12' fill='{P['oni']}' fill-opacity='.55' stroke='{P['ink']}' stroke-width='4'/>"
            f"<path d='M-10,{hy-33} L10,{hy-33}' stroke='{P['ink']}' stroke-width='5'/>"
            f"<path d='M-30,-150 L-10,-120 L0,-138 L10,-120 L30,-150' fill='{P['amber']}' stroke='{P['ink']}' stroke-width='3'/>"
            f"<circle cx='-38' cy='-100' r='9' fill='{P['amber']}' stroke='{P['ink']}' stroke-width='3'/>")


def acc_engineer():
    return (f"<rect x='-58' y='-50' width='116' height='16' fill='#6b4a2e' stroke='{P['ink']}' stroke-width='3'/>"
            f"<rect x='-40' y='-56' width='18' height='26' fill='#8a6a44' stroke='{P['ink']}' stroke-width='3'/>"
            f"<rect x='22' y='-56' width='18' height='26' fill='#8a6a44' stroke='{P['ink']}' stroke-width='3'/>"
            # wrench in right paw
            f"<g transform='translate(70,-60) rotate(-35)'><rect x='-5' y='-70' width='10' height='70' fill='{P['hulllt']}' stroke='{P['ink']}' stroke-width='3'/>"
            f"<path d='M-16,-70 L-16,-92 L-6,-86 L-6,-76 L6,-76 L6,-86 L16,-92 L16,-70 Z' fill='{P['hulllt']}' stroke='{P['ink']}' stroke-width='3'/></g>"
            f"<path d='M-70,-238 C-60,-275 60,-275 70,-238 Z' fill='{P['amber']}' stroke='{P['ink']}' stroke-width='4'/>"
            f"<rect x='-80' y='-243' width='160' height='12' rx='6' fill='{P['amber']}' stroke='{P['ink']}' stroke-width='4'/>")


def acc_researcher():
    hy = -200
    return (f"<circle cx='-30' cy='{hy-5}' r='22' fill='none' stroke='{P['ink']}' stroke-width='4'/>"
            f"<circle cx='30' cy='{hy-5}' r='22' fill='none' stroke='{P['ink']}' stroke-width='4'/>"
            f"<path d='M-8,{hy-8} L8,{hy-8}' stroke='{P['ink']}' stroke-width='4'/>"
            # holo tablet
            f"<g transform='translate(-92,-110)'><path d='M0,0 L-50,-60 L10,-80 L40,-15 Z' fill='{P['oni']}' opacity='.25'/>"
            f"<rect x='-20' y='-10' width='40' height='28' rx='4' fill='{P['hulldk']}' stroke='{P['ink']}' stroke-width='3'/>"
            f"<path d='M-30,-45 L0,-55 M-25,-35 L10,-45 M-18,-25 L18,-33' stroke='{P['oni']}' stroke-width='3'/></g>"
            f"<path d='M-48,-148 L0,-110 L48,-148' fill='none' stroke='#fff' stroke-width='6' opacity='.8'/>")


def acc_gunner():
    return (f"<path d='M-60,-150 C-80,-130 -78,-95 -68,-80 L-40,-95 L-40,-145 Z' fill='{P['hull']}' stroke='{P['ink']}' stroke-width='4'/>"
            f"<path d='M60,-150 C80,-130 78,-95 68,-80 L40,-95 L40,-145 Z' fill='{P['hull']}' stroke='{P['ink']}' stroke-width='4'/>"
            f"<path d='M-20,-150 L20,-150 L14,-120 L-14,-120 Z' fill='{P['red']}' stroke='{P['ink']}' stroke-width='3'/>"
            f"<path d='M-45,-258 L45,-258' stroke='{P['red']}' stroke-width='10' stroke-linecap='round'/>"
            f"<path d='M-20,-250 L-35,-290 M0,-252 L0,-298 M20,-250 L35,-290' stroke='{P['ink']}' stroke-width='0'/>")


def sheet_crew():
    b = [header("Sheet 1 — Oni crew lineup", "Cat-like Oni: the pilot (main character) and three companions, 2–3 of which crew the carrier")]
    # floor band
    b.append(f"<rect x='0' y='640' width='{W}' height='6' fill='{P['ink']}' opacity='.08'/>")
    crew = [
        (230, "PILOT — main character", "Flies the battleship; triggers Q/W/E/R by hand",
         dict(fur=P["ginger"], stripe="#b3601f", suit=P["oni"], suit2="#228a84", ears="pointy", accessory=acc_pilot())),
        (600, "ENGINEER — companion", "Fix / build / haul; auto-repair skill when HP low",
         dict(fur=P["charcoal"], belly="#d9d4cc", suit="#e0a040", suit2="#a8702a", ears="round", eyes="#8be38b", tail="long", accessory=acc_engineer())),
        (970, "RESEARCHER — companion", "Lab work: samples & tech documents",
         dict(fur=P["cream"], belly="#ffffff", suit="#8c7ae6", suit2="#5f50b3", ears="tall", eyes="#6fb7ff", tail="curl", accessory=acc_researcher())),
        (1340, "GUNNER — companion", "Auto-fires damage skill when enemy in range",
         dict(fur="#9a9aa8", belly=P["snow"], suit=P["hulldk"], suit2="#252c3a", ears="folded", eyes=P["amber"], stripe="#6c6c7c", tail="stub", accessory=acc_gunner())),
    ]
    for x, name, role, kw in crew:
        b.append(f"<ellipse cx='{x}' cy='640' rx='110' ry='16' fill='{P['ink']}' opacity='.12'/>")
        b.append(oni(x, 632, 1.25, **kw))
        b.append(text(x, 690, name, 20, P["ink"], "bold", "middle"))
        b.append(text(x, 716, role, 15, P["ink"], anchor="middle", opacity=.7))
    # silhouette variants
    b.append(text(50, 790, "Silhouette variants — ear / tail / build are the main readable differences at gameplay scale", 18, P["ink"], "bold"))
    vs = [("pointy", "curl"), ("round", "long"), ("tall", "curl"), ("folded", "stub"), ("pointy", "long"), ("round", "stub")]
    for i, (e, t) in enumerate(vs):
        b.append(oni(110 + i * 150, 975, .55, fur=P["ink"], belly=P["ink"], suit=P["ink"], suit2=P["ink"], eyes=P["ink"], ears=e, tail=t))
    # tiny gameplay-scale sprites
    b.append(text(1000, 830, "At carrier-interior scale (~32 px):", 15, P["ink"], opacity=.7))
    for i, (_, _, _, kw) in enumerate(crew):
        kw2 = dict(kw); kw2["accessory"] = ""
        b.append(oni(1030 + i * 60, 900, .22, **kw2))
    # palette
    b.append(text(1300, 830, "Crew palette", 15, P["ink"], opacity=.7))
    sw = [P["ginger"], P["charcoal"], P["cream"], "#9a9aa8", P["oni"], "#e0a040", "#8c7ae6", P["hulldk"]]
    for i, c in enumerate(sw):
        b.append(f"<rect x='{1300 + (i % 4) * 62}' y='{845 + (i // 4) * 62}' width='52' height='52' rx='8' fill='{c}' stroke='{P['ink']}' stroke-width='2'/>")
    b.append(text(1300, 985, "fur tones (top) · suit = role colour (bottom)", 13, P["ink"], opacity=.6))
    return svg("".join(b), P["paper"])


# ---------------------------------------------------------------- Carrier

def sheet_carrier():
    b = [f"<defs><linearGradient id='bg' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='{P['space']}'/><stop offset='1' stop-color='{P['space2']}'/></linearGradient>"
         f"<radialGradient id='eng' cx='.5' cy='.5' r='.5'><stop offset='0' stop-color='#fff'/><stop offset='.4' stop-color='{P['oni']}'/><stop offset='1' stop-color='{P['oni']}' stop-opacity='0'/></radialGradient>"
         f"<pattern id='grid' width='24' height='24' patternUnits='userSpaceOnUse'><rect width='24' height='24' fill='#2a3246'/><path d='M24,0 L0,0 0,24' fill='none' stroke='#39435b' stroke-width='1'/></pattern></defs>",
         f"<rect width='{W}' height='{H}' fill='url(#bg)'/>", stars(180, 2, 0, 0, W, 420),
         header("Sheet 2 — The carrier", "Persistent base. Exterior side view (top) and Mindustry-style interior, bounded by the hull (bottom)", dark=True)]
    # ---- exterior side view
    ox, oy = 180, 280
    b.append(f"<ellipse cx='{ox-40}' cy='{oy}' rx='90' ry='60' fill='url(#eng)'/>")
    b.append(f"<ellipse cx='{ox-20}' cy='{oy+50}' rx='50' ry='30' fill='url(#eng)'/>")
    # wormhole drive ring
    b.append(f"<ellipse cx='{ox+60}' cy='{oy}' rx='30' ry='110' fill='none' stroke='{P['voidglow']}' stroke-width='14' opacity='.85'/>")
    b.append(f"<ellipse cx='{ox+60}' cy='{oy}' rx='30' ry='110' fill='none' stroke='#fff' stroke-width='3' opacity='.6'/>")
    hull = (f"M{ox},{oy-50} L{ox+180},{oy-70} L{ox+260},{oy-100} L{ox+420},{oy-100} L{ox+460},{oy-70} L{ox+900},{oy-60} "
            f"L{ox+1100},{oy-30} L{ox+1230},{oy+10} L{ox+1100},{oy+50} L{ox+700},{oy+80} L{ox+200},{oy+80} L{ox},{oy+60} Z")
    b.append(f"<path d='{hull}' fill='{P['hull']}' stroke='{P['ink']}' stroke-width='4'/>")
    b.append(f"<path d='M{ox},{oy+20} L{ox+1150},{oy+30}' stroke='{P['hulldk']}' stroke-width='6'/>")
    # module segments
    segs = [(ox + 230, "DOCKING"), (ox + 480, "MANUFACTURING"), (ox + 730, "LAB"), (ox + 930, "(future module)")]
    for i, (sx, lab) in enumerate(segs):
        b.append(f"<path d='M{sx},{oy-62} L{sx},{oy+80}' stroke='{P['hulldk']}' stroke-width='4'/>")
        b.append(text(sx + 110, oy + 120, lab, 14, P["hulllt"], "bold", "middle", .8))
    # windows
    for i in range(30):
        b.append(f"<rect x='{ox+250+i*28}' y='{oy-40}' width='14' height='8' fill='{P['amber']}' opacity='{.9 if i%3 else .35}'/>")
    # docking bay opening + battleship
    b.append(f"<rect x='{ox+260}' y='{oy+5}' width='190' height='60' fill='#10141f' stroke='{P['ink']}' stroke-width='3'/>")
    b.append(f"<path d='M{ox+300},{oy+45} L{ox+400},{oy+45} L{ox+430},{oy+35} L{ox+400},{oy+25} L{ox+320},{oy+25} Z' fill='{P['hulllt']}' stroke='{P['ink']}' stroke-width='2'/>")
    b.append(f"<path d='M{ox+260},{oy+65} L{ox+450},{oy+65}' stroke='{P['oni']}' stroke-width='3'/>")
    # bridge tower
    b.append(f"<path d='M{ox+980},{oy-60} L{ox+1010},{oy-120} L{ox+1110},{oy-120} L{ox+1130},{oy-35} Z' fill='{P['hulllt']}' stroke='{P['ink']}' stroke-width='4'/>")
    b.append(f"<path d='M{ox+1025},{oy-108} L{ox+1100},{oy-108} L{ox+1108},{oy-90} L{ox+1020},{oy-90} Z' fill='{P['oni']}' opacity='.8'/>")
    # antenna / cat-ear fins
    b.append(f"<path d='M{ox+1030},{oy-120} L{ox+1045},{oy-165} L{ox+1060},{oy-120} Z M{ox+1070},{oy-120} L{ox+1085},{oy-165} L{ox+1100},{oy-120} Z' fill='{P['ginger']}' stroke='{P['ink']}' stroke-width='3'/>")
    b.append(text(ox + 1065, oy - 180, "‘ear’ sensor fins", 13, P["amber"], anchor="middle", opacity=.9))
    b.append(text(ox + 60, oy - 130, "wormhole drive ring", 13, P["voidglow"], anchor="middle"))
    # damage (semi-functional)
    b.append(f"<path d='M{ox+600},{oy+80} L{ox+620},{oy+55} L{ox+640},{oy+72} L{ox+665},{oy+48} L{ox+690},{oy+80} Z' fill='#10141f'/>")
    b.append(text(ox + 645, oy + 150, "hull breach — to repair (opening)", 13, P["red"], anchor="middle"))

    # ---- interior cutaway (top-down grid)
    iy = 470
    b.append(text(50, iy - 8, "Interior (top-down build view)", 18, P["snow"], "bold"))
    interior = (f"M120,{iy+30} L1100,{iy+30} L1380,{iy+140} L1480,{iy+250} L1380,{iy+360} L1100,{iy+470} L120,{iy+470} L70,{iy+400} L70,{iy+100} Z")
    b.append(f"<path d='{interior}' fill='#1c2233' stroke='{P['hulllt']}' stroke-width='10' stroke-linejoin='round'/>")
    b.append(f"<clipPath id='ic'><path d='{interior}'/></clipPath>")
    # module rooms
    rooms = [(100, 380, "DOCKING MODULE", P["oni"], True), (500, 350, "MANUFACTURING MODULE", P["amber"], True),
             (870, 250, "LAB MODULE", "#8c7ae6", True), (1140, 330, "LOCKED — adds area when unlocked", P["hulllt"], False)]
    for rx, rw, lab, col, on in rooms:
        if on:
            b.append(f"<g clip-path='url(#ic)'><rect x='{rx}' y='{iy+40}' width='{rw-10}' height='420' fill='url(#grid)'/></g>")
        else:
            b.append(f"<g clip-path='url(#ic)'><rect x='{rx}' y='{iy+40}' width='{rw}' height='420' fill='#141926'/>"
                     f"<rect x='{rx}' y='{iy+40}' width='{rw}' height='420' fill='none' stroke='{P['hulllt']}' stroke-dasharray='10 8' stroke-width='3' opacity='.5'/></g>")
        b.append(text(rx + (14 if on else 30), iy + 64, lab, 14, col, "bold"))
    # docking: battleship top-down on pad + crane
    b.append(f"<rect x='140' y='{iy+120}' width='280' height='240' fill='none' stroke='{P['oni']}' stroke-width='3' stroke-dasharray='12 6'/>")
    b.append(battleship(280, iy + 240, 1.1, 0))
    b.append(f"<rect x='150' y='{iy+390}' width='120' height='40' fill='{P['hulldk']}' stroke='{P['ink']}' stroke-width='2'/>")
    b.append(text(210, iy + 416, "ship bay", 12, P["snow"], anchor="middle"))
    # manufacturing: machines & conveyors (48px tiles)
    T = 48
    mx0, my0 = 520, iy + 96

    def machine(cx, cy, n, col, lab):
        return (f"<rect x='{cx}' y='{cy}' width='{n*T}' height='{n*T}' rx='6' fill='{col}' stroke='{P['ink']}' stroke-width='3'/>"
                f"<rect x='{cx+8}' y='{cy+8}' width='{n*T-16}' height='{n*T-16}' rx='4' fill='none' stroke='#000' stroke-opacity='.3' stroke-width='3'/>"
                f"<circle cx='{cx+n*T/2}' cy='{cy+n*T/2}' r='{n*8}' fill='#000' fill-opacity='.25'/>"
                + text(cx + n * T / 2, cy + n * T + 16, lab, 11, P["snow"], anchor="middle", opacity=.8))

    def conveyor(x1, y1, x2, y2):
        out = f"<path d='M{x1},{y1} L{x2},{y2}' stroke='#556079' stroke-width='22'/><path d='M{x1},{y1} L{x2},{y2}' stroke='#7c88a3' stroke-width='14'/>"
        L = math.hypot(x2 - x1, y2 - y1); ux, uy = (x2 - x1) / L, (y2 - y1) / L
        for d in range(12, int(L) - 6, 22):
            px, py = x1 + ux * d, y1 + uy * d
            out += (f"<path d='M{px-uy*5-ux*4:.1f},{py+ux*5-uy*4:.1f} L{px+ux*4:.1f},{py+uy*4:.1f} L{px+uy*5-ux*4:.1f},{py-ux*5-uy*4:.1f}' "
                    f"fill='none' stroke='{P['amber']}' stroke-width='2.5'/>")
        return out
    b.append(conveyor(mx0 + 96, my0 + 48, mx0 + 168, my0 + 48))
    b.append(conveyor(mx0 + 264, my0 + 48, mx0 + 264, my0 + 168))
    b.append(conveyor(mx0 + 96, my0 + 216, mx0 + 216, my0 + 216))
    b.append(conveyor(mx0 + 48, my0 + 96, mx0 + 48, my0 + 168))
    b.append(machine(mx0, my0, 2, "#b0703a", "ore intake"))
    b.append(machine(mx0 + 168, my0, 2, "#c2543a", "smelter"))
    b.append(machine(mx0, my0 + 168, 2, "#3a8fb0", "power cell"))
    b.append(machine(mx0 + 216, my0 + 168, 2, "#5aa060", "fabricator"))
    b.append(f"<circle cx='{mx0+264}' cy='{my0+300}' r='10' fill='{P['voidglow']}'/>"
             + text(mx0 + 280, my0 + 305, "extraordinary sample!", 11, P["voidglow"]))
    # lab: benches
    b.append(f"<rect x='900' y='{iy+120}' width='150' height='50' rx='6' fill='#8c7ae6' stroke='{P['ink']}' stroke-width='3'/>"
             f"<rect x='900' y='{iy+230}' width='150' height='50' rx='6' fill='#8c7ae6' stroke='{P['ink']}' stroke-width='3'/>"
             f"<circle cx='975' cy='{iy+360}' r='40' fill='none' stroke='{P['oni']}' stroke-width='4' stroke-dasharray='6 6'/>"
             + text(975, iy + 420, "sample analyser", 11, P["snow"], anchor="middle", opacity=.8))
    # tiny crew working
    b.append(oni(700, iy + 432, .16, fur=P["charcoal"], suit="#e0a040", suit2="#a8702a", ears="round", tail="long"))
    b.append(oni(975, iy + 215, .16, fur=P["cream"], suit="#8c7ae6", suit2="#5f50b3", ears="tall"))
    b.append(oni(360, iy + 420, .16, fur=P["ginger"], suit=P["oni"], suit2="#228a84"))
    b.append(text(1300, iy + 490, "Build area = hull interior; every new module extends the hull", 13, P["hulllt"], anchor="end", opacity=.8))
    return svg("".join(b), P["space"])


def battleship(x, y, s=1.0, rot=0):
    return (f"<g transform='translate({x},{y}) rotate({rot}) scale({s})'>"
            f"<ellipse cx='-95' cy='-22' rx='22' ry='10' fill='{P['oni']}' opacity='.7'/><ellipse cx='-95' cy='22' rx='22' ry='10' fill='{P['oni']}' opacity='.7'/>"
            f"<path d='M110,0 L40,-30 L-20,-70 L-80,-60 L-70,-30 L-85,-22 L-85,22 L-70,30 L-80,60 L-20,70 L40,30 Z' fill='{P['hulllt']}' stroke='{P['ink']}' stroke-width='4' stroke-linejoin='round'/>"
            f"<path d='M80,0 L20,-18 L-60,-18 L-60,18 L20,18 Z' fill='{P['hull']}' stroke='{P['ink']}' stroke-width='2'/>"
            f"<path d='M-20,-70 L-10,-90 L5,-55 Z M-20,70 L-10,90 L5,55 Z' fill='{P['ginger']}' stroke='{P['ink']}' stroke-width='3'/>"
            f"<ellipse cx='30' cy='0' rx='18' ry='10' fill='{P['oni']}' stroke='{P['ink']}' stroke-width='2'/>"
            f"<circle cx='-20' cy='-40' r='9' fill='{P['hulldk']}' stroke='{P['ink']}' stroke-width='2'/><circle cx='-20' cy='40' r='9' fill='{P['hulldk']}' stroke='{P['ink']}' stroke-width='2'/>"
            f"<path d='M-20,-40 L10,-44 M-20,40 L10,44' stroke='{P['ink']}' stroke-width='4'/>"
            "</g>")


# ---------------------------------------------------------------- Mission

def swarmling(x, y, s=1, rot=0):
    legs = "".join(f"<path d='M0,0 Q{math.cos(a)*30:.1f},{math.sin(a)*30+8:.1f} {math.cos(a)*42:.1f},{math.sin(a)*42:.1f}' stroke='{P['voidglow']}' stroke-width='4' fill='none'/>"
                   for a in [i * math.pi / 4 + .3 for i in range(8)])
    return (f"<g transform='translate({x},{y}) rotate({rot}) scale({s})'>{legs}"
            f"<circle r='20' fill='{P['void']}' stroke='{P['voidglow']}' stroke-width='3'/>"
            f"<circle cx='6' r='7' fill='#ffe45c'/><circle cx='8' r='3' fill='#000'/></g>")


def maw(x, y, s=1):
    tent = "".join(f"<path d='M{dx},40 C{dx+15},110 {dx-25},160 {dx+10},{200+abs(dx)//3}' stroke='{P['voidglow']}' stroke-width='{8-abs(dx)//20}' fill='none' opacity='.8'/>"
                   for dx in (-70, -40, -12, 16, 44, 72))
    return (f"<g transform='translate({x},{y}) scale({s})'>"
            f"<circle r='150' fill='{P['voidglow']}' opacity='.08'/>{tent}"
            f"<path d='M-110,40 C-120,-80 120,-80 110,40 C80,20 50,55 0,35 C-50,55 -80,20 -110,40 Z' fill='{P['void']}' stroke='{P['voidglow']}' stroke-width='4'/>"
            f"<ellipse cx='0' cy='-5' rx='36' ry='24' fill='#ffe45c'/><ellipse cx='0' cy='-5' rx='8' ry='20' fill='#000'/>"
            f"<circle cx='-60' cy='5' r='8' fill='#ffe45c'/><circle cx='60' cy='5' r='8' fill='#ffe45c'/></g>")


def sheet_mission():
    b = [f"<defs><radialGradient id='neb' cx='.75' cy='.3' r='.7'><stop offset='0' stop-color='{P['void']}' stop-opacity='.9'/><stop offset='1' stop-color='{P['space']}' stop-opacity='0'/></radialGradient>"
         f"<radialGradient id='neb2' cx='.1' cy='.9' r='.5'><stop offset='0' stop-color='#1d4a5c' stop-opacity='.8'/><stop offset='1' stop-color='{P['space']}' stop-opacity='0'/></radialGradient></defs>",
         f"<rect width='{W}' height='{H}' fill='url(#neb)'/><rect width='{W}' height='{H}' fill='url(#neb2)'/>", stars(260, 5),
         header("Sheet 3 — Battleship vs. void monsters", "Top-down mission moment: click-to-move, slow and ability-driven (SC2 Mothership feel), Q/W/E/R bar", dark=True)]
    # asteroids
    r = random.Random(9)
    for _ in range(14):
        ax, ay, ar = r.uniform(60, 1540), r.uniform(140, 760), r.uniform(10, 36)
        pts = " ".join(f"{ax+math.cos(a)*ar*r.uniform(.7,1.1):.1f},{ay+math.sin(a)*ar*r.uniform(.7,1.1):.1f}" for a in [i * math.pi / 4 for i in range(8)])
        b.append(f"<polygon points='{pts}' fill='#3a3f55' stroke='#565c78' stroke-width='2'/>")
    # move path + click marker
    bx, by = 520, 520
    b.append(f"<path d='M{bx+60},{by-10} Q700,700 880,690' fill='none' stroke='{P['oni']}' stroke-width='3' stroke-dasharray='10 10' opacity='.8'/>")
    b.append(f"<circle cx='880' cy='690' r='16' fill='none' stroke='{P['oni']}' stroke-width='3'/><circle cx='880' cy='690' r='5' fill='{P['oni']}'/>")
    b.append(text(902, 696, "right-click move", 13, P["oni"]))
    # ability range
    b.append(f"<circle cx='{bx}' cy='{by}' r='260' fill='{P['amber']}' fill-opacity='.05' stroke='{P['amber']}' stroke-width='2' stroke-dasharray='4 8'/>")
    b.append(text(bx - 30, by - 270, "W range", 13, P["amber"], opacity=.8))
    # companion shield bubble
    b.append(f"<circle cx='{bx}' cy='{by}' r='120' fill='{P['oni']}' fill-opacity='.08' stroke='{P['oni']}' stroke-width='2'/>")
    b.append(battleship(bx, by, 1.0, -20))
    # projectiles
    for i in range(5):
        b.append(f"<path d='M{bx+100+i*50},{by-40-i*28} l24,-14' stroke='{P['amber']}' stroke-width='5' stroke-linecap='round'/>")
    # E beam to maw
    b.append(f"<path d='M{bx+80},{by-40} L1140,300' stroke='{P['oni']}' stroke-width='10' opacity='.35'/><path d='M{bx+80},{by-40} L1140,300' stroke='#fff' stroke-width='3'/>")
    b.append(maw(1200, 260, .9))
    b.append(text(1200, 470, "MAW — large, slow, tentacle sweep", 14, P["voidglow"], "bold", "middle"))
    for (sx, sy, rot) in [(300, 330, 30), (360, 280, 60), (240, 700, -30), (330, 740, -60), (180, 620, 0), (760, 700, 180)]:
        b.append(swarmling(sx, sy, .9, rot))
    b.append(text(250, 250, "SWARMLINGS — small, fast, in packs", 14, P["voidglow"], "bold", "middle"))
    # other player's battleship (co-op)
    b.append(f"<g opacity='.6'>{battleship(1380, 690, .6, -120)}</g>" + text(1380, 770, "squadmate (optional co-op)", 12, P["snow"], anchor="middle", opacity=.6))

    # ---- HUD
    hy = 830
    b.append(f"<rect x='0' y='{hy}' width='{W}' height='{H-hy}' fill='#0a0d1c' opacity='.92'/><path d='M0,{hy} L{W},{hy}' stroke='{P['hulldk']}' stroke-width='3'/>")
    # pilot portrait
    b.append(f"<rect x='40' y='{hy+20}' width='130' height='130' rx='14' fill='{P['space2']}' stroke='{P['oni']}' stroke-width='3'/>")
    b.append(f"<clipPath id='pp'><rect x='40' y='{hy+20}' width='130' height='130' rx='14'/></clipPath>")
    b.append(f"<g clip-path='url(#pp)'>{oni(105, hy+250, .8, fur=P['ginger'], stripe='#b3601f', accessory=acc_pilot())}</g>")
    # HP / shield
    b.append(f"<rect x='200' y='{hy+30}' width='360' height='20' rx='4' fill='#331a1a'/><rect x='200' y='{hy+30}' width='260' height='20' rx='4' fill='#4fd07a'/>")
    b.append(f"<rect x='200' y='{hy+58}' width='360' height='12' rx='4' fill='#1a2a33'/><rect x='200' y='{hy+58}' width='300' height='12' rx='4' fill='{P['oni']}'/>")
    b.append(text(200, hy + 100, "HULL 72%  ·  SHIELD 83%", 14, P["snow"], opacity=.7))
    # skill bar
    keys = [("Q", "Volley", 0), ("W", "Gravity well", .6), ("E", "Lance beam", 0), ("R", "Nine-lives (ult)", 1.0)]
    for i, (k, name, cd) in enumerate(keys):
        kx = 640 + i * 120
        col = P["amber"] if k == "R" else P["oni"]
        b.append(f"<rect x='{kx}' y='{hy+25}' width='96' height='96' rx='12' fill='{P['space2']}' stroke='{col}' stroke-width='3'/>")
        b.append(f"<circle cx='{kx+48}' cy='{hy+73}' r='26' fill='{col}' opacity='.35'/>")
        if cd:
            b.append(f"<rect x='{kx}' y='{hy+25+96*(1-cd)}' width='96' height='{96*cd}' rx='12' fill='#000' opacity='.55'/>")
            b.append(text(kx + 48, hy + 82, f"{int(cd*12)}s", 22, P["snow"], "bold", "middle"))
        b.append(f"<rect x='{kx+4}' y='{hy+29}' width='26' height='24' rx='5' fill='#000' opacity='.6'/>" + text(kx + 17, hy + 47, k, 16, P["snow"], "bold", "middle"))
        b.append(text(kx + 48, hy + 145, name, 13, P["snow"], anchor="middle", opacity=.75))
    b.append(text(640, hy + 163, "Pilot's Combat Operation skills — triggered by hand", 12, P["snow"], opacity=.5))
    # companions auto
    b.append(text(1140, hy + 30, "Companions (auto-trigger)", 13, P["snow"], "bold", opacity=.8))
    comp = [("Engineer", "repair @ HP&lt;40%", P["charcoal"], "#e0a040", "round"), ("Gunner", "burst @ enemy in range", "#9a9aa8", P["hulldk"], "folded")]
    for i, (n, cond, fur, suit, ears) in enumerate(comp):
        cy = hy + 45 + i * 62
        b.append(f"<rect x='1140' y='{cy}' width='54' height='54' rx='10' fill='{P['space2']}' stroke='{suit}' stroke-width='3'/>")
        b.append(f"<clipPath id='c{i}'><rect x='1140' y='{cy}' width='54' height='54' rx='10'/></clipPath><g clip-path='url(#c{i})'>{oni(1167, cy+108, .36, fur=fur, suit=suit, ears=ears)}</g>")
        b.append(text(1206, cy + 22, n, 14, P["snow"], "bold") + text(1206, cy + 42, cond, 12, P["oni"]))
    # minimap
    b.append(f"<rect x='1420' y='{hy+20}' width='150' height='130' rx='8' fill='{P['space2']}' stroke='{P['hulldk']}' stroke-width='3'/>")
    b.append(f"<circle cx='1470' cy='{hy+95}' r='5' fill='{P['oni']}'/><circle cx='1530' cy='{hy+55}' r='7' fill='{P['voidglow']}'/><circle cx='1445' cy='{hy+70}' r='3' fill='{P['voidglow']}'/><circle cx='1450' cy='{hy+115}' r='3' fill='{P['voidglow']}'/>")
    b.append(f"<rect x='1500' y='{hy+120}' width='14' height='14' fill='none' stroke='{P['amber']}' stroke-width='2'/>" + text(1507, hy + 113, "objective", 11, P["amber"], anchor="middle"))
    return svg("".join(b), P["space"])


# ---------------------------------------------------------------- Mood

def sheet_mood():
    b = [f"<defs><radialGradient id='n1' cx='.3' cy='.4' r='.6'><stop offset='0' stop-color='#2b5c78'/><stop offset='1' stop-color='{P['space']}' stop-opacity='0'/></radialGradient>"
         f"<radialGradient id='n2' cx='.8' cy='.6' r='.5'><stop offset='0' stop-color='{P['void']}'/><stop offset='1' stop-color='{P['space']}' stop-opacity='0'/></radialGradient>"
         f"<radialGradient id='wh' cx='.5' cy='.5' r='.5'><stop offset='0' stop-color='#000'/><stop offset='.55' stop-color='#000'/><stop offset='.7' stop-color='{P['voidglow']}'/><stop offset='.85' stop-color='{P['oni']}' stop-opacity='.6'/><stop offset='1' stop-color='{P['oni']}' stop-opacity='0'/></radialGradient>"
         f"<clipPath id='bd'><rect x='50' y='130' width='1000' height='620' rx='16'/></clipPath></defs>",
         header("Sheet 4 — Mood & palette", "A space segment: a pocket of space the Oni cut out of the universe, edged by the torn void", dark=True)]
    # backdrop panel
    g = [f"<rect x='50' y='130' width='1000' height='620' fill='{P['space']}'/>",
         f"<rect x='50' y='130' width='1000' height='620' fill='url(#n1)'/><rect x='50' y='130' width='1000' height='620' fill='url(#n2)'/>",
         stars(320, 11, 50, 130, 1000, 620)]
    # torn segment edge (right side) -> void beyond
    r = random.Random(3)
    pts = [(820, 130)]
    y = 130
    while y < 750:
        y += r.uniform(25, 55)
        pts.append((820 + r.uniform(-60, 60) + (y - 130) * .15, min(y, 750)))
    edge = " ".join(f"L{x:.0f},{y:.0f}" for x, y in pts)
    g.append(f"<path d='M{pts[0][0]},{pts[0][1]} {edge} L1050,750 L1050,130 Z' fill='#07030f'/>")
    g.append(f"<path d='M{pts[0][0]},{pts[0][1]} {edge}' fill='none' stroke='{P['voidglow']}' stroke-width='5'/>")
    g.append(f"<path d='M{pts[0][0]},{pts[0][1]} {edge}' fill='none' stroke='#fff' stroke-width='1.5' opacity='.8'/>")
    for i in range(18):
        vx, vy = r.uniform(900, 1040), r.uniform(150, 740)
        g.append(f"<path d='M{vx:.0f},{vy:.0f} l{r.uniform(-30,10):.0f},{r.uniform(-8,8):.0f}' stroke='{P['voidglow']}' stroke-width='2' opacity='.5'/>")
    g.append(f"<g opacity='.8'>{swarmling(960, 300, .6, 180)}{swarmling(990, 560, .5, 200)}</g>")
    # wormhole
    g.append(f"<ellipse cx='330' cy='330' rx='130' ry='130' fill='url(#wh)'/>")
    for k in range(4):
        g.append(f"<ellipse cx='330' cy='330' rx='{75+k*12}' ry='{75+k*12}' fill='none' stroke='{P['oni']}' stroke-width='1.5' opacity='{.5-k*.1}' stroke-dasharray='30 {10+k*6}' transform='rotate({k*30} 330 330)'/>")
    # planet + carrier silhouette
    g.append(f"<circle cx='200' cy='860' r='300' fill='#1b2d3f'/><circle cx='200' cy='860' r='300' fill='none' stroke='{P['oni']}' stroke-width='3' opacity='.5'/>")
    g.append(f"<g transform='translate(430,540) scale(.3)'><path d='M0,-50 L180,-70 L260,-100 L420,-100 L460,-70 L900,-60 L1100,-30 L1230,10 L1100,50 L700,80 L200,80 L0,60 Z' fill='#0b0f1d'/>"
             f"<path d='M1030,-120 L1045,-165 L1060,-120 Z M1070,-120 L1085,-165 L1100,-120 Z' fill='#0b0f1d'/><path d='M1010,-120 L1110,-120 L1130,-35 L980,-60 Z' fill='#0b0f1d'/>"
             f"<ellipse cx='-30' cy='0' rx='60' ry='40' fill='{P['oni']}' opacity='.6'/></g>")
    b.append(f"<g clip-path='url(#bd)'>{''.join(g)}</g><rect x='50' y='130' width='1000' height='620' rx='16' fill='none' stroke='{P['hulldk']}' stroke-width='3'/>")
    b.append(text(70, 780, "Left: settled Oni space (teal, calm, readable). Right: the segment's torn edge, where void monsters leak in (violet/magenta).", 14, P["snow"], opacity=.7))
    # palette
    b.append(text(1100, 160, "Palette", 22, P["snow"], "bold"))
    sw = [("space", "Deep space", P["space"]), ("space2", "Segment navy", P["space2"]), ("oni", "Oni teal — player / tech", P["oni"]),
          ("amber", "Amber — lights, UI accents", P["amber"]), ("hull", "Hull steel", P["hull"]), ("hulllt", "Hull light", P["hulllt"]),
          ("ginger", "Ginger — Oni fur, ‘ear’ fins", P["ginger"]), ("cream", "Cream fur", P["cream"]),
          ("void", "Void violet — enemies", P["void"]), ("voidglow", "Void magenta — enemy glow", P["voidglow"]), ("red", "Alert red — damage", P["red"])]
    for i, (_, name, c) in enumerate(sw):
        yy = 185 + i * 44
        b.append(f"<rect x='1100' y='{yy}' width='60' height='34' rx='6' fill='{c}' stroke='{P['hulllt']}' stroke-width='1.5'/>")
        b.append(text(1175, yy + 16, name, 15, P["snow"]) + text(1175, yy + 32, c, 12, P["snow"], opacity=.5))
    # style notes
    notes = ["Style note (draft, for reaction)",
             "• Flat vector shapes, thick dark outlines — reads at",
             "   small sprite sizes and suits LÖVE2D.",
             "• Cute, chubby cats vs. alien, tentacled void:",
             "   warm/teal = friendly, violet/magenta = hostile.",
             "• Tech borrows cat motifs (ear fins, whisker",
             "   antennae) rather than human cockpits.",
             "• Interior = Mindustry-clean top-down grid;",
             "   missions = dark space with bright telegraphs."]
    for i, n in enumerate(notes):
        b.append(text(1100, 700 + i * 30, n, 17 if i == 0 else 14, P["snow"], "bold" if i == 0 else "normal", opacity=1 if i == 0 else .8))
    return svg("".join(b), "#0e1225")


if __name__ == "__main__":
    for name, fn in [("01-oni-crew", sheet_crew), ("02-carrier", sheet_carrier),
                     ("03-mission-combat", sheet_mission), ("04-mood-palette", sheet_mood)]:
        with open(os.path.join(HERE, name + ".svg"), "w", encoding="utf-8") as f:
            f.write(fn())
        print("wrote", name + ".svg")
