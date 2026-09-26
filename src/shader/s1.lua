return [[
extern vec2 screen;
vec4 effect(vec4 color, Image image, vec2 uvs, vec2 coords) {
    vec4 pixel = Texel(image, uvs);
    vec2 wh2 = vec2(screen.x / 2, screen.y / 2);
    float dis = distance(coords, wh2);
    float scd = max(0, min(1, 1 - (dis - 300) / 500));
    return vec4(scd, scd, scd, 1.0) * pixel * color;
}
]]