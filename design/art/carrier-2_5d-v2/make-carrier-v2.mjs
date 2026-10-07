import { mkdir, writeFile, copyFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawn } from "node:child_process";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../..");
const out = here;
const shipped = resolve(root, "assets/source/core/carrier");
const oldMock = resolve(root, "design/art/carrier-2_5d/mockup-starting-layout.png");
const chrome =
  process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

const masks = ["n", "e", "s", "w", "ns", "ew", "ne", "es", "sw", "nw", "nes", "esw", "nsw", "new", "nesw"];

const files = [];

const palette = {
  hull0: "#102b31",
  hull1: "#173f45",
  hull2: "#245b5d",
  floor0: "#547b73",
  floor1: "#78998c",
  floor2: "#a4bca9",
  teal: "#39d5c3",
  tealDim: "#1e807e",
  amber: "#f3b15c",
  amber2: "#ffd58c",
  red: "#cc6948",
  violet: "#6d4bc6",
  shadow: "#071316",
  ink: "#061012",
};

function esc(s) {
  return String(s).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

function svgDoc(w, h, body, extra = "") {
  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">
<defs>
  <filter id="paint" x="-10%" y="-10%" width="120%" height="120%">
    <feTurbulence type="fractalNoise" baseFrequency="0.018" numOctaves="3" seed="43" result="noise"/>
    <feColorMatrix in="noise" type="matrix" values="0.08 0 0 0 0  0 0.08 0 0 0  0 0 0.08 0 0  0 0 0 0.35 0" result="grain"/>
    <feBlend in="SourceGraphic" in2="grain" mode="multiply"/>
  </filter>
  <filter id="glow" x="-60%" y="-60%" width="220%" height="220%">
    <feGaussianBlur stdDeviation="4" result="blur"/>
    <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
  </filter>
  <linearGradient id="floorGrad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="${palette.floor2}"/>
    <stop offset="0.45" stop-color="${palette.floor1}"/>
    <stop offset="1" stop-color="${palette.floor0}"/>
  </linearGradient>
  <linearGradient id="wallGrad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="${palette.hull2}"/>
    <stop offset="0.5" stop-color="${palette.hull1}"/>
    <stop offset="1" stop-color="${palette.hull0}"/>
  </linearGradient>
  <linearGradient id="darkMetal" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0" stop-color="#28484c"/>
    <stop offset="1" stop-color="#0b2025"/>
  </linearGradient>
  <pattern id="floorGrid" width="64" height="64" patternUnits="userSpaceOnUse">
    <path d="M64 0H0V64" fill="none" stroke="#2f605d" stroke-width="2" opacity="0.32"/>
  </pattern>
  <pattern id="hatch" width="16" height="16" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
    <rect width="16" height="16" fill="transparent"/>
    <path d="M0 0V16" stroke="#f1ba61" stroke-width="4" opacity="0.45"/>
  </pattern>
  ${extra}
</defs>
${body}
</svg>`;
}

function rect(x, y, w, h, fill, attrs = "") {
  return `<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="${fill}" ${attrs}/>`;
}

function line(x1, y1, x2, y2, stroke, sw = 4, attrs = "") {
  return `<path d="M${x1} ${y1}L${x2} ${y2}" fill="none" stroke="${stroke}" stroke-width="${sw}" ${attrs}/>`;
}

function dotGrid(w, h, seed = 1) {
  let s = "";
  for (let y = 28; y < h; y += 50) {
    for (let x = 24; x < w; x += 55) {
      const r = ((x * 17 + y * 31 + seed) % 5) + 2;
      s += `<circle cx="${x + ((x + y) % 9)}" cy="${y}" r="${r / 2}" fill="#092126" opacity="0.38"/>`;
    }
  }
  return s;
}

function floorPanels(x, y, w, h, seed = 0) {
  let s = rect(x, y, w, h, "url(#floorGrad)", `rx="8" filter="url(#paint)"`);
  s += rect(x, y, w, h, "url(#floorGrid)", `rx="8" opacity="0.5"`);
  for (let yy = y + 44; yy < y + h - 20; yy += 86) {
    s += line(x + 22, yy, x + w - 22, yy + ((yy + seed) % 17) - 8, "#386964", 2, `opacity="0.45"`);
  }
  for (let xx = x + 55; xx < x + w - 20; xx += 94) {
    s += line(xx, y + 20, xx + ((xx + seed) % 15) - 7, y + h - 24, "#2f5f5d", 2, `opacity="0.35"`);
  }
  return s;
}

function roomShell(w, h, name, seed = 1) {
  const top = 96;
  const side = 24;
  const lip = 24;
  let s = rect(0, 0, w, h, palette.ink);
  s += rect(0, 0, w, h, "url(#wallGrad)", `filter="url(#paint)"`);
  s += floorPanels(side, top, w - side * 2, h - top - lip, seed);
  s += rect(0, 0, w, top, "url(#darkMetal)", `filter="url(#paint)"`);
  s += rect(0, 0, side, h, "#0b252a", `opacity="0.92"`);
  s += rect(w - side, 0, side, h, "#0b252a", `opacity="0.92"`);
  s += rect(0, h - lip, w, lip, "#061619", `opacity="0.8"`);
  s += rect(26, 18, w - 52, 18, "#2b6a68", `rx="9" opacity="0.65"`);
  s += rect(46, 32, w - 92, 6, palette.teal, `rx="3" opacity="0.55" filter="url(#glow)"`);
  s += dotGrid(w, h, seed);
  for (let x = 54; x < w - 30; x += 72) {
    s += `<circle cx="${x}" cy="42" r="5" fill="${palette.amber2}" opacity="0.72" filter="url(#glow)"/>`;
  }
  return s;
}

function wallSocket(side, x, y) {
  if (side === "s") return `${rect(x - 64, y - 12, 128, 24, "#203c40", `rx="4"`)}${rect(x - 54, y - 8, 108, 10, palette.floor1, `rx="3"`)}${line(x - 50, y - 18, x + 50, y - 18, palette.amber, 4, `opacity="0.65"`)}`;
  if (side === "n") return `${rect(x - 64, y - 2, 128, 26, "#203c40", `rx="4"`)}${rect(x - 54, y + 8, 108, 10, palette.floor1, `rx="3"`)}${line(x - 50, y + 28, x + 50, y + 28, palette.amber, 4, `opacity="0.55"`)}`;
  if (side === "e") return `${rect(x - 12, y - 64, 24, 128, "#203c40", `rx="4"`)}${rect(x - 2, y - 54, 10, 108, palette.floor1, `rx="3"`)}${line(x - 20, y - 50, x - 20, y + 50, palette.amber, 4, `opacity="0.6"`)}`;
  return `${rect(x - 12, y - 64, 24, 128, "#203c40", `rx="4"`)}${rect(x - 8, y - 54, 10, 108, palette.floor1, `rx="3"`)}${line(x + 20, y - 50, x + 20, y + 50, palette.amber, 4, `opacity="0.6"`)}`;
}

function consoleBank(x, y, w = 150, h = 78) {
  let s = rect(x, y, w, h, "#153941", `rx="10" filter="url(#paint)"`);
  s += rect(x + 12, y + 10, w - 24, 26, "#123039", `rx="5"`);
  s += line(x + 20, y + 26, x + w - 28, y + 17, palette.teal, 3, `opacity="0.8" filter="url(#glow)"`);
  s += `<circle cx="${x + 32}" cy="${y + h - 22}" r="8" fill="${palette.amber}" filter="url(#glow)"/>`;
  s += `<circle cx="${x + 58}" cy="${y + h - 22}" r="5" fill="${palette.teal}" filter="url(#glow)"/>`;
  s += rect(x + w - 42, y + h - 36, 24, 22, palette.violet, `rx="5" opacity="0.75"`);
  return s;
}

function catDecal(x, y, scale = 1, color = "#ffd58c") {
  return `<g transform="translate(${x} ${y}) scale(${scale})" opacity="0.75">
    <path d="M-16 -7L-8 -18L-2 -8L8 -8L14 -18L20 -5C23 10 15 22 2 23C-13 23 -23 10 -16 -7Z" fill="${color}"/>
    <circle cx="-6" cy="4" r="2.4" fill="#17313a"/><circle cx="9" cy="4" r="2.4" fill="#17313a"/>
    <path d="M0 12Q3 15 7 12" fill="none" stroke="#17313a" stroke-width="2"/>
  </g>`;
}

function bridge() {
  let s = roomShell(768, 512, "BRIDGE", 11);
  s += wallSocket("s", 384, 512);
  s += wallSocket("e", 768, 128);
  s += `<ellipse cx="390" cy="132" rx="150" ry="55" fill="#13323b" opacity="0.92"/>`;
  s += `<ellipse cx="390" cy="126" rx="112" ry="35" fill="#1a4d59" opacity="0.88"/>`;
  s += `<ellipse cx="390" cy="126" rx="86" ry="22" fill="none" stroke="${palette.teal}" stroke-width="5" opacity="0.75" filter="url(#glow)"/>`;
  s += line(318, 122, 462, 132, palette.teal, 2, `opacity="0.85"`);
  s += line(360, 100, 430, 150, palette.amber2, 2, `opacity="0.7"`);
  s += consoleBank(70, 135, 160, 84);
  s += consoleBank(535, 145, 150, 78);
  s += consoleBank(288, 310, 180, 80);
  s += catDecal(126, 66, 0.9);
  s += `<path d="M62 372C132 342 218 368 250 426" fill="none" stroke="#1b5151" stroke-width="10" opacity="0.55"/>`;
  s += `<path d="M518 350C606 320 672 354 708 420" fill="none" stroke="#1b5151" stroke-width="8" opacity="0.55"/>`;
  return svgDoc(768, 512, s);
}

function crewQuarters() {
  let s = roomShell(512, 512, "CREW", 23);
  s += wallSocket("s", 128, 512);
  s += wallSocket("w", 0, 128);
  s += wallSocket("e", 512, 128);
  for (const [x, y] of [[52, 120], [294, 120], [52, 228], [294, 228]]) {
    s += rect(x, y, 150, 72, "#22464c", `rx="12" filter="url(#paint)"`);
    s += rect(x + 18, y + 12, 48, 34, "#d7c6ad", `rx="9"`);
    s += rect(x + 70, y + 12, 64, 46, "#5e8f86", `rx="8"`);
    s += catDecal(x + 132, y + 54, 0.32, palette.amber2);
  }
  s += rect(222, 384, 72, 52, "#244b50", `rx="10"`);
  s += `<circle cx="256" cy="410" r="24" fill="#82aa98" opacity="0.85"/>`;
  s += rect(368, 330, 70, 88, "#16383d", `rx="10"`);
  s += `<path d="M384 380C410 320 438 352 420 405" fill="#3e765f" opacity="0.9"/>`;
  s += line(82, 88, 430, 86, palette.amber, 5, `opacity="0.42" filter="url(#glow)"`);
  return svgDoc(512, 512, s);
}

function workshop() {
  let s = roomShell(768, 512, "WORKSHOP", 37);
  s += wallSocket("n", 384, 0);
  s += wallSocket("e", 768, 384);
  s += rect(520, 116, 184, 112, "#203f44", `rx="12" filter="url(#paint)"`);
  s += rect(540, 134, 144, 18, palette.amber, `rx="6" opacity="0.6" filter="url(#glow)"`);
  for (let x = 548; x <= 670; x += 34) s += `<circle cx="${x}" cy="182" r="8" fill="${x % 2 ? palette.teal : palette.amber2}" opacity="0.75"/>`;
  s += rect(68, 136, 210, 126, "#173940", `rx="10"`);
  for (let x = 90; x < 250; x += 34) for (let y = 158; y < 240; y += 30) s += `<circle cx="${x}" cy="${y}" r="3" fill="#82b6a6" opacity="0.65"/>`;
  s += `<path d="M95 195H168L158 230H108Z" fill="#b96d4d" opacity="0.9"/>`;
  s += `<path d="M198 174L238 214" stroke="${palette.amber2}" stroke-width="8" stroke-linecap="round"/>`;
  s += rect(340, 286, 170, 92, "#24494d", `rx="11" filter="url(#paint)"`);
  s += rect(356, 302, 140, 22, "#73533a", `rx="6"`);
  s += `<path d="M398 350C430 320 462 340 474 374" fill="none" stroke="${palette.teal}" stroke-width="5" opacity="0.55"/>`;
  s += `<path d="M116 344C188 320 238 355 300 326" fill="none" stroke="#203c44" stroke-width="12" opacity="0.75"/>`;
  s += catDecal(686, 74, 0.7);
  return svgDoc(768, 512, s);
}

function salvageBay() {
  let s = roomShell(512, 512, "SALVAGE", 41);
  s += wallSocket("n", 128, 0) + wallSocket("e", 512, 128) + wallSocket("s", 384, 512) + wallSocket("w", 0, 384);
  s += rect(58, 140, 140, 130, "#2d4a46", `rx="12" filter="url(#paint)"`);
  for (let i = 0; i < 16; i++) {
    const x = 76 + (i % 4) * 28;
    const y = 158 + Math.floor(i / 4) * 24;
    s += `<path d="M${x} ${y}l22 9l-18 16l-22 -8z" fill="${i % 3 === 0 ? palette.amber : i % 3 === 1 ? "#8a7560" : palette.tealDim}" opacity="0.8"/>`;
  }
  s += `<path d="M312 116V238" stroke="#1a353a" stroke-width="16" stroke-linecap="round"/>`;
  s += `<path d="M244 122H376" stroke="#28545b" stroke-width="16" stroke-linecap="round"/>`;
  s += `<path d="M344 136C354 192 326 226 286 254" fill="none" stroke="${palette.amber2}" stroke-width="7" opacity="0.7"/>`;
  s += `<path d="M274 262Q302 300 338 262" fill="none" stroke="${palette.red}" stroke-width="10" stroke-linecap="round"/>`;
  s += rect(298, 334, 130, 62, "#1a383c", `rx="10"`);
  s += line(318, 365, 408, 350, palette.teal, 4, `opacity="0.75" filter="url(#glow)"`);
  s += catDecal(103, 84, 0.65);
  return svgDoc(512, 512, s);
}

function trainingRoom() {
  let s = roomShell(512, 512, "TRAINING", 53);
  s += wallSocket("n", 128, 0) + wallSocket("e", 512, 128) + wallSocket("s", 384, 512) + wallSocket("w", 0, 384);
  s += rect(70, 148, 142, 210, "#1b3c44", `rx="20" filter="url(#paint)"`);
  s += rect(92, 170, 98, 150, "#245866", `rx="18" opacity="0.85"`);
  s += `<ellipse cx="256" cy="292" rx="72" ry="34" fill="none" stroke="${palette.teal}" stroke-width="7" opacity="0.85" filter="url(#glow)"/>`;
  s += `<circle cx="256" cy="254" r="48" fill="none" stroke="${palette.amber2}" stroke-width="5" opacity="0.8" filter="url(#glow)"/>`;
  s += `<path d="M226 254H286M256 224V284" stroke="${palette.amber2}" stroke-width="5" opacity="0.9"/>`;
  s += consoleBank(334, 176, 112, 70);
  s += rect(336, 338, 106, 62, "#203f45", `rx="12"`);
  s += `<path d="M354 370H424" stroke="${palette.red}" stroke-width="7" opacity="0.75"/>`;
  s += catDecal(430, 74, 0.62);
  return svgDoc(512, 512, s);
}

function dockRoom() {
  let s = roomShell(512, 768, "DOCK", 67);
  s += wallSocket("w", 0, 640) + wallSocket("e", 512, 640) + wallSocket("s", 128, 768);
  s += rect(56, 120, 400, 386, "#17363c", `rx="18" filter="url(#paint)"`);
  s += rect(86, 154, 340, 292, "#264d54", `rx="20" opacity="0.9"`);
  s += `<ellipse cx="256" cy="300" rx="128" ry="74" fill="#365e62" opacity="0.85"/>`;
  s += `<ellipse cx="256" cy="300" rx="102" ry="52" fill="none" stroke="${palette.teal}" stroke-width="7" opacity="0.75" filter="url(#glow)"/>`;
  s += `<path d="M104 530H408" stroke="${palette.amber}" stroke-width="12" opacity="0.6"/>`;
  s += `<path d="M118 576H394" stroke="#244c50" stroke-width="18" stroke-dasharray="28 18" opacity="0.9"/>`;
  s += rect(70, 596, 112, 82, "#1a383e", `rx="10"`);
  s += rect(330, 594, 104, 78, "#1a383e", `rx="10"`);
  s += line(96, 124, 416, 500, "#203f45", 8, `opacity="0.6"`);
  s += line(416, 124, 96, 500, "#203f45", 8, `opacity="0.45"`);
  s += catDecal(86, 74, 0.6);
  return svgDoc(512, 768, s);
}

function berth() {
  let s = rect(0, 0, 480, 244, "transparent");
  s += rect(20, 16, 440, 210, "#112b31", `rx="24" filter="url(#paint)"`);
  s += rect(46, 38, 388, 164, "#315b61", `rx="22" opacity="0.9"`);
  s += `<ellipse cx="240" cy="121" rx="150" ry="70" fill="none" stroke="${palette.teal}" stroke-width="8" opacity="0.72" filter="url(#glow)"/>`;
  s += `<path d="M88 122H392" stroke="${palette.amber}" stroke-width="8" stroke-dasharray="24 18" opacity="0.75"/>`;
  s += `<path d="M116 68C188 28 294 30 364 68" fill="none" stroke="#193a40" stroke-width="18" opacity="0.65"/>`;
  s += catDecal(415, 55, 0.45);
  return svgDoc(480, 244, s);
}

function corridor(mask) {
  let s = rect(0, 0, 256, 256, "#071316");
  s += rect(0, 0, 256, 256, "url(#darkMetal)", `filter="url(#paint)"`);
  const has = (d) => mask.includes(d);
  const lane = "#88a998";
  const edge = palette.teal;
  s += rect(64, 80, 128, 96, lane, `rx="10" filter="url(#paint)"`);
  if (has("n")) s += rect(64, 0, 128, 96, lane, `filter="url(#paint)"`);
  if (has("s")) s += rect(64, 160, 128, 96, lane, `filter="url(#paint)"`);
  if (has("e")) s += rect(160, 80, 96, 96, lane, `filter="url(#paint)"`);
  if (has("w")) s += rect(0, 80, 96, 96, lane, `filter="url(#paint)"`);
  s += rect(70, 86, 116, 84, "url(#floorGrid)", `opacity="0.42"`);
  for (const [x1, y1, x2, y2] of [[64,80,192,80],[64,176,192,176],[64,80,64,176],[192,80,192,176]]) s += line(x1,y1,x2,y2,edge,4,`opacity="0.65" filter="url(#glow)"`);
  if (has("n")) { s += line(64, 0, 64, 82, edge, 4, `opacity="0.65" filter="url(#glow)"`); s += line(192, 0, 192, 82, edge, 4, `opacity="0.65" filter="url(#glow)"`); }
  if (has("s")) { s += line(64, 174, 64, 256, edge, 4, `opacity="0.65" filter="url(#glow)"`); s += line(192, 174, 192, 256, edge, 4, `opacity="0.65" filter="url(#glow)"`); }
  if (has("e")) { s += line(174, 80, 256, 80, edge, 4, `opacity="0.65" filter="url(#glow)"`); s += line(174, 176, 256, 176, edge, 4, `opacity="0.65" filter="url(#glow)"`); }
  if (has("w")) { s += line(0, 80, 82, 80, edge, 4, `opacity="0.65" filter="url(#glow)"`); s += line(0, 176, 82, 176, edge, 4, `opacity="0.65" filter="url(#glow)"`); }
  s += `<path d="M20 34C70 18 104 38 142 24S214 24 238 50" fill="none" stroke="#15373d" stroke-width="11" opacity="0.72"/>`;
  s += `<path d="M22 216C72 202 126 224 176 204S224 204 244 222" fill="none" stroke="#173f45" stroke-width="9" opacity="0.8"/>`;
  for (const [x, y] of [[38, 54], [218, 56], [38, 206], [218, 206]]) s += `<circle cx="${x}" cy="${y}" r="8" fill="${palette.amber2}" opacity="0.72" filter="url(#glow)"/>`;
  s += rect(18, 18, 46, 14, "url(#hatch)", `opacity="0.75"`);
  s += rect(190, 224, 46, 14, "url(#hatch)", `opacity="0.75"`);
  return svgDoc(256, 256, s);
}

function door(side) {
  const w = side === "e" || side === "w" ? 24 : 256;
  const h = side === "n" ? 96 : side === "s" ? 24 : 256;
  let s = rect(0, 0, w, h, "transparent");
  if (side === "n") {
    s += rect(0, 0, 256, 96, "#0d272c", `filter="url(#paint)"`);
    s += rect(64, 36, 128, 44, "#7f9d91", `rx="7"`);
    s += line(78, 31, 178, 31, palette.amber2, 5, `opacity="0.72" filter="url(#glow)"`);
  } else if (side === "s") {
    s += rect(0, 0, 256, 24, "#092025", `filter="url(#paint)"`);
    s += rect(64, 0, 128, 18, "#7f9d91", `rx="4"`);
    s += line(78, 20, 178, 20, palette.amber2, 3, `opacity="0.65" filter="url(#glow)"`);
  } else {
    s += rect(0, 0, 24, 256, "#092025", `filter="url(#paint)"`);
    s += rect(3, 64, 18, 128, "#7f9d91", `rx="4"`);
    s += line(side === "e" ? 3 : 21, 78, side === "e" ? 3 : 21, 178, palette.amber2, 3, `opacity="0.65" filter="url(#glow)"`);
  }
  return svgDoc(w, h, s);
}

function hullFloor() {
  let s = rect(0, 0, 256, 256, "#071013");
  s += rect(0, 0, 256, 256, "#10282d", `filter="url(#paint)"`);
  for (let y = 0; y < 256; y += 64) s += line(0, y, 256, y + 22, "#1a373d", 3, `opacity="0.45"`);
  for (let x = 0; x < 256; x += 64) s += line(x, 0, x - 20, 256, "#0b2025", 4, `opacity="0.45"`);
  s += `<path d="M20 34H236M32 214H224" stroke="#071316" stroke-width="8" opacity="0.5"/>`;
  s += rect(32, 34, 52, 14, "url(#hatch)", `opacity="0.22"`);
  return svgDoc(256, 256, s);
}

function buildSlot() {
  let s = rect(0, 0, 256, 256, "transparent");
  s += rect(44, 44, 168, 168, "#17343b", `rx="22" opacity="0.78" filter="url(#paint)"`);
  s += rect(58, 58, 140, 140, "none", `rx="18" stroke="${palette.teal}" stroke-width="7" stroke-dasharray="22 15" opacity="0.82" filter="url(#glow)"`);
  s += `<path d="M128 84V172M84 128H172" stroke="${palette.amber2}" stroke-width="8" stroke-linecap="round" opacity="0.82"/>`;
  s += catDecal(181, 78, 0.42);
  return svgDoc(256, 256, s);
}

function imageHref(path) {
  return pathToFileURL(path).href;
}

function mockupSvg() {
  const cell = 256;
  const W = 3072;
  const H = 2048;
  const img = (href, x, y, w, h) => `<image href="${imageHref(href)}" x="${x}" y="${y}" width="${w}" height="${h}"/>`;
  let s = rect(0, 0, W, H, "#071015");
  for (let y = 0; y < 8; y++) {
    for (let x = 0; x < 12; x++) s += img(resolve(out, "hull-floor.png"), x * cell, y * cell, cell, cell);
  }
  const rooms = [
    ["room-bridge.png", 0, 1, 768, 512],
    ["room-crew-quarters.png", 4, 1, 512, 512],
    ["room-workshop.png", 0, 4, 768, 512],
    ["room-dock.png", 4, 4, 512, 768],
    ["room-dock.png", 6, 4, 512, 768],
  ];
  const corridors = [
    ["nes", 1, 3], ["ew", 2, 3], ["esw", 3, 3], ["nw", 4, 3],
    ["ns", 3, 4], ["nsw", 3, 5], ["ne", 3, 6], ["ne", 4, 7], ["ew", 5, 7], ["nw", 6, 7],
  ];
  for (const [name, x, y] of corridors) s += img(resolve(out, `corridor-${name}.png`), x * cell, y * cell, cell, cell);
  for (const [name, x, y, w, h] of rooms) s += img(resolve(out, name), x * cell, y * cell, w, h);
  s += img(resolve(out, "dock-berth.png"), 4 * cell + 16, 4 * cell + 40, 480, 244);
  s += img(resolve(out, "dock-berth.png"), 6 * cell + 16, 4 * cell + 40, 480, 244);
  s += `<g opacity="0.95">
    <path d="M1092 1150C1164 1102 1276 1112 1326 1180L1278 1262H1120Z" fill="#46b5b0" stroke="#16363c" stroke-width="12"/>
    <path d="M1604 1148C1692 1100 1790 1120 1840 1190L1785 1268H1620Z" fill="#9f8163" stroke="#16363c" stroke-width="12"/>
    <circle cx="1192" cy="1182" r="18" fill="${palette.amber2}" filter="url(#glow)"/>
    <circle cx="1710" cy="1182" r="18" fill="${palette.amber2}" filter="url(#glow)"/>
  </g>`;
  s += rect(0, 0, W, H, "none", `stroke="#2f6365" stroke-width="8" opacity="0.6"`);
  return svgDoc(W, H, s);
}

function stripSvg() {
  const W = 3072;
  const H = 1024;
  const newMock = resolve(out, "mockup-starting-layout.png");
  let s = rect(0, 0, W, H, "#071015");
  s += `<image href="${imageHref(oldMock)}" x="0" y="0" width="1536" height="1024"/>`;
  s += `<image href="${imageHref(newMock)}" x="1536" y="0" width="1536" height="1024"/>`;
  s += rect(0, 0, 1536, H, "#000", `opacity="0.14"`);
  s += rect(1536, 0, 1536, H, "#000", `opacity="0.02"`);
  s += line(1536, 0, 1536, H, "#2d6968", 8, `opacity="0.85"`);
  s += rect(0, 0, W, H, "none", `stroke="#2d6968" stroke-width="8" opacity="0.65"`);
  return svgDoc(W, H, s);
}

async function saveSvg(name, w, h, contents) {
  const svgPath = resolve(out, `${name}.svg`);
  await writeFile(svgPath, contents, "utf8");
  files.push({ name, svgPath, pngPath: resolve(out, `${name}.png`), w, h });
}

function render(file) {
  return new Promise((resolvePromise, reject) => {
    const args = [
      "--headless=new",
      "--disable-gpu",
      "--hide-scrollbars",
      "--force-device-scale-factor=1",
      "--default-background-color=00000000",
      `--window-size=${file.w},${file.h}`,
      `--screenshot=${file.pngPath}`,
      pathToFileURL(file.svgPath).href,
    ];
    const child = spawn(chrome, args, { stdio: "ignore" });
    child.on("exit", (code) => {
      if (code === 0 && existsSync(file.pngPath)) resolvePromise();
      else reject(new Error(`Chrome failed to render ${file.name} with code ${code}`));
    });
    child.on("error", reject);
  });
}

async function main() {
  await mkdir(out, { recursive: true });
  await mkdir(resolve(shipped, "room"), { recursive: true });
  await mkdir(resolve(shipped, "corridor"), { recursive: true });
  await mkdir(resolve(shipped, "door"), { recursive: true });
  await mkdir(resolve(shipped, "dock"), { recursive: true });

  await saveSvg("room-bridge", 768, 512, bridge());
  await saveSvg("room-crew-quarters", 512, 512, crewQuarters());
  await saveSvg("room-workshop", 768, 512, workshop());
  await saveSvg("room-salvage-bay", 512, 512, salvageBay());
  await saveSvg("room-training-room", 512, 512, trainingRoom());
  await saveSvg("room-dock", 512, 768, dockRoom());
  await saveSvg("dock-berth", 480, 244, berth());
  for (const mask of masks) await saveSvg(`corridor-${mask}`, 256, 256, corridor(mask));
  for (const side of ["n", "e", "s", "w"]) {
    const w = side === "e" || side === "w" ? 24 : 256;
    const h = side === "n" ? 96 : side === "s" ? 24 : 256;
    await saveSvg(`door-${side}`, w, h, door(side));
  }
  await saveSvg("hull-floor", 256, 256, hullFloor());
  await saveSvg("build-slot", 256, 256, buildSlot());

  for (const file of files) await render(file);

  const copies = [
    ["room-bridge.png", "room/bridge.png"],
    ["room-crew-quarters.png", "room/crew_quarters.png"],
    ["room-workshop.png", "room/workshop.png"],
    ["room-salvage-bay.png", "room/salvage_bay.png"],
    ["room-training-room.png", "room/training_room.png"],
    ["room-dock.png", "room/dock.png"],
    ["dock-berth.png", "dock/berth.png"],
    ["hull-floor.png", "hull_floor.png"],
    ["build-slot.png", "build_slot.png"],
  ];
  for (const mask of masks) copies.push([`corridor-${mask}.png`, `corridor/${mask}.png`]);
  for (const side of ["n", "e", "s", "w"]) copies.push([`door-${side}.png`, `door/${side}.png`]);
  for (const [src, dest] of copies) await copyFile(resolve(out, src), resolve(shipped, dest));

  const mock = { name: "mockup-starting-layout", svgPath: resolve(out, "mockup-starting-layout.svg"), pngPath: resolve(out, "mockup-starting-layout.png"), w: 3072, h: 2048 };
  await writeFile(mock.svgPath, mockupSvg(), "utf8");
  await render(mock);

  const strip = { name: "before-after-strip", svgPath: resolve(out, "before-after-strip.svg"), pngPath: resolve(out, "before-after-strip.png"), w: 3072, h: 1024 };
  await writeFile(strip.svgPath, stripSvg(), "utf8");
  await render(strip);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
