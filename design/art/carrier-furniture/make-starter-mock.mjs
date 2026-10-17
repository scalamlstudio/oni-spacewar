// Starter carrier for the rooms-and-furniture interior (TAKOAI-88,
// Revision 3). Checks the layout against the rules in design/READINESS.md
// § Carrier and draws a rough diagram (not shipped art):
//   node design/art/carrier-furniture/make-starter-mock.mjs
// then render the SVG to PNG as described in README.md.
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const W = 36, H = 22; // hull in tiles; anything outside a room is bare hull
const here = dirname(fileURLToPath(import.meta.url));

// Room kinds: interior floor w × d; the wall ring adds one tile all round.
const ROOM_KINDS = { small_room: [6, 4], large_room: [13, 9] };
// [kind, x, y, name] — x, y = north-west wall tile. Names are for the
// diagram only; the game stores rooms by kind and position. `planned`
// rooms are the autoplay's expansion, drawn dashed.
const ROOMS = [
  ["small_room", 0, 0, "Bridge"],
  ["small_room", 7, 0, "Quarters"],
  ["small_room", 14, 0, "Lab"],
  ["large_room", 0, 5, "Hangar"],
  ["small_room", 14, 5, "Workshop"],
  ["small_room", 14, 10, "New room (autoplay)", "planned"],
];

// id: [w, d, layer, placement, colour, kind]
// layer: object (blocks walking) | floor (rugs) | wall (mounted on a wall face)
// placement: free | wall (back edge against the north wall) | mount
const CATALOG = {
  command_console: [2, 1, "object", "wall", "#3b82f6", "station"],
  upgrade_bench: [2, 1, "object", "wall", "#3b82f6", "station"],
  sample_analyzer: [2, 1, "object", "wall", "#3b82f6", "station"],
  berth: [4, 4, "object", "free", "#1d4ed8", "station"],
  salvage_sorter: [2, 2, "object", "free", "#3b82f6", "station"],
  combat_simulator: [2, 2, "object", "free", "#3b82f6", "station"],
  bunk_bed: [1, 2, "object", "wall", "#d97706"],
  locker: [1, 1, "object", "wall", "#d97706"],
  table: [2, 1, "object", "free", "#d97706"],
  cat_bed: [1, 1, "object", "free", "#d97706"],
  crate_stack: [1, 1, "object", "free", "#d97706"],
  potted_plant: [1, 1, "object", "free", "#16a34a"],
  hydro_planter: [2, 1, "object", "free", "#16a34a"],
  floor_lamp: [1, 1, "object", "free", "#d97706"],
  rug: [2, 2, "floor", "free", "#a78bfa"],
  wall_screen: [2, 1, "wall", "mount", "#06b6d4"],
  wall_light: [1, 1, "wall", "mount", "#facc15"],
  tool_rack: [1, 1, "wall", "mount", "#06b6d4"],
};

// [id, x, y, planned?] — x, y = north-west tile of the footprint. Every
// item faces S.
const ITEMS = [
  ["command_console", 3, 1],
  ["wall_screen", 5, 0],
  ["potted_plant", 1, 1],
  ["floor_lamp", 6, 1],
  ["bunk_bed", 9, 1],
  ["bunk_bed", 10, 1],
  ["locker", 12, 1],
  ["cat_bed", 13, 1],
  ["rug", 10, 3],
  ["table", 10, 3],
  ["sample_analyzer", 17, 1],
  ["potted_plant", 15, 1],
  ["hydro_planter", 19, 1],
  ["berth", 2, 8],
  ["berth", 8, 8],
  ["wall_light", 6, 5],
  ["wall_light", 13, 5],
  ["upgrade_bench", 15, 6],
  ["tool_rack", 20, 5],
  ["crate_stack", 19, 9],
  ["crate_stack", 20, 9],
  ["salvage_sorter", 19, 11, "planned"],
  ["combat_simulator", 12, 9, "planned"],
];
const CREW = { command_console: "Gunner", upgrade_bench: "Engineer", sample_analyzer: "Researcher" };

const fail = (m) => { console.error("INVALID: " + m); process.exitCode = 1; };

// Tile map: 0 hull, 1 wall, 2 floor, 3 doorway. Rooms are placed in order,
// each checked against the ones before it, as the game would.
const tile = Array.from({ length: H }, () => Array(W).fill(0));
const rect = ([kind, x, y]) => {
  const [w, d] = ROOM_KINDS[kind];
  return { x0: x, y0: y, x1: x + w + 1, y1: y + d + 1 };
};
const isRing = (r, x, y) => x === r.x0 || x === r.x1 || y === r.y0 || y === r.y1;
const doorways = []; // [x, y, a, b]
ROOMS.forEach((room, i) => {
  const r = rect(room);
  if (r.x0 < 0 || r.y0 < 0 || r.x1 >= W || r.y1 >= H) fail(`${room[3]} is outside the hull`);
  for (let y = r.y0; y <= r.y1; y++)
    for (let x = r.x0; x <= r.x1; x++) {
      const t = tile[y][x];
      if (t === 0) continue;
      if (!(t === 1 && isRing(r, x, y))) fail(`${room[3]} overlaps another room at (${x},${y})`);
    }
  // Doorways: a straight wall shared with an earlier room, with at least 2
  // interior tiles facing each other across it → one 2-tile doorway at the
  // middle of that overlap (rounded north / west).
  let links = 0;
  for (let j = 0; j < i; j++) {
    const o = rect(ROOMS[j]);
    let line = null;
    if (r.x0 === o.x1 || r.x1 === o.x0) {
      const lo = Math.max(r.y0, o.y0) + 1, hi = Math.min(r.y1, o.y1) - 1;
      if (hi - lo + 1 >= 2) line = { axis: "x", at: r.x0 === o.x1 ? r.x0 : r.x1, lo, hi };
    } else if (r.y0 === o.y1 || r.y1 === o.y0) {
      const lo = Math.max(r.x0, o.x0) + 1, hi = Math.min(r.x1, o.x1) - 1;
      if (hi - lo + 1 >= 2) line = { axis: "y", at: r.y0 === o.y1 ? r.y0 : r.y1, lo, hi };
    }
    if (!line) continue;
    const s = line.lo + Math.floor((line.hi - line.lo + 1 - 2) / 2);
    for (const k of [s, s + 1])
      doorways.push(line.axis === "x" ? [line.at, k, j, i] : [k, line.at, j, i]);
    links++;
  }
  if (i > 0 && links === 0) fail(`${room[3]} doesn't share a wall with any room`);
  for (let y = r.y0; y <= r.y1; y++)
    for (let x = r.x0; x <= r.x1; x++)
      if (tile[y][x] !== 3) tile[y][x] = isRing(r, x, y) ? 1 : 2;
  for (const [x, y] of doorways) tile[y][x] = 3;
});
const at = (x, y) => (x >= 0 && y >= 0 && x < W && y < H ? tile[y][x] : 0);
const isFloor = (x, y) => at(x, y) === 2;
const isWall = (x, y) => at(x, y) === 1;
const isDoorway = (x, y) => at(x, y) === 3;

// Each doorway keeps an apron: the floor tile on each side stays free of
// objects.
const apron = new Set();
for (const [x, y] of doorways) {
  const ns = isFloor(x, y - 1) && isFloor(x, y + 1);
  const ew = isFloor(x - 1, y) && isFloor(x + 1, y);
  if (!(ns || ew)) fail(`doorway (${x},${y}) is not between two floors`);
  if (ns) { apron.add(`${x},${y - 1}`); apron.add(`${x},${y + 1}`); }
  else { apron.add(`${x - 1},${y}`); apron.add(`${x + 1},${y}`); }
}

const used = { object: new Map(), floor: new Map(), wall: new Map() };
const front = []; // [item index, tiles]
ITEMS.forEach(([id, x, y], i) => {
  const [w, d, layer, place, , kind] = CATALOG[id];
  for (let ty = y; ty < y + d; ty++)
    for (let tx = x; tx < x + w; tx++) {
      const k = `${tx},${ty}`;
      if (used[layer].has(k)) fail(`${id} overlaps ${used[layer].get(k)} at ${k}`);
      used[layer].set(k, id);
      if (place === "mount") {
        if (!isWall(tx, ty) || !isFloor(tx, ty + 1)) fail(`${id} needs a wall face at ${k}`);
      } else {
        if (!isFloor(tx, ty)) fail(`${id} needs floor at ${k}`);
        if (layer === "object" && apron.has(k)) fail(`${id} blocks a doorway at ${k}`);
      }
    }
  if (place === "wall")
    for (let tx = x; tx < x + w; tx++) if (!isWall(tx, y - 1)) fail(`${id} needs a wall behind (${tx},${y - 1})`);
  if (kind === "station") {
    const strip = [];
    for (let tx = x; tx < x + w; tx++) strip.push([tx, y + d]);
    front.push([i, strip]);
  }
});
const blocked = (x, y) => used.object.has(`${x},${y}`);
const walk = (x, y) => (isFloor(x, y) || isDoorway(x, y)) && !blocked(x, y);
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

const pairs = [...new Set(doorways.map(([, , a, b]) => `${ROOMS[a][3]}–${ROOMS[b][3]}`))];
console.log(`rooms ${ROOMS.length}, doorways ${pairs.length}: ${pairs.join(", ")}`);
console.log("doorway tiles:", doorways.map(([x, y]) => `(${x},${y})`).join(" "));
console.log("crew:", crew.map(([n, [x, y]]) => `${n} (${x},${y})`).join(", "));

// ASCII map: space hull, # wall, . floor, + doorway, letters = objects.
const LETTER = {
  command_console: "C", upgrade_bench: "U", sample_analyzer: "A", berth: "B",
  salvage_sorter: "S", combat_simulator: "M",
  bunk_bed: "b", locker: "l", table: "t", cat_bed: "c", potted_plant: "p",
  hydro_planter: "h", floor_lamp: "f", crate_stack: "x",
};
const rows = [];
for (let y = 0; y < H; y++) {
  let r = "";
  for (let x = 0; x < W; x++) {
    const o = used.object.get(`${x},${y}`);
    r += o ? LETTER[o] ?? "?" : [" ", "#", ".", "+"][at(x, y)];
  }
  rows.push(`y${String(y).padStart(2)} ${r.split("").join(" ").trimEnd()}`);
}
console.log("    " + Array.from({ length: W }, (_, x) => x % 10).join(" "));
console.log(rows.join("\n"));

// Diagram.
const T = 30, M = 30, LEG = 330;
const out = [];
const sw = W * T + M * 2 + LEG, sh = H * T + M * 2;
out.push(`<svg xmlns="http://www.w3.org/2000/svg" width="${sw}" height="${sh}" font-family="Helvetica, Arial, sans-serif">`);
out.push(`<rect width="${sw}" height="${sh}" fill="#0b1020"/>`);
const px = (x) => M + x * T, py = (y) => M + y * T;
const plannedRoom = (x, y) => ROOMS.some((r) => r[4] && (() => {
  const q = rect(r); return x >= q.x0 && x <= q.x1 && y >= q.y0 && y <= q.y1;
})() && !ROOMS.some((s) => !s[4] && (() => {
  const q = rect(s); return x >= q.x0 && x <= q.x1 && y >= q.y0 && y <= q.y1;
})()));
for (let y = 0; y < H; y++)
  for (let x = 0; x < W; x++) {
    const t = at(x, y), op = plannedRoom(x, y) ? 0.45 : 1;
    if (t === 0) out.push(`<rect x="${px(x)}" y="${py(y)}" width="${T}" height="${T}" fill="#151c2c" stroke="#1b2336"/>`);
    else if (t === 2) out.push(`<rect x="${px(x)}" y="${py(y)}" width="${T}" height="${T}" fill="#4b5563" stroke="#374151" opacity="${op}"/>`);
    else if (t === 3) out.push(`<rect x="${px(x)}" y="${py(y)}" width="${T}" height="${T}" fill="#6b7280" opacity="${op}"/><rect x="${px(x) + 3}" y="${py(y) + 3}" width="${T - 6}" height="${T - 6}" fill="none" stroke="#fbbf24" stroke-width="2" opacity="${op}"/>`);
    else {
      out.push(`<rect x="${px(x)}" y="${py(y)}" width="${T}" height="${T}" fill="#1f2937" opacity="${op}"/>`);
      if (isFloor(x, y + 1)) // the wall's south face, seen in 3/4 view
        out.push(`<rect x="${px(x)}" y="${py(y) + T / 4}" width="${T}" height="${(T * 3) / 4}" fill="#334155" opacity="${op}"/>`);
    }
  }
for (const r of ROOMS.filter((r) => r[4])) {
  const q = rect(r);
  out.push(`<rect x="${px(q.x0) + 2}" y="${py(q.y0) + 2}" width="${(q.x1 - q.x0 + 1) * T - 4}" height="${(q.y1 - q.y0 + 1) * T - 4}" fill="none" stroke="#fbbf24" stroke-width="2" stroke-dasharray="8 5"/>`);
}
for (const k of apron) {
  const [x, y] = k.split(",").map(Number);
  out.push(`<circle cx="${px(x) + T / 2}" cy="${py(y) + T / 2}" r="3" fill="#fbbf24" opacity="0.7"/>`);
}
for (const [, strip] of front)
  for (const [x, y] of strip)
    out.push(`<rect x="${px(x) + 3}" y="${py(y) + 3}" width="${T - 6}" height="${T - 6}" fill="none" stroke="#93c5fd" stroke-dasharray="4 3"/>`);
const SHORT = {
  potted_plant: "plant", floor_lamp: "lamp", crate_stack: "crate", cat_bed: "cat", locker: "lkr",
  bunk_bed: "bunk", wall_light: "light", tool_rack: "rack", hydro_planter: "planter",
  command_console: "console", upgrade_bench: "upgrade", sample_analyzer: "analyzer",
  wall_screen: "screen", salvage_sorter: "sorter", combat_simulator: "sim",
};
for (const layer of ["floor", "object", "wall"])
  for (const [id, x, y, planned] of ITEMS) {
    const [w, d, l, , colour] = CATALOG[id];
    if (l !== layer) continue;
    const op = layer === "floor" ? 0.45 : planned ? 0.5 : 0.9;
    const ins = layer === "wall" ? 6 : 2;
    out.push(`<rect x="${px(x) + ins}" y="${py(y) + ins}" width="${w * T - ins * 2}" height="${d * T - ins * 2}" rx="4" fill="${colour}" fill-opacity="${op}" stroke="#0f172a" stroke-width="1.5"${planned ? ' stroke-dasharray="4 3"' : ""}/>`);
    const label = SHORT[id] ?? id.replace(/_/g, " ");
    if (layer !== "floor")
      out.push(`<text x="${px(x) + (w * T) / 2}" y="${py(y) + (d * T) / 2 + 3}" font-size="${w * d >= 4 ? 12 : 8}" fill="#fff" text-anchor="middle">${label}</text>`);
  }
for (const [name, [x, y]] of crew) {
  out.push(`<circle cx="${px(x) + T / 2}" cy="${py(y) + T / 2}" r="${T / 3}" fill="#f97316" stroke="#fff" stroke-width="1.5"/>`);
  out.push(`<text x="${px(x) + T / 2}" y="${py(y) + T / 2 + 4}" font-size="10" font-weight="bold" fill="#fff" text-anchor="middle">${name[0]}</text>`);
}
for (const room of ROOMS) {
  const q = rect(room);
  out.push(`<text x="${px(q.x0 + 1) + 3}" y="${py(q.y1) - 4}" font-size="10" fill="#cbd5e1" opacity="0.85">${room[3]}</text>`);
}
const lx = M * 2 + W * T;
const legend = [
  ["#151c2c", "Bare hull (room space, not walkable)"],
  ["#4b5563", "Room floor (comes with the room)"],
  ["#334155", "Wall face (auto)"],
  ["#1f2937", "Wall top (auto)"],
  ["#fbbf24", "Doorway (auto, 2 tiles) · dot = kept clear"],
  ["#3b82f6", "Work station"],
  ["#d97706", "Furniture"],
  ["#16a34a", "Plants"],
  ["#06b6d4", "Wall-mounted decor"],
  ["#a78bfa", "Rug (floor layer)"],
  ["#93c5fd", "Front strip, south (kept clear, reachable)"],
  ["#f97316", "Crew post: G Gunner, E Engineer, R Researcher"],
];
out.push(`<text x="${lx}" y="${M + 4}" font-size="16" font-weight="bold" fill="#fff">Starter carrier (hull 36 × 22 tiles)</text>`);
legend.forEach(([c, t], i) => {
  out.push(`<rect x="${lx}" y="${M + 24 + i * 28}" width="18" height="18" fill="${c}" stroke="#475569"/>`);
  out.push(`<text x="${lx + 26}" y="${M + 38 + i * 28}" font-size="12" fill="#e5e7eb">${t}</text>`);
});
const notes = [
  "Rooms: small 6 × 4, large 13 × 9 (floor).",
  "Adjacent rooms open a doorway by themselves.",
  "Every item faces S. One deck.",
  "Dashed: the autoplay's expansion (new small",
  "room + Salvage Sorter, Combat Simulator).",
  "Spawn: Command Console front.",
];
notes.forEach((t, i) =>
  out.push(`<text x="${lx}" y="${M + 24 + legend.length * 28 + 16 + i * 17}" font-size="12" fill="#9ca3af">${t}</text>`));
out.push(`</svg>`);
writeFileSync(join(here, "mockup-starter-carrier.svg"), out.join("\n"));
