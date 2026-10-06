import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../../..");
const assetRoot = resolve(root, "assets/source/core/carrier/pilot");
const chrome = process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

const W = 165;
const H = 192;
const frames = ["idle", "walk_1", "walk_2", "walk_3", "walk_4"];
const dirs = ["s", "se", "e", "ne", "n"];

for (const dir of dirs) {
  mkdirSync(join(assetRoot, dir), { recursive: true });
  mkdirSync(join(here, "frames", dir), { recursive: true });
}

const colors = {
  outline: "#132638",
  outlineSoft: "#234159",
  fur: "#f58436",
  furDark: "#c65b24",
  furLight: "#ffb35d",
  cream: "#fee2b8",
  blush: "#ff8f8a",
  suit: "#087c83",
  suitDark: "#05545f",
  suitLight: "#20b5b0",
  boot: "#33485b",
  bootDark: "#1c2f3d",
  goggleBand: "#243243",
  lens: "#43cce8",
  lensShade: "#176f8c",
  pack: "#1c394a",
  brass: "#ffc85a",
};

function svgHeader(width, height) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" shape-rendering="geometricPrecision">`;
}

function defs() {
  return `<defs>
  <filter id="shadow" x="-30%" y="-30%" width="160%" height="170%">
    <feDropShadow dx="0" dy="2" stdDeviation="1.6" flood-color="#07131b" flood-opacity=".28"/>
  </filter>
  <linearGradient id="furGrad" x1="0" y1="20" x2="0" y2="120" gradientUnits="userSpaceOnUse">
    <stop offset="0" stop-color="${colors.furLight}"/>
    <stop offset=".62" stop-color="${colors.fur}"/>
    <stop offset="1" stop-color="${colors.furDark}"/>
  </linearGradient>
  <linearGradient id="suitGrad" x1="0" y1="98" x2="0" y2="184" gradientUnits="userSpaceOnUse">
    <stop offset="0" stop-color="${colors.suitLight}"/>
    <stop offset=".45" stop-color="${colors.suit}"/>
    <stop offset="1" stop-color="${colors.suitDark}"/>
  </linearGradient>
  <radialGradient id="lensGrad" cx=".4" cy=".25" r=".8">
    <stop offset="0" stop-color="#aaf5ff"/>
    <stop offset=".48" stop-color="${colors.lens}"/>
    <stop offset="1" stop-color="${colors.lensShade}"/>
  </radialGradient>
</defs>`;
}

function limbOffsets(frame) {
  if (frame === "idle") return { bob: 0, armA: 0, armB: 0, legA: 0, legB: 0, tail: 0 };
  const i = Number(frame.at(-1)) - 1;
  const wave = [0, 1, 0, -1][i];
  const alt = [1, -1, -1, 1][i];
  return { bob: wave * 2, armA: alt * 5, armB: -alt * 5, legA: alt * 5, legB: -alt * 5, tail: wave * 3 };
}

function catFrame(dir, frame, ox = 0, oy = 0, scale = 1) {
  const m = limbOffsets(frame);
  const isN = dir === "n";
  const front = dir === "s";
  const diag = dir === "se" || dir === "ne";
  const east = dir === "e";
  const up = dir === "ne";
  const y = oy + m.bob * scale;
  const x = ox;
  const sx = scale;

  const headX = x + (east ? 91 : diag ? 88 : 82) * sx;
  const headY = y + (isN ? 52 : up ? 50 : 57) * sx;
  const bodyX = x + (east ? 78 : diag ? 78 : 82) * sx;
  const bodyY = y + 128 * sx;
  const faceCream = up
    ? `<ellipse cx="${x + 104 * sx}" cy="${y + 79 * sx}" rx="${14 * sx}" ry="${22 * sx}" fill="${colors.cream}" stroke="${colors.outline}" stroke-width="${3 * sx}" opacity=".9"/>`
    : front
    ? `<ellipse cx="${x + 82 * sx}" cy="${y + 78 * sx}" rx="${29 * sx}" ry="${31 * sx}" fill="${colors.cream}" stroke="${colors.outline}" stroke-width="${3 * sx}"/>`
    : diag
      ? `<ellipse cx="${x + 98 * sx}" cy="${y + 76 * sx}" rx="${24 * sx}" ry="${29 * sx}" fill="${colors.cream}" stroke="${colors.outline}" stroke-width="${3 * sx}"/>`
      : east
        ? `<ellipse cx="${x + 107 * sx}" cy="${y + 79 * sx}" rx="${25 * sx}" ry="${29 * sx}" fill="${colors.cream}" stroke="${colors.outline}" stroke-width="${3 * sx}"/>`
        : "";
  const eyes = up
    ? `<ellipse cx="${x + 109 * sx}" cy="${y + 78 * sx}" rx="${4.5 * sx}" ry="${8 * sx}" fill="${colors.outline}" opacity=".88"/><circle cx="${x + 110 * sx}" cy="${y + 75 * sx}" r="${1.4 * sx}" fill="#fff"/>`
    : front
    ? `<ellipse cx="${x + 66 * sx}" cy="${y + 76 * sx}" rx="${8 * sx}" ry="${13 * sx}" fill="${colors.outline}"/><circle cx="${x + 69 * sx}" cy="${y + 70 * sx}" r="${2.4 * sx}" fill="#fff"/>
       <ellipse cx="${x + 98 * sx}" cy="${y + 76 * sx}" rx="${8 * sx}" ry="${13 * sx}" fill="${colors.outline}"/><circle cx="${x + 101 * sx}" cy="${y + 70 * sx}" r="${2.4 * sx}" fill="#fff"/>`
    : diag
      ? `<ellipse cx="${x + 91 * sx}" cy="${y + 76 * sx}" rx="${7.8 * sx}" ry="${13 * sx}" fill="${colors.outline}"/><circle cx="${x + 94 * sx}" cy="${y + 70 * sx}" r="${2.4 * sx}" fill="#fff"/>
         <ellipse cx="${x + 113 * sx}" cy="${y + 78 * sx}" rx="${5.7 * sx}" ry="${10 * sx}" fill="${colors.outline}"/><circle cx="${x + 115 * sx}" cy="${y + 74 * sx}" r="${1.8 * sx}" fill="#fff"/>`
      : east
        ? `<ellipse cx="${x + 109 * sx}" cy="${y + 77 * sx}" rx="${9 * sx}" ry="${14 * sx}" fill="${colors.outline}"/><circle cx="${x + 112 * sx}" cy="${y + 71 * sx}" r="${2.5 * sx}" fill="#fff"/>`
        : "";
  const muzzle = isN || up
    ? ""
    : `<path d="M${x + (front ? 79 : diag ? 104 : 118) * sx} ${y + 93 * sx} q${7 * sx} ${6 * sx} ${15 * sx} 0" fill="none" stroke="${colors.outline}" stroke-width="${2.2 * sx}" stroke-linecap="round"/>
       <circle cx="${x + (front ? 82 : diag ? 107 : 121) * sx}" cy="${y + 88 * sx}" r="${2.4 * sx}" fill="${colors.blush}"/>`;
  const goggles = isN
    ? `<path d="M${x + 45 * sx} ${y + 49 * sx} q${37 * sx} -15 ${73 * sx} 0" fill="none" stroke="${colors.goggleBand}" stroke-width="${8 * sx}" stroke-linecap="round"/>
       <ellipse cx="${x + 65 * sx}" cy="${y + 44 * sx}" rx="${15 * sx}" ry="${9 * sx}" fill="url(#lensGrad)" stroke="${colors.outline}" stroke-width="${3 * sx}"/>
       <ellipse cx="${x + 99 * sx}" cy="${y + 44 * sx}" rx="${15 * sx}" ry="${9 * sx}" fill="url(#lensGrad)" stroke="${colors.outline}" stroke-width="${3 * sx}"/>`
    : `<path d="M${x + 39 * sx} ${y + 55 * sx} q${41 * sx} -23 ${83 * sx} 0" fill="none" stroke="${colors.goggleBand}" stroke-width="${8 * sx}" stroke-linecap="round"/>
       <ellipse cx="${x + (front ? 67 : diag ? 80 : 88) * sx}" cy="${y + 46 * sx}" rx="${16 * sx}" ry="${10 * sx}" transform="rotate(${front ? -7 : -13} ${x + 80 * sx} ${y + 46 * sx})" fill="url(#lensGrad)" stroke="${colors.outline}" stroke-width="${3 * sx}"/>
       <ellipse cx="${x + (front ? 99 : diag ? 110 : 116) * sx}" cy="${y + 48 * sx}" rx="${16 * sx}" ry="${10 * sx}" transform="rotate(${front ? 7 : 10} ${x + 105 * sx} ${y + 48 * sx})" fill="url(#lensGrad)" stroke="${colors.outline}" stroke-width="${3 * sx}"/>`;

  const ears = isN
    ? `<path d="M${x + 47 * sx} ${y + 39 * sx} L${x + 59 * sx} ${y + 10 * sx} L${x + 76 * sx} ${y + 43 * sx} Z" fill="url(#furGrad)" stroke="${colors.outline}" stroke-width="${3.5 * sx}"/>
       <path d="M${x + 88 * sx} ${y + 42 * sx} L${x + 106 * sx} ${y + 13 * sx} L${x + 119 * sx} ${y + 47 * sx} Z" fill="url(#furGrad)" stroke="${colors.outline}" stroke-width="${3.5 * sx}"/>`
    : `<path d="M${x + 45 * sx} ${y + 44 * sx} L${x + 58 * sx} ${y + 9 * sx} L${x + 79 * sx} ${y + 43 * sx} Z" fill="url(#furGrad)" stroke="${colors.outline}" stroke-width="${3.5 * sx}"/>
       <path d="M${x + 83 * sx} ${y + 43 * sx} L${x + 104 * sx} ${y + 14 * sx} L${x + 119 * sx} ${y + 52 * sx} Z" fill="url(#furGrad)" stroke="${colors.outline}" stroke-width="${3.5 * sx}"/>
       <path d="M${x + 57 * sx} ${y + 23 * sx} L${x + 64 * sx} ${y + 39 * sx} L${x + 50 * sx} ${y + 38 * sx} Z" fill="#ffc0ab" opacity=".75"/>
       <path d="M${x + 101 * sx} ${y + 28 * sx} L${x + 103 * sx} ${y + 45 * sx} L${x + 91 * sx} ${y + 42 * sx} Z" fill="#ffc0ab" opacity=".68"/>`;

  const tailX = east ? 38 : diag ? 43 : 31;
  const tail = `<path d="M${x + 49 * sx} ${y + 137 * sx} C${x + (tailX - 22) * sx} ${y + (112 + m.tail) * sx} ${x + (tailX - 11) * sx} ${y + (87 + m.tail) * sx} ${x + tailX * sx} ${y + 94 * sx} C${x + (tailX + 18) * sx} ${y + 104 * sx} ${x + (tailX + 12) * sx} ${y + 143 * sx} ${x + 50 * sx} ${y + 153 * sx}" fill="${colors.fur}" stroke="${colors.outline}" stroke-width="${4 * sx}"/>
    <path d="M${x + (tailX - 3) * sx} ${y + 105 * sx} q${13 * sx} ${13 * sx} ${7 * sx} ${30 * sx}" fill="none" stroke="${colors.furLight}" stroke-width="${2.5 * sx}" opacity=".5"/>`;
  const pack = (east || diag)
    ? `<rect x="${x + 41 * sx}" y="${y + 100 * sx}" width="${25 * sx}" height="${47 * sx}" rx="${7 * sx}" fill="${colors.pack}" stroke="${colors.outline}" stroke-width="${3 * sx}"/>
       <path d="M${x + 49 * sx} ${y + 107 * sx} v${31 * sx}" stroke="${colors.outlineSoft}" stroke-width="${2 * sx}"/>`
    : isN
      ? `<rect x="${x + 54 * sx}" y="${y + 91 * sx}" width="${55 * sx}" height="${52 * sx}" rx="${11 * sx}" fill="${colors.pack}" stroke="${colors.outline}" stroke-width="${3 * sx}"/>
         <path d="M${x + 65 * sx} ${y + 104 * sx} h${34 * sx} M${x + 65 * sx} ${y + 120 * sx} h${34 * sx}" stroke="${colors.outlineSoft}" stroke-width="${2 * sx}" opacity=".7"/>`
      : "";
  const legs = `<ellipse cx="${x + (66 + m.legA) * sx}" cy="${y + 174 * sx}" rx="${14 * sx}" ry="${8 * sx}" fill="${colors.bootDark}" stroke="${colors.outline}" stroke-width="${3 * sx}"/>
    <ellipse cx="${x + (99 + m.legB) * sx}" cy="${y + 174 * sx}" rx="${14 * sx}" ry="${8 * sx}" fill="${colors.bootDark}" stroke="${colors.outline}" stroke-width="${3 * sx}"/>`;
  const arms = `<ellipse cx="${x + (47 + m.armA) * sx}" cy="${y + 125 * sx}" rx="${10 * sx}" ry="${21 * sx}" fill="${colors.suit}" stroke="${colors.outline}" stroke-width="${3 * sx}" transform="rotate(${-18 + m.armA} ${x + (47 + m.armA) * sx} ${y + 125 * sx})"/>
    <circle cx="${x + (43 + m.armA) * sx}" cy="${y + 144 * sx}" r="${8 * sx}" fill="${colors.fur}" stroke="${colors.outline}" stroke-width="${3 * sx}"/>
    <ellipse cx="${x + (117 + m.armB) * sx}" cy="${y + 125 * sx}" rx="${10 * sx}" ry="${21 * sx}" fill="${colors.suit}" stroke="${colors.outline}" stroke-width="${3 * sx}" transform="rotate(${18 + m.armB} ${x + (117 + m.armB) * sx} ${y + 125 * sx})"/>
    <circle cx="${x + (121 + m.armB) * sx}" cy="${y + 144 * sx}" r="${8 * sx}" fill="${colors.fur}" stroke="${colors.outline}" stroke-width="${3 * sx}"/>`;

  return `<g filter="url(#shadow)">
    ${!isN ? tail : ""}
    ${pack}
    ${legs}
    <ellipse cx="${bodyX}" cy="${bodyY}" rx="${36 * sx}" ry="${48 * sx}" fill="url(#suitGrad)" stroke="${colors.outline}" stroke-width="${4 * sx}"/>
    <ellipse cx="${x + (front ? 82 : diag ? 91 : east ? 99 : 82) * sx}" cy="${y + 136 * sx}" rx="${20 * sx}" ry="${31 * sx}" fill="${isN ? colors.suitDark : colors.cream}" opacity="${isN ? ".55" : "1"}"/>
    ${arms}
    ${isN ? tail : ""}
    ${ears}
    <ellipse cx="${headX}" cy="${headY}" rx="${isN ? 43 : front ? 45 : diag ? 43 : 42 * sx}" ry="${isN ? 36 : front ? 40 : 39}" fill="url(#furGrad)" stroke="${colors.outline}" stroke-width="${4 * sx}"/>
    ${faceCream}
    <path d="M${x + 53 * sx} ${y + 63 * sx} q${18 * sx} 8 ${36 * sx} 0 M${x + 50 * sx} ${y + 76 * sx} q${18 * sx} 7 ${37 * sx} 0" fill="none" stroke="${colors.furDark}" stroke-width="${3 * sx}" opacity=".55" stroke-linecap="round"/>
    ${eyes}
    ${muzzle}
    ${goggles}
    <circle cx="${x + 72 * sx}" cy="${y + 104 * sx}" r="${5.5 * sx}" fill="${colors.brass}" stroke="${colors.outline}" stroke-width="${2.3 * sx}"/>
    <circle cx="${x + 92 * sx}" cy="${y + 104 * sx}" r="${5.5 * sx}" fill="${colors.brass}" stroke="${colors.outline}" stroke-width="${2.3 * sx}"/>
  </g>`;
}

function frameSvg(dir, frame) {
  return `${svgHeader(W, H)}
${defs()}
${catFrame(dir, frame)}
</svg>`;
}

for (const dir of dirs) {
  for (const frame of frames) {
    const svg = frameSvg(dir, frame);
    const svgPath = join(here, "frames", dir, `${frame}.svg`);
    const pngPath = join(assetRoot, dir, `${frame}.png`);
    writeFileSync(svgPath, svg);
    execFileSync(chrome, [
      "--headless=new",
      "--disable-gpu",
      "--hide-scrollbars",
      "--force-device-scale-factor=1",
      "--default-background-color=00000000",
      `--window-size=${W},${H}`,
      `--screenshot=${pngPath}`,
      `file://${svgPath}`,
    ], { stdio: "ignore" });
  }
}

const cellW = W;
const cellH = H;
const sheetW = cellW * frames.length;
const sheetH = cellH * dirs.length;
let sheet = `${svgHeader(sheetW, sheetH)}
${defs()}
<rect width="100%" height="100%" fill="#14232e"/>`;
for (let row = 0; row < dirs.length; row++) {
  for (let col = 0; col < frames.length; col++) {
    sheet += `<g transform="translate(${col * cellW},${row * cellH})">
      <rect x="0" y="0" width="${cellW}" height="${cellH}" fill="${(row + col) % 2 ? "#172a36" : "#1b3040"}"/>
      ${catFrame(dirs[row], frames[col])}
    </g>`;
  }
}
sheet += "</svg>";
const sheetSvg = join(here, "carrier-pilot-8dir-contact-sheet.svg");
const sheetPng = join(here, "carrier-pilot-8dir-contact-sheet.png");
writeFileSync(sheetSvg, sheet);
execFileSync(chrome, [
  "--headless=new",
  "--disable-gpu",
  "--hide-scrollbars",
  "--force-device-scale-factor=1",
  `--window-size=${sheetW},${sheetH}`,
  `--screenshot=${sheetPng}`,
  `file://${sheetSvg}`,
], { stdio: "ignore" });
