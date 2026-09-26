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

HY = -190  # head centre, relative to feet


def oni(x, y, s=1.0, fur="#e38b3f", belly="#f3e7d3", suit="#3fd0c9", suit2="#2a8f8a",
        ears="pointy", eyes="#ffb347", accessory="", stripe=None, tail="curl", flip=False):
    """A round, chubby, big-headed cat-like Oni. Origin = feet centre."""
    sx = -s if flip else s
    k = P["ink"]
    hy = HY
    g = [f"<g transform='translate({x},{y}) scale({sx},{s})' stroke-linejoin='round'>"]
    # tail
    if tail == "curl":
        g.append(f"<path d='M60,-45 C125,-45 135,-120 100,-140 C80,-150 72,-128 88,-120 C108,-108 100,-70 62,-68' fill='{fur}' stroke='{k}' stroke-width='4'/>")
    elif tail == "long":
        g.append(f"<path d='M60,-35 C130,-30 150,-100 145,-165' fill='none' stroke='{k}' stroke-width='26' stroke-linecap='round'/>"
                 f"<path d='M60,-35 C130,-30 150,-100 145,-165' fill='none' stroke='{fur}' stroke-width='18' stroke-linecap='round'/>")
    elif tail == "stub":
        g.append(f"<circle cx='78' cy='-45' r='24' fill='{fur}' stroke='{k}' stroke-width='4'/>")
    # feet (little beans)
    for fx in (-36, 36):
        g.append(f"<ellipse cx='{fx}' cy='-10' rx='30' ry='15' fill='{suit2}' stroke='{k}' stroke-width='4'/>")
    # body: a fat round loaf
    g.append(f"<path d='M-82,-30 C-98,-100 -62,-140 0,-140 C62,-140 98,-100 82,-30 C72,0 -72,0 -82,-30 Z' fill='{suit}' stroke='{k}' stroke-width='4'/>")
    g.append(f"<ellipse cx='0' cy='-58' rx='50' ry='44' fill='{belly}' opacity='.95'/>")
    # stubby arms + paws
    for m in (-1, 1):
        g.append(f"<ellipse cx='{m*82}' cy='-78' rx='18' ry='24' fill='{suit}' stroke='{k}' stroke-width='4' transform='rotate({-m*20} {m*82} -78)'/>"
                 f"<circle cx='{m*88}' cy='-58' r='14' fill='{fur}' stroke='{k}' stroke-width='4'/>")
    # ears
    ear = {
        "pointy": (f"M-86,{hy-32} C-92,{hy-85} -82,{hy-112} -68,{hy-108} C-55,{hy-102} -40,{hy-82} -28,{hy-70} Z",
                   f"M-76,{hy-40} C-80,{hy-78} -74,{hy-96} -66,{hy-93} C-58,{hy-88} -50,{hy-76} -42,{hy-68} Z"),
        "round": (f"M-90,{hy-35} C-102,{hy-105} -42,{hy-110} -32,{hy-70} Z",
                  f"M-80,{hy-42} C-88,{hy-90} -52,{hy-94} -44,{hy-68} Z"),
        "tall": (f"M-76,{hy-45} C-84,{hy-125} -66,{hy-148} -54,{hy-142} C-44,{hy-130} -36,{hy-95} -30,{hy-72} Z",
                 f"M-68,{hy-52} C-73,{hy-112} -62,{hy-128} -56,{hy-124} C-50,{hy-114} -45,{hy-90} -42,{hy-72} Z"),
        "folded": (f"M-90,{hy-40} C-92,{hy-88} -42,{hy-94} -30,{hy-70} C-52,{hy-64} -74,{hy-52} -90,{hy-40} Z", ""),
    }[ears]
    for m in (1, -1):
        g.append(f"<g transform='scale({m},1)'><path d='{ear[0]}' fill='{fur}' stroke='{k}' stroke-width='4'/>"
                 + (f"<path d='{ear[1]}' fill='#f2a7b0'/>" if ear[1] else "") + "</g>")
    # head: wide, with puffy cheeks
    g.append(f"<path d='M-98,{hy+8} C-104,{hy-62} -52,{hy-80} 0,{hy-80} C52,{hy-80} 104,{hy-62} 98,{hy+8} "
             f"C106,{hy+34} 88,{hy+72} 0,{hy+74} C-88,{hy+72} -106,{hy+34} -98,{hy+8} Z' fill='{fur}' stroke='{k}' stroke-width='4'/>")
    if stripe:
        for dx in (-16, 0, 16):
            g.append(f"<path d='M{dx},{hy-78} L{dx*0.7},{hy-52}' stroke='{stripe}' stroke-width='8' stroke-linecap='round'/>")
        for m in (1, -1):
            g.append(f"<path d='M{m*100},{hy+2} L{m*80},{hy+6} M{m*100},{hy+20} L{m*80},{hy+20}' stroke='{stripe}' stroke-width='7' stroke-linecap='round'/>")
    g.append(f"<ellipse cx='0' cy='{hy+40}' rx='34' ry='22' fill='{belly}'/>")
    # blush
    for m in (-1, 1):
        g.append(f"<ellipse cx='{m*66}' cy='{hy+34}' rx='16' ry='9' fill='#ff8fa3' opacity='.55'/>")
    # huge sparkly eyes
    for ex in (-40, 40):
        g.append(f"<ellipse cx='{ex}' cy='{hy+4}' rx='23' ry='26' fill='{eyes}' stroke='{k}' stroke-width='3'/>"
                 f"<ellipse cx='{ex}' cy='{hy+7}' rx='15' ry='20' fill='{k}'/>"
                 f"<circle cx='{ex+7}' cy='{hy-5}' r='8' fill='#fff'/><circle cx='{ex-7}' cy='{hy+15}' r='3.5' fill='#fff'/>")
    g.append(f"<path d='M-7,{hy+28} L7,{hy+28} L0,{hy+35} Z' fill='#e0707e'/>")
    g.append(f"<path d='M-11,{hy+40} Q-5,{hy+46} 0,{hy+38} Q5,{hy+46} 11,{hy+40}' fill='none' stroke='{k}' stroke-width='3' stroke-linecap='round'/>")
    for m in (1, -1):
        g.append(f"<path d='M{m*50},{hy+36} L{m*92},{hy+30} M{m*50},{hy+44} L{m*92},{hy+48}' stroke='{k}' stroke-width='2' opacity='.5'/>")
    g.append(accessory)
    g.append("</g>")
    return "".join(g)


def acc_pilot():
    hy = HY
    return (f"<path d='M-100,{hy-38} C-90,{hy-80} 90,{hy-80} 100,{hy-38}' fill='none' stroke='{P['hulldk']}' stroke-width='10'/>"
            f"<rect x='-72' y='{hy-66}' width='60' height='30' rx='14' fill='{P['oni']}' fill-opacity='.6' stroke='{P['ink']}' stroke-width='4'/>"
            f"<rect x='12' y='{hy-66}' width='60' height='30' rx='14' fill='{P['oni']}' fill-opacity='.6' stroke='{P['ink']}' stroke-width='4'/>"
            f"<path d='M-12,{hy-51} L12,{hy-51}' stroke='{P['ink']}' stroke-width='5'/>"
            f"<path d='M-36,-138 L-14,-112 L0,-128 L14,-112 L36,-138' fill='{P['amber']}' stroke='{P['ink']}' stroke-width='3'/>"
            f"<circle cx='-44' cy='-96' r='9' fill='{P['amber']}' stroke='{P['ink']}' stroke-width='3'/>")


def acc_engineer():
    return (f"<rect x='-80' y='-40' width='160' height='16' rx='6' fill='#6b4a2e' stroke='{P['ink']}' stroke-width='3'/>"
            f"<rect x='-46' y='-46' width='20' height='26' rx='3' fill='#8a6a44' stroke='{P['ink']}' stroke-width='3'/>"
            f"<rect x='26' y='-46' width='20' height='26' rx='3' fill='#8a6a44' stroke='{P['ink']}' stroke-width='3'/>"
            f"<g transform='translate(96,-60) rotate(-30)'><rect x='-5' y='-62' width='10' height='62' fill='{P['hulllt']}' stroke='{P['ink']}' stroke-width='3'/>"
            f"<path d='M-16,-62 L-16,-84 L-6,-78 L-6,-68 L6,-68 L6,-78 L16,-84 L16,-62 Z' fill='{P['hulllt']}' stroke='{P['ink']}' stroke-width='3'/></g>"
            f"<path d='M-78,-252 C-68,-300 68,-300 78,-252 Z' fill='{P['amber']}' stroke='{P['ink']}' stroke-width='4'/>"
            f"<rect x='-92' y='-258' width='184' height='13' rx='6' fill='{P['amber']}' stroke='{P['ink']}' stroke-width='4'/>")


def acc_researcher():
    hy = HY
    return (f"<circle cx='-40' cy='{hy+4}' r='30' fill='none' stroke='{P['ink']}' stroke-width='4'/>"
            f"<circle cx='40' cy='{hy+4}' r='30' fill='none' stroke='{P['ink']}' stroke-width='4'/>"
            f"<path d='M-10,{hy} L10,{hy}' stroke='{P['ink']}' stroke-width='4'/>"
            f"<g transform='translate(-112,-100)'><path d='M0,0 L-50,-60 L10,-80 L40,-15 Z' fill='{P['oni']}' opacity='.25'/>"
            f"<rect x='-20' y='-10' width='40' height='28' rx='4' fill='{P['hulldk']}' stroke='{P['ink']}' stroke-width='3'/>"
            f"<path d='M-30,-45 L0,-55 M-25,-35 L10,-45 M-18,-25 L18,-33' stroke='{P['oni']}' stroke-width='3'/></g>"
            f"<path d='M-52,-134 L0,-104 L52,-134' fill='none' stroke='#fff' stroke-width='6' opacity='.8'/>")


def acc_gunner():
    return (f"<path d='M-66,-136 C-96,-120 -100,-86 -92,-70 L-60,-84 L-54,-132 Z' fill='{P['hull']}' stroke='{P['ink']}' stroke-width='4'/>"
            f"<path d='M66,-136 C96,-120 100,-86 92,-70 L60,-84 L54,-132 Z' fill='{P['hull']}' stroke='{P['ink']}' stroke-width='4'/>"
            f"<path d='M-36,-72 L-20,-80 L-4,-72 L-8,-54 L-32,-54 Z' fill='{P['red']}' stroke='{P['ink']}' stroke-width='3'/>"
            f"<path d='M-60,-258 Q0,-280 60,-258' fill='none' stroke='{P['red']}' stroke-width='11' stroke-linecap='round'/>")


def sheet_crew():
    b = [header("Sheet 1 (v2) — Oni crew lineup", "Rounder and chubbier: bigger heads, puffy cheeks, big sparkly eyes, loaf-shaped bodies, stubby limbs")]
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
        b.append(f"<ellipse cx='{x}' cy='640' rx='130' ry='18' fill='{P['ink']}' opacity='.12'/>")
        b.append(oni(x, 632, 1.25, **kw))
        b.append(text(x, 690, name, 20, P["ink"], "bold", "middle"))
        b.append(text(x, 716, role, 15, P["ink"], anchor="middle", opacity=.7))
    b.append(text(50, 790, "Silhouette variants — ear / tail shape tell the crew apart at gameplay scale", 18, P["ink"], "bold"))
    vs = [("pointy", "curl"), ("round", "long"), ("tall", "curl"), ("folded", "stub"), ("pointy", "long"), ("round", "stub")]
    for i, (e, t) in enumerate(vs):
        b.append(f"<g opacity='.92'>{oni(110 + i * 150, 975, .55, fur=P['ink'], belly=P['ink'], suit=P['ink'], suit2=P['ink'], eyes=P['ink'], ears=e, tail=t)}</g>")
    b.append(text(1000, 830, "At carrier-interior scale (~32 px):", 15, P["ink"], opacity=.7))
    for i, (_, _, _, kw) in enumerate(crew):
        kw2 = dict(kw); kw2["accessory"] = ""
        b.append(oni(1030 + i * 60, 900, .22, **kw2))
    b.append(text(1300, 830, "Crew palette (unchanged)", 15, P["ink"], opacity=.7))
    sw = [P["ginger"], P["charcoal"], P["cream"], "#9a9aa8", P["oni"], "#e0a040", "#8c7ae6", P["hulldk"]]
    for i, c in enumerate(sw):
        b.append(f"<rect x='{1300 + (i % 4) * 62}' y='{845 + (i // 4) * 62}' width='52' height='52' rx='8' fill='{c}' stroke='{P['ink']}' stroke-width='2'/>")
    b.append(text(1300, 985, "fur tones (top) · suit = role colour (bottom)", 13, P["ink"], opacity=.6))
    return svg("".join(b), P["paper"])


# ---------------------------------------------------------------- Battleships

def ship_scout(x, y, s=1.0, rot=0):
    k = P["ink"]
    return (f"<g transform='translate({x},{y}) rotate({rot}) scale({s})'>"
            f"<ellipse cx='0' cy='52' rx='8' ry='16' fill='{P['oni']}' opacity='.8'/>"
            f"<path d='M0,-60 L14,-10 L40,30 L12,24 L0,44 L-12,24 L-40,30 L-14,-10 Z' fill='{P['hulllt']}' stroke='{k}' stroke-width='4'/>"
            f"<path d='M-14,-10 L-8,-34 L0,-10 Z M14,-10 L8,-34 L0,-10 Z' fill='{P['ginger']}' stroke='{k}' stroke-width='2'/>"
            f"<ellipse cx='0' cy='-14' rx='6' ry='12' fill='{P['oni']}' stroke='{k}' stroke-width='2'/></g>")


def ship_salvager(x, y, s=1.0, rot=0):
    k = P["ink"]
    return (f"<g transform='translate({x},{y}) rotate({rot}) scale({s})'>"
            f"<ellipse cx='-16' cy='56' rx='9' ry='14' fill='{P['oni']}' opacity='.8'/><ellipse cx='16' cy='56' rx='9' ry='14' fill='{P['oni']}' opacity='.8'/>"
            f"<path d='M-40,-20 L-50,-62 L-34,-66 L-26,-30 M40,-20 L50,-62 L34,-66 L26,-30' fill='none' stroke='{P['amber']}' stroke-width='8' stroke-linecap='round'/>"
            f"<rect x='-40' y='-30' width='80' height='80' rx='22' fill='{P['hulllt']}' stroke='{k}' stroke-width='4'/>"
            f"<rect x='-22' y='-4' width='44' height='34' rx='6' fill='{P['hull']}' stroke='{k}' stroke-width='2'/>"
            f"<circle cx='0' cy='-14' r='9' fill='{P['oni']}' stroke='{k}' stroke-width='2'/>"
            "</g>")


def ship_striker(x, y, s=1.0, rot=0):
    k = P["ink"]
    return (f"<g transform='translate({x},{y}) rotate({rot}) scale({s})'>"
            f"<ellipse cx='-24' cy='54' rx='10' ry='16' fill='{P['oni']}' opacity='.8'/><ellipse cx='24' cy='54' rx='10' ry='16' fill='{P['oni']}' opacity='.8'/>"
            f"<path d='M0,-50 L30,-30 L62,10 L60,40 L20,44 L0,34 L-20,44 L-60,40 L-62,10 L-30,-30 Z' fill='{P['hulllt']}' stroke='{k}' stroke-width='4'/>"
            f"<path d='M-46,10 L-46,-40 M46,10 L46,-40 M-20,-20 L-20,-58 M20,-20 L20,-58' stroke='{k}' stroke-width='7' stroke-linecap='round'/>"
            f"<path d='M-30,-30 L-22,-58 L-12,-36 Z M30,-30 L22,-58 L12,-36 Z' fill='{P['ginger']}' stroke='{k}' stroke-width='2'/>"
            f"<ellipse cx='0' cy='-4' rx='10' ry='14' fill='{P['oni']}' stroke='{k}' stroke-width='2'/></g>")


# ---------------------------------------------------------------- Carrier

MOD = {"D": ("DOCK", P["oni"]), "M": ("MFG", P["amber"]), "L": ("LAB", "#8c7ae6")}


def sheet_carrier():
    k = P["ink"]
    b = [f"<defs><linearGradient id='bg' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='{P['space']}'/><stop offset='1' stop-color='{P['space2']}'/></linearGradient>"
         f"<radialGradient id='eng' cx='.5' cy='.5' r='.5'><stop offset='0' stop-color='#fff'/><stop offset='.4' stop-color='{P['oni']}'/><stop offset='1' stop-color='{P['oni']}' stop-opacity='0'/></radialGradient>"
         f"<pattern id='grid' width='20' height='20' patternUnits='userSpaceOnUse'><rect width='20' height='20' fill='#2a3246'/><path d='M20,0 L0,0 0,20' fill='none' stroke='#39435b' stroke-width='1'/></pattern></defs>",
         f"<rect width='{W}' height='{H}' fill='url(#bg)'/>", stars(160, 2, 0, 0, W, 460),
         header("Sheet 2 (v2) — The modular carrier", "A spine with standard sockets; any mix of modules, several of each kind. Each Dock holds a different battleship, each Mfg a different product", dark=True)]
    # ---- spine + sockets
    sy = 290
    b.append(f"<ellipse cx='150' cy='{sy}' rx='80' ry='50' fill='url(#eng)'/>")
    b.append(f"<ellipse cx='205' cy='{sy}' rx='22' ry='80' fill='none' stroke='{P['voidglow']}' stroke-width='12' opacity='.85'/>")
    b.append(f"<rect x='190' y='{sy-22}' width='1140' height='44' rx='10' fill='{P['hull']}' stroke='{k}' stroke-width='4'/>")
    for i in range(38):
        b.append(f"<rect x='{215+i*29}' y='{sy-4}' width='14' height='8' fill='{P['amber']}' opacity='{.9 if i%3 else .3}'/>")
    # bridge head w/ ear fins
    b.append(f"<path d='M1320,{sy-50} L1420,{sy-40} L1480,{sy} L1420,{sy+40} L1320,{sy+50} Z' fill='{P['hulllt']}' stroke='{k}' stroke-width='4'/>"
             f"<path d='M1380,{sy-44} L1395,{sy-88} L1412,{sy-42} Z M1380,{sy+44} L1395,{sy+88} L1412,{sy+42} Z' fill='{P['ginger']}' stroke='{k}' stroke-width='3'/>"
             f"<path d='M1420,{sy-18} L1462,{sy} L1420,{sy+18} Z' fill='{P['oni']}' opacity='.85'/>")
    top = ["D", "D", "M", "M", "L", None, None]
    bot = ["D", "M", None, None, None, None, None]
    labels = {("t", 0): "Scout", ("t", 1): "Salvager", ("b", 0): "Striker",
              ("t", 2): "Plates", ("t", 3): "Ammo", ("b", 1): "Circuits", ("t", 4): "Samples"}
    for row, lst, y0 in (("t", top, sy - 132), ("b", bot, sy + 32)):
        for i, m in enumerate(lst):
            x0 = 240 + i * 150
            cy_conn = sy - 22 if row == "t" else sy + 22
            if m:
                name, col = MOD[m]
                b.append(f"<rect x='{x0+50}' y='{min(y0+100, cy_conn) if row=='t' else cy_conn}' width='30' height='10' fill='{P['hulldk']}'/>")
                b.append(f"<rect x='{x0}' y='{y0}' width='130' height='100' rx='14' fill='{P['hulldk']}' stroke='{col}' stroke-width='5'/>")
                b.append(f"<rect x='{x0+8}' y='{y0+8}' width='114' height='84' rx='9' fill='url(#grid)'/>")
                b.append(text(x0 + 65, y0 + 42, name, 20, col, "bold", "middle"))
                b.append(text(x0 + 65, y0 + 70, labels[(row, i)], 15, P["snow"], anchor="middle", opacity=.85))
            else:
                b.append(f"<rect x='{x0}' y='{y0}' width='130' height='100' rx='14' fill='none' stroke='{P['hulllt']}' stroke-width='3' stroke-dasharray='10 8' opacity='.45'/>")
                b.append(text(x0 + 65, y0 + 58, "+ socket", 15, P["hulllt"], anchor="middle", opacity=.55))
    b.append(text(800, sy + 170, "Add a module to any free socket → the carrier grows. Duplicates allowed: 3 Docks = 3 battleships ready.", 15, P["hulllt"], anchor="middle", opacity=.85))

    # ---- lower panels
    py = 500
    b.append(text(50, py - 12, "Docking modules — one battleship each, each with its own mechanic", 18, P["snow"], "bold"))
    ships = [("Scout", "fast, fragile; dash + wide sensor", ship_scout, P["oni"]),
             ("Salvager", "tractor claws; pulls loot & wrecks", ship_salvager, P["amber"]),
             ("Striker", "slow, heavy; dense forward guns", ship_striker, P["red"])]
    for i, (n, role, fn, col) in enumerate(ships):
        x0 = 50 + i * 245
        b.append(f"<rect x='{x0}' y='{py}' width='225' height='320' rx='14' fill='{P['hulldk']}' stroke='{P['oni']}' stroke-width='4'/>")
        b.append(f"<rect x='{x0+10}' y='{py+10}' width='205' height='250' rx='8' fill='url(#grid)'/>")
        b.append(f"<rect x='{x0+40}' y='{py+45}' width='145' height='180' fill='none' stroke='{P['oni']}' stroke-width='2' stroke-dasharray='10 6' opacity='.7'/>")
        b.append(fn(x0 + 112, py + 140, 1.15))
        b.append(text(x0 + 112, py + 285, f"DOCK · {n}", 16, P["oni"], "bold", "middle"))
        b.append(text(x0 + 112, py + 306, role, 12, P["snow"], anchor="middle", opacity=.8))

    b.append(text(820, py - 12, "Manufacturing modules — same room type, different production line", 18, P["snow"], "bold"))
    T = 34

    def mach(cx, cy, col):
        return (f"<rect x='{cx}' y='{cy}' width='{2*T}' height='{2*T}' rx='6' fill='{col}' stroke='{k}' stroke-width='3'/>"
                f"<circle cx='{cx+T}' cy='{cy+T}' r='12' fill='#000' fill-opacity='.25'/>")

    def belt(x1, y1, x2, y2):
        out = f"<path d='M{x1},{y1} L{x2},{y2}' stroke='#556079' stroke-width='16'/><path d='M{x1},{y1} L{x2},{y2}' stroke='#7c88a3' stroke-width='10'/>"
        L = math.hypot(x2 - x1, y2 - y1); ux, uy = (x2 - x1) / L, (y2 - y1) / L
        for d in range(10, int(L) - 4, 16):
            px, py_ = x1 + ux * d, y1 + uy * d
            out += (f"<path d='M{px-uy*4-ux*3:.1f},{py_+ux*4-uy*3:.1f} L{px+ux*3:.1f},{py_+uy*3:.1f} L{px+uy*4-ux*3:.1f},{py_-ux*4-uy*3:.1f}' fill='none' stroke='{P['amber']}' stroke-width='2'/>")
        return out

    def product(cx, cy, kind):
        if kind == "Plates":
            return "".join(f"<rect x='{cx-18+j*4}' y='{cy-10-j*6}' width='36' height='10' rx='2' fill='{P['hulllt']}' stroke='{k}' stroke-width='2'/>" for j in range(3))
        if kind == "Ammo":
            return "".join(f"<path d='M{cx-16+j*14},{cy+12} L{cx-16+j*14},{cy-6} Q{cx-10+j*14},{cy-20} {cx-4+j*14},{cy-6} L{cx-4+j*14},{cy+12} Z' fill='{P['amber']}' stroke='{k}' stroke-width='2'/>" for j in range(3))
        return (f"<rect x='{cx-20}' y='{cy-16}' width='40' height='30' rx='4' fill='#2f8f5a' stroke='{k}' stroke-width='2'/>"
                f"<path d='M{cx-12},{cy-6} L{cx+12},{cy-6} M{cx-12},{cy+4} L{cx+4},{cy+4}' stroke='{P['amber']}' stroke-width='3'/>")
    lines = [("Plates", "#c2543a", "ore → smelter → press"), ("Ammo", "#b0703a", "metal + powder → shells"), ("Circuits", "#3a8fb0", "silicon → etcher → boards")]
    for i, (n, col, recipe) in enumerate(lines):
        x0 = 820 + i * 245
        b.append(f"<rect x='{x0}' y='{py}' width='225' height='320' rx='14' fill='{P['hulldk']}' stroke='{P['amber']}' stroke-width='4'/>")
        b.append(f"<rect x='{x0+10}' y='{py+10}' width='205' height='250' rx='8' fill='url(#grid)'/>")
        b.append(belt(x0 + 88, py + 70, x0 + 130, py + 70))
        b.append(belt(x0 + 164, py + 104, x0 + 164, py + 150))
        b.append(belt(x0 + 130, py + 184, x0 + 88, py + 184))
        b.append(mach(x0 + 20, py + 36, "#6d6d7a"))
        b.append(mach(x0 + 130, py + 36, col))
        b.append(mach(x0 + 130, py + 150, col))
        b.append(f"<rect x='{x0+20}' y='{py+150}' width='68' height='68' rx='10' fill='#1c2233' stroke='{P['amber']}' stroke-width='2' stroke-dasharray='5 4'/>")
        b.append(product(x0 + 54, py + 186, n))
        b.append(text(x0 + 112, py + 285, f"MFG · {n}", 16, P["amber"], "bold", "middle"))
        b.append(text(x0 + 112, py + 306, recipe, 12, P["snow"], anchor="middle", opacity=.8))
    b.append(oni(84, py + 252, .14, fur=P["ginger"], suit=P["oni"], suit2="#228a84"))
    b.append(oni(1100, py + 250, .14, fur=P["charcoal"], suit="#e0a040", suit2="#a8702a", ears="round", tail="long"))
    b.append(text(800, 870, "Product names, ship roles and socket count are placeholders — the point is the structure: modules are repeatable building blocks.", 14, P["hulllt"], anchor="middle", opacity=.8))
    b.append(text(800, 896, "Build area = sum of module interiors (each still Mindustry-style inside, bounded by its walls).", 14, P["hulllt"], anchor="middle", opacity=.8))
    return svg("".join(b), P["space"])


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


def pill(cx, cy, s, col):
    w = len(s) * 7.6 + 24
    return (f"<rect x='{cx-w/2:.0f}' y='{cy-15}' width='{w:.0f}' height='26' rx='13' fill='#0a0d1c' opacity='.85' stroke='{col}' stroke-width='1.5'/>"
            + text(cx, cy + 3, s, 13, col, "bold", "middle"))


def bullet(x, y, r=6, col=None):
    col = col or P["voidglow"]
    return f"<circle cx='{x:.1f}' cy='{y:.1f}' r='{r+3}' fill='{col}' opacity='.35'/><circle cx='{x:.1f}' cy='{y:.1f}' r='{r}' fill='{col}'/><circle cx='{x:.1f}' cy='{y:.1f}' r='{r*.45:.1f}' fill='#fff'/>"


def sheet_mission():
    b = [f"<defs><radialGradient id='neb' cx='.5' cy='.2' r='.7'><stop offset='0' stop-color='{P['void']}' stop-opacity='.9'/><stop offset='1' stop-color='{P['space']}' stop-opacity='0'/></radialGradient>"
         f"<radialGradient id='neb2' cx='.1' cy='.9' r='.5'><stop offset='0' stop-color='#1d4a5c' stop-opacity='.8'/><stop offset='1' stop-color='{P['space']}' stop-opacity='0'/></radialGradient>"
         f"<clipPath id='arena'><rect x='0' y='0' width='{W}' height='830'/></clipPath></defs>",
         f"<rect width='{W}' height='{H}' fill='url(#neb)'/><rect width='{W}' height='{H}' fill='url(#neb2)'/>", stars(260, 5)]
    shx, shy = 800, 700  # player ship
    g = []
    mx, my = 800, 250
    g.append(maw(mx, my, .85))
    # ring + spiral bullet patterns from the Maw
    for ring, (R, n, off) in enumerate([(170, 26, 0), (250, 30, .1), (340, 34, .05)]):
        for i in range(n):
            a = off + i * 2 * math.pi / n
            x, y = mx + math.cos(a) * R, my + math.sin(a) * R * .95
            if math.hypot(x - shx, y - shy) > 70 and 110 < y < 820 and not (ring == 2 and i % 7 == 0):
                g.append(bullet(x, y, 6))
    for arm in range(4):
        for j in range(16):
            t = j * .22
            a = arm * math.pi / 2 + t * 1.6
            R = 70 + t * 150
            x, y = mx + math.cos(a) * R, my + math.sin(a) * R
            if math.hypot(x - shx, y - shy) > 70 and 110 < y < 820:
                g.append(bullet(x, y, 4, "#ff7de9"))
    # swarmlings firing aimed streams
    for (sx, sy, rot) in [(230, 430, 30), (300, 560, 10), (1360, 470, 150), (1290, 600, 170)]:
        g.append(swarmling(sx, sy, .8, rot))
        dx, dy = shx - sx, shy - sy
        L = math.hypot(dx, dy); ux, uy = dx / L, dy / L
        for d in range(50, int(L) - 90, 34):
            px, py = sx + ux * d, sy + uy * d
            if math.hypot(px - 1040, py - 560) < 86:
                continue
            g.append(f"<path d='M{px:.1f},{py:.1f} l{ux*14:.1f},{uy*14:.1f}' stroke='#ffe45c' stroke-width='6' stroke-linecap='round'/>")
    # player's own fire (three streams upward)
    for col in (-16, 0, 16):
        for j in range(9):
            y = shy - 40 - j * 38
            g.append(f"<path d='M{shx+col+ (col*j*.6):.1f},{y} l0,-16' stroke='{P['amber']}' stroke-width='4' stroke-linecap='round' opacity='{1 - j*.08:.2f}'/>")
    # W: bullet-clearing zone
    zx, zy = 1040, 560
    g.append(f"<circle cx='{zx}' cy='{zy}' r='80' fill='{P['oni']}' fill-opacity='.12' stroke='{P['oni']}' stroke-width='3'/>")
    r = random.Random(4)
    for _ in range(14):
        a, rr = r.uniform(0, 6.28), r.uniform(10, 70)
        g.append(f"<path d='M{zx+math.cos(a)*rr:.1f},{zy+math.sin(a)*rr:.1f} l3,-3 l3,3 l-3,3 Z' fill='{P['oni']}'/>")
    g.append(pill(zx, zy - 100, "W — clears bullets in a zone", P["oni"]))
    # companion shield ring + tiny ship + hitbox
    g.append(f"<circle cx='{shx}' cy='{shy}' r='44' fill='none' stroke='{P['oni']}' stroke-width='2' stroke-dasharray='6 5'/>")
    g.append(ship_striker(shx, shy, .42))
    g.append(f"<circle cx='{shx}' cy='{shy}' r='5' fill='#fff' stroke='{P['red']}' stroke-width='2'/>")
    g.append(f"<path d='M{shx+10},{shy+6} L{shx+120},{shy+60}' stroke='{P['snow']}' stroke-width='1.5' opacity='.7'/>"
             + pill(shx + 250, shy + 62, "tiny hitbox — the ship is small now", P["snow"]))
    g.append(f"<g opacity='.65'>{ship_scout(560, 760, .38)}</g>" + text(560, 805, "squadmate", 12, P["snow"], anchor="middle", opacity=.6))
    b.append(f"<g clip-path='url(#arena)'>{''.join(g)}</g>")
    b.append(pill(mx + 250, my - 120, "MAW — rings + spirals", P["voidglow"]))
    b.append(pill(280, 375, "SWARMLINGS — aimed streams", P["voidglow"]))
    b.append(header("Sheet 3 (v2) — Bullet-hell battle", "Much smaller battleship weaving through dense void-monster bullet patterns; skills still on Q/W/E/R", dark=True))
    # ---- HUD
    hy = 830
    b.append(f"<rect x='0' y='{hy}' width='{W}' height='{H-hy}' fill='#0a0d1c' opacity='.94'/><path d='M0,{hy} L{W},{hy}' stroke='{P['hulldk']}' stroke-width='3'/>")
    b.append(f"<rect x='40' y='{hy+20}' width='130' height='130' rx='14' fill='{P['space2']}' stroke='{P['oni']}' stroke-width='3'/>")
    b.append(f"<clipPath id='pp'><rect x='40' y='{hy+20}' width='130' height='130' rx='14'/></clipPath>")
    b.append(f"<g clip-path='url(#pp)'>{oni(105, hy+240, .72, fur=P['ginger'], stripe='#b3601f', accessory=acc_pilot())}</g>")
    b.append(f"<rect x='200' y='{hy+30}' width='360' height='20' rx='4' fill='#331a1a'/><rect x='200' y='{hy+30}' width='230' height='20' rx='4' fill='#4fd07a'/>")
    b.append(f"<rect x='200' y='{hy+58}' width='360' height='12' rx='4' fill='#1a2a33'/><rect x='200' y='{hy+58}' width='150' height='12' rx='4' fill='{P['oni']}'/>")
    b.append(text(200, hy + 100, "HULL 64%  ·  SHIELD 42%  ·  GRAZE ×128", 14, P["snow"], opacity=.7))
    keys = [("Q", "Spread shot", 0), ("W", "Bullet clear", .6), ("E", "Phase dash", 0), ("R", "Nine-lives (ult)", 1.0)]
    for i, (key, name, cd) in enumerate(keys):
        kx = 640 + i * 120
        col = P["amber"] if key == "R" else P["oni"]
        b.append(f"<rect x='{kx}' y='{hy+25}' width='96' height='96' rx='12' fill='{P['space2']}' stroke='{col}' stroke-width='3'/>")
        b.append(f"<circle cx='{kx+48}' cy='{hy+73}' r='26' fill='{col}' opacity='.35'/>")
        if cd:
            b.append(f"<rect x='{kx}' y='{hy+25+96*(1-cd)}' width='96' height='{96*cd}' rx='12' fill='#000' opacity='.55'/>")
            b.append(text(kx + 48, hy + 82, f"{int(cd*12)}s", 22, P["snow"], "bold", "middle"))
        b.append(f"<rect x='{kx+4}' y='{hy+29}' width='26' height='24' rx='5' fill='#000' opacity='.6'/>" + text(kx + 17, hy + 47, key, 16, P["snow"], "bold", "middle"))
        b.append(text(kx + 48, hy + 145, name, 13, P["snow"], anchor="middle", opacity=.75))
    b.append(text(640, hy + 163, "Pilot skills by hand · movement scheme TBD (click-to-move vs. direct)", 12, P["snow"], opacity=.5))
    b.append(text(1140, hy + 30, "Companions (auto-trigger)", 13, P["snow"], "bold", opacity=.8))
    comp = [("Engineer", "shield @ HP&lt;40%", P["charcoal"], "#e0a040", "round"), ("Gunner", "homing burst @ in range", "#9a9aa8", P["hulldk"], "folded")]
    for i, (n, cond, fur, suit, ears) in enumerate(comp):
        cy = hy + 45 + i * 62
        b.append(f"<rect x='1140' y='{cy}' width='54' height='54' rx='10' fill='{P['space2']}' stroke='{suit}' stroke-width='3'/>")
        b.append(f"<clipPath id='c{i}'><rect x='1140' y='{cy}' width='54' height='54' rx='10'/></clipPath><g clip-path='url(#c{i})'>{oni(1167, cy+100, .33, fur=fur, suit=suit, ears=ears)}</g>")
        b.append(text(1206, cy + 22, n, 14, P["snow"], "bold") + text(1206, cy + 42, cond, 12, P["oni"]))
    b.append(f"<rect x='1420' y='{hy+20}' width='150' height='130' rx='8' fill='{P['space2']}' stroke='{P['hulldk']}' stroke-width='3'/>")
    b.append(f"<circle cx='1495' cy='{hy+120}' r='4' fill='{P['oni']}'/><circle cx='1495' cy='{hy+50}' r='8' fill='{P['voidglow']}'/>")
    b.append(text(1495, hy + 90, "WAVE 3 / 5", 14, P["snow"], "bold", "middle"))
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
