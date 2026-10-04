// "Magical night sky with nebula" (TAKOAI-58): a deep blue-violet sky,
// soft drifting nebula clouds in magenta, teal and violet, and twinkling
// stars at three parallax depths. Client-only and purely visual; see
// `sky.rs`. Kept dark so ships, enemies and shots stay readable.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct Sky {
    // Camera centre in world px.
    camera: vec2<f32>,
    // Seconds since start (visual only).
    time: f32,
    // Overall brightness multiplier.
    brightness: f32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> sky: Sky;

fn hash(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(123.34, 456.21));
    q = q + dot(q, q + 45.32);
    return fract(q.x * q.y);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var amp = 0.5;
    var q = p;
    for (var i = 0; i < 5; i = i + 1) {
        v = v + amp * noise(q);
        q = q * 2.03 + vec2<f32>(17.1, 9.7);
        amp = amp * 0.5;
    }
    return v;
}

// One star layer: at most one star per `cell` px square, `density` of the
// cells lit. `p` is the layer's position in px.
fn stars(p: vec2<f32>, cell: f32, density: f32, size: f32, seed: f32) -> vec3<f32> {
    let g = p / cell;
    let id = floor(g) + vec2<f32>(seed, seed * 1.7);
    let h = hash(id);
    if h > density {
        return vec3<f32>(0.0);
    }
    // Kept 20% in from the cell edge so the halo fits inside the cell.
    let at = vec2<f32>(hash(id + 1.3), hash(id + 7.9)) * 0.6 + 0.2;
    let d = length((fract(g) - at) * cell);
    let r = size * mix(0.6, 1.4, hash(id + 3.1));
    let twinkle = 0.55 + 0.45 * sin(sky.time * mix(0.8, 3.0, hash(id + 5.7)) + h * 60.0);
    let core = smoothstep(r, 0.0, d);
    // Soft halo that reaches zero before the cell edge (no square seams).
    let reach = min(r * 6.0, cell * 0.18);
    let fall = max(1.0 - d / reach, 0.0);
    let halo = fall * fall * 0.3;
    let tint = mix(vec3<f32>(0.75, 0.85, 1.0), vec3<f32>(1.0, 0.8, 0.95), hash(id + 2.2));
    return tint * (core + halo) * twinkle;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Offset from the screen centre, in world px (+y up).
    let screen = mesh.world_position.xy - sky.camera;
    // A layer at depth `k` follows the camera by `k` (1 = fixed to the world).
    let neb = (screen + sky.camera * 0.05) / 700.0;
    let drift = vec2<f32>(sky.time * 0.006, sky.time * -0.004);

    // Sky: deep blue-violet, a little lighter in broad patches.
    let broad = fbm(neb * 0.35 + 3.0);
    var col = mix(vec3<f32>(0.004, 0.003, 0.018), vec3<f32>(0.012, 0.006, 0.035), broad);

    // Nebula: domain-warped fbm, coloured by a second slow field.
    let warp = vec2<f32>(fbm(neb + drift), fbm(neb + vec2<f32>(5.2, 1.3) - drift));
    let n = fbm(neb + 1.6 * warp + drift * 2.0);
    let hue = fbm(neb * 0.6 + vec2<f32>(11.0, 4.0) + drift);
    let magenta = vec3<f32>(0.32, 0.03, 0.22);
    let teal = vec3<f32>(0.02, 0.17, 0.2);
    let violet = vec3<f32>(0.13, 0.04, 0.3);
    var cloud = mix(teal, violet, smoothstep(0.3, 0.55, hue));
    cloud = mix(cloud, magenta, smoothstep(0.5, 0.72, hue));
    let density = smoothstep(0.42, 0.85, n);
    col = col + cloud * density * 0.7;
    // Brighter wisps inside the thickest clouds.
    col = col + cloud * smoothstep(0.7, 0.95, n) * 0.35;

    // Stars, far to near.
    col = col + stars(screen + sky.camera * 0.1, 22.0, 0.22, 0.7, 1.0) * 0.45;
    col = col + stars(screen + sky.camera * 0.25, 46.0, 0.16, 1.0, 7.0) * 0.7;
    col = col + stars(screen + sky.camera * 0.45, 90.0, 0.12, 1.5, 13.0);

    return vec4<f32>(col * sky.brightness, 1.0);
}
