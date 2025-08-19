// Compute shader simple qui multiplie des nombres
struct ComputeInput {
    value: f32,
    multiplier: f32,
}


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
fn hash3(p: vec3<u32>) -> f32 {
    // Hash rapide déterministe -> [0,1)
    var n = p.x * 1103515245u + p.y * 12345u + p.z * 2654435761u;
    n = (n ^ (n >> 13u)) * 1274126177u;
    let res = f32((n ^ (n >> 16u)) & 0x7fffffffu) / f32(0x7fffffffu);
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
    let pi = vec3<u32>(floor(p));
    let pf = p - floor(p);

    let h000 = hash3(pi + vec3<u32>(0,0,0));
    let h100 = hash3(pi + vec3<u32>(1,0,0));
    let h010 = hash3(pi + vec3<u32>(0,1,0));
    let h110 = hash3(pi + vec3<u32>(1,1,0));
    let h001 = hash3(pi + vec3<u32>(0,0,1));
    let h101 = hash3(pi + vec3<u32>(1,0,1));
    let h011 = hash3(pi + vec3<u32>(0,1,1));
    let h111 = hash3(pi + vec3<u32>(1,1,1));

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

@group(0) @binding(0) var<storage, read> input_data: array<ComputeInput>;
@group(0) @binding(1) var<storage, read_write> output_data: array<f32>;

@group(0) @binding(2)
var<uniform> offset: u32;

@compute @workgroup_size(2)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // let index = global_id.x;
    let index = global_id.x + offset;
    
    // Vérifier les limites
    if (index >= arrayLength(&input_data)) {
        return;
    }

    var one1 : f32 = 0;
    
    for (var i = 0u; i < 100000u; i = i + 1u) {
        one1 += fbm_perlin_noise(0.75, 0.756, 0.27, 10, 0.7, 2.0);
    }
    
    // Calcul simple: multiplier value par multiplier
    let input = input_data[index];
    let result = input.value * input.multiplier + one1;
    
    // Ajouter un peu de complexité pour montrer le parallélisme
    var complex_result = result;
    for (var i = 0u; i < 1000u; i = i + 1u) {
        complex_result = sin(complex_result) * cos(complex_result) + result;
    }
    
    output_data[index] = complex_result;
}
