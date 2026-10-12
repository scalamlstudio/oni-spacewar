// Starter carrier for the furniture-and-stations interior (TAKOAI-88).
// Checks the layout against the placement rules in design/READINESS.md
// § Carrier and draws a rough diagram (not shipped art):
//   node design/art/carrier-furniture/make-starter-mock.mjs
// then render the SVG to PNG as described in README.md.
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const W = 24, H = 16; // hull in tiles; the outer ring is bulkhead
const here = dirname(fileURLToPath(import.meta.url));

// Floor areas (inclusive tile rects). Names are for the diagram only;
// the game stores tiles, not rooms.
const AREAS = [
  ["Bridge", 1, 1, 7, 5],
  ["Quarters", 9, 1, 14, 5],
  ["Lab", 16, 1, 22, 5],
  ["Workshop", 1, 7, 7, 14],
  ["Hangar", 9, 7, 22, 14],
];
const DOORS = [[8, 3], [15, 3], [4, 6], [11, 6], [18, 6], [8, 10]];

// id: [w, d, layer, placement, cost, colour, kind]
// layer: object (blocks walking) | floor (rugs) | wall (mounted on a wall face)
// placement: free | wall (back edge against wall) | mount
const CATALOG = {
  command_console: [2, 1, "object", "wall", 0, "#3b82f6", "station"],
  upgrade_bench: [2, 1, "object", "wall", 0, "#3b82f6", "station"],
  sample_analyzer: [2, 1, "object", "wall", 0, "#3b82f6", "station"],
  berth: [4, 4, "object", "free", 0, "#1d4ed8", "station"],
  salvage_sorter: [2, 2, "object", "free", [120, 3], "#3b82f6", "station"],
  combat_simulator: [2, 2, "object", "free", [200, 6], "#3b82f6", "station"],
  bunk_bed: [1, 2, "object", "wall", 40, "#d97706"],
  locker: [1, 1, "object", "wall", 20, "#d97706"],
  table: [2, 1, "object", "free", 25, "#d97706"],
  stool: [1, 1, "object", "free", 10, "#d97706"],
  cat_bed: [1, 1, "object", "free", 15, "#d97706"],
  cat_tree: [1, 1, "object", "free", 30, "#d97706"],
  crate_stack: [1, 1, "object", "free", 10, "#d97706"],
  potted_plant: [1, 1, "object", "free", 15, "#16a34a"],
  hydro_planter: [2, 1, "object", "free", 35, "#16a34a"],
  floor_lamp: [1, 1, "object", "free", 15, "#d97706"],
  rug: [2, 2, "floor", "free", 20, "#a78bfa"],
  wall_screen: [2, 1, "wall", "mount", 25, "#06b6d4"],
  wall_light: [1, 1, "wall", "mount", 10, "#facc15"],
  tool_rack: [1, 1, "wall", "mount", 15, "#06b6d4"],
};

// [id, x, y, facing] — x, y = north-west tile of the footprint.
const ITEMS = [
  ["command_console", 3, 1, "s"],
  ["wall_screen", 3, 0, "s"],
  ["potted_plant", 1, 1, "s"],
  ["floor_lamp", 7, 1, "s"],
  ["bunk_bed", 9, 1, "s"],
  ["bunk_bed", 10, 1, "s"],
  ["locker", 14, 1, "s"],
  ["rug", 11, 3, "s"],
  ["table", 11, 4, "s"],
  ["cat_bed", 14, 5, "s"],
  ["sample_analyzer", 18, 1, "s"],
  ["potted_plant", 16, 1, "s"],
  ["hydro_planter", 21, 1, "s"],
  ["upgrade_bench", 1, 7, "s"],
  ["tool_rack", 5, 6, "s"],
  ["crate_stack", 7, 14, "s"],
  ["crate_stack", 6, 14, "s"],
  ["berth", 10, 8, "s"],
  ["berth", 16, 8, "s"],
  ["wall_light", 13, 6, "s"],
  ["wall_light", 20, 6, "s"],
];
const CREW = { command_console: "Gunner", upgrade_bench: "Engineer", sample_analyzer: "Researcher" };

const floor = Array.from({ length: H }, () => Array(W).fill(false));
for (const [, x0, y0, x1, y1] of AREAS)
  for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) floor[y][x] = true;
const isFloor = (x, y) => x >= 0 && y >= 0 && x < W && y < H && floor[y][x];
const isDoor = (x, y) => DOORS.some(([dx, dy]) => dx === x && dy === y);
const isWall = (x, y) => !isFloor(x, y) && !isDoor(x, y);

const fail = (m) => { console.error("INVALID: " + m); process.exitCode = 1; };
for (const [x, y] of DOORS) {
  const ns = isFloor(x, y - 1) && isFloor(x, y + 1) && isWall(x - 1, y) && isWall(x + 1, y);
  const ew = isFloor(x - 1, y) && isFloor(x + 1, y) && isWall(x, y - 1) && isWall(x, y + 1);
  if (!(ns || ew)) fail(`door (${x},${y}) is not between two floors`);
}

const used = { object: new Map(), floor: new Map(), wall: new Map() };
const front = []; // [item index, tiles]
ITEMS.forEach(([id, x, y], i) => {
  const [w, d, layer, place] = CATALOG[id];
  for (let ty = y; ty < y + d; ty++)
    for (let tx = x; tx < x + w; tx++) {
      const k = `${tx},${ty}`;
      if (used[layer].has(k)) fail(`${id} overlaps ${used[layer].get(k)} at ${k}`);
      used[layer].set(k, id);
      if (place === "mount") {
        if (!isWall(tx, ty) || !isFloor(tx, ty + 1)) fail(`${id} needs a wall face at ${k}`);
      } else if (!isFloor(tx, ty)) fail(`${id} needs floor at ${k}`);
    }
  if (place === "wall")
    for (let tx = x; tx < x + w; tx++) if (!isWall(tx, y - 1)) fail(`${id} needs a wall behind (${tx},${y - 1})`);
  if (layer === "object" && (CATALOG[id][6] === "station")) {
    const strip = [];
    for (let tx = x; tx < x + w; tx++) strip.push([tx, y + d]);
    front.push([i, strip]);
  }
});
const blocked = (x, y) => used.object.has(`${x},${y}`);
const walk = (x, y) => (isFloor(x, y) || isDoor(x, y)) && !blocked(x, y);
for (const [i, strip] of front)
  for (const [x, y] of strip) if (!walk(x, y)) fail(`${ITEMS[i][0]} front (${x},${y}) is not clear`);

// Reachability from the Command Console's front strip (the root).
const root = front.find(([i]) => ITEMS[i][0] === "command_console")[1][0];
const seen = new Set([`${root[0]},${root[1]}`]);
const queue = [root];
while (queue.length) {
  const [x, y] = queue.shift();
  for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
    const k = `${x + dx},${y + dy}`;
    if (!seen.has(k) && walk(x + dx, y + dy)) { seen.add(k); queue.push([x + dx, y + dy]); }
  }
}
for (const [i, strip] of front)
  if (!strip.some(([x, y]) => seen.has(`${x},${y}`))) fail(`${ITEMS[i][0]} is cut off`);

// Crew stand on the nearest walkable tile beside their post's front strip
// (east, then west, then south of the strip).
const crew = [];
for (const [i, strip] of front) {
  const name = CREW[ITEMS[i][0]];
  if (!name) continue;
  const [lx, y] = strip[0], rx = strip[strip.length - 1][0];
  const spot = [[rx + 1, y], [lx - 1, y], [lx, y + 1]].find(([x, yy]) => walk(x, yy));
  crew.push([name, spot]);
}

let floorTiles = 0;
for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) if (floor[y][x]) floorTiles++;
const value = floorTiles * 2 + DOORS.length * 10 +
  ITEMS.reduce((s, [id]) => s + (typeof CATALOG[id][4] === "number" ? CATALOG[id][4] : 0), 0);
console.log(`floor tiles ${floorTiles}, doors ${DOORS.length}, items ${ITEMS.length}, catalog value ${value} cr`);
console.log("crew:", crew.map(([n, [x, y]]) => `${n} (${x},${y})`).join(", "));

// ASCII map: # wall, . floor, + door, letters = items (object layer).
const LETTER = {
  command_console: "C", upgrade_bench: "U", sample_analyzer: "A", berth: "B",
  bunk_bed: "b", locker: "l", table: "t", cat_bed: "c", potted_plant: "p",
  hydro_planter: "h", floor_lamp: "f", crate_stack: "x",
};
const rows = [];
for (let y = 0; y < H; y++) {
  let r = "";
  for (let x = 0; x < W; x++) {
    const o = used.object.get(`${x},${y}`);
    r += o ? LETTER[o] ?? "?" : isDoor(x, y) ? "+" : isFloor(x, y) ? "." : "#";
  }
  rows.push(`y${String(y).padStart(2)} ${r.split("").join(" ")}`);
}
console.log("     " + Array.from({ length: W }, (_, x) => x % 10).join(" "));
console.log(rows.join("\n"));

// Diagram.
const T = 40, M = 40, LEG = 300;
const out = [];
const sw = W * T + M * 2 + LEG, sh = H * T + M * 2;
out.push(`<svg xmlns="http://www.w3.org/2000/svg" width="${sw}" height="${sh}" font-family="Helvetica, Arial, sans-serif">`);
out.push(`<rect width="${sw}" height="${sh}" fill="#0b1020"/>`);
const px = (x) => M + x * T, py = (y) => M + y * T;
for (let y = 0; y < H; y++)
  for (let x = 0; x < W; x++) {
    if (isFloor(x, y)) out.push(`<rect x="${px(x)}" y="${py(y)}" width="${T}" height="${T}" fill="#4b5563" stroke="#374151"/>`);
    else if (isDoor(x, y)) out.push(`<rect x="${px(x)}" y="${py(y)}" width="${T}" height="${T}" fill="#6b7280"/><rect x="${px(x) + 6}" y="${py(y) + 6}" width="${T - 12}" height="${T - 12}" fill="none" stroke="#fbbf24" stroke-width="3"/>`);
    else {
      out.push(`<rect x="${px(x)}" y="${py(y)}" width="${T}" height="${T}" fill="#1f2937"/>`);
      if (isFloor(x, y + 1)) // the wall's south face, seen in 3/4 view (48 of 64 px)
        out.push(`<rect x="${px(x)}" y="${py(y) + T / 4}" width="${T}" height="${(T * 3) / 4}" fill="#334155"/>`);
    }
  }
for (const [i, strip] of front)
  for (const [x, y] of strip)
    out.push(`<rect x="${px(x) + 3}" y="${py(y) + 3}" width="${T - 6}" height="${T - 6}" fill="none" stroke="#93c5fd" stroke-dasharray="4 3"/>`);
for (const layer of ["floor", "object", "wall"])
  for (const [id, x, y] of ITEMS) {
    const [w, d, l, , , colour] = CATALOG[id];
    if (l !== layer) continue;
    const op = layer === "floor" ? 0.45 : 0.9;
    const ins = layer === "wall" ? 8 : 3;
    out.push(`<rect x="${px(x) + ins}" y="${py(y) + ins}" width="${w * T - ins * 2}" height="${d * T - ins * 2}" rx="5" fill="${colour}" fill-opacity="${op}" stroke="#0f172a" stroke-width="2"/>`);
    const SHORT = { potted_plant: "plant", floor_lamp: "lamp", crate_stack: "crates", cat_bed: "cat bed" };
    const label = SHORT[id] ?? id.replace(/_/g, " ");
    if (layer !== "floor")
      out.push(`<text x="${px(x) + (w * T) / 2}" y="${py(y) + (d * T) / 2 + 4}" font-size="${w * d >= 4 ? 13 : 9}" fill="#fff" text-anchor="middle">${label}</text>`);
  }
for (const [name, [x, y]] of crew) {
  out.push(`<circle cx="${px(x) + T / 2}" cy="${py(y) + T / 2}" r="${T / 3}" fill="#f97316" stroke="#fff" stroke-width="2"/>`);
  out.push(`<text x="${px(x) + T / 2}" y="${py(y) + T / 2 + 4}" font-size="11" font-weight="bold" fill="#fff" text-anchor="middle">${name[0]}</text>`);
}
for (const [name, x0, , , y1] of AREAS)
  out.push(`<text x="${px(x0) + 4}" y="${py(y1 + 1) - 4}" font-size="11" fill="#cbd5e1" opacity="0.8">${name}</text>`);
const lx = M * 2 + W * T;
const legend = [
  ["#4b5563", "Floor tile (64 px, 2 cr)"],
  ["#334155", "Wall face (auto, any non-floor tile)"],
  ["#1f2937", "Wall top / hull bulkhead"],
  ["#fbbf24", "Door (10 cr)"],
  ["#3b82f6", "Work station"],
  ["#d97706", "Furniture"],
  ["#16a34a", "Plants"],
  ["#06b6d4", "Wall-mounted decor"],
  ["#a78bfa", "Rug (floor layer)"],
  ["#93c5fd", "Front strip (kept clear, reachable)"],
  ["#f97316", "Crew post: G Gunner, E Engineer, R Researcher"],
];
out.push(`<text x="${lx}" y="${M + 4}" font-size="16" font-weight="bold" fill="#fff">Starter carrier (24 × 16 tiles)</text>`);
legend.forEach(([c, t], i) => {
  out.push(`<rect x="${lx}" y="${M + 24 + i * 28}" width="18" height="18" fill="${c}"/>`);
  out.push(`<text x="${lx + 26}" y="${M + 38 + i * 28}" font-size="12" fill="#e5e7eb">${t}</text>`);
});
out.push(`<text x="${lx}" y="${M + 24 + legend.length * 28 + 16}" font-size="12" fill="#9ca3af">Area names are labels only:</text>`);
out.push(`<text x="${lx}" y="${M + 24 + legend.length * 28 + 32}" font-size="12" fill="#9ca3af">the game stores tiles and items.</text>`);
out.push(`<text x="${lx}" y="${M + 24 + legend.length * 28 + 56}" font-size="12" fill="#9ca3af">Spawn: Command Console front.</text>`);
out.push(`</svg>`);
writeFileSync(join(here, "mockup-starter-carrier.svg"), out.join("\n"));
