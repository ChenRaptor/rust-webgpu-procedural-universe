// compute_vertices.wgsl — Port fidèle de votre compute_vertices CPU (déformation + biomes + palettes)
// Entrées/Sorties:
//  @group(0)@binding(0): Params (uniform)
//  @group(0)@binding(1): in_vertices : array<vec3<f32>>  (positions unitaires icosphere)
//  @group(0)@binding(2): out_positions : array<vec3<f32>> (positions déformées)
//  @group(0)@binding(3): out_colors    : array<vec3<f32>> (couleurs finales)

struct Params {
    radius: f32,
    sea_level: f32,
    height_amplitude: f32,

    continent_octaves: u32,
    continent_persistence: f32,
    continent_noise_scale: f32,

    big_mountain_octaves: u32,
    big_mountain_persistence: f32,
    big_mountain_noise_scale: f32,

    mountain_octaves: u32,
    mountain_persistence: f32,
    mountain_noise_scale: f32,

    biome_octaves: u32,
    biome_persistence: f32,
    biome_noise_scale: f32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> in_vertices: array<vec3<f32>>;
@group(0) @binding(2) var<storage, read_write> out_positions: array<vec3<f32>>;
@group(0) @binding(3) var<storage, read_write> out_colors: array<vec3<f32>>;

// -----------------------------
// Palettes (hardcodées comme en CPU)
// -----------------------------
struct ColorPoint { key: f32, color: vec3<f32> };

// 0: Ocean
const OCEAN_COUNT: u32 = 4u;
const OCEAN: array<ColorPoint, 4u> = array<ColorPoint, 4u>(
    ColorPoint( -0.2, vec3<f32>(0.0, 0.0, 0.1882353) ),     // 0x000030
    ColorPoint( -0.1, vec3<f32>(0.0, 0.0, 0.2549020) ),     // 0x000041
    ColorPoint( -0.005, vec3<f32>(0.2078431, 0.4117647, 0.5490196) ), // 0x35698C
    ColorPoint(  0.0, vec3<f32>(0.2509804, 0.8784314, 0.8156863) )    // 0x40E0D0
);

// 1: Desert
const DESERT_COUNT: u32 = 3u;
const DESERT: array<ColorPoint, 3u> = array<ColorPoint, 3u>(
    ColorPoint( 0.0, vec3<f32>(0.7607843, 0.6980392, 0.5019608) ), // 0xC2B280
    ColorPoint( 0.5, vec3<f32>(0.9333333, 0.8627451, 0.5098039) ), // 0xEEDC82
    ColorPoint( 1.0, vec3<f32>(1.0, 0.8941177, 0.7098039) )        // 0xFFE4B5
);

// 2: Forest
const FOREST_COUNT: u32 = 3u;
const FOREST: array<ColorPoint, 3u> = array<ColorPoint, 3u>(
    ColorPoint( -1.0, vec3<f32>(0.0196078, 0.2509804, 0.0392157) ), // 0x05400A
    ColorPoint(  0.0, vec3<f32>(0.3215686, 0.4392157, 0.2823529) ), // 0x527048
    ColorPoint(  1.0, vec3<f32>(0.4862745, 0.9882353, 0.0) )        // 0x7CFC00
);

// 3: Tundra
const TUNDRA_COUNT: u32 = 2u;
const TUNDRA: array<ColorPoint, 2u> = array<ColorPoint, 2u>(
    ColorPoint( 0.0, vec3<f32>(0.6235294, 0.6588235, 0.6392157) ), // 0x9FA8A3
    ColorPoint( 1.0, vec3<f32>(0.8627451, 0.8901961, 0.8823529) )  // 0xDCE3E1
);

// 4: Montagne (gris)
const MOUNTAIN_COUNT: u32 = 6u;
const MOUNTAIN: array<ColorPoint, 6u> = array<ColorPoint, 6u>(
    ColorPoint( 0.0,  vec3<f32>(0.0, 0.0, 0.0) ),                  // 0x000000
    ColorPoint( 0.01, vec3<f32>(0.1333333, 0.1333333, 0.1333333) ),// 0x222222
    ColorPoint( 0.05, vec3<f32>(0.2, 0.2, 0.2) ),                  // 0x333333
    ColorPoint( 0.09, vec3<f32>(0.4, 0.4, 0.4) ),                  // 0x666666
    ColorPoint( 0.10, vec3<f32>(0.4666667, 0.4666667, 0.4666667) ),// 0x777777
    ColorPoint( 0.90, vec3<f32>(0.5490196, 0.5490196, 0.6117647) ) // 0x8c8c9c
);

// 5: Snow
const SNOW_COUNT: u32 = 2u;
const SNOW: array<ColorPoint, 2u> = array<ColorPoint, 2u>(
    ColorPoint( 0.0, vec3<f32>(0.9333333, 0.9333333, 0.9333333) ), // 0xEEEEEE
    ColorPoint( 1.0, vec3<f32>(1.0, 1.0, 1.0) )                    // 0xFFFFFF
);

fn color_from_noise(noise_value: f32, biome_idx: u32) -> vec3<f32> {
    if (biome_idx == 0u) { // ocean
        return color_from_palette(noise_value, &OCEAN, OCEAN_COUNT);
    } else if (biome_idx == 1u) { // desert
        return color_from_palette(noise_value, &DESERT, DESERT_COUNT);
    } else if (biome_idx == 2u) { // forest
        return color_from_palette(noise_value, &FOREST, FOREST_COUNT);
    } else if (biome_idx == 3u) { // tundra
        return color_from_palette(noise_value, &TUNDRA, TUNDRA_COUNT);
    } else if (biome_idx == 5u) { // snow
        return color_from_palette(noise_value, &SNOW, SNOW_COUNT);
    }
    // default sur montagne
    return color_from_palette(noise_value, &MOUNTAIN, MOUNTAIN_COUNT);
}

fn color_from_palette(noise_value: f32, palette: ptr<function, array<ColorPoint>>, count: u32) -> vec3<f32> {
    // Équivalent exact à get_color_from_noise côté CPU
    if (count == 0u) { return vec3<f32>(0.0,0.0,0.0); }
    let first = (*palette)[0u];
    let last  = (*palette)[count - 1u];
    if (noise_value <= first.key) { return first.color; }
    if (noise_value >= last.key)  { return last.color; }
    for (var i: u32 = 0u; i < count - 1u; i = i + 1u) {
        let a = (*palette)[i];
        let b = (*palette)[i + 1u];
        if (noise_value >= a.key && noise_value <= b.key) {
            let t = (noise_value - a.key) / max(b.key - a.key, 1e-6);
            return mix(a.color, b.color, t);
        }
    }
    return last.color;
}

// -----------------------------
// Math utils / bruit Perlin + fBm
// -----------------------------
fn fade(t: f32) -> f32 { return t*t*t*(t*(t*6.0 - 15.0) + 10.0); }
fn lerp(a: f32, b: f32, t: f32) -> f32 { return a + (b - a) * t; }
fn smoothstep_fn(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}
fn tanh_approx(x: f32) -> f32 {
    let e2x = exp(2.0 * x);
    return (e2x - 1.0) / (e2x + 1.0);
}

// Hash/gradient helpers
fn hash3(p: vec3<i32>) -> f32 {
    // Hash rapide déterministe -> [0,1)
    var n = p.x * 1103515245 + p.y * 12345 + p.z * 2654435761;
    n = (n ^ (n >> 13)) * 1274126177;
    let res = f32((n ^ (n >> 16)) & 0x7fffffff) / f32(0x7fffffff);
    return res;
}

fn grad(hash: f32, x: f32, y: f32, z: f32) -> f32 {
    let h = floor(hash * 16.0);
    let u = select(y, x, (h < 8.0) || (h == 12.0) || (h == 13.0));
    let v = select(z, y, (h < 4.0) || (h == 12.0) || (h == 13.0));
    let s1 = select(-u, u, fract(h/2.0) < 0.5);
    let s2 = select(-v, v, fract(h/4.0) < 0.5);
    return s1 + s2; // approx gradient dot
}

fn perlin3(p: vec3<f32>) -> f32 {
    let pi = vec3<i32>(floor(p));
    let pf = p - floor(p);

    let h000 = hash3(pi + vec3<i32>(0,0,0));
    let h100 = hash3(pi + vec3<i32>(1,0,0));
    let h010 = hash3(pi + vec3<i32>(0,1,0));
    let h110 = hash3(pi + vec3<i32>(1,1,0));
    let h001 = hash3(pi + vec3<i32>(0,0,1));
    let h101 = hash3(pi + vec3<i32>(1,0,1));
    let h011 = hash3(pi + vec3<i32>(0,1,1));
    let h111 = hash3(pi + vec3<i32>(1,1,1));

    let u = vec3<f32>(fade(pf.x), fade(pf.y), fade(pf.z));

    let x00 = lerp(grad(h000, pf.x, pf.y, pf.z), grad(h100, pf.x-1.0, pf.y, pf.z), u.x);
    let x10 = lerp(grad(h010, pf.x, pf.y-1.0, pf.z), grad(h110, pf.x-1.0, pf.y-1.0, pf.z), u.x);
    let x01 = lerp(grad(h001, pf.x, pf.y, pf.z-1.0), grad(h101, pf.x-1.0, pf.y, pf.z-1.0), u.x);
    let x11 = lerp(grad(h011, pf.x, pf.y-1.0, pf.z-1.0), grad(h111, pf.x-1.0, pf.y-1.0, pf.z-1.0), u.x);

    let y0 = lerp(x00, x10, u.y);
    let y1 = lerp(x01, x11, u.y);
    // Remap to [-1,1]
    return lerp(y0, y1, u.z) * 2.0 - 1.0;
}

fn fbm_perlin_noise(px: f32, py: f32, pz: f32, octaves: u32, persistence: f32, scale: f32) -> f32 {
    var amp = 1.0;
    var freq = scale;
    var sum = 0.0;
    var norm = 0.0;
    let p = vec3<f32>(px, py, pz);
    for (var i: u32 = 0u; i < octaves; i = i + 1u) {
        let n = perlin3(p * freq);
        sum += n * amp;
        norm += amp;
        amp *= persistence;
        freq *= 2.0;
    }
    return sum / max(norm, 1e-6);
}

// -----------------------------
// Température / Humidité (fidèles au CPU)
// -----------------------------
fn compute_temperature(latitude: f32, altitude: f32, v: vec3<f32>) -> f32 {
    var base_temp = 1.0 - abs(latitude - 0.5) * 2.0 - altitude * 0.7;
    base_temp = clamp(base_temp, 0.0, 1.0);
    let t1 = fbm_perlin_noise(v.x, v.y, v.z, 4u, 0.9, 2.0);
    let t2 = fbm_perlin_noise(v.x, v.y, v.z, 4u, 0.9, 20.0);
    return base_temp + 0.3 * t1 + 0.15 * t2;
}

fn compute_humidity(v: vec3<f32>) -> f32 {
    let h1 = fbm_perlin_noise(v.x + 100.0, v.y + 100.0, v.z + 100.0, 4u, 0.5, 2.0);
    let h2 = fbm_perlin_noise(v.x + 200.0, v.y + 200.0, v.z + 200.0, 4u, 0.6, 20.0);
    var h = 0.7 * h1 + 0.3 * h2;
    h = (h + 1.0) * 0.5 * 0.70;
    return h;
}

fn get_biome_index(temperature: f32, humidity: f32, altitude: f32, sea_level: f32) -> u32 {
    if (altitude < sea_level) { return 0u; }
    if (temperature > 0.7) {
        if (humidity < 0.3) { return 1u; } else { return 2u; }
    } else if (temperature > 0.3) {
        if (humidity < 0.3) { return 3u; } else { return 2u; }
    } else {
        if (humidity < 0.3) { return 3u; } else { return 5u; }
    }
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= arrayLength(&in_vertices)) { return; }

    let v = in_vertices[i];                  // direction normalisée

    // Bruits fBm
    let continent_noise = fbm_perlin_noise(v.x, v.y, v.z, params.continent_octaves, params.continent_persistence, params.continent_noise_scale);
    let big_mountain_noise = fbm_perlin_noise(v.x, v.y, v.z, params.big_mountain_octaves, params.big_mountain_persistence, params.big_mountain_noise_scale);
    let mountain_noise = fbm_perlin_noise(v.x, v.y, v.z, params.mountain_octaves, params.mountain_persistence, params.mountain_noise_scale);
    let biome_noise = fbm_perlin_noise(v.x, v.y, v.z, params.biome_octaves, params.biome_persistence, params.biome_noise_scale);

    let latitude = acos(clamp(v.y, -1.0, 1.0)) / 3.141592653589793;

    let continent_factor = mountain_noise * big_mountain_noise * 0.6 + continent_noise * 0.4;
    let weight_continent = smoothstep_fn(0.0, 0.1, continent_noise);
    let weight_big_mountain = smoothstep_fn(0.0, 0.2, big_mountain_noise);

    var deformed_radius = params.radius + continent_factor * params.height_amplitude;
    deformed_radius += weight_big_mountain * weight_continent * big_mountain_noise * params.height_amplitude / 4.0;

    var under_water = deformed_radius <= params.sea_level;
    if (under_water) { deformed_radius = params.sea_level; }

    out_positions[i] = v * deformed_radius;

    if (under_water) {
        // couleur océan depuis la palette 0 avec continent_factor
        let c = color_from_noise(continent_factor, 0u);
        out_colors[i] = c;
        return;
    }

    let altitude_normalized = (deformed_radius - params.radius) / params.height_amplitude;
    let temperature = compute_temperature(latitude, altitude_normalized, v);
    let humidity = compute_humidity(v);

    let biome_idx = get_biome_index(temperature, humidity, deformed_radius, params.sea_level);
    let biome_color = color_from_noise(biome_noise, biome_idx);

    let factor = mountain_noise * big_mountain_noise;
    let mountain_color = color_from_noise(factor, 4u); // palette montagne/gris

    let abs_factor = abs(tanh_approx(20.0 * factor));
    let inv_mix = 0.5 - abs_factor * 0.5;
    let mixv = 0.5 + abs_factor * 0.5;

    let final_color = biome_color * inv_mix + mountain_color * mixv;
    out_colors[i] = final_color;
}
